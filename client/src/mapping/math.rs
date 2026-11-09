 
 
 
 
 
 
 

use crate::mapping::{FieldType, MinecraftClassType as Cls};
use crate::state::mapping;
use jni::objects::{JObject, JValue};
use jni::JNIEnv;

 
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 { x, y, z }
    }

    pub fn read(obj: &JObject) -> anyhow::Result<Vec3> {
        let mapping = mapping();
        Ok(Vec3 {
            x: mapping
                .get_field(Cls::Vec3, obj, "x", FieldType::Double)?
                .d()?,
            y: mapping
                .get_field(Cls::Vec3, obj, "y", FieldType::Double)?
                .d()?,
            z: mapping
                .get_field(Cls::Vec3, obj, "z", FieldType::Double)?
                .d()?,
        })
    }

    pub const fn x(&self) -> f64 {
        self.x
    }

    pub const fn y(&self) -> f64 {
        self.y
    }

    pub const fn z(&self) -> f64 {
        self.z
    }

    pub fn to_java<'e>(self, env: &mut JNIEnv<'e>) -> anyhow::Result<JObject<'e>> {
        let class = mapping().resolve_class(env, &mapping().runtime_class_name(Cls::Vec3)?)?;
        Ok(env.new_object(
            &class,
            "(DDD)V",
            &[
                JValue::Double(self.x),
                JValue::Double(self.y),
                JValue::Double(self.z),
            ],
        )?)
    }
}

 
 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockPos {
    x: i32,
    y: i32,
    z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    pub fn read(obj: &JObject) -> anyhow::Result<BlockPos> {
        let mapping = mapping();
        Ok(BlockPos {
            x: mapping.call_method(Cls::BlockPos, obj, "getX", &[])?.i()?,
            y: mapping.call_method(Cls::BlockPos, obj, "getY", &[])?.i()?,
            z: mapping.call_method(Cls::BlockPos, obj, "getZ", &[])?.i()?,
        })
    }

    pub const fn x(&self) -> i32 {
        self.x
    }

    pub const fn y(&self) -> i32 {
        self.y
    }

    pub const fn z(&self) -> i32 {
        self.z
    }

    pub const fn offset(&self, dx: i32, dy: i32, dz: i32) -> BlockPos {
        BlockPos::new(self.x + dx, self.y + dy, self.z + dz)
    }

    pub fn to_java<'e>(self, env: &mut JNIEnv<'e>) -> anyhow::Result<JObject<'e>> {
        let class = mapping().resolve_class(env, &mapping().runtime_class_name(Cls::BlockPos)?)?;
        Ok(env.new_object(
            &class,
            "(III)V",
            &[
                JValue::Int(self.x),
                JValue::Int(self.y),
                JValue::Int(self.z),
            ],
        )?)
    }
}
