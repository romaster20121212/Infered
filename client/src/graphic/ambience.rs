 
 
 
 
 
 
 
 
 
 

use crate::graphic::theme;
use egui::{pos2, Color32, Mesh, Painter, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

 
pub(crate) fn halo(painter: &Painter, centre: Pos2, radius: f32, color: Color32, strength: f32) {
    if strength <= 0.001 || radius <= 0.5 {
        return;
    }
    const SEGMENTS: u32 = 40;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(centre, theme::with_alpha(color, strength));
    for i in 0..=SEGMENTS {
        let angle = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(
            centre + Vec2::angled(angle) * radius,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0),
        );
    }
    for i in 0..SEGMENTS {
        mesh.add_triangle(0, i + 1, i + 2);
    }
    painter.add(Shape::mesh(mesh));
}

 
 
 
 
pub(crate) fn panel_light(
    painter: &Painter,
    rect: Rect,
    rounding: f32,
    color: Color32,
    emphasis: f32,
) {
    if emphasis <= 0.001 {
        return;
    }
    for (grow, alpha) in [(1.0_f32, 0.10_f32), (2.5, 0.055), (4.5, 0.028)] {
        painter.rect_stroke(
            rect.expand(grow),
            Rounding::same(rounding + grow),
            Stroke::new(1.0_f32, theme::with_alpha(color, alpha * emphasis)),
        );
    }
}

 
 
pub(crate) fn row_sweep(painter: &Painter, rect: Rect, phase: f32, color: Color32, strength: f32) {
    if strength <= 0.001 || !(0.0..=1.0).contains(&phase) {
        return;
    }
    let head = rect.left() + rect.width() * phase;
    let falloff = rect.width() * 0.28;
    const SAMPLES: u32 = 16;
    let mut mesh = Mesh::default();
    let clear = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0);
    for i in 0..=SAMPLES {
        let x = rect.left() + rect.width() * i as f32 / SAMPLES as f32;
        let distance = ((x - head) / falloff).abs();
        let alpha = strength * (-distance * distance).exp();
        mesh.colored_vertex(pos2(x, rect.top()), clear);
        mesh.colored_vertex(pos2(x, rect.bottom()), theme::with_alpha(color, alpha));
    }
    for i in 0..SAMPLES {
        let base = i * 2;
        mesh.add_triangle(base, base + 1, base + 3);
        mesh.add_triangle(base, base + 3, base + 2);
    }
    painter.add(Shape::mesh(mesh));
}
