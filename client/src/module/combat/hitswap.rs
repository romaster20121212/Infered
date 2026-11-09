 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, minecraft};
use std::sync::Mutex;

 
const MACE_SCORE: i32 = 120;

 
const AXE_TIERS: &[(&str, i32)] = &[
    ("netherite_axe", 100),
    ("diamond_axe", 90),
    ("iron_axe", 80),
    ("stone_axe", 70),
    ("golden_axe", 60),
    ("wooden_axe", 50),
];

 
const SWORD_TIERS: &[(&str, i32)] = &[
    ("netherite_sword", 95),
    ("diamond_sword", 85),
    ("iron_sword", 75),
    ("stone_sword", 65),
    ("golden_sword", 55),
    ("wooden_sword", 45),
];

 
#[derive(Debug, Clone, Copy)]
struct Swap {
    previous: i32,
    weapon: i32,
}

#[derive(Debug, Default)]
struct State {
    swap: Option<Swap>,
     
    restore_at: Option<i32>,
    was_down: bool,
}

#[derive(Debug)]
pub struct HitSwapModule {
    pub module: ModuleData,
    state: Mutex<State>,
}

impl HitSwapModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::HitSwap,
                description: "Equips a mace, axe or sword for the click, then restores the slot"
                    .to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Toggle {
                        name: "Maces".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Axes".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Swords".to_string(),
                        value: false,
                    },
                    ModuleSetting::Toggle {
                        name: "Only On Entity".to_string(),
                        value: true,
                    },
                ],
            },
            state: Mutex::new(State::default()),
        }
    }

    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(fallback)
    }

     
     
    fn restore_if_ours(player: &crate::mapping::entity::player::LocalPlayer, state: &mut State) {
        let Some(swap) = state.swap.take() else {
            return;
        };
        state.restore_at = None;
        let Ok(inventory) = player.get_inventory() else {
            return;
        };
        if inventory.get_selected().ok() == Some(swap.weapon) {
            let _ = inventory.set_selected(swap.previous);
        }
    }

    fn step(&self) -> anyhow::Result<()> {
        let mc = minecraft();
        let Some(player) = mc.player()? else {
            *self.state.lock().unwrap() = State::default();
            return Ok(());
        };
        let tick = player.get_tick_count()?;
        let key_down = mc.options()?.key_attack()?.is_down()?;
        let rising = {
            let mut state = self.state.lock().unwrap();
            let rising = key_down && !state.was_down;
            state.was_down = key_down;
            rising
        };

         
         
        if mc.current_screen().is_open()
            || !player.is_alive()?
            || player.container_menu()?.container_id()? != 0
            || player.is_using_item()?
        {
            return Ok(());
        }

        {
            let mut state = self.state.lock().unwrap();
            if state.restore_at.is_some_and(|at| tick > at) {
                Self::restore_if_ours(&player, &mut state);
            }
        }
        if !rising {
            return Ok(());
        }
        if self.toggle("Only On Entity", true) && mc.crosshair_entity()?.is_none() {
            return Ok(());
        }
        if busy(ModuleId::AutoWeapon) {
            return Ok(());
        }

         
         
         
        let maces = self.toggle("Maces", true);
        let axes = self.toggle("Axes", true);
        let swords = self.toggle("Swords", false);
        let inventory = player.get_inventory()?;
        let selected = inventory.get_selected()?;
        let state = self.state.lock().unwrap();
        if state.swap.is_some_and(|swap| swap.weapon == selected) {
            return Ok(());
        }
        let held_score = inventory
            .get_item(selected)
            .ok()
            .filter(|stack| !stack.is_empty().unwrap_or(true))
            .and_then(|stack| stack.get_item().ok())
            .and_then(|item| item.registry_name().ok())
            .map(|name| weapon_score(&name, maces, axes, swords))
            .unwrap_or(0);
        let mut items = Vec::new();
        for slot in 0..9 {
            let stack = inventory.get_item(slot)?;
            if stack.is_empty()? {
                continue;
            }
            items.push((slot, stack.get_item()?.registry_name()?));
        }
        let Some((weapon, score)) = best_weapon(
            items.iter().map(|(slot, name)| (*slot, name.as_str())),
            maces,
            axes,
            swords,
        ) else {
            return Ok(());
        };
         
         
        if weapon == selected || score <= held_score {
            return Ok(());
        }

        let mut state = state;
        let previous = match state.swap {
            Some(swap) if selected == swap.weapon => swap.previous,
            _ => selected,
        };
        state.swap = Some(Swap { previous, weapon });
        state.restore_at = Some(tick);
        inventory.set_selected(weapon)?;
        Ok(())
    }
}

impl Default for HitSwapModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for HitSwapModule {
    fn on_start(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            crate::jvmti::transform_installed("ScaffoldInputPhase"),
            "HitSwap requires Minecraft 1.21.4 input-phase hook"
        );
        *self.state.lock().unwrap() = State::default();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        let mut state = self.state.lock().unwrap();
        if let Ok(Some(player)) = minecraft().player() {
            Self::restore_if_ours(&player, &mut state);
        }
        *state = State::default();
        Ok(())
    }

    fn on_input_phase(&self) -> anyhow::Result<()> {
        self.step()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

 
fn busy(id: ModuleId) -> bool {
    client().modules.get(id).is_some_and(|handle| {
        handle
            .try_lock()
            .map(|module| module.get_module_data().enabled)
            .unwrap_or(true)
    })
}

fn tier(table: &[(&str, i32)], registry_name: &str) -> i32 {
    table
        .iter()
        .find(|(suffix, _)| registry_name.ends_with(suffix))
        .map(|(_, score)| *score)
        .unwrap_or(0)
}

 
pub(crate) fn weapon_score(registry_name: &str, maces: bool, axes: bool, swords: bool) -> i32 {
    if maces && registry_name.ends_with("mace") {
        return MACE_SCORE;
    }
    if axes && registry_name.ends_with("_axe") {
        return tier(AXE_TIERS, registry_name);
    }
    if swords && registry_name.ends_with("_sword") {
        return tier(SWORD_TIERS, registry_name);
    }
    0
}

 
pub(crate) fn best_weapon<'a>(
    items: impl IntoIterator<Item = (i32, &'a str)>,
    maces: bool,
    axes: bool,
    swords: bool,
) -> Option<(i32, i32)> {
    let mut best: Option<(i32, i32)> = None;
    for (slot, name) in items {
        let score = weapon_score(name, maces, axes, swords);
        if score > 0 && best.map_or(true, |(_, best_score)| score > best_score) {
            best = Some((slot, score));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_enabled_weapon_kinds_score() {
        assert!(weapon_score("minecraft:mace", true, false, false) > 0);
        assert_eq!(weapon_score("minecraft:mace", false, true, true), 0);
        assert!(weapon_score("minecraft:diamond_axe", false, true, false) > 0);
        assert_eq!(weapon_score("minecraft:diamond_axe", true, false, false), 0);
        assert!(weapon_score("minecraft:diamond_sword", false, false, true) > 0);
        assert_eq!(
            weapon_score("minecraft:diamond_sword", true, true, false),
            0
        );
    }

    #[test]
    fn a_mace_outranks_every_axe_and_the_axe_outranks_its_sword() {
        let mace = weapon_score("minecraft:mace", true, true, true);
        let axe = weapon_score("minecraft:netherite_axe", true, true, true);
        let sword = weapon_score("minecraft:netherite_sword", true, true, true);
        assert!(mace > axe);
        assert!(axe > sword);
    }

    #[test]
    fn swords_rank_by_tier() {
        assert!(
            weapon_score("minecraft:netherite_sword", true, true, true)
                > weapon_score("minecraft:iron_sword", true, true, true)
        );
        assert!(
            weapon_score("minecraft:diamond_sword", true, true, true)
                > weapon_score("minecraft:wooden_sword", true, true, true)
        );
    }

    #[test]
    fn a_pickaxe_is_not_an_axe_and_tools_are_not_weapons() {
        assert_eq!(
            weapon_score("minecraft:diamond_pickaxe", true, true, true),
            0
        );
        assert_eq!(weapon_score("minecraft:shield", true, true, true), 0);
        assert_eq!(weapon_score("minecraft:bow", true, true, true), 0);
        assert_eq!(weapon_score("minecraft:axe", true, true, true), 0);
    }

    #[test]
    fn the_best_enabled_weapon_wins_the_slot() {
        let items = [
            (0, "minecraft:diamond_sword"),
            (3, "minecraft:iron_axe"),
            (5, "minecraft:mace"),
        ];
        assert_eq!(best_weapon(items, true, true, true), Some((5, MACE_SCORE)));
        assert_eq!(
            best_weapon(items, false, true, true),
            Some((3, weapon_score("minecraft:iron_axe", true, true, true)))
        );
        assert_eq!(
            best_weapon(items, false, false, true),
            Some((0, weapon_score("minecraft:diamond_sword", true, true, true)))
        );
        assert_eq!(best_weapon(items, false, false, false), None);
    }

    #[test]
    fn module_identity_and_defaults() {
        let module = HitSwapModule::new();
        assert_eq!(module.module.id, ModuleId::HitSwap);
        assert_eq!(module.module.id.display_name(), "HitSwap");
        assert_eq!(module.module.category, ModuleCategory::Combat);
        assert!(!module.module.enabled);
        assert!(module.toggle("Maces", false));
        assert!(module.toggle("Axes", false));
        assert!(!module.toggle("Swords", true));
        assert!(module.toggle("Only On Entity", false));
    }
}
