 
 
 
 
 
 
 
 

use crate::graphic::input::GUI_OPEN;
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

 
const MOVING_EPSILON: f64 = 0.03;

 
 
 
 
pub(crate) fn wants_jump(
    on_ground: bool,
    horizontal_speed: f64,
    forward: f32,
    only_while_moving: bool,
    only_while_sprinting: bool,
    sprinting: bool,
) -> bool {
    if !on_ground {
        return false;
    }
    if only_while_sprinting && !sprinting {
        return false;
    }
    if only_while_moving && (horizontal_speed <= MOVING_EPSILON || forward <= 0.0) {
        return false;
    }
    true
}

 
#[derive(Debug)]
pub struct AutoJumpModule {
    pub module: ModuleData,
     
    held: Mutex<bool>,
}

impl AutoJumpModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoJump,
                description: "Holds jump while walking on the ground".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Toggle {
                        name: "Only While Moving".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Only While Sprinting".to_string(),
                        value: false,
                    },
                ],
            },
            held: Mutex::new(false),
        }
    }

    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(fallback)
    }

    fn apply(&self, held: &mut bool, want: bool) -> anyhow::Result<()> {
        let key = minecraft().options()?.key_jump()?;
        if want {
             
            if !*held && !key.is_down()? {
                key.set_down(true)?;
                *held = true;
            }
        } else if *held {
            key.set_down(false)?;
            *held = false;
        }
        Ok(())
    }
}

impl Module for AutoJumpModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if let Ok(mut held) = self.held.lock() {
            *held = false;
        }
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        let Ok(mut held) = self.held.lock() else {
            return Ok(());
        };
        self.apply(&mut held, false)
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Ok(mut held) = self.held.lock() else {
            return Ok(());
        };
        let mc = minecraft();

        let Some(player) = mc.player()? else {
            return self.apply(&mut held, false);
        };
        if mc.current_screen().is_open() || GUI_OPEN.load(Ordering::Relaxed) {
            return self.apply(&mut held, false);
        }

        let motion = player.get_delta_movement()?;
        let horizontal = (motion.x() * motion.x() + motion.z() * motion.z()).sqrt();
        let (_, forward) = player.move_input()?;
        let want = wants_jump(
            player.on_ground()?,
            horizontal,
            forward,
            self.toggle("Only While Moving", true),
            self.toggle("Only While Sprinting", false),
            player.is_sprinting()?,
        );
        self.apply(&mut held, want)
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
    use super::wants_jump;

    fn jump(
        on_ground: bool,
        speed: f64,
        forward: f32,
        moving: bool,
        sprint_only: bool,
        sprinting: bool,
    ) -> bool {
        wants_jump(on_ground, speed, forward, moving, sprint_only, sprinting)
    }

    #[test]
    fn jumps_while_walking_forward() {
        assert!(jump(true, 0.15, 1.0, true, false, false));
    }

    #[test]
    fn never_jumps_in_the_air() {
        assert!(!jump(false, 0.15, 1.0, true, false, false));
        assert!(!jump(false, 0.15, 1.0, false, false, false));
    }

    #[test]
    fn a_standing_player_does_not_hop() {
        assert!(!jump(true, 0.0, 0.0, true, false, true));
         
        assert!(!jump(true, 0.01, 0.0, true, false, true));
    }

    #[test]
    fn strafing_only_does_not_hop() {
         
         
        assert!(!jump(true, 0.15, 0.0, true, false, false));
    }

    #[test]
    fn the_moving_gate_can_be_switched_off() {
        assert!(jump(true, 0.0, 0.0, false, false, false));
    }

    #[test]
    fn sprinting_can_be_required() {
        assert!(!jump(true, 0.17, 1.0, true, true, false));
        assert!(jump(true, 0.17, 1.0, true, true, true));
    }
}
