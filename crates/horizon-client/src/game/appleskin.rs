//! AppleSkin in the game (`horizon_client::appleskin`, docs/extras.md): what
//! the food in hand would do, drawn over the health and food bars.
use super::Game;
use horizon_client::{appleskin, hud_layout::HealthInputs};

impl Game<'_> {
    /// The food in the acting hand, when the player could eat it now.
    pub(super) fn held_meal(&self) -> Option<appleskin::Meal> {
        let stack = self.held()?;
        let info = self.res.items().get(&stack.item)?;
        let (nutrition, saturation) = info.food?;
        let food = self.player.as_ref()?.health.food.food;
        // `Player.canEat`, or creative's open mouth: Consumable.canConsume.
        let can_eat =
            info.consumable.is_some() && (info.can_always_eat || food < 20 || self.creative);
        can_eat.then_some(appleskin::Meal {
            nutrition: nutrition as i32,
            saturation,
        })
    }

    /// The AppleSkin overlays for this frame's bars (`health_hud.bars` holds
    /// where the layout just drew them), or `None` when the extra is off.
    pub(super) fn appleskin_overlay(&self, i: &HealthInputs) -> Option<appleskin::Overlay> {
        let extras = &self.settings.extras;
        if !extras.appleskin {
            return None;
        }
        let p = self.player.as_ref()?;
        let inputs = appleskin::Inputs {
            tick: i.tick,
            food: i.food,
            saturation: i.saturation,
            exhaustion: p.health.food.exhaustion,
            health: i.health,
            max_health: i.max_health,
            hardcore: i.hardcore,
            heart_kind: i.kind,
            hunger_effect: i.hunger,
            difficulty: self.world.health_rules.difficulty,
            natural_regeneration: self.world.health_rules.natural_regeneration,
            meal: self.held_meal(),
        };
        let options = appleskin::Options {
            restored: extras.appleskin_restored,
            saturation: extras.appleskin_saturation,
            exhaustion: extras.appleskin_exhaustion,
            health: extras.appleskin_health,
        };
        let gain = if options.health {
            self.appleskin_cache.borrow_mut().gain(&inputs)
        } else {
            None
        };
        Some(appleskin::overlay(
            &options,
            &inputs,
            &self.health_hud.bars,
            gain,
        ))
    }
}
