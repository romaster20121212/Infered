use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
 
 
 
 
#[derive(Debug)]
pub struct SaturationModule {
    pub module: ModuleData,
}

impl SaturationModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Saturation,
                description: "Raises or lowers the saturation of the whole window".to_string(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Saturation".to_string(),
                    value: 1.0,
                    min: 0.0,
                    max: 2.0,
                }],
            },
        }
    }
}

impl Module for SaturationModule {
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
