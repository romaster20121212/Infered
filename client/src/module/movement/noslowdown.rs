use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;

 
 
const USE_ITEM_MULTIPLIER: f64 = 0.2;
 
 
 
const COMPENSATION: f64 = 1.0 / USE_ITEM_MULTIPLIER - 1.0;
 
 
const GROUND_SPEED_FACTOR: f64 = 0.21600002 / (0.6 * 0.6 * 0.6);
 
const INPUT_EPSILON: f64 = 1.0e-7;

 
 
 
 
 
 
 
 
 
 
#[derive(Debug)]
pub struct NoSlowdownModule {
    pub module: ModuleData,
}

impl NoSlowdownModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::NoSlowdown,
                description: "Removes the slowdown from using items".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
        }
    }
}

impl Module for NoSlowdownModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
         
         
        if !player.is_using_item()? || !player.on_ground()? {
            return Ok(());
        }

        let (strafe, forward) = player.move_input()?;
        let input_sq = (strafe * strafe + forward * forward) as f64;
        if input_sq < INPUT_EPSILON {
            return Ok(());  
        }

         
         
         
        let speed = player.get_speed()? as f64 * GROUND_SPEED_FACTOR;
        let (sin, cos) = (player.get_yaw()? as f64).to_radians().sin_cos();
        let local_x = strafe as f64 * speed;
        let local_z = forward as f64 * speed;
        let accel_x = local_x * cos - local_z * sin;
        let accel_z = local_z * cos + local_x * sin;

         
        let motion = player.get_delta_movement()?;
        player.set_delta_movement(
            motion.x() + accel_x * COMPENSATION,
            motion.y(),
            motion.z() + accel_z * COMPENSATION,
        )
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
