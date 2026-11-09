 
 
 
 
 
 
 

use crate::graphic::anim;
use crate::graphic::hud::HudWidgetId;
use crate::graphic::settings;
use crate::graphic::theme;
use crate::module::{KeyboardKey, ModuleCategory, ModuleId, ModuleSetting};
use crate::state::client;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

 
 
static PANEL_POS: LazyLock<DashMap<ModuleCategory, [f32; 2]>> = LazyLock::new(DashMap::new);

 
 
static WIDGET_POS: LazyLock<DashMap<HudWidgetId, [f32; 2]>> = LazyLock::new(DashMap::new);

 
static EXPANDED: LazyLock<DashMap<ModuleId, bool>> = LazyLock::new(DashMap::new);

 
static COLLAPSED: LazyLock<DashMap<ModuleCategory, bool>> = LazyLock::new(DashMap::new);
static FAVORITES: LazyLock<DashMap<ModuleId, bool>> = LazyLock::new(DashMap::new);

pub fn is_favorite(id: ModuleId) -> bool {
    FAVORITES.contains_key(&id)
}

pub fn set_favorite(id: ModuleId, favorite: bool) {
    if favorite {
        FAVORITES.insert(id, true);
    } else {
        FAVORITES.remove(&id);
    }
}

 
pub fn panel_pos(category: ModuleCategory) -> Option<[f32; 2]> {
    PANEL_POS.get(&category).map(|entry| *entry)
}

 
pub fn set_panel_pos(category: ModuleCategory, pos: [f32; 2]) {
    PANEL_POS.insert(category, pos);
}

 
 
pub fn widget_pos(id: HudWidgetId) -> Option<[f32; 2]> {
    WIDGET_POS.get(&id).map(|entry| *entry)
}

 
pub fn set_widget_pos(id: HudWidgetId, pos: [f32; 2]) {
    WIDGET_POS.insert(id, pos);
}

 
pub fn is_expanded(id: ModuleId) -> bool {
    EXPANDED.get(&id).map(|entry| *entry).unwrap_or(false)
}

 
pub fn set_expanded(id: ModuleId, expanded: bool) {
    EXPANDED.insert(id, expanded);
}

 
pub fn is_collapsed(category: ModuleCategory) -> bool {
    COLLAPSED
        .get(&category)
        .map(|entry| *entry)
        .unwrap_or(false)
}

 
pub fn set_collapsed(category: ModuleCategory, collapsed: bool) {
    COLLAPSED.insert(category, collapsed);
}

 
 
pub fn reset_ui_state() {
    PANEL_POS.clear();
    WIDGET_POS.clear();
    EXPANDED.clear();
    COLLAPSED.clear();
}

 
 
pub fn reset_keybinds() {
    for handle in client().modules.handles() {
        let Ok(mut module) = handle.lock() else {
            continue;
        };
        let data = module.get_module_data_mut();
        data.key_bind = if data.id == ModuleId::MenuHud {
            KeyboardKey::KeyRShift
        } else {
            KeyboardKey::KeyNone
        };
    }
    save();
}

 
const CONFIG_FILE: &str = "infered_config.json";

 
const CONFIGS_DIR: &str = "configs";

 
 
 
 
const CATALOG_FILE: &str = "infered_modules.json";

 
 
 
 
 
pub fn base_dir() -> PathBuf {
    match std::env::var_os("INFERED_CONFIG_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from("."),
    }
}

 
fn config_path() -> PathBuf {
    base_dir().join(CONFIG_FILE)
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
     
     
     
    #[serde(default, deserialize_with = "tolerant_modules")]
    modules: Vec<ModuleConfig>,
     
     
    #[serde(default)]
    panels: Vec<PanelConfig>,
     
     
    #[serde(default)]
    collapsed: Vec<ModuleCategory>,
     
     
     
     
    #[serde(default, deserialize_with = "tolerant_widgets")]
    widgets: Vec<WidgetConfig>,
     
     
     
    #[serde(default)]
    ui: UiConfig,
     
     
    #[serde(default)]
    friends: Vec<crate::friends::Friend>,
    #[serde(default)]
    favorites: Vec<ModuleId>,
}

 
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct UiConfig {
    accent: [u8; 3],
    base: [u8; 3],
    animation_speed: f32,
    notifications: bool,
     
     
     
    transparency: f32,
     
     
    menu_style: crate::graphic::menu_sidebar::MenuStyle,
     
     
    module_gradient: bool,
     
     
    ui_font: usize,
}

impl Default for UiConfig {
    fn default() -> Self {
        UiConfig {
            accent: theme::DEFAULT_ACCENT,
            base: theme::DEFAULT_BASE,
            animation_speed: 1.0,
            notifications: true,
            transparency: 0.0,
            menu_style: crate::graphic::menu_sidebar::MenuStyle::Panels,
            module_gradient: false,
            ui_font: 0,
        }
    }
}

 
 
 
fn tolerant_modules<'de, D>(deserializer: D) -> Result<Vec<ModuleConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let entries = Vec::<serde_json::Value>::deserialize(deserializer)?;
    Ok(entries
        .into_iter()
        .filter_map(
            |entry| match serde_json::from_value::<ModuleConfig>(entry) {
                Ok(module) => Some(module),
                Err(error) => {
                    log::warn!("config: skipping unreadable module entry: {error}");
                    None
                }
            },
        )
        .collect())
}

 
 
 
fn tolerant_widgets<'de, D>(deserializer: D) -> Result<Vec<WidgetConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let entries = Vec::<serde_json::Value>::deserialize(deserializer)?;
    Ok(entries
        .into_iter()
        .filter_map(
            |entry| match serde_json::from_value::<WidgetConfig>(entry) {
                Ok(widget) => Some(widget),
                Err(error) => {
                    log::warn!("config: skipping unreadable widget entry: {error}");
                    None
                }
            },
        )
        .collect())
}

#[derive(Serialize, Deserialize)]
struct ModuleConfig {
    id: ModuleId,
    key_bind: i32,
    enabled: bool,
     
     
    #[serde(default)]
    expanded: bool,
    settings: Vec<SavedSetting>,
}

#[derive(Serialize, Deserialize)]
struct PanelConfig {
    category: ModuleCategory,
    x: f32,
    y: f32,
}

#[derive(Serialize, Deserialize)]
struct WidgetConfig {
    id: HudWidgetId,
    x: f32,
    y: f32,
}

#[derive(Serialize, Deserialize)]
struct SavedSetting {
    name: String,
    value: SettingValue,
}

 
 
#[derive(Serialize, Deserialize)]
enum SettingValue {
    Toggle(bool),
    Slider(f32),
    Choice(usize),
    Color([f32; 4]),
    BlockList(Vec<String>),
    Text(String),
}

impl SettingValue {
     
    fn capture(setting: &ModuleSetting) -> SettingValue {
        match setting {
            ModuleSetting::Toggle { value, .. } => SettingValue::Toggle(*value),
            ModuleSetting::Slider { value, .. } => SettingValue::Slider(*value),
            ModuleSetting::Choice { value, .. } => SettingValue::Choice(*value),
            ModuleSetting::Color { value, .. } => SettingValue::Color(*value),
            ModuleSetting::BlockList { value, .. } => SettingValue::BlockList(value.clone()),
            ModuleSetting::Text { value, .. } => SettingValue::Text(value.clone()),
        }
    }

     
     
    fn apply(&self, setting: &mut ModuleSetting) {
        match (self, setting) {
            (SettingValue::Toggle(v), ModuleSetting::Toggle { value, .. }) => *value = *v,
            (
                SettingValue::Slider(v),
                ModuleSetting::Slider {
                    value, min, max, ..
                },
            ) => *value = v.clamp(*min, *max),
            (SettingValue::Choice(v), ModuleSetting::Choice { value, options, .. })
                if *v < options.len() =>
            {
                *value = *v;
            }
            (SettingValue::Color(v), ModuleSetting::Color { value, .. }) => *value = *v,
            (SettingValue::BlockList(v), ModuleSetting::BlockList { value, .. }) => {
                *value = v.clone()
            }
            (SettingValue::Text(v), ModuleSetting::Text { value, .. }) => *value = v.clone(),
            _ => {}
        }
    }
}

 
 
 
 
 
 
 
 
fn persisted_enabled(id: ModuleId, enabled: bool) -> bool {
    enabled && id != ModuleId::Panic
}

 
 
 
const CUSTOM_UI_THEME_SETTING: &str = "Follow Theme";

fn migrated_enabled(id: ModuleId, enabled: bool, settings: &[SavedSetting]) -> bool {
    if id == ModuleId::CustomUi
        && !settings
            .iter()
            .any(|setting| setting.name == CUSTOM_UI_THEME_SETTING)
    {
        return false;
    }
    enabled
}

 
pub fn save() {
    if let Err(error) = save_current() {
        log::warn!("config: {error}");
    }
}

fn save_current() -> Result<(), String> {
    let mut config = Config::default();
    for handle in client().modules.handles() {
        let Ok(module) = handle.lock() else {
            continue;
        };
        let data = module.get_module_data();
        config.modules.push(ModuleConfig {
            id: data.id,
            key_bind: data.key_bind as i32,
            enabled: persisted_enabled(data.id, data.enabled),
            expanded: is_expanded(data.id),
            settings: data
                .settings
                .iter()
                 
                 
                 
                .filter(|setting| !crate::macros::module::is_dynamic_setting_name(setting.name()))
                .map(|setting| SavedSetting {
                    name: setting.name().to_string(),
                    value: SettingValue::capture(setting),
                })
                .collect(),
        });
    }

    config.panels = PANEL_POS
        .iter()
        .map(|entry| PanelConfig {
            category: *entry.key(),
            x: entry.value()[0],
            y: entry.value()[1],
        })
        .collect();

    config.collapsed = COLLAPSED
        .iter()
        .filter(|entry| *entry.value())
        .map(|entry| *entry.key())
        .collect();

    config.widgets = WIDGET_POS
        .iter()
        .map(|entry| WidgetConfig {
            id: *entry.key(),
            x: entry.value()[0],
            y: entry.value()[1],
        })
        .collect();

    config.ui = UiConfig {
        accent: theme::accent_rgb(),
        base: theme::base_rgb(),
        animation_speed: anim::speed(),
        notifications: settings::notifications_enabled(),
        transparency: crate::graphic::transparency::strength(),
        menu_style: crate::graphic::menu_sidebar::style(),
        module_gradient: crate::graphic::module_gradient::is_on(),
        ui_font: crate::graphic::fonts::ui_font(),
    };

    config.friends = crate::friends::list();
    config.favorites = FAVORITES.iter().map(|entry| *entry.key()).collect();

    let path = config_path();
    let json = serde_json::to_string_pretty(&config)
        .map_err(|error| format!("could not serialize: {error}"))?;
    std::fs::write(&path, json)
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    write_catalog();
    Ok(())
}

 
 
 
 
 
 
 
 
 
 
 
pub fn write_catalog() {
    let mut modules = Vec::new();
    for handle in client().modules.handles() {
        let Ok(module) = handle.lock() else {
            continue;
        };
        let data = module.get_module_data();
        modules.push(serde_json::json!({
            "id": data.id,
            "name": data.id.display_name(),
            "category": data.category.display_name(),
            "description": data.description,
            "enabled": persisted_enabled(data.id, data.enabled),
            "settings": data
                .settings
                .iter()
                .filter(|setting| !crate::macros::module::is_dynamic_setting_name(setting.name()))
                .map(setting_spec)
                .collect::<Vec<_>>(),
        }));
    }

    let path = base_dir().join(CATALOG_FILE);
    let catalog = serde_json::json!({ "version": 1, "modules": modules });
    match serde_json::to_string_pretty(&catalog) {
        Ok(json) => {
            if let Err(error) = std::fs::write(&path, json) {
                log::warn!("config: could not write {}: {error}", path.display());
            }
        }
        Err(error) => log::warn!("config: could not serialize the module catalog: {error}"),
    }
}

 
 
fn setting_spec(setting: &ModuleSetting) -> serde_json::Value {
    match setting {
        ModuleSetting::Toggle { name, value } => {
            serde_json::json!({ "name": name, "kind": "toggle", "value": value })
        }
        ModuleSetting::Slider {
            name,
            value,
            min,
            max,
        } => serde_json::json!({
            "name": name,
            "kind": "slider",
            "value": value,
            "min": min,
            "max": max,
        }),
        ModuleSetting::Choice {
            name,
            value,
            options,
        } => serde_json::json!({
            "name": name,
            "kind": "choice",
            "value": value,
            "options": options,
        }),
        ModuleSetting::Color { name, value } => {
            serde_json::json!({ "name": name, "kind": "color", "value": value })
        }
        ModuleSetting::BlockList { name, value } => {
            serde_json::json!({ "name": name, "kind": "block_list", "value": value })
        }
        ModuleSetting::Text { name, value } => {
            serde_json::json!({ "name": name, "kind": "text", "value": value })
        }
    }
}

 
 
fn migrated_accent(accent: [u8; 3]) -> [u8; 3] {
    if accent == theme::LEGACY_ACCENT {
        theme::DEFAULT_ACCENT
    } else {
        accent
    }
}

 
 
pub fn configs_dir() -> PathBuf {
    base_dir().join(CONFIGS_DIR)
}

 
 
pub fn named_configs() -> Vec<String> {
    snapshot_names(&configs_dir())
}

 
 
 
fn snapshot_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        })
        .filter_map(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_owned)
        })
        .collect();
    names.sort();
    names
}

 
 
 
fn validated_config_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("enter a config name".to_string());
    }
    if name.chars().count() > 40 {
        return Err("config names can be at most 40 characters".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_'))
    {
        return Err("use only letters, numbers, spaces, - and _".to_string());
    }
    Ok(name.to_string())
}

 
 
 
 
pub fn save_named(name: &str) -> Result<(), String> {
    use std::io::Write;
    let name = validated_config_name(name)?;
    let dir = configs_dir();
    std::fs::create_dir_all(&dir).map_err(|error| format!("could not create configs directory: {error}"))?;
    let path = dir.join(format!("{name}.json"));
    if path.exists() {
        return Err(format!("{name} already exists; choose another name"));
    }
    save_current()?;
    let text = std::fs::read_to_string(config_path())
        .map_err(|error| format!("could not read current config: {error}"))?;
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path)
        .map_err(|error| format!("could not create {name}: {error}"))?;
    if let Err(error) = file.write_all(text.as_bytes()) {
        drop(file);
        let _ = std::fs::remove_file(&path);
        return Err(format!("could not save {name}: {error}"));
    }
    Ok(())
}

pub fn load_named(name: &str) -> Result<(), String> {
    let name = validated_config_name(name)?;
    let text = read_snapshot(&configs_dir(), &name)?;
    serde_json::from_str::<Config>(&text)
        .map_err(|error| format!("{name} is not a valid config: {error}"))?;
    let live = config_path();
    std::fs::write(&live, &text).map_err(|error| format!("could not load {name}: {error}"))?;
    apply(&text).map_err(|error| format!("could not apply {name}: {error}"))
}

 
 
 
fn read_snapshot(dir: &Path, name: &str) -> Result<String, String> {
    let name = validated_config_name(name)?;
    let path = dir.join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read {name}: {error}"))?;
    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| format!("{name} is not a valid config"))?;
    if !parsed.is_object() {
        return Err(format!("{name} is not a valid config"));
    }
    Ok(text)
}

 
fn armed_flags() -> Vec<(ModuleId, bool)> {
    let mut flags = Vec::new();
    for handle in client().modules.handles() {
        if let Ok(module) = handle.lock() {
            let data = module.get_module_data();
            flags.push((data.id, data.enabled));
        }
    }
    flags
}

 
 
 
fn reconcile_armed(before: &[(ModuleId, bool)]) {
    let registry = &client().modules;
    if !registry.is_active() {
        return;  
    }
    for handle in registry.handles() {
        let Ok(module) = handle.lock() else {
            continue;
        };
        let data = module.get_module_data();
        let Some((_, was)) = before.iter().find(|(id, _)| *id == data.id) else {
            continue;
        };
        if *was == data.enabled {
            continue;
        }
        let result = if data.enabled {
            module.on_start()
        } else {
            module.on_stop()
        };
        if let Err(error) = result {
            let verb = if data.enabled { "start" } else { "stop" };
            log::warn!(
                "config: could not {verb} {} after loading: {error}",
                data.id.display_name()
            );
        }
    }
}

 
 
 
pub fn load() {
    let path = config_path();
    let json = match std::fs::read_to_string(&path) {
        Ok(json) => json,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            log::warn!("config: could not read {}: {error}", path.display());
            return;
        }
    };
    if let Err(error) = apply(&json) {
        log::warn!("config: could not parse {}: {error}", path.display());
    }
}

 
 
fn apply(json: &str) -> Result<(), serde_json::Error> {
    let config: Config = serde_json::from_str(json)?;
    let before = armed_flags();

    for handle in client().modules.handles() {
        let Ok(mut module) = handle.lock() else {
            continue;
        };
        let id = module.get_module_data().id;
        let Some(saved) = config.modules.iter().find(|entry| entry.id == id) else {
            continue;
        };

        {
            let data = module.get_module_data_mut();
            let saved_key = KeyboardKey::from(saved.key_bind);
            data.key_bind = if id == ModuleId::MenuHud && saved_key == KeyboardKey::KeyNone {
                KeyboardKey::KeyRShift
            } else {
                saved_key
            };
            for setting in &mut data.settings {
                if let Some(saved_setting) = saved
                    .settings
                    .iter()
                    .find(|entry| entry.name == setting.name())
                {
                    saved_setting.value.apply(setting);
                }
            }
             
             
             
            if id == ModuleId::BlockOutline {
                if let Some(ModuleSetting::Color { value, .. }) = data.get_setting_mut("Color") {
                    if *value == [1.0, 0.84, 0.0, 0.8] {
                        *value = [1.0, 0.0, 0.0, 1.0];
                    }
                }
                if let Some(ModuleSetting::Slider { value, .. }) = data.get_setting_mut("Width") {
                    if *value == 2.0 || *value == 4.0 {
                        *value = 6.0;
                    }
                }
            }
             
             
             
            if id == ModuleId::HandAura {
                if let Some(ModuleSetting::Color { value, .. }) = data.get_setting_mut("Color") {
                    if *value == [0.25, 0.6, 1.0, 0.5] {
                        *value = [1.0, 0.0, 0.0, 0.5];
                    }
                }
            }
             
             
             
            if id == ModuleId::TopBar {
                if let Some(ModuleSetting::Slider { value, .. }) =
                    data.get_setting_mut("Bar Height")
                {
                    if *value == 16.0 {
                        *value = 26.0;
                    }
                }
            }
             
             
             
            if id == ModuleId::Nametags {
                if let Some(ModuleSetting::Slider { value, .. }) = data.get_setting_mut("Range") {
                    if *value == 16.0 {
                        *value = 64.0;
                    }
                }
            }
             
             
             
            if id == ModuleId::AutoTotem {
                crate::module::player::autototem::migrate_legacy_defaults(data);
            }
            data.set_enabled(migrated_enabled(id, saved.enabled, &saved.settings));
        }

        set_expanded(id, saved.expanded);
         
         
         
    }

    for panel in &config.panels {
        set_panel_pos(panel.category, [panel.x, panel.y]);
    }

    for category in &config.collapsed {
        set_collapsed(*category, true);
    }

    for widget in &config.widgets {
        set_widget_pos(widget.id, [widget.x, widget.y]);
    }

    theme::set_accent(migrated_accent(config.ui.accent));
    theme::set_base(config.ui.base);
    anim::set_speed(config.ui.animation_speed);
    settings::set_notifications_enabled(config.ui.notifications);
    crate::graphic::transparency::set_strength(config.ui.transparency);
    crate::graphic::menu_sidebar::set_style(config.ui.menu_style);
    crate::graphic::module_gradient::set_on(config.ui.module_gradient);
    crate::graphic::fonts::set_ui_font(config.ui.ui_font);
    crate::friends::replace(config.friends);
    FAVORITES.clear();
    for id in config.favorites {
        set_favorite(id, true);
    }

     
     
     
    reconcile_armed(&before);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

     
     
     
    #[test]
    fn only_json_snapshots_are_listed_and_a_missing_folder_is_empty() {
        let dir = std::env::temp_dir().join(format!("infered-configs-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(snapshot_names(&dir).is_empty(), "no folder, no names");

        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("HvH.json"), "{}").unwrap();
        std::fs::write(dir.join("Main PVP.JSON"), "{}").unwrap();
        std::fs::write(dir.join("notes.txt"), "not a config").unwrap();
        std::fs::create_dir(dir.join("Folder.json")).unwrap();

        assert_eq!(snapshot_names(&dir), ["HvH", "Main PVP"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

     
     
     
    #[test]
    fn a_snapshot_is_validated_before_it_can_replace_the_live_config() {
        let dir = std::env::temp_dir().join(format!("infered-configs-read-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let good = "{\n  \"modules\": []\n}\n";
        std::fs::write(dir.join("Main PVP.json"), good).unwrap();
        std::fs::write(dir.join("Broken.json"), "{ not json").unwrap();
        std::fs::write(dir.join("Array.json"), "[]").unwrap();

        assert_eq!(read_snapshot(&dir, "Main PVP").unwrap(), good);
        let broken = read_snapshot(&dir, "Broken").unwrap_err();
        assert!(
            broken.contains("Broken"),
            "the name is in the message: {broken}"
        );
        assert!(read_snapshot(&dir, "Array")
            .unwrap_err()
            .contains("not a valid config"));
        assert!(read_snapshot(&dir, "Missing")
            .unwrap_err()
            .contains("Missing"));
        assert!(read_snapshot(&dir, "../Main PVP").is_err());

        std::fs::remove_dir_all(&dir).unwrap();
    }

     
     
    #[test]
    fn snapshot_names_are_checked_before_they_reach_a_path() {
        assert_eq!(validated_config_name("  Main PVP  ").unwrap(), "Main PVP");
        assert!(validated_config_name("HvH-2").is_ok());
        assert!(validated_config_name("_old_config").is_ok());
        for bad in ["", "   ", "../secret", "a/b", "a\\b", "name*", "name.json"] {
            assert!(
                validated_config_name(bad).is_err(),
                "{bad:?} must be refused"
            );
        }
        assert!(validated_config_name(&"x".repeat(41)).is_err());
    }

     
     
    #[test]
    fn minimised_panels_are_tracked_per_category() {
        assert!(!is_collapsed(ModuleCategory::Combat));
        set_collapsed(ModuleCategory::Combat, true);
        assert!(is_collapsed(ModuleCategory::Combat));
        assert!(!is_collapsed(ModuleCategory::Player));
        set_collapsed(ModuleCategory::Combat, false);
        assert!(!is_collapsed(ModuleCategory::Combat));
    }

     
     
     
    #[test]
    fn panic_is_never_saved_armed() {
        assert!(!persisted_enabled(ModuleId::Panic, true));
        assert!(persisted_enabled(ModuleId::Sprint, true));
        assert!(!persisted_enabled(ModuleId::Sprint, false));
    }

     
     
    #[test]
    fn legacy_custom_ui_is_migrated_to_off() {
        let old = vec![SavedSetting {
            name: "Accent".to_string(),
            value: SettingValue::Color([1.0, 0.0, 0.0, 1.0]),
        }];
        assert!(!migrated_enabled(ModuleId::CustomUi, true, &old));

        let current = vec![SavedSetting {
            name: CUSTOM_UI_THEME_SETTING.to_string(),
            value: SettingValue::Toggle(true),
        }];
        assert!(migrated_enabled(ModuleId::CustomUi, true, &current));
        assert!(!migrated_enabled(ModuleId::CustomUi, false, &current));
         
        assert!(migrated_enabled(ModuleId::Sprint, true, &old));
    }

     
     
     
    #[test]
    fn a_text_setting_survives_a_save_and_load_round_trip() {
        let saved = SavedSetting {
            name: "Message".to_string(),
            value: SettingValue::Text("still here".to_string()),
        };
        let json = serde_json::to_string(&saved).unwrap();
        let restored: SavedSetting = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "Message");

        let mut live = ModuleSetting::Text {
            name: "Message".to_string(),
            value: "afk".to_string(),
        };
        restored.value.apply(&mut live);
        assert_eq!(live.get_text_value(), Some("still here"));

         
        assert_eq!(
            serde_json::to_value(SettingValue::capture(&live)).unwrap(),
            serde_json::to_value(SettingValue::Text("still here".to_string())).unwrap()
        );
    }

     
     
    #[test]
    fn favorites_round_trip_and_old_configs_default_to_empty() {
        let config = Config {
            favorites: vec![ModuleId::Sprint, ModuleId::DurabilityAlert],
            ..Config::default()
        };
        let restored: Config = serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(restored.favorites, config.favorites);
        let legacy: Config = serde_json::from_str("{}").unwrap();
        assert!(legacy.favorites.is_empty());
    }

    #[test]
    fn friends_round_trip_through_the_config() {
        use crate::friends::{Friend, FriendMode};

        let config = Config {
            friends: vec![Friend {
                name: "Notch".to_string(),
                mode: FriendMode::Advanced,
                protected: false,
                exempt: vec![ModuleId::TriggerBot],
            }],
            ..Config::default()
        };
        let restored: Config =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(restored.friends.len(), 1);
        assert_eq!(restored.friends[0].name, "Notch");
        assert_eq!(restored.friends[0].mode, FriendMode::Advanced);
        assert_eq!(restored.friends[0].exempt, vec![ModuleId::TriggerBot]);

         
        let legacy: Config = serde_json::from_str("{}").unwrap();
        assert!(legacy.friends.is_empty());
    }

     
     
    #[test]
    fn ui_settings_round_trip_and_default_when_absent() {
        let config = Config {
            ui: UiConfig {
                accent: [1, 2, 3],
                base: [4, 5, 6],
                animation_speed: 2.0,
                notifications: false,
                transparency: 0.65,
                menu_style: crate::graphic::menu_sidebar::MenuStyle::Sidebar,
                module_gradient: true,
                ui_font: 2,
            },
            ..Config::default()
        };
        let restored: Config =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(restored.ui.accent, [1, 2, 3]);
        assert_eq!(restored.ui.base, [4, 5, 6]);
        assert_eq!(restored.ui.animation_speed, 2.0);
        assert!(!restored.ui.notifications);
        assert_eq!(restored.ui.transparency, 0.65);
        assert_eq!(
            restored.ui.menu_style,
            crate::graphic::menu_sidebar::MenuStyle::Sidebar
        );
        assert!(restored.ui.module_gradient);
        assert_eq!(restored.ui.ui_font, 2);

        let legacy: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(legacy.ui.accent, theme::DEFAULT_ACCENT);
        assert_eq!(legacy.ui.base, theme::DEFAULT_BASE);
        assert_eq!(legacy.ui.animation_speed, 1.0);
        assert!(legacy.ui.notifications);
        assert_eq!(legacy.ui.transparency, 0.0);
        assert_eq!(
            legacy.ui.menu_style,
            crate::graphic::menu_sidebar::MenuStyle::Panels
        );
        assert!(!legacy.ui.module_gradient);
        assert_eq!(legacy.ui.ui_font, 0);
    }

    #[test]
    fn the_old_bright_red_accent_migrates_but_custom_colours_do_not() {
        assert_eq!(migrated_accent(theme::LEGACY_ACCENT), theme::DEFAULT_ACCENT);
        assert_eq!(migrated_accent([10, 20, 30]), [10, 20, 30]);
        assert_ne!(theme::DEFAULT_ACCENT, theme::LEGACY_ACCENT);
    }

     
     
    #[test]
    fn the_catalog_describes_every_setting_kind() {
        let slider = setting_spec(&ModuleSetting::Slider {
            name: "Range".to_string(),
            value: 4.5,
            min: 1.0,
            max: 8.0,
        });
        assert_eq!(slider["kind"], "slider");
        assert_eq!(slider["min"], 1.0);
        assert_eq!(slider["max"], 8.0);
        assert_eq!(slider["value"], 4.5);

        let choice = setting_spec(&ModuleSetting::Choice {
            name: "Mode".to_string(),
            value: 1,
            options: vec!["Normal".to_string(), "Silent".to_string()],
        });
        assert_eq!(choice["kind"], "choice");
        assert_eq!(choice["options"][1], "Silent");
        assert_eq!(choice["value"], 1);

        let toggle = setting_spec(&ModuleSetting::Toggle {
            name: "Only while sprinting".to_string(),
            value: true,
        });
        assert_eq!(toggle["kind"], "toggle");
        assert_eq!(toggle["value"], true);

        let text = setting_spec(&ModuleSetting::Text {
            name: "Message".to_string(),
            value: "still here".to_string(),
        });
        assert_eq!(text["kind"], "text");
        assert_eq!(text["value"], "still here");
    }

     
     
    #[test]
    fn a_mismatched_saved_value_leaves_the_setting_alone() {
        let mut live = ModuleSetting::Text {
            name: "Message".to_string(),
            value: "afk".to_string(),
        };
        SettingValue::Toggle(true).apply(&mut live);
        SettingValue::Slider(3.0).apply(&mut live);
        assert_eq!(live.get_text_value(), Some("afk"));
    }
}
