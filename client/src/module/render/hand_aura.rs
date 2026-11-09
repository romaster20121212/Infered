 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use std::sync::Mutex;

use crate::camera_bridge::{set_hand_aura, set_hand_aura_look, HandAuraLook};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
 
const MAX_WIDTH: f32 = 32.0;

 
 
 
const DEFAULT_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 0.75];

 
 
 
const SLIDERS: [(&str, f32, f32, f32); 2] =
    [("Width", 0.0, MAX_WIDTH, 6.0), ("Density", 0.0, 1.0, 1.0)];

 
const TOGGLES: [(&str, bool); 3] = [
    ("Main Hand", true),
    ("Off Hand", true),
    ("Echo Bleed", true),
];

#[derive(Debug)]
pub struct HandAuraModule {
    pub module: ModuleData,
     
    applied: Mutex<Option<HandAuraLook>>,
}

impl HandAuraModule {
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
        settings.extend(
            TOGGLES
                .iter()
                .map(|&(name, default)| ModuleSetting::Toggle {
                    name: name.into(),
                    value: default,
                }),
        );
        Self {
            module: ModuleData {
                id: ModuleId::HandAura,
                description: "Draws a glowing 2D outline around your first-person arm \
                              and held item, like the Glowing effect. The width is in \
                              screen pixels, and echo bleed lets the glow follow a beat \
                              behind the hand without leaving a mark on the screen. \
                              Purely local."
                    .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings,
            },
            applied: Mutex::new(None),
        }
    }

     
     
    fn slider(&self, index: usize) -> f32 {
        let (name, min, max, default) = SLIDERS[index];
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(min, max))
            .unwrap_or(default)
    }

     
    fn toggle(&self, name: &str, default: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(default)
    }

     
     
     
    fn look(&self) -> HandAuraLook {
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
         
         
        let color = [color[0], color[1], color[2], color[3] * self.slider(1)];
        HandAuraLook {
            color,
            width: self.slider(0),
            main_hand: self.toggle("Main Hand", true),
            off_hand: self.toggle("Off Hand", true),
            echo_bleed: self.toggle("Echo Bleed", true),
        }
    }

     
     
    fn push_look(&self, look: HandAuraLook) {
        match set_hand_aura_look(look) {
            Ok(()) => {
                *self.applied.lock().unwrap_or_else(|p| p.into_inner()) = Some(look);
            }
            Err(error) => log::debug!("Hand Aura: {error}"),
        }
    }

     
     
    fn push(&self, enabled: bool) {
        if let Err(error) = set_hand_aura(enabled) {
            log::debug!("Hand Aura: {error}");
        }
    }
}

impl Module for HandAuraModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if !crate::jvmti::transform_installed(
            crate::jvmti::HAND_AURA_AFTER_TRANSFORM_PARAMS.log_label,
        ) {
            log::warn!(
                "Hand Aura: the after-hand hook is not installed on this build, so no aura can be \
                 drawn"
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

    fn set_slider(module: &mut HandAuraModule, name: &str, to: f32) {
        if let Some(ModuleSetting::Slider { value, .. }) = module.module.get_setting_mut(name) {
            *value = to;
        }
    }

    fn set_toggle(module: &mut HandAuraModule, name: &str, to: bool) {
        if let Some(ModuleSetting::Toggle { value, .. }) = module.module.get_setting_mut(name) {
            *value = to;
        }
    }

    #[test]
    fn defaults_are_a_bright_six_pixel_red_glow_with_echo_bleed() {
        let module = HandAuraModule::new();
        assert_eq!(
            module.look(),
            HandAuraLook {
                color: [1.0, 0.0, 0.0, 0.75],
                width: 6.0,
                main_hand: true,
                off_hand: true,
                echo_bleed: true,
            }
        );
    }

     
     
    #[test]
    fn the_default_glow_colour_matches_chams() {
        let chams = crate::module::render::chams::ChamsModule::new();
        let chams_colour = match chams.module.get_setting("Color") {
            Some(ModuleSetting::Color { value, .. }) => *value,
            _ => panic!("chams has a colour"),
        };
        let aura = HandAuraModule::new().look().color;
        assert_eq!(aura[..3], chams_colour[..3]);
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let mut module = HandAuraModule::new();
        set_slider(&mut module, "Width", 500.0);
        assert_eq!(module.look().width, MAX_WIDTH);
    }

    #[test]
    fn non_finite_slider_falls_back_to_the_default() {
        let mut module = HandAuraModule::new();
        set_slider(&mut module, "Width", f32::NAN);
        assert_eq!(module.look().width, 6.0);
    }

    #[test]
    fn density_scales_the_glow_alpha() {
        let mut module = HandAuraModule::new();
        set_slider(&mut module, "Density", 0.5);
        let look = module.look();
        assert_eq!(look.color[..3], [1.0, 0.0, 0.0]);
        assert!((look.color[3] - 0.375).abs() < 1e-6);
    }

    #[test]
    fn density_is_clamped_to_zero_and_one() {
        let mut module = HandAuraModule::new();
        set_slider(&mut module, "Density", 4.0);
        assert_eq!(module.look().color[3], 0.75);
        set_slider(&mut module, "Density", -2.0);
        assert_eq!(module.look().color[3], 0.0);
    }

    #[test]
    fn echo_bleed_is_a_toggle_that_reaches_the_look() {
        let mut module = HandAuraModule::new();
        set_toggle(&mut module, "Echo Bleed", false);
        assert!(!module.look().echo_bleed);
    }

    #[test]
    fn the_pulse_setting_is_gone() {
        let module = HandAuraModule::new();
        assert!(module.module.get_setting("Pulse Speed").is_none());
    }

    #[test]
    fn color_is_clamped_per_channel() {
        let mut module = HandAuraModule::new();
        if let Some(ModuleSetting::Color { value, .. }) = module.module.get_setting_mut("Color") {
            *value = [2.0, -1.0, f32::NAN, 0.5];
        }
         
        assert_eq!(module.look().color, [1.0, 0.0, 0.0, 0.5]);
    }

    #[test]
    fn hand_toggles_reach_the_look() {
        let mut module = HandAuraModule::new();
        set_toggle(&mut module, "Main Hand", false);
        let look = module.look();
        assert!(!look.main_hand);
        assert!(look.off_hand);
    }
}
