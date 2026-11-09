 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
#[derive(Debug)]
pub struct EspModule {
    pub module: ModuleData,
}

impl EspModule {
    fn new(id: ModuleId, description: &str, settings: Vec<ModuleSetting>) -> Self {
        Self {
            module: ModuleData {
                id,
                description: description.to_string(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings,
            },
        }
    }
}

impl Module for EspModule {
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

 
pub fn item_esp() -> EspModule {
    EspModule::new(
        ModuleId::ItemEsp,
        "Highlights dropped items",
        vec![
            ModuleSetting::Color {
                name: "Color".to_string(),
                value: [1.0, 0.0, 0.0, 1.0],
            },
            ModuleSetting::Toggle {
                name: "Name".to_string(),
                value: true,
            },
            ModuleSetting::Slider {
                name: "Range".to_string(),
                value: 48.0,
                min: 8.0,
                max: 128.0,
            },
        ],
    )
}

 
 
pub fn projectiles() -> EspModule {
    EspModule::new(
        ModuleId::Projectiles,
        "Highlights projectiles — arrows, tridents, fireballs, pearls and more",
        vec![
            ModuleSetting::Color {
                name: "Color".to_string(),
                value: [1.0, 0.0, 0.0, 1.0],
            },
            ModuleSetting::Toggle {
                name: "Name".to_string(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Distance".to_string(),
                value: true,
            },
            ModuleSetting::Slider {
                name: "Range".to_string(),
                value: 96.0,
                min: 8.0,
                max: 128.0,
            },
        ],
    )
}

 
 
pub fn spawner_finder() -> EspModule {
    EspModule::new(
        ModuleId::SpawnerFinder,
        "Highlights mob spawners and notifies you when one is found",
        vec![
            ModuleSetting::Color {
                name: "Color".to_string(),
                value: [1.0, 0.0, 0.0, 1.0],
            },
            ModuleSetting::Toggle {
                name: "Distance".to_string(),
                value: true,
            },
             
             
            ModuleSetting::Toggle {
                name: "Notify".to_string(),
                value: true,
            },
            ModuleSetting::Slider {
                name: "Notify Range".to_string(),
                value: 64.0,
                min: 8.0,
                max: 128.0,
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
    )
}
