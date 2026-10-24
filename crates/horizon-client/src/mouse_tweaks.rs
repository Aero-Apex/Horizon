//! Mouse Tweaks' wheel (`yalter.mousetweaks.Main.onMouseScrolled`, BSD 3-clause, see
//! THIRD_PARTY_NOTICES.md): the clicks that scroll items between the slot under the cursor and
//! the other inventory, planned on a model of the open menu so they can be replayed as ordinary
//! slot clicks.
//!
//! The planner follows the original line by line: it works with a stack on the cursor, over
//! crafting outputs (one batch per step), by the original's three settings (search order,
//! scroll direction, scaling), and picks right or left clicks where the original does.
use crate::items::ItemStack;

/// `WheelSearchOrder`: where a pull looks first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchOrder {
    FirstToLast,
    #[default]
    LastToFirst,
}
impl SearchOrder {
    pub const ALL: [Self; 2] = [Self::FirstToLast, Self::LastToFirst];
    pub fn name(self) -> &'static str {
        match self {
            Self::FirstToLast => "first",
            Self::LastToFirst => "last",
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        Self::ALL.into_iter().find(|v| v.name() == s).unwrap_or(default)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::FirstToLast => "First to last",
            Self::LastToFirst => "Last to first",
        }
    }
}

/// `WheelScrollDirection`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    Normal,
    Inverted,
    PositionAware,
    PositionAwareInverted,
}
impl Direction {
    pub const ALL: [Self; 4] = [
        Self::Normal,
        Self::Inverted,
        Self::PositionAware,
        Self::PositionAwareInverted,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Inverted => "inverted",
            Self::PositionAware => "position",
            Self::PositionAwareInverted => "positionInverted",
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        Self::ALL.into_iter().find(|v| v.name() == s).unwrap_or(default)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Inverted => "Inverted",
            Self::PositionAware => "Inventory position aware",
            Self::PositionAwareInverted => "Position aware, inverted",
        }
    }
    fn inverted(self) -> bool {
        matches!(self, Self::Inverted | Self::PositionAwareInverted)
    }
    fn position_aware(self) -> bool {
        matches!(self, Self::PositionAware | Self::PositionAwareInverted)
    }
}

/// `ScrollItemScaling`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scaling {
    #[default]
    Proportional,
    AlwaysOne,
}
impl Scaling {
    pub const ALL: [Self; 2] = [Self::Proportional, Self::AlwaysOne];
    pub fn name(self) -> &'static str {
        match self {
            Self::Proportional => "proportional",
            Self::AlwaysOne => "one",
        }
    }
    pub fn parse(s: &str, default: Self) -> Self {
        Self::ALL.into_iter().find(|v| v.name() == s).unwrap_or(default)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Proportional => "Proportional",
            Self::AlwaysOne => "Always one",
        }
    }
    /// The scroll distance as items to move, the sign the direction.
    pub fn scale(self, delta: f64) -> f64 {
        match self {
            Self::Proportional => delta,
            Self::AlwaysOne => delta.signum(),
        }
    }
}

/// The wheel settings that shape the plan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Config {
    pub order: SearchOrder,
    pub direction: Direction,
}

/// Wheel distance not yet worth a whole item (a trackpad scrolls in fractions), as `Main`
/// keeps it in `accumulatedScrollDelta`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Accumulator(f64);
impl Accumulator {
    pub fn reset(&mut self) {
        self.0 = 0.0;
    }
    /// Adds a scroll and takes the whole items out of it (negative: down).
    pub fn feed(&mut self, scaling: Scaling, delta: f64) -> i32 {
        let scaled = scaling.scale(delta);
        if self.0 != 0.0 && scaled.signum() != self.0.signum() {
            self.0 = 0.0;
        }
        self.0 += scaled;
        let whole = self.0.trunc();
        self.0 -= whole;
        whole as i32
    }
}

/// One slot of the open menu, in the menu's order.
#[derive(Clone, Debug)]
pub struct Slot {
    pub id: usize,
    /// Its container is the player's inventory.
    pub own: bool,
    /// Where it sits on screen (up is smaller), for the position aware directions.
    pub y: f32,
    /// A crafting, smelting or trading result.
    pub output: bool,
    pub stack: Option<ItemStack>,
}

/// The menu as the planner sees it: its slots, the stack on the cursor and the rules.
pub struct Menu<'a> {
    pub slots: Vec<Slot>,
    pub carried: Option<ItemStack>,
    /// `Item.getMaxStackSize` of a stack.
    pub item_max: &'a dyn Fn(&ItemStack) -> u32,
    /// `Slot.mayPlace` and `Slot.getMaxStackSize` of a slot for a stack.
    pub rules: &'a dyn Fn(usize, &ItemStack) -> (bool, u32),
    /// The clicks made so far: slot id and whether it was the right button.
    pub clicks: Vec<(usize, bool)>,
}

impl Menu<'_> {
    fn limit(&self, i: usize, stack: &ItemStack) -> u32 {
        (self.item_max)(stack).min((self.rules)(self.slots[i].id, stack).1)
    }
    fn may_place(&self, i: usize, stack: &ItemStack) -> bool {
        (self.rules)(self.slots[i].id, stack).0
    }
    fn count(&self, i: usize) -> u32 {
        self.slots[i].stack.as_ref().map_or(0, |s| s.count)
    }
    fn held(&self) -> u32 {
        self.carried.as_ref().map_or(0, |s| s.count)
    }

    /// One `PICKUP` click, as `AbstractContainerMenu.doClick` does it, on slot index `i`.
    fn click(&mut self, i: usize, right: bool) {
        self.clicks.push((self.slots[i].id, right));
        let slot = self.slots[i].stack.take();
        let carried = self.carried.take();
        if self.slots[i].output {
            // Taking a result: all of it, or added to a stack of the same item that has room.
            match (carried, slot) {
                (None, Some(s)) => {
                    self.carried = Some(s.clone());
                    // The craft is still the same one: the result stays.
                    self.slots[i].stack = Some(s);
                }
                (Some(mut c), Some(s)) => {
                    if c.same_item_same_components(&s)
                        && c.count + s.count <= (self.item_max)(&c)
                    {
                        c.count += s.count;
                    }
                    self.carried = Some(c);
                    self.slots[i].stack = Some(s);
                }
                (c, None) => self.carried = c,
            }
            return;
        }
        match (carried, slot) {
            (None, None) => {}
            // Take all, or half rounded up with the right button.
            (None, Some(mut s)) => {
                let take = if right { s.count.div_ceil(2) } else { s.count };
                let mut got = s.clone();
                got.count = take;
                s.count -= take;
                self.carried = Some(got);
                self.slots[i].stack = (s.count > 0).then_some(s);
            }
            (Some(mut c), None) => {
                if self.may_place(i, &c) {
                    let put = if right { 1 } else { c.count.min(self.limit(i, &c)) };
                    let mut placed = c.clone();
                    placed.count = put;
                    c.count -= put;
                    self.slots[i].stack = Some(placed);
                }
                self.carried = (c.count > 0).then_some(c);
            }
            (Some(mut c), Some(mut s)) if c.same_item_same_components(&s) => {
                let room = self.limit(i, &s).saturating_sub(s.count);
                let put = if right { 1 } else { c.count }.min(room);
                s.count += put;
                c.count -= put;
                self.slots[i].stack = Some(s);
                self.carried = (c.count > 0).then_some(c);
            }
            (Some(c), Some(s)) => {
                // Another item: they swap when the slot takes the hand's stack whole.
                if self.may_place(i, &c) && c.count <= self.limit(i, &c) {
                    self.slots[i].stack = Some(c);
                    self.carried = Some(s);
                } else {
                    self.slots[i].stack = Some(s);
                    self.carried = Some(c);
                }
            }
        }
    }
}

/// `Main.areStacksCompatible`: empty stacks match anything, the others the same item with the
/// same components.
fn compatible(a: &Option<ItemStack>, b: &Option<ItemStack>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.same_item_same_components(b),
        _ => true,
    }
}

/// `Main.otherInventoryIsAbove`: more of the other inventory's slots sit above the selected
/// slot than below it.
fn other_inventory_is_above(menu: &Menu, selected: usize) -> bool {
    let own = menu.slots[selected].own;
    let y = menu.slots[selected].y;
    let (mut above, mut below) = (0, 0);
    for s in menu.slots.iter().filter(|s| s.own != own) {
        if s.y < y {
            above += 1;
        } else {
            below += 1;
        }
    }
    above > below
}

/// `Main.findPullSlot`: the slot to pull from, searched in the configured order on the other
/// side.
fn find_pull_slot(menu: &Menu, selected: usize, order: SearchOrder) -> Option<usize> {
    let want_own = !menu.slots[selected].own;
    let stack = &menu.slots[selected].stack;
    let mut indices: Vec<usize> = (0..menu.slots.len()).collect();
    if order == SearchOrder::LastToFirst {
        indices.reverse();
    }
    indices.into_iter().find(|&i| {
        let s = &menu.slots[i];
        s.own == want_own && s.stack.is_some() && compatible(stack, &s.stack)
    })
}

/// `Main.findPushSlots`: the slots on the other side to put `count` items into, stacks with
/// room first, then empty slots; `None` when `must_all` and not everything fits.
fn find_push_slots(
    menu: &Menu,
    selected: usize,
    mut count: u32,
    must_all: bool,
) -> Option<Vec<usize>> {
    let want_own = !menu.slots[selected].own;
    let stack = menu.slots[selected].stack.clone()?;
    let mut found = Vec::new();
    let mut empties = Vec::new();
    for (i, s) in menu.slots.iter().enumerate() {
        if count == 0 {
            break;
        }
        if s.own != want_own || s.output {
            continue;
        }
        match &s.stack {
            None => {
                if menu.may_place(i, &stack) {
                    empties.push(i);
                }
            }
            Some(other) => {
                let max = menu.limit(i, other);
                if other.same_item_same_components(&stack) && other.count < max {
                    found.push(i);
                    count -= count.min(max - other.count);
                }
            }
        }
    }
    for i in empties {
        if count == 0 {
            break;
        }
        found.push(i);
        count -= count.min(menu.limit(i, &stack));
    }
    if must_all && count > 0 { None } else { Some(found) }
}

/// The clicks (slot id, right button) one wheel step of `delta` items makes over the slot at
/// index `selected`: positive pulls items in from the other inventory, negative pushes them
/// out. Empty when nothing can move. `menu` is left as the clicks leave it.
pub fn plan_wheel(menu: &mut Menu, selected: usize, delta: i32, cfg: Config) -> Vec<(usize, bool)> {
    menu.clicks.clear();
    if delta == 0 || menu.slots[selected].stack.is_none() {
        return Vec::new();
    }
    let mut num = delta.unsigned_abs();
    let mut push = delta < 0;
    if cfg.direction.position_aware() && other_inventory_is_above(menu, selected) {
        push = !push;
    }
    if cfg.direction.inverted() {
        push = !push;
    }
    let selected_stack = menu.slots[selected].stack.clone().expect("checked above");
    let on_mouse = menu.carried.clone();

    if menu.slots[selected].output {
        if !compatible(&Some(selected_stack.clone()), &on_mouse) {
            return Vec::new();
        }
        if on_mouse.is_none() {
            if !push {
                return Vec::new();
            }
            while num > 0 {
                num -= 1;
                let count = menu.count(selected);
                let Some(targets) = find_push_slots(menu, selected, count, true) else {
                    break;
                };
                menu.click(selected, false);
                let last = targets.len().saturating_sub(1);
                for (n, &t) in targets.iter().enumerate() {
                    if n == last {
                        menu.click(t, false);
                    } else {
                        let stack = menu.slots[t].stack.clone().expect("holds a stack to fill");
                        for _ in 0..menu.limit(t, &stack).saturating_sub(stack.count) {
                            menu.click(t, true);
                        }
                    }
                }
            }
        } else {
            while num > 0 {
                num -= 1;
                menu.click(selected, false);
            }
        }
        return std::mem::take(&mut menu.clicks);
    }

    // It's not possible to interact with the slot cleanly while holding a stack of its item.
    if on_mouse.is_some() && compatible(&Some(selected_stack.clone()), &on_mouse) {
        return Vec::new();
    }

    if push {
        if let Some(c) = &on_mouse
            && !menu.may_place(selected, c)
        {
            return Vec::new();
        }
        num = num.min(selected_stack.count);
        let targets = find_push_slots(menu, selected, num, false).unwrap_or_default();
        if targets.is_empty() {
            return Vec::new();
        }
        let had = on_mouse.is_some();
        // Right click picks up one at a time and swaps with a stack in hand; left click is for
        // taking a whole stack that is all we want.
        let right = !(on_mouse.is_none() && selected_stack.count <= num);
        menu.click(selected, right);
        let picked = menu.held();
        num = num.min(picked);
        for t in targets {
            let Some(c) = menu.carried.clone() else {
                break;
            };
            let existing = menu.count(t);
            let times = menu.limit(t, &c).saturating_sub(existing).min(num);
            num -= times;
            for _ in 0..times {
                menu.click(t, true);
            }
        }
        let leftover = menu.carried.is_some();
        if had || leftover {
            menu.click(selected, had && leftover);
        }
        return std::mem::take(&mut menu.clicks);
    }

    // Pull.
    let mut max_move = menu.limit(selected, &selected_stack).saturating_sub(selected_stack.count);
    num = num.min(max_move);
    while num > 0 {
        let Some(target) = find_pull_slot(menu, selected, cfg.order) else {
            break;
        };
        let target_stack = menu.slots[target].stack.clone().expect("found with a stack");
        if menu.slots[target].output {
            // One crafted batch per tick, and only when it all fits.
            if max_move < target_stack.count {
                break;
            }
            max_move -= target_stack.count;
            num = (num - 1).min(max_move);
            if let Some(c) = &on_mouse
                && !menu.may_place(selected, c)
            {
                break;
            }
            menu.click(selected, false);
            menu.click(target, false);
            menu.click(selected, false);
            continue;
        }
        let had = on_mouse.is_some();
        if let Some(c) = &on_mouse
            && !menu.may_place(target, c)
        {
            break;
        }
        let right = !(on_mouse.is_none() && target_stack.count == 1);
        menu.click(target, right);
        let picked = menu.held();
        let moving = picked.min(num);
        if moving == picked {
            menu.click(selected, false);
        } else {
            for _ in 0..moving {
                menu.click(selected, true);
            }
        }
        max_move = max_move.saturating_sub(moving);
        num -= moving;
        let leftover = menu.carried.is_some();
        if had || leftover {
            menu.click(target, had && leftover);
        }
    }
    std::mem::take(&mut menu.clicks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(item: &str, count: u32) -> Option<ItemStack> {
        Some(ItemStack {
            item: item.into(),
            count,
            damage: 0,
            extra: None,
        })
    }
    fn slot(id: usize, own: bool, y: f32, s: Option<ItemStack>) -> Slot {
        Slot {
            id,
            own,
            y,
            output: false,
            stack: s,
        }
    }
    fn max(s: &ItemStack) -> u32 {
        if s.item.contains("sword") { 1 } else { 64 }
    }
    fn rules(_: usize, _: &ItemStack) -> (bool, u32) {
        (true, 64)
    }
    fn menu(slots: Vec<Slot>, carried: Option<ItemStack>) -> Menu<'static> {
        Menu {
            slots,
            carried,
            item_max: &max,
            rules: &rules,
            clicks: Vec::new(),
        }
    }
    fn counts(m: &Menu) -> Vec<u32> {
        m.slots.iter().map(|s| s.stack.as_ref().map_or(0, |s| s.count)).collect()
    }

    /// A chest slot (0) over inventory slots (1, 2).
    fn chest(a: Option<ItemStack>, b: Option<ItemStack>, c: Option<ItemStack>) -> Vec<Slot> {
        vec![slot(0, false, 0.0, a), slot(1, true, 50.0, b), slot(2, true, 70.0, c)]
    }

    #[test]
    fn scrolling_down_moves_one_item_to_a_stack_with_room() {
        let mut m = menu(chest(stack("dirt", 10), stack("dirt", 5), None), None);
        let clicks = plan_wheel(&mut m, 0, -1, Config::default());
        assert_eq!(counts(&m), vec![9, 6, 0]);
        // Right click takes half, then one is put down, then the rest goes back.
        assert_eq!(clicks.first(), Some(&(0, true)));
        assert!(m.carried.is_none());
    }

    #[test]
    fn scrolling_down_uses_an_empty_slot_when_no_stack_has_room() {
        let mut m = menu(chest(stack("dirt", 3), stack("stone", 5), None), None);
        plan_wheel(&mut m, 0, -1, Config::default());
        assert_eq!(counts(&m), vec![2, 5, 1]);
    }

    #[test]
    fn a_single_item_is_taken_with_a_left_click() {
        let mut m = menu(chest(stack("dirt", 1), None, None), None);
        let clicks = plan_wheel(&mut m, 0, -1, Config::default());
        assert_eq!(clicks[0], (0, false));
        assert_eq!(counts(&m), vec![0, 1, 0]);
    }

    #[test]
    fn scrolling_up_pulls_from_the_other_side_last_slot_first() {
        let mut m = menu(chest(stack("dirt", 10), stack("dirt", 5), stack("dirt", 7)), None);
        plan_wheel(&mut m, 0, 1, Config::default());
        assert_eq!(counts(&m), vec![11, 5, 6]);
        let mut m = menu(chest(stack("dirt", 10), stack("dirt", 5), stack("dirt", 7)), None);
        let cfg = Config {
            order: SearchOrder::FirstToLast,
            ..Config::default()
        };
        plan_wheel(&mut m, 0, 1, cfg);
        assert_eq!(counts(&m), vec![11, 4, 7]);
    }

    #[test]
    fn several_ticks_move_several_items() {
        let mut m = menu(chest(stack("dirt", 10), None, None), None);
        plan_wheel(&mut m, 0, -3, Config::default());
        assert_eq!(counts(&m).iter().sum::<u32>(), 10);
        assert_eq!(counts(&m)[1], 3);
    }

    #[test]
    fn inverted_and_position_aware_directions_flip_it() {
        let cfg = Config {
            direction: Direction::Inverted,
            ..Config::default()
        };
        // Down now pulls.
        let mut m = menu(chest(stack("dirt", 10), stack("dirt", 5), None), None);
        plan_wheel(&mut m, 0, -1, cfg);
        assert_eq!(counts(&m), vec![11, 4, 0]);
        // The chest is above the inventory: position aware sends scroll-down on the
        // inventory's slots toward the chest (the other inventory is above) as "up".
        let aware = Config {
            direction: Direction::PositionAware,
            ..Config::default()
        };
        let mut m = menu(chest(stack("dirt", 10), stack("dirt", 5), None), None);
        // Over the inventory slot, the chest is above: down pulls from it.
        plan_wheel(&mut m, 1, -1, aware);
        assert_eq!(counts(&m), vec![9, 6, 0]);
        // Over the chest slot, the inventory is below: down pushes.
        let mut m = menu(chest(stack("dirt", 10), stack("dirt", 5), None), None);
        plan_wheel(&mut m, 0, -1, aware);
        assert_eq!(counts(&m), vec![9, 6, 0]);
    }

    #[test]
    fn holding_a_different_item_it_is_put_back_after() {
        let mut m = menu(chest(stack("dirt", 10), None, None), stack("stone", 4));
        plan_wheel(&mut m, 0, -1, Config::default());
        assert_eq!(m.carried, stack("stone", 4));
        assert_eq!(counts(&m), vec![9, 1, 0]);
    }

    #[test]
    fn holding_the_same_item_does_nothing() {
        let mut m = menu(chest(stack("dirt", 10), None, None), stack("dirt", 4));
        assert!(plan_wheel(&mut m, 0, -1, Config::default()).is_empty());
        assert!(plan_wheel(&mut m, 0, 1, Config::default()).is_empty());
    }

    #[test]
    fn nothing_moves_from_an_empty_slot_or_with_no_room() {
        let mut m = menu(chest(None, stack("dirt", 5), None), None);
        assert!(plan_wheel(&mut m, 0, 1, Config::default()).is_empty());
        let mut m = menu(chest(stack("dirt", 64), stack("dirt", 5), None), None);
        assert!(plan_wheel(&mut m, 0, 1, Config::default()).is_empty());
        let mut full = chest(stack("dirt", 4), stack("stone", 5), stack("stone", 5));
        full[1].stack = stack("stone", 5);
        let mut m = menu(full, None);
        assert!(plan_wheel(&mut m, 0, -1, Config::default()).is_empty());
    }

    #[test]
    fn a_crafting_result_is_pushed_a_batch_at_a_time() {
        let mut slots = vec![
            Slot {
                id: 0,
                own: false,
                y: 0.0,
                output: true,
                stack: stack("stick", 4),
            },
            slot(1, true, 50.0, None),
            slot(2, true, 70.0, None),
        ];
        slots[0].own = false;
        let mut m = menu(slots, None);
        plan_wheel(&mut m, 0, -1, Config::default());
        assert_eq!(m.carried, None);
        assert_eq!(counts(&m)[1], 4);
        // Scrolling up over a result pulls nothing.
        let mut m = menu(
            vec![Slot {
                id: 0,
                own: false,
                y: 0.0,
                output: true,
                stack: stack("stick", 4),
            }],
            None,
        );
        assert!(plan_wheel(&mut m, 0, 1, Config::default()).is_empty());
    }

    #[test]
    fn pulling_a_crafted_batch_into_a_stack() {
        let slots = vec![
            slot(0, true, 50.0, stack("stick", 8)),
            Slot {
                id: 1,
                own: false,
                y: 0.0,
                output: true,
                stack: stack("stick", 4),
            },
        ];
        let mut m = menu(slots, None);
        let clicks = plan_wheel(&mut m, 0, 1, Config::default());
        assert_eq!(clicks, vec![(0, false), (1, false), (0, false)]);
        assert_eq!(counts(&m)[0], 12);
    }

    #[test]
    fn the_scroll_accumulates_fractions_and_forgets_on_a_turn() {
        let mut a = Accumulator::default();
        assert_eq!(a.feed(Scaling::Proportional, 0.4), 0);
        assert_eq!(a.feed(Scaling::Proportional, 0.4), 0);
        assert_eq!(a.feed(Scaling::Proportional, 0.4), 1);
        assert_eq!(a.feed(Scaling::Proportional, -0.5), 0);
        assert_eq!(a.feed(Scaling::Proportional, -2.5), -3);
        assert_eq!(a.feed(Scaling::AlwaysOne, 0.2), 1);
        assert_eq!(a.feed(Scaling::AlwaysOne, -3.0), -1);
    }

    #[test]
    fn click_rules_follow_the_menu() {
        let mut m = menu(vec![slot(0, true, 0.0, stack("dirt", 5))], None);
        m.click(0, true);
        assert_eq!((counts(&m), m.held()), (vec![2], 3));
        m.click(0, true);
        assert_eq!((counts(&m), m.held()), (vec![3], 2));
        m.carried = stack("stone", 2);
        m.click(0, false);
        assert_eq!(m.carried, stack("dirt", 3));
        assert_eq!(m.slots[0].stack, stack("stone", 2));
    }

    #[test]
    fn settings_round_trip_by_name() {
        for v in SearchOrder::ALL {
            assert_eq!(SearchOrder::parse(v.name(), SearchOrder::default()), v);
        }
        for v in Direction::ALL {
            assert_eq!(Direction::parse(v.name(), Direction::default()), v);
        }
        for v in Scaling::ALL {
            assert_eq!(Scaling::parse(v.name(), Scaling::default()), v);
        }
        assert_eq!(Direction::parse("nope", Direction::Inverted), Direction::Inverted);
    }
}
