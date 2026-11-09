 
 
 
 
 
 
 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
const DEFAULT_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 1.0];

 
 
const SLIDERS: [(&str, f32, f32, f32); 2] = [
    ("GlowStrength", 0.0, 2.0, 1.0),
    ("Range", 16.0, 128.0, 64.0),
];

 
const TOGGLES: [(&str, bool); 4] = [
     
     
    ("Players", true),
    ("Self", false),
    ("Pulse", true),
    ("ThroughWalls", true),
];

#[derive(Debug)]
pub struct ItemChamsModule {
    pub module: ModuleData,
}

impl ItemChamsModule {
    pub fn new() -> Self {
        let mut settings = vec![ModuleSetting::Color {
            name: "Color".into(),
            value: DEFAULT_COLOR,
        }];
        settings.extend(TOGGLES.iter().map(|&(name, value)| ModuleSetting::Toggle {
            name: name.into(),
            value,
        }));
        settings.extend(
            SLIDERS
                .iter()
                .map(|&(name, min, max, value)| ModuleSetting::Slider {
                    name: name.into(),
                    value,
                    min,
                    max,
                }),
        );
        Self {
            module: ModuleData {
                id: ModuleId::ItemChams,
                description: "Applies the Chams material to the items other players \
                              hold. Never touches your own first-person hand."
                    .into(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings,
            },
        }
    }
}

impl Default for ItemChamsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ItemChamsModule {
    fn on_start(&self) -> anyhow::Result<()> {
        crate::graphic::smoke::start_source(ModuleId::ItemChams)
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        crate::graphic::smoke::stop_source(ModuleId::ItemChams)
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        crate::graphic::smoke::stop_source(ModuleId::ItemChams)
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
    use crate::graphic::chams::Config;

    fn toggle(module: &mut ItemChamsModule, name: &str, to: bool) {
        module
            .module
            .get_setting_mut(name)
            .unwrap_or_else(|| panic!("{name} must exist"))
            .set_toggle_value(to);
    }

    fn set_slider(module: &mut ItemChamsModule, name: &str, to: f32) {
        if let Some(ModuleSetting::Slider { value, .. }) = module.module.get_setting_mut(name) {
            *value = to;
        }
    }

    #[test]
    fn item_chams_is_a_world_material_pass() {
        let module = ItemChamsModule::new();
        assert_eq!(module.module.id, ModuleId::ItemChams);
        assert_eq!(module.module.category, ModuleCategory::Render);
         
        assert_eq!(
            module
                .module
                .get_setting("Players")
                .unwrap()
                .get_toggle_value(),
            Some(true)
        );
        assert_eq!(
            module
                .module
                .get_setting("Self")
                .unwrap()
                .get_toggle_value(),
            Some(false)
        );
         
        for name in ["Main Hand", "Off Hand", "Width", "Pulse Speed"] {
            assert!(
                module.module.get_setting(name).is_none(),
                "{name} belonged to the removed first-person outline"
            );
        }
    }

    #[test]
    fn the_look_is_the_chams_look() {
        let mut module = ItemChamsModule::new();
        let cfg = Config::read(&module.module);
        assert!(cfg.players);
        assert!(!cfg.self_player);
        assert!(cfg.through);
        assert!(cfg.pulse);
        assert_eq!(cfg.color, DEFAULT_COLOR);
        assert_eq!(cfg.strength, 1.0);
        assert_eq!(cfg.range, 64.0);

        toggle(&mut module, "Self", true);
        toggle(&mut module, "ThroughWalls", false);
        set_slider(&mut module, "Range", 900.0);
        let cfg = Config::read(&module.module);
        assert!(cfg.self_player);
        assert!(!cfg.through);
        assert_eq!(cfg.range, 128.0);
    }
}
