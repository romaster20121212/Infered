 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::esp::{color_setting, toggle_setting, View, V3};
use crate::graphic::theme;
use crate::module::{ModuleData, ModuleId, ModuleSetting};
use crate::state::client;
use egui::{Color32, Painter, Stroke};
use std::f64::consts::TAU;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const DEFAULT_LINES: f32 = 24.0;
const DEFAULT_SWITCHES: f32 = 14.0;
const DEFAULT_SPEED: f32 = 1.2;
const DEFAULT_RANGE: f32 = 14.0;
const DEFAULT_SPREAD: f32 = 4.0;
const DEFAULT_FADE: f32 = 0.7;
const DEFAULT_WIDTH: f32 = 2.2;
const DEFAULT_COLOR: Color32 = Color32::from_rgb(51, 229, 255);
const MAX_LINES: usize = 32;
 
const TELEPORT_DISTANCE: f64 = 64.0;
 
const TAIL_STEPS: f32 = 14.0;
 
 
 
const HALO_LAYERS: [(f32, f32); 2] = [(5.6, 0.10), (2.2, 0.26)];
 
const DIRECTIONS: [[f64; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];

 
#[derive(Debug, Clone, Copy)]
pub(crate) struct AuraConfig {
    pub(crate) enabled: bool,
    lines: usize,
    switches: usize,
    speed: f32,
    range: f64,
    spread: f64,
    fade: Duration,
    width: f32,
    color: Color32,
    rainbow: bool,
    glow: bool,
}

impl Default for AuraConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            lines: DEFAULT_LINES as usize,
            switches: DEFAULT_SWITCHES as usize,
            speed: DEFAULT_SPEED,
            range: DEFAULT_RANGE as f64,
            spread: DEFAULT_SPREAD as f64,
            fade: Duration::from_secs_f32(DEFAULT_FADE),
            width: DEFAULT_WIDTH,
            color: DEFAULT_COLOR,
            rainbow: false,
            glow: true,
        }
    }
}

impl AuraConfig {
    pub(crate) fn from_data(data: &ModuleData) -> Self {
        let slider = |name: &str, fallback: f32, min: f32, max: f32| {
            let value = data
                .get_setting(name)
                .and_then(ModuleSetting::get_slider_value)
                .unwrap_or(fallback);
            if value.is_finite() {
                value.clamp(min, max)
            } else {
                fallback
            }
        };
        Self {
            enabled: data.enabled,
            lines: slider("Lines", DEFAULT_LINES, 1.0, MAX_LINES as f32).round() as usize,
            switches: slider("Switches", DEFAULT_SWITCHES, 4.0, 64.0).round() as usize,
            speed: slider("Speed", DEFAULT_SPEED, 0.4, 6.0),
            range: slider("Range", DEFAULT_RANGE, 4.0, 48.0) as f64,
            spread: slider("Spread", DEFAULT_SPREAD, 2.0, 12.0) as f64,
            fade: Duration::from_secs_f32(slider("Fade Time", DEFAULT_FADE, 0.1, 2.0)),
            width: slider("Width", DEFAULT_WIDTH, 0.5, 6.0),
            color: color_setting(data, "Color", DEFAULT_COLOR),
            rainbow: toggle_setting(data, "Rainbow", false),
            glow: toggle_setting(data, "Glow", true),
        }
    }
}

 
#[derive(Debug, Clone)]
struct Line {
     
    points: Vec<V3>,
    seed: f32,
    spawned: Instant,
    step_seconds: f32,
    fade: f32,
}

impl Line {
    fn new(cfg: &AuraConfig, center: V3, now: Instant, stagger: bool) -> Self {
        let seed = crate::module::player::autototem::roll() as f32;
        let start = start_position(center, cfg.range, seed);
        let points = generate_path(start, cfg.switches, cfg.spread, seed);
        let step_seconds = 1.0 / cfg.speed;
        let fade = cfg.fade.as_secs_f32();
        let age = if stagger {
            hash01(0x51, seed) as f32 * (points.len() as f32 * step_seconds + fade) * 0.6
        } else {
            0.0
        };
        Self {
            points,
            seed,
            spawned: now.checked_sub(Duration::from_secs_f32(age)).unwrap_or(now),
            step_seconds,
            fade,
        }
    }

    fn walk_time(&self) -> f32 {
        self.points.len().saturating_sub(1) as f32 * self.step_seconds
    }

    fn lifetime(&self) -> f32 {
        self.walk_time() + self.fade
    }

    fn age(&self, now: Instant) -> f32 {
        now.duration_since(self.spawned).as_secs_f32()
    }
}

#[derive(Default)]
struct AuraState {
    center: Option<V3>,
    lines: Vec<Line>,
}

fn state() -> &'static Mutex<AuraState> {
    static STATE: OnceLock<Mutex<AuraState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(AuraState::default()))
}

pub(crate) fn reset() {
    *state().lock().unwrap_or_else(|p| p.into_inner()) = AuraState::default();
}

 
 
pub(crate) fn read_config() -> AuraConfig {
    let mut cfg = AuraConfig::default();
    if let Some(handle) = client().modules.get(ModuleId::Aura) {
        if let Ok(module) = handle.lock() {
            cfg = AuraConfig::from_data(module.get_module_data());
        }
    }
    if !cfg.enabled {
        reset();
    }
    cfg
}

 
 
pub(crate) fn update(cfg: &AuraConfig, center: V3, now: Instant) {
    if !cfg.enabled {
        reset();
        return;
    }
    let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
    let teleported = state
        .center
        .is_none_or(|old| old.sub(center).length() > TELEPORT_DISTANCE);
    if teleported {
        state.lines.clear();
    }
    state.center = Some(center);
    state.lines.retain(|line| line.age(now) < line.lifetime());
    while state.lines.len() < cfg.lines {
        state.lines.push(Line::new(cfg, center, now, true));
    }
    state.lines.truncate(cfg.lines);
}

fn hash01(index: usize, seed: f32) -> f64 {
    let x = (index as f64 * 12.989_8 + seed as f64 * 78.233).sin() * 43_758.545_3;
    x - x.floor()
}

fn direction(index: usize) -> V3 {
    let [x, y, z] = DIRECTIONS[index % DIRECTIONS.len()];
    V3 { x, y, z }
}

fn step(pos: V3, dir: usize) -> V3 {
    let d = direction(dir);
    V3 {
        x: pos.x + d.x,
        y: pos.y + d.y,
        z: pos.z + d.z,
    }
}

 
 
 
fn start_position(center: V3, range: f64, seed: f32) -> V3 {
    let angle = hash01(0x21, seed) * TAU;
    let radius = range * (0.45 + 0.55 * hash01(0x22, seed));
    let height = (hash01(0x23, seed) - 0.5) * range * 0.35;
    V3 {
        x: (center.x + radius * angle.cos()).round(),
        y: (center.y + height).round(),
        z: (center.z + radius * angle.sin()).round(),
    }
}

 
 
 
 
fn choose_direction(
    pos: V3,
    origin: V3,
    previous: Option<usize>,
    index: usize,
    seed: f32,
    spread: f64,
) -> usize {
    let mut candidates = Vec::with_capacity(5);
    for dir in 0..DIRECTIONS.len() {
        if previous.is_some_and(|previous| dir == (previous ^ 1)) {
            continue;
        }
        candidates.push(dir);
    }
    let pick = (hash01(index, seed) * candidates.len() as f64) as usize % candidates.len();
    let mut fallback = candidates[pick];
    let mut fallback_distance = f64::INFINITY;
    for turn in 0..candidates.len() {
        let dir = candidates[(pick + turn) % candidates.len()];
        let next = step(pos, dir);
        let distance = next.sub(origin).length();
        if distance <= spread {
            return dir;
        }
        if distance < fallback_distance {
            fallback = dir;
            fallback_distance = distance;
        }
    }
    fallback
}

 
fn generate_path(start: V3, segments: usize, spread: f64, seed: f32) -> Vec<V3> {
    let mut pos = start;
    let mut points = Vec::with_capacity(segments + 1);
    points.push(pos);
    let mut previous = None;
    for index in 0..segments {
        let dir = choose_direction(pos, start, previous, index, seed, spread);
        pos = step(pos, dir);
        points.push(pos);
        previous = Some(dir);
    }
    points
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

 
 
 
 
 
fn pulse(phase: f32) -> f32 {
    0.8 + 0.2 * phase.sin().abs()
}

 
fn lighten(color: Color32, amount: f32) -> Color32 {
    let amount = amount.clamp(0.0, 1.0);
    let mix = |channel: u8| (channel as f32 + (255.0 - channel as f32) * amount).round() as u8;
    Color32::from_rgb(mix(color.r()), mix(color.g()), mix(color.b()))
}

fn tint(cfg: &AuraConfig, hue: f32, alpha: f32) -> Color32 {
    let base = if cfg.rainbow { rainbow(hue) } else { cfg.color };
    theme::with_alpha(base, alpha)
}

 
 
pub(crate) fn draw(painter: &Painter, view: &View, now: Instant, cfg: &AuraConfig) {
    if !cfg.enabled {
        reset();
        return;
    }
    let state = state().lock().unwrap_or_else(|p| p.into_inner());
    for line in &state.lines {
        draw_line(painter, view, now, cfg, line);
    }
}

fn smoothstep01(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn draw_line(painter: &Painter, view: &View, now: Instant, cfg: &AuraConfig, line: &Line) {
    let age = line.age(now);
    let walk_time = line.walk_time();
    if age >= line.lifetime() {
        return;
    }
     
    let appear = smoothstep01(age / line.fade);
    let whole = appear
        * if age > walk_time {
            (1.0 - (age - walk_time) / line.fade).clamp(0.0, 1.0)
        } else {
            1.0
        };
    let head = (age / line.step_seconds).clamp(0.0, (line.points.len() - 1) as f32);
    let head_index = head.floor() as usize;
    let fraction = (head - head_index as f32) as f64;

    let local = |index: usize| line.points[index];

    let mut previous = local(0);
    let mut head_pos = previous;
    for index in 0..=head_index {
        let next = if index == head_index {
            let current = local(index);
            if fraction > 1e-4 && index + 1 < line.points.len() {
                head_pos = current.lerp(local(index + 1), fraction);
                head_pos
            } else {
                head_pos = current;
                current
            }
        } else {
            local(index + 1)
        };
        let behind = head - index as f32;
         
         
         
        let tail = (1.0 - behind / TAIL_STEPS)
            .clamp(0.0, 1.0)
            .powf(1.15)
            .max(0.04);
        let hue = line.seed + age * 0.07 + index as f32 * 0.01 - behind * 0.005;
        let color = tint(cfg, hue, whole * tail * pulse(age * 5.0 - behind * 0.8));
         
         
        let width = cfg.width * (0.55 + 0.45 * tail);
        draw_segment(
            painter, view, previous, next, width, color, cfg.glow, cfg.range,
        );
        if index < head_index {
            draw_joint(painter, view, next, width, color, cfg.range);
        }
        previous = next;
    }

     
     
     
    let base = if cfg.rainbow {
        rainbow(line.seed + age * 0.12)
    } else {
        cfg.color
    };
    let head_color = lighten(base, 0.7);
    let spark_color = lighten(base, 0.4);
    for (step, strength) in [(1.0_f32, 0.28_f32), (2.0, 0.12)] {
        let behind = (head - step).max(0.0);
        let index = (behind.floor() as usize).min(line.points.len() - 1);
        let fraction = (behind - behind.floor()) as f64;
        let point = if fraction > 1e-4 && index + 1 < line.points.len() {
            line.points[index].lerp(line.points[index + 1], fraction)
        } else {
            line.points[index]
        };
        if let Some(pixel) = view.project(point) {
            painter.circle_filled(
                pixel,
                cfg.width * (1.5 - 0.35 * step),
                theme::with_alpha(spark_color, whole * strength),
            );
        }
    }
    if let Some(pixel) = view.project(head_pos) {
        let falloff = depth_falloff(head_pos.sub(view.cam).length(), cfg.range);
        for (size, strength) in [(6.2, 0.05), (3.6, 0.13), (2.3, 0.30)] {
            painter.circle_filled(
                pixel,
                cfg.width * size * (0.7 + 0.3 * falloff),
                theme::with_alpha(base, whole * strength * falloff),
            );
        }
         
         
        painter.circle_filled(
            pixel,
            cfg.width * 1.1 * (0.7 + 0.3 * falloff),
            theme::with_alpha(head_color, whole * 0.9 * falloff),
        );
    }
}

 
 
 
 
 
fn depth_falloff(distance: f64, range: f64) -> f32 {
    let t = (distance / range.max(1e-3)).clamp(0.0, 1.0) as f32;
    (1.0 - 0.55 * t * t).clamp(0.35, 1.0)
}

 
fn midpoint(a: V3, b: V3) -> V3 {
    V3 {
        x: (a.x + b.x) * 0.5,
        y: (a.y + b.y) * 0.5,
        z: (a.z + b.z) * 0.5,
    }
}

fn draw_segment(
    painter: &Painter,
    view: &View,
    a: V3,
    b: V3,
    width: f32,
    color: Color32,
    glow: bool,
    range: f64,
) {
    let (Some(pa), Some(pb)) = (view.project(a), view.project(b)) else {
        return;
    };
    let falloff = depth_falloff(midpoint(a, b).sub(view.cam).length(), range);
     
     
    let width = width * (0.55 + 0.45 * falloff);
     
     
    let color = theme::with_alpha(color, falloff);
    if glow {
        for (scale, strength) in HALO_LAYERS {
            painter.line_segment(
                [pa, pb],
                Stroke::new(width * scale, theme::with_alpha(color, strength)),
            );
        }
    }
    painter.line_segment([pa, pb], Stroke::new(width, color));
}

fn draw_joint(painter: &Painter, view: &View, point: V3, width: f32, color: Color32, range: f64) {
    if let Some(pixel) = view.project(point) {
        let falloff = depth_falloff(point.sub(view.cam).length(), range);
        painter.circle_filled(
            pixel,
            (width * 0.5 * (0.6 + 0.4 * falloff)).max(0.4),
            theme::with_alpha(color, falloff),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offset(a: V3, b: V3) -> V3 {
        a.sub(b)
    }

    fn origin() -> V3 {
        V3 {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        }
    }

    #[test]
    fn every_step_is_exactly_one_block() {
        for seed in 0..32 {
            let points = generate_path(origin(), 24, 8.0, seed as f32 / 32.0);
            for pair in points.windows(2) {
                let distance = offset(pair[1], pair[0]).length();
                assert!((distance - 1.0).abs() < 1e-9, "step was {distance}");
            }
        }
    }

    #[test]
    fn a_line_never_immediately_reverses() {
        for seed in 0..32 {
            let points = generate_path(origin(), 32, 10.0, seed as f32 / 32.0);
            for pair in points.windows(3) {
                let first = offset(pair[1], pair[0]);
                let second = offset(pair[2], pair[1]);
                assert!(
                    first.dot(second) >= 0.0,
                    "a step reversed direction: {first:?} then {second:?}"
                );
            }
        }
    }

    #[test]
    fn path_is_deterministic_for_a_seed() {
        let a = generate_path(origin(), 30, 8.0, 0.42);
        let b = generate_path(origin(), 30, 8.0, 0.42);
        assert_eq!(a.len(), b.len());
        for (a, b) in a.iter().zip(&b) {
            assert!((a.x - b.x).abs() < 1e-12);
            assert!((a.y - b.y).abs() < 1e-12);
            assert!((a.z - b.z).abs() < 1e-12);
        }
    }

    #[test]
    fn spread_keeps_the_walk_near_the_start() {
        for seed in 0..64 {
            let spread = 7.0;
            let points = generate_path(origin(), 32, spread, seed as f32 / 64.0);
            let start = points[0];
            for point in points {
                let distance = point.sub(start).length();
                assert!(
                    distance <= spread + 1.0,
                    "line wandered {distance} blocks away"
                );
            }
        }
    }

     
     
    #[test]
    fn nearer_lines_are_thicker_and_brighter() {
        let near = depth_falloff(0.0, 14.0);
        let middle = depth_falloff(7.0, 14.0);
        let far = depth_falloff(14.0, 14.0);
        assert_eq!(near, 1.0);
        assert!(near > middle && middle > far, "{near} {middle} {far}");
        assert!(far >= 0.35, "far lines must stay visible: {far}");
         
        assert_eq!(depth_falloff(99.0, 14.0), far);
        assert!(depth_falloff(1.0, 0.0).is_finite());

         
        let width = |distance: f64| 1.8 * (0.55 + 0.45 * depth_falloff(distance, 14.0));
        assert!(width(0.0) > width(14.0));
        assert!((width(14.0) - 1.8 * (0.55 + 0.45 * far)).abs() < 1e-6);
    }

     
     
    #[test]
    fn the_pulse_stays_bright_enough_to_see() {
        let mut seen = Vec::new();
        for step in 0..600 {
            let value = pulse(step as f32 * 0.05);
            assert!((0.8..=1.0).contains(&value), "pulse left range: {value}");
            seen.push(value);
        }
        let min = seen.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = seen.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min > 0.15, "the pulse is flat: {min}..{max}");
    }

    #[test]
    fn rendered_lines_are_visible_and_connected() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let view = View {
            cam: V3 {
                x: 0.0,
                y: 65.0,
                z: 0.0,
            },
            fwd: V3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            right: V3 {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
            up: V3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            tan_x: 1.24,
            tan_y: 0.70,
            w: 960.0,
            h: 540.0,
        };
        let cfg = AuraConfig {
            enabled: true,
            lines: 24,
            switches: 14,
            speed: 1.2,
            range: 14.0,
            spread: 4.0,
            fade: Duration::from_secs_f32(0.7),
            width: DEFAULT_WIDTH,
            color: DEFAULT_COLOR,
            rainbow: false,
            glow: true,
        };
        let now = Instant::now();
        let center = V3 {
            x: 0.0,
            y: 64.8,
            z: 0.0,
        };
        let ctx = egui::Context::default();
        let mut textures = HashMap::new();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(960.0, 540.0),
                )),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("aura_preview"),
                ));
                 
                 
                for i in 0..cfg.lines {
                    let seed = i as f32 / cfg.lines as f32;
                    let start = start_position(center, cfg.range, seed);
                    let line = Line {
                        points: generate_path(start, cfg.switches, cfg.spread, seed),
                        seed,
                        spawned: now - Duration::from_secs_f32(1.6),
                        step_seconds: 0.2,
                        fade: 0.7,
                    };
                    draw_line(&painter, &view, now, &cfg, &line);
                }
            },
        );
        gather_textures(&mut textures, &output.textures_delta);
        let primitives = ctx.tessellate(output.shapes, 1.0);
        let image = rasterize(primitives, &textures, 960, 540, [0.05, 0.05, 0.07, 1.0]);
        let lit = image
            .pixels()
            .filter(|pixel| {
                pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32 > 150 && pixel[3] > 128
            })
            .count();
        assert!(
            lit > 800,
            "the aura rendered almost nothing ({lit} lit pixels)"
        );
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/aura-preview.png");
        image.save(&path).expect("the aura preview png saves");
    }

    #[test]
    fn config_uses_the_new_settings_and_clamps_bad_values() {
        let module = crate::module::render::aura::AuraModule::new();
        let mut data = module.module;
        let cfg = AuraConfig::from_data(&data);
        assert_eq!(cfg.lines, DEFAULT_LINES as usize);
        assert_eq!(cfg.switches, DEFAULT_SWITCHES as usize);
        assert_eq!(cfg.speed, DEFAULT_SPEED);
        assert_eq!(cfg.range, DEFAULT_RANGE as f64);
        assert_eq!(cfg.spread, DEFAULT_SPREAD as f64);
        assert_eq!(cfg.width, DEFAULT_WIDTH);
        assert!(cfg.glow);
        assert!(!cfg.rainbow);

        data.get_setting_mut("Lines")
            .unwrap()
            .set_slider_value(f32::INFINITY);
        data.get_setting_mut("Switches")
            .unwrap()
            .set_slider_value(999.0);
        data.get_setting_mut("Speed").unwrap().set_slider_value(0.0);
        let cfg = AuraConfig::from_data(&data);
        assert_eq!(cfg.lines, DEFAULT_LINES as usize);
        assert_eq!(cfg.switches, 64);
        assert_eq!(cfg.speed, 0.4);
    }
}
