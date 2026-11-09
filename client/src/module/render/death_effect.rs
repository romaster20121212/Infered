 
 
 
 
 
 
 
 
 

use crate::graphic::esp::{color_setting, toggle_setting, View, V3};
use crate::graphic::theme;
use crate::mapping::entity::mob::Mob;
use crate::mapping::entity::player::Player;
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::MappedObject;
use crate::module::player::autototem::roll;
use crate::module::render::trail::bounded;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, minecraft};
use egui::{Color32, Painter, Pos2, Shape, Stroke};
use std::collections::{HashMap, HashSet};
use std::f64::consts::TAU;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const DEFAULT_DURATION: f32 = 1.2;
const DEFAULT_RANGE: f32 = 64.0;
const DEFAULT_COLOR: [f32; 4] = [1.0, 0.25, 0.2, 1.0];
 
const FALLBACK_WIDTH: f64 = 0.6;
const FALLBACK_HEIGHT: f64 = 1.8;

 
const SHOCK_STEPS: usize = 32;
 
const SHARD_COUNT: usize = 22;
 
const SHATTER_PIECES: usize = 18;
 
const WISP_COUNT: usize = 10;
const WISP_STEPS: usize = 16;
 
const EFFECT_CAP: usize = 64;

 
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

 
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum DeathStyle {
    #[default]
    Hologram,
    Explosion,
    Shatter,
    Ascend,
}

impl DeathStyle {
    const OPTIONS: [&'static str; 4] = ["Hologram", "Explosion", "Shatter", "Ascend"];

    fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Explosion,
            2 => Self::Shatter,
            3 => Self::Ascend,
            _ => Self::Hologram,
        }
    }
}

 
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DeathConfig {
    pub(crate) enabled: bool,
    style: DeathStyle,
    duration: Duration,
    range: f64,
    players: bool,
    mobs: bool,
    remove: bool,
    color: Color32,
}

impl DeathConfig {
    fn from_data(data: &ModuleData) -> Self {
        let style_index = match data.get_setting("Style") {
            Some(ModuleSetting::Choice { value, .. }) => *value,
            _ => 0,
        };
        Self {
            enabled: data.enabled,
            style: DeathStyle::from_index(style_index),
            duration: Duration::from_secs_f32(bounded(
                data,
                "Duration",
                DEFAULT_DURATION,
                0.3,
                4.0,
            )),
            range: bounded(data, "Range", DEFAULT_RANGE, 8.0, 128.0) as f64,
            players: toggle_setting(data, "Players", true),
            mobs: toggle_setting(data, "Mobs", true),
            remove: toggle_setting(data, "Remove Entity", true),
            color: color_setting(data, "Color", Color32::from_rgb(255, 64, 51)),
        }
    }
}

 
#[derive(Debug, Clone)]
struct DeathEffect {
     
    feet: V3,
    width: f64,
    height: f64,
    style: DeathStyle,
    color: Color32,
     
    seed: f32,
    spawned: Instant,
    duration: Duration,
     
    yaw: f32,
     
    skin: Option<String>,
     
    shards: Vec<Shard>,
     
     
    model: Vec<ModelShard>,
}

 
 
 
#[derive(Default)]
struct DeathState {
    effects: Vec<DeathEffect>,
    alive: HashSet<i32>,
}

fn state() -> &'static Mutex<DeathState> {
    static STATE: OnceLock<Mutex<DeathState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(DeathState::default()))
}

fn reset() {
    *state().lock().unwrap_or_else(|p| p.into_inner()) = DeathState::default();
}

 

 
fn ease_out(progress: f64) -> f64 {
    let remaining = 1.0 - progress.clamp(0.0, 1.0);
    1.0 - remaining * remaining * remaining
}

 
fn sphere_dir(index: usize, count: usize) -> V3 {
    let y = 1.0 - 2.0 * (index as f64 + 0.5) / count as f64;
    let ring = (1.0 - y * y).max(0.0).sqrt();
    let theta = index as f64 * 2.399_963_229_728_653;
    V3 {
        x: ring * theta.cos(),
        y,
        z: ring * theta.sin(),
    }
}

 
fn hash01(index: usize, seed: f32) -> f64 {
    let x = (index as f64 * 12.989_8 + seed as f64 * 78.233).sin() * 43_758.545_3;
    x - x.floor()
}

fn offset(origin: V3, dir: V3, distance: f64) -> V3 {
    V3 {
        x: origin.x + dir.x * distance,
        y: origin.y + dir.y * distance,
        z: origin.z + dir.z * distance,
    }
}

 
fn rotate_y(x: f64, z: f64, angle: f64) -> (f64, f64) {
    let (sin, cos) = angle.sin_cos();
    (x * cos - z * sin, x * sin + z * cos)
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

 
fn project_path(view: &View, points: &[V3]) -> Option<Vec<Pos2>> {
    points.iter().map(|p| view.project(*p)).collect()
}

 
fn screen_scale(view: &View, point: V3) -> Option<f32> {
    let depth = point.sub(view.cam).dot(view.fwd);
    if depth < 0.05 {
        return None;
    }
    Some((view.h as f64 * 0.5 / (view.tan_y * depth)) as f32)
}

 
 
fn box_corners(feet: V3, width: f64, height: f64, scale: f64, lift: f64) -> [V3; 8] {
    let center_y = feet.y + height * 0.5 + lift;
    let half_w = width * 0.5 * scale;
    let half_h = height * 0.5 * scale;
    let mut corners = [feet; 8];
    for (i, corner) in corners.iter_mut().enumerate() {
        let sx = if i & 1 == 0 { -1.0 } else { 1.0 };
        let sy = if i & 2 == 0 { -1.0 } else { 1.0 };
        let sz = if i & 4 == 0 { -1.0 } else { 1.0 };
        *corner = V3 {
            x: feet.x + sx * half_w,
            y: center_y + sy * half_h,
            z: feet.z + sz * half_w,
        };
    }
    corners
}

fn draw_box(painter: &Painter, view: &View, corners: &[V3; 8], stroke: Stroke) {
    for (a, b) in BOX_EDGES {
        if let (Some(pa), Some(pb)) = (view.project(corners[a]), view.project(corners[b])) {
            painter.add(Shape::line_segment([pa, pb], stroke));
        }
    }
}

 

 
 
fn draw_explosion(painter: &Painter, view: &View, effect: &DeathEffect, progress: f64) {
    let feet = effect.feet;
    let center = V3 {
        x: feet.x,
        y: feet.y + effect.height * 0.5,
        z: feet.z,
    };
    let grow = ease_out(progress);

     
    let max_radius = 2.0 + effect.width;
    for ring in 0..2 {
        let delay = ring as f64 * 0.12;
        let ring_progress = ((progress - delay) / (1.0 - delay)).clamp(0.0, 1.0);
        if ring_progress <= 0.0 {
            continue;
        }
        let radius = max_radius * ease_out(ring_progress) * (1.0 - ring as f64 * 0.35);
        let ground = V3 {
            x: center.x,
            y: feet.y + 0.05,
            z: center.z,
        };
        let points: Vec<V3> = (0..=SHOCK_STEPS)
            .map(|i| {
                let angle = TAU * i as f64 / SHOCK_STEPS as f64;
                V3 {
                    x: ground.x + radius * angle.cos(),
                    y: ground.y,
                    z: ground.z + radius * angle.sin(),
                }
            })
            .collect();
        if let Some(path) = project_path(view, &points) {
            let width = (3.0 * (1.0 - ring_progress) + 0.5) as f32;
            let fade = (1.0 - ring_progress) as f32;
            painter.add(Shape::line(
                path,
                Stroke::new(width, theme::with_alpha(effect.color, fade)),
            ));
        }
    }

     
    let fade = (1.0 - progress) as f32;
    let hot = mix(effect.color, Color32::WHITE, fade * 0.7);
    for i in 0..SHARD_COUNT {
        let dir = sphere_dir(i, SHARD_COUNT);
        let speed = 0.6 + 0.8 * hash01(i, effect.seed);
        let head_distance = 0.4 + 3.2 * grow * speed;
        let length = 1.2 * (1.0 - progress * 0.6);
        let tail_distance = (head_distance - length).max(0.0);
        let tail = offset(center, dir, tail_distance);
        let head = offset(center, dir, head_distance);
        if let (Some(a), Some(b)) = (view.project(tail), view.project(head)) {
            painter.add(Shape::line_segment(
                [a, b],
                Stroke::new(2.5 * fade + 0.5, theme::with_alpha(hot, fade)),
            ));
        }
    }

     
    if progress < 0.3 {
        let flash = progress / 0.3;
        if let (Some(center_px), Some(scale)) = (view.project(center), screen_scale(view, center)) {
            let radius = scale * effect.width as f32 * (0.5 + flash as f32);
            painter.add(Shape::circle_filled(
                center_px,
                radius,
                theme::with_alpha(Color32::WHITE, (1.0 - flash) as f32 * 0.8),
            ));
        }
    }
}

 
 
fn draw_shatter(painter: &Painter, view: &View, effect: &DeathEffect, progress: f64, secs: f64) {
    let fade = (1.0 - progress) as f32;
    let feet = effect.feet;
    let (width, height) = (effect.width, effect.height);

     
    let crack = 1.0 - (progress / 0.2).min(1.0);
    if crack > 0.0 {
        let corners = box_corners(feet, width, height, 1.0, 0.0);
        draw_box(
            painter,
            view,
            &corners,
            Stroke::new(2.0_f32, theme::with_alpha(effect.color, crack as f32)),
        );
    }

    let body_center = V3 {
        x: feet.x,
        y: feet.y + height * 0.5,
        z: feet.z,
    };
    let gravity = 4.0;
    for i in 0..SHATTER_PIECES {
        let origin = V3 {
            x: feet.x + (hash01(i, effect.seed) - 0.5) * width,
            y: feet.y + hash01(i + 100, effect.seed) * height,
            z: feet.z + (hash01(i + 200, effect.seed) - 0.5) * width,
        };
         
        let (dx, dy, dz) = (
            origin.x - body_center.x,
            origin.y - body_center.y,
            origin.z - body_center.z,
        );
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        let out = if length > 1e-6 {
            V3 {
                x: dx / length,
                y: dy / length,
                z: dz / length,
            }
        } else {
            sphere_dir(i, SHATTER_PIECES)
        };
        let speed = 1.5 + 2.5 * hash01(i + 300, effect.seed);
        let position = V3 {
            x: origin.x + out.x * speed * secs,
            y: origin.y + out.y * speed * secs + 1.2 * secs - 0.5 * gravity * secs * secs,
            z: origin.z + out.z * speed * secs,
        };
        let spin = (hash01(i + 400, effect.seed) * 2.0 - 1.0) * 10.0 * secs;
        let size = width * 0.2 * (0.6 + 0.8 * hash01(i + 600, effect.seed));
        let local = [
            (-size, -size * 0.5, 0.0),
            (size, 0.0, size * 0.3),
            (0.0, size * 0.9, -size * 0.4),
        ];
        let corners: Option<Vec<Pos2>> = local
            .iter()
            .map(|(lx, ly, lz)| {
                let (rx, rz) = rotate_y(*lx, *lz, spin);
                view.project(V3 {
                    x: position.x + rx,
                    y: position.y + ly,
                    z: position.z + rz,
                })
            })
            .collect();
        if let Some(polygon) = corners {
            let tint = mix(
                effect.color,
                Color32::WHITE,
                hash01(i + 500, effect.seed) as f32 * 0.4,
            );
            painter.add(Shape::convex_polygon(
                polygon,
                theme::with_alpha(tint, fade),
                Stroke::NONE,
            ));
        }
    }
}

 
 
fn draw_ascend(painter: &Painter, view: &View, effect: &DeathEffect, progress: f64) {
    let fade = (1.0 - progress) as f32;
    let feet = effect.feet;
    let (width, height) = (effect.width, effect.height);

    let shrink = 1.0 - 0.8 * ease_out(progress);
    let corners = box_corners(feet, width, height, shrink, progress);
    draw_box(
        painter,
        view,
        &corners,
        Stroke::new(1.5_f32, theme::with_alpha(effect.color, fade * 0.8)),
    );

    let wisp_color = mix(effect.color, Color32::WHITE, 0.5);
    for i in 0..WISP_COUNT {
        let base = TAU * i as f64 / WISP_COUNT as f64 + hash01(i, effect.seed) * TAU * 0.2;
        let points: Vec<V3> = (0..=WISP_STEPS)
            .map(|k| {
                let s = k as f64 / WISP_STEPS as f64;
                let radius = width * 0.7 * (1.0 - s * 0.6) * (1.0 - 0.3 * progress);
                let angle = base + s * 4.0 + progress * 3.0;
                V3 {
                    x: feet.x + radius * angle.cos(),
                    y: feet.y + s * height * 1.2 + progress * 2.6,
                    z: feet.z + radius * angle.sin(),
                }
            })
            .collect();
        if let Some(path) = project_path(view, &points) {
            painter.add(Shape::line(
                path,
                Stroke::new(2.0 * fade + 0.5, theme::with_alpha(wisp_color, fade)),
            ));
        }
    }

     
    let glow_point = V3 {
        x: feet.x,
        y: feet.y + height * 0.5 + progress * 2.6,
        z: feet.z,
    };
    if let (Some(center_px), Some(scale)) =
        (view.project(glow_point), screen_scale(view, glow_point))
    {
        painter.add(Shape::circle_filled(
            center_px,
            scale * width as f32 * 0.5 * fade,
            theme::with_alpha(wisp_color, fade * 0.35),
        ));
    }
}

 

 
const SKIN_PX: f64 = 64.0;
 
const MODEL_HEIGHT: f64 = 2.0;
 
const HOLOGRAM_FLOAT: f64 = 0.55;
 
const HOLOGRAM_SHATTER: f64 = 0.68;
 
const HOLOGRAM_MIN_LIFETIME: f32 = 2.8;
 
const SHARD_GRID: usize = 2;
 
const HOLOGRAM_CACHE_CAP: usize = 16;

static HOLOGRAM_PIXELS: OnceLock<Mutex<HashMap<String, (Vec<u8>, usize, usize)>>> = OnceLock::new();
static HOLOGRAM_TEXTURES: OnceLock<Mutex<HashMap<String, egui::TextureHandle>>> = OnceLock::new();
static HOLOGRAM_REQUESTS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
 
 
static HOLOGRAM_WORKERS: OnceLock<Mutex<Vec<std::thread::JoinHandle<()>>>> = OnceLock::new();

 
pub(crate) fn shutdown_hologram_loaders() {
    let Some(workers) = HOLOGRAM_WORKERS.get() else {
        return;
    };
    let handles = workers
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .drain(..)
        .collect::<Vec<_>>();
    for handle in handles {
        let _ = handle.join();
    }
}

 
 
 
fn hologram_texture(ctx: &egui::Context, url: &str) -> Option<egui::TextureId> {
    let textures = HOLOGRAM_TEXTURES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(texture) = textures.lock().ok()?.get(url) {
        return Some(texture.id());
    }

    let pixels = HOLOGRAM_PIXELS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some((rgba, w, h)) = pixels.lock().ok()?.remove(url) {
        let name = url.rsplit('/').next().unwrap_or("skin").to_owned();
        let texture = ctx.load_texture(
            format!("death_hologram_{name}"),
            egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba),
            egui::TextureOptions::NEAREST,
        );
        let id = texture.id();
        let mut textures = textures.lock().ok()?;
        if textures.len() >= HOLOGRAM_CACHE_CAP {
            textures.clear();
        }
        textures.insert(url.to_owned(), texture);
        return Some(id);
    }

    let requests = HOLOGRAM_REQUESTS.get_or_init(|| Mutex::new(HashSet::new()));
    let should_start = requests.lock().ok()?.insert(url.to_owned());
    if should_start {
        let worker_url = url.to_owned();
        let worker = std::thread::spawn(move || {
            match crate::graphic::player_card::load_skin_image(&worker_url) {
                Ok(image) => {
                    if let Ok(mut pixels) = HOLOGRAM_PIXELS
                        .get_or_init(|| Mutex::new(HashMap::new()))
                        .lock()
                    {
                        pixels.insert(worker_url.clone(), image);
                    }
                }
                Err(error) => {
                    log::debug!("death hologram: skin load failed for {worker_url}: {error:#}");
                }
            }
            if let Some(requests) = HOLOGRAM_REQUESTS.get() {
                if let Ok(mut requests) = requests.lock() {
                    requests.remove(&worker_url);
                }
            }
        });
        HOLOGRAM_WORKERS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(worker);
    }
    None
}

 
 
 
#[derive(Debug, Clone, Copy)]
struct SkinBox {
    pivot: V3,
    width: f64,
    height: f64,
    depth: f64,
    uv: (f64, f64),
     
    hangs: bool,
     
    arm: f64,
}

 
fn body_blueprint() -> [SkinBox; 6] {
    let at = |x: f64, y: f64, z: f64| V3 { x, y, z };
    [
         
        SkinBox {
            pivot: at(-0.125, 0.75, 0.0),
            width: 0.25,
            height: 0.75,
            depth: 0.25,
            uv: (16.0, 48.0),
            hangs: true,
            arm: 0.0,
        },
         
        SkinBox {
            pivot: at(0.125, 0.75, 0.0),
            width: 0.25,
            height: 0.75,
            depth: 0.25,
            uv: (0.0, 16.0),
            hangs: true,
            arm: 0.0,
        },
         
        SkinBox {
            pivot: at(0.0, 0.75, 0.0),
            width: 0.5,
            height: 0.75,
            depth: 0.25,
            uv: (16.0, 16.0),
            hangs: false,
            arm: 0.0,
        },
         
        SkinBox {
            pivot: at(-0.375, 1.5, 0.0),
            width: 0.25,
            height: 0.75,
            depth: 0.25,
            uv: (32.0, 48.0),
            hangs: true,
            arm: -1.0,
        },
         
        SkinBox {
            pivot: at(0.375, 1.5, 0.0),
            width: 0.25,
            height: 0.75,
            depth: 0.25,
            uv: (40.0, 16.0),
            hangs: true,
            arm: 1.0,
        },
         
        SkinBox {
            pivot: at(0.0, 1.5, 0.0),
            width: 0.5,
            height: 0.5,
            depth: 0.5,
            uv: (0.0, 0.0),
            hangs: false,
            arm: 0.0,
        },
    ]
}

 
#[derive(Debug, Clone, Copy)]
struct SkinFace {
    corners: [V3; 4],
    uvs: [(f32, f32); 4],
    normal: V3,
}

fn add(a: V3, b: V3) -> V3 {
    V3 {
        x: a.x + b.x,
        y: a.y + b.y,
        z: a.z + b.z,
    }
}

fn scale_vec(a: V3, s: f64) -> V3 {
    V3 {
        x: a.x * s,
        y: a.y * s,
        z: a.z * s,
    }
}

fn cross(a: V3, b: V3) -> V3 {
    V3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

fn normalize(a: V3) -> V3 {
    let len = a.length();
    if len <= 1e-9 {
        V3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    } else {
        scale_vec(a, 1.0 / len)
    }
}

 
 
fn rotate_about_z(p: V3, pivot: V3, angle: f64) -> V3 {
    let (sin, cos) = angle.sin_cos();
    let (dx, dy) = (p.x - pivot.x, p.y - pivot.y);
    V3 {
        x: pivot.x + dx * cos - dy * sin,
        y: pivot.y + dx * sin + dy * cos,
        z: p.z,
    }
}

fn rotate_dir_z(dir: V3, angle: f64) -> V3 {
    let (sin, cos) = angle.sin_cos();
    V3 {
        x: dir.x * cos - dir.y * sin,
        y: dir.x * sin + dir.y * cos,
        z: dir.z,
    }
}

 
fn rotate_axis(v: V3, axis: V3, angle: f64) -> V3 {
    let (sin, cos) = angle.sin_cos();
    add(
        add(scale_vec(v, cos), scale_vec(cross(axis, v), sin)),
        scale_vec(axis, axis.dot(v) * (1.0 - cos)),
    )
}

 
 
fn skin_face(corners: [V3; 4], uv_rect: (f64, f64, f64, f64), normal: V3) -> SkinFace {
    let (u0, v0, u1, v1) = uv_rect;
    let uv = |u: f64, v: f64| ((u / SKIN_PX) as f32, (v / SKIN_PX) as f32);
    SkinFace {
        corners,
        uvs: [uv(u0, v1), uv(u1, v1), uv(u1, v0), uv(u0, v0)],
        normal,
    }
}

 
fn skin_faces(part: &SkinBox, arm_angle: f64) -> [SkinFace; 6] {
    let (u, v) = part.uv;
    let (wp, hp, dp) = (part.width * 16.0, part.height * 16.0, part.depth * 16.0);
    let x0 = part.pivot.x - part.width * 0.5;
    let x1 = part.pivot.x + part.width * 0.5;
    let z0 = part.pivot.z - part.depth * 0.5;
    let z1 = part.pivot.z + part.depth * 0.5;
    let (y0, y1) = if part.hangs {
        (part.pivot.y - part.height, part.pivot.y)
    } else {
        (part.pivot.y, part.pivot.y + part.height)
    };
    let at = |x: f64, y: f64, z: f64| V3 { x, y, z };
    let mut faces = [
         
        skin_face(
            [
                at(x0, y0, z1),
                at(x1, y0, z1),
                at(x1, y1, z1),
                at(x0, y1, z1),
            ],
            (u + dp, v + dp, u + dp + wp, v + dp + hp),
            at(0.0, 0.0, 1.0),
        ),
         
        skin_face(
            [
                at(x1, y0, z0),
                at(x0, y0, z0),
                at(x0, y1, z0),
                at(x1, y1, z0),
            ],
            (
                u + 2.0 * dp + wp,
                v + dp,
                u + 2.0 * dp + 2.0 * wp,
                v + dp + hp,
            ),
            at(0.0, 0.0, -1.0),
        ),
         
        skin_face(
            [
                at(x1, y0, z1),
                at(x1, y0, z0),
                at(x1, y1, z0),
                at(x1, y1, z1),
            ],
            (u, v + dp, u + dp, v + dp + hp),
            at(1.0, 0.0, 0.0),
        ),
         
        skin_face(
            [
                at(x0, y0, z0),
                at(x0, y0, z1),
                at(x0, y1, z1),
                at(x0, y1, z0),
            ],
            (u + dp + wp, v + dp, u + 2.0 * dp + wp, v + dp + hp),
            at(-1.0, 0.0, 0.0),
        ),
         
        skin_face(
            [
                at(x0, y1, z1),
                at(x1, y1, z1),
                at(x1, y1, z0),
                at(x0, y1, z0),
            ],
            (u + dp, v, u + dp + wp, v + dp),
            at(0.0, 1.0, 0.0),
        ),
         
        skin_face(
            [
                at(x0, y0, z0),
                at(x1, y0, z0),
                at(x1, y0, z1),
                at(x0, y0, z1),
            ],
            (u + dp + wp, v, u + 2.0 * dp + wp, v + dp),
            at(0.0, -1.0, 0.0),
        ),
    ];

    let angle = arm_angle * part.arm;
    if angle != 0.0 {
        for face in &mut faces {
            for corner in &mut face.corners {
                *corner = rotate_about_z(*corner, part.pivot, angle);
            }
            face.normal = rotate_dir_z(face.normal, angle);
        }
    }
    faces
}

 
fn body_to_world(p: V3, feet: V3, yaw: f32, scale: f64, lift: f64) -> V3 {
    let (sin, cos) = (yaw as f64).to_radians().sin_cos();
    V3 {
        x: feet.x + (p.x * cos - p.z * sin) * scale,
        y: feet.y + p.y * scale + lift,
        z: feet.z + (p.x * sin + p.z * cos) * scale,
    }
}

fn world_normal(n: V3, yaw: f32) -> V3 {
    let (sin, cos) = (yaw as f64).to_radians().sin_cos();
    V3 {
        x: n.x * cos - n.z * sin,
        y: n.y,
        z: n.x * sin + n.z * cos,
    }
}

 
#[derive(Debug, Clone)]
struct Shard {
     
    corners: [V3; 4],
    uvs: [(f32, f32); 4],
    center: V3,
    axis: V3,
    spin: f32,
    drift: V3,
    tint: Color32,
}

 
 
 
const MODEL_TRI_CAP: usize = 1400;

 
 
const MODEL_FRAME_TRIANGLES: usize = 6000;

static MODEL_BUDGET: AtomicUsize = AtomicUsize::new(MODEL_FRAME_TRIANGLES);

 
 
fn take_model_budget(wanted: usize) -> usize {
    MODEL_BUDGET
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |left| {
            Some(left.saturating_sub(wanted))
        })
        .map_or(0, |left| left.min(wanted))
}

 
 
#[derive(Debug, Clone, Copy)]
struct ModelShard {
    corners: [V3; 3],
    center: V3,
    drift: V3,
}

 
 
 
 
 
 
fn captures_model(style: DeathStyle, is_player: bool) -> bool {
    style == DeathStyle::Hologram && !is_player
}

 
 
 
 
 
fn model_hologram_shards(vertices: &[f32], cam: V3, pivot: V3, seed: f32) -> Vec<ModelShard> {
    if vertices.len() < 9 {
        return Vec::new();
    }
    let triangles = vertices.len() / 9;
     
     
    let step = triangles.div_ceil(MODEL_TRI_CAP).max(1);
    let mut shards = Vec::with_capacity(triangles / step + 1);
    let mut index = 0usize;
    for triangle in vertices.chunks_exact(9).step_by(step) {
        let corner = |i: usize| V3 {
            x: cam.x + triangle[i] as f64,
            y: cam.y + triangle[i + 1] as f64,
            z: cam.z + triangle[i + 2] as f64,
        };
        let corners = [corner(0), corner(3), corner(6)];
        let center = scale_vec(add(add(corners[0], corners[1]), corners[2]), 1.0 / 3.0);
        let mut out = normalize(center.sub(pivot));
        if out.length() < 1e-6 {
            out = V3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            };
        }
        let spread = 0.3 + 0.5 * hash01(index, seed);
        let rise = 0.32 + 0.42 * hash01(index + 512, seed);
        shards.push(ModelShard {
            corners,
            center,
            drift: V3 {
                x: out.x * spread,
                 
                 
                 
                y: out.y * spread * 0.35 + rise,
                z: out.z * spread,
            },
        });
        index += 1;
    }
    shards
}

 
 
 
 
 
 
fn capture_model<E: EntityRef>(entity: &E, feet: V3, height: f64, seed: f32) -> Vec<ModelShard> {
     
    let vertices: Vec<f32> = {
        let _ = (entity, feet);
        Vec::new()
    };
    let pivot = V3 {
        x: feet.x,
        y: feet.y + height * 0.5,
        z: feet.z,
    };
    model_hologram_shards(&vertices, feet, pivot, seed)
}

 
fn hologram_shards(feet: V3, yaw: f32, scale: f64, seed: f32) -> Vec<Shard> {
    let mut shards = Vec::new();
    let body_center = V3 {
        x: feet.x,
        y: feet.y + scale + HOLOGRAM_FLOAT,
        z: feet.z,
    };
    let mut index = 0usize;
    let lerp = |a: V3, b: V3, t: f64| add(a, scale_vec(b.sub(a), t));
    for part in body_blueprint() {
         
        for face in skin_faces(&part, TAU / 4.0) {
            let (u0, v0) = (face.uvs[0], face.uvs[2]);
            for gy in 0..SHARD_GRID {
                for gx in 0..SHARD_GRID {
                    let s0 = gx as f64 / SHARD_GRID as f64;
                    let s1 = (gx + 1) as f64 / SHARD_GRID as f64;
                    let t0 = gy as f64 / SHARD_GRID as f64;
                    let t1 = (gy + 1) as f64 / SHARD_GRID as f64;
                    let edge_bottom = |s: f64| lerp(face.corners[0], face.corners[1], s);
                    let edge_top = |s: f64| lerp(face.corners[3], face.corners[2], s);
                    let body_corners = [
                        lerp(edge_bottom(s0), edge_top(s0), t0),
                        lerp(edge_bottom(s1), edge_top(s1), t0),
                        lerp(edge_bottom(s1), edge_top(s1), t1),
                        lerp(edge_bottom(s0), edge_top(s0), t1),
                    ];
                    let corners =
                        body_corners.map(|c| body_to_world(c, feet, yaw, scale, HOLOGRAM_FLOAT));
                    let center = scale_vec(
                        add(add(corners[0], corners[1]), add(corners[2], corners[3])),
                        0.25,
                    );
                    let uv = |s: f64, t: f64| {
                        (
                            u0.0 + (v0.0 - u0.0) * s as f32,
                            u0.1 + (v0.1 - u0.1) * t as f32,
                        )
                    };
                    let out = normalize(center.sub(body_center));
                    let spread = 0.16 + 0.22 * hash01(index, seed);
                    let torso = V3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    };
                    let axis = normalize(add(cross(out, torso), scale_vec(torso, 0.35)));
                    if axis.length() < 1e-6 {
                        continue;
                    }
                    shards.push(Shard {
                        corners,
                        uvs: [uv(s0, t0), uv(s1, t0), uv(s1, t1), uv(s0, t1)],
                        center,
                        axis: normalize(axis),
                        spin: (hash01(index + 900, seed) * 2.0 - 1.0) as f32 * 1.1,
                        drift: V3 {
                            x: out.x * spread,
                             
                             
                            y: (out.y * spread + 0.14 + 0.12 * hash01(index + 700, seed)).max(0.05),
                            z: out.z * spread,
                        },
                        tint: mix(
                            Color32::from_rgb(255, 64, 51),
                            Color32::WHITE,
                            hash01(index + 500, seed) as f32 * 0.35,
                        ),
                    });
                    index += 1;
                }
            }
        }
    }
    shards
}

 
 
fn push_quad(
    mesh: &mut egui::Mesh,
    view: &View,
    corners: &[V3; 4],
    uvs: &[(f32, f32); 4],
    color: Color32,
) -> bool {
    let Some(points) = corners
        .iter()
        .map(|corner| view.project(*corner))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    let base = mesh.vertices.len() as u32;
    for (point, uv) in points.iter().zip(uvs) {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: *point,
            uv: egui::pos2(uv.0, uv.1),
            color,
        });
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base, base + 2, base + 3);
    true
}

 
 
fn draw_hologram_body(
    painter: &Painter,
    view: &View,
    effect: &DeathEffect,
    progress: f64,
    arm_angle: f64,
    lift: f64,
    alpha: f32,
    texture: Option<egui::TextureId>,
) {
    let scale = effect.height / MODEL_HEIGHT;
    let glow = mix(effect.color, Color32::WHITE, 0.22);
    let mut faces: Vec<(f64, SkinFace)> = Vec::with_capacity(36);
    for part in body_blueprint() {
        for face in skin_faces(&part, arm_angle) {
            let world: [V3; 4] = face
                .corners
                .map(|corner| body_to_world(corner, effect.feet, effect.yaw, scale, lift));
            let center = scale_vec(add(add(world[0], world[1]), add(world[2], world[3])), 0.25);
            let normal = world_normal(face.normal, effect.yaw);
             
            if normal.dot(view.cam.sub(center)) <= 0.0 {
                continue;
            }
            let depth = center.sub(view.cam).dot(view.fwd);
            faces.push((
                depth,
                SkinFace {
                    corners: world,
                    ..face
                },
            ));
        }
    }
     
    faces.sort_by(|a, b| b.0.total_cmp(&a.0));

    let mut mesh = egui::Mesh::default();
    if let Some(id) = texture {
        mesh.texture_id = id;
    }
    for (_, face) in &faces {
        if let Some(points) = face
            .corners
            .iter()
            .map(|corner| view.project(*corner))
            .collect::<Option<Vec<_>>>()
        {
            painter.add(Shape::convex_polygon(
                points,
                theme::with_alpha(effect.color, alpha * 0.3),
                Stroke::NONE,
            ));
        }
        if texture.is_some() {
            push_quad(
                &mut mesh,
                view,
                &face.corners,
                &face.uvs,
                theme::with_alpha(glow, alpha * 0.78),
            );
        }
    }
    if texture.is_some() {
        painter.add(Shape::mesh(mesh));
    }

     
    let pulse = (0.75 + 0.25 * (progress * 40.0).sin()) as f32;
    for part in body_blueprint() {
        let faces = skin_faces(&part, arm_angle);
        for face in faces.iter().step_by(2) {
            let world: [V3; 4] = face
                .corners
                .map(|corner| body_to_world(corner, effect.feet, effect.yaw, scale, lift));
            let center = scale_vec(add(add(world[0], world[1]), add(world[2], world[3])), 0.25);
            if world_normal(face.normal, effect.yaw).dot(view.cam.sub(center)) <= 0.0 {
                continue;
            }
            if let Some(points) = world
                .iter()
                .map(|corner| view.project(*corner))
                .collect::<Option<Vec<_>>>()
            {
                painter.add(Shape::closed_line(
                    points,
                    Stroke::new(1.0_f32, theme::with_alpha(glow, alpha * 0.5 * pulse)),
                ));
            }
        }
    }
}

 
 
fn draw_hologram_shards(
    painter: &Painter,
    view: &View,
    effect: &DeathEffect,
    dt: f64,
    alpha: f32,
    texture: Option<egui::TextureId>,
) {
    let mut mesh = egui::Mesh::default();
    if let Some(id) = texture {
        mesh.texture_id = id;
    }
    let mut shards: Vec<(f64, [V3; 4], usize)> = Vec::with_capacity(effect.shards.len());
    for (index, shard) in effect.shards.iter().enumerate() {
        let angle = shard.spin as f64 * dt;
        let offset = scale_vec(shard.drift, dt);
        let center = add(shard.center, offset);
        let corners = shard.corners.map(|corner| {
            add(
                center,
                rotate_axis(corner.sub(shard.center), shard.axis, angle),
            )
        });
        let depth = center.sub(view.cam).dot(view.fwd);
        shards.push((depth, corners, index));
    }
    shards.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, corners, index) in &shards {
        let shard = &effect.shards[*index];
        if let Some(points) = corners
            .iter()
            .map(|corner| view.project(*corner))
            .collect::<Option<Vec<_>>>()
        {
            painter.add(Shape::convex_polygon(
                points,
                theme::with_alpha(shard.tint, alpha * 0.32),
                Stroke::NONE,
            ));
        }
        if texture.is_some() {
            push_quad(
                &mut mesh,
                view,
                corners,
                &shard.uvs,
                theme::with_alpha(shard.tint, alpha * 0.85),
            );
        }
    }
    if texture.is_some() {
        painter.add(Shape::mesh(mesh));
    }
}

 
 
fn draw_hologram(painter: &Painter, view: &View, effect: &DeathEffect, progress: f64) {
    let texture = effect
        .skin
        .as_deref()
        .and_then(|url| hologram_texture(painter.ctx(), url));

     
    let fade_in = (progress / 0.1).min(1.0) as f32;
    let flicker = 1.0 - 0.12 * ((progress * 97.0).sin() * 0.5 + 0.5) as f32;
    let lift = HOLOGRAM_FLOAT * smoothstep((progress / 0.62).clamp(0.0, 1.0));
    let arm_angle = (TAU / 4.0) * smoothstep(((progress - 0.08) / 0.5).clamp(0.0, 1.0));
    let glow = mix(effect.color, Color32::WHITE, 0.3);

     
     
    if !effect.model.is_empty() {
        hologram_ambience(painter, view, effect, progress, lift, glow);
        draw_model_hologram(painter, view, effect, progress, lift, glow);
        return;
    }

    hologram_ambience(painter, view, effect, progress, lift, glow);

    if progress < HOLOGRAM_SHATTER {
        let alpha = fade_in * flicker * (1.0 - progress as f32 * 0.1);
        draw_hologram_body(
            painter, view, effect, progress, arm_angle, lift, alpha, texture,
        );
        return;
    }

     
    let break_t = ((progress - HOLOGRAM_SHATTER) / (1.0 - HOLOGRAM_SHATTER)).clamp(0.0, 1.0);
    let dt = effect.duration.as_secs_f64() * (progress - HOLOGRAM_SHATTER);
    hologram_break_flash(painter, view, effect, break_t, lift, glow);
    let alpha = fade_in * flicker * (1.0 - break_t).powf(1.35) as f32;
    draw_hologram_shards(painter, view, effect, dt, alpha, texture);
}

 
 
fn hologram_ambience(
    painter: &Painter,
    view: &View,
    effect: &DeathEffect,
    progress: f64,
    lift: f64,
    glow: Color32,
) {
     
    let ring_alpha = (1.0 - progress) as f32 * 0.55;
    let ring_points: Vec<V3> = (0..=SHOCK_STEPS)
        .map(|i| {
            let angle = TAU * i as f64 / SHOCK_STEPS as f64;
            let radius = (effect.width * 0.75) * (1.0 - lift * 0.5);
            V3 {
                x: effect.feet.x + radius * angle.cos(),
                y: effect.feet.y + 0.04,
                z: effect.feet.z + radius * angle.sin(),
            }
        })
        .collect();
    if let Some(path) = project_path(view, &ring_points) {
        painter.add(Shape::closed_line(
            path,
            Stroke::new(1.5_f32, theme::with_alpha(effect.color, ring_alpha)),
        ));
    }

     
    for i in 0..14 {
        let start = hash01(i, effect.seed);
        let rise = (progress * 1.6 + start) % 1.0;
        let mote = V3 {
            x: effect.feet.x + (hash01(i + 40, effect.seed) - 0.5) * effect.width * 2.2,
            y: effect.feet.y + rise * effect.height * 1.5,
            z: effect.feet.z + (hash01(i + 80, effect.seed) - 0.5) * effect.width * 2.2,
        };
        if let Some(px) = view.project(mote) {
            let alpha = (1.0 - rise) as f32 * (1.0 - progress) as f32 * 0.6;
            painter.add(Shape::circle_filled(
                px,
                1.6,
                theme::with_alpha(glow, alpha),
            ));
        }
    }
}

 
 
fn hologram_break_flash(
    painter: &Painter,
    view: &View,
    effect: &DeathEffect,
    break_t: f64,
    lift: f64,
    glow: Color32,
) {
    if break_t >= 0.16 {
        return;
    }
    let flash = 1.0 - break_t / 0.16;
    let chest = V3 {
        x: effect.feet.x,
        y: effect.feet.y + effect.height * 0.6 + lift,
        z: effect.feet.z,
    };
    let ring: Vec<V3> = (0..=SHOCK_STEPS)
        .map(|i| {
            let angle = TAU * i as f64 / SHOCK_STEPS as f64;
            let radius = effect.width * (0.6 + 1.4 * break_t);
            V3 {
                x: chest.x + radius * angle.cos(),
                y: chest.y,
                z: chest.z + radius * angle.sin(),
            }
        })
        .collect();
    if let Some(path) = project_path(view, &ring) {
        painter.add(Shape::closed_line(
            path,
            Stroke::new(
                2.0_f32 * flash as f32 + 0.5,
                theme::with_alpha(glow, flash as f32 * 0.8),
            ),
        ));
    }
    if let (Some(center), Some(scale)) = (view.project(chest), screen_scale(view, chest)) {
        painter.add(Shape::circle_filled(
            center,
            scale * effect.width as f32 * (0.4 + break_t as f32),
            theme::with_alpha(Color32::WHITE, flash as f32 * 0.25),
        ));
    }
}

 
 
 
 
 
 
fn draw_model_hologram(
    painter: &Painter,
    view: &View,
    effect: &DeathEffect,
    progress: f64,
    lift: f64,
    glow: Color32,
) {
    let fade_in = (progress / 0.1).min(1.0) as f32;
    let flicker = 1.0 - 0.12 * ((progress * 97.0).sin() * 0.5 + 0.5) as f32;
    if progress < HOLOGRAM_SHATTER {
        let alpha = fade_in * flicker * (1.0 - progress as f32 * 0.1);
        draw_model_faces(painter, view, effect, lift, alpha, glow, None);
        return;
    }
    let break_t = ((progress - HOLOGRAM_SHATTER) / (1.0 - HOLOGRAM_SHATTER)).clamp(0.0, 1.0);
    let dt = effect.duration.as_secs_f64() * (progress - HOLOGRAM_SHATTER);
    hologram_break_flash(painter, view, effect, break_t, lift, glow);
    let alpha = fade_in * flicker * (1.0 - break_t).powf(1.35) as f32;
    draw_model_faces(painter, view, effect, lift, alpha, glow, Some(dt));
}

 
 
fn draw_model_faces(
    painter: &Painter,
    view: &View,
    effect: &DeathEffect,
    lift: f64,
    alpha: f32,
    glow: Color32,
    seconds: Option<f64>,
) {
    if alpha <= 0.0 {
        return;
    }
    let lift = V3 {
        x: 0.0,
        y: lift,
        z: 0.0,
    };
    let mut faces: Vec<(f64, [V3; 3], V3)> = Vec::with_capacity(effect.model.len());
    for shard in &effect.model {
        let offset = match seconds {
            Some(dt) => add(scale_vec(shard.drift, dt), lift),
            None => lift,
        };
        let corners = shard.corners.map(|corner| add(corner, offset));
        let center = add(shard.center, offset);
        let depth = center.sub(view.cam).dot(view.fwd);
        if depth < 0.05 {
            continue;
        }
         
         
        let normal = cross(corners[1].sub(corners[0]), corners[2].sub(corners[0]));
        if normal.dot(view.cam.sub(center)) <= 0.0 {
            continue;
        }
        faces.push((depth, corners, center));
    }
     
    faces.sort_by(|a, b| b.0.total_cmp(&a.0));

     
     
     
    if take_model_budget(faces.len()) < faces.len() {
        return;
    }

    let wire = faces.len() <= 400;
    for (index, (_, corners, center)) in faces.iter().enumerate() {
        let Some(points) = corners
            .iter()
            .map(|corner| view.project(*corner))
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
         
         
         
        let scan = 0.78 + 0.22 * ((center.y * 44.0).sin() * 0.5 + 0.5) as f32;
        painter.add(Shape::convex_polygon(
            points.clone(),
            theme::with_alpha(effect.color, alpha * 0.34 * scan),
            Stroke::NONE,
        ));
        if wire && index % 3 == 0 {
             
             
             
             
            painter.add(Shape::closed_line(
                points,
                Stroke::new(0.7_f32, theme::with_alpha(glow, alpha * 0.2)),
            ));
        }
    }
}

 
fn smoothstep(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

 

 
 
pub(crate) fn read_config() -> DeathConfig {
    let mut cfg = DeathConfig::default();
    if let Some(arc) = client().modules.get(ModuleId::DeathEffect) {
        if let Ok(module) = arc.lock() {
            cfg = DeathConfig::from_data(module.get_module_data());
        }
    }
    cfg
}

 
pub(crate) fn draw(painter: &Painter, view: &View, now: Instant, cfg: &DeathConfig) {
    let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
    if !cfg.enabled {
        state.effects.clear();
        return;
    }
     
    MODEL_BUDGET.store(MODEL_FRAME_TRIANGLES, Ordering::Relaxed);
    state
        .effects
        .retain(|effect| now.duration_since(effect.spawned) < effect.duration);
    for effect in &state.effects {
        let elapsed = now.duration_since(effect.spawned).as_secs_f64();
        let progress = (elapsed / effect.duration.as_secs_f64()).clamp(0.0, 1.0);
        match effect.style {
            DeathStyle::Hologram => draw_hologram(painter, view, effect, progress),
            DeathStyle::Explosion => draw_explosion(painter, view, effect, progress),
            DeathStyle::Shatter => draw_shatter(painter, view, effect, progress, elapsed),
            DeathStyle::Ascend => draw_ascend(painter, view, effect, progress),
        }
    }
}

 

 
#[derive(Debug)]
pub struct DeathEffectModule {
    pub module: ModuleData,
}

impl DeathEffectModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::DeathEffect,
                description:
                    "Removes dying players and mobs at once, then paints a red hologram of the victim — the player's own skin, or a mob's own model — that rises and shatters into drifting fragments, or an explosion, shatter, or ascend effect"
                        .to_string(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Choice {
                        name: "Style".to_string(),
                        value: 0,
                        options: DeathStyle::OPTIONS.iter().map(|s| s.to_string()).collect(),
                    },
                    ModuleSetting::Slider {
                        name: "Duration".to_string(),
                        value: DEFAULT_DURATION,
                        min: 0.3,
                        max: 4.0,
                    },
                    ModuleSetting::Slider {
                        name: "Range".to_string(),
                        value: DEFAULT_RANGE,
                        min: 8.0,
                        max: 128.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Players".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Mobs".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Remove Entity".to_string(),
                        value: true,
                    },
                    ModuleSetting::Color {
                        name: "Color".to_string(),
                        value: DEFAULT_COLOR,
                    },
                ],
            },
        }
    }
}

impl Module for DeathEffectModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        reset();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let cfg = DeathConfig::from_data(&self.module);
        if !cfg.enabled {
            reset();
            return Ok(());
        }
        let now = Instant::now();
        let (Some(world), Some(player)) = (minecraft().world()?, minecraft().player()?) else {
             
            reset();
            return Ok(());
        };
        let local_id = player.id()?;
        let player_pos: V3 = player.get_position()?.into();
        let range_sq = cfg.range * cfg.range;
        let previously_alive =
            std::mem::take(&mut state().lock().unwrap_or_else(|p| p.into_inner()).alive);

        let mut alive_now = HashSet::new();
        let mut deaths = Vec::new();
        for entity in world.get_entities()? {
            let Ok(id) = entity.id() else {
                continue;
            };
            if id == local_id {
                continue;
            }
            let wanted = (cfg.players && entity.instance_of::<Player>())
                || (cfg.mobs && entity.instance_of::<Mob>());
            if !wanted {
                continue;
            }
            let Some(living) = entity.as_living() else {
                continue;
            };
            if entity
                .distance_to_sqr(player_pos.x, player_pos.y, player_pos.z)
                .is_ok_and(|dist_sq| dist_sq > range_sq)
            {
                continue;
            }
            if living.is_alive().unwrap_or(true) {
                alive_now.insert(id);
                continue;
            }
             
            if !previously_alive.contains(&id) {
                continue;
            }
            let Ok(position) = entity.get_position() else {
                continue;
            };
            let width = entity.bb_width().unwrap_or(FALLBACK_WIDTH as f32) as f64;
            let height = entity.bb_height().unwrap_or(FALLBACK_HEIGHT as f32) as f64;
             
             
            let is_player = entity.instance_of::<Player>();
            let yaw = entity.get_yaw().unwrap_or(0.0);
            let skin = is_player
                .then(|| Player::new(entity.jni_ref().clone()))
                .and_then(|player| player.player_info().ok())
                .and_then(|info| info.skin_url().ok().flatten());
            let mut duration = cfg.duration;
            if cfg.style == DeathStyle::Hologram {
                duration = duration.max(Duration::from_secs_f32(HOLOGRAM_MIN_LIFETIME));
            }
            let feet: V3 = position.into();
            let seed = roll() as f32;
             
             
             
             
            let model = if captures_model(cfg.style, is_player) {
                capture_model(&entity, feet, height, seed)
            } else {
                Vec::new()
            };
            let shards = if cfg.style == DeathStyle::Hologram {
                hologram_shards(feet, yaw, height / MODEL_HEIGHT, seed)
            } else {
                Vec::new()
            };
            deaths.push(DeathEffect {
                feet,
                width,
                height,
                style: cfg.style,
                color: cfg.color,
                seed,
                spawned: now,
                duration,
                yaw,
                skin,
                shards,
                model,
            });
             
            if cfg.remove {
                if let Err(error) = entity.call_method("discard", &[]) {
                    log::debug!("Death effect: could not remove the entity: {error}");
                }
            }
        }

        let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
        state.alive = alive_now;
        for death in deaths {
            if state.effects.len() >= EFFECT_CAP {
                state.effects.remove(0);
            }
            state.effects.push(death);
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

    #[test]
    fn sphere_directions_are_unit_length() {
        for i in 0..SHARD_COUNT {
            let dir = sphere_dir(i, SHARD_COUNT);
            assert!((dir.length() - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn hash_stays_in_unit_range() {
        for i in 0..200 {
            let value = hash01(i, 0.37);
            assert!((0.0..1.0).contains(&value));
        }
    }

    #[test]
    fn ease_out_is_monotonic_and_finishes_at_one() {
        assert_eq!(ease_out(0.0), 0.0);
        assert!((ease_out(1.0) - 1.0).abs() < 1e-12);
        assert!(ease_out(0.25) < ease_out(0.5));
        assert!(ease_out(0.5) < ease_out(0.75));
    }

    #[test]
    fn style_index_maps_to_the_option_order() {
        assert_eq!(DeathStyle::from_index(0), DeathStyle::Hologram);
        assert_eq!(DeathStyle::from_index(1), DeathStyle::Explosion);
        assert_eq!(DeathStyle::from_index(2), DeathStyle::Shatter);
        assert_eq!(DeathStyle::from_index(3), DeathStyle::Ascend);
        assert_eq!(DeathStyle::from_index(9), DeathStyle::Hologram);
        assert_eq!(DeathStyle::default(), DeathStyle::Hologram);
    }

    #[test]
    fn every_skin_face_samples_inside_the_texture() {
        for part in body_blueprint() {
            for face in skin_faces(&part, 0.0) {
                for (u, v) in face.uvs {
                    assert!((0.0..=1.0).contains(&u), "u {u} escaped the skin");
                    assert!((0.0..=1.0).contains(&v), "v {v} escaped the skin");
                }
            }
        }
    }

    #[test]
    fn raising_the_arms_swings_them_out_to_a_t_pose() {
        let right_arm = body_blueprint()[4];
        let rest = skin_faces(&right_arm, 0.0);
        let posed = skin_faces(&right_arm, TAU / 4.0);
        let reach = |faces: &[SkinFace]| {
            faces
                .iter()
                .flat_map(|face| face.corners)
                .map(|corner| corner.x)
                .fold(f64::MIN, f64::max)
        };
         
        assert!(reach(&rest) < 0.51);
        assert!(reach(&posed) > 1.0);
    }

    #[test]
    fn the_body_breaks_into_fragments() {
        let feet = V3 {
            x: 4.0,
            y: 70.0,
            z: -2.0,
        };
        let shards = hologram_shards(feet, 30.0, 0.9, 0.5);
        assert_eq!(shards.len(), 6 * 6 * SHARD_GRID * SHARD_GRID);
        for shard in &shards {
            for (u, v) in shard.uvs {
                assert!((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v));
            }
            assert!((shard.drift.y) > 0.0, "shards must drift upward");
        }
    }

    fn origin() -> V3 {
        V3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

     
     
    fn quad_vertices() -> Vec<f32> {
        vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0,
            0.0,
        ]
    }

    #[test]
    fn the_model_hologram_is_the_victims_own_mesh() {
        let cam = V3 {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        };
        let pivot = V3 {
            x: 10.0,
            y: 64.9,
            z: -3.0,
        };
        let shards = model_hologram_shards(&quad_vertices(), cam, pivot, 0.25);
        assert_eq!(shards.len(), 2);
         
        assert!((shards[0].corners[0].x - cam.x).abs() < 1e-6);
        assert!((shards[0].corners[0].y - cam.y).abs() < 1e-6);
        assert!((shards[0].corners[2].y - (cam.y + 1.0)).abs() < 1e-6);
        let mut spread_out = false;
        for shard in &shards {
            assert!(shard.drift.y > 0.0, "fragments must climb as they drift");
             
            let out = shard.center.sub(pivot);
            let sideways = out.x * shard.drift.x + out.z * shard.drift.z;
            assert!(sideways >= 0.0, "fragments must not drift inward");
            spread_out |= sideways > 0.0;
        }
        assert!(spread_out, "the break has to open out, not just rise");
    }

    #[test]
    fn a_dense_model_is_thinned_but_never_dropped() {
        let dense = vec![0.0_f32; 9 * 5000];
        let shards = model_hologram_shards(&dense, origin(), origin(), 0.5);
        assert!(!shards.is_empty());
        assert!(shards.len() <= MODEL_TRI_CAP);
        assert!(shards.len() * 4 >= MODEL_TRI_CAP);
    }

    #[test]
    fn an_empty_model_falls_back_to_the_skin_hologram() {
        assert!(model_hologram_shards(&[], origin(), origin(), 0.5).is_empty());
        assert!(model_hologram_shards(&[1.0, 2.0, 3.0], origin(), origin(), 0.5).is_empty());
    }

     
     
    #[test]
    fn only_a_non_player_hologram_uses_the_captured_model() {
        assert!(captures_model(DeathStyle::Hologram, false));
        assert!(!captures_model(DeathStyle::Hologram, true));
        for style in [
            DeathStyle::Explosion,
            DeathStyle::Shatter,
            DeathStyle::Ascend,
        ] {
            assert!(!captures_model(style, false));
            assert!(!captures_model(style, true));
        }
    }

    #[test]
    fn smoothstep_eases_at_both_ends() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert!((smoothstep(1.0) - 1.0).abs() < 1e-12);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-12);
        assert!(smoothstep(0.1) < 0.1);
        assert!(smoothstep(0.9) > 0.9);
    }
}
