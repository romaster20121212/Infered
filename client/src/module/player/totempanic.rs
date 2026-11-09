use crate::mapping::client::screen::Screen;
use crate::mapping::entity::LivingEntityRef;
use crate::mapping::inventory::{self, Inventory};
use crate::module::player::autototem::{
    find_totem, slot_holds, INVENTORY_HOLD, INVENTORY_MENU_ID, OFFHAND_SLOT, SWAP_COOLDOWN,
};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;
use std::time::Instant;

 
const DEFAULT_HEALTH: f32 = 6.0;
 
const MAX_HEALTH: f32 = 20.0;

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Offhand,
    MainHand,
}

 
 
fn next_target(offhand_ok: bool, hand_ok: bool) -> Option<Target> {
    if !offhand_ok {
        Some(Target::Offhand)
    } else if !hand_ok {
        Some(Target::MainHand)
    } else {
        None
    }
}

 
 
fn panic_armed(health: f32, threshold: f32) -> bool {
    health.is_finite() && health <= threshold
}

 
 
 
#[derive(Debug)]
pub struct TotemPanicModule {
    pub module: ModuleData,
     
    last_swap: Mutex<Option<Instant>>,
     
    opened: Mutex<Option<Instant>>,
}

impl TotemPanicModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::TotemPanic,
                description: "Puts a totem in your off-hand and main hand when health is low"
                    .to_string(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Health".to_string(),
                    value: DEFAULT_HEALTH,
                    min: 1.0,
                    max: MAX_HEALTH,
                }],
            },
            last_swap: Mutex::new(None),
            opened: Mutex::new(None),
        }
    }

     
    fn health_threshold(&self) -> f32 {
        self.module
            .get_setting("Health")
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .unwrap_or(DEFAULT_HEALTH)
            .clamp(1.0, MAX_HEALTH)
    }

     
     
    fn forget_opened(&self) {
        *self.opened.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }

     
     
    fn close_opened(&self) -> anyhow::Result<()> {
        let was_opened = self
            .opened
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
            .is_some();
        if was_opened && minecraft().current_screen() == Screen::Inventory {
            minecraft().close_screen()?;
        }
        Ok(())
    }
}

impl Module for TotemPanicModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        *self.last_swap.lock().unwrap_or_else(|p| p.into_inner()) = None;
        self.close_opened()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let mc = minecraft();
        let Some(player) = mc.player()? else {
            return Ok(());
        };

         
        if !panic_armed(player.get_health()?, self.health_threshold()) {
            return self.close_opened();
        }

         
        if player.container_menu()?.container_id()? != INVENTORY_MENU_ID {
            self.forget_opened();
            return Ok(());
        }

        let inventory = player.get_inventory()?;
        let totem = inventory::totem_of_undying()?;
        let selected = inventory.get_selected()?;
        let offhand_ok = slot_holds(&inventory, OFFHAND_SLOT, &totem)?;
        let hand_ok = slot_holds(&inventory, selected, &totem)?;

         
        let Some(target) = next_target(offhand_ok, hand_ok) else {
            return self.close_opened();
        };

         
        let Some(source) = find_totem(&inventory, &totem)? else {
            return self.close_opened();
        };

         
         
        let screen = mc.current_screen();
        if !matches!(screen, Screen::None | Screen::Inventory) {
            return Ok(());
        }

         
        let opened_at = *self.opened.lock().unwrap_or_else(|p| p.into_inner());
        match opened_at {
            None if screen == Screen::None => {
                 
                 
                 
                if !crate::module::combat::interaction::window_click_allowed(&player) {
                    return Ok(());
                }
                mc.open_inventory_screen(&player)?;
                *self.opened.lock().unwrap_or_else(|p| p.into_inner()) = Some(Instant::now());
                return Ok(());
            }
            Some(at) if screen == Screen::Inventory && at.elapsed() < INVENTORY_HOLD => {
                return Ok(());
            }
            Some(_) if screen != Screen::Inventory => {
                self.forget_opened();
            }
            _ => {}
        }

         
         
         
        if !crate::module::combat::interaction::window_click_allowed(&player) {
            return Ok(());
        }

         
        {
            let mut last = self.last_swap.lock().unwrap_or_else(|p| p.into_inner());
            if last.is_some_and(|at| at.elapsed() < SWAP_COOLDOWN) {
                return Ok(());
            }
            *last = Some(Instant::now());
        }

         
        let Some(game_mode) = mc.game_mode()? else {
            return Ok(());
        };
        let button = match target {
            Target::Offhand => OFFHAND_SLOT,
            Target::MainHand => selected,
        };
        game_mode.container_swap(
            INVENTORY_MENU_ID,
            Inventory::menu_slot(source),
            button,
            &player,
        )
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
    fn arms_at_or_below_the_threshold_only() {
        assert!(panic_armed(6.0, 6.0));
        assert!(panic_armed(2.0, 6.0));
        assert!(!panic_armed(6.5, 6.0));
        assert!(
            !panic_armed(f32::NAN, 6.0),
            "a non-finite health never arms"
        );
    }

    #[test]
    fn the_off_hand_is_filled_before_the_main_hand() {
        assert_eq!(next_target(false, false), Some(Target::Offhand));
        assert_eq!(next_target(false, true), Some(Target::Offhand));
        assert_eq!(next_target(true, false), Some(Target::MainHand));
        assert_eq!(next_target(true, true), None);
    }

    #[test]
    fn default_threshold_is_within_the_slider_range() {
        let module = TotemPanicModule::new();
        let threshold = module.health_threshold();
        assert!((1.0..=MAX_HEALTH).contains(&threshold));
        assert_eq!(threshold, DEFAULT_HEALTH);
    }
}
