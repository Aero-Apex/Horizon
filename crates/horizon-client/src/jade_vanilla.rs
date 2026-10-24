//! Jade's vanilla providers (`addon/vanilla` in Snownee's Jade, CC BY-NC-SA 4.0): the lines a
//! block or a mob shows as the mod words them, and how a tick count reads (`IThemeHelper.seconds`,
//! `StringUtil.formatTickDuration`). `game/jade.rs` gathers the state and draws the box.
use crate::jade::{GRAY, GREEN, Line, RED, WHITE, YELLOW};
use crate::mobs::spawner::Spawner;
use horizon_world::nbt::Tag;

/// `IThemeHelper.seconds`: a tick count as hours, minutes or seconds, rounded down.
pub fn seconds(ticks: i32, tick_rate: f32) -> String {
    let mut s = (ticks as f32 / tick_rate).floor() as i32;
    if s >= 3600 {
        let hours = s / 3600;
        let minutes = s % 3600 / 60;
        return if minutes == 0 {
            format!("{hours}h")
        } else {
            format!("{hours}h {minutes}m")
        };
    }
    if s >= 60 {
        let minutes = s / 60;
        s %= 60;
        return if s == 0 {
            format!("{minutes}m")
        } else {
            format!("{minutes}m {s}s")
        };
    }
    format!("{s}s")
}

/// `StringUtil.formatTickDuration`: `m:ss`, or `h:mm:ss` from an hour on.
pub fn tick_duration(ticks: i32, tick_rate: f32) -> String {
    let total = (ticks as f32 / tick_rate).floor() as i32;
    let (hours, minutes, seconds) = (total / 3600, total / 60 % 60, total % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// An effect's level as the game's `enchantment.level.N` writes it: I to X, else the number.
pub fn roman(level: i32) -> String {
    const NUMERALS: [&str; 10] = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"];
    match level {
        1..=10 => NUMERALS[(level - 1) as usize].to_owned(),
        _ => level.to_string(),
    }
}

/// `BeehiveProvider`: the honey level out of five (green when full) and the bees inside (green
/// when the hive is full of them).
pub fn beehive_lines(honey: u8, bees: usize, full: bool) -> Vec<Line> {
    let honey_color = if honey >= 5 { GREEN } else { YELLOW };
    let bees_color = if full { GREEN } else { YELLOW };
    vec![
        Line {
            text: format!("Honey: {honey} / 5"),
            color: honey_color,
        },
        Line {
            text: format!("Bees: {bees}"),
            color: bees_color,
        },
    ]
}

/// `BrewingStandProvider`: the fuel left, and while it brews, the time left.
pub fn brewing_lines(fuel: i32, brew_time: i32, tick_rate: f32) -> Vec<Line> {
    let mut lines = vec![Line {
        text: format!("Fuel: {fuel}"),
        color: YELLOW,
    }];
    if brew_time > 0 {
        lines.push(Line {
            text: format!("Brewing: {}", seconds(brew_time, tick_rate)),
            color: YELLOW,
        });
    }
    lines
}

/// `FurnaceProvider`: how far the cooking has got, 0 to 1 (the arrow between its items).
pub fn cooking_progress(progress: i32, total: i32) -> f32 {
    if total <= 0 {
        return 0.0;
    }
    (progress as f32 / total as f32).clamp(0.0, 1.0)
}

/// `JukeboxProvider`: the record playing, as its song's name.
pub fn jukebox_line(song: &str) -> Line {
    Line {
        text: format!("Now Playing: {song}"),
        color: WHITE,
    }
}

/// `SpawnerProvider`: the spawner's title names the mob it spawns, as `Spawner (Zombie)`. The mob
/// is the one it summons next, else the first of its potentials (`SpawnData.entity.id`).
pub fn spawner_title(block: &str, mob: &str) -> String {
    format!("{block} ({mob})")
}

/// The entity id a spawner shows: its next spawn's, else its first potential's.
pub fn spawner_mob(spawner: &Spawner) -> Option<String> {
    let data = spawner
        .next_data()
        .or_else(|| spawner.potentials.first().map(|(d, _)| d))?;
    match data.entity.get(&"id".into())? {
        Tag::String(id) => Some(id.to_string_lossy()),
        _ => None,
    }
}

/// `CommandBlockProvider`: the command, cut to 40 characters as `> command...`.
pub fn command_line(command: &str) -> Option<Line> {
    if command.trim().is_empty() {
        return None;
    }
    let shown = if command.chars().count() > 40 {
        format!("{}...", command.chars().take(37).collect::<String>())
    } else {
        command.to_owned()
    };
    Some(Line {
        text: format!("> {shown}"),
        color: WHITE,
    })
}

/// `TNTStabilityProvider`: TNT that ignites when hit.
pub fn unstable_line() -> Line {
    Line {
        text: "Unstable".into(),
        color: RED,
    }
}

/// `AnimalOwnerProvider`: the owner of a tamed mob.
pub fn owner_line(name: &str) -> Line {
    Line {
        text: format!("Owner: {name}"),
        color: WHITE,
    }
}

/// `MobBreedingProvider`: a fed animal is in love; the cooldown before the next one is shown
/// while it lasts.
pub fn breeding_line(in_love: bool, cooldown: i32, tick_rate: f32) -> Option<Line> {
    if in_love {
        Some(Line {
            text: "Breeding CD: In Love".into(),
            color: GRAY,
        })
    } else if cooldown > 0 {
        Some(Line {
            text: format!("Breeding CD: {}", seconds(cooldown, tick_rate)),
            color: GRAY,
        })
    } else {
        None
    }
}

/// `StatusEffectsProvider`: each effect with its level and the time left (`Infinite` when it has
/// none), coloured by its kind, at most `limit` of them. `effects` are (name, colour, amplifier,
/// ticks left, where a negative count is infinite).
pub fn effect_lines(
    effects: &[(String, u32, i32, i32)],
    limit: usize,
    tick_rate: f32,
) -> Vec<Line> {
    effects
        .iter()
        .take(limit)
        .map(|(name, color, amplifier, ticks)| {
            let left = if *ticks < 0 {
                "Infinite".to_owned()
            } else {
                tick_duration(*ticks, tick_rate)
            };
            Line {
                text: format!("{name} {} ({left})", roman(amplifier + 1)),
                color: *color,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_read_as_jade_writes_them() {
        assert_eq!(seconds(0, 20.0), "0s");
        assert_eq!(seconds(45 * 20, 20.0), "45s");
        assert_eq!(seconds(90 * 20, 20.0), "1m 30s");
        assert_eq!(seconds(120 * 20, 20.0), "2m");
        assert_eq!(seconds(3600 * 20, 20.0), "1h");
        assert_eq!(seconds(3660 * 20, 20.0), "1h 1m");
        // A slower tick rate counts more seconds for the same ticks.
        assert_eq!(seconds(20, 10.0), "2s");
        assert_eq!(tick_duration(65 * 20, 20.0), "1:05");
        assert_eq!(tick_duration(3725 * 20, 20.0), "1:02:05");
    }

    #[test]
    fn levels_and_progress() {
        assert_eq!(roman(1), "I");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(11), "11");
        assert_eq!(cooking_progress(0, 200), 0.0);
        assert_eq!(cooking_progress(100, 200), 0.5);
        assert_eq!(cooking_progress(300, 200), 1.0);
        assert_eq!(cooking_progress(5, 0), 0.0);
    }

    #[test]
    fn beehive_and_brewing_lines() {
        let hive = beehive_lines(5, 3, true);
        assert_eq!(hive[0].text, "Honey: 5 / 5");
        assert_eq!(hive[0].color, GREEN);
        assert_eq!(hive[1].text, "Bees: 3");
        assert_eq!(beehive_lines(2, 1, false)[0].color, YELLOW);
        let stand = brewing_lines(12, 400, 20.0);
        assert_eq!(stand[0].text, "Fuel: 12");
        assert_eq!(stand[1].text, "Brewing: 20s");
        assert_eq!(brewing_lines(0, 0, 20.0).len(), 1);
    }

    #[test]
    fn commands_and_owners() {
        assert_eq!(command_line("  "), None);
        assert_eq!(command_line("say hi").unwrap().text, "> say hi");
        let long = "x".repeat(50);
        let shown = command_line(&long).unwrap().text;
        assert_eq!(shown, format!("> {}...", "x".repeat(37)));
        assert_eq!(owner_line("Steve").text, "Owner: Steve");
        assert_eq!(
            breeding_line(true, 0, 20.0).unwrap().text,
            "Breeding CD: In Love"
        );
        assert_eq!(
            breeding_line(false, 100, 20.0).unwrap().text,
            "Breeding CD: 5s"
        );
        assert_eq!(breeding_line(false, 0, 20.0), None);
    }

    #[test]
    fn effects_are_listed_with_their_limit() {
        let effects = vec![
            ("Speed".to_owned(), 0x7CAFC6, 1, 30 * 20),
            ("Poison".to_owned(), 0x4E9331, 0, -1),
            ("Night Vision".to_owned(), 0x1F1FA1, 0, 5 * 60 * 20),
        ];
        let lines = effect_lines(&effects, 2, 20.0);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "Speed II (0:30)");
        assert_eq!(lines[1].text, "Poison I (Infinite)");
        assert_eq!(lines[1].color, 0x4E9331);
    }
}
