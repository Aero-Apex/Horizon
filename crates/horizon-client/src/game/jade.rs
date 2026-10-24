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
const PAD: f32 = 4.0;
const ICON: f32 = 16.0;
const ROW: f32 = 10.0;

/// What the crosshair is on, gathered from the world.
struct Target {
    icon: Option<ItemStack>,
    title: String,
    /// Whose it is, as the line under the rest: `Minecraft`.
    owner: String,
    lines: Vec<Line>,
    /// A mob's health and maximum.
    health: Option<(f32, f32)>,
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
                health: Some((m.health, horizon_client::mobs::finalize::max_health(m))),
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
        let owner = format!("\u{a7}9\u{a7}o{}", t.owner);
        let hearts = t.health.map(|(h, m)| jade::hearts(h, m));
        let text_width = |s: &str| HudBatch::text_width(atlas, s);
        let mut text_w = text_width(&t.title).max(text_width(&owner));
        for l in &t.lines {
            text_w = text_w.max(text_width(&l.text));
        }
        let health_text = t
            .health
            .map(|(h, m)| jade::health_text(h, m))
            .filter(|_| hearts.is_some_and(|h| h.containers >= 10) && t.health.is_some());
        if let Some(h) = &health_text {
            text_w = text_w.max(text_width(h));
        }
        if let Some(h) = hearts {
            text_w = text_w.max(h.containers as f32 * 8.0 + 1.0);
        }
        let icon_w = if t.icon.is_some() { ICON + PAD } else { 0.0 };
        let mut rows = 2 + t.lines.len();
        if hearts.is_some() {
            rows += 1;
        }
        let w = PAD * 2.0 + icon_w + text_w;
        let h = (rows as f32 * ROW + PAD * 2.0 - 2.0).max(ICON + PAD * 2.0);
        let bosses = self.bosses.bars.bars.len() as f32;
        let x0 = ((batch.width - w) / 2.0).floor();
        let y0 = 4.0 + if bosses > 0.0 { 10.0 + bosses * 19.0 } else { 0.0 };
        // Background and a pale border.
        batch.fill(atlas, x0, y0, w, h, rgba(0, 0, 0, 150));
        let edge = rgba(255, 255, 255, 70);
        batch.fill(atlas, x0, y0, w, 1.0, edge);
        batch.fill(atlas, x0, y0 + h - 1.0, w, 1.0, edge);
        batch.fill(atlas, x0, y0, 1.0, h, edge);
        batch.fill(atlas, x0 + w - 1.0, y0, 1.0, h, edge);
        let tx = x0 + PAD + icon_w;
        let mut ty = y0 + PAD;
        batch.text(atlas, &t.title, tx, ty, u32::MAX, true);
        ty += ROW;
        if let Some(hearts) = hearts {
            let mut draws = Vec::new();
            for n in 0..hearts.containers as i32 {
                let (hx, hy) = (tx as i32 + n * 8, ty as i32);
                draws.push(sprite("hud/heart/container", hx, hy));
                if (n as u32) < hearts.full {
                    draws.push(sprite("hud/heart/full", hx, hy));
                } else if n as u32 == hearts.full && hearts.half {
                    draws.push(sprite("hud/heart/half", hx, hy));
                }
            }
            hud_layout::paint(batch, atlas, &draws);
            if let Some(text) = &health_text {
                let hx = tx + hearts.containers as f32 * 8.0 + 4.0;
                batch.text(atlas, text, hx, ty, rgba(255, 85, 85, 255), true);
            }
            ty += ROW;
        }
        for l in &t.lines {
            let [_, r, g, b] = l.color.to_be_bytes();
            batch.text(atlas, &l.text, tx, ty, rgba(r, g, b, 255), true);
            ty += ROW;
        }
        batch.text(atlas, &owner, tx, ty, u32::MAX, true);
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
