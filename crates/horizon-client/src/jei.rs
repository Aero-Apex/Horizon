//! Just Enough Items, rewritten for Horizon (`docs/extras.md`). The Java mod
//! (mezz's JEI) lists every item beside the inventory and shows how an item
//! is made and what it is used in. This holds the decisions: how a recipe's
//! slots sit in its window, which ingredient shows now, and where the item
//! list goes; `game/jei.rs` draws it and takes the clicks.
use crate::recipe_table::{RecipeInfo, RecipeTable};

/// A slot, as the container screens draw it.
pub const SLOT: f32 = 18.0;
/// The item list's margin from the screen edge and the inventory.
const EDGE: f32 = 6.0;
const GAP: f32 = 12.0;
/// Space the item list keeps above its grid (page row) and below (search box).
const TOP: f32 = 16.0;
const BOTTOM: f32 = 24.0;
/// The narrowest list worth showing, in columns.
const MIN_COLUMNS: usize = 3;
/// One ingredient alternative shows this long before the next.
pub const CYCLE_MS: u128 = 1000;

/// What a recipe window shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Query {
    /// How the item is made.
    Output(String),
    /// What the item goes into.
    Uses(String),
}
impl Query {
    pub fn item(&self) -> &str {
        match self {
            Query::Output(i) | Query::Uses(i) => i,
        }
    }
    pub fn flipped(&self) -> Query {
        match self {
            Query::Output(i) => Query::Uses(i.clone()),
            Query::Uses(i) => Query::Output(i.clone()),
        }
    }
}

/// The recipes of a query, one shown at a time.
#[derive(Clone, Debug)]
pub struct RecipeView {
    pub query: Query,
    pub recipes: Vec<RecipeInfo>,
    pub index: usize,
}
impl RecipeView {
    /// None when nothing makes (or uses) the item.
    pub fn new(table: &RecipeTable, query: Query) -> Option<Self> {
        let found = match &query {
            Query::Output(item) => table.producing(item),
            Query::Uses(item) => table.consuming(item),
        };
        if found.is_empty() {
            return None;
        }
        Some(Self {
            recipes: found.into_iter().cloned().collect(),
            query,
            index: 0,
        })
    }
    pub fn current(&self) -> &RecipeInfo {
        &self.recipes[self.index.min(self.recipes.len() - 1)]
    }
    /// Steps through the recipes, wrapping round.
    pub fn step(&mut self, forward: bool) {
        let n = self.recipes.len();
        self.index = if forward {
            (self.index + 1) % n
        } else {
            (self.index + n - 1) % n
        };
    }
}

/// The ingredient alternative to show at `millis`, cycling through the items
/// a tag lists.
pub fn alternative(alternatives: &[String], millis: u128) -> Option<&str> {
    if alternatives.is_empty() {
        return None;
    }
    let i = (millis / CYCLE_MS) as usize % alternatives.len();
    Some(&alternatives[i])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Crafting,
    Smelting,
    Blasting,
    Smoking,
    Campfire,
    Stonecutting,
    Smithing,
    Transmute,
}
impl Category {
    pub fn of(kind: &str) -> Self {
        match kind {
            "minecraft:smelting" => Category::Smelting,
            "minecraft:blasting" => Category::Blasting,
            "minecraft:smoking" => Category::Smoking,
            "minecraft:campfire_cooking" => Category::Campfire,
            "minecraft:stonecutting" => Category::Stonecutting,
            "minecraft:smithing_transform" => Category::Smithing,
            "minecraft:crafting_transmute" => Category::Transmute,
            _ => Category::Crafting,
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Category::Crafting => "Crafting",
            Category::Smelting => "Smelting",
            Category::Blasting => "Blasting",
            Category::Smoking => "Smoking",
            Category::Campfire => "Campfire cooking",
            Category::Stonecutting => "Stonecutting",
            Category::Smithing => "Smithing",
            Category::Transmute => "Crafting",
        }
    }
    /// Whether the window shows a flame under the input.
    pub fn burns(self) -> bool {
        matches!(
            self,
            Category::Smelting | Category::Blasting | Category::Smoking | Category::Campfire
        )
    }
}

/// A recipe's slots in its window, relative to the window's top left.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// (x, y, index into the recipe's ingredients) per filled input slot.
    pub inputs: Vec<(f32, f32, usize)>,
    pub output: (f32, f32),
    /// The arrow's top left (it is 24 wide).
    pub arrow: (f32, f32),
    /// The flame under a cooking input.
    pub flame: Option<(f32, f32)>,
    /// The window's content size.
    pub size: (f32, f32),
}

pub fn layout(r: &RecipeInfo) -> Layout {
    let category = Category::of(&r.kind);
    match category {
        Category::Crafting | Category::Transmute if r.kind != "minecraft:crafting_transmute" => {
            let mut inputs = Vec::new();
            if r.width > 0 {
                for (cell, index) in r.cells.iter().enumerate() {
                    if let Some(index) = index {
                        let (col, row) = (cell % r.width, cell / r.width);
                        inputs.push((col as f32 * SLOT, row as f32 * SLOT, *index));
                    }
                }
            } else {
                for index in 0..r.ingredients.len().min(9) {
                    inputs.push(((index % 3) as f32 * SLOT, (index / 3) as f32 * SLOT, index));
                }
            }
            Layout {
                inputs,
                output: (92.0, SLOT),
                arrow: (58.0, SLOT + 1.0),
                flame: None,
                size: (110.0, 3.0 * SLOT),
            }
        }
        Category::Smithing | Category::Transmute => {
            let n = r.ingredients.len().min(3);
            Layout {
                inputs: (0..n).map(|i| (i as f32 * 22.0, 0.0, i)).collect(),
                output: (n as f32 * 22.0 + 28.0, 0.0),
                arrow: (n as f32 * 22.0 + 2.0, 1.0),
                flame: None,
                size: (n as f32 * 22.0 + 46.0, SLOT),
            }
        }
        Category::Stonecutting => Layout {
            inputs: vec![(0.0, 12.0, 0)],
            output: (58.0, 12.0),
            arrow: (26.0, 13.0),
            flame: None,
            size: (76.0, 42.0),
        },
        _ => Layout {
            inputs: vec![(0.0, 0.0, 0)],
            output: (62.0, 8.0),
            arrow: (28.0, 9.0),
            flame: Some((2.0, 22.0)),
            size: (80.0, 40.0),
        },
    }
}

/// The item list beside an inventory screen: where its grid, page row and
/// search box go, in GUI pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panel {
    pub x: f32,
    pub y: f32,
    pub columns: usize,
    pub rows: usize,
    /// The page row's y and the search box's.
    pub page_y: f32,
    pub search_y: f32,
}
impl Panel {
    pub fn width(&self) -> f32 {
        self.columns as f32 * SLOT
    }
    pub fn per_page(&self) -> usize {
        self.columns * self.rows
    }
    pub fn pages(&self, items: usize) -> usize {
        items.div_ceil(self.per_page().max(1)).max(1)
    }
    /// The list index of the slot at (x, y) on `page`, if the pointer is on one.
    pub fn slot_at(&self, x: f32, y: f32, page: usize, items: usize) -> Option<usize> {
        let (dx, dy) = (x - self.x, y - self.y);
        if dx < 0.0 || dy < 0.0 {
            return None;
        }
        let (col, row) = ((dx / SLOT) as usize, (dy / SLOT) as usize);
        if col >= self.columns || row >= self.rows {
            return None;
        }
        let i = page * self.per_page() + row * self.columns + col;
        (i < items).then_some(i)
    }
    /// The top left of the slot for position `n` on a page.
    pub fn slot_origin(&self, n: usize) -> (f32, f32) {
        (
            self.x + (n % self.columns) as f32 * SLOT,
            self.y + (n / self.columns) as f32 * SLOT,
        )
    }
}

/// The list's room right of a screen whose menu ends at `menu_right`, or none
/// when the window is too narrow.
pub fn panel(width: f32, height: f32, menu_right: f32) -> Option<Panel> {
    let right = width - EDGE;
    let room = right - (menu_right + GAP);
    let columns = (room / SLOT) as usize;
    let rows = ((height - TOP - BOTTOM) / SLOT) as usize;
    if columns < MIN_COLUMNS || rows < 2 {
        return None;
    }
    Some(Panel {
        x: right - columns as f32 * SLOT,
        y: TOP,
        columns,
        rows,
        page_y: 3.0,
        search_y: height - BOTTOM + 4.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe(kind: &str, ingredients: &[&[&str]], width: usize, cells: &[Option<usize>]) -> RecipeInfo {
        RecipeInfo {
            id: "minecraft:test".into(),
            kind: kind.into(),
            station: "minecraft:crafting_table".into(),
            result: "minecraft:stick".into(),
            result_count: 4,
            ingredients: ingredients
                .iter()
                .map(|alts| alts.iter().map(|s| (*s).to_owned()).collect())
                .collect(),
            width,
            height: if width == 0 { 0 } else { cells.len() / width },
            cells: cells.to_vec(),
        }
    }

    #[test]
    fn a_shaped_recipe_keeps_its_shape() {
        // Torch: coal over stick, in a 1 x 2 grid.
        let torch = recipe(
            "minecraft:crafting_shaped",
            &[&["minecraft:coal"], &["minecraft:stick"]],
            1,
            &[Some(0), Some(1)],
        );
        let l = layout(&torch);
        assert_eq!(l.inputs, vec![(0.0, 0.0, 0), (0.0, 18.0, 1)]);
        assert_eq!(l.output, (92.0, 18.0));
        // A pickaxe: three across the top, the stick down the middle.
        let pick = recipe(
            "minecraft:crafting_shaped",
            &[&["minecraft:cobblestone"], &["minecraft:stick"]],
            3,
            &[Some(0), Some(0), Some(0), None, Some(1), None, None, Some(1), None],
        );
        let l = layout(&pick);
        assert_eq!(l.inputs.len(), 5);
        assert_eq!(l.inputs[3], (18.0, 18.0, 1));
        assert_eq!(l.inputs[4], (18.0, 36.0, 1));
    }

    #[test]
    fn a_shapeless_recipe_fills_the_grid_in_order() {
        let r = recipe(
            "minecraft:crafting_shapeless",
            &[&["a"], &["b"], &["c"], &["d"]],
            0,
            &[],
        );
        let l = layout(&r);
        assert_eq!(l.inputs[3], (0.0, 18.0, 3));
        assert_eq!(l.inputs[2], (36.0, 0.0, 2));
    }

    #[test]
    fn cooking_has_a_flame_and_one_input() {
        let r = recipe("minecraft:smelting", &[&["minecraft:potato"]], 0, &[]);
        let l = layout(&r);
        assert_eq!(l.inputs, vec![(0.0, 0.0, 0)]);
        assert!(l.flame.is_some());
        assert!(Category::of("minecraft:smoking").burns());
        assert!(!Category::of("minecraft:stonecutting").burns());
        let cut = layout(&recipe("minecraft:stonecutting", &[&["minecraft:stone"]], 0, &[]));
        assert!(cut.flame.is_none());
    }

    #[test]
    fn smithing_lays_its_three_slots_in_a_row() {
        let r = recipe(
            "minecraft:smithing_transform",
            &[&["t"], &["b"], &["a"]],
            0,
            &[],
        );
        let l = layout(&r);
        assert_eq!(l.inputs, vec![(0.0, 0.0, 0), (22.0, 0.0, 1), (44.0, 0.0, 2)]);
        assert_eq!(l.output.1, 0.0);
    }

    #[test]
    fn ingredients_cycle_each_second() {
        let alts: Vec<String> = ["a", "b", "c"].map(String::from).to_vec();
        assert_eq!(alternative(&alts, 0), Some("a"));
        assert_eq!(alternative(&alts, 999), Some("a"));
        assert_eq!(alternative(&alts, 1000), Some("b"));
        assert_eq!(alternative(&alts, 3000), Some("a"));
        assert_eq!(alternative(&[], 5), None);
    }

    #[test]
    fn the_list_fits_beside_the_menu() {
        // 400 x 300 GUI pixels, a 176 wide menu in the middle: it ends at 288.
        let p = panel(400.0, 300.0, 288.0).unwrap();
        assert_eq!(p.columns, 5);
        assert_eq!(p.rows, 14);
        assert_eq!(p.x, 394.0 - 90.0);
        assert_eq!(p.per_page(), 70);
        assert_eq!(p.pages(0), 1);
        assert_eq!(p.pages(70), 1);
        assert_eq!(p.pages(71), 2);
        // Too narrow: no list.
        assert!(panel(300.0, 240.0, 248.0).is_none());
    }

    #[test]
    fn slots_are_found_by_position() {
        let p = panel(400.0, 300.0, 288.0).unwrap();
        let first = p.slot_origin(0);
        assert_eq!(p.slot_at(first.0 + 2.0, first.1 + 2.0, 0, 100), Some(0));
        let sixth = p.slot_origin(5);
        assert_eq!(p.slot_at(sixth.0 + 1.0, sixth.1 + 1.0, 0, 100), Some(5));
        // The same spot on page 2 is 70 further on, and empty past the end.
        assert_eq!(p.slot_at(first.0 + 1.0, first.1 + 1.0, 1, 100), Some(70));
        assert_eq!(p.slot_at(first.0 + 1.0, first.1 + 1.0, 1, 50), None);
        assert_eq!(p.slot_at(first.0 - 1.0, first.1 + 1.0, 0, 100), None);
    }

    #[test]
    fn views_step_round() {
        let mut table_dir = std::env::temp_dir();
        table_dir.push(format!("horizon-jei-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&table_dir);
        let dir = table_dir.join("data/minecraft/recipe");
        std::fs::create_dir_all(&dir).unwrap();
        for (name, json) in [
            ("a", r###"{"type":"minecraft:crafting_shapeless","ingredients":["minecraft:oak_planks"],"result":{"id":"minecraft:stick"}}"###),
            ("b", r###"{"type":"minecraft:crafting_shapeless","ingredients":["minecraft:bamboo"],"result":{"id":"minecraft:stick"}}"###),
        ] {
            std::fs::write(dir.join(format!("{name}.json")), json).unwrap();
        }
        let table = RecipeTable::load(&table_dir);
        let _ = std::fs::remove_dir_all(&table_dir);
        let mut view = RecipeView::new(&table, Query::Output("minecraft:stick".into())).unwrap();
        assert_eq!(view.recipes.len(), 2);
        assert_eq!(view.index, 0);
        view.step(true);
        assert_eq!(view.index, 1);
        view.step(true);
        assert_eq!(view.index, 0);
        view.step(false);
        assert_eq!(view.index, 1);
        assert!(RecipeView::new(&table, Query::Uses("minecraft:diamond".into())).is_none());
        assert_eq!(
            Query::Output("x".into()).flipped(),
            Query::Uses("x".into())
        );
    }
}
