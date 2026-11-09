 
 

use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};

 
#[derive(Debug, MappedObject)]
#[mapped(class = PlayerInfo)]
pub struct PlayerInfo {
    jni_ref: GlobalRef,
}

impl PlayerInfo {
     
    pub fn new(jni_ref: GlobalRef) -> PlayerInfo {
        PlayerInfo { jni_ref }
    }

     
    pub fn skin_url(&self) -> anyhow::Result<Option<String>> {
        self.in_frame(|| {
            let skin = self
                .call_method("getSkin", &[])
                .ok()
                .and_then(|value| value.l().ok());
            let Some(skin) = skin else {
                return Ok(None);
            };
            if skin.is_null() {
                return Ok(None);
            }
            let url = mapping()
                .call_method(MinecraftClassType::PlayerSkin, &skin, "textureUrl", &[])?
                .l()?;
            if url.is_null() {
                return Ok(None);
            }
            Ok(Some(mapping().get_string(url)?))
        })
    }

     
     
     
    pub fn latency(&self) -> anyhow::Result<i32> {
        Ok(self.get_field("latency", FieldType::Int)?.i()?)
    }

     
    pub fn get_game_mode(&self) -> anyhow::Result<GlobalRef> {
        self.in_frame(|| {
            let mode = self
                .get_field("gameMode", FieldType::Object(MinecraftClassType::GameType))?
                .l()?;
            mapping().new_global_ref(mode)
        })
    }

     
    pub fn set_game_mode(&self, mode: &GlobalRef) -> anyhow::Result<()> {
        self.set_field(
            "gameMode",
            FieldType::Object(MinecraftClassType::GameType),
            JValue::Object(mode.as_obj()),
        )?;
        Ok(())
    }
}
