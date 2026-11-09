use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

 
 
 
const MODIFIER_ID: &str = "infered:reach";

 
const ATTRIBUTE: &str = "ENTITY_INTERACTION_RANGE";

 
 
 
 
const MAX_EXTENSION: f64 = 0.1;

 
 
 
static ROLL_EXTENDS: AtomicBool = AtomicBool::new(true);

 
 
pub fn extends_reach_now() -> bool {
    ROLL_EXTENDS.load(Ordering::Relaxed)
}

 
 
fn roll_extends(roll: f64, chance: f64) -> bool {
    roll < chance
}

 
 
 
fn roll() -> f64 {
    let bits = RandomState::new().build_hasher().finish();
    (bits >> 11) as f64 / (1u64 << 53) as f64
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
#[derive(Debug)]
pub struct ReachModule {
    pub module: ModuleData,
     
     
    applied: Mutex<Option<(i32, f64)>>,
}

impl ReachModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Reach,
                description: "Extends attack distance".to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Distance".to_string(),
                        value: MAX_EXTENSION as f32,
                        min: 0.0,
                        max: MAX_EXTENSION as f32,
                    },
                    ModuleSetting::Slider {
                        name: "Chance".to_string(),
                        value: 100.0,
                        min: 0.0,
                        max: 100.0,
                    },
                ],
            },
            applied: Mutex::new(None),
        }
    }

     
    fn distance(&self) -> f64 {
        self.module
            .get_setting("Distance")
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .clamp(0.0, MAX_EXTENSION as f32) as f64
    }

     
    fn chance(&self) -> f64 {
        self.module
            .get_setting("Chance")
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .unwrap_or(100.0)
            .clamp(0.0, 100.0) as f64
            / 100.0
    }
}

impl Module for ReachModule {
    fn on_start(&self) -> anyhow::Result<()> {
        *self.applied.lock().unwrap() = None;
        ROLL_EXTENDS.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        if let Some(player) = minecraft().player()? {
            let _ = player.remove_attribute_modifier(ATTRIBUTE, MODIFIER_ID);
        }
        *self.applied.lock().unwrap() = None;
        ROLL_EXTENDS.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let extends = roll_extends(roll(), self.chance());
        ROLL_EXTENDS.store(extends, Ordering::Relaxed);
         
         
        if let Some(player) = minecraft().player()? {
            let id = player.id()?;
            let amount = if extends { self.distance() } else { 0.0 };
            if player.attribute_modifier_amount(ATTRIBUTE, MODIFIER_ID)? == Some(amount) {
                *self.applied.lock().unwrap() = Some((id, amount));
                return Ok(());
            }
            match player.add_attribute_modifier(ATTRIBUTE, MODIFIER_ID, amount) {
                Ok(()) => *self.applied.lock().unwrap() = Some((id, amount)),
                 
                 
                Err(error) => log::debug!("Reach: attribute update deferred: {error:#}"),
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

#[cfg(test)]
mod tests {
    use super::*;

     
    #[test]
    fn modifier_id_is_namespaced_and_stable() {
        assert!(MODIFIER_ID.starts_with("infered:"));
        assert_eq!(MODIFIER_ID, "infered:reach");
    }

    #[test]
    fn the_distance_slider_stops_at_the_margin() {
        let mut module = ReachModule::new();
        let slider = module
            .module
            .get_setting("Distance")
            .expect("Distance slider");
        assert_eq!(slider.get_slider_value(), Some(MAX_EXTENSION as f32));

        module
            .module
            .get_setting_mut("Distance")
            .unwrap()
            .set_slider_value(0.3);
         
        assert!((module.distance() - MAX_EXTENSION).abs() < 1e-6);

        module
            .module
            .get_setting_mut("Distance")
            .unwrap()
            .set_slider_value(f32::NAN);
        assert_eq!(module.distance(), 0.0);
    }

    #[test]
    fn attribute_name_matches_the_mojmap_constant() {
        assert_eq!(ATTRIBUTE, "ENTITY_INTERACTION_RANGE");
    }

    #[test]
    fn a_zero_chance_never_extends_reach_and_a_full_one_always_does() {
        for roll_value in [0.0, 0.25, 0.5, 0.999_999] {
            assert!(!roll_extends(roll_value, 0.0), "0% never reaches");
            assert!(roll_extends(roll_value, 1.0), "100% always reaches");
        }
    }

    #[test]
    fn the_roll_is_inside_the_unit_interval() {
        for _ in 0..1000 {
            let value = roll();
            assert!((0.0..1.0).contains(&value), "roll {value} escaped [0, 1)");
        }
    }

    #[test]
    fn a_fifty_percent_chance_reaches_about_half_of_the_ticks() {
        let samples = 10_000;
        let reached = (0..samples).filter(|_| roll_extends(roll(), 0.5)).count() as f64;
        let share = reached / samples as f64;
         
        assert!(
            (share - 0.5).abs() < 0.05,
            "a 50% chance reached {share:.3} of the ticks"
        );
    }
}
