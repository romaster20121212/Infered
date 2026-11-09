 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::esp::{color_setting, toggle_setting, View, V3};
use crate::graphic::theme;
use crate::mapping::entity::player::Player;
use crate::mapping::entity::EntityRef;
use crate::mapping::MappedObject;
use crate::module::player::autototem::roll;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::net::packet::{Packet, PacketAction};
use crate::state::{client, minecraft};
use egui::{vec2, Color32, Painter, Pos2, Shape, Stroke};
use std::f64::consts::{PI, TAU};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const DEFAULT_DURATION: f32 = 0.7;
const DEFAULT_SIZE: f32 = 1.0;
const DEFAULT_INTENSITY: f32 = 1.0;
const DEFAULT_COLOR: Color32 = Color32::from_rgb(255, 35, 48);
 
const MAX_BURSTS: usize = 20;
 
const FALLBACK_WIDTH: f64 = 0.6;
const FALLBACK_HEIGHT: f64 = 1.8;
 
const BOX_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (2, 3),
    (4, 5),
    (6, 7),
    (0, 2),
    (1, 3),
    (4, 6),
    (5, 7),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HitStyle {
     
    Slash,
     
    Impact,
     
    Shockwave,
     
    Shatter,
     
    Vortex,
     
    Sigil,
     
    Nova,
     
    Cross,
}

impl HitStyle {
    const OPTIONS: [&'static str; 8] = [
        "Slash",
        "Impact",
        "Shockwave",
        "Shatter",
        "Vortex",
        "Sigil",
        "Nova",
        "Cross",
    ];
    const ALL: [Self; 8] = [
        Self::Slash,
        Self::Impact,
        Self::Shockwave,
        Self::Shatter,
        Self::Vortex,
        Self::Sigil,
        Self::Nova,
        Self::Cross,
    ];

     
    pub(crate) fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Impact,
            2 => Self::Shockwave,
            3 => Self::Shatter,
            4 => Self::Vortex,
            5 => Self::Sigil,
            6 => Self::Nova,
            7 => Self::Cross,
            _ => Self::Slash,
        }
    }
}

 
 
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HitConfig {
    pub(crate) enabled: bool,
    style: HitStyle,
    duration: Duration,
    size: f64,
    intensity: f64,
    glow: bool,
    players_only: bool,
    color: Color32,
    rainbow: bool,
}

impl Default for HitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            style: HitStyle::Slash,
            duration: Duration::from_secs_f32(DEFAULT_DURATION),
            size: DEFAULT_SIZE as f64,
            intensity: DEFAULT_INTENSITY as f64,
            glow: true,
            players_only: false,
            color: DEFAULT_COLOR,
            rainbow: false,
        }
    }
}

impl HitConfig {
    fn from_data(data: &ModuleData) -> Self {
        let style_index = match data.get_setting("Style") {
            Some(ModuleSetting::Choice { value, .. }) => *value,
            _ => 0,
        };
        Self {
            enabled: data.enabled,
            style: HitStyle::from_index(style_index),
            duration: Duration::from_secs_f32(bounded(
                data,
                "Duration",
                DEFAULT_DURATION,
                0.25,
                2.0,
            )),
            size: bounded(data, "Size", DEFAULT_SIZE, 0.4, 2.5) as f64,
            intensity: bounded(data, "Intensity", DEFAULT_INTENSITY, 0.4, 2.0) as f64,
            glow: toggle_setting(data, "Glow", true),
            players_only: toggle_setting(data, "Players Only", false),
            color: color_setting(data, "Color", DEFAULT_COLOR),
            rainbow: toggle_setting(data, "Rainbow", false),
        }
    }
}

 
fn bounded(data: &ModuleData, name: &str, fallback: f32, min: f32, max: f32) -> f32 {
    let value = data
        .get_setting(name)
        .and_then(ModuleSetting::get_slider_value)
        .unwrap_or(fallback);
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

 
#[derive(Debug, Clone)]
struct HitBurst {
    feet: V3,
    width: f64,
    height: f64,
    style: HitStyle,
    color: Color32,
    rainbow: bool,
    seed: f32,
    spawned: Instant,
    duration: Duration,
    size: f64,
    intensity: f64,
    glow: bool,
}

impl HitBurst {
    fn centre(&self) -> V3 {
        V3 {
            x: self.feet.x,
            y: self.feet.y + self.height * 0.55,
            z: self.feet.z,
        }
    }
}

fn state() -> &'static Mutex<Vec<HitBurst>> {
    static STATE: OnceLock<Mutex<Vec<HitBurst>>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(Vec::new()))
}

fn clear() {
    state().lock().unwrap_or_else(|p| p.into_inner()).clear();
}

 

 
 
fn envelope(progress: f32) -> f32 {
    let p = progress.clamp(0.0, 1.0);
    let rise = (p / 0.08).min(1.0);
    let fall = ((1.0 - p) / 0.45).clamp(0.0, 1.0);
    rise * fall
}

fn ease_out(progress: f32) -> f32 {
    let p = progress.clamp(0.0, 1.0);
    1.0 - (1.0 - p).powi(3)
}

fn ease_out_back(progress: f32) -> f32 {
    let p = progress.clamp(0.0, 1.0);
    let c1 = 1.701_58;
    let c3 = c1 + 1.0;
    1.0 + c3 * (p - 1.0).powi(3) + c1 * (p - 1.0).powi(2)
}

 
fn hash01(index: usize, seed: f32) -> f64 {
    let x = (index as f64 * 12.989_8 + seed as f64 * 78.233).sin() * 43_758.545_3;
    x - x.floor()
}

 
fn rainbow(hue: f32) -> Color32 {
    let h = hue.rem_euclid(1.0) * 6.0;
    let band = h.floor() as i32;
    let f = h - band as f32;
    let (r, g, b) = match band.rem_euclid(6) {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    };
     
    let channel = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(channel(r), channel(g), channel(b))
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let blend = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        blend(a.r(), b.r()),
        blend(a.g(), b.g()),
        blend(a.b(), b.b()),
        blend(a.a(), b.a()),
    )
}

 
fn tint(burst: &HitBurst, hue: f32, alpha: f32) -> Color32 {
    let base = if burst.rainbow {
        rainbow(hue)
    } else {
        burst.color
    };
    theme::with_alpha(base, alpha.clamp(0.0, 1.0))
}

fn v3(x: f64, y: f64, z: f64) -> V3 {
    V3 { x, y, z }
}

fn offset(origin: V3, dir: V3, distance: f64) -> V3 {
    v3(
        origin.x + dir.x * distance,
        origin.y + dir.y * distance,
        origin.z + dir.z * distance,
    )
}

fn rotate_y(x: f64, z: f64, angle: f64) -> (f64, f64) {
    let (sin, cos) = angle.sin_cos();
    (x * cos - z * sin, x * sin + z * cos)
}

 
fn sphere_dir(index: usize, count: usize) -> V3 {
    let count = count.max(1) as f64;
    let y = 1.0 - 2.0 * (index as f64 + 0.5) / count;
    let ring = (1.0 - y * y).max(0.0).sqrt();
    let theta = index as f64 * 2.399_963_229_728_653;
    v3(ring * theta.cos(), y, ring * theta.sin())
}

 
fn pixels_per_block(view: &View, point: V3) -> Option<f32> {
    let depth = point.sub(view.cam).dot(view.fwd);
    if depth < 0.05 {
        return None;
    }
    Some((view.h as f64 * 0.5 / (view.tan_y * depth)) as f32)
}

 
fn anchor(view: &View, point: V3) -> Option<(Pos2, f32)> {
    Some((view.project(point)?, pixels_per_block(view, point)?))
}

fn project_path(view: &View, points: &[V3]) -> Option<Vec<Pos2>> {
    points.iter().map(|p| view.project(*p)).collect()
}

fn box_corners(feet: V3, width: f64, height: f64) -> [V3; 8] {
    let mut corners = [feet; 8];
    for (i, corner) in corners.iter_mut().enumerate() {
        let sx = if i & 1 == 0 { -1.0 } else { 1.0 };
        let sy = if i & 2 == 0 { 0.0 } else { 1.0 };
        let sz = if i & 4 == 0 { -1.0 } else { 1.0 };
        *corner = v3(
            feet.x + sx * width * 0.5,
            feet.y + sy * height,
            feet.z + sz * width * 0.5,
        );
    }
    corners
}

 
fn shock_radius(max_radius: f64, progress: f32, delay: f32, ring: usize) -> f64 {
    let ring_progress = ((progress - delay) / (1.0 - delay)).clamp(0.0, 1.0);
    max_radius * ease_out(ring_progress) as f64 * (1.0 - ring as f64 * 0.22)
}

 
fn shard_position(origin: V3, out: V3, speed: f64, secs: f64) -> V3 {
    let gravity = 6.0;
    v3(
        origin.x + out.x * speed * secs,
        origin.y + out.y * speed * secs + 1.4 * secs - 0.5 * gravity * secs * secs,
        origin.z + out.z * speed * secs,
    )
}

 
fn vortex_point(feet: V3, height: f64, radius: f64, t: f64, angle: f64, lift: f64) -> V3 {
    let r = radius * (0.22 + 0.78 * t);
    v3(
        feet.x + r * angle.cos(),
        feet.y + height * (0.05 + 1.12 * t) + lift,
        feet.z + r * angle.sin(),
    )
}

 

 
fn bloom(painter: &Painter, centre: Pos2, radius: f32, color: Color32, alpha: f32) {
    if radius <= 0.5 || alpha <= 0.0 {
        return;
    }
    for (scale, strength) in [(1.0, 0.16), (0.62, 0.26), (0.36, 0.45), (0.18, 0.9)] {
        painter.circle_filled(
            centre,
            radius * scale,
            theme::with_alpha(color, alpha * strength),
        );
    }
}

 
fn sparkle(painter: &Painter, centre: Pos2, size: f32, color: Color32, alpha: f32) {
    let color = theme::with_alpha(color, alpha);
    if size < 0.5 {
        return;
    }
    let points = vec![
        centre + vec2(0.0, -size),
        centre + vec2(size * 0.28, 0.0),
        centre + vec2(0.0, size),
        centre + vec2(-size * 0.28, 0.0),
    ];
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

 
 
fn draw_ribbon(painter: &Painter, points: &[Pos2], widths: &[f32], color: Color32, glow: bool) {
    if points.len() < 2 || widths.len() != points.len() {
        return;
    }
    if glow {
        ribbon_quads(painter, points, widths, 3.0, theme::with_alpha(color, 0.22));
    }
    ribbon_quads(painter, points, widths, 1.0, color);
}

fn ribbon_quads(painter: &Painter, points: &[Pos2], widths: &[f32], grow: f32, color: Color32) {
    for i in 0..points.len() - 1 {
        let a = points[i];
        let b = points[i + 1];
        let delta = b - a;
        let length = delta.length();
        if length < 0.05 {
            continue;
        }
        let normal = vec2(-delta.y, delta.x) / length;
        let wa = (widths[i] * grow).max(0.25);
        let wb = (widths[i + 1] * grow).max(0.25);
        painter.add(Shape::convex_polygon(
            vec![
                a + normal * wa,
                b + normal * wb,
                b - normal * wb,
                a - normal * wa,
            ],
            color,
            Stroke::NONE,
        ));
    }
}

 
#[derive(Clone, Copy)]
struct Spike {
    angle: f32,
    inner: f32,
    tip: f32,
    half_width: f32,
}

 
fn draw_spike(painter: &Painter, centre: Pos2, spike: Spike, color: Color32, glow: bool) {
    let Spike {
        angle,
        inner,
        tip,
        half_width,
    } = spike;
    let dir = vec2(angle.cos(), angle.sin());
    let normal = vec2(-dir.y, dir.x);
    let base = centre + dir * inner;
    let point = centre + dir * tip;
    if glow {
        let halo = theme::with_alpha(color, 0.2);
        painter.add(Shape::convex_polygon(
            vec![
                base + normal * half_width * 2.6,
                point,
                base - normal * half_width * 2.6,
            ],
            halo,
            Stroke::NONE,
        ));
    }
    painter.add(Shape::convex_polygon(
        vec![
            base + normal * half_width,
            point,
            base - normal * half_width,
        ],
        color,
        Stroke::NONE,
    ));
}

 
fn draw_ring_2d(
    painter: &Painter,
    centre: Pos2,
    radius: f32,
    rotation: f32,
    width: f32,
    color: Color32,
    glow: bool,
) {
    if radius < 0.5 {
        return;
    }
    let points: Vec<Pos2> = (0..=48)
        .map(|i| {
            let angle = rotation + TAU as f32 * i as f32 / 48.0;
            centre + vec2(angle.cos(), angle.sin()) * radius
        })
        .collect();
    if glow {
        painter.add(Shape::line(
            points.clone(),
            Stroke::new(width * 3.2, theme::with_alpha(color, 0.2)),
        ));
    }
    painter.add(Shape::line(points, Stroke::new(width, color)));
}

 
fn draw_world_ring(
    painter: &Painter,
    view: &View,
    centre: V3,
    radius: f64,
    width: f32,
    color: Color32,
    glow: bool,
) {
    if radius <= 0.01 {
        return;
    }
    let points: Vec<V3> = (0..=48)
        .map(|i| {
            let angle = TAU * i as f64 / 48.0;
            v3(
                centre.x + radius * angle.cos(),
                centre.y,
                centre.z + radius * angle.sin(),
            )
        })
        .collect();
    let Some(path) = project_path(view, &points) else {
        return;
    };
    if glow {
        painter.add(Shape::line(
            path.clone(),
            Stroke::new(width * 3.0, theme::with_alpha(color, 0.2)),
        ));
    }
    painter.add(Shape::line(path, Stroke::new(width, color)));
}

fn draw_world_line(
    painter: &Painter,
    view: &View,
    a: V3,
    b: V3,
    width: f32,
    color: Color32,
    glow: bool,
) {
    let (Some(pa), Some(pb)) = (view.project(a), view.project(b)) else {
        return;
    };
    if glow {
        painter.line_segment(
            [pa, pb],
            Stroke::new(width * 3.0, theme::with_alpha(color, 0.2)),
        );
    }
    painter.line_segment([pa, pb], Stroke::new(width, color));
}

 

 
 
fn draw_slash(painter: &Painter, view: &View, burst: &HitBurst, progress: f32) {
    let Some((centre, scale)) = anchor(view, burst.centre()) else {
        return;
    };
    let radius = (scale as f64 * (burst.size * 1.05 + burst.width * 0.45)) as f32;
    let fade = envelope(progress);
    if fade <= 0.0 {
        return;
    }
    let spin = (burst.seed - 0.5) * 0.55;
    let max_width = (scale * 0.045 * burst.size as f32).clamp(2.0, 18.0);
    for blade in 0..2 {
        let stagger = blade as f32 * 0.06;
        let local = ((progress - stagger) / (1.0 - stagger)).clamp(0.0, 1.0);
        let reveal = ease_out((local / 0.34).min(1.0));
        if reveal <= 0.0 {
            continue;
        }
        let alpha = fade * (1.0 - blade as f32 * 0.18);
        let (p0, p1, p2) = if blade == 0 {
            (vec2(-1.20, -0.80), vec2(0.0, 0.16), vec2(1.20, 0.80))
        } else {
            (vec2(-1.20, 0.80), vec2(0.0, -0.16), vec2(1.20, -0.80))
        };
        let rotate = |p: egui::Vec2| {
            let x = p.x * spin.cos() - p.y * spin.sin();
            let y = p.x * spin.sin() + p.y * spin.cos();
            vec2(x, y)
        };
        let (a, b, c) = (rotate(p0), rotate(p1), rotate(p2));
        let steps = 22usize;
        let drawn = ((steps as f32) * reveal).ceil().max(2.0) as usize;
        let mut points = Vec::with_capacity(drawn + 1);
        let mut widths = Vec::with_capacity(drawn + 1);
        let end = reveal.min(1.0);
        for i in 0..=drawn {
            let t = (i as f32 / steps as f32).min(end);
            let u = 1.0 - t;
            let p = a * (u * u) + b * (2.0 * u * t) + c * (t * t);
            let point = centre + p * radius;
            points.push(point);
            let taper = (PI as f32 * t).sin().max(0.0).powf(0.7);
            widths.push(max_width * taper * (1.0 - 0.25 * t));
        }
        let color = tint(burst, burst.seed + blade as f32 * 0.12, alpha * 0.9);
        draw_ribbon(painter, &points, &widths, color, burst.glow);
        if let Some(tip) = points.last() {
            sparkle(
                painter,
                *tip,
                max_width * 1.3,
                mix(burst.color, Color32::WHITE, 0.8),
                alpha,
            );
        }
    }

     
    if progress < 0.26 {
        let flash = 1.0 - progress / 0.26;
        bloom(
            painter,
            centre,
            radius * 0.34 * flash,
            Color32::WHITE,
            flash,
        );
    }
}

 
fn draw_impact(painter: &Painter, view: &View, burst: &HitBurst, progress: f32) {
    let Some((centre, scale)) = anchor(view, burst.centre()) else {
        return;
    };
    let fade = envelope(progress);
    if fade <= 0.0 {
        return;
    }
    let radius = scale * (0.95 * burst.size as f32 + burst.width as f32 * 0.4);
    let grow = ease_out((progress / 0.16).min(1.0));
    let spin = burst.seed * TAU as f32 + progress * 0.9;
    let count = (12.0 + 16.0 * burst.intensity).round() as usize;
    let shrink = 1.0 - progress * 0.35;
    for i in 0..count {
        let angle = spin + TAU as f32 * i as f32 / count as f32;
        let long = i % 3 == 0;
        let reach = radius
            * (if long { 1.25 } else { 0.72 })
            * (0.7 + 0.5 * hash01(i, burst.seed) as f32)
            * grow
            * shrink;
        let half = (scale * 0.020 * burst.size as f32).clamp(1.2, 8.0)
            * (0.6 + 0.6 * hash01(i + 7, burst.seed) as f32);
        let color = tint(
            burst,
            burst.seed + i as f32 / count as f32,
            fade * if long { 0.95 } else { 0.7 },
        );
        draw_spike(
            painter,
            centre,
            Spike {
                angle,
                inner: radius * 0.08,
                tip: reach,
                half_width: half,
            },
            color,
            burst.glow,
        );
    }
    draw_ring_2d(
        painter,
        centre,
        radius * (0.22 + 0.5 * grow),
        spin,
        1.6,
        tint(burst, burst.seed + 0.3, fade * 0.55),
        burst.glow,
    );
    bloom(
        painter,
        centre,
        radius * 0.42 * grow,
        mix(burst.color, Color32::WHITE, 0.5),
        fade,
    );
}

 
fn draw_shockwave(painter: &Painter, view: &View, burst: &HitBurst, progress: f32) {
    let fade = envelope(progress);
    if fade <= 0.0 {
        return;
    }
    let ground = v3(burst.feet.x, burst.feet.y + 0.04, burst.feet.z);
    let max_radius = 1.7 * burst.size + burst.width * 0.7;
    let mut outer = 0.0;
    for ring in 0..3 {
        let delay = ring as f32 * 0.08;
        let ring_progress = ((progress - delay) / (1.0 - delay)).clamp(0.0, 1.0);
        if ring_progress <= 0.0 {
            continue;
        }
        let radius = shock_radius(max_radius, progress, delay, ring);
        if ring == 0 {
            outer = radius;
        }
        let alpha = fade * (1.0 - ring_progress).powf(0.7) * (1.0 - ring as f32 * 0.18);
        let width = 4.0 * (1.0 - ring_progress) + 0.8;
        draw_world_ring(
            painter,
            view,
            ground,
            radius,
            width,
            tint(burst, burst.seed + ring as f32 * 0.08, alpha),
            burst.glow,
        );
    }

     
    let spokes = (12.0 + 10.0 * burst.intensity).round() as usize;
    let front = (1.0 - progress).clamp(0.0, 1.0);
    if outer > 0.02 && front > 0.05 {
        for i in 0..spokes {
            let angle = TAU * i as f64 / spokes as f64 + burst.seed as f64 * TAU;
            let dir = v3(angle.cos(), 0.0, angle.sin());
            let start = offset(ground, dir, outer * 0.88);
            let end = offset(ground, dir, outer * 1.06);
            draw_world_line(
                painter,
                view,
                start,
                end,
                2.2 * front + 0.4,
                tint(
                    burst,
                    burst.seed + i as f32 / spokes as f32,
                    fade * front * 0.75,
                ),
                burst.glow,
            );
        }
    }

    if progress < 0.2 {
        let flash = 1.0 - progress / 0.2;
        if let (Some(centre), Some(scale)) = (
            view.project(burst.centre()),
            pixels_per_block(view, burst.centre()),
        ) {
            bloom(
                painter,
                centre,
                scale * burst.size as f32 * 0.55 * flash,
                Color32::WHITE,
                flash * 0.8,
            );
        }
    }
}

 
fn draw_shatter(painter: &Painter, view: &View, burst: &HitBurst, progress: f32) {
    let fade = envelope(progress);
    if fade <= 0.0 {
        return;
    }
    let feet = burst.feet;
    let (width, height) = (burst.width, burst.height);
    let crack = 1.0 - (progress / 0.18).min(1.0);
    if crack > 0.0 {
        let corners = box_corners(feet, width, height);
        for (a, b) in BOX_EDGES {
            draw_world_line(
                painter,
                view,
                corners[a],
                corners[b],
                1.8,
                tint(burst, burst.seed, fade * crack * 0.9),
                burst.glow,
            );
        }
    }

    if progress < 0.2 {
        let flash = 1.0 - progress / 0.2;
        if let (Some(centre_px), Some(scale)) = (
            view.project(burst.centre()),
            pixels_per_block(view, burst.centre()),
        ) {
            bloom(
                painter,
                centre_px,
                scale * width as f32 * 0.45 * flash,
                Color32::WHITE,
                flash,
            );
        }
    }

    let secs = progress as f64 * burst.duration.as_secs_f64();
    let count = (16.0 + 12.0 * burst.intensity).round() as usize;
    for i in 0..count {
        let origin = v3(
            feet.x + (hash01(i, burst.seed) - 0.5) * width,
            feet.y + hash01(i + 40, burst.seed) * height,
            feet.z + (hash01(i + 80, burst.seed) - 0.5) * width,
        );
         
         
        let dir = sphere_dir(i, count);
        let out = v3(dir.x, dir.y + 0.15, dir.z);
        let speed = (1.4 + 2.2 * hash01(i + 120, burst.seed)) * burst.size;
        let position = shard_position(origin, out, speed, secs);
        let (Some(centre_px), Some(scale)) =
            (view.project(position), pixels_per_block(view, position))
        else {
            continue;
        };
         
         
        let size = width * 0.24 * (0.6 + 0.9 * hash01(i + 200, burst.seed));
        let radius = scale * size as f32;
        let spin = (hash01(i + 160, burst.seed) * 2.0 - 1.0) * 11.0 * secs;
        let corners: Vec<Pos2> = (0..3)
            .map(|k| {
                let angle = spin + TAU * k as f64 / 3.0 + hash01(i + k + 300, burst.seed) * 0.5;
                let reach = radius * (0.65 + 0.45 * hash01(i + k + 320, burst.seed) as f32);
                centre_px + vec2(angle.cos() as f32, angle.sin() as f32) * reach
            })
            .collect();
        let colour = mix(
            tint(burst, burst.seed + i as f32 / count as f32, fade),
            Color32::WHITE,
            (hash01(i + 240, burst.seed) as f32 * 0.35).min(fade * 0.35),
        );
        painter.add(Shape::convex_polygon(
            corners,
            colour,
            Stroke::new(0.8_f32, theme::with_alpha(Color32::WHITE, fade * 0.35)),
        ));
    }
}

 
fn draw_vortex(painter: &Painter, view: &View, burst: &HitBurst, progress: f32) {
    let fade = envelope(progress);
    if fade <= 0.0 {
        return;
    }
    let grow = 0.45 + 0.55 * ease_out((progress / 0.3).min(1.0));
    let radius = (0.9 * burst.size + burst.width * 0.65) * grow as f64;
    let lift = progress as f64 * 0.25;
    let strands = 3;
    let scale = pixels_per_block(view, burst.centre()).unwrap_or(1.0);
    let spin = burst.seed as f64 * TAU + progress as f64 * TAU * 1.6;
    for strand in 0..strands {
        let points: Vec<V3> = (0..=32)
            .map(|k| {
                let t = k as f64 / 32.0;
                let angle = spin + strand as f64 * TAU / 3.0 + t * TAU * 1.25;
                vortex_point(burst.feet, burst.height, radius, t, angle, lift)
            })
            .collect();
        let Some(path) = project_path(view, &points) else {
            continue;
        };
        let width = (scale * 0.020 * burst.size as f32).clamp(1.0, 7.0);
        let widths: Vec<f32> = (0..=32)
            .map(|k| {
                let t = k as f32 / 32.0;
                width * (PI as f32 * t).sin().max(0.0).powf(0.7)
            })
            .collect();
        let color = tint(
            burst,
            burst.seed + strand as f32 / strands as f32 + progress * 0.25,
            fade * (0.85 - strand as f32 * 0.12),
        );
        draw_ribbon(painter, &path, &widths, color, burst.glow);
    }

     
    let beam = tint(burst, burst.seed + 0.5, fade * 0.35);
    draw_world_line(
        painter,
        view,
        v3(burst.feet.x, burst.feet.y + 0.05, burst.feet.z),
        v3(
            burst.feet.x,
            burst.feet.y + burst.height * 1.15,
            burst.feet.z,
        ),
        2.0,
        beam,
        burst.glow,
    );
}

 
 
fn draw_sigil(painter: &Painter, view: &View, burst: &HitBurst, progress: f32) {
    let Some((centre, scale)) = anchor(view, burst.centre()) else {
        return;
    };
    let fade = envelope(progress);
    if fade <= 0.0 {
        return;
    }
    let grow = ease_out_back((progress / 0.22).min(1.0));
    let radius = scale * (0.85 * burst.size as f32 + burst.width as f32 * 0.35) * grow;
    if radius < 1.0 {
        return;
    }
    let spin = burst.seed * TAU as f32 + progress * TAU as f32 * 0.9;
    let outer = tint(burst, burst.seed, fade * 0.95);
    let inner = tint(burst, burst.seed + 0.2, fade * 0.55);
    draw_ring_2d(painter, centre, radius, spin, 2.2, outer, burst.glow);
    draw_ring_2d(
        painter,
        centre,
        radius * 0.7,
        -spin * 1.3,
        1.3,
        inner,
        burst.glow,
    );
    draw_ring_2d(
        painter,
        centre,
        radius * 0.34,
        spin * 1.8,
        1.0,
        tint(burst, burst.seed + 0.4, fade * 0.4),
        false,
    );

     
    let ticks = 12;
    for i in 0..ticks {
        let angle = spin + TAU as f32 * i as f32 / ticks as f32;
        let dir = vec2(angle.cos(), angle.sin());
        let long = i % 3 == 0;
        let a = centre + dir * radius * 0.86;
        let b = centre + dir * radius * if long { 1.16 } else { 1.04 };
        painter.line_segment(
            [a, b],
            Stroke::new(if long { 2.2_f32 } else { 1.2_f32 }, inner),
        );
    }

     
    let vertices: Vec<Pos2> = (0..6)
        .map(|i| {
            let angle = spin + TAU as f32 * i as f32 / 6.0;
            centre + vec2(angle.cos(), angle.sin()) * radius * 0.82
        })
        .collect();
    for start in 0..2 {
        let path = vec![
            vertices[start],
            vertices[start + 2],
            vertices[start + 4],
            vertices[start],
        ];
        if burst.glow {
            painter.add(Shape::line(
                path.clone(),
                Stroke::new(4.5_f32, theme::with_alpha(outer, 0.18)),
            ));
        }
        painter.add(Shape::line(path, Stroke::new(1.4_f32, outer)));
    }

    bloom(
        painter,
        centre,
        radius * 0.2,
        mix(burst.color, Color32::WHITE, 0.5),
        fade * 0.8,
    );
}

 
 
fn draw_nova(painter: &Painter, view: &View, burst: &HitBurst, progress: f32) {
    let Some((centre, scale)) = anchor(view, burst.centre()) else {
        return;
    };
    let fade = envelope(progress);
    if fade <= 0.0 {
        return;
    }
    let radius = (1.25 * burst.size + burst.width * 0.4) * scale as f64;
    let grow = ease_out((progress / 0.24).min(1.0));
    let spin = burst.seed as f64 * TAU + progress as f64 * 0.8;
    let count = (20.0 + 20.0 * burst.intensity).round() as usize;
    let centre_world = burst.centre();
    for i in 0..count {
        let dir = sphere_dir(i, count);
        let (x, z) = rotate_y(dir.x, dir.z, spin);
        let dir = v3(x, dir.y, z);
        let length = radius * (0.7 + 0.8 * hash01(i, burst.seed)) * grow as f64;
        let start = offset(centre_world, dir, 0.06);
        let end = offset(centre_world, dir, length);
        let (Some(pa), Some(pb)) = (view.project(start), view.project(end)) else {
            continue;
        };
        let base = (scale * 0.014 * burst.size as f32).clamp(0.8, 5.0);
        let alpha = fade * if i % 4 == 0 { 0.95 } else { 0.55 };
        let color = tint(
            burst,
            burst.seed + i as f32 / count as f32 + progress * 0.2,
            alpha,
        );
        draw_ribbon(painter, &[pa, pb], &[base, 0.35], color, burst.glow);
    }

    let halo = radius * (0.3 + 0.75 * ease_out(progress) as f64);
    draw_ring_2d(
        painter,
        centre,
        halo as f32,
        spin as f32,
        1.4,
        tint(burst, burst.seed + 0.4, fade * 0.45),
        burst.glow,
    );
    bloom(
        painter,
        centre,
        radius as f32 * 0.22 * grow,
        mix(burst.color, Color32::WHITE, 0.65),
        fade,
    );
}

 
 
fn draw_cross(painter: &Painter, view: &View, burst: &HitBurst, progress: f32) {
    let Some((centre, scale)) = anchor(view, burst.centre()) else {
        return;
    };
    let fade = envelope(progress);
    if fade <= 0.0 {
        return;
    }
    let radius = scale * (0.95 * burst.size as f32 + burst.width as f32 * 0.35);
    let grow = ease_out((progress / 0.15).min(1.0));
    let spin = burst.seed * TAU as f32 + progress * 0.55;
    for layer in 0..2 {
        let layer_radius = radius * if layer == 0 { 1.0 } else { 0.62 };
        let rotation = spin + layer as f32 * PI as f32 / 4.0;
        let alpha = fade * if layer == 0 { 0.95 } else { 0.5 };
        let half = (scale * 0.030 * burst.size as f32).clamp(1.5, 11.0) * (1.0 - progress * 0.45);
        for blade in 0..4 {
            let angle = rotation + blade as f32 * PI as f32 / 2.0;
            draw_spike(
                painter,
                centre,
                Spike {
                    angle,
                    inner: layer_radius * 0.12,
                    tip: layer_radius * (0.55 + 0.45 * grow),
                    half_width: half,
                },
                tint(
                    burst,
                    burst.seed + layer as f32 * 0.2 + blade as f32 * 0.05,
                    alpha,
                ),
                burst.glow,
            );
        }
    }
    draw_ring_2d(
        painter,
        centre,
        radius * (0.35 + 0.65 * grow),
        -spin,
        1.4,
        tint(burst, burst.seed + 0.6, fade * 0.5),
        burst.glow,
    );
    bloom(painter, centre, radius * 0.16 * grow, Color32::WHITE, fade);
}

 
fn draw_burst(painter: &Painter, view: &View, burst: &HitBurst, progress: f32) {
    match burst.style {
        HitStyle::Slash => draw_slash(painter, view, burst, progress),
        HitStyle::Impact => draw_impact(painter, view, burst, progress),
        HitStyle::Shockwave => draw_shockwave(painter, view, burst, progress),
        HitStyle::Shatter => draw_shatter(painter, view, burst, progress),
        HitStyle::Vortex => draw_vortex(painter, view, burst, progress),
        HitStyle::Sigil => draw_sigil(painter, view, burst, progress),
        HitStyle::Nova => draw_nova(painter, view, burst, progress),
        HitStyle::Cross => draw_cross(painter, view, burst, progress),
    }
}

 

 
 
pub(crate) fn read_config() -> HitConfig {
    let mut cfg = HitConfig::default();
    if let Some(arc) = client().modules.get(ModuleId::HitEffect) {
        if let Ok(module) = arc.lock() {
            cfg = HitConfig::from_data(module.get_module_data());
        }
    }
    if !cfg.enabled {
        clear();
    }
    cfg
}

 
pub(crate) fn draw(painter: &Painter, view: &View, now: Instant, cfg: &HitConfig) {
    if !cfg.enabled {
        clear();
        return;
    }
    let mut bursts = state().lock().unwrap_or_else(|p| p.into_inner());
    bursts.retain(|burst| now.duration_since(burst.spawned) < burst.duration);
    for burst in bursts.iter() {
        let elapsed = now.duration_since(burst.spawned).as_secs_f32();
        let progress = (elapsed / burst.duration.as_secs_f32()).clamp(0.0, 1.0);
        draw_burst(painter, view, burst, progress);
    }
}

 

 
 
 
 
 
 
fn picked_target(pick: Option<(i32, bool)>, self_id: Option<i32>, players_only: bool) -> bool {
    matches!(pick, Some((id, is_player)) if Some(id) != self_id && (!players_only || is_player))
}

 
 
#[derive(Debug)]
pub struct HitEffectModule {
    pub module: ModuleData,
     
    attack_pending: AtomicBool,
}

impl HitEffectModule {
    pub fn new() -> Self {
        let slider = |name: &str, value: f32, min: f32, max: f32| ModuleSetting::Slider {
            name: name.into(),
            value,
            min,
            max,
        };
        Self {
            module: ModuleData {
                id: ModuleId::HitEffect,
                description:
                    "Paints custom rendered slash, impact, shockwave and shatter effects where your attacks land"
                        .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Choice {
                        name: "Style".into(),
                        value: 0,
                        options: HitStyle::OPTIONS.iter().map(|s| s.to_string()).collect(),
                    },
                    ModuleSetting::Color {
                        name: "Color".into(),
                        value: [1.0, 0.14, 0.19, 1.0],
                    },
                    ModuleSetting::Toggle {
                        name: "Rainbow".into(),
                        value: false,
                    },
                    slider("Duration", DEFAULT_DURATION, 0.25, 2.0),
                    slider("Size", DEFAULT_SIZE, 0.4, 2.5),
                    slider("Intensity", DEFAULT_INTENSITY, 0.4, 2.0),
                    ModuleSetting::Toggle {
                        name: "Glow".into(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Players Only".into(),
                        value: false,
                    },
                ],
            },
            attack_pending: AtomicBool::new(false),
        }
    }

     
    fn spawn(&self, now: Instant, cfg: &HitConfig, feet: V3, width: f64, height: f64) {
        let mut bursts = state().lock().unwrap_or_else(|p| p.into_inner());
        bursts.retain(|burst| now.duration_since(burst.spawned) < burst.duration);
        bursts.push(HitBurst {
            feet,
            width,
            height,
            style: cfg.style,
            color: cfg.color,
            rainbow: cfg.rainbow,
            seed: roll() as f32,
            spawned: now,
            duration: cfg.duration,
            size: cfg.size,
            intensity: cfg.intensity,
            glow: cfg.glow,
        });
        if bursts.len() > MAX_BURSTS {
            let excess = bursts.len() - MAX_BURSTS;
            bursts.drain(..excess);
        }
    }
}

impl Module for HitEffectModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.attack_pending.store(false, Ordering::Relaxed);
        clear();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let pending = self.attack_pending.swap(false, Ordering::Relaxed);
        if !self.module.enabled {
            clear();
            return Ok(());
        }
        let now = Instant::now();
        state()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .retain(|burst| now.duration_since(burst.spawned) < burst.duration);
        if !pending {
            return Ok(());
        }

         
         
         
         
        let minecraft = minecraft();
        let cfg = HitConfig::from_data(&self.module);
        let pick = minecraft.crosshair_entity()?;
        let self_id = minecraft.player()?.map(|player| player.id()).transpose()?;
        let picked = match &pick {
            Some(entity) => Some((entity.id()?, entity.instance_of::<Player>())),
            None => None,
        };
        if !picked_target(picked, self_id, cfg.players_only) {
            return Ok(());
        }
        let target = pick.expect("a picked target");
        let feet = target.get_position()?;
        let width = target.bb_width().unwrap_or(FALLBACK_WIDTH as f32) as f64;
        let height = target.bb_height().unwrap_or(FALLBACK_HEIGHT as f32) as f64;
        self.spawn(now, &cfg, V3::from(feet), width, height);
        Ok(())
    }

    fn handle_packet(&self, packet: &mut Packet) -> PacketAction {
         
         
         
         
        if self.module.enabled && matches!(packet, Packet::ServerboundSwing(_)) {
            self.attack_pending.store(true, Ordering::Relaxed);
        }
        PacketAction::Forward
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_view() -> View {
        View {
            cam: v3(0.0, 64.0, 0.0),
            fwd: v3(0.0, 0.0, 1.0),
            right: v3(-1.0, 0.0, 0.0),
            up: v3(0.0, 1.0, 0.0),
            tan_x: 1.24,
            tan_y: 0.70,
            w: 1280.0,
            h: 720.0,
        }
    }

    fn burst(style: HitStyle) -> HitBurst {
        HitBurst {
            feet: v3(0.0, 63.0, 4.0),
            width: 0.6,
            height: 1.8,
            style,
            color: DEFAULT_COLOR,
            rainbow: false,
            seed: 0.37,
            spawned: Instant::now(),
            duration: Duration::from_secs_f32(DEFAULT_DURATION),
            size: 1.0,
            intensity: 1.0,
            glow: true,
        }
    }

    #[test]
    fn every_style_has_a_name_and_round_trips() {
        assert_eq!(HitStyle::ALL.len(), HitStyle::OPTIONS.len());
        for (index, style) in HitStyle::ALL.iter().enumerate() {
            assert_eq!(HitStyle::from_index(index), *style);
        }
    }

    #[test]
    fn unknown_style_falls_back_to_slash() {
        assert_eq!(HitStyle::from_index(99), HitStyle::Slash);
        assert_eq!(
            HitStyle::from_index(HitStyle::OPTIONS.len()),
            HitStyle::Slash
        );
    }

    #[test]
    fn envelope_starts_and_ends_at_zero_and_holds_in_the_middle() {
        assert_eq!(envelope(0.0), 0.0);
        assert_eq!(envelope(1.0), 0.0);
        assert_eq!(envelope(-2.0), 0.0);
        assert_eq!(envelope(7.0), 0.0);
        assert!((envelope(0.3) - 1.0).abs() < 1e-6);
        assert!((envelope(0.5) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn rainbow_hue_wraps_around() {
        assert_eq!(rainbow(0.0), rainbow(1.0));
        assert_eq!(rainbow(-0.25), rainbow(0.75));
        assert_ne!(rainbow(0.0), rainbow(0.5));
    }

    #[test]
    fn pixels_per_block_shrinks_with_distance() {
        let view = test_view();
        let near = pixels_per_block(&view, v3(0.0, 64.0, 2.0)).unwrap();
        let far = pixels_per_block(&view, v3(0.0, 64.0, 8.0)).unwrap();
        assert!(near > far);
        assert!(pixels_per_block(&view, v3(0.0, 64.0, -2.0)).is_none());
    }

     
     
     
    #[test]
    fn a_swing_only_counts_when_the_crosshair_picked_an_entity() {
        assert!(
            !picked_target(None, Some(1), false),
            "swinging at air or mining a block picks nothing"
        );
        assert!(
            !picked_target(Some((1, true)), Some(1), false),
            "the local player is never a target"
        );
        assert!(picked_target(Some((2, true)), Some(1), false));
        assert!(picked_target(Some((2, false)), Some(1), false));
        assert!(
            !picked_target(Some((2, false)), Some(1), true),
            "Players Only skips non-players"
        );
        assert!(picked_target(Some((2, true)), Some(1), true));
    }

    #[test]
    fn shockwave_radius_grows_over_time() {
        let early = shock_radius(3.0, 0.2, 0.0, 0);
        let late = shock_radius(3.0, 0.9, 0.0, 0);
        assert!(late > early);
        assert!(shock_radius(3.0, 0.2, 0.0, 0) <= shock_radius(3.0, 0.9, 0.0, 0));
         
        assert_eq!(shock_radius(3.0, 0.05, 0.1, 2), 0.0);
    }

    #[test]
    fn shards_are_pulled_down_by_gravity() {
        let origin = v3(0.0, 65.0, 0.0);
        let out = v3(0.0, 0.6, 0.8);
        let early = shard_position(origin, out, 3.0, 0.1);
        let late = shard_position(origin, out, 3.0, 1.0);
        assert!(late.y < early.y + 3.0);
        assert!(late.z > early.z);
    }

    #[test]
    fn vortex_climbs_and_widens() {
        let feet = v3(0.0, 64.0, 0.0);
        let low = vortex_point(feet, 1.8, 1.0, 0.1, 0.0, 0.0);
        let high = vortex_point(feet, 1.8, 1.0, 0.9, 0.0, 0.0);
        assert!(high.y > low.y);
        assert!(high.x.abs() > low.x.abs());
        assert!(high.y <= feet.y + 1.8 * 1.12 + 1e-9);
    }

    #[test]
    fn defaults_are_a_red_slash() {
        let module = HitEffectModule::new();
        let cfg = HitConfig::from_data(&module.module);
        assert!(!cfg.enabled);
        assert_eq!(cfg.style, HitStyle::Slash);
        assert_eq!(cfg.color, DEFAULT_COLOR);
        assert!(cfg.glow);
        assert!(!cfg.rainbow);
        assert_eq!(cfg.duration, Duration::from_secs_f32(DEFAULT_DURATION));
    }

    #[test]
    fn each_style_renders_a_visible_effect() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        const CELL_W: usize = 480;
        const CELL_H: usize = 360;
        let view = View {
            cam: v3(0.0, 65.0, 0.0),
            fwd: v3(0.0, 0.0, 1.0),
            right: v3(-1.0, 0.0, 0.0),
            up: v3(0.0, 1.0, 0.0),
            tan_x: 1.24,
            tan_y: 0.70,
            w: CELL_W as f32,
            h: CELL_H as f32,
        };
        let mut sheet = image::RgbaImage::from_pixel(
            CELL_W as u32 * 4,
            CELL_H as u32 * 2,
            image::Rgba([13, 14, 18, 255]),
        );
        for (index, style) in HitStyle::ALL.iter().enumerate() {
            let mut b = burst(*style);
            b.feet = v3(0.0, 64.0, 3.0);
            let ctx = egui::Context::default();
            let mut textures = HashMap::new();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        vec2(CELL_W as f32, CELL_H as f32),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Background,
                        egui::Id::new("hit_effect_preview"),
                    ));
                    draw_burst(&painter, &view, &b, 0.35);
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            let primitives = ctx.tessellate(output.shapes, 1.0);
            let cell = rasterize(
                primitives,
                &textures,
                CELL_W,
                CELL_H,
                [0.05, 0.05, 0.07, 1.0],
            );
            let lit = cell
                .pixels()
                .filter(|pixel| {
                    pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32 > 120 && pixel[3] > 128
                })
                .count();
            assert!(
                lit > 150,
                "{style:?} rendered almost nothing ({lit} lit pixels)"
            );
            image::imageops::replace(
                &mut sheet,
                &cell,
                ((index % 4) * CELL_W) as i64,
                ((index / 4) * CELL_H) as i64,
            );
        }
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/hit-effects-preview.png");
        sheet.save(&path).expect("the hit effect preview png saves");
    }

    #[test]
    fn every_style_draws_something_and_never_text() {
        let view = test_view();
        for style in HitStyle::ALL {
            let ctx = egui::Context::default();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(view.w, view.h))),
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Background,
                        egui::Id::new("hit_effect_test"),
                    ));
                    draw_burst(&painter, &view, &burst(style), 0.45);
                },
            );
            let shapes = output
                .shapes
                .iter()
                .filter(|shape| !matches!(shape.shape, egui::Shape::Noop | egui::Shape::Text(_)))
                .count();
            assert!(shapes > 0, "{style:?} should draw rendered geometry");
            assert!(
                !output
                    .shapes
                    .iter()
                    .any(|shape| matches!(shape.shape, egui::Shape::Text(_))),
                "{style:?} must not draw text"
            );
        }
    }
}
