 
 
 
 
 
 

use egui::{Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};

use crate::graphic::hud::paint_panel;
use crate::graphic::theme;
use crate::module::render::custom_ui::UiStyle;

 
pub(crate) const W: f32 = 154.0;
pub(crate) const H: f32 = 64.0;

 
pub(crate) const SPARK_SAMPLES: usize = 24;

 
const PAD: f32 = 8.0;

 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Quality {
    Excellent,
    Good,
    Fair,
    Poor,
    Offline,
}

impl Quality {
     
    pub(crate) fn of(ping: Option<u32>) -> Self {
        match ping {
            None => Self::Offline,
            Some(ms) if ms <= 40 => Self::Excellent,
            Some(ms) if ms <= 90 => Self::Good,
            Some(ms) if ms <= 160 => Self::Fair,
            Some(_) => Self::Poor,
        }
    }

     
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Excellent => "EXCELLENT",
            Self::Good => "GOOD",
            Self::Fair => "FAIR",
            Self::Poor => "POOR",
            Self::Offline => "NO LINK",
        }
    }

     
    pub(crate) fn color(self) -> Color32 {
        match self {
            Self::Excellent => theme::SUCCESS,
            Self::Good => theme::lerp_color(theme::SUCCESS, theme::WARN, 0.45),
            Self::Fair => theme::WARN,
            Self::Poor => theme::DANGER,
            Self::Offline => theme::TEXT_MUTED,
        }
    }

     
    pub(crate) fn bars(self) -> usize {
        match self {
            Self::Excellent => 5,
            Self::Good => 4,
            Self::Fair => 3,
            Self::Poor => 1,
            Self::Offline => 0,
        }
    }
}

 
pub(crate) fn size(scale: f32) -> Vec2 {
    Vec2::new(W, H) * scale
}

 
 
pub(crate) fn draw(painter: &Painter, origin: Pos2, ping: Option<u32>, history: &[u32]) {
    let style = UiStyle::hud_style();
    let s = style.scale;
    let rect = Rect::from_min_size(origin, size(s));
    paint_panel(painter, rect, &style);

    let quality = Quality::of(ping);
    let color = quality.color();
    let left = rect.left() + PAD * s;
    let right = rect.right() - PAD * s;

     
    let header_y = rect.top() + 13.0 * s;
    painter.circle_filled(
        Pos2::new(left + 1.7 * s, header_y),
        3.4 * s,
        theme::with_alpha(color, 0.14),
    );
    painter.circle_filled(Pos2::new(left + 1.7 * s, header_y), 1.7 * s, color);
    painter.text(
        Pos2::new(left + 8.0 * s, header_y),
        Align2::LEFT_CENTER,
        "PING",
        FontId::proportional((8.0 * s).max(6.0)),
        theme::with_alpha(style.text, 0.76),
    );
    painter.text(
        Pos2::new(right, header_y),
        Align2::RIGHT_CENTER,
        quality.label(),
        FontId::monospace((7.2 * s).max(6.0)),
        theme::with_alpha(color, 0.9),
    );
    let rule_y = rect.top() + 20.0 * s;
    painter.line_segment(
        [Pos2::new(left, rule_y), Pos2::new(right, rule_y)],
        Stroke::new(0.8 * s, theme::with_alpha(theme::BORDER, 0.55)),
    );
    painter.line_segment(
        [Pos2::new(left, rule_y), Pos2::new(left + 18.0 * s, rule_y)],
        Stroke::new(1.2 * s, theme::with_alpha(color, 0.55)),
    );

     
    let value_y = rect.top() + 34.0 * s;
    let value = match ping {
        Some(ms) => ms.to_string(),
        None => "--".to_string(),
    };
    let value_font = FontId::monospace((22.0 * s).max(12.0));
    painter.text(
        Pos2::new(left, value_y),
        Align2::LEFT_CENTER,
        &value,
        value_font.clone(),
        theme::lerp_color(style.text, color, 0.35),
    );
    let value_width = painter.ctx().fonts(|fonts| {
        fonts
            .layout_no_wrap(value, value_font, Color32::WHITE)
            .size()
            .x
    });
    let unit_color = if ping.is_some() {
        theme::with_alpha(theme::TEXT_DIM, 0.7)
    } else {
        theme::with_alpha(theme::TEXT_MUTED, 0.7)
    };
    painter.text(
        Pos2::new(left + value_width + 3.5 * s, value_y + 6.0 * s),
        Align2::LEFT_CENTER,
        "ms",
        FontId::monospace((8.0 * s).max(6.0)),
        unit_color,
    );

     
    const BAR_HEIGHTS: [f32; 5] = [4.0, 7.0, 10.0, 13.0, 16.0];
    let bar_width = 3.2 * s;
    let bar_gap = 2.8 * s;
    let bars_bottom = rect.top() + 42.0 * s;
    let bars_left = right - (BAR_HEIGHTS.len() as f32 * bar_width + 4.0 * bar_gap);
    let lit = quality.bars();
    for (index, height) in BAR_HEIGHTS.iter().enumerate() {
        let x = bars_left + index as f32 * (bar_width + bar_gap);
        let top = bars_bottom - height * s;
        if index < lit {
             
             
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(x - 0.8 * s, top - 0.8 * s),
                    Pos2::new(x + bar_width + 0.8 * s, bars_bottom + 0.4 * s),
                ),
                egui::Rounding::same(1.6 * s),
                theme::with_alpha(color, 0.12),
            );
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(x, top), Pos2::new(x + bar_width, bars_bottom)),
                egui::Rounding::same(1.2 * s),
                theme::with_alpha(color, 0.92),
            );
        } else {
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(x, top), Pos2::new(x + bar_width, bars_bottom)),
                egui::Rounding::same(1.1 * s),
                theme::with_alpha(theme::TEXT, 0.09),
            );
        }
    }
     
    let graph = Rect::from_min_max(
        Pos2::new(left, rect.top() + 46.0 * s),
        Pos2::new(right, rect.top() + 58.0 * s),
    );
    painter.line_segment(
        [graph.left_bottom(), graph.right_bottom()],
        Stroke::new(0.8 * s, theme::with_alpha(theme::TEXT, 0.05)),
    );
    let points = spark_points(history, graph);
    if let Some(last) = points.last() {
        if points.len() >= 2 {
            painter.add(Shape::mesh(spark_area(&points, graph.bottom(), color)));
            for pair in points.windows(2) {
                painter.line_segment(
                    [pair[0], pair[1]],
                    Stroke::new(2.4 * s, theme::with_alpha(color, 0.12)),
                );
            }
            for pair in points.windows(2) {
                painter.line_segment(
                    [pair[0], pair[1]],
                    Stroke::new(1.1 * s, theme::with_alpha(color, 0.85)),
                );
            }
        }
        painter.circle_filled(*last, 3.0 * s, theme::with_alpha(color, 0.16));
        painter.circle_filled(*last, 1.5 * s, color);
    }
}

 
pub(crate) fn draw_preview(painter: &Painter, origin: Pos2) {
    const SAMPLE: [u32; SPARK_SAMPLES] = [
        34, 31, 36, 29, 28, 32, 35, 30, 27, 25, 29, 33, 38, 41, 36, 31, 28, 26, 24, 27, 30, 28, 25,
        24,
    ];
    draw(painter, origin, Some(24), &SAMPLE);
}

 
 
 
pub(crate) fn spark_points(history: &[u32], rect: Rect) -> Vec<Pos2> {
    if history.is_empty() {
        return Vec::new();
    }
     
     
     
    let peak = (history.iter().copied().max().unwrap_or(0) as f32 * 1.12).max(35.0);
    let step = rect.width() / (SPARK_SAMPLES.saturating_sub(1)) as f32;
    let start = rect.right() - step * history.len().saturating_sub(1) as f32;
    history
        .iter()
        .enumerate()
        .map(|(index, ms)| {
            let level = (*ms as f32 / peak).clamp(0.0, 1.0);
            Pos2::new(
                start + step * index as f32,
                rect.bottom() - level * rect.height(),
            )
        })
        .collect()
}

 
 
fn spark_area(points: &[Pos2], base: f32, color: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    for point in points {
        mesh.colored_vertex(Pos2::new(point.x, base), theme::with_alpha(color, 0.0));
        mesh.colored_vertex(*point, theme::with_alpha(color, 0.18));
    }
    for index in 0..points.len().saturating_sub(1) {
        let at = (index * 2) as u32;
        mesh.indices
            .extend_from_slice(&[at, at + 1, at + 2, at + 1, at + 3, at + 2]);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_buckets_are_stable_at_their_edges() {
        assert_eq!(Quality::of(None), Quality::Offline);
        assert_eq!(Quality::of(Some(0)), Quality::Excellent);
        assert_eq!(Quality::of(Some(40)), Quality::Excellent);
        assert_eq!(Quality::of(Some(41)), Quality::Good);
        assert_eq!(Quality::of(Some(90)), Quality::Good);
        assert_eq!(Quality::of(Some(91)), Quality::Fair);
        assert_eq!(Quality::of(Some(160)), Quality::Fair);
        assert_eq!(Quality::of(Some(161)), Quality::Poor);
        assert_eq!(Quality::of(Some(5_000)), Quality::Poor);
    }

    #[test]
    fn every_bucket_has_a_label_and_a_bar_count() {
        let buckets = [
            (Quality::Excellent, 5),
            (Quality::Good, 4),
            (Quality::Fair, 3),
            (Quality::Poor, 1),
            (Quality::Offline, 0),
        ];
        for (quality, bars) in buckets {
            assert!(!quality.label().is_empty());
            assert_eq!(quality.bars(), bars, "{quality:?}");
            assert!(quality.bars() <= 5);
        }
        assert_eq!(Quality::Offline.label(), "NO LINK");
    }

    #[test]
    fn the_sparkline_puts_the_newest_reading_at_the_right_edge() {
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(100.0, 20.0));
        let points = spark_points(&[10, 20, 40], rect);
        assert_eq!(points.len(), 3);
        assert!((points[2].x - rect.right()).abs() < 1e-3);
         
        let step = 100.0 / 23.0;
        assert!((points[0].x - (rect.right() - 2.0 * step)).abs() < 1e-3);
         
        assert!(points[2].y < points[1].y);
        assert!(points[1].y < points[0].y);
         
        assert!(points.iter().all(|point| rect.contains(*point)));
    }

    #[test]
    fn an_empty_history_has_no_line_and_a_spike_rescales_the_graph() {
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(100.0, 20.0));
        assert!(spark_points(&[], rect).is_empty());
         
         
        let quiet = spark_points(&[40, 40], rect);
        assert!(quiet[0].y > rect.top());
        let spiky = spark_points(&[40, 400], rect);
        assert!(spiky[1].y > rect.top());
        assert!(spiky[1].y < rect.center().y);
        assert!(spiky[1].y < spiky[0].y);
    }

    #[test]
    fn a_full_history_spans_the_whole_graph() {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(120.0, 8.0));
        let history = [30u32; SPARK_SAMPLES];
        let points = spark_points(&history, rect);
        assert_eq!(points.len(), SPARK_SAMPLES);
        assert!((points[0].x - rect.left()).abs() < 1e-3);
        assert!((points[SPARK_SAMPLES - 1].x - rect.right()).abs() < 1e-3);
    }

     
    #[test]
    fn the_ping_card_renders_its_readout_bars_and_graph() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let (width, height) = (200usize, 100usize);
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
                        egui::Id::new("ping_preview"),
                    ));
                    draw_preview(&painter, Pos2::new(20.0, 20.0));
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
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/ping-preview.png");
        image.save(&path).expect("the ping preview saves");

        for expected in ["PING", "24", "ms", "EXCELLENT"] {
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
        assert!(lit > 4500, "the card should be visibly inked, got {lit}");
    }
}
