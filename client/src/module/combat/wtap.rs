use crate::mapping::entity::{EntityRef, LivingEntityRef, PlayerRef};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
 
const SPRINT_HIT_STRENGTH: f32 = 0.9;

 
 
const DEFAULT_RESET_MS: f32 = 100.0;

#[derive(Debug, Default)]
struct WTapState {
     
    last_strength: f32,
     
     
     
    was_sprinting: bool,
     
    tap_started: Option<Instant>,
     
     
     
    key_was_down: bool,
}

 
 
 
 
fn attack_fired(previous: f32, current: f32) -> bool {
    previous >= SPRINT_HIT_STRENGTH && current < previous
}

 
 
 
fn should_reset(previous: f32, current: f32, was_sprinting: bool, has_target: bool) -> bool {
    was_sprinting && has_target && attack_fired(previous, current)
}

 
fn reset_finished(started: Instant, delay: Duration, now: Instant) -> bool {
    now.saturating_duration_since(started) >= delay
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
#[derive(Debug)]
pub struct WTapModule {
    pub module: ModuleData,
    state: Mutex<WTapState>,
}

impl WTapModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::WTap,
                description:
                    "Resets sprint the moment a charged hit lands, so the next hit knocks back \
                     like a fresh sprint hit"
                        .to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Delay".to_string(),
                    value: DEFAULT_RESET_MS,
                    min: 50.0,
                    max: 500.0,
                }],
            },
            state: Mutex::new(WTapState::default()),
        }
    }

    fn delay(&self) -> Duration {
        let millis = self
            .module
            .get_setting("Delay")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(DEFAULT_RESET_MS);
        Duration::from_millis(millis.clamp(50.0, 500.0) as u64)
    }
}

impl Module for WTapModule {
    fn on_start(&self) -> anyhow::Result<()> {
        *self.state.lock().unwrap() = WTapState::default();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        let interrupted = {
            let mut state = self.state.lock().unwrap();
            let interrupted = state.tap_started.take().map(|_| state.key_was_down);
            *state = WTapState::default();
            interrupted
        };
         
         
         
        if let Some(key_was_down) = interrupted {
            crate::module::movement::sprint::release_sprint_hold();
            if key_was_down {
                set_sprint_key(true)?;
            }
            resume_sprint()?;
        }
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
        let strength = player.attack_strength_scale()?;
        let sprinting = player.is_sprinting()?;
        let now = Instant::now();
        let mut state = self.state.lock().unwrap();
        let previous = std::mem::replace(&mut state.last_strength, strength);
        let was_sprinting = std::mem::replace(&mut state.was_sprinting, sprinting);

         
         
         
         
         
        if let Some(started) = state.tap_started {
            if !reset_finished(started, self.delay(), now) {
                set_sprint_key(false)?;
                player.set_sprinting(false)?;
                return Ok(());
            }
            state.tap_started = None;
             
             
            if state.key_was_down {
                set_sprint_key(true)?;
            }
            crate::module::movement::sprint::release_sprint_hold();
             
             
            if player.move_input()?.1 > 0.0 {
                player.set_sprinting(true)?;
            }
            return Ok(());
        }

        let has_target = minecraft().crosshair_entity_id()?.is_some();
        if should_reset(previous, strength, was_sprinting, has_target) {
             
             
            crate::module::movement::sprint::hold_sprint_off(self.delay());
            let key = minecraft().options()?.key_sprint()?;
            state.key_was_down = key.is_down()?;
            key.set_down(false)?;
            player.set_sprinting(false)?;
            state.tap_started = Some(now);
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

 
fn set_sprint_key(down: bool) -> anyhow::Result<()> {
    minecraft().options()?.key_sprint()?.set_down(down)
}

 
 
fn resume_sprint() -> anyhow::Result<()> {
    let Some(player) = minecraft().player()? else {
        return Ok(());
    };
    if player.move_input()?.1 > 0.0 {
        player.set_sprinting(true)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{attack_fired, reset_finished, should_reset};
    use std::time::{Duration, Instant};

    #[test]
    fn a_charged_attack_that_spends_the_charge_counts() {
        assert!(attack_fired(1.0, 0.04));
        assert!(attack_fired(0.9, 0.0));
    }

    #[test]
    fn recharging_or_idling_is_not_an_attack() {
         
        assert!(!attack_fired(0.2, 0.3));
        assert!(!attack_fired(1.0, 1.0));
        assert!(!attack_fired(0.0, 0.0));
    }

    #[test]
    fn a_weak_attack_does_not_count_for_the_sprint_reset() {
         
         
        assert!(!attack_fired(0.5, 0.0));
    }

    #[test]
    fn a_charged_sprint_hit_on_an_entity_starts_a_reset() {
        assert!(should_reset(1.0, 0.04, true, true));
        assert!(should_reset(0.9, 0.0, true, true));
    }

    #[test]
    fn a_hit_taken_while_walking_is_not_reset() {
         
         
        assert!(!should_reset(1.0, 0.04, false, true));
    }

    #[test]
    fn a_swing_at_air_is_not_a_hit() {
        assert!(!should_reset(1.0, 0.04, true, false));
    }

    #[test]
    fn a_weak_or_recharging_swing_never_resets() {
        assert!(!should_reset(0.5, 0.0, true, true));
        assert!(!should_reset(1.0, 1.0, true, true));
        assert!(!should_reset(0.2, 0.3, true, true));
    }

    #[test]
    fn the_sprint_is_held_off_for_exactly_the_delay() {
        let started = Instant::now();
        let delay = Duration::from_millis(100);
        assert!(!reset_finished(started, delay, started));
        assert!(!reset_finished(
            started,
            delay,
            started + Duration::from_millis(99)
        ));
        assert!(reset_finished(started, delay, started + delay));
        assert!(reset_finished(
            started,
            delay,
            started + Duration::from_secs(5)
        ));
    }
}
