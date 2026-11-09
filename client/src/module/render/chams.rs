use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::graphic::chams_video;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
 
 
 
 
 
pub(crate) const STYLES: [&str; 16] = [
    "Neon Trail",
    "Solid",
    "Scanline",
    "Ember",
    "Video",
    "Glow",
    "Blur",
    "Galaxy",
    "Prism",
    "Neon",
    "Scan",
    "Liquid Glass",
    "Outline",
    "Contour",
    "Cel",
    "Pulse",
];

 
pub(crate) const CLIP_SETTING: &str = "Video File";
 
const FPS_SETTING: &str = "Frame Rate";
const DEFAULT_FPS: f32 = 24.0;

 
#[derive(Debug)]
pub struct ChamsModule {
    pub module: ModuleData,
     
     
    loaded: Mutex<Option<(PathBuf, u32)>>,
}
impl ChamsModule {
    pub fn new() -> Self {
        let toggle = |name: &str, value| ModuleSetting::Toggle {
            name: name.into(),
            value,
        };
        let folder = chams_video::folder();
        let _ = std::fs::create_dir_all(&folder);
        let options = chams_video::clip_names(&chams_video::clip_files(&folder));
        Self {
            loaded: Mutex::new(None),
            module: ModuleData {
                id: ModuleId::Chams,
                description: "Five classic body looks and eleven outline looks. Toggle Outline \
                     Glow to enable or disable the shared outline pipeline."
                    .into(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    toggle("Players", true),
                    toggle("Self", false),
                    toggle("Armor", false),
                    ModuleSetting::Choice {
                        name: "Style".into(),
                        value: 0,
                        options: STYLES.iter().map(|style| style.to_string()).collect(),
                    },
                    toggle("Outline Glow", true),
                    ModuleSetting::Choice {
                        name: CLIP_SETTING.into(),
                        value: 0,
                        options,
                    },
                    ModuleSetting::Slider {
                        name: FPS_SETTING.into(),
                        value: DEFAULT_FPS,
                        min: 5.0,
                        max: 60.0,
                    },
                    ModuleSetting::Color {
                        name: "Color".into(),
                         
                         
                         
                         
                        value: [1.0, 0.0, 0.0, 1.0],
                    },
                    ModuleSetting::Slider {
                        name: "GlowStrength".into(),
                        value: 1.0,
                        min: 0.0,
                        max: 2.0,
                    },
                    toggle("Pulse", true),
                    toggle("ThroughWalls", true),
                    toggle("OnlyVisible", false),
                    ModuleSetting::Slider {
                        name: "Range".into(),
                        value: 64.0,
                        min: 16.0,
                        max: 128.0,
                    },
                    ModuleSetting::Slider {
                        name: "Glow Radius".into(),
                        value: 12.0,
                        min: 1.0,
                        max: 60.0,
                    },
                    ModuleSetting::Slider {
                        name: "Intensity".into(),
                        value: 1.0,
                        min: 0.0,
                        max: 3.0,
                    },
                    ModuleSetting::Slider {
                        name: "Fill Opacity".into(),
                        value: 1.0,
                        min: 0.0,
                        max: 1.0,
                    },
                    toggle("Outline", true),
                    ModuleSetting::Slider {
                        name: "Outline Width".into(),
                        value: 2.5,
                        min: 0.5,
                        max: 12.0,
                    },
                    ModuleSetting::Slider {
                        name: "Blur".into(),
                        value: 10.0,
                        min: 0.0,
                        max: 40.0,
                    },
                    ModuleSetting::Slider {
                        name: "Tint Amount".into(),
                        value: 0.35,
                        min: 0.0,
                        max: 1.0,
                    },
                    ModuleSetting::Slider {
                        name: "Speed".into(),
                        value: 1.0,
                        min: 0.0,
                        max: 5.0,
                    },
                    toggle("Hairline", true),
                    ModuleSetting::Slider {
                        name: "Thickness".into(),
                        value: 14.0,
                        min: 4.0,
                        max: 40.0,
                    },
                    ModuleSetting::Slider {
                        name: "Refraction".into(),
                        value: 1.0,
                        min: 0.0,
                        max: 3.0,
                    },
                    ModuleSetting::Slider {
                        name: "Dispersion".into(),
                        value: 0.25,
                        min: 0.0,
                        max: 1.0,
                    },
                    ModuleSetting::Slider {
                        name: "Frost".into(),
                        value: 0.0,
                        min: 0.0,
                        max: 20.0,
                    },
                    ModuleSetting::Slider {
                        name: "Glass Tint".into(),
                        value: 0.25,
                        min: 0.0,
                        max: 1.0,
                    },
                    ModuleSetting::Slider {
                        name: "Lines".into(),
                        value: 6.0,
                        min: 2.0,
                        max: 16.0,
                    },
                    ModuleSetting::Slider {
                        name: "Cel Opacity".into(),
                        value: 0.9,
                        min: 0.0,
                        max: 1.0,
                    },
                    ModuleSetting::Slider {
                        name: "Reactivity".into(),
                        value: 0.6,
                        min: 0.0,
                        max: 1.5,
                    },
                    toggle("Idle Beats", true),
                    ModuleSetting::Slider {
                        name: "Audio Sensitivity".into(),
                        value: 1.0,
                        min: 0.0,
                        max: 2.0,
                    },
                ],
            },
        }
    }
}
impl ChamsModule {
     
    fn style(&self) -> i32 {
        match self.module.get_setting("Style") {
            Some(ModuleSetting::Choice { value, options, .. }) => {
                (*value).min(options.len().saturating_sub(1)) as i32
            }
            _ => 0,
        }
    }

     
    fn fps(&self) -> u32 {
        self.module
            .get_setting(FPS_SETTING)
            .and_then(ModuleSetting::get_slider_value)
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(5.0, 60.0).round() as u32)
            .unwrap_or(DEFAULT_FPS as u32)
    }

     
     
     
     
     
     
    fn selected_clip(&self) -> Option<PathBuf> {
        if self.style() != chams_video::STYLE {
            return None;
        }
        let label = match self.module.get_setting(CLIP_SETTING) {
            Some(ModuleSetting::Choice { value, options, .. }) => options.get(*value)?,
            _ => return None,
        };
        chams_video::path_for_label(label)
    }

     
     
    fn sync_clip(&self) {
        let wanted = self.selected_clip().map(|path| (path, self.fps()));
        let mut loaded = self.loaded.lock().unwrap_or_else(|p| p.into_inner());
        if loaded.as_ref() == wanted.as_ref() {
            return;
        }
        match wanted {
            Some((ref path, fps)) => match chams_video::load(path, fps) {
                Ok(clip) => {
                    log::info!(
                        "[Chams] video {}x{}, {} frames, {:.1}s",
                        clip.width(),
                        clip.height(),
                        clip.frames(),
                        clip.length()
                    );
                    chams_video::select(Some(Arc::new(clip)));
                }
                Err(error) => {
                    log::warn!("[Chams] video {}: {error:#}", path.display());
                    chams_video::select(None);
                }
            },
            None => chams_video::select(None),
        }
        *loaded = wanted;
    }

     
    fn release_clip(&self) {
        chams_video::select(None);
        *self.loaded.lock().unwrap_or_else(|p| p.into_inner()) = None;
         
         
        unsafe { chams_video::release() }
    }
}

impl Module for ChamsModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.sync_clip();
        crate::graphic::smoke::start_source(ModuleId::Chams)
    }
    fn on_stop(&self) -> anyhow::Result<()> {
        self.release_clip();
        crate::graphic::smoke::stop_source(ModuleId::Chams)
    }
    fn on_world_inactive(&self) -> anyhow::Result<()> {
        self.release_clip();
        Ok(())
    }
    fn on_tick(&self) -> anyhow::Result<()> {
         
        self.sync_clip();
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
    use crate::graphic::chams::{Config, COLLECT_DESCRIPTOR};
    #[test]
    fn model_chams_defaults_and_settings() {
        let m = ChamsModule::new();
        assert_eq!(m.module.category, ModuleCategory::Render);
        for (name, value) in [
            ("Players", true),
            ("Self", false),
            ("Armor", false),
            ("Pulse", true),
            ("ThroughWalls", true),
            ("OnlyVisible", false),
        ] {
            assert_eq!(
                m.module.get_setting(name).unwrap().get_toggle_value(),
                Some(value)
            );
        }
        for name in ["Color", "GlowStrength", "Range"] {
            assert!(m.module.get_setting(name).is_some());
        }
        if let Some(ModuleSetting::Color { value, .. }) = m.module.get_setting("Color") {
            assert!((value[0] - 1.0).abs() < 1e-5);
            assert!(value[1].abs() < 1e-5);
            assert!(value[2].abs() < 1e-5);
        } else {
            panic!("Color setting missing");
        }
        assert!(m.module.get_setting("Fill Color").is_none());
    }

     
     
     
    #[test]
    fn the_style_list_holds_both_kinds_in_order() {
        use crate::graphic::chams_warped;
        assert_eq!(STYLES[chams_video::STYLE as usize], "Video");
        assert_eq!(STYLES.len(), chams_warped::LAST_STYLE + 1);
        for (mode, name) in chams_warped::MODE_NAMES.iter().enumerate() {
            assert_eq!(STYLES[chams_warped::FIRST_STYLE + mode], *name);
        }
        let m = ChamsModule::new();
        match m.module.get_setting("Style") {
            Some(ModuleSetting::Choice { options, .. }) => assert_eq!(options, &STYLES),
            other => panic!("Style is not a choice: {other:?}"),
        }
    }

     
     
     
    #[test]
    fn the_armor_toggle_reaches_the_material_config() {
        let mut m = ChamsModule::new();
        assert!(!Config::read(&m.module).armor);
        m.module
            .get_setting_mut("Armor")
            .expect("Armor must exist")
            .set_toggle_value(true);
        assert!(Config::read(&m.module).armor);
        m.module
            .get_setting_mut("Armor")
            .expect("Armor must exist")
            .set_toggle_value(false);
        assert!(!Config::read(&m.module).armor);
    }

     
     
     
     
     
     
     
     
    #[test]
    fn the_bridge_collect_descriptor_carries_the_armor_flag() {
        assert_eq!(
            COLLECT_DESCRIPTOR,
            "(Ljava/lang/Object;[Ljava/lang/Object;FDDDZ)[F"
        );
        assert!(
            COLLECT_DESCRIPTOR.ends_with("Z)[F"),
            "the last argument is the armor flag and the result is still a float array"
        );
        assert_eq!(
            crate::graphic::chams_classic::COLLECT_DESCRIPTOR,
            COLLECT_DESCRIPTOR,
            "InferedClassicChamsBridge.collect takes the same seven arguments"
        );
    }
}
