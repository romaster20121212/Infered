 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::esp::{
    choice_setting, color_setting, slider_setting, toggle_setting, View, V3,
};
use crate::graphic::theme;
use crate::module::{ModuleData, ModuleId};
use crate::state::client;
use egui::{vec2, Color32, Mesh, Painter, Pos2, Shape, Stroke};
use std::f64::consts::TAU;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

 
const SEGMENTS: usize = 24;
 
const FLAME_STEPS: usize = 10;
 
 
const MIN_DRAW_DISTANCE: f64 = 0.9;
 
const TICK: Duration = Duration::from_millis(50);
 
const TELEPORT: f64 = 12.0;
 
const MIN_PRONG_BASE: f32 = 1.0;

const DEFAULT_BAND: Color32 = Color32::from_rgb(184, 115, 26);
const DEFAULT_FLAME: Color32 = Color32::from_rgb(255, 76, 13);
const DEFAULT_CORE: Color32 = Color32::from_rgb(255, 229, 140);
const DEFAULT_GEM: Color32 = Color32::from_rgb(255, 31, 26);

 
 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CrownStyle {
     
    Inferno,
     
    Halo,
     
    Spiked,
}

impl CrownStyle {
    pub(crate) const OPTIONS: [&'static str; 3] = ["Inferno", "Halo", "Spiked"];

    pub(crate) fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Halo,
            2 => Self::Spiked,
            _ => Self::Inferno,
        }
    }

     
    fn metal_band(self) -> bool {
        !matches!(self, Self::Halo)
    }

     
    fn prongs(self) -> bool {
        matches!(self, Self::Spiked)
    }

     
    fn lift(self) -> f64 {
        match self {
            Self::Halo => 0.24,
            _ => 0.0,
        }
    }

     
    fn flame_scale(self) -> f64 {
        match self {
            Self::Inferno => 1.0,
            Self::Halo => 1.15,
            Self::Spiked => 0.45,
        }
    }
}

 
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CrownConfig {
    pub(crate) enabled: bool,
    style: CrownStyle,
    band: Color32,
    flame: Color32,
    core: Color32,
    gem: Color32,
    rainbow: bool,
    glow: bool,
    glow_strength: f32,
    opacity: f32,
    size: f32,
    hover: f64,
    points: usize,
    spike_height: f64,
    band_height: f64,
    thickness: f32,
    flame_length: f64,
    waviness: f32,
    flame_speed: f32,
    flicker: f32,
    spin: f32,
    bob: f64,
    gems: bool,
    embers: usize,
    ember_speed: f32,
    pub(crate) show_self: bool,
    pub(crate) show_others: bool,
    pub(crate) range: f64,
}

impl Default for CrownConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            style: CrownStyle::Inferno,
            band: DEFAULT_BAND,
            flame: DEFAULT_FLAME,
            core: DEFAULT_CORE,
            gem: DEFAULT_GEM,
            rainbow: false,
            glow: true,
            glow_strength: 1.0,
            opacity: 0.95,
            size: 1.0,
            hover: 0.0,
            points: 8,
            spike_height: 0.22,
            band_height: 0.10,
            thickness: 1.0,
            flame_length: 0.45,
            waviness: 0.55,
            flame_speed: 1.6,
            flicker: 0.6,
            spin: 0.35,
            bob: 0.03,
            gems: true,
            embers: 18,
            ember_speed: 1.0,
            show_self: true,
            show_others: true,
            range: 48.0,
        }
    }
}

impl CrownConfig {
    pub(crate) fn from_data(data: &ModuleData) -> Self {
        let slider = |name: &str, fallback: f32, min: f32, max: f32| {
            let value = slider_setting(data, name, fallback);
            if value.is_finite() {
                value.clamp(min, max)
            } else {
                fallback
            }
        };
        Self {
            enabled: data.enabled,
            style: CrownStyle::from_index(choice_setting(data, "Style").unwrap_or(0)),
            band: color_setting(data, "Band Color", DEFAULT_BAND),
            flame: color_setting(data, "Flame Color", DEFAULT_FLAME),
            core: color_setting(data, "Core Color", DEFAULT_CORE),
            gem: color_setting(data, "Gem Color", DEFAULT_GEM),
            rainbow: toggle_setting(data, "Rainbow", false),
            glow: toggle_setting(data, "Glow", true),
            glow_strength: slider("Glow Strength", 1.0, 0.0, 2.0),
            opacity: slider("Opacity", 0.95, 0.2, 1.0),
            size: slider("Size", 1.0, 0.6, 2.0),
            hover: slider("Hover", 0.0, 0.0, 0.6) as f64,
            points: slider("Points", 8.0, 3.0, 16.0).round() as usize,
            spike_height: slider("Spike Height", 0.22, 0.05, 0.5) as f64,
            band_height: slider("Band Height", 0.10, 0.03, 0.2) as f64,
            thickness: slider("Thickness", 1.0, 0.4, 2.5),
            flame_length: slider("Flame Length", 0.45, 0.1, 0.9) as f64,
            waviness: slider("Flame Waviness", 0.55, 0.0, 1.0),
            flame_speed: slider("Flame Speed", 1.6, 0.2, 4.0),
            flicker: slider("Flicker", 0.6, 0.0, 1.0),
            spin: slider("Spin Speed", 0.35, 0.0, 2.0),
            bob: slider("Bob", 0.03, 0.0, 0.15) as f64,
            gems: toggle_setting(data, "Gems", true),
            embers: slider("Embers", 18.0, 0.0, 40.0).round() as usize,
            ember_speed: slider("Ember Speed", 1.0, 0.2, 3.0),
            show_self: toggle_setting(data, "Show Self", true),
            show_others: toggle_setting(data, "Show Others", true),
            range: slider("Range", 48.0, 8.0, 128.0) as f64,
        }
    }
}

 
pub(crate) fn read_config() -> CrownConfig {
    let mut cfg = CrownConfig::default();
    if let Some(arc) = client().modules.get(ModuleId::Crown) {
        if let Ok(module) = arc.lock() {
            cfg = CrownConfig::from_data(module.get_module_data());
        }
    }
    cfg
}

 

 
#[derive(Debug, Clone, Copy)]
pub(crate) struct CrownSample {
    pub(crate) id: i32,
    pub(crate) pos: V3,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

#[derive(Debug, Clone, Copy)]
struct CrownTarget {
    id: i32,
    prev: V3,
    pos: V3,
    width: f64,
    height: f64,
    seed: f32,
}

#[derive(Default)]
struct CrownState {
    targets: Vec<CrownTarget>,
    at: Option<Instant>,
}

fn state() -> &'static Mutex<CrownState> {
    static STATE: OnceLock<Mutex<CrownState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(CrownState::default()))
}

 
 
pub(crate) fn publish(samples: Vec<CrownSample>, at: Instant) {
    let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
    let targets = samples
        .into_iter()
        .map(|sample| {
            let previous = state
                .targets
                .iter()
                .find(|target| target.id == sample.id)
                .map(|target| target.pos);
            CrownTarget {
                id: sample.id,
                prev: previous.unwrap_or(sample.pos),
                pos: sample.pos,
                width: sample.width,
                height: sample.height,
                seed: seed_of(sample.id),
            }
        })
        .collect();
    state.targets = targets;
    state.at = Some(at);
}

 
pub(crate) fn reset() {
    let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
    state.targets.clear();
    state.at = None;
}

fn seed_of(id: i32) -> f32 {
    let mixed = (id as u32).wrapping_mul(2_654_435_761) ^ 0x9E37_79B9;
    (mixed % 10_007) as f32 / 10_007.0
}

 
fn elapsed() -> f32 {
    static CLOCK: OnceLock<Instant> = OnceLock::new();
    CLOCK.get_or_init(Instant::now).elapsed().as_secs_f32()
}

 

fn v3(x: f64, y: f64, z: f64) -> V3 {
    V3 { x, y, z }
}

 
fn ring_point(centre: V3, radius: f64, angle: f64, y: f64) -> V3 {
    v3(
        centre.x + radius * angle.cos(),
        y,
        centre.z + radius * angle.sin(),
    )
}

fn depth_of(view: &View, point: V3) -> f64 {
    point.sub(view.cam).dot(view.fwd)
}

 
fn pixels_per_block(view: &View, point: V3) -> Option<f32> {
    let depth = depth_of(view, point);
    if depth < 0.05 {
        return None;
    }
    Some((view.h as f64 * 0.5 / (view.tan_y * depth)) as f32)
}

fn project_path(view: &View, points: &[V3]) -> Option<Vec<Pos2>> {
    points.iter().map(|point| view.project(*point)).collect()
}

fn hash01(value: u32) -> f32 {
    let mut x = value.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    x = ((x ^ (x >> 16)).wrapping_mul(2_246_822_519)) ^ 0x9E37_79B9;
    ((x ^ (x >> 13)) & 0x00FF_FFFF) as f32 / 16_777_216.0
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let blend = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(
        blend(a.r(), b.r()),
        blend(a.g(), b.g()),
        blend(a.b(), b.b()),
    )
}

fn shade(color: Color32, factor: f32) -> Color32 {
    let scale = |channel: u8| ((channel as f32 * factor).clamp(0.0, 255.0)).round() as u8;
    Color32::from_rgb(scale(color.r()), scale(color.g()), scale(color.b()))
}

fn alpha(color: Color32, value: f32) -> Color32 {
    theme::with_alpha(color, value.clamp(0.0, 1.0))
}

fn hue_color(hue: f32) -> Color32 {
    let h = hue.rem_euclid(1.0) * 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(channel(r), channel(g), channel(b))
}

 
fn palette_color(cfg: &CrownConfig, base: Color32, hue: f32) -> Color32 {
    if cfg.rainbow {
        hue_color(hue)
    } else {
        base
    }
}

 
 
 
#[allow(clippy::too_many_arguments)]
fn flame_spine(
    cfg: &CrownConfig,
    base: V3,
    outward: V3,
    tangent: V3,
    time: f32,
    seed: f32,
    length_scale: f64,
) -> Vec<V3> {
    let length = cfg.flame_length * cfg.size as f64 * length_scale;
    let steps = FLAME_STEPS;
    let mut spine = Vec::with_capacity(steps + 1);
    for step in 0..=steps {
        let t = step as f64 / steps as f64;
        let flicker = 1.0
            + cfg.flicker as f64
                * 0.28
                * ((time * cfg.flame_speed * 3.1 + seed * 9.0) as f64).sin()
                * t;
        let sway = cfg.waviness as f64
            * (0.075 * ((time * cfg.flame_speed * 1.7 + t as f32 * 3.2 + seed * 7.0) as f64).sin()
                + 0.04
                    * ((time * cfg.flame_speed * 3.3 + t as f32 * 6.1 + seed * 13.0) as f64).sin())
            * t;
        spine.push(v3(
            base.x + outward.x * sway + tangent.x * sway * 0.7,
            base.y + length * t * flicker,
            base.z + outward.z * sway + tangent.z * sway * 0.7,
        ));
    }
    spine
}

 
fn ember_point(
    cfg: &CrownConfig,
    centre: V3,
    radius: f64,
    index: usize,
    time: f32,
    seed: f32,
) -> (V3, f32) {
    let h1 = hash01(index as u32 * 3 + 1);
    let h2 = hash01(index as u32 * 7 + 2);
    let h3 = hash01(index as u32 * 11 + 5);
    let rise = cfg.flame_length * cfg.size as f64 * 1.6 + 0.12;
    let phase = (time * cfg.ember_speed * (0.35 + 0.45 * h1) + h2 + seed).fract();
    let angle = h3 * TAU as f32 + time * cfg.spin * 0.6 + phase * 0.7;
    let r = radius * (0.55 + 0.75 * h1 as f64) + phase as f64 * 0.05;
    (
        v3(
            centre.x + r * angle.cos() as f64,
            centre.y + cfg.band_height + phase as f64 * rise,
            centre.z + r * angle.sin() as f64,
        ),
        phase,
    )
}

 

type Piece = (f64, Shape);

fn quad(corners: [Pos2; 4], colors: [Color32; 4]) -> Shape {
    let mut mesh = Mesh::default();
    for (corner, color) in corners.iter().zip(colors) {
        mesh.colored_vertex(*corner, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    Shape::mesh(mesh)
}

 
#[allow(clippy::too_many_arguments)]
fn add_ribbon(out: &mut Vec<Piece>, path: &[Pos2], widths: &[f32], colors: &[Color32], depth: f64) {
    if path.len() < 2 || widths.len() != path.len() || colors.len() != path.len() {
        return;
    }
    let mut mesh = Mesh::default();
    for i in 0..path.len() {
        let direction = if i == 0 {
            path[1] - path[0]
        } else if i == path.len() - 1 {
            path[i] - path[i - 1]
        } else {
            path[i + 1] - path[i - 1]
        };
        let length = direction.length().max(1e-3);
        let normal = vec2(-direction.y / length, direction.x / length) * widths[i].max(0.0);
        mesh.colored_vertex(path[i] + normal, colors[i]);
        mesh.colored_vertex(path[i] - normal, colors[i]);
    }
    for i in 0..path.len() - 1 {
        let base = (i * 2) as u32;
        mesh.add_triangle(base, base + 1, base + 3);
        mesh.add_triangle(base, base + 3, base + 2);
    }
    out.push((depth, Shape::mesh(mesh)));
}

 
fn halo(centre: Pos2, radius: f32, color: Color32, strength: f32) -> Shape {
    const STEPS: u32 = 28;
    let clear = alpha(color, 0.0);
    let mut mesh = Mesh::default();
    mesh.colored_vertex(centre, alpha(color, strength));
    for i in 0..=STEPS {
        let angle = i as f32 / STEPS as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(centre + vec2(angle.cos(), angle.sin()) * radius, clear);
    }
    for i in 0..STEPS {
        mesh.add_triangle(0, i + 1, i + 2);
    }
    Shape::mesh(mesh)
}

#[allow(clippy::too_many_arguments)]
fn add_ring_line(
    out: &mut Vec<Piece>,
    view: &View,
    centre: V3,
    radius: f64,
    y: f64,
    spin: f64,
    width: f32,
    color: Color32,
    glow: bool,
    opacity: f32,
) {
    let points: Vec<V3> = (0..=SEGMENTS)
        .map(|i| ring_point(centre, radius, TAU * i as f64 / SEGMENTS as f64 + spin, y))
        .collect();
    let Some(path) = project_path(view, &points) else {
        return;
    };
    let depth = depth_of(view, v3(centre.x, y, centre.z));
    if glow {
        out.push((
            depth + 0.05,
            Shape::line(
                path.clone(),
                Stroke::new(width * 3.4, alpha(color, 0.18 * opacity)),
            ),
        ));
    }
    out.push((
        depth,
        Shape::line(path, Stroke::new(width, alpha(color, 0.95 * opacity))),
    ));
}

#[allow(clippy::too_many_arguments)]
fn add_band(
    out: &mut Vec<Piece>,
    view: &View,
    cfg: &CrownConfig,
    centre: V3,
    radius: f64,
    y0: f64,
    y1: f64,
    spin: f64,
) {
    for segment in 0..SEGMENTS {
        let a0 = TAU * segment as f64 / SEGMENTS as f64 + spin;
        let a1 = TAU * (segment + 1) as f64 / SEGMENTS as f64 + spin;
        let corners = [
            ring_point(centre, radius, a0, y0),
            ring_point(centre, radius, a1, y0),
            ring_point(centre, radius, a1, y1),
            ring_point(centre, radius, a0, y1),
        ];
        let Some(screen) = project_path(view, &corners) else {
            continue;
        };
         
        let mid_angle = (a0 + a1) * 0.5;
        let normal = v3(mid_angle.cos(), 0.0, mid_angle.sin());
        let to_camera = view
            .cam
            .sub(ring_point(centre, radius, mid_angle, (y0 + y1) * 0.5));
        let facing = (normal.dot(to_camera) / to_camera.length().max(1e-6)).max(0.0) as f32;
        let low = shade(cfg.band, 0.28 + 0.42 * facing);
        let high = mix(
            shade(cfg.band, 0.85 + 0.60 * facing),
            cfg.flame,
            0.10 + 0.16 * facing,
        );
        let colors = [
            alpha(low, cfg.opacity),
            alpha(low, cfg.opacity),
            alpha(high, cfg.opacity),
            alpha(high, cfg.opacity),
        ];
        let depth = corners
            .iter()
            .map(|corner| depth_of(view, *corner))
            .sum::<f64>()
            / corners.len() as f64;
        out.push((
            depth,
            quad(screen.try_into().expect("four corners"), colors),
        ));
    }
}

 
fn add_prong(
    out: &mut Vec<Piece>,
    view: &View,
    cfg: &CrownConfig,
    centre: V3,
    radius: f64,
    y1: f64,
    angle: f64,
) {
    let half = 0.15;
    let base_left = ring_point(centre, radius, angle - half, y1);
    let base_right = ring_point(centre, radius, angle + half, y1);
    let apex = ring_point(
        centre,
        radius * 0.98,
        angle,
        y1 + cfg.spike_height * cfg.size as f64,
    );
    let Some(mut screen) = project_path(view, &[base_left, base_right, apex]) else {
        return;
    };
     
     
     
     
    let midpoint = Pos2::new(
        (screen[0].x + screen[1].x) * 0.5,
        (screen[0].y + screen[1].y) * 0.5,
    );
    let rise = screen[2] - midpoint;
    let axis = if rise.length() > 1e-3 {
        rise.normalized()
    } else {
        vec2(0.0, -1.0)
    };
    if (screen[0] - screen[1]).length() < 2.0 * MIN_PRONG_BASE {
        let across = vec2(-axis.y, axis.x) * MIN_PRONG_BASE;
        screen[0] = midpoint + across;
        screen[1] = midpoint - across;
    }
     
     
    let depth = depth_of(view, apex) - 0.02;
     
     
     
    let base = alpha(shade(cfg.band, 0.55), cfg.opacity);
    let tip = alpha(mix(shade(cfg.band, 1.35), cfg.core, 0.55), cfg.opacity);
    let mut mesh = Mesh::default();
    mesh.colored_vertex(screen[0], base);
    mesh.colored_vertex(screen[1], base);
    mesh.colored_vertex(screen[2], tip);
    mesh.add_triangle(0, 1, 2);
    out.push((depth, Shape::mesh(mesh)));
     
     
     
    let edge = Stroke::new(
        (0.9 * cfg.thickness).max(0.8_f32),
        alpha(cfg.core, 0.35 * cfg.opacity),
    );
    for (from, to) in [(screen[0], screen[2]), (screen[1], screen[2])] {
        out.push((depth - 0.005, Shape::line_segment([from, to], edge)));
    }
     
    if let (Some(root), Some(tip)) = (
        view.project(ring_point(centre, radius, angle, y1)),
        view.project(apex),
    ) {
        out.push((
            depth - 0.01,
            Shape::line_segment(
                [root, tip],
                Stroke::new(
                    (1.2 * cfg.thickness).max(1.0),
                    alpha(cfg.core, 0.75 * cfg.opacity),
                ),
            ),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn add_flame(
    out: &mut Vec<Piece>,
    view: &View,
    cfg: &CrownConfig,
    centre: V3,
    radius: f64,
    y: f64,
    angle: f64,
    time: f32,
    seed: f32,
    length_scale: f64,
) {
    let Some(ppb) = pixels_per_block(view, ring_point(centre, radius, angle, y)) else {
        return;
    };
    let outward = v3(angle.cos(), 0.0, angle.sin());
    let tangent = v3(-angle.sin(), 0.0, angle.cos());
    let base = ring_point(centre, radius * 0.99, angle, y);
    let spine = flame_spine(cfg, base, outward, tangent, time, seed, length_scale);
    let Some(path) = project_path(view, &spine) else {
        return;
    };
    let hue = (angle / TAU) as f32 + seed;
    let base_width = (ppb * 0.038 * cfg.size * cfg.thickness).max(0.8);
    let mut widths = Vec::with_capacity(spine.len());
    let mut colors = Vec::with_capacity(spine.len());
    let mut glow_colors = Vec::with_capacity(spine.len());
    let tip = shade(cfg.flame, 0.5);
    for step in 0..spine.len() {
        let t = step as f32 / (spine.len() - 1) as f32;
        widths.push(base_width * (1.0 - t).powf(0.62) + 0.35);
         
         
        let hot = if cfg.rainbow {
            hue_color(hue + t * 0.14 + time * 0.05)
        } else if t < 0.45 {
            mix(cfg.core, cfg.flame, t / 0.45)
        } else {
            mix(cfg.flame, tip, (t - 0.45) / 0.55)
        };
        let fade = (1.0 - t).powf(1.35);
        colors.push(alpha(hot, cfg.opacity * 0.95 * fade));
        glow_colors.push(alpha(hot, cfg.opacity * 0.15 * fade));
    }
    let depth = depth_of(view, base);
    if cfg.glow {
        let wide: Vec<f32> = widths.iter().map(|width| width * 2.2).collect();
        add_ribbon(out, &path, &wide, &glow_colors, depth + 0.08);
    }
    add_ribbon(out, &path, &widths, &colors, depth);
     
     
    let core_widths: Vec<f32> = widths.iter().map(|width| width * 0.34).collect();
    let core_colors: Vec<Color32> = (0..spine.len())
        .map(|step| {
            let t = step as f32 / (spine.len() - 1) as f32;
            alpha(cfg.core, cfg.opacity * 0.55 * (1.0 - t).powf(2.0))
        })
        .collect();
    add_ribbon(out, &path, &core_widths, &core_colors, depth - 0.005);
    if let Some(tip) = path.first() {
        out.push((
            depth - 0.01,
            Shape::circle_filled(*tip, base_width * 0.55, alpha(cfg.core, 0.75 * cfg.opacity)),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn add_gem(
    out: &mut Vec<Piece>,
    view: &View,
    cfg: &CrownConfig,
    centre: V3,
    radius: f64,
    y: f64,
    angle: f64,
    time: f32,
    seed: f32,
) {
    let point = ring_point(centre, radius * 1.01, angle, y);
    let (Some(screen), Some(ppb)) = (view.project(point), pixels_per_block(view, point)) else {
        return;
    };
    let size = (ppb * 0.030 * cfg.size * cfg.thickness).clamp(1.2, 9.0);
    let color = palette_color(cfg, cfg.gem, (angle / TAU) as f32 + seed + time * 0.05);
    let depth = depth_of(view, point);
    if cfg.glow {
        out.push((
            depth + 0.02,
            halo(
                screen,
                size * 4.4,
                color,
                0.5 * cfg.glow_strength * cfg.opacity,
            ),
        ));
    }
    out.push((
        depth,
        Shape::circle_filled(screen, size, alpha(color, cfg.opacity)),
    ));
    out.push((
        depth - 0.01,
        Shape::circle_filled(
            screen,
            size * 0.42,
            alpha(Color32::WHITE, 0.8 * cfg.opacity),
        ),
    ));
}

#[allow(clippy::too_many_arguments)]
fn add_embers(
    out: &mut Vec<Piece>,
    view: &View,
    cfg: &CrownConfig,
    centre: V3,
    radius: f64,
    time: f32,
    seed: f32,
) {
    if cfg.embers == 0 {
        return;
    }
    for index in 0..cfg.embers {
        let (point, life) = ember_point(cfg, centre, radius, index, time, seed);
        let (Some(screen), Some(ppb)) = (view.project(point), pixels_per_block(view, point)) else {
            continue;
        };
        let fade = (life * std::f32::consts::PI).sin();
        if fade <= 0.02 {
            continue;
        }
        let size = (ppb * 0.012 * cfg.size).clamp(0.5, 3.2);
        let color = palette_color(
            cfg,
            mix(cfg.flame, cfg.core, hash01(index as u32 + 9)),
            life + seed,
        );
        out.push((
            depth_of(view, point),
            Shape::circle_filled(screen, size, alpha(color, cfg.opacity * fade * 0.9)),
        ));
    }
}

fn draw_one(
    painter: &Painter,
    view: &View,
    cfg: &CrownConfig,
    target: &CrownTarget,
    feet: V3,
    time: f32,
) {
    let style = cfg.style;
    let bob = cfg.bob * ((time * 1.7 + target.seed * TAU as f32).sin() as f64);
    let radius = (target.width * 0.42).max(0.16) * cfg.size as f64;
    let band_height = cfg.band_height * cfg.size as f64;
     
     
     
    let centre = v3(
        feet.x,
        feet.y + target.height - band_height - 0.01 + cfg.hover + style.lift() + bob,
        feet.z,
    );
    if view.cam.sub(centre).length() < MIN_DRAW_DISTANCE {
         
         
        return;
    }
    let y0 = centre.y;
    let y1 = centre.y + band_height;
    let spin = time as f64 * cfg.spin as f64 + target.seed as f64 * TAU;
    let seed = target.seed;

    let mut pieces: Vec<Piece> = Vec::new();
    let depth = depth_of(view, centre);

     
    if cfg.glow {
        if let (Some(screen), Some(ppb)) = (view.project(centre), pixels_per_block(view, centre)) {
            let glow_radius = ppb * (radius as f32 * 2.4 + 0.12);
            pieces.push((
                depth + 4.0,
                halo(
                    screen,
                    glow_radius,
                    cfg.flame,
                    0.30 * cfg.glow_strength * cfg.opacity,
                ),
            ));
        }
    }

    if style.metal_band() {
        add_band(&mut pieces, view, cfg, centre, radius, y0, y1, spin);
        add_ring_line(
            &mut pieces,
            view,
            centre,
            radius,
            y0,
            spin,
            (1.0 * cfg.thickness).max(0.8),
            shade(cfg.band, 0.55),
            false,
            cfg.opacity * 0.8,
        );
        add_ring_line(
            &mut pieces,
            view,
            centre,
            radius,
            y1,
            spin,
            (1.4 * cfg.thickness).max(1.0),
            palette_color(cfg, cfg.core, time * 0.05 + seed),
            cfg.glow,
            cfg.opacity * 0.85,
        );
    } else {
         
        add_band_ring(
            &mut pieces,
            view,
            cfg,
            centre,
            radius,
            y0,
            band_height,
            spin,
        );
        add_ring_line(
            &mut pieces,
            view,
            centre,
            radius,
            y0 + band_height * 0.5,
            spin,
            (2.0 * cfg.thickness).max(1.0),
            palette_color(cfg, cfg.core, time * 0.05 + seed),
            cfg.glow,
            cfg.opacity,
        );
    }

    for index in 0..cfg.points {
        let angle = spin + TAU * index as f64 / cfg.points as f64;
        let hue = index as f32 / cfg.points as f32 + seed;
        if cfg.gems {
            add_gem(
                &mut pieces,
                view,
                cfg,
                centre,
                radius,
                y0 + band_height * 0.45,
                angle,
                time,
                hue,
            );
        }
        if style.prongs() {
            add_prong(&mut pieces, view, cfg, centre, radius, y1, angle);
            add_flame(
                &mut pieces,
                view,
                cfg,
                centre,
                radius * 0.98,
                y1 + cfg.spike_height * cfg.size as f64,
                angle,
                time,
                hue,
                0.45,
            );
        } else {
            add_flame(
                &mut pieces,
                view,
                cfg,
                centre,
                radius,
                y1,
                angle,
                time,
                hue,
                style.flame_scale(),
            );
        }
    }

    add_embers(&mut pieces, view, cfg, centre, radius, time, seed);

    pieces.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, shape) in pieces {
        painter.add(shape);
    }
}

 
#[allow(clippy::too_many_arguments)]
fn add_band_ring(
    out: &mut Vec<Piece>,
    view: &View,
    cfg: &CrownConfig,
    centre: V3,
    radius: f64,
    y: f64,
    height: f64,
    spin: f64,
) {
    for segment in 0..SEGMENTS {
        let a0 = TAU * segment as f64 / SEGMENTS as f64 + spin;
        let a1 = TAU * (segment + 1) as f64 / SEGMENTS as f64 + spin;
        let corners = [
            ring_point(centre, radius, a0, y),
            ring_point(centre, radius, a1, y),
            ring_point(centre, radius, a1, y + height),
            ring_point(centre, radius, a0, y + height),
        ];
        let Some(screen) = project_path(view, &corners) else {
            continue;
        };
        let hue = (a0 / TAU) as f32 + cfg.flame_speed;
        let base = palette_color(cfg, cfg.flame, hue);
        let color = alpha(mix(base, cfg.core, 0.25), cfg.opacity * 0.35);
        let depth = corners
            .iter()
            .map(|corner| depth_of(view, *corner))
            .sum::<f64>()
            / corners.len() as f64;
        out.push((
            depth,
            quad(
                screen.try_into().expect("four corners"),
                [
                    color,
                    color,
                    alpha(base, cfg.opacity * 0.55),
                    alpha(base, cfg.opacity * 0.55),
                ],
            ),
        ));
    }
}

 
 
pub(crate) fn draw(painter: &Painter, view: &View, now: Instant, cfg: &CrownConfig) {
    if !cfg.enabled {
        return;
    }
    let (targets, at) = {
        let state = state().lock().unwrap_or_else(|p| p.into_inner());
        (state.targets.clone(), state.at)
    };
    let Some(at) = at else {
        return;
    };
    let t = (now.saturating_duration_since(at).as_secs_f32() / TICK.as_secs_f32()).clamp(0.0, 1.0)
        as f64;
    let time = elapsed();
    for target in &targets {
        let feet = if target.prev.sub(target.pos).length() > TELEPORT {
            target.pos
        } else {
            target.prev.lerp(target.pos, t)
        };
        draw_one(painter, view, cfg, target, feet, time);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::render::crown::CrownModule;

    fn config() -> CrownConfig {
        CrownConfig::from_data(&CrownModule::new().module)
    }

    fn test_view(w: f32, h: f32) -> View {
        let tan_y = 0.34_f64;
        View {
            cam: v3(0.0, 65.9, 0.0),
            fwd: v3(0.0, 0.0, 1.0),
            right: v3(-1.0, 0.0, 0.0),
            up: v3(0.0, 1.0, 0.0),
            tan_x: tan_y * (w / h) as f64,
            tan_y,
            w,
            h,
        }
    }

     
     
    fn draw_head(painter: &Painter, view: &View, feet: V3, width: f64, height: f64) {
        let head = 0.5;
        let y0 = feet.y + height - head - 0.05;
        let y1 = feet.y + height;
        let mut corners = [feet; 8];
        for (index, corner) in corners.iter_mut().enumerate() {
            let x = if index & 1 == 0 { -1.0 } else { 1.0 };
            let y = if index & 2 == 0 { y0 } else { y1 };
            let z = if index & 4 == 0 { -1.0 } else { 1.0 };
            *corner = v3(feet.x + x * width * 0.42, y, feet.z + z * width * 0.42);
        }
        let edges = [
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
        for (a, b) in edges {
            if let (Some(pa), Some(pb)) = (view.project(corners[a]), view.project(corners[b])) {
                painter.line_segment([pa, pb], Stroke::new(1.2_f32, Color32::from_gray(70)));
            }
        }
    }

    fn sample_target() -> CrownTarget {
        CrownTarget {
            id: 7,
            prev: v3(0.0, 64.1, 4.0),
            pos: v3(0.0, 64.1, 4.0),
            width: 0.6,
            height: 1.8,
            seed: 0.42,
        }
    }

    #[test]
    fn defaults_are_a_fire_crown() {
        let cfg = config();
        assert_eq!(cfg.style, CrownStyle::Inferno);
        assert!(cfg.glow);
        assert!(cfg.gems);
        assert_eq!(cfg.points, 8);
        assert!(cfg.embers >= 8);
        assert!(cfg.size > 0.5);
        assert!(cfg.flame_length > 0.0);
    }

    #[test]
    fn sliders_clamp_and_non_finite_values_fall_back() {
        let mut data = CrownModule::new().module;
        for (name, value) in [
            ("Size", 99.0),
            ("Points", f32::NAN),
            ("Opacity", -3.0),
            ("Embers", 100.0),
        ] {
            data.get_setting_mut(name)
                .expect("slider setting")
                .set_slider_value(value);
        }
        let cfg = CrownConfig::from_data(&data);
        assert_eq!(cfg.size, 2.0);
        assert_eq!(cfg.points, 8, "a NaN slider falls back to its default");
        assert_eq!(cfg.opacity, 0.2);
        assert_eq!(cfg.embers, 40);
    }

    #[test]
    fn every_style_has_a_name_and_round_trips() {
        for (index, name) in CrownStyle::OPTIONS.iter().enumerate() {
            let style = CrownStyle::from_index(index);
            assert!(!name.is_empty());
            assert_eq!(style, CrownStyle::from_index(index));
        }
        assert_eq!(CrownStyle::from_index(99), CrownStyle::Inferno);
    }

    #[test]
    fn the_band_sits_around_the_head() {
        let centre = v3(10.0, 65.0, -4.0);
        let points: Vec<V3> = (0..SEGMENTS)
            .map(|i| ring_point(centre, 0.25, TAU * i as f64 / SEGMENTS as f64, 65.1))
            .collect();
        for point in &points {
            let radius = ((point.x - centre.x).powi(2) + (point.z - centre.z).powi(2)).sqrt();
            assert!((radius - 0.25).abs() < 1e-9, "ring is not circular");
            assert!((point.y - 65.1).abs() < 1e-9, "ring is not level");
        }
    }

    #[test]
    fn a_flame_rises_from_its_base_and_never_goes_negative() {
        let cfg = config();
        let base = v3(0.0, 66.0, 0.0);
        for time in [0.0, 0.7, 3.4] {
            let spine = flame_spine(
                &cfg,
                base,
                v3(1.0, 0.0, 0.0),
                v3(0.0, 0.0, 1.0),
                time,
                0.3,
                1.0,
            );
            assert_eq!(spine.len(), FLAME_STEPS + 1);
            assert!((spine[0].y - base.y).abs() < 1e-9);
            for step in &spine {
                assert!(step.y >= base.y - 1e-9, "a flame dipped below its base");
                assert!(
                    step.x.is_finite() && step.y.is_finite() && step.z.is_finite(),
                    "flame geometry is not finite"
                );
            }
            let last = spine.last().expect("spine");
            assert!(
                last.y > base.y + cfg.flame_length * 0.2,
                "the flame must rise"
            );
        }
    }

    #[test]
    fn embers_spawn_and_expire_inside_their_life() {
        let cfg = config();
        let centre = v3(0.0, 66.0, 0.0);
        let mut seen_low = false;
        let mut seen_high = false;
        for index in 0..cfg.embers {
            for time in [0.0, 1.3, 5.9] {
                let (point, life) = ember_point(&cfg, centre, 0.25, index, time, 0.2);
                assert!((0.0..1.0).contains(&life), "life out of range: {life}");
                assert!(point.y >= centre.y, "an ember sank below the band");
                assert!(point.x.is_finite() && point.z.is_finite());
                if life < 0.3 {
                    seen_low = true;
                }
                if life > 0.6 {
                    seen_high = true;
                }
            }
        }
        assert!(
            seen_low && seen_high,
            "embers must be spread through the rise"
        );
    }

     
     
    #[test]
    fn a_crown_around_the_camera_is_not_drawn() {
        let cfg = config();
        let target = sample_target();
        let feet = target.pos;
        let centre = v3(
            feet.x,
            feet.y + target.height - cfg.band_height - 0.01 + cfg.hover,
            feet.z,
        );
        let view = View {
            cam: centre,
            fwd: v3(0.0, 0.0, 1.0),
            right: v3(-1.0, 0.0, 0.0),
            up: v3(0.0, 1.0, 0.0),
            tan_x: 0.9,
            tan_y: 0.7,
            w: 480.0,
            h: 360.0,
        };
        let ctx = egui::Context::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(480.0, 360.0))),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("crown_first_person"),
                ));
                draw_one(&painter, &view, &cfg, &target, feet, 1.0);
            },
        );
        assert!(
            output.shapes.is_empty(),
            "the local crown must stay out of first person"
        );
    }

    #[test]
    fn every_style_draws_something_and_never_text() {
        let cfg = config();
        let view = test_view(480.0, 360.0);
        for style in [CrownStyle::Inferno, CrownStyle::Halo, CrownStyle::Spiked] {
            let mut cfg = cfg.clone();
            cfg.style = style;
            let ctx = egui::Context::default();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(480.0, 360.0))),
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Background,
                        egui::Id::new("crown_test"),
                    ));
                    draw_one(
                        &painter,
                        &view,
                        &cfg,
                        &sample_target(),
                        sample_target().pos,
                        1.7,
                    );
                },
            );
            let shapes = output
                .shapes
                .iter()
                .filter(|shape| !matches!(shape.shape, egui::Shape::Noop | egui::Shape::Text(_)))
                .count();
            assert!(shapes > 8, "{style:?} drew only {shapes} shapes");
            assert!(
                !output
                    .shapes
                    .iter()
                    .any(|shape| matches!(shape.shape, egui::Shape::Text(_))),
                "{style:?} must not draw text"
            );
        }
    }

     
     
     
     
    #[test]
    fn no_piece_paints_far_outside_its_own_shape() {
        let cfg = config();
        let view = test_view(480.0, 400.0);
        for style in [CrownStyle::Inferno, CrownStyle::Halo, CrownStyle::Spiked] {
            let mut cfg = cfg.clone();
            cfg.style = style;
            let ctx = egui::Context::default();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(480.0, 400.0))),
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Background,
                        egui::Id::new("crown_bounds"),
                    ));
                    let target = sample_target();
                    draw_one(&painter, &view, &cfg, &target, target.pos, 1.7);
                },
            );
             
             
            let allowed = egui::Rect::from_min_max(
                Pos2::new(-60.0, -60.0),
                Pos2::new(480.0 + 60.0, 400.0 + 60.0),
            );
            for clipped in &output.shapes {
                for primitive in ctx.tessellate(vec![clipped.clone()], 1.0) {
                    let egui::epaint::Primitive::Mesh(mesh) = &primitive.primitive else {
                        continue;
                    };
                    for vertex in &mesh.vertices {
                        assert!(
                            allowed.contains(vertex.pos),
                            "{style:?} painted {:?} outside {allowed:?}",
                            vertex.pos
                        );
                    }
                }
            }
        }
    }

     
     
    #[test]
    fn the_preview_sheet_renders() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        const CELL_W: usize = 640;
        const CELL_H: usize = 520;
        let view = test_view(CELL_W as f32, CELL_H as f32);
        let mut sheet = image::RgbaImage::from_pixel(
            CELL_W as u32 * 3,
            CELL_H as u32,
            image::Rgba([11, 12, 16, 255]),
        );
        for (index, style) in [CrownStyle::Inferno, CrownStyle::Halo, CrownStyle::Spiked]
            .iter()
            .enumerate()
        {
            let mut cfg = config();
            cfg.style = *style;
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
                        egui::Id::new("crown_preview"),
                    ));
                    let target = sample_target();
                    draw_head(&painter, &view, target.pos, target.width, target.height);
                    draw_one(&painter, &view, &cfg, &target, target.pos, 1.7);
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
                .filter(|pixel| pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32 > 90)
                .count();
            assert!(lit > 900, "{style:?} rendered almost nothing ({lit})");
            image::imageops::replace(&mut sheet, &cell, (index * CELL_W) as i64, 0);
        }
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/crown-preview.png");
        sheet.save(&path).expect("the crown preview png saves");
    }
}
