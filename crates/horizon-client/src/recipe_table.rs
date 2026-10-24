//! The datapack's recipes, for capability `recipes`: read once from
//! `data/minecraft/recipe` and the item tags beside them, then looked up by
//! the items they make and use.
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

/// Most items one ingredient slot lists after its tags are expanded.
const MAX_ALTERNATIVES: usize = 256;
/// How deep a tag may name other tags.
const MAX_TAG_DEPTH: u32 = 8;
/// Most ids `RecipeTable::search` returns.
const MAX_SEARCH: usize = 64;

/// One recipe, as the datapack states it.
#[derive(Clone, Debug, PartialEq)]
pub struct RecipeInfo {
    /// `minecraft:crafting_table`.
    pub id: String,
    /// `minecraft:crafting_shaped`, `minecraft:smelting`, ...
    pub kind: String,
    /// The workstation's item.
    pub station: String,
    pub result: String,
    pub result_count: u32,
    /// One entry per filled ingredient slot, each the items that fit it.
    pub ingredients: Vec<Vec<String>>,
    /// A shaped recipe's grid size (0 for the other kinds).
    pub width: usize,
    pub height: usize,
    /// A shaped recipe's grid cells, row by row (`width` x `height`): the
    /// index into `ingredients` each holds, none for an empty cell.
    pub cells: Vec<Option<usize>>,
    /// A fuel's burn time in ticks and the items one burn smelts (both 0 for
    /// the recipes).
    pub burn_time: i32,
    pub smelt_count: f32,
}

#[derive(Clone, Debug, Default)]
pub struct RecipeTable {
    recipes: Vec<RecipeInfo>,
}

fn station(kind: &str) -> &'static str {
    match kind {
        "minecraft:smelting" => "minecraft:furnace",
        "minecraft:blasting" => "minecraft:blast_furnace",
        "minecraft:smoking" => "minecraft:smoker",
        "minecraft:campfire_cooking" => "minecraft:campfire",
        "minecraft:stonecutting" => "minecraft:stonecutter",
        "minecraft:smithing_transform" => "minecraft:smithing_table",
        _ => "minecraft:crafting_table",
    }
}

/// Every `*.json` under `dir`, as (path relative to `dir` without the
/// extension, parsed file).
fn json_files(dir: &Path) -> Vec<(String, Value)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "json")
                && let Ok(rel) = path.strip_prefix(dir)
                && let Some(v) = std::fs::read(&path)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            {
                let name = rel.with_extension("").to_string_lossy().replace('\\', "/");
                out.push((name, v));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Item tags by id without `#`: `minecraft:planks` -> its entries.
struct Tags(BTreeMap<String, Vec<String>>);
impl Tags {
    fn load(root: &Path) -> Self {
        let tags = json_files(&root.join("data/minecraft/tags/item"))
            .into_iter()
            .map(|(name, v)| {
                let values = v["values"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|e| e.as_str().or_else(|| e["id"].as_str()).map(str::to_owned))
                    .collect();
                (format!("minecraft:{name}"), values)
            })
            .collect();
        Tags(tags)
    }
    /// The items an entry names: itself, or a tag's items.
    fn expand(&self, entry: &str, depth: u32, out: &mut Vec<String>) {
        if out.len() >= MAX_ALTERNATIVES {
            return;
        }
        match entry.strip_prefix('#') {
            None => {
                if !out.iter().any(|o| o == entry) {
                    out.push(entry.to_owned());
                }
            }
            Some(tag) if depth < MAX_TAG_DEPTH => {
                for e in self.0.get(tag).into_iter().flatten() {
                    self.expand(e, depth + 1, out);
                }
            }
            Some(_) => {}
        }
    }
    /// An ingredient: an item or tag id, or a list of them.
    fn ingredient(&self, v: &Value) -> Vec<String> {
        let mut out = Vec::new();
        match v {
            Value::String(s) => self.expand(s, 0, &mut out),
            Value::Array(a) => {
                for e in a {
                    if let Some(s) = e.as_str() {
                        self.expand(s, 0, &mut out);
                    }
                }
            }
            _ => {}
        }
        out
    }
}

fn parse(id: String, v: &Value, tags: &Tags) -> Option<RecipeInfo> {
    let kind = v["type"].as_str()?.to_owned();
    let (result, result_count) = match &v["result"] {
        Value::String(s) => (s.clone(), 1),
        r => (
            r["id"].as_str()?.to_owned(),
            r["count"].as_u64().unwrap_or(1) as u32,
        ),
    };
    let mut slots: Vec<Vec<String>> = Vec::new();
    let (mut width, mut height) = (0, 0);
    let mut cells: Vec<Option<usize>> = Vec::new();
    match kind.as_str() {
        "minecraft:crafting_shaped" => {
            let rows: Vec<&str> = v["pattern"]
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .collect();
            width = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
            height = rows.len();
            for row in rows {
                let mut filled = 0;
                for c in row.chars() {
                    filled += 1;
                    let alternatives = if c == ' ' {
                        Vec::new()
                    } else {
                        tags.ingredient(&v["key"][c.to_string()])
                    };
                    if alternatives.is_empty() {
                        cells.push(None);
                    } else {
                        slots.push(alternatives);
                        cells.push(Some(slots.len() - 1));
                    }
                }
                cells.extend(std::iter::repeat_n(None, width - filled));
            }
        }
        "minecraft:crafting_shapeless" => {
            for i in v["ingredients"].as_array()? {
                slots.push(tags.ingredient(i));
            }
        }
        "minecraft:smelting"
        | "minecraft:blasting"
        | "minecraft:smoking"
        | "minecraft:campfire_cooking"
        | "minecraft:stonecutting" => slots.push(tags.ingredient(&v["ingredient"])),
        "minecraft:smithing_transform" => {
            for k in ["template", "base", "addition"] {
                slots.push(tags.ingredient(&v[k]));
            }
        }
        "minecraft:crafting_transmute" => {
            for k in ["input", "material"] {
                slots.push(tags.ingredient(&v[k]));
            }
        }
        _ => return None,
    }
    slots.retain(|s| !s.is_empty());
    Some(RecipeInfo {
        id,
        station: station(&kind).to_owned(),
        kind,
        result,
        result_count,
        ingredients: slots,
        width,
        height,
        cells,
        burn_time: 0,
        smelt_count: 0.0,
    })
}

impl RecipeTable {
    /// Reads `data/minecraft/recipe/*.json` and `data/minecraft/tags/item/`
    /// under `root` (the game's vanilla resources). A recipe of a kind not
    /// handled here is left out.
    pub fn load(root: &Path) -> Self {
        let tags = Tags::load(root);
        let recipes = json_files(&root.join("data/minecraft/recipe"))
            .into_iter()
            .filter_map(|(name, v)| parse(format!("minecraft:{name}"), &v, &tags))
            .collect();
        Self { recipes }
    }
    pub fn len(&self) -> usize {
        self.recipes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }
    /// The recipes that make `item`.
    pub fn producing(&self, item: &str) -> Vec<&RecipeInfo> {
        self.recipes
            .iter()
            .filter(|r| !r.result.is_empty() && r.result == item)
            .collect()
    }
    /// The recipes that take `item` as an ingredient.
    pub fn consuming(&self, item: &str) -> Vec<&RecipeInfo> {
        self.recipes
            .iter()
            .filter(|r| r.ingredients.iter().any(|s| s.iter().any(|i| i == item)))
            .collect()
    }
    /// Adds a fuel: `item` burns for `burn_time` ticks in the furnace of
    /// `station` and smelts `smelt_count` items a burn (`FuelRecipeMaker`). A
    /// fuel makes nothing, so it is only found by what uses it.
    pub fn add_fuel(
        &mut self,
        kind: &str,
        station: &str,
        item: &str,
        burn_time: i32,
        smelt_count: f32,
    ) {
        self.recipes.push(RecipeInfo {
            id: format!("{kind}/{item}"),
            kind: kind.to_owned(),
            station: station.to_owned(),
            result: String::new(),
            result_count: 0,
            ingredients: vec![vec![item.to_owned()]],
            width: 0,
            height: 0,
            cells: Vec::new(),
            burn_time,
            smelt_count,
        });
    }
    /// Ids of items some recipe makes whose id contains `query` (lower case
    /// compared), sorted, at most [`MAX_SEARCH`].
    pub fn search(&self, query: &str) -> Vec<String> {
        let query = query.trim().to_lowercase();
        let mut found: Vec<String> = self
            .recipes
            .iter()
            .map(|r| &r.result)
            .filter(|id| !id.is_empty() && id.to_lowercase().contains(&query))
            .cloned()
            .collect();
        found.sort();
        found.dedup();
        found.truncate(MAX_SEARCH);
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, json: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, json).unwrap();
    }

    #[test]
    fn reads_shaped_cooking_and_tags() {
        let root = std::env::temp_dir().join(format!("horizon-recipes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write(
            &root,
            "data/minecraft/tags/item/planks.json",
            r###"{"values":["minecraft:oak_planks","minecraft:birch_planks"]}"###,
        );
        write(
            &root,
            "data/minecraft/tags/item/logs.json",
            r###"{"values":["#minecraft:planks","minecraft:oak_log"]}"###,
        );
        write(
            &root,
            "data/minecraft/recipe/crafting_table.json",
            r###"{"type":"minecraft:crafting_shaped","key":{"#":"#minecraft:planks"},
                 "pattern":["##","##"],"result":{"id":"minecraft:crafting_table"}}"###,
        );
        write(
            &root,
            "data/minecraft/recipe/torch.json",
            r###"{"type":"minecraft:crafting_shaped","key":{"#":"minecraft:stick",
                 "X":["minecraft:coal","minecraft:charcoal"]},"pattern":["X","#"],
                 "result":{"count":4,"id":"minecraft:torch"}}"###,
        );
        write(
            &root,
            "data/minecraft/recipe/baked_potato.json",
            r###"{"type":"minecraft:smelting","ingredient":"minecraft:potato",
                 "result":{"id":"minecraft:baked_potato"}}"###,
        );
        write(
            &root,
            "data/minecraft/recipe/stick_from_logs.json",
            r###"{"type":"minecraft:crafting_shapeless","ingredients":["#minecraft:logs"],
                 "result":{"id":"minecraft:stick","count":8}}"###,
        );
        write(
            &root,
            "data/minecraft/recipe/special.json",
            r###"{"type":"minecraft:crafting_special_firework_star"}"###,
        );
        let table = RecipeTable::load(&root);
        let _ = std::fs::remove_dir_all(&root);
        // The special recipe has no data to show.
        assert_eq!(table.len(), 4);
        // A tag naming a tag expands both, in order.
        assert_eq!(
            table.producing("minecraft:stick")[0].ingredients,
            vec![vec![
                "minecraft:oak_planks".to_owned(),
                "minecraft:birch_planks".to_owned(),
                "minecraft:oak_log".to_owned()
            ]]
        );
        let torch = table.producing("minecraft:torch");
        assert_eq!(torch.len(), 1);
        assert_eq!(torch[0].result_count, 4);
        assert_eq!(
            torch[0].ingredients,
            vec![
                vec!["minecraft:coal".to_owned(), "minecraft:charcoal".to_owned()],
                vec!["minecraft:stick".to_owned()],
            ]
        );
        // A tag expands to its items, nested tags included.
        let table_recipe = &table.producing("minecraft:crafting_table")[0];
        assert_eq!(table_recipe.ingredients.len(), 4);
        assert_eq!(
            table_recipe.ingredients[0],
            vec![
                "minecraft:oak_planks".to_owned(),
                "minecraft:birch_planks".to_owned()
            ]
        );
        assert_eq!(table_recipe.station, "minecraft:crafting_table");
        // The grid keeps its shape: 2 x 2, every cell filled.
        assert_eq!((table_recipe.width, table_recipe.height), (2, 2));
        assert_eq!(table_recipe.cells, vec![Some(0), Some(1), Some(2), Some(3)]);
        // The torch is one column: coal over stick.
        assert_eq!((torch[0].width, torch[0].height), (1, 2));
        assert_eq!(torch[0].cells, vec![Some(0), Some(1)]);
        // A shapeless recipe has no grid.
        let stick = &table.producing("minecraft:stick")[0];
        assert_eq!((stick.width, stick.height, stick.cells.len()), (0, 0, 0));
        let baked = table.consuming("minecraft:potato");
        assert_eq!(baked.len(), 1);
        assert_eq!(baked[0].station, "minecraft:furnace");
        // The crafting table and the stick recipe (through the logs tag).
        assert_eq!(table.consuming("minecraft:birch_planks").len(), 2);
        assert_eq!(table.search("TORCH"), ["minecraft:torch"]);
        assert!(table.search("nothing_like_this").is_empty());
    }

    #[test]
    fn fuels_are_used_not_made() {
        let mut table = RecipeTable::default();
        table.add_fuel(
            "minecraft:fuel_smelting",
            "minecraft:furnace",
            "minecraft:coal",
            1600,
            1.0,
        );
        let uses = table.consuming("minecraft:coal");
        assert_eq!(uses.len(), 1);
        assert_eq!((uses[0].burn_time, uses[0].smelt_count), (1600, 1.0));
        assert_eq!(uses[0].station, "minecraft:furnace");
        assert!(table.producing("").is_empty());
        assert!(table.search("coal").is_empty());
    }
}
