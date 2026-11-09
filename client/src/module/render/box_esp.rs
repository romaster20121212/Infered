 
 
 
 
 
 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
pub const TARGETS: [&str; 3] = ["Players", "Mobs", "Both"];

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxTargets {
    Players,
    Mobs,
    Both,
}

impl BoxTargets {
     
     
    pub fn from_choice(index: usize) -> BoxTargets {
        match index {
            0 => BoxTargets::Players,
            1 => BoxTargets::Mobs,
            _ => BoxTargets::Both,
        }
    }

    pub fn players(self) -> bool {
        self != BoxTargets::Mobs
    }

    pub fn mobs(self) -> bool {
        self != BoxTargets::Players
    }
}

 
#[derive(Debug)]
pub struct BoxEspModule {
    pub module: ModuleData,
}

impl BoxEspModule {
    pub fn new() -> Self {
        let slider = |name: &str, value: f32, min: f32, max: f32| ModuleSetting::Slider {
            name: name.into(),
            value,
            min,
            max,
        };
        Self {
            module: ModuleData {
                id: ModuleId::BoxEsp,
                description: "Draws each target's hitbox as a true 3D box — twelve projected \
                              edges with an optional translucent fill"
                    .to_string(),
                category: ModuleCategory::Render,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Color {
                        name: "Color".to_string(),
                        value: [1.0, 0.0, 0.0, 1.0],
                    },
                    ModuleSetting::Choice {
                        name: "Targets".to_string(),
                        value: 2,
                        options: TARGETS.iter().map(|target| target.to_string()).collect(),
                    },
                    slider("Range", 64.0, 16.0, 256.0),
                    slider("Thickness", 1.5, 0.5, 5.0),
                    ModuleSetting::Toggle {
                        name: "Fill".to_string(),
                        value: true,
                    },
                    slider("Fill Opacity", 0.16, 0.0, 0.6),
                ],
            },
        }
    }
}

impl Module for BoxEspModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
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

    #[test]
    fn the_targets_choice_maps_to_its_kinds() {
        for (index, players, mobs) in [
            (0, true, false),
            (1, false, true),
            (2, true, true),
            (99, true, true),
        ] {
            let targets = BoxTargets::from_choice(index);
            assert_eq!(targets.players(), players, "index {index}");
            assert_eq!(targets.mobs(), mobs, "index {index}");
        }
    }

     
     
    #[test]
    fn the_module_offers_the_targets_in_renderer_order() {
        let module = BoxEspModule::new();
        let Some(ModuleSetting::Choice { options, value, .. }) =
            module.module.get_setting("Targets")
        else {
            panic!("Box ESP must offer a Targets choice");
        };
        assert_eq!(
            options.iter().map(String::as_str).collect::<Vec<_>>(),
            TARGETS
        );
        assert_eq!(BoxTargets::from_choice(*value), BoxTargets::Both);
    }
}
