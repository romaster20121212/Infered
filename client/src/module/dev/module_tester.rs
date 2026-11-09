 
 
 
 
 
 
 
 
 
 

use crate::graphic::dev_tester::{self, Options};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

#[derive(Debug)]
pub struct ModuleTesterModule {
    pub module: ModuleData,
}

impl ModuleTesterModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::ModuleTester,
                description: "Steps through every module: Y = works, Page Down = broken (logs \
                              the reason to dev_overview.md)"
                    .to_string(),
                category: ModuleCategory::Dev,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Choice {
                        name: "Order".to_string(),
                        value: 0,
                        options: vec!["Menu order".to_string(), "Alphabetical".to_string()],
                    },
                    ModuleSetting::Toggle {
                        name: "Enable Each Module".to_string(),
                        value: true,
                    },
                ],
            },
        }
    }

     
    fn options(&self) -> Options {
        let alphabetical = matches!(
            self.module.get_setting("Order"),
            Some(ModuleSetting::Choice { value, .. }) if *value == 1
        );
        let auto_enable = self
            .module
            .get_setting("Enable Each Module")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(true);
        Options {
            alphabetical,
            auto_enable,
        }
    }
}

impl Module for ModuleTesterModule {
    fn on_start(&self) -> anyhow::Result<()> {
        dev_tester::start(self.options());
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        dev_tester::stop();
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

#[cfg(test)]
mod tests {
    use super::*;

     
     
    #[test]
    fn options_follow_the_settings() {
        let mut module = ModuleTesterModule::new();
        let options = module.options();
        assert!(!options.alphabetical);
        assert!(options.auto_enable);

        if let Some(ModuleSetting::Choice { value, .. }) = module.module.get_setting_mut("Order") {
            *value = 1;
        }
        if let Some(setting) = module.module.get_setting_mut("Enable Each Module") {
            setting.set_toggle_value(false);
        }
        let options = module.options();
        assert!(options.alphabetical);
        assert!(!options.auto_enable);
    }
}
