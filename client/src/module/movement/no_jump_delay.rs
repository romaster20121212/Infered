 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::entity::LivingEntityRef;
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{mapping, minecraft};
use jni::objects::JValue;

 
const VANILLA_DELAY: i32 = 10;

 
 
fn clamp_delay(value: f32) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    value.round().clamp(0.0, VANILLA_DELAY as f32) as i32
}

 
#[derive(Debug)]
pub struct NoJumpDelayModule {
    pub module: ModuleData,
}

impl NoJumpDelayModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::NoJumpDelay,
                description: "Removes the jump cooldown so bunny-hops chain without a pause"
                    .to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Delay".to_string(),
                    value: 0.0,
                    min: 0.0,
                    max: VANILLA_DELAY as f32,
                }],
            },
        }
    }

    fn delay(&self) -> i32 {
        self.module
            .get_setting("Delay")
            .and_then(ModuleSetting::get_slider_value)
            .map(clamp_delay)
            .unwrap_or(0)
    }
}

impl Module for NoJumpDelayModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
        if !player.is_alive()? {
            return Ok(());
        }
         
         
         
        mapping().set_field(
            MinecraftClassType::LivingEntity,
            player.jni_ref().as_obj(),
            "noJumpDelay",
            FieldType::Int,
            JValue::Int(self.delay()),
        )?;
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
    fn the_module_ships_off_with_its_documented_setting() {
        let module = NoJumpDelayModule::new();
        assert_eq!(module.module.id, ModuleId::NoJumpDelay);
        assert_eq!(module.module.category, ModuleCategory::Movement);
        assert!(!module.module.enabled);

        let names: Vec<&str> = module
            .module
            .settings
            .iter()
            .map(ModuleSetting::name)
            .collect();
        assert_eq!(names, vec!["Delay"]);
        assert_eq!(module.delay(), 0);
    }

    #[test]
    fn the_delay_is_clamped_to_the_vanilla_range() {
        assert_eq!(clamp_delay(0.0), 0);
        assert_eq!(clamp_delay(4.4), 4);
        assert_eq!(clamp_delay(4.6), 5);
        assert_eq!(clamp_delay(-3.0), 0);
        assert_eq!(clamp_delay(99.0), VANILLA_DELAY);
        assert_eq!(clamp_delay(f32::NAN), 0);
        assert_eq!(clamp_delay(f32::INFINITY), 0);
    }

    #[test]
    fn the_slider_value_drives_the_written_delay() {
        let mut module = NoJumpDelayModule::new();
        module
            .module
            .get_setting_mut("Delay")
            .expect("Delay slider")
            .set_slider_value(7.0);
        assert_eq!(module.delay(), 7);
        module
            .module
            .get_setting_mut("Delay")
            .expect("Delay slider")
            .set_slider_value(100.0);
        assert_eq!(module.delay(), VANILLA_DELAY);
    }
}
