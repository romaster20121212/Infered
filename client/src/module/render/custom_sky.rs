 
 
 
 
 
 
 
 
 
 
 

use std::sync::Mutex;

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

const MIN_CHANNEL: f32 = 0.0;
const MAX_CHANNEL: f32 = 255.0;

 
 
const DEFAULT_RED: f32 = 120.0;
const DEFAULT_GREEN: f32 = 170.0;
const DEFAULT_BLUE: f32 = 255.0;

#[derive(Debug)]
pub struct CustomSkyModule {
    pub module: ModuleData,
     
    applied: Mutex<Option<(u8, u8, u8)>>,
}

impl CustomSkyModule {
    pub fn new() -> Self {
        let slider = |name: &str, value: f32| ModuleSetting::Slider {
            name: name.into(),
            value,
            min: MIN_CHANNEL,
            max: MAX_CHANNEL,
        };
        Self {
            module: ModuleData {
                id: ModuleId::CustomSky,
                description: "Recolours the overworld sky: pick the red, green and blue \
                              channels. A picture-based sky lives in Render → Atmosphere \
                              (Sky → Custom Image); the sky goes back to vanilla when this \
                              module is off."
                    .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    slider("Red", DEFAULT_RED),
                    slider("Green", DEFAULT_GREEN),
                    slider("Blue", DEFAULT_BLUE),
                ],
            },
            applied: Mutex::new(None),
        }
    }

     
    fn colour(&self) -> (u8, u8, u8) {
        (
            self.channel("Red", DEFAULT_RED),
            self.channel("Green", DEFAULT_GREEN),
            self.channel("Blue", DEFAULT_BLUE),
        )
    }

    fn channel(&self, name: &str, default: f32) -> u8 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .unwrap_or(default)
            .clamp(MIN_CHANNEL, MAX_CHANNEL)
            .round() as u8
    }

     
     
     
    fn push(&self, colour: (u8, u8, u8), enabled: bool) {
        match crate::camera_bridge::set_sky_tint(enabled, colour.0, colour.1, colour.2) {
            Ok(()) => {
                let mut applied = self.applied.lock().unwrap_or_else(|p| p.into_inner());
                 
                 
                *applied = enabled.then_some(colour);
            }
            Err(error) => log::debug!("Custom Sky: {error}"),
        }
    }
}

impl Module for CustomSkyModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if !crate::jvmti::transform_installed(crate::jvmti::SKY_COLOR_TRANSFORM_PARAMS.log_label) {
            log::warn!(
                "Custom Sky: the sky colour hook is not installed on this build, so the sky \
                 cannot be recoloured"
            );
        }
        self.push(self.colour(), true);
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
        self.push(self.colour(), false);
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
         
        self.push(self.colour(), false);
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        if !self.module.enabled {
            return Ok(());
        }
        let colour = self.colour();
        {
            let applied = self.applied.lock().unwrap_or_else(|p| p.into_inner());
            if *applied == Some(colour) {
                return Ok(());
            }
        }
        self.push(colour, true);
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

    fn setting_names(module: &CustomSkyModule) -> Vec<String> {
        module
            .module
            .settings
            .iter()
            .map(|setting| setting.name().to_string())
            .collect()
    }

     
    #[test]
    fn the_module_owns_the_three_channels_and_nothing_else() {
        let module = CustomSkyModule::new();
        assert_eq!(setting_names(&module), vec!["Red", "Green", "Blue"]);
        assert_eq!(module.module.id, ModuleId::CustomSky);
        assert_eq!(module.module.category, ModuleCategory::Visuals);
        assert!(!module.module.enabled);
    }

     
    #[test]
    fn the_sliders_map_to_the_bridge_colour() {
        let mut module = CustomSkyModule::new();
        assert_eq!(module.colour(), (120, 170, 255));
        for (name, value) in [("Red", 300.0), ("Green", -8.0), ("Blue", f32::NAN)] {
            if let Some(ModuleSetting::Slider { value: slot, .. }) =
                module.module.get_setting_mut(name)
            {
                *slot = value;
            }
        }
        assert_eq!(
            module.colour(),
            (255, 0, 255),
            "out-of-range values clamp, and a NaN falls back to the default"
        );
    }
}
