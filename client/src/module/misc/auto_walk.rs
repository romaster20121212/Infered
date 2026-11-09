 
 
 
 
 
 
 
 

use crate::graphic::input;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Forward,
    Backward,
    Left,
    Right,
}

impl Direction {
    const ALL: [Direction; 4] = [
        Direction::Forward,
        Direction::Backward,
        Direction::Left,
        Direction::Right,
    ];

    fn from_choice(index: usize) -> Direction {
        Self::ALL.get(index).copied().unwrap_or(Direction::Forward)
    }

    fn label(self) -> &'static str {
        match self {
            Direction::Forward => "Forward",
            Direction::Backward => "Backward",
            Direction::Left => "Left",
            Direction::Right => "Right",
        }
    }
}

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeldKey {
    Forward,
    Back,
    Left,
    Right,
    Sprint,
}

const ALL_KEYS: [HeldKey; 5] = [
    HeldKey::Forward,
    HeldKey::Back,
    HeldKey::Left,
    HeldKey::Right,
    HeldKey::Sprint,
];

 
fn direction_key(direction: Direction) -> HeldKey {
    match direction {
        Direction::Forward => HeldKey::Forward,
        Direction::Backward => HeldKey::Back,
        Direction::Left => HeldKey::Left,
        Direction::Right => HeldKey::Right,
    }
}

 
fn should_hold(pause_in_menus: bool, menu_open: bool) -> bool {
    !(pause_in_menus && menu_open)
}

 
fn set_key(key: HeldKey, down: bool) -> anyhow::Result<()> {
    let options = minecraft().options()?;
    match key {
        HeldKey::Forward => options.key_up()?.set_down(down),
        HeldKey::Back => options.key_back()?.set_down(down),
        HeldKey::Left => options.key_left()?.set_down(down),
        HeldKey::Right => options.key_right()?.set_down(down),
        HeldKey::Sprint => options.key_sprint()?.set_down(down),
    }
}

 
 
 
fn key_is_down(key: HeldKey) -> anyhow::Result<bool> {
    let options = minecraft().options()?;
    match key {
        HeldKey::Forward => options.key_up()?.is_down(),
        HeldKey::Back => options.key_back()?.is_down(),
        HeldKey::Left => options.key_left()?.is_down(),
        HeldKey::Right => options.key_right()?.is_down(),
        HeldKey::Sprint => options.key_sprint()?.is_down(),
    }
}

#[derive(Debug)]
pub struct AutoWalkModule {
    pub module: ModuleData,
     
     
    held: Mutex<Vec<HeldKey>>,
}

impl AutoWalkModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoWalk,
                description: "Keeps walking in the chosen direction without holding a key, \
                              with optional sprint. Pauses while a screen is open."
                    .to_string(),
                category: ModuleCategory::Misc,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Choice {
                        name: "Direction".to_string(),
                        value: 0,
                        options: Direction::ALL
                            .iter()
                            .map(|direction| direction.label().to_string())
                            .collect(),
                    },
                    ModuleSetting::Toggle {
                        name: "Sprint".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Pause In Menus".to_string(),
                        value: true,
                    },
                ],
            },
            held: Mutex::new(Vec::new()),
        }
    }

    fn direction(&self) -> Direction {
        match self.module.get_setting("Direction") {
            Some(ModuleSetting::Choice { value, .. }) => Direction::from_choice(*value),
            _ => Direction::Forward,
        }
    }

    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(fallback)
    }

     
     
    fn hold(&self, direction: Direction, sprint: bool) -> anyhow::Result<()> {
        let mut wanted = vec![direction_key(direction)];
        if sprint && direction == Direction::Forward {
            wanted.push(HeldKey::Sprint);
        }
        let mut held = self.held.lock().unwrap_or_else(|p| p.into_inner());
        for key in held.iter().copied().filter(|key| !wanted.contains(key)) {
            let _ = set_key(key, false);
        }
        for key in wanted.iter().copied() {
            if !held.contains(&key) || !key_is_down(key).unwrap_or(false) {
                set_key(key, true)?;
            }
        }
        *held = wanted;
        Ok(())
    }

     
    fn release(&self) {
        let mut held = self.held.lock().unwrap_or_else(|p| p.into_inner());
        for key in held.drain(..) {
            let _ = set_key(key, false);
        }
    }
}

impl Module for AutoWalkModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.release();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.release();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let pause = self.toggle("Pause In Menus", true);
        if !should_hold(pause, input::menu_open()) {
            self.release();
            return Ok(());
        }
        self.hold(self.direction(), self.toggle("Sprint", true))
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
    fn ships_armed_off_with_the_documented_settings() {
        let module = AutoWalkModule::new();
        assert_eq!(module.module.id, ModuleId::AutoWalk);
        assert_eq!(module.module.category, ModuleCategory::Misc);
        assert!(!module.module.enabled);
        let names: Vec<&str> = module
            .module
            .settings
            .iter()
            .map(|setting| setting.name())
            .collect();
        assert_eq!(names, vec!["Direction", "Sprint", "Pause In Menus"]);
        assert_eq!(module.direction(), Direction::Forward);
        assert!(module.toggle("Sprint", false));
        assert!(module.toggle("Pause In Menus", false));
        assert!(module.held.lock().unwrap().is_empty());
    }

    #[test]
    fn every_direction_maps_to_its_own_key() {
        assert_eq!(direction_key(Direction::Forward), HeldKey::Forward);
        assert_eq!(direction_key(Direction::Backward), HeldKey::Back);
        assert_eq!(direction_key(Direction::Left), HeldKey::Left);
        assert_eq!(direction_key(Direction::Right), HeldKey::Right);
        let labels: Vec<&str> = Direction::ALL.iter().map(|d| d.label()).collect();
        assert_eq!(labels, vec!["Forward", "Backward", "Left", "Right"]);
        assert_eq!(Direction::from_choice(99), Direction::Forward);
    }

    #[test]
    fn a_menu_pauses_the_walk_only_when_the_setting_is_on() {
        assert!(!should_hold(true, true), "paused while a screen is open");
        assert!(should_hold(true, false));
        assert!(should_hold(false, true), "menus are allowed through");
    }

    #[test]
    fn only_a_forward_walk_asks_for_sprint() {
         
         
        let wanted = |direction: Direction, sprint: bool| {
            let mut keys = vec![direction_key(direction)];
            if sprint && direction == Direction::Forward {
                keys.push(HeldKey::Sprint);
            }
            keys
        };
        assert_eq!(
            wanted(Direction::Forward, true),
            vec![HeldKey::Forward, HeldKey::Sprint]
        );
        assert_eq!(wanted(Direction::Forward, false), vec![HeldKey::Forward]);
        assert_eq!(wanted(Direction::Left, true), vec![HeldKey::Left]);
        assert!(ALL_KEYS.contains(&HeldKey::Sprint));
    }
}
