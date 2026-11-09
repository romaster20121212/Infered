 
 
use super::chams_warped_render::{Frame as WarpedFrame, WarpedPass};
use super::smoke_shader;
use crate::gl;
use crate::mapping::math::{BlockPos, Vec3};
use crate::mapping::{MappedObject, MinecraftClassType};
use crate::module::{ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, mapping, minecraft};
use anyhow::{ensure, Context};
use jni::objects::{JClass, JFloatArray, JObject, JValue};
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

const MAX_PUFFS: usize = 384;
const SPAWN_KEYS: [i32; 8] = [71, 72, 74, 75, 86, 66, 78, 77];
static FAILED: AtomicBool = AtomicBool::new(false);
static CALLBACK_LOGGED: AtomicBool = AtomicBool::new(false);
static SOURCES: AtomicU8 = AtomicU8::new(0);

fn source_bit(id: ModuleId) -> u8 {
    match id {
        ModuleId::Smoke => 1,
         
         
        ModuleId::SkeletonEsp => 2,
        ModuleId::Chams => 4,
        ModuleId::CustomCape => 8,
        ModuleId::PlayerAura => 16,
        ModuleId::Explosion => 32,
        ModuleId::ItemChams => 64,
        ModuleId::HologramChams => 128,
        _ => 0,
    }
}

 
#[derive(Clone, Copy, Debug)]
pub(crate) struct DrawPuff {
    pub pos: [f64; 3],
    pub radius: f32,
    pub seed: f32,
    pub alpha: f32,
    pub light: f32,
    pub tint: [f32; 3],
     
    pub blast: bool,
}

 
 
static EXPLOSION_PUFFS: Mutex<Vec<DrawPuff>> = Mutex::new(Vec::new());

pub(crate) fn set_explosion_puffs(puffs: Vec<DrawPuff>) {
    *EXPLOSION_PUFFS.lock().unwrap_or_else(|p| p.into_inner()) = puffs;
}

#[derive(Clone, Debug)]
struct Config {
    density: f32,
    size: f32,
    lifetime: f32,
    wind: f32,
    direction: f32,
    tint: [f32; 4],
    opacity: f32,
    moving: bool,
    key: bool,
    spawn_key: i32,
}
pub(super) fn finite(value: f32, fallback: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}
impl Config {
    fn read(data: &ModuleData) -> Self {
        let slider = |name, fallback, min, max| {
            finite(
                data.get_setting(name)
                    .and_then(ModuleSetting::get_slider_value)
                    .unwrap_or(fallback),
                fallback,
                min,
                max,
            )
        };
        let toggle = |name| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_toggle_value)
                .unwrap_or(true)
        };
        let tint = match data.get_setting("Color Tint") {
            Some(ModuleSetting::Color { value, .. }) => {
                std::array::from_fn(|i| finite(value[i], 0.65, 0.0, 1.0))
            }
            _ => [1.0, 0.0, 0.0, 1.0],
        };
        let key_index = match data.get_setting("Spawn Key") {
            Some(ModuleSetting::Choice { value, .. }) => *value,
            _ => 0,
        };
        Self {
            density: slider("Density", 24.0, 1.0, 60.0),
            size: slider("Size", 0.65, 0.2, 2.0),
            lifetime: slider("Lifetime", 4.0, 0.5, 8.0),
            wind: slider("Wind", 0.25, 0.0, 2.0),
            direction: slider("Wind Direction", 0.0, 0.0, 360.0).to_radians(),
            opacity: slider("Opacity", 0.65, 0.0, 1.0),
            tint,
            moving: toggle("Spawn on Move"),
            key: toggle("Spawn on Key"),
            spawn_key: *SPAWN_KEYS.get(key_index).unwrap_or(&71),
        }
    }
}

#[derive(Debug, Clone)]
struct Puff {
    origin: [f64; 3],
    pos: [f64; 3],
    velocity: [f64; 3],
    age: f32,
    life: f32,
    size: f32,
    seed: f32,
    light: f32,
}
impl Puff {
    fn advance(&mut self, dt: f32) {
        self.age += dt;
        let t = self.age as f64;
         
        self.pos = std::array::from_fn(|i| self.origin[i] + self.velocity[i] * t);
        self.pos[0] += 0.10 * ((t * 1.1 + self.seed as f64).sin() - (self.seed as f64).sin());
        self.pos[2] += 0.10 * ((t * 0.9 + self.seed as f64).cos() - (self.seed as f64).cos());
    }
    fn alpha(&self) -> f32 {
        let u = (self.age / self.life).clamp(0.0, 1.0);
        smoothstep(0.0, 0.12, u) * (1.0 - smoothstep(0.45, 1.0, u))
    }
    fn radius(&self) -> f32 {
        self.size * (1.0 + 0.8 * self.age / self.life)
    }
}
fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
#[derive(Default)]
struct Simulation {
    puffs: Vec<Puff>,
    previous: Option<[f64; 3]>,
    credit: f32,
    serial: u32,
}
impl Simulation {
    fn step(
        &mut self,
        cfg: &Config,
        position: [f64; 3],
        dt: f32,
        key_down: bool,
        can_emit: bool,
        light: f32,
    ) {
        let dt = finite(dt, 0.0, 0.0, 0.1);
        for puff in &mut self.puffs {
            puff.advance(dt);
        }
        self.puffs.retain(|p| p.age < p.life);
        let old = self.previous.replace(position).unwrap_or(position);
        let distance2: f64 = old
            .iter()
            .zip(position)
            .map(|(a, b)| (a - b) * (a - b))
            .sum();
        if distance2 > 64.0 {
             
            self.puffs.clear();
            self.credit = 0.0;
            return;
        }
        let emitting = can_emit && ((cfg.moving && distance2 > 0.000_001) || (cfg.key && key_down));
        if !emitting || dt == 0.0 {
            self.credit = 0.0;
            return;
        }
        self.credit += cfg.density * dt;
        let count = self.credit.floor() as usize;
        self.credit -= count as f32;
        for i in 0..count.min(6) {
            if self.puffs.len() >= MAX_PUFFS {
                self.puffs.remove(0);
            }
            self.serial = self.serial.wrapping_add(1);
            let seed = ((self.serial.wrapping_mul(2654435761) >> 8) as f32 / 16777216.0) * 31.0;
            let f = (i + 1) as f64 / count.max(1) as f64;
            let mut origin = std::array::from_fn(|j| old[j] + (position[j] - old[j]) * f);
            origin[0] += 0.12 * (seed as f64).sin();
            origin[1] += 0.30;
            origin[2] += 0.12 * (seed as f64).cos();
            self.puffs.push(Puff {
                origin,
                pos: origin,
                velocity: [
                    -(cfg.direction.sin() * cfg.wind) as f64,
                    0.22 + 0.04 * (seed as f64).sin(),
                    (cfg.direction.cos() * cfg.wind) as f64,
                ],
                age: 0.0,
                life: cfg.lifetime,
                size: cfg.size,
                seed,
                light,
            });
        }
    }
}
struct State {
    simulation: Simulation,
    effects: super::smoke_effects::Effects,
    chams_logged: bool,
    chams_failed: bool,
    item_logged: bool,
    item_failed: bool,
    hologram_failed: bool,
    skeleton_failed: bool,
    orbs_logged: bool,
    gpu: Option<Gpu>,
     
     
    item_gpu: Option<Gpu>,
    last_frame: Option<Instant>,
    clock: f32,
    world: Option<jni::objects::GlobalRef>,
    light: f32,
    light_at: Option<Instant>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            simulation: Simulation::default(),
            effects: super::smoke_effects::Effects::default(),
            chams_logged: false,
            chams_failed: false,
            item_logged: false,
            item_failed: false,
            hologram_failed: false,
            skeleton_failed: false,
            orbs_logged: false,
            gpu: None,
            item_gpu: None,
            last_frame: None,
            clock: 0.0,
            world: None,
            light: 0.5,
            light_at: None,
        }
    }
}
fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(State::default()))
}
pub fn start_source(id: ModuleId) -> anyhow::Result<()> {
    ensure!(
        crate::jvmti::transform_installed("SmokeWorldRender"),
        "[Smoke] world render hook unavailable (requires Fabric intermediary Minecraft 1.21.4)"
    );
    FAILED.store(false, Ordering::Relaxed);
    FAILURES.store(0, Ordering::Relaxed);
    CALLBACK_LOGGED.store(false, Ordering::Relaxed);
    {
        let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
        if id == ModuleId::Smoke {
            state.simulation = Simulation::default();
            state.effects = super::smoke_effects::Effects::default();
            state.orbs_logged = false;
        } else if id == ModuleId::Chams {
            state.chams_logged = false;
            state.chams_failed = false;
        } else if id == ModuleId::ItemChams {
            state.item_logged = false;
            state.item_failed = false;
        } else if id == ModuleId::HologramChams {
            state.hologram_failed = false;
        } else if id == ModuleId::SkeletonEsp {
            state.skeleton_failed = false;
        }
    }
    crate::camera_bridge::set_smoke_active(true)?;
    SOURCES.fetch_or(source_bit(id), Ordering::Relaxed);
    log::info!("[Smoke] {} enabled", id.display_name());
    Ok(())
}
pub fn stop_source(id: ModuleId) -> anyhow::Result<()> {
    let remaining = SOURCES.fetch_and(!source_bit(id), Ordering::Relaxed) & !source_bit(id);
    if remaining == 0 {
        return stop();
    }
    let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
    if id == ModuleId::Smoke {
        state.simulation = Simulation::default();
        state.effects = super::smoke_effects::Effects::default();
    } else if id == ModuleId::ItemChams {
        state.item_logged = false;
        state.item_failed = false;
    } else if id == ModuleId::HologramChams {
        state.hologram_failed = false;
    } else if id == ModuleId::SkeletonEsp {
        state.skeleton_failed = false;
    }
    Ok(())
}
 
 
 
 
 
 
 
 
 
 
const FAILURE_FRAMES: u32 = 120;
static FAILURES: AtomicU32 = AtomicU32::new(0);

 
 
 
pub(crate) type HoloLook = crate::module::render::hologram_chams::Config;

 
 
 
 
 
pub(super) fn resolved_style(style: i32, has_video: bool) -> i32 {
    if style == super::chams_video::STYLE && !has_video {
        0
    } else {
        style
    }
}

 
fn disable_after(streak: u32) -> bool {
    streak >= FAILURE_FRAMES
}

pub fn report_failure(error: &str) {
    if FAILED.load(Ordering::Relaxed) {
        return;
    }
    let streak = FAILURES.fetch_add(1, Ordering::Relaxed) + 1;
    if streak == 1 {
        log::warn!("[Smoke] frame skipped: {error}");
    }
    if disable_after(streak) {
        FAILED.store(true, Ordering::Relaxed);
        log::error!("[Smoke] render disabled after {streak} failed frames: {error}");
    }
}

 
pub fn report_success() {
    if FAILURES.swap(0, Ordering::Relaxed) != 0 {
        log::info!("[Smoke] render recovered");
    }
}
pub fn stop() -> anyhow::Result<()> {
    SOURCES.store(0, Ordering::Relaxed);
    crate::camera_bridge::set_smoke_active(false)?;
    let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
    state.simulation = Simulation::default();
    state.effects = super::smoke_effects::Effects::default();
    state.chams_logged = false;
    state.chams_failed = false;
    state.item_logged = false;
    state.item_failed = false;
    state.orbs_logged = false;
    state.world = None;
    state.last_frame = None;
    if let Some(mut gpu) = state.gpu.take() {
         
         
        if let Some(pass) = gpu.warped.as_mut() {
            unsafe { pass.release() }
        }
        release_gpu(gpu.vao, gpu.vbo, gpu.depth)?;
        log::info!("[Smoke] resources queued for release");
    }
    if let Some(gpu) = state.item_gpu.take() {
        release_gpu(gpu.vao, gpu.vbo, gpu.depth)?;
        log::info!("[Item Chams] isolated resources queued for release");
    }
    Ok(())
}
fn release_gpu(vao: u32, vbo: u32, depth: u32) -> anyhow::Result<()> {
    let bridge =
        crate::camera_bridge::smoke_bridge().context("Smoke bridge unavailable during cleanup")?;
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    env.call_static_method(
        class,
        "release",
        "(III)V",
        &[
            JValue::Int(vao as i32),
            JValue::Int(vbo as i32),
            JValue::Int(depth as i32),
        ],
    )?;
    Ok(())
}
 
 
pub(super) fn matrix(object: &JObject) -> anyhow::Result<[f32; 16]> {
    let bridge = crate::camera_bridge::smoke_bridge().context("Smoke bridge unavailable")?;
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    let array = JFloatArray::from(
        env.call_static_method(
            class,
            "matrix",
            "(Ljava/lang/Object;)[F",
            &[JValue::Object(object)],
        )?
        .l()?,
    );
    let mut values = [0.0; 16];
    env.get_float_array_region(array, 0, &mut values)?;
    Ok(values)
}
pub(super) fn inverse(m: &[f32; 16]) -> Option<[f32; 16]> {
    let mut a = [[0.0_f64; 8]; 4];
    for r in 0..4 {
        for c in 0..4 {
            a[r][c] = m[c * 4 + r] as f64;
        }
        a[r][r + 4] = 1.0;
    }
    for c in 0..4 {
        let pivot = (c..4).max_by(|&r, &s| a[r][c].abs().total_cmp(&a[s][c].abs()))?;
        if !a[pivot][c].is_finite() || a[pivot][c].abs() < 1e-12 {
            return None;
        }
        a.swap(c, pivot);
        let divisor = a[c][c];
        for v in &mut a[c] {
            *v /= divisor;
        }
        for r in 0..4 {
            if r != c {
                let f = a[r][c];
                for k in 0..8 {
                    a[r][k] -= f * a[c][k];
                }
            }
        }
    }
    Some(std::array::from_fn(|i| a[i % 4][i / 4 + 4] as f32))
}

pub fn render_world(camera: &JObject, view: &JObject, projection: &JObject) -> anyhow::Result<()> {
    if FAILED.load(Ordering::Relaxed) || !client().modules.is_active() {
        return Ok(());
    }
    let sources = SOURCES.load(Ordering::Relaxed);
    let cfg = client().modules.get(ModuleId::Smoke).and_then(|handle| {
        let module = handle.try_lock().ok()?;
        (sources & 1 != 0 && module.get_module_data().enabled)
            .then(|| Config::read(module.get_module_data()))
    });
    let effect_cfg = client().modules.get(ModuleId::Smoke).and_then(|handle| {
        let module = handle.try_lock().ok()?;
        cfg.as_ref()
            .map(|_| super::smoke_effects::Config::read(module.get_module_data()))
    });
     
     
     
     
     
    let (chams_cfg, chams_style, chams_glow) = client()
        .modules
        .get(ModuleId::Chams)
        .and_then(|handle| {
            let module = handle.try_lock().ok()?;
            let data = module.get_module_data();
            (sources & 4 != 0 && data.enabled).then(|| {
                (
                    super::chams::Config::read(data),
                    super::chams_warped::style_index(data) as i32,
                    super::chams_warped::is_warped(data)
                        .then(|| super::chams_warped::Settings::read(data)),
                )
            })
        })
        .map_or((None, 0, None), |(cfg, style, glow)| {
            (Some(cfg), style, glow)
        });
     
     
    let item_cfg = client()
        .modules
        .get(ModuleId::ItemChams)
        .and_then(|handle| {
            let module = handle.try_lock().ok()?;
            (sources & 64 != 0 && module.get_module_data().enabled)
                .then(|| super::chams::Config::read(module.get_module_data()))
        });
     
     
    let holo_cfg = client()
        .modules
        .get(ModuleId::HologramChams)
        .and_then(|handle| {
            let module = handle.try_lock().ok()?;
            (sources & 128 != 0 && module.get_module_data().enabled).then(|| {
                crate::module::render::hologram_chams::Config::read(module.get_module_data())
            })
        });
     
     
    let skel_cfg = client()
        .modules
        .get(ModuleId::SkeletonEsp)
        .and_then(|handle| {
             
             
             
            let module = handle.try_lock().ok()?;
            (sources & 2 != 0 && module.get_module_data().enabled).then(|| {
                crate::module::render::skeleton_esp::Config::from_data(module.get_module_data())
            })
        });
    let combat_enabled = [ModuleId::KillAura, ModuleId::MobAura, ModuleId::SilentAura]
        .into_iter()
        .any(|id| {
            client()
                .modules
                .get(id)
                .and_then(|h| h.try_lock().ok().map(|m| m.get_module_data().enabled))
                .unwrap_or(false)
        });
     
    let extra_reach = crate::module::combat::reach_distance();
    let combat_range =
        crate::module::combat::aura::HIT_RANGE as f32 + finite(extra_reach, 0.0, 0.0, 60.0);
    let explosion_on = sources & 32 != 0;
    if cfg.is_none()
        && chams_cfg.is_none()
        && item_cfg.is_none()
        && holo_cfg.is_none()
        && skel_cfg.is_none()
        && !explosion_on
    {
        return Ok(());
    }
     
    if !CALLBACK_LOGGED.swap(true, Ordering::Relaxed) {
        log::info!("[Smoke] world render callback active");
    }
    mapping().in_frame(|| {
        let mut state = state().lock().unwrap_or_else(|p| p.into_inner());
        let Some(world) = minecraft().world()? else {
            return Ok(());
        };
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
        let mut env = mapping().get_env()?;
        let changed_world = match state.world.as_ref() {
            Some(old) => !env.is_same_object(old.as_obj(), world.jni_ref().as_obj())?,
            None => true,
        };
        if changed_world {
            state.simulation = Simulation::default();
            state.effects = super::smoke_effects::Effects::default();
            state.last_frame = None;
            state.light_at = None;
            state.world = Some(world.jni_ref().clone());
        }
        let now = Instant::now();
        let paused = minecraft().call_method("isPaused", &[])?.z()?;
        let dt = if paused {
            0.0
        } else {
            state
                .last_frame
                .map(|t| now.duration_since(t).as_secs_f32().min(0.1))
                .unwrap_or(0.0)
        };
        state.last_frame = Some(now);
        state.clock += dt;
         
         
        let tick = mapping()
            .call_method(
                MinecraftClassType::Camera,
                camera,
                "getPartialTickTime",
                &[],
            )?
            .f()?;
        let pos = player
            .call_method("getPosition", &[JValue::Float(tick)])?
            .l()?;
        let pos = Vec3::read(&pos)?;
        let position = [pos.x(), pos.y(), pos.z()];
        ensure!(
            position.iter().all(|v| v.is_finite()),
            "non-finite emitter position"
        );
        if state
            .light_at
            .is_none_or(|t| now.duration_since(t).as_secs_f32() >= 0.25)
        {
            let block = BlockPos::new(
                pos.x().floor() as i32,
                (pos.y() + 0.5).floor() as i32,
                pos.z().floor() as i32,
            )
            .to_java(&mut env)?;
            let packed = mapping()
                .call_static_method(
                    MinecraftClassType::LevelRenderer,
                    "getLightColor",
                    &[
                        JValue::Object(world.jni_ref().as_obj()),
                        JValue::Object(&block),
                    ],
                )?
                .i()?;
            let sky = ((packed >> 20) & 15) as f32 / 15.0;
            let block_light = ((packed >> 4) & 15) as f32 / 15.0;
            let daylight = world
                .call_method("getSkyDarken", &[JValue::Float(0.0)])?
                .f()?
                .clamp(0.0, 1.0);
            state.light = (0.08 + 0.92 * (sky * daylight).max(block_light)).clamp(0.08, 1.0);
            state.light_at = Some(now);
        }
        let can_emit = !paused && super::input::game_window_focused() && !super::input::menu_open();
        let light = state.light;
        let mut draws = Vec::with_capacity(MAX_PUFFS * 2);
        if explosion_on {
            draws.extend(
                EXPLOSION_PUFFS
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .iter()
                    .copied(),
            );
        }
        if let (Some(cfg), Some(effects)) = (&cfg, &effect_cfg) {
            state
                .effects
                .transition(effects, combat_enabled, dt, position);
            let blend = state.effects.blend;
            let mut emitter = cfg.clone();
            emitter.density *= 1.0 - blend;
            state.simulation.step(
                &emitter,
                position,
                dt,
                super::input::key_down(cfg.spawn_key),
                can_emit,
                light,
            );
            draws.extend(state.simulation.puffs.iter().map(|p| DrawPuff {
                pos: p.pos,
                radius: p.radius(),
                seed: p.seed,
                alpha: p.alpha()
                    * (cfg.opacity * (1.0 - blend) + effects.opacity * blend)
                    * cfg.tint[3],
                light: p.light,
                tint: [cfg.tint[0], cfg.tint[1], cfg.tint[2]],
                blast: false,
            }));
            state.effects.append_global(
                effects,
                position,
                combat_range,
                cfg.size,
                light,
                cfg.tint,
                state.clock,
                &mut draws,
            );
        }
        if let Some(effects) = &effect_cfg {
            state.effects.update_orbs(effects, &draws, dt, can_emit);
            if effects.orbs && !state.orbs_logged {
                log::info!("[Smoke] orb lighting active: cap=24, per-puff=4, block light=false");
            }
            state.orbs_logged = effects.orbs;
        }
        let mut orb_draws = Vec::new();
        if let Some(cfg) = &effect_cfg {
            orb_draws.extend(
                state
                    .effects
                    .orbs
                    .iter()
                    .map(|o| o.snapshot(cfg, state.clock)),
            );
        }
        draws.retain(|p| p.alpha > 0.0 && p.radius > 0.0);
        if draws.is_empty()
            && chams_cfg.is_none()
            && item_cfg.is_none()
            && holo_cfg.is_none()
            && skel_cfg.is_none()
            && orb_draws.is_empty()
        {
            return Ok(());
        }
        let camera_pos = mapping()
            .call_method(MinecraftClassType::Camera, camera, "getPosition", &[])?
            .l()?;
        let camera_pos = Vec3::read(&camera_pos)?;
        let cam = [camera_pos.x(), camera_pos.y(), camera_pos.z()];
        let camera_detached = mapping()
            .call_method(MinecraftClassType::Camera, camera, "isDetached", &[])?
            .z()?;
         
         
         
         
        let chams_geom_cfg = chams_cfg.as_ref().map(|cfg| {
            let mut cfg = cfg.clone();
            if chams_glow.is_some() {
                cfg.strength = 1.0;
                cfg.color[3] = 1.0;
            }
            cfg
        });
        let model_vertices = chams_pass(
            &mut env,
            chams_geom_cfg.as_ref(),
            &mut state.chams_failed,
            "Chams",
            |cfg| super::chams_classic::geometry(cfg, tick, cam, camera_detached),
        )?;
        if !state.chams_logged {
             
             
             
             
            match (chams_cfg.as_ref(), model_vertices.is_empty()) {
                (Some(cfg), false) => {
                    log::info!(
                        "[Chams] actual player model pass active: cap=40, strength={:.2}, opacity={:.2}",
                        cfg.strength,
                        cfg.color[3]
                    );
                    state.chams_logged = true;
                }
                (Some(cfg), true) => {
                    log::warn!(
                        "[Chams] enabled but nothing drawn: players={} strength={:.2}, opacity={:.2}, range={:.0}",
                        cfg.players,
                        cfg.strength,
                        cfg.color[3],
                        cfg.range
                    );
                    state.chams_logged = true;
                }
                (None, _) => {}
            }
        }
         
         
         
        let item_vertices = chams_pass(
            &mut env,
            item_cfg.as_ref(),
            &mut state.item_failed,
            "Item Chams",
            |cfg| super::chams_items::geometry(cfg, tick, cam, camera_detached),
        )?;
         
         
         
        let item_atlas = if item_vertices.is_empty() {
            0
        } else {
            super::chams_items::atlas(&mut env).unwrap_or(0)
        };
        if !state.item_logged {
             
             
            match (item_cfg.as_ref(), item_vertices.is_empty()) {
                (Some(cfg), false) => {
                    log::info!(
                        "[Item Chams] held-item pass active: strength={:.2}, opacity={:.2}, atlas={}",
                        cfg.strength,
                        cfg.color[3],
                        item_atlas
                    );
                    state.item_logged = true;
                }
                (Some(cfg), true) => {
                    log::warn!(
                        "[Item Chams] enabled but nothing drawn: players={} strength={:.2}, \
                         opacity={:.2}, range={:.0}",
                        cfg.players,
                        cfg.strength,
                        cfg.color[3],
                        cfg.range
                    );
                    state.item_logged = true;
                }
                (None, _) => {}
            }
        }
         
         
         
         
         
         
        let holo_vertices = chams_pass(
            &mut env,
            holo_cfg.as_ref().map(|holo| &holo.look),
            &mut state.hologram_failed,
            "Hologram Chams",
            |look| super::chams::geometry(look, tick, cam, camera_detached),
        )?;
         
         
         
         
        let skeleton_vertices = match skel_cfg.as_ref() {
            Some(cfg) if !state.skeleton_failed => match cfg.joints(tick, cam, camera_detached) {
                Ok(joints) => crate::module::render::skeleton_esp::bone_geometry(&joints, cfg),
                Err(error) => {
                    let error = super::chams::isolated_failure(&mut env, error)?;
                    state.skeleton_failed = true;
                    log::error!("[SkeletonEsp] pass disabled (toggle to retry): {error}");
                    Vec::new()
                }
            },
            _ => Vec::new(),
        };
        let view = matrix(view)?;
        let projection = matrix(projection)?;
        let inv = inverse(&projection).context("singular projection matrix")?;
        let mut order: Vec<usize> = (0..draws.len()).collect();
        order.sort_by(|&a, &b| {
            view_depth(&draws[b].pos, &cam, &view).total_cmp(&view_depth(
                &draws[a].pos,
                &cam,
                &view,
            ))
        });
        unsafe {
             
            if !gl::GetIntegerv::is_loaded() {
                gl::load_with(super::platform::gl_proc_address);
            }
            let guard = GlGuard::capture();
            if state.gpu.is_none() {
                state.gpu = Some(Gpu::create(SHADER_NAME)?);
            }
            let State {
                gpu,
                item_gpu,
                clock,
                ..
            } = &mut *state;
            gpu.as_mut().unwrap().draw(
                &draws,
                &order,
                &cam,
                &view,
                &projection,
                &inv,
                *clock,
                &guard,
                &orb_draws,
                &model_vertices,
                &[],
                &holo_vertices,
                &skeleton_vertices,
                chams_cfg.as_ref(),
                chams_style,
                chams_glow.as_ref(),
                None,
                0,
                holo_cfg.as_ref(),
                skel_cfg.as_ref(),
            )?;
             
             
             
            if let Some(item_cfg) = item_cfg.as_ref().filter(|_| !item_vertices.is_empty()) {
                if item_gpu.is_none() {
                    *item_gpu = Some(Gpu::create("item_chams")?);
                }
                item_gpu.as_mut().unwrap().draw(
                    &[],
                    &[],
                    &cam,
                    &view,
                    &projection,
                    &inv,
                    *clock,
                    &guard,
                    &[],
                    &[],
                    &item_vertices,
                    &[],
                    &[],
                    None,
                    0,
                    None,
                    Some(item_cfg),
                    item_atlas,
                    None,
                    None,
                )?;
            }
        }
         
         
        report_success();
        Ok(())
    })
}
 
 
fn chams_pass<T: Default>(
    env: &mut jni::JNIEnv,
    cfg: Option<&super::chams::Config>,
    failed: &mut bool,
    label: &str,
    collect: impl FnOnce(&super::chams::Config) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let Some(cfg) = cfg.filter(|_| !*failed) else {
        return Ok(T::default());
    };
    match collect(cfg) {
        Ok(value) => Ok(value),
        Err(error) => {
             
             
            let error = super::chams::isolated_failure(env, error)?;
            *failed = true;
            log::error!("[{label}] pass disabled (toggle to retry): {error}");
            Ok(T::default())
        }
    }
}
 
 
 
 
 
 
 
 
 
#[allow(clippy::too_many_arguments)]
unsafe fn chams_material(
    uniforms: &[i32],
    vbo: u32,
    depth: u32,
    cfg: &super::chams::Config,
    vertices: &[f32],
    stride: usize,
    time: f32,
    cutout: i32,
    mode: i32,
    style: i32,
) {
     
     
     
     
    let video = if style == super::chams_video::STYLE {
        super::chams_video::texture_at(time)
    } else {
        None
    };
    let style = resolved_style(style, video.is_some());
    gl::ActiveTexture(gl::TEXTURE0);
    gl::BindTexture(gl::TEXTURE_2D, video.unwrap_or(depth));
    gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
    gl::Uniform1i(uniforms[18], 0);
    gl::Uniform1i(uniforms[12], mode);
    gl::Uniform1i(uniforms[17], style);
    gl::Uniform1f(uniforms[7], cfg.color[3]);
    gl::Uniform3f(uniforms[9], cfg.color[0], cfg.color[1], cfg.color[2]);
    gl::Uniform1f(
        uniforms[8],
        cfg.strength
            * if cfg.pulse {
                super::smoke_effects::glow_pulse(time)
            } else {
                1.0
            },
    );
    gl::Enable(gl::DEPTH_TEST);
    gl::DepthFunc(if cfg.through { gl::ALWAYS } else { gl::LEQUAL });
    gl::Uniform1i(uniforms[20], if cutout != 0 { 1 } else { 0 });
    gl::BufferData(
        gl::ARRAY_BUFFER,
        std::mem::size_of_val(vertices) as isize,
        vertices.as_ptr().cast(),
        gl::STREAM_DRAW,
    );
    gl::DrawArrays(gl::TRIANGLES, 0, (vertices.len() / stride.max(1)) as i32);
    gl::Disable(gl::DEPTH_TEST);
    if video.is_some() {
         
        gl::BindTexture(gl::TEXTURE_2D, depth);
    }
}

fn view_depth(pos: &[f64; 3], cam: &[f64; 3], view: &[f32; 16]) -> f32 {
    -(view[2] * (pos[0] - cam[0]) as f32
        + view[6] * (pos[1] - cam[1]) as f32
        + view[10] * (pos[2] - cam[2]) as f32
        + view[14])
}

 
 
 
 
 
 
 
 
 
 
 
 
 
pub(super) const BODY_UV_SENTINEL: f32 = -1.0;

pub(crate) const ITEM_ATLAS_UNIT: u32 = gl::TEXTURE3;
pub(crate) const ITEM_ATLAS_SAMPLER: i32 = 3;

 
 
 
 
const UV_VERTEX_BYTES: i32 = super::chams_items::VERTEX_FLOATS as i32 * 4;
 
const UV_OFFSET_BYTES: i32 = 12;

 
 
 
const CHAMS_MODE: i32 = 2;

 
 
pub(crate) const SHADER_NAME: &str = "smoke";

struct Gpu {
    program: u32,
    vao: u32,
    vbo: u32,
    depth: u32,
    dimensions: [i32; 2],
    uniforms: [i32; 21],
     
    position_attr: u32,
    uv_attr: i32,
     
     
    warped: Option<WarpedPass>,
     
     
    warped_mode: Option<usize>,
     
     
    warped_failed: bool,
     
     
    drawn_style: Option<i32>,
}
impl Gpu {
    unsafe fn create(shader_name: &str) -> anyhow::Result<Self> {
        let bridge = crate::camera_bridge::smoke_bridge().context("Smoke bridge unavailable")?;
        let mut env = mapping().get_env()?;
        let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
        let vertex = env.new_string(smoke_shader::VERTEX)?;
        let fragment = env.new_string(smoke_shader::FRAGMENT)?;
        let name = env.new_string(shader_name)?;
        let result = env.call_static_method(
            class,
            "loadShader",
            "(Ljava/lang/Object;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
            &[
                JValue::Object(minecraft().jni_ref().as_obj()),
                JValue::Object(&name),
                JValue::Object(&vertex),
                JValue::Object(&fragment),
            ],
        );
        let program = match result {
            Ok(value) => value.i()? as u32,
            Err(error) => {
                let detail = java_exception(&mut env);
                anyhow::bail!("ShaderLoader.preload failed: {error}; {detail}");
            }
        };
        ensure!(
            program != 0 && gl::IsProgram(program) != 0,
            "ShaderLoader returned invalid GL program"
        );
        let names = [
            "View",
            "Projection",
            "InverseProjection",
            "Center",
            "Radius",
            "Time",
            "Seed",
            "Alpha",
            "Light",
            "Tint",
            "SceneDepth",
            "Viewport",
            "Mode",
            "OrbCount",
            "OrbPositions[0]",
            "OrbColors[0]",
            "CameraForward",
            "Style",
            "Video",
            "ItemTex",
            "ItemCutout",
        ];
        let uniforms = std::array::from_fn(|i| {
            gl::GetUniformLocation(program, CString::new(names[i]).unwrap().as_ptr())
        });
        if uniforms.iter().any(|v| *v < 0) {
            release_gpu(0, 0, 0)?;
            anyhow::bail!("shader uniform missing: {uniforms:?}");
        }
        let location = gl::GetAttribLocation(program, c"Position".as_ptr());
        if location < 0 {
            release_gpu(0, 0, 0)?;
            anyhow::bail!("Position vertex attribute missing");
        }
        let uv_location = gl::GetAttribLocation(program, c"ItemUv".as_ptr());
        if uv_location < 0 {
            release_gpu(0, 0, 0)?;
            anyhow::bail!("ItemUv vertex attribute missing");
        }
        let mut vao = 0;
        let mut vbo = 0;
        let mut depth = 0;
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
        gl::GenTextures(1, &mut depth);
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        let vertices: [f32; 18] = [
            -1., -1., 0., 1., -1., 0., 1., 1., 0., -1., -1., 0., 1., 1., 0., -1., 1., 0.,
        ];
        gl::BufferData(
            gl::ARRAY_BUFFER,
            std::mem::size_of_val(&vertices) as isize,
            vertices.as_ptr().cast(),
            gl::STATIC_DRAW,
        );

        gl::EnableVertexAttribArray(location as u32);
        gl::VertexAttribPointer(
            location as u32,
            3,
            gl::FLOAT,
            gl::FALSE,
            12,
            std::ptr::null(),
        );
        log::info!("[Smoke] ShaderLoader ready: GLSL 150 procedural smoke");
        Ok(Self {
            program,
            vao,
            vbo,
            depth,
            dimensions: [0, 0],
            uniforms,
            position_attr: location as u32,
            uv_attr: uv_location,
            warped: None,
            warped_mode: None,
            warped_failed: false,
            drawn_style: None,
        })
    }

     
     
     
     
     
     
     
     
     
     
     
    unsafe fn use_positions(&self) {
        gl::BindVertexArray(self.vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
        gl::EnableVertexAttribArray(self.position_attr);
        gl::VertexAttribPointer(
            self.position_attr,
            3,
            gl::FLOAT,
            gl::FALSE,
            12,
            std::ptr::null(),
        );
        gl::DisableVertexAttribArray(self.uv_attr as u32);
         
         
         
        gl::VertexAttrib2f(self.uv_attr as u32, BODY_UV_SENTINEL, BODY_UV_SENTINEL);
    }

     
     
     
    unsafe fn use_positions_with_uv(&self) {
        gl::BindVertexArray(self.vao);
         
        gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
        gl::EnableVertexAttribArray(self.position_attr);
        gl::VertexAttribPointer(
            self.position_attr,
            3,
            gl::FLOAT,
            gl::FALSE,
            UV_VERTEX_BYTES,
            std::ptr::null(),
        );
        gl::EnableVertexAttribArray(self.uv_attr as u32);
        gl::VertexAttribPointer(
            self.uv_attr as u32,
            2,
            gl::FLOAT,
            gl::FALSE,
            UV_VERTEX_BYTES,
            UV_OFFSET_BYTES as *const std::ffi::c_void,
        );
    }

    unsafe fn draw(
        &mut self,
        puffs: &[DrawPuff],
        order: &[usize],
        cam: &[f64; 3],
        view: &[f32; 16],
        projection: &[f32; 16],
        inverse: &[f32; 16],
        time: f32,
        guard: &GlGuard,
        orbs: &[super::smoke_effects::DrawOrb],
        model_vertices: &[f32],
        item_vertices: &[f32],
        holo_vertices: &[f32],
        skeleton_vertices: &[f32],
        chams_cfg: Option<&super::chams::Config>,
        chams_style: i32,
        chams_glow: Option<&super::chams_warped::Settings>,
        item_cfg: Option<&super::chams::Config>,
        item_atlas: i32,
        holo_cfg: Option<&HoloLook>,
        skel_cfg: Option<&crate::module::render::skeleton_esp::Config>,
    ) -> anyhow::Result<()> {
        let [x, y, w, h] = guard.viewport;
        ensure!(
            w > 0 && h > 0 && guard.draw_fbo != 0,
            "world framebuffer/viewport unavailable"
        );
        let mut attachment_type = 0;
        gl::GetFramebufferAttachmentParameteriv(
            gl::DRAW_FRAMEBUFFER,
            gl::DEPTH_ATTACHMENT,
            gl::FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE,
            &mut attachment_type,
        );
        ensure!(
            attachment_type != gl::NONE as i32,
            "world framebuffer has no depth attachment"
        );
        let mut samples = 0;
        gl::GetIntegerv(gl::SAMPLES, &mut samples);
        ensure!(
            samples == 0,
            "multisampled world depth unsupported in this first version"
        );
        gl::BindFramebuffer(gl::READ_FRAMEBUFFER, guard.draw_fbo as u32);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindSampler(0, 0);
        gl::BindTexture(gl::TEXTURE_2D, self.depth);
        gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, 0);
        if self.dimensions != [w, h] {
            gl::CopyTexImage2D(gl::TEXTURE_2D, 0, gl::DEPTH_COMPONENT24, x, y, w, h, 0);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            self.dimensions = [w, h];
            log::info!("[Smoke] depth copy resized: {w}x{h}");
        } else {
            gl::CopyTexSubImage2D(gl::TEXTURE_2D, 0, 0, 0, x, y, w, h);
        }
         
         
        gl::Disable(gl::DEPTH_TEST);
        gl::DepthMask(gl::FALSE);
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::SCISSOR_TEST);
        gl::Disable(gl::STENCIL_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendEquationSeparate(gl::FUNC_ADD, gl::FUNC_ADD);
        gl::BlendFuncSeparate(
            gl::ONE,
            gl::ONE_MINUS_SRC_ALPHA,
            gl::ONE,
            gl::ONE_MINUS_SRC_ALPHA,
        );
        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        gl::UseProgram(self.program);
         
         
        self.use_positions();
        let u = self.uniforms;
        gl::UniformMatrix4fv(u[0], 1, gl::FALSE, view.as_ptr());
        gl::UniformMatrix4fv(u[1], 1, gl::FALSE, projection.as_ptr());
        gl::UniformMatrix4fv(u[2], 1, gl::FALSE, inverse.as_ptr());
        gl::Uniform1f(u[5], time);
        gl::Uniform1i(u[10], 0);
        gl::Uniform4f(u[11], x as f32, y as f32, w as f32, h as f32);
        gl::Uniform3f(u[16], -view[2], -view[6], -view[10]);
        gl::Uniform1i(u[13], 0);
        let mut positions = Vec::with_capacity(orbs.len() * 4);
        let mut colors = Vec::with_capacity(orbs.len() * 4);
        for orb in orbs {
            positions.extend((0..3).map(|i| (orb.pos[i] - cam[i]) as f32));
            positions.push(1.6);
            colors.extend_from_slice(&orb.color);
            colors.push(orb.intensity);
        }
         
         
        gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
         
         
         
         
         
         
         
         
         
        if let Some(holo) = holo_cfg.filter(|_| !holo_vertices.is_empty()) {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.depth);
            gl::BlendFuncSeparate(
                gl::ONE,
                gl::ONE_MINUS_SRC_ALPHA,
                gl::ONE,
                gl::ONE_MINUS_SRC_ALPHA,
            );
             
             
            let mut core = holo.look.clone();
            core.color[3] = 1.0;
            chams_material(
                &u,
                self.vbo,
                self.depth,
                &core,
                holo_vertices,
                3,
                time,
                0,
                crate::module::render::hologram_chams::HOLOGRAM_CORE_MODE,
                0,
            );
            chams_material(
                &u,
                self.vbo,
                self.depth,
                &holo.look,
                holo_vertices,
                3,
                time,
                0,
                crate::module::render::hologram_chams::HOLOGRAM_MODE,
                0,
            );
        }
         
         
         
         
         
         
         
         
         
         
         
         
         
         
         
         
        if chams_cfg.is_some() && self.drawn_style != Some(chams_style) {
            self.drawn_style = Some(chams_style);
            let name = usize::try_from(chams_style)
                .ok()
                .and_then(|index| crate::module::render::chams::STYLES.get(index))
                .copied()
                .unwrap_or("Neon Trail");
            log::info!(
                "[Chams] drawing '{name}' with {}",
                if chams_glow.is_some() {
                    "the Warped outline pipeline"
                } else {
                    "the classic material"
                }
            );
        }
        let mut glow_drawn = false;
        let legacy_material_drawn = chams_cfg.is_some()
            && chams_style >= 0
            && (chams_style as usize) < super::chams_warped::FIRST_STYLE
            && !model_vertices.is_empty();
        if legacy_material_drawn {
            if let Some(cfg) = chams_cfg {
                chams_material(
                    &u,
                    self.vbo,
                    self.depth,
                    cfg,
                    model_vertices,
                    3,
                    time,
                    0,
                    CHAMS_MODE,
                    chams_style,
                );
            }
        }
        if let (Some(cfg), Some(glow)) =
            (chams_cfg.filter(|_| !model_vertices.is_empty()), chams_glow)
        {
             
             
            if self.warped_mode != Some(glow.mode) {
                self.warped_mode = Some(glow.mode);
                self.warped_failed = false;
            }
            if !self.warped_failed {
                if self.warped.is_none() {
                    match WarpedPass::create() {
                        Ok(pass) => self.warped = Some(pass),
                        Err(error) => {
                            self.warped_failed = true;
                            log::error!(
                                "[Chams] Warped look unavailable, using the classic material: {error:#}"
                            );
                        }
                    }
                }
                if let Some(pass) = self.warped.as_mut() {
                    let frame = WarpedFrame {
                        world_fbo: guard.draw_fbo,
                        viewport: [x, y, w, h],
                        depth: self.depth,
                        view,
                        projection,
                        inverse,
                    };
                    let mut glow_settings = *glow;
                    if legacy_material_drawn {
                         
                         
                        glow_settings.fill_opacity = 0.0;
                    }
                    match pass.render(&frame, model_vertices, &glow_settings, cfg, time) {
                        Ok(()) => glow_drawn = true,
                        Err(error) => {
                            self.warped_failed = true;
                            log::error!(
                                "[Chams] Warped look stopped, using the classic material \
                                 (choose another look to retry): {error:#}"
                            );
                        }
                    }
                }
            }
        }
        if let Some(cfg) = chams_cfg
            .filter(|_| !model_vertices.is_empty() && !glow_drawn && !legacy_material_drawn)
        {
             
             
            let style = super::chams_warped::classic_style(chams_style);
            chams_material(
                &u,
                self.vbo,
                self.depth,
                cfg,
                model_vertices,
                3,
                time,
                0,
                CHAMS_MODE,
                style,
            );
        }
         
         
         
        if let Some(cfg) = item_cfg.filter(|_| !item_vertices.is_empty()) {
             
             
             
            let atlas = item_atlas;
            self.use_positions_with_uv();
             
             
             
             
             
             
            let mut atlas_unit_previous = 0;
            gl::ActiveTexture(ITEM_ATLAS_UNIT);
            gl::GetIntegerv(gl::TEXTURE_BINDING_2D, &mut atlas_unit_previous);
            gl::BindTexture(gl::TEXTURE_2D, atlas.max(0) as u32);
            gl::Uniform1i(u[19], ITEM_ATLAS_SAMPLER);
             
             
             
             
            chams_material(
                &u,
                self.vbo,
                self.depth,
                cfg,
                item_vertices,
                super::chams_items::VERTEX_FLOATS,
                time,
                atlas,
                CHAMS_MODE,
                0,
            );
             
             
             
            gl::ActiveTexture(ITEM_ATLAS_UNIT);
            gl::BindTexture(gl::TEXTURE_2D, atlas_unit_previous as u32);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.depth);
            self.use_positions();
        }
         
         
         
         
         
        if let Some(cfg) = skel_cfg.filter(|_| !skeleton_vertices.is_empty()) {
            self.use_positions_with_uv();
            chams_material(
                &u,
                self.vbo,
                self.depth,
                cfg.look(),
                skeleton_vertices,
                5,
                time,
                0,
                crate::module::render::skeleton_esp::BONE_MODE,
                0,
            );
            self.use_positions();
        }
        let quad: [f32; 18] = [
            -1., -1., 0., 1., -1., 0., 1., 1., 0., -1., -1., 0., 1., 1., 0., -1., 1., 0.,
        ];
        gl::BufferData(
            gl::ARRAY_BUFFER,
            std::mem::size_of_val(&quad) as isize,
            quad.as_ptr().cast(),
            gl::STREAM_DRAW,
        );
        gl::Uniform1i(u[12], 0);
        for &index in order {
            let p = &puffs[index];
            if view_depth(&p.pos, cam, view) < 0.05 {
                continue;
            }
            let center: [f32; 3] = std::array::from_fn(|i| (p.pos[i] - cam[i]) as f32);
             
             
            let mut local: Vec<(usize, f64)> = orbs
                .iter()
                .enumerate()
                .filter_map(|(i, o)| {
                    let d2 = o
                        .pos
                        .iter()
                        .zip(p.pos)
                        .map(|(a, b)| (a - b).powi(2))
                        .sum::<f64>();
                    (d2 < (p.radius as f64 + 1.6).powi(2)).then_some((i, d2))
                })
                .collect();
            local.sort_by(|a, b| a.1.total_cmp(&b.1));
            local.truncate(4);
            gl::Uniform1i(u[13], local.len() as i32);
            let local_positions: Vec<f32> = local
                .iter()
                .flat_map(|(i, _)| positions[i * 4..i * 4 + 4].iter().copied())
                .collect();
            let local_colors: Vec<f32> = local
                .iter()
                .flat_map(|(i, _)| colors[i * 4..i * 4 + 4].iter().copied())
                .collect();
            if !local.is_empty() {
                gl::Uniform4fv(u[14], local.len() as i32, local_positions.as_ptr());
                gl::Uniform4fv(u[15], local.len() as i32, local_colors.as_ptr());
            }
            gl::Uniform3f(u[3], center[0], center[1], center[2]);
            gl::Uniform1f(u[4], p.radius);
            gl::Uniform1f(u[6], p.seed);
            gl::Uniform1f(u[7], p.alpha);
            gl::Uniform3f(u[9], p.tint[0], p.tint[1], p.tint[2]);
            gl::Uniform1f(u[8], p.light);
            gl::Uniform1i(u[12], if p.blast { 3 } else { 0 });
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }
         
        if !orbs.is_empty() {
            gl::Uniform1i(u[12], 1);
            gl::BlendFuncSeparate(gl::ONE, gl::ONE, gl::ZERO, gl::ONE);
            for orb in orbs {
                if view_depth(&orb.pos, cam, view) < 0.05 {
                    continue;
                }
                let center: [f32; 3] = std::array::from_fn(|i| (orb.pos[i] - cam[i]) as f32);
                gl::Uniform3f(u[3], center[0], center[1], center[2]);
                gl::Uniform1f(u[4], orb.size);
                gl::Uniform1f(u[7], orb.intensity);
                gl::Uniform1f(u[8], 1.0);
                gl::Uniform3f(u[9], orb.color[0], orb.color[1], orb.color[2]);
                gl::DrawArrays(gl::TRIANGLES, 0, 6);
            }
        }
        Ok(())
    }
}

pub(super) fn java_exception(env: &mut jni::JNIEnv) -> String {
    let Ok(throwable) = env.exception_occurred() else {
        return "no Java exception detail".into();
    };
    let _ = env.exception_clear();
     
    let _ = env.call_method(&throwable, "printStackTrace", "()V", &[]);
    let mut text = throwable_text(env, &throwable);
     
     
    let mut cause = cause_of(env, &throwable);
    for _ in 0..8 {
        let Some(current) = cause else { break };
        text.push_str(" caused by ");
        text.push_str(&throwable_text(env, &current));
        cause = cause_of(env, &current);
    }
    text
}

 
fn throwable_text(env: &mut jni::JNIEnv, throwable: &jni::objects::JObject) -> String {
    let result = env
        .call_method(throwable, "toString", "()Ljava/lang/String;", &[])
        .and_then(|v| v.l());
    match result {
        Ok(object) => env
            .get_string(&jni::objects::JString::from(object))
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        Err(_) => {
            let _ = env.exception_clear();
            "unavailable Java exception detail".into()
        }
    }
}

 
fn cause_of<'local>(
    env: &mut jni::JNIEnv<'local>,
    throwable: &jni::objects::JObject<'local>,
) -> Option<jni::objects::JObject<'local>> {
    let cause = env
        .call_method(throwable, "getCause", "()Ljava/lang/Throwable;", &[])
        .and_then(|v| v.l())
        .ok()?;
     
    if cause.is_null() || env.is_same_object(&cause, throwable).unwrap_or(true) {
        return None;
    }
    Some(cause)
}

 
 
 
pub(super) struct GlGuard {
    program: i32,
    vao: i32,
    array: i32,
    unpack: i32,
    active: i32,
    texture: i32,
    sampler: i32,
    read_fbo: i32,
    draw_fbo: i32,
    viewport: [i32; 4],
    clear: [f32; 4],
    blend: [i32; 6],
    enabled: [bool; 5],
    depth_mask: u8,
    depth_func: i32,
    color_mask: [u8; 4],
}
impl GlGuard {
    pub(super) unsafe fn capture() -> Self {
        let get = |name| {
            let mut v = 0;
            gl::GetIntegerv(name, &mut v);
            v
        };
        let active = get(gl::ACTIVE_TEXTURE);
        gl::ActiveTexture(gl::TEXTURE0);
        let texture = get(gl::TEXTURE_BINDING_2D);
        let sampler = get(gl::SAMPLER_BINDING);
        gl::ActiveTexture(active as u32);
        let mut viewport = [0; 4];
        gl::GetIntegerv(gl::VIEWPORT, viewport.as_mut_ptr());
        let mut clear = [0.0; 4];
        gl::GetFloatv(gl::COLOR_CLEAR_VALUE, clear.as_mut_ptr());
        let mut color_mask = [0; 4];
        gl::GetBooleanv(gl::COLOR_WRITEMASK, color_mask.as_mut_ptr());
        let mut depth_mask = 0;
        gl::GetBooleanv(gl::DEPTH_WRITEMASK, &mut depth_mask);
        Self {
            program: get(gl::CURRENT_PROGRAM),
            vao: get(gl::VERTEX_ARRAY_BINDING),
            array: get(gl::ARRAY_BUFFER_BINDING),
            unpack: get(gl::PIXEL_UNPACK_BUFFER_BINDING),
            active,
            texture,
            sampler,
            read_fbo: get(gl::READ_FRAMEBUFFER_BINDING),
            draw_fbo: get(gl::DRAW_FRAMEBUFFER_BINDING),
            viewport,
            clear,
            blend: [
                gl::BLEND_SRC_RGB,
                gl::BLEND_DST_RGB,
                gl::BLEND_SRC_ALPHA,
                gl::BLEND_DST_ALPHA,
                gl::BLEND_EQUATION_RGB,
                gl::BLEND_EQUATION_ALPHA,
            ]
            .map(get),
            enabled: [
                gl::BLEND,
                gl::DEPTH_TEST,
                gl::CULL_FACE,
                gl::SCISSOR_TEST,
                gl::STENCIL_TEST,
            ]
            .map(|v| gl::IsEnabled(v) != 0),
            depth_mask,
            depth_func: get(gl::DEPTH_FUNC),
            color_mask,
        }
    }
}
impl GlGuard {
     
     
    pub(super) fn viewport(&self) -> [i32; 4] {
        self.viewport
    }

     
    pub(super) fn draw_fbo(&self) -> i32 {
        self.draw_fbo
    }
}

impl Drop for GlGuard {
    fn drop(&mut self) {
        unsafe {
            gl::UseProgram(self.program as u32);
            gl::BindVertexArray(self.vao as u32);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.array as u32);
            gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, self.unpack as u32);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.texture as u32);
            gl::BindSampler(0, self.sampler as u32);
            gl::ActiveTexture(self.active as u32);
            gl::BindFramebuffer(gl::READ_FRAMEBUFFER, self.read_fbo as u32);
            gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.draw_fbo as u32);
            let c = self.clear;
            gl::ClearColor(c[0], c[1], c[2], c[3]);
            let b = self.blend;
            gl::BlendFuncSeparate(b[0] as u32, b[1] as u32, b[2] as u32, b[3] as u32);
            gl::BlendEquationSeparate(b[4] as u32, b[5] as u32);
            gl::DepthMask(self.depth_mask);
            gl::DepthFunc(self.depth_func as u32);
            let c = self.color_mask;
            gl::ColorMask(c[0], c[1], c[2], c[3]);
            for (name, on) in [
                gl::BLEND,
                gl::DEPTH_TEST,
                gl::CULL_FACE,
                gl::SCISSOR_TEST,
                gl::STENCIL_TEST,
            ]
            .into_iter()
            .zip(self.enabled)
            {
                if on {
                    gl::Enable(name);
                } else {
                    gl::Disable(name);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphic::chams_items;
    fn config() -> Config {
        Config::read(&crate::module::render::smoke::SmokeModule::new().module)
    }

     
     
    #[test]
    fn a_transient_frame_failure_does_not_disable_the_render_pass() {
        assert!(FAILURE_FRAMES > 1);
        assert!(!disable_after(0));
        assert!(!disable_after(1));
        assert!(!disable_after(FAILURE_FRAMES - 1));
        assert!(disable_after(FAILURE_FRAMES));
        assert!(disable_after(FAILURE_FRAMES + 500));
    }

     
     
     
    #[test]
    fn every_classical_look_reaches_its_own_shader_branch() {
        use super::super::chams_warped::FIRST_STYLE;
        for style in 0..FIRST_STYLE as i32 {
             
             
            if style != super::super::chams_video::STYLE {
                assert_eq!(
                    resolved_style(style, false),
                    style,
                    "style {style} must draw as itself"
                );
            }
            assert_eq!(
                resolved_style(style, true),
                style,
                "style {style} must draw as itself with a clip"
            );
            assert_eq!(
                super::super::chams_warped::classic_style(style),
                style,
                "a classical style is passed through"
            );
            if style > 0 {
                assert!(
                    smoke_shader::FRAGMENT.contains(&format!("Style == {style}")),
                    "the shader has no branch for style {style}"
                );
            }
        }
         
         
        for style in FIRST_STYLE as i32..=super::super::chams_warped::LAST_STYLE as i32 {
            assert_eq!(super::super::chams_warped::classic_style(style), 0);
        }
         
         
        assert!(smoke_shader::FRAGMENT.contains("uniform sampler2D Video"));
        assert!(smoke_shader::FRAGMENT.contains("texture(Video"));
    }

     
     
    #[test]
    fn chams_video_without_a_clip_falls_back_to_the_default_look() {
        let video = super::super::chams_video::STYLE;
        assert_eq!(resolved_style(video, false), 0);
        assert_eq!(resolved_style(video, true), video);
        for style in [0, 1, 2, 3] {
            assert_eq!(resolved_style(style, false), style);
        }
    }

     
     
     
    #[test]
    fn body_geometry_is_never_cut_by_the_item_atlas() {
        assert!(BODY_UV_SENTINEL < -0.5);
        assert!(smoke_shader::FRAGMENT.contains("itemUv.x >= -0.5"));
    }

     
     
     
     
     
    #[test]
    fn the_item_pass_draws_the_five_float_layout_it_builds() {
        assert_eq!(chams_items::VERTEX_FLOATS, 5);
        assert_eq!(UV_VERTEX_BYTES, chams_items::VERTEX_FLOATS as i32 * 4);
        assert_eq!(UV_OFFSET_BYTES, 12);
         
         
         
        assert_eq!(chams_items::QUAD_FLOATS % chams_items::VERTEX_FLOATS, 0);
        assert_eq!(chams_items::QUAD_FLOATS / chams_items::VERTEX_FLOATS, 6);
         
         
        assert!(smoke_shader::VERTEX.contains("in vec2 ItemUv;"));
    }

     
     
     
     
     
    #[test]
    fn the_item_atlas_never_uses_a_unit_vanilla_samples() {
        for vanilla in [gl::TEXTURE0, gl::TEXTURE1, gl::TEXTURE2] {
            assert_ne!(ITEM_ATLAS_UNIT, vanilla);
        }
        assert_eq!(ITEM_ATLAS_UNIT, gl::TEXTURE0 + ITEM_ATLAS_SAMPLER as u32);
        assert!(ITEM_ATLAS_SAMPLER >= 3);
         
         
        assert!(smoke_shader::FRAGMENT.contains("texture(ItemTex, itemUv)"));
        assert!(smoke_shader::FRAGMENT.contains("uniform sampler2D ItemTex"));
    }

    #[test]
    fn smoke_and_chams_source_lifecycles_do_not_disable_each_other() {
        let smoke = source_bit(ModuleId::Smoke);
        let chams = source_bit(ModuleId::Chams);
        assert_eq!(smoke, 1);
        assert_eq!(chams, 4);
        assert_eq!((smoke | chams) & !smoke, chams);
        assert_eq!((smoke | chams) & !chams, smoke);
        assert_eq!(smoke & !smoke, 0);
         
         
        let items = source_bit(ModuleId::ItemChams);
        assert_eq!(items, 64);
        assert_eq!((smoke | chams | items) & !items, smoke | chams);
        assert_eq!((smoke | chams | items) & !chams, smoke | items);
         
        assert_eq!(source_bit(ModuleId::Aura), 0);
    }

    #[test]
    fn inactive_combat_effects_preserve_original_smoke_snapshots() {
        let cfg = config();
        let mut effect_cfg = super::super::smoke_effects::Config::read(
            &crate::module::render::smoke::SmokeModule::new().module,
        );
        effect_cfg.global = false;
        let mut baseline = Simulation::default();
        let mut extended = Simulation::default();
        let mut effects = super::super::smoke_effects::Effects::default();
        for i in 0..240 {
            let pos = [i as f64 / 120.0, 64.0, 0.0];
            effects.transition(&effect_cfg, true, 1.0 / 120.0, pos);
            let mut actual = cfg.clone();
            actual.density *= 1.0 - effects.blend;
            baseline.step(&cfg, pos, 1.0 / 120.0, false, true, 0.5);
            extended.step(&actual, pos, 1.0 / 120.0, false, true, 0.5);
        }
        assert_eq!(effects.blend, 0.0);
        assert_eq!(baseline.puffs.len(), extended.puffs.len());
        for (a, b) in baseline.puffs.iter().zip(&extended.puffs) {
            assert_eq!(a.pos, b.pos);
            assert_eq!(a.age, b.age);
            assert_eq!(a.radius(), b.radius());
            assert_eq!(a.alpha(), b.alpha());
        }
    }

    #[test]
    fn smoke_emission_is_time_based_bounded_and_has_no_idle_backlog() {
        let mut cfg = config();
        cfg.density = 60.;
        cfg.lifetime = 8.;
        let mut sim = Simulation::default();
        for _ in 0..200 {
            sim.step(&cfg, [0., 0., 0.], 0.05, true, true, 1.);
        }
        assert_eq!(sim.puffs.len(), MAX_PUFFS);
        assert!(sim.puffs.iter().all(|p| p.age < p.life));
        sim.step(&cfg, [0., 0., 0.], 0.1, false, true, 1.);
        assert_eq!(sim.credit, 0.);
        let mut a = Simulation::default();
        let mut b = Simulation::default();
        cfg.density = 24.;
        for _ in 0..60 {
            a.step(&cfg, [0., 0., 0.], 1. / 60., true, true, 1.);
        }
        for _ in 0..20 {
            b.step(&cfg, [0., 0., 0.], 0.05, true, true, 1.);
        }
        assert!((a.puffs.len() as i32 - b.puffs.len() as i32).abs() <= 1);
    }
    #[test]
    fn smoke_move_only_emission_is_continuous_at_render_frame_rate() {
        let mut cfg = config();
        cfg.key = false;
        let mut sim = Simulation::default();
        sim.step(&cfg, [0., 0., 0.], 0., false, true, 1.);
        for frame in 1..=120 {
             
            sim.step(
                &cfg,
                [frame as f64 / 60., 0., 0.],
                1. / 120.,
                false,
                true,
                1.,
            );
        }
        assert!((sim.puffs.len() as i32 - 24).abs() <= 1);
        assert!(sim
            .puffs
            .iter()
            .all(|p| p.origin[0] >= -0.12 && p.origin[0] <= 2.12));
        let before = sim.puffs.len();
        for _ in 0..60 {
            sim.step(&cfg, [2., 0., 0.], 1. / 120., false, true, 1.);
        }
        assert_eq!(sim.puffs.len(), before, "standing still must stop emission");
    }

    #[test]
    fn smoke_pause_menus_teleports_and_expiry_do_not_leave_trails() {
        let cfg = config();
        let mut s = Simulation::default();
        s.step(&cfg, [0., 0., 0.], 0.1, true, false, 1.);
        assert!(s.puffs.is_empty());
        s.step(&cfg, [0., 0., 0.], 0., true, true, 1.);
        assert!(s.puffs.is_empty());
        s.step(&cfg, [0., 0., 0.], 0.1, true, true, 1.);
        assert!(!s.puffs.is_empty());
        s.step(&cfg, [100., 0., 0.], 0.1, true, true, 1.);
        assert!(s.puffs.is_empty());
        s.step(&cfg, [100., 0., 0.], 0.1, true, true, 1.);
        for _ in 0..90 {
            s.step(&cfg, [100., 0., 0.], 0.1, false, false, 1.);
        }
        assert!(s.puffs.is_empty());
    }
    #[test]
    fn smoke_trajectory_and_alpha_are_smooth_and_frame_rate_independent() {
        let mut s = Simulation::default();
        s.step(&config(), [0., 0., 0.], 0.1, true, true, 1.);
        let mut a = s.puffs[0].clone();
        let mut b = a.clone();
        assert_eq!(a.alpha(), 0.);
        a.advance(1.);
        for _ in 0..10 {
            b.advance(0.1);
        }
        for i in 0..3 {
            assert!((a.pos[i] - b.pos[i]).abs() < 1e-5);
        }
        assert!(a.alpha() > 0. && a.alpha() <= 1.);
        a.age = a.life;
        assert_eq!(a.alpha(), 0.);
    }
    #[test]
    fn smoke_matrix_inverse_reconstructs_depth_and_rejects_singular() {
        let near = 0.05;
        let far = 256.;
        let f = 1.7;
        let p = [
            f,
            0.,
            0.,
            0.,
            0.,
            f,
            0.,
            0.,
            0.,
            0.,
            -(far + near) / (far - near),
            -1.,
            0.,
            0.,
            -2. * far * near / (far - near),
            0.,
        ];
        let inv = inverse(&p).unwrap();
        for depth in [0.1, 1., 10., 100.] {
            let clip_z = p[10] * -depth + p[14];
            let ndc_z = clip_z / depth;
            let z = inv[10] * ndc_z + inv[14];
            let w = inv[11] * ndc_z + inv[15];
            assert!((-z / w - depth).abs() < 0.02);
        }
        assert!(inverse(&[0.; 16]).is_none());
    }
    #[test]
    fn smoke_corrupt_settings_are_finite_and_clamped() {
        let mut module = crate::module::render::smoke::SmokeModule::new();
        module
            .module
            .get_setting_mut("Density")
            .unwrap()
            .set_slider_value(f32::NAN);
        module
            .module
            .get_setting_mut("Lifetime")
            .unwrap()
            .set_slider_value(900.);
        let cfg = Config::read(&module.module);
        assert_eq!(cfg.density, 24.);
        assert_eq!(cfg.lifetime, 8.);
    }
}
