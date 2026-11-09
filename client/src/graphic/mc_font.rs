 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use egui::{Color32, Context, Painter, Pos2, TextureId, TextureOptions};
use std::sync::OnceLock;

 
const SHEET: &[u8] = include_bytes!("../../assets/minecraft/textures/font/ascii.png");

 
const SHEET_PIXELS: usize = 128;
 
const CELL: usize = 8;
 
const COLUMNS: usize = 16;
 
const FIRST_ASCII_ROW: usize = 2;
 
const GLYPHS: usize = 95;
 
pub(crate) const LINE_HEIGHT: f32 = 9.0;
 
const BLANK_ADVANCE: usize = 4;
 
 
const UNKNOWN_ADVANCE: usize = 6;

 
#[derive(Default)]
struct Glyph {
     
    right: Option<usize>,
     
    ink: Vec<(usize, usize)>,
}

 
struct Font {
     
    rgba: Vec<u8>,
    glyphs: [Glyph; GLYPHS],
}

 
 
fn font() -> Option<&'static Font> {
    static FONT: OnceLock<Option<Font>> = OnceLock::new();
    FONT.get_or_init(load).as_ref()
}

fn load() -> Option<Font> {
    let image = image::load_from_memory(SHEET).ok()?.to_rgba8();
    if image.width() as usize != SHEET_PIXELS || image.height() as usize != SHEET_PIXELS {
        return None;
    }
     
     
    let mut rgba = image.into_raw();
    for pixel in rgba.as_chunks_mut::<4>().0 {
        if pixel[3] > 0 {
            pixel[0] = 255;
            pixel[1] = 255;
            pixel[2] = 255;
        }
    }

    let mut glyphs: [Glyph; GLYPHS] = std::array::from_fn(|_| Glyph::default());
    for (index, glyph) in glyphs.iter_mut().enumerate() {
        let (x0, y0) = cell_origin(index);
        let mut right = None;
        for column in 0..CELL {
            for row in 0..CELL {
                let at = ((y0 + row) * SHEET_PIXELS + (x0 + column)) * 4;
                if rgba[at + 3] == 0 {
                    continue;
                }
                glyph.ink.push((column, row));
                right = Some(right.map_or(column, |right: usize| right.max(column)));
            }
        }
        glyph.right = right;
    }
    Some(Font { rgba, glyphs })
}

 
fn cell_origin(index: usize) -> (usize, usize) {
    (
        (index % COLUMNS) * CELL,
        (FIRST_ASCII_ROW + index / COLUMNS) * CELL,
    )
}

 
fn index_of(ch: char) -> Option<usize> {
    let code = ch as usize;
    (0x20..0x20 + GLYPHS).contains(&code).then_some(code - 0x20)
}

 
fn advance(ch: char) -> usize {
    let Some(font) = font() else {
        return UNKNOWN_ADVANCE;
    };
    let Some(index) = index_of(ch) else {
        return UNKNOWN_ADVANCE;
    };
    match font.glyphs[index].right {
        Some(right) => right + 2,
        None => BLANK_ADVANCE,
    }
}

 
 
pub(crate) fn width(text: &str) -> usize {
    text.chars().map(advance).sum()
}

 
 
 
 
 
 
pub(crate) fn texture(ctx: &Context) -> Option<TextureId> {
    let font = font()?;
    let key = egui::Id::new("infered_mc_font_sheet");
    if let Some(id) = ctx.data(|data| data.get_temp::<TextureId>(key)) {
        return Some(id);
    }
    let image = egui::ColorImage::from_rgba_unmultiplied([SHEET_PIXELS, SHEET_PIXELS], &font.rgba);
    let id = ctx
        .load_texture("infered_mc_font", image, TextureOptions::NEAREST)
        .id();
    ctx.data_mut(|data| data.insert_temp(key, id));
    Some(id)
}

 
 
 
 
 
 
 
pub(crate) fn paint(
    painter: &Painter,
    texture: TextureId,
    origin: Pos2,
    text: &str,
    pixel: f32,
    colour: Color32,
) -> f32 {
    let Some(font) = font() else {
        return 0.0;
    };
    if !pixel.is_finite() || pixel <= 0.0 {
        return 0.0;
    }
    let mut mesh = egui::epaint::Mesh {
        texture_id: texture,
        ..Default::default()
    };
    let mut pen = 0.0_f32;
    let texel = 1.0 / SHEET_PIXELS as f32;
    for ch in text.chars() {
        if let Some(index) = index_of(ch) {
            let (x0, y0) = cell_origin(index);
            for &(column, row) in &font.glyphs[index].ink {
                let left = origin.x + (pen + column as f32) * pixel;
                let top = origin.y + row as f32 * pixel;
                let right = left + pixel;
                let bottom = top + pixel;
                 
                 
                let uv = egui::pos2(
                    (x0 + column) as f32 * texel + texel * 0.5,
                    (y0 + row) as f32 * texel + texel * 0.5,
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
                        color: colour,
                    });
                }
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base, base + 2, base + 3);
            }
        }
        pen += advance(ch) as f32;
    }
    painter.add(egui::Shape::mesh(mesh));
    pen * pixel
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{LayerId, Order, Rect};

    fn inked(ch: char) -> usize {
        font().expect("the bundled sheet decodes").glyphs[index_of(ch).expect("ascii")]
            .ink
            .len()
    }

    #[test]
    fn the_bundled_sheet_is_the_games_own_ascii_font() {
        let font = font().expect("the bundled sheet decodes");
        assert_eq!(font.rgba.len(), SHEET_PIXELS * SHEET_PIXELS * 4);
         
         
        assert_eq!(inked('A'), 18);
        assert_eq!(inked('W'), 17);
        assert_eq!(inked('i'), 6);
        assert_eq!(inked(' '), 0);
        for row in 0..FIRST_ASCII_ROW {
            for column in 0..COLUMNS {
                let (x0, y0) = (column * CELL, row * CELL);
                for cell in 0..CELL {
                    for line in 0..CELL {
                        let at = ((y0 + line) * SHEET_PIXELS + (x0 + cell)) * 4;
                        assert_eq!(font.rgba[at + 3], 0, "row {row} must be empty");
                    }
                }
            }
        }
    }

    #[test]
    fn glyphs_advance_by_their_own_width_and_one_pixel_of_spacing() {
         
        assert_eq!(advance('A'), 6);
        assert_eq!(advance('W'), 6);
        assert_eq!(advance('i'), 2);
        assert_eq!(advance('l'), 3);
        assert_eq!(advance(' '), BLANK_ADVANCE);
         
         
        assert_eq!(advance('é'), UNKNOWN_ADVANCE);

        assert_eq!(width("Johnny"), 36);
        assert_eq!(width("Infered"), 39);
        assert_eq!(width(""), 0);
    }

     
    fn painted(text: &str, pixel: f32) -> (Vec<egui::epaint::Mesh>, f32) {
        let ctx = Context::default();
        let mut drawn = Vec::new();
        let mut width = 0.0;
        for frame in 0..2 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(200.0, 60.0))),
                    time: Some(frame as f64),
                    ..Default::default()
                },
                |ctx| {
                    let painter =
                        ctx.layer_painter(LayerId::new(Order::Background, egui::Id::new("font")));
                    let id = texture(ctx).expect("the sheet uploads");
                    width = paint(&painter, id, Pos2::ZERO, text, pixel, Color32::RED);
                },
            );
            drawn = output
                .shapes
                .into_iter()
                .filter_map(|clipped| match clipped.shape {
                    egui::Shape::Mesh(mesh) => Some(mesh),
                    _ => None,
                })
                .collect();
        }
        (drawn, width)
    }

    #[test]
    fn painting_emits_one_quad_per_inked_texel_at_the_given_scale() {
        let (meshes, width) = painted("Johnny", 2.0);
        assert_eq!(meshes.len(), 1, "one mesh for the whole line");
        let mesh = &meshes[0];
        let mut quads = 0;
        for chunk in mesh.vertices.chunks(4) {
            quads += 1;
            for vertex in chunk {
                assert!(vertex.color == Color32::RED, "glyphs take the tint");
                assert!(vertex.pos.y >= 0.0 && vertex.pos.y <= 8.0 * 2.0);
                assert!(vertex.pos.x >= 0.0 && vertex.pos.x <= width + 1e-3);
            }
        }
        let expected: usize = "Johnny".chars().map(inked).sum();
        assert_eq!(quads, expected, "one quad per inked texel");
        assert!(
            (width - 36.0 * 2.0).abs() < 1e-4,
            "width at the scale: {width}"
        );
    }

    #[test]
    fn a_corrupt_scale_draws_nothing_rather_than_a_screen_of_quads() {
        for pixel in [0.0, -2.0, f32::NAN, f32::INFINITY] {
            let ctx = Context::default();
            for frame in 0..2 {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(80.0, 40.0))),
                        time: Some(frame as f64),
                        ..Default::default()
                    },
                    |ctx| {
                        let painter = ctx
                            .layer_painter(LayerId::new(Order::Background, egui::Id::new("font")));
                        let id = texture(ctx).expect("the sheet uploads");
                        assert_eq!(
                            paint(&painter, id, Pos2::ZERO, "Infered", pixel, Color32::RED),
                            0.0
                        );
                    },
                );
                for clipped in output.shapes {
                    assert!(
                        !matches!(clipped.shape, egui::Shape::Mesh(_)),
                        "a corrupt scale must paint nothing"
                    );
                }
            }
        }
    }
}
