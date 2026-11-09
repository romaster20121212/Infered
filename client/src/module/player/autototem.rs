use crate::mapping::client::screen::Screen;
use crate::mapping::inventory::{self, Inventory, Item};
use crate::mapping::MappedObject;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
pub(crate) const INVENTORY_MENU_ID: i32 = 0;
 
 
pub(crate) const OFFHAND_SLOT: i32 = Inventory::SLOT_OFFHAND;
 
const STORAGE_SLOTS: i32 = 36;
 
 
 
 
 
 
const AUTO_TOTEM_SWAP_COOLDOWN: Duration = Duration::from_millis(100);
 
 
pub(crate) const SWAP_COOLDOWN: Duration = Duration::from_millis(250);
 
 
 
pub(crate) const INVENTORY_HOLD: Duration = Duration::from_millis(100);
 
const AUTO_TOTEM_INVENTORY_HOLD: Duration = Duration::ZERO;
 
const MAX_DELAY_SECONDS: f32 = 2.0;
 
 
 
 
pub(crate) const MAX_JITTER_SECONDS: f32 = 0.09;

 
 
 
 
 
 
#[derive(Debug)]
pub struct AutoTotemModule {
    pub module: ModuleData,
     
    last_swap: Mutex<Option<Instant>>,
     
    pending: Mutex<Option<(Instant, Duration)>>,
     
     
    opened: Mutex<Option<Instant>>,
     
     
    inventory_since: Mutex<Option<Instant>>,
     
    jitter_step: AtomicU32,
}

impl AutoTotemModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoTotem,
                description: "Keeps a Totem of Undying in the off-hand".to_string(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Min Delay".to_string(),
                        value: 0.02,
                        min: 0.0,
                        max: MAX_DELAY_SECONDS,
                    },
                    ModuleSetting::Slider {
                        name: "Max Delay".to_string(),
                        value: 0.06,
                        min: 0.0,
                        max: MAX_DELAY_SECONDS,
                    },
                    ModuleSetting::Slider {
                        name: "Inventory Delay".to_string(),
                        value: AUTO_TOTEM_INVENTORY_HOLD.as_secs_f32(),
                        min: 0.0,
                        max: MAX_DELAY_SECONDS,
                    },
                ],
            },
            last_swap: Mutex::new(None),
            pending: Mutex::new(None),
            opened: Mutex::new(None),
            inventory_since: Mutex::new(None),
            jitter_step: AtomicU32::new(0),
        }
    }

     
     
     
     
    fn jitter_fraction(&self) -> f32 {
        let step = self.jitter_step.fetch_add(1, Ordering::Relaxed);
        ((step as f64 * 0.618_033_988_749_895) % 1.0) as f32
    }

     
     
     
    fn random_delay(&self) -> Duration {
        let slider = |name: &str, fallback: f32| {
            self.module
                .get_setting(name)
                .and_then(|setting| setting.get_slider_value())
                .filter(|value| value.is_finite())
                .unwrap_or(fallback)
                .clamp(0.0, MAX_DELAY_SECONDS)
        };
        let (a, b) = (slider("Min Delay", 0.02), slider("Max Delay", 0.06));
        let (low, high) = if a <= b { (a, b) } else { (b, a) };
        let seconds =
            low + (high - low) * roll() as f32 + MAX_JITTER_SECONDS * self.jitter_fraction();
        Duration::from_secs_f32(seconds)
    }

     
     
    fn inventory_delay(&self) -> Duration {
        let seconds = self
            .module
            .get_setting("Inventory Delay")
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .unwrap_or(AUTO_TOTEM_INVENTORY_HOLD.as_secs_f32())
            .clamp(0.0, MAX_DELAY_SECONDS);
        Duration::from_secs_f32(seconds)
    }

     
     
    fn reset_wait(&self) {
        *self.pending.lock().unwrap_or_else(|p| p.into_inner()) = None;
        *self
            .inventory_since
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = None;
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

     
     
     
    fn close_when_still(
        &self,
        player: &crate::mapping::entity::player::LocalPlayer,
    ) -> anyhow::Result<()> {
        if crate::module::combat::interaction::window_click_allowed(player) {
            self.close_opened()
        } else {
            Ok(())
        }
    }
}

impl Module for AutoTotemModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        *self.last_swap.lock().unwrap_or_else(|p| p.into_inner()) = None;
        self.reset_wait();
        self.close_opened()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let mc = minecraft();
        let Some(player) = mc.player()? else {
            return Ok(());
        };

         
         
         
        if player.container_menu()?.container_id()? != INVENTORY_MENU_ID {
            self.forget_opened();
            self.reset_wait();
            return Ok(());
        }

        let inventory = player.get_inventory()?;
        let totem = inventory::totem_of_undying()?;

         
         
        if slot_holds(&inventory, OFFHAND_SLOT, &totem)? {
            self.reset_wait();
            return self.close_when_still(&player);
        }

         
        let Some(inventory_slot) = find_totem(&inventory, &totem)? else {
            self.reset_wait();
            return self.close_when_still(&player);  
        };

         
         
         
        self.pending
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get_or_insert_with(|| (Instant::now(), self.random_delay()));

         
         
        let screen = mc.current_screen();
        if screen != Screen::Inventory {
            *self
                .inventory_since
                .lock()
                .unwrap_or_else(|p| p.into_inner()) = None;
        }
        match screen {
            Screen::None => {
                 
                 
                 
                let user_closed = self
                    .opened
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .take()
                    .is_some();
                if user_closed {
                    self.reset_wait();
                    return Ok(());
                }
                 
                 
                 
                 
                if !crate::module::combat::interaction::window_click_allowed(&player) {
                    return Ok(());
                }
                 
                mc.open_inventory_screen(&player)?;
                *self.opened.lock().unwrap_or_else(|p| p.into_inner()) = Some(Instant::now());
                return Ok(());
            }
            Screen::Inventory => {}
            _ => return Ok(()),
        }

         
        let since = {
            let mut since = self
                .inventory_since
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            *since.get_or_insert_with(Instant::now)
        };
        if since.elapsed() < self.inventory_delay() {
            return Ok(());
        }

         
         
        {
            let pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
            let Some((started, delay)) = *pending else {
                return Ok(());
            };
            if started.elapsed() < delay {
                return Ok(());
            }
        }

         
         
         
        if !crate::module::combat::interaction::window_click_allowed(&player) {
            return Ok(());
        }

         
         
        {
            let mut last = self.last_swap.lock().unwrap_or_else(|p| p.into_inner());
            if last.is_some_and(|at| at.elapsed() < AUTO_TOTEM_SWAP_COOLDOWN) {
                return Ok(());
            }
            *last = Some(Instant::now());
        }

         
        let Some(game_mode) = mc.game_mode()? else {
            return Ok(());
        };
        game_mode.container_swap(
            INVENTORY_MENU_ID,
            Inventory::menu_slot(inventory_slot),
            OFFHAND_SLOT,
            &player,
        )?;
        self.reset_wait();

         
         
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

 
 
 
pub(crate) fn migrate_legacy_defaults(data: &mut ModuleData) {
    for (name, old, new) in [
        ("Min Delay", 0.1, 0.02),
        ("Max Delay", 0.3, 0.06),
        (
            "Inventory Delay",
            0.1,
            AUTO_TOTEM_INVENTORY_HOLD.as_secs_f32(),
        ),
    ] {
        if let Some(ModuleSetting::Slider { value, .. }) = data.get_setting_mut(name) {
            if (*value - old).abs() < 1e-6 {
                *value = new;
            }
        }
    }
}

 
 
 
pub(crate) fn roll() -> f64 {
    let bits = RandomState::new().build_hasher().finish();
    (bits >> 11) as f64 / (1u64 << 53) as f64
}

 
pub(crate) fn slot_holds(inventory: &Inventory, slot: i32, item: &Item) -> anyhow::Result<bool> {
    let stack = inventory.get_item(slot)?;
    if stack.is_empty()? {
        return Ok(false);
    }
    Ok(stack.get_item()?.is_same(item))
}

 
pub(crate) fn find_totem(inventory: &Inventory, totem: &Item) -> anyhow::Result<Option<i32>> {
    for slot in 0..STORAGE_SLOTS {
        if slot_holds(inventory, slot, totem)? {
            return Ok(Some(slot));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roll_stays_in_unit_interval() {
        for _ in 0..1000 {
            let value = roll();
            assert!((0.0..1.0).contains(&value));
        }
    }

    #[test]
    fn default_delay_sliders_are_within_range() {
        let module = AutoTotemModule::new();
        let min = module
            .module
            .get_setting("Min Delay")
            .unwrap()
            .get_slider_value()
            .unwrap();
        let max = module
            .module
            .get_setting("Max Delay")
            .unwrap()
            .get_slider_value()
            .unwrap();
        assert!(0.0 <= min && min <= max && max <= MAX_DELAY_SECONDS);
    }

    #[test]
    fn inventory_delay_defaults_to_the_hold_and_is_clamped() {
        let mut module = AutoTotemModule::new();
        let default = module.inventory_delay().as_secs_f32();
        assert!((default - AUTO_TOTEM_INVENTORY_HOLD.as_secs_f32()).abs() < 1e-6);

        for setting in module.module.settings.iter_mut() {
            if let ModuleSetting::Slider { name, value, .. } = setting {
                match name.as_str() {
                    "Inventory Delay" => *value = 99.0,
                    _ => {}
                }
            }
        }
        let clamped = module.inventory_delay().as_secs_f32();
        assert!((clamped - MAX_DELAY_SECONDS).abs() < 1e-6);
    }

    #[test]
    fn migration_retunes_only_the_old_stock_delays() {
        let mut module = AutoTotemModule::new();
        for (name, value) in [
            ("Min Delay", 0.1),
            ("Max Delay", 0.3),
            ("Inventory Delay", 0.1),
        ] {
            module
                .module
                .get_setting_mut(name)
                .unwrap()
                .set_slider_value(value);
        }
        migrate_legacy_defaults(&mut module.module);
        assert_eq!(
            module
                .module
                .get_setting("Min Delay")
                .unwrap()
                .get_slider_value(),
            Some(0.02)
        );
        assert_eq!(
            module
                .module
                .get_setting("Max Delay")
                .unwrap()
                .get_slider_value(),
            Some(0.06)
        );
        assert_eq!(
            module
                .module
                .get_setting("Inventory Delay")
                .unwrap()
                .get_slider_value(),
            Some(0.0)
        );

        let mut custom = AutoTotemModule::new();
        custom
            .module
            .get_setting_mut("Min Delay")
            .unwrap()
            .set_slider_value(0.4);
        migrate_legacy_defaults(&mut custom.module);
        assert_eq!(
            custom
                .module
                .get_setting("Min Delay")
                .unwrap()
                .get_slider_value(),
            Some(0.4)
        );
    }

    #[test]
    fn the_jitter_sequence_keeps_five_samples_spread_out() {
        let module = AutoTotemModule::new();
        for _ in 0..100 {
            let samples: Vec<f32> = (0..5)
                .map(|_| module.jitter_fraction() * MAX_JITTER_SECONDS)
                .collect();
            let mean = samples.iter().sum::<f32>() / samples.len() as f32;
            let variance = samples
                .iter()
                .map(|sample| (sample - mean).powi(2))
                .sum::<f32>()
                / (samples.len() - 1) as f32;
            let std_dev = variance.sqrt();
            assert!(
                std_dev > 0.015,
                "five-sample jitter std dev {std_dev} s is under 15 ms: {samples:?}"
            );
        }
    }

    #[test]
    fn random_delay_spreads_out_even_with_equal_sliders() {
        let mut module = AutoTotemModule::new();
        for setting in module.module.settings.iter_mut() {
            if let ModuleSetting::Slider { name, value, .. } = setting {
                match name.as_str() {
                    "Min Delay" | "Max Delay" => *value = 0.2,
                    _ => {}
                }
            }
        }
        let delays: Vec<f32> = (0..1000)
            .map(|_| module.random_delay().as_secs_f32())
            .collect();
        let mean = delays.iter().sum::<f32>() / delays.len() as f32;
        let variance = delays.iter().map(|d| (d - mean).powi(2)).sum::<f32>() / delays.len() as f32;
        let std_dev = variance.sqrt();
        assert!(std_dev > 0.015, "std dev {std_dev} s is under 15 ms");
    }

    #[test]
    fn random_delay_stays_between_the_two_sliders() {
        let mut module = AutoTotemModule::new();
        for setting in module.module.settings.iter_mut() {
            if let ModuleSetting::Slider { name, value, .. } = setting {
                match name.as_str() {
                    "Min Delay" => *value = 0.5,
                    "Max Delay" => *value = 1.0,
                    _ => {}
                }
            }
        }
        for _ in 0..1000 {
            let delay = module.random_delay().as_secs_f32();
            assert!(
                (0.5..=1.0 + MAX_JITTER_SECONDS).contains(&delay),
                "delay {delay} outside 0.5..=1.0 + jitter"
            );
        }
    }

    #[test]
    fn random_delay_honours_backwards_sliders() {
        let mut module = AutoTotemModule::new();
        for setting in module.module.settings.iter_mut() {
            if let ModuleSetting::Slider { name, value, .. } = setting {
                match name.as_str() {
                    "Min Delay" => *value = 1.0,
                    "Max Delay" => *value = 0.5,
                    _ => {}
                }
            }
        }
        for _ in 0..1000 {
            let delay = module.random_delay().as_secs_f32();
            assert!(
                (0.5..=1.0 + MAX_JITTER_SECONDS).contains(&delay),
                "delay {delay} outside 0.5..=1.0 + jitter"
            );
        }
    }
}
