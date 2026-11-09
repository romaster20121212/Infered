 
 
 
 
 
 
 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
const DEFAULT_INTERVAL_SECONDS: f32 = 60.0;
const MIN_INTERVAL_SECONDS: f32 = 5.0;
const MAX_INTERVAL_SECONDS: f32 = 600.0;

 
 
static SAVING: AtomicBool = AtomicBool::new(false);

 
 
fn save_due(last: Option<Instant>, now: Instant, interval: Duration) -> bool {
    match last {
        None => true,
        Some(last) => now.duration_since(last) >= interval,
    }
}

#[derive(Debug)]
pub struct AutoSaveModule {
    pub module: ModuleData,
     
    last_save: Mutex<Option<Instant>>,
}

impl AutoSaveModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoSave,
                description: "Saves the config and HUD layout on a timer, and when you \
                              leave a world, so a long session or a crash never loses edits."
                    .to_string(),
                category: ModuleCategory::Misc,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Interval".to_string(),
                        value: DEFAULT_INTERVAL_SECONDS,
                        min: MIN_INTERVAL_SECONDS,
                        max: MAX_INTERVAL_SECONDS,
                    },
                    ModuleSetting::Toggle {
                        name: "Save On Leave".to_string(),
                        value: true,
                    },
                ],
            },
            last_save: Mutex::new(None),
        }
    }

     
    fn interval(&self) -> Duration {
        let seconds = self
            .module
            .get_setting("Interval")
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .unwrap_or(DEFAULT_INTERVAL_SECONDS)
            .clamp(MIN_INTERVAL_SECONDS, MAX_INTERVAL_SECONDS);
        Duration::from_secs_f32(seconds)
    }

    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(fallback)
    }

     
     
     
     
     
     
     
    fn save_now(&self, reason: &'static str) {
        if SAVING.swap(true, Ordering::SeqCst) {
            return;
        }
        *self.last_save.lock().unwrap_or_else(|p| p.into_inner()) = Some(Instant::now());
        std::thread::spawn(move || {
             
             
             
            if crate::RUNNING.load(Ordering::SeqCst) {
                crate::config::save();
                log::debug!("[AutoSave] saved config ({reason})");
            }
            SAVING.store(false, Ordering::SeqCst);
        });
    }
}

impl Module for AutoSaveModule {
    fn on_start(&self) -> anyhow::Result<()> {
         
         
        *self.last_save.lock().unwrap_or_else(|p| p.into_inner()) = Some(Instant::now());
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
        if self.toggle("Save On Leave", true) && crate::RUNNING.load(Ordering::SeqCst) {
            self.save_now("world left");
        }
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let last = *self.last_save.lock().unwrap_or_else(|p| p.into_inner());
        if save_due(last, Instant::now(), self.interval()) {
            self.save_now("timer");
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
    fn ships_enabled_off_with_the_documented_settings() {
        let module = AutoSaveModule::new();
        assert_eq!(module.module.id, ModuleId::AutoSave);
        assert_eq!(module.module.category, ModuleCategory::Misc);
        assert!(!module.module.enabled);
        let names: Vec<&str> = module
            .module
            .settings
            .iter()
            .map(|setting| setting.name())
            .collect();
        assert_eq!(names, vec!["Interval", "Save On Leave"]);
        assert_eq!(module.interval(), Duration::from_secs(60));
        assert!(module.toggle("Save On Leave", false));
    }

    #[test]
    fn the_interval_is_clamped_and_never_nan() {
        let mut module = AutoSaveModule::new();
        module.module.settings[0].set_slider_value(0.0);
        assert_eq!(module.interval(), Duration::from_secs(5));
        module.module.settings[0].set_slider_value(10_000.0);
        assert_eq!(module.interval(), Duration::from_secs(600));
        module.module.settings[0].set_slider_value(f32::INFINITY);
        assert_eq!(module.interval(), Duration::from_secs(60));
    }

    #[test]
    fn a_save_is_due_immediately_then_only_after_the_interval() {
        let now = Instant::now();
        let interval = Duration::from_secs(30);
        assert!(save_due(None, now, interval));
        assert!(!save_due(
            Some(now),
            now + Duration::from_secs(29),
            interval
        ));
        assert!(save_due(Some(now), now + Duration::from_secs(30), interval));
    }

    #[test]
    fn only_one_save_runs_at_a_time() {
        SAVING.store(false, Ordering::SeqCst);
        assert!(!SAVING.swap(true, Ordering::SeqCst));
         
         
        assert!(SAVING.swap(true, Ordering::SeqCst));
        SAVING.store(false, Ordering::SeqCst);
    }

    #[test]
    fn starting_the_module_resets_the_timer() {
        let module = AutoSaveModule::new();
        module.on_start().unwrap();
        let last = *module.last_save.lock().unwrap();
        assert!(last.is_some(), "on_start must arm the timer");
        assert!(!save_due(last, Instant::now(), module.interval()));
    }
}
