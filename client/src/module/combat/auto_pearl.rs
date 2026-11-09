 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::entity::LivingEntityRef;
use crate::mapping::inventory::Inventory;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const PEARL_ITEM: &str = "minecraft:ender_pearl";

 
 
 
const RESTORE_DELAY: Duration = Duration::from_millis(150);

 
#[derive(Debug)]
struct Restore {
    original_slot: i32,
    pearl_slot: i32,
    at: Instant,
}

#[derive(Debug)]
pub struct AutoPearlModule {
    pub module: ModuleData,
    pending: Mutex<Option<Restore>>,
    cooldown_until: Mutex<Option<Instant>>,
}

impl AutoPearlModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoPearl,
                description: "Throws an ender pearl when health drops to the threshold".to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Health".to_string(),
                        value: 6.0,
                        min: 1.0,
                        max: 20.0,
                    },
                    ModuleSetting::Slider {
                        name: "Cooldown".to_string(),
                        value: 5.0,
                        min: 1.0,
                        max: 30.0,
                    },
                ],
            },
            pending: Mutex::new(None),
            cooldown_until: Mutex::new(None),
        }
    }

    fn threshold(&self) -> f32 {
        self.module
            .get_setting("Health")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(6.0)
            .clamp(1.0, 20.0)
    }

    fn cooldown(&self) -> Duration {
        let seconds = self
            .module
            .get_setting("Cooldown")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(5.0)
            .clamp(1.0, 30.0);
        Duration::from_secs_f32(seconds)
    }

     
     
     
     
    fn settle_pending(&self) -> bool {
        let mut guard = self.pending.lock().unwrap();
        match guard.as_ref() {
            Some(restore) if Instant::now() >= restore.at => {}
            Some(_) => return true,
            None => return false,
        }
        let restore = guard.take().expect("checked above");
        drop(guard);
        restore_slot(&restore);
        true
    }

     
    fn restore_now(&self) {
        let Some(restore) = self.pending.lock().unwrap().take() else {
            return;
        };
        restore_slot(&restore);
    }

    fn cooling_down(&self) -> bool {
        let mut until = self.cooldown_until.lock().unwrap();
        match *until {
            Some(at) if Instant::now() < at => true,
            Some(_) => {
                *until = None;
                false
            }
            None => false,
        }
    }

    fn tick(&self) -> anyhow::Result<()> {
        if self.settle_pending() {
            return Ok(());
        }
        let minecraft = minecraft();
        let Some(player) = minecraft.player()? else {
            return Ok(());
        };
        if minecraft.current_screen().is_open()
            || !player.is_alive()?
            || player.container_menu()?.container_id()? != 0
            || player.is_using_item()?
            || self.cooling_down()
        {
            return Ok(());
        }
        if !needs_pearl(player.get_health()?, self.threshold()) {
            return Ok(());
        }

        let inventory = player.get_inventory()?;
        let original_slot = inventory.get_selected()?;
        let Some(pearl_slot) = find_pearl(&inventory, original_slot)? else {
            return Ok(());
        };
        let Some(game_mode) = minecraft.game_mode()? else {
            return Ok(());
        };

        if pearl_slot != original_slot {
            inventory.set_selected(pearl_slot)?;
        }
        if !game_mode.use_item(&player)? {
            if pearl_slot != original_slot {
                inventory.set_selected(original_slot)?;
            }
            return Ok(());
        }
        *self.pending.lock().unwrap() = Some(Restore {
            original_slot,
            pearl_slot,
            at: Instant::now() + RESTORE_DELAY,
        });
        *self.cooldown_until.lock().unwrap() = Some(Instant::now() + self.cooldown());
        Ok(())
    }
}

impl Default for AutoPearlModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoPearlModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.restore_now();
        *self.cooldown_until.lock().unwrap() = None;
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.restore_now();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        match self.tick() {
            Ok(()) => Ok(()),
            Err(error) => {
                self.restore_now();
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

fn restore_slot(restore: &Restore) {
    let Ok(Some(player)) = minecraft().player() else {
        return;
    };
    let Ok(inventory) = player.get_inventory() else {
        return;
    };
    if inventory.get_selected().ok() == Some(restore.pearl_slot) {
        let _ = inventory.set_selected(restore.original_slot);
    }
}

fn is_pearl(registry_name: &str) -> bool {
    registry_name == PEARL_ITEM
}

 
fn needs_pearl(health: f32, threshold: f32) -> bool {
    health > 0.0 && health <= threshold
}

 
fn pearl_scan_order(selected: i32) -> impl Iterator<Item = i32> {
    std::iter::once(selected).chain((0..9).filter(move |slot| *slot != selected))
}

fn find_pearl(inventory: &Inventory, selected: i32) -> anyhow::Result<Option<i32>> {
    for slot in pearl_scan_order(selected) {
        let stack = inventory.get_item(slot)?;
        if stack.is_empty()? {
            continue;
        }
        if is_pearl(&stack.get_item()?.registry_name()?) {
            return Ok(Some(slot));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_identity_and_binding() {
        let module = AutoPearlModule::new();
        assert_eq!(module.module.id, ModuleId::AutoPearl);
        assert_eq!(module.module.id.display_name(), "AutoPearl");
        assert_eq!(module.module.category, ModuleCategory::Combat);
        assert_eq!(module.threshold(), 6.0);
        assert_eq!(module.cooldown(), Duration::from_secs(5));
    }

    #[test]
    fn pearls_are_matched_exactly() {
        assert!(is_pearl("minecraft:ender_pearl"));
        assert!(!is_pearl("minecraft:ender_eye"));
        assert!(!is_pearl("minecraft:pearl"));
    }

    #[test]
    fn pearls_at_or_below_the_threshold_but_not_when_dead() {
        assert!(needs_pearl(6.0, 6.0));
        assert!(needs_pearl(3.5, 6.0));
        assert!(!needs_pearl(6.5, 6.0));
        assert!(!needs_pearl(0.0, 6.0));
    }

    #[test]
    fn scan_order_starts_at_the_selected_slot() {
        let slots: Vec<i32> = pearl_scan_order(2).collect();
        assert_eq!(slots.len(), 9);
        assert_eq!(slots[0], 2);
        assert_eq!(slots[1..], [0, 1, 3, 4, 5, 6, 7, 8]);
    }
}
