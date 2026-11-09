use crate::mapping::block::Direction;
use crate::mapping::client::world::World;
use crate::mapping::entity::EntityRef;
use crate::mapping::math::BlockPos;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

 
const MINING_FACE: Direction = Direction::Up;

 
 
#[derive(Debug)]
pub struct NukerModule {
    pub module: ModuleData,
     
     
     
    target: Mutex<Option<BlockPos>>,
}

impl NukerModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Nuker,
                description: "Breaks the blocks around the player".to_string(),
                category: ModuleCategory::World,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Range".to_string(),
                    value: 4.0,
                    min: 1.0,
                    max: 5.0,
                }],
            },
            target: Mutex::new(None),
        }
    }

    fn range(&self) -> i32 {
        self.module
            .get_setting("Range")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(4.0) as i32
    }
}

impl Module for NukerModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
        let had_target = self.target.lock().unwrap().take().is_some();
        if had_target {
            if let Some(game_mode) = minecraft().game_mode()? {
                let _ = game_mode.stop_destroy_block();
            }
        }
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let minecraft = minecraft();
         
        if minecraft.current_screen().is_open() {
            return Ok(());
        }
        let (Some(player), Some(world), Some(game_mode)) = (
            minecraft.player()?,
            minecraft.world()?,
            minecraft.game_mode()?,
        ) else {
            *self.target.lock().unwrap() = None;
            return Ok(());
        };

        let feet = player.get_position()?;
        let center = BlockPos::new(
            feet.x().floor() as i32,
            feet.y().floor() as i32,
            feet.z().floor() as i32,
        );
        let range = self.range();

         
         
         
        let mut target = self.target.lock().unwrap();
        let previous = *target;
        let chosen = match *target {
            Some(pos) if in_range(pos, center, range) && !world.is_block_air(&pos)? => Some(pos),
            _ => nearest_block(&world, center, range)?,
        };
        *target = chosen;
        drop(target);

        match chosen {
             
             
             
            Some(pos) if previous == Some(pos) => {
                game_mode.continue_destroy_block(&pos, MINING_FACE)?;
            }
             
             
            Some(pos) => {
                game_mode.start_destroy_block(&pos, MINING_FACE)?;
            }
             
             
            None => {
                if previous.is_some() {
                    game_mode.stop_destroy_block()?;
                }
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

 
fn in_range(pos: BlockPos, center: BlockPos, range: i32) -> bool {
    let dx = pos.x() - center.x();
    let dy = pos.y() - center.y();
    let dz = pos.z() - center.z();
    dx * dx + dy * dy + dz * dz <= range * range
}

 
fn nearest_block(world: &World, center: BlockPos, range: i32) -> anyhow::Result<Option<BlockPos>> {
    let mut best: Option<(BlockPos, i32)> = None;
    let range_sq = range * range;
    for dx in -range..=range {
        for dy in -range..=range {
            for dz in -range..=range {
                let distance_sq = dx * dx + dy * dy + dz * dz;
                if distance_sq == 0 || distance_sq > range_sq {
                    continue;
                }
                let pos = center.offset(dx, dy, dz);
                if world.is_block_air(&pos)? {
                    continue;
                }
                if best.is_none_or(|(_, best_distance)| distance_sq < best_distance) {
                    best = Some((pos, distance_sq));
                }
            }
        }
    }
    Ok(best.map(|(pos, _)| pos))
}
