//! Connected textures: the OptiFine `optifine/ctm` format a resource pack
//! ships, read the way Continuity (LGPL-3.0, see THIRD_PARTY_NOTICES.md) reads
//! it and applied by the mesher. A rule names blocks (`matchBlocks`) or
//! sprites (`matchTiles`), and a method (`ctm`, `horizontal`, `vertical`,
//! `horizontal+vertical`, `vertical+horizontal`, `top`, `random`, `repeat`,
//! `fixed`) picks one of its tiles for a face from the blocks around it. A
//! tile is another layer of the sprite array, so the mesher only swaps the
//! layer of a face: its UVs stay.
use crate::sprites::Sprites;
use horizon_assets::bake::ModelQuad;
use horizon_world::registry::{BlockRegistry, BlockStateId};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::Path,
};

/// The six faces in `Direction` order: down, up, north, south, west, east.
pub const NORMALS: [[i32; 3]; 6] = [
    [0, -1, 0],
    [0, 1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];
const DOWN: usize = 0;
const UP: usize = 1;
const NORTH: usize = 2;
const SOUTH: usize = 3;
const WEST: usize = 4;
const EAST: usize = 5;

/// `CtmSpriteProvider.SPRITE_INDEX_MAP`: the tile of the 47 for the
/// connections to the left (1), below (4), right (16) and above (64) and the
/// corners between two connected sides (2, 8, 32, 128):
///
/// ```text
/// 128  64  32
///   1   *  16
///   2   4   8
/// ```
const CTM_TILES: [u8; 256] = [
    0, 3, 0, 3, 12, 5, 12, 15, 0, 3, 0, 3, 12, 5, 12, 15, //
    1, 2, 1, 2, 4, 7, 4, 29, 1, 2, 1, 2, 13, 31, 13, 14, //
    0, 3, 0, 3, 12, 5, 12, 15, 0, 3, 0, 3, 12, 5, 12, 15, //
    1, 2, 1, 2, 4, 7, 4, 29, 1, 2, 1, 2, 13, 31, 13, 14, //
    36, 17, 36, 17, 24, 19, 24, 43, 36, 17, 36, 17, 24, 19, 24, 43, //
    16, 18, 16, 18, 6, 46, 6, 21, 16, 18, 16, 18, 28, 9, 28, 22, //
    36, 17, 36, 17, 24, 19, 24, 43, 36, 17, 36, 17, 24, 19, 24, 43, //
    37, 40, 37, 40, 30, 8, 30, 34, 37, 40, 37, 40, 25, 23, 25, 45, //
    0, 3, 0, 3, 12, 5, 12, 15, 0, 3, 0, 3, 12, 5, 12, 15, //
    1, 2, 1, 2, 4, 7, 4, 29, 1, 2, 1, 2, 13, 31, 13, 14, //
    0, 3, 0, 3, 12, 5, 12, 15, 0, 3, 0, 3, 12, 5, 12, 15, //
    1, 2, 1, 2, 4, 7, 4, 29, 1, 2, 1, 2, 13, 31, 13, 14, //
    36, 39, 36, 39, 24, 41, 24, 27, 36, 39, 36, 39, 24, 41, 24, 27, //
    16, 42, 16, 42, 6, 20, 6, 10, 16, 42, 16, 42, 28, 35, 28, 44, //
    36, 39, 36, 39, 24, 41, 24, 27, 36, 39, 36, 39, 24, 41, 24, 27, //
    37, 38, 37, 38, 30, 11, 30, 32, 37, 38, 37, 38, 25, 33, 25, 26, //
];
/// `HorizontalSpriteProvider` and `VerticalSpriteProvider`: left or above (1)
/// and right or below (2).
const LINE_TILES: [u8; 4] = [3, 2, 0, 1];
/// `HorizontalVerticalSpriteProvider.SECONDARY_SPRITE_INDEX_MAP`.
const H_THEN_V: [u8; 64] = [
    3, 3, 6, 3, 3, 3, 3, 3, 3, 3, 6, 3, 3, 3, 3, 3, //
    4, 4, 5, 4, 4, 4, 4, 4, 3, 3, 6, 3, 3, 3, 3, 3, //
    3, 3, 6, 3, 3, 3, 3, 3, 3, 3, 6, 3, 3, 3, 3, 3, //
    3, 3, 6, 3, 3, 3, 3, 3, 3, 3, 6, 3, 3, 3, 3, 3, //
];
/// `VerticalHorizontalSpriteProvider.SECONDARY_SPRITE_INDEX_MAP`.
const V_THEN_H: [u8; 64] = [
    3, 6, 3, 3, 3, 6, 3, 3, 4, 5, 4, 4, 3, 6, 3, 3, //
    3, 6, 3, 3, 3, 6, 3, 3, 3, 6, 3, 3, 3, 6, 3, 3, //
    3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 3, 3, 3, 3, //
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, //
];
/// `OrientationMode.AXIS_ORIENTATIONS`: by the state's axis (x, y, z) and the
/// face.
const AXIS_ORIENTATIONS: [[u8; 6]; 3] =
    [[3, 3, 1, 3, 0, 2], [0, 0, 0, 0, 0, 0], [2, 0, 2, 0, 1, 3]];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Ctm,
    Horizontal,
    Vertical,
    HorizontalVertical,
    VerticalHorizontal,
    Top,
    Random,
    Repeat,
    Fixed,
    /// `overlay`: the 17 tiles drawn over a face at the sides and corners
    /// where a neighbour is not the same block (`StandardOverlayQuadProcessor`).
    Overlay,
    /// `overlay_fixed`, `overlay_random`, `overlay_repeat`: a tile over every
    /// face the rule matches, chosen as `fixed`, `random` or `repeat` chooses.
    OverlayFixed,
    OverlayRandom,
    OverlayRepeat,
}
impl Method {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "ctm" => Self::Ctm,
            "horizontal" => Self::Horizontal,
            "vertical" => Self::Vertical,
            "horizontal+vertical" | "h+v" => Self::HorizontalVertical,
            "vertical+horizontal" | "v+h" => Self::VerticalHorizontal,
            "top" => Self::Top,
            "random" => Self::Random,
            "repeat" => Self::Repeat,
            "fixed" | "static" => Self::Fixed,
            "overlay" => Self::Overlay,
            "overlay_fixed" => Self::OverlayFixed,
            "overlay_random" => Self::OverlayRandom,
            "overlay_repeat" => Self::OverlayRepeat,
            _ => return None,
        })
    }
    fn connects(self) -> bool {
        matches!(
            self,
            Self::Ctm
                | Self::Horizontal
                | Self::Vertical
                | Self::HorizontalVertical
                | Self::VerticalHorizontal
                | Self::Top
                | Self::Overlay
        )
    }
    /// The method whose tile choice this one uses (the overlay variants choose
    /// their tile as the plain method does).
    fn base(self) -> Self {
        match self {
            Self::OverlayFixed => Self::Fixed,
            Self::OverlayRandom => Self::Random,
            Self::OverlayRepeat => Self::Repeat,
            other => other,
        }
    }
    fn is_overlay(self) -> bool {
        matches!(
            self,
            Self::Overlay | Self::OverlayFixed | Self::OverlayRandom | Self::OverlayRepeat
        )
    }
    /// How many tiles the method needs at least.
    fn tiles(self, rule: &Rule) -> usize {
        match self.base() {
            Self::Ctm => 47,
            Self::Horizontal | Self::Vertical => 4,
            Self::HorizontalVertical | Self::VerticalHorizontal => 7,
            Self::Top | Self::Fixed | Self::Random => 1,
            Self::Repeat => rule.width * rule.height,
            Self::Overlay => 17,
            Self::OverlayFixed | Self::OverlayRandom | Self::OverlayRepeat => unreachable!(),
        }
    }
}
/// `connect`: what a neighbour must be to connect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connect {
    /// The same block.
    Block,
    /// A block showing the same sprite on that face.
    Tile,
    /// The same block state.
    State,
}
/// `orient`: how the face's directions follow the block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orient {
    None,
    StateAxis,
    Texture,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Symmetry {
    None,
    Opposite,
    All,
}
impl Symmetry {
    fn apply(self, face: usize) -> usize {
        match self {
            Self::None => face,
            Self::Opposite if NORMALS[face].iter().sum::<i32>() > 0 => face ^ 1,
            Self::Opposite => face,
            Self::All => DOWN,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tile {
    Layer(u32),
    /// `<skip>` and `<default>`: the face stays as the model has it.
    Skip,
    /// A texture that is not there.
    Missing,
}
/// An overlay rule's `layer` and `tintIndex` (`OverlayPropertiesSection`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OverlaySpec {
    translucent: bool,
    /// The model tint index the overlay takes, -1 for none.
    tint: i32,
}

/// One overlay quad the mesher draws over a face: a sprite layer, in the
/// translucent pass or the solid one, tinted by the model's tint index (-1 for
/// none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayQuad {
    pub layer: u32,
    pub translucent: bool,
    pub tint: i32,
}

#[derive(Clone, Debug)]
struct Rule {
    method: Method,
    /// Set for the overlay methods; their tiles are drawn over the face, not
    /// instead of it.
    overlay: Option<OverlaySpec>,
    /// `matchBlocks` or the block name: the rule has block matches.
    blocks: bool,
    /// `connectBlocks`: a neighbour must be one of these to connect (overlays).
    connect_blocks: bool,
    /// `connectTiles`, as sprite layers (overlays).
    connect_layers: Option<Vec<u32>>,
    connect: Connect,
    inner_seams: bool,
    orient: Orient,
    tiles: Vec<Tile>,
    /// Bit per face (`Direction` order) the rule applies to.
    faces: Option<u8>,
    heights: Vec<(i32, i32)>,
    /// `matchTiles`, as sprite layers.
    layers: Option<Vec<u32>>,
    weights: Vec<u32>,
    loops: i32,
    symmetry: Symmetry,
    linked: bool,
    width: usize,
    height: usize,
}

/// A face the mesher asks about.
pub struct Query<'a> {
    pub state: u32,
    /// The sprite layer the model gives the face.
    pub layer: u32,
    /// The face's direction (`Direction` order).
    pub face: usize,
    /// The block's world position.
    pub pos: [i32; 3],
    pub quad: &'a ModelQuad,
    /// The state at an offset from `pos` (the mesher's padded blocks).
    pub block_at: &'a dyn Fn([i32; 3]) -> u32,
}

enum Applied {
    /// This rule does not decide: try the next.
    Next,
    /// Leave the face as it is.
    Stop,
    Layer(u32),
}

#[derive(Default)]
pub struct Ctm {
    rules: Vec<Rule>,
    by_state: Vec<Vec<u16>>,
    by_layer: HashMap<u32, Vec<u16>>,
    block_of: Vec<u32>,
    /// 0 none, 1 x, 2 y, 3 z: the state's `axis`.
    axis: Vec<u8>,
    /// For `connect=tile`: the layers a state shows on a face.
    tile_layers: HashMap<(u32, u8), Box<[u32]>>,
    /// The overlay rules whose `connectBlocks` a state matches.
    connect_by_state: Vec<Vec<u16>>,
}

/// One `.properties` file under `optifine/ctm`.
pub struct Source {
    pub namespace: String,
    /// Relative to `assets/<namespace>/`, with `/`.
    pub path: String,
    pub props: HashMap<String, String>,
}

/// `java.util.Properties`: `key=value` lines, `#` and `!` comments, a `\` at
/// the end of a line continues it.
pub fn parse_properties(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let mut line = line.trim_start().to_owned();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        while line.ends_with('\\') {
            line.pop();
            match lines.next() {
                Some(next) => line.push_str(next.trim_start()),
                None => break,
            }
        }
        let end = line
            .find(|c: char| c == '=' || c == ':' || c.is_whitespace())
            .unwrap_or(line.len());
        let key = line[..end].to_owned();
        let mut rest = line[end..].trim_start();
        if rest.starts_with('=') || rest.starts_with(':') {
            rest = rest[1..].trim_start();
        }
        out.insert(key, rest.trim_end().to_owned());
    }
    out
}

/// The CTM files of a resource root (packs merged over vanilla).
pub fn discover(root: &Path) -> Vec<Source> {
    let mut out = Vec::new();
    let Ok(namespaces) = fs::read_dir(root.join("assets")) else {
        return out;
    };
    for namespace in namespaces.flatten() {
        let name = namespace.file_name().to_string_lossy().into_owned();
        let start = namespace.path().join("optifine/ctm");
        let mut stack = vec![start];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "properties")
                    && let Ok(text) = fs::read_to_string(&path)
                    && let Ok(relative) = path.strip_prefix(namespace.path())
                {
                    out.push(Source {
                        namespace: name.clone(),
                        path: relative.to_string_lossy().replace('\\', "/"),
                        props: parse_properties(&text),
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| (&a.namespace, &a.path).cmp(&(&b.namespace, &b.path)));
    out
}

fn base_path(path: &str) -> &str {
    path.rfind('/').map_or("", |i| &path[..=i])
}

/// A tile of a rule's `tiles`: its sprite name and texture file.
enum TileRef {
    Special,
    Texture { sprite: String, file: String },
}

/// `BaseCtmProperties.parseTiles`.
fn parse_tiles(source: &Source, text: &str) -> Vec<TileRef> {
    let base = base_path(&source.path).to_owned();
    let mut out = Vec::new();
    let texture = |namespace: &str, path: &str| -> TileRef {
        // `path` is relative to the namespace's assets, without `.png`.
        let sprite = match path.strip_prefix("textures/") {
            Some(rest) => format!("{namespace}:{rest}"),
            None => format!("{namespace}:../{path}"),
        };
        TileRef::Texture {
            sprite,
            file: format!("assets/{namespace}/{path}.png"),
        }
    };
    for token in text.split([' ', ',']).filter(|t| !t.is_empty()) {
        if token.ends_with("<skip>") || token.ends_with("<skip>.png") {
            out.push(TileRef::Special);
            continue;
        }
        if token.ends_with("<default>") || token.ends_with("<default>.png") {
            out.push(TileRef::Special);
            continue;
        }
        if let Some((a, b)) = token.split_once('-')
            && let (Ok(min), Ok(max)) = (a.parse::<i32>(), b.parse::<i32>())
            && min <= max
        {
            for n in min..=max {
                out.push(texture(&source.namespace, &format!("{base}{n}")));
            }
            continue;
        }
        let (mut namespace, mut path) = match token.split_once(':') {
            Some((n, p)) => (Some(n.to_owned()), p.to_owned()),
            None => (None, token.to_owned()),
        };
        if let Some(stripped) = path.strip_suffix(".png") {
            path = stripped.to_owned();
        }
        if namespace.is_none() {
            if let Some(rest) = path.strip_prefix("assets/minecraft/") {
                path = rest.to_owned();
            } else if let Some(rest) = path.strip_prefix("./") {
                path = format!("{base}{rest}");
            } else if let Some(rest) = path.strip_prefix("~/") {
                path = format!("optifine/{rest}");
            } else if let Some(rest) = path.strip_prefix('/') {
                path = format!("optifine/{rest}");
            }
            if !path.starts_with("textures/") && !path.starts_with("optifine/") {
                path = format!("{base}{path}");
            }
            if path.starts_with("optifine/") {
                namespace = Some(source.namespace.clone());
            }
        } else if !path.contains('/') {
            path = format!("textures/block/{path}");
        } else if !path.starts_with("textures/") && !path.starts_with("optifine/") {
            path = format!("textures/{path}");
        }
        out.push(texture(namespace.as_deref().unwrap_or("minecraft"), &path));
    }
    out
}

/// `PropertiesParsingHelper.parseMatchTiles`: sprite names.
fn parse_match_tiles(source: &Source, text: &str) -> Vec<String> {
    let base = base_path(&source.path).to_owned();
    let mut out = Vec::new();
    for token in text.split(' ').filter(|t| !t.is_empty()) {
        let (namespace, mut path) = match token.split_once(':') {
            Some((n, p)) => (Some(n.to_owned()), p.to_owned()),
            None => (None, token.to_owned()),
        };
        if let Some(stripped) = path.strip_suffix(".png") {
            path = stripped.to_owned();
        }
        if namespace.is_none() {
            if let Some(rest) = path.strip_prefix("assets/minecraft/") {
                path = rest.to_owned();
            } else if let Some(rest) = path.strip_prefix("./") {
                path = format!("{base}{rest}");
            } else if let Some(rest) = path.strip_prefix("~/") {
                path = format!("optifine/{rest}");
            } else if let Some(rest) = path.strip_prefix('/') {
                path = format!("optifine/{rest}");
            }
        }
        if let Some(rest) = path.strip_prefix("textures/") {
            path = rest.to_owned();
        } else if path.starts_with("optifine/") {
            // A texture of the pack's own as the match: not a model sprite.
            continue;
        } else if !path.contains('/') {
            path = format!("block/{path}");
        }
        out.push(format!(
            "{}:{path}",
            namespace.as_deref().unwrap_or("minecraft")
        ));
    }
    out
}

/// A `matchBlocks` element: a block and the property values it must have.
#[derive(Clone, Debug)]
struct BlockMatch {
    name: String,
    properties: Vec<(String, Vec<String>)>,
}

/// `PropertiesParsingHelper.parseBlockStates`.
fn parse_match_blocks(text: &str) -> Vec<BlockMatch> {
    let mut out = Vec::new();
    for token in text.split(' ').filter(|t| !t.is_empty()) {
        let parts: Vec<&str> = token.split(':').collect();
        let (name, start) = if parts.len() == 1 || parts[1].contains('=') {
            (format!("minecraft:{}", parts[0]), 1)
        } else {
            (format!("{}:{}", parts[0], parts[1]), 2)
        };
        let mut properties = Vec::new();
        for part in &parts[start.min(parts.len())..] {
            if let Some((key, values)) = part.split_once('=') {
                properties.push((
                    key.to_owned(),
                    values.split(',').map(str::to_owned).collect(),
                ));
            }
        }
        out.push(BlockMatch { name, properties });
    }
    out
}

fn parse_faces(text: &str) -> Option<Option<u8>> {
    let mut mask = 0u8;
    for token in text.split([' ', ',']).filter(|t| !t.is_empty()) {
        match token.to_ascii_uppercase().as_str() {
            "BOTTOM" | "DOWN" => mask |= 1 << DOWN,
            "TOP" | "UP" => mask |= 1 << UP,
            "NORTH" => mask |= 1 << NORTH,
            "SOUTH" => mask |= 1 << SOUTH,
            "WEST" => mask |= 1 << WEST,
            "EAST" => mask |= 1 << EAST,
            "SIDES" => mask |= 1 << NORTH | 1 << SOUTH | 1 << WEST | 1 << EAST,
            "ALL" => return Some(None),
            _ => {}
        }
    }
    match mask {
        0 => None,
        0b11_1111 => Some(None),
        m => Some(Some(m)),
    }
}

/// `heights` and the legacy `minHeight` and `maxHeight`.
fn parse_heights(props: &HashMap<String, String>) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    if let Some(text) = props.get("heights") {
        for token in text.split([' ', ',']).filter(|t| !t.is_empty()) {
            if let Some((a, b)) = token.split_once("..") {
                let min = if a.is_empty() {
                    Some(i32::MIN)
                } else {
                    a.parse().ok()
                };
                let max = if b.is_empty() {
                    Some(i32::MAX)
                } else {
                    b.parse().ok()
                };
                if let (Some(min), Some(max)) = (min, max) {
                    out.push((min.min(max), min.max(max)));
                }
                continue;
            }
            let t: String = token.chars().filter(|c| !"()".contains(*c)).collect();
            let split = t
                .char_indices()
                .skip(1)
                .find(|&(_, c)| c == '-')
                .map(|(i, _)| i);
            match split {
                None => {
                    if let Ok(y) = t.parse::<i32>() {
                        out.push((y, y));
                    }
                }
                Some(i) => {
                    if let (Ok(a), Ok(b)) = (t[..i].parse::<i32>(), t[i + 1..].parse::<i32>()) {
                        out.push((a.min(b), a.max(b)));
                    }
                }
            }
        }
    } else {
        let min = props
            .get("minHeight")
            .and_then(|v| v.trim().parse::<i32>().ok());
        let max = props
            .get("maxHeight")
            .and_then(|v| v.trim().parse::<i32>().ok());
        match (min, max) {
            (Some(a), Some(b)) => out.push((a.min(b), a.max(b))),
            (Some(a), None) => out.push((a, i32::MAX)),
            (None, Some(b)) => out.push((i32::MIN, b)),
            (None, None) => {}
        }
    }
    out
}

/// `weights`: `3`, `1-3` and lists of them.
fn parse_weights(text: &str) -> Vec<u32> {
    let mut out = Vec::new();
    for token in text.split([' ', ',']).filter(|t| !t.is_empty()) {
        match token.split_once('-') {
            Some((a, b)) => {
                if let (Ok(min), Ok(max)) = (a.parse::<u32>(), b.parse::<u32>())
                    && min > 0
                    && max >= min
                {
                    out.extend(min..=max);
                }
            }
            None => {
                if let Ok(w) = token.parse::<u32>()
                    && w > 0
                {
                    out.push(w);
                }
            }
        }
    }
    out
}

fn parse_bool(props: &HashMap<String, String>, key: &str) -> bool {
    props
        .get(key)
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

/// The face a face of a block with an `axis` is, for `faces`
/// (`BaseProcessingPredicate`): a log lying along x or z counts its ends as
/// its top and bottom.
fn effective_face(axis: u8, face: usize) -> usize {
    match (axis, face) {
        // rotateClockwise(Z)
        (1, UP) => EAST,
        (1, EAST) => DOWN,
        (1, DOWN) => WEST,
        (1, WEST) => UP,
        // rotateCounterclockwise(X)
        (3, NORTH) => UP,
        (3, UP) => SOUTH,
        (3, SOUTH) => DOWN,
        (3, DOWN) => NORTH,
        _ => face,
    }
}

/// `DirectionMaps`: for a face and an orientation, the directions of the
/// texture's left, down, right and up.
fn directions(face: usize, orientation: u8) -> [[i32; 3]; 4] {
    let n = NORMALS[face];
    let up = match face {
        UP => NORMALS[NORTH],
        DOWN => NORMALS[SOUTH],
        _ => NORMALS[UP],
    };
    // left = normal x up, seen from outside the face.
    let left = [
        n[1] * up[2] - n[2] * up[1],
        n[2] * up[0] - n[0] * up[2],
        n[0] * up[1] - n[1] * up[0],
    ];
    let neg = |v: [i32; 3]| v.map(|c| -c);
    let mut map = [left, neg(up), neg(left), up];
    let turns = (orientation & 3) as usize;
    map.rotate_left(turns);
    if orientation >= 4 {
        map.swap(0, 2);
    }
    map
}

/// `QuadUtil.getTextureOrientation`: how the quad's texture sits on its face,
/// 0 to 3 turns, plus 4 when the texture is mirrored.
fn texture_orientation(quad: &ModelQuad, face: usize) -> u8 {
    let (u, v) = (|i: usize| quad.uv[i][0], |i: usize| quad.uv[i][1]);
    let tm00 = u(3) - u(1);
    let tm01 = v(3) - v(1);
    let tm10 = u(2) - u(0);
    let tm11 = v(2) - v(0);
    let determinant = tm00 * tm11 - tm10 * tm01;
    if determinant == 0.0 {
        return 0;
    }
    let s = 1.0 / determinant;
    let itm10 = -tm10 * s;
    let itm11 = tm00 * s;
    let (x_axis, x_sign, y_axis, y_sign) = match face {
        DOWN => (0, 1.0, 2, 1.0),
        UP => (0, 1.0, 2, -1.0),
        NORTH => (0, -1.0, 1, 1.0),
        SOUTH => (0, 1.0, 1, 1.0),
        WEST => (2, 1.0, 1, 1.0),
        EAST => (2, -1.0, 1, 1.0),
        _ => return 0,
    };
    let p = &quad.positions;
    let pm00 = p[3][x_axis] - p[1][x_axis];
    let pm01 = p[3][y_axis] - p[1][y_axis];
    let pm10 = p[2][x_axis] - p[0][x_axis];
    let pm11 = p[2][y_axis] - p[0][y_axis];
    let x = -(pm00 * itm10 + pm10 * itm11) * x_sign;
    let y = -(pm01 * itm10 + pm11 * itm11) * y_sign;
    let turn = if y.abs() >= x.abs() {
        if y > 0.0 { 0 } else { 2 }
    } else if x > 0.0 {
        3
    } else {
        1
    };
    turn + if determinant < 0.0 { 4 } else { 0 }
}

// `MathUtil`, borrowed there from SplittableRandom.
const GOLDEN_GAMMA: u64 = 0x9e37_79b9_7f4a_7c15;
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}
fn mix32(mut z: u64) -> i32 {
    z = (z ^ (z >> 33)).wrapping_mul(0x62a9_d9ed_7997_05f5);
    ((z ^ (z >> 28)).wrapping_mul(0xcb24_d0a5_c88c_35b3) >> 32) as u32 as i32
}
/// `Mth.getSeed`'s hash of a position.
fn hash_code(x: i32, y: i32, z: i32) -> i64 {
    let mut l = i64::from(x.wrapping_mul(3_129_871))
        ^ i64::from(z).wrapping_mul(116_129_781)
        ^ i64::from(y);
    l = l
        .wrapping_mul(l)
        .wrapping_mul(42_317_861)
        .wrapping_add(l.wrapping_mul(11));
    l >> 16
}
fn mix(x: i32, y: i32, z: i32, face: usize, loops: i32) -> i32 {
    mix32(
        (hash_code(x, y, z) as u64 ^ mix64(GOLDEN_GAMMA.wrapping_mul(1 + face as u64)))
            .wrapping_add(GOLDEN_GAMMA.wrapping_mul(1 + loops as u64)),
    )
}

fn add(a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

impl Ctm {
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Reads the rules of `root` and gives their tiles sprite layers. Rules
    /// that do not parse or use a method this does not know are logged and
    /// left out.
    pub fn load(
        root: &Path,
        registry: &BlockRegistry,
        sprites: &mut Sprites,
        catalog: &crate::models::StateModels,
    ) -> Self {
        let sources = discover(root);
        if sources.is_empty() {
            return Self::default();
        }
        struct Def {
            rule: Rule,
            blocks: Option<Vec<BlockMatch>>,
            connect_blocks: Option<Vec<BlockMatch>>,
            priority: bool,
            path: String,
        }
        let mut defs: Vec<Def> = Vec::new();
        for source in &sources {
            let props = &source.props;
            let label = format!("{}:{}", source.namespace, source.path);
            let method_name = props.get("method").map_or("ctm", |m| m.trim());
            let Some(method) = Method::parse(method_name) else {
                log::warn!(
                    "Horizon: connected textures: method '{method_name}' of {label} is not supported"
                );
                continue;
            };
            if props.contains_key("biomes") || props.contains_key("name") {
                log::warn!(
                    "Horizon: connected textures: {label} filters by biome or name, which is not supported"
                );
                continue;
            }
            let tiles: Vec<Tile> =
                parse_tiles(source, props.get("tiles").map_or("", String::as_str))
                    .into_iter()
                    .map(|t| match t {
                        TileRef::Special => Tile::Skip,
                        TileRef::Texture { sprite, file } => {
                            if root.join(&file).is_file() {
                                sprites
                                    .insert(root, &sprite)
                                    .map_or(Tile::Missing, Tile::Layer)
                            } else {
                                Tile::Missing
                            }
                        }
                    })
                    .collect();
            let match_tiles = props
                .get("matchTiles")
                .map(|t| parse_match_tiles(source, t));
            let mut blocks = props.get("matchBlocks").map(|t| parse_match_blocks(t));
            if blocks.is_none()
                && let Some(name) = Path::new(&source.path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.strip_prefix("block_"))
            {
                blocks = Some(vec![BlockMatch {
                    name: format!("minecraft:{name}"),
                    properties: Vec::new(),
                }]);
            }
            if blocks.is_none() && match_tiles.is_none() {
                log::warn!("Horizon: connected textures: {label} names no blocks or tiles");
                continue;
            }
            let layers = match_tiles.map(|names| {
                names
                    .iter()
                    .filter_map(|name| sprites.layers.get(name).copied())
                    .collect::<Vec<u32>>()
            });
            if layers.as_ref().is_some_and(Vec::is_empty) && blocks.is_none() {
                // No model uses these sprites.
                continue;
            }
            let connect = match props.get("connect").map(|c| c.trim().to_ascii_lowercase()) {
                Some(c) if c == "block" => Connect::Block,
                Some(c) if c == "tile" => Connect::Tile,
                Some(c) if c == "state" => Connect::State,
                _ if blocks.is_some() => Connect::Block,
                _ => Connect::Tile,
            };
            let orient = match props.get("orient").map(|c| c.trim().to_ascii_lowercase()) {
                Some(c) if c == "none" => Orient::None,
                Some(c) if c == "state_axis" => Orient::StateAxis,
                Some(c) if c == "texture" => Orient::Texture,
                _ if method.base() == Method::Repeat => Orient::None,
                _ => Orient::Texture,
            };
            let symmetry = match props.get("symmetry").map(|c| c.trim().to_ascii_lowercase()) {
                Some(c) if c == "opposite" => Symmetry::Opposite,
                Some(c) if c == "all" => Symmetry::All,
                _ => Symmetry::None,
            };
            let number = |key: &str| props.get(key).and_then(|v| v.trim().parse::<usize>().ok());
            let faces = match props.get("faces") {
                Some(text) => match parse_faces(text) {
                    Some(mask) => mask,
                    None => continue,
                },
                None => None,
            };
            let overlay = method.is_overlay().then(|| {
                if props.contains_key("tintBlock") {
                    log::warn!("Horizon: connected textures: {label} has a tintBlock, which is not supported; only tintIndex is used");
                }
                OverlaySpec {
                    translucent: props
                        .get("layer")
                        .is_some_and(|v| v.trim().eq_ignore_ascii_case("translucent")),
                    tint: props
                        .get("tintIndex")
                        .and_then(|v| v.trim().parse::<i32>().ok())
                        .filter(|v| *v >= 0)
                        .unwrap_or(-1),
                }
            });
            let connect_layers = props.get("connectTiles").map(|text| {
                parse_match_tiles(source, text)
                    .iter()
                    .filter_map(|name| sprites.layers.get(name).copied())
                    .collect::<Vec<u32>>()
            });
            let has_blocks = blocks.is_some();
            let has_connect_blocks = props.contains_key("connectBlocks");
            let connect_blocks = props.get("connectBlocks").map(|t| parse_match_blocks(t));
            let rule = Rule {
                method,
                overlay,
                blocks: has_blocks,
                connect_blocks: has_connect_blocks,
                connect_layers,
                connect,
                inner_seams: parse_bool(props, "innerSeams"),
                orient,
                tiles,
                faces,
                heights: parse_heights(props),
                layers,
                weights: props
                    .get("weights")
                    .map(|w| parse_weights(w))
                    .unwrap_or_default(),
                loops: props
                    .get("randomLoops")
                    .and_then(|v| v.trim().parse::<i32>().ok())
                    .filter(|v| (0..=9).contains(v))
                    .unwrap_or(0),
                symmetry,
                linked: parse_bool(props, "linked"),
                width: number("width").unwrap_or(0),
                height: number("height").unwrap_or(0),
            };
            if method.base() == Method::Repeat && (rule.width == 0 || rule.height == 0) {
                log::warn!("Horizon: connected textures: {label} needs a width and a height");
                continue;
            }
            if rule.tiles.len() < method.tiles(&rule) {
                log::warn!(
                    "Horizon: connected textures: {label} has {} tiles, its method needs {}",
                    rule.tiles.len(),
                    method.tiles(&rule)
                );
                continue;
            }
            let priority = props.get("prioritize").map_or(rule.layers.is_some(), |v| {
                v.trim().eq_ignore_ascii_case("true")
            });
            defs.push(Def {
                rule,
                blocks,
                connect_blocks,
                priority,
                path: label,
            });
        }
        // Prioritised rules first, then by name.
        defs.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.path.cmp(&b.path))
        });

        let states = registry.len();
        let mut names: HashMap<&str, u32> = HashMap::new();
        let mut block_of = Vec::with_capacity(states);
        let mut axis = Vec::with_capacity(states);
        let mut wanted: HashMap<&str, Vec<(usize, &BlockMatch)>> = HashMap::new();
        for (i, def) in defs.iter().enumerate() {
            for m in def.blocks.iter().flatten() {
                wanted.entry(m.name.as_str()).or_default().push((i, m));
            }
        }
        let mut by_state: Vec<Vec<u16>> = vec![Vec::new(); states];
        let mut connect_by_state: Vec<Vec<u16>> = vec![Vec::new(); states];
        let mut wanted_connect: HashMap<&str, Vec<(usize, &BlockMatch)>> = HashMap::new();
        for (i, def) in defs.iter().enumerate() {
            for m in def.connect_blocks.iter().flatten() {
                wanted_connect
                    .entry(m.name.as_str())
                    .or_default()
                    .push((i, m));
            }
        }
        for id in 0..states {
            let Some(state) = registry.state(BlockStateId(id as u32)) else {
                block_of.push(u32::MAX);
                axis.push(0);
                continue;
            };
            let next = names.len() as u32;
            block_of.push(*names.entry(state.name.as_str()).or_insert(next));
            axis.push(match state.properties.get("axis").map(String::as_str) {
                Some("x") => 1,
                Some("y") => 2,
                Some("z") => 3,
                _ => 0,
            });
            if let Some(list) = wanted.get(state.name.as_str()) {
                for &(i, m) in list {
                    let ok = m.properties.iter().all(|(key, values)| {
                        state
                            .properties
                            .get(key)
                            .is_none_or(|have| values.iter().any(|v| v == have))
                    });
                    if ok && !by_state[id].contains(&(i as u16)) {
                        by_state[id].push(i as u16);
                    }
                }
            }
            if let Some(list) = wanted_connect.get(state.name.as_str()) {
                for &(i, m) in list {
                    let ok = m.properties.iter().all(|(key, values)| {
                        state
                            .properties
                            .get(key)
                            .is_none_or(|have| values.iter().any(|v| v == have))
                    });
                    if ok {
                        connect_by_state[id].push(i as u16);
                    }
                }
            }
        }
        let mut by_layer: HashMap<u32, Vec<u16>> = HashMap::new();
        for (i, def) in defs.iter().enumerate() {
            if def.blocks.is_none() {
                for &layer in def.rule.layers.iter().flatten() {
                    by_layer.entry(layer).or_default().push(i as u16);
                }
            }
        }
        let needs_tiles = defs.iter().any(|d| {
            (d.rule.connect == Connect::Tile && d.rule.method.connects())
                || (d.rule.overlay.is_some()
                    && (d.rule.layers.is_some() || d.rule.connect_layers.is_some()))
        });
        let mut tile_layers = HashMap::new();
        if needs_tiles {
            let mut sets: BTreeMap<(u32, u8), Vec<u32>> = BTreeMap::new();
            for id in 0..states {
                let Ok(models) = catalog.collect(
                    BlockStateId(id as u32),
                    horizon_core::position::BlockPos::new(0, 0, 0),
                ) else {
                    continue;
                };
                for (_, model) in models {
                    for face in &model.faces {
                        let list = sets.entry((id as u32, face.quad.facing as u8)).or_default();
                        if !list.contains(&face.layer) {
                            list.push(face.layer);
                        }
                    }
                }
            }
            tile_layers = sets
                .into_iter()
                .map(|(k, v)| (k, v.into_boxed_slice()))
                .collect();
        }
        let ctm = Self {
            rules: defs.into_iter().map(|d| d.rule).collect(),
            by_state,
            by_layer,
            block_of,
            axis,
            tile_layers,
            connect_by_state,
        };
        log::info!("Horizon: {} connected texture rules", ctm.rules.len());
        ctm
    }

    /// The layer to draw the face with instead of the model's, if a rule
    /// gives it one.
    pub fn resolve(&self, q: &Query) -> Option<u32> {
        let by_state = self
            .by_state
            .get(q.state as usize)
            .map_or(&[][..], Vec::as_slice);
        let by_layer = self.by_layer.get(&q.layer).map_or(&[][..], Vec::as_slice);
        if by_state.is_empty() && by_layer.is_empty() {
            return None;
        }
        // Both lists are in rule order: walk them together.
        let (mut a, mut b) = (0, 0);
        loop {
            let next = match (by_state.get(a), by_layer.get(b)) {
                (Some(&x), Some(&y)) if x < y => {
                    a += 1;
                    x
                }
                (Some(_), Some(&y)) => {
                    b += 1;
                    y
                }
                (Some(&x), None) => {
                    a += 1;
                    x
                }
                (None, Some(&y)) => {
                    b += 1;
                    y
                }
                (None, None) => return None,
            };
            let rule = &self.rules[next as usize];
            // Overlays draw over the face (`overlays`), they do not replace its layer.
            if rule.overlay.is_some() {
                continue;
            }
            if let Some(layers) = &rule.layers
                && !layers.contains(&q.layer)
            {
                continue;
            }
            match self.apply(rule, q) {
                Applied::Next => {}
                Applied::Stop => return None,
                Applied::Layer(layer) => return Some(layer),
            }
        }
    }

    fn axis_of(&self, state: u32) -> u8 {
        self.axis.get(state as usize).copied().unwrap_or(0)
    }

    /// Whether the rule's `heights` and `faces` take this face.
    fn face_matches(&self, rule: &Rule, q: &Query) -> bool {
        if !rule.heights.is_empty()
            && !rule
                .heights
                .iter()
                .any(|&(lo, hi)| (lo..=hi).contains(&q.pos[1]))
        {
            return false;
        }
        match rule.faces {
            Some(mask) => mask & (1 << effective_face(self.axis_of(q.state), q.face)) != 0,
            None => true,
        }
    }

    fn apply(&self, rule: &Rule, q: &Query) -> Applied {
        if !self.face_matches(rule, q) {
            return Applied::Next;
        }
        let tile = match rule.method.base() {
            Method::Ctm => {
                let dirs = self.directions(rule, q);
                let mut connections = 0usize;
                for (i, d) in dirs.iter().enumerate() {
                    if self.connects(rule, q, *d) {
                        connections |= 1 << (i * 2);
                    }
                }
                for i in 0..4 {
                    let (first, second) = (i, (i + 1) % 4);
                    if connections >> (first * 2) & 1 == 1
                        && connections >> (second * 2) & 1 == 1
                        && self.connects(rule, q, add(dirs[first], dirs[second]))
                    {
                        connections |= 1 << (i * 2 + 1);
                    }
                }
                rule.tiles[CTM_TILES[connections] as usize]
            }
            Method::Horizontal | Method::Vertical => {
                let dirs = self.directions(rule, q);
                let step = usize::from(rule.method == Method::Vertical);
                let mut connections = 0usize;
                for i in 0..2 {
                    if self.connects(rule, q, dirs[i * 2 + step]) {
                        connections |= 1 << i;
                    }
                }
                rule.tiles[LINE_TILES[connections] as usize]
            }
            Method::HorizontalVertical => {
                let dirs = self.directions(rule, q);
                let mut connections = 0usize;
                for i in 0..2 {
                    if self.connects(rule, q, dirs[i * 2]) {
                        connections |= 1 << i;
                    }
                }
                if connections != 0 {
                    rule.tiles[LINE_TILES[connections] as usize]
                } else {
                    let mut secondary = 0usize;
                    for i in 0..2 {
                        let direction = dirs[i * 2 + 1];
                        if self.connects(rule, q, direction) {
                            secondary |= 1 << (i * 3 + 1);
                            for j in 0..2 {
                                if self.connects(rule, q, add(direction, dirs[((i + j) % 2) * 2])) {
                                    secondary |= 1 << (i * 3 + j * 2);
                                }
                            }
                        }
                    }
                    rule.tiles[H_THEN_V[secondary] as usize]
                }
            }
            Method::VerticalHorizontal => {
                let dirs = self.directions(rule, q);
                let mut connections = 0usize;
                for i in 0..2 {
                    if self.connects(rule, q, dirs[i * 2 + 1]) {
                        connections |= 1 << i;
                    }
                }
                if connections != 0 {
                    rule.tiles[LINE_TILES[connections] as usize]
                } else {
                    let mut secondary = 0usize;
                    for i in 0..2 {
                        let direction = dirs[i * 2];
                        if self.connects(rule, q, direction) {
                            secondary |= 1 << (i * 3);
                            for j in 0..2 {
                                if self.connects(
                                    rule,
                                    q,
                                    add(direction, dirs[((i + j) % 2) * 2 + 1]),
                                ) {
                                    secondary |= 1 << ((i * 3 + j * 2 + 5) % 6);
                                }
                            }
                        }
                    }
                    rule.tiles[V_THEN_H[secondary] as usize]
                }
            }
            Method::Top => {
                let axis = match self.axis_of(q.state) {
                    1 => 0,
                    3 => 2,
                    _ => 1,
                };
                // The face's own axis (x, y, z) and the block's: the top of a
                // log lying along x is its east end.
                let face_axis = match q.face {
                    WEST | EAST => 0,
                    DOWN | UP => 1,
                    _ => 2,
                };
                if face_axis == axis {
                    return Applied::Next;
                }
                let up = NORMALS[[EAST, UP, SOUTH][axis]];
                if self.connects(rule, q, up) {
                    rule.tiles[0]
                } else {
                    return Applied::Next;
                }
            }
            Method::Fixed => rule.tiles[0],
            // Drawn by `overlays`, never as the face's own layer.
            Method::Overlay
            | Method::OverlayFixed
            | Method::OverlayRandom
            | Method::OverlayRepeat => {
                return Applied::Next;
            }
            Method::Random => {
                let [x, mut y, z] = q.pos;
                if rule.linked {
                    let block = self.block_of(q.state);
                    let mut i = 0;
                    let mut at = y;
                    loop {
                        at -= 1;
                        i += 1;
                        let other = (q.block_at)([0, at - q.pos[1], 0]);
                        if !(i < 3 && self.block_of(other) == block) {
                            break;
                        }
                    }
                    y = at + 1;
                }
                let seed = mix(x, y, z, rule.symmetry.apply(q.face), rule.loops);
                let index = random_index(seed, &rule.weights, rule.tiles.len());
                rule.tiles[index]
            }
            Method::Repeat => {
                let face = rule.symmetry.apply(q.face);
                let [x, y, z] = q.pos;
                let (mut sx, mut sy) = match face {
                    DOWN => (x, -z - 1),
                    UP => (x, z),
                    NORTH => (-x - 1, -y),
                    SOUTH => (x, -y),
                    WEST => (z, -y),
                    EAST => (-z - 1, -y),
                    _ => (0, 0),
                };
                match self.orientation(rule, q) {
                    1 => (sx, sy) = (-sy - 1, sx),
                    2 => (sx, sy) = (-sx - 1, -sy - 1),
                    3 => (sx, sy) = (sy, -sx - 1),
                    4 => sx = -sx - 1,
                    5 => (sx, sy) = (sy, sx),
                    6 => sy = -sy - 1,
                    7 => (sx, sy) = (-sy - 1, -sx - 1),
                    _ => {}
                }
                let (w, h) = (rule.width as i32, rule.height as i32);
                rule.tiles[(w * sy.rem_euclid(h) + sx.rem_euclid(w)) as usize]
            }
        };
        match tile {
            Tile::Layer(layer) => Applied::Layer(layer),
            Tile::Skip => Applied::Stop,
            Tile::Missing => Applied::Next,
        }
    }

    /// The overlays a face gets, in rule order, appended to `out`. `solid_full`
    /// gives a state's (`isSolidRender`, `isCollisionShapeFullBlock`).
    pub fn overlays(
        &self,
        q: &Query,
        solid_full: &dyn Fn(u32) -> (bool, bool),
        out: &mut Vec<OverlayQuad>,
    ) {
        let by_state = self
            .by_state
            .get(q.state as usize)
            .map_or(&[][..], Vec::as_slice);
        let by_layer = self.by_layer.get(&q.layer).map_or(&[][..], Vec::as_slice);
        for &i in by_state.iter().chain(by_layer) {
            let rule = &self.rules[i as usize];
            let Some(spec) = rule.overlay else {
                continue;
            };
            if let Some(layers) = &rule.layers
                && !layers.contains(&q.layer)
            {
                continue;
            }
            let quad = |layer| OverlayQuad {
                layer,
                translucent: spec.translucent,
                tint: spec.tint,
            };
            if rule.method == Method::Overlay {
                if !self.face_matches(rule, q) {
                    continue;
                }
                for tile in self.standard_overlay(usize::from(i), rule, q, solid_full) {
                    if let Tile::Layer(layer) = rule.tiles[tile] {
                        out.push(quad(layer));
                    }
                }
            } else if let Applied::Layer(layer) = self.apply(rule, q) {
                out.push(quad(layer));
            }
        }
    }

    /// `StandardOverlayQuadProcessor`: the tiles (indices into the rule's tiles)
    /// an overlay draws on a face, from its four sides and their corners. The
    /// sides are, in `directions` order, left, down, right and up.
    fn standard_overlay(
        &self,
        index: usize,
        rule: &Rule,
        q: &Query,
        solid_full: &dyn Fn(u32) -> (bool, bool),
    ) -> Vec<usize> {
        let light = NORMALS[q.face];
        let dirs = self.directions(rule, q);
        let solid = |offset: [i32; 3]| solid_full((q.block_at)(offset)).0;
        // A side is looked at only when the block in front of it is not solid.
        let mut sides: [Option<u32>; 4] = [None; 4];
        let mut applied = 0usize;
        for (i, &d) in dirs.iter().enumerate() {
            if !solid(add(d, light)) {
                sides[i] = Some((q.block_at)(d));
                if self.overlay_applies(index, rule, q, d, solid_full) {
                    applied |= 1 << i;
                }
            }
        }
        let same = |i: usize| self.overlay_same(index, rule, sides[i], q.face);
        let corner = |a: usize, b: usize| {
            let at = add(dirs[a], dirs[b]);
            self.overlay_applies(index, rule, q, at, solid_full) && !solid(add(at, light))
        };
        // Two adjacent sides, with the corner between them.
        let two = |a: usize, b: usize, side: usize, corner_tile: usize| {
            let mut tiles = vec![side];
            if (same(a) || same(b)) && corner(a, b) {
                tiles.push(corner_tile);
            }
            tiles
        };
        // One side, with the corners at its two ends.
        let one = |a: usize, b: usize, c: usize, side: usize, ab: usize, bc: usize| {
            let (near_ab, near_bc) = if same(b) {
                (true, true)
            } else {
                (same(a), same(c))
            };
            let mut tiles = vec![side];
            if near_ab && corner(a, b) {
                tiles.push(ab);
            }
            if near_bc && corner(b, c) {
                tiles.push(bc);
            }
            tiles
        };
        match applied {
            0b1111 => vec![8],
            0b0111 => vec![5],
            0b1011 => vec![6],
            0b1101 => vec![13],
            0b1110 => vec![12],
            0b0101 => vec![9, 7],
            0b1010 => vec![1, 15],
            0b0011 => two(2, 3, 4, 14),
            0b0110 => two(3, 0, 3, 16),
            0b1100 => two(0, 1, 10, 2),
            0b1001 => two(1, 2, 11, 0),
            0b0001 => one(1, 2, 3, 9, 0, 14),
            0b0010 => one(2, 3, 0, 1, 14, 16),
            0b0100 => one(3, 0, 1, 7, 16, 2),
            0b1000 => one(0, 1, 2, 15, 2, 0),
            // No side: only the corners next to a side that has the overlay.
            _ => {
                let mut tiles = Vec::new();
                if (same(0) || same(1)) && corner(0, 1) {
                    tiles.push(2);
                }
                if (same(1) || same(2)) && corner(1, 2) {
                    tiles.push(0);
                }
                if (same(2) || same(3)) && corner(2, 3) {
                    tiles.push(14);
                }
                if (same(3) || same(0)) && corner(3, 0) {
                    tiles.push(16);
                }
                tiles
            }
        }
    }

    /// `appliesOverlay` at `offset`: the block there is a full block, it matches
    /// the rule's `connectBlocks` and `connectTiles`, and it does not connect.
    fn overlay_applies(
        &self,
        index: usize,
        rule: &Rule,
        q: &Query,
        offset: [i32; 3],
        solid_full: &dyn Fn(u32) -> (bool, bool),
    ) -> bool {
        let other = (q.block_at)(offset);
        if !solid_full(other).1 {
            return false;
        }
        if rule.connect_blocks
            && !self
                .connect_by_state
                .get(other as usize)
                .is_some_and(|list| list.contains(&(index as u16)))
        {
            return false;
        }
        if let Some(layers) = &rule.connect_layers
            && !self.shows_any(other, q.face, layers)
        {
            return false;
        }
        !self.same(rule, q, other)
    }

    /// `hasSameOverlay`: the neighbour is a block the rule matches, and shows
    /// the rule's `matchTiles` on the face.
    fn overlay_same(&self, index: usize, rule: &Rule, other: Option<u32>, face: usize) -> bool {
        let Some(other) = other else {
            return false;
        };
        if rule.blocks
            && !self
                .by_state
                .get(other as usize)
                .is_some_and(|list| list.contains(&(index as u16)))
        {
            return false;
        }
        rule.layers
            .as_ref()
            .is_none_or(|layers| self.shows_any(other, face, layers))
    }

    /// Whether a state shows any of `layers` on a face.
    fn shows_any(&self, state: u32, face: usize, layers: &[u32]) -> bool {
        self.tile_layers
            .get(&(state, face as u8))
            .is_some_and(|shown| layers.iter().any(|layer| shown.contains(layer)))
    }

    fn block_of(&self, state: u32) -> u32 {
        self.block_of
            .get(state as usize)
            .copied()
            .unwrap_or(u32::MAX)
    }

    fn orientation(&self, rule: &Rule, q: &Query) -> u8 {
        match rule.orient {
            Orient::None => 0,
            Orient::StateAxis => match self.axis_of(q.state) {
                0 => 0,
                a => AXIS_ORIENTATIONS[a as usize - 1][q.face],
            },
            Orient::Texture => texture_orientation(q.quad, q.face),
        }
    }

    fn directions(&self, rule: &Rule, q: &Query) -> [[i32; 3]; 4] {
        directions(q.face, self.orientation(rule, q))
    }

    /// Whether the block at `offset` from the face's block connects to it
    /// (`ConnectionPredicate.shouldConnect` with its inner seams).
    fn connects(&self, rule: &Rule, q: &Query, offset: [i32; 3]) -> bool {
        if !self.same(rule, q, (q.block_at)(offset)) {
            return false;
        }
        if rule.inner_seams {
            // Not when something that connects sits in front of the neighbour.
            let front = add(offset, NORMALS[q.face]);
            return !self.same(rule, q, (q.block_at)(front));
        }
        true
    }

    fn same(&self, rule: &Rule, q: &Query, other: u32) -> bool {
        match rule.connect {
            Connect::State => q.state == other,
            Connect::Block => self.block_of(q.state) == self.block_of(other),
            Connect::Tile => {
                q.state == other
                    || self
                        .tile_layers
                        .get(&(other, q.face as u8))
                        .is_some_and(|layers| layers.contains(&q.layer))
            }
        }
    }
}

/// `RandomIndexProvider`: unweighted or by `weights`.
fn random_index(random: i32, weights: &[u32], size: usize) -> usize {
    let random = (random & 0x7FFF_FFFF) as u32;
    if weights.is_empty() || size == 0 {
        return random as usize % size.max(1);
    }
    // A tile without a weight gets the average of the given ones.
    let given = weights.len().min(size);
    let mut all: Vec<u32> = weights[..given].to_vec();
    let mut sum: u32 = all.iter().sum();
    let average = sum / given as u32;
    while all.len() < size {
        all.push(average);
        sum += average;
    }
    let mut left = random % sum.max(1);
    let mut index = 0;
    while index < size - 1 && left >= all[index] {
        left -= all[index];
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(face: usize) -> ModelQuad {
        // A unit face with the default texture orientation of a cube face.
        let positions = match face {
            NORTH => [
                [1.0, 1.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            SOUTH => [
                [0.0, 1.0, 1.0],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
            ],
            UP => [
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
            ],
            _ => [[0.0; 3]; 4],
        };
        let direction = match face {
            NORTH => horizon_assets::model::Direction::North,
            SOUTH => horizon_assets::model::Direction::South,
            UP => horizon_assets::model::Direction::Up,
            _ => horizon_assets::model::Direction::Down,
        };
        ModelQuad {
            positions,
            uv: [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            facing: direction,
            cull: Some(direction),
            material: horizon_assets::model::Material {
                sprite: "minecraft:block/glass".into(),
                force_translucent: false,
            },
            tint_index: -1,
            shade_direction_override: None,
            light_emission: 0,
        }
    }

    fn rule(method: Method, tiles: usize) -> Rule {
        Rule {
            method,
            overlay: None,
            blocks: true,
            connect_blocks: false,
            connect_layers: None,
            connect: Connect::State,
            inner_seams: false,
            orient: Orient::None,
            tiles: (0..tiles as u32).map(|l| Tile::Layer(100 + l)).collect(),
            faces: None,
            heights: Vec::new(),
            layers: None,
            weights: Vec::new(),
            loops: 0,
            symmetry: Symmetry::None,
            linked: false,
            width: 2,
            height: 2,
        }
    }

    /// A Ctm with `rule` for state 5 and a world given by `at`.
    fn pick(rule: Rule, face: usize, pos: [i32; 3], at: &dyn Fn([i32; 3]) -> u32) -> Option<u32> {
        let mut ctm = Ctm {
            rules: vec![rule],
            by_state: vec![Vec::new(); 8],
            block_of: (0..8).collect(),
            axis: vec![0; 8],
            ..Ctm::default()
        };
        ctm.by_state[5].push(0);
        let q = quad(face);
        ctm.resolve(&Query {
            state: 5,
            layer: 1,
            face,
            pos,
            quad: &q,
            block_at: at,
        })
    }

    #[test]
    fn properties_files_parse_like_java() {
        let p = parse_properties(
            "# comment\nmethod=ctm\n! other\ntiles = 0-46\nmatchBlocks:glass stained_glass\nlong=a \\\n  b\n",
        );
        assert_eq!(p["method"], "ctm");
        assert_eq!(p["tiles"], "0-46");
        assert_eq!(p["matchBlocks"], "glass stained_glass");
        assert_eq!(p["long"], "a b");
        assert_eq!(p.len(), 4);
    }

    #[test]
    fn tiles_resolve_relative_to_the_file() {
        let source = Source {
            namespace: "minecraft".into(),
            path: "optifine/ctm/glass/glass.properties".into(),
            props: HashMap::new(),
        };
        let tiles = parse_tiles(&source, "0-2 ./extra <skip> block/stone minecraft:dirt");
        let sprites: Vec<String> = tiles
            .iter()
            .map(|t| match t {
                TileRef::Special => "<special>".to_owned(),
                TileRef::Texture { sprite, .. } => sprite.clone(),
            })
            .collect();
        assert_eq!(
            sprites,
            vec![
                "minecraft:../optifine/ctm/glass/0",
                "minecraft:../optifine/ctm/glass/1",
                "minecraft:../optifine/ctm/glass/2",
                "minecraft:../optifine/ctm/glass/extra",
                "<special>",
                "minecraft:../optifine/ctm/glass/block/stone",
                "minecraft:block/dirt",
            ]
        );
        if let TileRef::Texture { file, .. } = &tiles[0] {
            assert_eq!(file, "assets/minecraft/optifine/ctm/glass/0.png");
        }
        assert_eq!(
            parse_match_tiles(
                &source,
                "glass minecraft:block/stone textures/block/dirt.png"
            ),
            vec![
                "minecraft:block/glass",
                "minecraft:block/stone",
                "minecraft:block/dirt"
            ]
        );
    }

    #[test]
    fn block_matches_take_properties() {
        let m = parse_match_blocks("glass minecraft:oak_log:axis=y,z create:casing:kind=a");
        assert_eq!(m[0].name, "minecraft:glass");
        assert_eq!(m[1].name, "minecraft:oak_log");
        assert_eq!(
            m[1].properties,
            vec![("axis".to_owned(), vec!["y".to_owned(), "z".to_owned()])]
        );
        assert_eq!(m[2].name, "create:casing");
    }

    #[test]
    fn the_ctm_table_has_the_corner_tiles() {
        // Nothing connected is tile 0, everything is tile 26.
        assert_eq!(CTM_TILES[0], 0);
        assert_eq!(CTM_TILES[255], 26);
        // Every tile of the 47 is used.
        let mut seen = [false; 47];
        for t in CTM_TILES {
            seen[t as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn directions_follow_the_face() {
        // The north face seen from outside: left is east, up is up.
        let d = directions(NORTH, 0);
        assert_eq!(
            d,
            [NORMALS[EAST], NORMALS[DOWN], NORMALS[WEST], NORMALS[UP]]
        );
        // The south face: left is west.
        assert_eq!(directions(SOUTH, 0)[0], NORMALS[WEST]);
        // The top face has north as its up, and west on its left.
        let d = directions(UP, 0);
        assert_eq!(d[3], NORMALS[NORTH]);
        assert_eq!(d[0], NORMALS[WEST]);
        // A quarter turn moves the first direction to the end.
        assert_eq!(directions(NORTH, 1)[3], NORMALS[EAST]);
        // Mirrored: left and right swap.
        assert_eq!(directions(NORTH, 4)[0], NORMALS[WEST]);
    }

    #[test]
    fn a_row_of_blocks_connects_sideways() {
        // State 5 along x on the north face: the middle block has both sides.
        let row = |p: [i32; 3]| {
            if p[1] == 0 && p[2] == 0 && (-1..=1).contains(&p[0]) {
                5
            } else {
                0
            }
        };
        let at = |o: [i32; 3]| row(o);
        // Left and right connected: 1 + 16 = 17, tile 2.
        assert_eq!(
            pick(rule(Method::Ctm, 47), NORTH, [0, 0, 0], &at),
            Some(102)
        );
        // Alone: tile 0.
        let alone = |o: [i32; 3]| if o == [0, 0, 0] { 5 } else { 0 };
        assert_eq!(
            pick(rule(Method::Ctm, 47), NORTH, [0, 0, 0], &alone),
            Some(100)
        );
        // Everything around: tile 26.
        let all = |_: [i32; 3]| 5;
        assert_eq!(
            pick(rule(Method::Ctm, 47), NORTH, [0, 0, 0], &all),
            Some(126)
        );
    }

    #[test]
    fn corners_need_both_sides() {
        // Left (east), below and the corner between them: connections 1+4+2.
        let l_shape = |o: [i32; 3]| match o {
            [1, -1, 0] | [1, 0, 0] | [0, -1, 0] => 5,
            _ => 0,
        };
        let with_corner = pick(rule(Method::Ctm, 47), NORTH, [0, 0, 0], &l_shape);
        let without = |o: [i32; 3]| match o {
            [1, 0, 0] | [0, -1, 0] => 5,
            _ => 0,
        };
        let no_corner = pick(rule(Method::Ctm, 47), NORTH, [0, 0, 0], &without);
        assert_eq!(with_corner, Some(100 + u32::from(CTM_TILES[1 | 4 | 2])));
        assert_eq!(no_corner, Some(100 + u32::from(CTM_TILES[1 | 4])));
        assert_ne!(with_corner, no_corner);
    }

    #[test]
    fn inner_seams_hide_the_face_behind_a_neighbour() {
        let mut r = rule(Method::Horizontal, 4);
        r.inner_seams = true;
        // A neighbour to the left (east of a north face) with another block in
        // front of it does not connect.
        let blocked = |o: [i32; 3]| match o {
            [1, 0, 0] | [1, 0, -1] => 5,
            _ => 0,
        };
        assert_eq!(
            pick(r.clone(), NORTH, [0, 0, 0], &blocked),
            Some(100 + u32::from(LINE_TILES[0]))
        );
        let open = |o: [i32; 3]| if o == [1, 0, 0] { 5 } else { 0 };
        assert_eq!(
            pick(r, NORTH, [0, 0, 0], &open),
            Some(100 + u32::from(LINE_TILES[1]))
        );
    }

    #[test]
    fn faces_heights_and_skips_filter() {
        let all = |_: [i32; 3]| 5;
        let mut r = rule(Method::Fixed, 1);
        r.faces = Some(1 << UP);
        assert_eq!(pick(r.clone(), UP, [0, 0, 0], &all), Some(100));
        assert_eq!(pick(r, NORTH, [0, 0, 0], &all), None);
        let mut r = rule(Method::Fixed, 1);
        r.heights = vec![(10, 20)];
        assert_eq!(pick(r.clone(), UP, [0, 15, 0], &all), Some(100));
        assert_eq!(pick(r, UP, [0, 5, 0], &all), None);
        let mut r = rule(Method::Fixed, 1);
        r.tiles = vec![Tile::Missing];
        assert_eq!(pick(r, UP, [0, 0, 0], &all), None);
        let mut r = rule(Method::Fixed, 1);
        r.tiles = vec![Tile::Skip];
        assert_eq!(pick(r, UP, [0, 0, 0], &all), None);
    }

    #[test]
    fn random_and_repeat_depend_on_the_position() {
        let none = |_: [i32; 3]| 0;
        let r = rule(Method::Random, 4);
        let a = pick(r.clone(), UP, [3, 4, 5], &none);
        assert_eq!(a, pick(r.clone(), UP, [3, 4, 5], &none));
        // Over many positions every tile shows.
        let mut seen = std::collections::HashSet::new();
        for x in 0..64 {
            seen.insert(pick(r.clone(), UP, [x, 0, 0], &none).unwrap());
        }
        assert_eq!(seen.len(), 4);
        // A 2 x 2 repeat alternates along x on the top face.
        let repeat = rule(Method::Repeat, 4);
        let at = |x| pick(repeat.clone(), UP, [x, 0, 0], &none).unwrap();
        assert_eq!(at(0), at(2));
        assert_ne!(at(0), at(1));
        assert_eq!(at(-1), at(1));
    }

    #[test]
    fn weights_bias_the_choice() {
        let mut counts = [0; 2];
        for i in 0..1000 {
            counts[random_index(i * 7919, &[9, 1], 2)] += 1;
        }
        assert!(counts[0] > counts[1] * 4, "{counts:?}");
        // A missing weight is the average.
        assert_eq!(random_index(0, &[2], 3), 0);
    }

    #[test]
    fn horizontal_vertical_falls_back_to_the_ends() {
        // Only the block above connects: the vertical tiles of h+v are 3..=6.
        let above = |o: [i32; 3]| if o == [0, 1, 0] { 5 } else { 0 };
        let tile = pick(
            rule(Method::HorizontalVertical, 7),
            NORTH,
            [0, 0, 0],
            &above,
        )
        .unwrap()
            - 100;
        assert!((3..=6).contains(&tile), "{tile}");
        // Nothing around: the first tile of the secondary map.
        let none = |_: [i32; 3]| 0;
        assert_eq!(
            pick(rule(Method::HorizontalVertical, 7), NORTH, [0, 0, 0], &none),
            Some(100 + u32::from(H_THEN_V[0]))
        );
    }

    #[test]
    fn the_top_method_joins_the_block_above() {
        let above = |o: [i32; 3]| if o == [0, 1, 0] { 5 } else { 0 };
        assert_eq!(
            pick(rule(Method::Top, 1), NORTH, [0, 0, 0], &above),
            Some(100)
        );
        // Not on the top face itself, and not without the block above.
        assert_eq!(pick(rule(Method::Top, 1), UP, [0, 0, 0], &above), None);
        let none = |_: [i32; 3]| 0;
        assert_eq!(pick(rule(Method::Top, 1), NORTH, [0, 0, 0], &none), None);
    }

    /// A Ctm with one overlay rule for state 5 (the base block).
    fn overlay_ctm(method: Method, tiles: usize) -> Ctm {
        let mut overlay = rule(method, tiles);
        overlay.overlay = Some(OverlaySpec {
            translucent: false,
            tint: -1,
        });
        let mut ctm = Ctm {
            rules: vec![overlay],
            by_state: vec![Vec::new(); 16],
            block_of: (0..16).collect(),
            axis: vec![0; 16],
            ..Ctm::default()
        };
        ctm.by_state[5].push(0);
        ctm
    }

    #[test]
    fn a_fixed_overlay_is_drawn_over_the_face_not_instead_of_it() {
        let ctm = overlay_ctm(Method::OverlayFixed, 1);
        let q = quad(NORTH);
        let at = |_: [i32; 3]| 0u32;
        let query = Query {
            state: 5,
            layer: 1,
            face: NORTH,
            pos: [0, 0, 0],
            quad: &q,
            block_at: &at,
        };
        let mut out = Vec::new();
        ctm.overlays(&query, &|_| (false, false), &mut out);
        assert_eq!(
            out,
            vec![OverlayQuad {
                layer: 100,
                translucent: false,
                tint: -1,
            }]
        );
        // The base face keeps its own layer: the overlay rule never replaces it.
        assert_eq!(ctm.resolve(&query), None);
    }

    #[test]
    fn a_standard_overlay_draws_its_side_tile_when_all_four_sides_show_it() {
        let ctm = overlay_ctm(Method::Overlay, 17);
        let q = quad(NORTH);
        // Full blocks beside the face (state 9), air in front of them (state 0).
        let at = |o: [i32; 3]| match o {
            [_, _, -1] => 0,
            [_, _, 0] if o != [0, 0, 0] => 9,
            _ => 5,
        };
        let query = Query {
            state: 5,
            layer: 1,
            face: NORTH,
            pos: [0, 0, 0],
            quad: &q,
            block_at: &at,
        };
        let mut out = Vec::new();
        ctm.overlays(&query, &|s| (s == 10, s == 9 || s == 10), &mut out);
        // Tile 8 is the middle one, with a side on all four sides (applications 0b1111).
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].layer, 108);
    }

    #[test]
    fn a_solid_block_in_front_hides_the_standard_overlay() {
        let ctm = overlay_ctm(Method::Overlay, 17);
        let q = quad(NORTH);
        // The same neighbours, but the block in front of every side is solid.
        let at = |o: [i32; 3]| match o {
            [_, _, -1] => 10,
            [_, _, 0] if o != [0, 0, 0] => 9,
            _ => 5,
        };
        let query = Query {
            state: 5,
            layer: 1,
            face: NORTH,
            pos: [0, 0, 0],
            quad: &q,
            block_at: &at,
        };
        let mut out = Vec::new();
        ctm.overlays(&query, &|s| (s == 10, s == 9 || s == 10), &mut out);
        assert!(out.is_empty());
    }
}
