use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::inventory::Inventory;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct RefillState {
    last_move: Option<Instant>,
}

 
 
 
const REFILL_ITEMS: &[&str] = &[
    "golden_apple",
    "enchanted_golden_apple",
    "splash_potion",
    "potion",
    "cooked_beef",
    "cooked_porkchop",
    "cooked_chicken",
    "bread",
    "totem_of_undying",
];

 
fn is_refill_item(name: &str) -> bool {
    REFILL_ITEMS.iter().any(|item| name.ends_with(item))
}

 
 
fn slot_wants_refill(inventory: &Inventory, hotbar_slot: i32) -> anyhow::Result<bool> {
    let stack = inventory.get_item(hotbar_slot)?;
    if stack.is_empty()? {
        return Ok(true);
    }
    let name = stack.get_item()?.registry_name()?;
    Ok(!is_refill_item(&name))
}

 
 
pub fn refill_hotbar(player: &LocalPlayer) -> anyhow::Result<bool> {
    let inventory = player.get_inventory()?;

     
    let mut wants: Option<i32> = None;
    for slot in 0..9 {
        if slot_wants_refill(&inventory, slot)? {
            wants = Some(slot);
            break;
        }
    }
    let Some(target_hotbar) = wants else {
        return Ok(false);  
    };

     
    for slot in 9..36 {
        let stack = inventory.get_item(slot)?;
        if stack.is_empty()? {
            continue;
        }
        let name = stack.get_item()?.registry_name()?;
        if is_refill_item(&name) {
             
             
            if !crate::module::combat::interaction::window_click_allowed(player) {
                return Ok(false);
            }
            let Some(game_mode) = minecraft().game_mode()? else {
                return Ok(false);
            };
             
             
             
            return Ok(game_mode
                .container_swap(0, slot, target_hotbar, player)
                .is_ok());
        }
    }
    Ok(false)
}

 
 
 
#[derive(Debug)]
pub struct RefillModule {
    pub module: ModuleData,
    state: Mutex<RefillState>,
}

impl RefillModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Refill,
                description: "Refills the hotbar with healing items".to_string(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Delay".to_string(),
                    value: 250.0,
                    min: 100.0,
                    max: 2000.0,
                }],
            },
            state: Mutex::new(RefillState::default()),
        }
    }
}

impl Module for RefillModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        *self.state.lock().unwrap() = RefillState::default();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
        if minecraft().current_screen().is_open() {
            return Ok(());  
        }

        let delay = Duration::from_millis(
            self.module
                .get_setting("Delay")
                .and_then(|setting| setting.get_slider_value())
                .unwrap_or(250.0) as u64,
        );
        if self
            .state
            .lock()
            .unwrap()
            .last_move
            .is_some_and(|last| last.elapsed() < delay)
        {
            return Ok(());
        }

        if refill_hotbar(&player)? {
            self.state.lock().unwrap().last_move = Some(Instant::now());
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
