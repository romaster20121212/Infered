 
 
 
 
 
 
 
 
 

use egui::{Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};

use crate::graphic::hud::paint_panel;
use crate::graphic::theme;
use crate::module::render::custom_ui::UiStyle;

 
pub(crate) const W: f32 = 180.0;
pub(crate) const H: f32 = 68.0;

 
 
pub(crate) const MAX_SPEED: f64 = 10.0;

const PAD: f32 = 7.0;
const DIAL: f32 = 54.0;
const ARC_START: f32 = 150.0;
const ARC_SWEEP: f32 = 240.0;

 
pub(crate) fn size(scale: f32) -> Vec2 {
    Vec2::new(W, H) * scale
}

 
 
pub(crate) fn draw(painter: &Painter, origin: Pos2, speed: f64, peak: f64) {
    let style = UiStyle::hud_style();
    let s = style.scale;
    let rect = Rect::from_min_size(origin, size(s));
    paint_panel(painter, rect, &style);

    let speed = if speed.is_finite() {
        speed.max(0.0)
    } else {
        0.0
    };
    let peak = peak.max(speed);
    let fraction = speed_fraction(speed);
    let color = speed_color(fraction);

     
    let center = origin + Vec2::new(PAD + DIAL * 0.5, H * 0.5) * s;
    let radius = (DIAL * 0.5 - 1.0) * s;
    painter.circle_filled(
        center + Vec2::new(0.0, 1.5 * s),
        radius,
        Color32::from_black_alpha(70),
    );
    painter.circle_filled(center, radius, theme::with_alpha(theme::base(), 0.92));
    painter.circle_stroke(
        center,
        radius - 0.5 * s,
        Stroke::new(0.9 * s, theme::with_alpha(theme::BORDER, 0.9)),
    );
    painter.circle_filled(
        center,
        radius - 3.0 * s,
        theme::with_alpha(theme::SURFACE, 0.42),
    );
     
     
    painter.circle_stroke(
        center,
        radius - 3.0 * s,
        Stroke::new(0.8 * s, theme::with_alpha(color, 0.16)),
    );

    let arc_radius = 20.5 * s;
    let arc_width = 3.4 * s;
    let track = arc_mesh(
        center,
        arc_radius,
        arc_width,
        ARC_START,
        ARC_SWEEP,
        48,
        |_| theme::with_alpha(theme::TEXT, 0.07),
    );
    painter.add(Shape::mesh(track));

    if fraction > 0.001 {
         
        let glow = arc_mesh(
            center,
            arc_radius,
            arc_width * 2.2,
            ARC_START,
            ARC_SWEEP * fraction,
            40,
            |t| theme::with_alpha(speed_color(fraction * t), 0.08),
        );
        painter.add(Shape::mesh(glow));
        let lit = arc_mesh(
            center,
            arc_radius,
            arc_width,
            ARC_START,
            ARC_SWEEP * fraction,
            40,
            |t| speed_color(fraction * t),
        );
        painter.add(Shape::mesh(lit));

        let tip_angle = (ARC_START + ARC_SWEEP * fraction).to_radians();
        let tip = center + Vec2::new(tip_angle.cos(), tip_angle.sin()) * arc_radius;
        painter.circle_filled(tip, 4.2 * s, theme::with_alpha(color, 0.18));
        painter.circle_filled(tip, 1.9 * s, color);
    }

     
    for step in 0..=20 {
        let t = step as f32 / 20.0;
        let angle = (ARC_START + ARC_SWEEP * t).to_radians();
        let direction = Vec2::new(angle.cos(), angle.sin());
        let major = step % 4 == 0;
        let (inner, outer, width, alpha) = if major {
            (17.0, 19.6, 1.1, 0.5)
        } else {
            (18.2, 19.6, 0.7, 0.22)
        };
        painter.line_segment(
            [
                center + direction * (inner * s),
                center + direction * (outer * s),
            ],
            Stroke::new(width * s, theme::with_alpha(theme::TEXT_DIM, alpha)),
        );
    }

     
    if peak > 0.01 {
        let peak_angle = (ARC_START + ARC_SWEEP * speed_fraction(peak)).to_radians();
        let direction = Vec2::new(peak_angle.cos(), peak_angle.sin());
        painter.line_segment(
            [
                center + direction * (23.6 * s),
                center + direction * (26.0 * s),
            ],
            Stroke::new(1.3 * s, theme::with_alpha(PEAK_COLOR, 0.9)),
        );
    }

     
    let needle_angle = (ARC_START + ARC_SWEEP * fraction).to_radians();
    let needle = Vec2::new(needle_angle.cos(), needle_angle.sin());
    let tip = center + needle * (17.4 * s);
    let heel = center - needle * (3.5 * s);
    painter.line_segment(
        [heel, tip],
        Stroke::new(3.6 * s, theme::with_alpha(color, 0.16)),
    );
    painter.line_segment(
        [heel, tip],
        Stroke::new(1.25 * s, theme::with_alpha(theme::TEXT, 0.94)),
    );
    painter.circle_filled(center, 4.4 * s, theme::base());
    painter.circle_stroke(
        center,
        4.4 * s,
        Stroke::new(0.9 * s, theme::with_alpha(theme::BORDER, 0.95)),
    );
    painter.circle_filled(center, 1.7 * s, color);

     
    let text_x = rect.left() + (PAD + DIAL + 9.0) * s;
    let right = rect.right() - PAD * s;
    let header_y = rect.top() + 13.0 * s;
    let value_y = rect.top() + 35.0 * s;
    let foot_y = rect.top() + 52.0 * s;

     
    painter.line_segment(
        [
            Pos2::new(text_x - 5.0 * s, rect.top() + 10.0 * s),
            Pos2::new(text_x - 5.0 * s, rect.bottom() - 10.0 * s),
        ],
        Stroke::new(0.8 * s, theme::with_alpha(theme::TEXT, 0.05)),
    );

    painter.circle_filled(
        Pos2::new(text_x + 1.5 * s, header_y),
        1.6 * s,
        theme::with_alpha(style.accent, 0.95),
    );
    painter.text(
        Pos2::new(text_x + 7.0 * s, header_y),
        Align2::LEFT_CENTER,
        "SPEED",
        FontId::proportional((8.0 * s).max(6.0)),
        theme::with_alpha(style.text, 0.76),
    );
    painter.text(
        Pos2::new(right, header_y),
        Align2::RIGHT_CENTER,
        format!("MAX {MAX_SPEED:.0}"),
        FontId::monospace((7.2 * s).max(6.0)),
        theme::with_alpha(theme::TEXT_DIM, 0.55),
    );
    let rule_y = rect.top() + 19.0 * s;
    painter.line_segment(
        [Pos2::new(text_x, rule_y), Pos2::new(right, rule_y)],
        Stroke::new(0.8 * s, theme::with_alpha(theme::BORDER, 0.55)),
    );
    painter.line_segment(
        [
            Pos2::new(text_x, rule_y),
            Pos2::new(text_x + 18.0 * s, rule_y),
        ],
        Stroke::new(1.2 * s, theme::with_alpha(style.accent, 0.5)),
    );

    let value = speed_text(speed);
    let value_font = FontId::monospace((23.0 * s).max(12.0));
    painter.text(
        Pos2::new(text_x, value_y),
        Align2::LEFT_CENTER,
        &value,
        value_font.clone(),
        theme::lerp_color(theme::TEXT, color, 0.28),
    );
    let value_width = painter.ctx().fonts(|fonts| {
        fonts
            .layout_no_wrap(value, value_font, Color32::WHITE)
            .size()
            .x
    });
    painter.text(
        Pos2::new(text_x + value_width + 4.0 * s, value_y + 6.5 * s),
        Align2::LEFT_CENTER,
        "b/s",
        FontId::monospace((8.0 * s).max(6.0)),
        theme::with_alpha(theme::TEXT_DIM, 0.62),
    );

    painter.text(
        Pos2::new(text_x, foot_y),
        Align2::LEFT_CENTER,
        "TOP",
        FontId::proportional((7.2 * s).max(6.0)),
        theme::with_alpha(theme::TEXT_MUTED, 0.8),
    );
    painter.text(
        Pos2::new(text_x + 19.0 * s, foot_y),
        Align2::LEFT_CENTER,
        speed_text(peak),
        FontId::monospace((8.2 * s).max(6.0)),
        theme::with_alpha(PEAK_COLOR, 0.82),
    );

     
    let meter = Rect::from_min_size(
        Pos2::new(text_x + 52.0 * s, foot_y - 1.2 * s),
        Vec2::new((right - text_x - 52.0 * s).max(1.0), 2.4 * s),
    );
    painter.rect_filled(
        meter,
        egui::Rounding::same(1.2 * s),
        theme::with_alpha(theme::TEXT, 0.07),
    );
    let meter_width = meter.width() * fraction;
    if meter_width > 0.5 {
        let fill = Rect::from_min_size(
            meter.min,
            Vec2::new(meter_width.max(2.4 * s), meter.height()),
        );
        painter.rect_filled(
            fill,
            egui::Rounding::same(1.2 * s),
            theme::with_alpha(color, 0.8),
        );
    }
     
    let sprint_x = meter.left() + meter.width() * speed_fraction(5.6);
    painter.line_segment(
        [
            Pos2::new(sprint_x, meter.top() - 1.0 * s),
            Pos2::new(sprint_x, meter.bottom() + 1.0 * s),
        ],
        Stroke::new(0.8 * s, theme::with_alpha(theme::TEXT, 0.22)),
    );
}

 
pub(crate) fn draw_preview(painter: &Painter, origin: Pos2) {
    draw(painter, origin, 5.6, 8.4);
}

 
pub(crate) fn speed_fraction(speed: f64) -> f32 {
    if !speed.is_finite() {
        return 0.0;
    }
    (speed / MAX_SPEED).clamp(0.0, 1.0) as f32
}

 
pub(crate) fn gauge_angle(fraction: f32) -> f32 {
    ARC_START + ARC_SWEEP * fraction.clamp(0.0, 1.0)
}

 
 
pub(crate) fn speed_color(fraction: f32) -> Color32 {
    const STOPS: [(f32, [u8; 3]); 4] = [
        (0.0, [64, 224, 208]),
        (0.45, [124, 227, 138]),
        (0.72, [255, 196, 84]),
        (1.0, [248, 96, 96]),
    ];
    let t = fraction.clamp(0.0, 1.0);
    let mut lower = STOPS[0];
    let mut upper = STOPS[STOPS.len() - 1];
    for pair in STOPS.windows(2) {
        if t >= pair[0].0 && t <= pair[1].0 {
            lower = pair[0];
            upper = pair[1];
            break;
        }
    }
    let span = (upper.0 - lower.0).max(1e-6);
    let k = ((t - lower.0) / span).clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * k).round() as u8;
    Color32::from_rgb(
        mix(lower.1[0], upper.1[0]),
        mix(lower.1[1], upper.1[1]),
        mix(lower.1[2], upper.1[2]),
    )
}

 
pub(crate) fn speed_text(speed: f64) -> String {
    if !speed.is_finite() || speed <= 0.0 {
        return "0.0".to_string();
    }
    if speed < 100.0 {
        format!("{speed:.1}")
    } else {
        format!("{speed:.0}")
    }
}

const PEAK_COLOR: Color32 = Color32::from_rgb(255, 198, 96);

 
 
fn arc_mesh(
    center: Pos2,
    radius: f32,
    width: f32,
    start: f32,
    sweep: f32,
    steps: usize,
    color_at: impl Fn(f32) -> Color32,
) -> Mesh {
    let mut mesh = Mesh::default();
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let angle = (start + sweep * t).to_radians();
        let direction = Vec2::new(angle.cos(), angle.sin());
        let color = color_at(t);
        mesh.colored_vertex(center + direction * (radius + width * 0.5), color);
        mesh.colored_vertex(center + direction * (radius - width * 0.5), color);
    }
    for step in 0..steps {
        let base = (step * 2) as u32;
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_clamps_and_survives_garbage() {
        assert_eq!(speed_fraction(0.0), 0.0);
        assert!((speed_fraction(5.0) - 0.5).abs() < 1e-6);
        assert_eq!(speed_fraction(MAX_SPEED), 1.0);
        assert_eq!(speed_fraction(MAX_SPEED * 6.0), 1.0);
        assert_eq!(speed_fraction(-3.0), 0.0);
        assert_eq!(speed_fraction(f64::NAN), 0.0);
        assert_eq!(speed_fraction(f64::INFINITY), 0.0);
    }

    #[test]
    fn the_gauge_sweeps_240_degrees() {
        assert!((gauge_angle(0.0) - ARC_START).abs() < 1e-6);
        assert!((gauge_angle(1.0) - (ARC_START + ARC_SWEEP)).abs() < 1e-6);
        assert!((gauge_angle(0.5) - 270.0).abs() < 1e-6);
        assert_eq!(gauge_angle(4.0), ARC_START + ARC_SWEEP);
        assert_eq!(gauge_angle(-1.0), ARC_START);
    }

    #[test]
    fn the_colour_ramp_runs_from_teal_to_red() {
        let cold = speed_color(0.0);
        let middle = speed_color(0.6);
        let hot = speed_color(1.0);
        assert!(cold.g() > cold.r(), "low speed should be cool");
        assert!(hot.r() > hot.g(), "full scale should be hot");
        assert_ne!(cold, middle);
        assert_ne!(middle, hot);
        assert_eq!(speed_color(-2.0), cold);
        assert_eq!(speed_color(4.0), hot);
    }

    #[test]
    fn the_readout_keeps_a_decimal_only_while_it_fits() {
        assert_eq!(speed_text(5.64), "5.6");
        assert_eq!(speed_text(0.0), "0.0");
        assert_eq!(speed_text(-2.0), "0.0");
        assert_eq!(speed_text(f64::NAN), "0.0");
        assert_eq!(speed_text(99.94), "99.9");
        assert_eq!(speed_text(123.4), "123");
    }

     
    #[test]
    fn the_speedometer_preview_renders_the_dial_and_readout() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let (width, height) = (230usize, 110usize);
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = egui::Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        let mut texts = Vec::new();
        for frame in 0..3 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Background,
                        egui::Id::new("speedometer_preview"),
                    ));
                    draw_preview(&painter, Pos2::new(20.0, 21.0));
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            texts = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    Shape::Text(text) => Some(text.galley.text().to_owned()),
                    _ => None,
                })
                .collect();
            primitives = ctx.tessellate(output.shapes, 1.0);
        }

        let image = rasterize(
            primitives,
            &textures,
            width,
            height,
            [0.06, 0.06, 0.075, 1.0],
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/speedometer-preview.png");
        image.save(&path).expect("the speedometer preview saves");

        for expected in ["SPEED", "5.6", "b/s", "TOP", "8.4"] {
            assert!(
                texts.iter().any(|text| text == expected),
                "missing {expected:?} in {texts:?}"
            );
        }
         
         
         
        let backdrop = [0.06f32, 0.06, 0.075];
        let lit = image
            .pixels()
            .filter(|pixel| {
                pixel[3] > 120
                    && (0..3).any(|i| (pixel[i] as f32 / 255.0 - backdrop[i]).abs() > 0.02)
            })
            .count();
        assert!(lit > 6000, "the gauge should be visibly lit, got {lit}");
    }
}
