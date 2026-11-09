 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::client::gamemode::MultiPlayerGameMode;
use crate::mapping::client::options::KeyMapping;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::LivingEntityRef;
use crate::mapping::inventory::{Inventory, Item};
use crate::mapping::MappedObject;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
 
const EAT_TIMEOUT: Duration = Duration::from_secs(8);

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GappleChoice {
    Any,
    Golden,
    Enchanted,
}

impl GappleChoice {
    const ALL: [GappleChoice; 3] = [
        GappleChoice::Any,
        GappleChoice::Golden,
        GappleChoice::Enchanted,
    ];

    fn from_choice(index: usize) -> GappleChoice {
        Self::ALL.get(index).copied().unwrap_or(GappleChoice::Any)
    }

    fn label(self) -> &'static str {
        match self {
            GappleChoice::Any => "Any",
            GappleChoice::Golden => "Golden",
            GappleChoice::Enchanted => "Enchanted",
        }
    }

     
     
    fn matches(self, registry_name: &str) -> bool {
        let short = registry_name
            .strip_prefix("minecraft:")
            .unwrap_or(registry_name);
        match self {
            GappleChoice::Any => short == "golden_apple" || short == "enchanted_golden_apple",
            GappleChoice::Golden => short == "golden_apple",
            GappleChoice::Enchanted => short == "enchanted_golden_apple",
        }
    }
}

 
 
fn needs_heal(health: f32, threshold: f32) -> bool {
    health > 0.0 && health <= threshold
}

 
fn gapple_scan_order(selected: i32) -> impl Iterator<Item = i32> {
    std::iter::once(selected).chain((0..36).filter(move |slot| *slot != selected))
}

 
 
#[derive(Debug)]
struct EatSession {
    player: LocalPlayer,
    inventory: Inventory,
    game_mode: MultiPlayerGameMode,
    use_key: KeyMapping,
    original_slot: i32,
    apple_slot: i32,
    swapped_from: Option<i32>,
    original_use_down: bool,
    started_use: bool,
    apple: Item,
    count: i32,
    health_before: f32,
    started_at: Instant,
}

 
 
 
 
 
 
#[derive(Debug)]
pub struct AutoGappleModule {
    pub module: ModuleData,
    session: Mutex<Option<EatSession>>,
     
    cooldown_until: Mutex<Option<Instant>>,
}

impl AutoGappleModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoGapple,
                description: "Eats a golden apple when health drops below the threshold, \
                              then restores your item"
                    .to_string(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Health".to_string(),
                        value: 10.0,
                        min: 1.0,
                        max: 20.0,
                    },
                    ModuleSetting::Choice {
                        name: "Item".to_string(),
                        value: 0,
                        options: GappleChoice::ALL
                            .iter()
                            .map(|choice| choice.label().to_string())
                            .collect(),
                    },
                    ModuleSetting::Slider {
                        name: "Cooldown".to_string(),
                         
                         
                        value: 8.0,
                        min: 0.0,
                        max: 20.0,
                    },
                ],
            },
            session: Mutex::new(None),
            cooldown_until: Mutex::new(None),
        }
    }

    fn slider(&self, name: &str, fallback: f32) -> f32 {
        self.module
            .get_setting(name)
            .and_then(ModuleSetting::get_slider_value)
            .unwrap_or(fallback)
    }

     
     
    fn cooling_down(&self) -> bool {
        let now = Instant::now();
        let mut until = self
            .cooldown_until
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        match *until {
            Some(deadline) if now < deadline => true,
            Some(_) => {
                *until = None;
                false
            }
            None => false,
        }
    }

    fn health_threshold(&self) -> f32 {
        self.module
            .get_setting("Health")
            .and_then(ModuleSetting::get_slider_value)
            .unwrap_or(10.0)
            .clamp(1.0, 20.0)
    }

    fn choice(&self) -> GappleChoice {
        match self.module.get_setting("Item") {
            Some(ModuleSetting::Choice { value, .. }) => GappleChoice::from_choice(*value),
            _ => GappleChoice::Any,
        }
    }

     
    fn best_apple(
        inventory: &Inventory,
        selected: i32,
        choice: GappleChoice,
    ) -> anyhow::Result<Option<i32>> {
        for slot in gapple_scan_order(selected) {
            let stack = inventory.get_item(slot)?;
            if stack.is_empty()? {
                continue;
            }
            if choice.matches(&stack.get_item()?.registry_name()?) {
                return Ok(Some(slot));
            }
        }
        Ok(None)
    }

     
     
     
    fn finish(&self) -> anyhow::Result<()> {
        let session = self.session.lock().unwrap().take();
        if session.is_some() {
            let seconds = self.slider("Cooldown", 8.0).clamp(0.0, 20.0);
            *self
                .cooldown_until
                .lock()
                .unwrap_or_else(|p| p.into_inner()) =
                Some(Instant::now() + Duration::from_secs_f32(seconds));
        }
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
                                session.apple_slot,
                                &session.player,
                            ) {
                                errors.push(error);
                            }
                        }
                    }
                    Ok(_) => {
                        if session.swapped_from.is_some() {
                            errors.push(anyhow::anyhow!(
                                "AutoGapple cannot restore a swap while another container is open"
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
                anyhow::bail!("AutoGapple cleanup failed: {errors:?}")
            }
        })();
        result
    }

     
     
    fn session_finished(session: &EatSession, player: &LocalPlayer) -> anyhow::Result<bool> {
        let same_player = player.is_same(&session.player);
        let stack = session.inventory.get_item(session.apple_slot)?;
        Ok(!same_player
            || session.started_at.elapsed() >= EAT_TIMEOUT
            || session.inventory.get_selected()? != session.apple_slot
            || player.get_health()? > session.health_before
            || stack.count()? < session.count
            || !stack.get_item()?.is_same(&session.apple)
            || !player.is_using_item()?)
    }

    fn tick(&self) -> anyhow::Result<()> {
        if !self.module.enabled || !crate::state::client().modules.is_active() {
            return self.finish();
        }
        if crate::graphic::input::GUI_OPEN.load(std::sync::atomic::Ordering::Relaxed)
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

        let active = {
            let guard = self.session.lock().unwrap();
            match guard.as_ref() {
                Some(session) => Some(Self::session_finished(session, &player)?),
                None => None,
            }
        };
        if let Some(finished) = active {
            if finished {
                return self.finish();
            }
             
             
            if let Some(session) = self.session.lock().unwrap().as_ref() {
                session.use_key.set_down(true)?;
            }
            return Ok(());
        }

        if !needs_heal(player.get_health()?, self.health_threshold())
            || player.is_using_item()?
            || crate::module::world::autoeat::is_eating()
            || self.cooling_down()
        {
            return Ok(());
        }
        if player.container_menu()?.container_id()? != 0 {
            return Ok(());
        }

        let inventory = player.get_inventory()?;
        let original_slot = inventory.get_selected()?;
        let Some(source) = Self::best_apple(&inventory, original_slot, self.choice())? else {
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
        let apple = stack.get_item()?;
        let count = stack.count()?;
        let apple_slot = if source < 9 { source } else { original_slot };
        *self.session.lock().unwrap() = Some(EatSession {
            player: player.clone(),
            inventory,
            game_mode,
            use_key,
            original_slot,
            apple_slot,
            swapped_from: None,
            original_use_down,
            started_use: false,
            apple,
            count,
            health_before: player.get_health()?,
            started_at: Instant::now(),
        });

        let mut guard = self.session.lock().unwrap();
        let session = guard.as_mut().expect("the session was just installed");
        if source >= 9 {
            session.game_mode.container_swap(
                0,
                Inventory::menu_slot(source),
                apple_slot,
                &player,
            )?;
            session.swapped_from = Some(source);
        }
        session.inventory.set_selected(apple_slot)?;
        session.use_key.set_down(true)?;
        session.started_use = true;
        if session.game_mode.use_item(&player)? {
            return Ok(());
        }
        drop(guard);
        self.finish()
    }
}

impl Module for AutoGappleModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.finish()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_module_ships_off_with_the_documented_settings() {
        let module = AutoGappleModule::new();
        assert_eq!(module.module.id, ModuleId::AutoGapple);
        assert_eq!(module.module.category, ModuleCategory::Player);
        assert!(!module.module.enabled);

        let names: Vec<&str> = module
            .module
            .settings
            .iter()
            .map(ModuleSetting::name)
            .collect();
        assert_eq!(names, vec!["Health", "Item", "Cooldown"]);
        assert_eq!(module.health_threshold(), 10.0);
        assert_eq!(module.choice(), GappleChoice::Any);
        assert_eq!(module.slider("Cooldown", 0.0), 8.0);
    }

    #[test]
    fn the_item_dropdown_covers_every_choice_in_order() {
        let labels: Vec<&str> = GappleChoice::ALL
            .iter()
            .map(|choice| choice.label())
            .collect();
        assert_eq!(labels, vec!["Any", "Golden", "Enchanted"]);
        for (index, choice) in GappleChoice::ALL.iter().enumerate() {
            assert_eq!(GappleChoice::from_choice(index), *choice);
        }
         
        assert_eq!(GappleChoice::from_choice(99), GappleChoice::Any);
    }

    #[test]
    fn each_choice_matches_the_right_apple() {
        assert!(GappleChoice::Any.matches("minecraft:golden_apple"));
        assert!(GappleChoice::Any.matches("minecraft:enchanted_golden_apple"));
        assert!(!GappleChoice::Any.matches("minecraft:apple"));

        assert!(GappleChoice::Golden.matches("minecraft:golden_apple"));
        assert!(!GappleChoice::Golden.matches("minecraft:enchanted_golden_apple"));

        assert!(GappleChoice::Enchanted.matches("minecraft:enchanted_golden_apple"));
        assert!(!GappleChoice::Enchanted.matches("minecraft:golden_apple"));

         
        assert!(GappleChoice::Golden.matches("golden_apple"));
    }

    #[test]
    fn the_health_threshold_is_inclusive_but_never_fires_at_death() {
        assert!(needs_heal(10.0, 10.0));
        assert!(needs_heal(1.0, 10.0));
        assert!(!needs_heal(10.5, 10.0));
        assert!(!needs_heal(0.0, 10.0), "a dead player must not eat");
         
        let mut module = AutoGappleModule::new();
        module
            .module
            .get_setting_mut("Health")
            .expect("Health slider")
            .set_slider_value(99.0);
        assert_eq!(module.health_threshold(), 20.0);
    }

    #[test]
    fn the_scan_prefers_the_held_slot_and_covers_the_whole_inventory() {
        let order: Vec<i32> = gapple_scan_order(5).collect();
        assert_eq!(&order[..3], &[5, 0, 1]);
        assert_eq!(order.len(), 36);
        let mut sorted = order;
        sorted.sort_unstable();
        assert_eq!(sorted, (0..36).collect::<Vec<_>>());
    }

    #[test]
    fn the_eat_timeout_is_shorter_than_a_bad_session_would_look() {
        assert!(EAT_TIMEOUT >= Duration::from_secs(2));
        assert!(EAT_TIMEOUT <= Duration::from_secs(10));
    }

    #[test]
    fn the_post_eat_cooldown_expires() {
        let module = AutoGappleModule::new();
        assert!(!module.cooling_down(), "a fresh module is ready to eat");

        *module.cooldown_until.lock().unwrap() = Some(Instant::now() - Duration::from_millis(1));
        assert!(
            !module.cooling_down(),
            "an expired deadline must clear itself"
        );
        assert!(module.cooldown_until.lock().unwrap().is_none());

        *module.cooldown_until.lock().unwrap() = Some(Instant::now() + Duration::from_secs(4));
        assert!(module.cooling_down());
    }
}
