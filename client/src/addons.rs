 
 
 
 
 

use log::{info, warn};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub addon: String,
    pub label: String,
    pub description: String,
}

#[derive(Clone, Debug)]
pub struct AddonModule {
    pub addon: String,
    pub category: String,
    pub label: String,
    pub description: String,
    pub builtin: String,
    pub setting: String,
    pub enabled: bool,
}

#[derive(Default)]
struct AddonManager {
    menu_items: Vec<MenuItem>,
    modules: Vec<AddonModule>,
}

static MANAGER: LazyLock<Mutex<AddonManager>> =
    LazyLock::new(|| Mutex::new(AddonManager::default()));

 
pub fn load_all() {
    let directory = crate::config::base_dir().join("addons");
    if let Err(error) = std::fs::create_dir_all(&directory) {
        warn!("addons: could not create {}: {error}", directory.display());
        return;
    }

    let disabled = disabled_addons(&directory);
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(&directory) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().is_some_and(|extension| extension == "inf")
                    && path
                        .file_name()
                        .map(|name| !disabled.contains(&name.to_string_lossy().to_string()))
                        .unwrap_or(false)
            })
            .collect(),
        Err(error) => {
            warn!("addons: could not scan {}: {error}", directory.display());
            return;
        }
    };
    paths.sort();

    for path in paths {
        if let Err(error) = load_source(&path) {
            warn!("addons: skipping {}: {error}", path.display());
        }
    }
    let manager = MANAGER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    info!(
        "addons: loaded {} source addon(s), {} module(s) from {}",
        manager.menu_items.len(),
        manager.modules.len(),
        directory.display()
    );
}

fn disabled_addons(directory: &Path) -> std::collections::HashSet<String> {
    let state_path = directory
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("infered_addons.state");
    std::fs::read_to_string(state_path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.strip_prefix("disabled="))
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

 
 
 
 
 
 
 
 
 
 
 
 
 
fn load_source(path: &Path) -> Result<(), String> {
    let source = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut description = String::new();
    let mut menus = Vec::new();
    let mut modules = Vec::new();
    let mut current_category: Option<String> = None;

    for (line_number, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("line {}: expected key = value", line_number + 1));
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "addon" => {
                if value.is_empty() {
                    return Err(format!(
                        "line {}: addon name cannot be empty",
                        line_number + 1
                    ));
                }
                name = value.to_string();
            }
            "description" => description = value.to_string(),
            "category" => {
                if value.is_empty() {
                    return Err(format!(
                        "line {}: category name cannot be empty",
                        line_number + 1
                    ));
                }
                current_category = Some(value.to_string());
            }
            "menu" => {
                if !value.is_empty() {
                    menus.push(value.to_string());
                }
            }
            "mod" => {
                let fields: Vec<_> = value.split('|').map(str::trim).collect();
                let Some(category) = current_category.as_deref() else {
                    return Err(format!(
                        "line {}: add `category = Name` before `mod`",
                        line_number + 1
                    ));
                };
                if fields.len() < 2
                    || fields.len() > 3
                    || fields[..2].iter().any(|field| field.is_empty())
                {
                    return Err(format!(
                        "line {}: mod needs label|action|description",
                        line_number + 1
                    ));
                }
                let Some((builtin, setting)) = action_binding(fields[1]) else {
                    return Err(format!(
                        "line {}: unknown action '{}'; use click, jump, hit, sprint, or none",
                        line_number + 1,
                        fields[1]
                    ));
                };
                modules.push(AddonModule {
                    addon: name.clone(),
                    category: category.to_string(),
                    label: fields[0].to_string(),
                    description: fields.get(2).copied().unwrap_or(fields[1]).to_string(),
                    builtin: builtin.to_string(),
                    setting: setting.to_string(),
                    enabled: false,
                });
            }
            "module" => {
                let fields: Vec<_> = value.split('|').map(str::trim).collect();
                if fields.len() != 5 || fields[..4].iter().any(|field| field.is_empty()) {
                    return Err(format!(
                        "line {}: module needs category|label|description|builtin|setting",
                        line_number + 1
                    ));
                }
                modules.push(AddonModule {
                    addon: name.clone(),
                    category: fields[0].to_string(),
                    label: fields[1].to_string(),
                    description: fields[2].to_string(),
                    builtin: fields[3].to_string(),
                    setting: fields[4].to_string(),
                    enabled: false,
                });
            }
            _ => {
                return Err(format!(
                    "line {}: unknown directive '{key}'",
                    line_number + 1
                ));
            }
        }
    }

    if modules.is_empty() && menus.is_empty() {
        return Err("source addon has no menu or module entries".to_string());
    }
    let mut manager = MANAGER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for menu in menus {
        manager.menu_items.push(MenuItem {
            addon: name.clone(),
            label: menu,
            description: description.clone(),
        });
    }
    manager.modules.extend(modules);
    Ok(())
}

fn action_binding(action: &str) -> Option<(&'static str, &'static str)> {
    match action.to_ascii_lowercase().as_str() {
        "click" | "autoclick" => Some(("AutoClicker", "")),
        "trigger" | "triggerbot" => Some(("TriggerBot", "")),
        "jump" | "bhop" => Some(("Sprint", "Bhop")),
        "hit" | "attack" => Some(("KillAura", "")),
        "sprint" => Some(("Sprint", "")),
        "none" => Some(("", "")),
        _ => None,
    }
}

pub fn menu_items() -> Vec<MenuItem> {
    MANAGER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .menu_items
        .clone()
}

pub fn modules() -> Vec<AddonModule> {
    MANAGER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .modules
        .clone()
}

 
pub fn set_module_enabled(index: usize, enabled: bool) {
    let module = {
        let mut manager = MANAGER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(module) = manager.modules.get_mut(index) else {
            return;
        };
        module.enabled = enabled;
        module.clone()
    };
    let id = match module.builtin.as_str() {
        "AutoClicker" => crate::module::ModuleId::AutoClicker,
        "TriggerBot" => crate::module::ModuleId::TriggerBot,
        "Sprint" => crate::module::ModuleId::Sprint,
        "KillAura" => crate::module::ModuleId::KillAura,
        _ => return,
    };
    let Some(handle) = crate::state::client().modules.get(id) else {
        return;
    };
    let mut builtin = handle
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if !module.setting.is_empty() {
        if let Some(crate::module::ModuleSetting::Toggle { value, .. }) = builtin
            .get_module_data_mut()
            .get_setting_mut(&module.setting)
        {
            *value = enabled;
        }
    }
    let was_enabled = builtin.get_module_data().enabled;
    builtin.get_module_data_mut().set_enabled(enabled);
    if crate::state::client().modules.is_active() && was_enabled != enabled {
        let _ = if enabled {
            builtin.on_start()
        } else {
            builtin.on_stop()
        };
    }
}

 
pub fn shutdown() {
    let mut manager = MANAGER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    manager.menu_items.clear();
    manager.modules.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_aliases_map_to_existing_modules() {
        assert_eq!(action_binding("click"), Some(("AutoClicker", "")));
        assert_eq!(action_binding("jump"), Some(("Sprint", "Bhop")));
        assert_eq!(action_binding("hit"), Some(("KillAura", "")));
        assert_eq!(action_binding("unknown"), None);
    }
}
