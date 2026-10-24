//! Jade in the game (`horizon_client::jade`, docs/extras.md): a box at the top
//! of the screen for the block or mob under the crosshair.
use super::{Game, Screen};
use horizon_client::{
    hud::{HudBatch, rgba},
    hud_layout::{self, Blend, GuiDraw},
    items::ItemStack,
    jade::{self, Line},
};
use std::borrow::Cow;

/// The tools a block can name as its best, as their `mineable` tags.
const TOOLS: [(&str, &str); 4] = [
    ("pickaxe", "minecraft:mineable/pickaxe"),
    ("axe", "minecraft:mineable/axe"),
    ("shovel", "minecraft:mineable/shovel"),
    ("hoe", "minecraft:mineable/hoe"),
];
/// `BoxStyle.DEFAULT_PADDING` (3) and the box's 1 pixel border.
const PAD: f32 = 4.0;
/// The gap between the icon and the text, and between lines (`marginBottom`).
const ICON_GAP: f32 = 3.0;
const LINE_GAP: f32 = 2.0;
const ICON: f32 = 16.0;

/// What the crosshair is on, gathered from the world.
struct Target {
    icon: Option<ItemStack>,
    title: String,
    /// Whose it is, as the line under the rest: `Minecraft`.
    owner: String,
    lines: Vec<Line>,
    /// A mob's health, maximum health and absorption.
    health: Option<(f32, f32, f32)>,
    /// A mob's armor points.
    armor: f32,
}

/// One line of the box.
enum Row {
    Text(String, u32),
    Hearts(jade::IconRow, f32),
    Armor(jade::IconRow, f32),
}

fn sprite(name: &'static str, x: i32, y: i32) -> GuiDraw {
    GuiDraw::Sprite {
        sprite: Cow::Borrowed(name),
        x,
        y,
        w: 9,
        h: 9,
        region: None,
        color: u32::MAX,
        blend: Blend::Textured,
    }
}

impl Game<'_> {
    /// `block.minecraft.oak_log` or `entity.minecraft.pig` for an id, or its
    /// title-cased name where the language has none.
    fn jade_name(&self, kind: &str, id: &str) -> String {
        let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
        self.translate(
            &format!("{kind}.{namespace}.{}", path.replace('/', ".")),
            &jade::title_case(id),
        )
    }

    fn jade_target(&self) -> Option<Target> {
        let eye = self.eye_position()?;
        let extras = &self.settings.extras;
        let block = self.pick().map(|hit| {
            let d = hit
                .point
                .iter()
                .zip(eye)
                .map(|(p, e)| (p - e) * (p - e))
                .sum::<f64>()
                .sqrt();
            (hit, d)
        });
        let mob = self.mob_hit();
        let mob_nearer = match (&block, &mob) {
            (Some((_, b)), Some((_, m))) => m <= b,
            (None, Some(_)) => true,
            _ => false,
        };
        if mob_nearer {
            let (i, _) = mob?;
            let m = self.mobs.get(i)?;
            let id = m.kind.id.to_string();
            let egg = format!("{id}_spawn_egg");
            let icon = self
                .res
                .items()
                .get(&egg)
                .map(|_| ItemStack::new(&egg, 1));
            let title = m
                .custom_name
                .clone()
                .unwrap_or_else(|| self.jade_name("entity", &id));
            return Some(Target {
                icon,
                title,
                owner: jade::namespace_name(&id),
                lines: Vec::new(),
                health: Some((
                    m.health,
                    horizon_client::mobs::finalize::max_health(m),
                    0.0,
                )),
                armor: horizon_client::mobs::finalize::value(
                    m,
                    horizon_client::mobs::finalize::Attr::Armor,
                ) as f32,
            });
        }
        let (hit, _) = block?;
        let [x, y, z] = hit.pos;
        let state = self.world.block(x, y, z);
        let facts = self.res.facts().get(state);
        let id = facts.name.to_string();
        let icon = (&*facts.item != "minecraft:air").then(|| ItemStack::new(&facts.item, 1));
        let mut lines = Vec::new();
        if extras.jade_details {
            let (_, props) = jade::split_state(&self.state_string(state));
            lines.extend(jade::details(&id, &props));
        }
        if facts.hardness < 0.0 {
            lines.push(Line {
                text: "Unbreakable".into(),
                color: jade::RED,
            });
        } else if extras.jade_harvest {
            let items = self.res.items();
            let tool = TOOLS
                .iter()
                .find(|(_, tag)| items.block_in_tag(tag, &id))
                .map(|(name, _)| *name);
            let can = !facts.requires_tool || items.correct_for_drops(self.held(), &id);
            lines.extend(jade::harvest(tool, facts.requires_tool, can));
        }
        Some(Target {
            icon,
            title: self.jade_name("block", &id),
            owner: jade::namespace_name(&id),
            lines,
            health: None,
            armor: 0.0,
        })
    }

    /// The Jade box, centred at the top under any boss bars.
    pub(super) fn draw_jade(&mut self, batch: &mut HudBatch) {
        if !self.settings.extras.jade
            || self.title.is_some()
            || !matches!(self.screen, Screen::Playing | Screen::Chat)
        {
            return;
        }
        let Some(t) = self.jade_target() else {
            return;
        };
        let atlas = &self.gui.atlas;
        let text_width = |s: &str| HudBatch::text_width(atlas, s);
        // The lines: name, health and armor, the details, the mod's name.
        let white = u32::MAX;
        let mut rows = vec![Row::Text(t.title.clone(), white)];
        if let Some((health, max, absorption)) = t.health {
            let row = jade::IconRow::health(max, health, absorption);
            rows.push(Row::Hearts(row, health));
        }
        if t.armor > 0.0 {
            rows.push(Row::Armor(jade::IconRow::armor(t.armor), t.armor));
        }
        for l in &t.lines {
            let [_, r, g, b] = l.color.to_be_bytes();
            rows.push(Row::Text(l.text.clone(), rgba(r, g, b, 255)));
        }
        // `ModNameProvider`: the owner in the item mod name style.
        rows.push(Row::Text(format!("\u{a7}9\u{a7}o{}", t.owner), white));
        let size = |row: &Row| -> (f32, f32) {
            match row {
                Row::Text(text, _) => (text_width(text), 8.0),
                Row::Hearts(r, _) | Row::Armor(r, _) => r.size(&text_width),
            }
        };
        let text_w = rows.iter().map(|r| size(r).0).fold(0.0, f32::max);
        let text_h: f32 = rows.iter().map(|r| size(r).1).sum::<f32>()
            + LINE_GAP * (rows.len() - 1) as f32;
        let icon_w = if t.icon.is_some() { ICON + ICON_GAP } else { 0.0 };
        let w = PAD * 2.0 + icon_w + text_w;
        let h = PAD * 2.0 + text_h.max(ICON);
        let bosses = self.bosses.bars.bars.len() as f32;
        let x0 = ((batch.width - w) / 2.0).floor();
        let y0 = 4.0 + if bosses > 0.0 { 10.0 + bosses * 19.0 } else { 0.0 };
        // `jade:dark`: a dark box with a lighter edge, at the overlay's
        // 70% opacity.
        let alpha = (0.7_f32 * 255.0) as u8;
        batch.fill(atlas, x0, y0, w, h, rgba(46, 46, 46, alpha));
        batch.fill(atlas, x0 + 1.0, y0 + 1.0, w - 2.0, h - 2.0, rgba(19, 19, 19, alpha));
        let tx = x0 + PAD + icon_w;
        let mut ty = y0 + PAD + ((text_h.max(ICON) - text_h) / 2.0).floor();
        let mut draws: Vec<GuiDraw> = Vec::new();
        for row in &rows {
            let (_, row_h) = size(row);
            match row {
                Row::Text(text, color) => {
                    batch.text(atlas, text, tx, ty, *color, true);
                }
                Row::Hearts(r, health) => {
                    for (dx, dy, slot) in r.slots() {
                        let (x, y) = (tx as i32 + dx, ty as i32 + dy);
                        draws.push(sprite("hud/heart/container", x, y));
                        let (full, half) = jade::heart_sprites(slot, *health);
                        if full {
                            draws.push(sprite("hud/heart/full", x, y));
                        } else if half {
                            draws.push(sprite("hud/heart/half", x, y));
                        }
                    }
                    if let Some(text) = &r.text {
                        batch.text(atlas, text, tx + 10.0, ty + 1.0, jade::NORMAL, true);
                    }
                }
                Row::Armor(r, armor) => {
                    for (dx, dy, slot) in r.slots() {
                        let (x, y) = (tx as i32 + dx, ty as i32 + dy);
                        draws.push(sprite("hud/armor_empty", x, y));
                        let (full, half) = jade::armor_sprites(slot, *armor);
                        if full {
                            draws.push(sprite("hud/armor_full", x, y));
                        }
                        if half {
                            draws.push(sprite("hud/armor_half", x, y));
                        }
                    }
                    if let Some(text) = &r.text {
                        batch.text(atlas, text, tx + 10.0, ty + 1.0, jade::NORMAL, true);
                    }
                }
            }
            ty += row_h + LINE_GAP;
        }
        hud_layout::paint(batch, atlas, &draws);
        // How far along the block being dug is.
        if let Some(m) = &self.mining {
            let done = m.progress.clamp(0.0, 1.0);
            batch.fill(atlas, x0, y0 + h, w * done, 2.0, rgba(85, 255, 85, 220));
        }
        if let Some(icon) = &t.icon {
            let (ix, iy) = (x0 + PAD, y0 + (h - ICON) / 2.0);
            self.draw_item_model(batch, icon, ix.round(), iy.round());
        }
    }
}
