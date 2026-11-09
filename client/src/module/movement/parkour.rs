 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::input::GUI_OPEN;
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::math::BlockPos;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

 
 
const EDGE_PROBE: f64 = 0.45;

 
 
 
 
pub(crate) fn movement_direction(yaw: f32, strafe: f32, forward: f32) -> Option<(f64, f64)> {
    let yaw = (yaw as f64).to_radians();
    let (sin, cos) = (yaw.sin(), yaw.cos());
    let dx = strafe as f64 * cos - forward as f64 * sin;
    let dz = forward as f64 * cos + strafe as f64 * sin;
    let length = (dx * dx + dz * dz).sqrt();
    if !length.is_finite() || length < 1e-6 {
        return None;
    }
    Some((dx / length, dz / length))
}

 
pub(crate) fn edge_cells(
    feet_x: f64,
    feet_y: f64,
    feet_z: f64,
    direction: (f64, f64),
) -> (BlockPos, BlockPos) {
    let x = (feet_x + direction.0 * EDGE_PROBE).floor() as i32;
    let z = (feet_z + direction.1 * EDGE_PROBE).floor() as i32;
    (
        BlockPos::new(x, (feet_y - 0.1).floor() as i32, z),
        BlockPos::new(x, (feet_y + 0.1).floor() as i32, z),
    )
}

 
 
 
 
 
pub(crate) fn wants_edge_jump(
    on_ground: bool,
    forward: f32,
    sprinting: bool,
    only_sprinting: bool,
    ground_ahead: bool,
    body_ahead: bool,
) -> bool {
    on_ground && forward > 0.0 && (!only_sprinting || sprinting) && !ground_ahead && !body_ahead
}

 
#[derive(Debug)]
pub struct ParkourModule {
    pub module: ModuleData,
     
    held: Mutex<bool>,
}

impl ParkourModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Parkour,
                description: "Jumps at the edge of a drop while walking forward".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Toggle {
                    name: "Only While Sprinting".to_string(),
                    value: false,
                }],
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

     
     
     
     
    fn input(&self) -> anyhow::Result<(f32, f32)> {
        let player = minecraft()
            .player()?
            .ok_or_else(|| anyhow::anyhow!("no player"))?;
        let (strafe, forward) = player.move_input()?;
        if forward > 0.0 {
            return Ok((strafe, forward));
        }
        let options = minecraft().options()?;
        if !options.key_up()?.is_down()? {
            return Ok((strafe, forward));
        }
        let left = options.key_left()?.is_down()?;
        let right = options.key_right()?.is_down()?;
        Ok(((left as i32 - right as i32) as f32, 1.0))
    }

    fn want(&self) -> anyhow::Result<bool> {
        let player = minecraft()
            .player()?
            .ok_or_else(|| anyhow::anyhow!("no player"))?;
        let (strafe, forward) = self.input()?;
        if forward <= 0.0 {
            return Ok(false);
        }
        let Some(direction) = movement_direction(player.get_yaw()?, strafe, forward) else {
            return Ok(false);
        };
        let feet = player.get_position()?;
        let (ground_cell, body_cell) = edge_cells(feet.x(), feet.y(), feet.z(), direction);
        let Some(world) = minecraft().world()? else {
            return Ok(false);
        };
        let ground_ahead = !world.block_collision_boxes(&ground_cell)?.is_empty();
        let body_ahead = !world.block_collision_boxes(&body_cell)?.is_empty();
        Ok(wants_edge_jump(
            player.on_ground()?,
            forward,
            player.is_sprinting()?,
            self.toggle("Only While Sprinting", false),
            ground_ahead,
            body_ahead,
        ))
    }
}

impl Default for ParkourModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ParkourModule {
    fn on_start(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            crate::jvmti::transform_installed("ScaffoldInputPhase"),
            "Parkour requires Minecraft 1.21.4 input-phase hook"
        );
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

    fn on_input_phase(&self) -> anyhow::Result<()> {
        let Ok(mut held) = self.held.lock() else {
            return Ok(());
        };
        let mc = minecraft();
        if mc.player()?.is_none()
            || mc.current_screen().is_open()
            || GUI_OPEN.load(Ordering::Relaxed)
        {
            return self.apply(&mut held, false);
        }
        let want = self.want()?;
        self.apply(&mut held, want)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn jump(
        on_ground: bool,
        forward: f32,
        sprint: bool,
        sprint_only: bool,
        ground: bool,
        body: bool,
    ) -> bool {
        wants_edge_jump(on_ground, forward, sprint, sprint_only, ground, body)
    }

    #[test]
    fn a_ledge_under_forward_input_is_a_jump() {
        assert!(jump(true, 1.0, false, false, false, false));
    }

    #[test]
    fn the_air_and_a_still_player_never_jump() {
        assert!(!jump(false, 1.0, false, false, false, false));
        assert!(!jump(true, 0.0, false, false, false, false));
        assert!(!jump(true, -1.0, false, false, false, false));
    }

    #[test]
    fn ordinary_floor_and_a_wall_are_not_edges() {
        assert!(
            !jump(true, 1.0, false, false, true, false),
            "a solid ground cell ahead is the floor, or a step"
        );
        assert!(
            !jump(true, 1.0, false, false, false, true),
            "a solid cell at leg level is a wall"
        );
    }

    #[test]
    fn sprinting_can_be_required() {
        assert!(!jump(true, 1.0, false, true, false, false));
        assert!(jump(true, 1.0, true, true, false, false));
    }

    #[test]
    fn the_heading_follows_the_yaw() {
        let ahead = movement_direction(0.0, 0.0, 1.0).unwrap();
        assert!((ahead.0 - 0.0).abs() < 1e-9 && (ahead.1 - 1.0).abs() < 1e-9);
         
        let west = movement_direction(90.0, 0.0, 1.0).unwrap();
        assert!((west.0 + 1.0).abs() < 1e-9 && west.1.abs() < 1e-9);
         
        let north = movement_direction(180.0, 0.0, 1.0).unwrap();
        assert!(north.0.abs() < 1e-9 && (north.1 + 1.0).abs() < 1e-9);
         
        let left = movement_direction(0.0, 1.0, 0.0).unwrap();
        assert!((left.0 - 1.0).abs() < 1e-9 && left.1.abs() < 1e-9);
        assert!(movement_direction(0.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn the_edge_cell_is_one_block_ahead_and_one_level_down() {
        let (ground, body) = edge_cells(12.5, 64.0, -7.0, (0.0, 1.0));
        assert_eq!(ground, BlockPos::new(12, 63, -7));
        assert_eq!(body, BlockPos::new(12, 64, -7));
    }

    #[test]
    fn module_identity() {
        let module = ParkourModule::new();
        assert_eq!(module.module.id, ModuleId::Parkour);
        assert_eq!(module.module.id.display_name(), "Parkour");
        assert_eq!(module.module.category, ModuleCategory::Movement);
        assert!(!module.module.enabled);
        assert!(!module.toggle("Only While Sprinting", true));
    }
}
