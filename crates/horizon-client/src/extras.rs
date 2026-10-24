//! Horizon's optional extras (zoom, FPS counter, keystroke overlay): their
//! settings, kept in `horizon-extras.txt` next to `options.txt` so that file
//! stays exactly vanilla's, and the logic behind them. Every extra is off by
//! default; see `docs/extras.md`.
use std::{collections::VecDeque, fs, path::Path};

pub const FILE: &str = "horizon-extras.txt";
/// The name the options screen uses while the zoom key waits for a key.
pub const ZOOM_KEY: &str = "horizon.zoom";
/// The same for the sort key.
pub const SORT_KEY: &str = "horizon.sort";
/// The name the options screen uses while the secondary zoom key waits for a key.
pub const ZOOM2_KEY: &str = "horizon.zoom2";
/// What the zoom settings cycle through.
pub const ZOOM_LEVELS: [i32; 11] = [2, 3, 4, 5, 6, 8, 10, 15, 20, 30, 50];
const ZOOM_TIMES: [f64; 7] = [0.0, 0.25, 0.5, 1.0, 1.5, 2.0, 3.0];
const ZOOM_SLOW_IN: [f64; 6] = [1.0, 2.0, 5.0, 10.0, 20.0, 30.0];
const ZOOM_SLOW_OUT: [f64; 5] = [0.5, 1.0, 2.0, 5.0, 10.0];
const ZOOM_STEPS: [i32; 5] = [3, 5, 10, 15, 20];
const ZOOM_PER_STEP: [i32; 5] = [110, 125, 150, 175, 200];
const ZOOM_SMOOTH: [i32; 6] = [0, 25, 50, 70, 85, 100];
const ZOOM_SENSITIVITY: [i32; 5] = [0, 25, 50, 75, 100];
const ZOOM_SLOW_AMOUNT: [i32; 6] = [2, 3, 4, 6, 8, 16];

/// The minimap's sizes in GUI pixels, and its zooms in blocks per pixel.
pub const MINIMAP_SIZES: [(u8, &str); 3] = [(64, "Small"), (96, "Medium"), (128, "Large")];
pub const MINIMAP_ZOOMS: [f32; 4] = [0.5, 1.0, 2.0, 4.0];

/// A screen corner for an overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
impl Corner {
    pub const ALL: [Corner; 4] = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Corner::TopLeft => "top_left",
            Corner::TopRight => "top_right",
            Corner::BottomLeft => "bottom_left",
            Corner::BottomRight => "bottom_right",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Corner::TopLeft => "Top left",
            Corner::TopRight => "Top right",
            Corner::BottomLeft => "Bottom left",
            Corner::BottomRight => "Bottom right",
        }
    }
    fn parse(s: &str, default: Corner) -> Corner {
        Self::ALL
            .into_iter()
            .find(|c| c.name() == s)
            .unwrap_or(default)
    }
    pub fn top(self) -> bool {
        matches!(self, Corner::TopLeft | Corner::TopRight)
    }
    pub fn left(self) -> bool {
        matches!(self, Corner::TopLeft | Corner::BottomLeft)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Extras {
    pub fullbright: bool,
    /// Held, dropped and burning light sources light their surroundings (`dynamic_light`).
    pub dynamic_lights: bool,
    pub zoom: bool,
    /// A vanilla key name (`key.keyboard.z`).
    pub zoom_key: String,
    /// Zoomify's settings: the zoom the key starts at, how it eases in and out, the scroll
    /// steps, hold or toggle, the mouse and bobbing scaling, the secondary zoom.
    pub zoomify: crate::zoomify::Settings,
    /// The secondary (slow, long) zoom's key, a vanilla key name.
    pub zoom_secondary_key: String,
    pub fps: bool,
    pub fps_corner: Corner,
    /// Also show the minimum and 1% low.
    pub fps_lows: bool,
    pub keystrokes: bool,
    pub keystrokes_corner: Corner,
    /// Clicks per second under the mouse buttons.
    pub keystrokes_cps: bool,
    pub minimap: bool,
    pub minimap_corner: Corner,
    /// Side in GUI pixels (`MINIMAP_SIZES`).
    pub minimap_size: u8,
    pub minimap_round: bool,
    /// Blocks per pixel (`MINIMAP_ZOOMS`).
    pub minimap_zoom: f32,
    /// Turn with the player (forward up) instead of north up.
    pub minimap_rotate: bool,
    /// Retry lost multiplayer connections (`reconnect`).
    pub auto_reconnect: bool,
    /// Mouse Tweaks-style drags in the inventory and storage containers.
    pub mouse_tweaks: bool,
    /// With them, the wheel over a slot moves its items one at a time.
    pub mouse_tweaks_wheel: bool,
    /// Mouse Tweaks' own settings: right-drag places one per slot, left-drag gathers the held
    /// item, Shift + left-drag moves slots with an empty hand, which slots the wheel pulls
    /// from first, which way it scrolls and how many items a tick moves.
    pub mouse_tweaks_rmb: bool,
    pub mouse_tweaks_lmb_item: bool,
    pub mouse_tweaks_lmb_empty: bool,
    pub mouse_tweaks_order: crate::mouse_tweaks::SearchOrder,
    pub mouse_tweaks_direction: crate::mouse_tweaks::Direction,
    pub mouse_tweaks_scaling: crate::mouse_tweaks::Scaling,
    /// AppleSkin: what food would do, drawn on the food and health bars, in
    /// tooltips and on F3 (`appleskin`).
    pub appleskin: bool,
    /// The flashing hunger and saturation preview for the food in hand.
    pub appleskin_restored: bool,
    /// The golden saturation outlines on the food icons.
    pub appleskin_saturation: bool,
    /// The exhaustion strip under the food icons.
    pub appleskin_exhaustion: bool,
    /// The flashing hearts food would regenerate.
    pub appleskin_health: bool,
    /// Hunger and saturation values in food tooltips.
    pub appleskin_tooltips: bool,
    /// Food values show without holding Shift (`showFoodValuesInTooltipAlways`).
    pub appleskin_tooltips_always: bool,
    /// The overlays work for food in the off hand too (`showFoodValuesHudOverlayWhenOffhand`).
    pub appleskin_offhand: bool,
    /// The food numbers on F3.
    pub appleskin_debug: bool,
    /// Jade: a box at the top of the screen for the block or mob under the
    /// crosshair (`jade`).
    pub jade: bool,
    /// Whether the held tool can harvest the block, and which tool does.
    pub jade_harvest: bool,
    /// Crop growth, redstone power and the like under the name.
    pub jade_details: bool,
    /// JEI: every item listed beside the inventory, with a search box, and
    /// recipe windows for what makes an item and what uses it (`jei`).
    pub jei: bool,
    /// Controlling: the Key Binds search also finds keys and categories, and
    /// a button filters to conflicts, unbound or changed keys.
    pub controlling: bool,
    /// Sort buttons on the inventory and chest-like containers.
    pub sort: bool,
    /// A vanilla key name: sorts the part under the cursor.
    pub sort_key: String,
    /// No AI: every AI feature hidden and off (`horizon_core::no_ai`; the launcher writes it too).
    pub no_ai: bool,
    /// Agents may control the game the player starts: it opens its control socket for
    /// horizon-ctl and MCP clients (off by default; an agent's own launch always opens it).
    pub agent_control: bool,
}
impl Default for Extras {
    fn default() -> Self {
        Self {
            fullbright: false,
            dynamic_lights: false,
            zoom: false,
            zoom_key: "key.keyboard.z".into(),
            zoomify: crate::zoomify::Settings::default(),
            zoom_secondary_key: "key.keyboard.f6".into(),
            fps: false,
            fps_corner: Corner::TopLeft,
            fps_lows: false,
            keystrokes: false,
            keystrokes_corner: Corner::TopLeft,
            keystrokes_cps: true,
            minimap: false,
            minimap_corner: Corner::TopRight,
            minimap_size: 96,
            minimap_round: false,
            minimap_zoom: 1.0,
            minimap_rotate: false,
            auto_reconnect: false,
            mouse_tweaks: false,
            mouse_tweaks_wheel: true,
            mouse_tweaks_rmb: true,
            mouse_tweaks_lmb_item: true,
            mouse_tweaks_lmb_empty: true,
            mouse_tweaks_order: Default::default(),
            mouse_tweaks_direction: Default::default(),
            mouse_tweaks_scaling: Default::default(),
            appleskin: false,
            appleskin_restored: true,
            appleskin_saturation: true,
            appleskin_exhaustion: true,
            appleskin_health: true,
            appleskin_tooltips: true,
            appleskin_tooltips_always: true,
            appleskin_offhand: true,
            appleskin_debug: true,
            jade: false,
            jade_harvest: true,
            jade_details: true,
            jei: false,
            controlling: false,
            sort: false,
            sort_key: "key.keyboard.unknown".into(),
            no_ai: false,
            agent_control: false,
        }
    }
}
fn bool_text(v: bool) -> &'static str {
    if v { "true" } else { "false" }
}
impl Extras {
    /// The zoom's lines of the file, from `zoomLevel` on.
    fn zoom_text(&self) -> String {
        let z = &self.zoomify;
        format!(
            "zoomLevel:{}\nzoomInTime:{}\nzoomOutTime:{}\nzoomInTransition:{}\nzoomOutTransition:{}\nzoomHand:{}\nzoomRetain:{}\nzoomScroll:{}\nzoomScrollSteps:{}\nzoomPerStep:{}\nzoomScrollSmooth:{}\nzoomKeyBehaviour:{}\nzoomSensitivity:{}\nzoomBobbing:{}\nzoomSecondaryKey:{}\nzoomSecondaryAmount:{}\nzoomSecondaryIn:{}\nzoomSecondaryOut:{}\n",
            z.initial_zoom,
            z.zoom_in_time,
            z.zoom_out_time,
            z.zoom_in_transition.name(),
            z.zoom_out_transition.name(),
            bool_text(z.affect_hand_fov),
            bool_text(z.retain_zoom_steps),
            bool_text(z.scroll_zoom),
            z.scroll_step_count,
            z.zoom_per_step,
            z.scroll_zoom_smoothness,
            z.key_behaviour.name(),
            z.relative_sensitivity,
            bool_text(z.relative_view_bobbing),
            self.zoom_secondary_key,
            z.secondary_zoom_amount,
            z.secondary_zoom_in_time,
            z.secondary_zoom_out_time,
        )
    }
    pub fn load(dir: &Path) -> Self {
        fs::read_to_string(dir.join(FILE))
            .map(|t| Self::parse(&t))
            .unwrap_or_default()
    }
    pub fn parse(text: &str) -> Self {
        let mut e = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim();
            let flag = |d: bool| match value {
                "true" => true,
                "false" => false,
                _ => d,
            };
            match key.trim() {
                "fullbright" => e.fullbright = flag(e.fullbright),
                "dynamicLights" => e.dynamic_lights = flag(e.dynamic_lights),
                "zoom" => e.zoom = flag(e.zoom),
                "zoomKey" if value.starts_with("key.") => e.zoom_key = value.to_owned(),
                "zoomSecondaryKey" if value.starts_with("key.") => {
                    e.zoom_secondary_key = value.to_owned()
                }
                "zoomLevel" => {
                    if let Ok(v) = value.parse::<i32>() {
                        e.zoomify.initial_zoom = v.clamp(2, 50);
                    }
                }
                // The settings before Zoomify's: smooth off meant no easing, slower turning off
                // no relative sensitivity.
                "zoomSmooth" => {
                    if value == "false" {
                        e.zoomify.zoom_in_transition = crate::zoomify::Transition::Instant;
                        e.zoomify.zoom_out_transition = crate::zoomify::Transition::Instant;
                    }
                }
                "zoomSensitivity" => match value {
                    "true" => e.zoomify.relative_sensitivity = 100,
                    "false" => e.zoomify.relative_sensitivity = 0,
                    v => {
                        if let Ok(v) = v.parse::<i32>() {
                            e.zoomify.relative_sensitivity = v.clamp(0, 100);
                        }
                    }
                },
                "zoomInTime" => {
                    if let Ok(v) = value.parse::<f64>() {
                        e.zoomify.zoom_in_time = v.clamp(0.0, 30.0);
                    }
                }
                "zoomOutTime" => {
                    if let Ok(v) = value.parse::<f64>() {
                        e.zoomify.zoom_out_time = v.clamp(0.0, 30.0);
                    }
                }
                "zoomInTransition" => {
                    e.zoomify.zoom_in_transition = crate::zoomify::Transition::parse(
                        value,
                        e.zoomify.zoom_in_transition,
                    )
                }
                "zoomOutTransition" => {
                    e.zoomify.zoom_out_transition = crate::zoomify::Transition::parse(
                        value,
                        e.zoomify.zoom_out_transition,
                    )
                }
                "zoomHand" => e.zoomify.affect_hand_fov = flag(e.zoomify.affect_hand_fov),
                "zoomRetain" => e.zoomify.retain_zoom_steps = flag(e.zoomify.retain_zoom_steps),
                "zoomScroll" => e.zoomify.scroll_zoom = flag(e.zoomify.scroll_zoom),
                "zoomScrollSteps" => {
                    if let Ok(v) = value.parse::<i32>() {
                        e.zoomify.scroll_step_count = v.clamp(1, 30);
                    }
                }
                "zoomPerStep" => {
                    if let Ok(v) = value.parse::<i32>() {
                        e.zoomify.zoom_per_step = v.clamp(101, 400);
                    }
                }
                "zoomScrollSmooth" => {
                    if let Ok(v) = value.parse::<i32>() {
                        e.zoomify.scroll_zoom_smoothness = v.clamp(0, 100);
                    }
                }
                "zoomKeyBehaviour" => {
                    e.zoomify.key_behaviour =
                        crate::zoomify::KeyBehaviour::parse(value, e.zoomify.key_behaviour)
                }
                "zoomBobbing" => {
                    e.zoomify.relative_view_bobbing = flag(e.zoomify.relative_view_bobbing)
                }
                "zoomSecondaryAmount" => {
                    if let Ok(v) = value.parse::<i32>() {
                        e.zoomify.secondary_zoom_amount = v.clamp(2, 50);
                    }
                }
                "zoomSecondaryIn" => {
                    if let Ok(v) = value.parse::<f64>() {
                        e.zoomify.secondary_zoom_in_time = v.clamp(0.0, 60.0);
                    }
                }
                "zoomSecondaryOut" => {
                    if let Ok(v) = value.parse::<f64>() {
                        e.zoomify.secondary_zoom_out_time = v.clamp(0.0, 60.0);
                    }
                }
                "fps" => e.fps = flag(e.fps),
                "fpsCorner" => e.fps_corner = Corner::parse(value, e.fps_corner),
                "fpsLows" => e.fps_lows = flag(e.fps_lows),
                "keystrokes" => e.keystrokes = flag(e.keystrokes),
                "keystrokesCorner" => {
                    e.keystrokes_corner = Corner::parse(value, e.keystrokes_corner)
                }
                "keystrokesCps" => e.keystrokes_cps = flag(e.keystrokes_cps),
                "minimap" => e.minimap = flag(e.minimap),
                "minimapCorner" => e.minimap_corner = Corner::parse(value, e.minimap_corner),
                "minimapSize" => {
                    if let Some(&(v, _)) = MINIMAP_SIZES.iter().find(|s| s.0.to_string() == value) {
                        e.minimap_size = v;
                    }
                }
                "minimapShape" => e.minimap_round = value == "round",
                "minimapZoom" => {
                    if let Some(&z) = MINIMAP_ZOOMS.iter().find(|z| z.to_string() == value) {
                        e.minimap_zoom = z;
                    }
                }
                "minimapRotate" => e.minimap_rotate = flag(e.minimap_rotate),
                "autoReconnect" => e.auto_reconnect = flag(e.auto_reconnect),
                "mouseTweaks" => e.mouse_tweaks = flag(e.mouse_tweaks),
                "mouseTweaksWheel" => e.mouse_tweaks_wheel = flag(e.mouse_tweaks_wheel),
                "mouseTweaksRmb" => e.mouse_tweaks_rmb = flag(e.mouse_tweaks_rmb),
                "mouseTweaksLmbItem" => e.mouse_tweaks_lmb_item = flag(e.mouse_tweaks_lmb_item),
                "mouseTweaksLmbEmpty" => e.mouse_tweaks_lmb_empty = flag(e.mouse_tweaks_lmb_empty),
                "mouseTweaksOrder" => {
                    e.mouse_tweaks_order =
                        crate::mouse_tweaks::SearchOrder::parse(value, e.mouse_tweaks_order)
                }
                "mouseTweaksDirection" => {
                    e.mouse_tweaks_direction =
                        crate::mouse_tweaks::Direction::parse(value, e.mouse_tweaks_direction)
                }
                "mouseTweaksScaling" => {
                    e.mouse_tweaks_scaling =
                        crate::mouse_tweaks::Scaling::parse(value, e.mouse_tweaks_scaling)
                }
                "appleSkin" => e.appleskin = flag(e.appleskin),
                "appleSkinRestored" => e.appleskin_restored = flag(e.appleskin_restored),
                "appleSkinSaturation" => e.appleskin_saturation = flag(e.appleskin_saturation),
                "appleSkinExhaustion" => e.appleskin_exhaustion = flag(e.appleskin_exhaustion),
                "appleSkinHealth" => e.appleskin_health = flag(e.appleskin_health),
                "appleSkinTooltips" => e.appleskin_tooltips = flag(e.appleskin_tooltips),
                "appleSkinTooltipsAlways" => {
                    e.appleskin_tooltips_always = flag(e.appleskin_tooltips_always)
                }
                "appleSkinOffhand" => e.appleskin_offhand = flag(e.appleskin_offhand),
                "appleSkinDebug" => e.appleskin_debug = flag(e.appleskin_debug),
                "jade" => e.jade = flag(e.jade),
                "jadeHarvest" => e.jade_harvest = flag(e.jade_harvest),
                "jadeDetails" => e.jade_details = flag(e.jade_details),
                "jei" => e.jei = flag(e.jei),
                "controlling" => e.controlling = flag(e.controlling),
                "sort" => e.sort = flag(e.sort),
                "sortKey" if value.starts_with("key.") => e.sort_key = value.to_owned(),
                horizon_core::no_ai::KEY => e.no_ai = flag(e.no_ai),
                "agentControl" => e.agent_control = flag(e.agent_control),
                _ => {}
            }
        }
        e
    }
    pub fn serialize(&self) -> String {
        format!(
            "fullbright:{}\ndynamicLights:{}\nzoom:{}\nzoomKey:{}\n{}fps:{}\nfpsCorner:{}\nfpsLows:{}\nkeystrokes:{}\nkeystrokesCorner:{}\nkeystrokesCps:{}\nminimap:{}\nminimapCorner:{}\nminimapSize:{}\nminimapShape:{}\nminimapZoom:{}\nminimapRotate:{}\nautoReconnect:{}\nmouseTweaks:{}\nmouseTweaksWheel:{}\nmouseTweaksRmb:{}\nmouseTweaksLmbItem:{}\nmouseTweaksLmbEmpty:{}\nmouseTweaksOrder:{}\nmouseTweaksDirection:{}\nmouseTweaksScaling:{}\nappleSkin:{}\nappleSkinRestored:{}\nappleSkinSaturation:{}\nappleSkinExhaustion:{}\nappleSkinHealth:{}\nappleSkinTooltips:{}\nappleSkinTooltipsAlways:{}\nappleSkinOffhand:{}\nappleSkinDebug:{}\njade:{}\njadeHarvest:{}\njadeDetails:{}\njei:{}\ncontrolling:{}\nsort:{}\nsortKey:{}\n{}:{}\nagentControl:{}\n",
            bool_text(self.fullbright),
            bool_text(self.dynamic_lights),
            bool_text(self.zoom),
            self.zoom_key,
            self.zoom_text(),
            bool_text(self.fps),
            self.fps_corner.name(),
            bool_text(self.fps_lows),
            bool_text(self.keystrokes),
            self.keystrokes_corner.name(),
            bool_text(self.keystrokes_cps),
            bool_text(self.minimap),
            self.minimap_corner.name(),
            self.minimap_size,
            if self.minimap_round {
                "round"
            } else {
                "square"
            },
            self.minimap_zoom,
            bool_text(self.minimap_rotate),
            bool_text(self.auto_reconnect),
            bool_text(self.mouse_tweaks),
            bool_text(self.mouse_tweaks_wheel),
            bool_text(self.mouse_tweaks_rmb),
            bool_text(self.mouse_tweaks_lmb_item),
            bool_text(self.mouse_tweaks_lmb_empty),
            self.mouse_tweaks_order.name(),
            self.mouse_tweaks_direction.name(),
            self.mouse_tweaks_scaling.name(),
            bool_text(self.appleskin),
            bool_text(self.appleskin_restored),
            bool_text(self.appleskin_saturation),
            bool_text(self.appleskin_exhaustion),
            bool_text(self.appleskin_health),
            bool_text(self.appleskin_tooltips),
            bool_text(self.appleskin_tooltips_always),
            bool_text(self.appleskin_offhand),
            bool_text(self.appleskin_debug),
            bool_text(self.jade),
            bool_text(self.jade_harvest),
            bool_text(self.jade_details),
            bool_text(self.jei),
            bool_text(self.controlling),
            bool_text(self.sort),
            self.sort_key,
            horizon_core::no_ai::KEY,
            bool_text(self.no_ai),
            bool_text(self.agent_control),
        )
    }
    /// Binds the extra key `name` (`ZOOM_KEY`, `SORT_KEY`); false for
    /// another name (a vanilla mapping).
    pub fn set_key(&mut self, name: &str, value: String) -> bool {
        match name {
            ZOOM_KEY => self.zoom_key = value,
            ZOOM2_KEY => self.zoom_secondary_key = value,
            SORT_KEY => self.sort_key = value,
            _ => return false,
        }
        true
    }
    /// Written only once something differs from the defaults, so a vanilla
    /// setup gains no file.
    pub fn save(&self, dir: &Path) {
        let path = dir.join(FILE);
        if *self == Self::default() && !path.exists() {
            return;
        }
        let _ = fs::write(path, self.serialize());
    }
}

/// Vanilla key mappings bound to the zoom key. The key still reaches them
/// (a key bound twice triggers both, as vanilla's do), so the Extras page
/// says so.
pub fn zoom_key_conflicts(e: &Extras, keys: &crate::keybinds::KeyBinds) -> Vec<&'static str> {
    if e.zoom_key == "key.keyboard.unknown" {
        return Vec::new();
    }
    crate::keybinds::MAPPINGS
        .iter()
        .map(|m| m.0)
        .filter(|name| keys.get(name) == e.zoom_key)
        .collect()
}
/// Frame times over the last few seconds: frames per second, the minimum
/// and the 1% low, recomputed twice a second.
pub struct FpsCounter {
    /// (end time in seconds, frame milliseconds), oldest first.
    frames: VecDeque<(f64, f32)>,
    shown: FpsReading,
    computed_at: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FpsReading {
    pub fps: u32,
    pub min: u32,
    pub low: u32,
}
/// Seconds the minimum and 1% low look back.
const LOW_WINDOW: f64 = 5.0;
impl Default for FpsCounter {
    fn default() -> Self {
        Self {
            frames: VecDeque::new(),
            shown: FpsReading::default(),
            computed_at: f64::NEG_INFINITY,
        }
    }
}
impl FpsCounter {
    /// A frame that ended at `now` (seconds) and took `ms`.
    pub fn record(&mut self, now: f64, ms: f32) {
        self.frames.push_back((now, ms));
        while self
            .frames
            .front()
            .is_some_and(|&(t, _)| now - t > LOW_WINDOW)
        {
            self.frames.pop_front();
        }
        if now - self.computed_at >= 0.5 {
            self.computed_at = now;
            self.shown = self.compute(now);
        }
    }
    fn compute(&self, now: f64) -> FpsReading {
        let fps = self.frames.iter().filter(|&&(t, _)| now - t < 1.0).count() as u32;
        let mut times: Vec<f32> = self.frames.iter().map(|f| f.1).collect();
        if times.is_empty() {
            return FpsReading::default();
        }
        times.sort_by(|a, b| b.total_cmp(a));
        let to_fps = |ms: f32| (1000.0 / ms.max(0.001)).round() as u32;
        // The 1% low: the frame rate of the 99th percentile frame time.
        let low = times[(times.len() - 1) / 100];
        FpsReading {
            fps,
            min: to_fps(times[0]),
            low: to_fps(low),
        }
    }
    pub fn reading(&self) -> FpsReading {
        self.shown
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Clicks of one button in the last second.
#[derive(Default)]
pub struct Cps {
    clicks: VecDeque<f64>,
}
impl Cps {
    pub fn click(&mut self, now: f64) {
        self.clicks.push_back(now);
        self.forget(now);
    }
    fn forget(&mut self, now: f64) {
        while self.clicks.front().is_some_and(|&t| now - t >= 1.0) {
            self.clicks.pop_front();
        }
    }
    pub fn per_second(&mut self, now: f64) -> usize {
        self.forget(now);
        self.clicks.len()
    }
}

/// The settings the Extras page shows, one per row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Fullbright,
    DynamicLights,
    Zoom,
    ZoomKey,
    ZoomLevel,
    ZoomInTime,
    ZoomOutTime,
    ZoomInTransition,
    ZoomOutTransition,
    ZoomToggle,
    ZoomScroll,
    ZoomScrollSteps,
    ZoomPerStep,
    ZoomScrollSmooth,
    ZoomRetain,
    ZoomHand,
    ZoomSensitivity,
    ZoomBobbing,
    ZoomSecondaryKey,
    ZoomSecondaryAmount,
    ZoomSecondaryIn,
    ZoomSecondaryOut,
    Fps,
    FpsCorner,
    FpsLows,
    Keystrokes,
    KeystrokesCorner,
    KeystrokesCps,
    Minimap,
    MinimapCorner,
    MinimapSize,
    MinimapShape,
    MinimapZoom,
    MinimapRotate,
    AutoReconnect,
    MouseTweaks,
    MouseTweaksWheel,
    MouseTweaksRmb,
    MouseTweaksLmbItem,
    MouseTweaksLmbEmpty,
    MouseTweaksOrder,
    MouseTweaksDirection,
    MouseTweaksScaling,
    AppleSkin,
    AppleSkinRestored,
    AppleSkinSaturation,
    AppleSkinExhaustion,
    AppleSkinHealth,
    AppleSkinTooltips,
    AppleSkinTooltipsAlways,
    AppleSkinOffhand,
    AppleSkinDebug,
    Jade,
    JadeHarvest,
    JadeDetails,
    Jei,
    Controlling,
    Sort,
    SortKey,
    /// Not on the Extras page: Horizon Settings shows No AI, its AI Agent page Agent Control.
    NoAi,
    AgentControl,
}
/// The Extras page: headers and items.
pub fn rows() -> Vec<Result<Item, &'static str>> {
    use Item::*;
    vec![
        Err("Lighting"),
        Ok(Fullbright),
        Ok(DynamicLights),
        Err("Zoom"),
        Ok(Zoom),
        Ok(ZoomKey),
        Ok(ZoomLevel),
        Ok(ZoomToggle),
        Ok(ZoomInTime),
        Ok(ZoomInTransition),
        Ok(ZoomOutTime),
        Ok(ZoomOutTransition),
        Ok(ZoomScroll),
        Ok(ZoomScrollSteps),
        Ok(ZoomPerStep),
        Ok(ZoomScrollSmooth),
        Ok(ZoomRetain),
        Ok(ZoomHand),
        Ok(ZoomSensitivity),
        Ok(ZoomBobbing),
        Ok(ZoomSecondaryKey),
        Ok(ZoomSecondaryAmount),
        Ok(ZoomSecondaryIn),
        Ok(ZoomSecondaryOut),
        Err("Minimap"),
        Ok(Minimap),
        Ok(MinimapCorner),
        Ok(MinimapSize),
        Ok(MinimapShape),
        Ok(MinimapZoom),
        Ok(MinimapRotate),
        Err("FPS counter"),
        Ok(Fps),
        Ok(FpsCorner),
        Ok(FpsLows),
        Err("Keystrokes"),
        Ok(Keystrokes),
        Ok(KeystrokesCorner),
        Ok(KeystrokesCps),
        Err("AppleSkin"),
        Ok(AppleSkin),
        Ok(AppleSkinRestored),
        Ok(AppleSkinSaturation),
        Ok(AppleSkinExhaustion),
        Ok(AppleSkinHealth),
        Ok(AppleSkinTooltips),
        Ok(AppleSkinTooltipsAlways),
        Ok(AppleSkinOffhand),
        Ok(AppleSkinDebug),
        Err("Jade"),
        Ok(Jade),
        Ok(JadeHarvest),
        Ok(JadeDetails),
        Err("Recipes"),
        Ok(Jei),
        Err("Controls"),
        Ok(Controlling),
        Err("Inventory"),
        Ok(MouseTweaks),
        Ok(MouseTweaksWheel),
        Ok(MouseTweaksRmb),
        Ok(MouseTweaksLmbItem),
        Ok(MouseTweaksLmbEmpty),
        Ok(MouseTweaksOrder),
        Ok(MouseTweaksDirection),
        Ok(MouseTweaksScaling),
        Ok(Sort),
        Ok(SortKey),
        Err("Multiplayer"),
        Ok(AutoReconnect),
    ]
}
/// "1 second", "½ s".
fn seconds(t: f64) -> String {
    if t == 0.0 {
        "Instant".into()
    } else {
        format!("{t} s")
    }
}
/// "1 block", "2 blocks", "½ block".
fn block_count(n: f32) -> String {
    if n < 1.0 {
        "½ block".into()
    } else if n == 1.0 {
        "1 block".into()
    } else {
        format!("{n} blocks")
    }
}
fn next<T: Copy + PartialEq>(all: &[T], current: T, back: bool) -> T {
    let i = all.iter().position(|v| *v == current).unwrap_or(0);
    let n = all.len();
    all[if back { (i + n - 1) % n } else { (i + 1) % n }]
}
impl Item {
    pub fn caption(self) -> &'static str {
        match self {
            Item::Fullbright => "Fullbright",
            Item::DynamicLights => "Dynamic lights",
            Item::Zoom => "Zoom",
            Item::ZoomKey => "Zoom key",
            Item::ZoomLevel => "Initial zoom",
            Item::ZoomInTime => "Zoom in time",
            Item::ZoomOutTime => "Zoom out time",
            Item::ZoomInTransition => "Zoom in transition",
            Item::ZoomOutTransition => "Zoom out transition",
            Item::ZoomToggle => "Zoom key behaviour",
            Item::ZoomScroll => "Scroll zoom",
            Item::ZoomScrollSteps => "Scroll steps",
            Item::ZoomPerStep => "Zoom per step",
            Item::ZoomScrollSmooth => "Scroll smoothness",
            Item::ZoomRetain => "Keep scroll steps",
            Item::ZoomHand => "Zoom the hand too",
            Item::ZoomSensitivity => "Relative sensitivity",
            Item::ZoomBobbing => "Relative view bobbing",
            Item::ZoomSecondaryKey => "Secondary zoom key",
            Item::ZoomSecondaryAmount => "Secondary zoom",
            Item::ZoomSecondaryIn => "Secondary zoom in time",
            Item::ZoomSecondaryOut => "Secondary zoom out time",
            Item::Fps => "FPS counter",
            Item::FpsCorner | Item::KeystrokesCorner | Item::MinimapCorner => "Corner",
            Item::Minimap => "Minimap",
            Item::MinimapSize => "Size",
            Item::MinimapShape => "Shape",
            Item::MinimapZoom => "Zoom",
            Item::MinimapRotate => "Turn with the player",
            Item::AutoReconnect => "Auto-reconnect",
            Item::MouseTweaks => "Mouse Tweaks controls",
            Item::MouseTweaksWheel => "Scroll wheel moves items",
            Item::MouseTweaksRmb => "Right-drag places one per slot",
            Item::MouseTweaksLmbItem => "Left-drag gathers the held item",
            Item::MouseTweaksLmbEmpty => "Shift + left-drag moves slots",
            Item::MouseTweaksOrder => "Wheel pulls from",
            Item::MouseTweaksDirection => "Wheel direction",
            Item::MouseTweaksScaling => "Wheel items per tick",
            Item::AppleSkin => "AppleSkin",
            Item::AppleSkinRestored => "Hunger preview",
            Item::AppleSkinSaturation => "Saturation outlines",
            Item::AppleSkinExhaustion => "Exhaustion strip",
            Item::AppleSkinHealth => "Health preview",
            Item::AppleSkinTooltips => "Food values in tooltips",
            Item::AppleSkinTooltipsAlways => "Tooltip values without Shift",
            Item::AppleSkinOffhand => "Off-hand food",
            Item::AppleSkinDebug => "Food values on F3",
            Item::Jade => "Jade",
            Item::JadeHarvest => "Harvest info",
            Item::JadeDetails => "Details",
            Item::Jei => "Item list and recipes (JEI)",
            Item::Controlling => "Controlling",
            Item::Sort => "Sort buttons",
            Item::SortKey => "Sort key",
            Item::FpsLows => "Minimum and 1% low",
            Item::Keystrokes => "Keystrokes",
            Item::KeystrokesCps => "Clicks per second",
            Item::NoAi => "No AI",
            Item::AgentControl => "Agent Control",
        }
    }
    /// The switch this setting belongs to: it does nothing while that is off.
    pub fn parent(self) -> Option<Item> {
        use Item::*;
        Some(match self {
            ZoomKey | ZoomLevel | ZoomInTime | ZoomOutTime | ZoomInTransition
            | ZoomOutTransition | ZoomToggle | ZoomScroll | ZoomScrollSteps | ZoomPerStep
            | ZoomScrollSmooth | ZoomRetain | ZoomHand | ZoomSensitivity | ZoomBobbing
            | ZoomSecondaryKey | ZoomSecondaryAmount | ZoomSecondaryIn | ZoomSecondaryOut => Zoom,
            MinimapCorner | MinimapSize | MinimapShape | MinimapZoom | MinimapRotate => Minimap,
            FpsCorner | FpsLows => Fps,
            KeystrokesCorner | KeystrokesCps => Keystrokes,
            MouseTweaksWheel | MouseTweaksRmb | MouseTweaksLmbItem | MouseTweaksLmbEmpty
            | MouseTweaksOrder | MouseTweaksDirection | MouseTweaksScaling => MouseTweaks,
            AppleSkinRestored | AppleSkinSaturation | AppleSkinExhaustion | AppleSkinHealth
            | AppleSkinTooltips | AppleSkinTooltipsAlways | AppleSkinOffhand | AppleSkinDebug => {
                AppleSkin
            }
            JadeHarvest | JadeDetails => Jade,
            SortKey => Sort,
            Fullbright | DynamicLights | Zoom | Minimap | Fps | Keystrokes | MouseTweaks | Sort
            | AppleSkin | Jade | Jei | Controlling | AutoReconnect | NoAi | AgentControl => {
                return None;
            }
        })
    }
    /// What the row says under its caption.
    pub fn description(self) -> &'static str {
        match self {
            Item::Fullbright => "Show blocks and entities at full light, even in dark areas.",
            Item::DynamicLights => {
                "Torches and other light sources held, dropped or on fire light the blocks around them."
            }
            Item::Zoom => {
                "Zoom with the zoom key; the wheel steps the zoom further while it is down. A rewrite of Zoomify."
            }
            Item::ZoomKey => "Click, then press a key. Escape unbinds it.",
            Item::ZoomLevel => "How far the key zooms in: the field of view is divided by this.",
            Item::ZoomInTime => "Seconds the zoom takes to ease in.",
            Item::ZoomOutTime => "Seconds the zoom takes to ease back out.",
            Item::ZoomInTransition => "The curve the zoom follows going in.",
            Item::ZoomOutTransition => "The curve the zoom follows coming out.",
            Item::ZoomToggle => "Hold: zoom while the key is down. Toggle: press to zoom, press to stop.",
            Item::ZoomScroll => "The wheel zooms in and out in steps while you zoom, instead of the hotbar.",
            Item::ZoomScrollSteps => "How many steps the wheel can zoom further in.",
            Item::ZoomPerStep => "How much each step multiplies the zoom.",
            Item::ZoomScrollSmooth => "How smoothly the steps ease; 100 is the smoothest.",
            Item::ZoomRetain => "Remembers the scroll steps after you let go of the key.",
            Item::ZoomHand => "Your hand and held item zoom with the world.",
            Item::ZoomSensitivity => {
                "How much the mouse slows with the zoom: 100 turns as slowly as the view moves, 0 not at all."
            }
            Item::ZoomBobbing => "View bobbing shrinks while zoomed in.",
            Item::ZoomSecondaryKey => "A second zoom that eases in slowly. Click, then press a key.",
            Item::ZoomSecondaryAmount => "How far the secondary zoom goes.",
            Item::ZoomSecondaryIn => "Seconds the secondary zoom takes to go in.",
            Item::ZoomSecondaryOut => "Seconds the secondary zoom takes to come out.",
            Item::Fps => "Frames per second in a corner of the screen.",
            Item::FpsCorner | Item::KeystrokesCorner | Item::MinimapCorner => {
                "Overlays in one corner stack."
            }
            Item::Minimap => "A map of the loaded terrain around you, in map colours.",
            Item::MinimapSize => "How big the map is on screen.",
            Item::MinimapShape => "Square or round.",
            Item::MinimapZoom => "Blocks per pixel: smaller shows more detail, larger more land.",
            Item::MinimapRotate => "Forward is up; off keeps north up.",
            Item::AutoReconnect => {
                "Rejoins a server after a lost connection or a restart, never after a kick or ban."
            }
            Item::FpsLows => "The slowest frame and the 1% low of the last 5 seconds.",
            Item::MouseTweaks => {
                "Right-drag places one per slot, left-drag gathers the held item, Shift + left-drag moves every slot."
            }
            Item::MouseTweaksWheel => {
                "Down sends one item of the slot to the other inventory, up brings one back. Works with a stack in hand too."
            }
            Item::MouseTweaksRmb => {
                "Holding a stack, right-drag over slots to put one item in each."
            }
            Item::MouseTweaksLmbItem => {
                "Holding a stack, left-drag over slots of the same item to gather them into your hand."
            }
            Item::MouseTweaksLmbEmpty => {
                "With an empty hand, Shift + left-drag over slots to move each one to the other inventory."
            }
            Item::MouseTweaksOrder => {
                "Which end of the other inventory the wheel takes items from first."
            }
            Item::MouseTweaksDirection => {
                "Normal: down moves items out. Inverted flips it. Position aware: down moves items away from the other inventory's side of the screen."
            }
            Item::MouseTweaksScaling => {
                "Proportional moves as many items as the wheel scrolled; always one moves one per tick."
            }
            Item::AppleSkin => {
                "Shows what food would do: on the food and health bars, in tooltips and on F3. A rewrite of the Java mod."
            }
            Item::AppleSkinRestored => {
                "Holding food, the hunger and saturation it would give flash on the food bar."
            }
            Item::AppleSkinSaturation => "Golden outlines on the food icons show your saturation.",
            Item::AppleSkinExhaustion => {
                "A strip under the food icons grows as exhaustion builds toward costing a point."
            }
            Item::AppleSkinHealth => {
                "Holding food, the hearts natural regeneration would give flash on the health bar."
            }
            Item::AppleSkinTooltips => "Food items show the hunger and saturation they give as icons.",
            Item::AppleSkinTooltipsAlways => "Show them without holding Shift.",
            Item::AppleSkinOffhand => "The previews also work for food in your off hand.",
            Item::AppleSkinDebug => "Hunger, saturation and exhaustion on the F3 screen.",
            Item::Jade => {
                "A box at the top of the screen names the block or mob under your crosshair, with its icon, health and details. A rewrite of the Java mod."
            }
            Item::JadeHarvest => {
                "Says which tool a block needs and whether the one in your hand can harvest it."
            }
            Item::JadeDetails => "Crop growth, redstone power and similar state, under the name.",
            Item::Jei => {
                "Lists every item beside the inventory with a search box. Click an item for how it is made, right-click for what uses it; R and U do the same over any item. A rewrite of the Java mod."
            }
            Item::Controlling => {
                "Key Binds search also finds keys and categories, and a button shows only conflicts, unbound or changed keys."
            }
            Item::Sort => {
                "Sorts the inventory, chests, barrels, ender chests and shulker boxes in creative tab order, merging stacks."
            }
            Item::SortKey => {
                "Sorts the part under the cursor. Click, then press a key; Escape unbinds it."
            }
            Item::Keystrokes => "Movement keys, jump and mouse buttons as you press them.",
            Item::KeystrokesCps => "Clicks in the last second under each mouse button.",
            Item::NoAi => {
                "Hides and turns off every AI feature: agent control, horizon-ctl and the MCP servers. The launcher's switch too."
            }
            Item::AgentControl => {
                "Your own AI agent can play this game through horizon-ctl or an MCP client, on this computer only."
            }
        }
    }
    /// The frame-time cost the main switch of an extra shows.
    pub fn cost(self) -> Option<crate::graphics_style::Cost> {
        use crate::graphics_style::Cost;
        match self {
            Item::Fullbright | Item::Zoom | Item::Fps | Item::Keystrokes => Some(Cost::None),
            Item::Minimap | Item::DynamicLights => Some(Cost::Low),
            Item::AutoReconnect
            | Item::MouseTweaks
            | Item::Sort
            | Item::AppleSkin
            | Item::Jade
            | Item::Jei
            | Item::Controlling => Some(Cost::None),
            _ => None,
        }
    }
    /// On/off items and their value.
    pub fn switch(self, e: &Extras) -> Option<bool> {
        Some(match self {
            Item::Fullbright => e.fullbright,
            Item::DynamicLights => e.dynamic_lights,
            Item::Zoom => e.zoom,
            Item::ZoomScroll => e.zoomify.scroll_zoom,
            Item::ZoomRetain => e.zoomify.retain_zoom_steps,
            Item::ZoomHand => e.zoomify.affect_hand_fov,
            Item::ZoomBobbing => e.zoomify.relative_view_bobbing,
            Item::Fps => e.fps,
            Item::FpsLows => e.fps_lows,
            Item::Keystrokes => e.keystrokes,
            Item::KeystrokesCps => e.keystrokes_cps,
            Item::Minimap => e.minimap,
            Item::MinimapRotate => e.minimap_rotate,
            Item::AutoReconnect => e.auto_reconnect,
            Item::MouseTweaks => e.mouse_tweaks,
            Item::MouseTweaksWheel => e.mouse_tweaks_wheel,
            Item::MouseTweaksRmb => e.mouse_tweaks_rmb,
            Item::MouseTweaksLmbItem => e.mouse_tweaks_lmb_item,
            Item::MouseTweaksLmbEmpty => e.mouse_tweaks_lmb_empty,
            Item::AppleSkin => e.appleskin,
            Item::AppleSkinRestored => e.appleskin_restored,
            Item::AppleSkinSaturation => e.appleskin_saturation,
            Item::AppleSkinExhaustion => e.appleskin_exhaustion,
            Item::AppleSkinHealth => e.appleskin_health,
            Item::AppleSkinTooltips => e.appleskin_tooltips,
            Item::AppleSkinTooltipsAlways => e.appleskin_tooltips_always,
            Item::AppleSkinOffhand => e.appleskin_offhand,
            Item::AppleSkinDebug => e.appleskin_debug,
            Item::Jade => e.jade,
            Item::JadeHarvest => e.jade_harvest,
            Item::JadeDetails => e.jade_details,
            Item::Jei => e.jei,
            Item::Controlling => e.controlling,
            Item::Sort => e.sort,
            Item::NoAi => e.no_ai,
            Item::AgentControl => e.agent_control,
            _ => return None,
        })
    }
    /// The value of the other items, as their button shows it.
    pub fn value(self, e: &Extras) -> String {
        match self {
            Item::ZoomKey => crate::keybinds::display_name(&e.zoom_key).plain(),
            Item::SortKey => crate::keybinds::display_name(&e.sort_key).plain(),
            Item::ZoomLevel => format!("{}x", e.zoomify.initial_zoom),
            Item::ZoomInTime => seconds(e.zoomify.zoom_in_time),
            Item::ZoomOutTime => seconds(e.zoomify.zoom_out_time),
            Item::ZoomInTransition => e.zoomify.zoom_in_transition.label().into(),
            Item::ZoomOutTransition => e.zoomify.zoom_out_transition.label().into(),
            Item::ZoomToggle => e.zoomify.key_behaviour.label().into(),
            Item::ZoomScrollSteps => e.zoomify.scroll_step_count.to_string(),
            Item::ZoomPerStep => format!("{}%", e.zoomify.zoom_per_step),
            Item::ZoomScrollSmooth => format!("{}%", e.zoomify.scroll_zoom_smoothness),
            Item::ZoomSensitivity => format!("{}%", e.zoomify.relative_sensitivity),
            Item::ZoomSecondaryKey => {
                crate::keybinds::display_name(&e.zoom_secondary_key).plain()
            }
            Item::ZoomSecondaryAmount => format!("{}x", e.zoomify.secondary_zoom_amount),
            Item::ZoomSecondaryIn => seconds(e.zoomify.secondary_zoom_in_time),
            Item::ZoomSecondaryOut => seconds(e.zoomify.secondary_zoom_out_time),
            Item::FpsCorner => e.fps_corner.label().into(),
            Item::KeystrokesCorner => e.keystrokes_corner.label().into(),
            Item::MinimapCorner => e.minimap_corner.label().into(),
            Item::MinimapSize => MINIMAP_SIZES
                .iter()
                .find(|s| s.0 == e.minimap_size)
                .map_or("Medium", |s| s.1)
                .into(),
            Item::MinimapShape => if e.minimap_round { "Round" } else { "Square" }.into(),
            Item::MinimapZoom => format!("{} per pixel", block_count(e.minimap_zoom)),
            Item::MouseTweaksOrder => e.mouse_tweaks_order.label().into(),
            Item::MouseTweaksDirection => e.mouse_tweaks_direction.label().into(),
            Item::MouseTweaksScaling => e.mouse_tweaks_scaling.label().into(),
            other => match other.switch(e) {
                Some(true) => "On".into(),
                _ => "Off".into(),
            },
        }
    }
    /// A click (`back` for the right button). The zoom key binds instead.
    pub fn cycle(self, e: &mut Extras, back: bool) {
        match self {
            Item::Fullbright => e.fullbright = !e.fullbright,
            Item::DynamicLights => e.dynamic_lights = !e.dynamic_lights,
            Item::Zoom => e.zoom = !e.zoom,
            Item::ZoomScroll => e.zoomify.scroll_zoom = !e.zoomify.scroll_zoom,
            Item::ZoomRetain => e.zoomify.retain_zoom_steps = !e.zoomify.retain_zoom_steps,
            Item::ZoomHand => e.zoomify.affect_hand_fov = !e.zoomify.affect_hand_fov,
            Item::ZoomBobbing => {
                e.zoomify.relative_view_bobbing = !e.zoomify.relative_view_bobbing
            }
            Item::ZoomInTime => {
                e.zoomify.zoom_in_time = next(&ZOOM_TIMES, e.zoomify.zoom_in_time, back)
            }
            Item::ZoomOutTime => {
                e.zoomify.zoom_out_time = next(&ZOOM_TIMES, e.zoomify.zoom_out_time, back)
            }
            Item::ZoomInTransition => {
                e.zoomify.zoom_in_transition = next(
                    &crate::zoomify::Transition::ALL,
                    e.zoomify.zoom_in_transition,
                    back,
                )
            }
            Item::ZoomOutTransition => {
                e.zoomify.zoom_out_transition = next(
                    &crate::zoomify::Transition::ALL,
                    e.zoomify.zoom_out_transition,
                    back,
                )
            }
            Item::ZoomToggle => {
                e.zoomify.key_behaviour = next(
                    &crate::zoomify::KeyBehaviour::ALL,
                    e.zoomify.key_behaviour,
                    back,
                )
            }
            Item::ZoomScrollSteps => {
                e.zoomify.scroll_step_count =
                    next(&ZOOM_STEPS, e.zoomify.scroll_step_count, back)
            }
            Item::ZoomPerStep => {
                e.zoomify.zoom_per_step = next(&ZOOM_PER_STEP, e.zoomify.zoom_per_step, back)
            }
            Item::ZoomScrollSmooth => {
                e.zoomify.scroll_zoom_smoothness =
                    next(&ZOOM_SMOOTH, e.zoomify.scroll_zoom_smoothness, back)
            }
            Item::ZoomSensitivity => {
                e.zoomify.relative_sensitivity =
                    next(&ZOOM_SENSITIVITY, e.zoomify.relative_sensitivity, back)
            }
            Item::ZoomSecondaryAmount => {
                e.zoomify.secondary_zoom_amount =
                    next(&ZOOM_SLOW_AMOUNT, e.zoomify.secondary_zoom_amount, back)
            }
            Item::ZoomSecondaryIn => {
                e.zoomify.secondary_zoom_in_time =
                    next(&ZOOM_SLOW_IN, e.zoomify.secondary_zoom_in_time, back)
            }
            Item::ZoomSecondaryOut => {
                e.zoomify.secondary_zoom_out_time =
                    next(&ZOOM_SLOW_OUT, e.zoomify.secondary_zoom_out_time, back)
            }
            Item::Fps => e.fps = !e.fps,
            Item::FpsLows => e.fps_lows = !e.fps_lows,
            Item::Keystrokes => e.keystrokes = !e.keystrokes,
            Item::KeystrokesCps => e.keystrokes_cps = !e.keystrokes_cps,
            Item::ZoomLevel => {
                e.zoomify.initial_zoom = next(&ZOOM_LEVELS, e.zoomify.initial_zoom, back)
            }
            Item::FpsCorner => e.fps_corner = next(&Corner::ALL, e.fps_corner, back),
            Item::KeystrokesCorner => {
                e.keystrokes_corner = next(&Corner::ALL, e.keystrokes_corner, back)
            }
            Item::Minimap => e.minimap = !e.minimap,
            Item::MinimapRotate => e.minimap_rotate = !e.minimap_rotate,
            Item::AutoReconnect => e.auto_reconnect = !e.auto_reconnect,
            Item::MouseTweaks => e.mouse_tweaks = !e.mouse_tweaks,
            Item::MouseTweaksWheel => e.mouse_tweaks_wheel = !e.mouse_tweaks_wheel,
            Item::MouseTweaksRmb => e.mouse_tweaks_rmb = !e.mouse_tweaks_rmb,
            Item::MouseTweaksLmbItem => e.mouse_tweaks_lmb_item = !e.mouse_tweaks_lmb_item,
            Item::MouseTweaksLmbEmpty => e.mouse_tweaks_lmb_empty = !e.mouse_tweaks_lmb_empty,
            Item::MouseTweaksOrder => {
                e.mouse_tweaks_order = next(
                    &crate::mouse_tweaks::SearchOrder::ALL,
                    e.mouse_tweaks_order,
                    back,
                )
            }
            Item::MouseTweaksDirection => {
                e.mouse_tweaks_direction = next(
                    &crate::mouse_tweaks::Direction::ALL,
                    e.mouse_tweaks_direction,
                    back,
                )
            }
            Item::MouseTweaksScaling => {
                e.mouse_tweaks_scaling = next(
                    &crate::mouse_tweaks::Scaling::ALL,
                    e.mouse_tweaks_scaling,
                    back,
                )
            }
            Item::AppleSkin => e.appleskin = !e.appleskin,
            Item::AppleSkinRestored => e.appleskin_restored = !e.appleskin_restored,
            Item::AppleSkinSaturation => e.appleskin_saturation = !e.appleskin_saturation,
            Item::AppleSkinExhaustion => e.appleskin_exhaustion = !e.appleskin_exhaustion,
            Item::AppleSkinHealth => e.appleskin_health = !e.appleskin_health,
            Item::AppleSkinTooltips => e.appleskin_tooltips = !e.appleskin_tooltips,
            Item::AppleSkinTooltipsAlways => {
                e.appleskin_tooltips_always = !e.appleskin_tooltips_always
            }
            Item::AppleSkinOffhand => e.appleskin_offhand = !e.appleskin_offhand,
            Item::AppleSkinDebug => e.appleskin_debug = !e.appleskin_debug,
            Item::Jade => e.jade = !e.jade,
            Item::JadeHarvest => e.jade_harvest = !e.jade_harvest,
            Item::JadeDetails => e.jade_details = !e.jade_details,
            Item::Jei => e.jei = !e.jei,
            Item::Controlling => e.controlling = !e.controlling,
            Item::Sort => e.sort = !e.sort,
            Item::MinimapCorner => e.minimap_corner = next(&Corner::ALL, e.minimap_corner, back),
            Item::MinimapSize => {
                let sizes: Vec<u8> = MINIMAP_SIZES.iter().map(|s| s.0).collect();
                e.minimap_size = next(&sizes, e.minimap_size, back);
            }
            Item::MinimapShape => e.minimap_round = !e.minimap_round,
            Item::MinimapZoom => e.minimap_zoom = next(&MINIMAP_ZOOMS, e.minimap_zoom, back),
            Item::NoAi => e.no_ai = !e.no_ai,
            Item::AgentControl => e.agent_control = !e.agent_control,
            Item::ZoomKey | Item::ZoomSecondaryKey | Item::SortKey => {}
        }
    }
    /// The key name the options screen waits on for a key item.
    pub fn key_name(self) -> Option<&'static str> {
        match self {
            Item::ZoomKey => Some(ZOOM_KEY),
            Item::ZoomSecondaryKey => Some(ZOOM2_KEY),
            Item::SortKey => Some(SORT_KEY),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_is_off_by_default_and_round_trips() {
        let d = Extras::default();
        assert!(!d.fullbright && !d.dynamic_lights);
        assert!(!d.zoom && !d.fps && !d.keystrokes && !d.minimap && !d.auto_reconnect);
        assert!(!d.mouse_tweaks && !d.sort);
        assert_eq!(Extras::parse(""), d);
        assert_eq!(Extras::parse(&d.serialize()), d);
        let mut e = d.clone();
        Item::Fullbright.cycle(&mut e, false);
        assert_eq!(Item::Fullbright.switch(&e), Some(true));
        Item::DynamicLights.cycle(&mut e, false);
        assert_eq!(Item::DynamicLights.switch(&e), Some(true));
        assert_eq!(Extras::parse(&e.serialize()), e);
        e.zoom = true;
        e.zoom_key = "key.keyboard.c".into();
        e.zoomify.initial_zoom = 6;
        e.zoomify.zoom_in_time = 0.25;
        e.zoomify.zoom_out_transition = crate::zoomify::Transition::EaseInOutCubic;
        e.zoomify.key_behaviour = crate::zoomify::KeyBehaviour::Toggle;
        e.zoomify.scroll_step_count = 15;
        e.zoomify.zoom_per_step = 125;
        e.zoomify.relative_sensitivity = 50;
        e.zoomify.affect_hand_fov = false;
        e.zoomify.secondary_zoom_in_time = 5.0;
        e.zoom_secondary_key = "key.keyboard.g".into();
        e.fps_corner = Corner::BottomRight;
        e.keystrokes = true;
        e.keystrokes_cps = false;
        e.minimap = true;
        e.minimap_size = 128;
        e.minimap_round = true;
        e.minimap_zoom = 0.5;
        e.minimap_rotate = true;
        e.minimap_corner = Corner::BottomLeft;
        e.auto_reconnect = true;
        e.mouse_tweaks = true;
        e.mouse_tweaks_wheel = false;
        e.mouse_tweaks_rmb = false;
        e.mouse_tweaks_order = crate::mouse_tweaks::SearchOrder::FirstToLast;
        e.mouse_tweaks_direction = crate::mouse_tweaks::Direction::PositionAwareInverted;
        e.mouse_tweaks_scaling = crate::mouse_tweaks::Scaling::AlwaysOne;
        e.appleskin = true;
        e.appleskin_restored = false;
        e.appleskin_debug = false;
        e.sort = true;
        e.sort_key = "key.keyboard.r".into();
        assert_eq!(Extras::parse(&e.serialize()), e);
    }

    #[test]
    fn bad_lines_keep_defaults() {
        let e = Extras::parse("zoom:maybe\nzoomLevel:99\nfpsCorner:middle\nzoomKey:z\nnoise");
        assert!(!e.zoom);
        assert_eq!(e.zoomify.initial_zoom, 50);
        assert_eq!(e.fps_corner, Corner::TopLeft);
        assert_eq!(e.zoom_key, "key.keyboard.z");
    }

    #[test]
    fn default_zoom_key_is_free_in_vanilla() {
        let key = Extras::default().zoom_key;
        assert!(crate::keybinds::key_code(&key).is_some());
        assert!(
            crate::keybinds::MAPPINGS.iter().all(|m| m.1 != key),
            "{key} is a vanilla binding"
        );
    }

    #[test]
    fn zoom_settings_from_before_zoomify_still_load() {
        let e = Extras::parse("zoomSmooth:false\nzoomSensitivity:false\nzoomLevel:6");
        assert_eq!(e.zoomify.initial_zoom, 6);
        assert_eq!(e.zoomify.zoom_in_transition, crate::zoomify::Transition::Instant);
        assert_eq!(e.zoomify.relative_sensitivity, 0);
        let on = Extras::parse("zoomSensitivity:true");
        assert_eq!(on.zoomify.relative_sensitivity, 100);
        assert_eq!(Extras::parse("zoomSensitivity:40").zoomify.relative_sensitivity, 40);
    }

    #[test]
    fn zoom_key_conflicts_name_the_vanilla_mappings() {
        let mut e = Extras::default();
        let mut keys = crate::keybinds::KeyBinds::default();
        assert!(zoom_key_conflicts(&e, &keys).is_empty());
        e.zoom_key = "key.keyboard.c".into();
        assert_eq!(
            zoom_key_conflicts(&e, &keys),
            [
                "key.saveToolbarActivator",
                "key.debug.crash",
                "key.debug.copyLocation"
            ]
        );
        keys.set("key.drop", "key.keyboard.z");
        e.zoom_key = "key.keyboard.z".into();
        assert_eq!(zoom_key_conflicts(&e, &keys), ["key.drop"]);
        e.zoom_key = "key.keyboard.unknown".into();
        keys.set("key.smoothCamera", "key.keyboard.unknown");
        assert!(zoom_key_conflicts(&e, &keys).is_empty());
    }

    #[test]
    fn fps_counter_reports_rate_minimum_and_low() {
        let mut c = FpsCounter::default();
        let mut t = 0.0;
        // Two seconds at 100 fps with one 50 ms hitch.
        for i in 0..200 {
            let ms = if i == 150 { 50.0 } else { 10.0 };
            t += f64::from(ms) / 1000.0;
            c.record(t, ms);
        }
        c.computed_at = f64::NEG_INFINITY;
        c.record(t + 0.01, 10.0);
        let r = c.reading();
        assert!((95..=101).contains(&r.fps), "{r:?}");
        assert_eq!(r.min, 20);
        assert_eq!(r.low, 100, "one frame in 200 is under the 1% mark");
        // Hitches beyond 1% of frames pull the 1% low down.
        let mut c = FpsCounter::default();
        let mut t = 0.0;
        for i in 0..100 {
            let ms = if i % 25 == 0 { 40.0 } else { 10.0 };
            t += f64::from(ms) / 1000.0;
            c.record(t, ms);
        }
        c.computed_at = f64::NEG_INFINITY;
        c.record(t, 10.0);
        assert_eq!(c.reading().low, 25);
    }

    #[test]
    fn old_frames_leave_the_window() {
        let mut c = FpsCounter::default();
        c.record(0.0, 100.0);
        for i in 1..=700 {
            c.record(f64::from(i) / 100.0, 10.0);
        }
        assert_eq!(c.reading().min, 100, "the 100 ms frame is 7 s old");
    }

    #[test]
    fn cps_counts_the_last_second() {
        let mut cps = Cps::default();
        for i in 0..10 {
            cps.click(f64::from(i) * 0.1);
        }
        assert_eq!(cps.per_second(0.95), 10);
        assert_eq!(cps.per_second(1.25), 7);
        assert_eq!(cps.per_second(3.0), 0);
    }

    #[test]
    fn items_cycle_through_their_values() {
        let mut e = Extras::default();
        Item::ZoomLevel.cycle(&mut e, false);
        assert_eq!(e.zoomify.initial_zoom, 5);
        Item::ZoomLevel.cycle(&mut e, true);
        Item::ZoomLevel.cycle(&mut e, true);
        assert_eq!(e.zoomify.initial_zoom, 3);
        Item::ZoomInTransition.cycle(&mut e, false);
        assert_eq!(
            e.zoomify.zoom_in_transition,
            crate::zoomify::Transition::EaseInOutExp
        );
        assert_eq!(Item::ZoomInTime.value(&e), "1 s");
        Item::ZoomInTime.cycle(&mut e, true);
        assert_eq!(Item::ZoomInTime.value(&e), "0.5 s");
        Item::FpsCorner.cycle(&mut e, true);
        assert_eq!(e.fps_corner, Corner::BottomRight);
        Item::Zoom.cycle(&mut e, false);
        assert_eq!(Item::Zoom.switch(&e), Some(true));
        assert_eq!(Item::ZoomKey.value(&e), "Z");
        Item::MinimapZoom.cycle(&mut e, true);
        assert_eq!(Item::MinimapZoom.value(&e), "½ block per pixel");
        Item::MinimapSize.cycle(&mut e, false);
        assert_eq!(Item::MinimapSize.value(&e), "Large");
        assert_eq!(
            Extras::parse("minimapSize:7\nminimapZoom:3").minimap_zoom,
            1.0
        );
        for item in rows().into_iter().flatten() {
            assert!(!item.caption().is_empty() && !item.description().is_empty());
        }
    }
}
