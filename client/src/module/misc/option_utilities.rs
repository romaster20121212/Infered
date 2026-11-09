 

use std::sync::Mutex;

use crate::mapping::client::options::OptionInstance;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Value {
    Bool(bool),
    Double(f64),
}

impl Value {
    fn read(self, option: &OptionInstance) -> anyhow::Result<Self> {
        match self {
            Self::Bool(_) => Ok(Self::Bool(option.get_bool()?)),
            Self::Double(_) => Ok(Self::Double(option.get_double()?)),
        }
    }

    fn write(self, option: &OptionInstance) -> anyhow::Result<()> {
        match self {
            Self::Bool(value) => option.force_bool(value),
            Self::Double(value) => option.force_double(value),
        }
    }
}

#[derive(Debug)]
pub struct OptionUtilityModule {
    module: ModuleData,
    field: &'static str,
    target: Value,
    original: Mutex<Option<Value>>,
}

impl OptionUtilityModule {
    fn new(id: ModuleId, description: &str, field: &'static str, target: Value) -> Self {
        Self {
            module: ModuleData {
                id,
                description: description.to_string(),
                category: ModuleCategory::Misc,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: Vec::new(),
            },
            field,
            target,
            original: Mutex::new(None),
        }
    }

    fn apply(&self) -> anyhow::Result<()> {
        let option = minecraft().options()?.misc_option(self.field)?;
        let current = self.target.read(&option)?;
        if current == self.target {
            return Ok(());
        }
        let mut original = self.original.lock().unwrap_or_else(|e| e.into_inner());
        original.get_or_insert(current);
        self.target.write(&option)
    }

    fn restore(&self) -> anyhow::Result<()> {
        let mut original = self.original.lock().unwrap_or_else(|e| e.into_inner());
        let Some(saved) = *original else {
            return Ok(());
        };
        let option = minecraft().options()?.misc_option(self.field)?;
        if self.target.read(&option)? == self.target {
            saved.write(&option)?;
        }
        *original = None;
        Ok(())
    }

    fn sync(&self) -> anyhow::Result<()> {
        if minecraft().world()?.is_some() {
            self.apply()
        } else {
            self.restore()
        }
    }
}

impl Module for OptionUtilityModule {
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

pub fn modules() -> [OptionUtilityModule; 5] {
    [
        OptionUtilityModule::new(
            ModuleId::NoDarknessEffect,
            "Disables the pulsing darkness effect while in a world.",
            "darknessEffectScale",
            Value::Double(0.0),
        ),
        OptionUtilityModule::new(
            ModuleId::NoScreenEffects,
            "Disables Minecraft's screen distortion effects while in a world.",
            "screenEffectScale",
            Value::Double(0.0),
        ),
        OptionUtilityModule::new(
            ModuleId::NoEnchantmentGlint,
            "Hides the enchantment glint while in a world.",
            "glintStrength",
            Value::Double(0.0),
        ),
        OptionUtilityModule::new(
            ModuleId::ToggleCrouch,
            "Uses Minecraft's toggle crouch option while in a world.",
            "toggleCrouch",
            Value::Bool(true),
        ),
        OptionUtilityModule::new(
            ModuleId::ToggleSprint,
            "Uses Minecraft's toggle sprint option while in a world.",
            "toggleSprint",
            Value::Bool(true),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn misc_option_modules_have_distinct_ids_and_fields() {
        let modules = modules();
        let mut ids = HashSet::new();
        let mut fields = HashSet::new();
        for module in modules {
            assert!(ids.insert(module.module.id));
            assert!(fields.insert(module.field));
            assert_eq!(module.module.category, ModuleCategory::Misc);
            assert!(!module.module.enabled);
            assert!(module.module.settings.is_empty());
        }
        assert_eq!(ids.len(), 5);
    }
}
