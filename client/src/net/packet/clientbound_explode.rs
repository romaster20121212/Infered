 
 
 
 
 
 
 
 
 
 

use crate::mapping::MinecraftClassType;
use crate::state::mapping;
use jni::objects::{GlobalRef, JObject};
use jni::JNIEnv;

 
pub const CLASS_TYPE: MinecraftClassType = MinecraftClassType::ClientboundExplodePacket;

 
#[derive(Debug, Clone)]
pub struct ClientboundExplodePacket {
     
    pub center: [f64; 3],
     
     
     
    pub radius: Option<f32>,
     
    pub jni_ref: GlobalRef,
}

impl ClientboundExplodePacket {
     
    pub fn read(env: &mut JNIEnv, packet: &JObject) -> anyhow::Result<ClientboundExplodePacket> {
        let center = mapping()
            .call_method(CLASS_TYPE, packet, "center", &[])?
            .l()?;
        let center = crate::mapping::math::Vec3::read(&center)?;
         
         
        let radius = mapping()
            .call_method(CLASS_TYPE, packet, "radius", &[])
            .and_then(|value| Ok(value.f()?))
            .ok()
            .filter(|radius| radius.is_finite() && *radius > 0.0);

        Ok(ClientboundExplodePacket {
            center: [center.x(), center.y(), center.z()],
            radius,
            jni_ref: env.new_global_ref(packet)?,
        })
    }
}

 
 
 
impl PartialEq for ClientboundExplodePacket {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

     
     
    #[test]
    fn the_class_type_points_at_the_explosion_packet() {
        assert_eq!(CLASS_TYPE, MinecraftClassType::ClientboundExplodePacket);
        assert_eq!(
            CLASS_TYPE.get_name(),
            "net/minecraft/network/protocol/game/ClientboundExplodePacket"
        );
    }
}
