 
 
 
 
 
 
 

use egui::{Color32, Pos2};
use std::collections::HashMap;

 
 
 
const EDGE_TOLERANCE: f32 = 1e-4;

 
#[derive(Clone)]
pub(crate) struct PreviewTexture {
    w: usize,
    h: usize,
    px: Vec<Color32>,
}

 
 
pub(crate) fn gather_textures(
    map: &mut HashMap<egui::TextureId, PreviewTexture>,
    delta: &egui::TexturesDelta,
) {
    for (id, image_delta) in &delta.set {
        let (w, h, pixels): (usize, usize, Vec<Color32>) = match &image_delta.image {
            egui::ImageData::Color(color) => (color.width(), color.height(), color.pixels.clone()),
            egui::ImageData::Font(font) => (
                font.width(),
                font.height(),
                font.srgba_pixels(None).collect(),
            ),
        };
        if let Some([x0, y0]) = image_delta.pos {
            let Some(texture) = map.get_mut(id) else {
                continue;
            };
            for y in 0..h {
                for x in 0..w {
                    let (dx, dy) = (x0 + x, y0 + y);
                    if dx < texture.w && dy < texture.h {
                        texture.px[dy * texture.w + dx] = pixels[y * w + x];
                    }
                }
            }
        } else {
            map.insert(*id, PreviewTexture { w, h, px: pixels });
        }
    }
}

 
 
fn sample_texture(texture: Option<&PreviewTexture>, uv: Pos2) -> [f32; 4] {
    let Some(texture) = texture else {
        return [1.0; 4];
    };
    let x = (uv.x * texture.w as f32 - 0.5).clamp(0.0, texture.w as f32 - 1.0);
    let y = (uv.y * texture.h as f32 - 0.5).clamp(0.0, texture.h as f32 - 1.0);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(texture.w - 1), (y0 + 1).min(texture.h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let at = |x: usize, y: usize| {
        let color = texture.px[y * texture.w + x];
        [
            color.r() as f32,
            color.g() as f32,
            color.b() as f32,
            color.a() as f32,
        ]
    };
    let mut out = [0.0; 4];
    for (i, channel) in out.iter_mut().enumerate() {
        let top = at(x0, y0)[i] * (1.0 - fx) + at(x1, y0)[i] * fx;
        let bottom = at(x0, y1)[i] * (1.0 - fx) + at(x1, y1)[i] * fx;
        *channel = (top * (1.0 - fy) + bottom * fy) / 255.0;
    }
    out
}

 
pub(crate) fn rasterize(
    primitives: Vec<egui::ClippedPrimitive>,
    textures: &HashMap<egui::TextureId, PreviewTexture>,
    w: usize,
    h: usize,
    backdrop: [f32; 4],
) -> image::RgbaImage {
    let mut buffer = vec![backdrop; w * h];
    for clipped in primitives {
        let egui::epaint::Primitive::Mesh(mesh) = clipped.primitive else {
            continue;
        };
        let texture = textures.get(&mesh.texture_id);
        let clip = clipped.clip_rect;
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let v = [
                mesh.vertices[triangle[0] as usize],
                mesh.vertices[triangle[1] as usize],
                mesh.vertices[triangle[2] as usize],
            ];
            let area = (v[1].pos.x - v[0].pos.x) * (v[2].pos.y - v[0].pos.y)
                - (v[2].pos.x - v[0].pos.x) * (v[1].pos.y - v[0].pos.y);
            if area.abs() < 1e-6 {
                continue;
            }
            let span = |axis: fn(&Pos2) -> f32, low: bool, clamp: f32, limit: f32| {
                let value = v.iter().map(|vertex| axis(&vertex.pos)).fold(
                    if low { f32::MAX } else { f32::MIN },
                    if low { f32::min } else { f32::max },
                );
                let value = if low {
                    value.max(clamp)
                } else {
                    value.min(clamp)
                };
                if low {
                    value.floor().max(0.0).min(limit) as usize
                } else {
                    value.ceil().max(0.0).min(limit) as usize
                }
            };
            let min_x = span(|p| p.x, true, clip.left(), w as f32 - 1.0);
            let max_x = span(|p| p.x, false, clip.right(), w as f32 - 1.0);
            let min_y = span(|p| p.y, true, clip.top(), h as f32 - 1.0);
            let max_y = span(|p| p.y, false, clip.bottom(), h as f32 - 1.0);
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let p = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let w0 = ((v[1].pos.x - p.x) * (v[2].pos.y - p.y)
                        - (v[2].pos.x - p.x) * (v[1].pos.y - p.y))
                        / area;
                    let w1 = ((v[2].pos.x - p.x) * (v[0].pos.y - p.y)
                        - (v[0].pos.x - p.x) * (v[2].pos.y - p.y))
                        / area;
                    let w2 = 1.0 - w0 - w1;
                     
                     
                     
                     
                     
                    if w0 < -EDGE_TOLERANCE || w1 < -EDGE_TOLERANCE || w2 < -EDGE_TOLERANCE {
                        continue;
                    }
                    let (w0, w1, w2) = (w0.max(0.0), w1.max(0.0), w2.max(0.0));
                    let color = |i: usize| {
                        w0 * v[0].color[i] as f32
                            + w1 * v[1].color[i] as f32
                            + w2 * v[2].color[i] as f32
                    };
                    let uv = Pos2::new(
                        w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x,
                        w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y,
                    );
                    let texel = sample_texture(texture, uv);
                    let mut src = [0.0; 4];
                    for i in 0..4 {
                        src[i] = color(i) / 255.0 * texel[i];
                    }
                    let dst = &mut buffer[y * w + x];
                    for i in 0..4 {
                        dst[i] = src[i] + dst[i] * (1.0 - src[3]);
                    }
                }
            }
        }
    }
    let mut image = image::RgbaImage::new(w as u32, h as u32);
    for (index, pixel) in buffer.iter().enumerate() {
        let alpha = pixel[3].clamp(0.0, 1.0);
        let channel = |i: usize| {
            if alpha <= 0.0 {
                0
            } else {
                ((pixel[i] / alpha).clamp(0.0, 1.0) * 255.0).round() as u8
            }
        };
        image.put_pixel(
            (index % w) as u32,
            (index / w) as u32,
            image::Rgba([channel(0), channel(1), channel(2), (alpha * 255.0) as u8]),
        );
    }
    image
}
