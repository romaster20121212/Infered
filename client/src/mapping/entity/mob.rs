 

use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::MappedObject;
use jni::objects::GlobalRef;

 
 
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Mob)]
pub struct Mob {
    jni_ref: GlobalRef,
}

impl Mob {
     
    pub fn new(jni_ref: GlobalRef) -> Mob {
        Mob { jni_ref }
    }
}

impl EntityRef for Mob {}
impl LivingEntityRef for Mob {}
