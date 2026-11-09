 
 
 
 
 
 

use crate::graphic::dev_popup;
use crate::graphic::notification::{Notification, NotificationType};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use std::sync::atomic::{AtomicBool, Ordering};

 
 
static CLAIMED: AtomicBool = AtomicBool::new(false);

 
const DEFAULT_SECONDS: f32 = 4.0;

#[derive(Debug)]
pub struct DevModeModule {
    pub module: ModuleData,
}

impl DevModeModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::DevMode,
                description: "Devtest build: opens the test popup and fires a test notification"
                    .to_string(),
                category: ModuleCategory::Dev,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Toggle {
                        name: "Test Popup".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Test Notification".to_string(),
                        value: true,
                    },
                    ModuleSetting::Choice {
                        name: "Notification Type".to_string(),
                        value: 0,
                        options: vec![
                            "Info".to_string(),
                            "Warning".to_string(),
                            "Alert".to_string(),
                        ],
                    },
                    ModuleSetting::Slider {
                        name: "Notification Seconds".to_string(),
                        value: DEFAULT_SECONDS,
                        min: 1.0,
                        max: 12.0,
                    },
                ],
            },
        }
    }

     
    fn wants_popup(&self) -> bool {
        self.module
            .get_setting("Test Popup")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(true)
    }

     
    fn wants_notification(&self) -> bool {
        self.module
            .get_setting("Test Notification")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(true)
    }

     
    fn notification_type(&self) -> NotificationType {
        let index = match self.module.get_setting("Notification Type") {
            Some(ModuleSetting::Choice { value, .. }) => *value,
            _ => 0,
        };
        match index {
            1 => NotificationType::Warning,
            2 => NotificationType::Alert,
            _ => NotificationType::Info,
        }
    }

     
    fn seconds(&self) -> f32 {
        self.module
            .get_setting("Notification Seconds")
            .and_then(|setting| setting.get_slider_value())
            .filter(|seconds| seconds.is_finite())
            .unwrap_or(DEFAULT_SECONDS)
            .clamp(1.0, 12.0)
    }

     
    fn fire(&self) {
        if CLAIMED.swap(true, Ordering::SeqCst) {
            return;
        }
        if self.wants_popup() {
            dev_popup::open();
        }
        if self.wants_notification() {
            let kind = self.notification_type();
            let label = match kind {
                NotificationType::Info => "info",
                NotificationType::Warning => "warning",
                NotificationType::Alert => "alert",
            };
            Notification::send_with_time(
                kind,
                "Dev Mode",
                &format!("test {label} notification — the overlay is live"),
                self.seconds(),
            );
        }
    }
}

impl Module for DevModeModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.fire();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
        dev_popup::close();
        CLAIMED.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
        self.fire();
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
    fn defaults_fire_both_triggers_with_a_bounded_duration() {
        let module = DevModeModule::new();
        assert!(module.wants_popup());
        assert!(module.wants_notification());
        assert_eq!(module.notification_type(), NotificationType::Info);
        assert_eq!(module.seconds(), DEFAULT_SECONDS);

        let mut module = DevModeModule::new();
        if let Some(setting) = module.module.get_setting_mut("Notification Seconds") {
            setting.set_slider_value(f32::INFINITY);
        }
        assert_eq!(module.seconds(), DEFAULT_SECONDS);

        if let Some(setting) = module.module.get_setting_mut("Notification Type") {
            if let ModuleSetting::Choice { value, .. } = setting {
                *value = 2;
            }
        }
        assert_eq!(module.notification_type(), NotificationType::Alert);
    }
}
