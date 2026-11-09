use crate::mapping::entity::EntityRef;
use crate::module::player::autototem::roll;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
 
const MOVING_THRESHOLD: f64 = 0.03;

 
const MODE_HOLD: usize = 0;

 
const MAX_INTERVAL_TICKS: f32 = 40.0;

 
#[derive(Debug, Default)]
struct SneakState {
     
    held: bool,
     
    resume_wait: u32,
     
    cycle_tick: u32,
     
    cycle_len: u32,
}

 
 
 
 
 
fn pulse_step(
    state: &mut SneakState,
    interval: u32,
    length: u32,
    jitter: u32,
    roll_value: f64,
) -> bool {
    if state.cycle_tick >= state.cycle_len {
        let extra = ((roll_value * (jitter + 1) as f64) as u32).min(jitter);
        state.cycle_len = (interval + extra).max(1);
        state.cycle_tick = 0;
    }
    let holding = state.cycle_tick < length;
    state.cycle_tick += 1;
    holding
}

 
 
#[derive(Debug)]
pub struct SneakModule {
    pub module: ModuleData,
    state: Mutex<SneakState>,
}

impl SneakModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Sneak,
                description: "Holds or pulses sneak, pausing while screens are open".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Choice {
                        name: "Mode".to_string(),
                        value: MODE_HOLD,
                        options: vec!["Hold".to_string(), "Pulse".to_string()],
                    },
                    ModuleSetting::Slider {
                        name: "Pulse Interval (ticks)".to_string(),
                        value: 10.0,
                        min: 2.0,
                        max: MAX_INTERVAL_TICKS,
                    },
                    ModuleSetting::Slider {
                        name: "Pulse Length (ticks)".to_string(),
                        value: 2.0,
                        min: 1.0,
                        max: 20.0,
                    },
                    ModuleSetting::Slider {
                        name: "Pulse Jitter (ticks)".to_string(),
                        value: 0.0,
                        min: 0.0,
                        max: 10.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Stop On Screen".to_string(),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: "Resume Delay (ticks)".to_string(),
                        value: 0.0,
                        min: 0.0,
                        max: 40.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Only While Moving".to_string(),
                        value: false,
                    },
                    ModuleSetting::Toggle {
                        name: "Only On Ground".to_string(),
                        value: false,
                    },
                    ModuleSetting::Toggle {
                        name: "Stop In Water".to_string(),
                        value: false,
                    },
                ],
            },
            state: Mutex::new(SneakState::default()),
        }
    }

    fn toggle(&self, name: &str) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(false)
    }

     
    fn ticks(&self, name: &str) -> u32 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(0.0)
            .round()
            .max(0.0) as u32
    }

    fn mode(&self) -> usize {
        match self.module.get_setting("Mode") {
            Some(ModuleSetting::Choice { value, .. }) => *value,
            _ => MODE_HOLD,
        }
    }

     
     
    fn desired_sneak(&self, state: &mut SneakState) -> anyhow::Result<bool> {
        let mc = minecraft();

        if self.toggle("Stop On Screen") && mc.current_screen().is_open() {
            state.resume_wait = self.ticks("Resume Delay (ticks)");
            state.cycle_tick = 0;
            return Ok(false);
        }
        if state.resume_wait > 0 {
            state.resume_wait -= 1;
            state.cycle_tick = 0;
            return Ok(false);
        }

        let Some(player) = mc.player()? else {
            state.cycle_tick = 0;
            return Ok(false);
        };

        if self.toggle("Stop In Water") && player.is_in_water()? {
            state.cycle_tick = 0;
            return Ok(false);
        }
        if self.toggle("Only On Ground") && !player.on_ground()? {
            state.cycle_tick = 0;
            return Ok(false);
        }
        if self.toggle("Only While Moving") {
            let motion = player.get_delta_movement()?;
            let horizontal = (motion.x() * motion.x() + motion.z() * motion.z()).sqrt();
            if horizontal <= MOVING_THRESHOLD {
                state.cycle_tick = 0;
                return Ok(false);
            }
        }

        if self.mode() == MODE_HOLD {
            return Ok(true);
        }
        Ok(pulse_step(
            state,
            self.ticks("Pulse Interval (ticks)").max(1),
            self.ticks("Pulse Length (ticks)"),
            self.ticks("Pulse Jitter (ticks)"),
            roll(),
        ))
    }

     
    fn set_held(state: &mut SneakState, want: bool) -> anyhow::Result<()> {
        if state.held == want {
            return Ok(());
        }
        minecraft().options()?.key_shift()?.set_down(want)?;
        state.held = want;
        Ok(())
    }
}

impl Module for SneakModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if let Ok(mut state) = self.state.lock() {
            *state = SneakState::default();
        }
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
        let Ok(mut state) = self.state.lock() else {
            return Ok(());
        };
        if state.held {
            minecraft().options()?.key_shift()?.set_down(false)?;
        }
        *state = SneakState::default();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Ok(mut state) = self.state.lock() else {
            return Ok(());
        };
        let want = self.desired_sneak(&mut state)?;
        Self::set_held(&mut state, want)
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

    fn run(
        state: &mut SneakState,
        ticks: usize,
        interval: u32,
        length: u32,
        jitter: u32,
        r: f64,
    ) -> Vec<bool> {
        (0..ticks)
            .map(|_| pulse_step(state, interval, length, jitter, r))
            .collect()
    }

    #[test]
    fn pulse_holds_for_length_then_releases_each_cycle() {
        let mut state = SneakState::default();
        let out = run(&mut state, 12, 5, 2, 0, 0.0);
        assert_eq!(
            out,
            vec![true, true, false, false, false, true, true, false, false, false, true, true]
        );
    }

    #[test]
    fn pulse_jitter_stays_within_bounds() {
         
        let mut state = SneakState::default();
        let _ = run(&mut state, 1, 5, 1, 3, 0.999);
        assert_eq!(state.cycle_len, 8);

        let mut state = SneakState::default();
        let _ = run(&mut state, 1, 5, 1, 3, 0.0);
        assert_eq!(state.cycle_len, 5);
    }

    #[test]
    fn default_settings_are_hold_mode_with_screen_stop() {
        let module = SneakModule::new();
        assert_eq!(module.mode(), MODE_HOLD);
        assert!(module.toggle("Stop On Screen"));
        assert!(!module.toggle("Only On Ground"));
        assert_eq!(module.ticks("Resume Delay (ticks)"), 0);
    }
}
