use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
 
 
 
 
 
#[derive(Debug)]
pub struct NametagsModule {
    pub module: ModuleData,
}

impl NametagsModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Nametags,
                description: "Shows a name tag above nearby players".to_string(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Color {
                        name: "Color".to_string(),
                        value: [1.0, 0.0, 0.0, 1.0],
                    },
                    ModuleSetting::Toggle {
                        name: "Health".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Distance".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Gear".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Armor".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Hands".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Durability".to_string(),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: "Range".to_string(),
                        value: 64.0,
                        min: 16.0,
                        max: 256.0,
                    },
                    ModuleSetting::Slider {
                        name: "HUD Scale".to_string(),
                        value: 1.06,
                        min: 0.6,
                        max: 1.2,
                    },
                    ModuleSetting::Slider {
                        name: "HUD X Offset".to_string(),
                        value: -95.0,
                        min: -100.0,
                        max: 100.0,
                    },
                    ModuleSetting::Slider {
                        name: "HUD Y Offset".to_string(),
                        value: 0.0,
                        min: -100.0,
                        max: 100.0,
                    },
                ],
            },
        }
    }
}

impl Module for NametagsModule {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_show_gear_armor_hands_and_durability() {
        let module = NametagsModule::new();
        for name in ["Gear", "Armor", "Hands", "Durability"] {
            assert_eq!(
                module.module.get_setting(name).unwrap().get_toggle_value(),
                Some(true),
                "{name} should default on"
            );
        }
         
        assert_eq!(
            module
                .module
                .get_setting("Range")
                .unwrap()
                .get_slider_value(),
            Some(64.0)
        );
    }

     
     
    #[test]
    fn hud_scale_is_capped_at_a_legible_size() {
        let module = NametagsModule::new();
        match module.module.get_setting("HUD Scale") {
            Some(ModuleSetting::Slider { min, max, .. }) => {
                assert_eq!(*min, 0.6);
                assert_eq!(*max, 1.2);
            }
            _ => panic!("HUD Scale must be a slider"),
        }
    }
}
