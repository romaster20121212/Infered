use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

#[derive(Debug)]
pub struct BlockOutlineModule {
    pub module: ModuleData,
}

impl BlockOutlineModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::BlockOutline,
                description: "Thick box around the looked-at block within reach, with liquid edges and a faint fill that glide between blocks"
                    .to_string(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Color {
                        name: "Color".to_string(),
                        value: [1.0, 0.0, 0.0, 1.0],
                    },
                    ModuleSetting::Slider {
                        name: "Width".to_string(),
                        value: 6.0,
                        min: 0.5,
                        max: 12.0,
                    },
                ],
            },
        }
    }
}

impl Module for BlockOutlineModule {
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
