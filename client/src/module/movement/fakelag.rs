use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
 
 
 
 
 
 
 
#[derive(Debug)]
pub struct FakeLagModule {
    pub module: ModuleData,
}

fn delay_ms(value: f32) -> i32 {
    if value.is_finite() {
        value.round().clamp(1.0, 1000.0) as i32
    } else {
        100
    }
}

impl FakeLagModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::FakeLag,
                description:
                    "Delays every outbound packet in one ordered queue, interactions included"
                        .into(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Delay (ms)".into(),
                    value: 100.0,
                    min: 1.0,
                    max: 1000.0,
                }],
            },
        }
    }

    fn configured_delay(&self) -> i32 {
        delay_ms(
            self.module
                .get_setting("Delay (ms)")
                .and_then(ModuleSetting::get_slider_value)
                .unwrap_or(100.0),
        )
    }
}

impl Default for FakeLagModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for FakeLagModule {
    fn on_start(&self) -> anyhow::Result<()> {
        crate::net::set_strict_outbound_delay(self.configured_delay())
    }
    fn on_stop(&self) -> anyhow::Result<()> {
        crate::net::set_strict_outbound_delay(0)
    }
    fn on_tick(&self) -> anyhow::Result<()> {
         
        crate::net::set_strict_outbound_delay(self.configured_delay())
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
    fn delay_is_clamped_to_the_supported_range() {
        assert_eq!(delay_ms(1.0), 1);
        assert_eq!(delay_ms(100.0), 100);
        assert_eq!(delay_ms(1000.0), 1000);
        assert_eq!(delay_ms(1500.0), 1000);
        assert_eq!(delay_ms(-10.0), 1);
        assert_eq!(delay_ms(f32::NAN), 100);
    }

    #[test]
    fn the_slider_defaults_to_the_supported_delay() {
        let module = FakeLagModule::new();
        assert_eq!(module.module.id, ModuleId::FakeLag);
        assert_eq!(module.module.id.display_name(), "Fake Lag");
        assert_eq!(module.configured_delay(), 100);
        let slider = module
            .module
            .get_setting("Delay (ms)")
            .expect("Delay slider");
        assert_eq!(slider.get_slider_value(), Some(100.0));
        assert!(
            module.module.description.contains("interactions"),
            "the strict queue is what this module is"
        );
    }
}
