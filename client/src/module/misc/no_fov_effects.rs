 
 
 
 
 
 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
const DEFAULT_SCALE: f32 = 0.0;
 
const MAX_SCALE: f32 = 1.0;

#[derive(Debug)]
pub struct NoFovEffectsModule {
    pub module: ModuleData,
     
     
    original: Mutex<Option<f64>>,
}

impl NoFovEffectsModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::NoFovEffects,
                description: "Pins the FOV effect scale so sprinting and speed potions \
                              never widen the view. Restores the original value when disabled."
                    .to_string(),
                category: ModuleCategory::Misc,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Scale".to_string(),
                    value: DEFAULT_SCALE,
                    min: 0.0,
                    max: MAX_SCALE,
                }],
            },
            original: Mutex::new(None),
        }
    }

     
     
    fn scale(&self) -> f64 {
        f64::from(
            self.module
                .get_setting("Scale")
                .and_then(|setting| setting.get_slider_value())
                .filter(|value| value.is_finite())
                .unwrap_or(DEFAULT_SCALE)
                .clamp(0.0, MAX_SCALE),
        )
    }

     
    fn apply(&self) -> anyhow::Result<()> {
        let option = minecraft().options()?.fov_effect_scale()?;
        let current = option.get_double()?;
        let target = self.scale();
        if (current - target).abs() < f64::EPSILON {
            return Ok(());
        }
        {
            let mut original = self.original.lock().unwrap_or_else(|p| p.into_inner());
             
             
            original.get_or_insert(current);
        }
        option.force_double(target)
    }

     
    fn restore(&self) -> anyhow::Result<()> {
        let Some(original) = *self.original.lock().unwrap_or_else(|p| p.into_inner()) else {
            return Ok(());
        };
        let option = minecraft().options()?.fov_effect_scale()?;
        let current = option.get_double()?;
        if (current - self.scale()).abs() < f64::EPSILON {
            option.force_double(original)?;
        }
        *self.original.lock().unwrap_or_else(|p| p.into_inner()) = None;
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

impl Module for NoFovEffectsModule {
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
    fn defaults_to_disabled_misc_module_with_a_zero_scale() {
        let module = NoFovEffectsModule::new();
        assert_eq!(module.module.id, ModuleId::NoFovEffects);
        assert_eq!(module.module.category, ModuleCategory::Misc);
        assert!(!module.module.enabled);
        assert_eq!(module.scale(), 0.0);
        assert_eq!(*module.original.lock().unwrap(), None);
    }

    #[test]
    fn the_scale_slider_is_clamped_and_never_nan() {
        let mut module = NoFovEffectsModule::new();
        module.module.settings[0].set_slider_value(4.0);
        assert_eq!(module.scale(), 1.0);
        module.module.settings[0].set_slider_value(-2.0);
        assert_eq!(module.scale(), 0.0);
        module.module.settings[0].set_slider_value(f32::NAN);
        assert_eq!(module.scale(), 0.0);
    }
}
