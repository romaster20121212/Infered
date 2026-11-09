 

use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::MappedObject;
use jni::objects::GlobalRef;

 
 
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = LivingEntity)]
pub struct LivingEntity {
    jni_ref: GlobalRef,
}

impl LivingEntity {
     
    pub fn new(jni_ref: GlobalRef) -> LivingEntity {
        LivingEntity { jni_ref }
    }
}

impl EntityRef for LivingEntity {}
impl LivingEntityRef for LivingEntity {}
