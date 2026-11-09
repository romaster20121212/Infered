 
 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
const DEFAULT_CAP_FPS: f32 = 60.0;
 
const MIN_CAP_FPS: f32 = 30.0;
 
 
const MAX_CAP_FPS: f32 = 240.0;

 
 
 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Clamp {
    original: i32,
    applied: i32,
}

#[derive(Debug)]
pub struct FramerateCapModule {
    pub module: ModuleData,
    clamp: Mutex<Option<Clamp>>,
}

impl FramerateCapModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::FramerateCap,
                description: "Caps the max framerate to cut GPU load on frames you \
                              cannot see anyway. Only lowers the option, and restores \
                              it when disabled."
                    .to_string(),
                category: ModuleCategory::Performance,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Max FPS".to_string(),
                    value: DEFAULT_CAP_FPS,
                    min: MIN_CAP_FPS,
                    max: MAX_CAP_FPS,
                }],
            },
            clamp: Mutex::new(None),
        }
    }

    fn cap_fps(&self) -> i32 {
        self.module
            .get_setting("Max FPS")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(DEFAULT_CAP_FPS)
            .round() as i32
    }

     
    fn apply(&self) -> anyhow::Result<()> {
        let cap = self.cap_fps();
        let framerate_limit = minecraft().options()?.framerate_limit()?;
        let current = framerate_limit.get_int()?;
        if current <= cap {
            return Ok(());
        }
        {
            let mut clamp = self.clamp.lock().unwrap();
             
            let original = clamp.map_or(current, |c| c.original);
            *clamp = Some(Clamp {
                original,
                applied: cap,
            });
        }
        framerate_limit.force_int(cap)
    }

     
    fn restore(&self) -> anyhow::Result<()> {
        let Some(clamp) = *self.clamp.lock().unwrap() else {
            return Ok(());
        };
        let framerate_limit = minecraft().options()?.framerate_limit()?;
        if framerate_limit.get_int()? == clamp.applied {
            framerate_limit.force_int(clamp.original)?;
        }
        *self.clamp.lock().unwrap() = None;
        Ok(())
    }

     
    fn sync(&self) -> anyhow::Result<()> {
        if matches!(minecraft().world(), Ok(Some(_))) {
            self.apply()
        } else {
            self.restore()
        }
    }
}

impl Module for FramerateCapModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.sync()
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.restore()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        self.sync()
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        self.restore()
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
    fn defaults_to_disabled_performance_module() {
        let module = FramerateCapModule::new();
        assert_eq!(module.module.id, ModuleId::FramerateCap);
        assert_eq!(module.module.category, ModuleCategory::Performance);
        assert!(!module.module.enabled);
    }

    #[test]
    fn default_cap_is_within_slider_range() {
        assert!(DEFAULT_CAP_FPS >= MIN_CAP_FPS);
        assert!(DEFAULT_CAP_FPS <= MAX_CAP_FPS);
    }
}
