use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct ManagerState {
    last_action: Option<Instant>,
}

 
 
 
 
 
 
 
#[derive(Debug)]
pub struct InventoryManagerModule {
    pub module: ModuleData,
    state: Mutex<ManagerState>,
}

 
const JUNK_ITEMS: &[&str] = &[
    "cobblestone",
    "dirt",
    "granite",
    "diorite",
    "andesite",
    "gravel",
    "netherrack",
    "dripstone_block",
    "tuff",
    "cobbled_deepslate",
    "rotten_flesh",
    "spider_eye",
    "poisonous_potato",
    "stick",
    "string",
    "wheat_seeds",
    "torchflower_seeds",
    "pitcher_pod",
    "sugar_cane",
    "kelp",
    "seagrass",
    "egg",
    "feather",
    "bone",
    "leather",
];

 
const HEALING_ITEMS: &[&str] = &[
    "golden_apple",
    "enchanted_golden_apple",
    "splash_potion",
    "potion",
];

 
const FOOD_ITEMS: &[&str] = &[
    "cooked_beef",
    "cooked_porkchop",
    "cooked_chicken",
    "cooked_mutton",
    "bread",
    "cooked_salmon",
    "cooked_cod",
    "baked_potato",
    "carrot",
];

 
const BRIDGE_ITEMS: &[&str] = &["obsidian", "ender_pearl", "cobweb", "water_bucket"];

impl InventoryManagerModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::InventoryManager,
                description: "Automates inventory management".to_string(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Delay".to_string(),
                        value: 200.0,
                        min: 50.0,
                        max: 1000.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Drop Junk".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Refill Hotbar".to_string(),
                        value: true,
                    },
                ],
            },
            state: Mutex::new(ManagerState::default()),
        }
    }

    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(fallback)
    }

     
    #[allow(clippy::too_many_lines)]
    fn step(&self, player: &crate::mapping::entity::player::LocalPlayer) -> anyhow::Result<bool> {
         
         
        if !crate::module::combat::interaction::window_click_allowed(player) {
            return Ok(false);
        }
        let Some(game_mode) = minecraft().game_mode()? else {
            return Ok(false);
        };
        let menu = player.container_menu()?;
        let container_id = menu.container_id()?;
        let items = menu.get_items()?;
        let own_start = items.len().saturating_sub(36);
        let inventory = player.get_inventory()?;

         
        if self.toggle("Drop Junk", true) {
            for slot in own_start..items.len() {
                let Some(stack) = items.get(slot) else {
                    continue;
                };
                if stack.is_empty()? {
                    continue;
                }
                let name = stack.get_item()?.registry_name()?;
                if JUNK_ITEMS.iter().any(|junk| name.ends_with(junk)) {
                     
                    if game_mode
                        .container_throw(container_id, slot as i32, true, player)
                        .is_ok()
                    {
                        return Ok(true);
                    }
                }
            }
        }

         
        if self.toggle("Refill Hotbar", true) {
            for hotbar in 0..9 {
                let held = inventory.get_item(hotbar)?;
                let held_name = if held.is_empty()? {
                    String::new()
                } else {
                    held.get_item()?.registry_name()?
                };
                let held_ok = HEALING_ITEMS
                    .iter()
                    .chain(FOOD_ITEMS)
                    .chain(BRIDGE_ITEMS)
                    .any(|item| held_name.contains(item));
                if held_ok {
                    continue;
                }
                 
                for slot in own_start..items.len() {
                    let Some(stack) = items.get(slot) else {
                        continue;
                    };
                    if stack.is_empty()? {
                        continue;
                    }
                    let name = stack.get_item()?.registry_name()?;
                    let wanted = HEALING_ITEMS.iter().any(|item| name.contains(item))
                        || FOOD_ITEMS.iter().any(|item| name.contains(item))
                        || BRIDGE_ITEMS.iter().any(|item| name.contains(item));
                    if wanted {
                         
                        if game_mode
                            .container_swap(container_id, slot as i32, hotbar, player)
                            .is_ok()
                        {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        Ok(false)
    }
}

impl Module for InventoryManagerModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        *self.state.lock().unwrap() = ManagerState::default();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let minecraft = minecraft();
        let Some(player) = minecraft.player()? else {
            return Ok(());
        };
         
        if !minecraft.current_screen().is_open() || player.container_menu()?.container_id()? != 0 {
            return Ok(());
        }
        let delay = Duration::from_millis(
            self.module
                .get_setting("Delay")
                .and_then(|setting| setting.get_slider_value())
                .unwrap_or(200.0) as u64,
        );
        if self
            .state
            .lock()
            .unwrap()
            .last_action
            .is_some_and(|last| last.elapsed() < delay)
        {
            return Ok(());
        }
        if self.step(&player)? {
            self.state.lock().unwrap().last_action = Some(Instant::now());
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
