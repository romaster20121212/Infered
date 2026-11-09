use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;

 
 
const PLAYER_INVENTORY_SLOTS: usize = 36;

 
 
#[derive(Debug)]
pub struct ChestStealerModule {
    pub module: ModuleData,
}

impl ChestStealerModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::ChestStealer,
                description: "Empties an open container into the inventory".to_string(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
        }
    }
}

impl Module for ChestStealerModule {
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

         
        let menu = player.container_menu()?;
        let container_id = menu.container_id()?;
        if container_id == 0 {
            return Ok(());
        }

         
        let items = menu.get_items()?;
        let container_slots = items.len().saturating_sub(PLAYER_INVENTORY_SLOTS);

         
        for (slot, item) in items.iter().enumerate().take(container_slots) {
            if !item.is_empty()? {
                let Some(game_mode) = minecraft().game_mode()? else {
                    return Ok(());
                };
                 
                let _ = game_mode.container_quick_move(container_id, slot as i32, &player);
                return Ok(());
            }
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
