 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::esp::{color_setting, toggle_setting, View, V3};
use crate::graphic::theme;
use crate::mapping::entity::player::Player;
use crate::mapping::entity::EntityRef;
use crate::mapping::MappedObject;
use crate::module::render::trail::{bounded, life};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, minecraft};
use egui::{Color32, Painter, Pos2, Shape, Stroke};
use std::collections::{HashMap, HashSet};
use std::f64::consts::TAU;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const DEFAULT_FADE: f32 = 1.0;
const DEFAULT_RADIUS: f32 = 1.5;
const DEFAULT_HEIGHT: f32 = 0.05;
const DEFAULT_RINGS: f32 = 2.0;
const DEFAULT_THICKNESS: f32 = 2.0;

 
const DEFAULT_COLOR_RGBA: [f32; 4] = [242.0 / 255.0, 46.0 / 255.0, 56.0 / 255.0, 1.0];

 
const RING_STEPS: usize = 28;
 
const MAX_RINGS: usize = 256;
 
const FADE_IN: f32 = 0.12;
 
const FILL_ALPHA: f32 = 0.22;
 
const MIN_SCALE: f64 = 0.15;
 
const MAX_DISTANCE: f64 = 64.0;
 
const RIPPLE_THINNING: f32 = 0.3;

 
const DEFAULT_COLOR: Color32 = Color32::from_rgb(
    (DEFAULT_COLOR_RGBA[0] * 255.0) as u8,
    (DEFAULT_COLOR_RGBA[1] * 255.0) as u8,
    (DEFAULT_COLOR_RGBA[2] * 255.0) as u8,
);

 
#[derive(Debug, Clone, Copy)]
pub(crate) struct JumpCircleConfig {
    pub(crate) enabled: bool,
    fade: Duration,
    radius: f32,
    height: f64,
    color: Color32,
    rings: usize,
    thickness: f32,
    fill: bool,
    players_only: bool,
}

impl Default for JumpCircleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            fade: Duration::from_secs_f32(DEFAULT_FADE),
            radius: DEFAULT_RADIUS,
            height: f64::from(DEFAULT_HEIGHT),
            color: DEFAULT_COLOR,
            rings: DEFAULT_RINGS as usize,
            thickness: DEFAULT_THICKNESS,
            fill: true,
            players_only: false,
        }
    }
}

impl JumpCircleConfig {
    fn from_data(data: &ModuleData) -> Self {
        Self {
            enabled: data.enabled,
            fade: Duration::from_secs_f32(bounded(data, "Fade Time", DEFAULT_FADE, 0.3, 5.0)),
            radius: bounded(data, "Radius", DEFAULT_RADIUS, 0.5, 5.0),
            height: f64::from(bounded(data, "Height", DEFAULT_HEIGHT, 0.0, 0.5)),
            color: color_setting(data, "Color", DEFAULT_COLOR),
            rings: bounded(data, "Rings", DEFAULT_RINGS, 1.0, 3.0).round() as usize,
            thickness: bounded(data, "Thickness", DEFAULT_THICKNESS, 0.5, 6.0),
            fill: toggle_setting(data, "Fill", true),
            players_only: toggle_setting(data, "Players Only", false),
        }
    }
}

 
#[derive(Debug, Clone, Copy)]
struct JumpRing {
    center: V3,
    spawned: Instant,
}

#[derive(Default)]
struct JumpState {
    rings: Vec<JumpRing>,
     
    grounded: HashMap<i32, bool>,
}

fn state() -> &'static Mutex<JumpState> {
    static STATE: OnceLock<Mutex<JumpState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(JumpState::default()))
}

fn clear_rings() {
    *state().lock().unwrap_or_else(|p| p.into_inner()) = JumpState::default();
}

 

 
fn is_jump(previous: Option<bool>, grounded: bool) -> bool {
    previous == Some(true) && !grounded
}

 
fn ring_points(center: V3, radius: f64) -> Vec<V3> {
    (0..RING_STEPS)
        .map(|step| {
            let angle = TAU * step as f64 / RING_STEPS as f64;
            V3 {
                x: center.x + radius * angle.cos(),
                y: center.y,
                z: center.z + radius * angle.sin(),
            }
        })
        .collect()
}

 
 
fn ring_scale(progress: f32) -> f64 {
    let progress = f64::from(progress.clamp(0.0, 1.0));
    MIN_SCALE + (1.0 - MIN_SCALE) * progress.powf(0.55)
}

 
 
fn ring_alpha(progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    if progress <= FADE_IN {
        progress / FADE_IN
    } else {
        let rest = (progress - FADE_IN) / (1.0 - FADE_IN);
        (1.0 - rest).powf(1.7)
    }
}

 
 
fn ripple_progress(progress: f32, index: usize, count: usize) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    if count <= 1 {
        return progress;
    }
    let last = (count - 1) as f32;
     
     
    let lead_span = 0.66f32;
    let stagger = (1.0 - lead_span) / last;
    let start = index as f32 * stagger;
    let span = 1.0 - stagger * last;
    ((progress - start) / span).clamp(0.0, 1.0)
}

 
 
 
fn ring_segments(points: &[Option<Pos2>]) -> Vec<Vec<Pos2>> {
    if points.is_empty() {
        return Vec::new();
    }
    if points.iter().all(Option::is_some) {
        let mut loop_points: Vec<Pos2> = points.iter().flatten().copied().collect();
        if let Some(first) = loop_points.first().copied() {
            loop_points.push(first);
        }
        return vec![loop_points];
    }
     
     
    let start = points.iter().position(Option::is_none).unwrap_or_default();
    let mut segments = Vec::new();
    let mut run: Vec<Pos2> = Vec::new();
    for offset in 0..points.len() {
        match points[(start + offset) % points.len()] {
            Some(point) => run.push(point),
            None => {
                if run.len() >= 2 {
                    segments.push(std::mem::take(&mut run));
                } else {
                    run.clear();
                }
            }
        }
    }
    if run.len() >= 2 {
        segments.push(run);
    }
    segments
}

 
fn prune(rings: &mut Vec<JumpRing>, now: Instant, fade: Duration) {
    let expired = rings
        .iter()
        .take_while(|ring| life(now.duration_since(ring.spawned), fade) <= 0.0)
        .count();
    rings.drain(..expired);
}

 
#[derive(Debug)]
pub struct JumpCirclesModule {
    pub module: ModuleData,
}

impl JumpCirclesModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::JumpCircles,
                description: "Expands a fading ring under every entity that jumps".into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Radius".into(),
                        value: DEFAULT_RADIUS,
                        min: 0.5,
                        max: 5.0,
                    },
                    ModuleSetting::Slider {
                        name: "Fade Time".into(),
                        value: DEFAULT_FADE,
                        min: 0.3,
                        max: 5.0,
                    },
                    ModuleSetting::Slider {
                        name: "Height".into(),
                        value: DEFAULT_HEIGHT,
                        min: 0.0,
                        max: 0.5,
                    },
                    ModuleSetting::Color {
                        name: "Color".into(),
                        value: DEFAULT_COLOR_RGBA,
                    },
                    ModuleSetting::Slider {
                        name: "Rings".into(),
                        value: DEFAULT_RINGS,
                        min: 1.0,
                        max: 3.0,
                    },
                    ModuleSetting::Slider {
                        name: "Thickness".into(),
                        value: DEFAULT_THICKNESS,
                        min: 0.5,
                        max: 6.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Fill".into(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Players Only".into(),
                        value: false,
                    },
                ],
            },
        }
    }
}

impl Default for JumpCirclesModule {
    fn default() -> Self {
        Self::new()
    }
}

 
 
pub(crate) fn read_config() -> JumpCircleConfig {
    let mut cfg = JumpCircleConfig::default();
    if let Some(arc) = client().modules.get(ModuleId::JumpCircles) {
        if let Ok(module) = arc.lock() {
            cfg = JumpCircleConfig::from_data(module.get_module_data());
        }
    }
    cfg
}

 
pub(crate) fn draw(painter: &Painter, view: &View, now: Instant, cfg: &JumpCircleConfig) {
    if !cfg.enabled {
        return;
    }
    let state = state().lock().unwrap_or_else(|p| p.into_inner());
    for ring in &state.rings {
        let alive = life(now.duration_since(ring.spawned), cfg.fade);
        if alive <= 0.0 {
            continue;
        }
        let progress = 1.0 - alive;
        let (dx, dy, dz) = (
            ring.center.x - view.cam.x,
            ring.center.y - view.cam.y,
            ring.center.z - view.cam.z,
        );
        if dx * dx + dy * dy + dz * dz > MAX_DISTANCE * MAX_DISTANCE {
            continue;
        }
        let lead = cfg.rings.max(1);
        for index in 0..lead {
            let local = ripple_progress(progress, index, lead);
            if local <= 0.0 {
                continue;
            }
            let alpha = ring_alpha(local) * (1.0 - RIPPLE_THINNING * index as f32);
            if alpha <= 0.0 {
                continue;
            }
            let radius = f64::from(cfg.radius) * ring_scale(local);
            let points: Vec<Option<Pos2>> = ring_points(ring.center, radius)
                .into_iter()
                .map(|point| {
                    view.project(V3 {
                        x: point.x,
                        y: point.y + cfg.height,
                        z: point.z,
                    })
                })
                .collect();
            let segments = ring_segments(&points);
            if segments.is_empty() {
                continue;
            }
             
             
            if cfg.fill && index == 0 && points.iter().all(Option::is_some) {
                let disc: Vec<Pos2> = points.iter().flatten().copied().collect();
                painter.add(Shape::convex_polygon(
                    disc,
                    theme::with_alpha(cfg.color, alpha * FILL_ALPHA),
                    Stroke::NONE,
                ));
            }
            let width = cfg.thickness * (1.0 - RIPPLE_THINNING * index as f32);
            for segment in segments {
                painter.add(Shape::line(
                    segment,
                    Stroke::new(width, theme::with_alpha(cfg.color, alpha)),
                ));
            }
        }
    }
}

impl Module for JumpCirclesModule {
    fn on_start(&self) -> anyhow::Result<()> {
         
        clear_rings();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        clear_rings();
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        clear_rings();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let cfg = JumpCircleConfig::from_data(&self.module);
        let now = Instant::now();
        let Some(player) = minecraft().player()? else {
            clear_rings();
            return Ok(());
        };
        let Some(world) = minecraft().world()? else {
            clear_rings();
            return Ok(());
        };

        let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
        prune(&mut state.rings, now, cfg.fade);
        let mut seen = HashSet::new();

        for entity in world.get_entities()? {
            if cfg.players_only && !entity.instance_of::<Player>() {
                continue;
            }
            if entity.as_living().is_none() {
                continue;
            }
            let (Ok(id), Ok(grounded), Ok(position)) =
                (entity.id(), entity.on_ground(), entity.get_position())
            else {
                continue;
            };
            seen.insert(id);
            record(&mut state, id, grounded, V3::from(position), now);
        }
         
         
        if let (Ok(id), Ok(grounded), Ok(position)) =
            (player.id(), player.on_ground(), player.get_position())
        {
            seen.insert(id);
            record(&mut state, id, grounded, V3::from(position), now);
        }

        state.grounded.retain(|id, _| seen.contains(id));
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

 
fn record(state: &mut JumpState, id: i32, grounded: bool, feet: V3, now: Instant) {
    let previous = state.grounded.insert(id, grounded);
    if !is_jump(previous, grounded) {
        return;
    }
    if state.rings.len() >= MAX_RINGS {
        state.rings.remove(0);
    }
    state.rings.push(JumpRing {
        center: feet,
        spawned: now,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEET: V3 = V3 {
        x: 4.0,
        y: 70.0,
        z: -2.0,
    };

    #[test]
    fn only_leaving_the_ground_counts_as_a_jump() {
        assert!(is_jump(Some(true), false), "standing then airborne");
        assert!(!is_jump(Some(true), true), "standing still is no jump");
        assert!(!is_jump(Some(false), false), "already airborne");
        assert!(!is_jump(Some(false), true), "landing is not a jump");
        assert!(!is_jump(None, false), "first sight is no jump");
        assert!(!is_jump(None, true));
    }

    #[test]
    fn a_ring_is_a_flat_circle_around_the_feet() {
        let radius = 1.2;
        let points = ring_points(FEET, radius);
        assert_eq!(points.len(), RING_STEPS);
        assert!(points.iter().all(|p| (p.y - FEET.y).abs() < 1e-9));
        assert!(points.iter().all(|p| {
            let dx = p.x - FEET.x;
            let dz = p.z - FEET.z;
            ((dx * dx + dz * dz).sqrt() - radius).abs() < 1e-9
        }));
         
        assert!((points[0].x - (FEET.x + radius)).abs() < 1e-9);
        assert!((points[0].z - FEET.z).abs() < 1e-9);
    }

    #[test]
    fn a_radius_of_zero_is_a_single_point() {
        let points = ring_points(FEET, 0.0);
        assert!(points
            .iter()
            .all(|p| p.x == FEET.x && p.y == FEET.y && p.z == FEET.z));
    }

    #[test]
    fn rings_prune_oldest_first() {
        let now = Instant::now();
        let mut rings = vec![
            JumpRing {
                center: FEET,
                spawned: now - Duration::from_millis(900),
            },
            JumpRing {
                center: FEET,
                spawned: now - Duration::from_millis(600),
            },
            JumpRing {
                center: FEET,
                spawned: now,
            },
        ];
        prune(&mut rings, now, Duration::from_millis(500));
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0].spawned, now);
    }

    #[test]
    fn a_jump_starts_a_ring_only_once() {
        let now = Instant::now();
        let mut state = JumpState::default();
        record(&mut state, 7, true, FEET, now);
        assert!(state.rings.is_empty(), "standing starts nothing");
        record(&mut state, 7, false, FEET, now);
        assert_eq!(state.rings.len(), 1);
        record(&mut state, 7, false, FEET, now);
        assert_eq!(state.rings.len(), 1, "already airborne is no new jump");
        record(&mut state, 7, true, FEET, now);
        record(&mut state, 7, false, FEET, now);
        assert_eq!(state.rings.len(), 2, "landing and jumping again does");
    }

    #[test]
    fn the_ring_leaves_fast_and_settles() {
         
        let start = ring_scale(0.0);
        assert!((start - MIN_SCALE).abs() < 1e-9);
         
        let mut previous = start;
        for step in 1..=20 {
            let radius = ring_scale(step as f32 / 20.0);
            assert!(radius >= previous, "the ring only expands");
            previous = radius;
        }
        assert!((previous - 1.0).abs() < 1e-9);
         
        assert!(ring_scale(0.5) > 0.5);
         
        assert!((ring_scale(-1.0) - MIN_SCALE).abs() < 1e-9);
        assert!((ring_scale(2.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn the_alpha_snaps_in_then_eases_out() {
        assert_eq!(ring_alpha(0.0), 0.0);
        assert!(
            (ring_alpha(FADE_IN) - 1.0).abs() < 1e-6,
            "fully on at the top"
        );
        assert_eq!(ring_alpha(1.0), 0.0);
         
        let mut previous = 1.0;
        let mut step = FADE_IN;
        while step <= 1.0 {
            let alpha = ring_alpha(step);
            assert!(alpha <= previous + 1e-6, "the fade only goes one way");
            previous = alpha;
            step += 0.01;
        }
        assert!(ring_alpha(0.5) > 0.2, "still readable half way through");
    }

    #[test]
    fn ripples_start_later_and_all_finish_together() {
        assert_eq!(ripple_progress(0.4, 0, 1), 0.4, "one ripple is the ring");
         
        assert!(ripple_progress(0.2, 0, 2) > 0.0);
        assert_eq!(ripple_progress(0.1, 1, 2), 0.0, "not started yet");
         
        for index in 0..3 {
            assert!((ripple_progress(1.0, index, 3) - 1.0).abs() < 1e-6);
            assert_eq!(ripple_progress(0.0, index, 3), 0.0);
        }
         
        for step in 0..=20 {
            let at = step as f32 / 20.0;
            assert!(ripple_progress(at, 0, 3) >= ripple_progress(at, 1, 3));
            assert!(ripple_progress(at, 1, 3) >= ripple_progress(at, 2, 3));
        }
    }

    #[test]
    fn a_whole_ring_is_one_closed_loop() {
        let points: Vec<Option<Pos2>> = (0..8).map(|i| Some(Pos2::new(i as f32, 0.0))).collect();
        let segments = ring_segments(&points);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].len(), 9, "closed by repeating the first point");
        assert_eq!(segments[0][0], segments[0][8]);
    }

    #[test]
    fn hidden_points_become_gaps_instead_of_hiding_the_ring() {
        let mut points: Vec<Option<Pos2>> =
            (0..8).map(|i| Some(Pos2::new(i as f32, 0.0))).collect();
        points[2] = None;
        points[6] = None;
        let segments = ring_segments(&points);
        assert_eq!(segments.len(), 2, "the ring drew as two arcs");
        assert_eq!(segments[0].len(), 3, "points 3, 4 and 5");
        assert_eq!(segments[0][0], Pos2::new(3.0, 0.0));
        assert_eq!(segments[1].len(), 3, "points 7, 0 and 1");
        assert_eq!(segments[1].last().copied(), Some(Pos2::new(1.0, 0.0)));
    }

    #[test]
    fn an_arc_across_the_seam_stays_one_arc() {
        let mut points: Vec<Option<Pos2>> =
            (0..8).map(|i| Some(Pos2::new(i as f32, 0.0))).collect();
        points[3] = None;
        let segments = ring_segments(&points);
         
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].len(), 7);
        assert_eq!(
            segments[0][0],
            Pos2::new(4.0, 0.0),
            "the arc starts after the gap"
        );
        assert_eq!(segments[0].last().copied(), Some(Pos2::new(2.0, 0.0)));
    }

    #[test]
    fn a_ring_entirely_behind_the_camera_draws_nothing() {
        assert!(ring_segments(&[None; RING_STEPS]).is_empty());
        assert!(ring_segments(&[]).is_empty());
         
        let mut points = [None; RING_STEPS];
        points[3] = Some(Pos2::new(1.0, 2.0));
        assert!(ring_segments(&points).is_empty());
    }

    #[test]
    fn module_identity_and_defaults() {
        let module = JumpCirclesModule::new();
        assert_eq!(module.module.id, ModuleId::JumpCircles);
        assert_eq!(module.module.id.display_name(), "Jump Circles");
        assert_eq!(module.module.category, ModuleCategory::Visuals);
        assert!(!module.module.enabled);
        let cfg = JumpCircleConfig::from_data(&module.module);
        assert!(!cfg.enabled);
        assert_eq!(cfg.fade, Duration::from_secs_f32(DEFAULT_FADE));
        assert!((f64::from(cfg.radius) - 1.5).abs() < 1e-6);
        assert!((cfg.height - 0.05).abs() < 1e-9);
        assert_eq!(cfg.rings, 2);
        assert!((cfg.thickness - 2.0).abs() < 1e-6);
        assert!(cfg.fill);
        assert!(!cfg.players_only);
    }

    #[test]
    fn the_default_colour_is_red() {
        let cfg = JumpCircleConfig::default();
        assert_eq!(cfg.color, DEFAULT_COLOR);
        assert!(cfg.color.r() > 200, "a red, not a pink or an orange");
        assert!(cfg.color.r() > cfg.color.g() + 150);
        assert!(cfg.color.r() > cfg.color.b() + 150);
        assert_eq!(
            cfg.color,
            color_setting(&JumpCirclesModule::new().module, "Color", Color32::WHITE),
            "the setting carries the red through"
        );
    }

    #[test]
    fn settings_are_editable_and_clamped() {
        let mut module = JumpCirclesModule::new();
        module
            .module
            .get_setting_mut("Radius")
            .unwrap()
            .set_slider_value(99.0);
        module
            .module
            .get_setting_mut("Fade Time")
            .unwrap()
            .set_slider_value(f32::NAN);
        let cfg = JumpCircleConfig::from_data(&module.module);
        assert!((f64::from(cfg.radius) - 5.0).abs() < 1e-6);
        assert_eq!(cfg.fade, Duration::from_secs_f32(DEFAULT_FADE));
        module
            .module
            .get_setting_mut("Height")
            .unwrap()
            .set_slider_value(1.0);
        assert!((JumpCircleConfig::from_data(&module.module).height - 0.5).abs() < 1e-9);
         
        module
            .module
            .get_setting_mut("Rings")
            .unwrap()
            .set_slider_value(9.0);
        assert_eq!(JumpCircleConfig::from_data(&module.module).rings, 3);
        module
            .module
            .get_setting_mut("Rings")
            .unwrap()
            .set_slider_value(0.0);
        assert_eq!(JumpCircleConfig::from_data(&module.module).rings, 1);
        module
            .module
            .get_setting_mut("Fill")
            .unwrap()
            .set_toggle_value(false);
        assert!(!JumpCircleConfig::from_data(&module.module).fill);
        module
            .module
            .get_setting_mut("Players Only")
            .unwrap()
            .set_toggle_value(true);
        assert!(JumpCircleConfig::from_data(&module.module).players_only);
    }
}
