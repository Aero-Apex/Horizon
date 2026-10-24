//! AppleSkin, rewritten for Horizon (`docs/extras.md`). The Java mod (squeek502's
//! AppleSkin, public domain) draws on the food and health bars what food would
//! do: a flashing preview of the hunger it restores, the golden saturation
//! outlines, the exhaustion underlay and the hearts regeneration would give,
//! plus food values in item tooltips and the food numbers on F3. This is that,
//! as draw calls over this client's own HUD layout (`hud_layout::BarPositions`
//! says where the bars drew), so it follows jitter, regeneration wobble and
//! rows exactly.
use crate::hud_layout::{BarPositions, Blend, GuiDraw, HeartKind, argb, ceil, fill, white};
use horizon_world::food::{Difficulty, FoodData, FoodWorld};
use std::borrow::Cow;
use std::convert::Infallible;

/// The food bar's icons, and its width in pixels (10 icons, 8 apart, the last 9 wide).
pub const ICONS: i32 = 10;
pub const BAR_WIDTH: i32 = 81;
/// `FoodData`'s exhaustion limit: at 4 a point of saturation (or hunger) goes.
pub const MAX_EXHAUSTION: f32 = 4.0;
/// How strong the flashing previews get (AppleSkin's `maxHudOverlayFlashAlpha`).
pub const MAX_FLASH: f32 = 0.65;
/// Ticks one flash takes: AppleSkin pulses with `sin(tick * 0.1)`.
const FLASH_SPEED: f32 = 0.1;
/// The simulation's cut-off: ticks of natural regeneration looked ahead.
const MAX_SIM_TICKS: u32 = 20_000;
/// The golden outline of a saturation icon.
const GOLD: (u32, u32, u32) = (0xFF, 0xD8, 0x3A);

/// What the player may switch off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// The flashing hunger and saturation preview for the food in hand.
    pub restored: bool,
    /// The golden saturation outlines on the food icons.
    pub saturation: bool,
    /// The exhaustion bar under the food icons.
    pub exhaustion: bool,
    /// The flashing hearts food would regenerate.
    pub health: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            restored: true,
            saturation: true,
            exhaustion: true,
            health: true,
        }
    }
}

/// What eating the food in hand gives (`FoodProperties`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Meal {
    pub nutrition: i32,
    pub saturation: f32,
}

/// The state the overlays are drawn from.
#[derive(Clone, Copy, Debug)]
pub struct Inputs {
    pub tick: i32,
    pub food: i32,
    pub saturation: f32,
    pub exhaustion: f32,
    pub health: f32,
    pub max_health: f32,
    pub hardcore: bool,
    pub heart_kind: HeartKind,
    pub hunger_effect: bool,
    pub difficulty: Difficulty,
    /// Natural regeneration game rule.
    pub natural_regeneration: bool,
    /// The food in the acting hand, when the player could eat it.
    pub meal: Option<Meal>,
}

/// The preview's pulse, 0 to `MAX_FLASH`: `sin(tick * 0.1)` mapped to 0..1.
pub fn flash(tick: i32) -> f32 {
    ((tick as f32 * FLASH_SPEED).sin() * 0.5 + 0.5) * MAX_FLASH
}

/// `FoodData.eat` for a meal: what the bar would hold after eating.
pub fn after_eating(food: i32, saturation: f32, meal: Meal) -> (i32, f32) {
    let mut data = FoodData {
        food,
        saturation,
        exhaustion: 0.0,
        timer: 0,
    };
    data.eat_properties(meal.nutrition, meal.saturation);
    (data.food, data.saturation)
}

/// A pretend world for `FoodData::tick`: only health and the game rules.
struct Sim {
    health: f32,
    max_health: f32,
    difficulty: Difficulty,
    regenerate: bool,
}
impl FoodWorld for Sim {
    type Error = Infallible;
    type Level = ();
    fn level(&mut self, _: &mut FoodData) -> Result<(), Infallible> {
        Ok(())
    }
    fn difficulty(&mut self, _: (), _: &mut FoodData) -> Result<Difficulty, Infallible> {
        Ok(self.difficulty)
    }
    fn natural_regeneration(&mut self, _: (), _: &mut FoodData) -> Result<bool, Infallible> {
        Ok(self.regenerate)
    }
    fn is_hurt(&mut self, _: &mut FoodData) -> Result<bool, Infallible> {
        Ok(self.health > 0.0 && self.health < self.max_health)
    }
    fn heal(&mut self, _: &mut FoodData, amount: f32) -> Result<(), Infallible> {
        self.health = (self.health + amount).min(self.max_health);
        Ok(())
    }
    fn health(&mut self, _: &mut FoodData) -> Result<f32, Infallible> {
        Ok(self.health)
    }
    fn hurt_starvation(&mut self, _: (), _: &mut FoodData, _: f32) -> Result<(), Infallible> {
        Ok(())
    }
}

/// The health natural regeneration would give from this food state with
/// nothing else spending it: vanilla's own `FoodData.tick`, run until it can
/// no longer heal (AppleSkin's `getEstimatedHealthIncrement`).
pub fn regeneration(i: &Inputs, food: i32, saturation: f32) -> f32 {
    if !i.natural_regeneration || i.health >= i.max_health || i.health <= 0.0 {
        return 0.0;
    }
    let mut sim = Sim {
        health: i.health,
        max_health: i.max_health,
        difficulty: i.difficulty,
        regenerate: i.natural_regeneration,
    };
    let mut data = FoodData {
        food,
        saturation,
        exhaustion: i.exhaustion,
        timer: 0,
    };
    for _ in 0..MAX_SIM_TICKS {
        let before = sim.health;
        let Ok(()) = data.tick(&mut sim);
        // Below 18 food nothing heals (full food with saturation is above that).
        let can_heal = data.food >= 18;
        if sim.health >= sim.max_health || (!can_heal && sim.health == before) {
            break;
        }
    }
    sim.health - i.health
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

/// The exhaustion underlay: a pale strip growing from the bar's right end as
/// exhaustion builds toward 4, drawn under the icons (the caller puts it
/// before the vanilla icons).
pub fn exhaustion_underlay(bars: &BarPositions, exhaustion: f32) -> Vec<GuiDraw> {
    let Some(&(x, y)) = bars.food.first() else {
        return Vec::new();
    };
    let ratio = (exhaustion / MAX_EXHAUSTION).clamp(0.0, 1.0);
    let width = (ratio * BAR_WIDTH as f32) as i32;
    if width <= 0 {
        return Vec::new();
    }
    let right = x + 9;
    vec![fill(right - width, y, right, y + 9, argb(0x80, 0xDC, 0xDC, 0xE6))]
}

/// How much of icon `n` (0 = rightmost) a saturation of `saturation` fills, 0..1.
fn icon_fill(saturation: f32, n: i32) -> f32 {
    (saturation / 2.0 - n as f32).clamp(0.0, 1.0)
}

/// A golden outline round an icon, `amount` of it (0..1) from the bottom up,
/// in quarters as the mod's four saturation sprites are.
fn outline(x: i32, y: i32, amount: f32, color: u32) -> Vec<GuiDraw> {
    let quarters = (amount * 4.0).ceil() as i32;
    if quarters <= 0 {
        return Vec::new();
    }
    // Rows of the 9 tall icon covered, from the bottom.
    let rows = match quarters {
        1 => 3,
        2 => 5,
        3 => 7,
        _ => 9,
    };
    let top = y + 9 - rows;
    let mut out = vec![
        fill(x, y + 8, x + 9, y + 9, color),
        fill(x, top, x + 1, y + 9, color),
        fill(x + 8, top, x + 9, y + 9, color),
    ];
    if rows == 9 {
        out.push(fill(x, y, x + 9, y + 1, color));
    }
    out
}

/// The golden saturation outlines for `saturation`, drawn over the icons.
/// `from` skips what that much saturation already covers (the preview shows
/// only the gain).
pub fn saturation_overlay(
    bars: &BarPositions,
    saturation: f32,
    from: f32,
    alpha: f32,
) -> Vec<GuiDraw> {
    let color = argb(
        (alpha.clamp(0.0, 1.0) * 255.0) as u32,
        GOLD.0,
        GOLD.1,
        GOLD.2,
    );
    let mut out = Vec::new();
    for (n, &(x, y)) in bars.food.iter().enumerate() {
        let now = icon_fill(saturation, n as i32);
        if now <= icon_fill(from, n as i32) {
            continue;
        }
        out.extend(outline(x, y, now, color));
    }
    out
}

/// The food icons eating would fill, drawn flashing over the bar.
pub fn restored_hunger(
    bars: &BarPositions,
    food: i32,
    after: i32,
    hunger_effect: bool,
    alpha: f32,
) -> Vec<GuiDraw> {
    let (empty, half, full) = if hunger_effect {
        (
            "hud/food_empty_hunger",
            "hud/food_half_hunger",
            "hud/food_full_hunger",
        )
    } else {
        ("hud/food_empty", "hud/food_half", "hud/food_full")
    };
    let mut out = Vec::new();
    for (n, &(x, y)) in bars.food.iter().enumerate() {
        let n = n as i32;
        // Icon n shows hunger points 2n+1 and 2n+2: skipped when eating
        // does not reach it, or the bar already fills it.
        if n * 2 + 1 > after || n * 2 + 2 <= food {
            continue;
        }
        // A faint background so the preview reads over a full-looking bar.
        out.push(sprite(empty, x, y, white(alpha * 0.25)));
        // As vanilla's: full when both points are reached, half on one.
        let name = if n * 2 + 1 < after { full } else { half };
        out.push(sprite(name, x, y, white(alpha)));
    }
    out
}

/// The hearts regeneration would give, flashing over the health bar:
/// `gain` health points above the current ones.
pub fn health_preview(
    bars: &BarPositions,
    health: f32,
    gain: f32,
    kind: HeartKind,
    hardcore: bool,
    alpha: f32,
) -> Vec<GuiDraw> {
    let current = ceil(health);
    let target = ceil((health + gain).max(health));
    let mut out = Vec::new();
    if target <= current {
        return out;
    }
    for (c, &(x, y)) in bars.hearts.iter().enumerate() {
        let halves = c as i32 * 2;
        if halves + 1 > target || halves + 2 <= current {
            continue;
        }
        let name = heart(kind, hardcore, halves + 1 == target);
        out.push(sprite(name, x, y, white(alpha)));
    }
    out
}

fn heart(kind: HeartKind, hardcore: bool, half: bool) -> &'static str {
    // The same sprites `hud_layout` draws: [normal, hardcore] x [full, half].
    let table: [&str; 4] = match kind {
        HeartKind::Normal => [
            "hud/heart/full",
            "hud/heart/half",
            "hud/heart/hardcore_full",
            "hud/heart/hardcore_half",
        ],
        HeartKind::Poisoned => [
            "hud/heart/poisoned_full",
            "hud/heart/poisoned_half",
            "hud/heart/poisoned_hardcore_full",
            "hud/heart/poisoned_hardcore_half",
        ],
        HeartKind::Withered => [
            "hud/heart/withered_full",
            "hud/heart/withered_half",
            "hud/heart/withered_hardcore_full",
            "hud/heart/withered_hardcore_half",
        ],
        HeartKind::Frozen => [
            "hud/heart/frozen_full",
            "hud/heart/frozen_half",
            "hud/heart/frozen_hardcore_full",
            "hud/heart/frozen_hardcore_half",
        ],
    };
    table[usize::from(hardcore) * 2 + usize::from(half)]
}

/// Everything drawn over the bars for this frame, plus the exhaustion
/// underlay that goes before the vanilla icons.
#[derive(Debug, Default, PartialEq)]
pub struct Overlay {
    pub under: Vec<GuiDraw>,
    pub over: Vec<GuiDraw>,
}

/// The overlays for one frame. `gain` is the heart estimate for the meal in
/// hand (`regeneration` of the food state after eating minus before), or
/// `None` when it is not known yet.
pub fn overlay(o: &Options, i: &Inputs, bars: &BarPositions, gain: Option<f32>) -> Overlay {
    let mut out = Overlay::default();
    if bars.food.is_empty() {
        // A mount's hearts replace the food bar.
        return out;
    }
    if o.exhaustion {
        out.under = exhaustion_underlay(bars, i.exhaustion);
    }
    if o.saturation {
        out.over
            .extend(saturation_overlay(bars, i.saturation, 0.0, 1.0));
    }
    let alpha = flash(i.tick);
    if let Some(meal) = i.meal {
        let (after, after_sat) = after_eating(i.food, i.saturation, meal);
        if o.restored {
            out.over
                .extend(restored_hunger(bars, i.food, after, i.hunger_effect, alpha));
            if o.saturation {
                out.over
                    .extend(saturation_overlay(bars, after_sat, i.saturation, alpha));
            }
        }
        if o.health
            && let Some(gain) = gain.filter(|g| *g > 0.0)
        {
            out.over.extend(health_preview(
                bars,
                i.health,
                gain,
                i.heart_kind,
                i.hardcore,
                alpha,
            ));
        }
    }
    out
}

/// The health food would regenerate, beyond what the current state does.
pub fn meal_gain(i: &Inputs) -> Option<f32> {
    let meal = i.meal?;
    let (after, after_sat) = after_eating(i.food, i.saturation, meal);
    Some(regeneration(i, after, after_sat) - regeneration(i, i.food, i.saturation))
}

/// What `meal_gain` last worked out and for which state, so the simulation
/// runs when the food state changes (20 times a second at most), not per frame.
#[derive(Debug, Default)]
pub struct GainCache {
    key: Option<[u32; 9]>,
    value: Option<f32>,
}
impl GainCache {
    pub fn gain(&mut self, i: &Inputs) -> Option<f32> {
        let meal = i.meal?;
        let key = [
            i.food as u32,
            i.saturation.to_bits(),
            i.exhaustion.to_bits(),
            i.health.to_bits(),
            i.max_health.to_bits(),
            meal.nutrition as u32,
            meal.saturation.to_bits(),
            i.difficulty as u32,
            u32::from(i.natural_regeneration),
        ];
        if self.key != Some(key) {
            self.value = meal_gain(i);
            self.key = Some(key);
        }
        self.value
    }
}

/// A tooltip's food lines: hunger and saturation, and what is wasted when the
/// bar cannot hold it all. `(text, 0xRRGGBB)` each.
pub fn tooltip_lines(meal: Meal, food: i32) -> Vec<(String, u32)> {
    let mut lines = vec![
        (
            format!("Hunger: +{}", meal.nutrition),
            0xE8A23A,
        ),
        (
            format!("Saturation: +{:.1}", meal.saturation),
            0xFFD83A,
        ),
    ];
    let room = (20 - food).max(0);
    if meal.nutrition > room {
        lines.push((
            format!("Wastes {} hunger when eaten now", meal.nutrition - room),
            0xFF8A3A,
        ));
    }
    lines
}

/// The food numbers AppleSkin adds to F3.
pub fn debug_lines(food: i32, saturation: f32, exhaustion: f32) -> Vec<String> {
    vec![format!(
        "[AppleSkin] hunger: {food}  saturation: {saturation:.2}  exhaustion: {exhaustion:.2}/{MAX_EXHAUSTION:.1}"
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars() -> BarPositions {
        BarPositions {
            food: (0..ICONS).map(|n| (200 - n * 8 - 9, 100)).collect(),
            hearts: (0..ICONS).map(|n| (100 + n * 8, 100)).collect(),
        }
    }
    fn inputs(food: i32, saturation: f32, health: f32, meal: Option<Meal>) -> Inputs {
        Inputs {
            tick: 7,
            food,
            saturation,
            exhaustion: 0.0,
            health,
            max_health: 20.0,
            hardcore: false,
            heart_kind: HeartKind::Normal,
            hunger_effect: false,
            difficulty: Difficulty::Normal,
            natural_regeneration: true,
            meal,
        }
    }
    const APPLE: Meal = Meal {
        nutrition: 4,
        saturation: 2.4,
    };

    #[test]
    fn eating_follows_food_data() {
        assert_eq!(after_eating(10, 1.0, APPLE).0, 14);
        let (food, saturation) = after_eating(19, 19.0, APPLE);
        assert_eq!(food, 20);
        assert_eq!(saturation, 20.0);
    }

    #[test]
    fn exhaustion_grows_from_the_right() {
        let b = bars();
        assert!(exhaustion_underlay(&b, 0.0).is_empty());
        let GuiDraw::Fill { x0, x1, .. } = exhaustion_underlay(&b, 2.0)[0] else {
            panic!("a fill");
        };
        // Half of the 81 pixel bar, ending at the rightmost icon's edge.
        assert_eq!(x1, 200);
        assert_eq!(x1 - x0, 40);
        let GuiDraw::Fill { x0, x1, .. } = exhaustion_underlay(&b, 9.0)[0] else {
            panic!("a fill");
        };
        assert_eq!(x1 - x0, BAR_WIDTH);
    }

    #[test]
    fn saturation_outlines_fill_icon_by_icon() {
        let b = bars();
        // 5 saturation is two and a half icons: two full outlines, a half one.
        let full = |n: usize| outline(b.food[n].0, b.food[n].1, 1.0, 0).len();
        assert_eq!(full(0), 4);
        let draws = saturation_overlay(&b, 5.0, 0.0, 1.0);
        let per_icon = [4, 4, 3];
        assert_eq!(draws.len(), per_icon.iter().sum::<usize>());
        // Only the gain is shown when a base is given.
        assert!(saturation_overlay(&b, 5.0, 5.0, 1.0).is_empty());
        // 5 -> 6 completes the third icon's outline.
        assert_eq!(saturation_overlay(&b, 6.0, 5.0, 1.0).len(), 4);
    }

    #[test]
    fn restored_hunger_covers_the_icons_eating_fills() {
        let b = bars();
        // 10 food -> 14: icons 5 and 6 (hunger 11..14).
        let draws = restored_hunger(&b, 10, 14, false, 0.5);
        assert_eq!(draws.len(), 4);
        // An odd gain ends on a half icon.
        let draws = restored_hunger(&b, 10, 13, false, 0.5);
        let GuiDraw::Sprite { sprite, .. } = &draws[3] else {
            panic!("a sprite");
        };
        assert_eq!(sprite, "hud/food_half");
        assert!(restored_hunger(&b, 20, 20, false, 0.5).is_empty());
    }

    #[test]
    fn regeneration_matches_vanilla_rules() {
        // Full health: nothing to regenerate.
        let full = inputs(20, 5.0, 20.0, None);
        assert_eq!(regeneration(&full, 20, 5.0), 0.0);
        // Hungry (food 10) cannot regenerate at all.
        let hungry = inputs(10, 0.0, 10.0, None);
        assert_eq!(regeneration(&hungry, 10, 0.0), 0.0);
        // Full food with saturation heals fast: some health, never past max.
        let healing = inputs(20, 6.0, 10.0, None);
        let gain = regeneration(&healing, 20, 6.0);
        assert!(gain > 0.0 && gain <= 10.0);
        // Eating an apple at 17 food (below the 18 needed) starts regeneration.
        let i = inputs(
            17,
            0.0,
            10.0,
            Some(Meal {
                nutrition: 4,
                saturation: 2.4,
            }),
        );
        assert_eq!(regeneration(&i, 17, 0.0), 0.0);
        assert!(meal_gain(&i).unwrap() > 0.0);
        // The game rule off: no regeneration either way.
        let mut off = i;
        off.natural_regeneration = false;
        assert_eq!(meal_gain(&off).unwrap(), 0.0);
    }

    #[test]
    fn overlay_respects_the_switches_and_the_mount() {
        let b = bars();
        let i = inputs(10, 3.0, 10.0, Some(APPLE));
        let all = overlay(&Options::default(), &i, &b, Some(2.0));
        assert!(!all.under.is_empty() || i.exhaustion == 0.0);
        assert!(!all.over.is_empty());
        let none = overlay(
            &Options {
                restored: false,
                saturation: false,
                exhaustion: false,
                health: false,
            },
            &i,
            &b,
            Some(2.0),
        );
        assert_eq!(none, Overlay::default());
        // No food icons (a ridden mount shows its hearts): nothing to overlay.
        let mounted = BarPositions {
            food: Vec::new(),
            hearts: b.hearts.clone(),
        };
        assert_eq!(
            overlay(&Options::default(), &i, &mounted, Some(2.0)),
            Overlay::default()
        );
    }

    #[test]
    fn health_preview_covers_the_gained_halves() {
        let b = bars();
        // 7 health -> 10: heart 3 goes from half to full and heart 4 fills.
        let draws = health_preview(&b, 7.0, 3.0, HeartKind::Normal, false, 0.5);
        assert_eq!(draws.len(), 2);
        // 7 -> 9 ends on a half heart.
        let draws = health_preview(&b, 7.0, 2.0, HeartKind::Normal, false, 0.5);
        let GuiDraw::Sprite { sprite, .. } = &draws[1] else {
            panic!("a sprite");
        };
        assert_eq!(sprite, "hud/heart/half");
        assert!(health_preview(&b, 7.0, 0.0, HeartKind::Normal, false, 0.5).is_empty());
    }

    #[test]
    fn tooltip_notes_waste() {
        let lines = tooltip_lines(APPLE, 10);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, "Hunger: +4");
        assert_eq!(lines[1].0, "Saturation: +2.4");
        let lines = tooltip_lines(APPLE, 18);
        assert_eq!(lines.len(), 3);
        assert!(lines[2].0.contains("Wastes 2"));
    }

    #[test]
    fn the_flash_stays_in_range() {
        for t in 0..200 {
            let a = flash(t);
            assert!((0.0..=MAX_FLASH).contains(&a));
        }
    }
}
