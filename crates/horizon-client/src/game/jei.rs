//! Just Enough Items in the game (`horizon_client::jei`, docs/extras.md): the
//! item list beside the inventory, with a search box, and the recipe window
//! for what makes an item and what uses it. It rides on the inventory and
//! creative screens: its draw call is the last of theirs, and the mouse,
//! wheel and keys reach it first.
use super::{Game, Screen};
use horizon_client::{
    hud::{HudBatch, rgba},
    hud_layout::{self, Blend, GuiDraw},
    items::ItemStack,
    jei::{self as logic, Category, Panel, Query, RecipeView, SLOT},
    recipe_table::RecipeTable,
    ui::EditBox,
};
use std::{borrow::Cow, sync::Arc, time::Instant};
use winit::{event::KeyEvent, keyboard::KeyCode};

/// Side of the recipe window's small buttons and the page arrows.
const BUTTON: f32 = 12.0;
/// How many views Backspace can go back through.
const HISTORY: usize = 32;

/// Everything the overlay keeps between frames.
pub(super) struct JeiUi {
    search: EditBox,
    page: usize,
    view: Option<RecipeView>,
    /// What the window showed before, for Backspace.
    history: Vec<RecipeView>,
    /// The item list's geometry from the last frame.
    panel: Option<Panel>,
    /// Item slots of the last frame (top left of the 16 x 16 item) for clicks
    /// and the R and U keys: the list, the window and the menu's own.
    hits: Vec<(f32, f32, String)>,
    /// The item under the pointer in the last frame.
    hover: Option<String>,
    /// The recipe window's box and its previous and next buttons.
    window: Option<[f32; 4]>,
    step: [[f32; 4]; 2],
    /// The page arrows of the list.
    pages: [[f32; 4]; 2],
    /// Indices into the creative screen's items for `term`.
    results: Vec<usize>,
    term: Option<String>,
    table: Option<Arc<RecipeTable>>,
    /// A press the overlay took: its release must not reach the screen.
    swallow: bool,
    started: Instant,
}
impl Default for JeiUi {
    fn default() -> Self {
        let mut search = EditBox::new(0.0, 0.0, 100.0, 15.0);
        search.focused = false;
        search.max_length = 48;
        Self {
            search,
            page: 0,
            view: None,
            history: Vec::new(),
            panel: None,
            hits: Vec::new(),
            hover: None,
            window: None,
            step: [[0.0; 4]; 2],
            pages: [[0.0; 4]; 2],
            results: Vec::new(),
            term: None,
            table: None,
            swallow: false,
            started: Instant::now(),
        }
    }
}

fn inside(p: (f32, f32), r: [f32; 4]) -> bool {
    p.0 >= r[0] && p.1 >= r[1] && p.0 < r[0] + r[2] && p.1 < r[1] + r[3]
}

/// A slot as the container screens draw it: grey, dark top left, white
/// bottom right.
fn slot_box(batch: &mut HudBatch, atlas: &horizon_client::hud::HudAtlas, x: f32, y: f32) {
    batch.fill(atlas, x, y, SLOT, SLOT, rgba(139, 139, 139, 255));
    batch.fill(atlas, x, y, SLOT - 1.0, 1.0, rgba(55, 55, 55, 255));
    batch.fill(atlas, x, y, 1.0, SLOT - 1.0, rgba(55, 55, 55, 255));
    batch.fill(atlas, x + 1.0, y + SLOT - 1.0, SLOT - 1.0, 1.0, rgba(255, 255, 255, 255));
    batch.fill(atlas, x + SLOT - 1.0, y + 1.0, 1.0, SLOT - 1.0, rgba(255, 255, 255, 255));
}

/// A small square button with a symbol in it.
fn button(
    batch: &mut HudBatch,
    atlas: &horizon_client::hud::HudAtlas,
    r: [f32; 4],
    label: &str,
    hover: bool,
) {
    let fill = if hover {
        rgba(160, 160, 190, 255)
    } else {
        rgba(110, 110, 110, 255)
    };
    batch.fill(atlas, r[0], r[1], r[2], r[3], rgba(20, 20, 20, 255));
    batch.fill(atlas, r[0] + 1.0, r[1] + 1.0, r[2] - 2.0, r[3] - 2.0, fill);
    let tw = HudBatch::text_width(atlas, label);
    batch.text(
        atlas,
        label,
        (r[0] + (r[2] - tw) / 2.0).round(),
        (r[1] + (r[3] - 8.0) / 2.0).round(),
        u32::MAX,
        true,
    );
}

/// The arrow between a recipe's inputs and its output, 24 wide and 16 tall.
fn arrow(batch: &mut HudBatch, atlas: &horizon_client::hud::HudAtlas, x: f32, y: f32) {
    let grey = rgba(139, 139, 139, 255);
    batch.fill(atlas, x, y + 6.0, 16.0, 4.0, grey);
    for i in 0..8 {
        let half = (8 - i) as f32;
        batch.fill(atlas, x + 16.0 + i as f32, y + 8.0 - half, 1.0, half * 2.0, grey);
    }
}

fn sprite(name: &'static str, x: i32, y: i32, size: i32) -> GuiDraw {
    GuiDraw::Sprite {
        sprite: Cow::Borrowed(name),
        x,
        y,
        w: size,
        h: size,
        region: None,
        color: u32::MAX,
        blend: Blend::Textured,
    }
}

impl Game<'_> {
    /// Whether the overlay is up: the extra is on and an inventory screen is.
    pub(super) fn jei_on(&self) -> bool {
        self.settings.extras.jei
            && self.title.is_none()
            && matches!(self.screen, Screen::Inventory | Screen::Creative)
    }

    /// The search results and the recipes, brought up to date.
    fn jei_refresh(&mut self) {
        let term = self.jei.search.value.clone();
        if self.jei.term.as_deref() != Some(term.as_str()) {
            self.jei.results = self.creative_screen.search_results(&term, &self.pack.names);
            self.jei.term = Some(term);
            self.jei.page = 0;
        }
        if self.jei.table.is_none() {
            let table = RecipeTable::load(&self.ui.vanilla_root);
            log::info!("Horizon: {} recipes for the item list", table.len());
            self.jei.table = Some(Arc::new(table));
        }
    }

    /// Opens a recipe window for `query`, remembering the one it replaces.
    fn jei_open(&mut self, query: Query) {
        self.jei_refresh();
        let Some(table) = self.jei.table.clone() else {
            return;
        };
        match RecipeView::new(&table, query.clone()) {
            Some(view) => {
                if let Some(old) = self.jei.view.take() {
                    self.jei.history.push(old);
                    if self.jei.history.len() > HISTORY {
                        self.jei.history.remove(0);
                    }
                }
                self.jei.view = Some(view);
            }
            None => {
                let name = self.translate(
                    &format!("item.{}", query.item().replace(':', ".")),
                    &horizon_client::jade::title_case(query.item()),
                );
                let what = match query {
                    Query::Output(_) => "No recipe makes",
                    Query::Uses(_) => "No recipe uses",
                };
                self.message = Some((format!("{what} {name}"), Instant::now()));
            }
        }
    }

    /// Backspace: the window the current one replaced, or closed.
    fn jei_back(&mut self) {
        self.jei.view = self.jei.history.pop();
    }

    /// Whether a recipe window is up (the menu's own tooltips give way to it).
    pub(super) fn jei_view_open(&self) -> bool {
        self.jei_on() && self.jei.view.is_some()
    }

    /// Text typed into the search box (the control socket's `type`).
    pub(super) fn jei_type(&mut self, text: &str) {
        self.jei.search.insert_text(text);
    }

    /// Escape: leaves the search box, then closes the window. True when it did.
    pub(super) fn jei_escape(&mut self) -> bool {
        if !self.jei_on() {
            return false;
        }
        if self.jei.search.focused {
            self.jei.search.focused = false;
            return true;
        }
        if self.jei.view.take().is_some() {
            self.jei.history.clear();
            return true;
        }
        false
    }

    /// Whether typed keys go to the search box.
    pub(super) fn jei_search_focused(&self) -> bool {
        self.jei_on() && self.jei.search.focused
    }

    /// A key press while the search box has the focus.
    pub(super) fn jei_search_key(&mut self, event: &KeyEvent) {
        use winit::keyboard::PhysicalKey;
        if let Some(shortcut) = self.edit_shortcut(event) {
            self.jei.search.shortcut(shortcut, &mut self.clipboard);
            return;
        }
        let ctrl = self.has_control_down();
        let shift = self.has_shift_down();
        let handled = match event.physical_key {
            PhysicalKey::Code(code) => self.jei.search.key(code, ctrl, shift),
            _ => false,
        };
        if !handled
            && !ctrl
            && let Some(text) = &event.text
        {
            let text: String = text.chars().filter(|c| !c.is_control()).collect();
            self.jei.search.insert_text(&text);
        }
        self.jei.search.reset_blink();
    }

    /// R and U over an item show how it is made and what uses it; Backspace
    /// goes back. True when the key was taken.
    pub(super) fn jei_hover_key(&mut self, code: KeyCode) -> bool {
        if !self.jei_on() || self.jei.search.focused {
            return false;
        }
        match code {
            KeyCode::KeyR | KeyCode::KeyU => {
                let Some(item) = self.jei.hover.clone() else {
                    return false;
                };
                self.jei_open(if code == KeyCode::KeyR {
                    Query::Output(item)
                } else {
                    Query::Uses(item)
                });
                true
            }
            KeyCode::Backspace if self.jei.view.is_some() => {
                self.jei_back();
                true
            }
            _ => false,
        }
    }

    /// A mouse press or release. True when the overlay took it.
    pub(super) fn jei_mouse(&mut self, pressed: bool, right: bool) -> bool {
        if !self.jei_on() {
            self.jei.swallow = false;
            return false;
        }
        if !pressed {
            return std::mem::take(&mut self.jei.swallow);
        }
        let p = self.cursor;
        // The recipe window takes every click while it is up.
        if let Some(window) = self.jei.window.filter(|_| self.jei.view.is_some()) {
            self.jei.swallow = true;
            if inside(p, self.jei.step[0]) {
                if let Some(v) = self.jei.view.as_mut() {
                    v.step(false);
                }
            } else if inside(p, self.jei.step[1]) {
                if let Some(v) = self.jei.view.as_mut() {
                    v.step(true);
                }
            } else if let Some(item) = self.jei_hit_at(p) {
                self.jei_open(if right {
                    Query::Uses(item)
                } else {
                    Query::Output(item)
                });
            } else if !inside(p, window) {
                self.jei.view = None;
                self.jei.history.clear();
            }
            return true;
        }
        let Some(panel) = self.jei.panel else {
            return false;
        };
        // The search box.
        let search = [
            self.jei.search.x,
            self.jei.search.y,
            self.jei.search.width,
            self.jei.search.height,
        ];
        if inside(p, search) {
            let shift = self.has_shift_down();
            self.jei.search.focused = true;
            self.jei.search.click(&self.gui.atlas, p.0, shift, false);
            self.jei.swallow = true;
            return true;
        }
        self.jei.search.focused = false;
        // Page arrows.
        let pages = panel.pages(self.jei.results.len());
        for (i, forward) in [false, true].into_iter().enumerate() {
            if inside(p, self.jei.pages[i]) {
                self.jei.page = if forward {
                    (self.jei.page + 1) % pages
                } else {
                    (self.jei.page + pages - 1) % pages
                };
                self.jei.swallow = true;
                return true;
            }
        }
        // An item of the list.
        if let Some(i) = panel.slot_at(p.0, p.1, self.jei.page, self.jei.results.len()) {
            let stacks = self.creative_screen.search_items();
            if let Some(stack) = self.jei.results.get(i).and_then(|&k| stacks.get(k)) {
                let item = stack.item.to_string();
                self.jei_open(if right {
                    Query::Uses(item)
                } else {
                    Query::Output(item)
                });
            }
            self.jei.swallow = true;
            return true;
        }
        false
    }

    /// The wheel: steps through recipes, or turns the list's pages. True when
    /// the overlay took it.
    pub(super) fn jei_wheel(&mut self, d: f32) -> bool {
        if !self.jei_on() || d == 0.0 {
            return false;
        }
        if let Some(view) = self.jei.view.as_mut() {
            view.step(d < 0.0);
            return true;
        }
        let Some(panel) = self.jei.panel else {
            return false;
        };
        if self.cursor.0 < panel.x - 4.0 {
            return false;
        }
        let pages = panel.pages(self.jei.results.len());
        self.jei.page = if d < 0.0 {
            (self.jei.page + 1) % pages
        } else {
            (self.jei.page + pages - 1) % pages
        };
        true
    }

    /// The item whose slot the point is on, in the last frame.
    fn jei_hit_at(&self, p: (f32, f32)) -> Option<String> {
        self.jei
            .hits
            .iter()
            .find(|(x, y, _)| inside(p, [*x, *y, 16.0, 16.0]))
            .map(|(_, _, item)| item.clone())
    }

    /// The overlay over the inventory or creative screen that is drawn.
    pub(super) fn draw_jei_screen(&mut self, batch: &mut HudBatch) {
        let (ox, _) = self.inventory_origin(batch);
        let width = if self.screen == Screen::Creative {
            horizon_client::creative::Creative::WIDTH
        } else {
            176.0
        };
        self.draw_jei(batch, ox + width);
    }

    /// The overlay for this frame. `menu_right` is where the open menu ends.
    fn draw_jei(&mut self, batch: &mut HudBatch, menu_right: f32) {
        self.jei.hits.clear();
        self.jei.window = None;
        self.jei.hover = None;
        if !self.jei_on() {
            self.jei.panel = None;
            return;
        }
        self.jei_refresh();
        let Some(panel) = logic::panel(batch.width, batch.height, menu_right) else {
            self.jei.panel = None;
            return;
        };
        self.jei.panel = Some(panel);
        let cursor = self.cursor;
        let items = self.jei.results.len();
        let pages = panel.pages(items);
        self.jei.page = self.jei.page.min(pages - 1);
        let page = self.jei.page;

        // The list: this page's stacks.
        let stacks: Vec<ItemStack> = {
            let all = self.creative_screen.search_items();
            (0..panel.per_page())
                .filter_map(|n| self.jei.results.get(page * panel.per_page() + n))
                .filter_map(|&k| all.get(k).cloned())
                .collect()
        };
        {
            let atlas = &self.gui.atlas;
            for n in 0..stacks.len() {
                let (x, y) = panel.slot_origin(n);
                slot_box(batch, atlas, x, y);
            }
        }
        for (n, stack) in stacks.iter().enumerate() {
            let (x, y) = panel.slot_origin(n);
            self.draw_item(batch, stack, x + 1.0, y + 1.0);
            self.jei.hits.push((x + 1.0, y + 1.0, stack.item.to_string()));
        }

        // The page row and the search box.
        let atlas = &self.gui.atlas;
        let width = panel.width();
        let arrows = [
            [panel.x, panel.page_y, BUTTON, BUTTON],
            [panel.x + width - BUTTON, panel.page_y, BUTTON, BUTTON],
        ];
        self.jei.pages = arrows;
        button(batch, atlas, arrows[0], "<", inside(cursor, arrows[0]));
        button(batch, atlas, arrows[1], ">", inside(cursor, arrows[1]));
        let label = format!("{}/{}", page + 1, pages);
        let tw = HudBatch::text_width(atlas, &label);
        batch.text(
            atlas,
            &label,
            (panel.x + (width - tw) / 2.0).round(),
            panel.page_y + 2.0,
            u32::MAX,
            true,
        );
        let search = &mut self.jei.search;
        search.x = panel.x;
        search.y = panel.search_y;
        search.width = width;
        search.draw(batch, atlas, None);
        if self.jei.search.value.is_empty() && !self.jei.search.focused {
            batch.text(
                atlas,
                "Search...",
                panel.x + 4.0,
                panel.search_y + 4.0,
                rgba(120, 120, 120, 255),
                false,
            );
        }

        // The hovered slot of the open menu feeds the R and U keys.
        if self.jei.view.is_none()
            && let Some(slot) = self.slot_at(batch, cursor.0, cursor.1)
            && let Some(stack) = self.slot_view(slot)
        {
            self.jei.hover = Some(stack.item.to_string());
        }
        // Highlight, then the recipe window over everything, then the tooltip.
        if let Some(item) = self.jei_hit_at(cursor) {
            self.jei.hover = Some(item);
        }
        if self.jei.view.is_some() {
            self.draw_jei_window(batch);
        }
        self.draw_jei_tooltip(batch);
    }

    /// The recipe window, centred.
    fn draw_jei_window(&mut self, batch: &mut HudBatch) {
        let Some(view) = self.jei.view.clone() else {
            return;
        };
        let recipe = view.current();
        let layout = logic::layout(recipe);
        let category = Category::of(&recipe.kind);
        let w = (layout.size.0 + 24.0).max(170.0);
        // The content, a header above it, and two footer lines below.
        let h = layout.size.1 + 28.0 + 44.0;
        let x = ((batch.width - w) / 2.0).floor();
        let y = ((batch.height - h) / 2.0).floor();
        self.jei.window = Some([x, y, w, h]);
        let cursor = self.cursor;
        let millis = self.jei.started.elapsed().as_millis();
        let (ox, oy) = (
            (x + (w - layout.size.0) / 2.0).floor(),
            (y + 28.0).floor(),
        );
        {
            let atlas = &self.gui.atlas;
            // The panel: black rim, light grey, white top left, dark bottom right.
            batch.fill(atlas, x - 1.0, y - 1.0, w + 2.0, h + 2.0, rgba(0, 0, 0, 255));
            batch.fill(atlas, x, y, w, h, rgba(198, 198, 198, 255));
            batch.fill(atlas, x, y, w - 1.0, 1.0, rgba(255, 255, 255, 255));
            batch.fill(atlas, x, y, 1.0, h - 1.0, rgba(255, 255, 255, 255));
            batch.fill(atlas, x + 1.0, y + h - 1.0, w - 1.0, 1.0, rgba(85, 85, 85, 255));
            batch.fill(atlas, x + w - 1.0, y + 1.0, 1.0, h - 1.0, rgba(85, 85, 85, 255));
            let dark = rgba(64, 64, 64, 255);
            batch.text(atlas, category.title(), x + 28.0, y + 10.0, dark, false);
            for &(dx, dy, _) in &layout.inputs {
                slot_box(batch, atlas, ox + dx, oy + dy);
            }
            slot_box(batch, atlas, ox + layout.output.0, oy + layout.output.1);
            arrow(batch, atlas, ox + layout.arrow.0, oy + layout.arrow.1);
        }
        if let Some((fx, fy)) = layout.flame {
            let draws = [sprite(
                "container/furnace/lit_progress",
                (ox + fx) as i32,
                (oy + fy) as i32,
                14,
            )];
            hud_layout::paint(batch, &self.gui.atlas, &draws);
        }
        // Items: the station, the inputs (cycling through a tag's items) and
        // the result.
        let station = ItemStack::new(&recipe.station, 1);
        self.draw_item(batch, &station, x + 8.0, y + 6.0);
        for &(dx, dy, index) in &layout.inputs {
            if let Some(item) = recipe
                .ingredients
                .get(index)
                .and_then(|alts| logic::alternative(alts, millis))
            {
                let stack = ItemStack::new(item, 1);
                let (ix, iy) = (ox + dx + 1.0, oy + dy + 1.0);
                self.draw_item(batch, &stack, ix, iy);
                self.jei.hits.push((ix, iy, item.to_owned()));
            }
        }
        let result = ItemStack::new(&recipe.result, recipe.result_count);
        let (rx, ry) = (ox + layout.output.0 + 1.0, oy + layout.output.1 + 1.0);
        self.draw_item(batch, &result, rx, ry);
        self.jei.hits.push((rx, ry, recipe.result.clone()));

        // The footer: what is shown, and the buttons through the recipes.
        let atlas = &self.gui.atlas;
        let dark = rgba(64, 64, 64, 255);
        let item_name = self.translate(
            &format!("item.{}", view.query.item().replace(':', ".")),
            &horizon_client::jade::title_case(view.query.item()),
        );
        let caption = match view.query {
            Query::Output(_) => format!("Makes: {item_name}"),
            Query::Uses(_) => format!("Uses: {item_name}"),
        };
        batch.text(atlas, &caption, x + 8.0, y + h - 36.0, dark, false);
        let count = format!("{}/{}", view.index + 1, view.recipes.len());
        let tw = HudBatch::text_width(atlas, &count);
        let by = y + h - 20.0;
        let prev = [x + w - 8.0 - BUTTON * 2.0 - tw - 8.0, by, BUTTON, BUTTON];
        let next = [x + w - 8.0 - BUTTON, by, BUTTON, BUTTON];
        self.jei.step = [prev, next];
        button(batch, atlas, prev, "<", inside(cursor, prev));
        button(batch, atlas, next, ">", inside(cursor, next));
        batch.text(atlas, &count, prev[0] + BUTTON + 4.0, by + 2.0, dark, false);
        // The window's items may be hovered too.
        if let Some(item) = self.jei_hit_at(cursor) {
            self.jei.hover = Some(item);
        }
    }

    /// The tooltip of the item under the pointer in the list or window.
    fn draw_jei_tooltip(&mut self, batch: &mut HudBatch) {
        let Some(item) = self.jei_hit_at(self.cursor) else {
            return;
        };
        let stack = ItemStack::new(&item, 1);
        let mut lines = self.stack_tooltip(&stack);
        lines.push((
            "Click: recipes   Right click: uses".into(),
            rgba(150, 150, 150, 255),
        ));
        let (cx, cy) = self.cursor;
        self.draw_tooltip_lines(batch, &lines, cx, cy);
    }
}
