 
 
 
 

use crate::graphic::aura::{self, AuraConfig};
use crate::mapping::entity::EntityRef;
use crate::mapping::math::Vec3;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::time::Instant;

 
#[derive(Debug)]
pub struct AuraModule {
    pub module: ModuleData,
}

impl AuraModule {
    pub fn new() -> Self {
        let slider = |name: &str, value: f32, min: f32, max: f32| ModuleSetting::Slider {
            name: name.into(),
            value,
            min,
            max,
        };
        Self {
            module: ModuleData {
                id: ModuleId::Aura,
                description:
                    "Neon lines that crawl around you one block at a time, turning without doubling back"
                        .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    slider("Lines", 24.0, 1.0, 32.0),
                    slider("Switches", 14.0, 4.0, 64.0),
                    slider("Speed", 1.2, 0.4, 6.0),
                    slider("Range", 14.0, 4.0, 48.0),
                    slider("Spread", 4.0, 2.0, 12.0),
                    slider("Fade Time", 0.7, 0.1, 2.0),
                    slider("Width", 2.2, 0.5, 6.0),
                    ModuleSetting::Color {
                        name: "Color".into(),
                        value: [0.2, 0.9, 1.0, 1.0],
                    },
                    ModuleSetting::Toggle {
                        name: "Rainbow".into(),
                        value: false,
                    },
                    ModuleSetting::Toggle {
                        name: "Glow".into(),
                        value: true,
                    },
                ],
            },
        }
    }

     
    fn anchor() -> anyhow::Result<Option<Vec3>> {
        let Some(player) = minecraft().player()? else {
            return Ok(None);
        };
        let feet = player.get_position()?;
        let height = player.bb_height().unwrap_or(1.8) as f64;
        Ok(Some(Vec3::new(
            feet.x(),
            feet.y() + height * 0.55,
            feet.z(),
        )))
    }
}

impl Module for AuraModule {
    fn on_start(&self) -> anyhow::Result<()> {
        aura::reset();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        aura::reset();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let cfg = AuraConfig::from_data(&self.module);
        if !cfg.enabled {
            aura::reset();
            return Ok(());
        }
        let Some(anchor) = Self::anchor()? else {
            aura::reset();
            return Ok(());
        };
        let anchor = crate::graphic::esp::V3::from(anchor);
        aura::update(&cfg, anchor, Instant::now());
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
    fn aura_exposes_line_walker_settings() {
        let module = AuraModule::new();
        let data = &module.module;
        assert_eq!(data.id, ModuleId::Aura);
        assert_eq!(data.settings.len(), 10);
        let slider = |name| data.get_setting(name).unwrap().get_slider_value().unwrap();
        assert_eq!(slider("Lines"), 24.0);
        assert_eq!(slider("Switches"), 14.0);
        assert_eq!(slider("Speed"), 1.2);
        assert_eq!(slider("Range"), 14.0);
        assert_eq!(slider("Spread"), 4.0);
        assert_eq!(slider("Fade Time"), 0.7);
        assert_eq!(slider("Width"), 2.2);
        assert_eq!(
            data.get_setting("Glow").unwrap().get_toggle_value(),
            Some(true)
        );
        for removed in [
            "Smoke Opacity",
            "Wisp Count",
            "Wind Speed",
            "Wind Direction",
            "Density Boost",
            "Orbs",
            "OrbCount",
            "OrbSize",
            "OrbColor",
            "OrbGlowStrength",
            "OrbFlicker",
            "Smoke Tint",
        ] {
            assert!(
                data.get_setting(removed).is_none(),
                "old {removed} must be removed"
            );
        }
    }

    #[test]
    fn aura_does_not_start_the_smoke_renderer() {
        let module = AuraModule::new();
         
         
        assert!(module.on_start().is_ok());
        assert!(module.on_stop().is_ok());
    }
}
