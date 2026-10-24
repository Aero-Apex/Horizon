//! Horizon's inventory extras (#332, `docs/extras.md`), Mouse Tweaks-style
//! controls and sorting, both off by default: while an extra is off its
//! hooks return false and the screen clicks exactly as vanilla's. Each action is made of ordinary slot clicks
//! (the menu engine's, or `ContainerClick`s on a server), so it works
//! wherever vanilla's clicks do. They run in the player inventory and the
//! chest-like containers (chests, barrels, ender chests, shulker boxes,
//! hoppers, dispensers).
//!
//! - Right-drag holding a stack: one item into each slot entered, at once.
//! - Left-drag holding a stack, before vanilla's drag has spread over a
//!   second slot: each slot entered holding the same item is gathered into
//!   the hand (when it all fits); with Shift, those slots are quick-moved.
//! - Shift + left-drag with an empty hand: every slot entered is quick-moved.
//! - The wheel over a slot: down moves items of it to the other inventory (the container,
//!   or the crafting grid), up brings items of the same kind back, as Mouse Tweaks'
//!   `onMouseScrolled` does (`mouse_tweaks.rs`), with a stack in hand too.
//!
//! Sorting draws a small button by the container's title and one by the
//! inventory's (an optional key sorts the part under the cursor); the
//! clicks come from `inv_sort::plan`.
use super::{Creative, Game, Screen, stations::Kind};
use horizon_client::{
    containers::BlockEntity,
    hud::{HudBatch, rgba},
    inv_sort,
    items::ItemStack,
    keybinds,
    menu::Input,
    mouse_tweaks,
    stations::hover_name,
};
use std::{collections::HashMap, sync::Arc};
use winit::{event::MouseButton, keyboard::KeyCode};

/// A sort button's side in GUI pixels.
const SORT_BUTTON: f32 = 9.0;

#[derive(Default)]
pub(super) struct Tweaks {
    drag: Option<Drag>,
    /// Wheel distance not yet worth an item (a trackpad scrolls in fractions).
    wheel: mouse_tweaks::Accumulator,
}
/// A drag the tweaks follow.
#[derive(Clone, Debug)]
struct Drag {
    right: bool,
    /// The stack in hand at the press and its count since, as the drag's
    /// own clicks change it (a server answers them later).
    held: Option<ItemStack>,
    /// The drag replaced vanilla's: its release clicks nothing.
    active: bool,
    visited: Vec<usize>,
}

impl Game<'_> {
    /// The open screen is one the tweaks run in, with the extra on: the player's inventory and
    /// crafting, the chests and their kin, furnaces, brewing stands, crafters, and the station
    /// screens the menu engine models. Not the horse's inventory.
    pub(super) fn tweaks_on(&self) -> bool {
        if !self.settings.extras.mouse_tweaks
            || self.screen != Screen::Inventory
            || self.mount_active()
        {
            return false;
        }
        if self.remote.is_some() {
            return self.remote_storage_open();
        }
        match self.station.as_ref().map(|s| s.kind) {
            Some(kind) => matches!(
                kind,
                Kind::Merchant
                    | Kind::Loom
                    | Kind::Stonecutter
                    | Kind::Grindstone
                    | Kind::Cartography
                    | Kind::Anvil
                    | Kind::Smithing
                    | Kind::Enchanting
                    | Kind::Beacon
            ),
            None => matches!(
                self.container_kind(),
                None | Some(
                    BlockEntity::Chest(_)
                        | BlockEntity::Furnace(_)
                        | BlockEntity::Brewing(_)
                        | BlockEntity::Crafter(_)
                )
            ),
        }
    }
    /// The result slots (`ResultSlot`, `FurnaceResultSlot`, `MerchantResultSlot`): items come
    /// out of them, never in.
    fn tweak_output(&self, slot: usize) -> bool {
        match slot {
            200 => true,
            302 => matches!(self.container_kind(), Some(BlockEntity::Furnace(_))),
            _ => self.station.is_some() && slot == super::stations::RESULT,
        }
    }
    /// One ordinary click on a game slot. False when nothing took it.
    pub(super) fn tweak_click(&mut self, slot: usize, button: i32, input: Input) -> bool {
        // On a server (the integrated one too) the click runs on the menu engine first and is
        // sent with its predicted result, as a player's own click is, so the clicks that follow
        // in the same frame see the hand and slots it left.
        if !self.menu_click(Some(slot), button, input) {
            return false;
        }
        if self.remote.is_none()
            && let Some(key) = self.container
        {
            self.container_changed(key);
        }
        true
    }
    /// A press on `slot`: true when the tweaks took it (a right-drag
    /// placing), else vanilla's press runs and the tweaks watch the drag.
    pub(super) fn tweak_press(&mut self, slot: Option<usize>, right: bool, shift: bool) -> bool {
        self.tweaks.drag = None;
        let Some(slot) =
            slot.filter(|&s| s != 200 && s < Creative::DISPLAY && !self.tweak_output(s))
        else {
            return false;
        };
        if !self.tweaks_on() {
            return false;
        }
        let held = self.carried.clone();
        if right {
            if shift || held.is_none() || !self.settings.extras.mouse_tweaks_rmb {
                return false;
            }
            // RMB: the press places at once, then the drag goes on.
            let mut drag = Drag {
                right,
                held,
                active: true,
                visited: vec![slot],
            };
            self.tweak_place(&mut drag, slot);
            self.inv_drag = None;
            self.inv_clicks.skip_release();
            self.tweaks.drag = Some(drag);
            return true;
        }
        let e = &self.settings.extras;
        if (held.is_some() && e.mouse_tweaks_lmb_item) || (shift && e.mouse_tweaks_lmb_empty) {
            self.tweaks.drag = Some(Drag {
                right,
                held,
                active: false,
                visited: vec![slot],
            });
        }
        false
    }
    /// Places one held item in `slot` when the menu takes it.
    fn tweak_place(&mut self, drag: &mut Drag, slot: usize) {
        let Some(held) = drag.held.clone() else {
            return;
        };
        let (may, max) = self.tweak_slot_rule(slot, &held);
        let accepts = may
            && match self.slot_view(slot) {
                None => true,
                Some(s) => s.same_item_same_components(&held) && s.count < max,
            };
        if accepts && self.tweak_click(slot, 1, Input::Pickup) {
            drag.held = self.carried.clone();
        }
    }
    /// The menu engine's `Slot.mayPlace` and largest stack for `stack` in `slot`.
    fn tweak_slot_rule(&mut self, slot: usize, stack: &ItemStack) -> (bool, u32) {
        let mut rule = (true, 64);
        self.menu_session(|menu, env, to_menu| {
            if let Some(i) = to_menu(slot) {
                rule = menu.slot_rules(env, i, stack);
            }
            Some(false)
        });
        rule
    }
    /// The menu engine's rules per slot and item, for the wheel's plan: what its clicks accept.
    fn tweak_rules(
        &mut self,
        slots: &[mouse_tweaks::Slot],
        carried: Option<&ItemStack>,
    ) -> HashMap<(usize, Arc<str>), (bool, u32)> {
        let mut items: Vec<Arc<str>> = slots
            .iter()
            .filter_map(|s| s.stack.as_ref().map(|i| i.item.clone()))
            .collect();
        items.extend(carried.map(|c| c.item.clone()));
        items.sort();
        items.dedup();
        let ids: Vec<usize> = slots.iter().map(|s| s.id).collect();
        let mut table = HashMap::new();
        self.menu_session(|menu, env, to_menu| {
            for &id in &ids {
                let Some(i) = to_menu(id) else {
                    continue;
                };
                for item in &items {
                    let stack = ItemStack {
                        item: item.clone(),
                        count: 1,
                        damage: 0,
                        extra: None,
                    };
                    table.insert((id, item.clone()), menu.slot_rules(env, i, &stack));
                }
            }
            Some(false)
        });
        table
    }
    /// The cursor moved while a button is held: true when the tweaks own
    /// the drag (vanilla's quick craft drag doesn't grow).
    pub(super) fn tweak_drag_move(&mut self, slot: Option<usize>) -> bool {
        let Some(mut drag) = self.tweaks.drag.take() else {
            return false;
        };
        let active = drag.active;
        let entered = slot.filter(|s| {
            *s != 200
                && *s < Creative::DISPLAY
                && !self.tweak_output(*s)
                && !drag.visited.contains(s)
        });
        if let Some(slot) = entered {
            drag.visited.push(slot);
            let stack = self.slot_view(slot);
            let shift = self.has_shift_down();
            if drag.right {
                self.tweak_place(&mut drag, slot);
            } else if let Some(held) = drag.held.clone() {
                // Holding a stack: only slots of the same item, and only
                // while vanilla's drag hasn't spread.
                let spread = self.inv_drag.as_ref().is_some_and(|(_, s)| s.len() > 1);
                if let Some(s) = stack.filter(|s| s.same_item_same_components(&held))
                    && (drag.active || !spread)
                {
                    if !drag.active {
                        drag.active = true;
                        self.inv_drag = None;
                        self.inv_clicks.skip_release();
                    }
                    if shift {
                        self.tweak_click(slot, 0, Input::QuickMove);
                    } else if held.count + s.count <= self.max_stack(&s)
                        && self.tweak_click(slot, 0, Input::Pickup)
                        && self.tweak_click(slot, 0, Input::Pickup)
                    {
                        // Put the hand's stack in the slot, then take it all.
                        drag.held = self.carried.clone();
                    }
                }
            } else if shift && stack.is_some() {
                drag.active = true;
                self.tweak_click(slot, 0, Input::QuickMove);
            }
        }
        let owned = drag.active || active;
        self.tweaks.drag = Some(drag);
        owned
    }
    /// The button went up: the drag ends.
    pub(super) fn tweak_release(&mut self) {
        self.tweaks.drag = None;
    }
    /// The slots the wheel works on, in the menu's order: the inventory, the crafting grid or
    /// container across from it, the results and a station's slots. The armour, off hand and the
    /// crafter's display slot are left out.
    fn wheel_slots(&mut self) -> Vec<mouse_tweaks::Slot> {
        let ids: Vec<usize> = self
            .slots()
            .into_iter()
            .filter(|&s| s != 309 && (s < 500 || s == 200 || (600..700).contains(&s)))
            .collect();
        let mut slots = Vec::with_capacity(ids.len());
        for id in ids {
            let y = self.slot_position(id).1;
            let output = self.tweak_output(id);
            let stack = self.slot_view(id);
            slots.push(mouse_tweaks::Slot {
                id,
                own: id < 36,
                y,
                output,
                stack,
            });
        }
        slots
    }
    /// The wheel over an inventory screen: true when the tweaks used it. Over any slot of the
    /// menu it is always theirs, even when nothing moves, so the hotbar doesn't turn instead.
    pub(super) fn tweak_scroll(&mut self, delta: f32, slot: Option<usize>) -> bool {
        if !self.settings.extras.mouse_tweaks_wheel || !self.tweaks_on() {
            return false;
        }
        let slots = self.wheel_slots();
        let Some(selected) = slot.and_then(|s| slots.iter().position(|v| v.id == s)) else {
            self.tweaks.wheel.reset();
            return false;
        };
        let e = &self.settings.extras;
        let cfg = mouse_tweaks::Config {
            order: e.mouse_tweaks_order,
            direction: e.mouse_tweaks_direction,
        };
        let steps = self
            .tweaks
            .wheel
            .feed(e.mouse_tweaks_scaling, f64::from(delta));
        if steps == 0 {
            return true;
        }
        let carried = self.carried.clone();
        let table = self.tweak_rules(&slots, carried.as_ref());
        let item_max = |s: &ItemStack| self.max_stack(s);
        let rules = |id: usize, s: &ItemStack| {
            table
                .get(&(id, s.item.clone()))
                .copied()
                .unwrap_or((true, 64))
        };
        let mut menu = mouse_tweaks::Menu {
            slots,
            carried,
            item_max: &item_max,
            rules: &rules,
            clicks: Vec::new(),
        };
        let clicks = mouse_tweaks::plan_wheel(&mut menu, selected, steps, cfg);
        for (slot, right) in clicks {
            if !self.tweak_click(slot, i32::from(right), Input::Pickup) {
                break;
            }
        }
        true
    }
}

impl Game<'_> {
    /// The sort buttons of the open screen, with the extra on: their
    /// corner (from the screen's) and the slots each sorts. The container's
    /// (chest-like ones only) sits by its title, the main inventory's by
    /// the "Inventory" label's line.
    fn sort_buttons(&self) -> Vec<((f32, f32), Vec<usize>)> {
        if !self.settings.extras.sort
            || self.screen != Screen::Inventory
            || self.station.is_some()
            || self.mount_active()
        {
            return Vec::new();
        }
        let chest_like = if self.remote.is_some() {
            if !self.remote_storage_open() {
                return Vec::new();
            }
            true
        } else {
            match self.container_kind() {
                None => false,
                Some(BlockEntity::Chest(_)) => {
                    self.chest_rows().is_some() || self.container_block_is_shulker()
                }
                Some(_) => return Vec::new(),
            }
        };
        let mut buttons = Vec::new();
        let container: Vec<usize> = self
            .slots()
            .into_iter()
            .filter(|s| (300..500).contains(s))
            .collect();
        if chest_like && !container.is_empty() {
            buttons.push(((160.0, 5.0), container));
        }
        buttons.push(((160.0, self.screen_height() - 95.0), (9..36).collect()));
        buttons
    }
    /// The sort buttons: a bordered square with three bars, lighter under
    /// the cursor.
    pub(super) fn draw_sort_buttons(&self, batch: &mut HudBatch, ox: f32, oy: f32) {
        let atlas = &self.gui.atlas;
        for ((x, y), _) in self.sort_buttons() {
            let (x, y) = (ox + x, oy + y);
            let (cx, cy) = self.cursor;
            let hovered = cx >= x && cx < x + SORT_BUTTON && cy >= y && cy < y + SORT_BUTTON;
            let face = if hovered {
                rgba(220, 220, 220, 255)
            } else {
                rgba(160, 160, 160, 255)
            };
            batch.fill(atlas, x, y, SORT_BUTTON, SORT_BUTTON, rgba(55, 55, 55, 255));
            batch.fill(
                atlas,
                x + 1.0,
                y + 1.0,
                SORT_BUTTON - 2.0,
                SORT_BUTTON - 2.0,
                face,
            );
            for (i, w) in [5.0, 4.0, 3.0].into_iter().enumerate() {
                let by = y + 2.0 + 2.0 * i as f32;
                batch.fill(atlas, x + 2.0, by, w, 1.0, rgba(55, 55, 55, 255));
            }
        }
    }
    /// A left press on a sort button sorts its slots (true).
    pub(super) fn sort_press(&mut self) -> bool {
        let Some(r) = self.renderer.as_ref() else {
            return false;
        };
        let batch = HudBatch::new([r.config.width, r.config.height]);
        let (ox, oy) = self.inventory_origin(&batch);
        let (cx, cy) = self.cursor;
        let hit = self.sort_buttons().into_iter().find(|((x, y), _)| {
            let (x, y) = (ox + x, oy + y);
            cx >= x && cx < x + SORT_BUTTON && cy >= y && cy < y + SORT_BUTTON
        });
        let Some((_, slots)) = hit else {
            return false;
        };
        self.sort_slots(&slots);
        true
    }
    /// The sort key: sorts the part under the cursor (the container when
    /// the cursor is over no slot).
    pub(super) fn sort_key_press(&mut self, key: KeyCode) -> bool {
        keybinds::key_name(key).is_some_and(|name| self.sort_input(name))
    }
    pub(super) fn sort_mouse_press(&mut self, button: MouseButton) -> bool {
        self.sort_input(&keybinds::mouse_name(button))
    }
    fn sort_input(&mut self, name: &str) -> bool {
        let e = &self.settings.extras;
        if !e.sort || e.sort_key != name || self.recipe_search_focused() {
            return false;
        }
        let buttons = self.sort_buttons();
        let hovered = self.inventory_target().0;
        let slots = buttons
            .iter()
            .find(|(_, slots)| hovered.is_some_and(|h| slots.contains(&h)))
            .or(buttons.first())
            .map(|(_, slots)| slots.clone());
        let Some(slots) = slots else {
            return false;
        };
        self.sort_slots(&slots);
        true
    }
    /// Sorts `slots` with ordinary left clicks (needs an empty hand).
    fn sort_slots(&mut self, slots: &[usize]) {
        if self.carried.is_some() {
            return;
        }
        let views: Vec<Option<ItemStack>> = slots.iter().map(|&s| self.slot_view(s)).collect();
        let clicks = {
            let names = |id: &str| self.pack.names.item(id);
            let rank = |s: &ItemStack| {
                let order = self.creative_screen.order_of(s).unwrap_or(usize::MAX);
                (order, hover_name(&names, s))
            };
            let max = |s: &ItemStack| self.max_stack(s);
            inv_sort::plan(&views, &rank, &max)
        };
        for i in clicks {
            self.tweak_click(slots[i], 0, Input::Pickup);
        }
    }
}
