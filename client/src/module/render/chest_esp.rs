use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
 
 
 
 
 
#[derive(Debug)]
pub struct ChestEspModule {
    pub module: ModuleData,
}

impl ChestEspModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::ChestEsp,
                description: "Draws a 3D box around selected storage blocks".to_string(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::BlockList {
                        name: "Storage Blocks".to_string(),
                        value: vec![
                            "minecraft:chest".to_string(),
                            "minecraft:trapped_chest".to_string(),
                            "minecraft:ender_chest".to_string(),
                            "minecraft:barrel".to_string(),
                            "minecraft:shulker_box".to_string(),
                            "minecraft:hopper".to_string(),
                            "minecraft:dispenser".to_string(),
                            "minecraft:dropper".to_string(),
                            "minecraft:furnace".to_string(),
                            "minecraft:blast_furnace".to_string(),
                            "minecraft:smoker".to_string(),
                            "minecraft:brewing_stand".to_string(),
                            "minecraft:decorated_pot".to_string(),
                        ],
                    },
                    ModuleSetting::Color {
                        name: "Color".to_string(),
                        value: [1.0, 0.0, 0.0, 1.0],
                    },
                    ModuleSetting::Toggle {
                        name: "Distance".to_string(),
                        value: true,
                    },
                     
                     
                     
                    ModuleSetting::Toggle {
                        name: "Tracers".to_string(),
                        value: false,
                    },
                    ModuleSetting::Toggle {
                        name: "Shade".to_string(),
                        value: false,
                    },
                    ModuleSetting::Slider {
                        name: "Shade Opacity".to_string(),
                        value: 0.2,
                        min: 0.0,
                        max: 1.0,
                    },
                ],
            },
        }
    }
}

impl Module for ChestEspModule {
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
