 
 
 
 
 
 
 

use jni::objects::GlobalRef;

 
#[derive(Debug, Clone)]
pub struct CapturedPacket {
    pub jni_ref: GlobalRef,
}

 
 
 
impl PartialEq for CapturedPacket {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
