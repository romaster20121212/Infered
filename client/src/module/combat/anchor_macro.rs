 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::input::{GUI_OPEN, MOUSE_STATE};
use crate::mapping::block::Direction;
use crate::mapping::client::gamemode::MultiPlayerGameMode;
use crate::mapping::client::minecraft::Minecraft;
use crate::mapping::client::world::World;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::LivingEntityRef;
use crate::mapping::inventory::Inventory;
use crate::mapping::math::{BlockPos, Vec3};
use crate::module::combat::crystal_macro::{
    crosshair_reaches, find_hotbar_item, press_hotbar_slot,
};
use crate::module::combat::interaction::{self, Action, RightButton};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
 
const ANCHOR_ITEM: &str = "respawn_anchor";
 
const ANCHOR_BLOCK: &str = "minecraft:respawn_anchor";
 
 
const GLOWSTONE: &str = "glowstone";
 
const GLOWSTONE_ITEM: &str = "minecraft:glowstone";

 
 
const STEP_TIMEOUT: Duration = Duration::from_millis(500);
 
 
 
const CHARGE_TIMEOUT: Duration = Duration::from_millis(800);

 
#[derive(Debug, Clone, Copy)]
struct Plan {
     
    anchor_slot: i32,
     
     
    glowstone_slot: i32,
     
    clicked: BlockPos,
     
    face: Direction,
     
    hit: Vec3,
     
    spot: BlockPos,
}

 
#[derive(Debug, Clone, Copy)]
enum Step {
     
     
    Place {
        anchor_slot: i32,
        glowstone_slot: i32,
        clicked: BlockPos,
        face: Direction,
        hit: Vec3,
        spot: BlockPos,
        deadline: Instant,
    },
     
     
    Charge {
        glowstone_slot: i32,
        spot: BlockPos,
        deadline: Instant,
    },
     
     
     
    Detonate {
        glowstone_slot: i32,
        fuse_slot: i32,
        spot: BlockPos,
        deadline: Instant,
    },
}

#[derive(Debug, Default)]
struct MacroState {
     
    right_was_down: bool,
    step: Option<Step>,
}

 
 
#[derive(Debug)]
pub struct AnchorMacroModule {
    pub module: ModuleData,
    state: Mutex<MacroState>,
}

impl AnchorMacroModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AnchorMacro,
                description: "Right-click while holding glowstone: places a respawn anchor, charges it, sets it off with another item and switches back to glowstone"
                    .to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
            state: Mutex::new(MacroState::default()),
        }
    }

     
     
     
    fn plan(
        &self,
        minecraft: &Minecraft,
        world: &World,
        inventory: &Inventory,
    ) -> anyhow::Result<Option<Plan>> {
         
         
        let glowstone_slot = inventory.get_selected()?;
        if !slot_holds(inventory, glowstone_slot, GLOWSTONE)? {
            return Ok(None);
        }
        let Some((clicked, face, hit)) = minecraft.hit_block_face_in_reach()? else {
            return Ok(None);
        };
         
         
        if world.block_registry_name(&clicked)? == ANCHOR_BLOCK {
            return Ok(None);
        }
        let (dx, dy, dz) = face.offset();
        let spot = BlockPos::new(clicked.x() + dx, clicked.y() + dy, clicked.z() + dz);
         
         
        if !world.is_block_air(&spot)? {
            return Ok(None);
        }
        let Some(anchor_slot) = find_hotbar_item(inventory, ANCHOR_ITEM)? else {
            return Ok(None);
        };
        Ok(Some(Plan {
            anchor_slot,
            glowstone_slot,
            clicked,
            face,
            hit,
            spot,
        }))
    }

     
     
    fn advance(
        &self,
        minecraft: &Minecraft,
        step: Step,
        player: &LocalPlayer,
        inventory: &Inventory,
        game_mode: &MultiPlayerGameMode,
        world: &World,
    ) -> anyhow::Result<Option<Step>> {
        match step {
            Step::Place {
                anchor_slot,
                glowstone_slot,
                clicked,
                face,
                hit,
                spot,
                deadline,
            } => {
                let selected = inventory.get_selected()?;
                if selected != anchor_slot {
                     
                     
                     
                     
                    if selected == glowstone_slot {
                        return Ok(keep_or_expire(step, deadline));
                    }
                    return Ok(None);
                }
                 
                 
                 
                if !slot_holds(inventory, glowstone_slot, GLOWSTONE)? {
                    return Ok(None);
                }
                 
                 
                if !slot_holds(inventory, anchor_slot, ANCHOR_ITEM)? {
                    return Ok(None);
                }
                 
                 
                 
                if !crosshair_reaches(player, &clicked)? {
                    return Ok(keep_or_expire(step, deadline));
                }
                 
                 
                if !world.is_block_air(&spot)? {
                    return Ok(None);
                }
                 
                if !interaction::take(player, Action::Block) {
                    return Ok(keep_or_expire(step, deadline));
                }
                game_mode.place_block_on(player, &clicked, face, hit)?;
                 
                 
                tap(glowstone_slot);
                Ok(Some(Step::Charge {
                    glowstone_slot,
                    spot,
                    deadline: Instant::now() + CHARGE_TIMEOUT,
                }))
            }
            Step::Charge {
                glowstone_slot,
                spot,
                deadline,
            } => {
                if inventory.get_selected()? != glowstone_slot
                    || !slot_holds(inventory, glowstone_slot, GLOWSTONE)?
                {
                    return Ok(keep_or_expire(step, deadline));
                }
                 
                 
                 
                if !is_anchor(world, &spot)? {
                    return Ok(keep_or_expire(step, deadline));
                }
                 
                 
                 
                let Some((hit_block, face, hit)) = minecraft.hit_block_face_in_reach()? else {
                    return Ok(keep_or_expire(step, deadline));
                };
                if hit_block != spot {
                    return Ok(keep_or_expire(step, deadline));
                }
                if !interaction::take(player, Action::Block) {
                    return Ok(keep_or_expire(step, deadline));
                }
                game_mode.place_block_on(player, &spot, face, hit)?;
                 
                 
                 
                 
                let Some(fuse_slot) = detonate_slot(inventory, glowstone_slot)? else {
                     
                     
                    return Ok(None);
                };
                tap(fuse_slot);
                Ok(Some(Step::Detonate {
                    glowstone_slot,
                    fuse_slot,
                    spot,
                    deadline: Instant::now() + STEP_TIMEOUT,
                }))
            }
            Step::Detonate {
                glowstone_slot,
                fuse_slot,
                spot,
                deadline,
            } => {
                 
                 
                 
                let wait = |step: Step| -> Option<Step> {
                    match keep_or_expire(step, deadline) {
                        Some(step) => Some(step),
                        None => {
                            tap(glowstone_slot);
                            None
                        }
                    }
                };
                let selected = inventory.get_selected()?;
                if selected != fuse_slot {
                     
                     
                     
                    if selected == glowstone_slot {
                        return Ok(wait(step));
                    }
                    return Ok(None);
                }
                 
                 
                 
                if !is_anchor(world, &spot)? {
                    tap(glowstone_slot);
                    return Ok(None);
                }
                 
                 
                 
                let Some((hit_block, face, hit)) = minecraft.hit_block_face_in_reach()? else {
                    return Ok(wait(step));
                };
                if hit_block != spot {
                    return Ok(wait(step));
                }
                if !interaction::take(player, Action::Block) {
                    return Ok(wait(step));
                }
                game_mode.place_block_on(player, &spot, face, hit)?;
                 
                 
                tap(glowstone_slot);
                Ok(None)
            }
        }
    }

     
     
    fn run_step(&self) -> anyhow::Result<()> {
        let minecraft = minecraft();
        if GUI_OPEN.load(Ordering::Relaxed) || minecraft.current_screen().is_open() {
            return self.stand_down();
        }
        let Some(player) = minecraft.player()? else {
            return Ok(());
        };
        if !player.is_alive().unwrap_or(false) {
            return self.stand_down();
        }
        let (Some(game_mode), Some(world)) = (minecraft.game_mode()?, minecraft.world()?) else {
            return self.stand_down();
        };
        let inventory = player.get_inventory()?;
         
         
        if other_loop_owns() {
            return self.stand_down();
        }
        let Some(step) = self.state.lock().unwrap().step else {
            return Ok(());
        };
        match self.advance(minecraft, step, &player, &inventory, &game_mode, &world) {
            Ok(next) => {
                let finished = next.is_none();
                self.state.lock().unwrap().step = next;
                if finished {
                    self.release();
                }
                Ok(())
            }
            Err(error) => {
                 
                 
                let _ = self.stand_down();
                Err(error)
            }
        }
    }

     
    fn release(&self) {
        interaction::release_right_button(RightButton::AnchorMacro);
    }

     
    fn stand_down(&self) -> anyhow::Result<()> {
        self.state.lock().unwrap().step = None;
        self.release();
        Ok(())
    }
}

impl Module for AnchorMacroModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.state.lock().unwrap().right_was_down = false;
        self.stand_down()
    }

     
    fn on_input_phase(&self) -> anyhow::Result<()> {
        if self.state.lock().unwrap().step.is_none() {
            return Ok(());
        }
        self.run_step()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let right_down = MOUSE_STATE
            .lock()
            .map(|mouse| mouse.right_down)
            .unwrap_or(false);
        let clicked = {
            let mut state = self.state.lock().unwrap();
            let clicked = right_down && !state.right_was_down;
            state.right_was_down = right_down;
            clicked
        };

        let minecraft = minecraft();
        if GUI_OPEN.load(Ordering::Relaxed) || minecraft.current_screen().is_open() {
            return self.stand_down();
        }
        let (Some(player), Some(world)) = (minecraft.player()?, minecraft.world()?) else {
            return self.stand_down();
        };
        if !player.is_alive().unwrap_or(false) {
            return self.stand_down();
        }
        let inventory = player.get_inventory()?;

         
         
        if other_loop_owns() {
            return self.stand_down();
        }

         
         
         
         
        let plan = self.plan(minecraft, &world, &inventory)?;
        let running = self.state.lock().unwrap().step.is_some();
        if running || plan.is_some() {
            interaction::claim_right_button(RightButton::AnchorMacro);
        } else {
            self.release();
        }

         
         
        if running {
            if !crate::jvmti::crystal_input_hook_installed() {
                self.run_step()?;
            }
            return Ok(());
        }

         
        if !clicked {
            return Ok(());
        }
        let Some(plan) = plan else {
            return Ok(());
        };
        press_hotbar_slot(plan.anchor_slot)?;
        self.state.lock().unwrap().step = Some(Step::Place {
            anchor_slot: plan.anchor_slot,
            glowstone_slot: plan.glowstone_slot,
            clicked: plan.clicked,
            face: plan.face,
            hit: plan.hit,
            spot: plan.spot,
            deadline: Instant::now() + STEP_TIMEOUT,
        });
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

 
 
 
fn other_loop_owns() -> bool {
    interaction::owns(RightButton::CrystalSpam)
}

 
fn is_anchor(world: &World, pos: &BlockPos) -> anyhow::Result<bool> {
    Ok(world.block_registry_name(pos)? == ANCHOR_BLOCK)
}

 
fn slot_holds(inventory: &Inventory, slot: i32, item: &str) -> anyhow::Result<bool> {
    let stack = inventory.get_item(slot)?;
    if stack.is_empty()? {
        return Ok(false);
    }
    Ok(stack.get_item()?.registry_name()? == format!("minecraft:{item}"))
}

 
 
fn tap(slot: i32) {
    if let Err(error) = press_hotbar_slot(slot) {
        log::debug!(
            "Anchor macro: could not tap hotbar slot {}: {error}",
            slot + 1
        );
    }
}

 
 
fn detonate_slot(inventory: &Inventory, glowstone_slot: i32) -> anyhow::Result<Option<i32>> {
    let mut names = Vec::with_capacity(9);
    for slot in 0..9 {
        let stack = inventory.get_item(slot)?;
        names.push(if stack.is_empty()? {
            None
        } else {
            Some(stack.get_item()?.registry_name()?)
        });
    }
    Ok(pick_fuse_slot(&names, glowstone_slot, GLOWSTONE_ITEM))
}

 
 
 
 
fn pick_fuse_slot(names: &[Option<String>], glowstone_slot: i32, glowstone: &str) -> Option<i32> {
    let mut fallback = None;
    for (slot, name) in names.iter().enumerate() {
        let slot = slot as i32;
        if slot == glowstone_slot {
            continue;
        }
        match name {
            None => return Some(slot),
            Some(name) if name.as_str() == glowstone => continue,
            Some(_) if fallback.is_none() => fallback = Some(slot),
            Some(_) => {}
        }
    }
    fallback
}

 
fn keep_or_expire(step: Step, deadline: Instant) -> Option<Step> {
    if Instant::now() >= deadline {
        None
    } else {
        Some(step)
    }
}

#[cfg(test)]
mod tests {
    use super::{keep_or_expire, pick_fuse_slot, AnchorMacroModule, Step, GLOWSTONE_ITEM};
    use crate::mapping::math::BlockPos;
    use crate::module::{Module, ModuleId};
    use std::time::{Duration, Instant};

     
    #[test]
    fn the_module_ships_as_a_settings_free_combat_macro() {
        let mut module = AnchorMacroModule::new();
        let data = module.get_module_data_mut();
        assert_eq!(data.id, ModuleId::AnchorMacro);
        assert_eq!(data.id.display_name(), "Anchor Macro");
        assert!(!data.enabled);
        assert!(data.settings.is_empty());
    }

     
     
     
    #[test]
    fn a_step_expires_at_its_deadline() {
        let deadline = Instant::now() + Duration::from_millis(50);
        let step = Step::Charge {
            glowstone_slot: 2,
            spot: BlockPos::new(1, 64, 1),
            deadline,
        };
        assert!(keep_or_expire(step, deadline).is_some());
        assert!(keep_or_expire(step, Instant::now() - Duration::from_millis(1)).is_none());
    }

     
     
    #[test]
    fn an_empty_slot_is_preferred_for_the_set_off_click() {
        let names = vec![
            Some("minecraft:glowstone".to_string()),
            Some("minecraft:glowstone".to_string()),
            Some("minecraft:netherite_sword".to_string()),
            None,
            Some("minecraft:obsidian".to_string()),
        ];
        assert_eq!(pick_fuse_slot(&names, 0, GLOWSTONE_ITEM), Some(3));
    }

     
    #[test]
    fn a_full_hotbar_sets_the_anchor_off_with_another_item() {
        let names = vec![
            Some("minecraft:glowstone".to_string()),
            Some("minecraft:netherite_axe".to_string()),
            Some("minecraft:obsidian".to_string()),
        ];
        assert_eq!(pick_fuse_slot(&names, 0, GLOWSTONE_ITEM), Some(1));
    }

     
     
     
    #[test]
    fn glowstone_never_sets_an_anchor_off() {
        let names = vec![
            Some("minecraft:glowstone".to_string()),
            Some("minecraft:glowstone".to_string()),
        ];
        assert_eq!(pick_fuse_slot(&names, 0, GLOWSTONE_ITEM), None);
        assert_eq!(pick_fuse_slot(&[None], 0, GLOWSTONE_ITEM), None);
    }
}
