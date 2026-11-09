 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::entity::LivingEntityRef;
use crate::mapping::inventory::Inventory;
use crate::module::combat::nearest_combatant;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
 
const SWORD_TIERS: &[(&str, i32)] = &[
    ("netherite_sword", 70),
    ("diamond_sword", 60),
    ("iron_sword", 50),
    ("stone_sword", 40),
    ("golden_sword", 30),
    ("wooden_sword", 20),
];

 
#[derive(Debug)]
struct Swap {
    previous: i32,
    sword: i32,
}

#[derive(Debug)]
pub struct AutoSwordModule {
    pub module: ModuleData,
    swapped: Mutex<Option<Swap>>,
}

impl AutoSwordModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoSword,
                description: "Equips the best hotbar sword while an enemy is close".to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Range".to_string(),
                        value: 6.0,
                        min: 3.0,
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
            swapped: Mutex::new(None),
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
            .unwrap_or(6.0)
            .clamp(1.0, 16.0)
    }

     
     
    fn restore_if_ours(&self, inventory: &Inventory, selected: i32) {
        let Some(swap) = self.swapped.lock().unwrap().take() else {
            return;
        };
        if selected == swap.sword {
            let _ = inventory.set_selected(swap.previous);
        }
    }

    fn tick(&self) -> anyhow::Result<()> {
        let minecraft = minecraft();
        let Some(player) = minecraft.player()? else {
            *self.swapped.lock().unwrap() = None;
            return Ok(());
        };
        if minecraft.current_screen().is_open()
            || !player.is_alive()?
            || player.container_menu()?.container_id()? != 0
        {
            *self.swapped.lock().unwrap() = None;
            return Ok(());
        }

        let inventory = player.get_inventory()?;
        let selected = inventory.get_selected()?;
        let threat = nearest_combatant(
            &player,
            self.range(),
            self.toggle("Players", true),
            self.toggle("Mobs", false),
        )?;
        if threat.is_none() {
            self.restore_if_ours(&inventory, selected);
            return Ok(());
        }
         
         
         
        if player.is_using_item()? {
            return Ok(());
        }

        let held = inventory.get_item(selected)?;
        let held_score = if held.is_empty()? {
            0
        } else {
            sword_score(&held.get_item()?.registry_name()?)
        };

        let Some((sword, score)) = best_sword(&inventory)? else {
            self.restore_if_ours(&inventory, selected);
            return Ok(());
        };
         
         
        if sword == selected || score <= held_score {
            return Ok(());
        }

         
         
         
         
        {
            let mut guard = self.swapped.lock().unwrap();
            let previous = match guard.as_ref() {
                Some(swap) if selected == swap.sword => swap.previous,
                _ => selected,
            };
            *guard = Some(Swap { previous, sword });
        }
        inventory.set_selected(sword)?;
        Ok(())
    }
}

impl Default for AutoSwordModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoSwordModule {
    fn on_start(&self) -> anyhow::Result<()> {
        *self.swapped.lock().unwrap() = None;
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        if let Ok(Some(player)) = minecraft().player() {
            if let Ok(inventory) = player.get_inventory() {
                if let Ok(selected) = inventory.get_selected() {
                    self.restore_if_ours(&inventory, selected);
                    return Ok(());
                }
            }
        }
        *self.swapped.lock().unwrap() = None;
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        match self.tick() {
            Ok(()) => Ok(()),
            Err(error) => {
                *self.swapped.lock().unwrap() = None;
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

 
fn sword_score(registry_name: &str) -> i32 {
    SWORD_TIERS
        .iter()
        .find(|(name, _)| registry_name.ends_with(name))
        .map(|(_, score)| *score)
        .unwrap_or(0)
}

 
fn best_sword(inventory: &Inventory) -> anyhow::Result<Option<(i32, i32)>> {
    let mut best = None;
    let mut best_score = 0;
    for slot in 0..9 {
        let stack = inventory.get_item(slot)?;
        if stack.is_empty()? {
            continue;
        }
        let score = sword_score(&stack.get_item()?.registry_name()?);
        if score > best_score {
            best_score = score;
            best = Some((slot, score));
        }
    }
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_identity_and_binding() {
        let module = AutoSwordModule::new();
        assert_eq!(module.module.id, ModuleId::AutoSword);
        assert_eq!(module.module.id.display_name(), "AutoSword");
        assert_eq!(module.module.category, ModuleCategory::Combat);
        assert_eq!(module.range(), 6.0);
        assert!(module.toggle("Players", false));
    }

    #[test]
    fn swords_rank_by_tier() {
        assert!(sword_score("minecraft:netherite_sword") > sword_score("minecraft:diamond_sword"));
        assert!(sword_score("minecraft:diamond_sword") > sword_score("minecraft:iron_sword"));
        assert!(sword_score("minecraft:iron_sword") > sword_score("minecraft:stone_sword"));
        assert!(sword_score("minecraft:golden_sword") > sword_score("minecraft:wooden_sword"));
    }

    #[test]
    fn non_swords_score_zero() {
        assert_eq!(sword_score("minecraft:diamond_axe"), 0);
        assert_eq!(sword_score("minecraft:diamond_pickaxe"), 0);
        assert_eq!(sword_score("minecraft:shield"), 0);
        assert_eq!(sword_score("minecraft:sword"), 0);
        assert_eq!(sword_score("minecraft:bow"), 0);
        assert!(sword_score("minecraft:stone_sword") > 0);
    }
}
