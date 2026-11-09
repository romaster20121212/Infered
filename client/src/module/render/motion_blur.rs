use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
 
 
 
 
#[derive(Debug)]
pub struct MotionBlurModule {
    pub module: ModuleData,
}

impl MotionBlurModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::MotionBlur,
                description: "Smears moving objects along a direction".to_string(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Strength".to_string(),
                        value: 0.55,
                        min: 0.0,
                        max: 0.95,
                    },
                    ModuleSetting::Slider {
                        name: "Smear".to_string(),
                        value: 4.0,
                        min: 0.0,
                        max: 32.0,
                    },
                    ModuleSetting::Slider {
                        name: "Direction".to_string(),
                        value: 0.0,
                        min: 0.0,
                        max: 360.0,
                    },
                ],
            },
        }
    }
}

impl Module for MotionBlurModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
