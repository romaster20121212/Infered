 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::chams::Config as Look;
use crate::module::render::trail::bounded;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::client;

 
 
 
 
 
 
pub(crate) const HOLOGRAM_CORE_MODE: i32 = 9;
pub(crate) const HOLOGRAM_MODE: i32 = 8;

const DEFAULT_RANGE: f32 = 64.0;
const DEFAULT_OPACITY: f32 = 0.38;

 
#[derive(Clone)]
pub(crate) struct Config {
    pub(crate) enabled: bool,
     
     
    pub(crate) opacity: f32,
     
     
    pub(crate) look: Look,
}

impl Config {
    pub(crate) fn read(data: &ModuleData) -> Self {
        let toggle = |name, fallback| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_toggle_value)
                .unwrap_or(fallback)
        };
        let color = match data.get_setting("Color") {
            Some(ModuleSetting::Color { value, .. }) => *value,
            _ => [0.34, 0.86, 1.0, 1.0],
        };
        let opacity = bounded(data, "Opacity", DEFAULT_OPACITY, 0.05, 1.0);
        Self {
            enabled: data.enabled,
            opacity,
            look: Look {
                players: toggle("Players", true),
                self_player: toggle("Self", false),
                armor: false,
                 
                 
                color: [color[0], color[1], color[2], opacity],
                strength: 1.0,
                pulse: false,
                through: toggle("ThroughWalls", true),
                range: bounded(data, "Range", DEFAULT_RANGE, 16.0, 128.0),
            },
        }
    }
}

 
 
pub(crate) fn read_config() -> Config {
    let mut cfg = Config {
        enabled: false,
        opacity: DEFAULT_OPACITY,
        look: Look {
            players: true,
            self_player: false,
             
            armor: false,
            color: [0.34, 0.86, 1.0, DEFAULT_OPACITY],
            strength: 1.0,
            pulse: false,
            through: true,
            range: DEFAULT_RANGE,
        },
    };
    if let Some(arc) = client().modules.get(ModuleId::HologramChams) {
        if let Ok(module) = arc.lock() {
            cfg = Config::read(module.get_module_data());
        }
    }
    cfg
}

 

 
 
#[derive(Debug)]
pub struct HologramChamsModule {
    pub module: ModuleData,
}

impl HologramChamsModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::HologramChams,
                description:
                    "Draws players as a translucent hologram of their own animated model, visible through itself"
                        .to_string(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Toggle {
                        name: "Players".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Self".to_string(),
                        value: false,
                    },
                    ModuleSetting::Toggle {
                        name: "ThroughWalls".to_string(),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: "Range".to_string(),
                        value: DEFAULT_RANGE,
                        min: 16.0,
                        max: 128.0,
                    },
                    ModuleSetting::Slider {
                        name: "Opacity".to_string(),
                        value: DEFAULT_OPACITY,
                        min: 0.05,
                        max: 1.0,
                    },
                    ModuleSetting::Color {
                        name: "Color".to_string(),
                        value: [0.34, 0.86, 1.0, 1.0],
                    },
                ],
            },
        }
    }
}

impl Default for HologramChamsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for HologramChamsModule {
    fn on_start(&self) -> anyhow::Result<()> {
        crate::graphic::smoke::start_source(ModuleId::HologramChams)
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        crate::graphic::smoke::stop_source(ModuleId::HologramChams)
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        crate::graphic::smoke::stop_source(ModuleId::HologramChams)
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

    fn module() -> HologramChamsModule {
        HologramChamsModule::new()
    }

    #[test]
    fn it_is_a_world_material_pass_not_a_body_hide() {
        let module = module();
        assert_eq!(module.module.id, ModuleId::HologramChams);
        assert_eq!(module.module.category, ModuleCategory::Render);
         
         
        for name in ["Hide Model", "Cubes"] {
            assert!(
                module.module.get_setting(name).is_none(),
                "{name} belonged to the removed body-hiding version"
            );
        }
        let cfg = Config::read(&module.module);
        assert!(!cfg.enabled);
        assert!(cfg.look.players);
        assert!(!cfg.look.self_player);
        assert!(cfg.look.through);
    }

    #[test]
    fn opacity_reaches_the_shader_as_material_alpha() {
        let mut module = module();
        module.module.enabled = true;
        module
            .module
            .get_setting_mut("Opacity")
            .unwrap()
            .set_slider_value(0.6);
        let cfg = Config::read(&module.module);
        assert!((cfg.opacity - 0.6).abs() < 1e-6);
         
        assert!((cfg.look.color[3] - 0.6).abs() < 1e-6);
         
        assert!((cfg.look.color[0] - 0.34).abs() < 1e-6);
    }

    #[test]
    fn corrupt_settings_stay_finite_and_clamped() {
        let mut module = module();
        module
            .module
            .get_setting_mut("Opacity")
            .unwrap()
            .set_slider_value(f32::NAN);
        module
            .module
            .get_setting_mut("Range")
            .unwrap()
            .set_slider_value(9_000.0);
        let cfg = Config::read(&module.module);
        assert!(cfg.opacity.is_finite());
        assert!((0.05..=1.0).contains(&cfg.opacity));
        assert_eq!(cfg.look.range, 128.0);
        assert_eq!(HOLOGRAM_MODE, 8);
    }
}
