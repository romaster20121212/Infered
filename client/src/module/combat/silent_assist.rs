 
 
 
 
 
 
 
 
 
 

use crate::friends::ignores_entity;
use crate::mapping::entity::player::{LocalPlayer, Player};
use crate::mapping::entity::{Entity, EntityRef, LivingEntityRef};
use crate::mapping::math::Vec3;
use crate::mapping::{FieldType, MappedObject};
use crate::module::combat::interaction::{self, Action};
use crate::module::combat::rotation::{self, BodyAimOwner, Rotation};
use crate::module::combat::{
    interaction_reach, look_dir, shadow_crosshair_on_target, target_box, within_reach_up_to,
};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::net::packet::serverbound_move_player::ServerboundMovePlayerPacket;
use crate::state::{mapping, minecraft};
use jni::objects::JValue;

const DEFAULT_STRENGTH: f32 = 50.0;
const STRENGTH_RANGE: (f32, f32) = (0.0, 100.0);

 
const MAX_CORRECTION_DEGREES: f32 = 10.0;

 
 
 
fn max_correction(strength: f32) -> f32 {
    let clamped = strength.clamp(STRENGTH_RANGE.0, STRENGTH_RANGE.1);
    clamped / STRENGTH_RANGE.1 * MAX_CORRECTION_DEGREES
}

 
#[derive(Debug)]
pub struct SilentAssistModule {
    pub module: ModuleData,
}

impl SilentAssistModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::SilentAssist,
                description: "Nudges a near-miss swing onto a player with a silent aim the camera never shows".to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Strength".into(),
                    value: DEFAULT_STRENGTH,
                    min: STRENGTH_RANGE.0,
                    max: STRENGTH_RANGE.1,
                }],
            },
        }
    }

     
    fn strength(&self) -> f32 {
        self.module
            .get_setting("Strength")
            .and_then(ModuleSetting::get_slider_value)
            .filter(|value| value.is_finite())
            .unwrap_or(DEFAULT_STRENGTH)
            .clamp(STRENGTH_RANGE.0, STRENGTH_RANGE.1)
    }

     
     
    fn assist(&self) -> anyhow::Result<()> {
        let strength = self.strength();
        let mc = minecraft();
        if strength <= 0.0 || mc.current_screen().is_open() {
            return Ok(());
        }
        let Some(player) = mc.player()? else {
            return Ok(());
        };
        let options = mc.options()?;
        let attack = options.key_attack()?;
        let clicks = attack.get_field("clickCount", FieldType::Int)?.i()?;
        let use_key = options.key_use()?;
         
         
        if clicks <= 0
            || mc.get_field("missTime", FieldType::Int)?.i()? > 0
            || use_key.is_down()?
            || use_key.get_field("clickCount", FieldType::Int)?.i()? > 0
            || !can_body_aim()
            || mc.crosshair_entity()?.is_some()
        {
            return Ok(());
        }
        let range = interaction_reach(&player);
        let view = Rotation::new(player.get_yaw()?, player.get_pitch()?);
        let Some((target, aim)) = nearest_target(&player, view, max_correction(strength), range)?
        else {
            return Ok(());
        };
        let Some(game_mode) = mc.game_mode()? else {
            return Ok(());
        };
        if rotation::aim_body_for(BodyAimOwner::SilentAssist, aim).is_err() {
            return Ok(());
        }
        if !interaction::take(&player, Action::Entity) {
            let _ = rotation::restore_body_for(BodyAimOwner::SilentAssist);
            return Ok(());
        }
         
         
        if let Err(error) = attack.set_field("clickCount", FieldType::Int, JValue::Int(clicks - 1))
        {
            let _ = rotation::restore_body_for(BodyAimOwner::SilentAssist);
            return Err(error);
        }
         
         
         
        if let Err(error) = send_attack_look(&player) {
            let _ = attack.set_field("clickCount", FieldType::Int, JValue::Int(clicks));
            let _ = rotation::restore_body_for(BodyAimOwner::SilentAssist);
            return Err(error);
        }
        if player.is_using_item().unwrap_or(false) {
            let _ = game_mode.release_using_item(&player);
        }
        if let Err(error) = game_mode.attack(&player, &target) {
            let _ = attack.set_field("clickCount", FieldType::Int, JValue::Int(clicks));
            let _ = rotation::restore_body_for(BodyAimOwner::SilentAssist);
            return Err(error);
        }
        let _ = player.swing();
         
         
        if mc.hit_block()?.is_some() {
            game_mode.stop_destroy_block()?;
            mc.set_field("missTime", FieldType::Int, JValue::Int(1))?;
        }
        Ok(())
    }
}

 
 
fn send_attack_look(player: &LocalPlayer) -> anyhow::Result<()> {
    let packet = ServerboundMovePlayerPacket {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        y_rot: player.get_yaw()?,
        x_rot: player.get_pitch()?,
        on_ground: player.on_ground()?,
        horizontal_collision: player
            .get_field("horizontalCollision", FieldType::Boolean)?
            .z()?,
        has_position: false,
        has_rotation: true,
    };
    let mut env = mapping().get_env()?;
    env.with_local_frame(8, |env| -> anyhow::Result<()> {
        crate::net::send_packet(packet.to_java(env)?)
    })
}

 
fn valid_player_target(player: &LocalPlayer, entity: &Entity, module_id: ModuleId) -> bool {
    if !entity.instance_of::<Player>() || entity.id().ok() == player.id().ok() {
        return false;
    }
    if entity.is_removed().unwrap_or(true) || entity.is_spectator().unwrap_or(true) {
        return false;
    }
    if !entity
        .as_living()
        .is_some_and(|living| living.is_alive().unwrap_or(false))
    {
        return false;
    }
    !ignores_entity(entity, module_id)
}

 
 
fn aim_rotation(player: &LocalPlayer, target: &Entity) -> anyhow::Result<Rotation> {
    let eye = player.get_eye_position()?;
    let feet = target.get_position()?;
    let height = target.bb_height()? as f64;
    Ok(Rotation::towards(
        eye,
        Vec3::new(feet.x(), feet.y() + height * 0.5, feet.z()),
    ))
}

 
 
fn can_body_aim() -> bool {
    let first_person = minecraft()
        .options()
        .and_then(|options| options.perspective())
        .map(|perspective| perspective == crate::mapping::client::options::Perspective::FirstPerson)
        .unwrap_or(true);
    first_person
        && crate::jvmti::camera_frame_hook_installed()
        && crate::jvmti::crystal_input_hook_installed()
}

 
 
 
fn hitbox_offset(eye: Vec3, view: Rotation, min: [f64; 3], max: [f64; 3], range: f64) -> f32 {
    let origin = [eye.x(), eye.y(), eye.z()];
    let dir = look_dir(view.yaw, view.pitch);
    let point_at = |distance: f64| {
        std::array::from_fn::<_, 3, _>(|axis| {
            (origin[axis] + dir[axis] * distance).clamp(min[axis], max[axis])
        })
    };
    let distance_squared = |distance: f64| {
        let point = point_at(distance);
        (0..3)
            .map(|axis| (origin[axis] + dir[axis] * distance - point[axis]).powi(2))
            .sum::<f64>()
    };
    let (mut lo, mut hi) = (0.0, range);
    for _ in 0..40 {
        let a = lo + (hi - lo) / 3.0;
        let b = hi - (hi - lo) / 3.0;
        if distance_squared(a) <= distance_squared(b) {
            hi = b;
        } else {
            lo = a;
        }
    }
    let point = point_at((lo + hi) / 2.0);
    view.angle_to(Rotation::towards(
        eye,
        Vec3::new(point[0], point[1], point[2]),
    ))
}

 
 
fn nearest_target(
    player: &LocalPlayer,
    view: Rotation,
    max_degrees: f32,
    range: f64,
) -> anyhow::Result<Option<(Entity, Rotation)>> {
    let Some(world) = minecraft().world()? else {
        return Ok(None);
    };
    let eye = player.get_eye_position()?;
    let mut best: Option<(f32, Entity, Rotation)> = None;
    for entity in world.get_entities()? {
        if !valid_player_target(player, &entity, ModuleId::SilentAssist)
            || !within_reach_up_to(player, &entity, range)?
        {
            continue;
        }
        let (min, max) = target_box(&entity)?;
        let offset = hitbox_offset(eye, view, min, max, range);
        if !offset.is_finite() || offset > max_degrees {
            continue;
        }
        let visible = player
            .call_method(
                "hasLineOfSight",
                &[JValue::Object(entity.jni_ref().as_obj())],
            )
            .and_then(|value| Ok(value.z()?))
            .unwrap_or(false);
        if !visible {
            continue;
        }
        if best.as_ref().map_or(true, |(best, _, _)| offset < *best) {
            let aim = aim_rotation(player, &entity)?;
            if shadow_crosshair_on_target(player, &entity, aim, range)? {
                best = Some((offset, entity, aim));
            }
        }
    }
    Ok(best.map(|(_, entity, aim)| (entity, aim)))
}

impl Module for SilentAssistModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
         
         
        let _ = rotation::restore_body_for(BodyAimOwner::SilentAssist);
        Ok(())
    }

    fn on_input_phase(&self) -> anyhow::Result<()> {
        self.assist()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
         
         
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
        let _ = rotation::restore_body_for(BodyAimOwner::SilentAssist);
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

    #[test]
    fn strength_scales_the_correction() {
        assert_eq!(max_correction(0.0), 0.0);
        assert_eq!(max_correction(100.0), MAX_CORRECTION_DEGREES);
        assert!((max_correction(50.0) - 5.0).abs() < 1e-6);
        assert_eq!(max_correction(250.0), MAX_CORRECTION_DEGREES);
        assert_eq!(max_correction(-10.0), 0.0);
    }

    #[test]
    fn correction_is_measured_from_the_hitbox_not_its_centre() {
        let eye = Vec3::new(0.0, 1.62, 0.0);
        let min = [-0.3, 0.0, 2.5];
        let max = [0.3, 1.8, 3.1];
        let view = Rotation::new(9.0, 0.0);
        let offset = hitbox_offset(eye, view, min, max, 3.0);
        assert!(offset > 0.0 && offset < max_correction(50.0));
        let centre = Rotation::towards(eye, Vec3::new(0.0, 0.9, 2.8));
        assert!(view.angle_to(centre) > max_correction(50.0));
        assert!(
            hitbox_offset(eye, Rotation::new(25.0, 0.0), min, max, 3.0) > max_correction(100.0)
        );
        assert!(hitbox_offset(eye, Rotation::new(0.0, -3.0), min, max, 3.0) < 0.1);
    }

    #[test]
    fn strength_setting_is_offered_with_its_bounds() {
        let module = SilentAssistModule::new();
        let Some(ModuleSetting::Slider {
            value, min, max, ..
        }) = module.get_module_data().get_setting("Strength")
        else {
            panic!("Silent Assist must expose a Strength slider");
        };
        assert_eq!((*value, *min, *max), (50.0, 0.0, 100.0));
    }
}
