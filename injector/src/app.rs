 
 
 
 

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::friends::{FriendMode, FriendView};
use crate::inject;
use crate::modules::{self, ModuleView, SettingValue};
use crate::platform::{find_minecraft_processes, InjectError, ProcessInfo};

 
const LIVE_CONFIG_FILE: &str = "infered_config.json";

 
#[derive(Debug, Clone)]
pub enum InjectionStatus {
     
    Idle,
     
    Scanning,
     
    Injecting(u32),
     
    Done(u32),
     
    Failed(String),
}

impl InjectionStatus {
     
    pub fn message(&self) -> String {
        match self {
            InjectionStatus::Idle => "Ready.".to_string(),
            InjectionStatus::Scanning => "Scanning for Minecraft…".to_string(),
            InjectionStatus::Injecting(pid) => format!("Injecting into process {pid}…"),
            InjectionStatus::Done(pid) => format!("Injected into process {pid}."),
            InjectionStatus::Failed(reason) => format!("Error: {reason}"),
        }
    }
}

 
#[derive(Debug, Clone)]
pub struct AddonInfo {
    pub file_name: String,
    pub title: String,
    pub description: String,
    pub enabled: bool,
}

 
pub struct InjectorApp {
    processes: Vec<ProcessInfo>,
    selected_pid: Option<u32>,
    status: InjectionStatus,
     
    pending: Option<(u32, Receiver<Result<(), InjectError>>)>,
    addon_dir: PathBuf,
    addons: Vec<AddonInfo>,
    config_dir: PathBuf,
    configs: Vec<String>,
}

impl InjectorApp {
     
    pub fn new() -> Self {
        let mut app = Self {
            processes: Vec::new(),
            selected_pid: None,
            status: InjectionStatus::Idle,
            pending: None,
            addon_dir: addon_directory(),
            addons: Vec::new(),
            config_dir: config_directory(),
            configs: Vec::new(),
        };
        app.refresh_addons();
        app.refresh_configs();
        app
    }

     
    pub fn processes(&self) -> &[ProcessInfo] {
        &self.processes
    }

     
    pub fn selected_process(&self) -> Option<&ProcessInfo> {
        let pid = self.selected_pid?;
        self.processes.iter().find(|p| p.pid == pid)
    }

     
    pub fn status(&self) -> &InjectionStatus {
        &self.status
    }

     
    pub fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

     
    pub fn addons(&self) -> &[AddonInfo] {
        &self.addons
    }

     
    pub fn refresh_addons(&mut self) {
        let _ = fs::create_dir_all(&self.addon_dir);
        let disabled = read_disabled_addons(&self.addon_dir);
        let mut paths: Vec<PathBuf> = match fs::read_dir(&self.addon_dir) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file()
                        && path
                            .extension()
                            .and_then(|extension| extension.to_str())
                            .is_some_and(|extension| extension.eq_ignore_ascii_case("inf"))
                })
                .collect(),
            Err(_) => {
                self.addons.clear();
                return;
            }
        };
        paths.sort();

        self.addons = paths
            .into_iter()
            .map(|path| {
                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let (title, description, _valid) = read_addon_metadata(&path);
                AddonInfo {
                    enabled: !disabled.contains(&file_name),
                    file_name,
                    title,
                    description,
                }
            })
            .collect();
    }

     
     
    pub fn set_addon_enabled(&mut self, file_name: &str, enabled: bool) -> Result<(), String> {
        self.require_addon_path(file_name)?;
        let mut disabled = read_disabled_addons(&self.addon_dir);
        if enabled {
            disabled.remove(file_name);
        } else {
            disabled.insert(file_name.to_string());
        }
        write_disabled_addons(&self.addon_dir, &disabled)?;
        self.refresh_addons();
        Ok(())
    }

     
     
     
    pub fn import_addon(&mut self, source: &Path) -> Result<String, String> {
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .ok_or_else(|| "invalid addon filename".to_string())?;
        if !file_name.to_ascii_lowercase().ends_with(".inf") {
            return Err("only .inf addon files can be loaded".to_string());
        }
        if !is_valid_addon_name(&file_name) {
            return Err("invalid addon filename".to_string());
        }
        if !source.is_file() {
            return Err(format!("{file_name} is not a file"));
        }
        fs::create_dir_all(&self.addon_dir)
            .map_err(|error| format!("could not create the addons folder: {error}"))?;
        let destination = self.addon_dir.join(&file_name);
        if destination.exists() {
            return Err(format!("an addon named {file_name} is already loaded"));
        }
        fs::copy(source, &destination)
            .map_err(|error| format!("could not load {file_name}: {error}"))?;
        let (_, _, valid) = read_addon_metadata(&destination);
        if !valid {
            let _ = fs::remove_file(&destination);
            return Err(format!("{file_name} is not a valid addon"));
        }
         
         
        let mut disabled = read_disabled_addons(&self.addon_dir);
        if disabled.remove(&file_name) {
            write_disabled_addons(&self.addon_dir, &disabled)?;
        }
        self.refresh_addons();
        Ok(file_name)
    }

     
    pub fn configs(&self) -> &[String] {
        &self.configs
    }

     
    pub fn refresh_configs(&mut self) {
        self.configs = list_configs(&self.config_dir);
    }

     
     
    pub fn save_config(&mut self, name: &str) -> Result<(), String> {
        save_config_into(&live_config_path(), &self.config_dir, name)?;
        self.refresh_configs();
        Ok(())
    }

     
     
     
    pub fn load_config(&mut self, name: &str) -> Result<(), String> {
        load_config_from(&live_config_path(), &self.config_dir, name)
    }

     
     
     
     
    pub fn load_config_file(&mut self, source: &Path) -> Result<(), String> {
        load_config_file_into(&live_config_path(), source)
    }

     
     
    pub fn module_views(&self) -> Vec<ModuleView> {
        modules::read_views(&live_config_path(), &module_catalog_path())
    }

     
     
    pub fn set_module_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        modules::set_enabled(&live_config_path(), &module_catalog_path(), id, enabled)
    }

     
    pub fn update_setting(
        &mut self,
        id: &str,
        setting: &str,
        value: &SettingValue,
    ) -> Result<(), String> {
        modules::set_setting(
            &live_config_path(),
            &module_catalog_path(),
            id,
            setting,
            value.clone(),
        )
    }

     
    pub fn friends(&self) -> Result<Vec<FriendView>, String> {
        crate::friends::read(&live_config_path())
    }

     
    pub fn add_friend(&mut self, name: &str) -> Result<(), String> {
        crate::friends::add(&live_config_path(), name)
    }

     
    pub fn remove_friend(&mut self, name: &str) -> Result<(), String> {
        crate::friends::remove(&live_config_path(), name)
    }

     
    pub fn set_friend_mode(&mut self, name: &str, mode: FriendMode) -> Result<(), String> {
        crate::friends::set_mode(&live_config_path(), name, mode)
    }

     
    pub fn set_friend_protected(&mut self, name: &str, protected: bool) -> Result<(), String> {
        crate::friends::set_protected(&live_config_path(), name, protected)
    }

     
    pub fn set_friend_exempt(
        &mut self,
        name: &str,
        module: &str,
        exempt: bool,
    ) -> Result<(), String> {
        crate::friends::set_exempt(&live_config_path(), name, module, exempt)
    }

     
    pub fn has_live_config(&self) -> bool {
        live_config_path().is_file()
    }

     
    pub fn has_module_catalog(&self) -> bool {
        module_catalog_path().is_file()
    }

     
    pub fn delete_addon(&mut self, file_name: &str) -> Result<(), String> {
        let path = self.require_addon_path(file_name)?;
        fs::remove_file(&path).map_err(|error| format!("could not delete {file_name}: {error}"))?;
        let mut disabled = read_disabled_addons(&self.addon_dir);
        disabled.remove(file_name);
        write_disabled_addons(&self.addon_dir, &disabled)?;
        self.refresh_addons();
        Ok(())
    }

    fn require_addon_path(&self, file_name: &str) -> Result<PathBuf, String> {
        if !is_valid_addon_name(file_name) {
            return Err("invalid addon filename".to_string());
        }
        let path = self.addon_dir.join(file_name);
        if !path.is_file() {
            return Err(format!("addon {file_name} no longer exists"));
        }
        Ok(path)
    }

     
    pub fn select(&mut self, pid: u32) {
        if self.processes.iter().any(|p| p.pid == pid) {
            self.selected_pid = Some(pid);
        }
    }

     
     
    pub fn scan(&mut self) {
        if self.is_busy() {
            return;
        }
        self.status = InjectionStatus::Scanning;
        self.processes = find_minecraft_processes();

        let kept = self
            .selected_pid
            .is_some_and(|pid| self.processes.iter().any(|p| p.pid == pid));
        if !kept {
            self.selected_pid = self.processes.first().map(|p| p.pid);
        }
        self.status = InjectionStatus::Idle;
    }

     
     
    pub fn inject_selected(&mut self) {
        if self.is_busy() {
            return;
        }
        let chosen = self.selected_pid;
        self.scan();
        match chosen {
            Some(pid) if self.selected_pid == Some(pid) => self.start_injection(),
            _ => {
                self.status = InjectionStatus::Failed(
                    "no Minecraft process selected — start the game first".to_string(),
                );
            }
        }
    }

     
     
    pub fn start_injection(&mut self) {
        if self.is_busy() {
            return;
        }
        let Some(pid) = self.selected_pid else {
            return;
        };

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(inject::inject(pid));
        });
        self.pending = Some((pid, rx));
        self.status = InjectionStatus::Injecting(pid);
    }

     
     
    pub fn poll(&mut self) -> bool {
        let Some((pid, rx)) = &self.pending else {
            return false;
        };
        let pid = *pid;
        match rx.try_recv() {
            Ok(result) => {
                self.status = match result {
                    Ok(()) => InjectionStatus::Done(pid),
                    Err(e) => InjectionStatus::Failed(e.to_string()),
                };
                self.pending = None;
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.status =
                    InjectionStatus::Failed("injection worker stopped unexpectedly".to_string());
                self.pending = None;
                true
            }
        }
    }
}

impl Default for InjectorApp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl InjectorApp {
     
    pub(crate) fn stub_configs(&mut self, configs: Vec<String>) {
        self.configs = configs;
    }

     
    pub(crate) fn stub_addons(&mut self, addons: Vec<AddonInfo>) {
        self.addons = addons;
    }

     
     
    pub(crate) fn stub() -> Self {
        Self {
            processes: Vec::new(),
            selected_pid: None,
            status: InjectionStatus::Idle,
            pending: None,
            addon_dir: PathBuf::new(),
            addons: Vec::new(),
            config_dir: PathBuf::new(),
            configs: Vec::new(),
        }
    }
}

 
 
fn working_directory() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

 
fn config_directory() -> PathBuf {
    working_directory().join("configs")
}

 
 
fn live_config_path() -> PathBuf {
    working_directory().join(LIVE_CONFIG_FILE)
}

 
fn module_catalog_path() -> PathBuf {
    working_directory().join(modules::CATALOG_FILE)
}

fn config_file(config_dir: &Path, name: &str) -> PathBuf {
    config_dir.join(format!("{name}.json"))
}

 
 
fn validate_config_name(name: &str) -> Result<String, String> {
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

 
fn list_configs(config_dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(config_dir) else {
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

 
 
fn save_config_into(live: &Path, config_dir: &Path, name: &str) -> Result<(), String> {
    let name = validate_config_name(name)?;
    if !live.is_file() {
        return Err("nothing to save yet: change a setting in Infered first".to_string());
    }
    fs::create_dir_all(config_dir)
        .map_err(|error| format!("could not create the configs folder: {error}"))?;
    fs::copy(live, config_file(config_dir, &name))
        .map_err(|error| format!("could not save {name}: {error}"))?;
    Ok(())
}

 
 
 
fn load_config_from(live: &Path, config_dir: &Path, name: &str) -> Result<(), String> {
    let name = validate_config_name(name)?;
    let text = validated_config_text(&config_file(config_dir, &name), &name)?;
    fs::write(live, text).map_err(|error| format!("could not load {name}: {error}"))?;
    Ok(())
}

 
 
 
fn load_config_file_into(live: &Path, source: &Path) -> Result<(), String> {
    let label = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config");
    let text = validated_config_text(source, label)?;
    fs::write(live, text).map_err(|error| format!("could not load {label}: {error}"))?;
    Ok(())
}

 
 
 
fn validated_config_text(path: &Path, label: &str) -> Result<String, String> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("could not read {label}: {error}"))?;
    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| format!("{label} is not a valid config"))?;
    if !parsed.is_object() {
        return Err(format!("{label} is not a valid config"));
    }
    Ok(text)
}

 
 
fn addon_directory() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("addons")
}

fn addon_state_path(addon_dir: &Path) -> PathBuf {
    addon_dir
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("infered_addons.state")
}

 
fn is_valid_addon_name(file_name: &str) -> bool {
    !file_name.is_empty()
        && file_name != "."
        && file_name != ".."
        && !file_name.contains('/')
        && !file_name.contains('\\')
        && file_name.to_ascii_lowercase().ends_with(".inf")
}

fn read_disabled_addons(addon_dir: &Path) -> BTreeSet<String> {
    let path = addon_state_path(addon_dir);
    let Ok(source) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    source
        .lines()
        .filter_map(|line| line.strip_prefix("disabled="))
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

fn write_disabled_addons(addon_dir: &Path, disabled: &BTreeSet<String>) -> Result<(), String> {
    let contents = disabled
        .iter()
        .map(|name| format!("disabled={name}\\n"))
        .collect::<String>();
    fs::write(addon_state_path(addon_dir), contents)
        .map_err(|error| format!("could not save addon settings: {error}"))
}

fn read_addon_metadata(path: &Path) -> (String, String, bool) {
    let fallback_title = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let Ok(source) = fs::read_to_string(path) else {
        return (
            fallback_title,
            "Could not read this source file.".to_string(),
            false,
        );
    };

    let mut title = fallback_title;
    let mut description = "No description provided.".to_string();
    let mut valid = true;
    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            valid = false;
            continue;
        };
        match key.trim() {
            "addon" | "title" => {
                if !value.trim().is_empty() {
                    title = value.trim().to_string();
                }
            }
            "description" => {
                if !value.trim().is_empty() {
                    description = value.trim().to_string();
                }
            }
            "category" | "menu" | "mod" | "module" => {}
            _ => valid = false,
        }
    }
    (title, description, valid)
}

#[cfg(test)]
mod tests {
    use super::*;

     
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("infered-{name}-{}", std::process::id()));
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

    fn app_for(addon_dir: PathBuf) -> InjectorApp {
        InjectorApp {
            processes: Vec::new(),
            selected_pid: None,
            status: InjectionStatus::Idle,
            pending: None,
            addon_dir,
            addons: Vec::new(),
            config_dir: PathBuf::new(),
            configs: Vec::new(),
        }
    }

    const VALID_ADDON: &str =
        "addon = Quick Mods\ncategory = Quick Mods\nmod = Spam|click|Auto clicker\n";

    #[test]
    fn importing_copies_the_addon_and_enables_it() {
        let scratch = Scratch::new("import-copy");
        let source = scratch.0.join("picked.inf");
        fs::write(&source, VALID_ADDON).unwrap();
        let addon_dir = scratch.0.join("addons");
        let mut app = app_for(addon_dir.clone());

        assert_eq!(app.import_addon(&source).unwrap(), "picked.inf");
        assert!(addon_dir.join("picked.inf").is_file());
        assert_eq!(app.addons().len(), 1);
        assert!(app.addons()[0].enabled);
    }

    #[test]
    fn importing_rejects_non_inf_files() {
        let scratch = Scratch::new("import-ext");
        let source = scratch.0.join("picked.dll");
        fs::write(&source, VALID_ADDON).unwrap();
        let addon_dir = scratch.0.join("addons");
        let mut app = app_for(addon_dir.clone());

        assert!(app.import_addon(&source).is_err());
        assert!(!addon_dir.join("picked.dll").exists());
        assert!(app.addons().is_empty());
    }

    #[test]
    fn importing_refuses_to_overwrite_an_existing_addon() {
        let scratch = Scratch::new("import-dup");
        let addon_dir = scratch.0.join("addons");
        fs::create_dir_all(&addon_dir).unwrap();
        fs::write(addon_dir.join("picked.inf"), VALID_ADDON).unwrap();
        let source = scratch.0.join("picked.inf");
        fs::write(&source, "addon = Other\ncategory = X\nmod = a|click|b\n").unwrap();
        let mut app = app_for(addon_dir.clone());

        assert!(app.import_addon(&source).is_err());
        assert_eq!(
            fs::read_to_string(addon_dir.join("picked.inf")).unwrap(),
            VALID_ADDON
        );
    }

    #[test]
    fn importing_rejects_a_file_that_is_not_an_addon() {
        let scratch = Scratch::new("import-invalid");
        let source = scratch.0.join("junk.inf");
        fs::write(&source, "this line has no equals sign\n").unwrap();
        let addon_dir = scratch.0.join("addons");
        let mut app = app_for(addon_dir.clone());

        assert!(app.import_addon(&source).is_err());
        assert!(!addon_dir.join("junk.inf").exists());
    }

    #[test]
    fn config_names_must_be_short_and_filename_safe() {
        assert_eq!(validate_config_name("  PvP setup ").unwrap(), "PvP setup");
        assert_eq!(validate_config_name("legit_1-b").unwrap(), "legit_1-b");
        assert!(validate_config_name("   ").is_err());
        assert!(validate_config_name("../escape").is_err());
        assert!(validate_config_name("a/b").is_err());
        assert!(validate_config_name("dot.json").is_err());
        assert!(validate_config_name(&"x".repeat(41)).is_err());
    }

    #[test]
    fn saving_then_loading_round_trips_the_live_config() {
        let scratch = Scratch::new("config-roundtrip");
        let live = scratch.0.join(LIVE_CONFIG_FILE);
        let config_dir = scratch.0.join("configs");
        fs::write(&live, r#"{"modules":[{"id":"sprint"}]}"#).unwrap();

        save_config_into(&live, &config_dir, "legit").unwrap();
        assert_eq!(list_configs(&config_dir), vec!["legit".to_string()]);

        fs::write(&live, r#"{"modules":[]}"#).unwrap();
        load_config_from(&live, &config_dir, "legit").unwrap();
        assert_eq!(
            fs::read_to_string(&live).unwrap(),
            r#"{"modules":[{"id":"sprint"}]}"#
        );
    }

    #[test]
    fn saving_needs_a_live_config_to_copy() {
        let scratch = Scratch::new("config-no-live");
        let live = scratch.0.join(LIVE_CONFIG_FILE);
        let config_dir = scratch.0.join("configs");

        assert!(save_config_into(&live, &config_dir, "legit").is_err());
        assert!(list_configs(&config_dir).is_empty());
    }

    #[test]
    fn loading_refuses_a_damaged_config_and_keeps_the_live_one() {
        let scratch = Scratch::new("config-damaged");
        let live = scratch.0.join(LIVE_CONFIG_FILE);
        let config_dir = scratch.0.join("configs");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("broken.json"), "{ not json").unwrap();
        fs::write(config_dir.join("array.json"), "[1, 2]").unwrap();
        fs::write(&live, r#"{"modules":[]}"#).unwrap();

        assert!(load_config_from(&live, &config_dir, "broken").is_err());
        assert!(load_config_from(&live, &config_dir, "array").is_err());
        assert!(load_config_from(&live, &config_dir, "missing").is_err());
        assert_eq!(fs::read_to_string(&live).unwrap(), r#"{"modules":[]}"#);
    }

    #[test]
    fn loading_a_config_file_replaces_the_live_config() {
        let scratch = Scratch::new("config-file-load");
        let live = scratch.0.join(LIVE_CONFIG_FILE);
        let source = scratch.0.join("downloaded.json");
        fs::write(&live, r#"{"modules":[]}"#).unwrap();
        fs::write(&source, r#"{"modules":[{"id":"Sprint","enabled":true}]}"#).unwrap();

        load_config_file_into(&live, &source).unwrap();
        assert_eq!(
            fs::read_to_string(&live).unwrap(),
            fs::read_to_string(&source).unwrap()
        );
    }

    #[test]
    fn loading_a_damaged_config_file_keeps_the_live_config() {
        let scratch = Scratch::new("config-file-damaged");
        let live = scratch.0.join(LIVE_CONFIG_FILE);
        let broken = scratch.0.join("broken.json");
        let array = scratch.0.join("array.json");
        fs::write(&live, r#"{"modules":[]}"#).unwrap();
        fs::write(&broken, "{ not json").unwrap();
        fs::write(&array, "[1, 2]").unwrap();

        assert!(load_config_file_into(&live, &broken).is_err());
        assert!(load_config_file_into(&live, &array).is_err());
        assert!(load_config_file_into(&live, &scratch.0.join("missing.json")).is_err());
        assert_eq!(fs::read_to_string(&live).unwrap(), r#"{"modules":[]}"#);
    }
}
