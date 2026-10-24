//! Jade in the game (`horizon_client::jade`, docs/extras.md): a box at the top
//! of the screen for the block or mob under the crosshair.
use super::{Game, Screen};
use horizon_client::{
    containers::BlockEntity,
    hud::{HudBatch, rgba},
    hud_layout::{self, Blend, GuiDraw},
    items::ItemStack,
    jade::{self, Line},
    jade_vanilla,
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
    /// Items a provider shows in a row (a furnace's slots, a lectern's book), and the
    /// progress arrow between them when there is one.
    items: Vec<Option<ItemStack>>,
    arrow: Option<f32>,
}

/// One line of the box.
enum Row {
    Text(String, u32),
    Hearts(jade::IconRow, f32),
    Armor(jade::IconRow, f32),
    /// Item slots (drawn over the box after it) and a progress arrow after them.
    Items(Vec<Option<ItemStack>>, Option<f32>),
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
            let icon = self.res.items().get(&egg).map(|_| ItemStack::new(&egg, 1));
            let title = m
                .custom_name
                .clone()
                .unwrap_or_else(|| self.jade_name("entity", &id));
            return Some(Target {
                icon,
                title,
                owner: jade::namespace_name(&id),
                lines: self.jade_mob_lines(m),
                health: Some((m.health, horizon_client::mobs::finalize::max_health(m), 0.0)),
                armor: horizon_client::mobs::finalize::value(
                    m,
                    horizon_client::mobs::finalize::Attr::Armor,
                ) as f32,
                items: Vec::new(),
                arrow: None,
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
        let props = jade::split_state(&self.state_string(state)).1;
        let (items, arrow, extra, title) = self.jade_block_providers(hit.pos, &id, &props);
        lines.extend(extra);
        Some(Target {
            icon,
            title: title.unwrap_or_else(|| self.jade_name("block", &id)),
            owner: jade::namespace_name(&id),
            lines,
            health: None,
            armor: 0.0,
            items,
            arrow,
        })
    }

    /// `Jade`'s providers for the block under the crosshair: the items and the progress a block
    /// shows (a furnace's slots and cooking, a lectern's book), its lines and a title it changes.
    fn jade_block_providers(
        &self,
        pos: [i32; 3],
        id: &str,
        props: &[(String, String)],
    ) -> (
        Vec<Option<ItemStack>>,
        Option<f32>,
        Vec<Line>,
        Option<String>,
    ) {
        let [x, y, z] = pos;
        let mut items = Vec::new();
        let mut arrow = None;
        let mut lines = Vec::new();
        let mut title = None;
        match self.world.block_entities.get(&(x, y, z)) {
            Some(BlockEntity::Furnace(f)) if f.items.iter().any(Option::is_some) => {
                items = f.items.to_vec();
                arrow = Some(jade_vanilla::cooking_progress(
                    f.cooking_timer,
                    f.cooking_total,
                ));
            }
            Some(BlockEntity::Brewing(b)) => {
                lines.extend(jade_vanilla::brewing_lines(b.fuel, b.brew_time, 20.0));
            }
            Some(BlockEntity::Jukebox(j)) => {
                if let Some(record) = &j.record {
                    lines.push(jade_vanilla::jukebox_line(
                        &self.jade_name("item", &record.item),
                    ));
                }
            }
            Some(BlockEntity::Lectern(l)) => {
                if let Some(book) = &l.book {
                    lines.push(Line {
                        text: self.jade_name("item", &book.item),
                        color: jade::WHITE,
                    });
                    items = vec![Some(book.clone())];
                }
            }
            Some(BlockEntity::Beehive(h)) => {
                let honey = props
                    .iter()
                    .find(|(k, _)| k == "honey_level")
                    .and_then(|(_, v)| v.parse::<u8>().ok())
                    .unwrap_or(0);
                // A hive is full at three bees (`Beehive.isFull`).
                lines.extend(jade_vanilla::beehive_lines(
                    honey,
                    h.bees.len(),
                    h.bees.len() >= 3,
                ));
            }
            Some(BlockEntity::Spawner(s)) => {
                if let Some(mob) = jade_vanilla::spawner_mob(s) {
                    let mob = self.jade_name("entity", &mob);
                    title = Some(jade_vanilla::spawner_title(
                        &self.jade_name("block", id),
                        &mob,
                    ));
                }
            }
            Some(BlockEntity::CommandBlock(c)) if self.can_use_game_master_blocks() => {
                lines.extend(jade_vanilla::command_line(&c.command));
            }
            _ => {}
        }
        if id == "minecraft:tnt" && props.iter().any(|(k, v)| k == "unstable" && v == "true") {
            lines.push(jade_vanilla::unstable_line());
        }
        (items, arrow, lines, title)
    }

    /// `Jade`'s providers for the mob under the crosshair: its breeding and its potion effects.
    fn jade_mob_lines(&self, m: &horizon_client::mobs::Mob) -> Vec<Line> {
        let mut lines = Vec::new();
        // A grown animal's positive age is its breeding cooldown (`AgeableMob.setAge(6000)`).
        if let Some(line) =
            jade_vanilla::breeding_line(m.extra.in_love > 0, m.extra.age.max(0), 20.0)
        {
            lines.push(line);
        }
        let effects: Vec<(String, u32, i32, i32)> = m
            .extra
            .effects
            .iter()
            .map(|(id, amplifier, ticks)| {
                let name = self.jade_name("effect", id);
                let color = horizon_client::brewing::effect(id).map_or(jade::WHITE, |e| e.color);
                (name, color, *amplifier, *ticks)
            })
            .collect();
        lines.extend(jade_vanilla::effect_lines(&effects, 3, 20.0));
        lines
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
        if !t.items.is_empty() {
            rows.push(Row::Items(t.items.clone(), t.arrow));
        }
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
                Row::Items(items, arrow) => (
                    items.len() as f32 * 18.0 + if arrow.is_some() { 18.0 } else { 0.0 },
                    16.0,
                ),
            }
        };
        let text_w = rows.iter().map(|r| size(r).0).fold(0.0, f32::max);
        let text_h: f32 =
            rows.iter().map(|r| size(r).1).sum::<f32>() + LINE_GAP * (rows.len() - 1) as f32;
        let icon_w = if t.icon.is_some() {
            ICON + ICON_GAP
        } else {
            0.0
        };
        let w = PAD * 2.0 + icon_w + text_w;
        let h = PAD * 2.0 + text_h.max(ICON);
        let bosses = self.bosses.bars.bars.len() as f32;
        let x0 = ((batch.width - w) / 2.0).floor();
        let y0 = 4.0
            + if bosses > 0.0 {
                10.0 + bosses * 19.0
            } else {
                0.0
            };
        // `jade:dark`: a dark box with a lighter edge, at the overlay's
        // 70% opacity.
        let alpha = (0.7_f32 * 255.0) as u8;
        batch.fill(atlas, x0, y0, w, h, rgba(46, 46, 46, alpha));
        batch.fill(
            atlas,
            x0 + 1.0,
            y0 + 1.0,
            w - 2.0,
            h - 2.0,
            rgba(19, 19, 19, alpha),
        );
        let tx = x0 + PAD + icon_w;
        let mut ty = y0 + PAD + ((text_h.max(ICON) - text_h) / 2.0).floor();
        let mut draws: Vec<GuiDraw> = Vec::new();
        let mut item_draws: Vec<(ItemStack, f32, f32)> = Vec::new();
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
                Row::Items(items, arrow) => {
                    for (i, item) in items.iter().enumerate() {
                        if let Some(item) = item {
                            item_draws.push((item.clone(), tx + i as f32 * 18.0, ty));
                        }
                    }
                    if let Some(done) = arrow {
                        let ax = tx + items.len() as f32 * 18.0;
                        batch.fill(atlas, ax, ty + 6.0, 16.0, 4.0, rgba(60, 60, 66, 255));
                        batch.fill(
                            atlas,
                            ax,
                            ty + 6.0,
                            16.0 * done.clamp(0.0, 1.0),
                            4.0,
                            rgba(220, 220, 225, 255),
                        );
                    }
                }
            }
            ty += row_h + LINE_GAP;
        }
        hud_layout::paint(batch, atlas, &draws);
        for (item, ix, iy) in item_draws {
            self.draw_item_model(batch, &item, ix.round(), iy.round());
        }
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
