 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;
use std::sync::Mutex;

#[derive(Debug)]
pub struct SmoothLightingOffModule {
    pub module: ModuleData,
     
     
    original: Mutex<Option<bool>>,
}

impl SmoothLightingOffModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::SmoothLightingOff,
                description: "Turns off smooth lighting so chunks mesh with flat lighting, \
                              which is cheaper to rebuild. Restores the option when disabled."
                    .to_string(),
                category: ModuleCategory::Performance,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: Vec::new(),
            },
            original: Mutex::new(None),
        }
    }

     
    fn apply(&self) -> anyhow::Result<()> {
        let option = minecraft().options()?.ambient_occlusion()?;
        let current = option.get_bool()?;
        if !current {
            return Ok(());
        }
        {
            let mut original = self.original.lock().unwrap();
             
            original.get_or_insert(current);
        }
        option.force_bool(false)
    }

     
    fn restore(&self) -> anyhow::Result<()> {
        let Some(original) = *self.original.lock().unwrap() else {
            return Ok(());
        };
        let option = minecraft().options()?.ambient_occlusion()?;
        if !option.get_bool()? {
            option.force_bool(original)?;
        }
        *self.original.lock().unwrap() = None;
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

impl Module for SmoothLightingOffModule {
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
        let module = SmoothLightingOffModule::new();
        assert_eq!(module.module.id, ModuleId::SmoothLightingOff);
        assert_eq!(module.module.category, ModuleCategory::Performance);
        assert!(!module.module.enabled);
        assert!(module.module.settings.is_empty());
    }
}
