 
 
 
 
 
 
 
 
 
 
 

use std::sync::Mutex;

use crate::camera_bridge::{set_hand_outline, set_hand_outline_look, HandOutlineLook};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
 
const MAX_SAMPLES: f32 = 16.0;

 
const DEFAULT_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

 
 
 
const SLIDERS: [(&str, f32, f32, f32); 2] = [
    ("Thickness", 0.0, 0.02, 0.0015),
    ("Samples", 4.0, MAX_SAMPLES, 8.0),
];

#[derive(Debug)]
pub struct HandOutlineModule {
    pub module: ModuleData,
     
    applied: Mutex<Option<HandOutlineLook>>,
}

impl HandOutlineModule {
    pub fn new() -> Self {
        let mut settings = vec![ModuleSetting::Color {
            name: "Color".into(),
            value: DEFAULT_COLOR,
        }];
        settings.extend(
            SLIDERS
                .iter()
                .map(|&(name, min, max, default)| ModuleSetting::Slider {
                    name: name.into(),
                    value: default,
                    min,
                    max,
                }),
        );
        Self {
            module: ModuleData {
                id: ModuleId::HandOutline,
                description: "Draws a rim around your first-person arm and held item. The \
                              colour and width are adjustable. Purely local."
                    .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings,
            },
            applied: Mutex::new(None),
        }
    }

     
     
     
    fn look(&self) -> HandOutlineLook {
        let color = match self.module.get_setting("Color") {
            Some(ModuleSetting::Color { value, .. }) => std::array::from_fn(|i| {
                let channel = value[i];
                if channel.is_finite() {
                    channel.clamp(0.0, 1.0)
                } else {
                    DEFAULT_COLOR[i]
                }
            }),
            _ => DEFAULT_COLOR,
        };
        let slider = |index: usize| {
            let (name, min, max, default) = SLIDERS[index];
            self.module
                .get_setting(name)
                .and_then(|setting| setting.get_slider_value())
                .filter(|value| value.is_finite())
                .map(|value| value.clamp(min, max))
                .unwrap_or(default)
        };
        HandOutlineLook {
            color,
            thickness: slider(0),
            samples: slider(1).round() as i32,
        }
    }

     
     
    fn push_look(&self, look: HandOutlineLook) {
        match set_hand_outline_look(look) {
            Ok(()) => {
                *self.applied.lock().unwrap_or_else(|p| p.into_inner()) = Some(look);
            }
            Err(error) => log::debug!("Hand Outline: {error}"),
        }
    }

     
     
    fn push(&self, enabled: bool) {
        if let Err(error) = set_hand_outline(enabled) {
            log::debug!("Hand Outline: {error}");
        }
    }
}

impl Module for HandOutlineModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if !crate::jvmti::transform_installed(crate::jvmti::HAND_OUTLINE_TRANSFORM_PARAMS.log_label)
        {
            log::warn!(
                "Hand Outline: the item pass hook is not installed on this build, so no outline \
                 can be drawn"
            );
        }
        self.push_look(self.look());
        self.push(true);
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.push(false);
        *self.applied.lock().unwrap_or_else(|p| p.into_inner()) = None;
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        self.push(false);
        *self.applied.lock().unwrap_or_else(|p| p.into_inner()) = None;
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
        let look = self.look();
        let applied = *self.applied.lock().unwrap_or_else(|p| p.into_inner());
        if applied != Some(look) {
            self.push_look(look);
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

    fn set_slider(module: &mut HandOutlineModule, name: &str, to: f32) {
        if let Some(ModuleSetting::Slider { value, .. }) = module.module.get_setting_mut(name) {
            *value = to;
        }
    }

    #[test]
    fn defaults_are_a_thin_black_rim() {
        let module = HandOutlineModule::new();
        assert_eq!(
            module.look(),
            HandOutlineLook {
                color: [0.0, 0.0, 0.0, 1.0],
                thickness: 0.0015,
                samples: 8,
            }
        );
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let mut module = HandOutlineModule::new();
        set_slider(&mut module, "Thickness", 5.0);
        set_slider(&mut module, "Samples", 1000.0);
        let look = module.look();
        assert_eq!(look.thickness, 0.02);
        assert_eq!(look.samples, 16);
    }

    #[test]
    fn non_finite_thickness_falls_back_to_the_default() {
        let mut module = HandOutlineModule::new();
        set_slider(&mut module, "Thickness", f32::NAN);
        assert_eq!(module.look().thickness, 0.0015);
    }

    #[test]
    fn color_is_clamped_per_channel() {
        let mut module = HandOutlineModule::new();
        if let Some(ModuleSetting::Color { value, .. }) = module.module.get_setting_mut("Color") {
            *value = [2.0, -1.0, f32::NAN, 0.5];
        }
        assert_eq!(module.look().color, [1.0, 0.0, 0.0, 0.5]);
    }
}
