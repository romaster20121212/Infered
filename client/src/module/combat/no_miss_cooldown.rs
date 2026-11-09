use crate::mapping::entity::PlayerRef;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;

#[derive(Debug)]
pub struct NoMissCooldownModule {
    pub module: ModuleData,
}

impl NoMissCooldownModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::NoMissCooldown,
                description: "Resets attack cooldown even when you miss".to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
        }
    }
}

impl Module for NoMissCooldownModule {
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
        if player.attack_strength_scale()? < 1.0 {
            player.reset_last_attacked_ticks()?;
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
