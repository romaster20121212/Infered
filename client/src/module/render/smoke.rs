 
 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

#[derive(Debug)]
pub struct SmokeModule {
    pub module: ModuleData,
}

impl SmokeModule {
    pub fn new() -> Self {
        let slider = |name: &str, value, min, max| ModuleSetting::Slider {
            name: name.into(),
            value,
            min,
            max,
        };
        Self {
            module: ModuleData {
                id: ModuleId::Smoke,
                description:
                    "Procedural world smoke. Move to emit; hold G for a plume. No assets required."
                        .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    slider("Density", 24.0, 1.0, 60.0),
                    slider("Size", 0.65, 0.2, 2.0),
                    slider("Lifetime", 4.0, 0.5, 8.0),
                    slider("Wind", 0.25, 0.0, 2.0),
                    slider("Wind Direction", 0.0, 0.0, 360.0),
                    ModuleSetting::Color {
                        name: "Color Tint".into(),
                        value: [1.0, 0.0, 0.0, 1.0],
                    },
                    slider("Opacity", 0.65, 0.0, 1.0),
                    ModuleSetting::Toggle {
                        name: "Spawn on Move".into(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Spawn on Key".into(),
                        value: true,
                    },
                    ModuleSetting::Choice {
                        name: "Spawn Key".into(),
                        value: 0,
                        options: ["G", "H", "J", "K", "V", "B", "N", "M"]
                            .into_iter()
                            .map(str::to_string)
                            .collect(),
                    },
                     
                    ModuleSetting::Toggle {
                        name: "AuraGlobalSmoke".into(),
                        value: true,
                    },
                    slider("AuraSmokeRadius", 1.5, 0.5, 8.0),  
                    slider("AuraSmokeOpacity", 0.055, 0.005, 0.2),
                    slider("AuraSmokeDensity", 12.0, 1.0, 32.0),  
                    slider("AuraSmokeFadeSpeed", 2.0, 0.5, 8.0),  
                    ModuleSetting::Toggle {
                        name: "Orbs".into(),
                        value: false,
                    },
                    slider("OrbCount", 12.0, 1.0, 24.0),
                    slider("OrbSize", 0.08, 0.05, 0.15),
                    ModuleSetting::Color {
                        name: "OrbColor".into(),
                        value: [1.0, 0.0, 0.0, 1.0],
                    },
                    slider("OrbGlowStrength", 0.6, 0.0, 2.0),
                    ModuleSetting::Toggle {
                        name: "OrbFlicker".into(),
                        value: true,
                    },
                ],
            },
        }
    }
}

impl Module for SmokeModule {
    fn on_start(&self) -> anyhow::Result<()> {
        crate::graphic::smoke::start_source(ModuleId::Smoke)
    }
    fn on_stop(&self) -> anyhow::Result<()> {
        crate::graphic::smoke::stop_source(ModuleId::Smoke)
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
    fn smoke_settings_are_persistable_and_spawn_key_is_separate_from_toggle() {
        let module = SmokeModule::new();
        assert_eq!(module.module.id, ModuleId::Smoke);
        assert!(!module.module.enabled);
        assert_eq!(module.module.key_bind as i32, -1);
        for name in [
            "Density",
            "Size",
            "Lifetime",
            "Wind",
            "Wind Direction",
            "Opacity",
        ] {
            assert!(module
                .module
                .get_setting(name)
                .unwrap()
                .get_slider_value()
                .is_some());
        }
        for name in ["Spawn on Move", "Spawn on Key"] {
            assert_eq!(
                module.module.get_setting(name).unwrap().get_toggle_value(),
                Some(true)
            );
        }
        assert!(matches!(
            module.module.get_setting("Color Tint"),
            Some(ModuleSetting::Color { .. })
        ));
        assert!(
            matches!(module.module.get_setting("Spawn Key"), Some(ModuleSetting::Choice { options, value: 0, .. }) if options[0] == "G")
        );
        let json = serde_json::to_string(&ModuleId::Smoke).unwrap();
        assert_eq!(
            serde_json::from_str::<ModuleId>(&json).unwrap(),
            ModuleId::Smoke
        );
    }
}
