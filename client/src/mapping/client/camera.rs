 

use crate::mapping::math::Vec3;
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JObject, JValue};

 
 
 
 
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Camera)]
pub struct Camera {
    jni_ref: GlobalRef,
}

impl Camera {
     
    pub fn new(jni_ref: GlobalRef) -> Camera {
        Camera { jni_ref }
    }

     
     
     
    pub fn position(&self) -> anyhow::Result<Vec3> {
        self.in_frame(|| {
            let vec3 = self.call_method("getPosition", &[])?.l()?;
            Vec3::read(&vec3)
        })
    }

     
     
     
    pub fn focus_position(&self) -> anyhow::Result<Vec3> {
        self.in_frame(|| {
            let entity = self.call_method("getEntity", &[])?.l()?;
            anyhow::ensure!(!entity.is_null(), "Camera has no focused entity");
            let tick = self.call_method("getPartialTickTime", &[])?.f()?;
            let position = mapping()
                .call_method(
                    MinecraftClassType::Entity,
                    &entity,
                    "getPosition",
                    &[JValue::Float(tick)],
                )?
                .l()?;
            let position = Vec3::read(&position)?;
            let old = self.get_field("eyeHeightOld", FieldType::Float)?.f()?;
            let current = self.get_field("eyeHeight", FieldType::Float)?.f()?;
            Ok(Vec3::new(
                position.x(),
                position.y() + (old + (current - old) * tick) as f64,
                position.z(),
            ))
        })
    }

     
     
    pub fn set_position(&self, position: Vec3) -> anyhow::Result<()> {
        self.in_frame(|| {
            let mut env = mapping().get_env()?;
            let position = position.to_java(&mut env)?;
            self.call_method("setPosition", &[JValue::Object(&position)])?;
            Ok(())
        })
    }

     
     
    pub fn set_rotation(&self, yaw: f32, pitch: f32) -> anyhow::Result<()> {
        self.call_method("setRotation", &[JValue::Float(yaw), JValue::Float(pitch)])?;
        Ok(())
    }

     
     
    pub fn setup_independent(
        &self,
        yaw: f32,
        pitch: f32,
        third_person: bool,
        front: bool,
    ) -> anyhow::Result<()> {
        let (yaw, pitch) = if front {
            (yaw + 180.0, -pitch)
        } else {
            (yaw, pitch)
        };
        self.set_rotation(yaw, pitch)?;
        self.set_position(self.focus_position()?)?;
        if third_person {
            let entity = self.call_method("getEntity", &[])?.l()?;
            let scale = mapping()
                .call_method(MinecraftClassType::LivingEntity, &entity, "getScale", &[])?
                .f()?;
            let distance = self
                .call_method("getMaxZoom", &[JValue::Float(4.0 * scale)])?
                .f()?;
            self.call_method(
                "move",
                &[
                    JValue::Float(-distance),
                    JValue::Float(0.0),
                    JValue::Float(0.0),
                ],
            )?;
        }
        Ok(())
    }

     
    pub fn yaw(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getYRot", &[])?.f()?)
    }

     
    pub fn pitch(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getXRot", &[])?.f()?)
    }

     
     
     
    pub fn basis(&self) -> anyhow::Result<(Vec3, Vec3, Vec3)> {
        self.in_frame(|| {
            let look = self.call_method("getLookVector", &[])?.l()?;
            let up = self.call_method("getUpVector", &[])?.l()?;
            let left = self.call_method("getLeftVector", &[])?.l()?;
            let mut env = mapping().get_env()?;
            Ok((
                read_vector(&mut env, &look)?,
                read_vector(&mut env, &up)?,
                read_vector(&mut env, &left)?,
            ))
        })
    }

     
     
     
     
     
    pub fn fov(&self) -> anyhow::Result<f32> {
        Ok(self.get_field("fov", FieldType::Float)?.f()?)
    }

     
     
     
     
     
     
     
    pub fn partial_tick_time(&self) -> anyhow::Result<f32> {
        Ok(self.get_field("partialTickTime", FieldType::Float)?.f()?)
    }
}

fn read_vector(env: &mut jni::JNIEnv<'_>, vector: &JObject<'_>) -> anyhow::Result<Vec3> {
    Ok(Vec3::new(
        env.call_method(vector, "x", "()F", &[])?.f()? as f64,
        env.call_method(vector, "y", "()F", &[])?.f()? as f64,
        env.call_method(vector, "z", "()F", &[])?.f()? as f64,
    ))
}
