 
 
 
 
 
 
 
 
 

use crate::graphic::crown::{self, CrownConfig, CrownSample, CrownStyle};
use crate::graphic::esp::V3;
use crate::mapping::entity::player::Player;
use crate::mapping::entity::EntityRef;
use crate::mapping::MappedObject;
use crate::module::render::trajectories::{block_at, classify_block, Cell};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::collections::HashMap;
use std::time::Instant;

 
const SIGHT_STEP: f64 = 0.25;

 
 
fn line_of_sight_clear(from: V3, to: V3, mut cell_at: impl FnMut(V3) -> Cell) -> bool {
    let steps = (to.sub(from).length() / SIGHT_STEP).ceil().max(1.0) as usize;
    for step in 1..steps {
        let point = from.lerp(to, step as f64 / steps as f64);
        if cell_at(point) == Cell::Solid {
            return false;
        }
    }
    true
}

 
const FALLBACK_WIDTH: f32 = 0.6;
const FALLBACK_HEIGHT: f32 = 1.8;

#[derive(Debug)]
pub struct CrownModule {
    pub module: ModuleData,
}

impl CrownModule {
    pub fn new() -> Self {
        let slider = |name: &str, value: f32, min: f32, max: f32| ModuleSetting::Slider {
            name: name.into(),
            value,
            min,
            max,
        };
        Self {
            module: ModuleData {
                id: ModuleId::Crown,
                description: "A custom fiery crown on every player's head: a spinning band with \
                              flame tongues, rising embers and glowing gems. Style, colours, \
                              flames, spin, hover and range are all adjustable. Client-side only."
                    .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Choice {
                        name: "Style".into(),
                        value: 0,
                        options: CrownStyle::OPTIONS
                            .iter()
                            .map(|name| name.to_string())
                            .collect(),
                    },
                    ModuleSetting::Color {
                        name: "Band Color".into(),
                        value: [0.72, 0.45, 0.10, 1.0],
                    },
                    ModuleSetting::Color {
                        name: "Flame Color".into(),
                        value: [1.0, 0.30, 0.05, 1.0],
                    },
                    ModuleSetting::Color {
                        name: "Core Color".into(),
                        value: [1.0, 0.90, 0.55, 1.0],
                    },
                    ModuleSetting::Color {
                        name: "Gem Color".into(),
                        value: [1.0, 0.12, 0.10, 1.0],
                    },
                    ModuleSetting::Toggle {
                        name: "Rainbow".into(),
                        value: false,
                    },
                    ModuleSetting::Toggle {
                        name: "Glow".into(),
                        value: true,
                    },
                    slider("Glow Strength", 1.0, 0.0, 2.0),
                    slider("Opacity", 0.95, 0.2, 1.0),
                    slider("Size", 1.0, 0.6, 2.0),
                    slider("Hover", 0.0, 0.0, 0.6),
                    slider("Points", 8.0, 3.0, 16.0),
                    slider("Spike Height", 0.22, 0.05, 0.5),
                    slider("Band Height", 0.10, 0.03, 0.2),
                    slider("Thickness", 1.0, 0.4, 2.5),
                    slider("Flame Length", 0.45, 0.1, 0.9),
                    slider("Flame Waviness", 0.55, 0.0, 1.0),
                    slider("Flame Speed", 1.6, 0.2, 4.0),
                    slider("Flicker", 0.6, 0.0, 1.0),
                    slider("Spin Speed", 0.35, 0.0, 2.0),
                    slider("Bob", 0.03, 0.0, 0.15),
                    ModuleSetting::Toggle {
                        name: "Gems".into(),
                        value: true,
                    },
                    slider("Embers", 18.0, 0.0, 40.0),
                    slider("Ember Speed", 1.0, 0.2, 3.0),
                    ModuleSetting::Toggle {
                        name: "Show Self".into(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Show Others".into(),
                        value: true,
                    },
                    slider("Range", 48.0, 8.0, 128.0),
                ],
            },
        }
    }
}

impl Module for CrownModule {
    fn on_start(&self) -> anyhow::Result<()> {
        crown::reset();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        crown::reset();
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
         
        crown::reset();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let cfg = CrownConfig::from_data(&self.module);
        if !cfg.enabled {
            crown::reset();
            return Ok(());
        }
        let mc = minecraft();
        let (Some(world), Some(local)) = (mc.world()?, mc.player()?) else {
            crown::reset();
            return Ok(());
        };
        let local_id = local.id()?;
        let local_feet: V3 = local.get_position()?.into();
         
         
        let eye: V3 = local.get_eye_position()?.into();
        let mut sight = HashMap::new();

        let mut samples = Vec::new();
        if cfg.show_self {
            samples.push(CrownSample {
                id: local_id,
                pos: local_feet,
                width: local.bb_width().unwrap_or(FALLBACK_WIDTH) as f64,
                height: local.bb_height().unwrap_or(FALLBACK_HEIGHT) as f64,
            });
        }
        if cfg.show_others {
            for entity in world.get_entities()? {
                if !entity.instance_of::<Player>() {
                    continue;
                }
                let Ok(id) = entity.id() else {
                    continue;
                };
                if id == local_id {
                    continue;
                }
                let Ok(position) = entity.get_position() else {
                    continue;
                };
                let position: V3 = position.into();
                let distance = position.sub(local_feet).length();
                if !distance.is_finite() || distance > cfg.range {
                    continue;
                }
                let head = V3 {
                    x: position.x,
                    y: position.y + entity.bb_height().unwrap_or(FALLBACK_HEIGHT) as f64,
                    z: position.z,
                };
                let visible = line_of_sight_clear(eye, head, |point| {
                    let pos = block_at(point);
                    *sight
                        .entry((pos.x(), pos.y(), pos.z()))
                        .or_insert_with(|| classify_block(&world, &pos))
                });
                if !visible {
                    continue;
                }
                samples.push(CrownSample {
                    id,
                    pos: position,
                    width: entity.bb_width().unwrap_or(FALLBACK_WIDTH) as f64,
                    height: entity.bb_height().unwrap_or(FALLBACK_HEIGHT) as f64,
                });
            }
        }
        crown::publish(samples, Instant::now());
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
    fn the_module_is_registered_under_its_own_id() {
        let module = CrownModule::new();
        assert_eq!(module.module.id, ModuleId::Crown);
        assert_eq!(module.module.category, ModuleCategory::Visuals);
        assert!(!module.module.enabled);
    }

    #[test]
    fn the_settings_cover_the_whole_look() {
        let module = CrownModule::new();
        for name in [
            "Style",
            "Band Color",
            "Flame Color",
            "Core Color",
            "Gem Color",
            "Rainbow",
            "Glow",
            "Glow Strength",
            "Opacity",
            "Size",
            "Hover",
            "Points",
            "Spike Height",
            "Band Height",
            "Thickness",
            "Flame Length",
            "Flame Waviness",
            "Flame Speed",
            "Flicker",
            "Spin Speed",
            "Bob",
            "Gems",
            "Embers",
            "Ember Speed",
            "Show Self",
            "Show Others",
            "Range",
        ] {
            assert!(module.module.get_setting(name).is_some(), "{name} missing");
        }
        assert_eq!(module.module.settings.len(), 27);
    }

    #[test]
    fn every_style_is_offered() {
        let module = CrownModule::new();
        let options = match module.module.get_setting("Style") {
            Some(ModuleSetting::Choice { options, .. }) => options.clone(),
            other => panic!("Style is not a choice: {other:?}"),
        };
        assert_eq!(options, CrownStyle::OPTIONS);
        assert_eq!(options.len(), 3);
    }

    #[test]
    fn it_ships_drawing_you_and_everyone_else() {
        let cfg = CrownConfig::from_data(&CrownModule::new().module);
        assert!(cfg.show_self, "your own crown shows in third person");
        assert!(cfg.show_others);
        assert_eq!(cfg.range, 48.0);
    }

     
    fn wall_at(wall: f64) -> impl FnMut(V3) -> Cell {
        move |point: V3| {
            if point.x >= wall && point.x < wall + 1.0 {
                Cell::Solid
            } else {
                Cell::Air
            }
        }
    }

    #[test]
    fn a_crown_in_the_open_is_seen() {
        let eye = V3 {
            x: 0.0,
            y: 1.6,
            z: 0.0,
        };
        let head = V3 {
            x: 5.0,
            y: 1.8,
            z: 0.0,
        };
        assert!(line_of_sight_clear(eye, head, wall_at(10.0)));
    }

    #[test]
    fn a_crown_behind_a_wall_is_hidden() {
        let eye = V3 {
            x: 0.0,
            y: 1.6,
            z: 0.0,
        };
        let head = V3 {
            x: 5.0,
            y: 1.8,
            z: 0.0,
        };
        assert!(!line_of_sight_clear(eye, head, wall_at(2.0)));
    }

    #[test]
    fn a_wall_beyond_the_crown_does_not_hide_it() {
        let eye = V3 {
            x: 0.0,
            y: 1.6,
            z: 0.0,
        };
        let head = V3 {
            x: 5.0,
            y: 1.8,
            z: 0.0,
        };
        assert!(line_of_sight_clear(eye, head, wall_at(8.0)));
    }
}
