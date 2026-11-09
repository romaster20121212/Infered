 

use crate::mapping::MappedObject;
use crate::state::mapping;
use jni::objects::GlobalRef;

 
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Component)]
pub struct Component {
    jni_ref: GlobalRef,
}

impl Component {
     
    pub fn new(jni_ref: GlobalRef) -> Component {
        Component { jni_ref }
    }

     
    pub fn get_string(&self) -> anyhow::Result<String> {
        self.in_frame(|| {
            let string = self.call_method("getString", &[])?.l()?;
            mapping().get_string(string)
        })
    }
}
