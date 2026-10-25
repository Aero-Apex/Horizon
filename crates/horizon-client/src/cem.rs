//! Custom entity models: OptiFine's CEM format (`assets/minecraft/optifine/cem/*.jem`), the
//! models and animations of packs such as Fresh Animations. Read as Entity Model Features
//! (LGPL-3.0, see THIRD_PARTY_NOTICES.md) reads them.
//!
//! A `.jem` part hangs from one of the entity's own (vanilla) model parts, which keeps its
//! pose from the game's own animation, and replaces that part's boxes unless it says
//! `attach`. Its `animations` are expressions that run every frame, in order, reading the
//! entity's state and the parts' current poses and writing parts' poses and the entity's
//! variables (`var.x`, which persist from frame to frame).
//!
//! `expr` reads the expressions, `jem` the model files; this module ties a model to the
//! game's model of the same entity and runs it.

mod expr;
mod jem;

use crate::mobs::{
    Mob, Part,
    anim::{Poses, State},
};
use expr::{Builtin, Ctx, Expr, PartProp, Resolver};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError},
};

/// OptiFine's part names for each model, and the game's names for them
/// (`optifine_parts.json`, from Entity Model Features' tables).
fn part_names() -> &'static HashMap<String, HashMap<String, String>> {
    static NAMES: std::sync::OnceLock<HashMap<String, HashMap<String, String>>> =
        std::sync::OnceLock::new();
    NAMES.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/cem/optifine_parts.json"))
            .expect("optifine_parts.json is valid")
    })
}

/// A rule of a `.properties` file: the first whose conditions hold sets `rule_index`.
#[derive(Debug)]
struct Rule {
    index: i32,
    blocks: Vec<String>,
    nbt: Vec<(String, String)>,
    /// A condition this reader does not know: the rule never holds.
    unknown: bool,
}

fn rules_of(text: &str) -> Vec<Rule> {
    let props = crate::ctm::parse_properties(text);
    let mut rules: std::collections::BTreeMap<i32, Rule> = Default::default();
    for (key, value) in &props {
        let mut parts = key.splitn(3, '.');
        let (Some(kind), Some(n)) = (parts.next(), parts.next()) else {
            continue;
        };
        let Ok(index) = n.parse::<i32>() else {
            continue;
        };
        let rule = rules.entry(index).or_insert_with(|| Rule {
            index,
            blocks: Vec::new(),
            nbt: Vec::new(),
            unknown: false,
        });
        match kind {
            "models" => {}
            "blocks" => {
                rule.blocks = value
                    .split_whitespace()
                    .map(|b| {
                        let b = b.strip_prefix("minecraft:").unwrap_or(b);
                        b.split(':').next().unwrap_or(b).to_owned()
                    })
                    .collect();
            }
            "nbt" => match parts.next() {
                Some(path) => rule.nbt.push((path.to_owned(), value.clone())),
                None => rule.unknown = true,
            },
            _ => rule.unknown = true,
        }
    }
    // A rule needs its `models.N`.
    rules
        .into_values()
        .filter(|r| props.contains_key(&format!("models.{}", r.index)))
        .collect()
}

/// What the world tells about the block a mob stands in and on.
#[derive(Clone, Copy, Default)]
pub struct Under<'a> {
    pub at: &'a str,
    pub below: &'a str,
}

/// The `nbt.N.<key>` conditions this reader understands.
fn nbt_holds(m: &Mob, key: &str, expected: &str) -> bool {
    use crate::mobs::Species;
    let expected = expected.trim();
    match key {
        // Snow golem: a pumpkin head, unless it was sheared.
        "Pumpkin" => {
            m.species() == Species::SnowGolem
                && !m.extra.sheared
                && matches!(expected, "raw:1b" | "1" | "1b" | "true")
        }
        "sleeping_pos" => {
            let sleeping = crate::mobs::villager::sleeping(m);
            match expected {
                "exists:true" => sleeping,
                "exists:false" => !sleeping,
                _ => false,
            }
        }
        "HasNectar" => {
            m.species() == Species::Bee
                && crate::mobs::bee::has_nectar(m) == matches!(expected, "1" | "1b" | "true")
        }
        _ => false,
    }
}

/// Everything about this frame the expressions can read.
#[derive(Clone, Copy, Debug, Default)]
struct Frame {
    counter: u64,
    time: f32,
    ticks: u64,
    paused: bool,
}

#[derive(Default)]
struct Runtime {
    frame: Frame,
    rules: HashMap<u32, i32>,
    vars: HashMap<u32, HashMap<String, f32>>,
    seen: HashMap<u32, u64>,
    globals: HashMap<String, f32>,
    last_frame: Option<std::time::Instant>,
}

/// A model ready to draw: the game's model with the pack's parts hung on it, and the
/// animations, parsed.
pub struct Built {
    pub tree: Arc<Part>,
    /// The atlas sprite of the model's own texture (`texture` of the file), when it has one.
    pub texture: Option<String>,
    /// The part table the animations index (names in `tree`'s poses).
    names: Vec<String>,
    lines: Vec<Line>,
    vars: Vec<(String, bool)>,
    globals: Vec<(String, bool)>,
    /// Parts of the pack with no part of the game's model to hang from.
    pub dropped: Vec<String>,
    warned: std::sync::atomic::AtomicBool,
}

struct Line {
    key: String,
    target: Target,
    expr: Expr,
}

enum Target {
    Part(usize, PartProp),
    Var(u32, bool),
    Global(u32, bool),
    Ignore,
}

/// The pack's models, found in a resource root.
pub struct Cem {
    dir: PathBuf,
    files: HashSet<String>,
    rules: HashMap<String, Vec<Rule>>,
    built: Mutex<HashMap<(String, String), Option<Arc<Built>>>>,
    runtime: Mutex<Runtime>,
}

impl Default for Cem {
    fn default() -> Self {
        Self {
            dir: PathBuf::new(),
            files: HashSet::new(),
            rules: HashMap::new(),
            built: Mutex::new(HashMap::new()),
            runtime: Mutex::new(Runtime::default()),
        }
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Cem {
    /// The models under `root/assets/minecraft/optifine/cem`.
    pub fn load(root: &Path) -> Self {
        let dir = root.join("assets/minecraft/optifine/cem");
        let mut files = HashSet::new();
        let mut rules = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if let Some(stem) = name.strip_suffix(".jem") {
                    files.insert(stem.to_owned());
                } else if let Some(stem) = name.strip_suffix(".properties")
                    && let Ok(text) = std::fs::read_to_string(entry.path())
                {
                    let r = rules_of(&text);
                    if !r.is_empty() {
                        rules.insert(stem.to_owned(), r);
                    }
                }
            }
        }
        if !files.is_empty() {
            log::info!(
                "Horizon: {} custom entity models in {}",
                files.len(),
                dir.display()
            );
        }
        Self {
            dir,
            files,
            rules,
            built: Mutex::new(HashMap::new()),
            runtime: Mutex::new(Runtime::default()),
        }
    }

    /// A pack with no custom models.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn has_model(&self, name: &str) -> bool {
        self.files.contains(name)
    }

    /// Called once per rendered frame, with the game's tick count.
    pub fn begin_frame(&self, ticks: u64, paused: bool) {
        if self.is_empty() {
            return;
        }
        let mut rt = lock(&self.runtime);
        let now = std::time::Instant::now();
        let seconds = rt
            .last_frame
            .replace(now)
            .map_or(0.0, |before| now.duration_since(before).as_secs_f32());
        rt.frame.counter += 1;
        rt.frame.ticks = ticks;
        rt.frame.paused = paused;
        // Expressions divide by this, so it is never 0.
        rt.frame.time = if paused {
            1e-9
        } else {
            seconds.clamp(1e-9, 0.25)
        };
        if rt.frame.counter.is_multiple_of(600) {
            let now = rt.frame.counter;
            let stale: Vec<u32> = rt
                .seen
                .iter()
                .filter(|(_, at)| now - **at > 1200)
                .map(|(id, _)| *id)
                .collect();
            for id in stale {
                rt.seen.remove(&id);
                rt.vars.remove(&id);
                rt.rules.remove(&id);
            }
        }
    }

    /// Whether the model `name` picks its rule by the blocks around the mob.
    pub fn wants_blocks(&self, name: &str) -> bool {
        self.rules
            .get(name)
            .is_some_and(|rules| rules.iter().any(|r| !r.blocks.is_empty()))
    }

    /// The rule of `name`'s `.properties` that holds for `m` (0 when none does).
    pub fn set_rule(&self, name: &str, m: &Mob, under: Under) {
        let Some(rules) = self.rules.get(name) else {
            return;
        };
        let strip = |b: &str| b.strip_prefix("minecraft:").unwrap_or(b).to_owned();
        let (at, below) = (strip(under.at), strip(under.below));
        let index = rules
            .iter()
            .find(|r| {
                !r.unknown
                    && (r.blocks.is_empty() || r.blocks.contains(&at) || r.blocks.contains(&below))
                    && r.nbt.iter().all(|(k, v)| nbt_holds(m, k, v))
            })
            .map_or(0, |r| r.index);
        lock(&self.runtime).rules.insert(m.id, index);
    }

    /// The model `name` hung on `vanilla` (the game's own model for `key`), if the pack
    /// has one.
    pub fn model(&self, name: &str, key: &str, vanilla: &Arc<Part>) -> Option<Arc<Built>> {
        if !self.has_model(name) {
            return None;
        }
        let cache_key = (name.to_owned(), key.to_owned());
        if let Some(found) = lock(&self.built).get(&cache_key) {
            return found.clone();
        }
        let built = match self.build(name, vanilla) {
            Ok(built) => Some(Arc::new(built)),
            Err(e) => {
                log::warn!("Horizon: custom entity model {name}: {e}");
                None
            }
        };
        lock(&self.built).insert(cache_key, built.clone());
        built
    }

    fn build(&self, name: &str, vanilla: &Arc<Part>) -> Result<Built, String> {
        struct Dir<'a>(&'a Path);
        impl jem::Files for Dir<'_> {
            fn jpm(&self, path: &str) -> Option<String> {
                let path = path.strip_prefix("./").unwrap_or(path);
                std::fs::read_to_string(self.0.join(path)).ok()
            }
        }
        let text = std::fs::read_to_string(self.dir.join(format!("{name}.jem")))
            .map_err(|e| e.to_string())?;
        let raw = jem::parse_jem(&text)?;
        let known = part_names().get(name);
        let mut names: HashMap<String, String> = HashMap::new();
        if let Some(map) = known {
            names.clone_from(map);
        }
        let model = jem::build(raw, &names, &Dir(&self.dir))?;
        assemble(vanilla, model, known)
    }

    /// Runs `built`'s animations for `m` over `poses`.
    pub fn animate(
        &self,
        built: &Built,
        m: &Mob,
        state: &State,
        camera: glam::DVec3,
        poses: &mut Poses,
    ) {
        let skipped = self.run(built, m, state, camera, poses);
        if !skipped.is_empty()
            && !built
                .warned
                .swap(true, std::sync::atomic::Ordering::Relaxed)
        {
            log::warn!(
                "Horizon: custom entity model animations gave no number for {} lines, such as {:?}",
                skipped.len(),
                &skipped[..skipped.len().min(6)]
            );
        }
    }

    /// The lines that gave no number (and so changed nothing).
    fn run(
        &self,
        built: &Built,
        m: &Mob,
        state: &State,
        camera: glam::DVec3,
        poses: &mut Poses,
    ) -> Vec<String> {
        if built.lines.is_empty() {
            return Vec::new();
        }
        let mut rt = lock(&self.runtime);
        let frame = rt.frame;
        let rule = rt.rules.get(&m.id).copied().unwrap_or(0);
        rt.seen.insert(m.id, frame.counter);
        let stored = rt.vars.entry(m.id).or_default();
        let load = |table: &[(String, bool)], from: &HashMap<String, f32>| -> Vec<f32> {
            table
                .iter()
                .map(|(key, is_bool)| {
                    from.get(key)
                        .copied()
                        .unwrap_or(if *is_bool { expr::FALSE } else { 0.0 })
                })
                .collect()
        };
        let vars = load(&built.vars, stored);
        let globals = load(&built.globals, &rt.globals);
        let mut env = Env {
            m,
            state,
            camera,
            frame,
            rule,
            poses,
            built,
            vars,
            globals,
        };
        let mut skipped: Vec<String> = Vec::new();
        for line in &built.lines {
            let value = line.expr.eval(&mut env);
            if value.is_nan() {
                skipped.push(line.key.clone());
                continue;
            }
            match line.target {
                Target::Part(part, prop) => env.write_part(part, prop, value),
                Target::Var(slot, is_bool) => {
                    env.vars[slot as usize] = coerce(value, is_bool);
                }
                Target::Global(slot, is_bool) => {
                    env.globals[slot as usize] = coerce(value, is_bool);
                }
                Target::Ignore => {}
            }
        }
        let Env { vars, globals, .. } = env;
        let stored = rt.vars.entry(m.id).or_default();
        for ((key, _), value) in built.vars.iter().zip(vars) {
            stored.insert(key.clone(), value);
        }
        for ((key, _), value) in built.globals.iter().zip(globals) {
            rt.globals.insert(key.clone(), value);
        }
        skipped
    }
}

/// A `varb` keeps booleans only, a `var` numbers only.
fn coerce(value: f32, is_bool: bool) -> f32 {
    if is_bool {
        expr::from_bool(expr::to_bool(value))
    } else if value.is_infinite() {
        0.0
    } else {
        value
    }
}

struct Env<'a> {
    m: &'a Mob,
    state: &'a State,
    camera: glam::DVec3,
    frame: Frame,
    rule: i32,
    poses: &'a mut Poses,
    built: &'a Built,
    vars: Vec<f32>,
    globals: Vec<f32>,
}

impl Env<'_> {
    fn position(&self) -> [f32; 3] {
        let (a, m) = (f64::from(self.state.partial), self.m);
        [
            (m.previous.x + (m.position.x - m.previous.x) * a) as f32,
            (m.previous.y + (m.position.y - m.previous.y) * a) as f32,
            (m.previous.z + (m.position.z - m.previous.z) * a) as f32,
        ]
    }

    fn write_part(&mut self, part: usize, prop: PartProp, value: f32) {
        let name = &self.built.names[part];
        let slot = match prop {
            PartProp::Tx => 0,
            PartProp::Ty => 1,
            PartProp::Tz => 2,
            PartProp::Rx => 3,
            PartProp::Ry => 4,
            PartProp::Rz => 5,
            PartProp::Sx => 6,
            PartProp::Sy => 7,
            PartProp::Sz => 8,
            PartProp::Visible => return self.poses.visible(name, expr::to_bool(value)),
            PartProp::VisibleBoxes => return self.poses.draw_boxes(name, expr::to_bool(value)),
        };
        self.poses.get(name)[slot] = value;
    }
}

const TICKS_WRAP: u64 = 27720;

impl Ctx for Env<'_> {
    fn builtin(&mut self, b: Builtin) -> f32 {
        use Builtin::*;
        let (m, s) = (self.m, self.state);
        let flag = expr::from_bool;
        match b {
            LimbSwing => s.pos * if m.is_baby() { 3.0 } else { 1.0 },
            LimbSpeed => s.speed,
            Age => (u64::from(m.age) % TICKS_WRAP) as f32 + s.partial,
            FrameTime => self.frame.time,
            FrameCounter => self.frame.counter as f32,
            HeadYaw => s.y_rot,
            HeadPitch => s.x_rot,
            SwingProgress => s.swing,
            HurtTime => {
                if m.hurt_time > 0 {
                    m.hurt_time as f32 - s.partial
                } else {
                    0.0
                }
            }
            DeathTime => {
                if m.death_time > 0 {
                    m.death_time as f32 + s.partial
                } else {
                    0.0
                }
            }
            Health => m.health,
            MaxHealth => m.attrs.max_health,
            Id => (u64::from(m.id) % TICKS_WRAP) as f32,
            Time => (self.frame.ticks % TICKS_WRAP) as f32 + s.partial,
            DayTime => (self.frame.ticks % 24000) as f32 + s.partial,
            DayCount => (self.frame.ticks / 24000) as f32,
            RuleIndex => self.rule as f32,
            Dimension => 0.0,
            PosX => self.position()[0],
            PosY => self.position()[1],
            PosZ => self.position()[2],
            RotX => (m.previous_head_pitch + (m.head_pitch - m.previous_head_pitch) * s.partial)
                .to_radians(),
            RotY => {
                let body = m.previous_body_yaw
                    + crate::mobs::living::wrap(m.body_yaw - m.previous_body_yaw) * s.partial;
                body.to_radians()
            }
            PlayerPosX => self.camera.x as f32,
            PlayerPosY => self.camera.y as f32,
            PlayerPosZ => self.camera.z as f32,
            Distance => {
                let p = self.position();
                (glam::Vec3::from_array(p) - self.camera.as_vec3()).length()
            }
            AngerTime => m.extra.anger.max(0) as f32,
            Nan => f32::NAN,
            IsChild => flag(m.is_baby()),
            IsInWater | IsWet => flag(m.fluids.in_water),
            IsRiding => flag(m.extra.passenger),
            IsOnGround => flag(m.on_ground),
            IsAggressive => flag(m.ai.aggressive),
            IsHurt => flag(m.hurt_time > 0),
            IsSitting => flag(m.extra.sitting),
            IsRidden => flag(m.rider.is_some()),
            IsInLava => flag(m.fluids.in_lava),
            IsAlive => flag(m.alive()),
            IsTamed => flag(m.extra.tame),
            IsBurning => flag(m.fire_ticks > 0),
            IsGlowing => flag(m.glowing),
            IsInvisible => flag(m.invisible),
            IsClimbing => flag(m.climbing),
            IsPaused => flag(self.frame.paused),
            IsBlocking => flag(m.extra.blocking),
            IsPlayerFirstPerson => expr::FALSE,
            IsPlayerThirdPerson => expr::TRUE,
            // Booleans the game has no state for are false, numbers 0.
            IsOnShoulder
            | IsSprinting
            | IsSneaking
            | IsJumping
            | IsSwimming
            | IsGliding
            | IsCrawling
            | IsHovered
            | IsInGui
            | IsInHand
            | IsInItemFrame
            | IsInGround
            | IsOnHead
            | IsFirstPersonHand
            | IsRightHanded
            | IsSwingingRightArm
            | IsSwingingLeftArm
            | IsHoldingItemRight
            | IsHoldingItemLeft
            | IsUsingItem
            | IsPlayerThirdPersonReversed => expr::FALSE,
            PlayerRotX | PlayerRotY | MoveForward | MoveStrafing | HeightAboveGround
            | FluidDepth | FluidDepthDown | FluidDepthUp | AngerTimeStart => 0.0,
        }
    }

    fn part(&mut self, part: usize, prop: PartProp) -> f32 {
        let name = &self.built.names[part];
        match prop {
            PartProp::Visible => expr::from_bool(!self.poses.is_hidden_name(name)),
            PartProp::VisibleBoxes => expr::from_bool(!self.poses.skips_name(name)),
            _ => {
                let pose = self.poses.get(name);
                match prop {
                    PartProp::Tx => pose[0],
                    PartProp::Ty => pose[1],
                    PartProp::Tz => pose[2],
                    PartProp::Rx => pose[3],
                    PartProp::Ry => pose[4],
                    PartProp::Rz => pose[5],
                    PartProp::Sx => pose[6],
                    PartProp::Sy => pose[7],
                    _ => pose[8],
                }
            }
        }
    }

    fn var(&mut self, slot: u32) -> f32 {
        self.vars[slot as usize]
    }

    fn global(&mut self, slot: u32) -> f32 {
        self.globals[slot as usize]
    }
}

/// What expressions' names mean in a model being put together.
struct Table {
    /// OptiFine name or id -> name in the combined tree's poses.
    address: HashMap<String, String>,
    names: Vec<String>,
    vars: Vec<(String, bool)>,
    globals: Vec<(String, bool)>,
}

impl Table {
    fn intern(list: &mut Vec<(String, bool)>, key: &str) -> u32 {
        if let Some(i) = list.iter().position(|(k, _)| k == key) {
            return i as u32;
        }
        list.push((key.to_owned(), key.contains("varb")));
        list.len() as u32 - 1
    }
}

impl Resolver for Table {
    fn part(&mut self, id: &str) -> Option<usize> {
        let id = id.rsplit(':').next().unwrap_or(id);
        let pose = self
            .address
            .get(id)
            .or_else(|| self.address.get(&format!("EMF_{id}")))?
            .clone();
        Some(match self.names.iter().position(|n| *n == pose) {
            Some(i) => i,
            None => {
                self.names.push(pose);
                self.names.len() - 1
            }
        })
    }

    fn entity_var(&mut self, key: &str) -> u32 {
        Self::intern(&mut self.vars, key)
    }

    fn global_var(&mut self, key: &str) -> u32 {
        Self::intern(&mut self.globals, key)
    }
}

/// The game's model with the pack's parts hung on it.
fn assemble(
    vanilla: &Arc<Part>,
    model: jem::Jem,
    known: Option<&HashMap<String, String>>,
) -> Result<Built, String> {
    // Parts of the game's model the pack can name: the values of OptiFine's table (all, for
    // a model the table does not know).
    let mut specified: HashSet<String> = HashSet::new();
    let mut reverse: HashMap<String, String> = HashMap::new();
    match known {
        Some(map) => {
            for (optifine, vanilla) in map {
                specified.insert(vanilla.clone());
                let entry = reverse
                    .entry(vanilla.clone())
                    .or_insert_with(|| optifine.clone());
                // Prefer the part's own name when it is one of them.
                if optifine == vanilla {
                    *entry = optifine.clone();
                }
            }
        }
        None => fn_all_names(vanilla, &mut specified),
    }

    // Unique names in the combined tree for every part of the pack.
    let mut used: HashSet<String> = HashSet::new();
    let mut by_parent: HashMap<String, Vec<Part>> = HashMap::new();
    let mut replaced: HashSet<String> = HashSet::new();
    let mut address: HashMap<String, String> = HashMap::new();
    let mut custom_ids: Vec<(String, String)> = Vec::new();

    fn convert(
        part: &jem::CemPart,
        used: &mut HashSet<String>,
        ids: &mut Vec<(String, String)>,
    ) -> Part {
        let mut name = format!("EMF_{}", part.id);
        let mut n = 1;
        while !used.insert(name.clone()) {
            name = format!("EMF_{}{n}", part.id);
            n += 1;
        }
        ids.push((part.id.clone(), name.clone()));
        Part {
            name,
            pose: part.pose,
            polygons: part.polygons.clone(),
            children: part
                .children
                .iter()
                .map(|c| convert(c, used, ids))
                .collect(),
        }
    }

    let mut groups: Vec<(String, Vec<&Vec<(String, String)>>)> = Vec::new();
    let mut dropped = Vec::new();
    for top in &model.parts {
        let Some(target) = &top.part else { continue };
        if !has_part(vanilla, target) {
            dropped.push(format!("{} (on {target})", top.id));
            continue;
        }
        let converted = convert(top, &mut used, &mut custom_ids);
        by_parent.entry(target.clone()).or_default().push(converted);
        if !top.attach {
            replaced.insert(target.clone());
        }
        if !top.animations.is_empty() {
            match groups.iter_mut().find(|(k, _)| k == target) {
                Some((_, list)) => list.extend(top.animations.iter()),
                None => groups.push((target.clone(), top.animations.iter().collect())),
            }
        }
    }

    // `EMFModelPartRoot.addAndSetVariantOfJem`: the part keeps its pose, loses its boxes
    // when replaced, and, when replaced, the children the pack cannot name.
    fn hang(
        v: &Part,
        by_parent: &mut HashMap<String, Vec<Part>>,
        replaced: &HashSet<String>,
        specified: &HashSet<String>,
    ) -> Part {
        let replaced_here = replaced.contains(&v.name);
        let mut children: Vec<Part> = v
            .children
            .iter()
            .filter(|c| !replaced_here || specified.contains(&c.name))
            .map(|c| hang(c, by_parent, replaced, specified))
            .collect();
        if let Some(custom) = by_parent.remove(&v.name) {
            children.extend(custom);
        }
        Part {
            name: v.name.clone(),
            pose: v.pose,
            polygons: if replaced_here {
                Vec::new()
            } else {
                v.polygons.clone()
            },
            children,
        }
    }
    let tree = hang(vanilla, &mut by_parent, &replaced, &specified);

    // Names expressions use: the model's parts by OptiFine's name, then the pack's ids.
    let mut walk = vec![&tree];
    let mut root_name = None;
    while let Some(p) = walk.pop() {
        if root_name.is_none() {
            root_name = Some(p.name.clone());
        }
        if specified.contains(&p.name) {
            let key = reverse
                .get(&p.name)
                .cloned()
                .unwrap_or_else(|| p.name.clone());
            address.entry(key).or_insert_with(|| p.name.clone());
        }
        walk.extend(
            p.children
                .iter()
                .rev()
                .filter(|c| !c.name.starts_with("EMF_")),
        );
    }
    if let Some(root) = root_name {
        address.entry("root".to_owned()).or_insert(root);
    }
    for (id, pose) in &custom_ids {
        address.entry(id.clone()).or_insert_with(|| pose.clone());
    }
    for (id, pose) in &custom_ids {
        address.insert(format!("EMF_{id}"), pose.clone());
    }

    // Parse the lines in EMF's order: grouped by the part they belong to.
    let mut table = Table {
        address,
        names: Vec::new(),
        vars: Vec::new(),
        globals: Vec::new(),
    };
    let mut lines = Vec::new();
    for (_, list) in &groups {
        for map in list {
            for (key, text) in map.iter() {
                let expr = Expr::parse(text, &mut table)
                    .map_err(|e| format!("animation {key} = {text}: {e}"))?;
                let target = match key.rsplit_once('.') {
                    Some(("var", _)) => Target::Var(table.entity_var(key), false),
                    Some(("varb", _)) => Target::Var(table.entity_var(key), true),
                    Some(("global_var", _)) => Target::Global(table.global_var(key), false),
                    Some(("global_varb", _)) => Target::Global(table.global_var(key), true),
                    Some(("render", _)) => Target::Ignore,
                    Some((owner, field)) => match (PartProp::from_name(field), table.part(owner)) {
                        (Some(prop), Some(part)) => Target::Part(part, prop),
                        _ => Target::Ignore,
                    },
                    None => return Err(format!("animation key {key} names no part")),
                };
                lines.push(Line {
                    key: key.clone(),
                    target,
                    expr,
                });
            }
        }
    }
    Ok(Built {
        tree: Arc::new(tree),
        texture: model.texture.as_deref().and_then(sprite_of),
        names: table.names,
        lines,
        vars: table.vars,
        globals: table.globals,
        dropped,
        warned: std::sync::atomic::AtomicBool::new(false),
    })
}

/// The atlas sprite (`entity/pig/pig`) of a texture path as OptiFine writes them
/// (`textures/entity/pig/pig.png`, `minecraft:textures/entity/pig/pig`); only textures
/// under `textures/` are in the atlas.
fn sprite_of(path: &str) -> Option<String> {
    let path = path.strip_prefix("minecraft:").unwrap_or(path);
    let path = path.strip_suffix(".png").unwrap_or(path);
    let path = path.strip_prefix("assets/minecraft/").unwrap_or(path);
    path.strip_prefix("textures/").map(str::to_owned)
}

fn fn_all_names(part: &Part, out: &mut HashSet<String>) {
    out.insert(part.name.clone());
    for c in &part.children {
        fn_all_names(c, out);
    }
}

fn has_part(part: &Part, name: &str) -> bool {
    part.name == name || part.children.iter().any(|c| has_part(c, name))
}

/// The model file a mob draws with: `pig`, `pig_baby`, `sheep_wool` (the model key
/// `pig#main`, `sheep#wool`), with the climate prefix of the pig and chicken variants.
pub fn model_name(m: &Mob, key: &str) -> String {
    use crate::mobs::Species;
    let (entity, layer) = key.split_once('#').unwrap_or((key, "main"));
    let mut entity = entity.to_owned();
    if matches!(m.species(), Species::Pig | Species::Chicken)
        && !entity.starts_with("cold_")
        && !entity.starts_with("warm_")
    {
        match m.extra.variant {
            1 => entity.insert_str(0, "warm_"),
            2 => entity.insert_str(0, "cold_"),
            _ => {}
        }
    }
    if layer == "main" {
        entity
    } else {
        format!("{entity}_{layer}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(name: &str, pose: [f32; 9], polygons: usize, children: Vec<Part>) -> Part {
        let polygon = crate::mobs::Polygon {
            normal: [0.0, 1.0, 0.0],
            vertices: [[0.0; 5]; 4],
        };
        Part {
            name: name.to_owned(),
            pose,
            polygons: vec![polygon; polygons],
            children,
        }
    }

    const REST: [f32; 9] = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0];

    #[test]
    fn a_pack_part_hangs_from_the_games_part_and_replaces_its_boxes() {
        // root > head (boxes) > hat (boxes), body (boxes); the pack replaces the head.
        let vanilla = Arc::new(part(
            "root",
            REST,
            0,
            vec![
                part(
                    "head",
                    [0.0, 10.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
                    1,
                    vec![
                        part("hat", REST, 1, Vec::new()),
                        part("nub", REST, 1, Vec::new()),
                    ],
                ),
                part("body", REST, 1, Vec::new()),
            ],
        ));
        let names: HashMap<String, String> =
            [("head", "head"), ("headwear", "hat"), ("body", "body")]
                .into_iter()
                .map(|(a, b)| (a.to_owned(), b.to_owned()))
                .collect();
        let raw = jem::parse_jem(
            r#"{"textureSize":[16,16],"models":[
                {"part":"head","id":"head","invertAxis":"xy","translate":[0,-24,0],
                 "boxes":[{"coordinates":[-1,0,-1,2,2,2],"textureOffset":[0,0]}]},
                {"part":"body","id":"body","attach":true,"boxes":[{"coordinates":[0,0,0,1,1,1],"textureOffset":[0,0]}],
                 "animations":[{"head.ry":"var.turn * 2","var.turn":"1.5","body.visible":"false"}]}]}"#,
        )
        .unwrap();
        struct NoFiles;
        impl jem::Files for NoFiles {
            fn jpm(&self, _: &str) -> Option<String> {
                None
            }
        }
        let model = jem::build(raw, &names, &NoFiles).unwrap();
        let built = assemble(&vanilla, model, Some(&names)).unwrap();
        let head = &built.tree.children[0];
        // The game's head keeps its pose but loses its boxes; the `nub` the pack cannot name
        // goes, the `hat` it can name stays, and the pack's part hangs last.
        assert_eq!(head.pose[1], 10.0);
        assert!(head.polygons.is_empty());
        let kids: Vec<&str> = head.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(kids, ["hat", "EMF_head"]);
        assert_eq!(head.children[1].pose[1], 24.0);
        // `attach` keeps the body's boxes and adds the pack's.
        let body = &built.tree.children[1];
        assert_eq!(body.polygons.len(), 1);
        assert_eq!(body.children[0].name, "EMF_body");
        assert_eq!(body.children[0].polygons.len(), 6);
        // `head` in an expression means the game's head part; the lines keep their order.
        assert_eq!(built.names, ["head", "body"]);
        let keys: Vec<&str> = built.lines.iter().map(|l| l.key.as_str()).collect();
        assert_eq!(keys, ["head.ry", "var.turn", "body.visible"]);
        assert!(built.dropped.is_empty());
    }

    #[test]
    fn a_part_the_game_lacks_is_dropped_and_a_bad_line_fails_the_model() {
        let vanilla = Arc::new(part(
            "root",
            REST,
            0,
            vec![part("head", REST, 1, Vec::new())],
        ));
        struct NoFiles;
        impl jem::Files for NoFiles {
            fn jpm(&self, _: &str) -> Option<String> {
                None
            }
        }
        let names = HashMap::new();
        let model =
            |text: &str| jem::build(jem::parse_jem(text).unwrap(), &names, &NoFiles).unwrap();
        let built = assemble(
            &vanilla,
            model(r#"{"textureSize":[16,16],"models":[{"part":"tail","id":"t"}]}"#),
            None,
        )
        .unwrap();
        assert_eq!(built.dropped.len(), 1);
        assert!(built.tree.children[0].children.is_empty());
        let bad = model(
            r#"{"textureSize":[16,16],"models":[{"part":"head","id":"h","animations":[{"head.rx":"sin("}]}]}"#,
        );
        assert!(assemble(&vanilla, bad, None).is_err());
    }

    /// Runs the pack's animations for a few mobs at rest and prints where their parts end up.
    #[test]
    #[ignore = "needs the prepared resources and a pack with custom entity models"]
    fn poses_of_mobs_at_rest() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let resources = root.join(".local/horizon-client/26.3");
        let pack = root.join("resourcepacks/Fresh Animations 1.11.1.zip");
        if !resources.join("entity-models.json").exists() || !pack.exists() {
            return;
        }
        let out = std::env::temp_dir().join(format!("horizon-cem-poses-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        zip::ZipArchive::new(std::fs::File::open(&pack).unwrap())
            .unwrap()
            .extract(&out)
            .unwrap();
        let cem = Cem::load(&out);
        cem.begin_frame(1000, false);
        cem.begin_frame(1001, false);
        let types = crate::mobs::MobTypes::load(&resources).unwrap();
        let animations = crate::mobs::MobData::load_animations(&resources);
        let wanted = std::env::var("HORIZON_CEM_MOBS")
            .unwrap_or_else(|_| "pig,cow,zombie,iron_golem,sheep,chicken,villager".into());
        for spec in wanted.split(',') {
            let (id, layer) = spec
                .split_once('#')
                .map_or((spec, None), |(a, b)| (a, Some(b)));
            let kind = Arc::new(types.types[&format!("minecraft:{id}")].clone());
            let mut mob = Mob::new(7, kind, horizon_core::geometry::Vec3::ZERO, 0.0);
            mob.age = 100;
            let (key, _) = crate::mobs::render::body(&mob);
            let key = match layer {
                Some(l) => format!("{}#{l}", key.split('#').next().unwrap()),
                None => key,
            };
            let vanilla = mob.kind.layer(&key).cloned().unwrap();
            let name = model_name(&mob, &key);
            let built = cem.model(&name, &key, &vanilla).expect("model builds");
            let state = crate::mobs::anim::state(&mob, 0.5);
            let mut poses = Poses::new(&built.tree);
            crate::mobs::anim::setup(&mob, &state, &mut poses, &animations);
            let before = poses.clone();
            let skipped = cem.run(
                &built,
                &mob,
                &state,
                glam::DVec3::new(0.0, 2.0, 5.0),
                &mut poses,
            );
            eprintln!(
                "== {id} ({name}): {} lines, skipped {skipped:?}",
                built.lines.len()
            );
            {
                let mut v = Vec::new();
                crate::mobs::render::draw(
                    &built.tree,
                    glam::Mat4::IDENTITY,
                    &poses,
                    [0.0, 0.0, 1.0, 1.0],
                    0,
                    0,
                    0,
                    &mut v,
                );
                let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                for p in &v {
                    for i in 0..3 {
                        lo[i] = lo[i].min(p.pos[i]);
                        hi[i] = hi[i].max(p.pos[i]);
                    }
                }
                eprintln!(
                    "   drawn {} vertices, x {:.2}..{:.2} y {:.2}..{:.2} z {:.2}..{:.2}",
                    v.len(),
                    lo[0],
                    hi[0],
                    lo[1],
                    hi[1],
                    lo[2],
                    hi[2]
                );
            }
            let mut names: Vec<&String> = built.names.iter().collect();
            names.sort();
            let all = std::env::var("HORIZON_CEM_ALL").is_ok();
            let listed: Vec<String> = if all {
                names.iter().map(|s| s.to_string()).collect()
            } else {
                [
                    "body",
                    "head",
                    "EMF_head2",
                    "EMF_rotation",
                    "left_leg",
                    "EMF_waist",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect()
            };
            for part in &listed {
                if poses.has(part) {
                    let (b, a) = (before.clone().get(part).to_vec(), poses.get(part).to_vec());
                    eprintln!(
                        "   {part}: vanilla {b:.2?}
          now     {a:.2?}"
                    );
                }
            }
        }
        let _ = std::fs::remove_dir_all(&out);
    }

    /// Draws each model layer of a mob (as `render` does) and prints where it ends up.
    #[test]
    #[ignore = "needs the prepared resources and a pack with custom entity models"]
    fn layers_of_a_mob() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let resources = root.join(".local/horizon-client/26.3");
        let pack = root.join("resourcepacks/Fresh Animations 1.11.1.zip");
        if !resources.join("entity-models.json").exists() || !pack.exists() {
            return;
        }
        let out = std::env::temp_dir().join(format!("horizon-cem-layers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        zip::ZipArchive::new(std::fs::File::open(&pack).unwrap())
            .unwrap()
            .extract(&out)
            .unwrap();
        let cem = Cem::load(&out);
        cem.begin_frame(1000, false);
        let types = crate::mobs::MobTypes::load(&resources).unwrap();
        let animations = crate::mobs::MobData::load_animations(&resources);
        let wanted = std::env::var("HORIZON_CEM_MOBS").unwrap_or_else(|_| "sheep,mooshroom".into());
        for id in wanted.split(',') {
            let kind = Arc::new(types.types[&format!("minecraft:{id}")].clone());
            let mut mob = Mob::new(7, kind, horizon_core::geometry::Vec3::ZERO, 0.0);
            mob.age = 100;
            let state = crate::mobs::anim::state(&mob, 0.5);
            let (body_key, _) = crate::mobs::render::body(&mob);
            let mut keys = vec![body_key];
            keys.extend(
                crate::mobs::render::layers(&mob)
                    .into_iter()
                    .map(|l| l.model),
            );
            for key in keys {
                let Some(vanilla) = mob.kind.layer(&key).cloned() else {
                    eprintln!("{id} {key}: no such model");
                    continue;
                };
                let name = model_name(&mob, &key);
                let built = cem.model(&name, &key, &vanilla);
                let tree = built.as_ref().map_or(vanilla.clone(), |b| b.tree.clone());
                let mut poses = Poses::new(&tree);
                crate::mobs::anim::setup(&mob, &state, &mut poses, &animations);
                if let Some(b) = &built {
                    cem.run(b, &mob, &state, glam::DVec3::ZERO, &mut poses);
                }
                let mut v = Vec::new();
                crate::mobs::render::draw(
                    &tree,
                    glam::Mat4::IDENTITY,
                    &poses,
                    [0.0, 0.0, 1.0, 1.0],
                    0,
                    0,
                    0,
                    &mut v,
                );
                let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                for p in &v {
                    for i in 0..3 {
                        lo[i] = lo[i].min(p.pos[i]);
                        hi[i] = hi[i].max(p.pos[i]);
                    }
                }
                if std::env::var("HORIZON_CEM_PARTS").is_ok() {
                    fn walk(part: &Part, base: glam::Mat4, poses: &Poses, out: &mut Vec<String>) {
                        let p = poses.pose(part);
                        let m =
                            base * glam::Mat4::from_translation(
                                glam::Vec3::new(p[0], p[1], p[2]) / 16.0,
                            ) * glam::Mat4::from_quat(glam::Quat::from_euler(
                                glam::EulerRot::ZYX,
                                p[5],
                                p[4],
                                p[3],
                            )) * glam::Mat4::from_scale(glam::Vec3::new(p[6], p[7], p[8]));
                        if !part.polygons.is_empty() {
                            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                            for poly in &part.polygons {
                                for v in poly.vertices {
                                    let q = m
                                        .transform_point3(glam::Vec3::new(v[0], v[1], v[2]) / 16.0);
                                    for i in 0..3 {
                                        lo[i] = lo[i].min(q[i]);
                                        hi[i] = hi[i].max(q[i]);
                                    }
                                }
                            }
                            out.push(format!(
                                "      {:22} {:3} polys  x {:.2}..{:.2} y {:.2}..{:.2} z {:.2}..{:.2}",
                                part.name, part.polygons.len(), lo[0], hi[0], lo[1], hi[1], lo[2], hi[2]
                            ));
                        }
                        for c in &part.children {
                            walk(c, m, poses, out);
                        }
                    }
                    let mut lines = Vec::new();
                    walk(&tree, glam::Mat4::IDENTITY, &poses, &mut lines);
                    eprintln!(
                        "{}",
                        lines.join(
                            "
"
                        )
                    );
                }
                eprintln!(
                    "{id} {key:24} {} {name:24} {:4} verts x {:.2}..{:.2} y {:.2}..{:.2} z {:.2}..{:.2}",
                    if built.is_some() {
                        "PACK   "
                    } else {
                        "vanilla"
                    },
                    v.len(),
                    lo[0],
                    hi[0],
                    lo[1],
                    hi[1],
                    lo[2],
                    hi[2]
                );
            }
        }
        let _ = std::fs::remove_dir_all(&out);
    }

    /// Where the drawn mobs' boxes end up, in blocks from their feet: a mob with and without
    /// the pack's model.
    #[test]
    #[ignore = "needs the prepared resources and a pack with custom entity models"]
    fn drawn_sizes_with_and_without_the_pack() {
        use crate::mobs::MobData;
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let resources = root.join(".local/horizon-client/26.3");
        let pack = root.join("resourcepacks/Fresh Animations 1.11.1.zip");
        if !resources.join("entity-models.json").exists() || !pack.exists() {
            return;
        }
        let out = std::env::temp_dir().join(format!("horizon-cem-size-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        zip::ZipArchive::new(std::fs::File::open(&pack).unwrap())
            .unwrap()
            .extract(&out)
            .unwrap();
        let types = crate::mobs::MobTypes::load(&resources).unwrap();
        let data = |with: bool| MobData {
            states: Vec::new(),
            placements: Default::default(),
            heightmaps: Default::default(),
            animations: MobData::load_animations(&resources),
            biome_tags: Default::default(),
            entity_tags: Default::default(),
            villager_hats: Default::default(),
            cem: std::sync::RwLock::new(if with {
                Cem::load(&out)
            } else {
                Cem::default()
            }),
            pois: Default::default(),
        };
        let (with, without) = (data(true), data(false));
        with.cem.read().unwrap().begin_frame(1000, false);
        let atlas = |_: &str| Some([0.0, 0.0, 1.0, 1.0]);
        let mut ids: Vec<&String> = types.types.keys().collect();
        ids.sort();
        let mut off = Vec::new();
        let mut next_id = 100;
        for id in ids {
            let kind = Arc::new(types.types[id].clone());
            for baby in [false, true] {
                next_id += 1;
                with.cem
                    .read()
                    .unwrap()
                    .begin_frame(1000 + u64::from(next_id), false);
                let mut mob = Mob::new(
                    next_id,
                    kind.clone(),
                    horizon_core::geometry::Vec3::ZERO,
                    0.0,
                );
                mob.age = 100;
                mob.extra.age = if baby { -24000 } else { 0 };
                if baby && !mob.is_baby() {
                    continue;
                }
                let mut boxes = Vec::new();
                for (which, d) in [&without, &with].into_iter().enumerate() {
                    let mut v = Vec::new();
                    crate::mobs::render::render(
                        &mob,
                        0.5,
                        glam::DVec3::ZERO,
                        &atlas,
                        0xf0f0,
                        d,
                        &mut v,
                    );
                    if which == 1
                        && std::env::var("HORIZON_CEM_ONLY")
                            .is_ok_and(|o| id.ends_with(&o) && !baby)
                    {
                        for (i, quad) in v.chunks(4).enumerate() {
                            let far = quad.iter().any(|p| p.pos.iter().any(|c| c.abs() > 2.5));
                            if far {
                                eprintln!(
                                    "   quad {i}: {:?}",
                                    quad.iter().map(|p| p.pos).collect::<Vec<_>>()
                                );
                            }
                        }
                    }
                    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                    for p in &v {
                        for i in 0..3 {
                            lo[i] = lo[i].min(p.pos[i]);
                            hi[i] = hi[i].max(p.pos[i]);
                        }
                    }
                    boxes.push((lo, hi, v.len()));
                }
                if boxes[0].2 == boxes[1].2 && boxes[0].0 == boxes[1].0 && boxes[0].1 == boxes[1].1
                {
                    continue; // no custom model for it
                }
                let worst = (0..3)
                    .map(|i| {
                        (boxes[0].0[i] - boxes[1].0[i])
                            .abs()
                            .max((boxes[0].1[i] - boxes[1].1[i]).abs())
                    })
                    .fold(0.0f32, f32::max);
                let tag = if worst > 0.3 { "  <== DIFFERENT" } else { "" };
                if worst > 0.3 {
                    off.push(format!("{id} baby={baby}"));
                }
                eprintln!(
                    "{id:34} baby={baby:5} vanilla x {:5.2}..{:5.2} y {:5.2}..{:5.2} z {:5.2}..{:5.2} | pack x {:5.2}..{:5.2} y {:5.2}..{:5.2} z {:5.2}..{:5.2}{tag}",
                    boxes[0].0[0],
                    boxes[0].1[0],
                    boxes[0].0[1],
                    boxes[0].1[1],
                    boxes[0].0[2],
                    boxes[0].1[2],
                    boxes[1].0[0],
                    boxes[1].1[0],
                    boxes[1].0[1],
                    boxes[1].1[1],
                    boxes[1].0[2],
                    boxes[1].1[2],
                );
            }
        }
        eprintln!("DIFFERENT: {off:?}");
        let _ = std::fs::remove_dir_all(&out);
    }

    /// Builds every model of the pack in `resourcepacks/` (or `HORIZON_CEM_PACK`) over the
    /// game's own models: the vanilla part names of the prepared resources must line up
    /// with the pack's, and every animation must parse.
    #[test]
    #[ignore = "needs the prepared resources and a pack with custom entity models"]
    fn every_model_of_the_pack_builds() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let resources = root.join(".local/horizon-client/26.3");
        let pack = std::env::var("HORIZON_CEM_PACK")
            .map(PathBuf::from)
            .unwrap_or_else(|_| root.join("resourcepacks/Fresh Animations 1.11.1.zip"));
        if !resources.join("entity-models.json").exists() || !pack.exists() {
            eprintln!("skipped: no resources or pack");
            return;
        }
        let out = std::env::temp_dir().join(format!("horizon-cem-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        zip::ZipArchive::new(std::fs::File::open(&pack).unwrap())
            .unwrap()
            .extract(&out)
            .unwrap();
        let cem = Cem::load(&out);
        assert!(!cem.is_empty());
        let types = crate::mobs::MobTypes::load(&resources).unwrap();
        let (mut built, mut failed) = (0, Vec::new());
        for kind in types.types.values() {
            for (key, vanilla) in &kind.layers {
                let (entity, layer) = key.split_once('#').unwrap_or((key, "main"));
                let name = if layer == "main" {
                    entity.to_owned()
                } else {
                    format!("{entity}_{layer}")
                };
                if !cem.has_model(&name) {
                    continue;
                }
                match cem.build(&name, vanilla) {
                    Ok(b) => {
                        built += 1;
                        assert!(b.tree.name == vanilla.name);
                        let armor = ["_boots", "_chestplate", "_helmet", "_leggings"]
                            .iter()
                            .any(|s| name.ends_with(s));
                        if !b.dropped.is_empty() && !armor {
                            failed.push(format!("{name}: no vanilla part for {:?}", b.dropped));
                        }
                    }
                    Err(e) => failed.push(format!("{name}: {e}")),
                }
            }
        }
        let _ = std::fs::remove_dir_all(&out);
        eprintln!("built {built} models");
        assert!(failed.is_empty(), "{}", failed.join("\n"));
        assert!(built > 100, "only {built} models built");
    }
}
