 
 
 
 
 
 
 
 
 
 

use crate::mapping::MinecraftClassType;
use crate::state::mapping;
use jni::objects::{GlobalRef, JObject, JString};
use jni::JNIEnv;

 
pub const CLASS_TYPE: MinecraftClassType = MinecraftClassType::ClientboundCustomPayloadPacket;

 
#[derive(Debug, Clone)]
pub struct ClientboundCustomPayloadPacket {
     
     
     
    pub brand: Option<String>,
     
    pub jni_ref: GlobalRef,
}

impl ClientboundCustomPayloadPacket {
     
    pub fn read(
        env: &mut JNIEnv,
        packet: &JObject,
    ) -> anyhow::Result<ClientboundCustomPayloadPacket> {
        let payload = mapping()
            .call_method(CLASS_TYPE, packet, "payload", &[])?
            .l()?;
        let brand = if payload.is_null() {
            None
        } else {
            read_brand(env, &payload)?
        };

        Ok(ClientboundCustomPayloadPacket {
            brand,
            jni_ref: env.new_global_ref(packet)?,
        })
    }
}

 
 
 
 
fn read_brand(env: &mut JNIEnv, payload: &JObject) -> anyhow::Result<Option<String>> {
    let Ok(brand_class_name) = mapping().runtime_class_name(MinecraftClassType::BrandPayload)
    else {
        return Ok(None);
    };
    let Ok(brand_class) = mapping().resolve_class(env, &brand_class_name) else {
        return Ok(None);
    };
    if !env.is_instance_of(payload, &brand_class)? {
        return Ok(None);
    }

    let value = mapping()
        .call_method(MinecraftClassType::BrandPayload, payload, "brand", &[])?
        .l()?;
    let text = JString::from(value);
    let text = env.get_string(&text)?.to_string_lossy().into_owned();
    Ok(Some(text))
}

 
 
 
impl PartialEq for ClientboundCustomPayloadPacket {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

     
     
    #[test]
    fn the_class_type_points_at_the_custom_payload_packet() {
        assert_eq!(
            CLASS_TYPE,
            MinecraftClassType::ClientboundCustomPayloadPacket
        );
        assert_eq!(
            CLASS_TYPE.get_name(),
            "net/minecraft/network/protocol/common/ClientboundCustomPayloadPacket"
        );
        assert_eq!(
            MinecraftClassType::BrandPayload.get_name(),
            "net/minecraft/network/protocol/common/custom/BrandPayload"
        );
    }
}
