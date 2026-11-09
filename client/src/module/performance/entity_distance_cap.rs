 
 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
const DEFAULT_CAP_SCALE: f32 = 0.75;
 
const MIN_CAP_SCALE: f32 = 0.5;
 
const MAX_CAP_SCALE: f32 = 1.0;
 
 
const VANILLA_DEFAULT_SCALE: f64 = 1.0;

 
 
#[derive(Debug, Clone, Copy, PartialEq)]
struct Clamp {
    original: f64,
    applied: f64,
}

#[derive(Debug)]
pub struct EntityDistanceCapModule {
    pub module: ModuleData,
    clamp: Mutex<Option<Clamp>>,
}

impl EntityDistanceCapModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::EntityDistanceCap,
                description: "Caps the entity distance so fewer distant entities are \
                              drawn. Only lowers the option, and restores it when disabled."
                    .to_string(),
                category: ModuleCategory::Performance,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Scale".to_string(),
                    value: DEFAULT_CAP_SCALE,
                    min: MIN_CAP_SCALE,
                    max: MAX_CAP_SCALE,
                }],
            },
            clamp: Mutex::new(None),
        }
    }

    fn cap_scale(&self) -> f64 {
        self.module
            .get_setting("Scale")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(DEFAULT_CAP_SCALE) as f64
    }

     
    fn apply(&self) -> anyhow::Result<()> {
        let cap = self.cap_scale();
        let option = minecraft().options()?.entity_distance_scaling()?;
        let current = option.get_double().unwrap_or(VANILLA_DEFAULT_SCALE);
        if !current.is_finite() || current <= cap {
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
        option.force_double(cap)
    }

     
    fn restore(&self) -> anyhow::Result<()> {
        let Some(clamp) = *self.clamp.lock().unwrap() else {
            return Ok(());
        };
        let option = minecraft().options()?.entity_distance_scaling()?;
        if option.get_double().ok() == Some(clamp.applied) {
            option.force_double(clamp.original)?;
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

impl Module for EntityDistanceCapModule {
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
