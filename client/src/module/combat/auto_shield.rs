 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::entity::LivingEntityRef;
use crate::mapping::inventory::Inventory;
use crate::module::combat::nearest_combatant;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::atomic::{AtomicBool, Ordering};

 
 
const SHIELD_ITEM: &str = "minecraft:shield";

#[derive(Debug)]
pub struct AutoShieldModule {
    pub module: ModuleData,
     
    held: AtomicBool,
}

impl AutoShieldModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoShield,
                description: "Raises the off-hand shield while a player or mob is close"
                    .to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Range".to_string(),
                        value: 5.0,
                        min: 2.0,
                        max: 12.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Players".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Mobs".to_string(),
                        value: false,
                    },
                ],
            },
            held: AtomicBool::new(false),
        }
    }

    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(fallback)
    }

    fn range(&self) -> f64 {
        self.module
            .get_setting("Range")
            .and_then(|setting| setting.get_slider_value())
            .map(|value| value as f64)
            .unwrap_or(5.0)
            .clamp(1.0, 16.0)
    }

     
     
    fn release(&self) {
        if !self.held.swap(false, Ordering::Relaxed) {
            return;
        }
        if let Ok(options) = minecraft().options() {
            if let Ok(key) = options.key_use() {
                let _ = key.set_down(false);
            }
        }
    }
}

impl Default for AutoShieldModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoShieldModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.release();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.release();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        match self.check() {
            Ok(()) => Ok(()),
             
             
            Err(error) => {
                self.release();
                Err(error)
            }
        }
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

impl AutoShieldModule {
    fn check(&self) -> anyhow::Result<()> {
        let minecraft = minecraft();
        let Some(player) = minecraft.player()? else {
            self.release();
            return Ok(());
        };
        if minecraft.current_screen().is_open() || player.container_menu()?.container_id()? != 0 {
            self.release();
            return Ok(());
        }

         
         
        let shield_ready = offhand_shield(&player.get_inventory()?)
            .map(|shield| shield && !left_mouse_down())
            .unwrap_or(false);
        if !shield_ready {
            self.release();
            return Ok(());
        }

        if nearest_combatant(
            &player,
            self.range(),
            self.toggle("Players", true),
            self.toggle("Mobs", false),
        )?
        .is_none()
        {
            self.release();
            return Ok(());
        }

         
         
         
        if self.held.load(Ordering::Relaxed) {
            minecraft.options()?.key_use()?.set_down(true)?;
            return Ok(());
        }
        if player.is_using_item()? {
            return Ok(());
        }
        let use_key = minecraft.options()?.key_use()?;
        if use_key.is_down()? {
            return Ok(());  
        }
        use_key.set_down(true)?;
        self.held.store(true, Ordering::Relaxed);
        Ok(())
    }
}

 
fn offhand_shield(inventory: &Inventory) -> anyhow::Result<bool> {
    let stack = inventory.get_item(Inventory::SLOT_OFFHAND)?;
    if stack.is_empty()? {
        return Ok(false);
    }
    Ok(is_shield(&stack.get_item()?.registry_name()?))
}

fn is_shield(registry_name: &str) -> bool {
    registry_name == SHIELD_ITEM
}

 
 
fn left_mouse_down() -> bool {
    crate::graphic::input::MOUSE_STATE
        .lock()
        .map(|mouse| mouse.left_down)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_identity_and_binding() {
        let module = AutoShieldModule::new();
        assert_eq!(module.module.id, ModuleId::AutoShield);
        assert_eq!(module.module.id.display_name(), "AutoShield");
        assert_eq!(module.module.category, ModuleCategory::Combat);
        assert_eq!(module.module.key_bind, KeyboardKey::KeyNone);
        assert!(!module.module.enabled);
    }

    #[test]
    fn settings_have_the_expected_shape() {
        let module = AutoShieldModule::new();
        assert_eq!(module.range(), 5.0);
        assert!(module.toggle("Players", false));
        assert!(!module.toggle("Mobs", true));
        assert_eq!(module.module.settings.len(), 3);
    }

    #[test]
    fn only_a_shield_counts() {
        assert!(is_shield("minecraft:shield"));
        assert!(!is_shield("minecraft:iron_sword"));
        assert!(!is_shield("minecraft:shield_of_awesome"));
    }
}
