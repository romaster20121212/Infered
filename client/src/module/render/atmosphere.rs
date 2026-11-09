 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::camera_bridge::{self, SkyHider};
use crate::graphic::atmosphere::{self, SkyMode, SkyStyle};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
 
const MODES: [&str; 9] = [
    "Starry Night",
    "Black Hole",
    "Custom Image",
    "Ice Nebula",
    "Crystal Lattice",
    "Frost Veins",
    "Aurora Veil",
    "Ember Drift",
    "Starfall Rain",
];

 
const IMAGE_FOLDER: &str = "sky_images";
 
const CUSTOM_SETTING: &str = "Custom Sky";

#[derive(Debug)]
pub struct AtmosphereModule {
    pub module: ModuleData,
     
     
     
    images: Vec<PathBuf>,
     
     
    custom: Mutex<Option<PathBuf>>,
}

impl AtmosphereModule {
    pub fn new() -> Self {
        let dir = crate::config::base_dir().join(IMAGE_FOLDER);
        let _ = std::fs::create_dir_all(&dir);
        let images = image_files(&dir);
        let slider = |name: &str, value: f32, min: f32, max: f32| ModuleSetting::Slider {
            name: name.into(),
            value,
            min,
            max,
        };
        let color = |name: &str, [r, g, b]: [f32; 3]| ModuleSetting::Color {
            name: name.into(),
            value: [r, g, b, 1.0],
        };
        Self {
            module: ModuleData {
                id: ModuleId::Atmosphere,
                description: "Replaces the sky with an animated scene: flickering glowing stars \
                              (the default), a black hole with a spinning accretion disk, a \
                              picture of your own dropped into the sky_images folder, or a \
                              noise study — Ice Nebula (Perlin fBm), Crystal Lattice (Worley \
                              cells) or Frost Veins (ridged noise) — each in a colour you pick \
                              (red by default). The scene is fixed in the world — it turns \
                              as you look around and moves with you — and the vanilla sky, \
                              clouds, sun, moon and stars step aside while it runs. Replaces \
                              the whole sky of any dimension that draws one, so it shows in \
                              the Overworld."
                    .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Choice {
                        name: "Sky".into(),
                        value: 0,
                        options: MODES.iter().map(|mode| mode.to_string()).collect(),
                    },
                    ModuleSetting::Choice {
                        name: CUSTOM_SETTING.into(),
                        value: 0,
                        options: image_names(&images),
                    },
                    slider("Star Density", 1.0, 0.2, 2.0),
                    slider("Star Size", 1.0, 0.4, 2.5),
                    slider("Twinkle Speed", 1.0, 0.0, 3.0),
                    ModuleSetting::Toggle {
                        name: "Glow".into(),
                        value: true,
                    },
                    slider("Black Hole Size", 0.24, 0.10, 0.42),
                    slider("Spin Speed", 0.35, 0.0, 1.5),
                    slider("Noise Scale", 1.0, 0.4, 4.0),
                    slider("Flow Speed", 0.6, 0.0, 2.0),
                    slider("Brightness", 1.0, 0.4, 1.6),
                    color("Ice Nebula Color", atmosphere::DEFAULT_NOISE_COLOR),
                    color("Crystal Lattice Color", atmosphere::DEFAULT_NOISE_COLOR),
                    color("Frost Veins Color", atmosphere::DEFAULT_NOISE_COLOR),
                    color("Aurora Veil Color", atmosphere::DEFAULT_NOISE_COLOR),
                    color("Ember Drift Color", atmosphere::DEFAULT_NOISE_COLOR),
                    color("Starfall Rain Color", atmosphere::DEFAULT_NOISE_COLOR),
                ],
            },
            images,
            custom: Mutex::new(None),
        }
    }

    fn style(&self) -> SkyStyle {
        style_from(&self.module)
    }

     
     
     
    fn selected_image(&self) -> Option<PathBuf> {
        if self.style().mode != SkyMode::Custom {
            return None;
        }
        match self.module.get_setting(CUSTOM_SETTING) {
            Some(ModuleSetting::Choice { value, .. }) => self.images.get(*value).cloned(),
            _ => None,
        }
    }

     
     
     
    fn sync_custom(&self, enabled: bool) {
        let wanted = if enabled { self.selected_image() } else { None };
        let mut current = self.custom.lock().unwrap_or_else(|p| p.into_inner());
        if *current != wanted {
            atmosphere::set_custom_image(wanted.clone());
            *current = wanted;
        }
    }

     
     
     
    fn hide(&self) {
        if let Err(error) = camera_bridge::set_sky_hidden(SkyHider::Atmosphere, true) {
            log::debug!("Atmosphere: {error}");
        }
    }

     
    fn release(&self) {
        atmosphere::clear_style();
        self.sync_custom(false);
        if let Err(error) = camera_bridge::set_sky_hidden(SkyHider::Atmosphere, false) {
            log::debug!("Atmosphere: {error}");
        }
    }
}

 
 
fn style_from(data: &ModuleData) -> SkyStyle {
    let stock = SkyStyle::default();
    let slider = |name: &str, fallback: f32| {
        data.get_setting(name)
            .and_then(ModuleSetting::get_slider_value)
            .filter(|value| value.is_finite())
            .unwrap_or(fallback)
    };
    let toggle = |name: &str, fallback: bool| {
        data.get_setting(name)
            .and_then(ModuleSetting::get_toggle_value)
            .unwrap_or(fallback)
    };
     
     
     
    let color = |name: &str, fallback: [f32; 3]| {
        let value = match data.get_setting(name) {
            Some(ModuleSetting::Color { value, .. }) => *value,
            _ => [fallback[0], fallback[1], fallback[2], 1.0],
        };
        let channel = |value: f32, fallback: f32| {
            if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                fallback
            }
        };
        [
            channel(value[0], fallback[0]),
            channel(value[1], fallback[1]),
            channel(value[2], fallback[2]),
        ]
    };
    let mode = match data.get_setting("Sky") {
        Some(ModuleSetting::Choice { value, .. }) => SkyMode::from_choice(*value),
        _ => stock.mode,
    };
    SkyStyle {
        mode,
        density: slider("Star Density", stock.density).clamp(0.2, 2.0),
        star_size: slider("Star Size", stock.star_size).clamp(0.4, 2.5),
        twinkle: slider("Twinkle Speed", stock.twinkle).clamp(0.0, 3.0),
        glow: toggle("Glow", stock.glow),
        hole: slider("Black Hole Size", stock.hole).clamp(0.10, 0.42),
        spin: slider("Spin Speed", stock.spin).clamp(0.0, 1.5),
        noise_scale: slider("Noise Scale", stock.noise_scale).clamp(0.4, 4.0),
        flow: slider("Flow Speed", stock.flow).clamp(0.0, 2.0),
        nebula_color: color("Ice Nebula Color", stock.nebula_color),
        lattice_color: color("Crystal Lattice Color", stock.lattice_color),
        frost_color: color("Frost Veins Color", stock.frost_color),
        aurora_color: color("Aurora Veil Color", stock.aurora_color),
        ember_color: color("Ember Drift Color", stock.ember_color),
        rain_color: color("Starfall Rain Color", stock.rain_color),
        brightness: slider("Brightness", stock.brightness).clamp(0.4, 1.6),
    }
}

 
 
fn image_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file()
                && path.extension().is_some_and(|ext| {
                    ["png", "jpg", "jpeg"]
                        .iter()
                        .any(|kind| ext.eq_ignore_ascii_case(kind))
                })
        })
        .collect();
    files.sort();
    files
}

 
fn image_names(files: &[PathBuf]) -> Vec<String> {
    files
        .iter()
        .map(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .map(Option::unwrap_or_default)
        .collect()
}

impl Module for AtmosphereModule {
    fn on_start(&self) -> anyhow::Result<()> {
         
         
         
         
         
        for guard in [
            crate::jvmti::SKY_DISC_HIDE_TRANSFORM_PARAMS.log_label,
            crate::jvmti::SUN_MOON_STARS_HIDE_TRANSFORM_PARAMS.log_label,
            crate::jvmti::DARK_DISC_HIDE_TRANSFORM_PARAMS.log_label,
            crate::jvmti::SUNRISE_HIDE_TRANSFORM_PARAMS.log_label,
            crate::jvmti::CLOUDS_HIDE_TRANSFORM_PARAMS.log_label,
        ] {
            if !crate::jvmti::transform_installed(guard) {
                log::warn!(
                    "Atmosphere: the {guard} guard is not installed, so the vanilla sky stays on screen"
                );
            }
        }
         
         
         
        atmosphere::retry();
        atmosphere::set_style(self.style());
        self.sync_custom(true);
        self.hide();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.release();
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
         
        self.release();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
         
         
        if atmosphere::is_broken() {
            self.release();
            return Ok(());
        }
         
         
         
        atmosphere::set_style(self.style());
        self.sync_custom(self.module.enabled);
        self.hide();
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

     
    struct ScratchDir(PathBuf);

    impl ScratchDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("infered_sky_images_{name}_{}", std::process::id()));
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
    fn the_starry_night_is_the_default_and_the_fallback() {
        let module = AtmosphereModule::new();
        assert_eq!(style_from(&module.module).mode, SkyMode::StarryNight);
        assert_eq!(MODES[0], "Starry Night");

        let mut module = module;
        for (choice, expected) in [
            (1, SkyMode::BlackHole),
            (2, SkyMode::Custom),
            (3, SkyMode::IceNebula),
            (4, SkyMode::CrystalLattice),
            (5, SkyMode::FrostVeins),
            (99, SkyMode::StarryNight),
        ] {
            if let Some(ModuleSetting::Choice { value, .. }) = module.module.get_setting_mut("Sky")
            {
                *value = choice;
            }
            assert_eq!(style_from(&module.module).mode, expected, "choice {choice}");
        }
    }

     
     
    #[test]
    fn image_files_lists_only_pictures_sorted() {
        let scratch = ScratchDir::new("filter");
        for name in ["b.png", "A.JPG", "notes.txt", "c.mp4", "d.webm", "e.jpeg"] {
            std::fs::write(scratch.0.join(name), b"x").unwrap();
        }
        assert_eq!(
            file_names(&image_files(&scratch.0)),
            vec!["A.JPG", "b.png", "e.jpeg"]
        );
        assert!(image_files(&scratch.0.join("absent")).is_empty());
    }

     
     
    #[test]
    fn image_names_drop_the_extension_and_match_their_file() {
        let files = vec![
            PathBuf::from("dir/Dragon.png"),
            PathBuf::from("dir/clouds.JPEG"),
        ];
        assert_eq!(image_names(&files), vec!["Dragon", "clouds"]);
        assert!(image_names(&[]).is_empty());
    }

     
     
    #[test]
    fn only_the_custom_scene_uses_a_picture() {
        let module = AtmosphereModule::new();
        let mut module = module;
        module.images = vec![
            PathBuf::from("sky_images/dragon.png"),
            PathBuf::from("sky_images/clouds.png"),
        ];
        if let Some(ModuleSetting::Choice { value, options, .. }) =
            module.module.get_setting_mut(CUSTOM_SETTING)
        {
            *value = 1;
            *options = image_names(&module.images);
        }
        assert_eq!(module.selected_image(), None, "still on the starry night");

        if let Some(ModuleSetting::Choice { value, .. }) = module.module.get_setting_mut("Sky") {
            *value = 2;
        }
        assert_eq!(
            module.selected_image(),
            Some(PathBuf::from("sky_images/clouds.png"))
        );

         
         
        if let Some(ModuleSetting::Choice { value, .. }) =
            module.module.get_setting_mut(CUSTOM_SETTING)
        {
            *value = 9;
        }
        assert_eq!(module.selected_image(), None);
    }

     
     
    #[test]
    fn settings_are_clamped_into_range() {
        let mut module = AtmosphereModule::new();
        for (name, value) in [
            ("Star Density", 99.0),
            ("Star Size", -4.0),
            ("Twinkle Speed", 30.0),
            ("Black Hole Size", 0.0),
            ("Spin Speed", 9.0),
            ("Noise Scale", 99.0),
            ("Flow Speed", -4.0),
            ("Brightness", f32::NAN),
        ] {
            if let Some(ModuleSetting::Slider { value: slot, .. }) =
                module.module.get_setting_mut(name)
            {
                *slot = value;
            }
        }
        let style = style_from(&module.module);
        assert_eq!(style.density, 2.0);
        assert_eq!(style.star_size, 0.4);
        assert_eq!(style.twinkle, 3.0);
        assert_eq!(style.hole, 0.10);
        assert_eq!(style.spin, 1.5);
        assert_eq!(style.noise_scale, 4.0);
        assert_eq!(style.flow, 0.0);
        assert_eq!(style.brightness, 1.0, "a NaN falls back to the default");
    }

    fn set_color(module: &mut ModuleData, name: &str, value: [f32; 4]) {
        match module.get_setting_mut(name) {
            Some(ModuleSetting::Color { value: slot, .. }) => *slot = value,
            _ => panic!("missing colour setting {name}"),
        }
    }

     
     
    #[test]
    fn each_noise_study_takes_its_own_colour() {
        let mut module = AtmosphereModule::new();
        let stock = style_from(&module.module);
        assert_eq!(stock.nebula_color, atmosphere::DEFAULT_NOISE_COLOR);
        assert_eq!(stock.lattice_color, atmosphere::DEFAULT_NOISE_COLOR);
        assert_eq!(stock.frost_color, atmosphere::DEFAULT_NOISE_COLOR);

        set_color(&mut module.module, "Ice Nebula Color", [0.1, 0.4, 1.0, 1.0]);
        set_color(
            &mut module.module,
            "Crystal Lattice Color",
            [0.2, 0.8, 0.6, 1.0],
        );
        set_color(
            &mut module.module,
            "Frost Veins Color",
            [0.9, 0.5, 0.1, 1.0],
        );

        let style = style_from(&module.module);
        assert_eq!(style.nebula_color, [0.1, 0.4, 1.0]);
        assert_eq!(style.lattice_color, [0.2, 0.8, 0.6]);
        assert_eq!(style.frost_color, [0.9, 0.5, 0.1]);
    }

     
     
     
    #[test]
    fn noise_colours_are_clamped_channel_by_channel() {
        let mut module = AtmosphereModule::new();
        set_color(
            &mut module.module,
            "Ice Nebula Color",
            [2.0, -1.0, f32::NAN, 0.0],
        );
        let style = style_from(&module.module);
        assert_eq!(
            style.nebula_color,
            [1.0, 0.0, atmosphere::DEFAULT_NOISE_COLOR[2]]
        );
         
        assert_eq!(style.lattice_color, atmosphere::DEFAULT_NOISE_COLOR);
        assert_eq!(style.frost_color, atmosphere::DEFAULT_NOISE_COLOR);
    }

     
     
    #[test]
    fn the_module_exposes_its_documented_controls() {
        let module = AtmosphereModule::new();
        let names: Vec<&str> = module
            .module
            .settings
            .iter()
            .map(ModuleSetting::name)
            .collect();
        for expected in [
            "Sky",
            "Custom Sky",
            "Star Density",
            "Star Size",
            "Twinkle Speed",
            "Glow",
            "Black Hole Size",
            "Spin Speed",
            "Noise Scale",
            "Flow Speed",
            "Brightness",
            "Ice Nebula Color",
            "Crystal Lattice Color",
            "Frost Veins Color",
        ] {
            assert!(names.contains(&expected), "missing setting {expected}");
        }
        assert_eq!(module.module.id, ModuleId::Atmosphere);
        assert_eq!(module.module.category, ModuleCategory::Visuals);
    }
}
