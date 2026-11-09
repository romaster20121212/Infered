 
 
 
 
 
 
 
 
 
 
 
 

use std::sync::Mutex;

use crate::camera_bridge::{set_swing_echo, set_swing_echo_look, SwingEchoLook};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
 
const MAX_COPIES: f32 = 32.0;

 
 
 
const SLIDERS: [(&str, f32, f32, f32); 7] = [
    ("Delay (s)", 0.02, 2.0, 0.2),
    ("Copies", 1.0, MAX_COPIES, 12.0),
    ("Spacing", 0.01, 0.5, 0.06),
    ("Hold (s)", 0.0, 2.0, 0.0),
    ("Fade (s)", 0.05, 3.0, 0.45),
    ("Start Opacity", 0.0, 1.0, 1.0),
    ("Smoothness", 0.0, 1.0, 0.0),
];

 
const FROZEN_SETTING: &str = "Frozen Copies";

#[derive(Debug)]
pub struct SwingEchoModule {
    pub module: ModuleData,
     
    applied: Mutex<Option<SwingEchoLook>>,
}

impl SwingEchoModule {
    pub fn new() -> Self {
        let mut settings: Vec<ModuleSetting> = vec![ModuleSetting::Toggle {
            name: FROZEN_SETTING.into(),
            value: false,
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
                id: ModuleId::SwingEcho,
                description: "Draws the held item again slightly behind the real sword, so \
                              each swing leaves a trailing copy. Frozen Copies keeps every \
                              copy where it was taken instead, then fades it out. Purely \
                              local."
                    .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings,
            },
            applied: Mutex::new(None),
        }
    }

     
     
    fn look(&self) -> SwingEchoLook {
        let value = |name: &str| {
            let (_, min, max, default) = *SLIDERS
                .iter()
                .find(|(slider, ..)| *slider == name)
                .expect("every slider read here is declared in SLIDERS");
            self.module
                .get_setting(name)
                .and_then(|setting| setting.get_slider_value())
                .filter(|value| value.is_finite())
                .map(|value| value.clamp(min, max))
                .unwrap_or(default)
        };
        SwingEchoLook {
            frozen: self
                .module
                .get_setting(FROZEN_SETTING)
                .and_then(|setting| setting.get_toggle_value())
                .unwrap_or(false),
            delay_seconds: value("Delay (s)"),
            copies: value("Copies").round() as i32,
            spacing: value("Spacing"),
            hold_seconds: value("Hold (s)"),
            fade_seconds: value("Fade (s)"),
            start_alpha: value("Start Opacity"),
            smoothness: value("Smoothness"),
        }
    }

     
     
     
    fn push_look(&self, look: SwingEchoLook) {
        match set_swing_echo_look(look) {
            Ok(()) => {
                *self.applied.lock().unwrap_or_else(|p| p.into_inner()) = Some(look);
            }
            Err(error) => log::debug!("Swing Echo: {error}"),
        }
    }

     
     
    fn push(&self, enabled: bool) {
        if let Err(error) = set_swing_echo(enabled) {
            log::debug!("Swing Echo: {error}");
        }
    }
}

impl Module for SwingEchoModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if !crate::jvmti::transform_installed(crate::jvmti::SWING_ECHO_TRANSFORM_PARAMS.log_label) {
            log::warn!(
                "Swing Echo: the item pass hook is not installed on this build, so no copies \
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

    fn set(module: &mut SwingEchoModule, name: &str, to: f32) {
        if let Some(ModuleSetting::Slider { value, .. }) = module.module.get_setting_mut(name) {
            *value = to;
        }
    }

    fn toggle(module: &mut SwingEchoModule, name: &str, to: bool) {
        if let Some(ModuleSetting::Toggle { value, .. }) = module.module.get_setting_mut(name) {
            *value = to;
        }
    }

    #[test]
    fn defaults_match_the_bridge() {
        let module = SwingEchoModule::new();
        assert_eq!(
            module.look(),
            SwingEchoLook {
                frozen: false,
                delay_seconds: 0.2,
                copies: 12,
                spacing: 0.06,
                hold_seconds: 0.0,
                fade_seconds: 0.45,
                start_alpha: 1.0,
                smoothness: 0.0,
            }
        );
    }

    #[test]
    fn frozen_copies_toggle_reaches_the_bridge() {
        let mut module = SwingEchoModule::new();
        assert!(!module.look().frozen);
        toggle(&mut module, "Frozen Copies", true);
        assert!(module.look().frozen);
    }

    #[test]
    fn delay_is_clamped() {
        let mut module = SwingEchoModule::new();
        set(&mut module, "Delay (s)", 50.0);
        assert_eq!(module.look().delay_seconds, 2.0);
        set(&mut module, "Delay (s)", 0.0);
        assert_eq!(module.look().delay_seconds, 0.02);
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let mut module = SwingEchoModule::new();
        set(&mut module, "Copies", 500.0);
        set(&mut module, "Fade (s)", -1.0);
        set(&mut module, "Start Opacity", 3.0);
        set(&mut module, "Smoothness", -0.5);
        let look = module.look();
        assert_eq!(look.copies, 32);
        assert_eq!(look.fade_seconds, 0.05);
        assert_eq!(look.start_alpha, 1.0);
        assert_eq!(look.smoothness, 0.0);
    }

    #[test]
    fn copies_round_to_a_whole_number() {
        let mut module = SwingEchoModule::new();
        set(&mut module, "Copies", 7.6);
        assert_eq!(module.look().copies, 8);
    }

    #[test]
    fn non_finite_values_fall_back_to_the_default() {
        let mut module = SwingEchoModule::new();
        set(&mut module, "Spacing", f32::NAN);
        set(&mut module, "Hold (s)", f32::INFINITY);
        set(&mut module, "Delay (s)", f32::NAN);
        let look = module.look();
        assert_eq!(look.spacing, 0.06);
        assert_eq!(look.hold_seconds, 0.0);
        assert_eq!(look.delay_seconds, 0.2);
    }
}
