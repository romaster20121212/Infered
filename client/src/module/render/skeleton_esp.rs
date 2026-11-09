 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::chams::{skeleton_positions, Config as Look};
use crate::module::render::skeleton_rig::{self, Anatomy, Detail};
use crate::module::render::trail::bounded;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
 
pub(crate) const BONE_MODE: i32 = 10;

const DEFAULT_RANGE: f32 = 48.0;
const DEFAULT_COLOR: [f32; 4] = [1.0, 0.32, 0.28, 1.0];
const DEFAULT_THICKNESS: f32 = 0.035;
const DEFAULT_GLOW: f32 = 1.0;
const DEFAULT_RIB_PAIRS: f32 = 6.0;
const DEFAULT_VERTEBRAE: f32 = 7.0;

 
#[derive(Clone)]
pub(crate) struct Config {
    pub(crate) enabled: bool,
     
    pub(crate) color: [f32; 4],
     
    pub(crate) anatomy: Anatomy,
     
     
     
    look: Look,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            color: DEFAULT_COLOR,
            anatomy: Anatomy::default(),
            look: Look {
                players: true,
                self_player: false,
                armor: false,
                color: DEFAULT_COLOR,
                strength: DEFAULT_GLOW,
                pulse: false,
                through: true,
                range: DEFAULT_RANGE,
            },
        }
    }
}

impl Config {
    pub(crate) fn from_data(data: &ModuleData) -> Self {
        let toggle = |name, fallback| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_toggle_value)
                .unwrap_or(fallback)
        };
        let color = match data.get_setting("Color") {
            Some(ModuleSetting::Color { value, .. }) => std::array::from_fn(|i| {
                if value[i].is_finite() {
                    value[i].clamp(0.0, 1.0)
                } else {
                    DEFAULT_COLOR[i]
                }
            }),
            _ => DEFAULT_COLOR,
        };
         
         
        let count = |name: &str, fallback: f32, min: f32, max: f32| -> usize {
            bounded(data, name, fallback, min, max).round() as usize
        };
        let detail = match data.get_setting("Detail") {
            Some(ModuleSetting::Choice { value, .. }) => Detail::from_choice(*value),
            _ => Detail::default(),
        };
        Self {
            enabled: data.enabled,
            color,
            anatomy: Anatomy {
                skull: toggle("Skull", true),
                ribcage: toggle("Ribcage", true),
                rib_pairs: count("Rib Pairs", DEFAULT_RIB_PAIRS, 2.0, 10.0),
                spine: toggle("Spine", true),
                vertebrae: count("Spine Segments", DEFAULT_VERTEBRAE, 1.0, 16.0),
                pelvis: toggle("Pelvis", true),
                clavicles: toggle("Clavicles", true),
                arms: toggle("Arms", true),
                legs: toggle("Legs", true),
                hands: toggle("Hands", true),
                feet: toggle("Feet", true),
                joints: toggle("Joints", true),
                thickness: bounded(data, "Thickness", DEFAULT_THICKNESS, 0.01, 0.12),
                detail,
            },
            look: Look {
                players: toggle("Players", true),
                 
                 
                self_player: false,
                 
                 
                armor: false,
                color,
                 
                 
                 
                strength: bounded(data, "Glow", DEFAULT_GLOW, 0.2, 2.0),
                pulse: false,
                through: toggle("ThroughWalls", true),
                range: bounded(data, "Range", DEFAULT_RANGE, 16.0, 128.0),
            },
        }
    }

     
    pub(crate) fn joints(
        &self,
        tick: f32,
        cam: [f64; 3],
        camera_detached: bool,
    ) -> anyhow::Result<Vec<f32>> {
        skeleton_positions(&self.look, tick, cam, camera_detached)
    }

     
    pub(crate) fn look(&self) -> &Look {
        &self.look
    }
}

 
 
pub(crate) fn bone_geometry(joints: &[f32], cfg: &Config) -> Vec<f32> {
    skeleton_rig::build(joints, &cfg.anatomy)
}

 

 
 
#[derive(Debug)]
pub struct SkeletonEspModule {
    pub module: ModuleData,
}

impl SkeletonEspModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::SkeletonEsp,
                description:
                    "A skull, spine, ribcage and pelvis through the limbs of nearby players, drawn from their real animated model like Chams"
                        .to_string(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Toggle {
                        name: "Players".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "ThroughWalls".to_string(),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: "Range".to_string(),
                        value: DEFAULT_RANGE,
                        min: 16.0,
                        max: 128.0,
                    },
                    ModuleSetting::Color {
                        name: "Color".to_string(),
                        value: DEFAULT_COLOR,
                    },
                    ModuleSetting::Slider {
                        name: "Thickness".to_string(),
                        value: DEFAULT_THICKNESS,
                        min: 0.01,
                        max: 0.12,
                    },
                    ModuleSetting::Slider {
                        name: "Glow".to_string(),
                        value: DEFAULT_GLOW,
                        min: 0.2,
                        max: 2.0,
                    },
                    ModuleSetting::Choice {
                        name: "Detail".to_string(),
                        value: Detail::default().choice(),
                        options: Detail::OPTIONS.iter().map(|o| o.to_string()).collect(),
                    },
                    ModuleSetting::Toggle {
                        name: "Skull".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Ribcage".to_string(),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: "Rib Pairs".to_string(),
                        value: DEFAULT_RIB_PAIRS,
                        min: 2.0,
                        max: 10.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Spine".to_string(),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: "Spine Segments".to_string(),
                        value: DEFAULT_VERTEBRAE,
                        min: 1.0,
                        max: 16.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Pelvis".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Clavicles".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Arms".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Legs".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Hands".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Feet".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Joints".to_string(),
                        value: true,
                    },
                ],
            },
        }
    }
}

impl Default for SkeletonEspModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SkeletonEspModule {
    fn on_start(&self) -> anyhow::Result<()> {
        crate::graphic::smoke::start_source(ModuleId::SkeletonEsp)
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        crate::graphic::smoke::stop_source(ModuleId::SkeletonEsp)
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        crate::graphic::smoke::stop_source(ModuleId::SkeletonEsp)
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
    use crate::module::render::skeleton_rig::standing_pose;

    fn joints() -> Vec<f32> {
        standing_pose().iter().flatten().copied().collect()
    }

    fn enabled() -> ModuleData {
        let mut data = SkeletonEspModule::new().module;
        data.enabled = true;
        data
    }

    fn geometry(data: &ModuleData) -> Vec<f32> {
        bone_geometry(&joints(), &Config::from_data(data))
    }

     
     
     
    #[test]
    fn every_anatomy_setting_reaches_the_geometry() {
        let full = geometry(&enabled());
        for name in [
            "Skull",
            "Ribcage",
            "Spine",
            "Pelvis",
            "Clavicles",
            "Arms",
            "Legs",
            "Hands",
            "Feet",
            "Joints",
        ] {
            let mut data = enabled();
            data.get_setting_mut(name).unwrap().set_toggle_value(false);
            let without = geometry(&data);
            assert!(
                without.len() < full.len(),
                "{name} is on the panel but does not change the rig"
            );
             
            assert!(!without.is_empty(), "turning {name} off drew nothing");
        }
    }

     
    #[test]
    fn the_count_settings_change_the_rig() {
        let mut few = enabled();
        few.get_setting_mut("Rib Pairs")
            .unwrap()
            .set_slider_value(3.0);
        let mut many = enabled();
        many.get_setting_mut("Rib Pairs")
            .unwrap()
            .set_slider_value(9.0);
        assert!(geometry(&many).len() > geometry(&few).len());

        let mut short = enabled();
        short
            .get_setting_mut("Spine Segments")
            .unwrap()
            .set_slider_value(3.0);
        let mut long = enabled();
        long.get_setting_mut("Spine Segments")
            .unwrap()
            .set_slider_value(12.0);
        assert!(geometry(&long).len() > geometry(&short).len());
    }

     
    #[test]
    fn detail_is_a_quality_knob() {
        let mut low = enabled();
        if let Some(ModuleSetting::Choice { value, .. }) = low.get_setting_mut("Detail") {
            *value = 0;
        }
        let mut high = enabled();
        if let Some(ModuleSetting::Choice { value, .. }) = high.get_setting_mut("Detail") {
            *value = 2;
        }
        let low = geometry(&low);
        let high = geometry(&high);
        assert!(high.len() > low.len());
    }

    #[test]
    fn the_module_traces_players_and_reads_the_shared_selection() {
        let module = SkeletonEspModule::new();
        assert_eq!(module.module.id, ModuleId::SkeletonEsp);
        assert_eq!(module.module.category, ModuleCategory::Render);
        let cfg = Config::from_data(&module.module);
        assert!(!cfg.enabled);
        assert!(cfg.look.players);
        assert!(!cfg.look.self_player);
        assert!(cfg.look.through);
        assert_eq!(cfg.look.range, DEFAULT_RANGE);
        assert!((cfg.anatomy.thickness - DEFAULT_THICKNESS).abs() < 1e-6);
         
         
        assert!(cfg.anatomy.skull);
        assert!(cfg.anatomy.ribcage);
        assert!(cfg.anatomy.spine);
        assert!(cfg.anatomy.pelvis);
        assert!(cfg.anatomy.clavicles);
        assert!(cfg.anatomy.arms);
        assert!(cfg.anatomy.legs);
        assert!(cfg.anatomy.hands);
        assert!(cfg.anatomy.feet);
        assert!(cfg.anatomy.joints);
        assert_eq!(cfg.anatomy.rib_pairs, DEFAULT_RIB_PAIRS as usize);
        assert_eq!(cfg.anatomy.vertebrae, DEFAULT_VERTEBRAE as usize);
        assert_eq!(cfg.anatomy.detail, Detail::MEDIUM);
         
        assert!(module.module.get_setting("Mobs").is_none());
    }

    #[test]
    fn corrupt_settings_stay_finite_and_clamped() {
        let mut data = enabled();
        data.get_setting_mut("Thickness")
            .unwrap()
            .set_slider_value(f32::NAN);
        data.get_setting_mut("Range")
            .unwrap()
            .set_slider_value(9_000.0);
        data.get_setting_mut("Glow").unwrap().set_slider_value(0.0);
        data.get_setting_mut("Rib Pairs")
            .unwrap()
            .set_slider_value(400.0);
        data.get_setting_mut("Spine Segments")
            .unwrap()
            .set_slider_value(-3.0);
        if let Some(ModuleSetting::Choice { value, .. }) = data.get_setting_mut("Detail") {
            *value = 900;
        }
        if let Some(ModuleSetting::Color { value, .. }) = data.get_setting_mut("Color") {
            *value = [f32::NAN, 2.0, -1.0, 1.0];
        }
        let cfg = Config::from_data(&data);
        assert!(cfg.anatomy.thickness.is_finite());
        assert!((0.01..=0.12).contains(&cfg.anatomy.thickness));
        assert_eq!(cfg.look.range, 128.0);
         
        assert!(cfg.look.strength >= 0.2);
        assert!(cfg.anatomy.rib_pairs <= 10);
        assert!(cfg.anatomy.vertebrae >= 1);
         
        assert_eq!(cfg.anatomy.detail, Detail::MEDIUM);
        assert_eq!(cfg.color[1], 1.0);
        assert_eq!(cfg.color[2], 0.0);
        assert!(cfg.color.iter().all(|c| c.is_finite()));
    }

    #[test]
    fn a_disabled_or_player_less_pass_collects_nothing() {
        let mut data = SkeletonEspModule::new().module;
        assert!(!Config::from_data(&data).enabled);
        data.enabled = true;
        data.get_setting_mut("Players")
            .unwrap()
            .set_toggle_value(false);
        let cfg = Config::from_data(&data);
        assert!(cfg.enabled);
        assert!(!cfg.look.players);
    }

     
     
    #[test]
    fn the_rig_keeps_the_pass_layout_at_every_detail_level() {
        for choice in 0..3 {
            let mut data = enabled();
            if let Some(ModuleSetting::Choice { value, .. }) = data.get_setting_mut("Detail") {
                *value = choice;
            }
            let mesh = geometry(&data);
            assert_eq!(
                mesh.len() % skeleton_rig::VERTEX_FLOATS,
                0,
                "detail {choice} broke the stride"
            );
            for vertex in mesh.chunks_exact(skeleton_rig::VERTEX_FLOATS) {
                assert!(vertex.iter().all(|v| v.is_finite()));
                assert!(vertex[3].abs() <= 1.0001 && vertex[4].abs() <= 1.0001);
            }
        }
    }

     
    #[test]
    fn each_player_gets_its_own_rig() {
        let one = geometry(&enabled());
        let mut two = joints();
        two.extend_from_slice(&joints());
        let both = bone_geometry(&two, &Config::from_data(&enabled()));
        assert!((both.len() as f32 / one.len() as f32 - 2.0).abs() < 0.05);
    }
}
