 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use super::motion_blur::{compile, uniform};
use super::smoke::{inverse, matrix, GlGuard};
use crate::gl;
use crate::mapping::chunk::{BitStorage, Palette};
use crate::mapping::client::world::LevelChunk;
use crate::mapping::math::Vec3;
use crate::mapping::{MappedObject, MinecraftClassType};
use crate::module::{ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, mapping, minecraft};
use anyhow::{ensure, Context};
use jni::objects::{GlobalRef, JObject};
use jni::JNIEnv;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const VERTEX: &str = r#"#version 150
in vec3 Position;
in vec3 Normal;
uniform mat4 View;
uniform mat4 Projection;
uniform vec3 Center;
out vec3 vNormal;
out vec3 vViewPosition;
out vec3 vWorld;
void main() {
    // The view matrix has the camera at the origin, so world positions are
    // made camera-relative first — the same convention the smoke pass uses.
    vec4 eye = View * vec4(Position - Center, 1.0);
    vViewPosition = eye.xyz;
    vNormal = mat3(View) * Normal;
    vWorld = Position;
    gl_Position = Projection * eye;
}
"#;

const FRAGMENT: &str = r#"#version 150
uniform sampler2D Scene;
uniform sampler2D SceneDepth;
uniform mat4 View;
uniform mat4 Projection;
uniform mat4 InverseProjection;
uniform vec4 Viewport;
uniform float Strength;
uniform float Blur;
uniform float Time;
uniform int Shimmer;
in vec3 vNormal;
in vec3 vViewPosition;
in vec3 vWorld;
out vec4 fragColor;

// More steps than the old build and a shorter stride near the surface: the
// near field is what fills most mirrored pixels, and a fine first stride is
// what keeps the hit point on the geometry it belongs to.
const int STEPS = 48;
const int REFINE = 5;
const float REACH = 128.0;
// How much of the frame a face shows straight on. The surface has no colour of
// its own, so this is the share that comes from the reflection while the rest
// is the block's own texture underneath; at a glancing angle it climbs to a
// full mirror.
const float SURFACE_REFLECTANCE = 0.62;

float hash(vec2 p) {
    p = fract(p * vec2(0.1031, 0.1030));
    p += dot(p, p.yx + 33.33);
    return fract((p.x + p.y) * p.x);
}

// Smooth value noise. Only ever sampled in world space at block scale, so it
// is a material texture rather than the per-pixel grain of a noise ray.
float grain(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x),
               mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

// Window pixel -> texture coordinate inside the copied viewport.
vec2 windowUv(vec2 window) {
    return clamp((window - Viewport.xy) / Viewport.zw, vec2(0.0), vec2(1.0));
}

bool onScreen(vec2 window) {
    return window.x >= Viewport.x && window.y >= Viewport.y
        && window.x <= Viewport.x + Viewport.z && window.y <= Viewport.y + Viewport.w;
}

// View-space position rebuilt from a copied depth texel with the game's own
// inverse projection. This is what makes the trace a real ray against real
// geometry instead of a screen-space offset.
vec3 viewPoint(vec2 uv, float depth) {
    vec4 clip = vec4(uv * 2.0 - 1.0, depth * 2.0 - 1.0, 1.0);
    vec4 eye = InverseProjection * clip;
    return eye.xyz / eye.w;
}

bool project(vec3 point, out vec2 window) {
    vec4 clip = Projection * vec4(point, 1.0);
    if (clip.w <= 0.0) {
        return false;
    }
    window = (clip.xy / clip.w * 0.5 + 0.5) * Viewport.zw + Viewport.xy;
    return onScreen(window);
}

// Marches the reflected ray through the copied depth buffer. On a crossing the
// hit is refined with a bisection, so a reflected edge stays an edge instead of
// wobbling by a whole stride from pixel to pixel.
bool trace(vec3 origin, vec3 direction, float skyUp, out vec2 hitUv, out float distance) {
    float stride = 0.25;
    float travelled = 0.0;
    float previous = 0.0;
    for (int i = 0; i < STEPS; ++i) {
        travelled += stride;
        stride = min(stride * 1.12, 6.0);
        if (travelled > REACH) {
            return false;
        }
        vec3 point = origin + direction * travelled;
        if (point.z > -0.08) {
            return false;
        }
        vec2 window;
        if (!project(point, window)) {
            return false;
        }
        vec2 uv = windowUv(window);
        float depth = texture(SceneDepth, uv).r;
        if (depth >= 0.99999) {
            // Open sky. A ray pointing up really does end on the sky, so take
            // the sky's own colour here instead of falling back to the border.
            // The margin keeps a horizon-hugging ray from cutting a distant
            // object short at the first sky texel it crosses.
            if (skyUp > 0.08) {
                hitUv = uv;
                distance = travelled;
                return true;
            }
            previous = travelled;
            continue;
        }
        float surface = -viewPoint(uv, depth).z;
        if (-point.z <= surface) {
            previous = travelled;
            continue;
        }
        float tNear = previous;
        float tFar = travelled;
        for (int r = 0; r < REFINE; ++r) {
            float middle = 0.5 * (tNear + tFar);
            vec3 probe = origin + direction * middle;
            vec2 probeWindow;
            if (!project(probe, probeWindow)) {
                tFar = middle;
                continue;
            }
            vec2 probeUv = windowUv(probeWindow);
            float probeDepth = texture(SceneDepth, probeUv).r;
            if (probeDepth >= 0.99999) {
                tNear = middle;
                continue;
            }
            if (-probe.z > -viewPoint(probeUv, probeDepth).z) {
                tFar = middle;
            } else {
                tNear = middle;
            }
        }
        vec3 hit = origin + direction * tFar;
        vec2 hitWindow;
        if (!project(hit, hitWindow)) {
            return false;
        }
        vec2 hitTexel = windowUv(hitWindow);
        float hitDepth = texture(SceneDepth, hitTexel).r;
        if (hitDepth < 0.99999) {
            float gap = -hit.z - (-viewPoint(hitTexel, hitDepth).z);
            // A ray that ends far behind the first surface it crosses grazed a
            // silhouette; that is a miss, not a reflection of empty space.
            if (gap > 0.6 + 0.02 * tFar) {
                return false;
            }
        }
        hitUv = hitTexel;
        distance = tFar;
        return true;
    }
    return false;
}

// What a mirror shows where the ray has no pixels of its own: the frame's own
// colour in that direction, clamped to the border and blurred. A reflection
// carries the picture's colours, so this is a sample of the frame and never a
// tint invented for the surface.
vec3 offscreen(vec3 direction, float lod) {
    vec4 clip = Projection * vec4(direction, 0.0);
    vec2 point = clip.xy / max(abs(clip.w), 0.0001);
    if (clip.w < 0.0) {
        // Behind the camera: the clip projection flips both axes, so undo it
        // before clamping to the frame.
        point = -point;
    }
    vec2 uv = clamp(point * 0.5 + 0.5, vec2(0.0), vec2(1.0));
    return textureLod(Scene, uv, clamp(lod, 0.0, 7.0)).rgb;
}

// Gloss is a mip chain, not a random sample: the fixed rotated cross below is
// the same for every pixel and every frame, so the blur is smooth.
vec3 sampleScene(vec2 uv, float lod, float spread) {
    vec2 texel = 1.0 / Viewport.zw;
    vec3 color = textureLod(Scene, uv, lod).rgb;
    if (spread < 0.75) {
        return color;
    }
    color *= 0.36;
    float total = 0.36;
    for (int i = 0; i < 4; ++i) {
        float angle = 1.5707963 * float(i) + 0.7853982;
        vec2 offset = vec2(cos(angle), sin(angle)) * spread * texel;
        color += textureLod(Scene, clamp(uv + offset, texel, 1.0 - texel), lod).rgb * 0.16;
        total += 0.16;
    }
    return color / total;
}

void main() {
    vec3 n = normalize(vNormal);
    vec3 v = normalize(-vViewPosition);
    float facing = clamp(dot(n, v), 0.0, 1.0);
    // A real surface shows more of the world at a glancing angle and more of
    // its own material straight on. There is no body colour here: what is not
    // reflected is the block's own texture showing through.
    float reflectivity = mix(SURFACE_REFLECTANCE, 1.0, pow(1.0 - facing, 2.4));

    float roughness = clamp(Blur, 0.0, 1.0);
    roughness *= roughness;

    vec3 incident = normalize(vViewPosition);
    vec3 direction = reflect(incident, n);
    vec3 worldDirection = transpose(mat3(View)) * direction;

    // The mirror starts as the frame's own colour in the reflected direction,
    // which is what the surface shows wherever the traced ray has no pixels.
    // `Blur` is the only thing that softens it, and it never shifts its hue.
    vec3 reflection = offscreen(worldDirection, roughness * 5.0 + 1.5);

    if (Strength > 0.02) {
        vec3 origin = vViewPosition + n * 0.02;
        vec2 hitUv;
        float distance;
        if (trace(origin, direction, worldDirection.y, hitUv, distance)) {
            // The cone a glossy reflection covers grows with roughness and with
            // the length of the ray; a trilinear mip is the smooth, stable way
            // to sample it.
            float lod = clamp(roughness * 3.5 + log2(1.0 + distance * 0.02), 0.0, 7.0);
            float spread = roughness * 6.0;
            // A slow, world-space ripple bends the mirror a hair: water, not
            // plate glass. Continuous across the surface, never per-pixel.
            vec2 ripple = vec2(grain(vWorld.xz * 0.9), grain(vWorld.zx * 0.9 + 4.7)) - 0.5;
            vec2 uv = clamp(hitUv + ripple * (0.0012 + roughness * 0.0035),
                            vec2(0.0015), vec2(0.9985));
            vec3 scene = sampleScene(uv, lod, spread);
            // A longer ray crosses more air, so the surface loses a little of
            // it — the picture stays the picture, just dimmer.
            scene *= mix(1.0, 0.85, clamp(distance / REACH, 0.0, 1.0));
            // Near the frame border the trace runs out of picture; blend to the
            // direction-sampled colour so the edge never snaps.
            float edge = smoothstep(0.0, 0.09, uv.x) * smoothstep(0.0, 0.09, 1.0 - uv.x)
                       * smoothstep(0.0, 0.09, uv.y) * smoothstep(0.0, 0.09, 1.0 - uv.y);
            reflection = mix(offscreen(worldDirection, lod + 1.5), scene, edge);
        }
    }

    if (Shimmer == 1) {
        // A slow, world-space highlight travelling across the surface. It is
        // neutral light: it brightens the mirror, it never colours it.
        float band = sin(Time * 0.65 - dot(vWorld.xz, vec2(0.24, 0.17)) + vWorld.y * 0.35);
        reflection *= 1.0 + 0.10 * smoothstep(0.82, 1.0, band);
    }

    float alpha = clamp(Strength * reflectivity, 0.0, 1.0);
    // Premultiplied: the pass blends with (ONE, ONE_MINUS_SRC_ALPHA).
    fragColor = vec4(reflection * alpha, alpha);
}
"#;

 
 
 
const MAX_SECTIONS_PER_FRAME: usize = 2048;
 
const SWEEP_BUDGET: Duration = Duration::from_micros(1_500);
 
const BUILD_BUDGET: Duration = Duration::from_micros(1_500);
 
 
 
const SWEEP_INTERVAL: Duration = Duration::from_millis(100);
 
 
const TRUNCATED_HOLD: Duration = Duration::from_secs(2);
 
const RADIUS_CACHE: Duration = Duration::from_millis(500);
 
 
const MAX_BLOCKS: usize = 131_072;
 
 
 
const MAX_VERTICES: usize = 786_432;
 
const MAX_RADIUS: f32 = 512.0;
 
const MIN_RADIUS: f32 = 8.0;
 
 
 
 
const MIN_SQUARE_RADIUS: f32 = 256.0;
 
const STRIDE: usize = 6;
 
const FACE_VERTICES: usize = 6;

 
 
#[derive(Clone, Debug, PartialEq)]
struct Config {
    blocks: Vec<String>,
     
    all_blocks: bool,
    follow_render_distance: bool,
    radius: f32,
    strength: f32,
    blur: f32,
    shimmer: bool,
}

impl Config {
    fn read(data: &ModuleData) -> Self {
        let slider = |name: &str, fallback: f32, min: f32, max: f32| {
            finite(
                data.get_setting(name)
                    .and_then(ModuleSetting::get_slider_value)
                    .unwrap_or(fallback),
                fallback,
                min,
                max,
            )
        };
        let blocks = match data.get_setting("Blocks") {
            Some(ModuleSetting::BlockList { value, .. }) => value.clone(),
            _ => Vec::new(),
        };
        Self {
            blocks,
            all_blocks: data
                .get_setting("All Blocks")
                .and_then(ModuleSetting::get_toggle_value)
                .unwrap_or(true),
            follow_render_distance: data
                .get_setting("Render Distance")
                .and_then(ModuleSetting::get_toggle_value)
                .unwrap_or(true),
            radius: slider("Radius", 24.0, MIN_RADIUS, MAX_RADIUS),
            strength: slider("Strength", 1.0, 0.0, 1.0),
            blur: slider("Blur", 0.35, 0.0, 1.0),
            shimmer: data
                .get_setting("Shimmer")
                .and_then(ModuleSetting::get_toggle_value)
                .unwrap_or(true),
        }
    }
}

fn finite(value: f32, fallback: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

 
 
fn radius_for_render_distance(chunks: i32, server_chunks: i32) -> f32 {
    let chunks = chunks.clamp(2, 32);
    let chunks = if server_chunks > 0 {
        chunks.min(server_chunks)
    } else {
        chunks
    };
    (chunks.max(1) * 16) as f32
}

 
fn render_distance_radius() -> anyhow::Result<f32> {
    let options = minecraft().options()?;
    let chunks = options.render_distance()?.get_int()?;
    let server = options.server_render_distance().unwrap_or(0);
    Ok(radius_for_render_distance(chunks, server))
}

 
 
 
fn effective_radius(cfg: &Config, cache: &mut Option<(f32, Instant)>) -> f32 {
    if !cfg.follow_render_distance {
        return cfg.radius;
    }
    if let Some((value, at)) = *cache {
        if at.elapsed() < RADIUS_CACHE {
            return value;
        }
    }
     
     
     
    let value = render_distance_radius()
        .unwrap_or(cfg.radius)
        .max(MIN_SQUARE_RADIUS);
    *cache = Some((value, Instant::now()));
    value
}

 

 
const FACE_NORMALS: [[f32; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];

 
const FACE_CORNERS: [[[f32; 3]; 4]; 6] = [
    [
        [1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [1.0, 1.0, 1.0],
    ],
    [
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [0.0, 1.0, 0.0],
    ],
    [
        [0.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ],
    [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ],
    [
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
    ],
    [
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ],
];

 

 
 
type ShapeBox = [f32; 6];
 
type ShapeId = u32;
 
const FULL_CUBE: ShapeId = 0;
 
const FULL_CUBE_BOXES: [ShapeBox; 1] = [[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]];
 
 
 
const SHAPE_UNITS: f64 = 64.0;

 
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Block {
    pos: [i32; 3],
    shape: ShapeId,
}

 
 
 
#[derive(Default)]
struct ShapeTable {
     
    boxes: Vec<Vec<ShapeBox>>,
     
    index: HashMap<Vec<[u8; 6]>, ShapeId>,
}

impl ShapeTable {
    fn new() -> Self {
        let mut table = Self::default();
        table.reset();
        table
    }

     
     
     
    fn reset(&mut self) {
        self.boxes.clear();
        self.boxes.push(FULL_CUBE_BOXES.to_vec());
        self.index.clear();
        self.index
            .insert(quantize(&[[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]), FULL_CUBE);
    }

     
     
    fn snapshot(&self) -> Vec<Vec<ShapeBox>> {
        self.boxes.clone()
    }

     
    fn insert(&mut self, boxes: &[[f64; 6]]) -> ShapeId {
        let key = quantize(boxes);
        if key.is_empty() {
            return FULL_CUBE;
        }
        if self.boxes.is_empty() {
             
             
            self.reset();
        }
        if let Some(id) = self.index.get(&key) {
            return *id;
        }
        let id = self.boxes.len() as ShapeId;
        self.boxes.push(
            key.iter()
                .map(|units| units.map(|unit| unit as f32 / SHAPE_UNITS as f32))
                .collect(),
        );
        self.index.insert(key, id);
        id
    }
}

 
 
 
 
fn quantize(boxes: &[[f64; 6]]) -> Vec<[u8; 6]> {
    boxes
        .iter()
        .filter(|bounds| bounds.iter().all(|value| value.is_finite()))
        .map(|bounds| {
            let mut units = [0u8; 6];
            for (index, value) in bounds.iter().enumerate() {
                units[index] = (*value * SHAPE_UNITS).round().clamp(0.0, SHAPE_UNITS) as u8;
            }
            units
        })
        .filter(|units| (0..3).all(|axis| units[axis] < units[axis + 3]))
        .collect()
}

 
fn neighbour(block: [i32; 3], face: usize) -> [i32; 3] {
    let n = FACE_NORMALS[face];
    [
        block[0] + n[0] as i32,
        block[1] + n[1] as i32,
        block[2] + n[2] as i32,
    ]
}

 
 
 
 
 
fn push_block(
    out: &mut Vec<f32>,
    block: Block,
    shapes: &[Vec<ShapeBox>],
    occupied: &HashSet<[i32; 3]>,
    max_vertices: usize,
) {
    let boxes = match shapes.get(block.shape as usize) {
        Some(boxes) if !boxes.is_empty() => boxes.as_slice(),
        _ => FULL_CUBE_BOXES.as_slice(),
    };
    for face in 0..6 {
        if occupied.contains(&neighbour(block.pos, face)) {
            continue;
        }
        for bounds in boxes {
             
             
            if (out.len() / STRIDE).saturating_add(FACE_VERTICES) > max_vertices {
                return;
            }
            push_face(out, block.pos, bounds, face);
        }
    }
}

 
 
 
fn push_face(out: &mut Vec<f32>, origin: [i32; 3], bounds: &ShapeBox, face: usize) {
    let corners = FACE_CORNERS[face];
    let normal = FACE_NORMALS[face];
    let axis = |flag: f32, axis: usize| {
        let index = if flag > 0.5 { axis + 3 } else { axis };
        origin[axis] as f32 + bounds[index]
    };
    for index in [0usize, 1, 2, 0, 2, 3] {
        let corner = corners[index];
        out.extend_from_slice(&[
            axis(corner[0], 0),
            axis(corner[1], 1),
            axis(corner[2], 2),
            normal[0],
            normal[1],
            normal[2],
        ]);
    }
}

 

 
 
 
 
fn sweep_sections(
    center: [f64; 3],
    radius: f32,
    min_y: i32,
    sections: i32,
) -> Vec<(i32, i32, i32)> {
    if sections <= 0 {
        return Vec::new();
    }
    let radius = finite(radius, MIN_SQUARE_RADIUS, 1.0, MAX_RADIUS) as f64;
    let reach = (radius / 16.0).ceil() as i32;
    let centre_chunk = [
        (center[0] / 16.0).floor() as i32,
        (center[2] / 16.0).floor() as i32,
    ];
    let lowest = min_y as f64;

    let mut columns: Vec<(i32, i32, i32)> = Vec::new();
    for dz in -reach..=reach {
        for dx in -reach..=reach {
            columns.push((
                centre_chunk[0] + dx,
                centre_chunk[1] + dz,
                dx * dx + dz * dz,
            ));
        }
    }
    columns.sort_by_key(|column| column.2);

    let mut queue = Vec::with_capacity(columns.len() * sections as usize);
     
     
     
     
     
     
     
    let eye = (((center[1] - lowest) / 16.0).floor() as i32).clamp(0, sections - 1);
    for &(cx, cz, _) in &columns {
        queue.push((cx, cz, eye));
    }
    for step in 1..sections {
        let below = eye - step;
        let above = eye + step;
        if below < 0 && above > sections - 1 {
            break;
        }
        for &(cx, cz, _) in &columns {
            if below >= 0 {
                queue.push((cx, cz, below));
            }
            if above < sections {
                queue.push((cx, cz, above));
            }
        }
    }
    queue
}

 
 
#[derive(Default)]
struct Scan {
    queue: Vec<(i32, i32, i32)>,
    cursor: usize,
    fresh: Vec<Block>,
    blocks: Vec<Block>,
     
    published: Vec<Vec<ShapeBox>>,
     
    shapes: ShapeTable,
    started: Option<Instant>,
     
     
     
     
    finished: Option<Instant>,
     
    last_duration: Duration,
     
    truncated: bool,
     
     
    sections: i32,
     
     
    generation: u64,
}

 
struct Section {
    chunk_x: i32,
    chunk_z: i32,
    index: i32,
    base_y: i32,
}

impl Scan {
    fn clear(&mut self) {
        self.queue.clear();
        self.cursor = 0;
        self.fresh.clear();
        self.blocks.clear();
        self.published.clear();
        self.shapes = ShapeTable::new();
        self.started = None;
        self.finished = None;
        self.last_duration = Duration::ZERO;
        self.truncated = false;
        self.generation = self.generation.wrapping_add(1);
    }

     
     
     
     
     
     
    fn hold(&self) -> Duration {
        if self.truncated {
            TRUNCATED_HOLD
        } else {
            SWEEP_INTERVAL
        }
    }

     
    fn begin(&mut self, radius: f32, center: [f64; 3], now: Instant) -> anyhow::Result<()> {
        let Some(world) = minecraft().world()? else {
            return Ok(());
        };
        let min_y = world.get_min_y()?;
        let sections = world.sections_count()?;
        self.sections = sections;
        self.queue = sweep_sections(center, radius, min_y, sections);
        self.cursor = 0;
        self.fresh.clear();
        self.shapes.reset();
        self.started = Some(now);
         
         
        self.finished = Some(now);
        self.truncated = false;
        if self.queue.is_empty() {
            self.blocks.clear();
            self.published.clear();
            self.generation = self.generation.wrapping_add(1);
        }
        Ok(())
    }

     
     
     
     
     
    fn advance(
        &mut self,
        cfg: &Config,
        radius: f32,
        center: [f64; 3],
        now: Instant,
    ) -> anyhow::Result<()> {
        if self.cursor >= self.queue.len() {
            let ready = self
                .finished
                .is_none_or(|finished| now.duration_since(finished) >= self.hold());
            if !ready {
                return Ok(());
            }
            self.begin(radius, center, now)?;
            if self.queue.is_empty() {
                return Ok(());
            }
        }
        let Some(world) = minecraft().world()? else {
            return Ok(());
        };
         
         
        let selected: Vec<GlobalRef> = if cfg.all_blocks {
            Vec::new()
        } else {
            cfg.blocks
                .iter()
                .filter_map(|id| crate::graphic::esp::resolve_block(id))
                .collect()
        };
        if !cfg.all_blocks && selected.is_empty() {
            self.clear();
            return Ok(());
        }
        let Ok(min_y) = world.get_min_y() else {
            let _ = mapping().get_env().map(|env| env.exception_clear());
            return Ok(());
        };

        let deadline = Instant::now() + SWEEP_BUDGET;
        let mut env = mapping().get_env()?;
         
         
        let mut column: Option<((i32, i32), LevelChunk)> = None;
        for _ in 0..MAX_SECTIONS_PER_FRAME {
            if Instant::now() >= deadline {
                break;
            }
            if self.fresh.len() >= MAX_BLOCKS {
                 
                 
                self.truncated = true;
                self.cursor = self.queue.len();
                break;
            }
            let Some(&(cx, cz, index)) = self.queue.get(self.cursor) else {
                break;
            };
            self.cursor += 1;
            let reused = column
                .as_ref()
                .is_some_and(|((open_x, open_z), _)| *open_x == cx && *open_z == cz);
            if !reused {
                column = match world.get_chunk(cx, cz) {
                    Ok(Some(chunk)) => Some(((cx, cz), chunk)),
                    Ok(None) => None,
                    Err(error) => {
                        let _ = env.exception_clear();
                        log::debug!("Reflection: get_chunk({cx}, {cz}) failed: {error}");
                        None
                    }
                };
            }
            let Some((_, chunk)) = column.as_ref() else {
                continue;
            };
            let section = Section {
                chunk_x: cx,
                chunk_z: cz,
                index,
                base_y: min_y + index * 16,
            };
            let fresh = &mut self.fresh;
            let shapes = &mut self.shapes;
            let all_blocks = cfg.all_blocks;
            let sections = self.sections;
            let world_ref = world.jni_ref().as_obj();
             
             
             
            let result = env.with_local_frame(1024, |env| {
                scan_section(
                    env, chunk, &section, &selected, all_blocks, sections, world_ref, shapes, fresh,
                )
            });
            if let Err(error) = result {
                let _ = env.exception_clear();
                log::debug!("Reflection: section ({cx}, {index}, {cz}) skipped: {error}");
            }
        }

        if self.cursor >= self.queue.len() {
            std::mem::swap(&mut self.blocks, &mut self.fresh);
            self.fresh.clear();
            self.published = self.shapes.snapshot();
             
             
            self.truncated |= self.blocks.len() >= MAX_BLOCKS;
            self.generation = self.generation.wrapping_add(1);
            self.last_duration = if self.truncated {
                TRUNCATED_HOLD
            } else {
                self.started
                    .map_or(Duration::ZERO, |started| started.elapsed())
            };
            self.finished = Some(Instant::now());
            log::trace!(
                "Reflection: swept {} blocks in {:?}{}",
                self.blocks.len(),
                self.last_duration,
                if self.truncated { " (capped)" } else { "" }
            );
        }
        Ok(())
    }
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
#[allow(clippy::too_many_arguments)]
fn scan_section(
    env: &mut JNIEnv,
    chunk: &LevelChunk,
    section: &Section,
    selected: &[GlobalRef],
    all_blocks: bool,
    sections: i32,
    world: &JObject,
    shapes: &mut ShapeTable,
    out: &mut Vec<Block>,
) -> anyhow::Result<()> {
    let Some(block_states) = chunk.get_section(section.index)? else {
        return Ok(());
    };
    if block_states.has_only_air()? {
        return Ok(());
    }
    let container = block_states.states()?;
    let data = container.data()?;
    let palette = Palette::new(data.palette()?);
    let storage = BitStorage::new(data.storage()?);
    let slots = palette.size()?.clamp(0, 512);
    if slots == 0 {
        return Ok(());
    }

    let mut matches = Vec::with_capacity(slots as usize);
    let mut air = Vec::new();
    if all_blocks {
        air.reserve(slots as usize);
        for slot in 0..slots {
            let state = palette.value_for(slot)?;
            air.push(
                mapping()
                    .call_method(MinecraftClassType::BlockState, &state, "isAir", &[])?
                    .z()?,
            );
        }
        matches.extend(air.iter().map(|is_air| !*is_air));
    } else {
        let mut any = false;
        for slot in 0..slots {
            let state = palette.value_for(slot)?;
            let block = crate::mapping::chunk::state_block(&state)?;
            let mut hit = false;
            for target in selected {
                if env.is_same_object(&block, target.as_obj())? {
                    hit = true;
                    break;
                }
            }
            any |= hit;
            matches.push(hit);
        }
        if !any {
            return Ok(());
        }
    }
    if !matches.iter().any(|matched| *matched) {
        return Ok(());
    }

     
     
     
     
     
     
    let (above_open, below_open) = if all_blocks {
        let above = if section.index + 1 < sections {
            section_open(env, world, chunk, section, section.index + 1, shapes)?
        } else {
            true
        };
        let below = if section.index > 0 {
            section_open(env, world, chunk, section, section.index - 1, shapes)?
        } else {
            true
        };
        (above, below)
    } else {
        (false, false)
    };

    let values = storage.unpack()?;
    let single_state = values.as_ref().is_some_and(|values| values.is_empty());
    let Some(values) = values else {
        return Ok(());
    };
    if single_state && !(slots == 1 && matches[0]) {
        return Ok(());
    }

     
     
    let mut first = vec![None; slots as usize];
    if single_state {
        first[0] = Some(0);
    } else {
        for (index, value) in values.iter().enumerate().take(4096) {
            if let Some(entry) = first.get_mut(*value as usize) {
                if entry.is_none() {
                    *entry = Some(index);
                }
            }
        }
    }
    let mut slot_shapes = vec![FULL_CUBE; slots as usize];
    for (slot, index) in first.iter().enumerate() {
        let Some(index) = *index else { continue };
        if !matches[slot] {
            continue;
        }
        slot_shapes[slot] = palette_shape(
            env,
            world,
            &palette,
            shapes,
            slot as i32,
            block_position(section, index),
        );
    }

     
     
     
    let open: Vec<bool> = if all_blocks {
        (0..slots as usize)
            .map(|slot| air[slot] || slot_shapes[slot] != FULL_CUBE)
            .collect()
    } else {
        Vec::new()
    };

    let mut emit = |index: usize, shape: ShapeId| -> bool {
        if out.len() >= MAX_BLOCKS {
            return false;
        }
        out.push(Block {
            pos: block_position(section, index),
            shape,
        });
        true
    };

    if single_state {
         
        let shape = slot_shapes[0];
        if all_blocks && shape == FULL_CUBE {
             
             
             
            if above_open {
                for index in 3840..4096 {
                    if !emit(index, shape) {
                        break;
                    }
                }
            }
            if below_open {
                for index in 0..256 {
                    if !emit(index, shape) {
                        break;
                    }
                }
            }
        } else {
             
             
             
            for index in 0..4096 {
                if !emit(index, shape) {
                    break;
                }
            }
        }
        return Ok(());
    }

    if all_blocks {
        for (index, value) in values.iter().enumerate().take(4096) {
            let slot = *value as usize;
            if air.get(slot).copied().unwrap_or(true) {
                continue;
            }
            if !exposed(index, &values, &open, above_open, below_open) {
                continue;
            }
            if !emit(index, slot_shapes.get(slot).copied().unwrap_or(FULL_CUBE)) {
                break;
            }
        }
    } else {
        for (index, value) in values.iter().enumerate().take(4096) {
            let slot = *value as usize;
            if !matches.get(slot).copied().unwrap_or(false) {
                continue;
            }
            if !emit(index, slot_shapes.get(slot).copied().unwrap_or(FULL_CUBE)) {
                break;
            }
        }
    }
    Ok(())
}

 
fn block_position(section: &Section, index: usize) -> [i32; 3] {
    [
        section.chunk_x * 16 + (index & 15) as i32,
        section.base_y + ((index >> 8) & 15) as i32,
        section.chunk_z * 16 + ((index >> 4) & 15) as i32,
    ]
}

 
 
 
 
fn palette_shape(
    env: &mut JNIEnv,
    world: &JObject,
    palette: &Palette,
    shapes: &mut ShapeTable,
    slot: i32,
    pos: [i32; 3],
) -> ShapeId {
     
     
     
    let result = env.with_local_frame(64, |env| -> anyhow::Result<ShapeId> {
        let state = palette.value_for(slot)?;
        let block_pos = crate::mapping::math::BlockPos::new(pos[0], pos[1], pos[2]).to_java(env)?;
        let boxes = crate::mapping::chunk::block_shape(env, &state, world, &block_pos)?;
        Ok(if boxes.is_empty() {
            FULL_CUBE
        } else {
            shapes.insert(&boxes)
        })
    });
    match result {
        Ok(shape) => shape,
        Err(_) => {
            let _ = env.exception_clear();
            FULL_CUBE
        }
    }
}

 
 
 
 
 
 
 
 
fn section_open(
    env: &mut JNIEnv,
    world: &JObject,
    chunk: &LevelChunk,
    section: &Section,
    index: i32,
    shapes: &mut ShapeTable,
) -> anyhow::Result<bool> {
    let Some(block_states) = chunk.get_section(index)? else {
        return Ok(true);
    };
    if block_states.has_only_air()? {
        return Ok(true);
    }
    let container = block_states.states()?;
    let data = container.data()?;
    let palette = Palette::new(data.palette()?);
    if palette.size()? != 1 {
        return Ok(true);
    }
    let state = palette.value_for(0)?;
    if mapping()
        .call_method(MinecraftClassType::BlockState, &state, "isAir", &[])?
        .z()?
    {
        return Ok(true);
    }
    let base_y = section.base_y + (index - section.index) * 16;
    let shape = palette_shape(
        env,
        world,
        &palette,
        shapes,
        0,
        [section.chunk_x * 16, base_y, section.chunk_z * 16],
    );
    Ok(shape != FULL_CUBE)
}

 
 
 
 
fn exposed(
    index: usize,
    indices: &[i32],
    open: &[bool],
    above_open: bool,
    below_open: bool,
) -> bool {
    let x = index & 15;
    let y = (index >> 8) & 15;
    let z = (index >> 4) & 15;
    let neighbour_is_open = |neighbour: usize| {
        indices
            .get(neighbour)
            .and_then(|slot| open.get(*slot as usize))
            .copied()
            .unwrap_or(false)
    };
    (x > 0 && neighbour_is_open(index - 1))
        || (x < 15 && neighbour_is_open(index + 1))
        || (y > 0 && neighbour_is_open(index - 256))
        || (y < 15 && neighbour_is_open(index + 256))
        || (z > 0 && neighbour_is_open(index - 16))
        || (z < 15 && neighbour_is_open(index + 16))
        || (y == 15 && above_open)
        || (y == 0 && below_open)
}

 

 
 
 
 
 
 
struct Build {
    blocks: Vec<Block>,
     
    shapes: Vec<Vec<ShapeBox>>,
    occupied: HashSet<[i32; 3]>,
    cursor: usize,
    occupancy_done: bool,
    vertices: Vec<f32>,
}

impl Build {
    fn new(blocks: Vec<Block>, shapes: Vec<Vec<ShapeBox>>) -> Self {
        let reserve = blocks.len().min(65_536) * FACE_VERTICES * STRIDE;
        Self {
            occupied: HashSet::with_capacity(blocks.len()),
            blocks,
            shapes,
            cursor: 0,
            occupancy_done: false,
            vertices: Vec::with_capacity(reserve),
        }
    }

     
     
    fn advance(&mut self, budget: Duration) -> bool {
        let deadline = Instant::now() + budget;
        if !self.occupancy_done {
            while self.cursor < self.blocks.len() {
                let block = self.blocks[self.cursor];
                if block.shape == FULL_CUBE {
                    self.occupied.insert(block.pos);
                }
                self.cursor += 1;
                if Instant::now() >= deadline {
                    return false;
                }
            }
            self.occupancy_done = true;
            self.cursor = 0;
        }
        while self.cursor < self.blocks.len() {
            push_block(
                &mut self.vertices,
                self.blocks[self.cursor],
                &self.shapes,
                &self.occupied,
                MAX_VERTICES,
            );
            self.cursor += 1;
            if self.vertices.len() / STRIDE >= MAX_VERTICES {
                self.cursor = self.blocks.len();
                break;
            }
            if Instant::now() >= deadline {
                return false;
            }
        }
        true
    }
}

 

struct Gpu {
    program: u32,
    vao: u32,
    vbo: u32,
    scene: u32,
    depth: u32,
    size: [i32; 2],
    depth_size: [i32; 2],
     
     
    uploaded: u64,
    view: i32,
    projection: i32,
    inverse_projection: i32,
    center: i32,
    time: i32,
    strength: i32,
    blur: i32,
    shimmer: i32,
    viewport: i32,
    scene_depth: i32,
}

impl Gpu {
    unsafe fn create() -> anyhow::Result<Self> {
        let vertex = compile(gl::VERTEX_SHADER, VERTEX)?;
        let fragment = compile(gl::FRAGMENT_SHADER, FRAGMENT)?;
        let program = gl::CreateProgram();
        gl::AttachShader(program, vertex);
        gl::AttachShader(program, fragment);
        gl::LinkProgram(program);
        gl::DeleteShader(vertex);
        gl::DeleteShader(fragment);
        let mut linked = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut linked);
        ensure!(linked != 0, "reflection program failed to link");

        let view = uniform(program, "View")?;
        let projection = uniform(program, "Projection")?;
        let inverse_projection = uniform(program, "InverseProjection")?;
        let center = uniform(program, "Center")?;
        let time = uniform(program, "Time")?;
        let strength = uniform(program, "Strength")?;
        let blur = uniform(program, "Blur")?;
        let shimmer = uniform(program, "Shimmer")?;
        let viewport = uniform(program, "Viewport")?;
        let scene_sampler = uniform(program, "Scene")?;
        let depth_sampler = uniform(program, "SceneDepth")?;
        gl::UseProgram(program);
         
         
        gl::Uniform1i(scene_sampler, 0);
        gl::Uniform1i(depth_sampler, 1);

        let mut vao = 0;
        let mut vbo = 0;
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        let stride = (STRIDE * std::mem::size_of::<f32>()) as i32;
        for (name, components, offset) in [("Position", 3, 0), ("Normal", 3, 12)] {
            let name = std::ffi::CString::new(name)?;
            let location = gl::GetAttribLocation(program, name.as_ptr());
            ensure!(location >= 0, "reflection shader is missing a vertex input");
            gl::EnableVertexAttribArray(location as u32);
            gl::VertexAttribPointer(
                location as u32,
                components,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset as *const std::ffi::c_void,
            );
        }

        let mut scene = 0;
        gl::GenTextures(1, &mut scene);
        gl::BindTexture(gl::TEXTURE_2D, scene);
         
         
         
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR_MIPMAP_LINEAR as i32,
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

         
         
        let mut depth = 0;
        gl::GenTextures(1, &mut depth);
        gl::BindTexture(gl::TEXTURE_2D, depth);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::ActiveTexture(gl::TEXTURE0);

        Ok(Self {
            program,
            vao,
            vbo,
            scene,
            depth,
            size: [0, 0],
            depth_size: [0, 0],
            uploaded: u64::MAX,
            view,
            projection,
            inverse_projection,
            center,
            time,
            strength,
            blur,
            shimmer,
            viewport,
            scene_depth: depth_sampler,
        })
    }
}

 

#[derive(Default)]
struct State {
    scan: Scan,
    gpu: Option<Gpu>,
    geometry: Vec<f32>,
    geometry_generation: u64,
     
     
    build: Option<Build>,
    build_generation: u64,
    ids: Vec<String>,
    all_blocks: bool,
     
    radius_cache: Option<(f32, Instant)>,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(State::default()))
}

static FAILED: AtomicBool = AtomicBool::new(false);
static CALLBACK_LOGGED: AtomicBool = AtomicBool::new(false);
static CLOCK: OnceLock<Instant> = OnceLock::new();

 
 
fn elapsed() -> f32 {
    CLOCK.get_or_init(Instant::now).elapsed().as_secs_f32()
}

 
 
pub(crate) fn reset() {
    let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
    state.scan.clear();
    state.geometry.clear();
    state.geometry_generation = state.geometry_generation.wrapping_add(1);
    state.build = None;
    state.build_generation = state.scan.generation;
    state.ids.clear();
    state.radius_cache = None;
}

 
 
fn report_failure(error: &str) {
    if !FAILED.swap(true, Ordering::Relaxed) {
        log::error!("[Reflection] render disabled: {error}");
    }
}

 
fn config() -> Option<Config> {
    let handle = client().modules.get(ModuleId::Reflection)?;
    let module = handle.try_lock().ok()?;
    let data = module.get_module_data();
    data.enabled.then(|| Config::read(data))
}

 
 
pub(crate) fn render_world(
    camera: &JObject,
    view: &JObject,
    projection: &JObject,
) -> anyhow::Result<()> {
    if FAILED.load(Ordering::Relaxed) || !client().modules.is_active() {
        return Ok(());
    }
    let Some(cfg) = config() else {
        return Ok(());
    };
    mapping().in_frame(|| {
        let view = matrix(view)?;
        let projection = matrix(projection)?;
        let position = mapping()
            .call_method(MinecraftClassType::Camera, camera, "getPosition", &[])?
            .l()?;
        let origin = Vec3::read(&position)?;
        let center = [origin.x(), origin.y(), origin.z()];
        if !center.iter().all(|value| value.is_finite()) {
            return Ok(());
        }
        if !CALLBACK_LOGGED.swap(true, Ordering::Relaxed) {
            log::info!("[Reflection] world render callback active");
        }

        let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
        if state.ids != cfg.blocks || state.all_blocks != cfg.all_blocks {
            state.ids = cfg.blocks.clone();
            state.all_blocks = cfg.all_blocks;
            state.scan.clear();
        }
        let radius = effective_radius(&cfg, &mut state.radius_cache);
        state.scan.advance(&cfg, radius, center, Instant::now())?;
        if state.scan.generation != state.build_generation {
            state.build_generation = state.scan.generation;
            state.build = Some(Build::new(
                state.scan.blocks.clone(),
                state.scan.published.clone(),
            ));
        }
        if let Some(build) = state.build.as_mut() {
            if build.advance(BUILD_BUDGET) {
                let build = state.build.take().expect("build just finished");
                log::debug!(
                    "[Reflection] {} blocks, {} faces",
                    build.blocks.len(),
                    build.vertices.len() / (STRIDE * FACE_VERTICES)
                );
                state.geometry = build.vertices;
                state.geometry_generation = state.geometry_generation.wrapping_add(1);
            }
        }
        if state.geometry.is_empty() {
            return Ok(());
        }
         
         
        match unsafe { state.draw(&cfg, &center, &view, &projection, elapsed()) } {
            Ok(()) => Ok(()),
            Err(error) => {
                 
                 
                 
                report_failure(&format!("{error:#}"));
                Ok(())
            }
        }
    })
}

impl State {
    unsafe fn draw(
        &mut self,
        cfg: &Config,
        center: &[f64; 3],
        view: &[f32; 16],
        projection: &[f32; 16],
        time: f32,
    ) -> anyhow::Result<()> {
        let guard = GlGuard::capture();
        let [x, y, width, height] = guard.viewport();
        ensure!(
            width > 0 && height > 0 && guard.draw_fbo() != 0,
            "world framebuffer unavailable"
        );
        let mut samples = 0;
        gl::GetIntegerv(gl::SAMPLES, &mut samples);
        ensure!(
            samples == 0,
            "multisampled world target cannot be copied with CopyTexImage2D"
        );
        let mut depth_attachment = 0;
        gl::GetFramebufferAttachmentParameteriv(
            gl::DRAW_FRAMEBUFFER,
            gl::DEPTH_ATTACHMENT,
            gl::FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE,
            &mut depth_attachment,
        );
        ensure!(
            depth_attachment != gl::NONE as i32,
            "world framebuffer has no depth attachment to reflect against"
        );
         
         
        let inverse = inverse(projection).context("the world projection matrix is singular")?;

        if self.gpu.is_none() {
            self.gpu = Some(Gpu::create().context("reflection shader setup failed")?);
        }
        let gpu = self.gpu.as_mut().expect("gpu just ensured");

         
         
        gl::BindFramebuffer(gl::READ_FRAMEBUFFER, guard.draw_fbo() as u32);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindSampler(0, 0);
        gl::BindTexture(gl::TEXTURE_2D, gpu.scene);
        gl::ReadBuffer(gl::COLOR_ATTACHMENT0);
        if gpu.size == [width, height] {
            gl::CopyTexSubImage2D(gl::TEXTURE_2D, 0, 0, 0, x, y, width, height);
        } else {
            gl::CopyTexImage2D(gl::TEXTURE_2D, 0, gl::RGBA8, x, y, width, height, 0);
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::LINEAR_MIPMAP_LINEAR as i32,
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gpu.size = [width, height];
        }
         
         
         
        gl::GenerateMipmap(gl::TEXTURE_2D);
         
        gl::ActiveTexture(gl::TEXTURE1);
        gl::BindSampler(1, 0);
        gl::BindTexture(gl::TEXTURE_2D, gpu.depth);
        if gpu.depth_size == [width, height] {
            gl::CopyTexSubImage2D(gl::TEXTURE_2D, 0, 0, 0, x, y, width, height);
        } else {
            gl::CopyTexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::DEPTH_COMPONENT24,
                x,
                y,
                width,
                height,
                0,
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gpu.depth_size = [width, height];
        }
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, guard.draw_fbo() as u32);

        gl::UseProgram(gpu.program);
        gl::Uniform1i(gpu.scene_depth, 1);
        gl::BindVertexArray(gpu.vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, gpu.vbo);
        if gpu.uploaded != self.geometry_generation {
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(self.geometry.as_slice()) as isize,
                self.geometry.as_ptr().cast(),
                gl::STREAM_DRAW,
            );
            gpu.uploaded = self.geometry_generation;
        }
        gl::UniformMatrix4fv(gpu.view, 1, gl::FALSE, view.as_ptr());
        gl::UniformMatrix4fv(gpu.projection, 1, gl::FALSE, projection.as_ptr());
        gl::UniformMatrix4fv(gpu.inverse_projection, 1, gl::FALSE, inverse.as_ptr());
        gl::Uniform3f(
            gpu.center,
            center[0] as f32,
            center[1] as f32,
            center[2] as f32,
        );
        gl::Uniform4f(
            gpu.viewport,
            x as f32,
            y as f32,
            width as f32,
            height as f32,
        );
        gl::Uniform1f(gpu.strength, cfg.strength);
        gl::Uniform1f(gpu.blur, cfg.blur);
        gl::Uniform1f(gpu.time, time);
        gl::Uniform1i(gpu.shimmer, i32::from(cfg.shimmer));

        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::SCISSOR_TEST);
        gl::Disable(gl::STENCIL_TEST);
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LEQUAL);
        gl::DepthMask(gl::FALSE);
        gl::Enable(gl::BLEND);
        gl::BlendEquationSeparate(gl::FUNC_ADD, gl::FUNC_ADD);
        gl::BlendFuncSeparate(
            gl::ONE,
            gl::ONE_MINUS_SRC_ALPHA,
            gl::ONE,
            gl::ONE_MINUS_SRC_ALPHA,
        );
        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
         
         
         
        let offset_was_on = gl::IsEnabled(gl::POLYGON_OFFSET_FILL) != 0;
        gl::Enable(gl::POLYGON_OFFSET_FILL);
        gl::PolygonOffset(-1.0, -1.0);
        gl::DrawArrays(gl::TRIANGLES, 0, (self.geometry.len() / STRIDE) as i32);
        gl::PolygonOffset(0.0, 0.0);
        if !offset_was_on {
            gl::Disable(gl::POLYGON_OFFSET_FILL);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::render::reflection::ReflectionModule;

    fn config() -> Config {
        Config::read(&ReflectionModule::new().module)
    }

     
    fn rows(vertices: &[f32]) -> Vec<([f32; 3], [f32; 3])> {
        vertices
            .chunks_exact(STRIDE)
            .map(|vertex| {
                (
                    [vertex[0], vertex[1], vertex[2]],
                    [vertex[3], vertex[4], vertex[5]],
                )
            })
            .collect()
    }

    fn block(pos: [i32; 3], shape: ShapeId) -> Block {
        Block { pos, shape }
    }

     
    fn cubes() -> Vec<Vec<ShapeBox>> {
        ShapeTable::new().snapshot()
    }

     
     
    const STAIR: [[f64; 6]; 2] = [
        [0.0, 0.0, 0.0, 1.0, 0.5, 1.0],
        [0.0, 0.5, 0.5, 1.0, 1.0, 1.0],
    ];

    #[test]
    fn defaults_are_the_polished_ones() {
        let cfg = config();
        assert_eq!(
            cfg.radius, MIN_SQUARE_RADIUS,
            "the manual range starts at the 32-chunk square"
        );
        assert!(cfg.all_blocks, "every block is the default surface");
        assert!(
            cfg.follow_render_distance,
            "the default range follows the render distance"
        );
        assert_eq!(cfg.strength, 1.0);
        assert!((cfg.blur - 0.35).abs() < 1e-6);
        assert!(cfg.shimmer);
        assert!(cfg.blocks.iter().any(|id| id == "minecraft:ice"));
        assert!(!cfg.blocks.is_empty());
    }

    #[test]
    fn sliders_are_clamped_and_non_finite_values_fall_back() {
        let mut data = ReflectionModule::new().module;
        for (name, value) in [("Radius", f32::NAN), ("Strength", 9.0), ("Blur", -4.0)] {
            data.get_setting_mut(name)
                .expect("slider setting")
                .set_slider_value(value);
        }
        let cfg = Config::read(&data);
        assert_eq!(
            cfg.radius, 24.0,
            "a broken slider falls back to its default"
        );
        assert_eq!(cfg.strength, 1.0);
        assert_eq!(cfg.blur, 0.0);
    }

    #[test]
    fn the_radius_slider_can_reach_the_vanilla_render_distance_ceiling() {
        let mut data = ReflectionModule::new().module;
        data.get_setting_mut("Radius")
            .expect("radius")
            .set_slider_value(5000.0);
        assert_eq!(Config::read(&data).radius, MAX_RADIUS);
        assert_eq!(MAX_RADIUS, 32.0 * 16.0);
    }

    #[test]
    fn a_render_distance_becomes_a_radius_in_blocks() {
        assert_eq!(radius_for_render_distance(12, 0), 192.0);
        assert_eq!(radius_for_render_distance(32, 0), 512.0);
         
        assert_eq!(radius_for_render_distance(32, 8), 128.0);
         
        assert_eq!(radius_for_render_distance(0, 0), 32.0);
        assert_eq!(radius_for_render_distance(9999, 0), 512.0);
    }

    #[test]
    fn a_lonely_block_draws_all_six_faces() {
        let mut out = Vec::new();
        push_block(
            &mut out,
            block([0, 0, 0], FULL_CUBE),
            &cubes(),
            &HashSet::new(),
            usize::MAX,
        );
        assert_eq!(out.len(), 6 * FACE_VERTICES * STRIDE);
        for (position, normal) in rows(&out) {
            let length: f32 = normal.iter().map(|value| value * value).sum();
            assert!((length - 1.0).abs() < 1e-5, "normal not unit");
            assert!(
                (0.0..=1.0).contains(&position[0]) && (0.0..=1.0).contains(&position[1]),
                "face corner left the block"
            );
        }
    }

    #[test]
    fn touching_blocks_drop_the_face_between_them() {
        let mut out = Vec::new();
        let occupied: HashSet<[i32; 3]> = [[0, 0, 0], [1, 0, 0]].into_iter().collect();
        for pos in [[0, 0, 0], [1, 0, 0]] {
            push_block(
                &mut out,
                block(pos, FULL_CUBE),
                &cubes(),
                &occupied,
                usize::MAX,
            );
        }
         
        assert_eq!(out.len(), 10 * FACE_VERTICES * STRIDE);
         
         
        for (position, normal) in rows(&out) {
            if (position[0] - 1.0).abs() < 1e-6 {
                assert_eq!(normal[0], 0.0, "the shared plane must not carry a face");
            }
        }
    }

     
     
     
    #[test]
    fn a_stair_draws_its_steps_and_not_the_air_in_front_of_them() {
        let mut table = ShapeTable::new();
        let stair = table.insert(&STAIR);
        assert_ne!(stair, FULL_CUBE, "a stair is not the cube");
        let shapes = table.snapshot();
        let mut out = Vec::new();
        push_block(
            &mut out,
            block([0, 0, 0], stair),
            &shapes,
            &HashSet::new(),
            usize::MAX,
        );
         
        assert_eq!(out.len(), 12 * FACE_VERTICES * STRIDE);
        for (position, normal) in rows(&out) {
            assert!((normal.iter().map(|v| v * v).sum::<f32>() - 1.0).abs() < 1e-5);
             
             
            assert!(
                position[1] <= 0.5 + 1e-6 || position[2] >= 0.5 - 1e-6,
                "corner {position:?} sits in the missing half of the stair"
            );
        }
         
         
        let tops: Vec<f32> = rows(&out)
            .into_iter()
            .filter(|(_, normal)| normal[1] > 0.5)
            .map(|(position, _)| position[1])
            .collect();
        assert!(tops.iter().any(|y| (y - 0.5).abs() < 1e-6), "no step");
        assert!(tops.iter().any(|y| (y - 1.0).abs() < 1e-6), "no top");
    }

    #[test]
    fn a_full_cube_hides_the_stair_face_between_them() {
        let mut table = ShapeTable::new();
        let stair = table.insert(&STAIR);
        let shapes = table.snapshot();
        let occupied: HashSet<[i32; 3]> = [[1, 0, 0]].into_iter().collect();
        let mut out = Vec::new();
        push_block(
            &mut out,
            block([0, 0, 0], stair),
            &shapes,
            &occupied,
            usize::MAX,
        );
         
         
        assert_eq!(out.len(), 10 * FACE_VERTICES * STRIDE);
        assert!(rows(&out).iter().all(|(_, normal)| normal[0] <= 0.0));
    }

     
     
     
    #[test]
    fn a_partial_shape_does_not_hide_the_cube_beside_it() {
        let mut table = ShapeTable::new();
        let stair = table.insert(&STAIR);
        let shapes = table.snapshot();
        let blocks = vec![block([0, 0, 0], stair), block([1, 0, 0], FULL_CUBE)];
        let mut build = Build::new(blocks, shapes);
        let mut rounds = 0;
        while !build.advance(Duration::ZERO) {
            rounds += 1;
            assert!(rounds < 64, "the build never finished");
        }
        assert_eq!(build.occupied.len(), 1, "only the cube hides a neighbour");
        assert!(
            rows(&build.vertices)
                .iter()
                .any(|(position, normal)| { (position[0] - 1.0).abs() < 1e-6 && normal[0] < -0.5 }),
            "the cube lost the face toward the stair"
        );
    }

    #[test]
    fn shapes_are_deduplicated_and_the_cube_keeps_its_fast_id() {
        let mut table = ShapeTable::new();
        assert_eq!(
            table.insert(&[[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]),
            FULL_CUBE,
            "the full cube is the fast path, not a new entry"
        );
        let first = table.insert(&STAIR);
        assert_eq!(first, table.insert(&STAIR), "equal boxes share one entry");
        let shifted = table.insert(&[[0.0, 0.0, 0.0, 0.9375, 1.0, 1.0]]);
        assert_ne!(shifted, first, "a different box is a different shape");
        let shapes = table.snapshot();
        assert_eq!(shapes[FULL_CUBE as usize], FULL_CUBE_BOXES.to_vec());
        let stair: Vec<ShapeBox> = STAIR
            .iter()
            .map(|bounds| bounds.map(|value| value as f32))
            .collect();
        assert_eq!(shapes[first as usize], stair);
         
        let mut out = Vec::new();
        push_block(
            &mut out,
            block([0, 0, 0], 999),
            &shapes,
            &HashSet::new(),
            usize::MAX,
        );
        assert_eq!(out.len(), 6 * FACE_VERTICES * STRIDE);
    }

    #[test]
    fn boxes_are_snapped_to_the_shape_grid_and_degenerate_ones_dropped() {
        assert_eq!(
            quantize(&[[0.0, 0.0, 0.0, 0.5001, 0.9997, 1.0]]),
            vec![[0, 0, 0, 32, 64, 64]],
            "boxes land on the 1/64 grid"
        );
        assert!(
            quantize(&[[0.0, 0.5, 0.0, 1.0, 0.5, 1.0]]).is_empty(),
            "a flat box would draw nothing"
        );
        assert!(
            quantize(&[[f64::NAN, 0.0, 0.0, 1.0, 1.0, 1.0]]).is_empty(),
            "a broken box is dropped, not drawn"
        );
        assert_eq!(
            quantize(&[[-2.0, 0.0, 0.0, 3.0, 1.0, 1.0]]),
            vec![[0, 0, 0, 64, 64, 64]],
            "a box leaving the cell is clamped into it"
        );
    }

    #[test]
    fn every_neighbour_is_the_adjacent_cube() {
        for face in 0..6 {
            let n = neighbour([3, -4, 7], face);
            let delta = [n[0] - 3, n[1] + 4, n[2] - 7];
            assert_eq!(delta, FACE_NORMALS[face].map(|value| value as i32));
        }
    }

     
     
    fn section_fixture(hole: Option<usize>) -> (Vec<i32>, Vec<bool>) {
        let mut indices = vec![1; 4096];
        if let Some(cell) = hole {
            indices[cell] = 0;
        }
        (indices, vec![true, false])
    }

     
     
     
    #[test]
    fn all_blocks_only_collects_cells_that_can_show_a_face() {
        let (solid, open) = section_fixture(None);
         
        assert!(!exposed(5 * 256 + 5 * 16 + 5, &solid, &open, false, false));
         
        assert!(exposed(15 * 256 + 5 * 16 + 5, &solid, &open, true, false));
        assert!(!exposed(15 * 256 + 5 * 16 + 5, &solid, &open, false, false));
        assert!(exposed(5 * 16 + 5, &solid, &open, false, true));
         
         
        let (with_hole, open) = section_fixture(Some(5 * 256 + 5 * 16 + 6));
        assert!(exposed(
            5 * 256 + 5 * 16 + 5,
            &with_hole,
            &open,
            false,
            false
        ));
        assert!(!exposed(
            5 * 256 + 9 * 16 + 9,
            &with_hole,
            &open,
            false,
            false
        ));
    }

     
     
     
    #[test]
    fn a_partial_neighbour_leaves_the_block_behind_it_exposed() {
        let cell = 5 * 256 + 5 * 16 + 5;
        let above = cell + 256;
         
        let open = [true, false, true];
        let mut with_slab = vec![1; 4096];
        with_slab[above] = 2;
        assert!(exposed(cell, &with_slab, &open, false, false));
        let mut with_cube = vec![1; 4096];
        with_cube[above] = 1;
        assert!(!exposed(cell, &with_cube, &open, false, false));
    }

    #[test]
    fn a_packed_index_becomes_its_world_position() {
        let section = Section {
            chunk_x: 3,
            chunk_z: -2,
            index: 4,
            base_y: -64 + 64,
        };
         
        assert_eq!(block_position(&section, 0), [48, 0, -32]);
        assert_eq!(block_position(&section, 1 | 2 << 4 | 3 << 8), [49, 3, -30]);
        assert_eq!(
            block_position(&section, 15 | 15 << 4 | 15 << 8),
            [63, 15, -17]
        );
    }

    #[test]
    fn the_vertex_cap_stops_the_buffer_growing_forever() {
        let mut out = Vec::new();
        let cap = 12;
        push_block(
            &mut out,
            block([0, 0, 0], FULL_CUBE),
            &cubes(),
            &HashSet::new(),
            cap,
        );
        assert_eq!(out.len(), cap * STRIDE);
    }

    #[test]
    fn the_sweep_is_a_square_of_columns_over_the_whole_world_height() {
        let queue = sweep_sections([0.0, 64.0, 0.0], 16.0, -64, 24);
        assert!(!queue.is_empty());
        assert!(queue
            .iter()
            .all(|(cx, cz, _)| (-1..=1).contains(cx) && (-1..=1).contains(cz)));
         
         
         
        assert_eq!(queue.len(), 9 * 24);
        for index in 0..24 {
            assert!(
                queue.contains(&(0, 0, index)),
                "section {index} is missing from the centre column"
            );
        }
         
        assert_eq!(queue.first().map(|(cx, cz, _)| (*cx, *cz)), Some((0, 0)));
         
         
        assert_eq!(queue.first(), Some(&(0, 0, 8)), "y = 64 is section 8");
        assert!(
            queue.iter().take(9).all(|(_, _, index)| *index == 8),
            "each column's eye section comes first"
        );
        assert_eq!(queue.get(9), Some(&(0, 0, 7)));
        assert_eq!(queue.get(10), Some(&(0, 0, 9)));
    }

    #[test]
    fn a_render_distance_sweep_covers_every_loaded_column_and_the_corner() {
        let queue = sweep_sections([0.0, 64.0, 0.0], 192.0, -64, 24);
        assert!(!queue.is_empty());
        let reach: i32 = 192 / 16;
        let expected = (reach * 2 + 1).pow(2) * 24;
        assert_eq!(queue.len(), expected as usize);
        assert!(queue
            .iter()
            .all(|(cx, cz, _)| cx.abs() <= reach && cz.abs() <= reach));
         
         
        assert!(queue.contains(&(reach, reach, 8)));
        assert!(queue.contains(&(-reach, -reach, 0)));
         
         
        assert_eq!(queue.first().map(|(cx, cz, _)| (*cx, *cz)), Some((0, 0)));
    }

    #[test]
    fn a_world_without_sections_lists_nothing() {
        assert!(sweep_sections([0.0, 64.0, 0.0], 16.0, -64, 0).is_empty());
    }

     
     
     
     
    #[test]
    fn a_finished_sweep_is_refreshed_on_the_short_interval() {
        let fresh = Scan::default();
        assert_eq!(fresh.hold(), SWEEP_INTERVAL);
        let slow = Scan {
            last_duration: Duration::from_secs(4),
            ..Default::default()
        };
        assert_eq!(
            slow.hold(),
            SWEEP_INTERVAL,
            "a slow pass must not double the refresh period"
        );
        let capped = Scan {
            truncated: true,
            ..Default::default()
        };
        assert_eq!(
            capped.hold(),
            TRUNCATED_HOLD,
            "a capped pass waits before hitting the same cap again"
        );
    }

     
     
     
    #[test]
    fn the_incremental_build_matches_a_single_pass() {
        let blocks: Vec<Block> = [[0, 0, 0], [1, 0, 0], [0, 1, 0], [5, 2, -3]]
            .into_iter()
            .map(|pos| block(pos, FULL_CUBE))
            .collect();
        let occupied: HashSet<[i32; 3]> = blocks.iter().map(|block| block.pos).collect();
        let shapes = cubes();
        let mut expected = Vec::new();
        for block in &blocks {
            push_block(&mut expected, *block, &shapes, &occupied, MAX_VERTICES);
        }
        let mut build = Build::new(blocks, shapes);
        let mut rounds = 0;
        while !build.advance(Duration::ZERO) {
            rounds += 1;
            assert!(rounds < 64, "the build never finished");
        }
        assert_eq!(build.vertices, expected);
        assert_eq!(build.occupied, occupied);
    }

    #[test]
    fn every_uniform_the_draw_sets_is_declared_by_a_shader() {
        let declared = |name: &str| {
            VERTEX
                .lines()
                .chain(FRAGMENT.lines())
                .any(|line| line.starts_with("uniform ") && line.contains(name))
        };
        for name in [
            "View",
            "Projection",
            "InverseProjection",
            "Center",
            "Time",
            "Strength",
            "Blur",
            "Shimmer",
            "Viewport",
            "Scene",
            "SceneDepth",
        ] {
            assert!(declared(name), "{name} is set but never declared");
        }
    }

    #[test]
    fn every_vertex_input_the_pass_binds_is_declared() {
        for attribute in ["in vec3 Position;", "in vec3 Normal;"] {
            assert!(VERTEX.contains(attribute), "{attribute} is missing");
        }
    }

     
     
     
    #[test]
    fn the_reflection_carries_the_frames_own_colours() {
        assert!(
            !FRAGMENT.contains("Tint"),
            "the reflection must not be tinted"
        );
        assert!(
            FRAGMENT.contains("textureLod(Scene, uv"),
            "an out-of-frame ray must sample the frame's own colour"
        );
        assert!(
            !FRAGMENT.contains("vec3(0.93, 0.97, 1.0)"),
            "the icy body colour must not come back"
        );
    }

     
     
     
     
    #[test]
    fn both_stages_are_syntactically_balanced_glsl_150() {
        for (stage, source) in [("vertex", VERTEX), ("fragment", FRAGMENT)] {
            assert!(source.starts_with("#version 150"), "{stage} stage");
            for (open, close) in [('{', '}'), ('(', ')')] {
                let opened = source.chars().filter(|c| *c == open).count();
                let closed = source.chars().filter(|c| *c == close).count();
                assert_eq!(opened, closed, "{stage} stage: unbalanced {open}{close}");
            }
        }
         
         
        for needed in [
            "reflect(incident, n)",
            "InverseProjection",
            "SceneDepth",
            "textureLod",
        ] {
            assert!(FRAGMENT.contains(needed), "fragment lost {needed}");
        }
         
         
        assert!(!FRAGMENT.contains("hash(gl_FragCoord"), "per-pixel dither");
    }
}
