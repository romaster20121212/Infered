 
 
 
 
 
 

use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};

#[derive(Debug)]
pub struct NoParticlesModule {
    pub module: ModuleData,
}

impl NoParticlesModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::NoParticles,
                description: "Removes every particle the game would draw: explosions, \
                              smoke, flames, rain and block-break dust. Purely local."
                    .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
        }
    }

     
     
    fn push(hidden: bool) {
        if let Err(error) = crate::camera_bridge::set_no_particles_hidden(hidden) {
            log::debug!("No Particles: {error}");
        }
    }
}

impl Module for NoParticlesModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if !crate::jvmti::transform_installed(crate::jvmti::NO_PARTICLES_TRANSFORM_PARAMS.log_label)
        {
            log::warn!(
                "No Particles: the particle hook is not installed on this build, so \
                 particles cannot be hidden"
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
