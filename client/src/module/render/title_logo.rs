 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::try_client;

 
 
 
pub(crate) const DEFAULT_WIDTH: f32 = 180.0;
 
 
pub(crate) const DEFAULT_TOP: f32 = 86.0;
 
pub(crate) const DEFAULT_BREATH: f32 = 0.06;
 
pub(crate) const DEFAULT_PERIOD: f32 = 3.6;

 
 
const WIDTH_RANGE: (f32, f32) = (80.0, 280.0);
const TOP_RANGE: (f32, f32) = (20.0, 200.0);
const BREATH_RANGE: (f32, f32) = (0.0, 0.25);
const PERIOD_RANGE: (f32, f32) = (1.0, 10.0);

#[derive(Debug)]
pub struct TitleLogoModule {
    pub module: ModuleData,
}

impl TitleLogoModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::TitleLogo,
                description: "Replaces the Minecraft logo and splash on the main menu with \
                              the Infered wordmark, breathing slowly"
                    .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Width".into(),
                        value: DEFAULT_WIDTH,
                        min: WIDTH_RANGE.0,
                        max: WIDTH_RANGE.1,
                    },
                    ModuleSetting::Slider {
                        name: "Top".into(),
                        value: DEFAULT_TOP,
                        min: TOP_RANGE.0,
                        max: TOP_RANGE.1,
                    },
                    ModuleSetting::Slider {
                        name: "Breath".into(),
                        value: DEFAULT_BREATH,
                        min: BREATH_RANGE.0,
                        max: BREATH_RANGE.1,
                    },
                    ModuleSetting::Slider {
                        name: "Period".into(),
                        value: DEFAULT_PERIOD,
                        min: PERIOD_RANGE.0,
                        max: PERIOD_RANGE.1,
                    },
                ],
            },
        }
    }
}

impl Module for TitleLogoModule {
     
     
     
     
     
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

 
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TitleLogoConfig {
    pub(crate) enabled: bool,
     
    pub(crate) width: f32,
     
    pub(crate) top: f32,
     
    pub(crate) breath: f32,
     
    pub(crate) period: f32,
}

impl Default for TitleLogoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            width: DEFAULT_WIDTH,
            top: DEFAULT_TOP,
            breath: DEFAULT_BREATH,
            period: DEFAULT_PERIOD,
        }
    }
}

impl TitleLogoConfig {
    fn from_data(data: &ModuleData) -> Self {
        Self {
            enabled: data.enabled,
            width: bounded(data, "Width", DEFAULT_WIDTH, WIDTH_RANGE),
            top: bounded(data, "Top", DEFAULT_TOP, TOP_RANGE),
            breath: bounded(data, "Breath", DEFAULT_BREATH, BREATH_RANGE),
            period: bounded(data, "Period", DEFAULT_PERIOD, PERIOD_RANGE),
        }
    }

     
     
    pub(crate) fn breath_at(&self, time: f64) -> f32 {
        if self.breath <= 0.0 || self.period <= 0.0 {
            return 1.0;
        }
        let phase = (time / self.period as f64) * std::f64::consts::TAU;
        1.0 + self.breath * phase.sin() as f32
    }
}

 
 
pub(crate) fn read_config() -> TitleLogoConfig {
    let Some(client) = try_client() else {
        return TitleLogoConfig::default();
    };
    let Some(handle) = client.modules.get(ModuleId::TitleLogo) else {
        return TitleLogoConfig::default();
    };
    let Ok(module) = handle.lock() else {
        return TitleLogoConfig::default();
    };
    TitleLogoConfig::from_data(module.get_module_data())
}

 
 
fn bounded(data: &ModuleData, name: &str, fallback: f32, range: (f32, f32)) -> f32 {
    data.get_setting(name)
        .and_then(ModuleSetting::get_slider_value)
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(range.0, range.1))
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_module_ships_the_infered_logo_and_a_slow_breath() {
        let module = TitleLogoModule::new();
        assert_eq!(module.module.id, ModuleId::TitleLogo);
        assert_eq!(module.module.name(), "Title Logo");
        assert_eq!(module.module.category, ModuleCategory::Visuals);
        assert!(!module.module.enabled, "the vanilla logo is the default");
        let names: Vec<&str> = module
            .module
            .settings
            .iter()
            .map(ModuleSetting::name)
            .collect();
        assert_eq!(names, vec!["Width", "Top", "Breath", "Period"]);
        let cfg = TitleLogoConfig::from_data(&module.module);
        assert_eq!(cfg.width, DEFAULT_WIDTH);
        assert_eq!(cfg.top, DEFAULT_TOP);
        assert_eq!(cfg.breath, DEFAULT_BREATH);
        assert_eq!(cfg.period, DEFAULT_PERIOD);
    }

    #[test]
    fn the_breath_grows_and_shrinks_around_its_resting_size() {
        let cfg = TitleLogoConfig {
            enabled: true,
            ..Default::default()
        };
         
        assert!((cfg.breath_at(0.0) - 1.0).abs() < 1e-6);
        assert!((cfg.breath_at(DEFAULT_PERIOD as f64 * 0.25) - 1.06).abs() < 1e-4);
        assert!((cfg.breath_at(DEFAULT_PERIOD as f64 * 0.75) - 0.94).abs() < 1e-4);
         
        assert!((cfg.breath_at(0.0) - cfg.breath_at(DEFAULT_PERIOD as f64)).abs() < 1e-5);
        let bounds = (0..400)
            .map(|i| cfg.breath_at(i as f64 / 40.0))
            .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)));
        assert!(bounds.0 >= 1.0 - DEFAULT_BREATH - 1e-4);
        assert!(bounds.1 <= 1.0 + DEFAULT_BREATH + 1e-4);
    }

    #[test]
    fn a_still_logo_has_no_breath() {
        let cfg = TitleLogoConfig {
            breath: 0.0,
            ..Default::default()
        };
        assert_eq!(cfg.breath_at(0.4), 1.0);
        assert_eq!(cfg.breath_at(17.0), 1.0);
    }

    #[test]
    fn a_corrupt_config_is_clamped_to_the_menu() {
        let mut module = TitleLogoModule::new();
        let mut set = |name: &str, value: f32| {
            module
                .module
                .get_setting_mut(name)
                .expect("setting is missing")
                .set_slider_value(value);
        };
        set("Width", f32::NAN);
        set("Top", 900.0);
        set("Breath", 5.0);
        set("Period", -3.0);
        let cfg = TitleLogoConfig::from_data(&module.module);
        assert_eq!(cfg.width, DEFAULT_WIDTH);
        assert_eq!(cfg.top, TOP_RANGE.1);
        assert_eq!(cfg.breath, BREATH_RANGE.1);
        assert_eq!(cfg.period, PERIOD_RANGE.0);
    }
}
