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
                lines.push(Line::new(format!("Instrument: {}", title_case(instrument)), GRAY));
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
            format!("Needs a {}", tool.unwrap_or_else(|| "tool".into()).to_lowercase()),
            RED,
        )),
    }
}

/// A mob's health bar as heart icons: `full` hearts, an optional half, and
/// how many containers (at most 10; more health than that is written out).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hearts {
    pub containers: u32,
    pub full: u32,
    pub half: bool,
}
pub fn hearts(health: f32, max_health: f32) -> Hearts {
    let max = max_health.max(health).max(0.0);
    let containers = ((max / 2.0).ceil() as u32).clamp(1, 10);
    // Scale into the ten icons when the mob has more health than they hold.
    let scale = if max > 20.0 { 20.0 / max } else { 1.0 };
    let halves = (health.max(0.0) * scale).ceil() as u32;
    Hearts {
        containers,
        full: (halves / 2).min(containers),
        half: halves % 2 == 1 && halves / 2 < containers,
    }
}

/// `7.5 / 20` style health text for a mob.
pub fn health_text(health: f32, max_health: f32) -> String {
    let trim = |v: f32| {
        if (v - v.round()).abs() < 0.05 {
            format!("{}", v.round() as i32)
        } else {
            format!("{v:.1}")
        }
    };
    format!("{} / {}", trim(health), trim(max_health))
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
        assert_eq!(split_state("minecraft:stone"), ("minecraft:stone".into(), vec![]));
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
        assert_eq!((good.text.as_str(), good.color), ("Can harvest (Pickaxe)", GREEN));
        let bad = harvest(Some("pickaxe"), true, false).unwrap();
        assert_eq!((bad.text.as_str(), bad.color), ("Needs a pickaxe", RED));
    }

    #[test]
    fn hearts_follow_health() {
        assert_eq!(
            hearts(20.0, 20.0),
            Hearts {
                containers: 10,
                full: 10,
                half: false
            }
        );
        assert_eq!(
            hearts(7.0, 20.0),
            Hearts {
                containers: 10,
                full: 3,
                half: true
            }
        );
        // A pig has 10 health: five hearts.
        assert_eq!(
            hearts(10.0, 10.0),
            Hearts {
                containers: 5,
                full: 5,
                half: false
            }
        );
        // 100 health scales into the ten icons: half of it is five hearts.
        assert_eq!(hearts(50.0, 100.0).full, 5);
        assert_eq!(hearts(0.0, 20.0).full, 0);
    }

    #[test]
    fn health_reads_as_numbers() {
        assert_eq!(health_text(7.5, 20.0), "7.5 / 20");
        assert_eq!(health_text(20.0, 20.0), "20 / 20");
    }
}
