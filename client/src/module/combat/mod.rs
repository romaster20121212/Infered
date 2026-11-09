pub mod aimbot;
pub mod anchor_macro;
pub mod aura;
pub mod auto_bow;
pub mod auto_mace;
pub mod auto_milk;
pub mod auto_pearl;
pub mod auto_shield;
pub mod auto_sword;
pub mod auto_weapon;
pub mod autoclicker;
pub mod autocrystal;
pub mod crystal_macro;
pub mod crystal_optimizer;
pub mod crystal_spam;
pub mod hit_speed;
pub mod hitselect;
pub mod hitswap;
pub mod interaction;
pub mod killaura;
pub mod mobaura;
pub mod no_miss_cooldown;
pub mod practical;
pub mod reach;
pub mod rotation;
pub mod silent_assist;
pub mod silent_aura;
pub mod silent_mace;
pub mod triggerbot;
pub mod velocity;
pub mod wtap;

use crate::mapping::entity::mob::Mob;
use crate::mapping::entity::player::{LocalPlayer, Player};
use crate::mapping::entity::{Entity, EntityRef, LivingEntityRef};
use crate::mapping::math::Vec3;
use crate::mapping::MappedObject;
use crate::state::{client, minecraft};
use rotation::Rotation;

 
 
pub(crate) const STILL_SPEED: f64 = 0.03;

 
pub(crate) fn is_still(horizontal_speed: f64) -> bool {
    horizontal_speed.is_finite() && horizontal_speed <= STILL_SPEED
}

 
 
pub(crate) fn standing_still(player: &LocalPlayer) -> bool {
    player
        .get_delta_movement()
        .map(|v| is_still((v.x() * v.x() + v.z() * v.z()).sqrt()))
        .unwrap_or(false)
}

 
 
 
 
 
 
 
 
 
pub(crate) fn moving_for_wire(player: &LocalPlayer) -> bool {
    !standing_still(player) || movement_key_held()
}

fn movement_key_held() -> bool {
    let Ok(options) = minecraft().options() else {
        return true;
    };
    [
        options.key_up(),
        options.key_back(),
        options.key_left(),
        options.key_right(),
    ]
    .into_iter()
    .any(|key| key.and_then(|key| key.is_down()).unwrap_or(true))
}

 
 
 
 
pub(crate) fn movement_input_held() -> bool {
    use crate::graphic::input::{key_down, FREE_KEY_A, FREE_KEY_D, FREE_KEY_S, FREE_KEY_W};
    [FREE_KEY_W, FREE_KEY_A, FREE_KEY_S, FREE_KEY_D]
        .into_iter()
        .any(key_down)
}

 
 
 
pub fn reach_distance() -> f32 {
    let Some(handle) = client().modules.get(crate::module::ModuleId::Reach) else {
        return 0.0;
    };
    let Ok(module) = handle.lock() else {
        return 0.0;
    };
    if !module.get_module_data().enabled || !reach::extends_reach_now() {
        return 0.0;
    }
    module
        .get_module_data()
        .get_setting("Distance")
        .and_then(|setting| setting.get_slider_value())
        .unwrap_or(0.0)
}

 
 
 
 
 
 
pub fn look_at(
    player: &LocalPlayer,
    target: &Entity,
    speed: f32,
    max_fov: f32,
    real_mouse: bool,
) -> anyhow::Result<f32> {
    let _eye = player.get_eye_position()?;
    let feet = target.get_position()?;
    let height = target.bb_height()? as f64;
     
    let aim = Vec3::new(feet.x(), feet.y() + height * 0.7, feet.z());
    look_at_position(player, aim, speed, max_fov, real_mouse)
}

pub fn look_at_position(
    player: &LocalPlayer,
    aim: Vec3,
    speed: f32,
    max_fov: f32,
    real_mouse: bool,
) -> anyhow::Result<f32> {
    let eye = player.get_eye_position()?;
    let target_rotation = Rotation::towards(eye, aim);

    let current = Rotation::new(player.get_yaw()?, player.get_pitch()?);
    let angle = current.angle_to(target_rotation);
    if angle <= max_fov {
        if real_mouse {
            rotation::aim_real_mouse(target_rotation, speed);
        } else {
            rotation::aim(target_rotation, speed);
        }
    }
    Ok(angle)
}

 
 
 
pub fn look_at_silent(
    player: &LocalPlayer,
    target: &Entity,
    speed: f32,
    max_fov: f32,
) -> anyhow::Result<f32> {
    let feet = target.get_position()?;
    let height = target.bb_height()? as f64;
     
     
     
     
     
     
    let aim = Vec3::new(feet.x(), feet.y() + height * 0.5, feet.z());
    look_at_position_silent(player, aim, speed, max_fov)
}

 
pub fn look_at_position_silent(
    player: &LocalPlayer,
    aim: Vec3,
    speed: f32,
    max_fov: f32,
) -> anyhow::Result<f32> {
    let eye = player.get_eye_position()?;
    let target_rotation = Rotation::towards(eye, aim);

    let current = Rotation::new(player.get_yaw()?, player.get_pitch()?);
    let angle = current.angle_to(target_rotation);
    if angle <= max_fov {
        rotation::aim_silent(target_rotation, speed);
    }
    Ok(angle)
}

 
 
 
 
pub fn pick_target(
    entities: &[Entity],
    eye: Vec3,
    range_sq: f64,
    exclude_id: i32,
    locked: Option<i32>,
    accept: impl Fn(&Entity) -> bool,
) -> Option<(i32, Entity)> {
    let mut candidates: Vec<(i32, Entity, f64)> = Vec::new();
    for entity in entities {
        let distance = match entity.distance_to_sqr(eye.x(), eye.y(), eye.z()) {
            Ok(distance) if distance <= range_sq => distance,
            _ => continue,
        };
        if !accept(entity) {
            continue;
        }
        let Ok(id) = entity.id() else {
            continue;
        };
        if id == exclude_id {
            continue;
        }
        candidates.push((id, entity.clone(), distance));
    }

     
    if let Some(locked) = locked {
        if let Some((id, entity, _)) = candidates.iter().find(|(id, _, _)| *id == locked) {
            return Some((*id, entity.clone()));
        }
    }
    candidates
        .into_iter()
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(id, entity, _)| (id, entity))
}

 
 
 
pub(crate) fn nearest_combatant(
    player: &LocalPlayer,
    range: f64,
    players: bool,
    mobs: bool,
) -> anyhow::Result<Option<Entity>> {
    if range <= 0.0 || !(players || mobs) {
        return Ok(None);
    }
    let Some(world) = minecraft().world()? else {
        return Ok(None);
    };
    let entities = world.get_entities()?;
    let eye = player.get_eye_position()?;
    Ok(pick_target(
        &entities,
        eye,
        range * range,
        player.id()?,
        None,
        |entity| {
            (players && entity.instance_of::<Player>()) || (mobs && entity.instance_of::<Mob>())
        },
    )
    .map(|(_, entity)| entity))
}

 
 
const VANILLA_REACH: f64 = 3.0;

 
 
const INTERACTION_RANGE_ATTRIBUTE: &str = "ENTITY_INTERACTION_RANGE";

 
 
 
 
const REACH_MARGIN: f64 = 0.1;

 
 
 
pub fn interaction_reach(player: &LocalPlayer) -> f64 {
    player
        .get_attribute_value(INTERACTION_RANGE_ATTRIBUTE)
        .ok()
        .flatten()
        .unwrap_or(VANILLA_REACH)
        .max(0.0)
}

 
 
 
 
 
 
 
 
pub fn within_reach(player: &LocalPlayer, target: &Entity) -> anyhow::Result<bool> {
    within_reach_up_to(player, target, f64::INFINITY)
}

 
 
 
pub fn within_reach_up_to(
    player: &LocalPlayer,
    target: &Entity,
    max_blocks: f64,
) -> anyhow::Result<bool> {
    let reach = effective_reach(interaction_reach(player), max_blocks);
    if reach <= 0.0 {
        return Ok(false);
    }
    let eye = player.get_eye_position()?;
    let (min, max) = target_box(target)?;
    Ok(point_box_distance([eye.x(), eye.y(), eye.z()], min, max) <= reach)
}

 
 
fn effective_reach(live_reach: f64, max_blocks: f64) -> f64 {
    live_reach.min(max_blocks) - REACH_MARGIN
}

 
 
fn target_box(target: &Entity) -> anyhow::Result<([f64; 3], [f64; 3])> {
    let feet = target.get_position()?;
    let half = target.bb_width()? as f64 / 2.0;
    let height = target.bb_height()? as f64;
    Ok((
        [feet.x() - half, feet.y(), feet.z() - half],
        [feet.x() + half, feet.y() + height, feet.z() + half],
    ))
}

 
 
fn point_box_distance(point: [f64; 3], min: [f64; 3], max: [f64; 3]) -> f64 {
    let mut sum = 0.0;
    for axis in 0..3 {
        let clamped = point[axis].clamp(min[axis], max[axis]);
        let delta = point[axis] - clamped;
        sum += delta * delta;
    }
    sum.sqrt()
}

 
 
 
 
pub fn crosshair_on_target(
    player: &LocalPlayer,
    target: &Entity,
    max_dist: f64,
) -> anyhow::Result<bool> {
    let eye = player.get_eye_position()?;
    let (min, max) = target_box(target)?;
    let dir = look_dir(player.get_yaw()?, player.get_pitch()?);
    Ok(ray_hits_box(
        [eye.x(), eye.y(), eye.z()],
        dir,
        min,
        max,
        max_dist,
    ))
}

 
 
 
 
pub fn shadow_crosshair_on_target(
    player: &LocalPlayer,
    target: &Entity,
    aim: Rotation,
    max_dist: f64,
) -> anyhow::Result<bool> {
    let eye = player.get_eye_position()?;
    let (min, max) = target_box(target)?;
    let dir = look_dir(aim.yaw, aim.pitch);
    Ok(ray_hits_box(
        [eye.x(), eye.y(), eye.z()],
        dir,
        min,
        max,
        max_dist,
    ))
}

 
 
pub(crate) fn look_dir(yaw: f32, pitch: f32) -> [f64; 3] {
    let yaw = yaw.to_radians() as f64;
    let pitch = pitch.to_radians() as f64;
    let cos_pitch = pitch.cos();
    [-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch]
}

 
 
pub(crate) fn ray_hits_box(
    origin: [f64; 3],
    dir: [f64; 3],
    min: [f64; 3],
    max: [f64; 3],
    max_dist: f64,
) -> bool {
    let mut t_min = 0.0_f64;
    let mut t_max = max_dist;
    for axis in 0..3 {
        if dir[axis].abs() < 1e-9 {
             
            if origin[axis] < min[axis] || origin[axis] > max[axis] {
                return false;
            }
        } else {
            let inv = 1.0 / dir[axis];
            let (mut t0, mut t1) = (
                (min[axis] - origin[axis]) * inv,
                (max[axis] - origin[axis]) * inv,
            );
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_min = t_min.max(t0);
            t_max = t_max.min(t1);
            if t_min > t_max {
                return false;
            }
        }
    }
    true
}

 
 
 
 
pub(crate) fn ray_hits_support_top(
    origin: [f64; 3],
    dir: [f64; 3],
    feet: [f64; 3],
    max_dist: f64,
) -> bool {
    if dir[1] >= -1e-9 {
        return false;
    }
    let t = (feet[1] - origin[1]) / dir[1];
    if !(0.0..=max_dist).contains(&t) {
        return false;
    }
    let x = origin[0] + dir[0] * t;
    let z = origin[2] + dir[2] * t;
    let (block_x, block_z) = (feet[0].floor(), feet[2].floor());
    x >= block_x && x <= block_x + 1.0 && z >= block_z && z <= block_z + 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

     
     
    #[test]
    fn a_reach_cap_clamps_boosted_reach_and_keeps_shorter_reach() {
        assert!((effective_reach(6.0, 3.0) - (3.0 - REACH_MARGIN)).abs() < 1e-9);
        assert!((effective_reach(3.0, 3.0) - (3.0 - REACH_MARGIN)).abs() < 1e-9);
        assert!((effective_reach(2.0, 3.0) - (2.0 - REACH_MARGIN)).abs() < 1e-9);
        assert!(effective_reach(f64::INFINITY, 3.0) < 3.0);
    }

     
    const MIN: [f64; 3] = [-0.3, 0.0, -0.3];
    const MAX: [f64; 3] = [0.3, 1.8, 0.3];

     
     
     
     
     
    #[test]
    fn a_look_direction_hits_only_the_box_it_points_at() {
        let origin = [0.0, 1.62, 0.0];
        let ahead = [-0.3, 1.0, 3.0];
        let ahead_max = [0.3, 2.8, 3.6];
        let dir = look_dir(0.0, 0.0);
        assert!(ray_hits_box(origin, dir, ahead, ahead_max, 4.5));
        assert!(!ray_hits_box(
            origin,
            look_dir(180.0, 0.0),
            ahead,
            ahead_max,
            4.5
        ));
        assert!(
            !ray_hits_box(origin, dir, ahead, ahead_max, 2.0),
            "a box past the reach distance is not hit"
        );
    }

    #[test]
    fn the_eye_to_hitbox_distance_is_zero_inside_the_box() {
        assert_eq!(point_box_distance([0.0, 0.9, 0.0], MIN, MAX), 0.0);
        assert_eq!(
            point_box_distance([0.3, 1.8, 0.3], MIN, MAX),
            0.0,
            "a point on the corner is still inside"
        );
        assert_eq!(
            point_box_distance([0.29, 1.79, -0.29], MIN, MAX),
            0.0,
            "an interior point is never measured out to a face"
        );
    }

     
     
    #[test]
    fn a_ray_down_onto_the_block_under_the_crystal_counts() {
        let feet = [0.5, 64.0, 0.5];
        let down = [0.0, -1.0, 0.0];
        assert!(ray_hits_support_top([0.5, 66.0, 0.5], down, feet, 4.5));
        assert!(
            ray_hits_support_top([0.1, 66.0, 0.9], down, feet, 4.5),
            "any point of the footprint is the block's top"
        );
    }

    #[test]
    fn the_block_top_needs_the_footprint_the_distance_and_a_downward_ray() {
        let feet = [0.5, 64.0, 0.5];
        let down = [0.0, -1.0, 0.0];
        assert!(
            !ray_hits_support_top([1.5, 66.0, 0.5], down, feet, 4.5),
            "a ray past the block's edge lands on the next block"
        );
        assert!(
            !ray_hits_support_top([0.5, 70.0, 0.5], down, feet, 4.5),
            "a top six blocks up is beyond the reach gate"
        );
        assert!(
            !ray_hits_support_top([0.5, 66.0, 0.5], [0.0, 1.0, 0.0], feet, 4.5),
            "an upward ray never reaches the block's top"
        );
        assert!(
            !ray_hits_support_top([0.5, 63.0, 0.5], down, feet, 4.5),
            "a ray starting below the top never reaches it"
        );
    }

    #[test]
    fn the_eye_to_hitbox_distance_measures_the_gap_to_the_nearest_face() {
        let eye_at = |point: [f64; 3]| point_box_distance(point, MIN, MAX);
         
         
        assert!((eye_at([1.3, 1.62, 0.0]) - 1.0).abs() < 1e-9);
         
        assert!((eye_at([0.0, 2.8, 0.0]) - 1.0).abs() < 1e-9);
         
        assert!((eye_at([0.0, 1.62, -3.3]) - 3.0).abs() < 1e-9);
         
         
        let diagonal = eye_at([1.3, 2.8, 0.0]);
        assert!((diagonal - 2.0_f64.sqrt()).abs() < 1e-9);
    }

     
     
     
    #[test]
    fn a_target_at_vanilla_reach_is_inside_the_gate_but_not_by_the_feet() {
        let eye = [0.0, 1.62, 0.0];
        let feet_away = 3.0;
        let box_min = [-0.3, 0.0, feet_away - 0.3];
        let box_max = [0.3, 1.8, feet_away + 0.3];
        let to_hitbox = point_box_distance(eye, box_min, box_max);
        assert!(to_hitbox <= VANILLA_REACH);
        let to_feet = (feet_away * feet_away + 1.62 * 1.62_f64).sqrt();
        assert!(
            to_feet > VANILLA_REACH,
            "eye-to-feet is longer than the reach at 3 blocks, so it must not be the gate"
        );
    }

    #[test]
    fn straight_ray_hits_centered_box() {
        assert!(ray_hits_box(
            [0.0, 1.6, -3.0],
            [0.0, 0.0, 1.0],
            MIN,
            MAX,
            4.0,
        ));
    }

    #[test]
    fn ray_beside_the_box_misses() {
        assert!(!ray_hits_box(
            [1.0, 1.6, -3.0],
            [0.0, 0.0, 1.0],
            MIN,
            MAX,
            4.0,
        ));
    }

    #[test]
    fn ray_beyond_max_distance_misses() {
        assert!(!ray_hits_box(
            [0.0, 1.6, -3.0],
            [0.0, 0.0, 1.0],
            MIN,
            MAX,
            2.0,
        ));
    }

    #[test]
    fn ray_from_inside_the_box_hits() {
        assert!(ray_hits_box(
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            MIN,
            MAX,
            4.0,
        ));
    }

    #[test]
    fn diagonal_ray_hits_the_box_but_not_the_air_beside_it() {
        let diagonal = [0.7071067811865475, 0.0, 0.7071067811865475];
         
        assert!(ray_hits_box([-3.0, 1.0, -3.0], diagonal, MIN, MAX, 5.0));
         
        assert!(!ray_hits_box([-3.0, 1.0, -4.0], diagonal, MIN, MAX, 5.0));
    }

     
     
     
     
    #[test]
    fn aiming_at_a_target_puts_the_crosshair_ray_on_their_hitbox() {
        let eye = Vec3::new(0.0, 1.62, 0.0);

         
        let rotation = Rotation::towards(eye, Vec3::new(0.0, 1.8 * 0.7, 3.0));
        let dir = look_dir(rotation.yaw, rotation.pitch);
        assert!(ray_hits_box(
            [0.0, 1.62, 0.0],
            dir,
            [-0.3, 0.0, 2.7],
            [0.3, 1.8, 3.3],
            4.0,
        ));

         
         
        let rotation = Rotation::towards(eye, Vec3::new(3.0, 1.8 * 0.7, 0.0));
        let dir = look_dir(rotation.yaw, rotation.pitch);
        assert!(ray_hits_box(
            [0.0, 1.62, 0.0],
            dir,
            [2.7, 0.0, -0.3],
            [3.3, 1.8, 0.3],
            4.0,
        ));
    }

     
     
     
    #[test]
    fn a_silent_aim_is_judged_on_the_shadow_not_the_camera() {
        let eye = Vec3::new(0.0, 1.62, 0.0);
        let target_min = [-0.3, 0.0, 2.7];
        let target_max = [0.3, 1.8, 3.3];

         
        let shadow = Rotation::towards(eye, Vec3::new(0.0, 1.8 * 0.7, 3.0));
        assert!(ray_hits_box(
            [0.0, 1.62, 0.0],
            look_dir(shadow.yaw, shadow.pitch),
            target_min,
            target_max,
            4.0,
        ));

         
        let camera = Rotation::towards(eye, Vec3::new(3.0, 1.62, 0.0));
        assert!(!ray_hits_box(
            [0.0, 1.62, 0.0],
            look_dir(camera.yaw, camera.pitch),
            target_min,
            target_max,
            4.0,
        ));
    }
}
