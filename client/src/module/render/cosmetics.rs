use crate::camera_bridge::{set_wing_look, WingLook};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

 
const DEFAULT_CAPE: &[u8] = include_bytes!("../../../../logo's/cape.png");
 
const CAPE_FOLDER: &str = "capes";
const DEFAULT_CHOICE: &str = "Default";
 
 
const CAPE_STYLES: [&str; 2] = ["Minecraft", "Realistic"];
const MINECRAFT_STYLE: usize = 0;
const REALISTIC_STYLE: usize = 1;

 
 
 
const WING_SLIDERS: [(&str, f32, f32, f32); 5] = [
    ("Size", 0.4, 1.6, 1.0),
    ("Flap Speed", 0.0, 3.0, 1.0),
    ("Flap Amount", 0.0, 2.0, 1.0),
    ("Glow", 0.0, 2.0, 1.0),
    ("Opacity", 0.0, 1.0, 1.0),
];
 
const WING_COLOR: [f32; 4] = [0.58, 0.169, 0.243, 1.0];
 
const EMBER_COLOR: [f32; 4] = [1.0, 0.651, 0.361, 1.0];

#[derive(Debug)]
pub struct CosmeticModule {
    data: ModuleData,
    flag: &'static str,
     
    cape_files: Vec<PathBuf>,
     
    applied: Mutex<Option<usize>>,
     
     
    wing_applied: Mutex<Option<WingLook>>,
     
     
    cape_style_applied: Mutex<Option<bool>>,
}

impl CosmeticModule {
    pub fn cape() -> Self {
        let dir = crate::config::base_dir().join(CAPE_FOLDER);
        let _ = std::fs::create_dir_all(&dir);
        let files = cape_files(&dir);
        let options = cape_option_names(&files);
        let mut module = Self::new(
            ModuleId::CustomCape,
            "CAPE",
            "Your artwork on a rigid Minecraft cape or a simulated cloth cape (local cosmetic)",
        );
        module.data.settings.push(ModuleSetting::Choice {
            name: "Style".to_string(),
            value: MINECRAFT_STYLE,
            options: CAPE_STYLES.iter().map(|style| style.to_string()).collect(),
        });
        module.data.settings.push(ModuleSetting::Choice {
            name: "Cape".to_string(),
            value: 0,
            options,
        });
        module.cape_files = files;
        module
    }
    pub fn wings() -> Self {
        let mut module = Self::new(
            ModuleId::PlayerAura,
            "WINGS",
            "Dragon wings with adjustable size, flap, colour and glow (local cosmetic)",
        );
        module
            .data
            .settings
            .extend(
                WING_SLIDERS
                    .iter()
                    .map(|&(name, min, max, default)| ModuleSetting::Slider {
                        name: name.into(),
                        value: default,
                        min,
                        max,
                    }),
            );
        module.data.settings.push(ModuleSetting::Color {
            name: "Wing Color".into(),
            value: WING_COLOR,
        });
        module.data.settings.push(ModuleSetting::Color {
            name: "Ember Color".into(),
            value: EMBER_COLOR,
        });
        module
    }
    fn new(id: ModuleId, flag: &'static str, description: &str) -> Self {
        Self {
            data: ModuleData {
                id,
                description: description.into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
            flag,
            cape_files: Vec::new(),
            applied: Mutex::new(None),
            wing_applied: Mutex::new(None),
            cape_style_applied: Mutex::new(None),
        }
    }

     
     
     
    fn minecraft_style(&self) -> bool {
        match self.data.get_setting("Style") {
            Some(ModuleSetting::Choice { value, .. }) => *value != REALISTIC_STYLE,
            _ => true,
        }
    }

    fn selected_cape(&self) -> usize {
        match self.data.get_setting("Cape") {
            Some(ModuleSetting::Choice { value, .. }) => *value,
            _ => 0,
        }
    }

     
     
     
    fn wing_look(&self) -> WingLook {
        let slider = |index: usize| -> f32 {
            let (name, min, max, default) = WING_SLIDERS[index];
            self.data
                .get_setting(name)
                .and_then(ModuleSetting::get_slider_value)
                .filter(|value| value.is_finite())
                .map(|value| value.clamp(min, max))
                .unwrap_or(default)
        };
        WingLook {
            size: slider(0),
            flap_speed: slider(1),
            flap_amount: slider(2),
            glow: slider(3),
            opacity: slider(4),
            membrane: rgb(self.color("Wing Color", WING_COLOR)),
            glow_color: rgb(self.color("Ember Color", EMBER_COLOR)),
        }
    }

     
     
    fn color(&self, name: &str, fallback: [f32; 4]) -> [f32; 4] {
        match self.data.get_setting(name) {
            Some(ModuleSetting::Color { value, .. }) => std::array::from_fn(|i| {
                if value[i].is_finite() {
                    value[i].clamp(0.0, 1.0)
                } else {
                    fallback[i]
                }
            }),
            _ => fallback,
        }
    }

     
     
    fn sync_wing_look(&self) -> anyhow::Result<()> {
        if self.data.id != ModuleId::PlayerAura {
            return Ok(());
        }
        let look = self.wing_look();
        let mut applied = self
            .wing_applied
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *applied == Some(look) {
            return Ok(());
        }
        set_wing_look(look)?;
        *applied = Some(look);
        Ok(())
    }

     
     
     
    fn sync_cape_style(&self) -> anyhow::Result<()> {
        if self.data.id != ModuleId::CustomCape {
            return Ok(());
        }
        let minecraft = self.minecraft_style();
        let mut applied = self
            .cape_style_applied
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *applied == Some(minecraft) {
            return Ok(());
        }
        crate::camera_bridge::set_cape_style(minecraft)?;
        *applied = Some(minecraft);
        Ok(())
    }

     
     
     
    fn sync_artwork(&self) -> anyhow::Result<()> {
        if self.data.id != ModuleId::CustomCape {
            return Ok(());
        }
        let choice = self.selected_cape();
        let mut applied = self.applied.lock().unwrap();
        if *applied == Some(choice) {
            return Ok(());
        }
        match cape_path(&self.cape_files, choice) {
            Some(path) => {
                let loaded = std::fs::read(path)
                    .map_err(anyhow::Error::from)
                    .and_then(|bytes| crate::camera_bridge::set_cape_artwork(&bytes));
                if let Err(error) = loaded {
                    log::warn!(
                        "Cape {} could not be used, showing the default: {error}",
                        path.display()
                    );
                    crate::camera_bridge::set_cape_artwork(DEFAULT_CAPE)?;
                }
            }
            None => crate::camera_bridge::set_cape_artwork(DEFAULT_CAPE)?,
        }
        *applied = Some(choice);
        Ok(())
    }
}

 
fn cape_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        })
        .collect();
    files.sort();
    files
}

 
fn cape_option_names(files: &[PathBuf]) -> Vec<String> {
    std::iter::once(DEFAULT_CHOICE.to_string())
        .chain(files.iter().map(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default()
        }))
        .collect()
}

 
 
fn cape_path(files: &[PathBuf], choice: usize) -> Option<&PathBuf> {
    choice.checked_sub(1).and_then(|index| files.get(index))
}

impl Module for CosmeticModule {
    fn on_start(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            crate::jvmti::transform_installed("SmokeWorldRender"),
            "Cosmetics require the Minecraft 1.21.4 world-render hook"
        );
        self.sync_artwork()?;
         
         
        self.sync_cape_style()?;
        self.sync_wing_look()?;
        crate::camera_bridge::set_cosmetic_flag(self.flag, true)?;
        if let Err(error) = crate::graphic::smoke::start_source(self.data.id) {
            crate::camera_bridge::set_cosmetic_flag(self.flag, false)?;
            return Err(error);
        }
        Ok(())
    }
    fn on_stop(&self) -> anyhow::Result<()> {
        crate::camera_bridge::set_cosmetic_flag(self.flag, false)?;
        *self
            .wing_applied
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
        *self
            .cape_style_applied
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
        crate::graphic::smoke::stop_source(self.data.id)
    }
    fn on_tick(&self) -> anyhow::Result<()> {
        if !self.data.enabled {
            return Ok(());
        }
        self.sync_artwork()?;
         
         
        self.sync_cape_style()?;
        self.sync_wing_look()
    }
    fn get_module_data(&self) -> &ModuleData {
        &self.data
    }
    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.data
    }
}

 
 
fn rgb(color: [f32; 4]) -> i32 {
    let channel = |value: f32| -> i32 {
        let scaled = value.clamp(0.0, 1.0) * 255.0;
        if scaled.is_finite() {
            scaled.round() as i32 & 0xff
        } else {
            0
        }
    };
    (channel(color[0]) << 16) | (channel(color[1]) << 8) | channel(color[2])
}

#[cfg(test)]
mod tests {
    use super::*;

     
    struct ScratchDir(PathBuf);

    impl ScratchDir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("infered_capes_{name}_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn file_names(files: &[PathBuf]) -> Vec<String> {
        files
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

     
    #[test]
    fn cape_files_lists_only_pngs_sorted() {
        let scratch = ScratchDir::new("filter");
        for name in ["b.png", "A.PNG", "notes.txt", "c.jpg"] {
            std::fs::write(scratch.0.join(name), b"x").unwrap();
        }
        assert_eq!(
            file_names(&cape_files(&scratch.0)),
            vec!["A.PNG".to_string(), "b.png".to_string()]
        );
    }

     
    #[test]
    fn cape_files_missing_folder_is_empty() {
        let scratch = ScratchDir::new("missing");
        assert!(cape_files(&scratch.0.join("absent")).is_empty());
    }

     
    #[test]
    fn cape_option_names_start_with_default() {
        let files = vec![
            PathBuf::from("dir/Dragon.png"),
            PathBuf::from("dir/logo.PNG"),
        ];
        assert_eq!(cape_option_names(&files), vec!["Default", "Dragon", "logo"]);
        assert_eq!(cape_option_names(&[]), vec!["Default"]);
    }

     
     
    #[test]
    fn cape_ships_named_cape_with_minecraft_style() {
        let module = CosmeticModule::cape();
        assert_eq!(module.data.id, ModuleId::CustomCape);
        assert_eq!(module.data.name(), "Cape");
        assert!(!module.data.enabled);
        let names: Vec<&str> = module
            .data
            .settings
            .iter()
            .map(ModuleSetting::name)
            .collect();
        assert_eq!(names, vec!["Style", "Cape"]);
        match module.data.get_setting("Style") {
            Some(ModuleSetting::Choice { value, options, .. }) => {
                assert_eq!(*value, MINECRAFT_STYLE);
                assert_eq!(options, &["Minecraft".to_string(), "Realistic".to_string()]);
            }
            other => panic!("wrong style setting: {other:?}"),
        }
        assert!(module.minecraft_style());
    }

     
     
     
    #[test]
    fn cape_style_choice_selects_realistic_or_minecraft() {
        let mut module = CosmeticModule::cape();
        {
            let Some(ModuleSetting::Choice { value, .. }) = module.data.get_setting_mut("Style")
            else {
                panic!("style setting is missing");
            };
            *value = REALISTIC_STYLE;
        }
        assert!(!module.minecraft_style());
        if let Some(ModuleSetting::Choice { value, .. }) = module.data.get_setting_mut("Style") {
            *value = usize::MAX;
        }
        assert!(module.minecraft_style());
    }

     
     
    #[test]
    fn wings_ship_with_their_style_settings() {
        let module = CosmeticModule::wings();
        assert_eq!(module.data.id, ModuleId::PlayerAura);
        assert!(!module.data.enabled);
        let names: Vec<&str> = module
            .data
            .settings
            .iter()
            .map(ModuleSetting::name)
            .collect();
        assert_eq!(
            names,
            vec![
                "Size",
                "Flap Speed",
                "Flap Amount",
                "Glow",
                "Opacity",
                "Wing Color",
                "Ember Color"
            ]
        );
        let look = module.wing_look();
        assert_eq!(look.size, 1.0);
        assert_eq!(look.flap_speed, 1.0);
        assert_eq!(look.flap_amount, 1.0);
        assert_eq!(look.glow, 1.0);
        assert_eq!(look.opacity, 1.0);
         
        assert_eq!(look.membrane, 0x942b3e);
        assert_eq!(look.glow_color, 0xffa65c);
    }

     
     
    #[test]
    fn wing_style_clamps_corrupt_settings() {
        let mut module = CosmeticModule::wings();
        let mut set = |name: &str, value: f32| {
            module
                .data
                .get_setting_mut(name)
                .expect("wing slider is missing")
                .set_slider_value(value);
        };
        set("Size", f32::NAN);
        set("Flap Speed", 99.0);
        set("Flap Amount", -4.0);
        set("Glow", f32::INFINITY);
        set("Opacity", -1.0);
        let look = module.wing_look();
        assert_eq!(look.size, 1.0);
        assert_eq!(look.flap_speed, 3.0);
        assert_eq!(look.flap_amount, 0.0);
        assert_eq!(look.glow, 1.0);
        assert_eq!(look.opacity, 0.0);
    }

     
     
    #[test]
    fn wing_colours_become_bridge_ints() {
        assert_eq!(rgb([1.0, 0.5, 0.0, 1.0]), 0xff8000);
        assert_eq!(rgb([2.0, -1.0, 0.0, 1.0]), 0xff0000);
        assert_eq!(rgb([f32::NAN, 0.0, 0.0, 1.0]), 0x000000);
        let mut module = CosmeticModule::wings();
        if let Some(ModuleSetting::Color { value, .. }) = module.data.get_setting_mut("Wing Color")
        {
            *value = [0.0, 0.0, 1.0, 1.0];
        }
        assert_eq!(module.wing_look().membrane, 0x0000ff);
    }

     
     
    #[test]
    fn cape_path_maps_choice_to_file() {
        let files = vec![PathBuf::from("a.png"), PathBuf::from("b.png")];
        assert_eq!(cape_path(&files, 0), None);
        assert_eq!(cape_path(&files, 1), Some(&PathBuf::from("a.png")));
        assert_eq!(cape_path(&files, 2), Some(&PathBuf::from("b.png")));
        assert_eq!(cape_path(&files, 3), None);
    }
}
