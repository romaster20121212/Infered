 
 
 
 
 
 
 
 

use crate::mapping::{MappedObject, MinecraftClassType};
use crate::state::{mapping, minecraft};
use jni::objects::{GlobalRef, JValue};

 
#[derive(Debug, MappedObject)]
#[mapped(class = PlayerList)]
pub struct PlayerList {
    jni_ref: GlobalRef,
}

impl PlayerList {
     
    pub fn new(jni_ref: GlobalRef) -> PlayerList {
        PlayerList { jni_ref }
    }

     
    pub fn view_distance(&self) -> anyhow::Result<i32> {
        self.in_frame(|| Ok(self.call_method("getViewDistance", &[])?.i()?))
    }

     
    pub fn set_view_distance(&self, chunks: i32) -> anyhow::Result<()> {
        self.in_frame(|| {
            self.call_method("setViewDistance", &[JValue::Int(chunks)])?;
            Ok(())
        })
    }

     
     
     
    pub fn simulation_distance(&self) -> anyhow::Result<i32> {
        self.in_frame(|| Ok(self.call_method("getSimulationDistance", &[])?.i()?))
    }

     
    pub fn set_simulation_distance(&self, chunks: i32) -> anyhow::Result<()> {
        self.in_frame(|| {
            self.call_method("setSimulationDistance", &[JValue::Int(chunks)])?;
            Ok(())
        })
    }
}

 
 
 
pub fn singleplayer_player_list() -> anyhow::Result<Option<PlayerList>> {
    mapping().in_frame(|| {
        let server = mapping()
            .call_method(
                MinecraftClassType::Minecraft,
                minecraft().jni_ref().as_obj(),
                "getSingleplayerServer",
                &[],
            )?
            .l()?;
        if server.is_null() {
            return Ok(None);
        }
         
         
        let player_list = mapping()
            .call_method(
                MinecraftClassType::MinecraftServer,
                &server,
                "getPlayerList",
                &[],
            )?
            .l()?;
        if player_list.is_null() {
            return Ok(None);
        }
        Ok(Some(PlayerList::new(
            mapping().new_global_ref(player_list)?,
        )))
    })
}
