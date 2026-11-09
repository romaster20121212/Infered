use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

#[derive(Debug)]
pub struct BreakVisualizerModule {
    pub module: ModuleData,
}

impl BreakVisualizerModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::BreakVisualizer,
                description: "A cube grows inside the block you are mining until it fills the block, then the block breaks"
                    .to_string(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Color {
                    name: "Color".to_string(),
                    value: [1.0, 0.25, 0.25, 0.9],
                }],
            },
        }
    }
}

impl Module for BreakVisualizerModule {
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
