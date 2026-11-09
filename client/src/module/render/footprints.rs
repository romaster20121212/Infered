 
 
 
 
 
 
 

use crate::graphic::esp::{toggle_setting, View, V3};
use crate::graphic::theme;
use crate::mapping::entity::EntityRef;
use crate::module::player::autototem::roll;
use crate::module::render::trail::{
    bounded, is_red, life, should_record, trail_color, GLOW_ALPHA, MAX_POINTS, TELEPORT_DISTANCE,
};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, minecraft};
use egui::{Color32, Painter, Pos2, Shape, Stroke};
use std::f64::consts::FRAC_PI_2;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const DEFAULT_FADE: f32 = 3.0;
const DEFAULT_SIZE: f32 = 0.3;
const DEFAULT_SPACING: f32 = 0.7;
const DEFAULT_HEIGHT: f32 = 0.02;

 
const FOOT_OFFSET: f64 = 0.12;
 
const CORNER_RATIO: f64 = 0.4;
 
const ARC_STEPS: usize = 3;
 
const GLOW_SCALE: f64 = 1.6;

 
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FootprintConfig {
    pub(crate) enabled: bool,
    fade: Duration,
    size: f32,
    spacing: f64,
    height: f64,
    glow: bool,
}

impl FootprintConfig {
    fn from_data(data: &ModuleData) -> Self {
        Self {
            enabled: data.enabled,
            fade: Duration::from_secs_f32(bounded(data, "Fade Time", DEFAULT_FADE, 0.5, 10.0)),
            size: bounded(data, "Size", DEFAULT_SIZE, 0.1, 1.0),
            spacing: bounded(data, "Spacing", DEFAULT_SPACING, 0.2, 3.0) as f64,
            height: bounded(data, "Height", DEFAULT_HEIGHT, 0.0, 0.5) as f64,
            glow: toggle_setting(data, "Glow", true),
        }
    }
}

 
#[derive(Debug, Clone, Copy)]
struct Footprint {
     
    pos: V3,
     
    origin: V3,
    yaw: f32,
    spawned: Instant,
    color: Color32,
}

 
#[derive(Default)]
struct FootprintState {
    points: Vec<Footprint>,
    next_is_right: bool,
}

fn state() -> &'static Mutex<FootprintState> {
    static STATE: OnceLock<Mutex<FootprintState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(FootprintState::default()))
}

fn clear_points() {
    *state().lock().unwrap_or_else(|p| p.into_inner()) = FootprintState::default();
}

 

 
fn ground_axes(yaw: f32) -> ((f64, f64), (f64, f64)) {
    let yaw_r = f64::from(yaw).to_radians();
    ((-yaw_r.sin(), yaw_r.cos()), (-yaw_r.cos(), -yaw_r.sin()))
}

 
fn foot_anchor(feet: V3, yaw: f32, side: f64) -> V3 {
    let (_, (rx, rz)) = ground_axes(yaw);
    V3 {
        x: feet.x + rx * FOOT_OFFSET * side,
        y: feet.y,
        z: feet.z + rz * FOOT_OFFSET * side,
    }
}

 
 
fn rounded_square(center: V3, yaw: f32, half: f64) -> Vec<V3> {
    let ((fx, fz), (rx, rz)) = ground_axes(yaw);
    let radius = half * CORNER_RATIO;
    let inner = half - radius;
    let quadrants = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)];
    let mut outline = Vec::with_capacity(4 * ARC_STEPS);
    for (quadrant, (su, sv)) in quadrants.into_iter().enumerate() {
        let base = quadrant as f64 * FRAC_PI_2;
        for step in 0..ARC_STEPS {
            let angle = base + FRAC_PI_2 * step as f64 / ARC_STEPS as f64;
             
            let u = su * inner + radius * angle.cos();
            let v = sv * inner + radius * angle.sin();
            outline.push(V3 {
                x: center.x + rx * u + fx * v,
                y: center.y,
                z: center.z + rz * u + fz * v,
            });
        }
    }
    outline
}

 
fn project_outline(view: &View, center: V3, yaw: f32, half: f64, lift: f64) -> Option<Vec<Pos2>> {
    rounded_square(center, yaw, half)
        .into_iter()
        .map(|p| {
            view.project(V3 {
                x: p.x,
                y: p.y + lift,
                z: p.z,
            })
        })
        .collect()
}

 
 
fn prune(points: &mut Vec<Footprint>, now: Instant, fade: Duration) {
    let expired = points
        .iter()
        .take_while(|print| life(now.duration_since(print.spawned), fade) <= 0.0)
        .count();
    points.drain(..expired);
}

 

 
#[derive(Debug)]
pub struct FootprintsModule {
    pub module: ModuleData,
}

impl FootprintsModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Footprints,
                description: "Rounded neon footprints left on the ground as you walk".to_string(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Fade Time".to_string(),
                        value: DEFAULT_FADE,
                        min: 0.5,
                        max: 10.0,
                    },
                    ModuleSetting::Slider {
                        name: "Size".to_string(),
                        value: DEFAULT_SIZE,
                        min: 0.1,
                        max: 1.0,
                    },
                    ModuleSetting::Slider {
                        name: "Spacing".to_string(),
                        value: DEFAULT_SPACING,
                        min: 0.2,
                        max: 3.0,
                    },
                    ModuleSetting::Slider {
                        name: "Height".to_string(),
                        value: DEFAULT_HEIGHT,
                        min: 0.0,
                        max: 0.5,
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

 
 
pub(crate) fn read_config() -> FootprintConfig {
    let mut cfg = FootprintConfig::default();
    if let Some(arc) = client().modules.get(ModuleId::Footprints) {
        if let Ok(module) = arc.lock() {
            cfg = FootprintConfig::from_data(module.get_module_data());
        }
    }
    cfg
}

 
pub(crate) fn draw(painter: &Painter, view: &View, now: Instant, cfg: &FootprintConfig) {
    if !cfg.enabled {
        return;
    }
    let state = state().lock().unwrap_or_else(|p| p.into_inner());
    for print in &state.points {
        let alive = life(now.duration_since(print.spawned), cfg.fade);
        if alive <= 0.0 {
            continue;
        }
         
        let half = f64::from(cfg.size) * 0.5 * f64::from(alive);
        let Some(outline) = project_outline(view, print.pos, print.yaw, half, cfg.height) else {
            continue;
        };
        if cfg.glow && is_red(print.color) {
            if let Some(halo) =
                project_outline(view, print.pos, print.yaw, half * GLOW_SCALE, cfg.height)
            {
                painter.add(Shape::convex_polygon(
                    halo,
                    theme::with_alpha(print.color, GLOW_ALPHA * alive),
                    Stroke::NONE,
                ));
            }
        }
        painter.add(Shape::convex_polygon(
            outline,
            theme::with_alpha(print.color, alive),
            Stroke::NONE,
        ));
    }
}

impl Module for FootprintsModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        clear_points();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let cfg = FootprintConfig::from_data(&self.module);
        let now = Instant::now();
        let Some(player) = minecraft().player()? else {
             
            clear_points();
            return Ok(());
        };
        let feet = V3::from(player.get_position()?);
        let yaw = player.get_yaw()?;
        let grounded = player.on_ground()?;

        let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
        prune(&mut state.points, now, cfg.fade);
        let teleported = state
            .points
            .last()
            .is_some_and(|print| feet.sub(print.origin).length() > TELEPORT_DISTANCE);
        if teleported {
            state.points.clear();
        }

        let last = state.points.last().map(|print| print.origin);
        if grounded && should_record(last, feet, cfg.spacing) {
            if state.points.len() >= MAX_POINTS {
                state.points.remove(0);
            }
            let side = if state.next_is_right { 1.0 } else { -1.0 };
            state.next_is_right = !state.next_is_right;
            state.points.push(Footprint {
                pos: foot_anchor(feet, yaw, side),
                origin: feet,
                yaw,
                spawned: now,
                color: trail_color(roll() as f32),
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

    const FEET: V3 = V3 {
        x: 0.0,
        y: 64.0,
        z: 0.0,
    };

    #[test]
    fn the_two_feet_land_on_opposite_sides_of_the_body() {
         
        let left = foot_anchor(FEET, 0.0, -1.0);
        let right = foot_anchor(FEET, 0.0, 1.0);
        assert!(left.x > 0.0 && right.x < 0.0);
        assert!((left.x + right.x).abs() < 1e-9);
        assert_eq!(left.y, FEET.y);
        assert_eq!(right.y, FEET.y);
    }

    #[test]
    fn a_footprint_is_a_rounded_square_of_the_given_size() {
        let half = 0.15;
        let outline = rounded_square(FEET, 0.0, half);
        assert_eq!(outline.len(), 4 * ARC_STEPS);
         
        assert!(outline
            .iter()
            .all(|p| p.x.abs() <= half + 1e-9 && (p.z - FEET.z).abs() <= half + 1e-9));
         
        assert!(outline
            .iter()
            .all(|p| !(p.x.abs() > half - 1e-9 && p.z.abs() > half - 1e-9)));
         
        assert!(outline.iter().any(|p| (p.x.abs() - half).abs() < 1e-9));
        assert!(outline.iter().any(|p| (p.z.abs() - half).abs() < 1e-9));
    }

    #[test]
    fn a_footprint_is_centred_on_its_print_point() {
        let outline = rounded_square(FEET, 37.0, 0.2);
        let n = outline.len() as f64;
        let cx = outline.iter().map(|p| p.x).sum::<f64>() / n;
        let cz = outline.iter().map(|p| p.z).sum::<f64>() / n;
        assert!(cx.abs() < 1e-9);
        assert!(cz.abs() < 1e-9);
    }

    #[test]
    fn the_module_defaults_are_disabled_with_a_glow() {
        let module = FootprintsModule::new();
        assert!(!module.module.enabled);
        assert_eq!(module.module.id, ModuleId::Footprints);
        let cfg = FootprintConfig::from_data(&module.module);
        assert!(cfg.glow);
        assert_eq!(cfg.fade, Duration::from_secs_f32(DEFAULT_FADE));
    }
}
