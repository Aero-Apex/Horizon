//! Horizon Extras' dynamic lights (docs/extras.md), the rules of LambDynamicLights
//! (LambdAurora, the Lambda License, see THIRD_PARTY_NOTICES.md): a torch, lantern or other light
//! source held or worn by a player or mob, dropped as an item, on a minecart or item frame, or an
//! entity on fire, plus primed TNT and swelling creepers, lights the blocks and entities around it.
//! Not in vanilla; off by default.
//!
//! Client-only and at render time: the light engine, chunk meshes and saves never see it. Each
//! frame the game picks the [`MAX`] sources nearest the camera; terrain.wgsl and entity.wgsl
//! raise a vertex's block light to the source's level minus its distance (one level per block
//! over 7.75 blocks, as LambDynamicLights falls off). Off, the list is empty and the shaders skip
//! the loop.
use crate::blocks::BlockFacts;
use glam::DVec3;
use horizon_world::registry::BlockRegistry;
use std::collections::{HashMap, HashSet};

/// Lights the shaders read: `Globals.dynamic_lights` in gfx.rs, terrain.wgsl and entity.wgsl.
pub const MAX: usize = 16;
/// What an entity on fire gives off, as a lit campfire or fire block does.
pub const BURNING: u8 = 15;
/// Sources farther than this from the camera light nothing it can see well.
const REACH: f64 = 96.0;

/// LambDynamicLights' reach (`DynamicLightingEngine.MAX_RADIUS`): a source lights nothing past
/// it, and its level falls by 15 over it (`SpatialLookupEntry.lightAtPos`).
pub const RADIUS: f64 = 7.75;

/// Primed TNT's fuse as it is set when lit (`PrimedTnt`'s default), which the fancy TNT light
/// measures its burn down from.
pub const TNT_FUSE: i32 = 80;

/// LambDynamicLights' own item table (`data/lambdynlights/dynamiclights/item`), the items that
/// do not just glow like their block: (item, level, goes dark under water). `None` takes the
/// level from the block, as its `block_self` does for torches and campfires.
const ITEM_TABLE: &[(&str, Option<u8>, bool)] = &[
    ("minecraft:blaze_powder", Some(10), false),
    ("minecraft:blaze_rod", Some(10), false),
    ("minecraft:campfire", None, true),
    ("minecraft:soul_campfire", None, true),
    ("minecraft:fire_charge", Some(10), true),
    ("minecraft:glow_berries", Some(14), false),
    ("minecraft:glow_ink_sac", Some(12), false),
    ("minecraft:glowstone_dust", Some(10), false),
    ("minecraft:lava_bucket", Some(15), true),
    ("minecraft:nether_star", Some(8), false),
    ("minecraft:prismarine_crystals", Some(10), false),
    ("minecraft:spectral_arrow", Some(8), false),
    ("minecraft:torch", None, true),
    ("minecraft:redstone_torch", None, true),
    ("minecraft:soul_torch", None, true),
    ("minecraft:copper_torch", None, true),
];

/// How brightly each item glows when held, worn or dropped.
#[derive(Debug, Default)]
pub struct Luminance {
    levels: HashMap<String, u8>,
    /// Items that go dark when their holder's eyes, or the item, are in water.
    water_sensitive: HashSet<&'static str>,
}

impl Luminance {
    /// An item without an entry in LambDynamicLights' table glows like its block's default
    /// state (`getLightEmission`): lanterns and glowstone 15; a furnace's item, unlit, not at
    /// all. The table's items take its levels (`ITEM_TABLE`).
    pub fn new(facts: &BlockFacts, registry: &BlockRegistry) -> Self {
        let mut levels = HashMap::new();
        for (id, state) in facts.states.iter().enumerate() {
            if state.emission == 0 || *state.item != *state.name {
                continue;
            }
            if registry.default_state(&state.name).map(|d| d.0 as usize) == Some(id) {
                levels.insert(state.item.to_string(), state.emission);
            }
        }
        Self::with_table(levels)
    }

    fn with_table(mut levels: HashMap<String, u8>) -> Self {
        let mut water_sensitive = HashSet::new();
        for &(item, level, wet) in ITEM_TABLE {
            match level {
                Some(level) => {
                    levels.insert(item.to_owned(), level);
                }
                None => {
                    // `block_self`: the block's own level.
                    levels.entry(item.to_owned()).or_insert(0);
                }
            }
            if wet {
                water_sensitive.insert(item);
            }
        }
        Self {
            levels,
            water_sensitive,
        }
    }

    /// The level of `item` held in the open air.
    pub fn of(&self, item: &str) -> u8 {
        self.in_fluid(item, false)
    }

    /// `ItemLightSourceManager.getLuminance(stack, submergedInWater)`: the items that need air
    /// to burn give nothing when submerged.
    pub fn in_fluid(&self, item: &str, submerged: bool) -> u8 {
        if submerged && self.water_sensitive.contains(item) {
            return 0;
        }
        self.levels.get(item).copied().unwrap_or(0)
    }

    #[cfg(test)]
    fn from_pairs(pairs: &[(&str, u8)]) -> Self {
        Self::with_table(pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect())
    }
}

/// LambDynamicLights' lighting mode for TNT and creepers (`ExplosiveLightingMode`): off, a fixed
/// level while it lasts, or one that follows the fuse or the swell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Explosive {
    Off,
    Simple,
    Fancy,
}

impl Explosive {
    pub const ALL: [Self; 3] = [Self::Off, Self::Simple, Self::Fancy];

    /// The name the settings file writes.
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Simple => "simple",
            Self::Fancy => "fancy",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Simple => "Simple",
            Self::Fancy => "Fancy",
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        Self::ALL
            .into_iter()
            .find(|v| v.name() == s)
            .unwrap_or(default)
    }
}

/// The light of primed TNT with `fuse` ticks left of `start` (`ExplosiveLuminance`): fancy is
/// off while the fuse is full and 10 when it is burnt down, as Mth's smoothstep eases it.
pub fn tnt_level(mode: Explosive, fuse: i32, start: i32) -> u8 {
    match mode {
        Explosive::Off => 0,
        Explosive::Simple => 10,
        Explosive::Fancy => {
            let t = fuse as f32 / start as f32;
            let eased = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
            ((-eased * 10.0) as i32 + 10).clamp(0, 15) as u8
        }
    }
}

/// A creeper's light (`CreeperLuminance`): none until it swells, then 10 (simple) or its swell
/// in tenths (fancy).
pub fn creeper_level(mode: Explosive, swelling: f32) -> u8 {
    if swelling <= 0.001 {
        return 0;
    }
    match mode {
        Explosive::Off => 0,
        Explosive::Simple => 10,
        Explosive::Fancy => (swelling * 10.0) as u8,
    }
}

/// What the game knows of an entity for LambDynamicLights' per-type luminance
/// (`data/lambdynlights/dynamiclights/entity`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Being {
    /// `isInWaterOrRain`: a blaze is dimmer wet.
    pub wet: bool,
    /// A creeper's swelling (`getSwelling(0)`), 0 to 1.
    pub swelling: f32,
    /// The light of the block an enderman carries.
    pub carried: u8,
    /// A cube mob squashed past 0.6 (a magma cube then glows brighter).
    pub squished: bool,
}

/// The level LambDynamicLights gives a living entity of `kind` for what it is, not what it
/// holds: allays 8, blazes 10 (4 wet), magma cubes 8 or 11, glow squid 12, a creeper as `creeper`
/// says, an enderman the light of its block. 0 for the rest.
pub fn mob_level(kind: &str, being: Being, creeper: Explosive) -> u8 {
    match kind {
        "minecraft:allay" => 8,
        "minecraft:blaze" => {
            if being.wet {
                4
            } else {
                10
            }
        }
        "minecraft:magma_cube" => {
            if being.squished {
                11
            } else {
                8
            }
        }
        // Not hurt: its dark ticks are 0, so the full 12.
        "minecraft:glow_squid" => 12,
        "minecraft:creeper" => creeper_level(creeper, being.swelling),
        "minecraft:enderman" => being.carried,
        _ => 0,
    }
}

/// The level of a projectile of `kind`: fireballs 14 and 12 and wither skulls 12 (the fireballs
/// not under water), an arrow of the item it was shot from (a spectral arrow 8), any of them 15
/// on fire.
pub fn projectile_level(kind: &str, in_water: bool, on_fire: bool, arrow_item: u8) -> u8 {
    let own = match kind {
        "fireball" if in_water => 0,
        "fireball" => 14,
        "small_fireball" if in_water => 0,
        "small_fireball" => 12,
        "wither_skull" => 12,
        "arrow" => arrow_item,
        _ => 0,
    };
    if on_fire { BURNING } else { own }
}

/// One light this frame: where it is (world space) and its level, 1 to 15.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Source {
    pub at: DVec3,
    pub level: u8,
}

/// The [`MAX`] sources nearest `camera` within reach, as the shaders read them: camera-relative
/// position and level. Overwrites `out`.
pub fn select(sources: &[Source], camera: DVec3, out: &mut Vec<[f32; 4]>) {
    out.clear();
    let key = |s: &Source| s.at.distance_squared(camera);
    let mut near: Vec<&Source> = sources
        .iter()
        .filter(|s| s.level > 0 && key(s) <= REACH * REACH)
        .collect();
    near.sort_by(|a, b| key(a).total_cmp(&key(b)));
    out.extend(near.into_iter().take(MAX).map(|s| {
        let p = (s.at - camera).as_vec3();
        [p.x, p.y, p.z, f32::from(s.level.min(15))]
    }));
}

/// The block light a vertex at `pos` (camera-relative) gets from `lights`, 0 to 15: the WGSL
/// `dynamic_block_light` in Rust, for tests.
pub fn level_at(pos: [f32; 3], lights: &[[f32; 4]]) -> f32 {
    lights.iter().fold(0.0f32, |level, l| {
        let d = f64::from(
            ((pos[0] - l[0]).powi(2) + (pos[1] - l[1]).powi(2) + (pos[2] - l[2]).powi(2)).sqrt(),
        );
        if d <= RADIUS {
            level.max(l[3] - (d / RADIUS * 15.0) as f32)
        } else {
            level
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_sources_win() {
        let camera = DVec3::new(100.0, 64.0, 100.0);
        let mut sources: Vec<Source> = (0..40)
            .map(|i| Source {
                at: camera + DVec3::new(f64::from(i), 0.0, 0.0),
                level: 14,
            })
            .collect();
        sources.push(Source {
            at: camera + DVec3::new(0.5, 0.0, 0.0),
            level: 0,
        });
        sources.push(Source {
            at: camera + DVec3::new(0.0, 0.0, 200.0),
            level: 15,
        });
        let mut out = Vec::new();
        select(&sources, camera, &mut out);
        assert_eq!(out.len(), MAX);
        assert_eq!(out[0], [0.0, 0.0, 0.0, 14.0]);
        assert_eq!(out[MAX - 1][0], (MAX - 1) as f32);
        select(&[], camera, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn light_falls_like_lambdynamiclights() {
        // luminance - distance / 7.75 * 15, and nothing past 7.75 blocks.
        let torch = [[0.0, 0.0, 0.0, 14.0]];
        assert_eq!(level_at([0.0, 0.0, 0.0], &torch), 14.0);
        let at_five = level_at([3.0, 4.0, 0.0], &torch);
        assert!(
            (at_five - (14.0 - 5.0 / 7.75 * 15.0)).abs() < 1e-4,
            "{at_five}"
        );
        assert_eq!(level_at([7.76, 0.0, 0.0], &torch), 0.0);
        // A 14 is out of light a little before the edge, a 15 just reaches it.
        assert_eq!(level_at([7.7, 0.0, 0.0], &torch), 0.0);
        assert!(level_at([7.0, 0.0, 0.0], &torch) > 0.0);
        assert!(level_at([7.7, 0.0, 0.0], &[[0.0, 0.0, 0.0, 15.0]]) > 0.0);
        let two = [[0.0, 0.0, 0.0, 14.0], [10.0, 0.0, 0.0, 15.0]];
        let a = level_at([8.0, 0.0, 0.0], &two);
        assert!((a - (15.0 - 2.0 / 7.75 * 15.0)).abs() < 1e-4, "{a}");
        assert_eq!(level_at([0.0; 3], &[]), 0.0);
    }

    #[test]
    fn item_luminance() {
        let l = Luminance::from_pairs(&[("minecraft:torch", 14), ("minecraft:lantern", 15)]);
        assert_eq!(l.of("minecraft:torch"), 14);
        assert_eq!(l.of("minecraft:stone"), 0);
        // The table's own items, and the ones that need air.
        assert_eq!(l.of("minecraft:glow_berries"), 14);
        assert_eq!(l.of("minecraft:glow_ink_sac"), 12);
        assert_eq!(l.of("minecraft:nether_star"), 8);
        assert_eq!(l.of("minecraft:blaze_rod"), 10);
        assert_eq!(l.in_fluid("minecraft:torch", true), 0);
        assert_eq!(l.in_fluid("minecraft:lava_bucket", true), 0);
        assert_eq!(l.in_fluid("minecraft:lava_bucket", false), 15);
        assert_eq!(l.in_fluid("minecraft:lantern", true), 15);
        assert_eq!(l.in_fluid("minecraft:glow_berries", true), 14);
    }

    #[test]
    fn entities_glow_by_type() {
        let dry = Being::default();
        let fancy = Explosive::Fancy;
        assert_eq!(mob_level("minecraft:allay", dry, fancy), 8);
        assert_eq!(mob_level("minecraft:blaze", dry, fancy), 10);
        assert_eq!(
            mob_level("minecraft:blaze", Being { wet: true, ..dry }, fancy),
            4
        );
        assert_eq!(mob_level("minecraft:magma_cube", dry, fancy), 8);
        assert_eq!(
            mob_level(
                "minecraft:magma_cube",
                Being {
                    squished: true,
                    ..dry
                },
                fancy
            ),
            11
        );
        assert_eq!(mob_level("minecraft:glow_squid", dry, fancy), 12);
        assert_eq!(mob_level("minecraft:creeper", dry, fancy), 0);
        assert_eq!(
            mob_level(
                "minecraft:creeper",
                Being {
                    swelling: 0.5,
                    ..dry
                },
                fancy
            ),
            5
        );
        assert_eq!(
            mob_level("minecraft:enderman", Being { carried: 15, ..dry }, fancy),
            15
        );
        assert_eq!(mob_level("minecraft:cow", dry, fancy), 0);
    }

    #[test]
    fn creepers_and_tnt_follow_their_modes() {
        // The defaults: creepers simple (10 while swelling), TNT off.
        let half = Being {
            swelling: 0.5,
            ..Being::default()
        };
        assert_eq!(mob_level("minecraft:creeper", half, Explosive::Simple), 10);
        assert_eq!(mob_level("minecraft:creeper", half, Explosive::Off), 0);
        assert_eq!(creeper_level(Explosive::Fancy, 0.0), 0);
        assert_eq!(creeper_level(Explosive::Simple, 0.0005), 0);
        assert_eq!(tnt_level(Explosive::Off, 40, TNT_FUSE), 0);
        assert_eq!(tnt_level(Explosive::Simple, 40, TNT_FUSE), 10);
        // Fancy: nothing with the fuse full, 10 when it is out, eased in between.
        assert_eq!(tnt_level(Explosive::Fancy, TNT_FUSE, TNT_FUSE), 0);
        assert_eq!(tnt_level(Explosive::Fancy, 0, TNT_FUSE), 10);
        let middle = tnt_level(Explosive::Fancy, 40, TNT_FUSE);
        assert!((4..=6).contains(&middle), "{middle}");
        assert_eq!(Explosive::parse("fancy", Explosive::Off), Explosive::Fancy);
        assert_eq!(
            Explosive::parse("nope", Explosive::Simple),
            Explosive::Simple
        );
    }

    #[test]
    fn projectiles_glow_unless_wet() {
        assert_eq!(projectile_level("fireball", false, false, 0), 14);
        assert_eq!(projectile_level("fireball", true, false, 0), 0);
        assert_eq!(projectile_level("small_fireball", false, false, 0), 12);
        assert_eq!(projectile_level("wither_skull", true, false, 0), 12);
        assert_eq!(projectile_level("arrow", false, false, 8), 8);
        assert_eq!(projectile_level("arrow", false, true, 0), BURNING);
    }
}
