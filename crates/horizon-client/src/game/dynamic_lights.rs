//! Horizon Extras' dynamic lights (`horizon_client::dynamic_light`): the light sources near the
//! camera this frame, found as LambDynamicLights finds them. Nothing is gathered while the
//! extra is off.
use super::{Game, shield};
use glam::DVec3;
use horizon_client::{
    blocks::Fluid,
    dynamic_light::{
        self, BURNING, Being, Explosive, Luminance, Source, TNT_FUSE, mob_level, projectile_level,
        tnt_level,
    },
    projectiles::ProjectileKind,
};

/// The inventory slots a player wears armour in (feet to head), then the off hand: every slot
/// LambDynamicLights reads besides the main hand.
const WORN_SLOTS: [usize; 5] = [36, 37, 38, 39, shield::OFFHAND_SLOT];
/// `Entity.getEyeY` above the feet: a player standing, a mob's eye height is its own.
const PLAYER_EYE: f64 = 1.62;
/// A dropped item's eye height (its box is a quarter of a block, eyes at 85%).
const ITEM_EYE: f64 = 0.2125;
/// A minecart's eye height (its box is 0.7 blocks tall, eyes at 85%).
const CART_EYE: f64 = 0.6;
/// Primed TNT's eye height (its box is 0.98 blocks tall, eyes at 85%).
const TNT_EYE: f64 = 0.833;
/// `Creeper.maxSwell` less 2: what `getSwelling` divides by.
const SWELL_SPAN: f32 = 28.0;
/// A glowing item frame's own light (`glow_item_frame.json`).
const GLOW_FRAME: u8 = 12;
/// A magma cube squashed past this glows brighter (`CubeMobSquishLuminance`).
const SQUISHED: f32 = 0.6;

impl Game<'_> {
    /// Fills `dynamic_lights` for the frame, the nearest [`dynamic_light::MAX`] of them: what
    /// the player and mobs hold or wear, mobs that glow by what they are, anything on fire or
    /// glowing, glowing dropped items, minecarts with a lit block, glowing item frames, primed
    /// TNT and swelling creepers (both as the settings say), fireballs, wither skulls, arrows and
    /// falling glowing blocks. Mobs give off light from their eyes, as LambDynamicLights has them.
    pub(super) fn gather_dynamic_lights(&mut self, camera: DVec3, alpha: f64) {
        let mut out = std::mem::take(&mut self.dynamic_lights);
        out.clear();
        if !self.settings.extras.dynamic_lights {
            self.dynamic_lights = out;
            return;
        }
        if self.luminance.is_none() {
            self.luminance = Some(Luminance::new(self.res.facts(), self.pack.assets.registry));
        }
        let lum = self.luminance.as_ref().expect("built above");
        let creeper = self.settings.extras.dynamic_lights_creeper;
        let tnt = self.settings.extras.dynamic_lights_tnt;
        let mut sources = Vec::new();
        if let Some(p) = &self.player
            && !p.spectator
        {
            let feet = self
                .previous_position
                .lerp(p.position, self.clock.camera_fraction());
            let submerged = p.is_under_water();
            let item_level = |slot: usize| {
                self.stack_in(slot)
                    .map_or(0, |s| lum.in_fluid(&s.item, submerged))
            };
            let level = WORN_SLOTS
                .iter()
                .map(|&slot| item_level(slot))
                .fold(item_level(self.selected), u8::max)
                .max(if p.fire_ticks > 0 { BURNING } else { 0 });
            sources.push(Source {
                at: DVec3::new(feet.x, feet.y + PLAYER_EYE, feet.z),
                level,
            });
        }
        for m in self.mobs.iter().filter(|m| !m.removed) {
            let submerged = m.fluids.eye_in_water;
            let held = |item: &Option<std::sync::Arc<str>>| {
                item.as_deref().map_or(0, |i| lum.in_fluid(i, submerged))
            };
            let being = Being {
                wet: m.fluids.in_water,
                swelling: (m.previous_swell as f32 / SWELL_SPAN).clamp(0.0, 1.0),
                carried: m
                    .extra
                    .carried
                    .map_or(0, |state| self.res.facts().get(state).emission),
                squished: horizon_client::mobs::cube::squish(m) > SQUISHED,
            };
            let level = [
                &m.extra.main_hand,
                &m.extra.off_hand,
                &m.extra.head,
                &m.extra.chest,
                &m.extra.legs,
                &m.extra.feet,
            ]
            .into_iter()
            .map(held)
            .fold(mob_level(&m.kind.id, being, creeper), u8::max)
            .max(if m.fire_ticks > 0 || m.glowing {
                BURNING
            } else {
                0
            });
            if level == 0 {
                continue;
            }
            let at = m.previous.lerp(m.position, alpha);
            let scale = f64::from(m.height()) / m.kind.height.max(0.01);
            sources.push(Source {
                at: DVec3::new(at.x, at.y + m.kind.eye_height * scale, at.z),
                level,
            });
        }
        for item in &self.dropped {
            let level = lum
                .in_fluid(&item.stack.item, false)
                .max(if item.fire_ticks > 0 { BURNING } else { 0 });
            if level == 0 {
                continue;
            }
            let at = item.previous.lerp(item.position, alpha);
            sources.push(Source {
                at: DVec3::new(at.x, at.y + ITEM_EYE, at.z),
                level,
            });
        }
        if tnt != Explosive::Off {
            for t in &self.tnts {
                let level = tnt_level(tnt, t.fuse, TNT_FUSE);
                if level == 0 {
                    continue;
                }
                let at = t.previous.lerp(t.position, alpha);
                sources.push(Source {
                    at: DVec3::new(at.x, at.y + TNT_EYE, at.z),
                    level,
                });
            }
        }
        for c in &self.carts {
            // AbstractMinecart: the light of its display block (a lit furnace cart glows).
            let level = self
                .display_state(c)
                .map_or(0, |state| self.res.facts().get(state).emission);
            if level == 0 {
                continue;
            }
            let at = c.previous.lerp(c.position, alpha);
            sources.push(Source {
                at: DVec3::new(at.x, at.y + CART_EYE, at.z),
                level,
            });
        }
        for f in &self.item_frames {
            // ItemFrameLuminance: the item's light, underwater when the frame is in water.
            let [x, y, z] = f.pos;
            let wet = self.res.facts().get(self.world.block(x, y, z)).fluid != Fluid::None;
            let mut level = if f.glow { GLOW_FRAME } else { 0 };
            if let Some(item) = &f.item {
                level = level.max(lum.in_fluid(&item.item, wet));
            }
            if level == 0 {
                continue;
            }
            let (lo, hi) = f.bounds();
            let mid = |i: usize| (lo[i] + hi[i]) / 2.0;
            sources.push(Source {
                at: DVec3::new(mid(0), mid(1), mid(2)),
                level,
            });
        }
        for p in &self.creatures.projectiles {
            let (kind, fire, arrow_item) = match (&p.kind, &p.arrow) {
                (ProjectileKind::SmallFireball, _) => ("small_fireball", false, 0),
                (ProjectileKind::LargeFireball { .. }, _) => ("fireball", false, 0),
                (ProjectileKind::WitherSkull { .. }, _) => ("wither_skull", false, 0),
                (ProjectileKind::Arrow { .. }, Some(a)) => {
                    ("arrow", a.fire_ticks > 0, lum.of(&a.pickup_item.item))
                }
                _ => continue,
            };
            let level = projectile_level(kind, false, fire, arrow_item);
            if level == 0 {
                continue;
            }
            let at = p.previous.lerp(p.position, alpha);
            sources.push(Source {
                at: DVec3::new(at.x, at.y + 0.1, at.z),
                level,
            });
        }
        for block in &self.falling {
            let level = self.res.facts().get(block.state).emission;
            if level == 0 {
                continue;
            }
            let at = block.previous.lerp(block.position, alpha);
            sources.push(Source {
                at: DVec3::new(at.x, at.y + 0.5, at.z),
                level,
            });
        }
        dynamic_light::select(&sources, camera, &mut out);
        self.dynamic_lights = out;
    }
}
