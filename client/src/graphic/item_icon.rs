 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::{MappedObject, MinecraftClassType};
use crate::state::{mapping, minecraft};
use egui::{Color32, Context};
use jni::objects::{JObject, JValue};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

 
include!(concat!(env!("OUT_DIR"), "/vanilla_textures.rs"));

 
const ITEM_TEXTURES_TSV: &str = include_str!("../../assets/minecraft/item_textures.tsv");

 
 
 
 
 
 
const ITEM_FACES_TSV: &str = include_str!("../../assets/minecraft/item_faces.tsv");

 
 
pub(crate) const ICON: usize = 16;

 
 
 
 
 
 
 
pub(crate) type ItemIcon = (egui::TextureId, Color32);

 
struct CachedIcon {
    handle: egui::TextureHandle,
    average: Color32,
}

impl CachedIcon {
    fn icon(&self) -> ItemIcon {
        (self.handle.id(), self.average)
    }
}

 
 
 
static ITEM_ICONS: OnceLock<Mutex<HashMap<String, Option<CachedIcon>>>> = OnceLock::new();

 
fn item_textures() -> &'static HashMap<&'static str, &'static str> {
    static TABLE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    TABLE.get_or_init(|| {
        ITEM_TEXTURES_TSV
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .collect()
    })
}

 
pub(crate) fn icon(ctx: &Context, item: &str) -> Option<ItemIcon> {
    if item.is_empty() {
        return None;
    }
    let cache = ITEM_ICONS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(hit) = cache.lock().ok()?.get(item) {
        return hit.as_ref().map(CachedIcon::icon);
    }
    let icon = build_icon(item).map(|rgba| {
        let average = average_color(&rgba);
        let handle = ctx.load_texture(
            format!("item_icon_{item}"),
            egui::ColorImage::from_rgba_unmultiplied([ICON, ICON], &rgba),
            egui::TextureOptions::NEAREST,
        );
        CachedIcon { handle, average }
    });
     
     
    let value = icon.as_ref().map(CachedIcon::icon);
    if let Ok(mut cache) = cache.lock() {
        cache.insert(item.to_owned(), icon);
    }
    value
}

 
 
 
 
 
 
 
pub(crate) fn paint(
    painter: &egui::Painter,
    rect: egui::Rect,
    texture: egui::TextureId,
    alpha: f32,
) {
    let tint = Color32::from_white_alpha((alpha.clamp(0.0, 1.0) * 255.0).round() as u8);
    let mut mesh = egui::epaint::Mesh {
        texture_id: texture,
        ..Default::default()
    };
    let step = egui::Vec2::new(rect.width() / ICON as f32, rect.height() / ICON as f32);
    for row in 0..ICON {
        for col in 0..ICON {
            let left = rect.min.x + col as f32 * step.x;
            let right = rect.min.x + (col + 1) as f32 * step.x;
            let top = rect.min.y + row as f32 * step.y;
            let bottom = rect.min.y + (row + 1) as f32 * step.y;
             
            let uv = egui::pos2(
                (col as f32 + 0.5) / ICON as f32,
                (row as f32 + 0.5) / ICON as f32,
            );
            let base = mesh.vertices.len() as u32;
            for corner in [
                egui::pos2(left, top),
                egui::pos2(right, top),
                egui::pos2(right, bottom),
                egui::pos2(left, bottom),
            ] {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: corner,
                    uv,
                    color: tint,
                });
            }
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base, base + 2, base + 3);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

 
 
 
 
 
 
fn build_icon(item: &str) -> Option<Vec<u8>> {
    if let Some(cube) = item_faces().get(item) {
        let images: Option<Vec<Vec<u8>>> = cube
            .faces
            .iter()
            .map(|face| read_texture(face.path).and_then(|png| decode_icon(&png)))
            .collect();
        if let Some(images) = images {
            let tints: [Option<[f32; 3]>; 6] = std::array::from_fn(|index| cube.faces[index].tint);
            return Some(render_cube(&images, &tints));
        }
    }
    candidates(item)
        .into_iter()
        .find_map(|path| read_texture(&path))
        .and_then(|png| decode_icon(&png))
}

 
 
struct CubeFaceTexture {
    path: &'static str,
    tint: Option<[f32; 3]>,
}

 
struct CubeModel {
    faces: [CubeFaceTexture; 6],
}

 
fn item_faces() -> &'static HashMap<&'static str, CubeModel> {
    static TABLE: OnceLock<HashMap<&'static str, CubeModel>> = OnceLock::new();
    TABLE.get_or_init(|| {
        ITEM_FACES_TSV
            .lines()
            .filter_map(|line| {
                let mut columns = line.split('\t');
                let item = columns.next()?;
                let faces: Vec<CubeFaceTexture> = columns
                    .by_ref()
                    .take(6)
                    .map(|cell| {
                        let (path, tint) = cell.split_once('@').unwrap_or((cell, ""));
                        CubeFaceTexture {
                            path,
                            tint: parse_tint(tint),
                        }
                    })
                    .collect();
                let faces: [CubeFaceTexture; 6] = faces.try_into().ok()?;
                Some((item, CubeModel { faces }))
            })
            .collect()
    })
}

 
 
fn parse_tint(column: &str) -> Option<[f32; 3]> {
    let bytes = column.as_bytes();
    if bytes.len() != 6 {
        return None;
    }
    let channel = |at: usize| -> Option<u8> {
        u8::from_str_radix(std::str::from_utf8(&bytes[at..at + 2]).ok()?, 16).ok()
    };
    Some([
        channel(0)? as f32 / 255.0,
        channel(2)? as f32 / 255.0,
        channel(4)? as f32 / 255.0,
    ])
}

 
 
struct CubeFace {
    corners: [[f32; 3]; 4],
    normal: [f32; 3],
    shade: f32,
}

 
const CUBE_FACES: [CubeFace; 6] = [
    CubeFace {
        corners: [
            [-0.5, 0.5, -0.5],
            [0.5, 0.5, -0.5],
            [0.5, 0.5, 0.5],
            [-0.5, 0.5, 0.5],
        ],
        normal: [0.0, 1.0, 0.0],
        shade: 1.0,
    },
    CubeFace {
        corners: [
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, -0.5, -0.5],
            [-0.5, -0.5, -0.5],
        ],
        normal: [0.0, -1.0, 0.0],
        shade: 0.5,
    },
    CubeFace {
        corners: [
            [-0.5, 0.5, -0.5],
            [0.5, 0.5, -0.5],
            [0.5, -0.5, -0.5],
            [-0.5, -0.5, -0.5],
        ],
        normal: [0.0, 0.0, -1.0],
        shade: 0.8,
    },
    CubeFace {
        corners: [
            [0.5, 0.5, 0.5],
            [-0.5, 0.5, 0.5],
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
        ],
        normal: [0.0, 0.0, 1.0],
        shade: 0.8,
    },
    CubeFace {
        corners: [
            [0.5, 0.5, -0.5],
            [0.5, 0.5, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, -0.5, -0.5],
        ],
        normal: [1.0, 0.0, 0.0],
        shade: 0.6,
    },
    CubeFace {
        corners: [
            [-0.5, 0.5, 0.5],
            [-0.5, 0.5, -0.5],
            [-0.5, -0.5, -0.5],
            [-0.5, -0.5, 0.5],
        ],
        normal: [-1.0, 0.0, 0.0],
        shade: 0.6,
    },
];

 
 
 
 
 
 
 
fn gui_rotate([x, y, z]: [f32; 3]) -> [f32; 3] {
    let (sin_yaw, cos_yaw) = (-225f32).to_radians().sin_cos();
    let (sin_pitch, cos_pitch) = 30f32.to_radians().sin_cos();
    let yawed_x = x * cos_yaw + z * sin_yaw;
    let yawed_z = -x * sin_yaw + z * cos_yaw;
    [
        yawed_x,
        y * cos_pitch - yawed_z * sin_pitch,
        y * sin_pitch + yawed_z * cos_pitch,
    ]
}

 
 
 
 
 
fn render_cube(faces: &[Vec<u8>], tints: &[Option<[f32; 3]>; 6]) -> Vec<u8> {
    let mut icon = vec![0u8; ICON * ICON * 4];

     
    let (mut min_x, mut min_y) = (f32::INFINITY, f32::INFINITY);
    let (mut max_x, mut max_y) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for corner in CUBE_CORNERS {
        let p = gui_rotate(corner);
        min_x = min_x.min(p[0]);
        max_x = max_x.max(p[0]);
        min_y = min_y.min(-p[1]);
        max_y = max_y.max(-p[1]);
    }
    let span = (max_x - min_x).max(max_y - min_y).max(1e-3);
    let scale = (ICON as f32 - 1.5) / span;
    let centre = ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
    let to_screen = |p: [f32; 3]| {
        (
            (p[0] - centre.0) * scale + ICON as f32 * 0.5,
            (-p[1] - centre.1) * scale + ICON as f32 * 0.5,
        )
    };

    let mut visible: Vec<(usize, f32)> = CUBE_FACES
        .iter()
        .enumerate()
        .filter_map(|(index, face)| {
            let normal = gui_rotate(face.normal);
            if normal[2] <= 1e-4 {
                return None;
            }
            let depth = face
                .corners
                .iter()
                .map(|corner| gui_rotate(*corner)[2])
                .sum::<f32>()
                / 4.0;
            Some((index, depth))
        })
        .collect();
     
    visible.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    for (index, _) in visible {
        let face = &CUBE_FACES[index];
        let texture = &faces[index];
        let screen: [[f32; 2]; 4] = std::array::from_fn(|corner| {
            let (x, y) = to_screen(gui_rotate(face.corners[corner]));
            [x, y]
        });

         
         
        let (a, b, d) = (screen[0], screen[1], screen[3]);
        let (e1, e2) = ((b[0] - a[0], b[1] - a[1]), (d[0] - a[0], d[1] - a[1]));
        let det = e1.0 * e2.1 - e1.1 * e2.0;
        if det.abs() < 1e-6 {
            continue;
        }
        let min_x = screen.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
        let max_x = screen
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = screen.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
        let max_y = screen
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max);
        let first_x = (min_x.floor() as i32).max(0);
        let last_x = (max_x.ceil() as i32).min(ICON as i32 - 1);
        let first_y = (min_y.floor() as i32).max(0);
        let last_y = (max_y.ceil() as i32).min(ICON as i32 - 1);

        for py in first_y..=last_y {
            for px in first_x..=last_x {
                let rel = (px as f32 + 0.5 - a[0], py as f32 + 0.5 - a[1]);
                let u = (rel.0 * e2.1 - rel.1 * e2.0) / det;
                let v = (e1.0 * rel.1 - e1.1 * rel.0) / det;
                if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                    continue;
                }
                let tx = ((u * ICON as f32) as usize).min(ICON - 1);
                let ty = ((v * ICON as f32) as usize).min(ICON - 1);
                let texel = (ty * ICON + tx) * 4;
                if texture[texel + 3] == 0 {
                    continue;
                }
                let out = (py as usize * ICON + px as usize) * 4;
                let tint = tints[index];
                for channel in 0..3 {
                    let tint = tint.map_or(1.0, |tint| tint[channel]);
                    icon[out + channel] =
                        (texture[texel + channel] as f32 * tint * face.shade).round() as u8;
                }
                icon[out + 3] = texture[texel + 3];
            }
        }
    }
    icon
}

 
const CUBE_CORNERS: [[f32; 3]; 8] = [
    [-0.5, -0.5, -0.5],
    [0.5, -0.5, -0.5],
    [0.5, -0.5, 0.5],
    [-0.5, -0.5, 0.5],
    [-0.5, 0.5, -0.5],
    [0.5, 0.5, -0.5],
    [0.5, 0.5, 0.5],
    [-0.5, 0.5, 0.5],
];

 
 
 
fn candidates(item: &str) -> Vec<String> {
    let mut paths = Vec::with_capacity(3);
    if let Some(mapped) = item_textures().get(item) {
        paths.push((*mapped).to_owned());
    }
    for folder in ["item", "block"] {
        let path = format!("{folder}/{item}");
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    paths
}

 
 
fn read_texture(path: &str) -> Option<Vec<u8>> {
    let game = read_from_game(path);
    let bundled = || read_bundled(path).map(<[u8]>::to_vec);
    match game {
         
         
         
        Some(bytes) if decode_icon(&bytes).is_some() => Some(bytes),
        _ => bundled(),
    }
}

 
 
 
 
fn read_from_game(path: &str) -> Option<Vec<u8>> {
    crate::state::try_client()?;
    let png = mapping()
        .in_frame(|| -> anyhow::Result<Option<Vec<u8>>> {
            let manager = mapping()
                .call_method(
                    MinecraftClassType::Minecraft,
                    minecraft().jni_ref().as_obj(),
                    "getResourceManager",
                    &[],
                )?
                .l()?;
            if manager.is_null() {
                return Ok(None);
            }
            let mut env = mapping().get_env()?;
            let namespace: JObject = env.new_string("minecraft")?.into();
            let file: JObject = env.new_string(format!("textures/{path}.png"))?.into();
            let location = mapping()
                .call_static_method(
                    MinecraftClassType::ResourceLocation,
                    "fromNamespaceAndPath",
                    &[JValue::Object(&namespace), JValue::Object(&file)],
                )?
                .l()?;
            let found = mapping()
                .call_method(
                    MinecraftClassType::ResourceManager,
                    &manager,
                    "getResource",
                    &[JValue::Object(&location)],
                )?
                .l()?;
            if found.is_null() {
                return Ok(None);
            }
             
             
             
            let present = mapping()
                .call_method(MinecraftClassType::Optional, &found, "isPresent", &[])?
                .z()?;
            if !present {
                return Ok(None);
            }
            let resource = mapping()
                .call_method(MinecraftClassType::Optional, &found, "get", &[])?
                .l()?;
            let stream = mapping()
                .call_method(MinecraftClassType::Resource, &resource, "open", &[])?
                .l()?;
            let array = mapping()
                .call_method(
                    MinecraftClassType::InputStream,
                    &stream,
                    "readAllBytes",
                    &[],
                )?
                .l()?;
            let bytes = env.convert_byte_array(jni::objects::JByteArray::from(array))?;
            if env.call_method(&stream, "close", "()V", &[]).is_err() {
                let _ = env.exception_clear();
            }
            Ok(Some(bytes))
        })
        .ok()??;
    Some(png)
}

 
fn read_bundled(path: &str) -> Option<&'static [u8]> {
    VANILLA_TEXTURES
        .binary_search_by_key(&path, |(name, _)| name)
        .ok()
        .map(|index| VANILLA_TEXTURES[index].1)
}

 
 
 
 
 
fn decode_icon(png: &[u8]) -> Option<Vec<u8>> {
    let image = image::load_from_memory(png).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return None;
    }
    let frame = if height > width && height % width == 0 {
        image::imageops::crop_imm(&image, 0, 0, width, width).to_image()
    } else {
        image
    };
    let scaled = image::imageops::resize(&frame, 16, 16, image::imageops::FilterType::Nearest);
    Some(scaled.into_raw())
}

 
 
fn average_color(rgba: &[u8]) -> Color32 {
    let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
    for pixel in rgba.as_chunks::<4>().0 {
        if pixel[3] > 32 {
            r += u64::from(pixel[0]);
            g += u64::from(pixel[1]);
            b += u64::from(pixel[2]);
            n += 1;
        }
    }
    if n == 0 {
        return Color32::from_gray(150);
    }
    Color32::from_rgb((r / n) as u8, (g / n) as u8, (b / n) as u8)
}

 
 
#[cfg(test)]
pub(crate) fn bundled_pixels(item: &str) -> Option<Vec<u8>> {
    candidates(item)
        .into_iter()
        .find_map(|path| read_bundled(&path))
        .and_then(decode_icon)
}

 
 
#[cfg(test)]
pub(crate) fn clear_cache() {
    if let Some(cache) = ITEM_ICONS.get() {
        if let Ok(mut cache) = cache.lock() {
            cache.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

     
     
     
    #[test]
    fn every_mapped_item_texture_is_bundled() {
        let table = item_textures();
        assert!(
            table.len() > 1_200,
            "only {} items mapped; the asset table looks truncated",
            table.len()
        );
        let missing: Vec<&str> = table
            .values()
            .copied()
            .filter(|path| read_bundled(path).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "{} mapped textures are not bundled, e.g. {:?}",
            missing.len(),
            &missing[..missing.len().min(8)]
        );
    }

     
     
     
     
    #[test]
    fn a_cached_icon_owns_the_texture_it_hands_out() {
        clear_cache();
        let ctx = Context::default();
        let mut loaded = None;
        for frame in 0..3 {
            let output = ctx.run(egui::RawInput::default(), |ctx| {
                loaded = icon(ctx, "diamond_helmet");
            });
            let (id, _) = loaded.expect("the bundled assets carry a helmet icon");
            if frame == 0 {
                assert!(
                    output.textures_delta.set.iter().any(|(set, _)| *set == id),
                    "the icon is uploaded on first use"
                );
            }
            assert!(
                !output.textures_delta.free.contains(&id),
                "the icon's texture was freed while it was still cached"
            );
        }
        clear_cache();
    }

     
     
    #[test]
    fn items_resolve_to_their_own_texture() {
        let table = item_textures();
        for (item, path) in [
            ("diamond_sword", "item/diamond_sword"),
            ("stone", "block/stone"),
            ("crafting_table", "block/crafting_table_front"),
            ("grass_block", "block/grass_block_top"),
            ("potion", "item/potion"),
            ("black_carpet", "block/black_wool"),
        ] {
            assert_eq!(table.get(item).copied(), Some(path), "{item}");
            assert!(read_bundled(path).is_some(), "{path} must be bundled");
        }
    }

    #[test]
    fn candidates_prefer_the_mapped_texture() {
        assert_eq!(
            candidates("crafting_table"),
            vec![
                "block/crafting_table_front",
                "item/crafting_table",
                "block/crafting_table"
            ]
        );
        assert_eq!(
            candidates("diamond_sword"),
            vec!["item/diamond_sword", "block/diamond_sword"]
        );
        assert_eq!(
            candidates("not_a_real_item"),
            vec!["item/not_a_real_item", "block/not_a_real_item"]
        );
    }

    #[test]
    fn every_bundled_texture_decodes_to_a_sixteen_pixel_icon() {
        for (name, png) in VANILLA_TEXTURES {
            let icon = decode_icon(png).unwrap_or_else(|| panic!("{name} did not decode"));
            assert_eq!(icon.len(), 16 * 16 * 4, "{name} is not a 16×16 icon");
        }
    }

     
    const CUBES: [&str; 8] = [
        "stone",
        "dirt",
        "oak_log",
        "tnt",
        "oak_leaves",
        "grass_block",
        "netherrack",
        "white_wool",
    ];

     
     
     
     
    #[test]
    fn full_cubes_are_drawn_as_cubes() {
        assert!(
            item_faces().len() > 300,
            "only {} cube models; the faces table looks truncated",
            item_faces().len()
        );
        for item in CUBES {
            let cube = build_icon(item).unwrap_or_else(|| panic!("{item} has no icon"));
            let flat = bundled_pixels(item).unwrap_or_else(|| panic!("{item} has no sprite"));
            assert_ne!(cube, flat, "{item} was drawn as its flat sprite");
            assert_eq!(
                cube[3], 0,
                "{item}'s top-left pixel is opaque, so its icon is not a rotated cube"
            );
            let opaque = cube.as_chunks::<4>().0.iter().filter(|p| p[3] > 0).count();
            assert!(
                opaque > 40,
                "{item}'s cube is only {opaque} pixels; it should fill most of the icon"
            );
        }
    }

     
     
    #[test]
    fn transparent_cubes_stay_transparent() {
        let glass = build_icon("glass").expect("glass has an icon");
        let opaque = glass.as_chunks::<4>().0.iter().filter(|p| p[3] > 0).count();
        assert!(
            (20..160).contains(&opaque),
            "glass is {opaque} opaque pixels; that is not a wireframe cube"
        );
        let centre = (8 * ICON + 8) * 4;
        assert_eq!(glass[centre + 3], 0, "the middle of glass is not a pane");
    }

     
    #[test]
    fn everything_else_keeps_its_sprite() {
        for item in [
            "diamond_sword",
            "observer",
            "diamond",
            "bow",
            "red_glazed_terracotta",
        ] {
            assert_eq!(
                build_icon(item),
                bundled_pixels(item),
                "{item} should be its own sprite"
            );
        }
    }

     
     
    #[test]
    fn tinted_cubes_apply_the_items_constant_tint() {
        let cube = item_faces().get("oak_leaves").expect("leaves are a cube");
        assert!(
            cube.faces.iter().all(|face| face.tint.is_some()),
            "oak_leaves lost its constant tint"
        );
        let icon = build_icon("oak_leaves").expect("leaves have an icon");
        let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
        for pixel in icon.as_chunks::<4>().0 {
            if pixel[3] > 32 {
                r += u64::from(pixel[0]);
                g += u64::from(pixel[1]);
                b += u64::from(pixel[2]);
                n += 1;
            }
        }
        assert!(n > 0, "the leaves icon is empty");
        let (r, g, b) = (r / n, g / n, b / n);
        assert!(
            g > r && g > b,
            "oak leaves came out rgb({r},{g},{b}), which is not tinted green"
        );
    }

     
     
    #[test]
    fn the_icon_sheet_shows_cubes_beside_sprites() {
        const ITEMS: [&str; 20] = [
            "stone",
            "dirt",
            "oak_log",
            "tnt",
            "oak_leaves",
            "glass",
            "white_wool",
            "netherrack",
            "crafting_table",
            "diamond_sword",
            "furnace",
            "sandstone",
            "pumpkin",
            "bookshelf",
            "deepslate",
            "grass_block",
            "cherry_leaves",
            "cobblestone",
            "glowstone",
            "diamond_pickaxe",
        ];
        const ZOOM: u32 = 10;
        const COLUMNS: u32 = 5;
        let rows = ITEMS.len().div_ceil(COLUMNS as usize) as u32;
        let tile = ICON as u32 * ZOOM;
         
        let mut sheet =
            image::RgbaImage::from_pixel(COLUMNS * tile, rows * tile, image::Rgba([0, 0, 0, 0]));
        for (index, item) in ITEMS.iter().enumerate() {
            let icon = build_icon(item).unwrap_or_else(|| panic!("{item} has no icon"));
            let icon = image::RgbaImage::from_raw(ICON as u32, ICON as u32, icon).unwrap();
            let zoomed =
                image::imageops::resize(&icon, tile, tile, image::imageops::FilterType::Nearest);
            let (x, y) = (index as u32 % COLUMNS, index as u32 / COLUMNS);
            image::imageops::overlay(&mut sheet, &zoomed, (x * tile) as i64, (y * tile) as i64);
        }
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/item-icons.png");
        sheet.save(&path).expect("the icon sheet saves");
        println!("item icons: {path:?}");
    }

     
     
    #[test]
    fn animated_textures_use_their_first_frame() {
        let mut strip = image::RgbaImage::new(16, 48);
        for (_, _, pixel) in strip.enumerate_pixels_mut() {
            *pixel = image::Rgba([0, 0, 0, 255]);
        }
        for y in 0..16 {
            for x in 0..16 {
                strip.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(strip)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let icon = decode_icon(png.get_ref()).expect("the strip decodes");
         
        assert_eq!(&icon[..4], &[255, 0, 0, 255]);
    }
}
