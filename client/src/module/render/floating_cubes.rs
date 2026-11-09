 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::esp::{color_setting, toggle_setting, View, V3};
use crate::module::render::trail::bounded;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::client;
use egui::{Color32, Painter, Pos2, Shape, Stroke};
use std::f64::consts::TAU;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

const DEFAULT_CUBES: f32 = 10.0;
const DEFAULT_RANGE: f32 = 14.0;
const DEFAULT_SIZE: f32 = 0.45;
const DEFAULT_SPIN: f32 = 1.0;
const DEFAULT_DRIFT: f32 = 1.0;
const DEFAULT_WIDTH: f32 = 1.4;
const DEFAULT_GLOW: bool = true;
 
const DEFAULT_COLOR: Color32 = Color32::from_rgb(255, 0, 0);

 
 
const MAX_CUBES: usize = 32;
 
const MIN_RADIUS: f64 = 2.2;
 
const EDGE_ALPHA: f32 = 0.8;
 
 
 
const FACE_ALPHA: f32 = 0.18;
 
const STROKE_FADE: f32 = 0.45;
 
const GLOW_ALPHA: f32 = 0.16;
 
const GLOW_WIDTH: f32 = 3.0;

 
 
 
 
const CUBE_FACES: [[usize; 4]; 6] = [
    [0, 2, 6, 4],
    [1, 5, 7, 3],
    [0, 1, 5, 4],
    [2, 3, 7, 6],
    [0, 1, 3, 2],
    [4, 5, 7, 6],
];

const CUBE_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 3),
    (3, 2),
    (2, 0),
    (4, 5),
    (5, 7),
    (7, 6),
    (6, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

 
static CLOCK: LazyLock<Instant> = LazyLock::new(Instant::now);

 
 
static FIELD: LazyLock<Mutex<Option<V3>>> = LazyLock::new(|| Mutex::new(None));

 
 
const EYE_DROP: f64 = 0.25;

 
fn clock() -> f64 {
    CLOCK.elapsed().as_secs_f64()
}

 
#[derive(Debug, Clone, Copy)]
pub(crate) struct CubeConfig {
    pub(crate) enabled: bool,
    cubes: usize,
    range: f64,
    size: f64,
    spin: f64,
    drift: f64,
    width: f32,
    color: Color32,
    glow: bool,
     
    faces: bool,
}

impl Default for CubeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cubes: DEFAULT_CUBES as usize,
            range: DEFAULT_RANGE as f64,
            size: DEFAULT_SIZE as f64,
            spin: DEFAULT_SPIN as f64,
            drift: DEFAULT_DRIFT as f64,
            width: DEFAULT_WIDTH,
            color: DEFAULT_COLOR,
            glow: DEFAULT_GLOW,
            faces: true,
        }
    }
}

impl CubeConfig {
    fn from_data(data: &ModuleData) -> Self {
        Self {
            enabled: data.enabled,
            cubes: (bounded(data, "Cubes", DEFAULT_CUBES, 1.0, MAX_CUBES as f32).round() as usize)
                .clamp(1, MAX_CUBES),
            range: bounded(data, "Range", DEFAULT_RANGE, 4.0, 48.0) as f64,
            size: bounded(data, "Size", DEFAULT_SIZE, 0.1, 1.2) as f64,
            spin: bounded(data, "Spin", DEFAULT_SPIN, 0.1, 3.0) as f64,
            drift: bounded(data, "Drift", DEFAULT_DRIFT, 0.1, 3.0) as f64,
            width: bounded(data, "Width", DEFAULT_WIDTH, 0.5, 5.0),
            color: color_setting(data, "Color", DEFAULT_COLOR),
            glow: toggle_setting(data, "Glow", DEFAULT_GLOW),
            faces: toggle_setting(data, "Faces", true),
        }
    }
}

 
pub(crate) fn read_config() -> CubeConfig {
    let mut cfg = CubeConfig::default();
    if let Some(arc) = client().modules.get(ModuleId::FloatingCubes) {
        if let Ok(module) = arc.lock() {
            cfg = CubeConfig::from_data(module.get_module_data());
        }
    }
    cfg
}

 

fn add(a: V3, b: V3) -> V3 {
    V3 {
        x: a.x + b.x,
        y: a.y + b.y,
        z: a.z + b.z,
    }
}

fn scale(a: V3, k: f64) -> V3 {
    V3 {
        x: a.x * k,
        y: a.y * k,
        z: a.z * k,
    }
}

 
fn normalized(a: V3, fallback: V3) -> V3 {
    let length = a.length();
    if length < 1e-6 {
        fallback
    } else {
        scale(a, 1.0 / length)
    }
}

 
fn rotate(axis: V3, angle: f64, v: V3) -> V3 {
    let (sin, cos) = angle.sin_cos();
    let cross = axis.cross(v);
    let along = scale(axis, axis.dot(v) * (1.0 - cos));
    add(add(scale(v, cos), scale(cross, sin)), along)
}

 
 
fn unit(index: usize, salt: u64) -> f64 {
    let mut z = (index as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(salt.wrapping_mul(0xBF58_476D_1CE4_E5B9));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

 
fn corner_local(corner: usize) -> (f64, f64, f64) {
    let axis = |bit: usize| if corner & bit == 0 { -1.0 } else { 1.0 };
    (axis(1), axis(2), axis(4))
}

 
fn tint(base: Color32, alpha: f32) -> Color32 {
    let alpha = (alpha.clamp(0.0, 1.0) * base.a() as f32) as u8;
    Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha)
}

 
 
 
fn cube_alpha(cube: &CubeFrame, cam: V3, range: f64) -> f32 {
    distance_alpha(cube.center.sub(cam).length(), range) * cube.life
}

 
const LIFE_FADE_SECONDS: f64 = 3.0;
 
const LIFE_SECONDS: f64 = 14.0;

 
 
 
 
fn cube_life(index: usize, time: f64) -> f32 {
    let period = LIFE_SECONDS;
    let phase = (time / period + unit(index, 20)).rem_euclid(1.0) * period;
    let fade_in = smoothstep01((phase / LIFE_FADE_SECONDS) as f32);
    let fade_out = smoothstep01(((period - phase) / LIFE_FADE_SECONDS) as f32);
    fade_in.min(fade_out)
}

fn smoothstep01(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

 
 
fn distance_alpha(distance: f64, range: f64) -> f32 {
    if range <= 0.0 || !distance.is_finite() {
        return 0.0;
    }
    let near = ((distance - 1.0) / 2.0).clamp(0.0, 1.0);
    let far = ((range - distance) / (range * 0.35).max(1.0)).clamp(0.0, 1.0);
    (near * far) as f32
}

 
 
 
 
 
 
 
fn field_anchor(cam: V3, range: f64) -> V3 {
    let eye = V3 {
        x: cam.x,
        y: cam.y - EYE_DROP,
        z: cam.z,
    };
    let mut slot = FIELD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match *slot {
        Some(anchor) if anchor.sub(eye).length() < range.max(MIN_RADIUS) => anchor,
        _ => {
            *slot = Some(eye);
            eye
        }
    }
}

 
 
 
pub(crate) fn clear_field() {
    *FIELD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
}

 
#[derive(Debug, Clone, Copy)]
struct CubeFrame {
    center: V3,
    corners: [V3; 8],
     
    life: f32,
}

 
 
 
 
fn cube_frame(index: usize, cfg: &CubeConfig, anchor: V3, time: f64) -> CubeFrame {
     
    let azimuth = TAU * unit(index, 1);
    let elevation = -0.55 + 1.25 * unit(index, 2);
    let radius = (cfg.range * (0.30 + 0.70 * unit(index, 3))).max(MIN_RADIUS);
     
     
    let orbit = time * (0.014 + 0.02 * unit(index, 4)) * cfg.drift;
    let azimuth = azimuth + orbit;
    let base = V3 {
        x: azimuth.cos() * elevation.cos() * radius,
        y: elevation.sin() * radius * 0.75,
        z: azimuth.sin() * elevation.cos() * radius,
    };

     
     
    let amplitude = 0.5 + 1.2 * unit(index, 5);
    let speed_x = (0.09 + 0.07 * unit(index, 6)) * cfg.drift;
    let speed_y = (0.07 + 0.05 * unit(index, 7)) * cfg.drift;
    let speed_z = (0.09 + 0.07 * unit(index, 8)) * cfg.drift;
    let phase_x = TAU * unit(index, 9);
    let phase_y = TAU * unit(index, 10);
    let phase_z = TAU * unit(index, 11);
    let drift = V3 {
        x: (time * speed_x + phase_x).sin() * amplitude,
        y: (time * speed_y + phase_y).sin() * amplitude * 0.55,
        z: (time * speed_z + phase_z).sin() * amplitude,
    };
    let center = add(anchor, add(base, drift));

     
    let seed_axis = V3 {
        x: unit(index, 12) - 0.5,
        y: unit(index, 13) - 0.5,
        z: unit(index, 14) - 0.5,
    };
    let seed_axis = normalized(
        seed_axis,
        V3 {
            x: 1.0,
            y: 0.4,
            z: 0.2,
        },
    );
     
     
    let wobble = V3 {
        x: 0.35 * (time * 0.05 * cfg.spin + TAU * unit(index, 15)).sin(),
        y: 0.35 * (time * 0.041 * cfg.spin + TAU * unit(index, 16)).cos(),
        z: 0.30 * (time * 0.037 * cfg.spin + TAU * unit(index, 17)).sin(),
    };
    let axis = normalized(
        add(seed_axis, wobble),
        V3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
    );
    let angle = time * (0.12 + 0.30 * unit(index, 18)) * cfg.spin;

    let corners = std::array::from_fn(|corner| {
        let (x, y, z) = corner_local(corner);
        let local = scale(V3 { x, y, z }, cfg.size);
        add(center, rotate(axis, angle, local))
    });

     
    CubeFrame {
        center,
        corners,
        life: cube_life(index, time),
    }
}

 
fn face_centre(corners: &[V3; 8], quad: &[usize; 4]) -> V3 {
    let sum = quad.iter().fold(
        V3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        |acc, &corner| add(acc, corners[corner]),
    );
    scale(sum, 0.25)
}

 
 
 
 
 
 
fn visible_faces(cam: V3, cube: &CubeFrame) -> Vec<(usize, f64)> {
    let mut faces: Vec<(usize, f64)> = CUBE_FACES
        .iter()
        .enumerate()
        .filter_map(|(index, quad)| {
            let centre = face_centre(&cube.corners, quad);
            let outward = centre.sub(cube.center);
            (outward.dot(cam.sub(centre)) > 0.0).then(|| (index, centre.sub(cam).length()))
        })
        .collect();
    faces.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    faces
}

 
 
 
fn face_shade(outward: V3) -> f32 {
    let length = outward.length();
    if length < 1e-6 {
        return 1.0;
    }
    let up = (outward.y / length) as f32;
    (0.8 + 0.4 * (0.5 + 0.5 * up)).clamp(0.65, 1.2)
}

 
 
 
fn stroke_scale(distance: f64, range: f64) -> f32 {
    let t = (distance / range.max(1e-3)).clamp(0.0, 1.0) as f32;
    1.0 - STROKE_FADE * t
}

 
 
pub(crate) fn draw(painter: &Painter, view: &View, cfg: &CubeConfig) {
    if !cfg.enabled || cfg.cubes == 0 {
        return;
    }
    let time = clock();
    let anchor = field_anchor(view.cam, cfg.range);

    for index in 0..cfg.cubes.min(MAX_CUBES) {
        let cube = cube_frame(index, cfg, anchor, time);
         
         
        let alpha = cube_alpha(&cube, view.cam, cfg.range) * EDGE_ALPHA;
        if alpha <= 0.01 {
            continue;
        }
        let screen: [Option<Pos2>; 8] =
            std::array::from_fn(|corner| view.project(cube.corners[corner]));
         
         
        if screen.iter().all(Option::is_none) {
            continue;
        }
        let distance = cube.center.sub(view.cam).length();
        let scale = stroke_scale(distance, cfg.range);
        let width = (cfg.width * scale).max(0.5);

         
         
        if cfg.faces {
            for (face, _) in visible_faces(view.cam, &cube) {
                let quad = CUBE_FACES[face];
                let points: Vec<Pos2> = quad.iter().filter_map(|&corner| screen[corner]).collect();
                if points.len() < 3 {
                    continue;
                }
                let outward = face_centre(&cube.corners, &quad).sub(cube.center);
                let shade = face_shade(outward);
                let face = tint(cfg.color, (alpha * FACE_ALPHA * shade).clamp(0.0, 1.0));
                painter.add(Shape::convex_polygon(points, face, Stroke::NONE));
            }
        }

        for &(a, b) in CUBE_EDGES.iter() {
            let (Some(from), Some(to)) = (screen[a], screen[b]) else {
                continue;
            };
            if cfg.glow {
                painter.line_segment(
                    [from, to],
                    Stroke::new(
                        width * GLOW_WIDTH,
                        tint(cfg.color, alpha * GLOW_ALPHA * scale),
                    ),
                );
            }
            painter.line_segment([from, to], Stroke::new(width, tint(cfg.color, alpha)));
        }

         
         
        for corner in screen.iter().flatten() {
            painter.circle_filled(
                *corner,
                (width * 0.85).max(0.6),
                tint(cfg.color, alpha * 0.9),
            );
        }
    }
}

 
#[derive(Debug)]
pub struct FloatingCubesModule {
    pub module: ModuleData,
}

impl FloatingCubesModule {
    pub fn new() -> Self {
        let slider = |name: &str, value: f32, min: f32, max: f32| ModuleSetting::Slider {
            name: name.into(),
            value,
            min,
            max,
        };
        Self {
            module: ModuleData {
                id: ModuleId::FloatingCubes,
                description:
                    "Transparent wireframe cubes drifting around you, tumbling slowly while \
                              their colours sweep across each shape"
                        .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Color {
                        name: "Color".into(),
                        value: [1.0, 0.0, 0.0, 1.0],
                    },
                    slider("Cubes", DEFAULT_CUBES, 1.0, MAX_CUBES as f32),
                    slider("Range", DEFAULT_RANGE, 4.0, 48.0),
                    slider("Size", DEFAULT_SIZE, 0.1, 1.2),
                    slider("Spin", DEFAULT_SPIN, 0.1, 3.0),
                    slider("Drift", DEFAULT_DRIFT, 0.1, 3.0),
                    slider("Width", DEFAULT_WIDTH, 0.5, 5.0),
                    ModuleSetting::Toggle {
                        name: "Faces".into(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Glow".into(),
                        value: DEFAULT_GLOW,
                    },
                ],
            },
        }
    }
}

impl Module for FloatingCubesModule {
    fn on_start(&self) -> anyhow::Result<()> {
         
        clear_field();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        clear_field();
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
         
        clear_field();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
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

    fn cfg(cubes: usize) -> CubeConfig {
        CubeConfig {
            enabled: true,
            cubes,
            ..CubeConfig::default()
        }
    }

    fn anchor() -> V3 {
        V3 {
            x: 100.0,
            y: 64.0,
            z: -30.0,
        }
    }

     
     
    #[test]
    fn the_edges_are_the_twelve_cube_edges() {
        let mut seen = Vec::new();
        for (a, b) in CUBE_EDGES {
            assert!(a < 8 && b < 8);
            let diff = a ^ b;
            assert_eq!(diff.count_ones(), 1, "edge {a}-{b} is not axis-aligned");
            let pair = (a.min(b), a.max(b));
            assert!(!seen.contains(&pair), "edge {pair:?} appears twice");
            seen.push(pair);
        }
        assert_eq!(seen.len(), 12);
    }

     
     
    #[test]
    fn the_faces_are_the_six_cube_faces() {
        let mut seen: Vec<(usize, usize)> = Vec::new();
        for (index, quad) in CUBE_FACES.iter().enumerate() {
             
             
            let shared = (0..3)
                .find(|axis| {
                    quad.iter()
                        .all(|corner| (corner >> axis) & 1 == (quad[0] >> axis) & 1)
                })
                .unwrap_or_else(|| panic!("face {index} {quad:?} is not planar"));
            assert!(
                (0..3)
                    .filter(|axis| quad
                        .iter()
                        .all(|corner| (corner >> axis) & 1 == (quad[0] >> axis) & 1))
                    .count()
                    == 1,
                "face {index} {quad:?} lies on more than one plane"
            );
            let key = (shared, (quad[0] >> shared) & 1);
            assert!(!seen.contains(&key), "face {index} repeats plane {key:?}");
            seen.push(key);
             
            for side in 0..4 {
                let (a, b) = (quad[side], quad[(side + 1) % 4]);
                assert_eq!((a ^ b).count_ones(), 1, "face {index} has diagonal {a}-{b}");
            }
        }
        assert_eq!(seen.len(), 6, "a cube has six faces");
    }

     
     
     
     
    #[test]
    fn only_the_faces_turned_toward_the_camera_are_drawn() {
        let cfg = cfg(1);
        let cube = cube_frame(0, &cfg, anchor(), 2.0);
        let front = visible_faces(
            V3 {
                x: cube.center.x,
                y: cube.center.y,
                z: cube.center.z - 6.0,
            },
            &cube,
        );
        assert_eq!(front.len(), 3, "half a cube's faces show from any side");
        for pair in front.windows(2) {
            assert!(
                pair[0].1 >= pair[1].1,
                "faces must be painted farthest first: {front:?}"
            );
        }
        let back = visible_faces(
            V3 {
                x: cube.center.x,
                y: cube.center.y,
                z: cube.center.z + 6.0,
            },
            &cube,
        );
        assert_eq!(back.len(), 3);
        for (face, _) in &back {
            assert!(
                !front.iter().any(|(shown, _)| shown == face),
                "face {face} shows from both sides"
            );
        }
    }

     
     
    #[test]
    fn faces_are_shaded_by_the_way_they_point() {
        let up = face_shade(V3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        });
        let side = face_shade(V3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let down = face_shade(V3 {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        });
        assert!(up > side && side > down, "{up} {side} {down}");
        assert!(down >= 0.65, "the underside went black: {down}");
        assert!(up <= 1.2, "the top blew out: {up}");
        assert_eq!(
            face_shade(V3 {
                x: 0.0,
                y: 0.0,
                z: 0.0
            }),
            1.0
        );
    }

     
     
    #[test]
    fn nearer_cubes_get_bolder_strokes() {
        let near = stroke_scale(0.0, 14.0);
        let far = stroke_scale(14.0, 14.0);
        assert_eq!(near, 1.0);
        assert!(near > far);
        assert!((far - (1.0 - STROKE_FADE)).abs() < 1e-6, "{far}");
        assert_eq!(
            stroke_scale(99.0, 14.0),
            far,
            "the fade must not go negative"
        );
        assert!(
            stroke_scale(1.0, 0.0).is_finite(),
            "a zero range must not divide by zero"
        );
    }

     
     
     
    #[test]
    fn corners_are_a_cube_around_its_centre() {
        let cfg = cfg(1);
        for &t in &[0.0, 7.3, 120.0] {
            let cube = cube_frame(0, &cfg, anchor(), t);
            let diagonal = cfg.size * 3f64.sqrt();
            for corner in cube.corners {
                let offset = corner.sub(cube.center);
                assert!((offset.length() - diagonal).abs() < 1e-9);
            }
            let edge = 2.0 * cfg.size;
            let face = edge * 2f64.sqrt();
            for a in 0..8 {
                for b in (a + 1)..8 {
                    let gap = cube.corners[a].sub(cube.corners[b]).length();
                    let matches = [edge, face, edge * 3f64.sqrt()]
                        .iter()
                        .any(|want| (gap - want).abs() < 1e-9);
                    assert!(matches, "corners {a} and {b} are {gap} apart");
                }
            }
        }
    }

     
     
    #[test]
    fn rotation_preserves_shape() {
        let axis = normalized(
            V3 {
                x: 0.3,
                y: -0.8,
                z: 0.5,
            },
            V3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
        );
        let v = V3 {
            x: 1.0,
            y: -2.0,
            z: 0.5,
        };
        let same = rotate(axis, 0.0, v);
        assert!((same.x - v.x).abs() < 1e-12);
        assert!((same.y - v.y).abs() < 1e-12);
        assert!((same.z - v.z).abs() < 1e-12);
        for angle in [0.3, 1.7, 12.0] {
            let turned = rotate(axis, angle, v);
            assert!((turned.length() - v.length()).abs() < 1e-9);
        }
    }

     
    #[test]
    fn the_drift_is_smooth_over_a_frame() {
        let cfg = cfg(8);
        let frame = 1.0 / 60.0;
        for index in 0..8 {
            for &t in &[0.0, 3.5, 61.0, 900.0] {
                let a = cube_frame(index, &cfg, anchor(), t);
                let b = cube_frame(index, &cfg, anchor(), t + frame);
                let step = a.center.sub(b.center).length();
                assert!(step < 0.05, "cube {index} jumped {step} blocks in a frame");
            }
        }
    }

     
     
    #[test]
    fn cubes_stay_in_their_shell() {
        let cfg = cfg(16);
        for index in 0..16 {
            for &t in &[0.0, 20.0, 400.0] {
                let cube = cube_frame(index, &cfg, anchor(), t);
                let distance = cube.center.sub(anchor()).length();
                assert!(distance >= MIN_RADIUS, "cube {index} on the lens");
                assert!(
                    distance <= cfg.range + 2.5,
                    "cube {index} drifted out of range: {distance}"
                );
            }
        }
    }

     
     
     
     
    #[test]
    fn the_field_stays_in_the_world_while_the_player_walks() {
        clear_field();
        let start = anchor();
        let seeded = field_anchor(start, 14.0);
        assert!((seeded.y - (start.y - EYE_DROP)).abs() < 1e-9);

        let walked = V3 {
            x: start.x + 5.0,
            y: start.y + 1.0,
            z: start.z - 3.0,
        };
        let held = field_anchor(walked, 14.0);
        assert!(
            (held.x - seeded.x).abs() < 1e-9,
            "the field followed the player"
        );
        assert!(
            (held.z - seeded.z).abs() < 1e-9,
            "the field followed the player"
        );

        let away = V3 {
            x: start.x + 60.0,
            y: start.y,
            z: start.z,
        };
        let reseeded = field_anchor(away, 14.0);
        assert!(
            (reseeded.x - away.x).abs() < 1e-9,
            "walking clear of the field must seed a new one: {reseeded:?}"
        );
        assert!((reseeded.y - (away.y - EYE_DROP)).abs() < 1e-9);
        clear_field();
    }

     
     
    #[test]
    fn a_cube_grows_as_the_player_walks_toward_it() {
        let cfg = cfg(6);
        let (w, h) = (640, 360);
        for index in 0..6 {
            let cube = cube_frame(index, &cfg, anchor(), 5.0);
            let near_cam = V3 {
                x: cube.center.x,
                y: cube.center.y,
                z: cube.center.z - 3.0,
            };
            let far_cam = V3 {
                x: cube.center.x,
                y: cube.center.y,
                z: cube.center.z - 12.0,
            };
            let near = screen_span(&view_looking_at(near_cam, cube.center, w, h), cube.corners);
            let far = screen_span(&view_looking_at(far_cam, cube.center, w, h), cube.corners);
            assert!(far > 0.5, "cube {index} is not drawn at all: {far} px");
            assert!(
                near > far * 2.5,
                "cube {index} did not grow when walked up to: near {near} px, far {far} px"
            );
        }
    }

     
    #[test]
    fn a_cube_fades_as_the_player_walks_away() {
        let cfg = cfg(1);
        let cube = cube_frame(0, &cfg, anchor(), 5.0);
        let close = V3 {
            x: cube.center.x,
            y: cube.center.y,
            z: cube.center.z - 4.0,
        };
        let distant = V3 {
            x: cube.center.x + 40.0,
            y: cube.center.y,
            z: cube.center.z,
        };
        assert!(cube_alpha(&cube, close, cfg.range) > 0.9);
        assert_eq!(cube_alpha(&cube, distant, cfg.range), 0.0);
    }

     
    #[test]
    fn the_default_colour_is_red() {
        assert_eq!(CubeConfig::default().color, Color32::from_rgb(255, 0, 0));
    }

     
    #[test]
    fn tint_keeps_the_chosen_rgb() {
        let blue = Color32::from_rgb(0, 80, 255);
        let faded = tint(blue, 0.5);
         
        let [r, g, b, a] = faded.to_srgba_unmultiplied();
        assert!(r.abs_diff(0) <= 2 && g.abs_diff(80) <= 2 && b.abs_diff(255) <= 2);
        assert!(a > 100 && a < 140, "alpha {a}");
    }

     
    #[test]
    fn distance_fades_both_ends() {
        assert_eq!(distance_alpha(0.5, 14.0), 0.0);
        assert!(distance_alpha(6.0, 14.0) > 0.9);
        assert_eq!(distance_alpha(14.0, 14.0), 0.0);
        assert_eq!(distance_alpha(30.0, 14.0), 0.0);
    }

     
     
    #[test]
    fn the_config_clamps_every_slider() {
        let mut module = FloatingCubesModule::new();
        module.module.enabled = true;
        let data = module.get_module_data_mut();
        let mut set = |name: &str, value: f32| {
            if let Some(setting) = data.get_setting_mut(name) {
                setting.set_slider_value(value);
            }
        };
        set("Cubes", 999.0);
        set("Range", f32::NAN);
        set("Size", -5.0);
        let cfg = CubeConfig::from_data(data);
        assert_eq!(cfg.cubes, MAX_CUBES);
        assert!(
            (cfg.range - DEFAULT_RANGE as f64).abs() < 1e-6,
            "NaN falls back"
        );
        assert!((cfg.size - 0.1).abs() < 1e-6);
    }

     

     
    fn preview_view(w: usize, h: usize) -> View {
        view_looking_at(
            anchor(),
            V3 {
                x: anchor().x,
                y: anchor().y,
                z: anchor().z + 1.0,
            },
            w,
            h,
        )
    }

     
    fn view_looking_at(cam: V3, target: V3, w: usize, h: usize) -> View {
        let fwd = normalized(
            target.sub(cam),
            V3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        );
        let right = normalized(
            fwd.cross(V3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            }),
            V3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        );
        let up = right.cross(fwd);
        let tan_y = 35f64.to_radians().tan();
        View {
            cam,
            fwd,
            right,
            up,
            tan_x: tan_y * (w as f64 / h as f64),
            tan_y,
            w: w as f32,
            h: h as f32,
        }
    }

     
     
    fn screen_span(view: &View, corners: [V3; 8]) -> f32 {
        let points: Vec<Pos2> = corners
            .iter()
            .filter_map(|corner| view.project(*corner))
            .collect();
        assert!(
            !points.is_empty(),
            "no corner projected in front of the camera"
        );
        let min_x = points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let max_x = points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        let min_y = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_y = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
        (max_x - min_x).max(max_y - min_y)
    }

    fn plot(buf: &mut [u8], w: usize, h: usize, x: f32, y: f32, rgba: [u8; 4], radius: i32) {
        let alpha = rgba[3] as f32 / 255.0;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let px = x as i32 + dx;
                let py = y as i32 + dy;
                if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
                    continue;
                }
                let index = (py as usize * w + px as usize) * 4;
                for channel in 0..3 {
                    let old = buf[index + channel] as f32;
                    buf[index + channel] =
                        (old * (1.0 - alpha) + rgba[channel] as f32 * alpha) as u8;
                }
                buf[index + 3] = 255;
            }
        }
    }

    fn line(buf: &mut [u8], w: usize, h: usize, from: Pos2, to: Pos2, rgba: [u8; 4], width: i32) {
        let steps = (to.x - from.x).abs().max((to.y - from.y).abs()).max(1.0) as usize;
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            plot(
                buf,
                w,
                h,
                from.x + (to.x - from.x) * t,
                from.y + (to.y - from.y) * t,
                rgba,
                width,
            );
        }
    }

     
     
     
     
    fn render(cfg: &CubeConfig, time: f64, w: usize, h: usize, view: View) -> Vec<u8> {
         
         
        let mut buf = vec![0u8; w * h * 4];
        for y in 0..h {
            let t = y as f32 / (h.max(2) - 1) as f32;
            let sky = [
                (74.0 - 46.0 * t) as u8,
                (104.0 - 72.0 * t) as u8,
                (142.0 - 106.0 * t) as u8,
                255,
            ];
            for x in 0..w {
                let index = (y * w + x) * 4;
                buf[index..index + 4].copy_from_slice(&sky);
            }
        }
        let anchor = anchor();
        for index in 0..cfg.cubes {
            let cube = cube_frame(index, cfg, anchor, time);
            let alpha = cube_alpha(&cube, view.cam, cfg.range) * EDGE_ALPHA;
            if alpha <= 0.01 {
                continue;
            }
            let screen: [Option<Pos2>; 8] =
                std::array::from_fn(|corner| view.project(cube.corners[corner]));
            let distance = cube.center.sub(view.cam).length();
            let scale = stroke_scale(distance, cfg.range);
            let width = ((cfg.width * scale).max(0.5)).round() as i32;
            if cfg.faces {
                for (face, _) in visible_faces(view.cam, &cube) {
                    let quad = CUBE_FACES[face];
                    let points: Vec<Pos2> =
                        quad.iter().filter_map(|&corner| screen[corner]).collect();
                    let outward = face_centre(&cube.corners, &quad).sub(cube.center);
                    let shade = face_shade(outward);
                    let face = tint(cfg.color, (alpha * FACE_ALPHA * shade).clamp(0.0, 1.0));
                    polygon(
                        &mut buf,
                        w,
                        h,
                        &points,
                        [face.r(), face.g(), face.b(), face.a()],
                    );
                }
            }
            for &(a, b) in CUBE_EDGES.iter() {
                let (Some(from), Some(to)) = (screen[a], screen[b]) else {
                    continue;
                };
                let color = tint(cfg.color, alpha);
                let glow = tint(cfg.color, alpha * GLOW_ALPHA * scale);
                if cfg.glow {
                    line(
                        &mut buf,
                        w,
                        h,
                        from,
                        to,
                        [color.r(), color.g(), color.b(), glow.a()],
                        (width as f32 * GLOW_WIDTH).round() as i32,
                    );
                }
                line(
                    &mut buf,
                    w,
                    h,
                    from,
                    to,
                    [color.r(), color.g(), color.b(), color.a()],
                    width,
                );
            }
            for corner in screen.iter().flatten() {
                let node = tint(cfg.color, alpha * 0.9);
                plot(
                    &mut buf,
                    w,
                    h,
                    corner.x,
                    corner.y,
                    [node.r(), node.g(), node.b(), node.a()],
                    (width as f32 * 0.85).max(0.6) as i32,
                );
            }
        }
        buf
    }

     
     
    fn polygon(buf: &mut [u8], w: usize, h: usize, corners: &[Pos2], rgba: [u8; 4]) {
        if corners.len() < 3 || rgba[3] == 0 {
            return;
        }
        let alpha = rgba[3] as f32 / 255.0;
        let min_x = corners.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let max_x = corners
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = corners.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_y = corners
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max);
        let first = (min_x.floor().max(0.0)) as usize;
        let last = (max_x.ceil().min(w as f32 - 1.0)).max(0.0) as usize;
        let top = (min_y.floor().max(0.0)) as usize;
        let bottom = (max_y.ceil().min(h as f32 - 1.0)).max(0.0) as usize;
        for py in top..=bottom.min(h - 1) {
            for px in first..=last.min(w - 1) {
                let (x, y) = (px as f32 + 0.5, py as f32 + 0.5);
                 
                 
                let mut inside = false;
                for index in 0..corners.len() {
                    let a = corners[index];
                    let b = corners[(index + 1) % corners.len()];
                    if (a.y > y) != (b.y > y) {
                        let t = (y - a.y) / (b.y - a.y);
                        if x < a.x + (b.x - a.x) * t {
                            inside = !inside;
                        }
                    }
                }
                if !inside {
                    continue;
                }
                let at = (py * w + px) * 4;
                for channel in 0..3 {
                    let old = buf[at + channel] as f32;
                    buf[at + channel] = (old * (1.0 - alpha) + rgba[channel] as f32 * alpha) as u8;
                }
            }
        }
    }

     
     
    #[test]
    #[ignore = "writes a preview sheet into screenshots/"]
    fn preview_sheet() {
        let cfg = CubeConfig {
            enabled: true,
            cubes: 9,
            range: 10.0,
            size: 0.4,
            width: 2.0,
            ..CubeConfig::default()
        };
        let (w, h) = (640usize, 360usize);
        let times = [0.0, 3.0, 6.5, 10.0];
        let mut sheet = vec![0u8; w * 2 * h * 2 * 4];
        for (tile, time) in times.into_iter().enumerate() {
            let panel = render(&cfg, time, w, h, preview_view(w, h));
            let (ox, oy) = ((tile % 2) * w, (tile / 2) * h);
            for y in 0..h {
                let from = y * w * 4;
                let to = ((oy + y) * w * 2 + ox) * 4;
                sheet[to..to + w * 4].copy_from_slice(&panel[from..from + w * 4]);
            }
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("floating_cubes.png");
        image::save_buffer(
            &path,
            &sheet,
            (w * 2) as u32,
            (h * 2) as u32,
            image::ColorType::Rgba8,
        )
        .unwrap();
        eprintln!("wrote {}", path.display());
    }

     
     
     
    #[test]
    #[ignore = "writes a walk preview into screenshots/"]
    fn walk_preview() {
        let cfg = CubeConfig {
            enabled: true,
            cubes: 6,
            range: 10.0,
            size: 0.5,
            width: 2.0,
            ..CubeConfig::default()
        };
        let (w, h) = (640usize, 360usize);
        let eye = anchor();
        let look_at = V3 {
            x: eye.x,
            y: eye.y,
            z: eye.z + 1.0,
        };
        let mut sheet = vec![0u8; w * 3 * h * 4];
        for (tile, back) in [8.0, 5.0, 2.0].into_iter().enumerate() {
            let cam = V3 {
                x: eye.x,
                y: eye.y,
                z: eye.z - back,
            };
            let panel = render(&cfg, 5.0, w, h, view_looking_at(cam, look_at, w, h));
            let ox = tile * w;
            for y in 0..h {
                let from = y * w * 4;
                let to = (y * w * 3 + ox) * 4;
                sheet[to..to + w * 4].copy_from_slice(&panel[from..from + w * 4]);
            }
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("floating_cubes_walk.png");
        image::save_buffer(
            &path,
            &sheet,
            (w * 3) as u32,
            h as u32,
            image::ColorType::Rgba8,
        )
        .unwrap();
        eprintln!("wrote {}", path.display());
    }

    #[test]
    fn the_module_ships_with_sensible_defaults() {
        let module = FloatingCubesModule::new();
        assert_eq!(module.module.id, ModuleId::FloatingCubes);
        assert_eq!(module.module.category, ModuleCategory::Visuals);
        assert!(!module.module.enabled);
        let names: Vec<&str> = module
            .module
            .settings
            .iter()
            .map(ModuleSetting::name)
            .collect();
        assert_eq!(
            names,
            vec!["Color", "Cubes", "Range", "Size", "Spin", "Drift", "Width", "Faces", "Glow"]
        );
        let cfg = CubeConfig::from_data(&module.module);
        assert_eq!(cfg.cubes, DEFAULT_CUBES as usize);
        assert_eq!(cfg.range, DEFAULT_RANGE as f64);
        assert!(cfg.glow);
        assert!(cfg.faces, "the glass faces are on by default");
    }
}
