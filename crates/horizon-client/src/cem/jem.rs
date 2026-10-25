//! `.jem` and `.jpm` files: a custom entity model's parts, boxes and animations
//! (OptiFine's CEM format, read as Entity Model Features reads it).

use crate::mobs::Polygon;
use serde::{Deserialize, Deserializer, de::MapAccess, de::Visitor};
use std::fmt;

/// An object whose keys keep the order they were written in (an animation's
/// lines run in file order).
#[derive(Debug, Default)]
pub struct Ordered(pub Vec<(String, String)>);

impl<'de> Deserialize<'de> for Ordered {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Pairs;
        impl<'de> Visitor<'de> for Pairs {
            type Value = Ordered;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object of animation lines")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Ordered, A::Error> {
                let mut pairs = Vec::new();
                while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
                    let text = match value {
                        serde_json::Value::String(s) => s,
                        other => other.to_string(),
                    };
                    pairs.push((key, text));
                }
                Ok(Ordered(pairs))
            }
        }
        deserializer.deserialize_map(Pairs)
    }
}

/// `true`, or the string `"true"` some packs write.
fn lenient_bool<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Bool(b) => b,
        serde_json::Value::String(s) => s.trim().eq_ignore_ascii_case("true"),
        _ => false,
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawBox {
    coordinates: Vec<f32>,
    #[serde(rename = "textureOffset")]
    texture_offset: Vec<f32>,
    #[serde(rename = "uvDown")]
    uv_down: Vec<f32>,
    #[serde(rename = "uvUp")]
    uv_up: Vec<f32>,
    #[serde(rename = "uvNorth")]
    uv_north: Vec<f32>,
    #[serde(rename = "uvSouth")]
    uv_south: Vec<f32>,
    #[serde(rename = "uvWest")]
    uv_west: Vec<f32>,
    #[serde(rename = "uvEast")]
    uv_east: Vec<f32>,
    #[serde(rename = "uvFront")]
    uv_front: Vec<f32>,
    #[serde(rename = "uvBack")]
    uv_back: Vec<f32>,
    #[serde(rename = "uvLeft")]
    uv_left: Vec<f32>,
    #[serde(rename = "uvRight")]
    uv_right: Vec<f32>,
    #[serde(rename = "sizeAdd")]
    size_add: f32,
    #[serde(rename = "sizeAddX")]
    size_add_x: f32,
    #[serde(rename = "sizeAddY")]
    size_add_y: f32,
    #[serde(rename = "sizeAddZ")]
    size_add_z: f32,
    #[serde(rename = "sizesAdd")]
    sizes_add: Vec<f32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RawPart {
    texture: String,
    #[serde(rename = "textureSize")]
    texture_size: Option<Vec<f32>>,
    #[serde(rename = "invertAxis")]
    invert_axis: String,
    translate: Option<Vec<f32>>,
    rotate: Option<Vec<f32>>,
    #[serde(rename = "mirrorTexture")]
    mirror_texture: String,
    boxes: Vec<RawBox>,
    submodel: Option<Box<RawPart>>,
    submodels: Vec<RawPart>,
    #[serde(rename = "baseId")]
    base_id: String,
    model: String,
    id: String,
    part: Option<String>,
    #[serde(deserialize_with = "lenient_bool")]
    attach: bool,
    scale: Option<f32>,
    animations: Option<Vec<Ordered>>,
}

/// A `.jem` file.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RawJem {
    texture: String,
    #[serde(rename = "textureSize")]
    texture_size: Option<Vec<f32>>,
    models: Vec<RawPart>,
}

pub fn parse_jem(text: &str) -> Result<RawJem, String> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

pub fn parse_jpm(text: &str) -> Result<RawPart, String> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

/// A part of the model once its axes are flipped and its boxes built.
#[derive(Debug)]
pub struct CemPart {
    pub id: String,
    /// The vanilla part (its name in 26.3's model) this part hangs from.
    pub part: Option<String>,
    pub attach: bool,
    /// x, y, z, rotations in radians, scales.
    pub pose: [f32; 9],
    pub polygons: Vec<Polygon>,
    pub children: Vec<CemPart>,
    /// Lines to run, in order (top-level parts only).
    pub animations: Vec<Vec<(String, String)>>,
}

/// What loading a `.jem` needs from its surroundings.
pub trait Files {
    /// The `.jpm` a part's `model` names, as text.
    fn jpm(&self, path: &str) -> Option<String>;
}

pub struct Jem {
    /// The model's own texture (`texture` of the file), when it names one.
    pub texture: Option<String>,
    pub parts: Vec<CemPart>,
}

/// Builds a model: `names` maps OptiFine part names to the model's own.
pub fn build(
    raw: RawJem,
    names: &std::collections::HashMap<String, String>,
    files: &dyn Files,
) -> Result<Jem, String> {
    let texture_size = size_of(&raw.texture_size).unwrap_or([64.0, 32.0]);
    let mut parts = Vec::new();
    let mut seen = 0usize;
    for mut model in raw.models {
        let original = model.part.clone();
        if let Some(part) = &model.part {
            model.part = Some(names.get(part).cloned().unwrap_or_else(|| part.clone()));
        }
        let mut cem = prepare(model, texture_size, files, &mut seen)?;
        // A model with no part to hang from has no use.
        let Some(original) = original else { continue };
        for lines in &mut cem.animations {
            for (key, value) in lines {
                *key = replace_word(key, "part", &original);
                *value = replace_word(value, "part", &original);
            }
        }
        parts.push(cem);
    }
    Ok(Jem {
        texture: (!raw.texture.trim().is_empty()).then(|| raw.texture.trim().to_owned()),
        parts,
    })
}

fn size_of(v: &Option<Vec<f32>>) -> Option<[f32; 2]> {
    match v.as_deref() {
        Some([w, h]) if *w > 0.0 && *h > 0.0 => Some([*w, *h]),
        _ => None,
    }
}

/// `EMFPartData.copyFrom`: a `.jpm` fills what the `.jem` part leaves out.
fn copy_from(part: &mut RawPart, jpm: RawPart) {
    if part.submodels.is_empty() {
        part.submodels = jpm.submodels;
    }
    if part.submodel.is_none() {
        part.submodel = jpm.submodel;
    }
    if part.texture_size.is_none() {
        part.texture_size = jpm.texture_size;
    }
    if part.texture.is_empty() {
        part.texture = jpm.texture;
    }
    if part.invert_axis.is_empty() {
        part.invert_axis = jpm.invert_axis;
    }
    if part.translate.is_none() {
        part.translate = jpm.translate;
    }
    if part.rotate.is_none() {
        part.rotate = jpm.rotate;
    }
    if part.mirror_texture.is_empty() {
        part.mirror_texture = jpm.mirror_texture;
    }
    if part.boxes.is_empty() {
        part.boxes = jpm.boxes;
    }
    if part.scale.is_none() {
        part.scale = jpm.scale;
    }
    if part.animations.as_ref().is_none_or(Vec::is_empty) {
        part.animations = jpm.animations;
    }
    if part.base_id.is_empty() {
        part.base_id = jpm.base_id;
    }
}

fn prepare(
    mut part: RawPart,
    parent_size: [f32; 2],
    files: &dyn Files,
    seen: &mut usize,
) -> Result<CemPart, String> {
    if !part.model.trim().is_empty()
        && let Some(text) = files.jpm(part.model.trim())
    {
        copy_from(&mut part, parse_jpm(&text)?);
    }
    *seen += 1;
    let id = if part.id.trim().is_empty() {
        format!("part{seen}")
    } else {
        part.id.trim().to_owned()
    };
    let size = size_of(&part.texture_size).unwrap_or(parent_size);
    let invert = |axis: char| part.invert_axis.contains(axis);
    let (ix, iy, iz) = (invert('x'), invert('y'), invert('z'));
    let flip = |v: &Option<Vec<f32>>| -> [f32; 3] {
        match v.as_deref() {
            Some([a, b, c]) => [
                if ix { -a } else { *a },
                if iy { -b } else { *b },
                if iz { -c } else { *c },
            ],
            _ => [0.0; 3],
        }
    };
    let translate = flip(&part.translate);
    let rotate = flip(&part.rotate).map(f32::to_radians);
    let scale = part.scale.unwrap_or(1.0);
    let mirror_u = part.mirror_texture.contains('u');
    let mirror_v = part.mirror_texture.contains('v');

    let mut polygons = Vec::new();
    for raw in &part.boxes {
        polygons.extend(cube(raw, (ix, iy, iz), size, mirror_u, mirror_v)?);
    }

    let mut children = Vec::new();
    let nested = part
        .submodel
        .take()
        .map(|b| *b)
        .into_iter()
        .chain(part.submodels);
    for sub in nested {
        children.push(prepare(sub, size, files, seen)?);
    }

    let mut animations = Vec::new();
    for lines in part.animations.into_iter().flatten() {
        let cleaned: Vec<(String, String)> = lines
            .0
            .into_iter()
            .map(|(k, v)| (strip_space(&k), strip_space(&v)))
            .filter(|(k, v)| !k.is_empty() && !v.is_empty())
            .map(|(k, v)| (replace_word(&k, "this", &id), replace_word(&v, "this", &id)))
            .collect();
        if !cleaned.is_empty() {
            animations.push(cleaned);
        }
    }

    Ok(CemPart {
        id,
        part: part.part,
        attach: part.attach,
        pose: [
            translate[0],
            translate[1],
            translate[2],
            rotate[0],
            rotate[1],
            rotate[2],
            scale,
            scale,
            scale,
        ],
        polygons,
        children,
        animations,
    })
}

fn strip_space(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Replaces whole words only (`this` but not `thistle`).
fn replace_word(text: &str, word: &str, with: &str) -> String {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(word) {
        let before = rest[..at].chars().next_back();
        let after = rest[at + word.len()..].chars().next();
        out.push_str(&rest[..at]);
        if before.is_none_or(|c| !is_word(c)) && after.is_none_or(|c| !is_word(c)) {
            out.push_str(with);
        } else {
            out.push_str(word);
        }
        rest = &rest[at + word.len()..];
    }
    out.push_str(rest);
    out
}

const DOWN: [f32; 3] = [0.0, -1.0, 0.0];
const UP: [f32; 3] = [0.0, 1.0, 0.0];
const NORTH: [f32; 3] = [0.0, 0.0, -1.0];
const SOUTH: [f32; 3] = [0.0, 0.0, 1.0];
const WEST: [f32; 3] = [-1.0, 0.0, 0.0];
const EAST: [f32; 3] = [1.0, 0.0, 0.0];

type V3 = [f32; 3];

/// `Polygon(vertices, u1, v1, u2, v2, w, h)`: the corners' texture coordinates.
fn quad(v: [V3; 4], uv: [f32; 4], size: [f32; 2], normal: V3) -> Polygon {
    let [u1, v1, u2, v2] = uv;
    let (w, h) = (size[0], size[1]);
    Polygon {
        normal,
        vertices: [
            [v[0][0], v[0][1], v[0][2], u2 / w, v1 / h],
            [v[1][0], v[1][1], v[1][2], u1 / w, v1 / h],
            [v[2][0], v[2][1], v[2][2], u1 / w, v2 / h],
            [v[3][0], v[3][1], v[3][2], u2 / w, v2 / h],
        ],
    }
}

fn four(v: &[f32]) -> Option<[f32; 4]> {
    (v.len() == 4).then(|| [v[0], v[1], v[2], v[3]])
}

/// One box: coordinates `[x, y, z, w, h, d]` flipped on the inverted axes, then
/// either the usual texture layout from `textureOffset`, or a rectangle per face.
fn cube(
    raw: &RawBox,
    invert: (bool, bool, bool),
    size: [f32; 2],
    mirror_u: bool,
    mirror_v: bool,
) -> Result<Vec<Polygon>, String> {
    let &[mut x, mut y, mut z, w, h, d, ..] = raw.coordinates.as_slice() else {
        return Err("a box needs 6 coordinates".into());
    };
    if invert.0 {
        x = -x - w;
    }
    if invert.1 {
        y = -y - h;
    }
    if invert.2 {
        z = -z - d;
    }
    let (mut ex, mut ey, mut ez) = (raw.size_add_x, raw.size_add_y, raw.size_add_z);
    if raw.size_add != 0.0 && ex == 0.0 && ey == 0.0 && ez == 0.0 {
        (ex, ey, ez) = (raw.size_add, raw.size_add, raw.size_add);
    }
    if let [a, b, c] = raw.sizes_add[..] {
        (ex, ey, ez) = (a, b, c);
    }
    let (x1, y1, z1) = (x - ex, y - ey, z - ez);
    let (x2, y2, z2) = (x + w + ex, y + h + ey, z + d + ez);
    let v1 = [x1, y1, z1];
    let v2 = [x2, y1, z1];
    let v3 = [x2, y2, z1];
    let v4 = [x1, y2, z1];
    let v5 = [x1, y1, z2];
    let v6 = [x2, y1, z2];
    let v7 = [x2, y2, z2];
    let v8 = [x1, y2, z2];

    if let [u, v] = raw.texture_offset[..] {
        let (j, k, l) = (u, u + d, u + d + w);
        let m = u + d + w + w;
        let n = u + d + w + d;
        let o = u + d + w + d + w;
        let (p, q, r) = (v, v + d, v + d + h);
        let pick = |flag: bool, a: f32, b: f32| if flag { a } else { b };
        return Ok(vec![
            quad(
                if mirror_v {
                    [v3, v4, v8, v7]
                } else {
                    [v6, v5, v1, v2]
                },
                [
                    pick(mirror_u, l, k),
                    pick(mirror_v, q, p),
                    pick(mirror_u, k, l),
                    pick(mirror_v, p, q),
                ],
                size,
                if mirror_v { UP } else { DOWN },
            ),
            quad(
                if mirror_v {
                    [v6, v5, v1, v2]
                } else {
                    [v3, v4, v8, v7]
                },
                [
                    pick(mirror_u, m, l),
                    pick(mirror_v, p, q),
                    pick(mirror_u, l, m),
                    pick(mirror_v, q, p),
                ],
                size,
                if mirror_v { DOWN } else { UP },
            ),
            quad(
                if mirror_u {
                    [v6, v2, v3, v7]
                } else {
                    [v1, v5, v8, v4]
                },
                [
                    pick(mirror_u, k, j),
                    pick(mirror_v, r, q),
                    pick(mirror_u, j, k),
                    pick(mirror_v, q, r),
                ],
                size,
                if mirror_u { EAST } else { WEST },
            ),
            quad(
                [v2, v1, v4, v3],
                [
                    pick(mirror_u, l, k),
                    pick(mirror_v, r, q),
                    pick(mirror_u, k, l),
                    pick(mirror_v, q, r),
                ],
                size,
                NORTH,
            ),
            quad(
                if mirror_u {
                    [v1, v5, v8, v4]
                } else {
                    [v6, v2, v3, v7]
                },
                [
                    pick(mirror_u, n, l),
                    pick(mirror_v, r, q),
                    pick(mirror_u, l, n),
                    pick(mirror_v, q, r),
                ],
                size,
                if mirror_u { WEST } else { EAST },
            ),
            quad(
                [v5, v6, v7, v8],
                [
                    pick(mirror_u, o, n),
                    pick(mirror_v, r, q),
                    pick(mirror_u, n, o),
                    pick(mirror_v, q, r),
                ],
                size,
                SOUTH,
            ),
        ]);
    }

    // A rectangle per face; the legacy names mean north (front, left) and west (back).
    let north = four(&raw.uv_north)
        .or_else(|| four(&raw.uv_left))
        .or_else(|| four(&raw.uv_front));
    let west = four(&raw.uv_west).or_else(|| four(&raw.uv_back));
    let east = four(&raw.uv_east).or_else(|| four(&raw.uv_right));
    // OptiFine ignores the mirror flags on rectangles.
    let mut out = Vec::new();
    if let Some(uv) = four(&raw.uv_up) {
        out.push(quad([v1, v2, v6, v5], uv, size, DOWN));
    }
    if let Some(uv) = four(&raw.uv_down) {
        out.push(quad([v8, v7, v3, v4], uv, size, UP));
    }
    if let Some(uv) = west {
        out.push(quad([v6, v2, v3, v7], uv, size, EAST));
    }
    if let Some(uv) = north {
        out.push(quad([v2, v1, v4, v3], uv, size, NORTH));
    }
    if let Some(uv) = east {
        out.push(quad([v1, v5, v8, v4], uv, size, WEST));
    }
    if let Some(uv) = four(&raw.uv_south) {
        out.push(quad([v5, v6, v7, v8], uv, size, SOUTH));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct NoFiles;
    impl Files for NoFiles {
        fn jpm(&self, _: &str) -> Option<String> {
            None
        }
    }

    fn build_text(text: &str) -> Jem {
        build(parse_jem(text).unwrap(), &HashMap::new(), &NoFiles).unwrap()
    }

    #[test]
    fn axes_flip_for_translate_rotate_and_boxes() {
        let jem = build_text(
            r#"{"textureSize":[64,64],"models":[{"part":"head","id":"h","invertAxis":"xy",
               "translate":[1,-24,3],"rotate":[90,10,20],
               "boxes":[{"coordinates":[-4,0,-4,8,8,8],"textureOffset":[0,0]}]}]}"#,
        );
        let part = &jem.parts[0];
        assert_eq!(part.part.as_deref(), Some("head"));
        assert_eq!(&part.pose[..3], &[-1.0, 24.0, 3.0]);
        assert!((part.pose[3] + 90f32.to_radians()).abs() < 1e-6);
        assert!((part.pose[4] + 10f32.to_radians()).abs() < 1e-6);
        assert!((part.pose[5] - 20f32.to_radians()).abs() < 1e-6);
        assert_eq!(part.polygons.len(), 6);
        // x: -(-4) - 8 = -4 .. 4; y: -0 - 8 = -8 .. 0.
        let xs: Vec<f32> = part
            .polygons
            .iter()
            .flat_map(|p| p.vertices.iter().map(|v| v[0]))
            .collect();
        let ys: Vec<f32> = part
            .polygons
            .iter()
            .flat_map(|p| p.vertices.iter().map(|v| v[1]))
            .collect();
        assert_eq!(
            (
                xs.iter().cloned().fold(f32::MAX, f32::min),
                xs.iter().cloned().fold(f32::MIN, f32::max)
            ),
            (-4.0, 4.0)
        );
        assert_eq!(
            (
                ys.iter().cloned().fold(f32::MAX, f32::min),
                ys.iter().cloned().fold(f32::MIN, f32::max)
            ),
            (-8.0, 0.0)
        );
    }

    #[test]
    fn box_texture_layout_matches_the_usual_cuboid() {
        let jem = build_text(
            r#"{"textureSize":[64,64],"models":[{"part":"body","id":"b",
               "boxes":[{"coordinates":[0,0,0,8,12,4],"textureOffset":[16,16]}]}]}"#,
        );
        let polys = &jem.parts[0].polygons;
        // The first face (the minimum-y one) takes the strip right of the sides:
        // u from 16 + depth to 16 + depth + width, v from 16 to 16 + depth.
        let top = &polys[0];
        let us: Vec<f32> = top.vertices.iter().map(|v| v[3] * 64.0).collect();
        let vs: Vec<f32> = top.vertices.iter().map(|v| v[4] * 64.0).collect();
        assert_eq!(us, [28.0, 20.0, 20.0, 28.0]);
        assert_eq!(vs, [16.0, 16.0, 20.0, 20.0]);
        assert_eq!(top.normal, [0.0, -1.0, 0.0]);
        // The north face spans width x height below the top strip.
        let north = &polys[3];
        assert_eq!(north.normal, [0.0, 0.0, -1.0]);
        let nu: Vec<f32> = north.vertices.iter().map(|v| v[3] * 64.0).collect();
        let nv: Vec<f32> = north.vertices.iter().map(|v| v[4] * 64.0).collect();
        assert_eq!(nu, [28.0, 20.0, 20.0, 28.0]);
        assert_eq!(nv, [20.0, 20.0, 32.0, 32.0]);
    }

    #[test]
    fn size_add_grows_the_box() {
        let jem = build_text(
            r#"{"textureSize":[16,16],"models":[{"part":"head","id":"h",
               "boxes":[{"coordinates":[0,0,0,2,2,2],"textureOffset":[0,0],"sizeAdd":0.5}]}]}"#,
        );
        let xs: Vec<f32> = jem.parts[0]
            .polygons
            .iter()
            .flat_map(|p| p.vertices.iter().map(|v| v[0]))
            .collect();
        assert_eq!(xs.iter().cloned().fold(f32::MAX, f32::min), -0.5);
        assert_eq!(xs.iter().cloned().fold(f32::MIN, f32::max), 2.5);
    }

    #[test]
    fn rectangle_faces_take_their_own_uv() {
        let jem = build_text(
            r#"{"textureSize":[32,32],"models":[{"part":"head","id":"h",
               "boxes":[{"coordinates":[0,0,0,2,1,0],"uvNorth":[8,16,10,17]}]}]}"#,
        );
        let polys = &jem.parts[0].polygons;
        assert_eq!(polys.len(), 1);
        assert_eq!(polys[0].normal, NORTH);
        assert_eq!(polys[0].vertices[0][3], 10.0 / 32.0);
        assert_eq!(polys[0].vertices[1][3], 8.0 / 32.0);
        assert_eq!(polys[0].vertices[0][4], 16.0 / 32.0);
        assert_eq!(polys[0].vertices[2][4], 17.0 / 32.0);
    }

    #[test]
    fn animations_keep_order_and_this_names_the_part() {
        let jem = build_text(
            r#"{"textureSize":[16,16],"models":[{"part":"head","id":"hd",
               "animations":[{"var.b":"1","this.rx":"var.b * 2","head.ry":"this.rx"}]}]}"#,
        );
        let lines = &jem.parts[0].animations[0];
        assert_eq!(lines[0], ("var.b".to_owned(), "1".to_owned()));
        assert_eq!(lines[1], ("hd.rx".to_owned(), "var.b*2".to_owned()));
        assert_eq!(lines[2], ("head.ry".to_owned(), "hd.rx".to_owned()));
    }

    #[test]
    fn part_names_map_to_the_models_own() {
        let mut names = HashMap::new();
        names.insert("leg1".to_owned(), "right_hind_leg".to_owned());
        let jem = build(
            parse_jem(r#"{"textureSize":[16,16],"models":[{"part":"leg1","id":"l"},{"part":"head","id":"h"}]}"#).unwrap(),
            &names,
            &NoFiles,
        )
        .unwrap();
        assert_eq!(jem.parts[0].part.as_deref(), Some("right_hind_leg"));
        assert_eq!(jem.parts[1].part.as_deref(), Some("head"));
    }

    #[test]
    fn a_jpm_fills_the_part_in() {
        struct Files;
        impl super::Files for Files {
            fn jpm(&self, path: &str) -> Option<String> {
                (path == "x.jpm").then(|| {
                    r#"{"animations":[{"var.a":"3"}],"translate":[0,5,0],"invertAxis":"y"}"#
                        .to_owned()
                })
            }
        }
        let jem = build(
            parse_jem(
                r#"{"textureSize":[16,16],"models":[{"part":"root","id":"root","model":"x.jpm"}]}"#,
            )
            .unwrap(),
            &HashMap::new(),
            &Files,
        )
        .unwrap();
        assert_eq!(
            jem.parts[0].animations[0][0],
            ("var.a".to_owned(), "3".to_owned())
        );
        assert_eq!(jem.parts[0].pose[1], -5.0);
    }
}
