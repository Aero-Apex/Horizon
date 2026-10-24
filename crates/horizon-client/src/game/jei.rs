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
    jei::{self as logic, Panel, Query, RecipeView, SLOT, TAB_HEIGHT, TAB_STEP},
    recipe_table::RecipeTable,
    ui::EditBox,
};
use std::{borrow::Cow, sync::Arc, time::Instant};
use winit::{event::KeyEvent, keyboard::KeyCode};

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
    /// The recipe window's box, the arrows through its pages and through its
    /// categories, its category tabs and the buttons that add a recipe's items.
    window: Option<[f32; 4]>,
    step: [[f32; 4]; 2],
    cats: [[f32; 4]; 2],
    tabs: Vec<[f32; 4]>,
    plus: Vec<([f32; 4], Vec<(String, u32)>)>,
    /// How many recipes the window's page holds.
    per_page: usize,
    /// The page arrows of the list and the wrench beside the search box.
    pages: [[f32; 4]; 2],
    wrench: [f32; 4],
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
            cats: [[0.0; 4]; 2],
            tabs: Vec::new(),
            plus: Vec::new(),
            per_page: 1,
            pages: [[0.0; 4]; 2],
            wrench: [0.0; 4],
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
    batch.fill(
        atlas,
        x + 1.0,
        y + SLOT - 1.0,
        SLOT - 1.0,
        1.0,
        rgba(255, 255, 255, 255),
    );
    batch.fill(
        atlas,
        x + SLOT - 1.0,
        y + 1.0,
        1.0,
        SLOT - 1.0,
        rgba(255, 255, 255, 255),
    );
}

/// A button as the game's own: black rim, light edge top left, dark bottom
/// right, bluish when the pointer is on it.
fn button_box(
    batch: &mut HudBatch,
    atlas: &horizon_client::hud::HudAtlas,
    r: [f32; 4],
    hover: bool,
) {
    let fill = if hover {
        rgba(125, 134, 188, 255)
    } else {
        rgba(111, 111, 111, 255)
    };
    batch.fill(atlas, r[0], r[1], r[2], r[3], rgba(0, 0, 0, 255));
    batch.fill(atlas, r[0] + 1.0, r[1] + 1.0, r[2] - 2.0, r[3] - 2.0, fill);
    batch.fill(
        atlas,
        r[0] + 1.0,
        r[1] + 1.0,
        r[2] - 3.0,
        1.0,
        rgba(190, 190, 200, 255),
    );
    batch.fill(
        atlas,
        r[0] + 1.0,
        r[1] + 1.0,
        1.0,
        r[3] - 3.0,
        rgba(190, 190, 200, 255),
    );
    batch.fill(
        atlas,
        r[0] + 2.0,
        r[1] + r[3] - 2.0,
        r[2] - 3.0,
        1.0,
        rgba(60, 60, 66, 255),
    );
    batch.fill(
        atlas,
        r[0] + r[2] - 2.0,
        r[1] + 2.0,
        1.0,
        r[3] - 3.0,
        rgba(60, 60, 66, 255),
    );
}

/// A small square button with a symbol in it. `<` and `>` are drawn as
/// triangles.
fn button(
    batch: &mut HudBatch,
    atlas: &horizon_client::hud::HudAtlas,
    r: [f32; 4],
    label: &str,
    hover: bool,
) {
    button_box(batch, atlas, r, hover);
    let white = rgba(255, 255, 255, 255);
    if label == "<" || label == ">" {
        let (cx, cy) = (r[0] + (r[2] / 2.0).floor(), r[1] + (r[3] / 2.0).floor());
        for j in 0..4 {
            let tall = (1 + 2 * j) as f32;
            let x = if label == "<" {
                cx - 2.0 + j as f32
            } else {
                cx + 1.0 - j as f32
            };
            batch.fill(atlas, x, cy - (tall / 2.0).floor(), 1.0, tall, white);
        }
        return;
    }
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

/// A wrench, 12 by 12, in a button.
fn wrench(batch: &mut HudBatch, atlas: &horizon_client::hud::HudAtlas, r: [f32; 4], hover: bool) {
    button_box(batch, atlas, r, hover);
    let (x, y) = (
        r[0] + ((r[2] - 12.0) / 2.0).floor(),
        r[1] + ((r[3] - 12.0) / 2.0).floor(),
    );
    let light = rgba(220, 220, 225, 255);
    let shade = rgba(60, 60, 66, 255);
    for i in 0..7 {
        batch.fill(
            atlas,
            x + 1.0 + i as f32 + 1.0,
            y + 10.0 - i as f32 + 1.0,
            2.0,
            2.0,
            shade,
        );
        batch.fill(
            atlas,
            x + 1.0 + i as f32,
            y + 10.0 - i as f32,
            2.0,
            2.0,
            light,
        );
    }
    batch.fill(atlas, x + 7.0, y + 1.0, 5.0, 5.0, light);
    batch.fill(atlas, x + 9.0, y + 1.0, 3.0, 3.0, rgba(111, 111, 111, 255));
}

/// A grey strip with a centred title, as the window's header rows.
fn strip(batch: &mut HudBatch, atlas: &horizon_client::hud::HudAtlas, r: [f32; 4], title: &str) {
    batch.fill(atlas, r[0], r[1], r[2], r[3], rgba(55, 55, 55, 255));
    batch.fill(
        atlas,
        r[0] + 1.0,
        r[1] + 1.0,
        r[2] - 2.0,
        r[3] - 2.0,
        rgba(139, 139, 139, 255),
    );
    let tw = HudBatch::text_width(atlas, title);
    batch.text(
        atlas,
        title,
        (r[0] + (r[2] - tw) / 2.0).round(),
        (r[1] + (r[3] - 8.0) / 2.0).round() + 1.0,
        u32::MAX,
        true,
    );
}

/// A rectangle in the window's grey: white top left, dark bottom right.
fn raised(
    batch: &mut HudBatch,
    atlas: &horizon_client::hud::HudAtlas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    batch.fill(atlas, x, y, w, h, rgba(198, 198, 198, 255));
    batch.fill(atlas, x, y, w - 1.0, 1.0, rgba(255, 255, 255, 255));
    batch.fill(atlas, x, y, 1.0, h - 1.0, rgba(255, 255, 255, 255));
    batch.fill(
        atlas,
        x + 1.0,
        y + h - 1.0,
        w - 1.0,
        1.0,
        rgba(85, 85, 85, 255),
    );
    batch.fill(
        atlas,
        x + w - 1.0,
        y + 1.0,
        1.0,
        h - 1.0,
        rgba(85, 85, 85, 255),
    );
}

/// The arrow between a recipe's inputs and its output, 24 wide and 16 tall.
fn arrow(batch: &mut HudBatch, atlas: &horizon_client::hud::HudAtlas, x: f32, y: f32) {
    let grey = rgba(139, 139, 139, 255);
    batch.fill(atlas, x, y + 6.0, 16.0, 4.0, grey);
    for i in 0..8 {
        let half = (8 - i) as f32;
        batch.fill(
            atlas,
            x + 16.0 + i as f32,
            y + 8.0 - half,
            1.0,
            half * 2.0,
            grey,
        );
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
            self.jei.results = self.jei_search(&term);
            self.jei.term = Some(term);
            self.jei.page = 0;
        }
        if self.jei.table.is_none() {
            let mut table = RecipeTable::load(&self.ui.vanilla_root);
            self.jei_fuel_recipes(&mut table);
            log::info!("Horizon: {} recipes for the item list", table.len());
            self.jei.table = Some(Arc::new(table));
        }
    }

    /// `FuelRecipeMaker`: every item with a burn time is a fuel in the category of
    /// each furnace that burns it. The smoker and the blast furnace take the fast
    /// providers, as the furnace tick does.
    fn jei_fuel_recipes(&self, table: &mut RecipeTable) {
        let items = self.res.items();
        let providers = self.res.providers();
        for (kind, station, fast) in [
            ("minecraft:fuel_smelting", "minecraft:furnace", false),
            ("minecraft:fuel_smoking", "minecraft:smoker", true),
            ("minecraft:fuel_blasting", "minecraft:blast_furnace", true),
        ] {
            for id in items.items.keys() {
                let stack = ItemStack::new(id, 1);
                let burn = providers.burn_time(items, &stack, fast);
                if burn > 0 {
                    let smelt = providers.speed(items, &stack, fast);
                    table.add_fuel(kind, station, id, burn, smelt);
                }
            }
        }
    }

    /// The creative items a search finds, as JEI reads it (`logic::parse_query`):
    /// indices into the creative screen's items.
    fn jei_search(&self, term: &str) -> Vec<usize> {
        let query = logic::parse_query(term);
        let stacks = self.creative_screen.search_items();
        if query.is_empty() {
            return (0..stacks.len()).collect();
        }
        let tooltip = logic::needs_tooltip(&query);
        let wants_tags = query.iter().flatten().any(|p| p.field == logic::Field::Tag);
        // Tag -> items, turned round, when a term asks for tags.
        let mut tags: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
        if wants_tags {
            for (tag, items) in &self.res.items().item_tags {
                for item in items {
                    tags.entry(item.as_str()).or_default().push(tag.as_str());
                }
            }
        }
        stacks
            .iter()
            .enumerate()
            .filter(|(_, stack)| {
                let id = stack.item.to_string();
                let haystack = logic::Haystack {
                    name: self.pack.names.item(&stack.item),
                    mod_name: horizon_client::jade::namespace_name(&id),
                    tooltip: if tooltip {
                        self.stack_tooltip(stack)
                            .into_iter()
                            .map(|(line, _)| line)
                            .collect::<Vec<_>>()
                            .join(
                                "
",
                            )
                    } else {
                        String::new()
                    },
                    tags: tags
                        .get(id.as_str())
                        .map(|t| t.iter().map(|s| (*s).to_owned()).collect())
                        .unwrap_or_default(),
                    id,
                };
                logic::matches(&query, &haystack)
            })
            .map(|(i, _)| i)
            .collect()
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
            let per_page = self.jei.per_page;
            if let Some(i) = (0..2).find(|&i| inside(p, self.jei.step[i])) {
                if let Some(v) = self.jei.view.as_mut() {
                    v.step(i == 1, per_page);
                }
            } else if let Some(i) = (0..2).find(|&i| inside(p, self.jei.cats[i])) {
                if let Some(v) = self.jei.view.as_mut() {
                    v.step_category(i == 1);
                }
            } else if let Some(i) = self.jei.tabs.iter().position(|t| inside(p, *t)) {
                if let Some(v) = self.jei.view.as_mut() {
                    v.select(i);
                }
            } else if let Some(i) = self.jei.plus.iter().position(|(r, _)| inside(p, *r)) {
                self.jei_give(i);
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
        if inside(p, self.jei.wrench) {
            self.jei.search.focused = false;
            self.jei.swallow = true;
            self.open_options_at(false, horizon_client::options_ui::Page::Extras);
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
        let per_page = self.jei.per_page;
        if let Some(view) = self.jei.view.as_mut() {
            view.step(d < 0.0, per_page);
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

    /// The `+` of a recipe: in creative, puts its ingredients in the inventory.
    fn jei_give(&mut self, index: usize) {
        if !self.creative {
            self.message = Some((
                "Adding ingredients needs creative mode".into(),
                Instant::now(),
            ));
            return;
        }
        let Some((_, items)) = self.jei.plus.get(index).cloned() else {
            return;
        };
        for (item, count) in items {
            let _ = self.give(ItemStack::new(&item, count));
        }
        self.message = Some(("Added the recipe's items".into(), Instant::now()));
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
        // No boxes: the items float over the world, the one under the pointer
        // lit (unless the recipe window covers the list).
        for (n, stack) in stacks.iter().enumerate() {
            let (x, y) = panel.slot_origin(n);
            if self.jei.view.is_none() && inside(cursor, [x, y, SLOT, SLOT]) {
                batch.fill(
                    &self.gui.atlas,
                    x + 1.0,
                    y + 1.0,
                    SLOT - 2.0,
                    SLOT - 2.0,
                    rgba(255, 255, 255, 110),
                );
            }
            self.draw_item(batch, stack, x + 1.0, y + 1.0);
            self.jei
                .hits
                .push((x + 1.0, y + 1.0, stack.item.to_string()));
        }

        // The page row and the search box with its wrench.
        let atlas = &self.gui.atlas;
        let width = panel.width();
        let arrows = [
            [panel.x, panel.page_y, 20.0, 14.0],
            [panel.x + width - 20.0, panel.page_y, 20.0, 14.0],
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
            panel.page_y + 3.0,
            u32::MAX,
            true,
        );
        let wrench_box = [panel.x + width - 16.0, panel.search_y, 16.0, 16.0];
        self.jei.wrench = wrench_box;
        wrench(batch, atlas, wrench_box, inside(cursor, wrench_box));
        let search = &mut self.jei.search;
        search.x = panel.x;
        search.y = panel.search_y;
        search.width = width - 19.0;
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

    /// The recipe window, centred: category tabs above it, a title row and a
    /// page row, then a page of recipes each in its own frame.
    fn draw_jei_window(&mut self, batch: &mut HudBatch) {
        let Some(view) = self.jei.view.clone() else {
            return;
        };
        let category = view.category();
        let plan = logic::window(batch.height, category, view.recipes().len());
        self.jei.per_page = plan.per_page;
        let (w, h) = (plan.w, plan.h);
        let x = ((batch.width - w) / 2.0).floor();
        let y = (((batch.height - h - TAB_HEIGHT) / 2.0).floor() + TAB_HEIGHT).max(TAB_HEIGHT);
        self.jei.window = Some([x, y, w, h]);
        let cursor = self.cursor;
        let millis = self.jei.started.elapsed().as_millis();
        let selected = view.category.min(view.groups.len() - 1);
        self.jei.tabs.clear();
        self.jei.plus.clear();
        {
            let atlas = &self.gui.atlas;
            // The panel: black rim, light grey, white top left, dark bottom right.
            batch.fill(
                atlas,
                x - 1.0,
                y - 1.0,
                w + 2.0,
                h + 2.0,
                rgba(0, 0, 0, 255),
            );
            raised(batch, atlas, x, y, w, h);
            // Tabs: the chosen one is part of the window, the others sit lower.
            for i in 0..view.groups.len() {
                let tx = x + 4.0 + i as f32 * TAB_STEP;
                self.jei.tabs.push([tx, y - TAB_HEIGHT, 22.0, TAB_HEIGHT]);
                if i == selected {
                    batch.fill(
                        atlas,
                        tx - 1.0,
                        y - TAB_HEIGHT - 1.0,
                        24.0,
                        TAB_HEIGHT + 1.0,
                        rgba(0, 0, 0, 255),
                    );
                    raised(batch, atlas, tx, y - TAB_HEIGHT, 22.0, TAB_HEIGHT + 1.0);
                } else {
                    let top = y - TAB_HEIGHT + 3.0;
                    batch.fill(
                        atlas,
                        tx - 1.0,
                        top - 1.0,
                        24.0,
                        TAB_HEIGHT - 2.0,
                        rgba(0, 0, 0, 255),
                    );
                    batch.fill(
                        atlas,
                        tx,
                        top,
                        22.0,
                        TAB_HEIGHT - 3.0,
                        rgba(139, 139, 139, 255),
                    );
                    batch.fill(atlas, tx, top, 21.0, 1.0, rgba(200, 200, 200, 255));
                    batch.fill(
                        atlas,
                        tx,
                        top,
                        1.0,
                        TAB_HEIGHT - 4.0,
                        rgba(200, 200, 200, 255),
                    );
                }
            }
            // The title row (previous and next category) and the page row.
            let pages = view.pages(plan.per_page);
            let page = view.page.min(pages - 1);
            let rows = [
                [x + 6.0, y + 5.0, w - 12.0, 14.0],
                [x + 6.0, y + 24.0, w - 12.0, 14.0],
            ];
            strip(batch, atlas, rows[0], category.title());
            strip(batch, atlas, rows[1], &format!("{}/{}", page + 1, pages));
            for (row, arrows) in [(rows[0], 0), (rows[1], 1)] {
                let pair = [
                    [row[0], row[1], 14.0, 14.0],
                    [row[0] + row[2] - 14.0, row[1], 14.0, 14.0],
                ];
                if arrows == 0 {
                    self.jei.cats = pair;
                } else {
                    self.jei.step = pair;
                }
                button(batch, atlas, pair[0], "<", inside(cursor, pair[0]));
                button(batch, atlas, pair[1], ">", inside(cursor, pair[1]));
            }
        }
        // The tab icons.
        for (i, (group, _)) in view.groups.iter().enumerate() {
            let tab = self.jei.tabs[i];
            let icon = ItemStack::new(group.station(), 1);
            let lift = if i == selected { 5.0 } else { 7.0 };
            self.draw_item(batch, &icon, tab[0] + 3.0, tab[1] + lift);
        }
        // The recipes of the page.
        let recipes: Vec<_> = view.page_recipes(plan.per_page).to_vec();
        for (n, recipe) in recipes.iter().enumerate() {
            let layout = logic::layout(recipe);
            let (fx, fy, fw, fh) = (x + 8.0, y + plan.frame_y(n), w - 16.0, plan.frame_h);
            let ox = (fx + 4.0 + ((fw - 22.0 - layout.size.0) / 2.0).floor().max(0.0)).floor();
            let oy = (fy + ((fh - layout.size.1) / 2.0).floor()).floor();
            {
                let atlas = &self.gui.atlas;
                batch.fill(atlas, fx, fy, fw, fh, rgba(139, 139, 139, 255));
                batch.fill(
                    atlas,
                    fx + 1.0,
                    fy + 1.0,
                    fw - 2.0,
                    fh - 2.0,
                    rgba(212, 212, 212, 255),
                );
                batch.fill(
                    atlas,
                    fx + 1.0,
                    fy + fh - 2.0,
                    fw - 2.0,
                    1.0,
                    rgba(255, 255, 255, 255),
                );
                batch.fill(
                    atlas,
                    fx + fw - 2.0,
                    fy + 1.0,
                    1.0,
                    fh - 2.0,
                    rgba(255, 255, 255, 255),
                );
                for &(dx, dy, _) in &layout.inputs {
                    slot_box(batch, atlas, ox + dx, oy + dy);
                }
                if !recipe.result.is_empty() {
                    slot_box(batch, atlas, ox + layout.output.0, oy + layout.output.1);
                    arrow(batch, atlas, ox + layout.arrow.0, oy + layout.arrow.1);
                }
            }
            if let Some((flx, fly)) = layout.flame {
                let draws = [sprite(
                    "container/furnace/lit_progress",
                    (ox + flx) as i32,
                    (oy + fly) as i32,
                    14,
                )];
                hud_layout::paint(batch, &self.gui.atlas, &draws);
            }
            if let Some((nx, ny)) = layout.note {
                let text = logic::smelt_count_text(recipe.smelt_count);
                let atlas = &self.gui.atlas;
                let tw = HudBatch::text_width(atlas, &text);
                let (aw, ah) = (layout.size.0 - nx, layout.size.1 - ny);
                batch.text(
                    atlas,
                    &text,
                    ox + nx + ((aw - tw) / 2.0).round(),
                    oy + ny + ((ah - 8.0) / 2.0).round(),
                    0xFF80_8080,
                    false,
                );
            }
            // Items: the inputs (cycling through a tag's items) and the result.
            let mut needs: Vec<(String, u32)> = Vec::new();
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
                if let Some(first) = recipe.ingredients.get(index).and_then(|alts| alts.first()) {
                    match needs.iter_mut().find(|(item, _)| item == first) {
                        Some(entry) => entry.1 += 1,
                        None => needs.push((first.clone(), 1)),
                    }
                }
            }
            if !recipe.result.is_empty() {
                let result = ItemStack::new(&recipe.result, recipe.result_count);
                let (rx, ry) = (ox + layout.output.0 + 1.0, oy + layout.output.1 + 1.0);
                self.draw_item(batch, &result, rx, ry);
                self.jei.hits.push((rx, ry, recipe.result.clone()));
            }
            // The plus: adds the recipe's items.
            let plus = [fx + fw - 15.0, fy + fh - 15.0, 12.0, 12.0];
            button(batch, &self.gui.atlas, plus, "+", inside(cursor, plus));
            self.jei.plus.push((plus, needs));
        }
        // The window's items may be hovered too.
        if let Some(item) = self.jei_hit_at(cursor) {
            self.jei.hover = Some(item);
        }
    }

    /// The tip of the window's tab or plus button under the pointer.
    fn jei_widget_tip(&self) -> Option<String> {
        let p = self.cursor;
        let view = self.jei.view.as_ref()?;
        if let Some(i) = self.jei.tabs.iter().position(|t| inside(p, *t)) {
            return view.groups.get(i).map(|g| g.0.title().to_owned());
        }
        if self.jei.plus.iter().any(|(r, _)| inside(p, *r)) {
            return Some(
                if self.creative {
                    "Add the recipe's items"
                } else {
                    "Adding items needs creative mode"
                }
                .to_owned(),
            );
        }
        if inside(p, self.jei.wrench) {
            return Some("Horizon extras".to_owned());
        }
        None
    }

    /// The tooltip of the item under the pointer in the list or window.
    fn draw_jei_tooltip(&mut self, batch: &mut HudBatch) {
        if let Some(tip) = self.jei_widget_tip() {
            let (cx, cy) = self.cursor;
            self.draw_tooltip_lines(batch, &[(tip, u32::MAX)], cx, cy);
            return;
        }
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
