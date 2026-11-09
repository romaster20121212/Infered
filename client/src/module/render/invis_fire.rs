 
 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};

#[derive(Debug)]
pub struct InvisFireModule {
    pub module: ModuleData,
}

impl InvisFireModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::InvisFire,
                description: "Hides the fire overlay drawn over your view while you are on \
                              fire. Purely local: you are still on fire."
                    .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
        }
    }

     
     
    fn push(hidden: bool) {
        if let Err(error) = crate::camera_bridge::set_fire_hidden(hidden) {
            log::debug!("Invis Fire: {error}");
        }
    }
}

impl Module for InvisFireModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if !crate::jvmti::transform_installed(crate::jvmti::FIRE_HIDE_TRANSFORM_PARAMS.log_label) {
            log::warn!(
                "Invis Fire: the fire overlay hook is not installed on this build, so the \
                 overlay cannot be hidden"
            );
        }
        Self::push(true);
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
        Self::push(false);
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        Self::push(false);
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
