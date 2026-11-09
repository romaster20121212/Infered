 
 
 
 
 
 
 

use crate::camera_bridge;
use crate::graphic::haze::{self, HazeStyle};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

#[derive(Debug)]
pub struct HazeModule {
    pub module: ModuleData,
}

impl HazeModule {
    pub fn new() -> Self {
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
                id: ModuleId::Haze,
                description: "Adds light, depth-aware fog to the world and darkens the scene a \
                              little. Small glowing motes float around you, and the fog glows \
                              where their light reaches it."
                    .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    slider("Fog Density", 0.012, 0.0, 0.06),
                    slider("Darken", 0.12, 0.0, 0.5),
                    slider("Glow", 1.0, 0.0, 3.0),
                    color("Fog Color", [0.78, 0.82, 0.9]),
                    color("Mote Color", [1.0, 0.85, 0.55]),
                ],
            },
        }
    }

    fn style(&self) -> HazeStyle {
        let slider = |name: &str, fallback: f32| {
            self.module
                .get_setting(name)
                .and_then(ModuleSetting::get_slider_value)
                .filter(|value| value.is_finite())
                .unwrap_or(fallback)
        };
        let color = |name: &str, fallback: [f32; 3]| match self.module.get_setting(name) {
            Some(ModuleSetting::Color { value, .. }) => [value[0], value[1], value[2]],
            _ => fallback,
        };
        HazeStyle {
            density: slider("Fog Density", 0.012).clamp(0.0, 0.06),
            darken: slider("Darken", 0.12).clamp(0.0, 0.5),
            glow: slider("Glow", 1.0).clamp(0.0, 3.0),
            fog_color: color("Fog Color", [0.78, 0.82, 0.9]),
            mote_color: color("Mote Color", [1.0, 0.85, 0.55]),
        }
    }

     
    fn apply(&self) {
        haze::set_style(self.style());
        if let Err(error) = camera_bridge::set_haze_active(true) {
            log::debug!("Haze: {error}");
        }
    }

     
    fn release(&self) {
        haze::clear_style();
        if let Err(error) = camera_bridge::set_haze_active(false) {
            log::debug!("Haze: {error}");
        }
    }
}

impl Module for HazeModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.apply();
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
        if self.module.enabled {
            self.apply();
        }
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
    fn defaults_are_registered_as_haze_visuals() {
        let module = HazeModule::new();
        assert_eq!(module.module.id, ModuleId::Haze);
        assert_eq!(module.module.category, ModuleCategory::Visuals);
        assert!(!module.module.enabled);
    }

    #[test]
    fn style_clamps_out_of_range_sliders() {
        let mut module = HazeModule::new();
        for setting in &mut module.module.settings {
            if let ModuleSetting::Slider { name, value, .. } = setting {
                if name == "Darken" {
                    *value = 9.0;
                }
            }
        }
        assert_eq!(module.style().darken, 0.5);
    }
}
