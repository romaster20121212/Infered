 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::EndCrystal;
use crate::mapping::entity::{Entity, EntityRef, LivingEntityRef};
use crate::mapping::math::Vec3;
use crate::mapping::MappedObject;
use crate::module::combat::moving_for_wire;
use crate::module::combat::rotation::Rotation;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::net::packet::{Packet, PacketAction};
use crate::state::{client, minecraft};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

 
 
const HIT_WINDOW_LEN: Duration = Duration::from_millis(250);
const HIT_WINDOW_PACKETS: u8 = 3;

 
static HIT_WINDOW: Mutex<Option<(u8, Instant)>> = Mutex::new(None);

 
 
 
static AIM_PENDING: AtomicBool = AtomicBool::new(false);

 
 
 
fn open_hit_window(now: Instant) {
    let mut window = HIT_WINDOW
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *window = Some((HIT_WINDOW_PACKETS, now + HIT_WINDOW_LEN));
    AIM_PENDING.store(true, Ordering::Release);
}

fn close_hit_window() {
    let mut window = HIT_WINDOW
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *window = None;
}

fn take_hit_packet(now: Instant) -> bool {
    let mut window = HIT_WINDOW
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    spend_hit_window(&mut window, now)
}

 
fn spend_hit_window(window: &mut Option<(u8, Instant)>, now: Instant) -> bool {
    match window {
        Some((left, closes)) if now <= *closes && *left > 0 => {
            *left -= 1;
            true
        }
        _ => false,
    }
}

 
fn hit_action(allowed: bool) -> PacketAction {
    if allowed {
        PacketAction::Forward
    } else {
        PacketAction::Cancel
    }
}

 
 
 
 

 
 
const DEFAULT_NEW_TARGET_WINDOW: Duration = Duration::from_secs(4);

 
const DEFAULT_BREAK_RETRY: Duration = Duration::from_millis(180);

 
 
const DEFAULT_BREAK_TIMEOUT: Duration = Duration::from_millis(900);

 
 
const DEFAULT_TARGET_TIMEOUT: Duration = Duration::from_secs(2);

 
const DEFAULT_MAX_BREAK_ATTEMPTS: u32 = 3;

 
 
const DEFAULT_PICK_SETTLE: Duration = Duration::from_millis(250);

 
 
const VANILLA_REACH: f64 = 3.0;

 
 
 
 
const REACH_MODIFIER_ID: &str = "infered:autocrystal_reach";

 
 
const REACH_ATTRIBUTE: &str = "ENTITY_INTERACTION_RANGE";

 
 
 
 
 
 

 
 
 
const SWEEP_FOLDS: f32 = 3.7;

 
 
 
const SNAP_RATE: f32 = 400.0;

 
 
const MIN_SWEEP_RATE: f32 = 1.0;

 
 
 
fn sweep_rate(duration_ms: f32) -> f32 {
    if duration_ms <= 0.0 {
        return SNAP_RATE;
    }
    (SWEEP_FOLDS * 1000.0 / duration_ms).clamp(MIN_SWEEP_RATE, SNAP_RATE)
}

 
 
 
fn missing_reach(want: f64, live: f64) -> f64 {
    (want - live).max(0.0)
}

 
 
 
 
 
const MAX_SEQUENCE_STEPS: usize = 8;

 
 
fn retry_ready(
    last_attack: Instant,
    attempts: u32,
    now: Instant,
    retry: Duration,
    max_attempts: u32,
) -> bool {
    attempts < max_attempts && now.saturating_duration_since(last_attack) >= retry
}

fn crystal_aim_point(eye: Vec3, feet: Vec3, width: f64, height: f64) -> Vec3 {
    let half = width * 0.5;
    let inset = 0.03_f64.min(half * 0.5).min(height * 0.25);
    Vec3::new(
        eye.x()
            .clamp(feet.x() - half + inset, feet.x() + half - inset),
        eye.y().clamp(feet.y() + inset, feet.y() + height - inset),
        eye.z()
            .clamp(feet.z() - half + inset, feet.z() + half - inset),
    )
}

 
fn delay_elapsed(started_at: Instant, delay_ms: u32, jitter_ms: i32, now: Instant) -> bool {
    let delay_ms = (delay_ms as i64 + jitter_ms as i64).max(0) as u64;
    now.saturating_duration_since(started_at) >= Duration::from_millis(delay_ms)
}

 
fn click_jitter_ms(max_ms: u32, target_id: i32) -> i32 {
    if max_ms == 0 {
        return 0;
    }
    let time_seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let mut value = time_seed ^ (target_id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 12;
    value ^= value << 25;
    value ^= value >> 27;
    let width = (max_ms as u64).saturating_mul(2).saturating_add(1);
    (value.wrapping_mul(0x2545_F491_4F6C_DD1D) % width) as i32 - max_ms as i32
}

fn click_due(started_at: Option<Instant>, delay_ms: u32, jitter_ms: i32, now: Instant) -> bool {
    started_at.is_some_and(|start| delay_elapsed(start, delay_ms, jitter_ms, now))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyAimStatus {
    Ready,
    Turning,
    Missed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    ClickTimer,
    Click,
    Confirming,
    Returning,
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Idle
    }
}

#[derive(Debug, Default)]
struct AutoCrystalState {
    phase: Phase,
    target: Option<i32>,
     
     
    target_entity: Option<Entity>,
     
    locked_at: Option<Instant>,
     
    click_timer_at: Option<Instant>,
     
    click_jitter_ms: i32,
     
    click_release_at: Option<Instant>,
     
    click_held: bool,
     
     
    single_target_completed: bool,
     
    return_rotation: Option<Rotation>,
     
    user_cursor_at_attack: Option<u64>,
     
     
    range: f64,
     
     
    entities: Vec<Entity>,
     
    first_seen: HashMap<i32, Instant>,
    last_attack: Option<Instant>,
    first_attack: Option<Instant>,
    attempts: u32,
    returning_at: Option<Instant>,
    retry_after: HashMap<i32, Instant>,
}

 
fn begin_return(state: &mut AutoCrystalState, now: Instant) {
    state.phase = Phase::Returning;
    state.returning_at = Some(now);
    state.click_timer_at = None;
}
 
fn reset_sequence(state: &mut AutoCrystalState) {
    state.phase = Phase::Idle;
    state.target = None;
    state.target_entity = None;
    state.locked_at = None;
    state.click_timer_at = None;
    state.click_jitter_ms = 0;
    if !state.click_held {
        state.click_release_at = None;
    }
    state.return_rotation = None;
    state.user_cursor_at_attack = None;
    state.last_attack = None;
    state.first_attack = None;
    state.attempts = 0;
    state.returning_at = None;
}

 
 
#[derive(Debug)]
pub struct AutoCrystalModule {
    pub module: ModuleData,
    state: Mutex<AutoCrystalState>,
     
     
    reach_applied: Mutex<Option<(i32, f64)>>,
}

impl AutoCrystalModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoCrystal,
                description: "Manually aims at and clicks visible end crystals".to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                     
                     
                    ModuleSetting::Slider {
                        name: "Attack Delay (ms)".to_string(),
                        value: 10.0,
                        min: 0.0,
                        max: 200.0,
                    },
                    ModuleSetting::Slider {
                        name: "Click Randomization (ms)".to_string(),
                        value: 0.0,
                        min: 0.0,
                        max: 100.0,
                    },
                    ModuleSetting::Slider {
                        name: "Click Hold (ms)".to_string(),
                        value: 12.0,
                        min: 0.0,
                        max: 100.0,
                    },
                     
                     
                    ModuleSetting::Slider {
                        name: "Aim Time (ms)".to_string(),
                        value: 50.0,
                        min: 0.0,
                        max: 400.0,
                    },
                    ModuleSetting::Slider {
                        name: "Aim Tolerance (blocks)".to_string(),
                        value: 0.5,
                        min: 0.0,
                        max: 2.0,
                    },
                     
                     
                     
                     
                    ModuleSetting::Toggle {
                        name: "Silent Aim".to_string(),
                        value: false,
                    },
                    ModuleSetting::Toggle {
                        name: "Multi Target".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Return to View".to_string(),
                        value: true,
                    },
                     
                     
                    ModuleSetting::Slider {
                        name: "Return Time (ms)".to_string(),
                        value: 150.0,
                        min: 0.0,
                        max: 400.0,
                    },
                     
                     
                     
                     
                     
                    ModuleSetting::Slider {
                        name: "Break Retry (ms)".to_string(),
                        value: DEFAULT_BREAK_RETRY.as_millis() as f32,
                        min: 0.0,
                        max: 1000.0,
                    },
                     
                     
                    ModuleSetting::Slider {
                        name: "Break Timeout (ms)".to_string(),
                        value: DEFAULT_BREAK_TIMEOUT.as_millis() as f32,
                        min: 100.0,
                        max: 5000.0,
                    },
                     
                     
                     
                    ModuleSetting::Slider {
                        name: "Target Timeout (ms)".to_string(),
                        value: DEFAULT_TARGET_TIMEOUT.as_millis() as f32,
                        min: 200.0,
                        max: 10000.0,
                    },
                     
                    ModuleSetting::Slider {
                        name: "Retry Attempts".to_string(),
                        value: DEFAULT_MAX_BREAK_ATTEMPTS as f32,
                        min: 1.0,
                        max: 10.0,
                    },
                     
                     
                    ModuleSetting::Slider {
                        name: "New Target Window (ms)".to_string(),
                        value: DEFAULT_NEW_TARGET_WINDOW.as_millis() as f32,
                        min: 0.0,
                        max: 20000.0,
                    },
                     
                     
                    ModuleSetting::Slider {
                        name: "Aim Settle (ms)".to_string(),
                        value: DEFAULT_PICK_SETTLE.as_millis() as f32,
                        min: 0.0,
                        max: 1000.0,
                    },
                     
                     
                     
                     
                    ModuleSetting::Slider {
                        name: "Range".to_string(),
                        value: 4.5,
                        min: 1.0,
                        max: 4.5,
                    },
                    ModuleSetting::Choice {
                        name: "Priority".to_string(),
                        value: 1,
                        options: vec!["Nearest".to_string(), "Newly Placed".to_string()],
                    },
                    ModuleSetting::Toggle {
                        name: "Debug Mode".to_string(),
                        value: false,
                    },
                ],
            },
            state: Mutex::new(AutoCrystalState::default()),
            reach_applied: Mutex::new(None),
        }
    }

    fn slider(&self, name: &str, fallback: f32) -> f32 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(fallback)
    }

     
    fn delay_ms(&self) -> u32 {
        self.slider("Attack Delay (ms)", 10.0).clamp(0.0, 200.0) as u32
    }

     
    fn aim_ms(&self) -> u32 {
        self.slider("Aim Time (ms)", 50.0).clamp(0.0, 400.0) as u32
    }

     
    fn return_ms(&self) -> u32 {
        self.slider("Return Time (ms)", 150.0).clamp(0.0, 400.0) as u32
    }

     
    fn break_retry(&self) -> Duration {
        Duration::from_millis(self.slider("Break Retry (ms)", 180.0).clamp(0.0, 1000.0) as u64)
    }

     
    fn break_timeout(&self) -> Duration {
        Duration::from_millis(
            self.slider("Break Timeout (ms)", 900.0)
                .clamp(100.0, 5000.0) as u64,
        )
    }

     
     
    fn target_timeout(&self) -> Duration {
        Duration::from_millis(
            self.slider("Target Timeout (ms)", 2000.0)
                .clamp(200.0, 10000.0) as u64,
        )
    }

     
    fn max_attempts(&self) -> u32 {
        self.slider("Retry Attempts", DEFAULT_MAX_BREAK_ATTEMPTS as f32)
            .clamp(1.0, 10.0) as u32
    }

     
    fn new_target_window(&self) -> Duration {
        Duration::from_millis(
            self.slider("New Target Window (ms)", 4000.0)
                .clamp(0.0, 20000.0) as u64,
        )
    }

     
    fn pick_settle(&self) -> Duration {
        Duration::from_millis(self.slider("Aim Settle (ms)", 250.0).clamp(0.0, 1000.0) as u64)
    }

     
     
     
    fn log_walk_sync_probe(&self, player: &LocalPlayer) {
        let Some(aim) = crate::module::combat::rotation::silent_rotation() else {
            return;
        };
        let moving = [
            crate::graphic::input::FREE_KEY_W,
            crate::graphic::input::FREE_KEY_A,
            crate::graphic::input::FREE_KEY_S,
            crate::graphic::input::FREE_KEY_D,
        ]
        .into_iter()
        .any(crate::graphic::input::key_down);
        if !moving {
            return;
        }
        let (Ok(body_yaw), Ok(camera_yaw)) = (
            player.get_yaw(),
            minecraft()
                .game_renderer()
                .and_then(|renderer| renderer.get_main_camera())
                .and_then(|camera| camera.yaw()),
        ) else {
            return;
        };
        log::info!(
            "[walk-sync probe] body_yaw={body_yaw:.2} camera_yaw={camera_yaw:.2} crystal_yaw={:.2}",
            aim.yaw
        );
    }

     
     
    fn silent(&self) -> bool {
        self.module
            .get_setting("Silent Aim")
            .and_then(ModuleSetting::get_toggle_value)
            .unwrap_or(false)
    }

     
    fn range_slider(&self) -> f32 {
        self.slider("Range", 4.5).clamp(1.0, 4.5)
    }

     
     
     
     
     
     
     
     
     
     
    fn sync_reach(&self, player: &LocalPlayer) -> anyhow::Result<()> {
        let id = player.id()?;
         
         
        let live = player
            .get_attribute_value(REACH_ATTRIBUTE)?
            .unwrap_or(VANILLA_REACH)
            .max(0.0);
        let own = match *self.reach_applied.lock().unwrap() {
            Some((applied_id, amount)) if applied_id == id => amount,
            _ => 0.0,
        };
        let extra = missing_reach(self.range_slider() as f64, live - own);
        if (extra - own).abs() < 1e-3 {
            return Ok(());
        }
        if extra > 0.0 {
            player.add_attribute_modifier(REACH_ATTRIBUTE, REACH_MODIFIER_ID, extra)?;
        } else {
            player.remove_attribute_modifier(REACH_ATTRIBUTE, REACH_MODIFIER_ID)?;
        }
        *self.reach_applied.lock().unwrap() = Some((id, extra));
        Ok(())
    }

     
    fn clear_reach(&self) {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.remove_attribute_modifier(REACH_ATTRIBUTE, REACH_MODIFIER_ID);
        }
        *self.reach_applied.lock().unwrap() = None;
    }

    fn priority_new(&self) -> bool {
        self.module
            .get_setting("Priority")
            .and_then(|setting| match setting {
                ModuleSetting::Choice { value, options, .. } => {
                    Some(*value == 1 && options.get(1).is_some_and(|o| o == "Newly Placed"))
                }
                _ => None,
            })
            .unwrap_or(false)
    }

    fn click_randomization_ms(&self) -> u32 {
        self.slider("Click Randomization (ms)", 0.0)
            .clamp(0.0, 100.0) as u32
    }

    fn click_hold_ms(&self) -> u32 {
        self.slider("Click Hold (ms)", 12.0).clamp(0.0, 100.0) as u32
    }

    fn aim_tolerance(&self) -> f64 {
        self.slider("Aim Tolerance (blocks)", 0.5).clamp(0.0, 2.0) as f64
    }

    fn multi_target(&self) -> bool {
        self.module
            .get_setting("Multi Target")
            .and_then(ModuleSetting::get_toggle_value)
            .unwrap_or(true)
    }

    fn return_to_view(&self) -> bool {
        self.module
            .get_setting("Return to View")
            .and_then(ModuleSetting::get_toggle_value)
            .unwrap_or(true)
    }

     
     
    fn body_view_on_target(
        &self,
        player: &LocalPlayer,
        target: &Entity,
        max_distance: f64,
    ) -> bool {
        let Ok(eye) = player.get_eye_position() else {
            return false;
        };
        let (Ok(yaw), Ok(pitch)) = (player.get_yaw(), player.get_pitch()) else {
            return false;
        };
        let Ok((min, max)) = super::target_box(target) else {
            return false;
        };
        let ray_hits = super::ray_hits_box(
            [eye.x(), eye.y(), eye.z()],
            super::look_dir(yaw, pitch),
            min,
            max,
            max_distance + self.aim_tolerance(),
        );
        let Some(interaction_reach) = player
            .get_attribute_value(REACH_ATTRIBUTE)
            .ok()
            .flatten()
            .filter(|reach| reach.is_finite() && *reach >= 0.0)
        else {
            return false;
        };
        ray_hits
            && super::point_box_distance([eye.x(), eye.y(), eye.z()], min, max) <= interaction_reach
    }

     
     
     
    fn check_body_aim(
        &self,
        player: &LocalPlayer,
        target: &Entity,
        max_distance: f64,
    ) -> BodyAimStatus {
        let ray_hits_target = self.body_view_on_target(player, target, max_distance);
        if ray_hits_target {
            let picked_target = target
                .id()
                .ok()
                .zip(minecraft().crosshair_entity_id().ok().flatten())
                .is_some_and(|(target_id, picked_id)| target_id == picked_id);
            if picked_target {
                return BodyAimStatus::Ready;
            }
            let pick_still_updating = self.state.lock().ok().is_some_and(|state| {
                state
                    .locked_at
                    .is_some_and(|at| at.elapsed() < self.pick_settle())
            });
            if pick_still_updating {
                return BodyAimStatus::Turning;
            }
        }
        let Ok(desired) = (|| {
            let eye = player.get_eye_position()?;
            let feet = target.get_position()?;
            let width = target.bb_width()? as f64;
            let height = target.bb_height()? as f64;
            let point = crystal_aim_point(eye, feet, width, height);
            Ok::<Rotation, anyhow::Error>(Rotation::towards(eye, point))
        })() else {
            return BodyAimStatus::Turning;
        };
        let current = match (player.get_yaw(), player.get_pitch()) {
            (Ok(yaw), Ok(pitch)) => Rotation::new(yaw, pitch),
            _ => return BodyAimStatus::Turning,
        };
        let angle_error = current.angle_to(desired);
        if angle_error <= 1.5 {
            return if ray_hits_target {
                BodyAimStatus::Ready
            } else {
                BodyAimStatus::Missed
            };
        }
        BodyAimStatus::Turning
    }

     
     
    fn aim_rotation(player: &LocalPlayer, target: &Entity) -> anyhow::Result<Rotation> {
        let eye = player.get_eye_position()?;
        let feet = target.get_position()?;
        let width = target.bb_width()? as f64;
        let height = target.bb_height()? as f64;
        Ok(Rotation::towards(
            eye,
            crystal_aim_point(eye, feet, width, height),
        ))
    }

     
     
     
    fn silent_aim_on_target(
        &self,
        player: &LocalPlayer,
        target: &Entity,
        max_distance: f64,
    ) -> bool {
        let Some(aim) = crate::module::combat::rotation::silent_rotation() else {
            return false;
        };
        let Ok(eye) = player.get_eye_position() else {
            return false;
        };
        let Ok((min, max)) = super::target_box(target) else {
            return false;
        };
        let Ok(feet) = target.get_position() else {
            return false;
        };
        let origin = [eye.x(), eye.y(), eye.z()];
        let dir = super::look_dir(aim.yaw, aim.pitch);
        let max_dist = max_distance + self.aim_tolerance();
         
         
        let on_crystal = super::ray_hits_box(origin, dir, min, max, max_dist)
            || super::ray_hits_support_top(origin, dir, [feet.x(), feet.y(), feet.z()], max_dist);
        on_crystal
            && super::point_box_distance(origin, min, max) <= super::interaction_reach(player)
    }

     
     
     
    fn check_silent_aim(
        &self,
        player: &LocalPlayer,
        target: &Entity,
        max_distance: f64,
    ) -> BodyAimStatus {
        if self.silent_aim_on_target(player, target, max_distance) {
            return BodyAimStatus::Ready;
        }
        let Ok(desired) = Self::aim_rotation(player, target) else {
            return BodyAimStatus::Turning;
        };
        let Some(aim) = crate::module::combat::rotation::silent_rotation() else {
            return BodyAimStatus::Turning;
        };
        if aim.angle_to(desired) <= 1.5 {
            BodyAimStatus::Missed
        } else {
            BodyAimStatus::Turning
        }
    }

     
    fn check_aim(&self, player: &LocalPlayer, target: &Entity, max_distance: f64) -> BodyAimStatus {
        if self.silent() {
            self.check_silent_aim(player, target, max_distance)
        } else {
            self.check_body_aim(player, target, max_distance)
        }
    }

     
     
    fn aim(&self, desired: Rotation, duration: u32) {
        if self.silent() {
             
             
            crate::module::combat::rotation::aim_silent(desired, SNAP_RATE);
        } else {
            crate::module::combat::rotation::aim_real_mouse(desired, sweep_rate(duration as f32));
        }
    }

     
     
     
    fn claim_camera(&self) {
        if self.silent() {
            crate::module::misc::freelook::release_body_view();
            return;
        }
        crate::module::misc::freelook::request_body_view();
    }

     
     
     
    fn user_took_the_mouse(&self) -> bool {
        if self.silent() {
            return false;
        }
        self.state.lock().ok().is_some_and(|state| {
            state.phase != Phase::Idle
                && state.user_cursor_at_attack.is_some_and(|at| {
                    crate::graphic::input::USER_CURSOR_MOVED
                        .load(std::sync::atomic::Ordering::Relaxed)
                        != at
                })
        })
    }

    fn release_click_if_due(&self, now: Instant) {
        let should_release = self.state.lock().ok().is_some_and(|state| {
            state.click_held && state.click_release_at.is_some_and(|at| now >= at)
        });
        if should_release && crate::graphic::platform::left_mouse_up() {
            if let Ok(mut state) = self.state.lock() {
                state.click_held = false;
                state.click_release_at = None;
                state.last_attack = Some(now);
                state.first_attack.get_or_insert(now);
                state.attempts += 1;
                state.phase = Phase::Confirming;
            }
        }
    }

    fn release_click(&self) {
        let held = self.state.lock().ok().is_some_and(|state| state.click_held);
        if held && crate::graphic::platform::left_mouse_up() {
            if let Ok(mut state) = self.state.lock() {
                state.click_held = false;
                state.click_release_at = None;
            }
        }
    }

     
     
     
     
     
    fn silent_attack(
        &self,
        player: &LocalPlayer,
        target: &Entity,
        entities: &[Entity],
        range: f64,
        now: Instant,
    ) {
        let Ok(Some(game_mode)) = minecraft().game_mode() else {
            return;
        };
        open_hit_window(now);
        if game_mode.attack(player, target).is_err() {
            close_hit_window();
            return;
        }
        let _ = player.swing();

        let fired = if let Ok(mut state) = self.state.lock() {
             
             
            if state.phase == Phase::ClickTimer {
                state.target_entity = Some(target.clone());
                state.last_attack = Some(now);
                state.first_attack.get_or_insert(now);
                state.attempts += 1;
                state.phase = Phase::Confirming;
                true
            } else {
                false
            }
        } else {
            false
        };
        if !fired {
            return;
        }
        if self.debug() {
            log::info!(
                "auto crystal: silent attack on entity {:?}",
                target.id().unwrap_or_default()
            );
        }
        self.step_sequence(player, entities, range, now);
    }

    fn input_available(&self) -> bool {
        crate::graphic::input::game_window_focused()
            && !crate::graphic::input::menu_open()
            && !minecraft().current_screen().is_open()
             
             
             
            && (self.silent()
                || !crate::module::misc::freelook::owns_mouse()
                || crate::module::misc::freelook::body_view_requested())
            && !client()
                .modules
                .get(ModuleId::Freecam)
                .map(|handle| {
                    handle
                        .try_lock()
                        .map(|module| module.get_module_data().enabled)
                        .unwrap_or(true)
                })
                .unwrap_or(false)
    }

    fn debug(&self) -> bool {
        self.module
            .get_setting("Debug Mode")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(false)
    }

     
     
     
     
    fn select_target(
        &self,
        entities: &[Entity],
        player: &LocalPlayer,
        anchor: Vec3,
        range: f64,
        first_seen: &mut HashMap<i32, Instant>,
        now: Instant,
        excluded: Option<i32>,
        retry_after: &HashMap<i32, Instant>,
    ) -> Option<(i32, Entity)> {
        let mut candidates: Vec<(i32, Entity, f64)> = Vec::new();
        let mut loaded = HashSet::new();
        let candidate_range = range + self.aim_tolerance();
        for crystal in entities {
            if !crystal.instance_of::<EndCrystal>() {
                continue;
            }
            let Ok(id) = crystal.id() else {
                continue;
            };
            loaded.insert(id);
            first_seen.entry(id).or_insert(now);
            if excluded == Some(id) || retry_after.get(&id).is_some_and(|until| now < *until) {
                continue;
            }
            if crystal
                .call_method("isRemoved", &[])
                .and_then(|value| Ok(value.z()?))
                .unwrap_or(true)
            {
                continue;
            }
            let Ok((min, max)) = super::target_box(crystal) else {
                continue;
            };
            let distance =
                super::point_box_distance([anchor.x(), anchor.y(), anchor.z()], min, max);
            if !distance.is_finite() || distance > candidate_range {
                continue;
            }
            let visible = player
                .call_method(
                    "hasLineOfSight",
                    &[jni::objects::JValue::Object(crystal.jni_ref().as_obj())],
                )
                .and_then(|value| Ok(value.z()?))
                .unwrap_or(false);
            if !visible {
                continue;
            }
            candidates.push((id, crystal.clone(), distance));
        }
         
         
        first_seen.retain(|id, _| loaded.contains(id));
        if candidates.is_empty() {
            return None;
        }

        if self.priority_new() {
            if let Some((id, crystal)) = candidates
                .iter()
                .filter_map(|(id, crystal, _)| {
                    let first = *first_seen.get(id)?;
                    (now - first <= self.new_target_window()).then_some((
                        *id,
                        crystal.clone(),
                        first,
                    ))
                })
                .max_by(|a, b| a.2.cmp(&b.2))
                .map(|(id, crystal, _)| (id, crystal))
            {
                return Some((id, crystal));
            }
        }
        candidates
            .into_iter()
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(id, crystal, _)| (id, crystal))
    }

     
    fn cancel_sequence(&self) {
        self.release_click();
         
         
        AIM_PENDING.store(false, Ordering::Release);
        close_hit_window();
        if let Ok(mut state) = self.state.lock() {
            let controlled_camera = state.phase != Phase::Idle;
            reset_sequence(&mut state);
            if controlled_camera {
                crate::module::combat::rotation::clear();
            }
        }
        crate::module::misc::freelook::release_body_view();
    }

     
     
     
     
     
     
    fn step_sequence(&self, player: &LocalPlayer, entities: &[Entity], range: f64, now: Instant) {
        let Ok(feet) = player.get_eye_position() else {
            return;
        };
         
         
         
        if self.user_took_the_mouse() {
            self.cancel_sequence();
            return;
        }
         
         
         
        for _ in 0..MAX_SEQUENCE_STEPS {
            let phase = match self.state.lock() {
                Ok(state) => state.phase,
                Err(_) => return,
            };
            match phase {
                Phase::Idle => {
                     
                     
                    let paused = self
                        .state
                        .lock()
                        .ok()
                        .is_some_and(|state| state.single_target_completed);
                    if paused {
                        let remaining = entities.iter().any(|entity| {
                            if !entity.instance_of::<EndCrystal>()
                                || entity
                                    .call_method("isRemoved", &[])
                                    .and_then(|value| Ok(value.z()?))
                                    .unwrap_or(true)
                            {
                                return false;
                            }
                            super::target_box(entity).is_ok_and(|(min, max)| {
                                super::point_box_distance([feet.x(), feet.y(), feet.z()], min, max)
                                    <= range + self.aim_tolerance()
                            })
                        });
                        if remaining {
                            return;
                        }
                        if let Ok(mut state) = self.state.lock() {
                            state.single_target_completed = false;
                        }
                    }

                     
                     
                    let locked = {
                        let Ok(mut state) = self.state.lock() else {
                            return;
                        };
                        state.retry_after.retain(|_, until| now < *until);
                        let retry_after = state.retry_after.clone();
                        self.select_target(
                            entities,
                            player,
                            feet,
                            range,
                            &mut state.first_seen,
                            now,
                            None,
                            &retry_after,
                        )
                        .map(|(target_id, target)| {
                            self.claim_camera();
                            state.target = Some(target_id);
                            state.target_entity = Some(target.clone());
                            state.locked_at = Some(now);
                            state.click_timer_at = Some(now);
                            state.click_jitter_ms =
                                click_jitter_ms(self.click_randomization_ms(), target_id);
                            state.phase = Phase::ClickTimer;
                            (target_id, target)
                        })
                    };
                    let Some((target_id, _)) = locked else {
                        return;
                    };
                     
                     
                    let needs_original_view = self
                        .state
                        .lock()
                        .map(|state| state.return_rotation.is_none())
                        .unwrap_or(false);
                    let saved_view = if needs_original_view {
                        let (Ok(yaw), Ok(pitch)) = (player.get_yaw(), player.get_pitch()) else {
                            return;
                        };
                        Some(Rotation::new(yaw, pitch))
                    } else {
                        None
                    };
                    if let Ok(mut state) = self.state.lock() {
                        if let Some(view) = saved_view {
                            state.return_rotation = Some(view);
                            state.user_cursor_at_attack = Some(
                                crate::graphic::input::USER_CURSOR_MOVED
                                    .load(std::sync::atomic::Ordering::Relaxed),
                            );
                        }
                        state.phase = Phase::ClickTimer;
                    }
                    if self.debug() {
                        log::info!("auto crystal: locked entity {target_id}");
                    }
                     
                     
                    self.aim_at_target(player, entities, feet, range);
                    return;
                }
                Phase::ClickTimer => {
                    let expired = self.state.lock().ok().is_some_and(|state| {
                        state.first_attack.is_some_and(|at| {
                            now.saturating_duration_since(at) >= self.break_timeout()
                        })
                    });
                    if expired {
                        let break_timeout = self.break_timeout();
                        if let Ok(mut state) = self.state.lock() {
                            if let Some(id) = state.target {
                                state.retry_after.insert(id, now + break_timeout);
                            }
                            begin_return(&mut state, now);
                        }
                        continue;
                    }
                    self.aim_at_target(player, entities, feet, range);
                     
                     
                    return;
                }
                Phase::Click => {
                    self.release_click_if_due(now);
                    return;
                }
                Phase::Confirming => {
                    let pending = self.state.lock().ok().and_then(|state| {
                        Some((
                            state.target_entity.clone()?,
                            state.first_attack?,
                            state.last_attack?,
                            state.attempts,
                        ))
                    });
                    let Some((target, first, last, attempts)) = pending else {
                        self.cancel_sequence();
                        return;
                    };
                    let removed = target
                        .call_method("isRemoved", &[])
                        .and_then(|value| Ok(value.z()?))
                        .unwrap_or(false);
                    if removed || now.saturating_duration_since(first) >= self.break_timeout() {
                        let break_timeout = self.break_timeout();
                        if let Ok(mut state) = self.state.lock() {
                            if !self.multi_target() {
                                state.single_target_completed = true;
                            }
                            if !removed {
                                if let Some(id) = state.target {
                                    state.retry_after.insert(id, now + break_timeout);
                                }
                                log::debug!(
                                    "auto crystal: break not confirmed after {attempts} attempts"
                                );
                            }
                            begin_return(&mut state, now);
                        }
                        continue;
                    }
                    if !removed && self.multi_target() {
                         
                         
                         
                         
                        let chained = {
                            let Ok(mut state) = self.state.lock() else {
                                return;
                            };
                            if let Some(id) = state.target {
                                state.retry_after.insert(id, now + self.break_retry());
                            }
                            let retry_after = state.retry_after.clone();
                            let previous = state.target;
                            self.select_target(
                                entities,
                                player,
                                feet,
                                range,
                                &mut state.first_seen,
                                now,
                                previous,
                                &retry_after,
                            )
                            .map(|(next_id, next)| {
                                self.claim_camera();
                                state.target = Some(next_id);
                                state.target_entity = Some(next);
                                state.locked_at = Some(now);
                                state.click_timer_at = Some(now);
                                state.click_jitter_ms =
                                    click_jitter_ms(self.click_randomization_ms(), next_id);
                                state.last_attack = None;
                                state.first_attack = None;
                                state.attempts = 0;
                                state.phase = Phase::ClickTimer;
                            })
                            .is_some()
                        };
                        if chained {
                            continue;
                        }
                    }
                    if retry_ready(last, attempts, now, self.break_retry(), self.max_attempts()) {
                        if attempts >= self.max_attempts() {
                            if !self.multi_target() {
                                if let Ok(mut state) = self.state.lock() {
                                    state.single_target_completed = true;
                                }
                            }
                            if let Ok(mut state) = self.state.lock() {
                                begin_return(&mut state, now);
                            }
                            continue;
                        }
                        if let Ok(mut state) = self.state.lock() {
                            state.click_timer_at = Some(now);
                            state.click_jitter_ms = click_jitter_ms(
                                self.click_randomization_ms(),
                                state.target.unwrap_or_default(),
                            );
                            state.phase = Phase::ClickTimer;
                        }
                        continue;
                    }
                    return;
                }
                Phase::Returning => {
                    if self.return_step(player, entities, feet, range, now) {
                         
                         
                        continue;
                    }
                    return;
                }
            }
        }
    }

     
     
     
    fn aim_at_target(&self, player: &LocalPlayer, entities: &[Entity], feet: Vec3, range: f64) {
        let target = match self.state.lock() {
            Ok(state) => state.target_entity.clone(),
            Err(_) => return,
        };
        let Some(target) = target else {
            self.cancel_sequence();
            return;
        };
        let Ok(target_id) = target.id() else {
            self.cancel_sequence();
            return;
        };
         
         
        let Some(live) = entities
            .iter()
            .find(|entity| entity.id().ok() == Some(target_id))
            .cloned()
        else {
            if self.debug() {
                log::info!("auto crystal: target {target_id} vanished, returning");
            }
            if let Ok(mut state) = self.state.lock() {
                begin_return(&mut state, Instant::now());
            }
            return;
        };
        let removed = live
            .call_method("isRemoved", &[])
            .and_then(|value| Ok(value.z()?))
            .unwrap_or(true);
        let in_range = !removed
            && super::target_box(&live).is_ok_and(|(min, max)| {
                super::point_box_distance([feet.x(), feet.y(), feet.z()], min, max)
                    <= range + self.aim_tolerance()
            });
        let timed_out = self.state.lock().ok().is_some_and(|state| {
            state
                .locked_at
                .is_some_and(|at| at.elapsed() >= self.target_timeout())
        });
        if !in_range || timed_out {
            if timed_out {
                let break_timeout = self.break_timeout();
                if let Ok(mut state) = self.state.lock() {
                    state
                        .retry_after
                        .insert(target_id, Instant::now() + break_timeout);
                }
            }
            if self.debug() {
                log::info!("auto crystal: target {target_id} invalid or timed out, returning");
            }
            if let Ok(mut state) = self.state.lock() {
                begin_return(&mut state, Instant::now());
            }
            return;
        }

         
         
        let (Ok(eye), Ok(target_feet), Ok(height)) = (
            player.get_eye_position(),
            live.get_position(),
            live.bb_height(),
        ) else {
            return;
        };
        let Ok(width) = live.bb_width() else {
            return;
        };
         
         
        let aim = crystal_aim_point(eye, target_feet, width as f64, height as f64);
        let desired = Rotation::towards(eye, aim);
        self.aim(desired, self.aim_ms());

         
        if let Ok(mut state) = self.state.lock() {
            state.target_entity = Some(live);
        }
    }

     
     
     
     
    fn return_step(
        &self,
        player: &LocalPlayer,
        entities: &[Entity],
        feet: Vec3,
        range: f64,
        now: Instant,
    ) -> bool {
         
         
         
        let previous = match self.state.lock() {
            Ok(state) => state.target,
            Err(_) => return false,
        };
        if self.user_took_the_mouse() {
            self.release_click();
            crate::module::combat::rotation::clear();
            self.cancel_sequence();
            return false;
        }

         
        let chained = if self.multi_target() {
            let Ok(mut state) = self.state.lock() else {
                return false;
            };
            let retry_after = state.retry_after.clone();
            self.select_target(
                entities,
                player,
                feet,
                range,
                &mut state.first_seen,
                now,
                previous,
                &retry_after,
            )
            .map(|(next_id, next)| {
                self.claim_camera();
                state.target = Some(next_id);

                state.target_entity = Some(next);
                state.locked_at = Some(now);
                state.click_timer_at = Some(now);
                state.click_jitter_ms = click_jitter_ms(self.click_randomization_ms(), next_id);
                state.last_attack = None;
                state.first_attack = None;
                state.attempts = 0;
                state.returning_at = None;
                state.phase = Phase::ClickTimer;
            })
            .is_some()
        } else {
            if let Ok(mut state) = self.state.lock() {
                state.single_target_completed = true;
            }
            false
        };
        if chained {
            return true;
        }

         
         
        if self.silent() {
            crate::module::combat::rotation::clear();
            self.cancel_sequence();
            return true;
        }

         
         
        let goal = match self.state.lock() {
            Ok(state) => state.return_rotation,
            Err(_) => return false,
        };
        let expired = self.state.lock().ok().is_some_and(|state| {
            state
                .returning_at
                .is_some_and(|at| now.saturating_duration_since(at) >= self.target_timeout())
        });
        if expired {
            crate::module::combat::rotation::clear();
            self.cancel_sequence();
            return true;
        }
        if !self.return_to_view() {
            if self.debug() {
                log::info!("auto crystal: sequence done without camera return");
            }
            crate::module::combat::rotation::clear();
            self.cancel_sequence();
            return true;
        }
        if let Some(return_rotation) = goal {
            self.aim(return_rotation, self.return_ms());
             
             
             
            let commanded_residual = crate::module::combat::rotation::current_rotation()
                .map(|current| current.angle_to(return_rotation))
                .unwrap_or(f32::INFINITY);
            let residual = player
                .get_yaw()
                .ok()
                .zip(player.get_pitch().ok())
                .map(|(yaw, pitch)| Rotation::new(yaw, pitch).angle_to(return_rotation))
                .unwrap_or(f32::INFINITY);
            if residual > 1.5 || commanded_residual > 1.5 {
                return false;
            }
        }
        if self.debug() {
            log::info!("auto crystal: sequence done");
        }
        self.cancel_sequence();
        true
    }
}

impl Module for AutoCrystalModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.release_click();
        *self.state.lock().unwrap() = AutoCrystalState::default();
        *self.reach_applied.lock().unwrap() = None;
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.release_click();
        *self.state.lock().unwrap() = AutoCrystalState::default();
        crate::module::misc::freelook::release_body_view();
        crate::module::combat::rotation::clear();
         
        self.clear_reach();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let minecraft = minecraft();
         
         
        if !self.input_available() {
            self.cancel_sequence();
            return Ok(());
        }
         
        if minecraft.current_screen().is_open() {
            self.cancel_sequence();
            return Ok(());
        }
        let (Some(player), Some(world)) = (minecraft.player()?, minecraft.world()?) else {
            self.cancel_sequence();
            return Ok(());  
        };

        if self.silent() {
            self.log_walk_sync_probe(&player);
        }

         
         
        if let Err(error) = self.sync_reach(&player) {
             
            log::debug!("auto crystal: reach update deferred: {error:#}");
        }
        let reach = player
            .get_attribute_value(REACH_ATTRIBUTE)?
            .unwrap_or(VANILLA_REACH)
            .max(0.0);
        let range = self.range_slider().min(reach as f32) as f64;
        let entities = world.get_entities()?;

         
         
        if let Ok(mut state) = self.state.lock() {
            state.range = range;
            state.entities = entities.clone();
        }

         
         
        self.step_sequence(&player, &entities, range, Instant::now());
        Ok(())
    }

     
     
     
     
    fn on_render(&self) {
        if !client().modules.is_active() {
            return;
        }
        let minecraft = minecraft();
        if !self.input_available() {
             
             
            self.cancel_sequence();
            return;
        }
        let Some(player) = minecraft.player().ok().flatten() else {
            return;
        };
        let now = Instant::now();
        if self
            .state
            .lock()
            .ok()
            .is_some_and(|state| state.phase != Phase::Idle)
        {
            self.claim_camera();
        }
         
         
        self.release_click_if_due(now);

         
        let (entities, range) = match self.state.lock() {
            Ok(state) => (state.entities.clone(), state.range),
            Err(_) => return,
        };
         
         
        self.step_sequence(&player, &entities, range, now);

         
         
         
         
        let pending = {
            let Ok(state) = self.state.lock() else {
                return;
            };
            if state.phase != Phase::ClickTimer {
                return;
            }
            let (Some(target_id), Some(started_at)) = (state.target, state.click_timer_at) else {
                return;
            };
            (target_id, started_at, state.click_jitter_ms)
        };
        let (target_id, started_at, jitter_ms) = pending;
        self.claim_camera();
        if !click_due(Some(started_at), self.delay_ms(), jitter_ms, now) {
            return;
        }

         
         
        let Some(live) = entities
            .iter()
            .find(|entity| entity.id().ok() == Some(target_id))
            .cloned()
        else {
            if let Ok(mut state) = self.state.lock() {
                begin_return(&mut state, now);
            }
            self.step_sequence(&player, &entities, range, now);
            return;
        };
        let valid = live.instance_of::<EndCrystal>()
            && !live
                .call_method("isRemoved", &[])
                .and_then(|value| Ok(value.z()?))
                .unwrap_or(true)
            && super::target_box(&live).is_ok_and(|(min, max)| {
                let (Ok(eye), Ok(reach)) = (
                    player.get_eye_position(),
                    player.get_attribute_value(REACH_ATTRIBUTE),
                ) else {
                    return false;
                };
                let reach = reach.unwrap_or(VANILLA_REACH);
                reach.is_finite()
                    && reach >= 0.0
                    && super::point_box_distance([eye.x(), eye.y(), eye.z()], min, max) <= reach
            });
        if !valid {
            if let Ok(mut state) = self.state.lock() {
                begin_return(&mut state, now);
            }
            self.step_sequence(&player, &entities, range, now);
            return;
        }
         
         
        self.claim_camera();
        match self.check_aim(&player, &live, range) {
            BodyAimStatus::Ready => {}
            BodyAimStatus::Turning => return,
            BodyAimStatus::Missed => {
                if let Ok(mut state) = self.state.lock() {
                    begin_return(&mut state, now);
                }
                self.step_sequence(&player, &entities, range, now);
                return;
            }
        }
         
         
         
         
        if self.silent() {
             
             
             
             
            if moving_for_wire(&player) {
                return;
            }
            self.silent_attack(&player, &live, &entities, range, now);
            return;
        }
         
         
        if !crate::graphic::platform::left_mouse_down() {
            return;
        }

        let retained_press = if let Ok(mut state) = self.state.lock() {
             
             
            if state.phase == Phase::ClickTimer {
                state.target_entity = Some(live);
                state.click_held = true;
                state.click_release_at =
                    Some(now + Duration::from_millis(self.click_hold_ms() as u64));
                state.phase = Phase::Click;
                true
            } else {
                false
            }
        } else {
            false
        };
        if !retained_press {
            let _ = crate::graphic::platform::left_mouse_up();
            return;
        }
         
         
        self.release_click_if_due(now);
        self.step_sequence(&player, &entities, range, now);
    }

     
     
     
     
     
     
     
     
     
     
     
    fn handle_packet(&self, packet: &mut Packet) -> PacketAction {
        if !self.silent() {
            return PacketAction::Forward;
        }
        let aim = crate::module::combat::rotation::silent_rotation();
        let aim_live = aim.is_some();
        match packet {
            Packet::ServerboundMovePlayer(snapshot) => {
                if AIM_PENDING.swap(false, Ordering::AcqRel) {
                    snapshot.apply_silent_aim(aim);
                }
                PacketAction::Forward
            }
            Packet::ServerboundSwing(_) | Packet::ServerboundInteract(_) => {
                hit_action(!aim_live || take_hit_packet(Instant::now()))
            }
            _ => PacketAction::Forward,
        }
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
    use crate::module::combat::{is_still, STILL_SPEED};
    use crate::net::packet::serverbound_move_player::ServerboundMovePlayerPacket;

    #[test]
    fn only_the_modules_own_hit_passes_the_ghost_look() {
        let now = Instant::now();
        let mut window = Some((HIT_WINDOW_PACKETS, now + HIT_WINDOW_LEN));
        for _ in 0..HIT_WINDOW_PACKETS {
            assert!(spend_hit_window(&mut window, now));
        }
        assert!(
            !spend_hit_window(&mut window, now),
            "the window carries only its own packets"
        );

        let mut expiring = Some((HIT_WINDOW_PACKETS, now + HIT_WINDOW_LEN));
        assert!(
            !spend_hit_window(
                &mut expiring,
                now + HIT_WINDOW_LEN + Duration::from_millis(1)
            ),
            "an expired window passes nothing"
        );
        assert!(!spend_hit_window(&mut None, now));

        assert_eq!(hit_action(false), PacketAction::Cancel);
        assert_eq!(hit_action(true), PacketAction::Forward);
    }

    #[test]
    fn silent_hits_need_the_player_standing_still() {
        assert!(is_still(0.0));
        assert!(is_still(STILL_SPEED));
        assert!(!is_still(STILL_SPEED + 0.001));
        assert!(!is_still(0.1), "walking speed must block a silent hit");
        assert!(!is_still(f64::NAN), "an unreadable speed must block it");
    }

    #[test]
    fn the_hit_window_closes_on_demand() {
        let _guard = crate::module::combat::rotation::test_guard();
        open_hit_window(Instant::now());
        assert!(AIM_PENDING.load(Ordering::Acquire));
        AIM_PENDING.store(false, Ordering::Release);
        close_hit_window();
        assert!(HIT_WINDOW.lock().unwrap().is_none());
    }

    #[test]
    fn only_the_first_movement_packet_after_a_hit_carries_the_aim() {
        let _guard = crate::module::combat::rotation::test_guard();
        AIM_PENDING.store(false, Ordering::Release);
        open_hit_window(Instant::now());
        assert!(AIM_PENDING.swap(false, Ordering::AcqRel));
        assert!(!AIM_PENDING.swap(false, Ordering::AcqRel));
        close_hit_window();
    }

    #[test]
    fn module_preserves_crystal_aura_identity_and_uses_manual_mouse() {
        let module = AutoCrystalModule::new();
        assert_eq!(module.module.id, ModuleId::AutoCrystal);
        assert!(module.module.description.contains("end crystals"));
        assert!(module
            .module
            .settings
            .iter()
            .any(|setting| setting.name() == "Aim Time (ms)"));
        assert!(module.module.get_setting("Actual Mouse").is_none());
        assert!(module.multi_target());
        assert!(module.return_to_view());
        assert!(!module.silent(), "camera-following aim is the default");
    }

    #[test]
    fn manual_click_hold_defaults_and_retries_are_bounded() {
        assert_eq!(AutoCrystalModule::new().click_hold_ms(), 12);
        let now = Instant::now();
        let retry = Duration::from_millis(180);
        assert!(!retry_ready(
            now,
            1,
            now + Duration::from_millis(179),
            retry,
            3
        ));
        assert!(retry_ready(
            now,
            1,
            now + Duration::from_millis(180),
            retry,
            3
        ));
        assert!(
            !retry_ready(now, 3, now + Duration::from_secs(2), retry, 3),
            "the attempt cap stops the retries"
        );
         
         
        assert!(!retry_ready(
            now,
            1,
            now + Duration::from_millis(180),
            Duration::from_millis(500),
            3
        ));
        assert!(retry_ready(
            now,
            1,
            now + Duration::from_millis(180),
            Duration::from_millis(50),
            3
        ));
        assert!(retry_ready(now, 4, now + Duration::from_secs(2), retry, 6));
        let mut state = AutoCrystalState::default();
        state.last_attack = Some(now);
        state.first_attack = Some(now);
        state.attempts = 2;
        reset_sequence(&mut state);
        assert!(state.last_attack.is_none() && state.first_attack.is_none());
        assert_eq!(state.attempts, 0);
    }

    #[test]
    fn returning_keeps_the_original_view_until_the_sequence_resets() {
        let now = Instant::now();
        let view = Rotation::new(25.0, -10.0);
        let mut state = AutoCrystalState::default();
        state.phase = Phase::ClickTimer;
        state.return_rotation = Some(view);
        state.click_timer_at = Some(now);
        state
            .retry_after
            .insert(9, now + Duration::from_millis(900));
        begin_return(&mut state, now);
        assert_eq!(state.phase, Phase::Returning);
        assert_eq!(state.return_rotation, Some(view));
        assert_eq!(state.returning_at, Some(now));
        assert!(state.click_timer_at.is_none());
        reset_sequence(&mut state);
        assert_eq!(state.phase, Phase::Idle);
        assert!(state.return_rotation.is_none() && state.returning_at.is_none());
        assert_eq!(
            state.retry_after.get(&9),
            Some(&(now + Duration::from_millis(900)))
        );
    }

    #[test]
    fn wide_crystals_aim_at_the_reachable_face_not_the_center() {
        let eye = Vec3::new(0.0, 1.62, 0.0);
        let aim = crystal_aim_point(eye, Vec3::new(0.0, 0.0, 4.8), 2.0, 2.0);
        assert!(aim.z() > 3.8 && aim.z() < 3.9);
        assert!((aim.y() - eye.y()).abs() < 1e-9);
        let rotation = Rotation::towards(eye, aim);
        let direction = super::super::look_dir(rotation.yaw, rotation.pitch);
        assert!(super::super::ray_hits_box(
            [eye.x(), eye.y(), eye.z()],
            direction,
            [-1.0, 0.0, 3.8],
            [1.0, 2.0, 5.8],
            4.5,
        ));
    }

    #[test]
    fn priority_defaults_to_newly_placed() {
        assert!(AutoCrystalModule::new().priority_new());
    }

     
    #[test]
    fn range_setting_is_editable_up_to_four_and_a_half_blocks() {
        let module = AutoCrystalModule::new();
        let Some(ModuleSetting::Slider {
            value, min, max, ..
        }) = module.module.get_setting("Range")
        else {
            panic!("Auto Crystal must expose a Range slider");
        };
        assert_eq!(*max, 4.5, "the range ceiling is four and a half blocks");
        assert!(*min < *max, "the range must be adjustable, not frozen");
        assert_eq!(*value, 4.5, "the range starts at the full reach");
    }

     
     
    #[test]
    fn all_required_timing_and_aim_settings_are_editable() {
        let mut module = AutoCrystalModule::new();
        assert_eq!(module.delay_ms(), 10);
        assert_eq!(module.click_hold_ms(), 12);
        assert_eq!(module.aim_ms(), 50);
        assert_eq!(module.return_ms(), 150);
        assert_eq!(module.aim_tolerance(), 0.5);
        assert_eq!(module.click_randomization_ms(), 0);
        let now = Instant::now();
        let jitter = click_jitter_ms(30, 11);
        assert!((-30..=30).contains(&jitter));
        assert!(click_due(Some(now), 10, -10, now));
        assert!(!click_due(
            Some(now),
            10,
            10,
            now + Duration::from_millis(19)
        ));
        for name in [
            "Attack Delay (ms)",
            "Click Randomization (ms)",
            "Click Hold (ms)",
            "Aim Time (ms)",
            "Aim Tolerance (blocks)",
            "Return Time (ms)",
            "Break Retry (ms)",
            "Break Timeout (ms)",
            "Target Timeout (ms)",
            "Retry Attempts",
            "New Target Window (ms)",
            "Aim Settle (ms)",
            "Range",
        ] {
            assert!(
                module.module.get_setting_mut(name).is_some(),
                "missing {name}"
            );
        }
        module
            .module
            .get_setting_mut("Multi Target")
            .unwrap()
            .set_toggle_value(false);
        module
            .module
            .get_setting_mut("Return to View")
            .unwrap()
            .set_toggle_value(false);
        assert!(!module.multi_target());
        assert!(!module.return_to_view());
    }

     
     
     
    #[test]
    fn a_lower_duration_converges_harder_and_zero_snaps() {
        let time_to_land = |rate: f32| SWEEP_FOLDS / rate * 1000.0;
        assert_eq!(sweep_rate(0.0), SNAP_RATE, "zero duration snaps");
        assert_eq!(
            sweep_rate(1.0),
            SNAP_RATE,
            "a duration too short to ease is capped at the snap rate"
        );
        let mut previous = f32::INFINITY;
        for duration_ms in [50.0, 100.0, 150.0, 200.0, 400.0] {
            let rate = sweep_rate(duration_ms);
            assert!(rate < previous, "the rate must fall as the duration grows");
             
             
            assert!(
                (time_to_land(rate) - duration_ms).abs() < 1.0,
                "{duration_ms} ms of Aim Time produced a {:.0} ms sweep",
                time_to_land(rate)
            );
            previous = rate;
        }
        assert!(
            sweep_rate(400.0) >= MIN_SWEEP_RATE,
            "even the slowest sweep keeps converging"
        );
        assert!(
            sweep_rate(f32::MAX) >= MIN_SWEEP_RATE,
            "an absurd duration can only ever slow a sweep down"
        );
    }

     
     
    #[test]
    fn the_reach_modifier_only_covers_the_shortfall() {
        assert_eq!(missing_reach(4.5, 3.0), 1.5, "vanilla reach needs 1.5 more");
        assert_eq!(missing_reach(4.5, 4.5), 0.0, "an exact reach needs nothing");
        assert_eq!(
            missing_reach(3.0, 3.3),
            0.0,
            "another module's reach is left alone"
        );
        assert_eq!(missing_reach(1.0, 0.0), 1.0);
    }

     
     
    #[test]
    fn zero_pacing_on_every_slider_selects_the_snap_path() {
        let mut module = AutoCrystalModule::new();
        for name in ["Attack Delay (ms)", "Aim Time (ms)", "Return Time (ms)"] {
            module
                .module
                .get_setting_mut(name)
                .unwrap_or_else(|| panic!("Auto Crystal must expose {name}"))
                .set_slider_value(0.0);
        }
        assert_eq!(module.delay_ms(), 0, "the break fires immediately");
        assert_eq!(module.aim_ms(), 0);
        assert_eq!(module.return_ms(), 0);
         
        assert_eq!(sweep_rate(module.aim_ms() as f32), SNAP_RATE);
        assert_eq!(sweep_rate(module.return_ms() as f32), SNAP_RATE);
         
        let now = Instant::now();
        assert!(delay_elapsed(now, module.delay_ms(), 0, now));
    }

     
     
    #[test]
    fn click_timer_runs_during_aim_and_honors_randomization() {
        let armed_at = Instant::now();
        assert!(!delay_elapsed(
            armed_at,
            10,
            0,
            armed_at + Duration::from_millis(9)
        ));
        assert!(delay_elapsed(
            armed_at,
            10,
            0,
            armed_at + Duration::from_millis(10)
        ));
        assert!(delay_elapsed(
            armed_at,
            10,
            0,
            armed_at + Duration::from_millis(500)
        ));
        assert!(delay_elapsed(
            armed_at,
            10,
            -5,
            armed_at + Duration::from_millis(5)
        ));
        assert!(!delay_elapsed(
            armed_at,
            10,
            5,
            armed_at + Duration::from_millis(14)
        ));
        assert!(delay_elapsed(
            armed_at,
            10,
            5,
            armed_at + Duration::from_millis(15)
        ));
        assert!(click_due(
            Some(armed_at),
            10,
            0,
            armed_at + Duration::from_millis(10)
        ));
        assert!(!click_due(None, 0, 0, armed_at + Duration::from_secs(1)));
    }

     
     
     
    #[test]
    fn every_sequence_delay_starts_at_its_default_and_is_editable() {
        let mut module = AutoCrystalModule::new();
        assert_eq!(module.break_retry(), DEFAULT_BREAK_RETRY);
        assert_eq!(module.break_timeout(), DEFAULT_BREAK_TIMEOUT);
        assert_eq!(module.target_timeout(), DEFAULT_TARGET_TIMEOUT);
        assert_eq!(module.max_attempts(), DEFAULT_MAX_BREAK_ATTEMPTS);
        assert_eq!(module.new_target_window(), DEFAULT_NEW_TARGET_WINDOW);
        assert_eq!(module.pick_settle(), DEFAULT_PICK_SETTLE);

        let edits: [(&str, f32, Duration); 5] = [
            ("Break Retry (ms)", 40.0, Duration::from_millis(40)),
            ("Break Timeout (ms)", 1500.0, Duration::from_millis(1500)),
            ("Target Timeout (ms)", 5000.0, Duration::from_millis(5000)),
            (
                "New Target Window (ms)",
                12_000.0,
                Duration::from_millis(12_000),
            ),
            ("Aim Settle (ms)", 600.0, Duration::from_millis(600)),
        ];
        for (name, value, expected) in edits {
            module
                .module
                .get_setting_mut(name)
                .unwrap_or_else(|| panic!("Auto Crystal must expose {name}"))
                .set_slider_value(value);
            let read = match name {
                "Break Retry (ms)" => module.break_retry(),
                "Break Timeout (ms)" => module.break_timeout(),
                "Target Timeout (ms)" => module.target_timeout(),
                "New Target Window (ms)" => module.new_target_window(),
                _ => module.pick_settle(),
            };
            assert_eq!(read, expected, "{name} must be the delay the module uses");
        }
        module
            .module
            .get_setting_mut("Retry Attempts")
            .unwrap()
            .set_slider_value(7.0);
        assert_eq!(module.max_attempts(), 7);
    }

     
     
     
     
     
    #[test]
    fn a_silent_aim_rewrites_outgoing_movement_and_a_camera_aim_does_not() {
        use crate::module::combat::rotation::{self, test_guard};

        let _guard = test_guard();
        let mut module = AutoCrystalModule::new();
         
         
        let outbound = || {
            Packet::ServerboundMovePlayer(ServerboundMovePlayerPacket {
                x: 12.0,
                y: 64.0,
                z: -9.0,
                y_rot: 45.0,
                x_rot: 20.0,
                on_ground: false,
                horizontal_collision: true,
                has_position: true,
                has_rotation: false,
            })
        };
        let snapshot = |packet: Packet| {
            let Packet::ServerboundMovePlayer(snapshot) = packet else {
                unreachable!("the module must never change the packet's kind")
            };
            snapshot
        };

         
         
         
        rotation::aim_silent(Rotation::new(-31.5, 14.25), 100_000.0);
        let mut camera_aim = outbound();
        let untouched = camera_aim.clone();
        assert_eq!(module.handle_packet(&mut camera_aim), PacketAction::Forward);
        assert_eq!(camera_aim, untouched);
        let packet = snapshot(camera_aim);
        assert!(!packet.has_rotation);
        assert_eq!((packet.y_rot, packet.x_rot), (45.0, 20.0));

         
         
        module
            .module
            .get_setting_mut("Silent Aim")
            .unwrap()
            .set_toggle_value(true);
         
        open_hit_window(Instant::now());
        let mut silent = outbound();
        let before = silent.clone();
        assert_eq!(module.handle_packet(&mut silent), PacketAction::Forward);
        assert_ne!(
            silent, before,
            "the dispatch rebuilds a movement packet only when it changed"
        );
        let packet = snapshot(silent);
        assert!(
            packet.has_rotation,
            "the aim cannot arrive without rotation"
        );
        assert!(
            (packet.y_rot + 31.5).abs() < 1e-3 && (packet.x_rot - 14.25).abs() < 1e-3,
            "the advertised angle must be the silent aim, got {packet:?}"
        );
        assert_eq!(
            (packet.x, packet.y, packet.z, packet.on_ground),
            (12.0, 64.0, -9.0, false),
            "position and ground state are not the aim's business"
        );

         
        let mut next = outbound();
        let next_before = next.clone();
        assert_eq!(module.handle_packet(&mut next), PacketAction::Forward);
        assert_eq!(
            next, next_before,
            "only the first packet after a hit is stamped"
        );
        close_hit_window();

         
        module
            .module
            .get_setting_mut("Silent Aim")
            .unwrap()
            .set_toggle_value(true);
        rotation::clear();
        let mut released = outbound();
        let untouched = released.clone();
        module.handle_packet(&mut released);
        assert_eq!(released, untouched);
        assert!(!snapshot(released).has_rotation);
    }

     
     
    #[test]
    fn silent_aim_is_a_toggle_that_selects_the_shadow_aim() {
        let mut module = AutoCrystalModule::new();
        assert!(!module.silent());
        module
            .module
            .get_setting_mut("Silent Aim")
            .expect("Auto Crystal must expose Silent Aim")
            .set_toggle_value(true);
        assert!(module.silent(), "the toggle selects the shadow aim");
        module
            .module
            .get_setting_mut("Silent Aim")
            .unwrap()
            .set_toggle_value(false);
        assert!(!module.silent());
        assert!(matches!(
            module.module.get_setting("Silent Aim"),
            Some(ModuleSetting::Toggle { .. })
        ));
    }
}
