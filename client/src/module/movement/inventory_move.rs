 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::input::{
    key_down, user_action_active, CLICK_TIMES_LEFT, CLICK_TIMES_RIGHT, GUI_OPEN, MOUSE_STATE,
};
use crate::mapping::client::options::KeyMapping;
use crate::mapping::client::screen::Screen;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

 
 
 
const CLICK_PAUSE_MS: u128 = 150;

 
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MoveKeys {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
}

 
 
pub(crate) fn allows_movement(screen: Screen) -> bool {
    matches!(
        screen,
        Screen::Inventory | Screen::Container | Screen::Crafting
    )
}

 
 
pub(crate) fn plan(screen: Screen, held: MoveKeys, jump_enabled: bool) -> MoveKeys {
    if !allows_movement(screen) {
        return MoveKeys::default();
    }
    MoveKeys {
        jump: held.jump && jump_enabled,
        ..held
    }
}

 
#[derive(Debug, Default)]
struct InventoryMoveState {
    applied: MoveKeys,
    active: bool,
}

 
fn mouse_down(left: bool) -> bool {
    MOUSE_STATE
        .lock()
        .map(|state| {
            if left {
                state.left_down
            } else {
                state.right_down
            }
        })
        .unwrap_or(false)
}

 
 
 
 
 
 
 
fn binding_down(mapping: &KeyMapping) -> bool {
    let Ok(name) = mapping.key_name() else {
        return false;
    };
    if name.starts_with("key.keyboard.") {
        return mapping
            .key_value()
            .map(|code| key_down(code))
            .unwrap_or(false);
    }
    match name.as_str() {
        "key.mouse.left" => mouse_down(true),
        "key.mouse.right" => mouse_down(false),
        _ => false,
    }
}

 
 
 
 
fn pause_keys(button_held: bool, last_click_ms: Option<u128>, now_ms: u128) -> bool {
    button_held || last_click_ms.is_some_and(|at| now_ms.saturating_sub(at) <= CLICK_PAUSE_MS)
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or(0)
}

 
fn last_click_ms() -> Option<u128> {
    fn latest(clicks: &Mutex<Vec<u128>>) -> Option<u128> {
        clicks.lock().ok().and_then(|times| times.last().copied())
    }
    match (latest(&CLICK_TIMES_LEFT), latest(&CLICK_TIMES_RIGHT)) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (left, right) => left.or(right),
    }
}

 
#[derive(Debug)]
pub struct InventoryMoveModule {
    pub module: ModuleData,
    state: Mutex<InventoryMoveState>,
}

impl InventoryMoveModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::InventoryMove,
                description: "Keeps movement keys working while a screen is open".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Toggle {
                    name: "Jump".to_string(),
                    value: true,
                }],
            },
            state: Mutex::new(InventoryMoveState::default()),
        }
    }

    fn jump_enabled(&self) -> bool {
        self.module
            .get_setting("Jump")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(true)
    }

     
     
     
     
    fn physical() -> MoveKeys {
        let Ok(options) = minecraft().options() else {
            return MoveKeys::default();
        };
        let down = |mapping: anyhow::Result<KeyMapping>| {
            mapping.map(|key| binding_down(&key)).unwrap_or(false)
        };
        MoveKeys {
            forward: down(options.key_up()),
            back: down(options.key_back()),
            left: down(options.key_left()),
            right: down(options.key_right()),
            jump: down(options.key_jump()),
        }
    }

     
    fn write(applied: &mut MoveKeys, want: MoveKeys) -> anyhow::Result<()> {
        if *applied == want {
            return Ok(());
        }
        let options = minecraft().options()?;
        if applied.forward != want.forward {
            options.key_up()?.set_down(want.forward)?;
        }
        if applied.back != want.back {
            options.key_back()?.set_down(want.back)?;
        }
        if applied.left != want.left {
            options.key_left()?.set_down(want.left)?;
        }
        if applied.right != want.right {
            options.key_right()?.set_down(want.right)?;
        }
        if applied.jump != want.jump {
            options.key_jump()?.set_down(want.jump)?;
        }
        *applied = want;
        Ok(())
    }
}

impl Module for InventoryMoveModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if let Ok(mut state) = self.state.lock() {
            *state = InventoryMoveState::default();
        }
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        let Ok(mut state) = self.state.lock() else {
            return Ok(());
        };
         
        let result = Self::write(&mut state.applied, MoveKeys::default());
        state.active = false;
        result
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Ok(mut state) = self.state.lock() else {
            return Ok(());
        };
        let screen = minecraft().current_screen();
        let active = !GUI_OPEN.load(Ordering::Relaxed) && allows_movement(screen);
        if !active && !state.active {
            return Ok(());  
        }
        let physical = Self::physical();

        let want = if !active {
             
             
             
            physical
        } else if pause_keys(user_action_active(), last_click_ms(), now_millis()) {
             
             
            MoveKeys::default()
        } else {
            plan(screen, physical, self.jump_enabled())
        };

        state.active = active;
        Self::write(&mut state.applied, want)
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

    fn held() -> MoveKeys {
        MoveKeys {
            forward: true,
            back: false,
            left: true,
            right: false,
            jump: true,
        }
    }

    #[test]
    fn inventory_and_container_screens_carry_the_keys() {
        for screen in [Screen::Inventory, Screen::Container, Screen::Crafting] {
            assert!(allows_movement(screen));
            let planned = plan(screen, held(), true);
            assert_eq!(planned, held());
        }
    }

    #[test]
    fn chat_and_menus_never_move_the_player() {
        for screen in [
            Screen::Chat,
            Screen::Menu,
            Screen::Disconnected,
            Screen::Unknown,
            Screen::None,
        ] {
            assert!(!allows_movement(screen));
            assert_eq!(plan(screen, held(), true), MoveKeys::default());
        }
    }

    #[test]
    fn the_jump_setting_only_removes_the_jump_key() {
        let planned = plan(Screen::Inventory, held(), false);
        assert!(!planned.jump);
        assert!(planned.forward && planned.left);
    }

    #[test]
    fn no_physical_keys_means_no_held_keys() {
        assert_eq!(
            plan(Screen::Inventory, MoveKeys::default(), true),
            MoveKeys::default()
        );
    }

    #[test]
    fn a_container_plan_carries_every_held_key() {
         
         
        let want = plan(Screen::Container, held(), true);
        assert_ne!(want, MoveKeys::default());
        assert_eq!(want, held());
    }

    #[test]
    fn a_held_button_suspends_the_keys() {
        assert!(pause_keys(true, None, 1_000));
        assert!(pause_keys(true, Some(1_000), 1_000));
    }

    #[test]
    fn clicks_suspend_the_keys_for_the_pause_window() {
        let click = 10_000;
        assert!(pause_keys(false, Some(click), click));
        assert!(pause_keys(false, Some(click), click + CLICK_PAUSE_MS));
        assert!(!pause_keys(false, Some(click), click + CLICK_PAUSE_MS + 1));
        assert!(!pause_keys(false, None, click));
    }

    #[test]
    fn a_clock_that_went_backwards_still_pauses() {
         
        assert!(pause_keys(false, Some(20_000), 10_000));
    }
}
