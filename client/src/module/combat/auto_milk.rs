 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::client::options::KeyMapping;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::LivingEntityRef;
use crate::mapping::inventory::Inventory;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MILK_ITEM: &str = "minecraft:milk_bucket";

 
 
const DRINK_TIMEOUT: Duration = Duration::from_secs(6);

 
 
const DRINK_START_GRACE: Duration = Duration::from_millis(250);

 
const HARMFUL_EFFECTS: &[&str] = &[
    "poison",
    "wither",
    "slowness",
    "weakness",
    "mining_fatigue",
    "blindness",
    "nausea",
    "levitation",
    "hunger",
    "darkness",
    "bad_omen",
    "trial_omen",
    "unluck",
    "infested",
    "oozing",
    "weaving",
    "wind_charged",
];

#[derive(Debug)]
struct Drink {
    player: LocalPlayer,
    use_key: KeyMapping,
     
    original_slot: i32,
     
    milk_slot: i32,
     
    pressed: bool,
    started: Instant,
}

#[derive(Debug)]
pub struct AutoMilkModule {
    pub module: ModuleData,
    drink: Mutex<Option<Drink>>,
    cooldown_until: Mutex<Option<Instant>>,
}

impl AutoMilkModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoMilk,
                description: "Drinks a milk bucket to clear a harmful potion effect".to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Cooldown".to_string(),
                    value: 10.0,
                    min: 0.0,
                    max: 30.0,
                }],
            },
            drink: Mutex::new(None),
            cooldown_until: Mutex::new(None),
        }
    }

    fn cooldown(&self) -> Duration {
        let seconds = self
            .module
            .get_setting("Cooldown")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(10.0)
            .clamp(0.0, 30.0);
        Duration::from_secs_f32(seconds)
    }

    fn tick(&self) -> anyhow::Result<()> {
        let minecraft = minecraft();
        let Some(player) = minecraft.player()? else {
            return self.finish();
        };
        if minecraft.current_screen().is_open()
            || !player.is_alive()?
            || player.container_menu()?.container_id()? != 0
        {
            return self.finish();
        }
        if self.drink.lock().unwrap().is_some() {
            return self.continue_drink(&player);
        }
        if self.cooling_down() {
            return Ok(());
        }
        if player.is_using_item()? {
            return Ok(());
        }
        if !has_harmful_effect(&player.get_active_effects()?) {
            return Ok(());
        }

        let inventory = player.get_inventory()?;
        let original_slot = inventory.get_selected()?;
        let Some(milk_slot) = find_milk(&inventory, original_slot)? else {
            return Ok(());
        };
        let use_key = minecraft.options()?.key_use()?;
         
        if use_key.is_down()? {
            return Ok(());
        }

        if milk_slot != original_slot {
            inventory.set_selected(milk_slot)?;
        }
        use_key.set_down(true)?;
        *self.drink.lock().unwrap() = Some(Drink {
            player,
            use_key,
            original_slot,
            milk_slot,
            pressed: true,
            started: Instant::now(),
        });
        Ok(())
    }

     
     
    fn continue_drink(&self, player: &LocalPlayer) -> anyhow::Result<()> {
        let elapsed = {
            let guard = self.drink.lock().unwrap();
            let Some(drink) = guard.as_ref() else {
                return Ok(());
            };
            if drink.pressed {
                drink.use_key.set_down(true)?;
            }
            drink.started.elapsed()
        };
        let still_using = player.is_using_item().unwrap_or(false);
        let finished = elapsed > DRINK_TIMEOUT || (elapsed > DRINK_START_GRACE && !still_using);
        if finished {
            return self.finish();
        }
        Ok(())
    }

     
     
    fn finish(&self) -> anyhow::Result<()> {
        let Some(drink) = self.drink.lock().unwrap().take() else {
            return Ok(());
        };
        if drink.pressed {
            let _ = drink.use_key.set_down(false);
        }
        let player = minecraft().player().ok().flatten().unwrap_or(drink.player);
        if let Ok(inventory) = player.get_inventory() {
            if inventory.get_selected().ok() == Some(drink.milk_slot) {
                let _ = inventory.set_selected(drink.original_slot);
            }
        }
        *self.cooldown_until.lock().unwrap() = Some(Instant::now() + self.cooldown());
        Ok(())
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
}

impl Default for AutoMilkModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoMilkModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.finish()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        match self.tick() {
            Ok(()) => Ok(()),
            Err(error) => match self.finish() {
                Ok(()) => Err(error),
                Err(cleanup) => Err(error.context(format!("cleanup also failed: {cleanup}"))),
            },
        }
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

fn is_milk(registry_name: &str) -> bool {
    registry_name == MILK_ITEM
}

 
 
fn is_harmful_effect(name: &str) -> bool {
    let short = name.strip_prefix("minecraft:").unwrap_or(name);
    HARMFUL_EFFECTS.contains(&short)
}

fn has_harmful_effect(effects: &[(String, i32, i32)]) -> bool {
    effects.iter().any(|(name, _, _)| is_harmful_effect(name))
}

 
 
fn milk_scan_order(selected: i32) -> impl Iterator<Item = i32> {
    std::iter::once(selected).chain((0..9).filter(move |slot| *slot != selected))
}

fn find_milk(inventory: &Inventory, selected: i32) -> anyhow::Result<Option<i32>> {
    for slot in milk_scan_order(selected) {
        let stack = inventory.get_item(slot)?;
        if stack.is_empty()? {
            continue;
        }
        if is_milk(&stack.get_item()?.registry_name()?) {
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
        let module = AutoMilkModule::new();
        assert_eq!(module.module.id, ModuleId::AutoMilk);
        assert_eq!(module.module.id.display_name(), "AutoMilk");
        assert_eq!(module.module.category, ModuleCategory::Combat);
        assert_eq!(module.cooldown(), Duration::from_secs(10));
    }

    #[test]
    fn milk_is_the_bucket_only() {
        assert!(is_milk("minecraft:milk_bucket"));
        assert!(!is_milk("minecraft:water_bucket"));
        assert!(!is_milk("minecraft:milk"));
    }

    #[test]
    fn harmful_effects_are_recognised_with_or_without_namespace() {
        assert!(is_harmful_effect("poison"));
        assert!(is_harmful_effect("minecraft:poison"));
        assert!(is_harmful_effect("mining_fatigue"));
        assert!(!is_harmful_effect("speed"));
        assert!(!is_harmful_effect("regeneration"));
        assert!(!is_harmful_effect("night_vision"));
    }

    #[test]
    fn any_harmful_effect_warrants_a_drink() {
        assert!(!has_harmful_effect(&[]));
        assert!(!has_harmful_effect(&[("speed".into(), 0, 200)]));
        assert!(has_harmful_effect(&[
            ("speed".into(), 0, 200),
            ("wither".into(), 1, 100)
        ]));
    }

    #[test]
    fn scan_order_starts_at_the_selected_slot() {
        let slots: Vec<i32> = milk_scan_order(4).collect();
        assert_eq!(slots.len(), 9);
        assert_eq!(slots[0], 4);
        assert_eq!(slots[1..], [0, 1, 2, 3, 5, 6, 7, 8]);
    }
}
