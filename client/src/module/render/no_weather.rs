 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::client::world::World;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
 
fn levels_to_apply(hide_rain: bool, hide_thunder: bool) -> (Option<f32>, Option<f32>) {
    (hide_rain.then_some(0.0), hide_thunder.then_some(0.0))
}

 
 
#[derive(Debug)]
pub struct NoWeatherModule {
    pub module: ModuleData,
     
     
    saved: Mutex<Option<(f32, f32)>>,
}

impl NoWeatherModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::NoWeather,
                description: "Hides rain and thunder client-side, restoring them when disabled"
                    .to_string(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Toggle {
                        name: "Rain".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Thunder".to_string(),
                        value: true,
                    },
                ],
            },
            saved: Mutex::new(None),
        }
    }

    fn toggle(&self, name: &str) -> bool {
        self.module
            .get_setting(name)
            .and_then(ModuleSetting::get_toggle_value)
            .unwrap_or(true)
    }

     
     
    fn restore(&self) {
        let saved = self.saved.lock().unwrap_or_else(|p| p.into_inner()).take();
        let Some((rain, thunder)) = saved else {
            return;
        };
        let Ok(Some(world)) = minecraft().world() else {
            return;
        };
        if self.toggle("Rain") {
            let _ = world.set_rain_level(rain);
        }
        if self.toggle("Thunder") {
            let _ = world.set_thunder_level(thunder);
        }
    }

     
     
    fn remember(&self) {
        let Ok(Some(world)) = minecraft().world() else {
            return;
        };
        let Ok(rain) = read_level(&world, true) else {
            return;
        };
        let Ok(thunder) = read_level(&world, false) else {
            return;
        };
        *self.saved.lock().unwrap_or_else(|p| p.into_inner()) = Some((rain, thunder));
    }
}

 
 
fn read_level(world: &World, rain: bool) -> anyhow::Result<f32> {
    if rain {
        world.rain_level()
    } else {
        world.thunder_level()
    }
}

impl Module for NoWeatherModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.remember();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.restore();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Some(world) = minecraft().world()? else {
            return Ok(());
        };
        let (rain, thunder) = levels_to_apply(self.toggle("Rain"), self.toggle("Thunder"));
        if let Some(level) = rain {
            world.set_rain_level(level)?;
        }
        if let Some(level) = thunder {
            world.set_thunder_level(level)?;
        }
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
    fn the_module_ships_off_with_both_effects_hidden() {
        let module = NoWeatherModule::new();
        assert_eq!(module.module.id, ModuleId::NoWeather);
        assert_eq!(module.module.category, ModuleCategory::Visuals);
        assert!(!module.module.enabled);
        assert!(module.toggle("Rain"));
        assert!(module.toggle("Thunder"));
        assert!(module.saved.lock().unwrap().is_none());
    }

    #[test]
    fn each_toggle_controls_its_own_level() {
        assert_eq!(levels_to_apply(true, true), (Some(0.0), Some(0.0)));
        assert_eq!(levels_to_apply(true, false), (Some(0.0), None));
        assert_eq!(levels_to_apply(false, true), (None, Some(0.0)));
        assert_eq!(levels_to_apply(false, false), (None, None));
    }

    #[test]
    fn a_stop_without_a_saved_level_is_harmless() {
        let module = NoWeatherModule::new();
        module.restore();
        assert!(module.saved.lock().unwrap().is_none());
    }
}
