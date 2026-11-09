use crate::mapping::MappedObject;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;

#[derive(Debug)]
pub struct NoHurtCamModule {
    pub module: ModuleData,
}

impl NoHurtCamModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::NoHurtCam,
                description: "Removes screen shake when taking damage".to_string(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
        }
    }
}

impl Module for NoHurtCamModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn on_tick(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn on_render(&self) {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.set_field(
                "hurtTime",
                crate::mapping::FieldType::Int,
                jni::objects::JValue::Int(0),
            );
        }
    }
    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }
    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
