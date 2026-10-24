//! AppleSkin in the game (`horizon_client::appleskin`, docs/extras.md): what
//! the food in hand would do, drawn over the health and food bars, and the
//! food row of item tooltips.
use super::{Game, shield};
use horizon_client::{
    appleskin::{self, Meal},
    hud_layout::HealthInputs,
    items::{ConsumeEffect, ItemStack},
};
use horizon_world::food::Difficulty;

impl Game<'_> {
    /// `FoodHelper.query`: the food an item is, when it is one (it has the
    /// food and consumable components).
    pub(super) fn meal_of(&self, stack: &ItemStack) -> Option<Meal> {
        let info = self.res.items().get(&stack.item)?;
        let (nutrition, saturation) = info.food?;
        let consumable = info.consumable.as_ref()?;
        let mut rotten = false;
        let mut regeneration = Vec::new();
        for effect in &consumable.effects {
            let ConsumeEffect::ApplyEffects { effects, .. } = effect else {
                continue;
            };
            rotten |= effects.iter().any(|e| appleskin::harmful_effect(&e.id));
            // The first Regeneration of each group.
            if let Some(e) = effects
                .iter()
                .find(|e| e.id.strip_prefix("minecraft:").unwrap_or(&e.id) == "regeneration")
            {
                regeneration.push((e.amplifier, e.duration));
            }
        }
        Some(Meal {
            nutrition: nutrition as i32,
            saturation,
            rotten,
            regeneration,
        })
    }

    /// `Player.canEat`: creative ignores hunger.
    fn can_eat(&self, always: bool) -> bool {
        let food = self.player.as_ref().map_or(20, |p| p.health.food.food);
        self.creative || always || food < 20
    }

    /// `HeldFoodCache.query`: the food in the main hand that the player could
    /// eat now, else (when allowed) the one in the off hand.
    pub(super) fn held_meal(&self) -> Option<Meal> {
        let hands = [self.selected, shield::OFFHAND_SLOT];
        let allowed = if self.settings.extras.appleskin_offhand {
            2
        } else {
            1
        };
        for &slot in &hands[..allowed] {
            let Some(stack) = self.stack_in(slot) else {
                continue;
            };
            let Some(meal) = self.meal_of(stack) else {
                continue;
            };
            let always = self
                .res
                .items()
                .get(&stack.item)
                .is_some_and(|i| i.can_always_eat);
            if self.can_eat(always) {
                return Some(meal);
            }
        }
        None
    }

    /// `HUDOverlayHandler.onClientTick`: the previews' pulse.
    pub(super) fn appleskin_tick(&mut self) {
        if self.settings.extras.appleskin {
            let mut flash = self.appleskin_flash.get();
            flash.tick();
            self.appleskin_flash.set(flash);
        }
    }

    /// The AppleSkin overlays for this frame's bars (`health_hud.bars` holds
    /// where the layout just drew them), or `None` when the extra is off.
    pub(super) fn appleskin_overlay(&self, i: &HealthInputs) -> Option<appleskin::Overlay> {
        let extras = &self.settings.extras;
        if !extras.appleskin {
            return None;
        }
        let p = self.player.as_ref()?;
        let effects = &self.potions.effects;
        let inputs = appleskin::Inputs {
            food: i.food,
            saturation: i.saturation,
            exhaustion: p.health.food.exhaustion,
            health: i.health,
            max_health: i.max_health,
            hardcore: i.hardcore,
            peaceful: self.world.health_rules.difficulty == Difficulty::Peaceful,
            poisoned: effects.get("poison").is_some(),
            withered: effects.get("wither").is_some(),
            regenerating: effects.get("regeneration").is_some(),
            natural_regeneration: self.world.health_rules.natural_regeneration,
        };
        let options = appleskin::Options {
            restored: extras.appleskin_restored,
            saturation: extras.appleskin_saturation,
            exhaustion: extras.appleskin_exhaustion,
            health: extras.appleskin_health,
            offhand: extras.appleskin_offhand,
        };
        let meal = self.held_meal();
        let mut flash = self.appleskin_flash.get();
        let overlay = appleskin::overlay(
            &options,
            &inputs,
            &self.health_hud.bars,
            meal.as_ref(),
            flash.alpha(appleskin::MAX_FLASH),
        );
        if !overlay.holding_food {
            // `resetFlash`: the pulse starts over with the next food.
            flash.reset();
            self.appleskin_flash.set(flash);
        }
        Some(overlay)
    }

    /// The marker line that makes a tooltip draw AppleSkin's food row
    /// (`TooltipOverlayHandler.onItemTooltip`), when it should.
    pub(super) fn appleskin_tooltip_marker(&self, stack: &ItemStack) -> Option<String> {
        let extras = &self.settings.extras;
        if !extras.appleskin || !extras.appleskin_tooltips {
            return None;
        }
        if !(extras.appleskin_tooltips_always || self.has_shift_down()) {
            return None;
        }
        let meal = self.meal_of(stack)?;
        appleskin::TooltipRow::marker(meal.nutrition, meal.saturation, meal.rotten)
    }
}
