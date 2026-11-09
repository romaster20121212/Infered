 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::entity::player::Player;
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::MappedObject;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
const DEFAULT_HEALTH: f32 = 6.0;
const MIN_HEALTH: f32 = 1.0;
const MAX_HEALTH: f32 = 20.0;
const DEFAULT_RANGE: f32 = 16.0;
const MIN_RANGE: f32 = 2.0;
const MAX_RANGE: f32 = 64.0;
const DEFAULT_DELAY_SECONDS: f32 = 0.75;
const MAX_DELAY_SECONDS: f32 = 10.0;

 
 
static LOGGED: AtomicBool = AtomicBool::new(false);

 
 
fn should_log(health: f32, threshold: f32, require_players: bool, players_nearby: bool) -> bool {
    health <= threshold && (!require_players || players_nearby)
}

 
fn within_range(dx: f64, dy: f64, dz: f64, range: f64) -> bool {
    dx * dx + dy * dy + dz * dz <= range * range
}

 
 
fn claim_log() -> bool {
    !LOGGED.swap(true, Ordering::SeqCst)
}

#[derive(Debug)]
pub struct AutoLogModule {
    pub module: ModuleData,
     
    armed_since: Mutex<Option<Instant>>,
}

impl AutoLogModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoLog,
                description: "Presses Panic automatically at low health — optionally only \
                              while another player is nearby."
                    .to_string(),
                category: ModuleCategory::Misc,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Health".to_string(),
                        value: DEFAULT_HEALTH,
                        min: MIN_HEALTH,
                        max: MAX_HEALTH,
                    },
                    ModuleSetting::Toggle {
                        name: "Players".to_string(),
                        value: false,
                    },
                    ModuleSetting::Slider {
                        name: "Range".to_string(),
                        value: DEFAULT_RANGE,
                        min: MIN_RANGE,
                        max: MAX_RANGE,
                    },
                    ModuleSetting::Slider {
                        name: "Delay".to_string(),
                        value: DEFAULT_DELAY_SECONDS,
                        min: 0.0,
                        max: MAX_DELAY_SECONDS,
                    },
                ],
            },
            armed_since: Mutex::new(None),
        }
    }

    fn slider(&self, name: &str, fallback: f32, min: f32, max: f32) -> f32 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .unwrap_or(fallback)
            .clamp(min, max)
    }

    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(fallback)
    }

    fn threshold(&self) -> f32 {
        self.slider("Health", DEFAULT_HEALTH, MIN_HEALTH, MAX_HEALTH)
    }

    fn range(&self) -> f64 {
        f64::from(self.slider("Range", DEFAULT_RANGE, MIN_RANGE, MAX_RANGE))
    }

    fn delay(&self) -> Duration {
        Duration::from_secs_f32(self.slider("Delay", DEFAULT_DELAY_SECONDS, 0.0, MAX_DELAY_SECONDS))
    }

     
     
     
    fn players_nearby(&self) -> bool {
        let Ok(Some(world)) = minecraft().world() else {
            return false;
        };
        let Ok(Some(local)) = minecraft().player() else {
            return false;
        };
        let (Ok(local_id), Ok(center)) = (local.id(), local.get_position()) else {
            return false;
        };
        let range = self.range();
        let Ok(entities) = world.get_entities() else {
            return false;
        };
        entities.iter().any(|entity| {
            if !entity.instance_of::<Player>() {
                return false;
            }
            if entity.id().is_ok_and(|id| id == local_id) {
                return false;
            }
            let Ok(pos) = entity.get_position() else {
                return false;
            };
            within_range(
                pos.x() - center.x(),
                pos.y() - center.y(),
                pos.z() - center.z(),
                range,
            )
        })
    }

     
     
    fn disarm(&self) {
        *self.armed_since.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }
}

impl Module for AutoLogModule {
    fn on_start(&self) -> anyhow::Result<()> {
        LOGGED.store(false, Ordering::SeqCst);
        self.disarm();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        LOGGED.store(false, Ordering::SeqCst);
        self.disarm();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        if LOGGED.load(Ordering::SeqCst) {
            return Ok(());
        }
        let Some(player) = minecraft().player()? else {
            self.disarm();
            return Ok(());
        };
        let health = player.get_health()?;
        let require_players = self.toggle("Players", false);
        let nearby = !require_players || self.players_nearby();
        if !should_log(health, self.threshold(), require_players, nearby) {
            self.disarm();
            return Ok(());
        }

        let now = Instant::now();
        let mut armed = self.armed_since.lock().unwrap_or_else(|p| p.into_inner());
        let since = *armed.get_or_insert(now);
        if now.duration_since(since) < self.delay() {
            return Ok(());
        }
        drop(armed);

        if claim_log() {
            log::info!(
                "[AutoLog] health {health:.1} at or below {:.1} — pressing Panic",
                self.threshold()
            );
             
             
             
            std::thread::spawn(crate::graphic::ui_engine::call_panic);
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
    fn ships_disabled_with_the_documented_settings() {
        let module = AutoLogModule::new();
        assert_eq!(module.module.id, ModuleId::AutoLog);
        assert_eq!(module.module.category, ModuleCategory::Misc);
        assert!(!module.module.enabled);
        let names: Vec<&str> = module
            .module
            .settings
            .iter()
            .map(|setting| setting.name())
            .collect();
        assert_eq!(names, vec!["Health", "Players", "Range", "Delay"]);
        assert_eq!(module.threshold(), 6.0);
        assert!(!module.toggle("Players", true));
        assert_eq!(module.range(), 16.0);
        assert_eq!(module.delay(), Duration::from_millis(750));
    }

    #[test]
    fn only_health_at_or_below_the_threshold_logs() {
        assert!(!should_log(6.1, 6.0, false, false));
        assert!(should_log(6.0, 6.0, false, false));
        assert!(should_log(1.0, 6.0, false, false));
    }

    #[test]
    fn the_players_requirement_only_applies_when_it_is_on() {
        assert!(
            should_log(4.0, 6.0, false, false),
            "no requirement, no players"
        );
        assert!(
            !should_log(4.0, 6.0, true, false),
            "players required but none"
        );
        assert!(should_log(4.0, 6.0, true, true));
    }

    #[test]
    fn the_range_check_is_inclusive_and_three_dimensional() {
        assert!(within_range(0.0, 0.0, 0.0, 16.0));
        assert!(within_range(16.0, 0.0, 0.0, 16.0));
        assert!(!within_range(16.1, 0.0, 0.0, 16.0));
         
        assert!(within_range(0.0, 9.0, 12.0, 15.0));
    }

    #[test]
    fn the_settings_are_clamped_and_never_nan() {
        let mut module = AutoLogModule::new();
        module.module.settings[0].set_slider_value(99.0);
        assert_eq!(module.threshold(), MAX_HEALTH);
        module.module.settings[2].set_slider_value(f32::NAN);
        assert_eq!(module.range(), f64::from(DEFAULT_RANGE));
        module.module.settings[3].set_slider_value(-3.0);
        assert_eq!(module.delay(), Duration::ZERO);
    }

    #[test]
    fn a_log_is_claimed_once_then_rearmed_on_stop() {
        LOGGED.store(false, Ordering::SeqCst);
        assert!(claim_log());
        assert!(!claim_log(), "a second tick must not press Panic again");
         
        LOGGED.store(false, Ordering::SeqCst);
        assert!(claim_log());
        LOGGED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn disarming_forgets_the_pending_countdown() {
        let module = AutoLogModule::new();
        *module.armed_since.lock().unwrap() = Some(Instant::now());
        module.disarm();
        assert!(module.armed_since.lock().unwrap().is_none());
    }
}
