 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::block::Direction;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::{EndCrystal, Entity, EntityRef, LivingEntityRef};
use crate::mapping::inventory::Inventory;
use crate::mapping::math::{BlockPos, Vec3};
use crate::mapping::MappedObject;
use crate::module::combat::crystal_macro::{
    aim_reaches, crosshair_reaches, find_hotbar_item, held_item_is, is_base_block,
    press_hotbar_slot, END_CRYSTAL,
};
use crate::module::combat::interaction::{self, Action};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::net::packet::{Packet, PacketAction};
use crate::state::minecraft;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

 
 
 
 
const DEFAULT_DELAY_MS: f32 = 0.0;

 
 
 
 
const DEFAULT_POP_RETRY_MS: f32 = 100.0;

 
 
const CRYSTAL_HALF_WIDTH: f64 = 1.0;
const CRYSTAL_HEIGHT: f64 = 2.0;

 
 
 
 
const SPAWN_TIMEOUT: Duration = Duration::from_millis(500);

 
 
const SILENT_AIM_SPEED: f32 = 400.0;

 
 
static TAKEOVER_REPORTED: AtomicBool = AtomicBool::new(false);

 
 
 
 
static SILENT_AIMED: AtomicBool = AtomicBool::new(false);

 
 
 
 
 
pub(crate) fn owns_right_button() -> bool {
    interaction::owns_right_button()
}

 
#[derive(Debug)]
enum Target {
     
    Crystal(Entity),
     
     
    Base(BlockPos, Direction, Vec3),
}

#[derive(Debug, Default)]
struct SpamState {
     
     
    single_click: bool,
     
     
     
    active_base: Option<BlockPos>,
     
    placed_at: Option<Instant>,
     
     
     
     
    awaiting: Option<Instant>,
     
     
    attacked: Option<(i32, Instant)>,
}

 
 
#[derive(Debug)]
pub struct CrystalSpamModule {
    pub module: ModuleData,
    state: Mutex<SpamState>,
}

impl CrystalSpamModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::CrystalSpam,
                description: "Hold right-click on obsidian or bedrock: places an end crystal from your hotbar and pops it, over and over".to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Slider {
                        name: "Delay (ms)".to_string(),
                        value: DEFAULT_DELAY_MS,
                        min: 0.0,
                        max: 1000.0,
                    },
                    ModuleSetting::Slider {
                        name: "Pop Retry (ms)".to_string(),
                        value: DEFAULT_POP_RETRY_MS,
                        min: 0.0,
                        max: 1000.0,
                    },
                ],
            },
            state: Mutex::new(SpamState::default()),
        }
    }

    fn slider(&self, name: &str, fallback: f32) -> f32 {
        self.module
            .get_setting(name)
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(fallback)
    }

     
     
    fn delay(&self) -> Duration {
        Duration::from_millis(
            self.slider("Delay (ms)", DEFAULT_DELAY_MS)
                .clamp(0.0, 1000.0) as u64,
        )
    }

     
    fn pop_retry(&self) -> Duration {
        Duration::from_millis(
            self.slider("Pop Retry (ms)", DEFAULT_POP_RETRY_MS)
                .clamp(0.0, 1000.0) as u64,
        )
    }

     
     
     
    fn placement_due(&self, now: Instant) -> bool {
        let state = self.state.lock().unwrap();
        may_place(
            state.awaiting,
            state.placed_at,
            SPAWN_TIMEOUT,
            self.delay(),
            now,
        )
    }

     
    fn attacked_id(&self) -> Option<i32> {
        self.state.lock().unwrap().attacked.map(|(id, _)| id)
    }

     
     
    fn pop_due(&self, id: i32, now: Instant) -> bool {
        match self.state.lock().unwrap().attacked {
            Some((attacked, at)) => {
                attacked != id || now.saturating_duration_since(at) >= self.pop_retry()
            }
            None => true,
        }
    }

     
     
     
     
     
    fn engage(&self) -> anyhow::Result<()> {
        if interaction::owns(interaction::RightButton::CrystalSpam) {
            return Ok(());
        }
        interaction::claim_right_button(interaction::RightButton::CrystalSpam);
        if TAKEOVER_REPORTED.swap(true, Ordering::AcqRel) {
            log::debug!("Crystal spam: right button taken over");
        } else {
            log::info!(
                "Crystal spam: right button taken over, vanilla placement suppressed while the loop runs"
            );
        }
        Ok(())
    }

     
    fn disengage(&self) -> anyhow::Result<()> {
        interaction::release_right_button(interaction::RightButton::CrystalSpam);
        let mut state = self.state.lock().unwrap();
        state.placed_at = None;
        state.awaiting = None;
        state.attacked = None;
        state.single_click = false;
        state.active_base = None;
         
        if SILENT_AIMED.swap(false, Ordering::AcqRel) {
            crate::module::combat::rotation::clear_silent();
        }
        Ok(())
    }

     
     
    fn try_run(&self) -> anyhow::Result<()> {
        match self.run() {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = self.disengage();
                Err(error)
            }
        }
    }

     
     
     
    fn run(&self) -> anyhow::Result<()> {
         
         
         
        let macro_active = crate::module::combat::crystal_macro::ACTIVE_CRYSTAL_MACROS
            .load(Ordering::Acquire)
            != 0;
        let (holding, clicked) = crate::graphic::input::MOUSE_STATE
            .lock()
            .map(|mut mouse| {
                let clicked = mouse.right_clicked;
                if !macro_active {
                    mouse.right_clicked = false;
                }
                (mouse.right_down, clicked)
            })
            .unwrap_or((false, false));
         
         
        if macro_active && clicked {
            return Ok(());
        }
        let minecraft = minecraft();
        if crate::graphic::input::GUI_OPEN.load(Ordering::Relaxed)
            || minecraft.current_screen().is_open()
        {
            return self.disengage();
        }
        let single_click_done = {
            let state = self.state.lock().unwrap();
            !holding && !clicked && state.single_click && state.attacked.is_some()
        };
        if single_click_done {
            return self.disengage();
        }
        let single_click_active = self.state.lock().unwrap().single_click;
        if !holding && !clicked && !single_click_active {
            return self.disengage();
        }
        let (Some(player), Some(world), Some(game_mode)) = (
            minecraft.player()?,
            minecraft.world()?,
            minecraft.game_mode()?,
        ) else {
            return self.disengage();
        };
        if !player.is_alive()? {
            return self.disengage();
        }
        let inventory = player.get_inventory()?;
         
         
         
        if interaction::owns(interaction::RightButton::AnchorMacro) {
            return Ok(());
        }
        interaction::release_right_button(interaction::RightButton::CrystalMacro);
         
         
         
         
         
         
        let target = match self.locked_hold_target(&world, &player)? {
            Some(target) => Some(target),
            None => self.target(&minecraft, &world, &player, &inventory)?,
        };
        let Some(target) = target else {
             
             
            return Ok(());
        };
        let target_base = match &target {
            Target::Crystal(entity) => base_below(&entity.get_position()?),
            Target::Base(base, _, _) => *base,
        };
        self.state.lock().unwrap().active_base = Some(target_base);
        if clicked && !holding {
            self.state.lock().unwrap().single_click = true;
        }
         
         
        self.engage()?;
        let now = Instant::now();
        let (base, cursor, standing) = match target {
            Target::Crystal(entity) => (base_below(&entity.get_position()?), None, Some(entity)),
             
            Target::Base(base, face, hit) => {
                (base, Some((face, hit)), self.next_crystal(&world, &base)?)
            }
        };
         
         
         
        if let Some(entity) = standing {
            if self.pop(&game_mode, &player, &entity, now)? {
                return Ok(());
            }
             
             
             
             
        }
        self.place_crystal(&game_mode, &player, &world, &inventory, &base, cursor, now)
    }

     
     
     
     
     
     
    fn locked_hold_target(
        &self,
        world: &crate::mapping::client::world::World,
        player: &LocalPlayer,
    ) -> anyhow::Result<Option<Target>> {
        let base = self.state.lock().unwrap().active_base;
        let Some(base) = base else {
            return Ok(None);
        };
        if !is_base_block(world, &base)? || !crystal_in_attack_reach(player, &base)? {
            return Ok(None);
        }
        if let Some(crystal) = crystals_on(world, &base)?.into_iter().next() {
            return Ok(Some(Target::Crystal(crystal)));
        }
        let [x, y, z] = crystal_spot(&base);
        Ok(Some(Target::Base(base, Direction::Up, Vec3::new(x, y, z))))
    }

     
     
    fn next_crystal(
        &self,
        world: &crate::mapping::client::world::World,
        base: &BlockPos,
    ) -> anyhow::Result<Option<Entity>> {
        let attacked = self.attacked_id();
        let mut hit_before = None;
        for entity in crystals_on(world, base)? {
            if Some(entity.id()?) == attacked {
                hit_before = hit_before.or(Some(entity));
                continue;
            }
            return Ok(Some(entity));
        }
        Ok(hit_before)
    }

     
     
     
    fn target(
        &self,
        minecraft: &crate::mapping::client::minecraft::Minecraft,
        world: &crate::mapping::client::world::World,
        player: &LocalPlayer,
        inventory: &Inventory,
    ) -> anyhow::Result<Option<Target>> {
        if !crystal_available(inventory)? {
            return Ok(None);
        }
         
         
        if let Some(entity) = minecraft.crosshair_entity()? {
            if entity.instance_of::<EndCrystal>() {
                let base = base_below(&entity.get_position()?);
                if is_base_block(world, &base)? && crystal_in_attack_reach(player, &base)? {
                    return Ok(Some(Target::Crystal(entity)));
                }
            }
        }
        let Some((anchor, face, hit)) = minecraft.hit_block_face_in_reach()? else {
            return self.locked_base_target(world, player);
        };
        if !is_base_block(world, &anchor)? {
            return self.locked_base_target(world, player);
        }
        Ok(Some(Target::Base(anchor, face, hit)))
    }

     
     
    fn locked_base_target(
        &self,
        world: &crate::mapping::client::world::World,
        player: &LocalPlayer,
    ) -> anyhow::Result<Option<Target>> {
        let base = self.state.lock().unwrap().active_base;
        let Some(base) = base else {
            return Ok(None);
        };
        if !is_base_block(world, &base)? || !crystal_in_attack_reach(player, &base)? {
            return Ok(None);
        }
        let [x, y, z] = crystal_spot(&base);
        Ok(Some(Target::Base(base, Direction::Up, Vec3::new(x, y, z))))
    }

     
     
     
    fn pop(
        &self,
        game_mode: &crate::mapping::client::gamemode::MultiPlayerGameMode,
        player: &LocalPlayer,
        entity: &Entity,
        now: Instant,
    ) -> anyhow::Result<bool> {
        let id = entity.id()?;
        if !self.pop_due(id, now) {
            return Ok(false);
        }
         
         
         
         
        if !crosshair_reaches_entity(player, entity)? {
            if !silent_reaches_entity(player, entity)? {
                let (x, y, z) = {
                    let at = entity.get_position()?;
                    (at.x(), at.y() + 1.0, at.z())
                };
                crate::module::combat::look_at_position_silent(
                    player,
                    crate::mapping::math::Vec3::new(x, y, z),
                    SILENT_AIM_SPEED,
                    360.0,
                )?;
                SILENT_AIMED.store(true, Ordering::Release);
                return Ok(false);
            }
        }
         
         
         
         
        let Some(world) = minecraft().world()? else {
            return Ok(false);
        };
        if ray_blocked_before_entity(player, &world, entity)? {
            return Ok(false);
        }
         
         
        if !interaction::take(player, Action::Entity) {
            return Ok(false);
        }
        game_mode.attack(player, entity)?;
        let _ = player.swing();
         
         
         
        let mut state = self.state.lock().unwrap();
        state.attacked = Some((id, now));
        state.awaiting = None;
        Ok(true)
    }

     
     
     
     
     
    #[allow(clippy::too_many_arguments)]
    fn place_crystal(
        &self,
        game_mode: &crate::mapping::client::gamemode::MultiPlayerGameMode,
        player: &LocalPlayer,
        world: &crate::mapping::client::world::World,
        inventory: &Inventory,
        base: &BlockPos,
        cursor: Option<(Direction, Vec3)>,
        now: Instant,
    ) -> anyhow::Result<()> {
         
         
        if unattacked_crystal_on(world, base, self.attacked_id())?.is_some() {
            return Ok(());
        }
         
         
        if !world.is_block_air(&BlockPos::new(base.x(), base.y() + 1, base.z()))? {
            return Ok(());
        }
        if !self.placement_due(now) {
            return Ok(());
        }
         
         
         
        if !crystal_in_attack_reach(player, base)? {
            return Ok(());
        }
        if !held_item_is(inventory, END_CRYSTAL)? {
             
             
            let Some(slot) = find_hotbar_item(inventory, END_CRYSTAL)? else {
                return Ok(());
            };
            press_hotbar_slot(slot)?;
            self.state.lock().unwrap().placed_at = Some(now);
            return Ok(());
        }
         
         
        let [spot_x, spot_y, spot_z] = crystal_spot(base);
         
         
        let top_click = Vec3::new(spot_x, spot_y - 0.1, spot_z);
        let (face, hit) = if crosshair_reaches(player, base)? {
             
             
            match cursor {
                Some(click) => click,
                None => (Direction::Up, top_click),
            }
        } else if server_aim_on_base(player, base)? {
             
            (Direction::Up, top_click)
        } else if crate::module::combat::movement_input_held() {
             
             
             
             
            return Ok(());
        } else {
             
             
             
             
             
             
            crate::module::combat::look_at_position_silent(
                player,
                top_click,
                SILENT_AIM_SPEED,
                360.0,
            )?;
            SILENT_AIMED.store(true, Ordering::Release);
            return Ok(());
        };
         
         
         
         
        if !interaction::take(player, Action::Block) {
            return Ok(());
        }
        game_mode.place_block_on(player, base, face, hit)?;
        let mut state = self.state.lock().unwrap();
        state.placed_at = Some(now);
        state.awaiting = Some(now);
        Ok(())
    }
}

 
 
fn ray_blocked_before_entity(
    player: &LocalPlayer,
    world: &crate::mapping::client::world::World,
    target: &Entity,
) -> anyhow::Result<bool> {
    let eye = player.get_eye_position()?;
    let origin = [eye.x(), eye.y(), eye.z()];
    let aim = crate::module::combat::rotation::silent_rotation().unwrap_or(
        crate::module::combat::rotation::Rotation {
            yaw: player.get_yaw()?,
            pitch: player.get_pitch()?,
        },
    );
    let direction = super::look_dir(aim.yaw, aim.pitch);
    let (min, max) = super::target_box(target)?;
    let Some(end) = ray_box_entry(
        origin,
        direction,
        min,
        max,
        super::interaction_reach(player),
    ) else {
        return Ok(true);
    };

     
     
    let mut checked = std::collections::HashSet::new();
    let mut distance = 0.05;
    while distance < end {
        let point = [
            origin[0] + direction[0] * distance,
            origin[1] + direction[1] * distance,
            origin[2] + direction[2] * distance,
        ];
        let pos = BlockPos::new(
            point[0].floor() as i32,
            point[1].floor() as i32,
            point[2].floor() as i32,
        );
        let key = (pos.x(), pos.y(), pos.z());
        if checked.insert(key) {
            for shape in world.block_collision_boxes(&pos)? {
                if point[0] >= f64::from(pos.x()) + shape[0]
                    && point[0] <= f64::from(pos.x()) + shape[3]
                    && point[1] >= f64::from(pos.y()) + shape[1]
                    && point[1] <= f64::from(pos.y()) + shape[4]
                    && point[2] >= f64::from(pos.z()) + shape[2]
                    && point[2] <= f64::from(pos.z()) + shape[5]
                {
                    return Ok(true);
                }
            }
        }
        distance += 0.05;
    }
    Ok(false)
}

fn ray_box_entry(
    origin: [f64; 3],
    direction: [f64; 3],
    min: [f64; 3],
    max: [f64; 3],
    max_distance: f64,
) -> Option<f64> {
    let (mut near, mut far) = (0.0_f64, max_distance);
    for axis in 0..3 {
        if direction[axis].abs() < 1e-9 {
            if origin[axis] < min[axis] || origin[axis] > max[axis] {
                return None;
            }
            continue;
        }
        let mut a = (min[axis] - origin[axis]) / direction[axis];
        let mut b = (max[axis] - origin[axis]) / direction[axis];
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        near = near.max(a);
        far = far.min(b);
        if near > far {
            return None;
        }
    }
    (far >= 0.0 && near <= max_distance).then_some(near.max(0.0))
}

impl Module for CrystalSpamModule {
    fn on_start(&self) -> anyhow::Result<()> {
        *self.state.lock().unwrap() = SpamState::default();
        if let Ok(mut mouse) = crate::graphic::input::MOUSE_STATE.lock() {
            mouse.right_clicked = false;
        }
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.disengage()
    }

     
     
    fn on_input_phase(&self) -> anyhow::Result<()> {
        self.try_run()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
         
        if !crate::jvmti::crystal_input_hook_installed() {
            self.try_run()?;
        }
        Ok(())
    }

     
     
     
     
     
     
     
    fn handle_packet(&self, packet: &mut Packet) -> PacketAction {
        if let Packet::ServerboundMovePlayer(snapshot) = packet {
            let aim = if SILENT_AIMED.load(Ordering::Acquire)
                && !crate::module::combat::movement_input_held()
            {
                crate::module::combat::rotation::silent_rotation()
            } else {
                None
            };
            snapshot.apply_silent_aim(aim);
        }
        PacketAction::Forward
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

 
 
fn crystal_available(inventory: &Inventory) -> anyhow::Result<bool> {
    if held_item_is(inventory, END_CRYSTAL)? {
        return Ok(true);
    }
    Ok(find_hotbar_item(inventory, END_CRYSTAL)?.is_some())
}

 
 
 
fn base_below(position: &Vec3) -> BlockPos {
    BlockPos::new(
        position.x().floor() as i32,
        position.y().floor() as i32 - 1,
        position.z().floor() as i32,
    )
}

 
fn crystal_spot(base: &BlockPos) -> [f64; 3] {
    [
        f64::from(base.x()) + 0.5,
        f64::from(base.y()) + 1.0,
        f64::from(base.z()) + 0.5,
    ]
}

 
fn crystal_box(base: &BlockPos) -> ([f64; 3], [f64; 3]) {
    let [x, y, z] = crystal_spot(base);
    (
        [x - CRYSTAL_HALF_WIDTH, y, z - CRYSTAL_HALF_WIDTH],
        [
            x + CRYSTAL_HALF_WIDTH,
            y + CRYSTAL_HEIGHT,
            z + CRYSTAL_HALF_WIDTH,
        ],
    )
}

 
 
fn is_crystal_on(position: &Vec3, base: &BlockPos) -> bool {
    let [x, y, z] = crystal_spot(base);
    (position.x() - x).abs() <= 0.5
        && (position.y() - y).abs() <= 0.5
        && (position.z() - z).abs() <= 0.5
}

 
pub(crate) fn crystals_on(
    world: &crate::mapping::client::world::World,
    base: &BlockPos,
) -> anyhow::Result<Vec<Entity>> {
    let mut crystals = Vec::new();
    for entity in world.get_entities()? {
        if !entity.instance_of::<EndCrystal>() {
            continue;
        }
        let removed = entity
            .call_method("isRemoved", &[])
            .and_then(|value| Ok(value.z()?))
            .unwrap_or(true);
        if removed {
            continue;
        }
        if is_crystal_on(&entity.get_position()?, base) {
            crystals.push(entity);
        }
    }
    Ok(crystals)
}

 
 
 
fn unattacked_crystal_on(
    world: &crate::mapping::client::world::World,
    base: &BlockPos,
    attacked: Option<i32>,
) -> anyhow::Result<Option<Entity>> {
    for entity in crystals_on(world, base)? {
        if Some(entity.id()?) != attacked {
            return Ok(Some(entity));
        }
    }
    Ok(None)
}

 
 
 
fn may_place(
    awaiting_since: Option<Instant>,
    placed_at: Option<Instant>,
    spawn_timeout: Duration,
    delay: Duration,
    now: Instant,
) -> bool {
    if let Some(at) = awaiting_since {
        if now.saturating_duration_since(at) < spawn_timeout {
            return false;
        }
    }
    match placed_at {
        Some(at) => now.saturating_duration_since(at) >= delay,
        None => true,
    }
}

 
 
fn crystal_in_attack_reach(player: &LocalPlayer, base: &BlockPos) -> anyhow::Result<bool> {
    let eye = player.get_eye_position()?;
    let (min, max) = crystal_box(base);
    Ok(
        super::point_box_distance([eye.x(), eye.y(), eye.z()], min, max)
            <= super::interaction_reach(player),
    )
}

 
 
 
fn crosshair_reaches_entity(player: &LocalPlayer, target: &Entity) -> anyhow::Result<bool> {
    let eye = player.get_eye_position()?;
    let direction = super::look_dir(player.get_yaw()?, player.get_pitch()?);
    let (min, max) = super::target_box(target)?;
    Ok(super::ray_hits_box(
        [eye.x(), eye.y(), eye.z()],
        direction,
        min,
        max,
        super::interaction_reach(player),
    ))
}

 
 
 
fn server_aim_on_base(player: &LocalPlayer, base: &BlockPos) -> anyhow::Result<bool> {
    match crate::net::packet::serverbound_move_player::silent_aim_on_wire() {
        Some(aim) => aim_reaches(player, base, aim),
        None => Ok(false),
    }
}

 
 
fn silent_reaches_entity(player: &LocalPlayer, target: &Entity) -> anyhow::Result<bool> {
    let Some(aim) = crate::module::combat::rotation::silent_rotation() else {
        return Ok(false);
    };
    super::shadow_crosshair_on_target(player, target, aim, super::interaction_reach(player))
}

#[cfg(test)]
mod tests {
    use super::{
        base_below, crystal_box, crystal_spot, is_crystal_on, may_place, CrystalSpamModule,
        SPAWN_TIMEOUT,
    };
    use crate::mapping::math::{BlockPos, Vec3};
    use crate::module::{Module, ModuleId};
    use std::time::{Duration, Instant};

     
     
     
    #[test]
    fn the_loop_defaults_to_no_delay_and_stays_editable() {
        let mut module = CrystalSpamModule::new();
        let data = module.get_module_data_mut();
        assert_eq!(data.id, ModuleId::CrystalSpam);
        assert_eq!(data.id.display_name(), "Crystal Spam");
        for (name, default) in [("Delay (ms)", 0.0), ("Pop Retry (ms)", 100.0)] {
            let setting = data
                .get_setting_mut(name)
                .unwrap_or_else(|| panic!("setting {name} is missing"));
            assert_eq!(setting.get_slider_value(), Some(default), "{name}");
        }
        let data = module.get_module_data_mut();
        for (name, value) in [("Delay (ms)", 250.0), ("Pop Retry (ms)", 0.0)] {
            data.get_setting_mut(name)
                .expect("checked above")
                .set_slider_value(value);
        }
        assert_eq!(module.delay(), Duration::from_millis(250));
        assert_eq!(module.pop_retry(), Duration::ZERO);
    }

     
     
     
     
    #[test]
    fn a_placement_is_held_only_until_it_is_swung_at() {
        let now = Instant::now();
        assert!(may_place(None, None, SPAWN_TIMEOUT, Duration::ZERO, now));
         
        assert!(!may_place(
            Some(now),
            Some(now),
            SPAWN_TIMEOUT,
            Duration::ZERO,
            now
        ));
         
         
        assert!(may_place(
            None,
            Some(now),
            SPAWN_TIMEOUT,
            Duration::ZERO,
            now + Duration::from_millis(50)
        ));
         
        assert!(may_place(
            Some(now),
            Some(now),
            SPAWN_TIMEOUT,
            Duration::ZERO,
            now + SPAWN_TIMEOUT
        ));
         
        assert!(!may_place(
            None,
            Some(now),
            SPAWN_TIMEOUT,
            Duration::from_millis(200),
            now + Duration::from_millis(100)
        ));
        assert!(may_place(
            None,
            Some(now),
            SPAWN_TIMEOUT,
            Duration::from_millis(200),
            now + Duration::from_millis(200)
        ));
    }

    #[test]
    fn a_crystals_blowup_box_sits_on_its_base_block() {
        let base = BlockPos::new(4, 70, -3);
        assert_eq!(crystal_spot(&base), [4.5, 71.0, -2.5]);
        let (min, max) = crystal_box(&base);
        assert_eq!(min, [3.5, 71.0, -3.5]);
        assert_eq!(max, [5.5, 73.0, -1.5]);
    }

     
     
     
    #[test]
    fn the_base_is_the_block_a_crystal_stands_on() {
        assert_eq!(
            base_below(&Vec3::new(4.5, 71.0, -2.5)),
            BlockPos::new(4, 70, -3)
        );
        assert_eq!(
            base_below(&Vec3::new(-0.5, 65.0, -0.5)),
            BlockPos::new(-1, 64, -1)
        );
        assert_eq!(
            base_below(&Vec3::new(0.5, 4.0, 0.5)),
            BlockPos::new(0, 3, 0)
        );
    }

    #[test]
    fn only_the_crystal_on_a_base_counts_as_that_bases_crystal() {
        let base = BlockPos::new(2, 64, 2);
        assert!(is_crystal_on(&Vec3::new(2.5, 65.0, 2.5), &base));
         
         
        assert!(is_crystal_on(&Vec3::new(2.4, 65.3, 2.6), &base));
        assert!(!is_crystal_on(&Vec3::new(3.5, 65.0, 2.5), &base));
        assert!(!is_crystal_on(&Vec3::new(2.5, 66.0, 2.5), &base));
        assert!(!is_crystal_on(&Vec3::new(2.5, 64.0, 2.5), &base));
    }
}
