 
 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;
use std::sync::Mutex;

#[derive(Debug)]
pub struct MenuBlurOffModule {
    pub module: ModuleData,
     
    original: Mutex<Option<i32>>,
}

impl MenuBlurOffModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::MenuBlurOff,
                description: "Turns off the blur behind menus and the inventory, which \
                              removes a full-screen render pass each frame. Restores the \
                              original blur when disabled."
                    .to_string(),
                category: ModuleCategory::Performance,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
            original: Mutex::new(None),
        }
    }

     
    fn apply(&self) -> anyhow::Result<()> {
        let option = minecraft().options()?.menu_background_blurriness()?;
        let current = option.get_int()?;
        if current == 0 {
            return Ok(());
        }
        {
            let mut original = self.original.lock().unwrap();
             
            original.get_or_insert(current);
        }
        option.force_int(0)
    }

     
    fn restore(&self) -> anyhow::Result<()> {
        let Some(original) = *self.original.lock().unwrap() else {
            return Ok(());
        };
        let option = minecraft().options()?.menu_background_blurriness()?;
        if option.get_int().ok() == Some(0) {
            option.force_int(original)?;
        }
        *self.original.lock().unwrap() = None;
        Ok(())
    }
}

impl Module for MenuBlurOffModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.apply()
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.restore()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        self.apply()
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
        let module = MenuBlurOffModule::new();
        assert_eq!(module.module.id, ModuleId::MenuBlurOff);
        assert_eq!(module.module.category, ModuleCategory::Performance);
        assert!(!module.module.enabled);
        assert!(module.module.settings.is_empty());
    }

    #[test]
    fn starts_with_no_captured_original() {
        let module = MenuBlurOffModule::new();
        assert_eq!(*module.original.lock().unwrap(), None);
    }
}
