use crate::mapping::entity::PlayerRef;
use crate::module::player::autototem::roll;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
const MIN_DELAY_MS: f32 = 50.0;
 
const MAX_DELAY_MS: f32 = 1000.0;

#[derive(Debug, Default)]
struct ClickerState {
    last_click: Option<Instant>,
     
    next_delay: Duration,
}

 
 
 
 
 
#[derive(Debug)]
pub struct AutoClickerModule {
    pub module: ModuleData,
    state: Mutex<ClickerState>,
}

impl AutoClickerModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoClicker,
                description: "Automatically clicks while attacking".to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "CPS".to_string(),
                        value: 10.0,
                        min: 1.0,
                        max: 20.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Random Delay".to_string(),
                        value: false,
                    },
                    ModuleSetting::Slider {
                        name: "Min Delay (ms)".to_string(),
                        value: 80.0,
                        min: MIN_DELAY_MS,
                        max: MAX_DELAY_MS,
                    },
                    ModuleSetting::Slider {
                        name: "Max Delay (ms)".to_string(),
                        value: 120.0,
                        min: MIN_DELAY_MS,
                        max: MAX_DELAY_MS,
                    },
                    ModuleSetting::Toggle {
                        name: "Cooldown".to_string(),
                        value: true,
                    },
                ],
            },
            state: Mutex::new(ClickerState::default()),
        }
    }

    fn slider(&self, name: &str, fallback: f32) -> f32 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(fallback)
    }

     
    fn click_interval(&self, cps: f32) -> Duration {
        Duration::from_secs_f32(1.0 / cps.clamp(1.0, 20.0))
    }

     
     
    fn pick_interval(&self) -> Duration {
        let random_on = self
            .module
            .get_setting("Random Delay")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(false);
        if random_on {
            random_interval(
                self.slider("Min Delay (ms)", 80.0),
                self.slider("Max Delay (ms)", 120.0),
                roll(),
            )
        } else {
            self.click_interval(self.slider("CPS", 10.0))
        }
    }
}

 
 
 
fn random_interval(first_ms: f32, second_ms: f32, roll: f64) -> Duration {
    let clean = |value: f32| -> f64 {
        if value.is_finite() {
            value.clamp(MIN_DELAY_MS, MAX_DELAY_MS) as f64
        } else {
            MIN_DELAY_MS as f64
        }
    };
    let (first, second) = (clean(first_ms), clean(second_ms));
    let (low, high) = if first <= second {
        (first, second)
    } else {
        (second, first)
    };
    let ms = low + (high - low) * roll.clamp(0.0, 1.0);
    Duration::from_secs_f64(ms / 1000.0)
}

impl Module for AutoClickerModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        *self.state.lock().unwrap() = ClickerState::default();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
         
        if crate::module::world::autoeat::is_eating()
            || crate::graphic::input::GUI_OPEN.load(std::sync::atomic::Ordering::Relaxed)
        {
            *self.state.lock().unwrap() = ClickerState::default();
            return Ok(());
        }
         
         
         
         
        let minecraft = minecraft();
        if minecraft.current_screen().is_open() {
            *self.state.lock().unwrap() = ClickerState::default();
            return Ok(());
        }
        let Some(player) = minecraft.player()? else {
            return Ok(());
        };

         
        if crate::mapping::entity::LivingEntityRef::is_using_item(&player)? {
            *self.state.lock().unwrap() = ClickerState::default();
            return Ok(());
        }

         
        let (last, delay) = {
            let state = self.state.lock().unwrap();
            (state.last_click, state.next_delay)
        };
        let ready = last.is_none_or(|previous| previous.elapsed() >= delay);
        if !ready {
            return Ok(());
        }
        let cooldown_on = self
            .module
            .get_setting("Cooldown")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(true);
        if cooldown_on && player.attack_strength_scale()? < 1.0 {
            return Ok(());
        }

         
         
         
         
         
        if crate::module::combat::hitselect::take_click() {
            let next_delay = self.pick_interval();
            let mut state = self.state.lock().unwrap();
            state.last_click = Some(Instant::now());
            state.next_delay = next_delay;
            return Ok(());
        }

         
         
         
        minecraft.start_attack()?;
        let next_delay = self.pick_interval();
        let mut state = self.state.lock().unwrap();
        state.last_click = Some(Instant::now());
        state.next_delay = next_delay;
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
    fn click_interval_matches_cps() {
        let module = AutoClickerModule::new();
        assert!((module.click_interval(10.0).as_secs_f64() - 0.1).abs() < 0.0001);
        assert!((module.click_interval(20.0).as_secs_f64() - 0.05).abs() < 0.0001);
    }

     
    #[test]
    fn click_interval_never_divides_by_zero() {
        let module = AutoClickerModule::new();
        assert!(module.click_interval(0.0).as_secs_f64().is_finite());
    }

     
    #[test]
    fn random_delay_stays_between_the_two_sliders() {
        for step in 0..=100 {
            let roll = step as f64 / 100.0;
            let ms = random_interval(80.0, 120.0, roll).as_secs_f64() * 1000.0;
            assert!((80.0..=120.0).contains(&ms), "{ms} ms outside 80-120");
            let reversed = random_interval(120.0, 80.0, roll).as_secs_f64() * 1000.0;
            assert!(
                (80.0..=120.0).contains(&reversed),
                "{reversed} ms outside 80-120"
            );
        }
    }

     
    #[test]
    fn random_delay_spans_exactly_the_configured_range() {
        assert_eq!(
            random_interval(200.0, 300.0, 0.0),
            Duration::from_millis(200)
        );
        let top = random_interval(200.0, 300.0, 0.999_999).as_secs_f64() * 1000.0;
        assert!((299.9..=300.0).contains(&top), "{top} ms");
    }

     
    #[test]
    fn random_delay_is_clamped_to_the_slider_range() {
        assert_eq!(random_interval(0.0, 10.0, 0.5), Duration::from_millis(50));
        assert_eq!(
            random_interval(f32::NAN, 100.0, 0.0),
            Duration::from_millis(50)
        );
    }
}
