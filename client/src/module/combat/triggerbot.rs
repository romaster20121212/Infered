use crate::mapping::entity::mob::Mob;
use crate::mapping::entity::player::{LocalPlayer, Player};
use crate::mapping::entity::{EndCrystal, Entity, EntityRef, LivingEntityRef, PlayerRef};
use crate::mapping::MappedObject;
use crate::module::combat::within_reach_up_to;
use crate::module::player::autototem::roll;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, minecraft};
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
const MIN_DELAY_MS: f32 = 0.0;
 
const MAX_DELAY_MS: f32 = 1000.0;
 
const MAX_REACTION_MS: f32 = 500.0;

 
 
const MAX_TRIGGER_REACH: f64 = 3.0;

 
const SPRINT_RESET_DELAY: Duration = Duration::from_millis(50);

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickButton {
    Left,
    Right,
    Middle,
    Both,
}

impl ClickButton {
    pub fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Right,
            2 => Self::Middle,
            3 => Self::Both,
            _ => Self::Left,
        }
    }
}

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponFilter {
    Any,
    WeaponsOnly,
    SwordOnly,
    AxeOnly,
}

impl WeaponFilter {
    pub fn from_index(index: usize) -> Self {
        match index {
            1 => Self::WeaponsOnly,
            2 => Self::SwordOnly,
            3 => Self::AxeOnly,
            _ => Self::Any,
        }
    }

     
     
    pub fn matches(self, registry_name: &str) -> bool {
        let short = registry_name
            .strip_prefix("minecraft:")
            .unwrap_or(registry_name);
        match self {
            Self::Any => true,
            Self::WeaponsOnly => {
                short.ends_with("_sword")
                    || short.ends_with("_axe")
                    || short == "trident"
                    || short == "mace"
            }
            Self::SwordOnly => short.ends_with("_sword"),
            Self::AxeOnly => short.ends_with("_axe"),
        }
    }
}

#[derive(Debug, Default)]
struct TriggerState {
     
    last_click: Option<Instant>,
     
    next_delay: Duration,
     
     
    acquired_target: Option<(i32, Instant)>,
     
    sprint_reset_at: Option<Instant>,
}

 
 
 
 
#[derive(Debug)]
pub struct TriggerBotModule {
    pub module: ModuleData,
    state: Mutex<TriggerState>,
}

impl TriggerBotModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::TriggerBot,
                description: "Automatically clicks when your crosshair is over a valid target"
                    .to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Choice {
                        name: "Click Button".to_string(),
                        value: 0,
                        options: vec![
                            "Left".to_string(),
                            "Right".to_string(),
                            "Middle".to_string(),
                            "Both".to_string(),
                        ],
                    },
                    ModuleSetting::Slider {
                        name: "Delay (ms)".to_string(),
                        value: 90.0,
                        min: MIN_DELAY_MS,
                        max: MAX_DELAY_MS,
                    },
                    ModuleSetting::Toggle {
                        name: "Random Delay".to_string(),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: "Min Delay (ms)".to_string(),
                        value: 70.0,
                        min: MIN_DELAY_MS,
                        max: MAX_DELAY_MS,
                    },
                    ModuleSetting::Slider {
                        name: "Max Delay (ms)".to_string(),
                        value: 120.0,
                        min: MIN_DELAY_MS,
                        max: MAX_DELAY_MS,
                    },
                    ModuleSetting::Slider {
                        name: "Reaction Delay (ms)".to_string(),
                        value: 15.0,
                        min: 0.0,
                        max: MAX_REACTION_MS,
                    },
                    ModuleSetting::Slider {
                        name: "Hit Chance (%)".to_string(),
                        value: 100.0,
                        min: 1.0,
                        max: 100.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Cooldown".to_string(),
                        value: true,
                    },
                    ModuleSetting::Slider {
                        name: "Cooldown Threshold".to_string(),
                        value: 0.95,
                        min: 0.1,
                        max: 1.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Players".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Mobs".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Crystals".to_string(),
                        value: false,
                    },
                    ModuleSetting::Toggle {
                        name: "Ignore Invisible".to_string(),
                        value: false,
                    },
                    ModuleSetting::Toggle {
                        name: "Shield Check".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "HurtTime Check".to_string(),
                        value: false,
                    },
                    ModuleSetting::Slider {
                        name: "Max HurtTime".to_string(),
                        value: 3.0,
                        min: 0.0,
                        max: 10.0,
                    },
                    ModuleSetting::Choice {
                        name: "Weapon Filter".to_string(),
                        value: 0,
                        options: vec![
                            "Any".to_string(),
                            "Weapons Only".to_string(),
                            "Sword Only".to_string(),
                            "Axe Only".to_string(),
                        ],
                    },
                    ModuleSetting::Toggle {
                        name: "Criticals Only".to_string(),
                        value: false,
                    },
                    ModuleSetting::Toggle {
                        name: "Require Mouse Hold".to_string(),
                        value: false,
                    },
                    ModuleSetting::Toggle {
                        name: "Pause While Using Item".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Sprint Reset".to_string(),
                        value: false,
                    },
                    ModuleSetting::Toggle {
                        name: "Actual Mouse".to_string(),
                        value: false,
                    },
                ],
            },
            state: Mutex::new(TriggerState::default()),
        }
    }

    fn slider(&self, name: &str, fallback: f32) -> f32 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(fallback)
    }

    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(fallback)
    }

    fn choice(&self, name: &str, fallback: usize) -> usize {
        match self.module.get_setting(name) {
            Some(ModuleSetting::Choice { value, .. }) => *value,
            _ => fallback,
        }
    }

    pub fn click_button(&self) -> ClickButton {
        ClickButton::from_index(self.choice("Click Button", 0))
    }

    pub fn weapon_filter(&self) -> WeaponFilter {
        WeaponFilter::from_index(self.choice("Weapon Filter", 0))
    }

     
     
    pub fn pick_interval(&self) -> Duration {
        compute_click_interval(
            self.toggle("Random Delay", true),
            self.slider("Delay (ms)", 90.0),
            self.slider("Min Delay (ms)", 70.0),
            self.slider("Max Delay (ms)", 120.0),
            roll(),
        )
    }

     
     
    fn pick_reaction_delay(&self) -> Duration {
        compute_reaction_delay(self.slider("Reaction Delay (ms)", 15.0), roll())
    }

     
    fn passes_weapon_filter(&self, player: &LocalPlayer) -> bool {
        let filter = self.weapon_filter();
        if filter == WeaponFilter::Any {
            return true;
        }
        let Ok(inventory) = player.get_inventory() else {
            return false;
        };
        let Ok(selected) = inventory.get_selected() else {
            return false;
        };
        let Ok(stack) = inventory.get_item(selected) else {
            return false;
        };
        if stack.is_empty().unwrap_or(true) {
            return false;
        }
        let Ok(item) = stack.get_item() else {
            return false;
        };
        let Ok(name) = item.registry_name() else {
            return false;
        };
        filter.matches(&name)
    }

     
    fn is_valid_target(&self, player: &LocalPlayer, target: &Entity) -> bool {
        let Ok(target_id) = target.id() else {
            return false;
        };
        if player.id().ok() == Some(target_id) {
            return false;
        }
         
         
        if crate::friends::ignores_entity(target, ModuleId::TriggerBot) {
            return false;
        }
        if target.is_spectator().unwrap_or(false) {
            return false;
        }
        if self.toggle("Ignore Invisible", false) && target.is_invisible().unwrap_or(false) {
            return false;
        }

        let is_player = target.instance_of::<Player>();
        let is_mob = target.instance_of::<Mob>();
        let is_crystal = target.instance_of::<EndCrystal>();

        let kind_allowed = (is_player && self.toggle("Players", true))
            || (is_mob && self.toggle("Mobs", true))
            || (is_crystal && self.toggle("Crystals", false));
        if !kind_allowed {
            return false;
        }

        if let Some(living) = target.as_living() {
            if !living.is_alive().unwrap_or(true) {
                return false;
            }
            if self.toggle("Shield Check", true) && living.is_blocking().unwrap_or(false) {
                return false;
            }
            if self.toggle("HurtTime Check", false) {
                let max_hurt = self.slider("Max HurtTime", 3.0).round() as i32;
                if let Ok(hurt_time) = living.hurt_time() {
                    if hurt_time > max_hurt {
                        return false;
                    }
                }
            }
        }

         
         
        within_reach_up_to(player, target, MAX_TRIGGER_REACH).unwrap_or(false)
    }

     
     
    fn perform_click(&self, player: &LocalPlayer, target: &Entity) -> bool {
        let button = self.click_button();
        let actual_mouse = self.toggle("Actual Mouse", false);
        let mc = minecraft();

        if actual_mouse {
            match button {
                ClickButton::Left => crate::graphic::platform::left_click(),
                ClickButton::Right => crate::graphic::platform::right_click(),
                ClickButton::Middle => crate::graphic::platform::middle_click(),
                ClickButton::Both => {
                    let left = crate::graphic::platform::left_click();
                    let right = crate::graphic::platform::right_click();
                    left || right
                }
            }
        } else {
            match button {
                ClickButton::Left => {
                    if mc.start_attack().is_ok() {
                        true
                    } else if let Ok(Some(game_mode)) = mc.game_mode() {
                        if game_mode.attack(player, target).is_ok() {
                            let _ = player.swing();
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                ClickButton::Right => {
                    if mc.start_use_item().is_ok() {
                        true
                    } else if let Ok(Some(game_mode)) = mc.game_mode() {
                        game_mode.use_item(player).is_ok()
                    } else {
                        false
                    }
                }
                ClickButton::Middle => mc.pick_block().is_ok(),
                ClickButton::Both => {
                    let left = mc.start_attack().is_ok();
                    let right = mc.start_use_item().is_ok();
                    left || right
                }
            }
        }
    }

    fn run_step(&self) {
        if !client().modules.is_active() {
            return;
        }
        if crate::module::world::autoeat::is_eating()
            || crate::graphic::input::GUI_OPEN.load(std::sync::atomic::Ordering::Relaxed)
        {
            self.reset_state();
            return;
        }

        let mc = minecraft();
        if mc.current_screen().is_open() {
            self.reset_state();
            return;
        }

        let Some(player) = mc.player().ok().flatten() else {
            self.reset_state();
            return;
        };

        let now = Instant::now();

         
        {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if let Some(reset_at) = state.sprint_reset_at {
                if now >= reset_at {
                    let _ = set_sprint_key(true);
                    state.sprint_reset_at = None;
                }
            }
        }

         
        if self.toggle("Require Mouse Hold", false) && !is_required_button_held(self.click_button())
        {
            if let Ok(mut state) = self.state.lock() {
                state.acquired_target = None;
            }
            return;
        }

         
        if self.toggle("Pause While Using Item", true) && player.is_using_item().unwrap_or(false) {
            if let Ok(mut state) = self.state.lock() {
                state.acquired_target = None;
            }
            return;
        }

         
        if !self.passes_weapon_filter(&player) {
            if let Ok(mut state) = self.state.lock() {
                state.acquired_target = None;
            }
            return;
        }

         
        let Some(target) = mc.crosshair_entity().ok().flatten() else {
            if let Ok(mut state) = self.state.lock() {
                state.acquired_target = None;
            }
            return;
        };

        if !self.is_valid_target(&player, &target) {
            if let Ok(mut state) = self.state.lock() {
                state.acquired_target = None;
            }
            return;
        }

        let Ok(target_id) = target.id() else {
            return;
        };

         
        let reaction_ready = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            match state.acquired_target {
                Some((id, ready_at)) if id == target_id => now >= ready_at,
                _ => {
                    let ready_at = now + self.pick_reaction_delay();
                    state.acquired_target = Some((target_id, ready_at));
                    now >= ready_at
                }
            }
        };
        if !reaction_ready {
            return;
        }

         
        let delay_ready = {
            let Ok(state) = self.state.lock() else {
                return;
            };
            state
                .last_click
                .is_none_or(|last| now.saturating_duration_since(last) >= state.next_delay)
        };
        if !delay_ready {
            return;
        }

         
        let button = self.click_button();
        if matches!(button, ClickButton::Left | ClickButton::Both) && self.toggle("Cooldown", true)
        {
            let threshold = self.slider("Cooldown Threshold", 0.95).clamp(0.1, 1.0);
            if player.attack_strength_scale().ok().unwrap_or(1.0) < threshold {
                return;
            }
        }

         
         
        if self.toggle("Criticals Only", false) && !can_critical_hit(&player) {
            return;
        }

         
        let chance = self.slider("Hit Chance (%)", 100.0);
        if !passes_hit_chance(chance, roll()) {
            let next_delay = self.pick_interval();
            if let Ok(mut state) = self.state.lock() {
                state.last_click = Some(now);
                state.next_delay = next_delay;
            }
            return;
        }

        if self.perform_click(&player, &target) {
            let next_delay = self.pick_interval();
            let sprint_reset = self.toggle("Sprint Reset", false)
                && matches!(button, ClickButton::Left | ClickButton::Both)
                && player.is_sprinting().unwrap_or(false);

            if sprint_reset {
                let _ = set_sprint_key(false);
            }

            if let Ok(mut state) = self.state.lock() {
                state.last_click = Some(now);
                state.next_delay = next_delay;
                if sprint_reset {
                    state.sprint_reset_at = Some(now + SPRINT_RESET_DELAY);
                }
            }
        }
    }

    fn reset_state(&self) {
        if let Ok(mut state) = self.state.lock() {
            if state.sprint_reset_at.is_some() {
                let _ = set_sprint_key(true);
            }
            *state = TriggerState::default();
        }
    }
}

fn set_sprint_key(down: bool) -> anyhow::Result<()> {
    minecraft().options()?.key_sprint()?.set_down(down)
}

fn is_required_button_held(button: ClickButton) -> bool {
    let Ok(mouse) = crate::graphic::input::MOUSE_STATE.lock() else {
        return false;
    };
    match button {
        ClickButton::Left => mouse.left_down,
        ClickButton::Right => mouse.right_down,
        ClickButton::Middle | ClickButton::Both => mouse.left_down || mouse.right_down,
    }
}

fn can_critical_hit(player: &LocalPlayer) -> bool {
    if player.is_in_water().unwrap_or(false)
        || player.is_in_lava().unwrap_or(false)
        || player.on_climbable().unwrap_or(false)
    {
        return true;
    }
    if player.on_ground().unwrap_or(false) {
        return false;
    }
    let falling = player
        .get_delta_movement()
        .map(|motion| motion.y() < 0.0)
        .unwrap_or(false);
    let fall_distance = player.get_fall_distance().unwrap_or(0.0);
    falling || fall_distance > 0.0
}

 
 
pub fn compute_click_interval(
    random_on: bool,
    delay_ms: f32,
    min_ms: f32,
    max_ms: f32,
    roll: f64,
) -> Duration {
    let clean = |value: f32, fallback: f32| -> f64 {
        if value.is_finite() {
            value.clamp(MIN_DELAY_MS, MAX_DELAY_MS) as f64
        } else {
            fallback as f64
        }
    };
    if random_on {
        let first = clean(min_ms, 70.0);
        let second = clean(max_ms, 120.0);
        let (low, high) = if first <= second {
            (first, second)
        } else {
            (second, first)
        };
        let ms = low + (high - low) * roll.clamp(0.0, 1.0);
        Duration::from_secs_f64(ms / 1000.0)
    } else {
        let ms = clean(delay_ms, 90.0);
        Duration::from_secs_f64(ms / 1000.0)
    }
}

 
 
pub fn compute_reaction_delay(reaction_ms: f32, roll: f64) -> Duration {
    if !reaction_ms.is_finite() || reaction_ms <= 0.0 {
        return Duration::ZERO;
    }
    let clamped = reaction_ms.clamp(0.0, MAX_REACTION_MS) as f64;
    let factor = 0.8 + 0.2 * roll.clamp(0.0, 1.0);
    Duration::from_secs_f64((clamped * factor) / 1000.0)
}

 
pub fn passes_hit_chance(chance_percent: f32, roll: f64) -> bool {
    if !chance_percent.is_finite() {
        return true;
    }
    let normalized = (chance_percent.clamp(0.0, 100.0) as f64) / 100.0;
    if normalized >= 1.0 {
        return true;
    }
    if normalized <= 0.0 {
        return false;
    }
    roll.clamp(0.0, 1.0) < normalized
}

impl Module for TriggerBotModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.reset_state();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        self.run_step();
        Ok(())
    }

    fn on_render(&self) {
        self.run_step();
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
    fn triggerbot_exposes_all_expected_settings() {
        let module = TriggerBotModule::new();
        assert_eq!(module.module.id, ModuleId::TriggerBot);
        assert_eq!(module.module.category, ModuleCategory::Combat);
        let names: Vec<&str> = module
            .module
            .settings
            .iter()
            .map(ModuleSetting::name)
            .collect();
        assert_eq!(
            names,
            vec![
                "Click Button",
                "Delay (ms)",
                "Random Delay",
                "Min Delay (ms)",
                "Max Delay (ms)",
                "Reaction Delay (ms)",
                "Hit Chance (%)",
                "Cooldown",
                "Cooldown Threshold",
                "Players",
                "Mobs",
                "Crystals",
                "Ignore Invisible",
                "Shield Check",
                "HurtTime Check",
                "Max HurtTime",
                "Weapon Filter",
                "Criticals Only",
                "Require Mouse Hold",
                "Pause While Using Item",
                "Sprint Reset",
                "Actual Mouse",
            ]
        );
    }

    #[test]
    fn fixed_and_random_delay_calculations_respect_bounds() {
         
        assert_eq!(
            compute_click_interval(false, 125.0, 50.0, 200.0, 0.5),
            Duration::from_millis(125)
        );
         
        for step in 0..=100 {
            let r = step as f64 / 100.0;
            let d1 = compute_click_interval(true, 90.0, 60.0, 140.0, r).as_secs_f64() * 1000.0;
            assert!((60.0..=140.0).contains(&d1), "{d1} ms outside 60..=140");
            let d2 = compute_click_interval(true, 90.0, 140.0, 60.0, r).as_secs_f64() * 1000.0;
            assert!((60.0..=140.0).contains(&d2), "{d2} ms outside 60..=140");
        }
         
        assert_eq!(
            compute_click_interval(false, 0.0, 0.0, 0.0, 0.0),
            Duration::ZERO
        );
    }

    #[test]
    fn reaction_delay_scales_with_slider() {
        assert_eq!(compute_reaction_delay(0.0, 0.5), Duration::ZERO);
        let min_react = compute_reaction_delay(100.0, 0.0).as_secs_f64() * 1000.0;
        let max_react = compute_reaction_delay(100.0, 1.0).as_secs_f64() * 1000.0;
        assert!((min_react - 80.0).abs() < 1e-6);
        assert!((max_react - 100.0).abs() < 1e-6);
    }

    #[test]
    fn weapon_filter_matches_expected_items() {
        assert!(WeaponFilter::Any.matches(""));
        assert!(WeaponFilter::Any.matches("minecraft:stick"));

        assert!(WeaponFilter::WeaponsOnly.matches("minecraft:netherite_sword"));
        assert!(WeaponFilter::WeaponsOnly.matches("minecraft:diamond_axe"));
        assert!(WeaponFilter::WeaponsOnly.matches("minecraft:trident"));
        assert!(WeaponFilter::WeaponsOnly.matches("minecraft:mace"));
        assert!(!WeaponFilter::WeaponsOnly.matches("minecraft:diamond_pickaxe"));

        assert!(WeaponFilter::SwordOnly.matches("minecraft:iron_sword"));
        assert!(!WeaponFilter::SwordOnly.matches("minecraft:iron_axe"));

        assert!(WeaponFilter::AxeOnly.matches("minecraft:stone_axe"));
        assert!(!WeaponFilter::AxeOnly.matches("minecraft:stone_sword"));
    }

    #[test]
    fn click_button_maps_all_four_modes() {
        assert_eq!(ClickButton::from_index(0), ClickButton::Left);
        assert_eq!(ClickButton::from_index(1), ClickButton::Right);
        assert_eq!(ClickButton::from_index(2), ClickButton::Middle);
        assert_eq!(ClickButton::from_index(3), ClickButton::Both);
        assert_eq!(ClickButton::from_index(99), ClickButton::Left);
    }

    #[test]
    fn hit_chance_respects_percentage_roll() {
        assert!(passes_hit_chance(100.0, 0.999));
        assert!(!passes_hit_chance(0.0, 0.0));
        assert!(passes_hit_chance(50.0, 0.49));
        assert!(!passes_hit_chance(50.0, 0.51));
    }
}
