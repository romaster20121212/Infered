 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::input::{key_down, GUI_OPEN};
use crate::mapping::client::options::KeyMapping;
use crate::mapping::entity::EntityRef;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

 
const MIN_SPEED: f64 = 0.03;

 
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Keys {
    forward: bool,
    back: bool,
    left: bool,
    right: bool,
}

impl Keys {
     
     
     
     
     
     
    fn angle(self) -> Option<f64> {
        let forward = self.forward as i32 - self.back as i32;
        let strafe = self.left as i32 - self.right as i32;
        if forward == 0 && strafe == 0 {
            return None;
        }
        Some((strafe as f64).atan2(forward as f64).to_degrees())
    }
}

const fn keys(forward: bool, back: bool, left: bool, right: bool) -> Keys {
    Keys {
        forward,
        back,
        left,
        right,
    }
}

 
 
const INPUT_COMBOS: [(f64, Keys); 8] = [
    (0.0, keys(true, false, false, false)),
    (45.0, keys(true, false, true, false)),
    (90.0, keys(false, false, true, false)),
    (135.0, keys(false, true, true, false)),
    (180.0, keys(false, true, false, false)),
    (-135.0, keys(false, true, false, true)),
    (-90.0, keys(false, false, false, true)),
    (-45.0, keys(true, false, false, true)),
];

 
fn wrap_degrees(mut angle: f64) -> f64 {
    angle %= 360.0;
    if angle >= 180.0 {
        angle -= 360.0;
    } else if angle < -180.0 {
        angle += 360.0;
    }
    angle
}

 
 
 
 
 
 
 
 
fn plan(held: Keys, yaw: f64, vx: f64, vz: f64, strength: f64) -> Keys {
    let Some(held_angle) = held.angle() else {
        return held;
    };
    let speed = (vx * vx + vz * vz).sqrt();
    let strength = strength.clamp(0.0, 1.0);
    if !speed.is_finite() || speed <= MIN_SPEED || strength <= 0.0 {
        return held;
    }
    let desired = vx.atan2(vz).to_degrees() + yaw;
    let wanted = held_angle + strength * wrap_degrees(desired - held_angle);
    nearest(wanted)
}

 
fn nearest(angle: f64) -> Keys {
    let mut best = INPUT_COMBOS[0];
    let mut best_error = f64::INFINITY;
    for combo in INPUT_COMBOS {
        let error = wrap_degrees(combo.0 - angle).abs();
        if error < best_error {
            best_error = error;
            best = combo;
        }
    }
    best.1
}

 
#[derive(Debug)]
struct Movement {
    forward: KeyMapping,
    back: KeyMapping,
    left: KeyMapping,
    right: KeyMapping,
}

impl Movement {
    fn read() -> anyhow::Result<Movement> {
        let options = minecraft().options()?;
        Ok(Movement {
            forward: options.key_up()?,
            back: options.key_back()?,
            left: options.key_left()?,
            right: options.key_right()?,
        })
    }

     
     
    fn keys(&self) -> anyhow::Result<Keys> {
        Ok(Keys {
            forward: self.forward.is_down()?,
            back: self.back.is_down()?,
            left: self.left.is_down()?,
            right: self.right.is_down()?,
        })
    }

     
     
     
    fn physical(&self, fallback: Keys) -> Keys {
        Keys {
            forward: physical_down(&self.forward).unwrap_or(fallback.forward),
            back: physical_down(&self.back).unwrap_or(fallback.back),
            left: physical_down(&self.left).unwrap_or(fallback.left),
            right: physical_down(&self.right).unwrap_or(fallback.right),
        }
    }

    fn write(&self, keys: Keys) -> anyhow::Result<()> {
        self.forward.set_down(keys.forward)?;
        self.back.set_down(keys.back)?;
        self.left.set_down(keys.left)?;
        self.right.set_down(keys.right)?;
        Ok(())
    }
}

 
 
fn physical_down(mapping: &KeyMapping) -> Option<bool> {
    if !mapping.key_name().ok()?.starts_with("key.keyboard.") {
        return None;
    }
    Some(key_down(mapping.key_value().ok()?))
}

 
#[derive(Debug, Default)]
struct StrafeState {
    applied: Keys,
    active: bool,
}

impl StrafeState {
     
    fn owns(&self, current: Keys) -> bool {
        self.active && current == self.applied
    }
}

 
 
#[derive(Debug)]
pub struct StrafeModule {
    pub module: ModuleData,
    state: Mutex<StrafeState>,
}

impl StrafeModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Strafe,
                description: "Keeps airborne movement pointed along your momentum".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Strength".to_string(),
                        value: 1.0,
                        min: 0.0,
                        max: 1.0,
                    },
                    ModuleSetting::Toggle {
                        name: "Only Air".to_string(),
                        value: true,
                    },
                ],
            },
            state: Mutex::new(StrafeState::default()),
        }
    }

    fn strength(&self) -> f64 {
        self.module
            .get_setting("Strength")
            .and_then(|setting| setting.get_slider_value())
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .clamp(0.0, 1.0) as f64
    }

    fn only_air(&self) -> bool {
        self.module
            .get_setting("Only Air")
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(true)
    }

     
     
    fn hand_back(
        &self,
        state: &mut StrafeState,
        movement: &Movement,
        current: Keys,
        physical: Keys,
    ) -> anyhow::Result<()> {
        if state.owns(current) {
            movement.write(physical)?;
        }
        *state = StrafeState::default();
        Ok(())
    }
}

impl Module for StrafeModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if let Ok(mut state) = self.state.lock() {
            *state = StrafeState::default();
        }
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        let Ok(mut state) = self.state.lock() else {
            return Ok(());
        };
        let movement = Movement::read()?;
        let current = movement.keys()?;
        let physical = movement.physical(current);
        self.hand_back(&mut state, &movement, current, physical)
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Ok(mut state) = self.state.lock() else {
            return Ok(());
        };
        let mc = minecraft();
        let movement = Movement::read()?;
        let current = movement.keys()?;
        let physical = movement.physical(current);

        let Some(player) = mc.player()? else {
            return self.hand_back(&mut state, &movement, current, physical);
        };

        let screen_open = mc.current_screen().is_open() || GUI_OPEN.load(Ordering::Relaxed);
        if screen_open || (self.only_air() && player.on_ground()?) {
            return self.hand_back(&mut state, &movement, current, physical);
        }

         
         
        if current != physical && !state.owns(current) {
            *state = StrafeState::default();
            return Ok(());
        }

        let velocity = player.get_delta_movement()?;
        let want = plan(
            physical,
            player.get_yaw()? as f64,
            velocity.x(),
            velocity.z(),
            self.strength(),
        );
        if want == current {
            if current == physical {
                *state = StrafeState::default();
            }
            return Ok(());
        }
        movement.write(want)?;
        state.applied = want;
        state.active = true;
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

    const W: Keys = keys(true, false, false, false);
    const A: Keys = keys(false, false, true, false);

     
     
    #[test]
    fn every_input_combination_points_at_its_own_yaw_offset() {
        assert_eq!(W.angle(), Some(0.0));
        assert_eq!(keys(true, false, true, false).angle(), Some(45.0));
        assert_eq!(A.angle(), Some(90.0));
        assert_eq!(keys(false, true, true, false).angle(), Some(135.0));
        assert_eq!(
            keys(false, true, false, false).angle().map(f64::abs),
            Some(180.0)
        );
        assert_eq!(keys(false, true, false, true).angle(), Some(-135.0));
        assert_eq!(keys(false, false, false, true).angle(), Some(-90.0));
        assert_eq!(keys(true, false, false, true).angle(), Some(-45.0));
        assert_eq!(Keys::default().angle(), None);
        assert_eq!(keys(true, true, false, false).angle(), None);
    }

     
     
    #[test]
    fn the_plan_points_the_step_along_the_players_momentum() {
        assert_eq!(plan(W, 0.0, 1.0, 0.0, 1.0), A);
         
        assert_eq!(
            plan(W, 0.0, 0.0, -1.0, 1.0),
            keys(false, true, false, false)
        );
         
         
        assert_eq!(plan(W, 90.0, -1.0, 0.0, 1.0), W);
    }

     
     
    #[test]
    fn strength_blends_between_the_held_keys_and_the_momentum() {
        assert_eq!(plan(W, 0.0, 1.0, 0.0, 0.0), W);
        assert_eq!(plan(W, 0.0, 1.0, 0.0, 0.5), keys(true, false, true, false));
         
         
        assert_eq!(
            plan(W, 0.0, 0.0, -1.0, 0.5),
            keys(false, false, false, true)
        );
    }

     
    #[test]
    fn nothing_is_planned_without_input_or_momentum() {
        assert_eq!(plan(Keys::default(), 0.0, 1.0, 0.0, 1.0), Keys::default());
        assert_eq!(plan(W, 0.0, 0.0, 0.0, 1.0), W);
        assert_eq!(plan(W, 0.0, f64::NAN, 0.0, 1.0), W);
        assert_eq!(plan(W, 0.0, 0.01, 0.0, 1.0), W);
    }

     
    #[test]
    fn every_plan_is_one_of_the_eight_key_combinations() {
        for yaw in [-179.0, -90.0, 0.0, 23.0, 90.0, 179.0] {
            for vx in [-1.0, -0.2, 0.0, 0.2, 1.0] {
                for vz in [-1.0, -0.2, 0.2, 1.0] {
                    let planned = plan(W, yaw, vx, vz, 1.0);
                    assert!(
                        INPUT_COMBOS.iter().any(|(_, keys)| *keys == planned),
                        "{planned:?} is not a vanilla input"
                    );
                }
            }
        }
    }
}
