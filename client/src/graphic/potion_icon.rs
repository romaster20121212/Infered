 
 
 
 
 
 
 

use egui::{Color32, Painter, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

use crate::graphic::theme;

 
 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Glyph {
    Bolt,
    Chevrons,
    Pickaxe,
    PickaxeWorn,
    Sword,
    BrokenSword,
    Heart,
    HeartPulse,
    HeartUp,
    HeartCrack,
    ShieldHeart,
    Shield,
    Droplet,
    Skull,
    Drumstick,
    DrumstickSweet,
    Spiral,
    Flame,
    Bubbles,
    EyeSlash,
    Ghost,
    MoonStar,
    Moon,
    Sparkle,
    ArrowUp,
    Jump,
    Feather,
    Wave,
    Conduit,
    Clover,
    CloverSlash,
    Omen,
    OmenSkull,
    OmenSpiral,
    Star,
    Wind,
    Web,
    Blob,
    Bug,
    Flask,
}

 
pub(crate) fn color(id: &str) -> Color32 {
    let rgb = |r: u8, g: u8, b: u8| Color32::from_rgb(r, g, b);
    match short(id) {
        "speed" => rgb(79, 195, 247),
        "slowness" => rgb(144, 164, 174),
        "haste" => rgb(255, 213, 79),
        "mining_fatigue" => rgb(161, 136, 127),
        "strength" => rgb(255, 112, 67),
        "instant_health" => rgb(255, 92, 138),
        "instant_damage" => rgb(198, 40, 40),
        "jump_boost" => rgb(139, 195, 74),
        "nausea" => rgb(156, 122, 192),
        "regeneration" => rgb(240, 98, 146),
        "resistance" => rgb(92, 156, 230),
        "fire_resistance" => rgb(255, 167, 38),
        "water_breathing" => rgb(77, 208, 225),
        "invisibility" => rgb(176, 190, 197),
        "blindness" => rgb(130, 130, 130),
        "night_vision" => rgb(121, 134, 203),
        "hunger" => rgb(208, 139, 74),
        "weakness" => rgb(158, 158, 158),
        "poison" => rgb(102, 187, 106),
        "wither" => rgb(120, 120, 120),
        "health_boost" => rgb(255, 138, 128),
        "absorption" => rgb(255, 209, 128),
        "saturation" => rgb(240, 184, 90),
        "glowing" => rgb(255, 224, 130),
        "levitation" => rgb(206, 147, 216),
        "luck" => rgb(105, 240, 174),
        "unluck" => rgb(141, 110, 99),
        "slow_falling" => rgb(179, 229, 252),
        "conduit_power" => rgb(38, 198, 218),
        "dolphins_grace" => rgb(79, 195, 247),
        "bad_omen" => rgb(142, 124, 195),
        "hero_of_the_village" => rgb(255, 213, 79),
        "darkness" => rgb(104, 104, 150),
        "trial_omen" => rgb(126, 156, 216),
        "raid_omen" => rgb(160, 90, 90),
        "wind_charged" => rgb(144, 202, 249),
        "weaving" => rgb(179, 157, 219),
        "oozing" => rgb(156, 204, 101),
        "infested" => rgb(165, 214, 167),
        _ => rgb(148, 163, 184),
    }
}

 
 
pub(crate) fn glyph(id: &str) -> Glyph {
    match short(id) {
        "speed" => Glyph::Bolt,
        "slowness" => Glyph::Chevrons,
        "haste" => Glyph::Pickaxe,
        "mining_fatigue" => Glyph::PickaxeWorn,
        "strength" => Glyph::Sword,
        "weakness" => Glyph::BrokenSword,
        "instant_health" => Glyph::Heart,
        "instant_damage" => Glyph::HeartCrack,
        "regeneration" => Glyph::HeartPulse,
        "health_boost" => Glyph::HeartUp,
        "absorption" => Glyph::ShieldHeart,
        "resistance" => Glyph::Shield,
        "poison" => Glyph::Droplet,
        "wither" => Glyph::Skull,
        "hunger" => Glyph::Drumstick,
        "saturation" => Glyph::DrumstickSweet,
        "nausea" => Glyph::Spiral,
        "fire_resistance" => Glyph::Flame,
        "water_breathing" => Glyph::Bubbles,
        "invisibility" => Glyph::Ghost,
        "blindness" => Glyph::EyeSlash,
        "night_vision" => Glyph::MoonStar,
        "darkness" => Glyph::Moon,
        "glowing" => Glyph::Sparkle,
        "jump_boost" => Glyph::Jump,
        "levitation" => Glyph::ArrowUp,
        "slow_falling" => Glyph::Feather,
        "conduit_power" => Glyph::Conduit,
        "dolphins_grace" => Glyph::Wave,
        "luck" => Glyph::Clover,
        "unluck" => Glyph::CloverSlash,
        "bad_omen" => Glyph::Omen,
        "raid_omen" => Glyph::OmenSkull,
        "trial_omen" => Glyph::OmenSpiral,
        "hero_of_the_village" => Glyph::Star,
        "wind_charged" => Glyph::Wind,
        "weaving" => Glyph::Web,
        "oozing" => Glyph::Blob,
        "infested" => Glyph::Bug,
        _ => Glyph::Flask,
    }
}

fn short(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}

 
 
 
pub(crate) fn paint(painter: &Painter, rect: Rect, id: &str, alpha: f32) {
    let accent = color(id);
    let alpha = alpha.clamp(0.0, 1.0);
    let round = Rounding::same((rect.width() * 0.30).max(2.0));

    painter.rect_filled(
        rect.expand(rect.width() * 0.12),
        round,
        theme::with_alpha(accent, 0.10 * alpha),
    );
    painter.rect_filled(rect, round, theme::with_alpha(accent, 0.17 * alpha));
    painter.rect_stroke(
        rect,
        round,
        Stroke::new(0.9_f32, theme::with_alpha(accent, 0.46 * alpha)),
    );
    painter.line_segment(
        [
            Pos2::new(
                rect.left() + rect.width() * 0.28,
                rect.top() + rect.height() * 0.13,
            ),
            Pos2::new(
                rect.right() - rect.width() * 0.28,
                rect.top() + rect.height() * 0.13,
            ),
        ],
        Stroke::new(0.7_f32, Color32::from_white_alpha((26.0 * alpha) as u8)),
    );

    let glyph_rect = rect.shrink(rect.width() * 0.235);
    let ink = theme::with_alpha(theme::lerp_color(accent, Color32::WHITE, 0.35), alpha);
    draw(painter, glyph_rect, glyph(id), ink, alpha);
}

fn ink_for(alpha: f32) -> Color32 {
    Color32::from_black_alpha((185.0 * alpha.clamp(0.0, 1.0)) as u8)
}

fn point(rect: Rect, x: f32, y: f32) -> Pos2 {
    Pos2::new(
        rect.left() + rect.width() * x,
        rect.top() + rect.height() * y,
    )
}

fn width(rect: Rect, factor: f32) -> f32 {
    (rect.width() * factor).max(0.8)
}

fn line(painter: &Painter, rect: Rect, a: (f32, f32), b: (f32, f32), w: f32, color: Color32) {
    painter.line_segment(
        [point(rect, a.0, a.1), point(rect, b.0, b.1)],
        Stroke::new(width(rect, w), color),
    );
}

fn poly(painter: &Painter, rect: Rect, pts: &[(f32, f32)], color: Color32) {
    let points = pts.iter().map(|&(x, y)| point(rect, x, y)).collect();
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

fn ring(painter: &Painter, rect: Rect, center: (f32, f32), radius: f32, w: f32, color: Color32) {
    painter.circle_stroke(
        point(rect, center.0, center.1),
        rect.width() * radius,
        Stroke::new(width(rect, w), color),
    );
}

fn dot(painter: &Painter, rect: Rect, center: (f32, f32), radius: f32, color: Color32) {
    painter.circle_filled(
        point(rect, center.0, center.1),
        rect.width() * radius,
        color,
    );
}

fn path(painter: &Painter, rect: Rect, pts: &[(f32, f32)], w: f32, color: Color32, closed: bool) {
    for pair in pts.windows(2) {
        line(painter, rect, pair[0], pair[1], w, color);
    }
    if closed {
        if let (Some(first), Some(last)) = (pts.first(), pts.last()) {
            line(painter, rect, *last, *first, w, color);
        }
    }
}

fn heart(painter: &Painter, rect: Rect, center: (f32, f32), size: f32, color: Color32) {
    let (cx, cy) = center;
    dot(
        painter,
        rect,
        (cx - size * 0.36, cy - size * 0.16),
        size * 0.4,
        color,
    );
    dot(
        painter,
        rect,
        (cx + size * 0.36, cy - size * 0.16),
        size * 0.4,
        color,
    );
    poly(
        painter,
        rect,
        &[
            (cx - size * 0.74, cy - size * 0.1),
            (cx + size * 0.74, cy - size * 0.1),
            (cx, cy + size * 0.82),
        ],
        color,
    );
}

fn star(
    painter: &Painter,
    rect: Rect,
    center: (f32, f32),
    outer: f32,
    inner: f32,
    points: usize,
    color: Color32,
) {
    let c = point(rect, center.0, center.1);
    let mut vertices = Vec::with_capacity(points * 2);
    for i in 0..points * 2 {
        let radius = if i % 2 == 0 { outer } else { inner } * rect.width();
        let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / points as f32;
        vertices.push(c + Vec2::new(angle.cos() * radius, angle.sin() * radius));
    }
    for i in 0..vertices.len() {
        painter.add(Shape::convex_polygon(
            vec![c, vertices[i], vertices[(i + 1) % vertices.len()]],
            color,
            Stroke::NONE,
        ));
    }
}

fn eye(painter: &Painter, rect: Rect, ink: Color32) {
    let mut pts = Vec::new();
    for i in 0..=10 {
        let t = i as f32 / 10.0;
        pts.push((
            0.06 + 0.88 * t,
            0.5 - (t * std::f32::consts::PI).sin() * 0.36,
        ));
    }
    for i in 0..=10 {
        let t = (10 - i) as f32 / 10.0;
        pts.push((
            0.06 + 0.88 * t,
            0.5 + (t * std::f32::consts::PI).sin() * 0.36,
        ));
    }
    path(painter, rect, &pts, 0.09, ink, true);
    dot(painter, rect, (0.5, 0.5), 0.16, ink);
}

fn clover(painter: &Painter, rect: Rect, color: Color32) {
    for (x, y) in [(0.36, 0.36), (0.64, 0.36), (0.36, 0.64), (0.64, 0.64)] {
        dot(painter, rect, (x, y), 0.2, color);
    }
    line(painter, rect, (0.5, 0.62), (0.66, 0.96), 0.07, color);
}

fn pickaxe(painter: &Painter, rect: Rect, ink: Color32) {
    line(painter, rect, (0.18, 0.90), (0.60, 0.42), 0.12, ink);
    path(
        painter,
        rect,
        &[
            (0.34, 0.36),
            (0.42, 0.12),
            (0.64, 0.03),
            (0.86, 0.10),
            (0.95, 0.28),
        ],
        0.17,
        ink,
        false,
    );
}

fn drumstick(painter: &Painter, rect: Rect, ink: Color32) {
    dot(painter, rect, (0.62, 0.32), 0.27, ink);
    dot(painter, rect, (0.73, 0.44), 0.17, ink);
    line(painter, rect, (0.44, 0.52), (0.16, 0.80), 0.11, ink);
    dot(painter, rect, (0.12, 0.86), 0.10, ink);
    dot(painter, rect, (0.22, 0.94), 0.09, ink);
}

fn omen_frame(painter: &Painter, rect: Rect, ink: Color32) {
    path(
        painter,
        rect,
        &[(0.5, 0.05), (0.95, 0.92), (0.05, 0.92)],
        0.09,
        ink,
        true,
    );
}

fn draw(painter: &Painter, rect: Rect, glyph: Glyph, ink: Color32, alpha: f32) {
    let dark = ink_for(alpha);
    match glyph {
        Glyph::Bolt => poly(
            painter,
            rect,
            &[
                (0.60, 0.02),
                (0.18, 0.57),
                (0.45, 0.57),
                (0.33, 0.98),
                (0.82, 0.42),
                (0.53, 0.42),
            ],
            ink,
        ),
        Glyph::Chevrons => {
            path(
                painter,
                rect,
                &[(0.18, 0.30), (0.5, 0.58), (0.82, 0.30)],
                0.12,
                ink,
                false,
            );
            path(
                painter,
                rect,
                &[(0.18, 0.54), (0.5, 0.82), (0.82, 0.54)],
                0.12,
                ink,
                false,
            );
        }
        Glyph::Pickaxe => pickaxe(painter, rect, ink),
        Glyph::PickaxeWorn => {
            pickaxe(painter, rect, ink);
             
             
            path(
                painter,
                rect,
                &[(0.40, 0.10), (0.50, 0.22), (0.44, 0.30)],
                0.055,
                dark,
                false,
            );
            dot(painter, rect, (0.90, 0.40), 0.045, dark);
        }
        Glyph::Sword => {
            poly(
                painter,
                rect,
                &[(0.86, 0.04), (0.97, 0.17), (0.46, 0.68), (0.33, 0.55)],
                ink,
            );
            line(painter, rect, (0.22, 0.60), (0.42, 0.80), 0.11, ink);
            line(painter, rect, (0.16, 0.78), (0.03, 0.95), 0.10, ink);
            dot(painter, rect, (0.03, 0.96), 0.06, ink);
        }
        Glyph::BrokenSword => {
            poly(
                painter,
                rect,
                &[(0.86, 0.04), (0.97, 0.17), (0.64, 0.49), (0.53, 0.38)],
                ink,
            );
            poly(
                painter,
                rect,
                &[(0.47, 0.58), (0.57, 0.69), (0.40, 0.73), (0.33, 0.63)],
                ink,
            );
            line(painter, rect, (0.22, 0.60), (0.42, 0.80), 0.11, ink);
            line(painter, rect, (0.16, 0.78), (0.03, 0.95), 0.10, ink);
        }
        Glyph::Heart => {
            heart(painter, rect, (0.5, 0.52), 0.46, ink);
            line(painter, rect, (0.5, 0.40), (0.5, 0.66), 0.11, dark);
            line(painter, rect, (0.37, 0.53), (0.63, 0.53), 0.11, dark);
        }
        Glyph::HeartPulse => {
            heart(painter, rect, (0.5, 0.52), 0.46, ink);
            path(
                painter,
                rect,
                &[
                    (0.22, 0.52),
                    (0.38, 0.52),
                    (0.45, 0.38),
                    (0.55, 0.68),
                    (0.62, 0.52),
                    (0.80, 0.52),
                ],
                0.08,
                dark,
                false,
            );
        }
        Glyph::HeartUp => {
            heart(painter, rect, (0.5, 0.52), 0.46, ink);
            path(
                painter,
                rect,
                &[(0.36, 0.58), (0.5, 0.42), (0.64, 0.58)],
                0.09,
                dark,
                false,
            );
        }
        Glyph::HeartCrack => {
            heart(painter, rect, (0.5, 0.52), 0.46, ink);
            path(
                painter,
                rect,
                &[
                    (0.5, 0.24),
                    (0.42, 0.40),
                    (0.56, 0.50),
                    (0.44, 0.62),
                    (0.50, 0.78),
                ],
                0.08,
                dark,
                false,
            );
        }
        Glyph::ShieldHeart => {
            poly(
                painter,
                rect,
                &[
                    (0.5, 0.03),
                    (0.90, 0.20),
                    (0.84, 0.62),
                    (0.5, 0.96),
                    (0.16, 0.62),
                    (0.10, 0.20),
                ],
                ink,
            );
            heart(painter, rect, (0.5, 0.5), 0.26, dark);
        }
        Glyph::Shield => {
            poly(
                painter,
                rect,
                &[
                    (0.5, 0.03),
                    (0.90, 0.20),
                    (0.84, 0.62),
                    (0.5, 0.96),
                    (0.16, 0.62),
                    (0.10, 0.20),
                ],
                ink,
            );
            path(painter, rect, &[(0.5, 0.2), (0.5, 0.72)], 0.08, dark, false);
        }
        Glyph::Droplet => {
            poly(
                painter,
                rect,
                &[(0.5, 0.02), (0.84, 0.56), (0.16, 0.56)],
                ink,
            );
            dot(painter, rect, (0.5, 0.66), 0.32, ink);
            dot(painter, rect, (0.39, 0.62), 0.07, dark);
            dot(painter, rect, (0.6, 0.74), 0.05, dark);
        }
        Glyph::Skull => {
            dot(painter, rect, (0.5, 0.40), 0.35, ink);
            painter.rect_filled(
                Rect::from_min_max(point(rect, 0.34, 0.60), point(rect, 0.66, 0.88)),
                Rounding::same(rect.width() * 0.08),
                ink,
            );
            dot(painter, rect, (0.38, 0.37), 0.09, dark);
            dot(painter, rect, (0.62, 0.37), 0.09, dark);
            poly(
                painter,
                rect,
                &[(0.5, 0.46), (0.56, 0.58), (0.44, 0.58)],
                dark,
            );
            line(painter, rect, (0.42, 0.70), (0.42, 0.86), 0.05, dark);
            line(painter, rect, (0.5, 0.70), (0.5, 0.86), 0.05, dark);
            line(painter, rect, (0.58, 0.70), (0.58, 0.86), 0.05, dark);
        }
        Glyph::Drumstick => drumstick(painter, rect, ink),
        Glyph::DrumstickSweet => {
            drumstick(painter, rect, ink);
            star(painter, rect, (0.86, 0.18), 0.12, 0.04, 4, dark);
        }
        Glyph::Spiral => {
            let mut pts = Vec::new();
            for i in 0..=36 {
                let t = i as f32 / 36.0;
                let angle = t * std::f32::consts::TAU * 2.1;
                let radius = 0.04 + 0.42 * t;
                pts.push((0.5 + angle.cos() * radius, 0.5 + angle.sin() * radius));
            }
            path(painter, rect, &pts, 0.09, ink, false);
        }
        Glyph::Flame => {
             
             
            poly(
                painter,
                rect,
                &[
                    (0.5, 0.03),
                    (0.80, 0.34),
                    (0.92, 0.62),
                    (0.68, 0.94),
                    (0.5, 0.72),
                ],
                ink,
            );
            poly(
                painter,
                rect,
                &[
                    (0.5, 0.03),
                    (0.24, 0.34),
                    (0.08, 0.62),
                    (0.32, 0.94),
                    (0.5, 0.72),
                ],
                ink,
            );
            poly(
                painter,
                rect,
                &[(0.5, 0.70), (0.32, 0.94), (0.68, 0.94)],
                ink,
            );
            poly(
                painter,
                rect,
                &[(0.5, 0.40), (0.64, 0.66), (0.5, 0.84), (0.36, 0.66)],
                dark,
            );
        }
        Glyph::Bubbles => {
            dot(painter, rect, (0.36, 0.62), 0.19, ink);
            dot(painter, rect, (0.66, 0.42), 0.13, ink);
            dot(painter, rect, (0.68, 0.74), 0.09, ink);
            ring(painter, rect, (0.30, 0.26), 0.11, 0.06, ink);
        }
        Glyph::EyeSlash => {
            eye(painter, rect, ink);
            line(painter, rect, (0.12, 0.10), (0.88, 0.90), 0.10, dark);
        }
        Glyph::Ghost => {
            dot(painter, rect, (0.5, 0.36), 0.24, ink);
            path(
                painter,
                rect,
                &[
                    (0.26, 0.36),
                    (0.26, 0.84),
                    (0.38, 0.74),
                    (0.5, 0.86),
                    (0.62, 0.74),
                    (0.74, 0.84),
                    (0.74, 0.36),
                ],
                0.08,
                ink,
                false,
            );
            dot(painter, rect, (0.42, 0.36), 0.055, dark);
            dot(painter, rect, (0.58, 0.36), 0.055, dark);
        }
        Glyph::MoonStar => {
            dot(painter, rect, (0.44, 0.5), 0.40, ink);
            dot(painter, rect, (0.62, 0.40), 0.32, dark);
            star(painter, rect, (0.78, 0.22), 0.16, 0.05, 4, ink);
        }
        Glyph::Moon => {
            dot(painter, rect, (0.44, 0.5), 0.40, ink);
            dot(painter, rect, (0.62, 0.40), 0.32, dark);
            dot(painter, rect, (0.76, 0.74), 0.05, ink);
            dot(painter, rect, (0.86, 0.58), 0.035, ink);
        }
        Glyph::Sparkle => {
            star(painter, rect, (0.5, 0.5), 0.48, 0.11, 4, ink);
            dot(painter, rect, (0.84, 0.16), 0.07, ink);
            dot(painter, rect, (0.14, 0.8), 0.05, ink);
        }
        Glyph::ArrowUp => {
             
             
            poly(
                painter,
                rect,
                &[(0.5, 0.03), (0.16, 0.43), (0.84, 0.43)],
                ink,
            );
            poly(
                painter,
                rect,
                &[(0.39, 0.38), (0.61, 0.38), (0.61, 0.78), (0.39, 0.78)],
                ink,
            );
            dot(painter, rect, (0.30, 0.90), 0.05, ink);
            dot(painter, rect, (0.56, 0.94), 0.04, ink);
            dot(painter, rect, (0.76, 0.86), 0.035, ink);
        }
        Glyph::Jump => {
            poly(
                painter,
                rect,
                &[(0.5, 0.03), (0.20, 0.42), (0.80, 0.42)],
                ink,
            );
            poly(
                painter,
                rect,
                &[(0.40, 0.38), (0.60, 0.38), (0.60, 0.82), (0.40, 0.82)],
                ink,
            );
            line(painter, rect, (0.10, 0.94), (0.90, 0.94), 0.09, ink);
        }
        Glyph::Feather => {
             
             
            let center = (0.47, 0.47);
            let (dx, dy) = (0.30, -0.30);
            let (mx, my) = (0.11, 0.11);
            let mut leaf = Vec::new();
            for step in 0..14 {
                let angle = step as f32 * std::f32::consts::TAU / 14.0;
                let (sin, cos) = angle.sin_cos();
                leaf.push((
                    center.0 + dx * cos + mx * sin,
                    center.1 + dy * cos + my * sin,
                ));
            }
            poly(painter, rect, &leaf, ink);
            line(painter, rect, (0.90, 0.06), (0.08, 0.92), 0.055, dark);
            for step in 0..3 {
                let t = 0.26 + step as f32 * 0.16;
                let (x, y) = (0.82 - t * 0.72, 0.18 + t * 0.72);
                line(painter, rect, (x, y), (x - 0.12, y - 0.05), 0.04, dark);
                line(painter, rect, (x, y), (x + 0.02, y + 0.12), 0.04, dark);
            }
        }
        Glyph::Wave => {
            for (i, base) in [0.38_f32, 0.64].into_iter().enumerate() {
                let mut pts = Vec::new();
                for step in 0..=12 {
                    let t = step as f32 / 12.0;
                    let phase = t * std::f32::consts::TAU + i as f32 * 0.9;
                    pts.push((0.06 + 0.88 * t, base + phase.sin() * 0.10));
                }
                path(painter, rect, &pts, 0.09, ink, false);
            }
        }
        Glyph::Conduit => {
            ring(painter, rect, (0.5, 0.5), 0.42, 0.06, ink);
            ring(painter, rect, (0.5, 0.5), 0.22, 0.11, ink);
            dot(painter, rect, (0.5, 0.5), 0.08, ink);
            for (dx, dy) in [(0.0, -0.48), (0.0, 0.48), (-0.48, 0.0), (0.48, 0.0)] {
                line(
                    painter,
                    rect,
                    (0.5 + dx * 0.72, 0.5 + dy * 0.72),
                    (0.5 + dx, 0.5 + dy),
                    0.06,
                    ink,
                );
            }
        }
        Glyph::Clover => clover(painter, rect, ink),
        Glyph::CloverSlash => {
            clover(painter, rect, ink);
            line(painter, rect, (0.12, 0.10), (0.88, 0.90), 0.10, dark);
        }
        Glyph::Omen => {
            omen_frame(painter, rect, ink);
            line(painter, rect, (0.5, 0.34), (0.5, 0.64), 0.11, ink);
            dot(painter, rect, (0.5, 0.78), 0.06, ink);
        }
        Glyph::OmenSkull => {
            omen_frame(painter, rect, ink);
            dot(painter, rect, (0.5, 0.58), 0.15, ink);
            painter.rect_filled(
                Rect::from_min_max(point(rect, 0.42, 0.68), point(rect, 0.58, 0.82)),
                Rounding::same(rect.width() * 0.03),
                ink,
            );
            dot(painter, rect, (0.45, 0.56), 0.035, dark);
            dot(painter, rect, (0.55, 0.56), 0.035, dark);
        }
        Glyph::OmenSpiral => {
            omen_frame(painter, rect, ink);
            let mut pts = Vec::new();
            for step in 0..=18 {
                let t = step as f32 / 18.0;
                let angle = t * std::f32::consts::TAU * 1.8;
                let radius = 0.02 + 0.12 * t;
                pts.push((0.5 + angle.cos() * radius, 0.62 + angle.sin() * radius));
            }
            path(painter, rect, &pts, 0.055, ink, false);
        }
        Glyph::Star => star(painter, rect, (0.5, 0.5), 0.48, 0.20, 5, ink),
        Glyph::Wind => {
            for (y, from, to) in [(0.26, 0.10, 0.72), (0.5, 0.10, 0.86), (0.74, 0.10, 0.62)] {
                line(painter, rect, (from, y), (to, y), 0.09, ink);
                poly(
                    painter,
                    rect,
                    &[(to, y - 0.12), (to + 0.14, y), (to, y + 0.12)],
                    ink,
                );
            }
        }
        Glyph::Web => {
            for step in 0..8 {
                let angle = step as f32 * std::f32::consts::TAU / 8.0;
                line(
                    painter,
                    rect,
                    (0.5, 0.5),
                    (0.5 + angle.cos() * 0.46, 0.5 + angle.sin() * 0.46),
                    0.05,
                    ink,
                );
            }
            for radius in [0.2_f32, 0.38] {
                let mut pts = Vec::new();
                for step in 0..8 {
                    let angle = step as f32 * std::f32::consts::TAU / 8.0;
                    pts.push((0.5 + angle.cos() * radius, 0.5 + angle.sin() * radius));
                }
                path(painter, rect, &pts, 0.05, ink, true);
            }
        }
        Glyph::Blob => {
            dot(painter, rect, (0.5, 0.48), 0.30, ink);
            dot(painter, rect, (0.30, 0.70), 0.14, ink);
            dot(painter, rect, (0.5, 0.76), 0.15, ink);
            dot(painter, rect, (0.7, 0.70), 0.14, ink);
            dot(painter, rect, (0.5, 0.95), 0.06, ink);
        }
        Glyph::Bug => {
            dot(painter, rect, (0.5, 0.30), 0.13, ink);
            dot(painter, rect, (0.5, 0.58), 0.22, ink);
            for (y, spread) in [(0.38, 0.0), (0.55, 0.1), (0.72, 0.0)] {
                line(
                    painter,
                    rect,
                    (0.30, y),
                    (0.16 - spread, y + 0.06),
                    0.05,
                    ink,
                );
                line(
                    painter,
                    rect,
                    (0.70, y),
                    (0.84 + spread, y + 0.06),
                    0.05,
                    ink,
                );
            }
            line(painter, rect, (0.44, 0.19), (0.38, 0.06), 0.05, ink);
            line(painter, rect, (0.56, 0.19), (0.62, 0.06), 0.05, ink);
        }
        Glyph::Flask => {
            painter.rect_filled(
                Rect::from_min_max(point(rect, 0.40, 0.16), point(rect, 0.60, 0.44)),
                Rounding::ZERO,
                ink,
            );
            painter.rect_filled(
                Rect::from_min_max(point(rect, 0.32, 0.02), point(rect, 0.68, 0.14)),
                Rounding::same(rect.width() * 0.04),
                ink,
            );
            dot(painter, rect, (0.5, 0.62), 0.33, ink);
            line(painter, rect, (0.22, 0.58), (0.78, 0.58), 0.07, dark);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_gameplay_effect_has_a_dedicated_glyph() {
        let known = [
            "speed",
            "slowness",
            "haste",
            "mining_fatigue",
            "strength",
            "instant_health",
            "instant_damage",
            "jump_boost",
            "nausea",
            "regeneration",
            "resistance",
            "fire_resistance",
            "water_breathing",
            "invisibility",
            "blindness",
            "night_vision",
            "hunger",
            "weakness",
            "poison",
            "wither",
            "health_boost",
            "absorption",
            "saturation",
            "glowing",
            "levitation",
            "luck",
            "unluck",
            "slow_falling",
            "conduit_power",
            "dolphins_grace",
            "bad_omen",
            "hero_of_the_village",
            "darkness",
            "trial_omen",
            "raid_omen",
            "wind_charged",
            "weaving",
            "oozing",
            "infested",
        ];
        for id in known {
            assert_ne!(glyph(id), Glyph::Flask, "{id} fell back to the bottle");
            assert_ne!(
                color(id),
                color("unknown_modded_effect"),
                "{id} has no accent"
            );
        }
        assert_eq!(glyph("minecraft:strength"), Glyph::Sword);
        assert_eq!(glyph("some_modded_effect"), Glyph::Flask);

         
        assert_ne!(glyph("haste"), glyph("mining_fatigue"));
        assert_ne!(glyph("hunger"), glyph("saturation"));
        assert_ne!(glyph("bad_omen"), glyph("raid_omen"));
        assert_ne!(glyph("raid_omen"), glyph("trial_omen"));
        assert_ne!(glyph("bad_omen"), glyph("trial_omen"));
    }

     
    #[test]
    fn the_icon_sheet_renders_every_glyph() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let ids = [
            "speed",
            "slowness",
            "haste",
            "mining_fatigue",
            "strength",
            "weakness",
            "instant_health",
            "regeneration",
            "health_boost",
            "absorption",
            "resistance",
            "poison",
            "wither",
            "hunger",
            "saturation",
            "nausea",
            "fire_resistance",
            "water_breathing",
            "invisibility",
            "blindness",
            "night_vision",
            "darkness",
            "glowing",
            "jump_boost",
            "levitation",
            "slow_falling",
            "conduit_power",
            "dolphins_grace",
            "luck",
            "unluck",
            "bad_omen",
            "raid_omen",
            "trial_omen",
            "hero_of_the_village",
            "wind_charged",
            "weaving",
            "oozing",
            "infested",
            "unknown_effect",
        ];
        let columns = 6usize;
        let cell = (84.0_f32, 72.0_f32);
        let rows = ids.len().div_ceil(columns);
        let (width, height) = (
            (columns as f32 * cell.0) as usize,
            (rows as f32 * cell.1 + 12.0) as usize,
        );
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = egui::Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
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
                        egui::Id::new("icon_sheet"),
                    ));
                    for (index, id) in ids.iter().enumerate() {
                        let column = index % columns;
                        let row = index / columns;
                        let tile = Rect::from_min_size(
                            Pos2::new(column as f32 * cell.0 + 22.0, row as f32 * cell.1 + 16.0),
                            Vec2::splat(36.0),
                        );
                        paint(&painter, tile, id, 1.0);
                        painter.text(
                            Pos2::new(tile.center().x, tile.bottom() + 9.0),
                            egui::Align2::CENTER_CENTER,
                            id,
                            egui::FontId::proportional(8.0),
                            Color32::from_rgb(170, 176, 190),
                        );
                    }
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
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
            .join("../target/potion-icons-preview.png");
        image.save(&path).expect("the icon sheet saves");
        assert_eq!(image.width() as usize, width);
    }

    #[test]
    fn effect_colours_are_distinct_per_family() {
        let mut seen = std::collections::HashSet::new();
        for id in [
            "speed",
            "strength",
            "regeneration",
            "poison",
            "fire_resistance",
            "water_breathing",
            "night_vision",
            "luck",
            "wind_charged",
        ] {
            assert!(seen.insert(color(id)), "{id} repeats another accent");
        }
    }
}
