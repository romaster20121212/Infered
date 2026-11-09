 
 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
const DEFAULT_CAP_RADIUS: i32 = 2;
 
const MIN_CAP_RADIUS: f32 = 0.0;
 
const MAX_CAP_RADIUS: f32 = 7.0;

 
 
#[derive(Debug, Clone, Copy, PartialEq)]
struct Clamp {
    original: i32,
    applied: i32,
}

#[derive(Debug)]
pub struct BiomeBlendCapModule {
    pub module: ModuleData,
    clamp: Mutex<Option<Clamp>>,
}

impl BiomeBlendCapModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::BiomeBlendCap,
                description: "Caps the biome blend radius so chunks sample fewer biome \
                              colours when they are meshed. Only lowers the option, and \
                              restores it when disabled."
                    .to_string(),
                category: ModuleCategory::Performance,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Radius".to_string(),
                    value: DEFAULT_CAP_RADIUS as f32,
                    min: MIN_CAP_RADIUS,
                    max: MAX_CAP_RADIUS,
                }],
            },
            clamp: Mutex::new(None),
        }
    }

    fn cap_radius(&self) -> i32 {
        self.module
            .get_setting("Radius")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(DEFAULT_CAP_RADIUS as f32)
            .round() as i32
    }

     
    fn apply(&self) -> anyhow::Result<()> {
        let cap = self.cap_radius();
        let option = minecraft().options()?.biome_blend_radius()?;
        let current = option.get_int()?;
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
        option.force_int(cap)
    }

     
    fn restore(&self) -> anyhow::Result<()> {
        let Some(clamp) = *self.clamp.lock().unwrap() else {
            return Ok(());
        };
        let option = minecraft().options()?.biome_blend_radius()?;
        if option.get_int().ok() == Some(clamp.applied) {
            option.force_int(clamp.original)?;
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

impl Module for BiomeBlendCapModule {
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
    fn defaults_to_disabled_performance_module_with_radius_slider() {
        let module = BiomeBlendCapModule::new();
        assert_eq!(module.module.id, ModuleId::BiomeBlendCap);
        assert_eq!(module.module.category, ModuleCategory::Performance);
        assert!(!module.module.enabled);
        assert_eq!(module.cap_radius(), DEFAULT_CAP_RADIUS);
    }

    #[test]
    fn cap_radius_rounds_slider_value() {
        let mut module = BiomeBlendCapModule::new();
        let setting = module.module.get_setting_mut("Radius").unwrap();
        setting.set_slider_value(4.6);
        assert_eq!(module.cap_radius(), 5);
    }
}
