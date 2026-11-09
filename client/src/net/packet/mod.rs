 
 
 
 
 
 
 
 
 
 
 
 
 

pub mod captured;
pub mod clientbound_chat;
pub mod clientbound_custom_payload;
pub mod clientbound_explode;
pub mod clientbound_player_info_update;
pub mod clientbound_set_entity_motion;
pub mod serverbound_move_player;
pub mod serverbound_use_item;

use crate::mapping::MinecraftClassType;
use crate::state::mapping;
use captured::CapturedPacket;
use clientbound_chat::{ClientboundPlayerChatPacket, ClientboundSystemChatPacket};
use clientbound_custom_payload::ClientboundCustomPayloadPacket;
use clientbound_explode::ClientboundExplodePacket;
use clientbound_player_info_update::ClientboundPlayerInfoUpdatePacket;
use clientbound_set_entity_motion::ClientboundSetEntityMotionPacket;
use jni::objects::JObject;
use jni::sys::jobject;
use jni::JNIEnv;
use serverbound_move_player::ServerboundMovePlayerPacket;
use serverbound_use_item::ServerboundUseItemPacket;

 
 
#[derive(Debug, Clone, PartialEq)]
pub enum Packet {
     
    ServerboundMovePlayer(ServerboundMovePlayerPacket),
     
     
     
    ServerboundUseItem(ServerboundUseItemPacket),
     
     
    ServerboundSwing(CapturedPacket),
     
     
    ServerboundInteract(CapturedPacket),
     
    ClientboundSetEntityMotion(ClientboundSetEntityMotionPacket),
     
     
    ClientboundPlayerInfoUpdate(ClientboundPlayerInfoUpdatePacket),
     
     
    ClientboundPlayerPosition(CapturedPacket),
     
     
    ClientboundPlayerRotation(CapturedPacket),
     
     
    ClientboundBlockUpdate(CapturedPacket),
     
     
     
    ClientboundExplode(ClientboundExplodePacket),
     
     
    ClientboundCustomPayload(ClientboundCustomPayloadPacket),
     
    ClientboundSystemChat(ClientboundSystemChatPacket),
     
    ClientboundPlayerChat(ClientboundPlayerChatPacket),
}

 
 
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PacketAction {
     
    #[default]
    Forward,
     
     
    Cancel,
}

impl Packet {
     
     
    pub fn from_outbound(env: &mut JNIEnv, packet: &JObject) -> anyhow::Result<Option<Packet>> {
        if is_instance(env, packet, serverbound_move_player::CLASS_TYPE)? {
            return Ok(Some(Packet::ServerboundMovePlayer(
                ServerboundMovePlayerPacket::read(env, packet)?,
            )));
        }
        if is_instance(env, packet, serverbound_use_item::CLASS_TYPE)? {
            return Ok(Some(Packet::ServerboundUseItem(
                ServerboundUseItemPacket::read(env, packet)?,
            )));
        }
        if is_instance(env, packet, MinecraftClassType::ServerboundSwingPacket)? {
            return Ok(Some(Packet::ServerboundSwing(CapturedPacket {
                jni_ref: env.new_global_ref(packet)?,
            })));
        }
        if is_instance(env, packet, MinecraftClassType::ServerboundInteractPacket)? {
            return Ok(Some(Packet::ServerboundInteract(CapturedPacket {
                jni_ref: env.new_global_ref(packet)?,
            })));
        }
        Ok(None)
    }

     
     
    pub fn from_inbound(env: &mut JNIEnv, packet: &JObject) -> anyhow::Result<Option<Packet>> {
        if is_instance(env, packet, clientbound_set_entity_motion::CLASS_TYPE)? {
            return Ok(Some(Packet::ClientboundSetEntityMotion(
                ClientboundSetEntityMotionPacket::read(env, packet)?,
            )));
        }
        if is_instance(env, packet, clientbound_player_info_update::CLASS_TYPE)? {
            return Ok(Some(Packet::ClientboundPlayerInfoUpdate(
                ClientboundPlayerInfoUpdatePacket {
                    jni_ref: env.new_global_ref(packet)?,
                },
            )));
        }
        if is_instance(
            env,
            packet,
            MinecraftClassType::ClientboundPlayerPositionPacket,
        )? {
            return Ok(Some(Packet::ClientboundPlayerPosition(CapturedPacket {
                jni_ref: env.new_global_ref(packet)?,
            })));
        }
        if is_instance(
            env,
            packet,
            MinecraftClassType::ClientboundPlayerRotationPacket,
        )? {
            return Ok(Some(Packet::ClientboundPlayerRotation(CapturedPacket {
                jni_ref: env.new_global_ref(packet)?,
            })));
        }
        if is_instance(
            env,
            packet,
            MinecraftClassType::ClientboundBlockUpdatePacket,
        )? {
            return Ok(Some(Packet::ClientboundBlockUpdate(CapturedPacket {
                jni_ref: env.new_global_ref(packet)?,
            })));
        }
        if is_instance(env, packet, clientbound_explode::CLASS_TYPE)? {
            return Ok(Some(Packet::ClientboundExplode(
                ClientboundExplodePacket::read(env, packet)?,
            )));
        }
        if is_instance(env, packet, clientbound_custom_payload::CLASS_TYPE)? {
            return Ok(Some(Packet::ClientboundCustomPayload(
                ClientboundCustomPayloadPacket::read(env, packet)?,
            )));
        }
        if is_instance(env, packet, clientbound_chat::SYSTEM_CLASS_TYPE)? {
            return Ok(Some(Packet::ClientboundSystemChat(
                ClientboundSystemChatPacket::read(env, packet)?,
            )));
        }
        if is_instance(env, packet, clientbound_chat::PLAYER_CLASS_TYPE)? {
            return Ok(Some(Packet::ClientboundPlayerChat(
                ClientboundPlayerChatPacket::read(env, packet)?,
            )));
        }
        Ok(None)
    }

     
    pub fn to_java(self, env: &mut JNIEnv) -> anyhow::Result<jobject> {
        match self {
            Packet::ServerboundMovePlayer(packet) => packet.to_java(env),
            Packet::ServerboundUseItem(packet) => packet.to_java(env),
            Packet::ClientboundSetEntityMotion(packet) => packet.to_java(env),
             
             
            Packet::ClientboundPlayerInfoUpdate(packet) => {
                Ok(env.new_local_ref(packet.jni_ref.as_obj())?.into_raw())
            }
            Packet::ClientboundExplode(packet) => {
                Ok(env.new_local_ref(packet.jni_ref.as_obj())?.into_raw())
            }
            Packet::ClientboundCustomPayload(packet) => {
                Ok(env.new_local_ref(packet.jni_ref.as_obj())?.into_raw())
            }
            Packet::ClientboundSystemChat(packet) => {
                Ok(env.new_local_ref(packet.jni_ref.as_obj())?.into_raw())
            }
            Packet::ClientboundPlayerChat(packet) => {
                Ok(env.new_local_ref(packet.jni_ref.as_obj())?.into_raw())
            }
            Packet::ServerboundSwing(CapturedPacket { jni_ref })
            | Packet::ServerboundInteract(CapturedPacket { jni_ref })
            | Packet::ClientboundPlayerPosition(CapturedPacket { jni_ref })
            | Packet::ClientboundPlayerRotation(CapturedPacket { jni_ref })
            | Packet::ClientboundBlockUpdate(CapturedPacket { jni_ref }) => {
                Ok(env.new_local_ref(jni_ref.as_obj())?.into_raw())
            }
        }
    }
}

 
fn is_instance(
    env: &mut JNIEnv,
    object: &JObject,
    class: MinecraftClassType,
) -> anyhow::Result<bool> {
    let jclass = mapping().resolve_class(env, &mapping().runtime_class_name(class)?)?;
    Ok(env.is_instance_of(object, &jclass)?)
}
