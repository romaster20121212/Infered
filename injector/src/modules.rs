 
 
 
 
 
 
 
 
 
 
 
 
 
 

use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

 
pub const CATALOG_FILE: &str = "infered_modules.json";

 
 
#[derive(Debug, Clone)]
pub struct ModuleView {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub enabled: bool,
    pub settings: Vec<SettingView>,
}

 
#[derive(Debug, Clone)]
pub struct SettingView {
    pub name: String,
    pub kind: SettingKind,
    pub value: SettingValue,
}

 
#[derive(Debug, Clone, PartialEq)]
pub enum SettingKind {
    Toggle,
    Slider { min: f32, max: f32 },
    Choice { options: Vec<String> },
    Color,
    BlockList,
    Text,
}

 
#[derive(Debug, Clone, PartialEq)]
pub enum SettingValue {
    Toggle(bool),
    Slider(f32),
    Choice(usize),
    Color([f32; 4]),
    BlockList(Vec<String>),
    Text(String),
}

impl SettingValue {
     
    fn to_json(&self) -> Value {
        match self {
            SettingValue::Toggle(value) => tagged("Toggle", Value::Bool(*value)),
            SettingValue::Slider(value) => tagged("Slider", number(*value)),
            SettingValue::Choice(value) => tagged("Choice", Value::from(*value)),
            SettingValue::Color(value) => tagged(
                "Color",
                Value::from(
                    value
                        .iter()
                        .map(|channel| *channel as f64)
                        .collect::<Vec<_>>(),
                ),
            ),
            SettingValue::BlockList(value) => tagged("BlockList", Value::from(value.clone())),
            SettingValue::Text(value) => tagged("Text", Value::String(value.clone())),
        }
    }
}

 
fn tagged(key: &str, value: Value) -> Value {
    let mut map = Map::new();
    map.insert(key.to_string(), value);
    Value::Object(map)
}

 
 
fn number(value: f32) -> Value {
    serde_json::Number::from_f64(value as f64)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

 
 
 
 
pub fn read_views(live: &Path, catalog: &Path) -> Vec<ModuleView> {
    let catalog = read_catalog(catalog);
    let config = read_json(live);
    let modules = config
        .as_ref()
        .and_then(|root| root.get("modules"))
        .and_then(Value::as_array);

    let mut views = Vec::with_capacity(catalog.len());
    for module in catalog {
        let saved = modules.and_then(|list| {
            list.iter()
                .find(|entry| entry.get("id").and_then(Value::as_str) == Some(module.id.as_str()))
        });
        let enabled = saved
            .and_then(|entry| entry.get("enabled"))
            .and_then(Value::as_bool)
            .unwrap_or(module.enabled);
        let saved_settings = saved.map(parse_config_settings).unwrap_or_default();
        let settings = module
            .settings
            .into_iter()
            .map(|mut setting| {
                 
                 
                 
                if let Some((_, value)) = saved_settings
                    .iter()
                    .find(|(name, _)| *name == setting.name)
                {
                    if std::mem::discriminant(value) == std::mem::discriminant(&setting.value) {
                        setting.value = value.clone();
                    }
                }
                setting
            })
            .collect();
        views.push(ModuleView {
            id: module.id,
            name: module.name,
            category: module.category,
            description: module.description,
            enabled,
            settings,
        });
    }

    if let Some(list) = modules {
        for entry in list {
            let Some(id) = entry.get("id").and_then(Value::as_str) else {
                continue;
            };
            if views.iter().any(|view| view.id == id) {
                continue;
            }
            views.push(ModuleView {
                id: id.to_string(),
                name: id.to_string(),
                category: "Other".to_string(),
                description: "Not in the current module catalog.".to_string(),
                enabled: entry
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                settings: Vec::new(),
            });
        }
    }

    views
}

 
 
pub fn set_enabled(live: &Path, catalog: &Path, id: &str, enabled: bool) -> Result<(), String> {
    let mut root = read_config_or_empty(live)?;
    let modules = modules_mut(&mut root)?;

    let mut found = false;
    for entry in modules.iter_mut() {
        if entry.get("id").and_then(Value::as_str) == Some(id) {
            if let Some(map) = entry.as_object_mut() {
                map.insert("enabled".to_string(), Value::Bool(enabled));
                found = true;
            }
            break;
        }
    }
    if !found {
        let spec = read_catalog(catalog)
            .into_iter()
            .find(|module| module.id == id)
            .ok_or_else(|| format!("{id} is not in the module catalog"))?;
        modules.push(module_entry(&spec, enabled));
    }

    write_config(live, &root)
}

 
 
 
 
 
pub fn set_setting(
    live: &Path,
    catalog: &Path,
    id: &str,
    setting: &str,
    value: SettingValue,
) -> Result<(), String> {
    let mut root = read_config_or_empty(live)?;
    let modules = modules_mut(&mut root)?;
    if !modules
        .iter()
        .any(|entry| entry.get("id").and_then(Value::as_str) == Some(id))
    {
        let spec = read_catalog(catalog)
            .into_iter()
            .find(|module| module.id == id)
            .ok_or_else(|| format!("{id} is not in the module catalog"))?;
        modules.push(module_entry(&spec, spec.enabled));
    }
    let module = modules
        .iter_mut()
        .find(|entry| entry.get("id").and_then(Value::as_str) == Some(id))
        .ok_or_else(|| format!("{id} is not in the live config yet"))?;
    let settings = module
        .get_mut("settings")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| format!("{id} has no saved settings"))?;
    match settings
        .iter_mut()
        .find(|entry| entry.get("name").and_then(Value::as_str) == Some(setting))
    {
        Some(entry) => {
            if let Some(map) = entry.as_object_mut() {
                map.insert("value".to_string(), value.to_json());
            }
        }
        None => settings.push(serde_json::json!({
            "name": setting,
            "value": value.to_json(),
        })),
    }
    write_config(live, &root)
}

fn modules_mut(root: &mut Value) -> Result<&mut Vec<Value>, String> {
    root.get_mut("modules")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "the live config has no module list".to_string())
}

 
fn module_entry(module: &CatalogModule, enabled: bool) -> Value {
    let settings = module
        .settings
        .iter()
        .map(|setting| {
            serde_json::json!({
                "name": setting.name,
                "value": setting.value.to_json(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "id": module.id,
        "key_bind": 0,
        "enabled": enabled,
        "expanded": false,
        "settings": settings,
    })
}

 
struct CatalogModule {
    id: String,
    name: String,
    category: String,
    description: String,
    enabled: bool,
    settings: Vec<SettingView>,
}

fn read_catalog(path: &Path) -> Vec<CatalogModule> {
    let Some(root) = read_json(path) else {
        return Vec::new();
    };
    root.get("modules")
        .and_then(Value::as_array)
        .map(|entries| entries.iter().filter_map(parse_catalog_module).collect())
        .unwrap_or_default()
}

fn parse_catalog_module(entry: &Value) -> Option<CatalogModule> {
    let id = entry.get("id")?.as_str()?.to_string();
    let name = entry
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&id)
        .to_string();
    let category = entry
        .get("category")
        .and_then(Value::as_str)
        .unwrap_or("Other")
        .to_string();
    let description = entry
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let enabled = entry
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let settings = entry
        .get("settings")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(parse_catalog_setting)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(CatalogModule {
        id,
        name,
        category,
        description,
        enabled,
        settings,
    })
}

fn parse_catalog_setting(entry: &Value) -> Option<SettingView> {
    let name = entry.get("name")?.as_str()?.to_string();
    let kind = entry.get("kind")?.as_str()?;
    let raw = entry.get("value")?;
    match kind {
        "toggle" => Some(SettingView {
            name,
            kind: SettingKind::Toggle,
            value: SettingValue::Toggle(raw.as_bool()?),
        }),
        "slider" => Some(SettingView {
            name,
            kind: SettingKind::Slider {
                min: entry.get("min")?.as_f64()? as f32,
                max: entry.get("max")?.as_f64()? as f32,
            },
            value: SettingValue::Slider(raw.as_f64()? as f32),
        }),
        "choice" => Some(SettingView {
            name,
            kind: SettingKind::Choice {
                options: entry
                    .get("options")?
                    .as_array()?
                    .iter()
                    .filter_map(|option| option.as_str())
                    .map(str::to_string)
                    .collect(),
            },
            value: SettingValue::Choice(raw.as_u64()? as usize),
        }),
        "color" => Some(SettingView {
            name,
            kind: SettingKind::Color,
            value: SettingValue::Color(parse_color(raw)?),
        }),
        "block_list" => Some(SettingView {
            name,
            kind: SettingKind::BlockList,
            value: SettingValue::BlockList(parse_strings(raw)?),
        }),
        "text" => Some(SettingView {
            name,
            kind: SettingKind::Text,
            value: SettingValue::Text(raw.as_str()?.to_string()),
        }),
        _ => None,
    }
}

fn parse_config_settings(entry: &Value) -> Vec<(String, SettingValue)> {
    entry
        .get("settings")
        .and_then(Value::as_array)
        .map(|settings| {
            settings
                .iter()
                .filter_map(|setting| {
                    let name = setting.get("name")?.as_str()?.to_string();
                    let value = parse_config_value(setting.get("value")?)?;
                    Some((name, value))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_config_value(value: &Value) -> Option<SettingValue> {
    let (kind, raw) = value.as_object()?.iter().next()?;
    match kind.as_str() {
        "Toggle" => Some(SettingValue::Toggle(raw.as_bool()?)),
        "Slider" => Some(SettingValue::Slider(raw.as_f64()? as f32)),
        "Choice" => Some(SettingValue::Choice(raw.as_u64()? as usize)),
        "Color" => Some(SettingValue::Color(parse_color(raw)?)),
        "BlockList" => Some(SettingValue::BlockList(parse_strings(raw)?)),
        "Text" => Some(SettingValue::Text(raw.as_str()?.to_string())),
        _ => None,
    }
}

fn parse_color(value: &Value) -> Option<[f32; 4]> {
    let list = value.as_array()?;
    if list.len() != 4 {
        return None;
    }
    let mut color = [0.0; 4];
    for (channel, item) in color.iter_mut().zip(list) {
        *channel = item.as_f64()? as f32;
    }
    Some(color)
}

fn parse_strings(value: &Value) -> Option<Vec<String>> {
    Some(
        value
            .as_array()?
            .iter()
            .filter_map(|item| item.as_str())
            .map(str::to_string)
            .collect(),
    )
}

 
fn read_config_or_empty(live: &Path) -> Result<Value, String> {
    match read_json(live) {
        Some(value) if value.is_object() => Ok(value),
        Some(_) => Err("the live config is not a JSON object".to_string()),
        None if !live.exists() => Ok(serde_json::json!({ "modules": [] })),
        None => Err("the live config could not be parsed".to_string()),
    }
}

fn read_json(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_config(live: &Path, root: &Value) -> Result<(), String> {
    let json = serde_json::to_string_pretty(root)
        .map_err(|error| format!("could not serialize the config: {error}"))?;
    fs::write(live, json).map_err(|error| format!("could not write the live config: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

     
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("infered-modules-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Scratch(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const CATALOG: &str = r#"{
        "version": 1,
        "modules": [
            {
                "id": "Sprint",
                "name": "Sprint",
                "category": "Movement",
                "description": "Keeps you sprinting",
                "enabled": false,
                "settings": [
                    { "name": "Mode", "kind": "choice", "value": 0, "options": ["Vanilla", "Legit"] },
                    { "name": "Range", "kind": "slider", "value": 3.0, "min": 1.0, "max": 6.0 }
                ]
            },
            {
                "id": "Fly",
                "name": "Fly",
                "category": "Movement",
                "description": "Lets you fly",
                "enabled": false,
                "settings": []
            }
        ]
    }"#;

    const CONFIG: &str = r#"{
        "modules": [
            {
                "id": "Sprint",
                "key_bind": 0,
                "enabled": true,
                "expanded": false,
                "settings": [
                    { "name": "Mode", "value": { "Choice": 1 } },
                    { "name": "Range", "value": { "Slider": 4.5 } }
                ]
            }
        ],
        "ui": { "accent": [1, 2, 3], "notifications": true }
    }"#;

    fn fixture(scratch: &Scratch) -> (PathBuf, PathBuf) {
        let live = scratch.0.join("infered_config.json");
        let catalog = scratch.0.join(CATALOG_FILE);
        fs::write(&live, CONFIG).unwrap();
        fs::write(&catalog, CATALOG).unwrap();
        (live, catalog)
    }

    #[test]
    fn views_merge_catalog_names_with_saved_state() {
        let scratch = Scratch::new("merge");
        let (live, catalog) = fixture(&scratch);
        let views = read_views(&live, &catalog);

        let sprint = views.iter().find(|view| view.id == "Sprint").unwrap();
        assert_eq!(sprint.name, "Sprint");
        assert_eq!(sprint.category, "Movement");
        assert!(sprint.enabled, "the saved enabled flag wins");
        assert_eq!(sprint.settings[0].value, SettingValue::Choice(1));
        assert_eq!(sprint.settings[1].value, SettingValue::Slider(4.5));
        assert_eq!(
            sprint.settings[1].kind,
            SettingKind::Slider { min: 1.0, max: 6.0 },
            "bounds come from the catalog"
        );

         
        let fly = views.iter().find(|view| view.id == "Fly").unwrap();
        assert!(!fly.enabled);
        assert!(fly.settings.is_empty());
    }

    #[test]
    fn config_only_modules_stay_reachable() {
        let scratch = Scratch::new("orphan");
        let live = scratch.0.join("infered_config.json");
        let catalog = scratch.0.join(CATALOG_FILE);
        fs::write(&live, r#"{"modules":[{"id":"OldMod","enabled":true}]}"#).unwrap();
        fs::write(&catalog, CATALOG).unwrap();

        let views = read_views(&live, &catalog);
        let orphan = views.iter().find(|view| view.id == "OldMod").unwrap();
        assert!(orphan.enabled);
        assert_eq!(orphan.category, "Other");
    }

    #[test]
    fn enabling_a_module_adds_its_entry_and_keeps_the_rest_of_the_config() {
        let scratch = Scratch::new("add-entry");
        let live = scratch.0.join("infered_config.json");
        let catalog = scratch.0.join(CATALOG_FILE);
        fs::write(&live, r#"{"modules":[],"ui":{"accent":[1,2,3]}}"#).unwrap();
        fs::write(&catalog, CATALOG).unwrap();

        set_enabled(&live, &catalog, "Sprint", true).unwrap();
        let saved: Value = serde_json::from_str(&fs::read_to_string(&live).unwrap()).unwrap();
        let module = &saved["modules"][0];
        assert_eq!(module["id"], "Sprint");
        assert_eq!(module["enabled"], true);
        assert_eq!(module["settings"][0]["name"], "Mode");
        assert_eq!(module["settings"][0]["value"]["Choice"], 0);
        assert_eq!(module["settings"][1]["value"]["Slider"], 3.0);
        assert_eq!(
            saved["ui"]["accent"],
            serde_json::json!([1, 2, 3]),
            "unrelated config fields must survive"
        );
    }

    #[test]
    fn editing_a_setting_writes_the_clients_value_shape() {
        let scratch = Scratch::new("set-value");
        let (live, _) = fixture(&scratch);

        let catalog = scratch.0.join(CATALOG_FILE);
        fs::write(&catalog, CATALOG).unwrap();

        set_setting(
            &live,
            &catalog,
            "Sprint",
            "Range",
            SettingValue::Slider(5.5),
        )
        .unwrap();
        set_setting(&live, &catalog, "Sprint", "Mode", SettingValue::Choice(0)).unwrap();
        let saved: Value = serde_json::from_str(&fs::read_to_string(&live).unwrap()).unwrap();
        let settings = &saved["modules"][0]["settings"];
        assert_eq!(settings[1]["value"], serde_json::json!({ "Slider": 5.5 }));
        assert_eq!(settings[0]["value"], serde_json::json!({ "Choice": 0 }));
        assert_eq!(saved["ui"]["accent"], serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn editing_a_catalog_only_module_creates_its_entry_first() {
        let scratch = Scratch::new("catalog-only");
        let live = scratch.0.join("infered_config.json");
        let catalog = scratch.0.join(CATALOG_FILE);
        fs::write(&live, r#"{"modules":[],"ui":{"accent":[1,2,3]}}"#).unwrap();
        fs::write(&catalog, CATALOG).unwrap();

        set_setting(
            &live,
            &catalog,
            "Sprint",
            "Range",
            SettingValue::Slider(2.0),
        )
        .expect("the entry is created from the catalog before the value is written");
        let saved: Value = serde_json::from_str(&fs::read_to_string(&live).unwrap()).unwrap();
        assert_eq!(saved["modules"][0]["id"], "Sprint");
        assert_eq!(saved["modules"][0]["enabled"], false);
        assert_eq!(
            saved["modules"][0]["settings"][1]["value"],
            serde_json::json!({ "Slider": 2.0 })
        );
        assert_eq!(saved["ui"]["accent"], serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn a_setting_the_saved_entry_lacks_is_appended() {
        let scratch = Scratch::new("new-setting");
        let live = scratch.0.join("infered_config.json");
        let catalog = scratch.0.join(CATALOG_FILE);
        fs::write(
            &live,
            r#"{"modules":[{"id":"Sprint","enabled":true,"settings":[{"name":"Mode","value":{"Choice":1}}]}]}"#,
        )
        .unwrap();
        fs::write(&catalog, CATALOG).unwrap();

        set_setting(
            &live,
            &catalog,
            "Sprint",
            "Range",
            SettingValue::Slider(2.5),
        )
        .unwrap();
        let saved: Value = serde_json::from_str(&fs::read_to_string(&live).unwrap()).unwrap();
        let settings = &saved["modules"][0]["settings"];
        assert_eq!(settings[0]["value"], serde_json::json!({ "Choice": 1 }));
        assert_eq!(settings[1]["name"], "Range");
        assert_eq!(settings[1]["value"], serde_json::json!({ "Slider": 2.5 }));
    }

    #[test]
    fn a_damaged_config_is_refused_and_left_alone() {
        let scratch = Scratch::new("damaged");
        let live = scratch.0.join("infered_config.json");
        let catalog = scratch.0.join(CATALOG_FILE);
        fs::write(&live, "{ not json").unwrap();

        assert!(set_enabled(&live, &catalog, "Sprint", true).is_err());
        assert_eq!(fs::read_to_string(&live).unwrap(), "{ not json");
    }
}
