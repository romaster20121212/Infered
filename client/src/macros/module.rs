 
 
 
 
 
 
 
 
 
 
 
 

use infered_macros::toggle_label;

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::client;

use super::engine::{Engine, MacroControl, Policy};
use super::host::Host;

 
 
pub const MACRO_SETTING_PREFIX: &str = "Macro: ";

 
pub fn is_dynamic_setting_name(name: &str) -> bool {
    name.starts_with(MACRO_SETTING_PREFIX)
}

 
#[derive(Debug)]
pub struct MacrosModule {
    pub module: ModuleData,
}

impl MacrosModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Macros,
                description: "Runs .macro files from the client's macros folder".to_string(),
                category: ModuleCategory::Macros,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: base_settings(),
            },
        }
    }
}

fn base_settings() -> Vec<ModuleSetting> {
    vec![
        ModuleSetting::Toggle {
            name: "Stop In Menus".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Stop On World Change".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Stop On Death".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Status HUD".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Reload Now".to_string(),
            value: false,
        },
        ModuleSetting::Toggle {
            name: "Stop All".to_string(),
            value: false,
        },
    ]
}

fn toggle(data: &ModuleData, name: &str) -> bool {
    data.get_setting(name)
        .and_then(ModuleSetting::get_toggle_value)
        .unwrap_or(false)
}

 
 
pub fn policy() -> Policy {
    let Some(handle) = client().modules.get(ModuleId::Macros) else {
        return Policy::default();
    };
    let Ok(module) = handle.lock() else {
        return Policy::default();
    };
    let data = module.get_module_data();
    Policy {
        enabled: data.enabled,
        stop_in_menus: toggle(data, "Stop In Menus"),
        stop_on_world_change: toggle(data, "Stop On World Change"),
        stop_on_death: toggle(data, "Stop On Death"),
    }
}

 
 
pub fn sync(engine: &mut Engine, host: &mut dyn Host) {
    let Some(handle) = client().modules.get(ModuleId::Macros) else {
        return;
    };
    let Ok(mut module) = handle.lock() else {
        return;
    };
    let data = module.get_module_data_mut();

     
     
    if toggle(data, "Stop All") {
        engine.stop_everything(host, "Stop All switch");
        if let Some(setting) = data.get_setting_mut("Stop All") {
            setting.set_toggle_value(false);
        }
    }
    if toggle(data, "Reload Now") {
        engine.invalidate_scan();
        engine.maintain(host, true);
        if let Some(setting) = data.get_setting_mut("Reload Now") {
            setting.set_toggle_value(false);
        }
    }

     
     
     
    let controls = engine.controls();
    let old_labels = labels(&controls);
    let mut changes: Vec<(String, bool)> = Vec::new();
    for (control, label) in controls.iter().zip(&old_labels) {
        if let Some(ModuleSetting::Toggle { value, .. }) = data.get_setting(label) {
            if *value != control.enabled {
                changes.push((control.id.clone(), *value));
            }
        }
    }
    for (id, enabled) in changes {
        if let Err(error) = engine.set_macro_enabled(host, &id, enabled) {
            log::warn!(
                "macros: could not {} {id}: {error}",
                if enabled { "enable" } else { "disable" }
            );
            engine.last_error = Some(error);
        }
    }

    let controls = engine.controls();
    let new_labels = labels(&controls);
    let mut settings = base_settings();
    for setting in &mut settings {
        if let Some(value) = data
            .get_setting(setting.name())
            .and_then(ModuleSetting::get_toggle_value)
        {
            setting.set_toggle_value(value);
        }
    }
    for (control, label) in controls.iter().zip(new_labels) {
        settings.push(ModuleSetting::Toggle {
            name: label,
            value: control.enabled,
        });
    }
    let changed = settings.len() != data.settings.len()
        || settings.iter().zip(&data.settings).any(|(new, old)| {
            new.name() != old.name() || new.get_toggle_value() != old.get_toggle_value()
        });
    if changed {
        data.settings = settings;
    }

    let running = controls.iter().filter(|control| control.running).count();
    let description = if controls.is_empty() {
        format!(
            "No .macro files yet — add them in the launcher's Macros page ({})",
            engine.directory().display()
        )
    } else {
        format!(
            "{} macro{} loaded · {} running · {}",
            controls.len(),
            if controls.len() == 1 { "" } else { "s" },
            running,
            engine.directory().display()
        )
    };
    if data.description != description {
        data.description = description;
    }
}

 
 
fn labels(controls: &[MacroControl]) -> Vec<String> {
    let mut counts = std::collections::HashMap::new();
    for control in controls {
        *counts.entry(control.name.as_str()).or_insert(0usize) += 1;
    }
    controls
        .iter()
        .map(|control| {
            let name = toggle_label(&control.name);
            let label = if counts.get(control.name.as_str()).copied().unwrap_or(0) > 1 {
                format!("{}{} ({})", MACRO_SETTING_PREFIX, name, control.id)
            } else {
                format!("{MACRO_SETTING_PREFIX}{name}")
            };
            label.chars().take(52).collect()
        })
        .collect()
}

impl Module for MacrosModule {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_module_carries_the_runtime_switches() {
        let module = MacrosModule::new();
        let data = &module.module;
        assert_eq!(data.id, ModuleId::Macros);
        assert_eq!(data.category, ModuleCategory::Macros);
        for name in [
            "Stop In Menus",
            "Stop On World Change",
            "Stop On Death",
            "Status HUD",
            "Reload Now",
            "Stop All",
        ] {
            assert!(data.get_setting(name).is_some(), "missing {name}");
        }
        assert!(!data.enabled);
    }

    #[test]
    fn dynamic_setting_names_are_recognised() {
        assert!(is_dynamic_setting_name("Macro: Farm"));
        assert!(!is_dynamic_setting_name("Stop All"));
    }

    #[test]
    fn labels_are_unique_when_names_collide() {
        let controls = vec![
            MacroControl {
                id: "farm".into(),
                name: "Farm".into(),
                enabled: true,
                running: false,
                status: String::new(),
            },
            MacroControl {
                id: "farm_2".into(),
                name: "Farm".into(),
                enabled: false,
                running: false,
                status: String::new(),
            },
        ];
        let labels = labels(&controls);
        assert_eq!(labels[0], "Macro: Farm (farm)");
        assert_eq!(labels[1], "Macro: Farm (farm_2)");
    }
}
