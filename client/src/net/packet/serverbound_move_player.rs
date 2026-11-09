 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::entity::EntityRef;
use crate::mapping::MinecraftClassType;
use crate::module::combat::rotation::{align_yaw, Rotation};
use crate::module::misc::freelook;
use crate::state::{mapping, minecraft};
use jni::objects::{JClass, JObject, JValue};
use jni::sys::jobject;
use jni::JNIEnv;
use std::sync::Mutex;

 
pub const CLASS_TYPE: MinecraftClassType = MinecraftClassType::ServerboundMovePlayerPacket;

 
 
static LAST_STAMPED: Mutex<Option<Rotation>> = Mutex::new(None);

 
 
 
 
 
 
 
pub fn silent_aim_on_wire() -> Option<Rotation> {
    *LAST_STAMPED.lock().unwrap_or_else(|p| p.into_inner())
}

 
 
 
 
pub(crate) fn forget_silent_aim() {
    let mut last = LAST_STAMPED.lock().unwrap_or_else(|p| p.into_inner());
    *last = None;
}

 
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServerboundMovePlayerPacket {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
    pub horizontal_collision: bool,
    pub has_position: bool,
    pub has_rotation: bool,
}

impl ServerboundMovePlayerPacket {
     
    pub fn read(
        _env: &mut JNIEnv,
        packet: &JObject,
    ) -> anyhow::Result<ServerboundMovePlayerPacket> {
         
         
         
        let mut snapshot = ServerboundMovePlayerPacket {
            x: mapping()
                .call_method(CLASS_TYPE, packet, "getX", &[JValue::Double(0.0)])?
                .d()?,
            y: mapping()
                .call_method(CLASS_TYPE, packet, "getY", &[JValue::Double(0.0)])?
                .d()?,
            z: mapping()
                .call_method(CLASS_TYPE, packet, "getZ", &[JValue::Double(0.0)])?
                .d()?,
            y_rot: mapping()
                .call_method(CLASS_TYPE, packet, "getYRot", &[JValue::Float(0.0)])?
                .f()?,
            x_rot: mapping()
                .call_method(CLASS_TYPE, packet, "getXRot", &[JValue::Float(0.0)])?
                .f()?,
            on_ground: mapping()
                .call_method(CLASS_TYPE, packet, "isOnGround", &[])?
                .z()?,
            horizontal_collision: mapping()
                .call_method(CLASS_TYPE, packet, "horizontalCollision", &[])?
                .z()?,
            has_position: mapping()
                .call_method(CLASS_TYPE, packet, "hasPosition", &[])?
                .z()?,
            has_rotation: mapping()
                .call_method(CLASS_TYPE, packet, "hasRotation", &[])?
                .z()?,
        };
        if freelook::body_view_requested() && snapshot.has_rotation {
            if let Some(player) = minecraft().player().ok().flatten() {
                if let (Ok(yaw), Ok(pitch)) = (player.get_yaw(), player.get_pitch()) {
                    snapshot.y_rot = yaw;
                    snapshot.x_rot = pitch;
                    snapshot.drop_repeated_rotation(Rotation::new(yaw, pitch));
                }
            }
        }
        Ok(snapshot)
    }

     
     
     
    fn drop_repeated_rotation(&mut self, angle: Rotation) {
        if !self.has_rotation {
            return;
        }
        let mut last = LAST_STAMPED.lock().unwrap_or_else(|p| p.into_inner());
        if *last == Some(angle) {
            self.has_rotation = false;
        } else {
            *last = Some(angle);
        }
    }

     
     
     
     
     
     
     
     
     
     
     
    pub fn apply_silent_aim(&mut self, aim: Option<Rotation>) {
        let mut last = LAST_STAMPED.lock().unwrap_or_else(|p| p.into_inner());
        let Some(aim) = aim else {
             
             
             
             
             
            if self.has_rotation {
                *last = Some(Rotation::new(self.y_rot, self.x_rot));
            }
            return;
        };
        if *last == Some(aim) {
             
             
            self.has_rotation = false;
            return;
        }
         
         
         
         
         
        let reference = last.as_ref().map_or(self.y_rot, |rotation| rotation.yaw);
        let aligned = Rotation::new(align_yaw(aim.yaw, reference), aim.pitch);
        if *last == Some(aligned) {
             
            self.has_rotation = false;
            return;
        }
        let aim = aligned;
        *last = Some(aim);
        self.y_rot = aim.yaw;
        self.x_rot = aim.pitch;
        self.has_rotation = true;
    }

     
     
    pub fn to_java(self, env: &mut JNIEnv) -> anyhow::Result<jobject> {
         
         
        fn subclass_class<'e>(
            env: &mut JNIEnv<'e>,
            class: MinecraftClassType,
        ) -> anyhow::Result<JClass<'e>> {
            let runtime = mapping().runtime_class_name(class)?;
            mapping().resolve_class(env, &runtime)
        }

        let on_ground = JValue::Bool(self.on_ground as u8);
        let collision = JValue::Bool(self.horizontal_collision as u8);

        let object = match (self.has_position, self.has_rotation) {
            (true, true) => {
                let class =
                    subclass_class(env, MinecraftClassType::ServerboundMovePlayerPacketPosRot)?;
                env.new_object(
                    &class,
                    "(DDDFFZZ)V",
                    &[
                        JValue::Double(self.x),
                        JValue::Double(self.y),
                        JValue::Double(self.z),
                        JValue::Float(self.y_rot),
                        JValue::Float(self.x_rot),
                        on_ground,
                        collision,
                    ],
                )?
            }
            (true, false) => {
                let class =
                    subclass_class(env, MinecraftClassType::ServerboundMovePlayerPacketPos)?;
                env.new_object(
                    &class,
                    "(DDDZZ)V",
                    &[
                        JValue::Double(self.x),
                        JValue::Double(self.y),
                        JValue::Double(self.z),
                        on_ground,
                        collision,
                    ],
                )?
            }
            (false, true) => {
                let class =
                    subclass_class(env, MinecraftClassType::ServerboundMovePlayerPacketRot)?;
                env.new_object(
                    &class,
                    "(FFZZ)V",
                    &[
                        JValue::Float(self.y_rot),
                        JValue::Float(self.x_rot),
                        on_ground,
                        collision,
                    ],
                )?
            }
            (false, false) => {
                let class = subclass_class(
                    env,
                    MinecraftClassType::ServerboundMovePlayerPacketStatusOnly,
                )?;
                env.new_object(&class, "(ZZ)V", &[on_ground, collision])?
            }
        };
        Ok(object.into_raw())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(has_position: bool, has_rotation: bool) -> ServerboundMovePlayerPacket {
        ServerboundMovePlayerPacket {
            x: 12.0,
            y: 64.0,
            z: -9.0,
            y_rot: 45.0,
            x_rot: 20.0,
            on_ground: false,
            horizontal_collision: true,
            has_position,
            has_rotation,
        }
    }

     
    #[test]
    fn repeated_body_view_rotation_is_dropped_and_new_one_kept() {
        let _guard = crate::module::combat::rotation::test_guard();
        forget_silent_aim();
        let mut first = snapshot(true, true);
        first.drop_repeated_rotation(Rotation::new(45.0, 20.0));
        assert!(
            first.has_rotation,
            "the first angle on the wire must be sent"
        );
        let mut repeat = snapshot(true, true);
        repeat.drop_repeated_rotation(Rotation::new(45.0, 20.0));
        assert!(
            !repeat.has_rotation,
            "an unchanged body angle must not be resent"
        );
        assert_eq!((repeat.x, repeat.y, repeat.z), (12.0, 64.0, -9.0));
        let mut moved = snapshot(true, true);
        moved.drop_repeated_rotation(Rotation::new(60.0, 20.0));
        assert!(moved.has_rotation, "a changed body angle must be sent");
        forget_silent_aim();
    }

     
     
     
    #[test]
    fn no_aim_leaves_every_packet_shape_untouched() {
        let _guard = crate::module::combat::rotation::test_guard();
        for (has_position, has_rotation) in
            [(true, true), (true, false), (false, true), (false, false)]
        {
            let original = snapshot(has_position, has_rotation);
            let mut out = original;
            out.apply_silent_aim(None);
            assert_eq!(out, original, "an absent aim must be a no-op");
        }
    }

     
     
     
    #[test]
    fn a_silent_aim_is_stamped_on_every_packet_shape() {
        let _guard = crate::module::combat::rotation::test_guard();
        for (has_position, has_rotation) in
            [(true, true), (true, false), (false, true), (false, false)]
        {
             
             
             
            let mut camera = snapshot(false, true);
            camera.apply_silent_aim(None);
            let mut out = snapshot(has_position, has_rotation);
            out.apply_silent_aim(Some(Rotation::new(-31.5, 14.25)));
            assert_eq!(out.y_rot, -31.5);
            assert_eq!(out.x_rot, 14.25);
            assert!(
                out.has_rotation,
                "the aim cannot reach the server without rotation"
            );
            assert_eq!(
                (out.x, out.y, out.z, out.has_position),
                (12.0, 64.0, -9.0, has_position),
                "position and its presence flag are not the aim's business"
            );
        }
    }

     
     
     
    #[test]
    fn the_angle_on_the_wire_is_the_last_aim_a_packet_carried() {
        let _guard = crate::module::combat::rotation::test_guard();
         
        forget_silent_aim();
        assert_eq!(silent_aim_on_wire(), None, "nothing has been sent yet");

        let aim = Rotation::new(-44.0, 71.0);
        let mut movement = snapshot(true, false);
        movement.apply_silent_aim(Some(aim));
        assert!(movement.has_rotation);
        assert_eq!(silent_aim_on_wire(), Some(aim));

         
         
        let mut camera = snapshot(false, true);
        camera.apply_silent_aim(None);
        assert_eq!(
            silent_aim_on_wire(),
            Some(Rotation::new(camera.y_rot, camera.x_rot))
        );
    }

     
     
     
    #[test]
    fn only_a_real_camera_rotation_invalidates_the_stamp() {
        let _guard = crate::module::combat::rotation::test_guard();
        let aim = Rotation::new(80.0, 12.0);

        let mut first = snapshot(false, true);
        first.apply_silent_aim(None);
        first.apply_silent_aim(Some(aim));
        assert!(first.has_rotation, "a new aim is sent");

         
        let mut position_only = snapshot(true, false);
        position_only.apply_silent_aim(None);
        let mut repeat = snapshot(false, true);
        repeat.apply_silent_aim(Some(aim));
        assert!(!repeat.has_rotation, "the server already holds this angle");

         
        let mut camera_turn = snapshot(false, true);
        camera_turn.apply_silent_aim(None);
        let mut resend = snapshot(false, true);
        resend.apply_silent_aim(Some(aim));
        assert!(
            resend.has_rotation,
            "the camera's angle is on the wire now, so the aim is new"
        );
        resend.apply_silent_aim(None);
    }

     
     
     
     
    #[test]
    fn an_unchanged_aim_is_not_resent_as_rotation() {
        let _guard = crate::module::combat::rotation::test_guard();
        let aim = Rotation::new(12.5, -3.0);

        let mut first = snapshot(true, true);
        first.apply_silent_aim(None);
        first.apply_silent_aim(Some(aim));
        assert!(first.has_rotation, "a new aim is sent");

        let mut repeat = snapshot(true, true);
        repeat.apply_silent_aim(Some(aim));
        assert!(!repeat.has_rotation, "an unchanged aim is not resent");
        assert!(repeat.has_position, "the position still goes out");

        let mut moved = snapshot(true, true);
        moved.apply_silent_aim(Some(Rotation::new(13.0, -3.0)));
        assert!(moved.has_rotation, "a moved aim is sent again");

        let mut released = snapshot(true, true);
        released.apply_silent_aim(None);
        released.apply_silent_aim(Some(aim));
        assert!(
            released.has_rotation,
            "after a release the aim is sent in full"
        );
    }
}
