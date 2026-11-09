 
 
 
 
 
 
 
 

use crate::mapping::MinecraftClassType;
use jni::objects::GlobalRef;

 
pub const CLASS_TYPE: MinecraftClassType = MinecraftClassType::ClientboundPlayerInfoUpdatePacket;

 
#[derive(Debug, Clone)]
pub struct ClientboundPlayerInfoUpdatePacket {
    pub jni_ref: GlobalRef,
}

 
 
 
impl PartialEq for ClientboundPlayerInfoUpdatePacket {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
