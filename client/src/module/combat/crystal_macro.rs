use crate::mapping::block::Direction;
use crate::mapping::client::options::KeyMapping;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::inventory::Inventory;
use crate::mapping::math::{BlockPos, Vec3};
use crate::module::combat::interaction::{self, Action};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use jni::objects::{JObject, JValue};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
 
pub(crate) const END_CRYSTAL: &str = "end_crystal";
const OBSIDIAN: &str = "obsidian";

 
pub(crate) static ACTIVE_CRYSTAL_MACROS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Default)]
struct ServerPlacementConfirmation {
    pending: Option<BlockPos>,
    confirmed: Option<BlockPos>,
}

 
 
static SERVER_PLACEMENT_CONFIRMATION: Mutex<ServerPlacementConfirmation> =
    Mutex::new(ServerPlacementConfirmation {
        pending: None,
        confirmed: None,
    });

 
 
 
 
const BLOCK_REACH: f64 = 4.5;

 
 
const SLOT_TIMEOUT: Duration = Duration::from_millis(500);
const DEFAULT_CONFIRMATION_TIMEOUT_MS: f32 = 500.0;

 
#[derive(Debug, Clone, Copy)]
enum Step {
     
     
    SelectObsidian {
        slot: i32,
        crystal_slot: i32,
        anchor: BlockPos,
        face: Direction,
        hit: Vec3,
        base: BlockPos,
        due: Instant,
        deadline: Instant,
    },
     
     
    WaitForObsidian {
        slot: i32,
        crystal_slot: i32,
        base: BlockPos,
        deadline: Instant,
    },
     
     
     
    SelectCrystal {
        slot: i32,
        base: BlockPos,
        due: Instant,
        deadline: Instant,
    },
}

#[derive(Debug, Default)]
struct MacroState {
    step: Option<Step>,
     
     
     
     
     
    restore: Option<(i32, i32)>,
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
#[derive(Debug)]
pub struct CrystalMacroModule {
    pub module: ModuleData,
    state: Mutex<MacroState>,
    active: AtomicBool,
}

impl CrystalMacroModule {
    pub fn new() -> Self {
         
        let settings = vec![
            ModuleSetting::Slider {
                name: "Delay".to_string(),
                value: 50.0,
                min: 0.0,
                max: 500.0,
            },
            ModuleSetting::Toggle {
                name: "Zero Delay".to_string(),
                value: true,
            },
            ModuleSetting::Toggle {
                name: "Auto Place Obsidian".to_string(),
                value: true,
            },
            ModuleSetting::Slider {
                name: "Hotbar Timeout".to_string(),
                value: SLOT_TIMEOUT.as_millis() as f32,
                min: 100.0,
                max: 3000.0,
            },
            ModuleSetting::Toggle {
                name: "Wait for Server Confirmation".to_string(),
                value: false,
            },
            ModuleSetting::Slider {
                name: "Server Confirm Timeout".to_string(),
                value: DEFAULT_CONFIRMATION_TIMEOUT_MS,
                min: 100.0,
                max: 3000.0,
            },
            ModuleSetting::Toggle {
                name: "Restore Crystal on Cancel".to_string(),
                value: true,
            },
        ];
        Self {
            module: ModuleData {
                id: ModuleId::CrystalMacro,
                description:
                    "Right-click with an end crystal: place obsidian when the clicked block is not a base, then place a crystal"
                        .to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings,
            },
            state: Mutex::new(MacroState::default()),
            active: AtomicBool::new(false),
        }
    }

     
     
    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_toggle_value())
            .unwrap_or(fallback)
    }

    fn slider(&self, name: &str, fallback: f32) -> f32 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(fallback)
    }

     
    fn delay(&self) -> Duration {
        if self.zero_delays() {
            return Duration::ZERO;
        }
        Duration::from_millis(self.slider("Delay", 50.0).max(0.0) as u64)
    }

     
     
    fn zero_delays(&self) -> bool {
        self.toggle("Zero Delay", true)
    }

     
     
    fn auto_place_obsidian(&self) -> bool {
        self.toggle("Auto Place Obsidian", true)
    }

    fn setting_duration(&self, name: &str, fallback: Duration) -> Duration {
        let millis = self
            .slider(name, fallback.as_millis() as f32)
            .clamp(100.0, 3000.0);
        Duration::from_millis(millis as u64)
    }

    fn hotbar_timeout(&self) -> Duration {
        self.setting_duration("Hotbar Timeout", SLOT_TIMEOUT)
    }

    fn confirmation_timeout(&self) -> Duration {
        self.setting_duration(
            "Server Confirm Timeout",
            Duration::from_millis(DEFAULT_CONFIRMATION_TIMEOUT_MS as u64),
        )
    }

    fn require_server_confirmation(&self) -> bool {
        self.toggle("Wait for Server Confirmation", false)
    }

    fn restore_on_cancel(&self) -> bool {
        self.toggle("Restore Crystal on Cancel", true)
    }

     
     
    fn advance(
        &self,
        step: Step,
        inventory: &Inventory,
        game_mode: &crate::mapping::client::gamemode::MultiPlayerGameMode,
        player: &LocalPlayer,
        world: &crate::mapping::client::world::World,
    ) -> anyhow::Result<Option<Step>> {
        match step {
            Step::SelectObsidian {
                slot,
                crystal_slot,
                anchor,
                face,
                hit,
                base,
                due,
                deadline,
            } => {
                let selected = inventory.get_selected()?;
                if selected != slot {
                     
                     
                     
                    if selected == crystal_slot {
                        return Ok(keep_or_expire(step, deadline));
                    }
                    self.restore_selection(inventory)?;
                    return Ok(None);
                }
                 
                 
                 
                 
                let Some(crystal_slot) = crystal_slot_for(inventory, crystal_slot)? else {
                    self.restore_selection(inventory)?;
                    return Ok(None);
                };
                 
                 
                self.retarget_restore_crystal(crystal_slot);
                 
                 
                if !slot_holds_obsidian(inventory, slot)? {
                    self.restore_selection(inventory)?;
                    return Ok(None);
                }
                 
                 
                 
                if !crosshair_reaches(player, &anchor)? {
                    return Ok(keep_or_expire(step, deadline));
                }
                if Instant::now() < due {
                    return Ok(keep_or_expire(step, deadline));
                }
                 
                 
                 
                if !interaction::take(player, Action::Block) {
                    return Ok(keep_or_expire(step, deadline));
                }
                 
                 
                 
                 
                 
                 
                let wait_for_server = self.require_server_confirmation();
                if wait_for_server {
                    watch_server_placement(base);
                }
                game_mode.place_block_on(player, &anchor, face, hit)?;
                if wait_for_server {
                     
                    return Ok(Some(Step::WaitForObsidian {
                        slot,
                        crystal_slot,
                        base,
                        deadline: Instant::now() + self.confirmation_timeout(),
                    }));
                }
                 
                 
                 
                 
                 
                 
                 
                press_hotbar_slot(crystal_slot)?;
                let due = Instant::now() + self.delay();
                Ok(Some(Step::SelectCrystal {
                    slot: crystal_slot,
                    base,
                    due,
                    deadline: due + self.hotbar_timeout(),
                }))
            }
            Step::WaitForObsidian {
                slot,
                crystal_slot,
                base,
                deadline,
            } => {
                if inventory.get_selected()? != slot {
                     
                     
                    self.clear_restore();
                    return Ok(None);
                }
                if !slot_holds_obsidian(inventory, slot)? {
                    self.clear_restore();
                    return Ok(None);
                }
                if self.require_server_confirmation() && !server_confirmed_placement(base) {
                    if Instant::now() >= deadline {
                         
                         
                         
                        clear_server_placement_confirmation();
                        return Ok(None);
                    }
                    return Ok(Some(step));
                }
                if !is_obsidian(world, &base)? {
                    if Instant::now() >= deadline {
                         
                         
                        clear_server_placement_confirmation();
                        return Ok(None);
                    }
                    return Ok(Some(step));
                }
                let Some(crystal_slot) = crystal_slot_for(inventory, crystal_slot)? else {
                     
                     
                    clear_server_placement_confirmation();
                    self.clear_restore();
                    return Ok(None);
                };
                self.retarget_restore_crystal(crystal_slot);
                clear_server_placement_confirmation();
                press_hotbar_slot(crystal_slot)?;
                let due = Instant::now() + self.delay();
                Ok(Some(Step::SelectCrystal {
                    slot: crystal_slot,
                    base,
                    due,
                    deadline: due + self.hotbar_timeout(),
                }))
            }
            Step::SelectCrystal {
                slot,
                base,
                due,
                deadline,
            } => {
                if inventory.get_selected()? != slot {
                     
                     
                     
                     
                     
                    return Ok(keep_or_expire(step, deadline));
                }
                 
                 
                self.clear_restore();
                if Instant::now() < due {
                    return Ok(Some(step));
                }
                if place_crystal(inventory, game_mode, player, world, base)? {
                    return Ok(None);
                }
                Ok(keep_or_expire(step, deadline))
            }
        }
    }

     
     
     
     
    fn abandon(&self) -> anyhow::Result<()> {
        self.state.lock().unwrap().step = None;
        clear_server_placement_confirmation();
        interaction::release_right_button(interaction::RightButton::CrystalMacro);
        match minecraft().player() {
            Ok(Some(player)) => {
                let inventory = player.get_inventory()?;
                if self.restore_on_cancel() {
                    self.restore_selection(&inventory)
                } else {
                    self.clear_restore();
                    Ok(())
                }
            }
             
             
            _ => {
                self.clear_restore();
                Ok(())
            }
        }
    }

     
     
    fn restore_selection(&self, inventory: &Inventory) -> anyhow::Result<()> {
        let Some((tapped, crystal_slot)) = self.state.lock().unwrap().restore else {
            return Ok(());
        };
        let holds_crystal = {
            let stack = inventory.get_item(crystal_slot)?;
            !stack.is_empty()? && is_item_named(&stack.get_item()?.registry_name()?, END_CRYSTAL)
        };
        let target = restore_target(
            inventory.get_selected()?,
            tapped,
            crystal_slot,
            holds_crystal,
        );
        self.clear_restore();
        if let Some(slot) = target {
            press_hotbar_slot(slot)?;
        }
        Ok(())
    }

     
     
     
    fn retarget_restore_crystal(&self, crystal_slot: i32) {
        let mut state = self.state.lock().unwrap();
        if let Some((_, saved)) = state.restore.as_mut() {
            *saved = crystal_slot;
        }
    }

     
    fn clear_restore(&self) {
        self.state.lock().unwrap().restore = None;
    }

     
     
     
    fn run_step(&self) -> anyhow::Result<()> {
        let minecraft = minecraft();
        if crate::graphic::input::GUI_OPEN.load(std::sync::atomic::Ordering::Relaxed)
            || minecraft.current_screen().is_open()
        {
            return self.abandon();
        }
        let Some(player) = minecraft.player()? else {
            return Ok(());
        };
        if !player.is_alive().unwrap_or(false) {
            return self.abandon();
        }
        let (Some(game_mode), Some(world)) = (minecraft.game_mode()?, minecraft.world()?) else {
            return Ok(());
        };
        let inventory = player.get_inventory()?;
        let Some(step) = self.state.lock().unwrap().step else {
            return Ok(());
        };
         
         
         
         
         
        if interaction::owns(interaction::RightButton::CrystalSpam)
            || interaction::owns(interaction::RightButton::AnchorMacro)
        {
            return self.abandon();
        }
        match self.advance(step, &inventory, &game_mode, &player, &world) {
            Ok(next) => {
                let finished = next.is_none();
                self.state.lock().unwrap().step = next;
                if finished {
                     
                     
                     
                     
                     
                     
                    interaction::release_right_button(interaction::RightButton::CrystalMacro);
                    if self.restore_on_cancel() {
                        return self.restore_selection(&inventory);
                    }
                    self.clear_restore();
                }
                Ok(())
            }
            Err(error) => {
                 
                 
                 
                let _ = self.abandon();
                Err(error)
            }
        }
    }
}

 
 
 
 
fn restore_target(
    selected: i32,
    tapped: i32,
    crystal_slot: i32,
    holds_crystal: bool,
) -> Option<i32> {
    (selected == tapped && holds_crystal).then_some(crystal_slot)
}

 
 
 
fn crystal_slot_for(inventory: &Inventory, preferred: i32) -> anyhow::Result<Option<i32>> {
    let stack = inventory.get_item(preferred)?;
    if !stack.is_empty()? && is_item_named(&stack.get_item()?.registry_name()?, END_CRYSTAL) {
        return Ok(Some(preferred));
    }
    find_hotbar_item(inventory, END_CRYSTAL)
}

 
fn keep_or_expire(step: Step, deadline: Instant) -> Option<Step> {
    if Instant::now() >= deadline {
        None
    } else {
        Some(step)
    }
}

impl Module for CrystalMacroModule {
    fn on_start(&self) -> anyhow::Result<()> {
        if !self.active.swap(true, Ordering::AcqRel) {
            ACTIVE_CRYSTAL_MACROS.fetch_add(1, Ordering::AcqRel);
        }
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        if self.active.swap(false, Ordering::AcqRel) {
            ACTIVE_CRYSTAL_MACROS.fetch_sub(1, Ordering::AcqRel);
        }
         
         
        let _ = self.abandon();
        *self.state.lock().unwrap() = MacroState::default();
        Ok(())
    }

     
    fn on_input_phase(&self) -> anyhow::Result<()> {
        if self.state.lock().unwrap().step.is_none() {
            return Ok(());
        }
        self.run_step()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
         
        let right_clicked = crate::graphic::input::MOUSE_STATE
            .lock()
            .map(|mut mouse| {
                let clicked = mouse.right_clicked;
                mouse.right_clicked = false;
                clicked
            })
            .unwrap_or(false);

         
        let minecraft = minecraft();
        if crate::graphic::input::GUI_OPEN.load(std::sync::atomic::Ordering::Relaxed)
            || minecraft.current_screen().is_open()
        {
            return self.abandon();
        }
        let (Some(player), Some(world)) = (minecraft.player()?, minecraft.world()?) else {
            return Ok(());
        };

        let inventory = player.get_inventory()?;

         
         
         
         
        if self.state.lock().unwrap().step.is_some() {
            if !crate::jvmti::crystal_input_hook_installed() {
                self.run_step()?;
            }
            return Ok(());
        }

         
         
         
         
         
        if !right_clicked
            || interaction::owns(interaction::RightButton::CrystalSpam)
            || interaction::owns(interaction::RightButton::AnchorMacro)
            || !held_item_is(&inventory, END_CRYSTAL)?
        {
            return Ok(());
        }
        let crystal_slot = inventory.get_selected()?;

         
        let Some((anchor, face, hit)) = minecraft.hit_block_face_in_reach()? else {
            return Ok(());
        };
        let (dx, dy, dz) = face.offset();
        let base = BlockPos::new(anchor.x() + dx, anchor.y() + dy, anchor.z() + dz);

         
         
         
        let existing = existing_base(
            &anchor,
            &base,
            is_base_block(&world, &anchor)?,
            is_base_block(&world, &base)?,
        );

         
         
        let crystal_on = existing.unwrap_or(base);
        if existing.is_some()
            && !crate::module::combat::crystal_spam::crystals_on(&world, &crystal_on)?.is_empty()
        {
            return Ok(());
        }

        if let Some(existing) = existing {
             
             
             
            interaction::claim_right_button(interaction::RightButton::CrystalMacro);
            self.state.lock().unwrap().step = Some(Step::SelectCrystal {
                slot: crystal_slot,
                base: existing,
                due: Instant::now(),
                deadline: Instant::now() + self.hotbar_timeout(),
            });
            return Ok(());
        }

         
         
        if !self.auto_place_obsidian() || !world.is_block_air(&base)? {
            return Ok(());
        }
        let Some(base_slot) = find_hotbar_item(&inventory, OBSIDIAN)? else {
            return Ok(());
        };
         
         
        interaction::claim_right_button(interaction::RightButton::CrystalMacro);
        if base_slot != crystal_slot {
            press_hotbar_slot(base_slot)?;
        }
        let mut state = self.state.lock().unwrap();
         
         
        state.restore = Some((base_slot, crystal_slot));
        state.step = Some(Step::SelectObsidian {
            slot: base_slot,
            crystal_slot,
            anchor,
            face,
            hit,
            base,
            due: Instant::now(),
            deadline: Instant::now() + self.hotbar_timeout(),
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

 
 
 
pub(crate) fn capture_right_click() -> bool {
    if ACTIVE_CRYSTAL_MACROS.load(Ordering::Acquire) == 0 {
        return false;
    }
    if crate::graphic::input::GUI_OPEN.load(Ordering::Relaxed)
        || crate::graphic::edit_ui::is_open()
        || minecraft().current_screen().is_open()
    {
        return false;
    }
    let Ok(Some(player)) = minecraft().player() else {
        return false;
    };
    let Ok(inventory) = player.get_inventory() else {
        return false;
    };
    held_item_is(&inventory, END_CRYSTAL).unwrap_or(false)
        && find_hotbar_item(&inventory, OBSIDIAN)
            .ok()
            .flatten()
            .is_some()
}

 
 
pub(crate) fn awaiting_server_placement_confirmation() -> bool {
    SERVER_PLACEMENT_CONFIRMATION
        .lock()
        .map(|confirmation| confirmation.pending.is_some())
        .unwrap_or(false)
}

 
 
 
pub(crate) fn note_server_block_update(packet: &JObject) -> anyhow::Result<()> {
    let update = crate::state::mapping().in_frame(|| {
        let packet_class = crate::mapping::MinecraftClassType::ClientboundBlockUpdatePacket;
        let pos_object = crate::state::mapping()
            .call_method(packet_class, packet, "getPos", &[])?
            .l()?;
        let pos = BlockPos::read(&pos_object)?;
        if !is_pending_server_placement(pos) {
            return Ok(None);
        }
        let state = crate::state::mapping()
            .call_method(packet_class, packet, "getBlockState", &[])?
            .l()?;
        let block = crate::mapping::chunk::state_block(&state)?;
        let registry = crate::mapping::chunk::block_registry()?;
        let key = crate::state::mapping()
            .call_method(
                crate::mapping::MinecraftClassType::Registry,
                registry.as_obj(),
                "getKey",
                &[JValue::Object(&block)],
            )?
            .l()?;
        let name = crate::state::mapping()
            .call_method(
                crate::mapping::MinecraftClassType::ResourceLocation,
                &key,
                "toString",
                &[],
            )?
            .l()?;
        Ok(Some((pos, crate::state::mapping().get_string(name)?)))
    })?;
    if let Some((pos, name)) = update {
        if is_item_named(&name, OBSIDIAN) {
            confirm_server_placement(pos);
        }
    }
    Ok(())
}

fn watch_server_placement(base: BlockPos) {
    let mut confirmation = SERVER_PLACEMENT_CONFIRMATION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    confirmation.pending = Some(base);
    confirmation.confirmed = None;
}

fn is_pending_server_placement(base: BlockPos) -> bool {
    SERVER_PLACEMENT_CONFIRMATION
        .lock()
        .map(|confirmation| confirmation.pending == Some(base))
        .unwrap_or(false)
}

fn confirm_server_placement(base: BlockPos) {
    let mut confirmation = SERVER_PLACEMENT_CONFIRMATION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if confirmation.pending == Some(base) {
        confirmation.confirmed = Some(base);
    }
}

fn server_confirmed_placement(base: BlockPos) -> bool {
    SERVER_PLACEMENT_CONFIRMATION
        .lock()
        .map(|confirmation| {
            confirmation.pending == Some(base) && confirmation.confirmed == Some(base)
        })
        .unwrap_or(false)
}

fn clear_server_placement_confirmation() {
    let mut confirmation = SERVER_PLACEMENT_CONFIRMATION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *confirmation = ServerPlacementConfirmation::default();
}

 
pub(crate) fn press_hotbar_slot(slot: i32) -> anyhow::Result<()> {
    KeyMapping::click_named(&format!("key.keyboard.{}", slot + 1))
}

 
 
 
 
 
fn place_crystal(
    inventory: &Inventory,
    game_mode: &crate::mapping::client::gamemode::MultiPlayerGameMode,
    player: &LocalPlayer,
    world: &crate::mapping::client::world::World,
    base: BlockPos,
) -> anyhow::Result<bool> {
    if !held_item_is(inventory, END_CRYSTAL)? {
        return Ok(false);
    }
    if !is_base_block(world, &base)? {
        return Ok(false);
    }
    let above = BlockPos::new(base.x(), base.y() + 1, base.z());
    if !world.is_block_air(&above)? {
        return Ok(false);
    }
     
     
     
    if !crosshair_reaches(player, &base)? {
        return Ok(false);
    }
    let hit = Vec3::new(
        f64::from(base.x()) + 0.5,
        f64::from(base.y()) + 1.0,
        f64::from(base.z()) + 0.5,
    );
     
     
    if !interaction::take(player, Action::Block) {
        return Ok(false);
    }
    game_mode.place_block_on(player, &base, Direction::Up, hit)?;
    Ok(true)
}

 
fn is_base_block_name(name: &str) -> bool {
    name == "minecraft:obsidian" || name == "minecraft:bedrock"
}

 
pub(crate) fn is_base_block(
    world: &crate::mapping::client::world::World,
    pos: &BlockPos,
) -> anyhow::Result<bool> {
    Ok(is_base_block_name(&world.block_registry_name(pos)?))
}

 
 
 
fn existing_base(
    anchor: &BlockPos,
    face_base: &BlockPos,
    anchor_is_base: bool,
    face_base_is_base: bool,
) -> Option<BlockPos> {
    if face_base_is_base {
        Some(*face_base)
    } else if anchor_is_base {
        Some(*anchor)
    } else {
        None
    }
}

 
 
 
 
pub(crate) fn crosshair_reaches(player: &LocalPlayer, pos: &BlockPos) -> anyhow::Result<bool> {
    aim_reaches(
        player,
        pos,
        crate::module::combat::rotation::Rotation {
            yaw: player.get_yaw()?,
            pitch: player.get_pitch()?,
        },
    )
}

 
 
pub(crate) fn aim_reaches(
    player: &LocalPlayer,
    pos: &BlockPos,
    aim: crate::module::combat::rotation::Rotation,
) -> anyhow::Result<bool> {
    let eye = player.get_eye_position()?;
    let direction = super::look_dir(aim.yaw, aim.pitch);
    let (x, y, z) = (f64::from(pos.x()), f64::from(pos.y()), f64::from(pos.z()));
    Ok(super::ray_hits_box(
        [eye.x(), eye.y(), eye.z()],
        direction,
        [x, y, z],
        [x + 1.0, y + 1.0, z + 1.0],
        BLOCK_REACH,
    ))
}

 
fn slot_holds_obsidian(inventory: &Inventory, slot: i32) -> anyhow::Result<bool> {
    let stack = inventory.get_item(slot)?;
    if stack.is_empty()? {
        return Ok(false);
    }
    Ok(is_item_named(&stack.get_item()?.registry_name()?, OBSIDIAN))
}

 
pub(crate) fn held_item_is(inventory: &Inventory, item: &str) -> anyhow::Result<bool> {
    let held = inventory.get_item(inventory.get_selected()?)?;
    if held.is_empty()? {
        return Ok(false);
    }
    Ok(is_item_named(&held.get_item()?.registry_name()?, item))
}

 
fn is_obsidian(
    world: &crate::mapping::client::world::World,
    pos: &BlockPos,
) -> anyhow::Result<bool> {
    Ok(is_item_named(&world.block_registry_name(pos)?, OBSIDIAN))
}

 
fn is_item_named(registry_name: &str, item: &str) -> bool {
    registry_name == format!("minecraft:{item}")
}

 
pub(crate) fn find_hotbar_item(inventory: &Inventory, item: &str) -> anyhow::Result<Option<i32>> {
    for slot in 0..9 {
        let stack = inventory.get_item(slot)?;
        if stack.is_empty()? {
            continue;
        }
        if is_item_named(&stack.get_item()?.registry_name()?, item) {
            return Ok(Some(slot));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{
        clear_server_placement_confirmation, confirm_server_placement, existing_base,
        is_base_block_name, is_item_named, server_confirmed_placement, watch_server_placement,
        CrystalMacroModule, SLOT_TIMEOUT,
    };
    use crate::mapping::math::BlockPos;
    use crate::module::ModuleId;

    #[test]
    fn the_macro_exposes_its_sequence_settings_with_zero_delay_on() {
        let mut module = CrystalMacroModule::new();
        assert_eq!(module.module.id, ModuleId::CrystalMacro);
        assert_eq!(module.module.name(), "Crystal Macro");
        assert_eq!(module.module.settings.len(), 7);

         
        assert!(module.zero_delays());
        assert_eq!(module.delay(), std::time::Duration::ZERO);
        assert_eq!(module.hotbar_timeout(), SLOT_TIMEOUT);
        assert!(module.auto_place_obsidian());
        assert!(!module.require_server_confirmation());

         
        let real = CrystalMacroModule::new();
        assert_eq!(real.module.id, ModuleId::CrystalMacro);
        assert!(real.zero_delays());
        assert!(real.auto_place_obsidian());
        assert_eq!(real.module.settings.len(), 7);

         
        module
            .module
            .get_setting_mut("Zero Delay")
            .unwrap()
            .set_toggle_value(false);
        assert!(!module.zero_delays());
        assert_eq!(module.delay(), std::time::Duration::from_millis(50));

        module
            .module
            .get_setting_mut("Auto Place Obsidian")
            .unwrap()
            .set_toggle_value(false);
        assert!(!module.auto_place_obsidian());

        module
            .module
            .get_setting_mut("Wait for Server Confirmation")
            .unwrap()
            .set_toggle_value(true);
        assert!(module.require_server_confirmation());
    }

    #[test]
    fn item_match_is_exact_so_crying_obsidian_is_not_obsidian() {
        assert!(is_item_named("minecraft:obsidian", "obsidian"));
        assert!(!is_item_named("minecraft:crying_obsidian", "obsidian"));
        assert!(is_item_named("minecraft:end_crystal", "end_crystal"));
    }

    #[test]
    fn crystal_bases_are_obsidian_or_bedrock_only() {
        assert!(is_base_block_name("minecraft:obsidian"));
        assert!(is_base_block_name("minecraft:bedrock"));
        assert!(!is_base_block_name("minecraft:crying_obsidian"));
        assert!(!is_base_block_name("minecraft:stone"));
        assert!(!is_base_block_name(""));
    }

    #[test]
    fn an_existing_base_is_reused_instead_of_spending_a_block() {
        let anchor = BlockPos::new(1, 64, 1);
        let face_base = BlockPos::new(1, 65, 1);
         
        assert_eq!(
            existing_base(&anchor, &face_base, false, true),
            Some(face_base)
        );
        assert_eq!(
            existing_base(&anchor, &face_base, true, true),
            Some(face_base)
        );
         
        assert_eq!(
            existing_base(&anchor, &face_base, true, false),
            Some(anchor)
        );
        assert_eq!(existing_base(&anchor, &face_base, false, false), None);
    }

     
     
     
     
    #[test]
    fn an_abandoned_sequence_switches_back_to_the_crystal() {
        assert_eq!(super::restore_target(3, 3, 0, true), Some(0));
        assert_eq!(
            super::restore_target(5, 3, 0, true),
            None,
            "the player is on a slot the macro did not pick"
        );
        assert_eq!(
            super::restore_target(3, 3, 0, false),
            None,
            "the crystal stack is gone: nothing to switch back to"
        );
    }

    #[test]
    fn a_predicted_obsidian_does_not_confirm_the_base_placement() {
        let base = BlockPos::new(4, 64, -2);
        clear_server_placement_confirmation();
        watch_server_placement(base);

        assert!(!server_confirmed_placement(base));
        confirm_server_placement(base.offset(1, 0, 0));
        assert!(!server_confirmed_placement(base));

         
         
        confirm_server_placement(base);
        assert!(server_confirmed_placement(base));
        clear_server_placement_confirmation();
        assert!(!server_confirmed_placement(base));
    }

     
     
    #[test]
    fn the_crosshair_gate_rejects_blocks_behind_or_out_of_reach() {
        let eye = [0.5, 1.62, 0.5];
         
        let look = super::super::look_dir(0.0, 0.0);
        assert!(super::super::ray_hits_box(
            eye,
            look,
            [0.0, 1.0, 2.0],
            [1.0, 2.0, 3.0],
            super::BLOCK_REACH
        ));
        assert!(!super::super::ray_hits_box(
            eye,
            look,
            [0.0, 1.0, -3.0],
            [1.0, 2.0, -2.0],
            super::BLOCK_REACH
        ));
        assert!(!super::super::ray_hits_box(
            eye,
            look,
            [0.0, 1.0, 10.0],
            [1.0, 2.0, 11.0],
            super::BLOCK_REACH
        ));
    }
}
