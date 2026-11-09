 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};

#[derive(Debug)]
pub struct MenuEspPauseModule {
    pub module: ModuleData,
}

impl MenuEspPauseModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::MenuEspPause,
                description: "Skips the ESP overlay while the ClickGUI is open, so the \
                              menu frame does less work. ESP returns when the menu closes."
                    .to_string(),
                category: ModuleCategory::Performance,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: Vec::new(),
            },
        }
    }
}

 
 
pub fn pauses_esp() -> bool {
    let Some(handle) = crate::state::client().modules.get(ModuleId::MenuEspPause) else {
        return false;
    };
    let Ok(module) = handle.lock() else {
        return false;
    };
    module.get_module_data().enabled
}

impl Module for MenuEspPauseModule {
     
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
