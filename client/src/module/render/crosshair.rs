 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::client;
use std::sync::atomic::{AtomicBool, Ordering};

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CooldownStyle {
     
    Ring,
     
    Bar,
}

impl CooldownStyle {
    pub(crate) const OPTIONS: [&'static str; 2] = ["Ring", "Bar"];

    pub(crate) fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Bar,
            _ => Self::Ring,
        }
    }
}

 
pub(crate) const DEFAULT_COLOR: [f32; 4] = [0.93, 0.11, 0.14, 1.0];
pub(crate) const DEFAULT_SIZE: f32 = 1.0;
pub(crate) const DEFAULT_GAP: f32 = 3.5;
pub(crate) const DEFAULT_THICKNESS: f32 = 2.0;

 
 
static ACTIVE: AtomicBool = AtomicBool::new(false);

pub(crate) fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

#[derive(Debug)]
pub struct CrosshairModule {
    pub module: ModuleData,
}

impl CrosshairModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::CustomCrosshair,
                description: "Replaces the vanilla crosshair with a red, angular Infered reticle, \
                              including the attack-strength readout it used to show: a charge \
                              ring with a flash when the hit is ready, or the classic bar \
                              (local HUD only)"
                    .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Size".into(),
                        value: DEFAULT_SIZE,
                        min: 0.5,
                        max: 2.0,
                    },
                    ModuleSetting::Slider {
                        name: "Gap".into(),
                        value: DEFAULT_GAP,
                        min: 0.0,
                        max: 10.0,
                    },
                    ModuleSetting::Slider {
                        name: "Thickness".into(),
                        value: DEFAULT_THICKNESS,
                        min: 1.0,
                        max: 5.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Dot".into(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Glow".into(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Outline".into(),
                        value: true,
                    },
                    ModuleSetting::Choice {
                        name: "Cooldown Style".into(),
                        value: 0,
                        options: CooldownStyle::OPTIONS
                            .iter()
                            .map(|name| name.to_string())
                            .collect(),
                    },
                    ModuleSetting::Color {
                        name: "Color".into(),
                        value: DEFAULT_COLOR,
                    },
                ],
            },
        }
    }

     
     
    fn push(hidden: bool) -> anyhow::Result<()> {
        crate::camera_bridge::set_crosshair_hidden(hidden)
    }
}

impl Module for CrosshairModule {
    fn on_start(&self) -> anyhow::Result<()> {
        ACTIVE.store(false, Ordering::Relaxed);
        anyhow::ensure!(
            crate::jvmti::transform_installed(
                crate::jvmti::CROSSHAIR_HIDE_TRANSFORM_PARAMS.log_label
            ),
            "Crosshair: the vanilla reticle hook is not installed on this build"
        );
        Self::push(true)?;
        ACTIVE.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
         
        ACTIVE.store(false, Ordering::Relaxed);
        Self::push(false)
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        ACTIVE.store(false, Ordering::Relaxed);
        Self::push(false)
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

 
 
#[derive(Debug, Clone, Copy)]
pub(crate) struct CrosshairConfig {
    pub(crate) enabled: bool,
    pub(crate) size: f32,
    pub(crate) gap: f32,
    pub(crate) thickness: f32,
    pub(crate) dot: bool,
    pub(crate) glow: bool,
    pub(crate) outline: bool,
    pub(crate) color: egui::Color32,
    pub(crate) cooldown_style: CooldownStyle,
}

impl Default for CrosshairConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            size: DEFAULT_SIZE,
            gap: DEFAULT_GAP,
            thickness: DEFAULT_THICKNESS,
            dot: true,
            glow: true,
            outline: true,
            color: color(DEFAULT_COLOR),
            cooldown_style: CooldownStyle::Ring,
        }
    }
}

impl CrosshairConfig {
    fn from_data(data: &ModuleData) -> Self {
        Self {
            enabled: data.enabled,
            size: bounded(data, "Size", DEFAULT_SIZE, 0.5, 2.0),
            gap: bounded(data, "Gap", DEFAULT_GAP, 0.0, 10.0),
            thickness: bounded(data, "Thickness", DEFAULT_THICKNESS, 1.0, 5.0),
            dot: toggle(data, "Dot", true),
            glow: toggle(data, "Glow", true),
            outline: toggle(data, "Outline", true),
            color: match data.get_setting("Color") {
                Some(ModuleSetting::Color { value, .. }) => color(*value),
                _ => color(DEFAULT_COLOR),
            },
            cooldown_style: match data.get_setting("Cooldown Style") {
                Some(ModuleSetting::Choice { value, .. }) => CooldownStyle::from_index(*value),
                _ => CooldownStyle::Ring,
            },
        }
    }
}

pub(crate) fn read_config() -> CrosshairConfig {
    let mut cfg = CrosshairConfig::default();
    if let Some(arc) = client().modules.get(ModuleId::CustomCrosshair) {
        if let Ok(module) = arc.lock() {
            cfg = CrosshairConfig::from_data(module.get_module_data());
        }
    }
    cfg
}

fn bounded(data: &ModuleData, name: &str, fallback: f32, min: f32, max: f32) -> f32 {
    data.get_setting(name)
        .and_then(ModuleSetting::get_slider_value)
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(min, max))
        .unwrap_or(fallback)
}

fn toggle(data: &ModuleData, name: &str, fallback: bool) -> bool {
    data.get_setting(name)
        .and_then(ModuleSetting::get_toggle_value)
        .unwrap_or(fallback)
}

fn color(value: [f32; 4]) -> egui::Color32 {
    let channel = |index: usize| {
        let fallback = DEFAULT_COLOR[index];
        let value = if value[index].is_finite() {
            value[index]
        } else {
            fallback
        };
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    egui::Color32::from_rgba_unmultiplied(channel(0), channel(1), channel(2), channel(3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_module_ships_an_infered_reticle() {
        let module = CrosshairModule::new();
        assert_eq!(module.module.id, ModuleId::CustomCrosshair);
        assert_eq!(module.module.name(), "Crosshair");
        assert!(!module.module.enabled);
        let names: Vec<&str> = module
            .module
            .settings
            .iter()
            .map(ModuleSetting::name)
            .collect();
        assert_eq!(
            names,
            vec![
                "Size",
                "Gap",
                "Thickness",
                "Dot",
                "Glow",
                "Outline",
                "Cooldown Style",
                "Color"
            ]
        );
        let cfg = CrosshairConfig::from_data(&module.module);
        assert_eq!(cfg.size, DEFAULT_SIZE);
        assert_eq!(cfg.gap, DEFAULT_GAP);
        assert_eq!(cfg.thickness, DEFAULT_THICKNESS);
        assert!(cfg.dot && cfg.glow && cfg.outline);
        assert_eq!(cfg.cooldown_style, CooldownStyle::Ring);
        assert_eq!(cfg.color, color(DEFAULT_COLOR));
    }

    #[test]
    fn the_cooldown_readout_offers_a_ring_and_a_bar() {
        let mut module = CrosshairModule::new();
        assert_eq!(
            CrosshairConfig::from_data(&module.module).cooldown_style,
            CooldownStyle::Ring
        );
        match module.module.get_setting_mut("Cooldown Style") {
            Some(ModuleSetting::Choice { value, .. }) => *value = 1,
            other => panic!("Cooldown Style is not a choice: {other:?}"),
        }
        assert_eq!(
            CrosshairConfig::from_data(&module.module).cooldown_style,
            CooldownStyle::Bar
        );
        assert_eq!(
            CooldownStyle::from_index(7),
            CooldownStyle::Ring,
            "an unknown index falls back"
        );
        assert_eq!(CooldownStyle::OPTIONS, ["Ring", "Bar"]);
    }

    #[test]
    fn a_corrupt_crosshair_config_is_clamped() {
        let mut module = CrosshairModule::new();
        let mut set = |name: &str, value: f32| {
            module
                .module
                .get_setting_mut(name)
                .expect("setting is missing")
                .set_slider_value(value);
        };
        set("Size", f32::NAN);
        set("Gap", 99.0);
        set("Thickness", -5.0);
        if let Some(ModuleSetting::Color { value, .. }) = module.module.get_setting_mut("Color") {
            *value = [f32::NAN, -1.0, 2.0, 2.0];
        }
        let cfg = CrosshairConfig::from_data(&module.module);
        assert_eq!(cfg.size, DEFAULT_SIZE);
        assert_eq!(cfg.gap, 10.0);
        assert_eq!(cfg.thickness, 1.0);
        assert_eq!(cfg.color.r(), color(DEFAULT_COLOR).r());
        assert_eq!(cfg.color.g(), 0);
        assert_eq!(cfg.color.b(), 255);
        assert_eq!(cfg.color.a(), 255);
    }
}
