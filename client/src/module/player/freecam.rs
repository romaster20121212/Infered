 
 
 
 
 
 

use crate::graphic::input;
use crate::mapping::client::camera::Camera;
use crate::mapping::entity::EntityRef;
use crate::mapping::math::Vec3;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

const MAX_SPEED: f64 = 50.0;
const MIN_SPEED: f64 = 0.1;
const SPRINT_MULTIPLIER: f64 = 3.0;
const DEFAULT_SPEED: f32 = 1.5;

static DEBUG_EPOCH: OnceLock<Instant> = OnceLock::new();
static DEBUG_LAST_LOG_MS: AtomicU64 = AtomicU64::new(0);
static DEBUG_STATE_LAST_LOG_MS: AtomicU64 = AtomicU64::new(0);
static FREECAM_BRIDGE_LOGGED: AtomicU64 = AtomicU64::new(0);

fn debug_now_ms() -> u64 {
    DEBUG_EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64
}

fn debug_allowed() -> bool {
    let now = debug_now_ms();
    let last = DEBUG_LAST_LOG_MS.load(Ordering::Relaxed);
    if now < last + 1000 {
        return false;
    }
    DEBUG_LAST_LOG_MS.store(now, Ordering::Relaxed);
    true
}

fn debug_state_allowed() -> bool {
    let now = debug_now_ms();
    let last = DEBUG_STATE_LAST_LOG_MS.load(Ordering::Relaxed);
    if now < last + 1000 {
        return false;
    }
    DEBUG_STATE_LAST_LOG_MS.store(now, Ordering::Relaxed);
    true
}

 
 
#[derive(Debug)]
struct FreecamState {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    sensitivity: f32,
    last_frame: Instant,
}

#[derive(Debug)]
pub struct FreecamModule {
    pub module: ModuleData,
    state: Mutex<Option<FreecamState>>,
}

impl FreecamModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Freecam,
                description: "Detached spectator camera that flies through blocks".to_string(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                 
                 
                 
                 
                 
                 
                settings: vec![ModuleSetting::Slider {
                    name: "Speed".to_string(),
                    value: DEFAULT_SPEED,
                    min: MIN_SPEED as f32,
                    max: MAX_SPEED as f32,
                }],
            },
            state: Mutex::new(None),
        }
    }

    fn speed(&self) -> f64 {
        self.module
            .get_setting("Speed")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(DEFAULT_SPEED)
            .clamp(MIN_SPEED as f32, MAX_SPEED as f32) as f64
    }
}

impl Module for FreecamModule {
    fn on_start(&self) -> anyhow::Result<()> {
        let camera = minecraft().game_renderer()?.get_main_camera()?;

         
         
         
        let (position, yaw, pitch) = match (camera.position(), camera.yaw(), camera.pitch()) {
            (Ok(position), Ok(yaw), Ok(pitch)) => (position, yaw, pitch),
            _ => {
                let player = minecraft().player()?.ok_or_else(|| {
                    anyhow::anyhow!("Freecam: no camera or local player available")
                })?;
                (
                    player.get_eye_position()?,
                    player.get_yaw()?,
                    player.get_pitch()?,
                )
            }
        };

        let sensitivity = match minecraft().options()?.sensitivity()?.get_double() {
            Ok(value) => ((value * 0.6 + 0.2) * 0.15) as f32,
            Err(_) => 0.15,
        };

        crate::camera_bridge::set_cosmetic_flag("FREECAM", true)?;
        input::freecam_drop_in();
        *self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(FreecamState {
            position,
            yaw,
            pitch: pitch.clamp(-90.0, 90.0),
            sensitivity,
            last_frame: Instant::now(),
        });
        log::info!(
            "FREECAM DEBUG enabled: position=({:.3},{:.3},{:.3}) yaw={:.3} pitch={:.3}",
            position.x(),
            position.y(),
            position.z(),
            yaw,
            pitch.clamp(-90.0, 90.0)
        );
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
         
        let _ = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        crate::camera_bridge::set_cosmetic_flag("FREECAM", false)?;
        input::freecam_drop_in();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
        Ok(())
    }

    fn on_camera_setup_object(&self, camera: &jni::objects::JObject) -> anyhow::Result<()> {
        let (position, yaw, pitch) = {
            let guard = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(state) = guard.as_ref() else {
                return Ok(());
            };
            (state.position, state.yaw, state.pitch)
        };

        let mut env = crate::state::mapping().get_env()?;
        if debug_allowed() {
            log::info!(
                "FREECAM DEBUG bridge state: enabled=true position=({:.3},{:.3},{:.3}) yaw={:.3} pitch={:.3}",
                position.x(), position.y(), position.z(), yaw, pitch
            );
        }
         
         
         
        env.set_field(camera, "field_18719", "Z", jni::objects::JValue::Bool(1))?;
        let java_position = position.to_java(&mut env)?;
        crate::state::mapping().call_method(
            crate::mapping::MinecraftClassType::Camera,
            camera,
            "setPosition",
            &[jni::objects::JValue::Object(&java_position)],
        )?;
        crate::state::mapping().call_method(
            crate::mapping::MinecraftClassType::Camera,
            camera,
            "setRotation",
            &[
                jni::objects::JValue::Float(yaw),
                jni::objects::JValue::Float(pitch),
            ],
        )?;

        if debug_allowed() {
            let camera_ref = env.new_global_ref(camera)?;
            let actual = Camera::new(camera_ref);
            match (actual.position(), actual.yaw(), actual.pitch()) {
                (Ok(actual_position), Ok(actual_yaw), Ok(actual_pitch)) => log::info!(
                    "FREECAM TRACE CAMERA: setPosition argument=({:.3},{:.3},{:.3}) setRotation argument=({:.3},{:.3}) readback=({:.3},{:.3},{:.3};{:.3},{:.3})",
                    position.x(), position.y(), position.z(), yaw, pitch,
                    actual_position.x(), actual_position.y(), actual_position.z(), actual_yaw, actual_pitch
                ),
                _ => log::warn!("FREECAM TRACE CAMERA: setters called but Camera readback failed"),
            }
        }

        if FREECAM_BRIDGE_LOGGED.fetch_add(1, Ordering::Relaxed) == 0 {
            log::info!("Freecam first successful camera override after Camera.setup");
        }
        Ok(())
    }

    fn on_camera_setup(&self) {
         
         
    }

    fn on_render(&self) {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(state) = guard.as_mut() else {
            return;
        };
         
         
         
         
         
         
         
         
         
         
        if input::menu_open() {
            input::freecam_drop_in();
            return;
        }
        step_camera(state, self.speed());
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

 
fn step_camera(state: &mut FreecamState, speed: f64) {
    let (mouse_dx, mouse_dy) = input::freecam_take_look_delta();
    let w = input::key_down(input::FREE_KEY_W);
    let a = input::key_down(input::FREE_KEY_A);
    let s = input::key_down(input::FREE_KEY_S);
    let d = input::key_down(input::FREE_KEY_D);
    let up = input::key_down(input::FREE_KEY_SPACE);
    let down = input::key_down(input::FREE_KEY_SHIFT);
    let has_input = w || a || s || d || up || down || mouse_dx != 0.0 || mouse_dy != 0.0;
    if has_input && debug_allowed() {
        log::info!(
                "FREECAM DEBUG input: W={w} A={a} S={s} D={d} UP={up} DOWN={down} mouse_dx={mouse_dx:.3} mouse_dy={mouse_dy:.3}"
            );
    }

    let yaw_before = state.yaw;
    let pitch_before = state.pitch;
    if mouse_dx != 0.0 || mouse_dy != 0.0 {
        state.yaw = (state.yaw + mouse_dx as f32 * state.sensitivity).rem_euclid(360.0);
        state.pitch = (state.pitch + mouse_dy as f32 * state.sensitivity).clamp(-90.0, 90.0);
    }

    let dt = state.last_frame.elapsed().as_secs_f64().clamp(0.001, 0.1);
    state.last_frame = Instant::now();
    let speed = speed
        * if input::key_down(input::FREE_KEY_CTRL) {
            SPRINT_MULTIPLIER
        } else {
            1.0
        };

    let yaw_rad = (state.yaw as f64).to_radians();
    let (forward_x, forward_z) = (-yaw_rad.sin(), yaw_rad.cos());
    let (right_x, right_z) = (-yaw_rad.cos(), -yaw_rad.sin());
    let mut dx = 0.0;
    let mut dz = 0.0;
    if w {
        dx += forward_x;
        dz += forward_z;
    }
    if s {
        dx -= forward_x;
        dz -= forward_z;
    }
    if d {
        dx += right_x;
        dz += right_z;
    }
    if a {
        dx -= right_x;
        dz -= right_z;
    }

    let horizontal = (dx * dx + dz * dz).sqrt();
    if horizontal > 0.0 {
        let scale = speed * dt / horizontal;
        dx *= scale;
        dz *= scale;
    }
    let vertical = if up {
        speed * dt
    } else if down {
        -speed * dt
    } else {
        0.0
    };

    let before = state.position;
    state.position = Vec3::new(
        state.position.x() + dx,
        state.position.y() + vertical,
        state.position.z() + dz,
    );
    if has_input && debug_state_allowed() {
        log::info!(
            "FREECAM DEBUG state: position_before=({:.3},{:.3},{:.3}) position_after=({:.3},{:.3},{:.3}) yaw_before={yaw_before:.3} yaw_after={:.3} pitch_before={pitch_before:.3} pitch_after={:.3}",
            before.x(), before.y(), before.z(),
            state.position.x(), state.position.y(), state.position.z(),
            state.yaw, state.pitch
        );
    }
}
