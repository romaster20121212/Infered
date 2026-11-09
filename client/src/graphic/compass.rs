 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::theme;
use egui::{Align2, FontId, Painter, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

 
pub(crate) const DIAL: f32 = 92.0;
 
const PILL_H: f32 = 16.0;
 
const PILL_GAP: f32 = 2.0;
 
pub(crate) const SIZE: Vec2 = Vec2::new(DIAL, DIAL + PILL_GAP + PILL_H);

 
const RIM: f32 = DIAL * 0.5;
 
const FACE: f32 = RIM - 3.0;
 
const TICK_OUTER: f32 = FACE - 1.0;
 
const CARDINAL_TICK: f32 = 4.2;
 
 
const LABEL_RADIUS: f32 = 30.0;
 
const LABEL_CHIP: f32 = 6.2;
 
const NEEDLE_LEN: f32 = 22.0;
const TAIL_LEN: f32 = 14.0;
 
const NEEDLE_HALF_WIDTH: f32 = 3.2;

 
const ARC_STEPS: usize = 72;
 
const ARC_RADIUS: f32 = RIM - 2.0;

 
const STAR_CARDINAL: f32 = 18.0;
 
const STAR_INTERCARDINAL: f32 = 23.0;
 
const STAR_CARDINAL_HALF: f32 = 4.0;
const STAR_INTERCARDINAL_HALF: f32 = 3.0;
 
const STAR_LIT_ALPHA: f32 = 0.26;
const STAR_SHADE_ALPHA: f32 = 0.55;

 
const ROSE: [(&str, f32, bool); 8] = [
    ("N", 0.0, true),
    ("NE", 45.0, false),
    ("E", 90.0, true),
    ("SE", 135.0, false),
    ("S", 180.0, true),
    ("SW", 225.0, false),
    ("W", 270.0, true),
    ("NW", 315.0, false),
];

 
 
 
pub(crate) fn facing_bearing(yaw: f32) -> f32 {
    if !yaw.is_finite() {
        return 0.0;
    }
    (yaw + 180.0).rem_euclid(360.0)
}

 
 
pub(crate) fn dial_angle(bearing: f32) -> f32 {
    bearing.rem_euclid(360.0).to_radians()
}

 
 
pub(crate) fn bearing_offset(bearing: f32, radius: f32) -> Vec2 {
    let angle = dial_angle(bearing);
    egui::vec2(angle.sin() * radius, -angle.cos() * radius)
}

 
 
pub(crate) fn heading(yaw: f32) -> i32 {
    facing_bearing(yaw).round().rem_euclid(360.0) as i32
}

 
fn direction(bearing: f32) -> Vec2 {
    bearing_offset(bearing, 1.0)
}

 
 
fn needle_half(
    center: Pos2,
    dir: Vec2,
    tip_len: f32,
    half_width: f32,
    fill: egui::Color32,
) -> Shape {
    let side = egui::vec2(-dir.y, dir.x) * half_width;
    Shape::convex_polygon(
        vec![center + dir * tip_len, center + side, center - side],
        fill,
        Stroke::NONE,
    )
}

 
 
 
fn draw_rose_star(painter: &Painter, center: Pos2) {
    let lit = theme::with_alpha(theme::TEXT_DIM, STAR_LIT_ALPHA);
    let shaded = theme::with_alpha(theme::base(), STAR_SHADE_ALPHA);
    for step in 0..8 {
        let dir = direction(step as f32 * 45.0);
        let side = egui::vec2(-dir.y, dir.x);
        let (tip_len, half) = if step % 2 == 0 {
            (STAR_CARDINAL, STAR_CARDINAL_HALF)
        } else {
            (STAR_INTERCARDINAL, STAR_INTERCARDINAL_HALF)
        };
        let tip = center + dir * tip_len;
        painter.add(Shape::convex_polygon(
            vec![center, center + side * half, tip],
            lit,
            Stroke::NONE,
        ));
        painter.add(Shape::convex_polygon(
            vec![center, tip, center - side * half],
            shaded,
            Stroke::NONE,
        ));
    }
}

 
 
 
fn heading_arc_points(center: Pos2, angle: f32) -> Vec<Pos2> {
    let angle = angle.rem_euclid(std::f32::consts::TAU);
    if angle < 0.01 {
        return Vec::new();
    }
    let steps = ((angle / std::f32::consts::TAU) * ARC_STEPS as f32).ceil() as usize;
    let steps = steps.max(1);
    (0..=steps)
        .map(|step| {
            let a = angle * step as f32 / steps as f32;
            center + egui::vec2(a.sin(), -a.cos()) * ARC_RADIUS
        })
        .collect()
}

fn draw_heading_arc(painter: &Painter, center: Pos2, angle: f32) {
    let points = heading_arc_points(center, angle);
    if points.len() < 2 {
        return;
    }
    painter.add(Shape::line(
        points,
        Stroke::new(1.8_f32, theme::with_alpha(theme::DANGER, 0.30)),
    ));
}

 
 
fn draw_heading_needle(painter: &Painter, center: Pos2, bearing: f32) {
    let dir = direction(bearing);
    painter.add(needle_half(
        center,
        dir,
        NEEDLE_LEN,
        NEEDLE_HALF_WIDTH,
        theme::DANGER,
    ));
    painter.add(needle_half(
        center,
        -dir,
        TAIL_LEN,
        NEEDLE_HALF_WIDTH * 0.7,
        theme::with_alpha(theme::TEXT_MUTED, 0.85),
    ));
}

 
 
fn draw_bearing_pip(painter: &Painter, center: Pos2, bearing: f32) {
    let pip = center + direction(bearing) * (RIM - 3.4);
    painter.circle_filled(pip, 4.6, theme::with_alpha(theme::DANGER, 0.16));
    painter.circle_filled(pip, 2.1, theme::DANGER);
}

 
fn nearest_rose(bearing: f32) -> usize {
    (((bearing.rem_euclid(360.0) + 22.5) / 45.0).floor() as usize) % ROSE.len()
}

 
 
 
 
pub(crate) fn draw(painter: &Painter, origin: Pos2, yaw: f32, label: &str) {
    let bearing = facing_bearing(yaw);
    let angle = dial_angle(bearing);
    let center = origin + Vec2::splat(RIM);

     
    painter.circle_filled(center, RIM, theme::with_alpha(theme::base(), 0.92));
    painter.circle_stroke(center, RIM - 0.6, Stroke::new(1.2_f32, theme::BORDER));
    painter.circle_filled(center, FACE, theme::with_alpha(theme::SURFACE, 0.72));
    painter.circle_stroke(
        center,
        FACE,
        Stroke::new(0.8_f32, theme::with_alpha(theme::BORDER, 0.9)),
    );

     
    draw_heading_arc(painter, center, angle);
    draw_rose_star(painter, center);
    draw_heading_needle(painter, center, bearing);

     
     
     
     
    for step in 0..24 {
        let dir = direction(step as f32 * 15.0);
        let (length, width, color): (f32, f32, egui::Color32) = match step % 6 {
            0 => (CARDINAL_TICK, 1.8, theme::with_alpha(theme::TEXT_DIM, 0.95)),
            3 => (4.4, 1.2, theme::with_alpha(theme::TEXT_MUTED, 0.85)),
            _ => (2.6, 1.0, theme::with_alpha(theme::TEXT_MUTED, 0.55)),
        };
        painter.line_segment(
            [
                center + dir * (TICK_OUTER - length),
                center + dir * TICK_OUTER,
            ],
            Stroke::new(width, color),
        );
    }

     
     
     
     
     
    let chip = theme::lerp_color(theme::base(), theme::SURFACE, 0.72);
    let active = nearest_rose(bearing);
    for (index, (letter, point, cardinal)) in ROSE.iter().enumerate() {
        let position = center + bearing_offset(*point, LABEL_RADIUS);
        painter.circle_filled(position, LABEL_CHIP, chip);
        let highlighted = index == active;
        let color = if highlighted {
            theme::TEXT
        } else if *cardinal {
            theme::with_alpha(theme::TEXT_DIM, 0.95)
        } else {
            theme::with_alpha(theme::TEXT_MUTED, 0.85)
        };
        let size = if highlighted {
            if *cardinal {
                12.0
            } else {
                8.5
            }
        } else if *cardinal {
            10.5
        } else {
            7.0
        };
        painter.text(
            position,
            Align2::CENTER_CENTER,
            letter,
            FontId::proportional(size),
            color,
        );
    }

     
     
    painter.circle_filled(center, 2.8, theme::base());
    painter.circle_stroke(center, 2.8, Stroke::new(1.0_f32, theme::BORDER));
    draw_bearing_pip(painter, center, bearing);

     
    let pill = Rect::from_min_size(
        origin + egui::vec2(0.0, DIAL + PILL_GAP),
        egui::vec2(DIAL, PILL_H),
    );
    painter.rect_filled(
        pill,
        Rounding::same(3.0),
        egui::Color32::from_black_alpha(170),
    );
    painter.rect_filled(
        Rect::from_min_size(pill.min, egui::vec2(2.0, PILL_H)),
        Rounding::ZERO,
        theme::DANGER,
    );
    painter.text(
        pill.center() + egui::vec2(2.0, 0.0),
        Align2::CENTER_CENTER,
        format!("{}° {}", heading(yaw), label),
        FontId::proportional(9.5),
        theme::TEXT,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Color32, Context, LayerId, Order};

    #[test]
    fn the_heading_is_the_bearing_the_player_faces() {
         
        assert_eq!(facing_bearing(180.0), 0.0);
        assert_eq!(facing_bearing(0.0), 180.0);
        assert_eq!(facing_bearing(270.0), 90.0);
        assert_eq!(facing_bearing(90.0), 270.0);
        assert_eq!(facing_bearing(-10.0), 170.0);
        assert_eq!(facing_bearing(540.0), 0.0);
        assert_eq!(facing_bearing(f32::NAN), 0.0);
    }

    #[test]
    fn the_readout_is_the_heading_in_compass_degrees() {
        assert_eq!(heading(180.0), 0);
        assert_eq!(heading(0.0), 180);
        assert_eq!(heading(90.0), 270);
        assert_eq!(heading(270.0), 90);
        assert_eq!(heading(-10.0), 170);
         
        assert_eq!(heading(179.7), 0);
        assert_eq!(heading(f32::NAN), 0);
    }

    #[test]
    fn north_sits_at_the_top_whatever_way_the_player_faces() {
         
        let north = bearing_offset(0.0, 10.0);
        assert!(north.x.abs() < 1e-4 && (north.y + 10.0).abs() < 1e-4);
        let east = bearing_offset(90.0, 10.0);
        assert!((east.x - 10.0).abs() < 1e-4 && east.y.abs() < 1e-4);
        let south = bearing_offset(180.0, 10.0);
        assert!(south.x.abs() < 1e-4 && (south.y - 10.0).abs() < 1e-4);
        let west = bearing_offset(270.0, 10.0);
        assert!((west.x + 10.0).abs() < 1e-4 && west.y.abs() < 1e-4);
        for bearing in [0.0, 45.0, 123.0, 200.0, 359.0] {
            let offset = bearing_offset(bearing, 12.0);
            assert!((offset.length() - 12.0).abs() < 1e-3);
        }
    }

    #[test]
    fn the_needle_only_points_up_when_facing_north() {
         
        for (yaw, expected) in [
            (180.0, Vec2::new(0.0, -1.0)),
            (270.0, Vec2::new(1.0, 0.0)),
            (0.0, Vec2::new(0.0, 1.0)),
            (90.0, Vec2::new(-1.0, 0.0)),
        ] {
            let dir = direction(facing_bearing(yaw));
            assert!(
                (dir - expected).length() < 1e-4,
                "facing yaw {yaw}: needle {dir:?}, expected {expected:?}"
            );
        }
    }

    #[test]
    fn the_heading_arc_spans_from_north_to_the_needle() {
        assert!(
            heading_arc_points(Pos2::ZERO, 0.0).is_empty(),
            "facing north draws no arc"
        );
        let quarter = heading_arc_points(Pos2::ZERO, std::f32::consts::FRAC_PI_2);
        assert!(quarter.len() > 2);
        let last = *quarter.last().unwrap();
        assert!((last.x - ARC_RADIUS).abs() < 1e-3 && last.y.abs() < 1e-3);
        for point in &quarter {
            assert!((point.to_vec2().length() - ARC_RADIUS).abs() < 1e-3);
        }
    }

    #[test]
    fn the_nearest_rose_point_follows_the_heading() {
        assert_eq!(nearest_rose(0.0), 0, "north");
        assert_eq!(nearest_rose(44.0), 1, "NE");
        assert_eq!(nearest_rose(90.0), 2, "east");
        assert_eq!(nearest_rose(181.0), 4, "south");
        assert_eq!(nearest_rose(359.0), 0, "wraps back to north");
    }

    #[test]
    fn the_widget_is_a_dial_plus_a_readout_row() {
        let content = Vec2::new(DIAL, DIAL + PILL_GAP + PILL_H);
        assert_eq!(SIZE, content);
        assert!(content.y > content.x, "the readout hangs under the dial");
    }

     
    struct PaintedText {
        text: String,
        color: Color32,
        center: Pos2,
    }

     
     
    fn painted(yaw: f32, label: &str) -> (Vec<PaintedText>, Vec<egui::Shape>) {
        let ctx = Context::default();
        let origin = egui::pos2(40.0, 20.0);
        let mut texts = Vec::new();
        let mut shapes = Vec::new();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 400.0))),
                ..Default::default()
            },
            |ctx| {
                let painter =
                    ctx.layer_painter(LayerId::new(Order::Background, egui::Id::new("compass")));
                draw(&painter, origin, yaw, label);
            },
        );
        for clipped in &output.shapes {
            shapes.push(clipped.shape.clone());
            if let egui::Shape::Text(text) = &clipped.shape {
                texts.push(PaintedText {
                    text: text.galley.text().to_owned(),
                    color: text.override_text_color.unwrap_or(text.fallback_color),
                    center: text.pos + text.galley.size() * 0.5,
                });
            }
        }
        (texts, shapes)
    }

     
    fn danger_tips(shapes: &[egui::Shape], center: Pos2) -> Vec<Pos2> {
        shapes
            .iter()
            .filter_map(|shape| {
                let egui::Shape::Path(path) = shape else {
                    return None;
                };
                if path.fill != theme::DANGER {
                    return None;
                }
                path.points
                    .iter()
                    .copied()
                    .max_by(|a, b| (*a - center).length().total_cmp(&(*b - center).length()))
            })
            .collect()
    }

    #[test]
    fn the_dial_paints_the_rose_heading_needle_and_readout() {
        let (texts, shapes) = painted(180.0, "North");
        let labels: Vec<&str> = texts.iter().map(|text| text.text.as_str()).collect();
        for point in ["N", "NE", "E", "SE", "S", "SW", "W", "NW"] {
            assert!(labels.contains(&point), "missing {point} on the rose");
        }
        assert!(
            labels.iter().any(|text| text.ends_with("° North")),
            "missing the heading readout, got {labels:?}"
        );
         
        let center = egui::pos2(40.0, 20.0) + Vec2::splat(RIM);
        let tips = danger_tips(&shapes, center);
        assert!(!tips.is_empty(), "the heading needle must be painted");
        let tip = tips[0];
        assert!(
            (tip.x - center.x).abs() < 1e-3 && tip.y < center.y,
            "facing north, the red needle points up: {tip:?}"
        );
         
        let circles = shapes
            .iter()
            .filter(|shape| matches!(shape, egui::Shape::Circle(_)))
            .count();
        assert!(circles >= 3, "only {circles} circles painted");
    }

    #[test]
    fn the_red_needle_follows_the_facing_not_north() {
        let center = egui::pos2(40.0, 20.0) + Vec2::splat(RIM);
        for (yaw, side) in [(270.0, "east"), (0.0, "south"), (90.0, "west")] {
            let (_, shapes) = painted(yaw, "Somewhere");
            let tips = danger_tips(&shapes, center);
            let tip = tips[0];
            let dx = tip.x - center.x;
            let dy = tip.y - center.y;
            let points_right = dx > 5.0 && dy.abs() < 2.0;
            let points_left = dx < -5.0 && dy.abs() < 2.0;
            let points_down = dy > 5.0 && dx.abs() < 2.0;
            match side {
                "east" => assert!(points_right, "facing east: needle at {tip:?}"),
                "west" => assert!(points_left, "facing west: needle at {tip:?}"),
                _ => assert!(points_down, "facing south: needle at {tip:?}"),
            }
        }
    }

    #[test]
    fn the_letter_closest_to_the_heading_is_highlighted() {
        let (texts, _) = painted(270.0, "East");
        let east = texts.iter().find(|text| text.text == "E").unwrap();
        let north = texts.iter().find(|text| text.text == "N").unwrap();
        assert_eq!(east.color, theme::TEXT);
        assert_ne!(north.color, theme::TEXT);
         
        let (texts, _) = painted(0.0, "South");
        let south = texts.iter().find(|text| text.text == "S").unwrap();
        assert_eq!(south.color, theme::TEXT);
    }

    #[test]
    fn the_cardinal_ticks_clear_the_letters() {
         
         
        let letter_reach = LABEL_RADIUS + 7.0 + 0.5;
        assert!(
            TICK_OUTER - CARDINAL_TICK > letter_reach,
            "the cardinal ticks start at {}, the letters reach {letter_reach}",
            TICK_OUTER - CARDINAL_TICK
        );
    }

    #[test]
    fn the_card_is_an_eight_point_star_under_the_needle() {
        let (_, shapes) = painted(180.0, "North");
        let center = egui::pos2(40.0, 20.0) + Vec2::splat(RIM);
        let lit = theme::with_alpha(theme::TEXT_DIM, STAR_LIT_ALPHA);
        let shaded = theme::with_alpha(theme::base(), STAR_SHADE_ALPHA);
        let (mut lit_halves, mut shaded_halves) = (0, 0);
        let mut tips = Vec::new();
        for shape in &shapes {
            let egui::Shape::Path(path) = shape else {
                continue;
            };
            if path.fill != lit && path.fill != shaded {
                continue;
            }
            if path.fill == lit {
                lit_halves += 1;
            } else {
                shaded_halves += 1;
            }
             
            let tip = path
                .points
                .iter()
                .copied()
                .max_by(|a, b| (*a - center).length().total_cmp(&(*b - center).length()))
                .expect("a star half has vertices");
            tips.push(tip);
        }
        assert_eq!(lit_halves, 8, "every point needs a lit half");
        assert_eq!(shaded_halves, 8, "every point needs a shaded half");

        let at_len = |len: f32| {
            tips.iter()
                .filter(|tip| ((**tip - center).length() - len).abs() < 1e-3)
                .count()
        };
         
        assert_eq!(at_len(STAR_CARDINAL), 8);
        assert_eq!(at_len(STAR_INTERCARDINAL), 8);

         
        let up = tips
            .iter()
            .copied()
            .find(|tip| (tip.x - center.x).abs() < 1e-3 && tip.y < center.y)
            .expect("a point straight up");
        assert!(((up - center).length() - STAR_CARDINAL).abs() < 1e-3);
        assert!(
            tips.iter()
                .copied()
                .all(|tip| (tip - center).length() <= FACE),
            "the card must not poke through the bezel"
        );
    }

    #[test]
    fn every_rose_letter_sits_on_a_face_chip() {
        let (_, shapes) = painted(200.0, "South West");
        let center = egui::pos2(40.0, 20.0) + Vec2::splat(RIM);
        let chip = theme::lerp_color(theme::base(), theme::SURFACE, 0.72);
        let chips: Vec<Pos2> = shapes
            .iter()
            .filter_map(|shape| {
                let egui::Shape::Circle(circle) = shape else {
                    return None;
                };
                (circle.fill == chip && (circle.radius - LABEL_CHIP).abs() < 1e-3)
                    .then_some(circle.center)
            })
            .collect();
        assert_eq!(chips.len(), 8, "one chip per rose letter");
        for (_, point, _) in ROSE {
            let position = center + bearing_offset(point, LABEL_RADIUS);
            assert!(
                chips.iter().any(|chip| (*chip - position).length() < 1e-3),
                "no chip under the letter at {point}°"
            );
        }
    }

    #[test]
    fn the_rose_letters_sit_where_they_point() {
         
        let (texts, _) = painted(240.0, "South West");
        let center = egui::pos2(40.0, 20.0) + Vec2::splat(RIM);
        let at = |label: &str| {
            texts
                .iter()
                .find(|text| text.text == label)
                .unwrap_or_else(|| panic!("no {label} letter painted"))
                .center
        };
        assert!(at("N").y < center.y - 10.0, "N must be above the hub");
        assert!(at("S").y > center.y + 10.0, "S must be below the hub");
        assert!(at("E").x > center.x + 10.0, "E must be right of the hub");
        assert!(at("W").x < center.x - 10.0, "W must be left of the hub");
        assert!(at("NE").x > center.x && at("NE").y < center.y);
        assert!(at("SW").x < center.x && at("SW").y > center.y);
        for point in ["N", "NE", "E", "SE", "S", "SW", "W", "NW"] {
            let radius = (at(point) - center).length();
            assert!(
                (radius - LABEL_RADIUS).abs() < 6.0,
                "{point} at radius {radius}, expected about {LABEL_RADIUS}"
            );
        }
    }

    #[test]
    fn the_longest_readout_fits_its_pill() {
        let ctx = Context::default();
         
        let text = format!("{}° South West", heading(135.0));
        let mut width = f32::INFINITY;
         
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 400.0))),
                ..Default::default()
            },
            |ctx| {
                width = ctx.fonts(|fonts| {
                    fonts
                        .layout_no_wrap(text.clone(), FontId::proportional(9.5), theme::TEXT)
                        .size()
                        .x
                });
            },
        );
        assert!(
            width <= DIAL - 8.0,
            "{text:?} is {width} px wide, the pill only has {}",
            DIAL - 8.0
        );
    }

     
     
     
    #[test]
    fn the_compass_preview_renders_one_dial_per_facing() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let facings = [
            (180.0, "North"),
            (270.0, "East"),
            (0.0, "South"),
            (90.0, "West"),
        ];
        let slot = DIAL + 16.0;
        let width = (slot * facings.len() as f32) as usize;
        let height = (SIZE.y + 16.0) as usize;
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        for frame in 0..4 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    let painter =
                        ctx.layer_painter(LayerId::new(Order::Background, egui::Id::new("dials")));
                    for (index, (yaw, label)) in facings.iter().enumerate() {
                        let origin = egui::pos2(8.0 + slot * index as f32, 8.0);
                        draw(&painter, origin, *yaw, label);
                    }
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
        let backdrop = [0.055, 0.055, 0.07, 1.0];
        let image = rasterize(primitives, &textures, width, height, backdrop);
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/compass-preview.png");
        image.save(&path).expect("the compass preview png saves");
        let lit = image
            .pixels()
            .filter(|pixel| {
                pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32 > 120 && pixel[3] > 128
            })
            .count();
        let red = image
            .pixels()
            .filter(|pixel| pixel[0] > 180 && pixel[1] < 120 && pixel[2] < 120)
            .count();
        println!("compass preview: {lit} lit, {red} red pixels -> {path:?}");
        assert!(
            lit > 4_000,
            "the dials are barely painted ({lit} lit pixels)"
        );
        assert!(
            red > 250,
            "the heading marker is missing ({red} red pixels)"
        );
    }

    #[test]
    fn the_readout_name_and_degrees_follow_the_yaw() {
        let (texts, _) = painted(135.0, "North West");
        assert!(
            texts.iter().any(|text| text.text == "315° North West"),
            "got {:?}",
            texts.iter().map(|text| &text.text).collect::<Vec<_>>()
        );
    }
}
