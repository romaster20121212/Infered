 
 
 
 
 
 
 

use crate::graphic::input::GUI_OPEN;
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

 
#[derive(Debug, Default)]
pub(crate) struct ResetState {
     
     
    last_hurt_time: i32,
     
    wait: u32,
     
    pressing: bool,
}

 
 
pub(crate) fn step(
    state: &mut ResetState,
    hurt_time: i32,
    on_ground: bool,
    delay_ticks: u32,
) -> bool {
    if hurt_time > state.last_hurt_time {
        state.wait = delay_ticks.saturating_add(1);
    }
    state.last_hurt_time = hurt_time;

    if state.pressing {
        state.pressing = false;
        return false;  
    }
    if state.wait > 0 {
        state.wait -= 1;
        if state.wait == 0 && on_ground {
            state.pressing = true;
            return true;
        }
    }
    false
}

 
#[derive(Debug)]
pub struct JumpResetModule {
    pub module: ModuleData,
    state: Mutex<ResetState>,
}

impl JumpResetModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::JumpReset,
                description: "Jumps when damage lands".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Delay (ticks)".to_string(),
                    value: 0.0,
                    min: 0.0,
                    max: 4.0,
                }],
            },
            state: Mutex::new(ResetState::default()),
        }
    }

    fn delay(&self) -> u32 {
        self.module
            .get_setting("Delay (ticks)")
            .and_then(|setting| setting.get_slider_value())
            .map(|value| value.max(0.0).round() as u32)
            .unwrap_or(0)
    }

    fn apply(&self, held: &mut bool, want: bool) -> anyhow::Result<()> {
        let key = minecraft().options()?.key_jump()?;
        if want {
            if !*held && !key.is_down()? {
                key.set_down(true)?;
                *held = true;
            }
        } else if *held {
            key.set_down(false)?;
            *held = false;
        }
        Ok(())
    }
}

impl Module for JumpResetModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if let Ok(mut state) = self.state.lock() {
            *state = ResetState::default();
        }
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        let Ok(mut state) = self.state.lock() else {
            return Ok(());
        };
        let mut held = state.pressing;
        state.pressing = false;
        state.wait = 0;
        self.apply(&mut held, false)
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Ok(mut state) = self.state.lock() else {
            return Ok(());
        };
        let mc = minecraft();

        let Some(player) = mc.player()? else {
            return Ok(());
        };
        if mc.current_screen().is_open() || GUI_OPEN.load(Ordering::Relaxed) {
            state.wait = 0;
            let mut held = state.pressing;
            state.pressing = false;
            return self.apply(&mut held, false);
        }

        let hurt_time = player.hurt_time()?;
        let on_ground = player.on_ground()?;
        let mut held = state.pressing;
        let want = step(&mut state, hurt_time, on_ground, self.delay());
        self.apply(&mut held, want)?;
         
         
        state.pressing = held;
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
    use super::{step, ResetState};

    #[test]
    fn the_first_nonzero_damage_counter_counts_as_a_hit() {
        let mut state = ResetState::default();
         
         
        assert!(step(&mut state, 10, true, 0));
         
        assert!(!step(&mut state, 9, true, 0));
    }

    #[test]
    fn a_new_hit_after_the_countdown_jumps() {
        let mut state = ResetState::default();
         
        assert!(!step(&mut state, 0, true, 0));
         
        assert!(step(&mut state, 10, true, 0));
         
        assert!(!step(&mut state, 10, true, 0));
    }

    #[test]
    fn the_jump_is_skipped_in_the_air() {
        let mut state = ResetState::default();
        assert!(!step(&mut state, 0, false, 0));
         
        assert!(!step(&mut state, 10, false, 0));
         
        assert!(!step(&mut state, 9, true, 0));
    }

    #[test]
    fn the_delay_shifts_the_press() {
        let mut state = ResetState::default();
        assert!(!step(&mut state, 0, true, 2));
         
        assert!(!step(&mut state, 10, true, 2));
        assert!(!step(&mut state, 10, true, 2));
        assert!(step(&mut state, 10, true, 2));
        assert!(!step(&mut state, 10, true, 2));
    }

    #[test]
    fn the_endless_hurt_countdown_does_not_retrigger() {
        let mut state = ResetState::default();
        assert!(!step(&mut state, 0, true, 0));
        let mut presses = 0;
         
        for hurt in (0..=10).rev() {
            if step(&mut state, hurt, true, 0) {
                presses += 1;
            }
        }
        assert_eq!(presses, 1);
    }
}
