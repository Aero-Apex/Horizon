//! AppleSkin for Horizon (`docs/extras.md`): a port of squeek502's AppleSkin
//! (the Unlicense; its `HUDOverlayHandler`, `TooltipOverlayHandler`,
//! `FoodHelper` and `DebugInfoHudEntry`). What food in hand would do is drawn
//! over the food and health bars, the exhaustion builds up under the food
//! bar, food tooltips show hunger and saturation as icons, and F3 gets the
//! food numbers. The icons and the exhaustion strip are the mod's own
//! `icons.png` (`assets/appleskin`), packed into the HUD atlas; the overlays
//! go where this client's HUD layout drew the icons (`BarPositions`), which
//! is where the mod's own copy of vanilla's jitter puts them.
use crate::hud_layout::{BarPositions, Blend, GuiDraw, PoseOp, argb, white};
use crate::text::Text;
use std::borrow::Cow;

/// The mod's icon sheet and tooltip outline in the HUD atlas.
pub const ICONS: &str = "appleskin/icons";
pub const OUTLINE: &str = "appleskin/tooltip_hunger_outline";
/// The sheet is 256 by 256.
const SHEET: i32 = 256;
/// `FoodHelper.MAX_EXHAUSTION`.
pub const MAX_EXHAUSTION: f32 = 4.0;
/// `FoodHelper.REGEN_EXHAUSTION_INCREMENT`.
const REGEN_EXHAUSTION_INCREMENT: f32 = 6.0;
/// `maxHudOverlayFlashAlpha`'s default.
pub const MAX_FLASH: f32 = 0.65;
/// The food bar's width in pixels (ten icons, eight apart, the last nine wide).
pub const BAR_WIDTH: i32 = 81;

/// What the player may switch off (`ModConfig`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// `showFoodValuesHudOverlay`: the hunger (and saturation) food in hand restores.
    pub restored: bool,
    /// `showSaturationHudOverlay`: the golden saturation on the food icons.
    pub saturation: bool,
    /// `showFoodExhaustionHudUnderlay`: the exhaustion strip behind the food bar.
    pub exhaustion: bool,
    /// `showFoodHealthHudOverlay`: the hearts food would regenerate.
    pub health: bool,
    /// `showFoodValuesHudOverlayWhenOffhand`.
    pub offhand: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            restored: true,
            saturation: true,
            exhaustion: true,
            health: true,
            offhand: true,
        }
    }
}

/// A food item as the overlays see it (`FoodProperties` and `Consumable`).
#[derive(Clone, Debug, PartialEq)]
pub struct Meal {
    pub nutrition: i32,
    pub saturation: f32,
    /// A harmful effect among what eating applies (`FoodHelper.isRotten`):
    /// the icons are the green hunger ones.
    pub rotten: bool,
    /// The (amplifier, duration in ticks) of the Regeneration each group of
    /// applied effects gives, for the health estimate.
    pub regeneration: Vec<(i32, i32)>,
}

/// The pulse of the previews: `HUDOverlayHandler.onClientTick`'s triangle
/// wave, a step of 1/8 a client tick between -0.5 and 1.5, clamped to 0..1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flash {
    unclamped: f32,
    direction: f32,
}
impl Default for Flash {
    fn default() -> Self {
        Self {
            unclamped: 0.0,
            direction: 1.0,
        }
    }
}
impl Flash {
    /// One client tick.
    pub fn tick(&mut self) {
        self.unclamped += self.direction * 0.125;
        if self.unclamped >= 1.5 {
            self.direction = -1.0;
        } else if self.unclamped <= -0.5 {
            self.direction = 1.0;
        }
    }
    /// `resetFlash`: the pulse starts again from nothing.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    /// The alpha of the previews, 0 to `max`.
    pub fn alpha(&self, max: f32) -> f32 {
        self.unclamped.clamp(0.0, 1.0) * max.clamp(0.0, 1.0)
    }
}

/// The player's state the overlays are drawn from.
#[derive(Clone, Copy, Debug)]
pub struct Inputs {
    pub food: i32,
    pub saturation: f32,
    pub exhaustion: f32,
    pub health: f32,
    pub max_health: f32,
    pub hardcore: bool,
    /// The difficulty is Peaceful (health comes back at once there).
    pub peaceful: bool,
    pub poisoned: bool,
    pub withered: bool,
    pub regenerating: bool,
    /// The `naturalRegeneration` game rule (`ClientSyncHandler.naturalRegeneration`).
    pub natural_regeneration: bool,
}

/// `FoodHelper.getEstimatedHealthIncrement(foodLevel, saturation, exhaustion)`:
/// the health natural regeneration gives while the food lasts.
pub fn estimated_health_increment(
    mut food: i32,
    mut saturation: f32,
    mut exhaustion: f32,
) -> f32 {
    let mut health = 0.0f32;
    if !exhaustion.is_finite() || !saturation.is_finite() {
        return 0.0;
    }
    while food >= 18 {
        while exhaustion > MAX_EXHAUSTION {
            exhaustion -= MAX_EXHAUSTION;
            if saturation > 0.0 {
                saturation = (saturation - 1.0).max(0.0);
            } else {
                food -= 1;
            }
        }
        // `Float.compare(saturation, Float.MIN_NORMAL) > 0`.
        if food >= 20 && saturation > f32::MIN_POSITIVE {
            // Fast regeneration, as many ticks as it takes to pass the
            // exhaustion limit at once.
            let limited = saturation.min(REGEN_EXHAUSTION_INCREMENT);
            let until_above_max = MAX_EXHAUSTION.next_up() - exhaustion;
            let iterations = ((until_above_max / limited).ceil() as i32).max(1);
            health += (limited / REGEN_EXHAUSTION_INCREMENT) * iterations as f32;
            exhaustion += limited * iterations as f32;
        } else if food >= 18 {
            // Slow regeneration.
            health += 1.0;
            exhaustion += REGEN_EXHAUSTION_INCREMENT;
        }
    }
    health
}

/// `FoodHelper.getEstimatedHealthIncrement(player, food)`: what eating `meal`
/// would regenerate, naturally and through its Regeneration.
pub fn meal_health(i: &Inputs, meal: &Meal) -> f32 {
    if !(i.health > 0.0 && i.health < i.max_health) {
        return 0.0;
    }
    let food = (i.food + meal.nutrition).min(20);
    let mut increment = 0.0;
    if food >= 18 && i.natural_regeneration {
        let saturation = (i.saturation + meal.saturation).min(food as f32);
        increment = estimated_health_increment(food, saturation, i.exhaustion);
    }
    // Regeneration: a heal every `50 >> amplifier` ticks.
    for &(amplifier, duration) in &meal.regeneration {
        let interval = (50i32.checked_shr(amplifier.max(0) as u32).unwrap_or(0)).max(1);
        increment += (duration / interval) as f32;
    }
    increment
}

fn ceil(x: f32) -> i32 {
    x.ceil() as i32
}

/// `blit(GUI_TEXTURED, texture, x, y, u, v, w, h, 256, 256, color)`.
fn blit(x: i32, y: i32, u: i32, v: i32, w: i32, h: i32, color: u32) -> GuiDraw {
    GuiDraw::Blit {
        texture: Cow::Borrowed(ICONS),
        blend: Blend::Textured,
        x,
        y,
        w,
        h,
        uv: [u as f32, v as f32],
        src: [w, h],
        tex: [SHEET, SHEET],
        color,
    }
}

fn sprite(name: &'static str, x: i32, y: i32, color: u32) -> GuiDraw {
    GuiDraw::Sprite {
        sprite: Cow::Borrowed(name),
        x,
        y,
        w: 9,
        h: 9,
        region: None,
        color,
        blend: Blend::Textured,
    }
}

/// `drawExhaustionOverlay`: the strip of the sheet at (81 - width, 18),
/// growing from the bar's right end, 75% opaque, under the food icons.
pub fn exhaustion_underlay(bars: &BarPositions, exhaustion: f32) -> Vec<GuiDraw> {
    if bars.food.is_empty() {
        return Vec::new();
    }
    let (right, top) = bars.food_base;
    let ratio = (exhaustion / MAX_EXHAUSTION).clamp(0.0, 1.0);
    let width = (ratio * BAR_WIDTH as f32) as i32;
    vec![blit(
        right - width,
        top,
        BAR_WIDTH - width,
        18,
        width,
        9,
        white(0.75),
    )]
}

/// `drawSaturationOverlay`: the golden icons of the sheet over the food
/// bar. `gained` is saturation to show beyond `level`.
pub fn saturation_overlay(
    bars: &BarPositions,
    gained: f32,
    level: f32,
    alpha: f32,
) -> Vec<GuiDraw> {
    if level + gained < 0.0 {
        return Vec::new();
    }
    let color = white(alpha);
    let modified = (level + gained).min(20.0).max(0.0);
    let mut start = 0;
    let end = ceil(modified / 2.0);
    // The gained part starts where the current saturation stops.
    if gained != 0.0 {
        start = (level / 2.0).max(0.0) as i32;
    }
    let mut out = Vec::new();
    for i in start..end {
        let Some(&(x, y)) = bars.food.get(i as usize) else {
            continue;
        };
        let effective = modified / 2.0 - i as f32;
        let u = if effective >= 1.0 {
            27
        } else if effective > 0.5 {
            18
        } else if effective > 0.25 {
            9
        } else {
            0
        };
        out.push(blit(x, y, u, 0, 9, 9, color));
    }
    out
}

/// `drawHungerOverlay`: the food icons eating would fill, over a faint
/// background, flashing.
pub fn hunger_overlay(
    bars: &BarPositions,
    restored: i32,
    food: i32,
    rotten: bool,
    alpha: f32,
) -> Vec<GuiDraw> {
    if restored <= 0 {
        return Vec::new();
    }
    let (empty, half, full) = if rotten {
        (
            "hud/food_empty_hunger",
            "hud/food_half_hunger",
            "hud/food_full_hunger",
        )
    } else {
        ("hud/food_empty", "hud/food_half", "hud/food_full")
    };
    let modified = (food + restored).clamp(0, 20);
    let start = (food / 2).max(0);
    let end = ceil(modified as f32 / 2.0);
    let mut out = Vec::new();
    for i in start..end {
        let Some(&(x, y)) = bars.food.get(i as usize) else {
            continue;
        };
        out.push(sprite(empty, x, y, white(alpha * 0.25)));
        let is_half = i * 2 + 1 == modified;
        out.push(sprite(if is_half { half } else { full }, x, y, white(alpha)));
    }
    out
}

/// `drawHealthOverlay`: the hearts food would regenerate, flashing.
pub fn health_overlay(
    bars: &BarPositions,
    health: f32,
    modified_health: f32,
    hardcore: bool,
    alpha: f32,
) -> Vec<GuiDraw> {
    if modified_health <= health {
        return Vec::new();
    }
    let (container, half, full) = if hardcore {
        (
            "hud/heart/container_hardcore",
            "hud/heart/hardcore_half",
            "hud/heart/hardcore_full",
        )
    } else {
        ("hud/heart/container", "hud/heart/half", "hud/heart/full")
    };
    let fixed = ceil(modified_health);
    let start = ((ceil(health) as f32 / 2.0).max(0.0)) as i32;
    let end = ((modified_health / 2.0).ceil().max(0.0)) as i32;
    let mut out = Vec::new();
    for i in start..end {
        let Some(&(x, y)) = bars.hearts.get(i as usize) else {
            continue;
        };
        out.push(sprite(container, x, y, white(alpha * 0.25)));
        let is_half = i * 2 + 1 == fixed;
        out.push(sprite(if is_half { half } else { full }, x, y, white(alpha)));
    }
    out
}

/// Everything drawn over the bars for this frame, plus the exhaustion strip
/// that goes before the food icons.
#[derive(Debug, Default, PartialEq)]
pub struct Overlay {
    pub under: Vec<GuiDraw>,
    pub over: Vec<GuiDraw>,
    /// Whether food is in hand and could be eaten: the pulse runs only then.
    pub holding_food: bool,
}

/// The overlays of one frame (`onPreRenderFood`, `onRenderFood` and
/// `onRenderHealth`). `meal` is the food in hand the player could eat, if
/// any; `alpha` is the pulse's (`Flash::alpha`).
pub fn overlay(
    o: &Options,
    i: &Inputs,
    bars: &BarPositions,
    meal: Option<&Meal>,
    alpha: f32,
) -> Overlay {
    let mut out = Overlay::default();
    if o.exhaustion && !bars.food.is_empty() {
        out.under = exhaustion_underlay(bars, i.exhaustion);
    }
    let any = o.restored || o.saturation || o.health;
    if !any {
        return out;
    }
    if !bars.food.is_empty() && o.saturation {
        out.over
            .extend(saturation_overlay(bars, 0.0, i.saturation, 1.0));
    }
    let Some(meal) = meal else {
        return out;
    };
    out.holding_food = true;
    if !bars.food.is_empty() && o.restored {
        out.over
            .extend(hunger_overlay(bars, meal.nutrition, i.food, meal.rotten, alpha));
        let new_food = i.food + meal.nutrition;
        let new_saturation = i.saturation + meal.saturation;
        if o.saturation {
            let gained = if new_saturation > new_food as f32 {
                new_food as f32 - i.saturation
            } else {
                meal.saturation
            };
            out.over
                .extend(saturation_overlay(bars, gained, i.saturation, alpha));
        }
    }
    if o.health && should_show_estimated_health(i, bars) {
        let increment = meal_health(i, meal);
        let modified = (i.health + increment).min(i.max_health);
        if i.health < modified {
            out.over
                .extend(health_overlay(bars, i.health, modified, i.hardcore, alpha));
        }
    }
    out
}

/// `shouldShowEstimatedHealth`.
fn should_show_estimated_health(i: &Inputs, bars: &BarPositions) -> bool {
    // No hearts to draw on (infinite health): nothing.
    if bars.hearts.is_empty() || i.peaceful {
        return false;
    }
    // Food that heals right away has nothing to show beside the heal.
    if i.food >= 18 {
        return false;
    }
    !(i.poisoned || i.withered || i.regenerating)
}

/// Whether a status effect is harmful (`MobEffectCategory.HARMFUL`), for the
/// food that makes the hunger icons green.
pub fn harmful_effect(id: &str) -> bool {
    matches!(
        id.strip_prefix("minecraft:").unwrap_or(id),
        "slowness"
            | "mining_fatigue"
            | "instant_damage"
            | "nausea"
            | "blindness"
            | "hunger"
            | "weakness"
            | "poison"
            | "wither"
            | "levitation"
            | "unluck"
            | "darkness"
            | "bad_omen"
            | "raid_omen"
            | "trial_omen"
            | "infested"
            | "oozing"
            | "weaving"
    )
}

/// How an icon of the tooltip's hunger row is outlined
/// (`TooltipOverlayHandler.FoodOutline`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outline {
    Negative,
    Extra,
    Normal,
    Partial,
    Missing,
}
impl Outline {
    fn get(modified: i32, default: i32, i: i32) -> Self {
        if modified < 0 {
            Self::Negative
        } else if modified > default && default <= i {
            Self::Extra
        } else if modified > i + 1 || default == modified {
            Self::Normal
        } else if modified == i + 1 {
            Self::Partial
        } else {
            Self::Missing
        }
    }
    fn argb(self) -> u32 {
        let c = |r: f32, g: f32, b: f32, a: f32| {
            let f = |v: f32| (v * 255.0).floor() as u32;
            argb(f(a), f(r), f(g), f(b))
        };
        match self {
            Self::Negative => c(1.0, 1.0, 1.0, 1.0),
            Self::Extra => c(0.06, 0.32, 0.02, 1.0),
            Self::Normal => c(0.0, 0.0, 0.0, 1.0),
            Self::Partial => c(0.53, 0.21, 0.08, 1.0),
            Self::Missing => c(0.62, 0.0, 0.0, 0.5),
        }
    }
}

/// The food row of an item tooltip (`TooltipOverlayHandler.FoodOverlay`):
/// hunger icons, and under them saturation icons.
#[derive(Clone, Debug, PartialEq)]
pub struct TooltipRow {
    default_hunger: i32,
    modified_hunger: i32,
    modified_saturation: f32,
    rotten: bool,
    hunger_bars: i32,
    hunger_text: Option<String>,
    saturation_bars: i32,
    saturation_text: Option<String>,
}
impl TooltipRow {
    /// The row for an item whose default values are (`default_hunger`,
    /// `default_saturation`) and whose modified ones are the `modified_*`
    /// (mods change them; here they are the same). `None` when there is no
    /// hunger to show.
    pub fn new(
        default_hunger: i32,
        default_saturation: f32,
        modified_hunger: i32,
        modified_saturation: f32,
        rotten: bool,
    ) -> Option<Self> {
        let biggest_hunger = default_hunger.max(modified_hunger);
        let biggest_saturation = default_saturation.max(modified_saturation);
        let mut hunger_bars = (biggest_hunger.abs() as f32 / 2.0).ceil() as i32;
        let mut hunger_text = None;
        if hunger_bars > 10 {
            hunger_text = Some(format!(
                "x{}",
                if biggest_hunger < 0 { -1 } else { 1 } * hunger_bars
            ));
            hunger_bars = 1;
        }
        let mut saturation_bars = (biggest_saturation.abs() / 2.0).ceil() as i32;
        let mut saturation_text = None;
        if saturation_bars > 10 || saturation_bars == 0 {
            saturation_text = Some(format!(
                "x{}",
                if biggest_saturation < 0.0 { -1 } else { 1 } * saturation_bars
            ));
            saturation_bars = 1;
        }
        (hunger_bars > 0).then_some(Self {
            default_hunger,
            modified_hunger,
            modified_saturation,
            rotten,
            hunger_bars,
            hunger_text,
            saturation_bars,
            saturation_text,
        })
    }
    /// The text of the tooltip line that stands for this row (it starts with
    /// U+0001); `None` when the food gives no hunger.
    pub fn marker(nutrition: i32, saturation: f32, rotten: bool) -> Option<String> {
        Self::new(nutrition, saturation, nutrition, saturation, rotten)?;
        Some(format!(
            "\u{1}{nutrition}:{}:{}",
            saturation.to_bits(),
            u8::from(rotten)
        ))
    }
    /// The row a marker line stands for.
    pub fn from_marker(line: &str) -> Option<Self> {
        let mut parts = line.strip_prefix('\u{1}')?.split(':');
        let nutrition: i32 = parts.next()?.parse().ok()?;
        let saturation = f32::from_bits(parts.next()?.parse().ok()?);
        let rotten = parts.next()? == "1";
        Self::new(nutrition, saturation, nutrition, saturation, rotten)
    }
    /// `getHeight`: hunger, a pixel, saturation, and the three that look best.
    pub fn height(&self) -> i32 {
        9 + 1 + 7 + 3
    }
    /// `getWidth`; `text` measures a string in the tooltip's font.
    pub fn width(&self, text: &dyn Fn(&str) -> f32) -> f32 {
        let hunger = self.hunger_bars as f32 * 9.0
            + self.hunger_text.as_deref().map_or(0.0, text);
        let saturation = self.saturation_bars as f32 * 7.0
            + self.saturation_text.as_deref().map_or(0.0, text);
        hunger.max(saturation)
    }
    /// `onRenderTooltip` with the row's top left at (`x0`, `y0`).
    pub fn draws(&self, x0: i32, y0: i32) -> Vec<GuiDraw> {
        let mut out = Vec::new();
        let (empty, half, full) = if self.rotten {
            (
                "hud/food_empty",
                "hud/food_half_hunger",
                "hud/food_full_hunger",
            )
        } else {
            ("hud/food_empty", "hud/food_half", "hud/food_full")
        };
        // Right to left, so that the icons face the right way.
        let mut x = x0 + (self.hunger_bars - 1) * 9;
        let y = y0;
        let mut i = 0;
        while i < self.hunger_bars * 2 {
            out.push(sprite(empty, x, y, u32::MAX));
            let outline = Outline::get(self.modified_hunger, self.default_hunger, i);
            if outline != Outline::Normal {
                out.push(GuiDraw::Sprite {
                    sprite: Cow::Borrowed(OUTLINE),
                    x,
                    y,
                    w: 9,
                    h: 9,
                    region: None,
                    color: outline.argb(),
                    blend: Blend::Textured,
                });
            }
            let default_half = self.default_hunger - 1 == i;
            out.push(sprite(
                if default_half { half } else { full },
                x,
                y,
                white(0.25),
            ));
            if self.modified_hunger > i {
                let modified_half = self.modified_hunger - 1 == i;
                out.push(sprite(
                    if modified_half { half } else { full },
                    x,
                    y,
                    u32::MAX,
                ));
            }
            x -= 9;
            i += 2;
        }
        if let Some(text) = &self.hunger_text {
            x += 18;
            out.push(GuiDraw::Push(vec![
                PoseOp::Translate(x as f32, y as f32),
                PoseOp::Scale(0.75, 0.75),
            ]));
            out.push(text_draw(text, 2, 2));
            out.push(GuiDraw::Pop);
        }
        let y = y0 + 10;
        let absolute = self.modified_saturation.abs();
        let mut x = x0 + (self.saturation_bars - 1) * 7;
        let mut i = 0;
        while i < self.saturation_bars * 2 {
            let effective = (absolute - i as f32) / 2.0;
            let faded = absolute <= i as f32;
            let color = if faded { white(0.5) } else { u32::MAX };
            let u = if effective >= 1.0 {
                21
            } else if effective > 0.5 {
                14
            } else if effective > 0.25 {
                7
            } else if effective > 0.0 {
                0
            } else {
                28
            };
            let v = if self.modified_saturation >= 0.0 { 27 } else { 34 };
            out.push(blit(x, y, u, v, 7, 7, color));
            x -= 7;
            i += 2;
        }
        if let Some(text) = &self.saturation_text {
            x += 14;
            out.push(GuiDraw::Push(vec![
                PoseOp::Translate(x as f32, y as f32),
                PoseOp::Scale(0.75, 0.75),
            ]));
            out.push(text_draw(text, 2, 1));
            out.push(GuiDraw::Pop);
        }
        out
    }
}

fn text_draw(text: &str, x: i32, y: i32) -> GuiDraw {
    GuiDraw::Text {
        text: Text::literal(text),
        x,
        y,
        color: 0xFFAA_AAAA,
        shadow: false,
    }
}

/// Java's `DecimalFormat("#.##")`: at most two decimals, none when whole.
fn up_to_two(x: f32) -> String {
    let s = format!("{x:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// `DebugInfoHudEntry`: the line AppleSkin adds to F3.
pub fn debug_line(food: i32, saturation: f32, exhaustion: f32) -> String {
    format!(
        "hunger: {food}, sat: {}, exh: {exhaustion:.2}/{}",
        up_to_two(saturation),
        up_to_two(MAX_EXHAUSTION)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud_layout::HeartKind;

    fn bars() -> BarPositions {
        BarPositions {
            food: (0..10).map(|n| (200 - n * 8 - 9, 100)).collect(),
            hearts: (0..10).map(|n| (100 + n * 8, 100)).collect(),
            food_base: (200, 100),
        }
    }
    fn inputs(food: i32, saturation: f32, health: f32) -> Inputs {
        Inputs {
            food,
            saturation,
            exhaustion: 0.0,
            health,
            max_health: 20.0,
            hardcore: false,
            peaceful: false,
            poisoned: false,
            withered: false,
            regenerating: false,
            natural_regeneration: true,
        }
    }
    fn apple() -> Meal {
        Meal {
            nutrition: 4,
            saturation: 2.4,
            rotten: false,
            regeneration: Vec::new(),
        }
    }
    fn blit_at(d: &GuiDraw) -> (i32, i32, [f32; 2], i32) {
        match d {
            GuiDraw::Blit { x, y, uv, w, .. } => (*x, *y, *uv, *w),
            _ => panic!("a blit"),
        }
    }

    #[test]
    fn the_flash_is_the_triangle_wave_of_the_mod() {
        let mut f = Flash::default();
        assert_eq!(f.alpha(MAX_FLASH), 0.0);
        let mut alphas = Vec::new();
        for _ in 0..40 {
            f.tick();
            alphas.push(f.alpha(1.0));
        }
        // Up by an eighth a tick, to the top and past it, then back down.
        assert_eq!(alphas[0], 0.125);
        assert_eq!(alphas[7], 1.0);
        assert_eq!(alphas[11], 1.0); // 1.5 reached, going down from here
        assert!(alphas[16] < 1.0 && alphas[16] > 0.5);
        assert_eq!(alphas[27], 0.0); // -0.5 reached
        assert_eq!(alphas[28], 0.0);
        assert_eq!(alphas[35], 0.5); // going up again
        assert_eq!(alphas[39], 1.0);
        f.reset();
        assert_eq!(f, Flash::default());
        let mut f = Flash::default();
        for _ in 0..8 {
            f.tick();
        }
        assert_eq!(f.alpha(MAX_FLASH), MAX_FLASH);
    }

    #[test]
    fn exhaustion_is_the_sheets_strip_from_the_right() {
        let b = bars();
        let d = exhaustion_underlay(&b, 2.0);
        // Half of 81 pixels: 40, drawn from the sheet's (41, 18).
        assert_eq!(blit_at(&d[0]), (200 - 40, 100, [41.0, 18.0], 40));
        let d = exhaustion_underlay(&b, 99.0);
        assert_eq!(blit_at(&d[0]), (200 - 81, 100, [0.0, 18.0], 81));
        assert_eq!(blit_at(&exhaustion_underlay(&b, 0.0)[0]).3, 0);
    }

    #[test]
    fn saturation_picks_the_sheets_quarter_icons() {
        let b = bars();
        // 5 saturation: two full icons, then one at 0.5: 9 (> 0.25, not > 0.5).
        let d = saturation_overlay(&b, 0.0, 5.0, 1.0);
        let us: Vec<f32> = d.iter().map(|d| blit_at(d).2[0]).collect();
        assert_eq!(us, vec![27.0, 27.0, 9.0]);
        // 5.4 gives the third icon 0.7: the three quarters.
        let d = saturation_overlay(&b, 0.0, 5.4, 1.0);
        assert_eq!(blit_at(&d[2]).2[0], 18.0);
        // The gained part starts where the level stops: 5 -> 8 is icons 2 and 3.
        let d = saturation_overlay(&b, 3.0, 5.0, 0.5);
        let xs: Vec<i32> = d.iter().map(|d| blit_at(d).0).collect();
        assert_eq!(xs, vec![b.food[2].0, b.food[3].0]);
        assert!(saturation_overlay(&b, -9.0, 5.0, 1.0).is_empty());
    }

    #[test]
    fn hunger_fills_the_icons_eating_reaches() {
        let b = bars();
        // 10 food, +4: icons 5 and 6 (food 11..14), the last a full one.
        let d = hunger_overlay(&b, 4, 10, false, 0.5);
        let names: Vec<_> = d
            .iter()
            .map(|d| match d {
                GuiDraw::Sprite { sprite, x, .. } => (sprite.to_string(), *x),
                _ => panic!("a sprite"),
            })
            .collect();
        assert_eq!(names.len(), 4);
        assert_eq!(names[1], ("hud/food_full".into(), b.food[5].0));
        assert_eq!(names[3], ("hud/food_full".into(), b.food[6].0));
        // An odd result ends on a half icon.
        let d = hunger_overlay(&b, 3, 10, true, 1.0);
        match &d[3] {
            GuiDraw::Sprite { sprite, .. } => assert_eq!(sprite, "hud/food_half_hunger"),
            _ => panic!(),
        }
        assert!(hunger_overlay(&b, 0, 10, false, 1.0).is_empty());
        assert!(hunger_overlay(&b, -2, 10, false, 1.0).is_empty());
    }

    #[test]
    fn health_hearts_follow_the_mod() {
        let b = bars();
        // 7 health to 10: icons 3 (start is ceil(7)/2 = 3) to 4.
        let d = health_overlay(&b, 7.0, 10.0, false, 0.5);
        assert_eq!(d.len(), 4);
        match &d[1] {
            GuiDraw::Sprite { sprite, .. } => assert_eq!(sprite, "hud/heart/full"),
            _ => panic!(),
        }
        // 7 to 9 ends on a half heart.
        let d = health_overlay(&b, 7.0, 9.0, false, 0.5);
        match &d[d.len() - 1] {
            GuiDraw::Sprite { sprite, .. } => assert_eq!(sprite, "hud/heart/half"),
            _ => panic!(),
        }
        assert!(health_overlay(&b, 7.0, 7.0, false, 1.0).is_empty());
        let d = health_overlay(&b, 7.0, 9.0, true, 1.0);
        match &d[0] {
            GuiDraw::Sprite { sprite, .. } => assert_eq!(sprite, "hud/heart/container_hardcore"),
            _ => panic!(),
        }
    }

    #[test]
    fn the_health_estimate_is_the_mods_arithmetic() {
        // Food 20, saturation 5, no exhaustion: fast regeneration.
        let h = estimated_health_increment(20, 5.0, 0.0);
        assert!(h > 5.0 && h < 7.0, "{h}");
        // Food 18 gives one heart's point per 6 exhaustion until the food is
        // too low: the loop ends when exhaustion drops food below 18.
        let h = estimated_health_increment(18, 0.0, 0.0);
        assert_eq!(h, 1.0);
        assert_eq!(estimated_health_increment(17, 20.0, 0.0), 0.0);
        assert_eq!(estimated_health_increment(20, f32::NAN, 0.0), 0.0);
        assert_eq!(estimated_health_increment(20, 5.0, f32::INFINITY), 0.0);
    }

    #[test]
    fn a_meals_health_needs_a_hurt_player_and_adds_regeneration() {
        let golden = Meal {
            nutrition: 4,
            saturation: 9.6,
            rotten: false,
            regeneration: vec![(1, 100)],
        };
        // Regeneration II heals every 25 ticks: 100 / 25 = 4.
        let hurt = inputs(10, 0.0, 10.0);
        assert!(meal_health(&hurt, &golden) >= 4.0);
        // A full-health player gets no estimate.
        assert_eq!(meal_health(&inputs(10, 0.0, 20.0), &golden), 0.0);
        // Natural regeneration off: only the effect.
        let mut off = inputs(19, 5.0, 10.0);
        off.natural_regeneration = false;
        assert_eq!(meal_health(&off, &golden), 4.0);
        // Food below 18 after eating: no natural regeneration.
        assert_eq!(meal_health(&inputs(2, 0.0, 10.0), &apple()), 0.0);
    }

    #[test]
    fn the_overlay_follows_the_held_food() {
        let b = bars();
        let i = inputs(10, 3.0, 10.0);
        let nothing = overlay(&Options::default(), &i, &b, None, 0.5);
        assert!(!nothing.holding_food);
        // The current saturation and the exhaustion strip are always there.
        assert_eq!(nothing.under.len(), 1);
        assert_eq!(nothing.over.len(), 2);
        let eating = overlay(&Options::default(), &i, &b, Some(&apple()), 0.5);
        assert!(eating.holding_food);
        assert!(eating.over.len() > nothing.over.len());
        // Switched off: nothing.
        let off = Options {
            restored: false,
            saturation: false,
            exhaustion: false,
            health: false,
            offhand: false,
        };
        let none = overlay(&off, &i, &b, Some(&apple()), 0.5);
        assert!(none.under.is_empty() && none.over.is_empty());
        // At 18 food or more the health estimate is not shown.
        let full = overlay(&Options::default(), &inputs(19, 0.0, 10.0), &b, Some(&apple()), 0.5);
        let hearts = full
            .over
            .iter()
            .filter(|d| matches!(d, GuiDraw::Sprite { sprite, .. } if sprite.starts_with("hud/heart")))
            .count();
        assert_eq!(hearts, 0);
        // Peaceful or poisoned: neither.
        let mut p = inputs(5, 0.0, 10.0);
        p.peaceful = true;
        assert!(!should_show_estimated_health(&p, &b));
        p.peaceful = false;
        p.poisoned = true;
        assert!(!should_show_estimated_health(&p, &b));
        let _ = HeartKind::Normal;
    }

    #[test]
    fn rotten_food_is_food_with_a_harmful_effect() {
        assert!(harmful_effect("minecraft:hunger"));
        assert!(harmful_effect("poison"));
        assert!(!harmful_effect("minecraft:regeneration"));
        assert!(!harmful_effect("minecraft:absorption"));
    }

    #[test]
    fn the_tooltip_row_has_the_mods_size_and_icons() {
        // An apple: 4 hunger, 2.4 saturation: 2 icons, 2 saturation icons...
        let row = TooltipRow::new(4, 2.4, 4, 2.4, false).unwrap();
        let measure = |s: &str| s.len() as f32 * 6.0;
        assert_eq!(row.height(), 20);
        assert_eq!(row.width(&measure), 18.0);
        let d = row.draws(10, 20);
        // Two hunger icons (empty, default, modified each: 3 sprites)...
        let sprites = d.iter().filter(|d| matches!(d, GuiDraw::Sprite { .. })).count();
        assert_eq!(sprites, 6);
        // ...and two saturation blits: 2.4 is one and a fifth icons.
        let blits: Vec<_> = d.iter().filter_map(|d| match d {
            GuiDraw::Blit { .. } => Some(blit_at(d)),
            _ => None,
        }).collect();
        assert_eq!(blits.len(), 2);
        // The right icon first: the second saturation icon is drawn first.
        assert_eq!(blits[0].1, 30);
        assert_eq!(blits[0].2[1], 27.0);
        // Nothing to eat: no row.
        assert!(TooltipRow::new(0, 0.0, 0, 0.0, false).is_none());
        // More than ten bars of hunger collapse to one with a count.
        let big = TooltipRow::new(30, 0.5, 30, 0.5, false).unwrap();
        assert_eq!(big.hunger_text.as_deref(), Some("x15"));
        assert_eq!(big.width(&measure), 9.0 + 18.0);
        // No saturation: the "x0" text.
        let none = TooltipRow::new(4, 0.0, 4, 0.0, false).unwrap();
        assert_eq!(none.saturation_text.as_deref(), Some("x0"));
    }

    #[test]
    fn a_marker_line_stands_for_the_row() {
        let line = TooltipRow::marker(6, 7.2, true).unwrap();
        assert!(line.starts_with('\u{1}'));
        assert_eq!(
            TooltipRow::from_marker(&line),
            TooltipRow::new(6, 7.2, 6, 7.2, true)
        );
        assert!(TooltipRow::marker(0, 1.0, false).is_none());
        assert!(TooltipRow::from_marker("Iron Ingot").is_none());
    }

    #[test]
    fn outlines_tell_what_a_mod_changed() {
        assert_eq!(Outline::get(-2, 4, 0), Outline::Negative);
        assert_eq!(Outline::get(6, 4, 4), Outline::Extra);
        assert_eq!(Outline::get(4, 4, 2), Outline::Normal);
        assert_eq!(Outline::get(3, 6, 2), Outline::Partial);
        assert_eq!(Outline::get(2, 6, 4), Outline::Missing);
    }

    #[test]
    fn the_debug_line_formats_like_java() {
        assert_eq!(debug_line(20, 5.0, 1.234), "hunger: 20, sat: 5, exh: 1.23/4");
        assert_eq!(debug_line(7, 2.5, 0.0), "hunger: 7, sat: 2.5, exh: 0.00/4");
        // Ties go to the even digit, as Java's HALF_EVEN does.
        assert_eq!(up_to_two(0.125), "0.12");
    }
}
