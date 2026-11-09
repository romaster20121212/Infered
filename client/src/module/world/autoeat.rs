use crate::mapping::client::gamemode::MultiPlayerGameMode;
use crate::mapping::client::options::KeyMapping;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::LivingEntityRef;
use crate::mapping::inventory::{Inventory, Item};
use crate::mapping::MappedObject;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
 
static EATING: AtomicBool = AtomicBool::new(false);
const EAT_TIMEOUT: Duration = Duration::from_secs(8);

pub(crate) fn is_eating() -> bool {
    EATING.load(Ordering::SeqCst)
}

fn hunger_requires_food(level: i32, threshold: i32) -> bool {
    level < 20 && level <= threshold
}

 
fn food_scan_order(selected: i32) -> impl Iterator<Item = i32> {
    std::iter::once(selected).chain((0..36).filter(move |slot| *slot != selected))
}

#[derive(Debug)]
struct EatSession {
    player: LocalPlayer,
    inventory: Inventory,
    game_mode: MultiPlayerGameMode,
    use_key: KeyMapping,
    original_slot: i32,
    food_slot: i32,
    swapped_from: Option<i32>,
    original_use_down: bool,
    started_use: bool,
    food_item: Item,
    food_count: i32,
    hunger_before: i32,
    started_at: Instant,
}

 
 
#[derive(Debug)]
pub struct AutoEatModule {
    pub module: ModuleData,
    session: Mutex<Option<EatSession>>,
}

impl AutoEatModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoEat,
                description:
                    "Eats at the hunger threshold, then restores your item and AutoClicker"
                        .to_string(),
                category: ModuleCategory::World,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Hunger Threshold".to_string(),
                    value: 14.0,
                    min: 0.0,
                    max: 19.0,
                }],
            },
            session: Mutex::new(None),
        }
    }

    fn hunger_threshold(&self) -> i32 {
        self.module
            .get_setting("Hunger Threshold")
            .and_then(ModuleSetting::get_slider_value)
            .unwrap_or(14.0)
            .clamp(0.0, 19.0)
            .round() as i32
    }

    #[cfg(test)]
    pub(crate) fn expire_session_for_test(&self) {
        if let Some(session) = self.session.lock().unwrap().as_mut() {
            session.started_at = Instant::now() - EAT_TIMEOUT;
        }
    }

    fn best_food(inventory: &Inventory, selected: i32) -> anyhow::Result<Option<i32>> {
        for slot in food_scan_order(selected) {
            if inventory.get_item(slot)?.is_food()? {
                return Ok(Some(slot));
            }
        }
        Ok(None)
    }

     
     
    fn finish(&self) -> anyhow::Result<()> {
        let session = self.session.lock().unwrap().take();
        let result = (|| {
            let Some(session) = session else {
                return Ok(());
            };
            let mut errors = Vec::new();
            if let Err(error) = session.use_key.set_down(session.original_use_down) {
                errors.push(error);
            }
             
            let current_player = minecraft().player()?;
            if current_player
                .as_ref()
                .is_some_and(|player| player.is_same(&session.player))
            {
                if session.started_use {
                    if let Err(error) = session.game_mode.release_using_item(&session.player) {
                        errors.push(error);
                    }
                }
                match session
                    .player
                    .container_menu()
                    .and_then(|menu| menu.container_id())
                {
                    Ok(0) => {
                        if let Some(source) = session.swapped_from {
                            if let Err(error) = session.game_mode.container_swap(
                                0,
                                Inventory::menu_slot(source),
                                session.food_slot,
                                &session.player,
                            ) {
                                errors.push(error);
                            }
                        }
                    }
                    Ok(_) => {
                        if session.swapped_from.is_some() {
                            errors.push(anyhow::anyhow!(
                                "AutoEat cannot restore a swap while another container is open"
                            ));
                        }
                    }
                    Err(error) => errors.push(error),
                }
                if let Err(error) = session.inventory.set_selected(session.original_slot) {
                    errors.push(error);
                }
            }
            if errors.is_empty() {
                Ok(())
            } else {
                anyhow::bail!("AutoEat cleanup failed: {errors:?}")
            }
        })();
        EATING.store(false, Ordering::SeqCst);
        result
    }

    fn tick(&self) -> anyhow::Result<()> {
        if !self.module.enabled || !crate::state::client().modules.is_active() {
            return self.finish();
        }
        if crate::graphic::input::GUI_OPEN.load(Ordering::Relaxed)
            || minecraft().current_screen().is_open()
        {
            return self.finish();
        }
        let Some(player) = minecraft().player()? else {
            return self.finish();
        };
        if !player.is_alive()? || !minecraft().in_world() {
            return self.finish();
        }
        let session_finished = {
            let guard = self.session.lock().unwrap();
            if let Some(session) = guard.as_ref() {
                let same_player = player.is_same(&session.player);
                let stack = session.inventory.get_item(session.food_slot)?;
                let finished = !same_player
                    || session.started_at.elapsed() >= EAT_TIMEOUT
                    || session.inventory.get_selected()? != session.food_slot
                    || player.food_level()? > session.hunger_before
                    || stack.count()? < session.food_count
                    || !stack.get_item()?.is_same(&session.food_item)
                    || !player.is_using_item()?;
                if !finished {
                     
                    session.use_key.set_down(true)?;
                }
                Some(finished)
            } else {
                None
            }
        };
        if let Some(finished) = session_finished {
            return if finished { self.finish() } else { Ok(()) };
        } else {
            let hunger = player.food_level()?;
            if !hunger_requires_food(hunger, self.hunger_threshold()) || player.is_using_item()? {
                return Ok(());
            }
            if player.container_menu()?.container_id()? != 0 {
                return Ok(());
            }
            let inventory = player.get_inventory()?;
            let original_slot = inventory.get_selected()?;
            let Some(source) = Self::best_food(&inventory, original_slot)? else {
                return Ok(());
            };
            let Some(game_mode) = minecraft().game_mode()? else {
                return Ok(());
            };
            let use_key = minecraft().options()?.key_use()?;
            let original_use_down = use_key.is_down()?;
             
            if original_use_down {
                return Ok(());
            }
            let stack = inventory.get_item(source)?;
            let food_item = stack.get_item()?;
            let food_count = stack.count()?;
            let food_slot = if source < 9 { source } else { original_slot };
            *self.session.lock().unwrap() = Some(EatSession {
                player: player.clone(),
                inventory,
                game_mode,
                use_key,
                original_slot,
                food_slot,
                swapped_from: None,
                original_use_down,
                started_use: false,
                food_item,
                food_count,
                hunger_before: hunger,
                started_at: Instant::now(),
            });
            EATING.store(true, Ordering::SeqCst);
            let mut guard = self.session.lock().unwrap();
            let session = guard.as_mut().unwrap();
            if source >= 9 {
                session.game_mode.container_swap(
                    0,
                    Inventory::menu_slot(source),
                    food_slot,
                    &player,
                )?;
                session.swapped_from = Some(source);
            }
            session.inventory.set_selected(food_slot)?;
            session.use_key.set_down(true)?;
            session.started_use = true;
            if session.game_mode.use_item(&player)? {
                return Ok(());
            }
        }
        self.finish()
    }
}

impl Module for AutoEatModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.finish()
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.finish()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        match self.tick() {
            Ok(()) => Ok(()),
            Err(error) => {
                let cleanup = self.finish();
                match cleanup {
                    Ok(()) => Err(error),
                    Err(cleanup) => Err(error.context(format!("cleanup also failed: {cleanup}"))),
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunger_threshold_is_inclusive_but_never_eats_at_full_hunger() {
        assert!(!hunger_requires_food(15, 14));
        assert!(hunger_requires_food(14, 14));
        assert!(hunger_requires_food(0, 0));
        assert!(!hunger_requires_food(20, 20));
    }

    #[test]
    fn scans_the_whole_inventory_without_duplicates_and_prefers_held_food() {
        let order: Vec<_> = food_scan_order(5).collect();
        assert_eq!(&order[..3], &[5, 0, 1]);
        assert_eq!(order.len(), 36);
        let mut sorted = order;
        sorted.sort();
        assert_eq!(sorted, (0..36).collect::<Vec<_>>());
    }

    #[test]
    fn menu_exposes_an_editable_hunger_threshold() {
        let mut module = AutoEatModule::new();
        assert_eq!(module.module.id, ModuleId::AutoEat);
        assert_eq!(module.module.category, ModuleCategory::World);
        assert!(!module.module.enabled);
        assert_eq!(module.hunger_threshold(), 14);
        module
            .module
            .get_setting_mut("Hunger Threshold")
            .unwrap()
            .set_slider_value(8.0);
        assert_eq!(module.hunger_threshold(), 8);
    }
}
