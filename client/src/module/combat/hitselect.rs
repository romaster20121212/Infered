 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::client::world::World;
use crate::mapping::entity::player::{LocalPlayer, Player};
use crate::mapping::entity::{Entity, EntityRef, LivingEntityRef, PlayerRef};
use crate::mapping::math::Vec3;
use crate::mapping::MappedObject;
use crate::module::combat::rotation::Rotation;
use crate::module::combat::within_reach;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

 
 
const DEFAULT_MAX_HOLD_MS: f32 = 200.0;
 
const MAX_HOLD_MS: f32 = 500.0;
 
 
const HARD_CAP_MS: u64 = 1_000;
 
 
const CHARGED: f32 = 0.99;
 
const MAX_AIM_ANGLE: f32 = 40.0;

 
 
static ARMED: AtomicBool = AtomicBool::new(false);
 
static ARMED_AT_MS: AtomicU64 = AtomicU64::new(0);
 
 
 
const ARM_STALE_MS: u64 = 250;
 
static HELD: AtomicBool = AtomicBool::new(false);
 
static HELD_AT_MS: AtomicU64 = AtomicU64::new(0);
 
 
static CLOCK: OnceLock<Instant> = OnceLock::new();
 
 
static FIRST_RELEASE_REPORTED: AtomicBool = AtomicBool::new(false);

 
fn now_ms() -> u64 {
    CLOCK.get_or_init(Instant::now).elapsed().as_millis() as u64
}

 
 
 
 
 
 
 
 
pub(crate) fn take_click() -> bool {
    if !armed_now() {
        return false;
    }
    if HELD.load(Ordering::Acquire) {
        return true;
    }
    HELD_AT_MS.store(now_ms(), Ordering::Relaxed);
    HELD.store(true, Ordering::Release);
    true
}

 
 
pub(crate) fn holds_click() -> bool {
    HELD.load(Ordering::Acquire)
}

 
fn armed_now() -> bool {
    arming_is_live(
        ARMED.load(Ordering::Acquire),
        ARMED_AT_MS.load(Ordering::Relaxed),
        now_ms(),
        ARM_STALE_MS,
    )
}

 
 
 
fn arming_is_live(armed: bool, armed_at_ms: u64, now_ms: u64, stale_ms: u64) -> bool {
    armed && now_ms.saturating_sub(armed_at_ms) <= stale_ms
}

 
 
#[derive(Debug, Default)]
struct SelectState {
     
    previous_health: Option<f32>,
     
     
    previous_hurt: Option<i32>,
}

 
 
#[derive(Debug)]
pub struct HitSelectModule {
    pub module: ModuleData,
    state: Mutex<SelectState>,
     
     
    warned: AtomicBool,
}

impl HitSelectModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::HitSelect,
                description: "Holds your click until it lands in the trade window".to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Max Hold (ms)".to_string(),
                    value: DEFAULT_MAX_HOLD_MS,
                    min: 0.0,
                    max: MAX_HOLD_MS,
                }],
            },
            state: Mutex::new(SelectState::default()),
            warned: AtomicBool::new(false),
        }
    }

    fn slider(&self, name: &str, fallback: f32) -> f32 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .unwrap_or(fallback)
    }

     
    fn max_hold_ms(&self) -> u64 {
        self.slider("Max Hold (ms)", DEFAULT_MAX_HOLD_MS)
            .clamp(0.0, MAX_HOLD_MS)
            .round() as u64
    }

     
     
     
    fn disarm(&self) {
        ARMED.store(false, Ordering::Release);
        HELD.store(false, Ordering::Release);
        *self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = SelectState::default();
    }

     
     
     
     
     
     
    fn aimed_target(&self, player: &LocalPlayer, world: &World) -> Option<Entity> {
        let eye = player.get_eye_position().ok()?;
        let self_id = player.id().ok()?;
        let look = Rotation::new(player.get_yaw().ok()?, player.get_pitch().ok()?);
        let entities = world.get_entities().ok()?;
        let mut best: Option<(f64, Entity)> = None;
        for entity in entities {
            if !entity.instance_of::<Player>() || entity.id().ok() == Some(self_id) {
                continue;
            }
            if crate::friends::ignores_entity(&entity, ModuleId::HitSelect) {
                continue;
            }
            if !entity
                .as_living()
                .is_some_and(|living| living.is_alive().unwrap_or(false))
            {
                continue;
            }
            let (Ok(feet), Ok(height)) = (entity.get_position(), entity.bb_height()) else {
                continue;
            };
            let aim = Vec3::new(feet.x(), feet.y() + height as f64 * 0.7, feet.z());
            if look.angle_to(Rotation::towards(eye, aim)) > MAX_AIM_ANGLE {
                continue;
            }
            if !within_reach(player, &entity).unwrap_or(false) {
                continue;
            }
            let Ok(distance) = entity.distance_to_sqr(eye.x(), eye.y(), eye.z()) else {
                continue;
            };
            if best.as_ref().is_none_or(|(closest, _)| distance < *closest) {
                best = Some((distance, entity));
            }
        }
        best.map(|(_, entity)| entity)
    }

     
    fn send_click(&self) {
        if let Err(error) = minecraft().start_attack() {
            if !self.warned.swap(true, Ordering::AcqRel) {
                log::warn!(
                    "HitSelect: vanilla's click path is unreachable, held clicks are dropped: \
                     {error}"
                );
            }
            return;
        }
        if !FIRST_RELEASE_REPORTED.swap(true, Ordering::AcqRel) {
            log::info!("HitSelect: held clicks are being released as vanilla attacks");
        }
    }
}

impl Module for HitSelectModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.disarm();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.disarm();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let mc = minecraft();
         
         
        if mc.current_screen().is_open() {
            self.disarm();
            return Ok(());
        }
        let (Some(player), Some(world)) = (mc.player()?, mc.world()?) else {
            self.disarm();
            return Ok(());
        };
        if !player.is_alive().unwrap_or(true) {
            self.disarm();
            return Ok(());
        }

        let hurt_time = player.hurt_time().unwrap_or(0);
        let health = player.get_health().ok();
        let damaged = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let damaged = damage_taken(
                state.previous_health,
                health,
                state.previous_hurt,
                hurt_time,
            );
            state.previous_health = health;
            state.previous_hurt = Some(hurt_time);
            damaged
        };

         
         
         
        let armed = self.max_hold_ms() > 0 && self.aimed_target(&player, &world).is_some();
        ARMED_AT_MS.store(now_ms(), Ordering::Release);
        ARMED.store(armed, Ordering::Release);

        if !HELD.load(Ordering::Acquire) {
            return Ok(());
        }

        let held_ms = now_ms().saturating_sub(HELD_AT_MS.load(Ordering::Relaxed));
        let due = damaged || hold_expired(held_ms, self.max_hold_ms());
        let charged = player
            .attack_strength_scale()
            .map(|scale| scale >= CHARGED)
            .unwrap_or(true);
        if !due || !may_send(charged, held_ms) {
            return Ok(());
        }

         
         
        HELD.store(false, Ordering::Release);
        self.send_click();
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

 
 
 
 
fn damage_taken(
    previous_health: Option<f32>,
    health: Option<f32>,
    previous_hurt: Option<i32>,
    hurt_time: i32,
) -> bool {
    let health_fell = match (previous_health, health) {
        (Some(previous), Some(current)) => {
            previous.is_finite() && current.is_finite() && current < previous
        }
        _ => false,
    };
    let hurt_restarted = previous_hurt.is_some_and(|previous| hurt_time > previous);
    health_fell || hurt_restarted
}

 
fn hold_expired(held_ms: u64, max_hold_ms: u64) -> bool {
    held_ms >= max_hold_ms
}

 
 
 
fn may_send(charged: bool, held_ms: u64) -> bool {
    charged || held_ms >= HARD_CAP_MS
}

#[cfg(test)]
mod tests {
    use super::{
        arming_is_live, damage_taken, hold_expired, may_send, HitSelectModule, ARM_STALE_MS,
        CHARGED, DEFAULT_MAX_HOLD_MS, HARD_CAP_MS, MAX_HOLD_MS,
    };
    use crate::module::{Module, ModuleId};

     
     
    #[test]
    fn a_hit_is_spotted_from_either_signal() {
         
        assert!(damage_taken(Some(20.0), Some(13.0), Some(0), 10));
         
        assert!(damage_taken(Some(20.0), Some(20.0), Some(0), 9));
         
        assert!(!damage_taken(Some(20.0), Some(20.0), Some(10), 9));
         
        assert!(!damage_taken(Some(13.0), Some(14.0), Some(0), 0));
         
        assert!(!damage_taken(None, Some(13.0), Some(7), 7));
        assert!(!damage_taken(None, Some(13.0), None, 7));
         
        assert!(!damage_taken(Some(20.0), None, Some(0), 0));
        assert!(!damage_taken(Some(f32::NAN), Some(20.0), Some(0), 0));
    }

     
     
    #[test]
    fn a_stale_arming_stops_taking_clicks() {
        assert!(arming_is_live(true, 1_000, 1_000, ARM_STALE_MS));
        assert!(arming_is_live(
            true,
            1_000,
            1_000 + ARM_STALE_MS,
            ARM_STALE_MS
        ));
        assert!(!arming_is_live(
            true,
            1_000,
            1_001 + ARM_STALE_MS,
            ARM_STALE_MS
        ));
        assert!(!arming_is_live(false, 1_000, 1_000, ARM_STALE_MS));
         
         
        assert!(ARM_STALE_MS >= 100);
    }

     
    #[test]
    fn a_hold_expires_at_the_configured_maximum() {
        assert!(!hold_expired(0, 200));
        assert!(!hold_expired(199, 200));
        assert!(hold_expired(200, 200));
        assert!(hold_expired(900, 200));
         
        assert!(hold_expired(0, 0));
    }

     
     
    #[test]
    fn a_click_waits_for_charge_unless_the_cap_expires() {
        assert!(may_send(true, 0));
        assert!(!may_send(false, 0));
        assert!(!may_send(false, HARD_CAP_MS - 1));
        assert!(may_send(false, HARD_CAP_MS));
         
         
        assert!(HARD_CAP_MS > MAX_HOLD_MS as u64);
    }

     
    #[test]
    fn the_module_ships_with_its_hold_setting() {
        let mut module = HitSelectModule::new();
        {
            let data = module.get_module_data_mut();
            assert_eq!(data.id, ModuleId::HitSelect);
            assert_eq!(data.id.display_name(), "HitSelect");
            assert_eq!(
                data.description,
                "Holds your click until it lands in the trade window"
            );
            assert!(!data.enabled);

            let hold = data
                .get_setting("Max Hold (ms)")
                .expect("Max Hold (ms) is missing");
            assert_eq!(hold.get_slider_value(), Some(DEFAULT_MAX_HOLD_MS));

            data.get_setting_mut("Max Hold (ms)")
                .expect("checked above")
                .set_slider_value(0.0);
        }
        assert_eq!(module.max_hold_ms(), 0);
        module
            .get_module_data_mut()
            .get_setting_mut("Max Hold (ms)")
            .expect("checked above")
            .set_slider_value(MAX_HOLD_MS + 400.0);
        assert_eq!(module.max_hold_ms(), MAX_HOLD_MS as u64);
    }

     
    #[test]
    fn the_charge_gate_is_strict() {
        assert!(1.0 >= CHARGED);
        assert!(!(0.6 >= CHARGED));
    }
}
