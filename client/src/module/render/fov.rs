use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
const FOV_MIN: f32 = 30.0;
 
 
 
 
 
const FOV_MAX: f32 = 170.0;
 
 
const DEFAULT_FOV: i32 = 70;
 
 
 
const VANILLA_MAX: i32 = 110;

 
 
 
 
 
 
 
 
 
#[derive(Debug)]
pub struct FovModule {
    pub module: ModuleData,
     
     
     
    original: Mutex<Option<i32>>,
     
     
    applied: Mutex<Option<i32>>,
}

impl FovModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Fov,
                description: "Changes your field of view with a slider".to_string(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "FOV".to_string(),
                    value: 90.0,
                    min: FOV_MIN,
                    max: FOV_MAX,
                }],
            },
            original: Mutex::new(None),
            applied: Mutex::new(None),
        }
    }

     
    fn target(&self) -> i32 {
        let value = self
            .module
            .get_setting("FOV")
            .and_then(ModuleSetting::get_slider_value)
            .unwrap_or(DEFAULT_FOV as f32);
        (value.round() as i32).clamp(FOV_MIN as i32, FOV_MAX as i32)
    }

     
     
    fn apply(&self, value: i32) -> anyhow::Result<()> {
        if *self.applied.lock().unwrap() == Some(value) {
            return Ok(());
        }
        minecraft().options()?.fov()?.force_int(value)?;
        *self.applied.lock().unwrap() = Some(value);
        Ok(())
    }

     
    fn restore(&self) -> anyhow::Result<()> {
        let Some(original) = self.original.lock().unwrap().take() else {
            return Ok(());
        };
        minecraft().options()?.fov()?.force_int(original)?;
        *self.applied.lock().unwrap() = None;
        Ok(())
    }
}

impl Module for FovModule {
    fn on_start(&self) -> anyhow::Result<()> {
        let option = minecraft().options()?.fov()?;
        let current = option.get_int().unwrap_or(DEFAULT_FOV);
         
         
         
         
         
         
         
        let original = if (FOV_MIN as i32..=VANILLA_MAX).contains(&current) {
            current
        } else {
            DEFAULT_FOV
        };
        *self.original.lock().unwrap() = Some(original);
        self.apply(self.target())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.restore()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
        if self.original.lock().unwrap().is_none() {
            return self.on_start();
        }
        self.apply(self.target())
    }

    fn on_render(&self) {
         
         
         
         
         
         
        if self.original.lock().unwrap().is_none() {
            return;
        }
        let _ = self.apply(self.target());
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        self.restore()
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
    fn the_target_is_rounded_and_clamped_to_the_offered_range() {
        let module = FovModule::new();
        assert_eq!(module.target(), 90);

        let set = |value: f32| {
            let mut module = FovModule::new();
            module
                .module
                .get_setting_mut("FOV")
                .expect("the FOV slider exists")
                .set_slider_value(value);
            module.target()
        };
        assert_eq!(set(110.4), 110);
        assert_eq!(set(0.0), FOV_MIN as i32);
        assert_eq!(set(-40.0), FOV_MIN as i32);
        assert_eq!(set(999.0), FOV_MAX as i32);
        assert_eq!(set(70.0), 70);
    }

     
     
    #[test]
    fn a_fresh_module_has_captured_nothing() {
        let module = FovModule::new();
        assert!(module.original.lock().unwrap().is_none());
        assert!(module.applied.lock().unwrap().is_none());
    }

     
     
    #[test]
    fn the_offered_range_stays_under_the_projection_ceiling() {
        const { assert!(FOV_MIN < VANILLA_MAX as f32) };
        const { assert!(FOV_MAX > VANILLA_MAX as f32) };
        const { assert!(FOV_MAX < 179.0) };
    }
}
