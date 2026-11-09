 
use super::smoke::finite;
use crate::mapping::{MappedObject, MinecraftClassType};
use crate::module::{ModuleData, ModuleSetting};
use crate::state::{mapping, minecraft};
use anyhow::{ensure, Context};
use jni::objects::{JClass, JFloatArray, JObject, JValue};

pub(super) const MAX_PLAYERS: usize = 40;

 
 
 
 
pub(crate) const COLLECT_DESCRIPTOR: &str = "(Ljava/lang/Object;[Ljava/lang/Object;FDDDZ)[F";

 
 
static ARMOR_REPORTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

 
 
pub(crate) const SKELETON_JOINTS: usize = 12;
pub(crate) const JOINT_STRIDE: usize = 3;
#[derive(Clone)]
pub(crate) struct Config {
    pub players: bool,
    pub self_player: bool,
     
     
    pub armor: bool,
    pub color: [f32; 4],
    pub strength: f32,
    pub pulse: bool,
    pub through: bool,
    pub range: f32,
}
impl Config {
    pub fn read(data: &ModuleData) -> Self {
        let toggle = |name, fallback| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_toggle_value)
                .unwrap_or(fallback)
        };
        Self {
            players: toggle("Players", true),
            self_player: toggle("Self", false),
            armor: toggle("Armor", false),
            color: match data.get_setting("Color") {
                Some(ModuleSetting::Color { value, .. }) => {
                    std::array::from_fn(|i| finite(value[i], [1.0, 0.0, 0.0, 1.0][i], 0.0, 1.0))
                }
                _ => [1.0, 0.0, 0.0, 1.0],
            },
            strength: finite(
                data.get_setting("GlowStrength")
                    .and_then(ModuleSetting::get_slider_value)
                    .unwrap_or(1.0),
                1.0,
                0.0,
                2.0,
            ),
            pulse: toggle("Pulse", true),
            through: toggle("ThroughWalls", true) && !toggle("OnlyVisible", false),
            range: finite(
                data.get_setting("Range")
                    .and_then(ModuleSetting::get_slider_value)
                    .unwrap_or(64.0),
                64.0,
                16.0,
                128.0,
            ),
        }
    }
}
 
 
pub(crate) fn isolated_failure(
    env: &mut jni::JNIEnv,
    error: anyhow::Error,
) -> anyhow::Result<anyhow::Error> {
    if env.exception_check()? {
        let detail = super::smoke::java_exception(env);
        Ok(error.context(format!("Chams Java failure: {detail}")))
    } else {
        Ok(error)
    }
}

 
 
 
pub(super) fn should_include_self(self_player: bool, camera_detached: bool) -> bool {
    self_player && camera_detached
}

pub(super) fn geometry(
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
    let bridge =
        crate::camera_bridge::glow_bridge().context("Chams geometry bridge unavailable")?;
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
            JValue::Bool(cfg.armor as u8),
        ],
    );
    let array = match result {
        Ok(value) => JFloatArray::from(value.l()?),
        Err(error) => {
            let detail = super::smoke::java_exception(&mut env);
            anyhow::bail!("Chams model collection failed: {error}; {detail}");
        }
    };
     
     
    if cfg.armor && !ARMOR_REPORTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        let check = JClass::from(env.new_local_ref(bridge.as_obj())?);
        let unavailable = env
            .call_static_method(check, "armorUnavailable", "()Z", &[])
            .ok()
            .and_then(|value| value.z().ok())
            .unwrap_or(false);
        if unavailable {
            log::warn!("[Chams] armor layer unavailable on this build; body chams is unaffected");
        }
    }
    let len = env.get_array_length(&array)? as usize;
    ensure!(
        len <= MAX_PLAYERS * 4096 * 3 && len % 9 == 0,
        "invalid Chams geometry length {len}"
    );
    let mut vertices = vec![0.0; len];
    env.get_float_array_region(&array, 0, &mut vertices)?;
    ensure!(
        vertices.iter().all(|v| v.is_finite()),
        "non-finite Chams geometry"
    );
    Ok(vertices)
}
 
 
 
 
 
 
 
pub(crate) fn skeleton_positions(
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
    let bridge =
        crate::camera_bridge::skeleton_bridge().context("Skeleton geometry bridge unavailable")?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    let result = env.call_static_method(
        class,
        "collect",
        "(Ljava/lang/Object;[Ljava/lang/Object;FDDD)[F",
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
            let detail = super::smoke::java_exception(&mut env);
            anyhow::bail!("Skeleton joint collection failed: {error}; {detail}");
        }
    };
    let len = env.get_array_length(&array)? as usize;
    ensure!(
        len <= MAX_PLAYERS * SKELETON_JOINTS * JOINT_STRIDE
            && len % (SKELETON_JOINTS * JOINT_STRIDE) == 0,
        "invalid Skeleton joint count {len}"
    );
    let mut points = vec![0.0; len];
    env.get_float_array_region(&array, 0, &mut points)?;
    ensure!(
        points.iter().all(|v| v.is_finite()),
        "non-finite Skeleton joints"
    );
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_visible_always_overrides_through_wall_setting() {
        let mut module = crate::module::render::chams::ChamsModule::new();
        assert!(Config::read(&module.module).through);
        module
            .module
            .get_setting_mut("OnlyVisible")
            .unwrap()
            .set_toggle_value(true);
        assert!(!Config::read(&module.module).through);
        module
            .module
            .get_setting_mut("OnlyVisible")
            .unwrap()
            .set_toggle_value(false);
        module
            .module
            .get_setting_mut("ThroughWalls")
            .unwrap()
            .set_toggle_value(false);
        assert!(!Config::read(&module.module).through);
    }

    #[test]
    fn self_chams_requires_third_person_detached_camera() {
        assert!(!should_include_self(false, false));
        assert!(!should_include_self(false, true));
        assert!(!should_include_self(true, false));
        assert!(should_include_self(true, true));
    }
}
