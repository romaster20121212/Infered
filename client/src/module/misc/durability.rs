use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::graphic::notification::{Notification, NotificationType};
use crate::mapping::entity::player::{GearStack, Player};
use crate::mapping::MappedObject;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;

#[derive(Debug, Default)]
struct AlertState {
    last_check: Option<Instant>,
    warned: HashMap<&'static str, String>,
}

#[derive(Debug)]
pub struct DurabilityModule {
    module: ModuleData,
    elytra_only: bool,
    state: Mutex<AlertState>,
}

impl DurabilityModule {
    pub fn new(elytra_only: bool) -> Self {
        Self {
            module: ModuleData {
                id: if elytra_only {
                    ModuleId::ElytraDurabilityWarning
                } else {
                    ModuleId::DurabilityAlert
                },
                description: if elytra_only {
                    "Warns when equipped elytra durability is low."
                } else {
                    "Warns when equipped armor or held tools have low durability."
                }
                .into(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Remaining %".into(),
                    value: 15.0,
                    min: 1.0,
                    max: 100.0,
                }],
            },
            elytra_only,
            state: Mutex::new(AlertState::default()),
        }
    }

    fn threshold(&self) -> f32 {
        self.module
            .get_setting("Remaining %")
            .and_then(ModuleSetting::get_slider_value)
            .filter(|value| value.is_finite())
            .unwrap_or(15.0)
            .clamp(1.0, 100.0)
            / 100.0
    }
}

fn low_durability(stack: &GearStack, threshold: f32, elytra_only: bool) -> Option<f32> {
    if stack.count <= 0 || (elytra_only && stack.name != "elytra") {
        return None;
    }
    let remaining = stack.durability()?;
    (remaining <= threshold).then_some(remaining)
}

impl Module for DurabilityModule {
    fn on_start(&self) -> anyhow::Result<()> {
        *self.state.lock().unwrap_or_else(|p| p.into_inner()) = AlertState::default();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        *self.state.lock().unwrap_or_else(|p| p.into_inner()) = AlertState::default();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state
            .last_check
            .is_some_and(|last| last.elapsed() < Duration::from_secs(1))
        {
            return Ok(());
        }
        state.last_check = Some(Instant::now());
        let Some(local) = minecraft().player()? else {
            state.warned.clear();
            return Ok(());
        };
        let equipment = Player::new(local.jni_ref().clone()).equipment()?;
        let slots = [
            ("Helmet", equipment.armor[0].as_ref()),
            ("Chest", equipment.armor[1].as_ref()),
            ("Legs", equipment.armor[2].as_ref()),
            ("Boots", equipment.armor[3].as_ref()),
            ("Main hand", equipment.main_hand.as_ref()),
            ("Off hand", equipment.off_hand.as_ref()),
        ];
        for (slot, stack) in slots {
            let low = stack.and_then(|item| {
                low_durability(item, self.threshold(), self.elytra_only)
                    .map(|fraction| (item, fraction))
            });
            if let Some((item, remaining)) = low {
                if state.warned.get(slot) != Some(&item.name) {
                    Notification::send_with_time(
                        NotificationType::Warning,
                        if self.elytra_only {
                            "Elytra durability"
                        } else {
                            "Low durability"
                        },
                        &format!(
                            "{slot}: {} ({:.0}% remaining)",
                            item.name.replace('_', " "),
                            remaining * 100.0
                        ),
                        6.0,
                    );
                    state.warned.insert(slot, item.name.clone());
                }
            } else {
                state.warned.remove(slot);
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
    fn only_damaged_equipment_below_threshold_is_reported() {
        let stack = GearStack {
            name: "elytra".into(),
            damage: 90,
            max_damage: 100,
            count: 1,
        };
        assert!(low_durability(&stack, 0.15, true).is_some());
        assert!(low_durability(&stack, 0.05, true).is_none());
        let sword = GearStack {
            name: "diamond_sword".into(),
            ..stack
        };
        assert!(low_durability(&sword, 0.15, true).is_none());
        assert!(low_durability(&sword, 0.15, false).is_some());
        let undamageable = GearStack {
            max_damage: 0,
            ..sword
        };
        assert!(low_durability(&undamageable, 0.15, false).is_none());
    }
}
