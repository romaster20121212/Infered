use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
 
 
 
 
#[derive(Debug)]
pub struct MobEspModule {
    pub module: ModuleData,
}

impl MobEspModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::MobEsp,
                description: "Draws a clean box with corner brackets or a rounded animated \
                              frame, a health bar and a name/distance label around mobs"
                    .to_string(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Color {
                        name: "Color".to_string(),
                        value: [1.0, 0.0, 0.0, 1.0],
                    },
                    ModuleSetting::Choice {
                        name: "Style".to_string(),
                        value: 0,
                        options: crate::module::render::player_esp::BOX_STYLES
                            .iter()
                            .map(|style| style.to_string())
                            .collect(),
                    },
                    ModuleSetting::Toggle {
                        name: "Animate".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Glow".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Name".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Distance".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Health".to_string(),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: "Range".to_string(),
                        value: 64.0,
                        min: 16.0,
                        max: 256.0,
                    },
                ],
            },
        }
    }
}

impl Module for MobEspModule {
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
