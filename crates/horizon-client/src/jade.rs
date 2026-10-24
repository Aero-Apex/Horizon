//! Jade, rewritten for Horizon (`docs/extras.md`). The Java mod (Snownee's Jade)
//! shows a box at the top of the screen for what the crosshair is on: the block
//! or mob, its icon, health, harvest tool and the state that matters (crop
//! growth, redstone power...). This holds the decisions: names, the lines
//! under them and the heart counts; `game/jade.rs` gathers the target and draws.

pub const WHITE: u32 = 0xFFFFFF;
pub const GRAY: u32 = 0xAAAAAA;
pub const GREEN: u32 = 0x55FF55;
pub const RED: u32 = 0xFF5555;
pub const YELLOW: u32 = 0xFFFF55;
pub const ORANGE: u32 = 0xFFAA00;
/// The mod name line's blue, Jade's own.
pub const MOD_BLUE: u32 = 0x5555FF;

/// One line of text and its `0xRRGGBB` colour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub color: u32,
}
impl Line {
    fn new(text: impl Into<String>, color: u32) -> Self {
        Self {
            text: text.into(),
            color,
        }
    }
}

/// `oak_log`, or `minecraft:oak_log`, as `Oak Log`.
pub fn title_case(id: &str) -> String {
    let path = id.split_once(':').map_or(id, |(_, p)| p);
    path.split(['_', '/'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The line under the name: whose block or mob it is (`minecraft` is
/// `Minecraft`, a mod's namespace its title-cased name).
pub fn namespace_name(id: &str) -> String {
    let namespace = id.split_once(':').map_or("minecraft", |(n, _)| n);
    title_case(namespace)
}

/// `minecraft:oak_stairs[facing=north,half=bottom]` as the block id and its
/// state properties.
pub fn split_state(state: &str) -> (String, Vec<(String, String)>) {
    let Some((id, rest)) = state.split_once('[') else {
        return (state.to_owned(), Vec::new());
    };
    let properties = rest
        .trim_end_matches(']')
        .split(',')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    (id.to_owned(), properties)
}

/// A crop's last growth stage (`age` runs 0 to this).
pub fn crop_max_age(block: &str) -> Option<u32> {
    Some(match block.strip_prefix("minecraft:").unwrap_or(block) {
        "wheat" | "carrots" | "potatoes" | "pumpkin_stem" | "melon_stem" => 7,
        "beetroots" | "nether_wart" | "sweet_berry_bush" => 3,
        "cocoa" => 2,
        "pitcher_crop" => 4,
        "torchflower_crop" => 1,
        _ => return None,
    })
}

fn property<'a>(props: &'a [(String, String)], key: &str) -> Option<&'a str> {
    props
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}
fn number(props: &[(String, String)], key: &str) -> Option<u32> {
    property(props, key)?.parse().ok()
}

/// The lines for a block's state that matter to a player.
pub fn details(block: &str, props: &[(String, String)]) -> Vec<Line> {
    let name = block.strip_prefix("minecraft:").unwrap_or(block);
    let mut lines = Vec::new();
    if let (Some(max), Some(age)) = (crop_max_age(block), number(props, "age")) {
        let percent = (age * 100 / max).min(100);
        lines.push(if age >= max {
            Line::new("Growth: 100% (ripe)", GREEN)
        } else {
            Line::new(format!("Growth: {percent}%"), YELLOW)
        });
    }
    match name {
        "redstone_wire" | "daylight_detector" | "inverted_daylight_detector" => {
            if let Some(power) = number(props, "power") {
                lines.push(Line::new(format!("Power: {power}"), RED));
            }
        }
        "repeater" => {
            if let Some(delay) = number(props, "delay") {
                lines.push(Line::new(format!("Delay: {delay}"), RED));
            }
        }
        "comparator" => {
            if let Some(mode) = property(props, "mode") {
                lines.push(Line::new(format!("Mode: {mode}"), RED));
            }
        }
        "note_block" => {
            if let Some(note) = number(props, "note") {
                lines.push(Line::new(format!("Note: {note}"), YELLOW));
            }
            if let Some(instrument) = property(props, "instrument") {
                lines.push(Line::new(
                    format!("Instrument: {}", title_case(instrument)),
                    GRAY,
                ));
            }
        }
        "composter" => {
            if let Some(level) = number(props, "level") {
                lines.push(Line::new(format!("Level: {level}/8"), YELLOW));
            }
        }
        "beehive" | "bee_nest" => {
            if let Some(honey) = number(props, "honey_level") {
                lines.push(Line::new(format!("Honey: {honey}/5"), ORANGE));
            }
        }
        "water_cauldron" | "powder_snow_cauldron" => {
            if let Some(level) = number(props, "level") {
                lines.push(Line::new(format!("Level: {level}/3"), YELLOW));
            }
        }
        "farmland" => {
            if let Some(moisture) = number(props, "moisture") {
                lines.push(Line::new(format!("Moisture: {moisture}/7"), YELLOW));
            }
        }
        _ => {}
    }
    lines
}

/// The harvest line: which tool breaks the block well, and whether the one
/// in hand does. `tool` is `pickaxe`, `axe`, `shovel`, `hoe` or none.
pub fn harvest(tool: Option<&str>, requires_tool: bool, can_harvest: bool) -> Option<Line> {
    let tool_name = tool.map(title_case);
    match (tool_name, requires_tool, can_harvest) {
        // Anything breaks it, and drops: nothing worth a line.
        (None, false, _) => None,
        (Some(tool), false, _) => Some(Line::new(format!("Best tool: {tool}"), GRAY)),
        (tool, true, true) => Some(Line::new(
            format!("Can harvest ({})", tool.unwrap_or_else(|| "tool".into())),
            GREEN,
        )),
        (tool, true, false) => Some(Line::new(
            format!(
                "Needs a {}",
                tool.unwrap_or_else(|| "tool".into()).to_lowercase()
            ),
            RED,
        )),
    }
}

/// Jade's defaults for the health and armor rows (`VanillaPlugin`):
/// `entity_health.max_for_render`, `.icons_per_line`, `entity_armor.max_for_render`.
pub const HEALTH_MAX_FOR_RENDER: i32 = 40;
pub const ICONS_PER_LINE: i32 = 10;
pub const ARMOR_MAX_FOR_RENDER: f32 = 20.0;
/// The theme's normal text colour (`ColorPalette.DEFAULT.normal`).
pub const NORMAL: u32 = 0xFFA0_A0A0;

/// `Mth.ceil`.
fn ceil(x: f32) -> i32 {
    x.ceil() as i32
}

/// `DisplayHelper.dfCommas`: `0.##`, no grouping.
pub fn two_decimals(x: f32) -> String {
    let s = format!("{x:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A row of icons, hearts or armor (`HealthElement`, `ArmorElement`): too
/// much to draw is a single icon and a number.
#[derive(Clone, Debug, PartialEq)]
pub struct IconRow {
    /// The number shown beside one icon, when the value is too big for icons.
    pub text: Option<String>,
    pub icons: i32,
    pub per_line: i32,
    pub lines: i32,
}
impl IconRow {
    /// `HealthElement`'s constructor. `absorption` is extra health shown as
    /// more hearts.
    pub fn health(max_health: f32, health: f32, absorption: f32) -> Self {
        let icon_count = ceil(max_health) + ceil(absorption);
        if icon_count > HEALTH_MAX_FOR_RENDER {
            // Without fractions: whole numbers.
            let max = ceil(max_health) as f32;
            let now = ceil(health + absorption) as f32;
            return Self {
                text: Some(format!("{}/{}", two_decimals(now), two_decimals(max))),
                icons: 1,
                per_line: 1,
                lines: 1,
            };
        }
        let icons = ceil(icon_count as f32 * 0.5);
        Self {
            text: None,
            icons,
            per_line: ICONS_PER_LINE.min(icons),
            lines: ceil(icons as f32 / ICONS_PER_LINE as f32),
        }
    }
    /// `ArmorElement`'s constructor.
    pub fn armor(armor: f32) -> Self {
        if armor > ARMOR_MAX_FOR_RENDER {
            return Self {
                text: Some(two_decimals(ceil(armor) as f32)),
                icons: 1,
                per_line: 1,
                lines: 1,
            };
        }
        let half = armor * 0.5;
        let icons = ceil(half);
        Self {
            text: None,
            icons,
            per_line: ICONS_PER_LINE.min(icons),
            lines: ceil(half / ICONS_PER_LINE as f32),
        }
    }
    /// The element's size: a number is 10 pixels right of its icon and 9
    /// high; icons are 8 apart and 4 rows apart, so overlapping rows.
    pub fn size(&self, text_width: &dyn Fn(&str) -> f32) -> (f32, f32) {
        match &self.text {
            Some(t) => (text_width(t) + 10.0, 9.0),
            None => ((8 * self.per_line + 1) as f32, (5 + 4 * self.lines) as f32),
        }
    }
    /// The icons of a row, back to front (`HealthElement.extractRenderState`):
    /// (x, y, slot) from the element's top left, where `slot` counts from 1.
    pub fn slots(&self) -> Vec<(i32, i32, i32)> {
        let mut out = Vec::new();
        let mut x = (self.icons - 1) % self.per_line.max(1) * 8;
        let mut y = self.lines * 4 - 4;
        for i in (1..=self.icons).rev() {
            out.push((x, y, i));
            x -= 8;
            if x < 0 {
                x = self.per_line * 8 - 8;
                y -= 4;
            }
        }
        out
    }
}

/// Which sprites a heart slot shows (`HealthElement`): the container always,
/// a full heart up to `floor(hearts)`, a half heart for the fraction.
/// `health` is rounded as Jade rounds it.
pub fn heart_sprites(slot: i32, health: f32) -> (bool, bool) {
    let hearts = health.round() * 0.5;
    let full = slot as f32 <= hearts.floor();
    let half = !full && (slot as f32) > hearts && (slot as f32) < hearts + 1.0;
    (full, half)
}

/// The armor sprites of a slot (`ArmorElement`): full to `floor(armor)`,
/// half for the fraction (the empty icon is always under).
pub fn armor_sprites(slot: i32, armor: f32) -> (bool, bool) {
    let a = armor * 0.5;
    let full = slot as f32 <= a.floor();
    let half = (slot as f32) > a && (slot as f32) < a + 1.0;
    (full, half)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn ids_read_as_names_and_owners() {
        assert_eq!(title_case("minecraft:oak_log"), "Oak Log");
        assert_eq!(title_case("cobblestone"), "Cobblestone");
        assert_eq!(namespace_name("minecraft:stone"), "Minecraft");
        assert_eq!(namespace_name("macaws_fences:oak_picket"), "Macaws Fences");
        assert_eq!(namespace_name("stone"), "Minecraft");
    }

    #[test]
    fn state_strings_split() {
        let (id, p) = split_state("minecraft:oak_stairs[facing=north,half=bottom]");
        assert_eq!(id, "minecraft:oak_stairs");
        assert_eq!(p, props(&[("facing", "north"), ("half", "bottom")]));
        assert_eq!(
            split_state("minecraft:stone"),
            ("minecraft:stone".into(), vec![])
        );
    }

    #[test]
    fn crops_show_growth() {
        let wheat = |age: &str| details("minecraft:wheat", &props(&[("age", age)]));
        assert_eq!(wheat("0")[0].text, "Growth: 0%");
        assert_eq!(wheat("3")[0].text, "Growth: 42%");
        assert_eq!(wheat("7")[0].text, "Growth: 100% (ripe)");
        assert_eq!(wheat("7")[0].color, GREEN);
        let beets = details("minecraft:beetroots", &props(&[("age", "2")]));
        assert_eq!(beets[0].text, "Growth: 66%");
        // Not a crop: its age is not growth.
        assert!(details("minecraft:fire", &props(&[("age", "4")])).is_empty());
    }

    #[test]
    fn redstone_and_friends_show_state() {
        let wire = details("minecraft:redstone_wire", &props(&[("power", "12")]));
        assert_eq!(wire[0].text, "Power: 12");
        let repeater = details("minecraft:repeater", &props(&[("delay", "3")]));
        assert_eq!(repeater[0].text, "Delay: 3");
        let note = details(
            "minecraft:note_block",
            &props(&[("note", "5"), ("instrument", "bass_drum")]),
        );
        assert_eq!(note[0].text, "Note: 5");
        assert_eq!(note[1].text, "Instrument: Bass Drum");
        assert_eq!(
            details("minecraft:composter", &props(&[("level", "7")]))[0].text,
            "Level: 7/8"
        );
        assert!(details("minecraft:stone", &[]).is_empty());
    }

    #[test]
    fn harvest_says_what_to_use() {
        assert_eq!(harvest(None, false, true), None);
        assert_eq!(
            harvest(Some("pickaxe"), false, false).unwrap().text,
            "Best tool: Pickaxe"
        );
        let good = harvest(Some("pickaxe"), true, true).unwrap();
        assert_eq!(
            (good.text.as_str(), good.color),
            ("Can harvest (Pickaxe)", GREEN)
        );
        let bad = harvest(Some("pickaxe"), true, false).unwrap();
        assert_eq!((bad.text.as_str(), bad.color), ("Needs a pickaxe", RED));
    }

    #[test]
    fn small_health_is_hearts_big_health_is_a_number() {
        // A villager: 20 health is 10 hearts and no text.
        let row = IconRow::health(20.0, 20.0, 0.0);
        assert_eq!((row.icons, row.per_line, row.lines), (10, 10, 1));
        assert!(row.text.is_none());
        let measure = |s: &str| s.len() as f32 * 6.0;
        assert_eq!(row.size(&measure), (81.0, 9.0));
        // A pig: 10 health is 5 hearts.
        let pig = IconRow::health(10.0, 10.0, 0.0);
        assert_eq!(pig.icons, 5);
        assert_eq!(pig.size(&measure), (41.0, 9.0));
        // 40 health points is the most drawn as hearts: 20 hearts, two rows
        // of ten, drawn 4 pixels apart.
        let big = IconRow::health(40.0, 40.0, 0.0);
        assert_eq!((big.icons, big.lines), (20, 2));
        assert_eq!(big.size(&measure), (81.0, 13.0));
        // More than that: one icon and the numbers, without fractions.
        let boss = IconRow::health(300.0, 299.5, 0.0);
        assert_eq!(boss.text.as_deref(), Some("300/300"));
        assert_eq!(boss.icons, 1);
        assert_eq!(boss.size(&measure), (7.0 * 6.0 + 10.0, 9.0));
        // Absorption adds hearts.
        assert_eq!(IconRow::health(20.0, 20.0, 4.0).icons, 12);
    }

    #[test]
    fn hearts_are_drawn_back_to_front_in_rows() {
        let row = IconRow::health(20.0, 20.0, 0.0);
        let slots = row.slots();
        assert_eq!(slots.len(), 10);
        // The last heart first, at the right.
        assert_eq!(slots[0], (72, 0, 10));
        assert_eq!(slots[9], (0, 0, 1));
        // Two rows: the first row (slots 1 to 10) is the lower one, 4 apart.
        let two = IconRow::health(40.0, 40.0, 0.0).slots();
        assert_eq!(two[0], (72, 4, 20));
        assert_eq!(two[9], (0, 4, 11));
        assert_eq!(two[10], (72, 0, 10));
        // A single icon of a short row sits at the left.
        let three = IconRow::health(6.0, 6.0, 0.0).slots();
        assert_eq!(three[0], (16, 0, 3));
    }

    #[test]
    fn a_heart_is_full_half_or_empty() {
        // 7 health rounds to 7: three and a half hearts.
        assert_eq!(heart_sprites(3, 7.0), (true, false));
        assert_eq!(heart_sprites(4, 7.0), (false, true));
        assert_eq!(heart_sprites(5, 7.0), (false, false));
        assert_eq!(heart_sprites(10, 20.0), (true, false));
        assert_eq!(heart_sprites(1, 1.0), (false, true));
        // Armor 5: two full and a half.
        assert_eq!(armor_sprites(2, 5.0), (true, false));
        assert_eq!(armor_sprites(3, 5.0), (false, true));
        assert_eq!(IconRow::armor(25.0).text.as_deref(), Some("25"));
        assert_eq!(IconRow::armor(20.0).icons, 10);
    }

    #[test]
    fn numbers_print_without_trailing_zeros() {
        assert_eq!(two_decimals(7.5), "7.5");
        assert_eq!(two_decimals(20.0), "20");
        assert_eq!(two_decimals(0.126), "0.13");
    }
}
