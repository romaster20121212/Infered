 
 
 
 
 
 
 

use crate::mapping::client::view_distance::singleplayer_player_list;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
const DEFAULT_CAP_CHUNKS: f32 = 6.0;
 
const MIN_CAP_CHUNKS: f32 = 2.0;
 
const MAX_CAP_CHUNKS: f32 = 32.0;

 
 
 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Clamp {
    original: i32,
    applied: i32,
}

#[derive(Debug)]
pub struct SimulationDistanceCapModule {
    pub module: ModuleData,
    clamp: Mutex<Option<Clamp>>,
}

impl SimulationDistanceCapModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::SimulationDistanceCap,
                description: "Caps the singleplayer simulation distance to tick fewer \
                              entities and block updates. Only lowers the value, and \
                              restores it when disabled."
                    .to_string(),
                category: ModuleCategory::Performance,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Distance".to_string(),
                    value: DEFAULT_CAP_CHUNKS,
                    min: MIN_CAP_CHUNKS,
                    max: MAX_CAP_CHUNKS,
                }],
            },
            clamp: Mutex::new(None),
        }
    }

    fn cap_chunks(&self) -> i32 {
        self.module
            .get_setting("Distance")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(DEFAULT_CAP_CHUNKS)
            .round() as i32
    }

     
    fn apply(&self) -> anyhow::Result<()> {
        let Some(player_list) = singleplayer_player_list()? else {
            return Ok(());
        };
        let cap = self.cap_chunks();
        let current = player_list.simulation_distance()?;
        if current <= cap {
            return Ok(());
        }
        {
            let mut clamp = self.clamp.lock().unwrap();
             
            let original = clamp.map_or(current, |c| c.original);
            *clamp = Some(Clamp {
                original,
                applied: cap,
            });
        }
        player_list.set_simulation_distance(cap)
    }

     
    fn restore(&self) -> anyhow::Result<()> {
        let Some(clamp) = *self.clamp.lock().unwrap() else {
            return Ok(());
        };
        if let Some(player_list) = singleplayer_player_list()? {
            if player_list.simulation_distance()? == clamp.applied {
                player_list.set_simulation_distance(clamp.original)?;
            }
        }
        *self.clamp.lock().unwrap() = None;
        Ok(())
    }

     
    fn sync(&self) -> anyhow::Result<()> {
        if matches!(minecraft().world(), Ok(Some(_))) {
            self.apply()
        } else {
            self.restore()
        }
    }
}

impl Module for SimulationDistanceCapModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.sync()
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.restore()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        self.sync()
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        self.restore()
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
    fn defaults_to_disabled_performance_module() {
        let module = SimulationDistanceCapModule::new();
        assert_eq!(module.module.id, ModuleId::SimulationDistanceCap);
        assert_eq!(module.module.category, ModuleCategory::Performance);
        assert!(!module.module.enabled);
    }

    #[test]
    fn default_cap_is_within_slider_range() {
        assert!(DEFAULT_CAP_CHUNKS >= MIN_CAP_CHUNKS);
        assert!(DEFAULT_CAP_CHUNKS <= MAX_CAP_CHUNKS);
    }
}
