 
 
 
 
 
 
 
 
 

use crate::mapping::{FieldType, MinecraftClassType};
use crate::module::combat::rotation::Rotation;
use crate::state::mapping;
use jni::objects::{JObject, JValue};
use jni::sys::jobject;
use jni::JNIEnv;

 
pub const CLASS_TYPE: MinecraftClassType = MinecraftClassType::ServerboundUseItemPacket;

 
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServerboundUseItemPacket {
     
    pub main_hand: bool,
     
     
     
    pub sequence: i32,
    pub y_rot: f32,
    pub x_rot: f32,
}

impl ServerboundUseItemPacket {
     
    pub fn read(env: &mut JNIEnv, packet: &JObject) -> anyhow::Result<Self> {
        let hand = mapping()
            .call_method(CLASS_TYPE, packet, "getHand", &[])?
            .l()?;
        let main_hand = mapping()
            .get_static_field(
                MinecraftClassType::InteractionHand,
                "MAIN_HAND",
                FieldType::Object(MinecraftClassType::InteractionHand),
            )?
            .l()?;
        let main_hand = env.is_same_object(&hand, &main_hand)?;
        Ok(Self {
            main_hand,
            sequence: mapping()
                .call_method(CLASS_TYPE, packet, "getSequence", &[])?
                .i()?,
            y_rot: mapping()
                .call_method(CLASS_TYPE, packet, "getYRot", &[])?
                .f()?,
            x_rot: mapping()
                .call_method(CLASS_TYPE, packet, "getXRot", &[])?
                .f()?,
        })
    }

     
     
    pub fn apply_aim(&mut self, aim: Rotation) {
        self.y_rot = aim.yaw;
        self.x_rot = aim.pitch;
    }

     
    pub fn to_java(self, env: &mut JNIEnv) -> anyhow::Result<jobject> {
        let hand_name = if self.main_hand {
            "MAIN_HAND"
        } else {
            "OFF_HAND"
        };
        let hand = mapping()
            .get_static_field(
                MinecraftClassType::InteractionHand,
                hand_name,
                FieldType::Object(MinecraftClassType::InteractionHand),
            )?
            .l()?;
        let runtime = mapping().runtime_class_name(CLASS_TYPE)?;
        let class = mapping().resolve_class(env, &runtime)?;
        let hand_class = mapping().runtime_class_name(MinecraftClassType::InteractionHand)?;
        let descriptor = format!("(L{hand_class};IFF)V");
        let object = env.new_object(
            &class,
            &descriptor,
            &[
                JValue::Object(&hand),
                JValue::Int(self.sequence),
                JValue::Float(self.y_rot),
                JValue::Float(self.x_rot),
            ],
        )?;
        Ok(object.into_raw())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> ServerboundUseItemPacket {
        ServerboundUseItemPacket {
            main_hand: true,
            sequence: 7,
            y_rot: 30.0,
            x_rot: -10.0,
        }
    }

     
     
    #[test]
    fn the_class_type_points_at_the_use_item_packet() {
        assert_eq!(CLASS_TYPE, MinecraftClassType::ServerboundUseItemPacket);
        assert_eq!(
            CLASS_TYPE.get_name(),
            "net/minecraft/network/protocol/game/ServerboundUseItemPacket"
        );
    }

     
     
    #[test]
    fn an_aim_replaces_the_angles_and_nothing_else() {
        let mut packet = snapshot();
        packet.apply_aim(Rotation::new(-120.0, 90.0));
        assert_eq!(packet.y_rot, -120.0);
        assert_eq!(packet.x_rot, 90.0);
        assert_eq!(packet.sequence, 7);
        assert!(packet.main_hand);
         
        assert_ne!(packet, snapshot());
    }
}
