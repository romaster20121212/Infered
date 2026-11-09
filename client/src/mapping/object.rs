 
 
 
 
 
 
 
 
 

use crate::mapping::{FieldType, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue, JValueOwned};

 
pub trait MappedObject {
     
    fn jni_ref(&self) -> &GlobalRef;

     
    fn class_type() -> MinecraftClassType;

     
     
    fn call_method(&self, name: &str, args: &[JValue]) -> anyhow::Result<JValueOwned<'static>>
    where
        Self: Sized,
    {
        mapping().call_method(Self::class_type(), self.jni_ref().as_obj(), name, args)
    }

     
    fn get_field(&self, name: &str, field_type: FieldType) -> anyhow::Result<JValueOwned<'static>>
    where
        Self: Sized,
    {
        mapping().get_field(
            Self::class_type(),
            self.jni_ref().as_obj(),
            name,
            field_type,
        )
    }

     
    fn set_field(&self, name: &str, field_type: FieldType, value: JValue) -> anyhow::Result<()>
    where
        Self: Sized,
    {
        mapping().set_field(
            Self::class_type(),
            self.jni_ref().as_obj(),
            name,
            field_type,
            value,
        )
    }

     
     
    fn in_frame<R>(&self, f: impl FnOnce() -> anyhow::Result<R>) -> anyhow::Result<R> {
        mapping().in_frame(f)
    }

     
     
    fn instance_of<T: MappedObject>(&self) -> bool {
        mapping()
            .in_frame(|| mapping().is_instance_of(T::class_type(), self.jni_ref().as_obj()))
            .unwrap_or(false)
    }

     
    fn is_same<T: MappedObject>(&self, other: &T) -> bool {
        let Ok(env) = mapping().get_env() else {
            return false;
        };
        env.is_same_object(self.jni_ref().as_obj(), other.jni_ref().as_obj())
            .unwrap_or(false)
    }

     
     
    fn equals<T: MappedObject>(&self, other: &T) -> bool
    where
        Self: Sized,
    {
        self.in_frame(|| {
            Ok(self
                .call_method("equals", &[JValue::Object(other.jni_ref().as_obj())])?
                .z()?)
        })
        .unwrap_or(false)
    }
}
