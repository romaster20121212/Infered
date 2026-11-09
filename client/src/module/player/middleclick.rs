 
 

use crate::graphic::input::{
    clear_middle_clicks, request_gui_toggle, request_module_toggle, take_middle_click, MiddleClick,
    GUI_OPEN,
};
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::EntityRef;
use crate::mapping::inventory::Inventory;
use crate::module::player::refill::refill_hotbar;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, minecraft};
use std::collections::BTreeSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const ACTION_NAMES: [&str; 13] = [
    "None",
    "Pick Block",
    "Throw Pearl",
    "Golden Apple",
    "Firework",
    "Chorus Fruit",
    "Refill",
    "Open GUI",
    "Toggle Freecam",
    "Add Friend",
    "Add Enemy",
    "Drop Item",
    "Custom Command",
];

const ENDER_PEARL: &[&str] = &["ender_pearl"];
const GOLDEN_APPLE: &[&str] = &["golden_apple", "enchanted_golden_apple"];
const FIREWORK: &[&str] = &["firework_rocket"];
const CHORUS_FRUIT: &[&str] = &["chorus_fruit"];

 
 
 
static ENEMIES: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiddleAction {
    None,
    PickBlock,
    ThrowPearl,
    GoldenApple,
    Firework,
    ChorusFruit,
    Refill,
    OpenGui,
    ToggleFreecam,
    AddFriend,
    AddEnemy,
    DropItem,
    CustomCommand,
}

const ALL_ACTIONS: [MiddleAction; 13] = [
    MiddleAction::None,
    MiddleAction::PickBlock,
    MiddleAction::ThrowPearl,
    MiddleAction::GoldenApple,
    MiddleAction::Firework,
    MiddleAction::ChorusFruit,
    MiddleAction::Refill,
    MiddleAction::OpenGui,
    MiddleAction::ToggleFreecam,
    MiddleAction::AddFriend,
    MiddleAction::AddEnemy,
    MiddleAction::DropItem,
    MiddleAction::CustomCommand,
];

impl MiddleAction {
    pub fn from_index(index: usize) -> Self {
        ALL_ACTIONS.get(index).copied().unwrap_or(Self::None)
    }

    pub fn index(self) -> usize {
        ALL_ACTIONS
            .iter()
            .position(|action| *action == self)
            .unwrap_or(0)
    }
}

 
 
pub fn pick_action(
    ctrl: bool,
    shift: bool,
    primary: MiddleAction,
    shift_action: MiddleAction,
    ctrl_action: MiddleAction,
) -> MiddleAction {
    if ctrl {
        ctrl_action
    } else if shift {
        shift_action
    } else {
        primary
    }
}

 
 
fn is_item_named(registry_name: &str, names: &[&str]) -> bool {
    let short = registry_name
        .strip_prefix("minecraft:")
        .unwrap_or(registry_name);
    names.contains(&short)
}

 
fn cooldown_ready(last: Option<Instant>, delay: Duration, now: Instant) -> bool {
    !matches!(last, Some(last) if now.saturating_duration_since(last) < delay)
}

 
#[derive(Debug, Clone)]
struct MiddleClickConfig {
    primary: MiddleAction,
    shift: MiddleAction,
    ctrl: MiddleAction,
    delay: Duration,
    cancel_vanilla: bool,
    swap_from_inventory: bool,
    command: String,
}

impl MiddleClickConfig {
    fn from_module(data: &ModuleData) -> Self {
        let action = |name: &str, default: usize| {
            let index = match data.get_setting(name) {
                Some(ModuleSetting::Choice { value, .. }) => *value,
                _ => default,
            };
            MiddleAction::from_index(index)
        };
        let toggle = |name: &str| {
            data.get_setting(name)
                .and_then(|setting| setting.get_toggle_value())
                .unwrap_or(true)
        };
        let delay_ms = data
            .get_setting("Delay (ms)")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(100.0)
            .max(0.0);
        let command = data
            .get_setting("Custom Command")
            .and_then(|setting| setting.get_text_value())
            .unwrap_or("/home")
            .to_string();

        Self {
            primary: action("Action", 2),
            shift: action("Shift Action", 3),
            ctrl: action("Ctrl Action", 0),
            delay: Duration::from_millis(delay_ms as u64),
            cancel_vanilla: toggle("Cancel Vanilla"),
            swap_from_inventory: toggle("Swap From Inventory"),
            command,
        }
    }

     
    fn active_action(&self, click: MiddleClick) -> MiddleAction {
        pick_action(click.ctrl, click.shift, self.primary, self.shift, self.ctrl)
    }
}

 
 
 
pub fn swallows_vanilla_middle_click(mods: i32) -> bool {
    let Some(handle) = client().modules.get(ModuleId::MiddleClick) else {
        return false;
    };
    let Ok(module) = handle.lock() else {
        return false;
    };
    let data = module.get_module_data();
    if !data.enabled {
        return false;
    }
    let config = MiddleClickConfig::from_module(data);
    let click = MiddleClick::from_mods(mods);
    config.cancel_vanilla && config.active_action(click) != MiddleAction::None
}

#[derive(Debug, Default)]
struct MiddleClickState {
    last_action: Option<Instant>,
}

 
fn find_slot(
    inventory: &Inventory,
    slots: std::ops::Range<i32>,
    names: &[&str],
) -> anyhow::Result<Option<i32>> {
    for slot in slots {
        let stack = inventory.get_item(slot)?;
        if stack.is_empty()? {
            continue;
        }
        let name = stack.get_item()?.registry_name()?;
        if is_item_named(&name, names) {
            return Ok(Some(slot));
        }
    }
    Ok(None)
}

 
 
 
 
 
 
 
 
fn use_held_item(player: &LocalPlayer) -> anyhow::Result<bool> {
    if let Some(game_mode) = minecraft().game_mode()? {
        if game_mode.use_item(player)? {
            return Ok(true);
        }
    }
    Ok(minecraft().start_use_item().is_ok())
}

 
 
 
fn use_item(
    player: &LocalPlayer,
    names: &[&str],
    swap_from_inventory: bool,
) -> anyhow::Result<bool> {
    let inventory = player.get_inventory()?;

    let hotbar = match find_slot(&inventory, 0..9, names)? {
        Some(slot) => slot,
        None => {
            if !swap_from_inventory {
                log::debug!("MiddleClick: item not in hotbar and swap is off");
                return Ok(false);
            }
            let Some(source) = find_slot(&inventory, 9..36, names)? else {
                log::debug!("MiddleClick: item not found in inventory");
                return Ok(false);
            };
             
             
            if !crate::module::combat::interaction::window_click_allowed(player) {
                return Ok(false);
            }
            let Some(game_mode) = minecraft().game_mode()? else {
                return Ok(false);
            };
            let target = inventory.get_selected()?;
            if game_mode
                .container_swap(0, Inventory::menu_slot(source), target, player)
                .is_err()
            {
                return Ok(false);
            }
            target
        }
    };

    inventory.set_selected(hotbar.clamp(0, 8))?;
    use_held_item(player)
}

 
 
 
fn command_to_run(text: &str) -> Option<&str> {
    let command = text.trim().strip_prefix('/')?.trim();
    (!command.is_empty()).then_some(command)
}

 
fn crosshair_entity_name() -> anyhow::Result<Option<String>> {
    let Some(entity) = minecraft().crosshair_entity()? else {
        return Ok(None);
    };
    Ok(Some(entity.get_name()?.get_string()?))
}

 
fn record_crosshair_entity(list: &Mutex<BTreeSet<String>>) -> anyhow::Result<bool> {
    let Some(name) = crosshair_entity_name()? else {
        return Ok(false);
    };
    if let Ok(mut set) = list.lock() {
        set.insert(name);
    }
    Ok(true)
}

 
fn record_crosshair_friend() -> anyhow::Result<bool> {
    let Some(name) = crosshair_entity_name()? else {
        return Ok(false);
    };
    if crate::friends::add(&name) {
        crate::config::save();
    }
    Ok(true)
}

 
 
fn execute(
    action: MiddleAction,
    player: &LocalPlayer,
    config: &MiddleClickConfig,
) -> anyhow::Result<bool> {
    match action {
        MiddleAction::None => Ok(false),
        MiddleAction::PickBlock => {
            minecraft().pick_block()?;
            Ok(true)
        }
        MiddleAction::ThrowPearl => use_item(player, ENDER_PEARL, config.swap_from_inventory),
        MiddleAction::GoldenApple => use_item(player, GOLDEN_APPLE, config.swap_from_inventory),
        MiddleAction::Firework => use_item(player, FIREWORK, config.swap_from_inventory),
        MiddleAction::ChorusFruit => use_item(player, CHORUS_FRUIT, config.swap_from_inventory),
        MiddleAction::Refill => refill_hotbar(player),
        MiddleAction::OpenGui => {
            request_gui_toggle();
            Ok(true)
        }
        MiddleAction::ToggleFreecam => {
            request_module_toggle(ModuleId::Freecam);
            Ok(true)
        }
        MiddleAction::AddFriend => record_crosshair_friend(),
        MiddleAction::AddEnemy => record_crosshair_entity(&ENEMIES),
        MiddleAction::DropItem => {
             
             
            player.drop_selected(true)
        }
        MiddleAction::CustomCommand => {
            let text = config.command.trim();
            if text.is_empty() {
                log::debug!("MiddleClick: custom command is empty");
                return Ok(false);
            }
            let result = match command_to_run(text) {
                Some(command) => minecraft().send_command(command),
                None => minecraft().send_chat(text),
            };
            if let Err(error) = result {
                log::debug!("MiddleClick: custom command failed: {error}");
                return Ok(false);
            }
            Ok(true)
        }
    }
}

 
 
#[derive(Debug)]
pub struct MiddleClickModule {
    pub module: ModuleData,
    state: Mutex<MiddleClickState>,
}

fn action_options() -> Vec<String> {
    ACTION_NAMES.iter().map(|name| name.to_string()).collect()
}

impl MiddleClickModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::MiddleClick,
                description: "Runs an action when the middle mouse button is pressed".to_string(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Choice {
                        name: "Action".to_string(),
                        value: 2,
                        options: action_options(),
                    },
                    ModuleSetting::Choice {
                        name: "Shift Action".to_string(),
                        value: 3,
                        options: action_options(),
                    },
                    ModuleSetting::Choice {
                        name: "Ctrl Action".to_string(),
                        value: 0,
                        options: action_options(),
                    },
                    ModuleSetting::Slider {
                        name: "Delay (ms)".to_string(),
                        value: 100.0,
                        min: 0.0,
                        max: 1000.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Cancel Vanilla".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Swap From Inventory".to_string(),
                        value: true,
                    },
                    ModuleSetting::Text {
                        name: "Custom Command".to_string(),
                        value: "/home".to_string(),
                    },
                ],
            },
            state: Mutex::new(MiddleClickState::default()),
        }
    }
}

impl Module for MiddleClickModule {
    fn on_start(&self) -> anyhow::Result<()> {
        *self.state.lock().unwrap() = MiddleClickState::default();
         
        clear_middle_clicks();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        *self.state.lock().unwrap() = MiddleClickState::default();
        clear_middle_clicks();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        if !client().modules.is_active() {
            return Ok(());
        }
         
         
        let Some(click) = take_middle_click() else {
            return Ok(());
        };
        if crate::module::world::autoeat::is_eating()
            || GUI_OPEN.load(std::sync::atomic::Ordering::Relaxed)
            || minecraft().current_screen().is_open()
        {
            return Ok(());
        }
        let Some(player) = minecraft().player().ok().flatten() else {
            return Ok(());
        };

        let config = MiddleClickConfig::from_module(&self.module);
        let now = Instant::now();
        if !cooldown_ready(self.state.lock().unwrap().last_action, config.delay, now) {
            return Ok(());
        }

         
         
         
         
        let action = config.active_action(click);
        match execute(action, &player, &config) {
            Ok(true) => self.state.lock().unwrap().last_action = Some(now),
            Ok(false) => {}
            Err(error) => log::debug!("MiddleClick: {action:?} failed: {error}"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_index_round_trips() {
        for (index, action) in ALL_ACTIONS.iter().enumerate() {
            assert_eq!(MiddleAction::from_index(index), *action);
            assert_eq!(action.index(), index);
        }
        assert_eq!(MiddleAction::from_index(99), MiddleAction::None);
    }

    #[test]
    fn item_matcher_accepts_prefixed_and_bare_names() {
        assert!(is_item_named("minecraft:ender_pearl", ENDER_PEARL));
        assert!(is_item_named("ender_pearl", ENDER_PEARL));
        assert!(is_item_named("minecraft:golden_apple", GOLDEN_APPLE));
        assert!(!is_item_named("minecraft:ender_chest", ENDER_PEARL));
        assert!(!is_item_named("minecraft:ender_pearl_fake", ENDER_PEARL));
    }

    #[test]
    fn ctrl_wins_over_shift_and_shift_over_primary() {
        let p = MiddleAction::ThrowPearl;
        let s = MiddleAction::GoldenApple;
        let c = MiddleAction::Refill;
        assert_eq!(pick_action(true, true, p, s, c), c);
        assert_eq!(pick_action(false, true, p, s, c), s);
        assert_eq!(pick_action(false, false, p, s, c), p);
    }

    #[test]
    fn modifiers_come_from_the_click_event() {
         
        assert_eq!(MiddleClick::from_mods(0), MiddleClick::default());
        assert!(MiddleClick::from_mods(0x1).shift);
        assert!(MiddleClick::from_mods(0x2).ctrl);
        assert_eq!(
            MiddleClick::from_mods(0x3),
            MiddleClick {
                ctrl: true,
                shift: true
            }
        );
         
         
        assert_eq!(
            pick_action(
                true,
                true,
                MiddleAction::ThrowPearl,
                MiddleAction::GoldenApple,
                MiddleAction::Refill
            ),
            MiddleAction::Refill
        );
    }

    #[test]
    fn a_slash_command_runs_as_a_command() {
        assert_eq!(command_to_run("/home"), Some("home"));
        assert_eq!(command_to_run("  /spawn  "), Some("spawn"));
         
        assert_eq!(command_to_run("hello"), None);
        assert_eq!(command_to_run("/"), None);
        assert_eq!(command_to_run("  "), None);
    }

    #[test]
    fn cooldown_blocks_until_delay_elapses() {
        let base = Instant::now();
        let delay = Duration::from_millis(100);
        assert!(cooldown_ready(None, delay, base));
        assert!(!cooldown_ready(
            Some(base),
            delay,
            base + Duration::from_millis(50)
        ));
        assert!(cooldown_ready(
            Some(base),
            delay,
            base + Duration::from_millis(100)
        ));
    }
}
