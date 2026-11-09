 
 
 
 
 

use crate::mapping::component::Component;
use crate::mapping::MinecraftClassType;
use crate::state::mapping;
use jni::objects::{GlobalRef, JObject, JString};
use jni::JNIEnv;

 
pub const SYSTEM_CLASS_TYPE: MinecraftClassType = MinecraftClassType::ClientboundSystemChatPacket;
 
pub const PLAYER_CLASS_TYPE: MinecraftClassType = MinecraftClassType::ClientboundPlayerChatPacket;

 
#[derive(Debug, Clone)]
pub struct ClientboundSystemChatPacket {
     
    pub text: String,
     
    pub jni_ref: GlobalRef,
}

impl ClientboundSystemChatPacket {
     
    pub fn read(env: &mut JNIEnv, packet: &JObject) -> anyhow::Result<ClientboundSystemChatPacket> {
        let content = mapping()
            .call_method(SYSTEM_CLASS_TYPE, packet, "content", &[])?
            .l()?;
        let text = Component::new(env.new_global_ref(&content)?).get_string()?;
        Ok(ClientboundSystemChatPacket {
            text,
            jni_ref: env.new_global_ref(packet)?,
        })
    }
}

 
#[derive(Debug, Clone)]
pub struct ClientboundPlayerChatPacket {
     
    pub text: String,
     
    pub jni_ref: GlobalRef,
}

impl ClientboundPlayerChatPacket {
     
    pub fn read(env: &mut JNIEnv, packet: &JObject) -> anyhow::Result<ClientboundPlayerChatPacket> {
        let body = mapping()
            .call_method(PLAYER_CLASS_TYPE, packet, "body", &[])?
            .l()?;
        let content = mapping()
            .call_method(MinecraftClassType::SignedMessageBody, &body, "content", &[])?
            .l()?;
        let text = JString::from(content);
        let text = env.get_string(&text)?.to_string_lossy().into_owned();
        Ok(ClientboundPlayerChatPacket {
            text,
            jni_ref: env.new_global_ref(packet)?,
        })
    }
}

 
impl PartialEq for ClientboundSystemChatPacket {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

 
impl PartialEq for ClientboundPlayerChatPacket {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

     
    #[test]
    fn the_class_types_point_at_the_chat_packets() {
        assert_eq!(
            SYSTEM_CLASS_TYPE.get_name(),
            "net/minecraft/network/protocol/game/ClientboundSystemChatPacket"
        );
        assert_eq!(
            PLAYER_CLASS_TYPE.get_name(),
            "net/minecraft/network/protocol/game/ClientboundPlayerChatPacket"
        );
    }
}
