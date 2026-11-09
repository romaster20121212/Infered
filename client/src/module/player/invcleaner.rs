use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct CleanState {
    last_drop: Option<Instant>,
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
];

 
 
 
#[derive(Debug)]
pub struct InvCleanerModule {
    pub module: ModuleData,
    state: Mutex<CleanState>,
}

impl InvCleanerModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::InvCleaner,
                description: "Removes unwanted inventory items".to_string(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Delay".to_string(),
                    value: 150.0,
                    min: 50.0,
                    max: 1000.0,
                }],
            },
            state: Mutex::new(CleanState::default()),
        }
    }
}

impl Module for InvCleanerModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        *self.state.lock().unwrap() = CleanState::default();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
         
         
        let menu = player.container_menu()?;
        if menu.container_id()? != 0 {
            return Ok(());
        }
        if !minecraft().current_screen().is_open() {
            return Ok(());
        }

        let delay = Duration::from_millis(
            self.module
                .get_setting("Delay")
                .and_then(|setting| setting.get_slider_value())
                .unwrap_or(150.0) as u64,
        );
        if self
            .state
            .lock()
            .unwrap()
            .last_drop
            .is_some_and(|last| last.elapsed() < delay)
        {
            return Ok(());
        }

         
         
        let items = menu.get_items()?;
        let own_start = items.len().saturating_sub(36);
        for (slot, item) in items.iter().enumerate().skip(own_start) {
            if item.is_empty()? {
                continue;
            }
            let name = item.get_item()?.registry_name()?;
            if JUNK_ITEMS.iter().any(|junk| name.ends_with(junk)) {
                 
                 
                if !crate::module::combat::interaction::window_click_allowed(&player) {
                    return Ok(());
                }
                let Some(game_mode) = minecraft().game_mode()? else {
                    return Ok(());
                };
                 
                if game_mode
                    .container_throw(0, slot as i32, true, &player)
                    .is_ok()
                {
                    self.state.lock().unwrap().last_drop = Some(Instant::now());
                }
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
