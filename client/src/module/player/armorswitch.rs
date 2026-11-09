use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct SwitchState {
    last_switch: Option<Instant>,
}

 
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArmorSet {
     
    Best,
     
    Diamond,
     
    Netherite,
}

impl ArmorSet {
    fn from_index(index: usize) -> ArmorSet {
        match index {
            0 => ArmorSet::Best,
            1 => ArmorSet::Diamond,
            _ => ArmorSet::Netherite,
        }
    }

    fn matches(&self, item_name: &str) -> bool {
        let bare = item_name.strip_prefix("minecraft:").unwrap_or(item_name);
        match self {
            ArmorSet::Best => true,
            ArmorSet::Diamond => bare.contains("diamond"),
            ArmorSet::Netherite => bare.contains("netherite"),
        }
    }
}

 
 
fn armor_score(item_name: &str) -> u8 {
    let bare = item_name.strip_prefix("minecraft:").unwrap_or(item_name);
    if bare.contains("netherite") {
        5
    } else if bare.contains("diamond") {
        4
    } else if bare.contains("iron") {
        3
    } else if bare.contains("golden") || bare.contains("chainmail") {
        2
    } else if bare.contains("leather") || bare.contains("turtle") {
        1
    } else {
        0
    }
}

 
 
 
#[derive(Debug)]
pub struct ArmorSwitchModule {
    pub module: ModuleData,
    state: Mutex<SwitchState>,
}

impl ArmorSwitchModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::ArmorSwitch,
                description: "Switches armor sets".to_string(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Choice {
                    name: "Set".to_string(),
                    value: 0,
                    options: vec![
                        "Best".to_string(),
                        "Diamond".to_string(),
                        "Netherite".to_string(),
                    ],
                }],
            },
            state: Mutex::new(SwitchState::default()),
        }
    }

    fn set(&self) -> ArmorSet {
        self.module
            .get_setting("Set")
            .and_then(|setting| match setting {
                ModuleSetting::Choice { value, .. } => Some(ArmorSet::from_index(*value)),
                _ => None,
            })
            .unwrap_or(ArmorSet::Best)
    }

     
     
    fn step(&self, player: &crate::mapping::entity::player::LocalPlayer) -> anyhow::Result<bool> {
        let armor_set = self.set();
        let menu = player.container_menu()?;
        let container_id = menu.container_id()?;
        let items = menu.get_items()?;
        let own_start = items.len().saturating_sub(36);

         
        let Some(game_mode) = minecraft().game_mode()? else {
            return Ok(false);
        };

         
        for slot in 5..9 {
            let Some(stack) = items.get(slot) else {
                continue;
            };
            if stack.is_empty()? {
                continue;
            }
            let name = stack.get_item()?.registry_name()?;
            if is_armor_name(&name) && !armor_set.matches(&name) {
                game_mode.container_quick_move(container_id, slot as i32, player)?;
                return Ok(true);
            }
        }

         
         
         
        let mut best: Option<(i32, u8, String)> = None;  
        for slot in own_start..items.len() {
            let Some(stack) = items.get(slot) else {
                continue;
            };
            if stack.is_empty()? {
                continue;
            }
            let name = stack.get_item()?.registry_name()?;
            if !is_armor_name(&name) || !armor_set.matches(&name) {
                continue;
            }
            let score = armor_score(&name);
            if score == 0 {
                continue;
            }
            if best.as_ref().is_none_or(|(_, b, _)| score > *b) {
                best = Some((slot as i32, score, name));
            }
        }
        if let Some((slot, score, name)) = best {
            let armor_slot = armor_menu_slot_for(&name);
            let worn_score = match items.get(armor_slot) {
                Some(worn) if !worn.is_empty()? => armor_score(&worn.get_item()?.registry_name()?),
                _ => 0,
            };
            if score > worn_score {
                 
                 
                game_mode.container_quick_move(container_id, slot, player)?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}

 
fn is_armor_name(name: &str) -> bool {
    name.contains("helmet")
        || name.contains("chestplate")
        || name.contains("leggings")
        || name.contains("boots")
}

 
fn armor_menu_slot_for(name: &str) -> usize {
    if name.contains("helmet") {
        5
    } else if name.contains("chestplate") {
        6
    } else if name.contains("leggings") {
        7
    } else {
        8  
    }
}

impl Module for ArmorSwitchModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        *self.state.lock().unwrap() = SwitchState::default();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
         
         
        let minecraft = minecraft();
        if !minecraft.current_screen().is_open() || !menu_is_player(&player)? {
            return Ok(());
        }
        if self
            .state
            .lock()
            .unwrap()
            .last_switch
            .is_some_and(|last| last.elapsed() < Duration::from_millis(200))
        {
            return Ok(());
        }
        if self.step(&player)? {
            self.state.lock().unwrap().last_switch = Some(Instant::now());
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

 
fn menu_is_player(player: &crate::mapping::entity::player::LocalPlayer) -> anyhow::Result<bool> {
    Ok(player.container_menu()?.container_id()? == 0)
}
