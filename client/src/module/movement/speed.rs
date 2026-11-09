use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;

 
 
const SPRINT_SPEED: f64 = 0.2806;
 
 
 
const TICK_ACCEL: f64 = 0.13;
 
 
const EPSILON: f64 = 0.001;

 
 
 
 
 
 
 
#[derive(Debug)]
pub struct SpeedModule {
    pub module: ModuleData,
}

impl SpeedModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Speed,
                description: "Moves the player faster than vanilla".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Multiplier".to_string(),
                    value: 1.5,
                    min: 1.0,
                    max: 3.0,
                }],
            },
        }
    }

    fn multiplier(&self) -> f64 {
        self.module
            .get_setting("Multiplier")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(1.5) as f64
    }
}

impl Module for SpeedModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
        if let Some(player) = minecraft().player()? {
            let motion = player.get_delta_movement()?;
            let horizontal = (motion.x() * motion.x() + motion.z() * motion.z()).sqrt();
            if horizontal > SPRINT_SPEED {
                let scale = SPRINT_SPEED / horizontal;
                let _ =
                    player.set_delta_movement(motion.x() * scale, motion.y(), motion.z() * scale);
            }
        }
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
         
         
        if !player.on_ground()? || !player.has_move_input()? {
            return Ok(());
        }

        let motion = player.get_delta_movement()?;
        let horizontal = (motion.x() * motion.x() + motion.z() * motion.z()).sqrt();
        if horizontal <= EPSILON {
            return Ok(());  
        }

         
         
        let target = (SPRINT_SPEED * self.multiplier() - TICK_ACCEL).max(EPSILON);
        if horizontal >= target {
            return Ok(());  
        }

        let scale = target / horizontal;
        player.set_delta_movement(motion.x() * scale, motion.y(), motion.z() * scale)
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
