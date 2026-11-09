 
 
 
 
 

use std::sync::Mutex;

use crate::camera_bridge::set_totem_pop_scale;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};

 
const MIN_SIZE: f32 = 0.1;
const MAX_SIZE: f32 = 5.0;
const DEFAULT_SIZE: f32 = 1.0;
 
const VANILLA_SIZE: f32 = 1.0;

#[derive(Debug)]
pub struct TotemPopModule {
    pub module: ModuleData,
     
    applied: Mutex<Option<f32>>,
}

impl TotemPopModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::TotemPop,
                description: "Changes how big the totem pop is drawn. 1.0 is vanilla.".into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Size".into(),
                    value: DEFAULT_SIZE,
                    min: MIN_SIZE,
                    max: MAX_SIZE,
                }],
            },
            applied: Mutex::new(None),
        }
    }

     
    fn size(&self) -> f32 {
        self.module
            .get_setting("Size")
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(MIN_SIZE, MAX_SIZE))
            .unwrap_or(DEFAULT_SIZE)
    }

     
     
    fn push(&self, size: f32) {
        match set_totem_pop_scale(size) {
            Ok(()) => {
                *self.applied.lock().unwrap_or_else(|p| p.into_inner()) = Some(size);
            }
            Err(error) => log::debug!("Totem Pop: {error}"),
        }
    }
}

impl Module for TotemPopModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if !crate::jvmti::transform_installed(crate::jvmti::TOTEM_POP_TRANSFORM_PARAMS.log_label) {
            log::warn!(
                "Totem Pop: the floating item hook is not installed on this build, so the pop \
                 cannot be resized"
            );
        }
        self.push(self.size());
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
        self.push(VANILLA_SIZE);
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        self.push(VANILLA_SIZE);
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
        let size = self.size();
        let applied = *self.applied.lock().unwrap_or_else(|p| p.into_inner());
        if applied != Some(size) {
            self.push(size);
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
