 
 
 
 

use crate::mapping::MinecraftClassType;
use crate::state::mapping;
use jni::objects::{JObject, JValue};
use jni::sys::jobject;
use jni::JNIEnv;

 
pub const CLASS_TYPE: MinecraftClassType = MinecraftClassType::ClientboundSetEntityMotionPacket;

 
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClientboundSetEntityMotionPacket {
     
    pub entity_id: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl ClientboundSetEntityMotionPacket {
     
    pub fn read(
        _env: &mut JNIEnv,
        packet: &JObject,
    ) -> anyhow::Result<ClientboundSetEntityMotionPacket> {
        let entity_id = mapping()
            .call_method(CLASS_TYPE, packet, "getId", &[])?
            .i()?;

         
         
         
         
        if let Ok(movement) = mapping()
            .call_method(CLASS_TYPE, packet, "getMovement", &[])
            .and_then(|value| Ok(value.l()?))
        {
            let vec = crate::mapping::math::Vec3::read(&movement)?;
            return Ok(ClientboundSetEntityMotionPacket {
                entity_id,
                x: vec.x(),
                y: vec.y(),
                z: vec.z(),
            });
        }

        Ok(ClientboundSetEntityMotionPacket {
            entity_id,
            x: mapping()
                .call_method(CLASS_TYPE, packet, "getXa", &[])?
                .d()?,
            y: mapping()
                .call_method(CLASS_TYPE, packet, "getYa", &[])?
                .d()?,
            z: mapping()
                .call_method(CLASS_TYPE, packet, "getZa", &[])?
                .d()?,
        })
    }

     
     
    pub fn to_java(self, env: &mut JNIEnv) -> anyhow::Result<jobject> {
        let vec3_class = mapping().resolve_class(
            env,
            &mapping().runtime_class_name(MinecraftClassType::Vec3)?,
        )?;
        let movement = env.new_object(
            &vec3_class,
            "(DDD)V",
            &[
                JValue::Double(self.x),
                JValue::Double(self.y),
                JValue::Double(self.z),
            ],
        )?;

        let packet_class =
            mapping().resolve_class(env, &mapping().runtime_class_name(CLASS_TYPE)?)?;
        let packet_desc = format!(
            "(IL{};)V",
            mapping().runtime_class_name(MinecraftClassType::Vec3)?
        );
        let packet = env.new_object(
            &packet_class,
            &packet_desc,
            &[JValue::Int(self.entity_id), JValue::Object(&movement)],
        )?;
        Ok(packet.into_raw())
    }
}
