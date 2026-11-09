 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::client::world::World;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::{EntityRef, FishingHook, LivingEntityRef};
use crate::mapping::inventory::Inventory;
use crate::mapping::MappedObject;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
const FISHING_ROD: &str = "minecraft:fishing_rod";

 
const CAST_TIMEOUT: Duration = Duration::from_millis(2500);

 
 
const BITE_MOTION_Y: f64 = -0.02;

 
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Idle,
    Waiting,
    Reeling,
}

#[derive(Debug, Default)]
struct FishState {
    phase: Phase,
     
    cast_at: Option<Instant>,
     
    cast_ready: Option<Instant>,
     
    reel_at: Option<Instant>,
}

 
fn should_cast(phase: Phase, has_hook: bool, auto_cast: bool, cast_ready: bool) -> bool {
    phase == Phase::Idle && !has_hook && auto_cast && cast_ready
}

 
fn should_start_reeling(phase: Phase, has_hook: bool, biting: bool) -> bool {
    phase == Phase::Waiting && has_hook && biting
}

 
 
fn millis(value: f32, max_ms: f32) -> Duration {
    if !value.is_finite() {
        return Duration::ZERO;
    }
    Duration::from_millis(value.round().clamp(0.0, max_ms) as u64)
}

 
fn rod_hotbar_slot(inventory: &Inventory) -> anyhow::Result<Option<i32>> {
    for slot in 0..9 {
        let stack = inventory.get_item(slot)?;
        if stack.is_empty()? {
            continue;
        }
        if stack.get_item()?.registry_name()? == FISHING_ROD {
            return Ok(Some(slot));
        }
    }
    Ok(None)
}

 
fn find_own_hook(world: &World, player: &LocalPlayer) -> anyhow::Result<Option<FishingHook>> {
    for entity in world.get_entities()? {
        if !entity.instance_of::<FishingHook>() {
            continue;
        }
        let hook = FishingHook::new(entity.jni_ref().clone());
         
         
        if hook
            .player_owner()
            .ok()
            .flatten()
            .is_some_and(|owner| owner.is_same(player))
        {
            return Ok(Some(hook));
        }
    }
    Ok(None)
}

 
 
fn hook_is_biting(hook: &FishingHook) -> bool {
    match hook.is_biting() {
        Ok(biting) => biting,
        Err(_) => hook
            .get_delta_movement()
            .map(|motion| motion.y() < BITE_MOTION_Y)
            .unwrap_or(false),
    }
}

 
#[derive(Debug)]
pub struct AutoFishModule {
    pub module: ModuleData,
    state: Mutex<FishState>,
}

impl AutoFishModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoFish,
                description: "Casts and reels your fishing rod automatically, with adjustable \
                              reaction and recast delays"
                    .to_string(),
                category: ModuleCategory::World,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Toggle {
                        name: "Auto Cast".to_string(),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: "Reel Delay".to_string(),
                        value: 200.0,
                        min: 0.0,
                        max: 1000.0,
                    },
                    ModuleSetting::Slider {
                        name: "Cast Delay".to_string(),
                        value: 500.0,
                        min: 0.0,
                        max: 5000.0,
                    },
                ],
            },
            state: Mutex::new(FishState::default()),
        }
    }

    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(ModuleSetting::get_toggle_value)
            .unwrap_or(fallback)
    }

    fn slider(&self, name: &str, fallback: f32) -> f32 {
        self.module
            .get_setting(name)
            .and_then(ModuleSetting::get_slider_value)
            .unwrap_or(fallback)
    }

    fn reset(&self) {
        *self.state.lock().unwrap_or_else(|p| p.into_inner()) = FishState::default();
    }

    fn tick(&self) -> anyhow::Result<()> {
         
         
        if crate::graphic::input::GUI_OPEN.load(Ordering::Relaxed)
            || minecraft().current_screen().is_open()
        {
            return Ok(());
        }
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
        if !player.is_alive()? {
            return Ok(());
        }
        let Some(world) = minecraft().world()? else {
            return Ok(());
        };
         
         
        if player.container_menu()?.container_id()? != 0 {
            return Ok(());
        }

        let inventory = player.get_inventory()?;
        let Some(rod_slot) = rod_hotbar_slot(&inventory)? else {
            return Ok(());
        };
        if inventory.get_selected()? != rod_slot {
            inventory.set_selected(rod_slot)?;
        }
        let Some(game_mode) = minecraft().game_mode()? else {
            return Ok(());
        };

        let hook = find_own_hook(&world, &player)?;
        let has_hook = hook.is_some();
        let biting = hook.as_ref().is_some_and(hook_is_biting);

        let auto_cast = self.toggle("Auto Cast", true);
        let reel_delay = millis(self.slider("Reel Delay", 200.0), 1000.0);
        let cast_delay = millis(self.slider("Cast Delay", 500.0), 5000.0);

        let now = Instant::now();
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        match state.phase {
            Phase::Idle => {
                if has_hook {
                     
                     
                     
                    state.phase = Phase::Waiting;
                    state.cast_at = Some(now);
                } else if should_cast(
                    state.phase,
                    has_hook,
                    auto_cast,
                    state.cast_ready.is_none_or(|ready| now >= ready),
                ) {
                    game_mode.use_item(&player)?;
                    state.phase = Phase::Waiting;
                    state.cast_at = Some(now);
                    state.cast_ready = Some(now + cast_delay);
                }
            }
            Phase::Waiting => {
                if should_start_reeling(state.phase, has_hook, biting) {
                    state.phase = Phase::Reeling;
                    state.reel_at = Some(now + reel_delay);
                } else if !has_hook
                    && state
                        .cast_at
                        .is_some_and(|cast| now.duration_since(cast) >= CAST_TIMEOUT)
                {
                     
                    state.phase = Phase::Idle;
                    state.cast_at = None;
                    state.cast_ready = Some(now + cast_delay);
                }
            }
            Phase::Reeling => {
                if state.reel_at.is_some_and(|reel| now >= reel) {
                    game_mode.use_item(&player)?;
                    state.phase = Phase::Idle;
                    state.cast_at = None;
                    state.cast_ready = Some(now + cast_delay);
                    state.reel_at = None;
                }
            }
        }
        Ok(())
    }
}

impl Module for AutoFishModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.reset();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.reset();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        self.tick()
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
        let module = AutoFishModule::new();
        assert_eq!(module.module.id, ModuleId::AutoFish);
        assert_eq!(module.module.category, ModuleCategory::World);
        assert!(!module.module.enabled);

        let names: Vec<&str> = module
            .module
            .settings
            .iter()
            .map(ModuleSetting::name)
            .collect();
        assert_eq!(names, vec!["Auto Cast", "Reel Delay", "Cast Delay"]);
        assert!(module.toggle("Auto Cast", false));
        assert_eq!(module.slider("Reel Delay", 0.0), 200.0);
        assert_eq!(module.slider("Cast Delay", 0.0), 500.0);
    }

    #[test]
    fn a_fresh_module_starts_idle_with_no_deadlines() {
        let module = AutoFishModule::new();
        let state = module.state.lock().unwrap();
        assert_eq!(state.phase, Phase::Idle);
        assert!(state.cast_at.is_none());
        assert!(state.cast_ready.is_none());
        assert!(state.reel_at.is_none());
    }

    #[test]
    fn casting_needs_an_idle_rodless_player_and_the_auto_cast_toggle() {
        assert!(should_cast(Phase::Idle, false, true, true));
        assert!(
            !should_cast(Phase::Idle, true, true, true),
            "never cast while a hook is already out"
        );
        assert!(
            !should_cast(Phase::Idle, false, false, true),
            "Auto Cast off means manual casting only"
        );
        assert!(
            !should_cast(Phase::Idle, false, true, false),
            "the cast delay must elapse first"
        );
        assert!(!should_cast(Phase::Waiting, false, true, true));
        assert!(!should_cast(Phase::Reeling, false, true, true));
    }

    #[test]
    fn reeling_starts_only_from_waiting_on_a_real_bite() {
        assert!(should_start_reeling(Phase::Waiting, true, true));
        assert!(!should_start_reeling(Phase::Waiting, true, false));
        assert!(!should_start_reeling(Phase::Waiting, false, true));
        assert!(!should_start_reeling(Phase::Idle, true, true));
        assert!(!should_start_reeling(Phase::Reeling, true, true));
    }

    #[test]
    fn slider_values_become_safe_durations() {
        assert_eq!(millis(0.0, 1000.0), Duration::ZERO);
        assert_eq!(millis(250.4, 1000.0), Duration::from_millis(250));
        assert_eq!(millis(-5.0, 1000.0), Duration::ZERO);
        assert_eq!(millis(9_000.0, 1000.0), Duration::from_millis(1000));
        assert_eq!(millis(f32::NAN, 1000.0), Duration::ZERO);
        assert_eq!(millis(f32::INFINITY, 1000.0), Duration::ZERO);
    }

    #[test]
    fn stopping_resets_a_partial_cycle() {
        let module = AutoFishModule::new();
        {
            let mut state = module.state.lock().unwrap();
            state.phase = Phase::Reeling;
            state.reel_at = Some(Instant::now());
        }
        module.on_stop().unwrap();
        assert_eq!(module.state.lock().unwrap().phase, Phase::Idle);
    }
}
