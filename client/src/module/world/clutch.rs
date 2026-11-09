 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::input::GUI_OPEN;
use crate::mapping::block::Direction;
use crate::mapping::client::options::Perspective;
use crate::mapping::client::world::World;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::inventory::Inventory;
use crate::mapping::math::{BlockPos, Vec3};
use crate::module::combat::interaction::{self, Action};
use crate::module::combat::rotation::{self, BodyAimOwner, Rotation};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{client, minecraft};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

 
const DEFAULT_MIN_FALL: f32 = 3.0;
const MIN_FALL_RANGE: (f32, f32) = (1.0, 20.0);

 
const VANILLA_BLOCK_REACH: f64 = 4.5;
 
const REACH_MARGIN: f64 = 0.1;

 
 
 
 
static BODY_HELD: AtomicBool = AtomicBool::new(false);

 
 
 
 
const CAMERA_TOLERANCE: f32 = 5.0;

 
 
 
const MAX_RINGS: i32 = 5;

 
 
const MAX_PROBES: usize = 128;

 
 
 
const NOT_FULL_BLOCKS: &[&str] = &[
    "torch",
    "lantern",
    "candle",
    "sign",
    "banner",
    "button",
    "pressure_plate",
    "lever",
    "_rail",
    "flower",
    "sapling",
    "carpet",
    "slab",
    "stairs",
    "fence",
    "gate",
    "_door",
    "trapdoor",
    "scaffolding",
    "vine",
    "ladder",
    "cobweb",
    "water",
    "lava",
    "bucket",
    "cake",
    "pane",
    "chain",
    "lightning_rod",
    "end_rod",
    "tripwire",
    "comparator",
    "repeater",
    "lily_pad",
    "potted_",
];

 
 
 
pub(crate) fn wants_clutch(
    on_ground: bool,
    fall_distance: f64,
    falling_speed: f64,
    min_fall: f64,
) -> bool {
    !on_ground && fall_distance >= min_fall && falling_speed < 0.0
}

 
 
 
pub(crate) fn drop_would_hurt(drop: Option<f64>, min_fall: f64) -> bool {
    match drop {
        Some(distance) => distance >= min_fall,
        None => true,
    }
}

 
 
 
 
fn drop_below(world: &World, feet: Vec3, min_fall: f64) -> anyhow::Result<Option<f64>> {
    let column = BlockPos::new(
        feet.x().floor() as i32,
        feet.y().floor() as i32,
        feet.z().floor() as i32,
    );
    let depth = min_fall.ceil() as i32 + 1;
    for step in 1..=depth {
        let cell = column.offset(0, -step, 0);
        if !world.block_collision_boxes(&cell)?.is_empty() {
            return Ok(Some(feet.y() - (cell.y() + 1) as f64));
        }
    }
    Ok(None)
}

 
 
 
pub(crate) fn target_below(feet_x: f64, feet_y: f64, feet_z: f64) -> BlockPos {
    BlockPos::new(
        feet_x.floor() as i32,
        feet_y.floor() as i32 - 1,
        feet_z.floor() as i32,
    )
}

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Placement {
    pub anchor: BlockPos,
    pub face: Direction,
}

 
 
 
pub(crate) fn placement_candidates(target: BlockPos) -> [Placement; 5] {
    [
        Placement {
            anchor: target.offset(0, -1, 0),
            face: Direction::Up,
        },
        Placement {
            anchor: target.offset(-1, 0, 0),
            face: Direction::East,
        },
        Placement {
            anchor: target.offset(1, 0, 0),
            face: Direction::West,
        },
        Placement {
            anchor: target.offset(0, 0, -1),
            face: Direction::South,
        },
        Placement {
            anchor: target.offset(0, 0, 1),
            face: Direction::North,
        },
    ]
}

 
pub(crate) fn hit_point(anchor: BlockPos, face: Direction) -> Vec3 {
    let (x, y, z) = (anchor.x() as f64, anchor.y() as f64, anchor.z() as f64);
    match face {
        Direction::Up => Vec3::new(x + 0.5, y + 1.0, z + 0.5),
        Direction::Down => Vec3::new(x + 0.5, y, z + 0.5),
        Direction::East => Vec3::new(x + 1.0, y + 0.5, z + 0.5),
        Direction::West => Vec3::new(x, y + 0.5, z + 0.5),
        Direction::South => Vec3::new(x + 0.5, y + 0.5, z + 1.0),
        Direction::North => Vec3::new(x + 0.5, y + 0.5, z),
    }
}

 
 
 
 
 
 
 
 
 
 
 
pub(crate) fn face_faces_eye(eye: Vec3, anchor: BlockPos, face: Direction) -> bool {
    const MARGIN: f64 = 0.05;
    let (x, y, z) = (anchor.x() as f64, anchor.y() as f64, anchor.z() as f64);
    match face {
        Direction::Up => eye.y() > y + 1.0 + MARGIN,
        Direction::Down => eye.y() < y - MARGIN,
        Direction::East => eye.x() > x + 1.0 + MARGIN,
        Direction::West => eye.x() < x - MARGIN,
        Direction::South => eye.z() > z + 1.0 + MARGIN,
        Direction::North => eye.z() < z - MARGIN,
    }
}

 
 
pub(crate) fn occupies(feet: Vec3, width: f64, height: f64, cell: BlockPos) -> bool {
    let half = width / 2.0;
    let (min_x, min_y, min_z) = (feet.x() - half, feet.y(), feet.z() - half);
    let (max_x, max_y, max_z) = (feet.x() + half, feet.y() + height, feet.z() + half);
    let (cx, cy, cz) = (cell.x() as f64, cell.y() as f64, cell.z() as f64);
    min_x < cx + 1.0
        && max_x > cx
        && min_y < cy + 1.0
        && max_y > cy
        && min_z < cz + 1.0
        && max_z > cz
}

 
 
pub(crate) fn ring(center: BlockPos, radius: i32) -> Vec<BlockPos> {
    let mut cells = Vec::new();
    for dx in -radius..=radius {
        for dy in -radius..=radius {
            for dz in -radius..=radius {
                if dx.abs().max(dy.abs()).max(dz.abs()) != radius {
                    continue;
                }
                cells.push(center.offset(dx, dy, dz));
            }
        }
    }
    cells
}

 
 
 
pub(crate) fn order_ring(cells: &mut [BlockPos], goal: BlockPos) {
    cells.sort_by_key(|cell| {
        let manhattan =
            (cell.x() - goal.x()).abs() + (cell.y() - goal.y()).abs() + (cell.z() - goal.z()).abs();
        (manhattan, -cell.y())
    });
}

 
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Candidate {
    pub cell: BlockPos,
    pub placement: Placement,
    pub hit: Vec3,
}

 
 
 
pub(crate) trait Probe {
    fn is_air(&self, cell: BlockPos) -> anyhow::Result<bool>;
    fn is_solid(&self, cell: BlockPos) -> anyhow::Result<bool>;
}

 
 
struct WorldProbe<'a> {
    world: &'a World,
}

impl Probe for WorldProbe<'_> {
    fn is_air(&self, cell: BlockPos) -> anyhow::Result<bool> {
        self.world.is_block_air(&cell)
    }

    fn is_solid(&self, cell: BlockPos) -> anyhow::Result<bool> {
        Ok(!self.world.block_collision_boxes(&cell)?.is_empty())
    }
}

 
 
 
 
 
 
 
fn next_placement(
    probe: &dyn Probe,
    body: (f64, f64),
    goal: BlockPos,
    eye: Vec3,
    reach: f64,
    feet: Vec3,
) -> anyhow::Result<Option<Candidate>> {
    if reach <= 0.0 {
        return Ok(None);
    }
     
    if probe.is_solid(goal)? {
        return Ok(None);
    }
    let (width, height) = body;
    let mut probed = 0usize;
    for radius in 0..=MAX_RINGS {
        let mut cells = ring(goal, radius);
        order_ring(&mut cells, goal);
        for cell in cells {
            if probed >= MAX_PROBES {
                return Ok(None);
            }
            probed += 1;
            if occupies(feet, width, height, cell) {
                continue;
            }
             
            if !probe.is_air(cell)? {
                continue;
            }
            for placement in placement_candidates(cell) {
                 
                if probe.is_air(placement.anchor)? {
                    continue;
                }
                 
                 
                 
                if !face_faces_eye(eye, placement.anchor, placement.face) {
                    continue;
                }
                let hit = hit_point(placement.anchor, placement.face);
                let (dx, dy, dz) = (hit.x() - eye.x(), hit.y() - eye.y(), hit.z() - eye.z());
                if (dx * dx + dy * dy + dz * dz).sqrt() > reach {
                    continue;
                }
                 
                if !probe.is_solid(placement.anchor)? {
                    continue;
                }
                return Ok(Some(Candidate {
                    cell,
                    placement,
                    hit,
                }));
            }
        }
    }
    Ok(None)
}

 
pub(crate) fn is_placeable_block(registry_name: &str) -> bool {
    let name = registry_name
        .strip_prefix("minecraft:")
        .unwrap_or(registry_name);
    if name == "snow" || name.ends_with("_bed") {
        return false;
    }
    !NOT_FULL_BLOCKS.iter().any(|part| name.contains(part))
}

fn block_reach(player: &LocalPlayer) -> f64 {
    (player
        .get_attribute_value("BLOCK_INTERACTION_RANGE")
        .ok()
        .flatten()
        .unwrap_or(VANILLA_BLOCK_REACH)
        .max(0.0)
        - REACH_MARGIN)
        .max(0.0)
}

 
 
 
 
fn first_person_view() -> bool {
    minecraft()
        .options()
        .and_then(|options| options.perspective())
        .map(|perspective| perspective == Perspective::FirstPerson)
        .unwrap_or(true)
}

 
 
fn body_aim_available() -> bool {
    crate::jvmti::camera_frame_hook_installed() && first_person_view()
}

 
 
 
 
 
 
 
 
 
 
 
fn arm_body(aim: Rotation) -> anyhow::Result<()> {
    anyhow::ensure!(
        body_aim_available(),
        "the body cannot carry the aim (third person, or the camera restore is not installed)"
    );
    rotation::aim_body_for(BodyAimOwner::Clutch, aim)?;
    BODY_HELD.store(true, Ordering::Relaxed);
    Ok(())
}

 
fn release_body() {
    BODY_HELD.store(false, Ordering::Relaxed);
    let _ = rotation::restore_body_for(BodyAimOwner::Clutch);
}

fn placeable_slot(inventory: &Inventory, slot: i32) -> anyhow::Result<bool> {
    let stack = inventory.get_item(slot)?;
    if stack.is_empty()? {
        return Ok(false);
    }
    let item = stack.get_item()?;
    Ok(item.is_block_item() && is_placeable_block(&item.registry_name()?))
}

fn block_slot(inventory: &Inventory) -> anyhow::Result<Option<i32>> {
    for slot in 0..9 {
        if placeable_slot(inventory, slot)? {
            return Ok(Some(slot));
        }
    }
    Ok(None)
}

 
#[derive(Debug, Clone, Copy)]
struct Swap {
    previous: i32,
    block: i32,
}

#[derive(Debug, Default)]
struct State {
    swap: Option<Swap>,
     
     
    warned: bool,
}

 
#[derive(Debug)]
pub struct ClutchModule {
    pub module: ModuleData,
    state: Mutex<State>,
}

impl ClutchModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Clutch,
                description:
                    "Builds a path of blocks under a fall that would hurt, ring by ring, and turns the body onto each click the way Scaffold does"
                        .to_string(),
                category: ModuleCategory::World,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Min Fall".to_string(),
                    value: DEFAULT_MIN_FALL,
                    min: MIN_FALL_RANGE.0,
                    max: MIN_FALL_RANGE.1,
                }],
            },
            state: Mutex::new(State::default()),
        }
    }

    fn min_fall(&self) -> f32 {
        let value = self
            .module
            .get_setting("Min Fall")
            .and_then(ModuleSetting::get_slider_value)
            .unwrap_or(DEFAULT_MIN_FALL);
        if value.is_finite() {
            value.clamp(MIN_FALL_RANGE.0, MIN_FALL_RANGE.1)
        } else {
            DEFAULT_MIN_FALL
        }
    }

     
    fn select_block(&self, player: &LocalPlayer, state: &mut State) -> anyhow::Result<()> {
        let inventory = player.get_inventory()?;
        let selected = inventory.get_selected()?;
        if state.swap.is_some_and(|swap| swap.block == selected) {
            return Ok(());
        }
         
         
        state.swap = None;
        if placeable_slot(&inventory, selected)? {
            return Ok(());
        }
        let Some(slot) = block_slot(&inventory)? else {
            return Ok(());
        };
        state.swap = Some(Swap {
            previous: selected,
            block: slot,
        });
        inventory.set_selected(slot)?;
        Ok(())
    }

     
     
    fn restore_slot(&self, state: &mut State) -> anyhow::Result<()> {
        let Some(swap) = state.swap else {
            return Ok(());
        };
        let mc = minecraft();
        let Some(player) = mc.player()? else {
            state.swap = None;
            return Ok(());
        };
        if mc.current_screen().is_open()
            || player.container_menu()?.container_id()? != 0
            || player.is_using_item()?
        {
            return Ok(());
        }
        state.swap = None;
        let inventory = player.get_inventory()?;
        if inventory.get_selected()? == swap.block {
            inventory.set_selected(swap.previous)?;
        }
        Ok(())
    }

    fn stand_down(&self) -> anyhow::Result<()> {
        release_body();
        let mut state = self.state.lock().unwrap();
        self.restore_slot(&mut state)
    }

    fn step(&self) -> anyhow::Result<()> {
        let mc = minecraft();
        let Some(player) = mc.player()? else {
            return self.stand_down();
        };
        let mut state = self.state.lock().unwrap();

        if mc.current_screen().is_open()
            || GUI_OPEN.load(Ordering::Relaxed)
            || !player.is_alive()?
            || player.is_spectator()?
            || player.is_fall_flying()?
            || player.is_in_water()?
            || player.container_menu()?.container_id()? != 0
            || player.is_using_item()?
            || busy(ModuleId::Scaffold)
            || busy(ModuleId::FastScaffold)
            || busy(ModuleId::SilentAura)
            || busy(ModuleId::KillAura)
        {
            drop(state);
            return self.stand_down();
        }

        let min_fall = self.min_fall() as f64;
        let feet = player.get_position()?;
        let falling = !player.on_ground()? && player.get_delta_movement()?.y() < 0.0;
        if !falling {
            drop(state);
            return self.stand_down();
        }
        let Some(world) = mc.world()? else {
            drop(state);
            return self.stand_down();
        };
         
         
         
        let accumulated = player.get_fall_distance()?;
        if accumulated < min_fall {
            let drop_ahead = drop_below(&world, feet, min_fall)?;
            if !drop_would_hurt(drop_ahead, min_fall) {
                drop(state);
                return self.stand_down();
            }
        }

        let goal = target_below(feet.x(), feet.y(), feet.z());
        let eye = player.get_eye_position()?;
        let reach = block_reach(&player);
        let body = (player.bb_width()? as f64, player.bb_height()? as f64);
        let probe = WorldProbe { world: &world };
        let Some(candidate) = next_placement(&probe, body, goal, eye, reach, feet)? else {
             
             
             
            release_body();
            return Ok(());
        };
        let Some(game_mode) = mc.game_mode()? else {
            return Ok(());
        };

        self.select_block(&player, &mut state)?;
        let inventory = player.get_inventory()?;
        let selected = inventory.get_selected()?;
        if !placeable_slot(&inventory, selected)? {
            return Ok(());
        }

        let hit = candidate.hit;
        let mut aim = Rotation::towards(eye, hit);
         
         
        let horizontal = ((hit.x() - eye.x()).powi(2) + (hit.z() - eye.z()).powi(2)).sqrt();
        if horizontal < 1e-3 {
            aim.yaw = player.get_yaw()?;
        }
         
         
         
         
         
         
         
         
        if body_aim_available() {
            if let Err(error) = arm_body(aim) {
                if !state.warned {
                    state.warned = true;
                    log::warn!("Clutch: not clicking, the body cannot carry the aim: {error}");
                }
                return Ok(());
            }
        } else {
             
             
            let camera = Rotation::new(player.get_yaw()?, player.get_pitch()?);
            if camera.angle_to(aim) > CAMERA_TOLERANCE {
                return Ok(());
            }
        }

        if !interaction::take(&player, Action::Block) {
            return Ok(());
        }
        game_mode.place_block_on(
            &player,
            &candidate.placement.anchor,
            candidate.placement.face,
            hit,
        )?;
        Ok(())
    }
}

impl Default for ClutchModule {
    fn default() -> Self {
        Self::new()
    }
}

 
fn busy(id: ModuleId) -> bool {
    client().modules.get(id).is_some_and(|handle| {
        handle
            .try_lock()
            .map(|module| module.get_module_data().enabled)
            .unwrap_or(true)
    })
}

impl Module for ClutchModule {
    fn on_start(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            crate::jvmti::transform_installed("ScaffoldInputPhase"),
            "Clutch requires Minecraft 1.21.4 input-phase hook"
        );
        *self.state.lock().unwrap() = State::default();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
        release_body();
        let mut state = self.state.lock().unwrap();
        let _ = self.restore_slot(&mut state);
        *state = State::default();
        Ok(())
    }

    fn on_input_phase(&self) -> anyhow::Result<()> {
         
         
         
         
        if BODY_HELD.swap(false, Ordering::Relaxed) {
            let _ = rotation::restore_body_for(BodyAimOwner::Clutch);
        }
        self.step()
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

    fn key(cell: BlockPos) -> (i32, i32, i32) {
        (cell.x(), cell.y(), cell.z())
    }

     
     
    #[derive(Default)]
    struct FakeWorld {
        solid: std::collections::HashSet<(i32, i32, i32)>,
    }

    impl Probe for FakeWorld {
        fn is_air(&self, cell: BlockPos) -> anyhow::Result<bool> {
            Ok(!self.solid.contains(&key(cell)))
        }

        fn is_solid(&self, cell: BlockPos) -> anyhow::Result<bool> {
            Ok(self.solid.contains(&key(cell)))
        }
    }

     
     
     
     
    #[test]
    fn the_path_grows_ring_by_ring_until_it_is_under_the_player() {
         
        let mut world = FakeWorld::default();
        for x in 0..=8 {
            world.solid.insert((x, 69, 10));
        }
        let feet = Vec3::new(10.5, 70.0, 10.5);
        let eye = Vec3::new(10.5, 71.62, 10.5);
        let body = (0.6, 1.8);
        let reach = 4.4;
        let goal = target_below(feet.x(), feet.y(), feet.z());
        assert_eq!(goal, BlockPos::new(10, 69, 10));
        assert!(!world.is_solid(goal).unwrap(), "the player is over air");

        let mut placed = Vec::new();
        for _ in 0..8 {
            let Some(candidate) = next_placement(&world, body, goal, eye, reach, feet).unwrap()
            else {
                break;
            };
            assert!(
                !occupies(feet, body.0, body.1, candidate.cell),
                "{:?} would be inside the player",
                candidate.cell
            );
            assert!(
                world.is_solid(candidate.placement.anchor).unwrap(),
                "{:?} is not clicked off a block that exists",
                candidate.cell
            );
            assert!(world.is_air(candidate.cell).unwrap());
            world.solid.insert(key(candidate.cell));
            placed.push(candidate.cell);
        }

        assert!(
            world.is_solid(goal).unwrap(),
            "the path never reached under the player: {placed:?}"
        );
         
         
        assert_eq!(placed.len(), 2, "{placed:?}");
        assert_eq!(placed[0], BlockPos::new(9, 69, 10));
        assert_eq!(placed[1], goal);
    }

     
     
     
    #[test]
    fn a_fall_with_nothing_in_reach_places_nothing() {
        let world = FakeWorld::default();
        let feet = Vec3::new(0.5, 120.0, 0.5);
        let eye = Vec3::new(0.5, 121.62, 0.5);
        let goal = target_below(feet.x(), feet.y(), feet.z());
        assert_eq!(
            next_placement(&world, (0.6, 1.8), goal, eye, 4.4, feet).unwrap(),
            None
        );
    }

     
     
    #[test]
    fn a_supported_player_gets_no_more_blocks() {
        let mut world = FakeWorld::default();
        world.solid.insert((10, 69, 10));
        let feet = Vec3::new(10.5, 70.0, 10.5);
        let eye = Vec3::new(10.5, 71.62, 10.5);
        let goal = target_below(feet.x(), feet.y(), feet.z());
        assert_eq!(
            next_placement(&world, (0.6, 1.8), goal, eye, 4.4, feet).unwrap(),
            None
        );
    }

    #[test]
    fn a_fall_worth_a_block_needs_ground_distance_and_speed() {
        assert!(wants_clutch(false, 3.0, -0.6, 3.0));
        assert!(
            !wants_clutch(true, 3.0, -0.6, 3.0),
            "standing is not falling"
        );
        assert!(!wants_clutch(false, 2.9, -0.6, 3.0), "too short to hurt");
        assert!(
            !wants_clutch(false, 5.0, 0.0, 3.0),
            "a fall that has stopped is already broken"
        );
    }

     
     
     
    #[test]
    fn the_drop_ahead_decides_too() {
        assert!(drop_would_hurt(Some(3.0), 3.0));
        assert!(drop_would_hurt(Some(48.0), 3.0));
        assert!(!drop_would_hurt(Some(2.5), 3.0), "a hop is not a fall");
        assert!(!drop_would_hurt(Some(0.0), 3.0), "standing on a block");
        assert!(drop_would_hurt(None, 3.0), "the void is a long fall");
    }

    #[test]
    fn the_target_is_the_cell_below_the_feet() {
        assert_eq!(target_below(12.9, 70.7, -3.2), BlockPos::new(12, 69, -4));
        assert_eq!(target_below(-0.1, 64.0, 8.0), BlockPos::new(-1, 63, 8));
    }

    #[test]
    fn every_candidate_face_points_into_the_target() {
        let target = BlockPos::new(5, 40, 9);
        for placement in placement_candidates(target) {
            let (dx, dy, dz) = placement.face.offset();
            assert_eq!(
                placement.anchor.offset(dx, dy, dz),
                target,
                "{:?} of {:?} does not point at the target",
                placement.face,
                placement.anchor
            );
        }
        assert_eq!(placement_candidates(target)[0].face, Direction::Up);
    }

    #[test]
    fn a_face_hit_sits_in_the_middle_of_that_face() {
        let anchor = BlockPos::new(2, 10, 3);
        let up = hit_point(anchor, Direction::Up);
        assert_eq!((up.x(), up.y(), up.z()), (2.5, 11.0, 3.5));
        let west = hit_point(anchor, Direction::West);
        assert_eq!((west.x(), west.y(), west.z()), (2.0, 10.5, 3.5));
        let north = hit_point(anchor, Direction::North);
        assert_eq!((north.x(), north.y(), north.z()), (2.5, 10.5, 3.0));
    }

     
     
     
    #[test]
    fn a_click_only_names_a_face_the_eye_is_outside_of() {
        let anchor = BlockPos::new(2, 10, 3);
         
        let above = Vec3::new(2.5, 12.0, 3.5);
        assert!(face_faces_eye(above, anchor, Direction::Up));
        for face in [
            Direction::East,
            Direction::West,
            Direction::North,
            Direction::South,
        ] {
            assert!(
                !face_faces_eye(above, anchor, face),
                "{face:?} from directly above is inside the block span"
            );
        }
         
        let east = Vec3::new(4.0, 10.6, 3.5);
        assert!(face_faces_eye(east, anchor, Direction::East));
        assert!(!face_faces_eye(east, anchor, Direction::West));
        assert!(!face_faces_eye(east, anchor, Direction::Up));
         
         
        let on_plane = Vec3::new(3.0, 10.6, 3.5);
        assert!(!face_faces_eye(on_plane, anchor, Direction::East));
         
        let south = Vec3::new(2.5, 10.6, 5.0);
        assert!(face_faces_eye(south, anchor, Direction::South));
        assert!(!face_faces_eye(south, anchor, Direction::North));
    }

    #[test]
    fn the_player_is_never_given_a_block_inside_their_own_box() {
        let feet = Vec3::new(10.5, 70.0, 10.5);
         
        assert!(occupies(feet, 0.6, 1.8, BlockPos::new(10, 70, 10)));
        assert!(occupies(feet, 0.6, 1.8, BlockPos::new(10, 71, 10)));
         
         
        assert!(!occupies(feet, 0.6, 1.8, BlockPos::new(10, 69, 10)));
        assert!(!occupies(feet, 0.6, 1.8, BlockPos::new(9, 70, 10)));
        assert!(!occupies(feet, 0.6, 1.8, BlockPos::new(10, 72, 10)));
    }

     
     
    #[test]
    fn the_search_walks_out_ring_by_ring() {
        let goal = BlockPos::new(4, 30, 4);
        assert_eq!(ring(goal, 0), vec![goal]);
        let one = ring(goal, 1);
        assert_eq!(one.len(), 26);
        let mut seen = std::collections::HashSet::new();
        for cell in &one {
            assert!(
                seen.insert((cell.x(), cell.y(), cell.z())),
                "{cell:?} appears twice"
            );
            let reach = (cell.x() - goal.x())
                .abs()
                .max((cell.y() - goal.y()).abs())
                .max((cell.z() - goal.z()).abs());
            assert_eq!(reach, 1, "{cell:?} is not on ring 1");
        }
         
         
        let two = ring(goal, 2);
        assert_eq!(two.len(), 5 * 5 * 5 - 3 * 3 * 3);
        assert!(two
            .iter()
            .all(|cell| !seen.contains(&(cell.x(), cell.y(), cell.z()))));
    }

     
     
    #[test]
    fn a_ring_is_tried_from_under_the_player_outward() {
        let goal = BlockPos::new(4, 30, 4);
        let mut cells = ring(goal, 2);
        order_ring(&mut cells, goal);
        let manhattan = |cell: BlockPos| {
            (cell.x() - goal.x()).abs() + (cell.y() - goal.y()).abs() + (cell.z() - goal.z()).abs()
        };
        let mut previous = 0;
        for cell in &cells {
            assert!(
                manhattan(*cell) >= previous,
                "{cell:?} is nearer the goal than the cell before it"
            );
            previous = manhattan(*cell);
        }
         
         
        let flat = cells
            .iter()
            .position(|cell| *cell == goal.offset(2, 0, 0))
            .unwrap();
        let corner = cells
            .iter()
            .position(|cell| *cell == goal.offset(2, -2, 2))
            .unwrap();
        assert!(flat < corner);
    }

    #[test]
    fn ordinary_building_blocks_are_accepted_and_decorations_are_not() {
        for good in [
            "minecraft:stone",
            "minecraft:oak_planks",
            "minecraft:obsidian",
            "minecraft:bedrock",
            "minecraft:black_wool",
        ] {
            assert!(is_placeable_block(good), "{good}");
        }
        for bad in [
            "minecraft:torch",
            "minecraft:water_bucket",
            "minecraft:oak_slab",
            "minecraft:cobweb",
            "minecraft:oak_sign",
            "minecraft:red_bed",
            "minecraft:oak_pressure_plate",
        ] {
            assert!(!is_placeable_block(bad), "{bad}");
        }
    }

    #[test]
    fn min_fall_is_clamped() {
        let module = ClutchModule::new();
        assert_eq!(module.min_fall(), DEFAULT_MIN_FALL);
        let mut module = ClutchModule::new();
        module
            .module
            .get_setting_mut("Min Fall")
            .unwrap()
            .set_slider_value(f32::NAN);
        assert!(module.min_fall().is_finite());
        module
            .module
            .get_setting_mut("Min Fall")
            .unwrap()
            .set_slider_value(100.0);
        assert_eq!(module.min_fall(), MIN_FALL_RANGE.1);
    }

    #[test]
    fn module_identity() {
        let module = ClutchModule::new();
        assert_eq!(module.module.id, ModuleId::Clutch);
        assert_eq!(module.module.id.display_name(), "Clutch");
        assert_eq!(module.module.category, ModuleCategory::World);
        assert!(!module.module.enabled);
    }
}
