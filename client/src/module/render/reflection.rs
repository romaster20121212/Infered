 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::camera_bridge;
use crate::graphic::reflection;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

#[derive(Debug)]
pub struct ReflectionModule {
    pub module: ModuleData,
}

impl ReflectionModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Reflection,
                description: "Turns blocks into mirrors: their exposed faces show the frame \
                              behind them in its own colours — no tint of the surface's own — \
                              with a slow highlight across it. Every block mirrors on its own \
                              shape — a stair on its steps, a slab on its half, a fence on its \
                              posts — never on a cube around it. All Blocks is on by default, so \
                              the whole world becomes the surface; turn it off to use the \
                              Blocks list. Covers the full square your render distance draws — \
                              at least 32 by 32 chunks, corners included, not a circle. \
                              Client-side only."
                    .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Toggle {
                        name: "All Blocks".into(),
                        value: true,
                    },
                    ModuleSetting::BlockList {
                        name: "Blocks".into(),
                        value: vec![
                            "minecraft:ice".into(),
                            "minecraft:packed_ice".into(),
                            "minecraft:blue_ice".into(),
                            "minecraft:frosted_ice".into(),
                            "minecraft:glass".into(),
                            "minecraft:tinted_glass".into(),
                        ],
                    },
                    ModuleSetting::Toggle {
                        name: "Render Distance".into(),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: "Radius".into(),
                        value: 256.0,
                        min: 8.0,
                        max: 512.0,
                    },
                    ModuleSetting::Slider {
                        name: "Strength".into(),
                        value: 1.0,
                        min: 0.0,
                        max: 1.0,
                    },
                    ModuleSetting::Slider {
                        name: "Blur".into(),
                        value: 0.35,
                        min: 0.0,
                        max: 1.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Shimmer".into(),
                        value: true,
                    },
                ],
            },
        }
    }
}

impl Module for ReflectionModule {
    fn on_start(&self) -> anyhow::Result<()> {
         
         
         
        anyhow::ensure!(
            crate::jvmti::transform_installed("SmokeWorldRender"),
            "the world-render tail is not installed on this Minecraft build"
        );
        reflection::reset();
        camera_bridge::set_reflection_active(true)
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        reflection::reset();
        camera_bridge::set_reflection_active(false)
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
         
        reflection::reset();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        crate::camera_bridge::set_reflection_active(self.module.enabled)
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
    fn the_module_is_registered_under_its_own_id() {
        let module = ReflectionModule::new();
        assert_eq!(module.module.id, ModuleId::Reflection);
        assert_eq!(module.module.category, ModuleCategory::Visuals);
        assert!(!module.module.enabled);
    }

    #[test]
    fn every_block_is_the_default_surface() {
        let module = ReflectionModule::new();
        assert_eq!(
            module
                .module
                .get_setting("All Blocks")
                .and_then(ModuleSetting::get_toggle_value),
            Some(true),
            "reflections must cover everything by default"
        );
    }

    #[test]
    fn it_ships_with_a_glassy_block_list_for_when_all_blocks_is_off() {
        let module = ReflectionModule::new();
        let blocks = module
            .module
            .get_setting("Blocks")
            .and_then(ModuleSetting::get_block_list_value)
            .expect("Blocks setting");
        for id in ["minecraft:ice", "minecraft:glass", "minecraft:blue_ice"] {
            assert!(
                blocks.iter().any(|entry| entry.as_str() == id),
                "{id} missing"
            );
        }
    }

    #[test]
    fn it_ships_reaching_the_players_render_distance() {
        let module = ReflectionModule::new();
        assert_eq!(
            module
                .module
                .get_setting("Render Distance")
                .and_then(ModuleSetting::get_toggle_value),
            Some(true),
            "the default range must follow the render distance"
        );
        let radius = module.module.get_setting("Radius").expect("Radius setting");
        match radius {
            ModuleSetting::Slider {
                value, min, max, ..
            } => {
                assert_eq!(
                    *value, 256.0,
                    "the default range is the 32 by 32 chunk square"
                );
                assert_eq!(*min, 8.0);
                assert_eq!(
                    *max, 512.0,
                    "the manual radius must reach the 32-chunk vanilla maximum"
                );
            }
            other => panic!("Radius is not a slider: {other:?}"),
        }
    }

    #[test]
    fn the_settings_are_the_seven_that_shape_the_look() {
        let module = ReflectionModule::new();
        for name in [
            "All Blocks",
            "Blocks",
            "Render Distance",
            "Radius",
            "Strength",
            "Blur",
            "Shimmer",
        ] {
            assert!(module.module.get_setting(name).is_some(), "{name} missing");
        }
        assert_eq!(module.module.settings.len(), 7);
    }

    #[test]
    fn there_is_no_tint_to_colour_the_reflection() {
        let module = ReflectionModule::new();
        assert!(
            module.module.get_setting("Tint").is_none(),
            "the mirror must show the world's own colours"
        );
    }
}
