 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::esp::{color_setting, slider_setting, toggle_setting, View, V3};
use crate::graphic::theme;
use crate::mapping::client::options::Perspective;
use crate::mapping::entity::EntityRef;
use crate::module::player::autototem::roll;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, minecraft};
use egui::{vec2, Color32, Mesh, Painter, Pos2, Shape, Vec2};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const NEON_RED: Color32 = Color32::from_rgb(255, 24, 48);
const DARK_BLACK: Color32 = Color32::from_rgb(14, 12, 13);

 
 
const HALO_WIDE: f32 = 2.6;
const HALO_NEAR: f32 = 1.8;
 
const CORE_WIDTH: f32 = 0.42;
 
 
const PULSE_DEPTH: f32 = 0.34;

 
pub(crate) const MAX_POINTS: usize = 400;
 
pub(crate) const TELEPORT_DISTANCE: f64 = 8.0;
 
const BACK_OFFSET: f64 = 0.3;
 
const BACK_HEIGHT: f64 = 1.2;

pub(crate) const GLOW_ALPHA: f32 = 0.22;
 
const SHIMMER_DEPTH: f32 = 0.18;
 
const SHIMMER_SPEED: f32 = 10.0;

const DEFAULT_FADE: f32 = 2.5;
const DEFAULT_WIDTH: f32 = 4.0;
const DEFAULT_SPACING: f32 = 0.15;
const DEFAULT_HEIGHT: f32 = 0.1;

 
 
 
const FIRST_PERSON_DELAY: Duration = Duration::from_millis(500);

 
#[derive(Debug, Clone, Copy)]
pub(crate) struct TrailConfig {
    pub(crate) enabled: bool,
    fade: Duration,
    width: f32,
    spacing: f64,
    height: f64,
    glow: bool,
     
    color: Color32,
}

impl Default for TrailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            fade: Duration::from_secs_f32(DEFAULT_FADE),
            width: DEFAULT_WIDTH,
            spacing: DEFAULT_SPACING as f64,
            height: DEFAULT_HEIGHT as f64,
            glow: true,
            color: NEON_RED,
        }
    }
}

impl TrailConfig {
    fn from_data(data: &ModuleData) -> Self {
        Self {
            enabled: data.enabled,
            fade: Duration::from_secs_f32(bounded(data, "Fade Time", DEFAULT_FADE, 0.5, 10.0)),
            width: bounded(data, "Width", DEFAULT_WIDTH, 0.5, 12.0),
            spacing: bounded(data, "Spacing", DEFAULT_SPACING, 0.05, 1.0) as f64,
            height: bounded(data, "Height", DEFAULT_HEIGHT, 0.0, 1.5) as f64,
            glow: toggle_setting(data, "Glow", true),
            color: color_setting(data, "Color", NEON_RED),
        }
    }
}

 
 
pub(crate) fn bounded(data: &ModuleData, name: &str, fallback: f32, min: f32, max: f32) -> f32 {
    let value = slider_setting(data, name, fallback);
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

 
#[derive(Debug, Clone, Copy)]
struct TrailPoint {
     
    pos: V3,
    spawned: Instant,
     
     
    phase: f32,
}

 
 
struct TrailState {
    points: Vec<TrailPoint>,
     
     
    first_person_since: Option<Instant>,
}

fn state() -> &'static Mutex<TrailState> {
    static STATE: OnceLock<Mutex<TrailState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(TrailState {
            points: Vec::new(),
            first_person_since: None,
        })
    })
}

fn clear_points() {
    let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
    state.points.clear();
    state.first_person_since = None;
}

 

 
pub(crate) fn life(age: Duration, fade: Duration) -> f32 {
    if fade.is_zero() {
        return 0.0;
    }
    (1.0 - age.as_secs_f32() / fade.as_secs_f32()).clamp(0.0, 1.0)
}

 
pub(crate) fn trail_color(mix: f32) -> Color32 {
    ramp_color(NEON_RED, mix)
}

 
 
pub(crate) fn ramp_color(hot: Color32, mix: f32) -> Color32 {
    theme::lerp_color(DARK_BLACK, hot, mix)
}

 
pub(crate) fn is_red(color: Color32) -> bool {
    color.r() > color.g() + 80
}

 
 
fn half_width(width: f32, life: f32, shimmer: f32) -> f32 {
    width * life * shimmer * 0.5
}

 
 
fn shimmer(age_secs: f32) -> f32 {
    1.0 + SHIMMER_DEPTH * (age_secs * SHIMMER_SPEED).sin()
}

 
 
 
fn heat(life: f32, age_secs: f32, phase: f32) -> f32 {
    let cooling = life.powf(0.65);
    let ripple = 0.5 + 0.5 * (age_secs * SHIMMER_SPEED - phase * std::f32::consts::TAU).sin();
    (cooling * (1.0 - PULSE_DEPTH + PULSE_DEPTH * ripple)).clamp(0.0, 1.0)
}

 
 
 
fn back_anchor(feet: V3, yaw: f32) -> V3 {
    let yaw_r = f64::from(yaw).to_radians();
    V3 {
        x: feet.x + yaw_r.sin() * BACK_OFFSET,
        y: feet.y + BACK_HEIGHT,
        z: feet.z - yaw_r.cos() * BACK_OFFSET,
    }
}

 
pub(crate) fn should_record(last: Option<V3>, pos: V3, spacing: f64) -> bool {
    last.is_none_or(|last| pos.sub(last).length() >= spacing)
}

 
 
fn prune(points: &mut Vec<TrailPoint>, now: Instant, fade: Duration) {
    let expired = points
        .iter()
        .take_while(|point| life(now.duration_since(point.spawned), fade) <= 0.0)
        .count();
    points.drain(..expired);
}

 
 
 
fn first_person_view() -> bool {
    minecraft()
        .options()
        .and_then(|options| options.perspective())
        .is_ok_and(|perspective| perspective == Perspective::FirstPerson)
}

 
 
fn sync_first_person(since: &mut Option<Instant>, first_person: bool, now: Instant) {
    if first_person {
        since.get_or_insert(now);
    } else {
        *since = None;
    }
}

 
fn hidden_in_first_person(since: Option<Instant>, now: Instant) -> bool {
    since.is_some_and(|since| now.saturating_duration_since(since) >= FIRST_PERSON_DELAY)
}

 

 
#[derive(Debug)]
pub struct TrailModule {
    pub module: ModuleData,
}

impl TrailModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Trail,
                description: "A ribbon of neon light that cools behind you".to_string(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                     
                     
                    ModuleSetting::Color {
                        name: "Color".to_string(),
                        value: [1.0, 24.0 / 255.0, 48.0 / 255.0, 1.0],
                    },
                    ModuleSetting::Slider {
                        name: "Fade Time".to_string(),
                        value: DEFAULT_FADE,
                        min: 0.5,
                        max: 10.0,
                    },
                    ModuleSetting::Slider {
                        name: "Width".to_string(),
                        value: DEFAULT_WIDTH,
                        min: 0.5,
                        max: 12.0,
                    },
                    ModuleSetting::Slider {
                        name: "Spacing".to_string(),
                        value: DEFAULT_SPACING,
                        min: 0.05,
                        max: 1.0,
                    },
                    ModuleSetting::Slider {
                        name: "Height".to_string(),
                        value: DEFAULT_HEIGHT,
                        min: 0.0,
                        max: 1.5,
                    },
                    ModuleSetting::Toggle {
                        name: "Glow".to_string(),
                        value: true,
                    },
                ],
            },
        }
    }
}

 
 
pub(crate) fn read_config() -> TrailConfig {
    let mut cfg = TrailConfig::default();
    if let Some(arc) = client().modules.get(ModuleId::Trail) {
        if let Ok(module) = arc.lock() {
            cfg = TrailConfig::from_data(module.get_module_data());
        }
    }
    cfg
}

 
#[derive(Clone, Copy, Debug)]
struct Sample {
    pos: Pos2,
     
    half: f32,
     
     
    heat: f32,
     
    life: f32,
}

 
pub(crate) fn draw(painter: &Painter, view: &View, now: Instant, cfg: &TrailConfig) {
    if !cfg.enabled {
        return;
    }
    let state = state().lock().unwrap_or_else(|p| p.into_inner());
     
     
     
     
    if hidden_in_first_person(state.first_person_since, now) {
        return;
    }
     
     
     
    let mut runs: Vec<Vec<Sample>> = Vec::new();
    let mut run: Vec<Sample> = Vec::new();
    for point in &state.points {
        let age = now.duration_since(point.spawned);
        let alive = life(age, cfg.fade);
        let projected = if alive > 0.0 {
            view.project(V3 {
                x: point.pos.x,
                y: point.pos.y + cfg.height,
                z: point.pos.z,
            })
        } else {
            None
        };
        match projected {
            Some(pos) => {
                let age_secs = age.as_secs_f32();
                run.push(Sample {
                    pos,
                    half: half_width(cfg.width, alive, shimmer(age_secs)),
                    heat: heat(alive, age_secs, point.phase),
                    life: alive,
                });
            }
            None => {
                if run.len() >= 2 {
                    runs.push(std::mem::take(&mut run));
                } else {
                    run.clear();
                }
            }
        }
    }
    if run.len() >= 2 {
        runs.push(run);
    }

    for samples in &runs {
        paint_run(painter, samples, cfg.color, cfg.glow);
    }
}

 
 
fn paint_run(painter: &Painter, samples: &[Sample], hot: Color32, glow: bool) {
    if glow {
         
        paint_ribbon(painter, samples, HALO_WIDE, |sample| {
            theme::with_alpha(hot, 0.10 * sample.life * sample.heat)
        });
        paint_ribbon(painter, samples, HALO_NEAR, |sample| {
            theme::with_alpha(hot, 0.14 * sample.life * sample.heat)
        });
    }
    paint_ribbon(painter, samples, 1.0, |sample| {
        theme::with_alpha(ramp_color(hot, sample.heat), sample.life)
    });
     
     
    paint_ribbon(painter, samples, CORE_WIDTH, |sample| {
        theme::with_alpha(
            theme::lerp_color(ramp_color(hot, sample.heat), Color32::WHITE, 0.45),
            0.7 * sample.life * sample.heat,
        )
    });
}

 
 
 
 
fn paint_ribbon(
    painter: &Painter,
    samples: &[Sample],
    scale: f32,
    color: impl Fn(&Sample) -> Color32,
) {
    if samples.len() < 2 {
        return;
    }
    painter.add(Shape::mesh(ribbon_mesh(samples, scale, color)));
}

 
 
fn ribbon_mesh(samples: &[Sample], scale: f32, color: impl Fn(&Sample) -> Color32) -> Mesh {
    let mut mesh = Mesh::default();
    for (index, sample) in samples.iter().enumerate() {
        let direction = ribbon_direction(samples, index);
        let offset = vec2(-direction.y, direction.x) * (sample.half * scale);
        let shade = color(sample);
        mesh.colored_vertex(sample.pos + offset, shade);
        mesh.colored_vertex(sample.pos - offset, shade);
    }
    for index in 0..samples.len().saturating_sub(1) {
        let base = (index * 2) as u32;
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base + 1, base + 3, base + 2);
    }
    mesh
}

 
 
 
fn ribbon_direction(samples: &[Sample], index: usize) -> Vec2 {
    let previous = index.checked_sub(1).map(|i| samples[i].pos);
    let next = samples.get(index + 1).map(|sample| sample.pos);
    let delta = match (previous, next) {
        (Some(a), Some(b)) => b - a,
        (None, Some(b)) => b - samples[index].pos,
        (Some(a), None) => samples[index].pos - a,
        (None, None) => Vec2::X,
    };
    let length = delta.length();
    if length > 1e-4 {
        delta / length
    } else {
        Vec2::X
    }
}

impl Module for TrailModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        clear_points();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let cfg = TrailConfig::from_data(&self.module);
        let now = Instant::now();
        let Some(player) = minecraft().player()? else {
             
            clear_points();
            return Ok(());
        };
        let feet = V3::from(player.get_position()?);
        let anchor = back_anchor(feet, player.get_yaw()?);

        let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
        sync_first_person(&mut state.first_person_since, first_person_view(), now);
        prune(&mut state.points, now, cfg.fade);
        let teleported = state
            .points
            .last()
            .is_some_and(|point| anchor.sub(point.pos).length() > TELEPORT_DISTANCE);
        if teleported {
            state.points.clear();
        }

        let last = state.points.last().map(|point| point.pos);
        if should_record(last, anchor, cfg.spacing) {
            if state.points.len() >= MAX_POINTS {
                state.points.remove(0);
            }
            state.points.push(TrailPoint {
                pos: anchor,
                spawned: now,
                phase: roll() as f32,
            });
        }
        Ok(())
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

    fn at(x: f64) -> V3 {
        V3 { x, y: 0.0, z: 0.0 }
    }

    fn point(x: f64, spawned: Instant) -> TrailPoint {
        TrailPoint {
            pos: at(x),
            spawned,
            phase: 0.0,
        }
    }

    fn sample(x: f32, y: f32) -> Sample {
        Sample {
            pos: Pos2::new(x, y),
            half: 2.0,
            heat: 0.5,
            life: 0.5,
        }
    }

    #[test]
    fn the_first_person_clock_arms_once_and_clears_on_leaving() {
        let start = Instant::now();
        let mut since = None;
        sync_first_person(&mut since, true, start);
        assert_eq!(since, Some(start));
         
         
        sync_first_person(&mut since, true, start + Duration::from_secs(2));
        assert_eq!(since, Some(start));
        sync_first_person(&mut since, false, start + Duration::from_secs(3));
        assert_eq!(since, None, "third person clears the wait");
    }

    #[test]
    fn the_trail_hides_only_after_half_a_second_in_first_person() {
        let start = Instant::now();
        assert!(!hidden_in_first_person(None, start));
        assert!(!hidden_in_first_person(Some(start), start));
        assert!(!hidden_in_first_person(
            Some(start),
            start + Duration::from_millis(499)
        ));
        assert!(hidden_in_first_person(
            Some(start),
            start + FIRST_PERSON_DELAY
        ));
        assert!(hidden_in_first_person(
            Some(start),
            start + Duration::from_secs(4)
        ));
         
        assert!(!hidden_in_first_person(
            Some(start),
            start - Duration::from_secs(1)
        ));
    }

     
     
    #[test]
    fn first_person_stops_the_ribbon_after_the_delay() {
        use crate::graphic::esp::build_view;

        let now = Instant::now();
        let mut cfg = TrailConfig::default();
        cfg.enabled = true;
        cfg.fade = Duration::from_secs(5);
        cfg.glow = false;
        {
            let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
            state.points = (1..=3)
                .map(|step| TrailPoint {
                    pos: V3 {
                        x: 0.0,
                        y: 1.2,
                        z: step as f64 * 2.0,
                    },
                    spawned: now,
                    phase: 0.0,
                })
                .collect();
            state.first_person_since = Some(now - Duration::from_secs(1));
        }
        let view = build_view(
            V3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            0.0,
            0.0,
            90.0,
            320.0,
            180.0,
        );
        let ctx = egui::Context::default();
        let frame = |now: Instant| {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(320.0, 180.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Background,
                        egui::Id::new("trail_first_person"),
                    ));
                    draw(&painter, &view, now, &cfg);
                },
            );
            output
                .shapes
                .iter()
                .filter(|clipped| matches!(clipped.shape, Shape::Mesh(_)))
                .count()
        };

        assert_eq!(frame(now), 0, "first person hides the ribbon after 0.5s");
        state()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .first_person_since = None;
        assert!(frame(now) > 0, "third person brings the ribbon back");
        clear_points();
    }

    #[test]
    fn a_point_fades_from_full_to_gone_over_the_fade_time() {
        let fade = Duration::from_secs(2);
        assert_eq!(life(Duration::ZERO, fade), 1.0);
        assert_eq!(life(Duration::from_secs(1), fade), 0.5);
        assert_eq!(life(Duration::from_secs(2), fade), 0.0);
        assert_eq!(life(Duration::from_secs(9), fade), 0.0);
    }

    #[test]
    fn the_old_end_tapers_to_a_point() {
        assert_eq!(half_width(4.0, 0.0, 1.0), 0.0);
        assert!(half_width(4.0, 0.5, 1.0) < half_width(4.0, 1.0, 1.0));
    }

    #[test]
    fn colour_mix_runs_from_dark_black_to_neon_red() {
        assert_eq!(trail_color(0.0), DARK_BLACK);
        assert_eq!(trail_color(1.0), NEON_RED);
        assert!(is_red(NEON_RED));
        assert!(!is_red(DARK_BLACK));
    }

    #[test]
    fn prune_drops_only_the_points_that_have_faded() {
        let fade = Duration::from_secs(2);
        let now = Instant::now();
        let mut points = vec![
            point(0.0, now - Duration::from_secs(3)),
            point(1.0, now - Duration::from_secs(1)),
            point(2.0, now),
        ];
        prune(&mut points, now, fade);
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].pos.x, 1.0);
    }

    #[test]
    fn a_sample_is_recorded_only_after_moving_the_spacing_distance() {
        assert!(should_record(None, at(0.0), 0.15));
        assert!(!should_record(Some(at(0.0)), at(0.1), 0.15));
        assert!(should_record(Some(at(0.0)), at(0.2), 0.15));
    }

    #[test]
    fn the_trail_starts_on_the_back_not_the_feet() {
        let feet = V3 {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        };
         
        let back = back_anchor(feet, 0.0);
        assert!(back.z < feet.z);
        assert!((back.z - (feet.z - BACK_OFFSET)).abs() < 1e-9);
        assert!((back.y - (feet.y + BACK_HEIGHT)).abs() < 1e-9);
         
        let back = back_anchor(feet, 90.0);
        assert!(back.x > feet.x);
        assert!(back.y > feet.y + 1.0);
    }

     
     
    #[test]
    fn the_ribbon_cools_from_its_head_to_its_tail() {
        assert!(heat(1.0, 0.0, 0.0) > heat(0.5, 0.0, 0.0));
        assert!(heat(0.5, 0.0, 0.0) > heat(0.1, 0.0, 0.0));
        assert_eq!(heat(0.0, 0.0, 0.0), 0.0, "a dead point is cold");
        assert!(heat(1.0, 0.0, 0.0) <= 1.0);

         
         
        let brightest = (0..20)
            .map(|step| heat(0.8, step as f32 * 0.06, 0.3))
            .fold(0.0_f32, f32::max);
        let dimmest = (0..20)
            .map(|step| heat(0.8, step as f32 * 0.06, 0.3))
            .fold(1.0_f32, f32::min);
        assert!(brightest > dimmest);
        assert!(
            dimmest > heat(0.2, 0.0, 0.3),
            "the pulse never outshines age"
        );
    }

     
     
    #[test]
    fn the_ribbon_is_one_unbroken_strip() {
        let samples = [sample(0.0, 0.0), sample(10.0, 0.0), sample(20.0, 4.0)];
        let mesh = ribbon_mesh(&samples, 1.0, |s| theme::with_alpha(NEON_RED, s.life));
        assert_eq!(mesh.vertices.len(), 2 * samples.len());
        assert_eq!(mesh.indices.len(), 6 * (samples.len() - 1));
         
         
        assert!((mesh.vertices[0].pos - Pos2::new(0.0, 2.0)).length() < 1e-3);
        assert!((mesh.vertices[1].pos - Pos2::new(0.0, -2.0)).length() < 1e-3);
         
        assert!(mesh.vertices.iter().all(|v| v.color.a() > 0));
        assert_eq!(mesh.texture_id, egui::TextureId::default());
    }

     
     
    #[test]
    fn a_turn_is_smoothed_across_the_chord() {
        let samples = [sample(0.0, 0.0), sample(10.0, 0.0), sample(10.0, 10.0)];
        let direction = ribbon_direction(&samples, 1);
         
        assert!((direction.x - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-3);
        assert!((direction.y - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-3);
         
        assert!((ribbon_direction(&samples, 0) - Vec2::X).length() < 1e-3);
        assert!(ribbon_direction(&samples, 2).y > 0.99);
    }

     
     
    #[test]
    fn the_trail_is_halo_body_and_core() {
        let samples = [sample(0.0, 0.0), sample(20.0, 0.0), sample(40.0, 6.0)];
        let ctx = egui::Context::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(200.0, 100.0),
                )),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("trail_preview"),
                ));
                paint_run(&painter, &samples, NEON_RED, true);
            },
        );
        let meshes: Vec<&Mesh> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Mesh(mesh) => Some(mesh),
                _ => None,
            })
            .collect();
        assert_eq!(meshes.len(), 4, "two halo passes, the body, the core");
         
        let widths: Vec<f32> = meshes
            .iter()
            .map(|mesh| mesh.vertices[0].pos.distance(mesh.vertices[1].pos))
            .collect();
        assert!(widths[0] > widths[1]);
        assert!(widths[1] > widths[2]);
        assert!(widths[2] > widths[3]);
    }

     
     
     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn trail_preview() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

         
        let run = |amplitude: f32, y: f32, width: f32| -> Vec<Sample> {
            (0..60)
                .map(|step| {
                    let t = step as f32 / 59.0;
                    let life = 1.0 - t;
                    let age = t * 3.0;
                    Sample {
                        pos: Pos2::new(24.0 + t * 660.0, y + (t * 6.0).sin() * amplitude),
                        half: half_width(width, life, shimmer(age)),
                        heat: heat(life, age, 0.0),
                        life,
                    }
                })
                .collect()
        };
        let (w, h) = (720usize, 260usize);
        let ctx = egui::Context::default();
        let mut textures = HashMap::new();
        let mut shapes = Vec::new();
        let mut laid_out_at = 1.0;
        for _ in 0..2 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(w as f32, h as f32),
                    )),
                    time: Some(1.0),
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Background,
                        egui::Id::new("trail_preview"),
                    ));
                    paint_run(&painter, &run(0.0, 72.0, 14.0), NEON_RED, true);
                    paint_run(&painter, &run(40.0, 192.0, 14.0), NEON_RED, true);
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            shapes = output.shapes;
            laid_out_at = output.pixels_per_point;
        }
        let primitives = ctx.tessellate(shapes, laid_out_at);
        let sheet = rasterize(primitives, &textures, w, h, [0.055, 0.045, 0.05, 1.0]);
        let image = image::imageops::resize(
            &sheet,
            w as u32 * 2,
            h as u32 * 2,
            image::imageops::FilterType::Nearest,
        );
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).expect("the screenshots directory exists");
        let path = dir.join("trail.png");
        image.save(&path).expect("the trail preview saves");
        println!("trail: {path:?}");
    }

    #[test]
    fn the_module_defaults_are_disabled_with_a_red_black_glow() {
        let module = TrailModule::new();
        assert!(!module.module.enabled);
        let cfg = TrailConfig::from_data(&module.module);
        assert!(cfg.glow);
        assert_eq!(cfg.fade, Duration::from_secs_f32(DEFAULT_FADE));
        assert_eq!(cfg.color, NEON_RED, "the stock trail is the brand neon red");
         
         
        let mut custom = TrailModule::new();
        if let Some(ModuleSetting::Color { value, .. }) = custom.module.get_setting_mut("Color") {
            *value = [0.2, 0.4, 1.0, 1.0];
        }
        let cfg = TrailConfig::from_data(&custom.module);
        assert_eq!(cfg.color.g(), 102);
        assert!(ramp_color(cfg.color, 1.0).b() > ramp_color(cfg.color, 1.0).r());
    }
}
