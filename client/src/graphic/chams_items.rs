 
use super::chams::{should_include_self, Config, MAX_PLAYERS};
use super::smoke::java_exception;
use crate::mapping::{MappedObject, MinecraftClassType};
use crate::state::{mapping, minecraft};
use anyhow::{ensure, Context};
use jni::objects::{JClass, JFloatArray, JObject, JValue};

const COLLECT_DESCRIPTOR: &str = "(Ljava/lang/Object;[Ljava/lang/Object;FDDD)[F";

 
 
 
 
pub(crate) const VERTEX_FLOATS: usize = 5;

 
 
pub(crate) const QUAD_FLOATS: usize = VERTEX_FLOATS * 6;

 
 
 
 
 
 
 
 
 
pub(crate) fn atlas(env: &mut jni::JNIEnv) -> anyhow::Result<i32> {
    let bridge = crate::camera_bridge::item_chams_bridge()
        .context("Item Chams geometry bridge unavailable")?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    let result = env.call_static_method(
        class,
        "atlas",
        "(Ljava/lang/Object;)I",
        &[JValue::Object(minecraft().jni_ref().as_obj())],
    );
    match result {
        Ok(value) => Ok(value.i()?.max(0)),
        Err(error) => {
             
             
            let detail = java_exception(env);
            log::debug!(
                "Item Chams: item atlas unavailable, drawing whole quads: {error}; {detail}"
            );
            Ok(0)
        }
    }
}

pub(crate) fn geometry(
    cfg: &Config,
    tick: f32,
    cam: [f64; 3],
    camera_detached: bool,
) -> anyhow::Result<Vec<f32>> {
    if !cfg.players || cfg.strength == 0.0 || cfg.color[3] == 0.0 {
        return Ok(Vec::new());
    }
    let include_self = should_include_self(cfg.self_player, camera_detached);
    let Some(world) = minecraft().world()? else {
        return Ok(Vec::new());
    };
    let Some(player) = minecraft().player()? else {
        return Ok(Vec::new());
    };
    let mut env = mapping().get_env()?;
    let list = env
        .call_method(
            world.jni_ref().as_obj(),
            "method_18456",
            "()Ljava/util/List;",
            &[],
        )?
        .l()?;
    let count = env.call_method(&list, "size", "()I", &[])?.i()?.max(0) as usize;
    let mut nearest: Vec<(f64, jni::objects::GlobalRef)> = Vec::with_capacity(MAX_PLAYERS);
    for i in 0..count.min(256) {
        let candidate = env.with_local_frame(
            16,
            |env| -> anyhow::Result<Option<(f64, jni::objects::GlobalRef)>> {
                let entity = env
                    .call_method(
                        &list,
                        "get",
                        "(I)Ljava/lang/Object;",
                        &[JValue::Int(i as i32)],
                    )?
                    .l()?;
                if !include_self && env.is_same_object(&entity, player.jni_ref().as_obj())? {
                    return Ok(None);
                }
                if mapping()
                    .call_method(MinecraftClassType::Entity, &entity, "isRemoved", &[])?
                    .z()?
                {
                    return Ok(None);
                }
                let pos = mapping()
                    .call_method(
                        MinecraftClassType::Entity,
                        &entity,
                        "getPosition",
                        &[JValue::Float(tick)],
                    )?
                    .l()?;
                let pos = crate::mapping::math::Vec3::read(&pos)?;
                let distance = (pos.x() - cam[0]).powi(2)
                    + (pos.y() - cam[1]).powi(2)
                    + (pos.z() - cam[2]).powi(2);
                if !distance.is_finite() || distance > (cfg.range as f64).powi(2) {
                    return Ok(None);
                }
                Ok(Some((distance, env.new_global_ref(&entity)?)))
            },
        )?;
        if let Some(candidate) = candidate {
            nearest.push(candidate);
            nearest.sort_by(|a, b| a.0.total_cmp(&b.0));
            nearest.truncate(MAX_PLAYERS);
        }
    }
    if nearest.is_empty() {
        return Ok(Vec::new());
    }
    let object = env.find_class("java/lang/Object")?;
    let array = env.new_object_array(nearest.len() as i32, object, JObject::null())?;
    for (i, (_, entity)) in nearest.iter().enumerate() {
        env.set_object_array_element(&array, i as i32, entity.as_obj())?;
    }
    let bridge = crate::camera_bridge::item_chams_bridge()
        .context("Item Chams geometry bridge unavailable")?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    let result = env.call_static_method(
        class,
        "collect",
        COLLECT_DESCRIPTOR,
        &[
            JValue::Object(minecraft().jni_ref().as_obj()),
            JValue::Object(&array),
            JValue::Float(tick),
            JValue::Double(cam[0]),
            JValue::Double(cam[1]),
            JValue::Double(cam[2]),
        ],
    );
    let array = match result {
        Ok(value) => JFloatArray::from(value.l()?),
        Err(error) => {
            let detail = java_exception(&mut env);
            anyhow::bail!("Item Chams collection failed: {error}; {detail}");
        }
    };
    let len = env.get_array_length(&array)? as usize;
    ensure!(
        len <= MAX_PLAYERS * 8192 * VERTEX_FLOATS && len % QUAD_FLOATS == 0,
        "invalid Item Chams geometry length {len}"
    );
    let mut vertices = vec![0.0; len];
    env.get_float_array_region(&array, 0, &mut vertices)?;
    ensure!(
        vertices.iter().all(|v| v.is_finite()),
        "non-finite Item Chams geometry"
    );
    Ok(vertices)
}
