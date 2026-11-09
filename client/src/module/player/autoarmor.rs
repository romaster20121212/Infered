use crate::mapping::inventory::Inventory;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;

 
const INVENTORY_MENU_ID: i32 = 0;
 
const ARMOR_BASE: i32 = 36;
 
const STORAGE_SLOTS: i32 = 36;

 
 
#[derive(Debug)]
pub struct AutoArmorModule {
    pub module: ModuleData,
}

impl AutoArmorModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoArmor,
                description: "Equips armor from the inventory automatically".to_string(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
        }
    }
}

impl Module for AutoArmorModule {
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

         
         
        if player.container_menu()?.container_id()? != INVENTORY_MENU_ID {
            return Ok(());
        }

        let inventory = player.get_inventory()?;
        for slot in 0..STORAGE_SLOTS {
            let stack = inventory.get_item(slot)?;
            if stack.is_empty()? {
                continue;
            }

             
             
            let equipment_slot = player.equipment_slot_for_item(&stack)?;
            if !equipment_slot.is_armor()? {
                continue;
            }
            let armor_slot = equipment_slot.index(ARMOR_BASE)?;
            if !inventory.get_item(armor_slot)?.is_empty()? {
                continue;
            }

             
            let Some(game_mode) = minecraft().game_mode()? else {
                return Ok(());
            };
             
            let _ = game_mode.container_quick_move(
                INVENTORY_MENU_ID,
                Inventory::menu_slot(slot),
                &player,
            );
            return Ok(());
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
