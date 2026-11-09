use crate::mapping::MappedObject;
use crate::module::ModuleId;
use crate::state::{client, mapping, minecraft};
use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::{JNIEnv, NativeMethod};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

const BRIDGE_CLASS: &str = "InferedCameraBridge";
const LEVEL_BRIDGE_CLASS: &str = "InferedLevelRendererBridge";
const XRAY_BRIDGE_CLASS: &str = "InferedXrayBridge";
const EXPLOSION_BRIDGE_CLASS: &str = "InferedExplosionBridge";
const ITEM_SCALE_BRIDGE_CLASS: &str = "InferedItemScaleBridge";
const SHIELD_BRIDGE_CLASS: &str = "InferedShieldBridge";
const FIRE_BRIDGE_CLASS: &str = "InferedFireBridge";
const CROSSHAIR_BRIDGE_CLASS: &str = "InferedCrosshairBridge";
const TITLE_LOGO_BRIDGE_CLASS: &str = "InferedTitleLogoBridge";
const NO_PARTICLES_BRIDGE_CLASS: &str = "InferedNoParticlesBridge";
const TOTEM_POP_BRIDGE_CLASS: &str = "InferedTotemPopBridge";
const SKY_BRIDGE_CLASS: &str = "InferedSkyBridge";
const SWING_ECHO_BRIDGE_CLASS: &str = "InferedSwingEchoBridge";
const HAND_OUTLINE_BRIDGE_CLASS: &str = "InferedHandOutlineBridge";
const HAND_AURA_BRIDGE_CLASS: &str = "InferedHandAuraBridge";
const CRYSTAL_BRIDGE_CLASS: &str = "InferedCrystalBridge";
const CRYSTAL_OPTIMIZER_BRIDGE_CLASS: &str = "InferedCrystalOptimizerBridge";
static CAMERA_CALLBACK_LOGGED: AtomicBool = AtomicBool::new(false);
static CAMERA_OVERRIDE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static CULL_OVERRIDE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static GRAPH_INVALIDATE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static FREECAM_DEBUG_LAST_MS: AtomicU64 = AtomicU64::new(0);
static FREECAM_DEBUG_EPOCH: OnceLock<Instant> = OnceLock::new();
const BRIDGE_BYTECODE: &[u8] = include_bytes!(env!("INFERED_CAMERA_BRIDGE_CLASS"));
const LEVEL_BRIDGE_BYTECODE: &[u8] = include_bytes!(env!("INFERED_LEVEL_RENDERER_BRIDGE_CLASS"));
const XRAY_BRIDGE_BYTECODE: &[u8] = include_bytes!(env!("INFERED_XRAY_BRIDGE_CLASS"));
const CRYSTAL_BRIDGE_BYTECODE: &[u8] = include_bytes!(env!("INFERED_CRYSTAL_BRIDGE_CLASS"));
const CRYSTAL_OPTIMIZER_BRIDGE_BYTECODE: &[u8] =
    include_bytes!(env!("INFERED_CRYSTAL_OPTIMIZER_BRIDGE_CLASS"));

static BRIDGE: OnceLock<Mutex<Option<GlobalRef>>> = OnceLock::new();
static LEVEL_BRIDGE: OnceLock<Mutex<Option<GlobalRef>>> = OnceLock::new();
static XRAY_BRIDGE: OnceLock<Mutex<Option<GlobalRef>>> = OnceLock::new();
static FREELOOK_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static SMOKE_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static GLOW_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static CLASSIC_CHAMS_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static ITEM_CHAMS_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static SKELETON_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static COSMETICS_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static SCAFFOLD_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static FAST_SCAFFOLD_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static EXPLOSION_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static ITEM_SCALE_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static SHIELD_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static FIRE_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static CROSSHAIR_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static TITLE_LOGO_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static NO_PARTICLES_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static TOTEM_POP_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static SKY_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static SWING_ECHO_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static HAND_OUTLINE_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static HAND_AURA_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static CRYSTAL_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
static CRYSTAL_OPTIMIZER_BRIDGE: Mutex<Option<GlobalRef>> = Mutex::new(None);
 
 
 
static CRYSTAL_INPUT_TICKS: AtomicU64 = AtomicU64::new(0);

 
pub(crate) fn scaffold_bridge() -> Option<GlobalRef> {
    SCAFFOLD_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

 
pub(crate) fn fast_scaffold_bridge() -> Option<GlobalRef> {
    FAST_SCAFFOLD_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

 
 
 
pub(crate) fn crystal_input_phase_ran() -> bool {
    CRYSTAL_INPUT_TICKS.load(Ordering::Relaxed) > 0
}

pub(crate) fn crystal_bridge() -> Option<GlobalRef> {
    CRYSTAL_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

 
 
 
 
 
extern "system" fn crystal_input_phase(_env: *mut jni::sys::JNIEnv, _class: jni::sys::jclass) {
    if CRYSTAL_INPUT_TICKS.fetch_add(1, Ordering::Relaxed) == 0 {
        log::info!(
            "Crystal input phase live: the macro advances ahead of the tick's movement packet"
        );
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if let Some(client) = crate::state::try_client() {
            client.modules.input_phase();
        }
    }));
     
     
    if let Ok(env) = mapping().get_env() {
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
    }
}

 
 
 
fn set_crystal_input_enabled(enabled: bool) -> anyhow::Result<()> {
    let slot = CRYSTAL_BRIDGE.lock().unwrap_or_else(|p| p.into_inner());
    let Some(class) = slot.as_ref() else {
        anyhow::ensure!(!enabled, "Crystal bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let local = env.new_local_ref(class.as_obj())?;
    let class = JClass::from(local);
    let field = env.get_static_field_id(&class, "ENABLED", "Z")?;
    env.set_static_field(&class, field, JValue::Bool(enabled.into()))?;
    Ok(())
}

 
 
 
 
pub(crate) fn crystal_optimizer_bridge_defined() -> bool {
    CRYSTAL_OPTIMIZER_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .is_some()
}

 
 
 
 
pub(crate) fn set_crystal_optimizer_enabled(enabled: bool) -> anyhow::Result<()> {
    let slot = CRYSTAL_OPTIMIZER_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let Some(class) = slot.as_ref() else {
        anyhow::ensure!(!enabled, "Crystal optimizer bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let local = env.new_local_ref(class.as_obj())?;
    let class = JClass::from(local);
    let field = env.get_static_field_id(&class, "ENABLED", "Z")?;
    env.set_static_field(&class, field, JValue::Bool(enabled.into()))?;
    Ok(())
}

 
 
 
 
 
 
 
 
unsafe extern "system" fn scaffold_arm_body(
    _env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    yaw: jni::sys::jfloat,
    pitch: jni::sys::jfloat,
) -> jni::sys::jboolean {
    let mut armed = match catch_unwind(AssertUnwindSafe(|| {
        crate::module::world::scaffold::arm_body(yaw, pitch)
    })) {
        Ok(Ok(())) => true,
        Ok(Err(error)) => {
            log::warn!("Scaffold body aim skipped: {error}");
            false
        }
        Err(_) => {
            log::error!("Scaffold body aim panicked; tick left on the camera");
            false
        }
    };
     
    if let Ok(env) = mapping().get_env() {
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
            armed = false;
        }
    } else {
        armed = false;
    }
    armed.into()
}

unsafe extern "system" fn fast_scaffold_arm_body(
    _env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    yaw: jni::sys::jfloat,
    pitch: jni::sys::jfloat,
) -> jni::sys::jboolean {
    let mut armed = match catch_unwind(AssertUnwindSafe(|| {
        crate::module::world::fast_scaffold::arm_body(yaw, pitch)
    })) {
        Ok(Ok(())) => true,
        Ok(Err(error)) => {
            log::warn!("Fast Scaffold body aim skipped: {error}");
            false
        }
        Err(_) => {
            log::error!("Fast Scaffold body aim panicked; tick left on the camera");
            false
        }
    };
    if let Ok(env) = mapping().get_env() {
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
            armed = false;
        }
    } else {
        armed = false;
    }
    armed.into()
}

pub(crate) fn set_fast_scaffold_active(active: bool) -> anyhow::Result<()> {
    let Some(bridge) = fast_scaffold_bridge() else {
        anyhow::ensure!(!active, "Fast Scaffold bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    if active {
        env.set_static_field(&class, (&class, "ACTIVE", "Z"), JValue::Bool(1))?;
    } else {
        env.call_static_method(&class, "stop", "()V", &[])?;
    }
    Ok(())
}

pub(crate) fn set_scaffold_active(active: bool) -> anyhow::Result<()> {
    let bridge = SCAFFOLD_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!active, "Scaffold bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    if active {
        env.set_static_field(&class, (&class, "ACTIVE", "Z"), JValue::Bool(1))?;
    } else {
        env.call_static_method(&class, "stop", "()V", &[])?;
    }
    Ok(())
}

pub(crate) fn set_cosmetic_flag(flag: &str, active: bool) -> anyhow::Result<()> {
    let bridge = COSMETICS_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!active, "Cosmetics bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    env.set_static_field(&class, (&class, flag, "Z"), JValue::Bool(active.into()))?;
    Ok(())
}

 
 
 
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct WingLook {
     
    pub size: f32,
     
    pub flap_speed: f32,
     
    pub flap_amount: f32,
     
    pub glow: f32,
     
    pub opacity: f32,
     
    pub membrane: i32,
     
    pub glow_color: i32,
}

 
 
pub(crate) fn set_wing_look(look: WingLook) -> anyhow::Result<()> {
    let bridge = COSMETICS_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    let fields = [
        ("WING_SIZE", "F", JValue::Float(look.size)),
        ("WING_FLAP_SPEED", "F", JValue::Float(look.flap_speed)),
        ("WING_FLAP_AMOUNT", "F", JValue::Float(look.flap_amount)),
        ("WING_GLOW", "F", JValue::Float(look.glow)),
        ("WING_OPACITY", "F", JValue::Float(look.opacity)),
        ("WING_MEMBRANE", "I", JValue::Int(look.membrane)),
        ("WING_GLOW_COLOR", "I", JValue::Int(look.glow_color)),
    ];
    for (name, signature, value) in fields {
         
         
        env.set_static_field(&class, (&class, name, signature), value)
            .map_err(|error| describe_jni_error(&mut env, "wing look", error))?;
    }
    Ok(())
}

 
 
pub(crate) fn set_cape_artwork(png: &[u8]) -> anyhow::Result<()> {
    let bridge = COSMETICS_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::bail!("Cosmetics bridge unavailable");
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    let artwork = env.byte_array_from_slice(png)?;
    env.call_static_method(&class, "setArtwork", "([B)V", &[JValue::Object(&artwork)])
        .map_err(|error| describe_jni_error(&mut env, "cape artwork", error))?;
    Ok(())
}

 
 
 
 
pub(crate) fn set_cape_style(minecraft: bool) -> anyhow::Result<()> {
    let bridge = COSMETICS_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    env.call_static_method(
        &class,
        "setStyle",
        "(Z)V",
        &[JValue::Bool(minecraft.into())],
    )
    .map_err(|error| describe_jni_error(&mut env, "cape style", error))?;
    Ok(())
}

 
 
 
 
 
 
 
 
 
pub(crate) fn set_fire_hidden(hidden: bool) -> anyhow::Result<()> {
    let bridge = FIRE_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!hidden, "Fire bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    env.set_static_field(
        &class,
        (&class, "SUPPRESS_FIRE", "Z"),
        JValue::Bool(hidden.into()),
    )?;
    Ok(())
}

 
 
pub(crate) fn set_crosshair_hidden(hidden: bool) -> anyhow::Result<()> {
    let bridge = CROSSHAIR_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!hidden, "Crosshair bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    env.set_static_field(
        &class,
        (&class, "HIDE_VANILLA", "Z"),
        JValue::Bool(hidden.into()),
    )?;
    Ok(())
}

 
 
 
pub(crate) fn set_title_logo_hidden(hidden: bool) -> anyhow::Result<()> {
    let bridge = TITLE_LOGO_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!hidden, "Title logo bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    env.set_static_field(
        &class,
        (&class, "HIDE_VANILLA", "Z"),
        JValue::Bool(hidden.into()),
    )?;
    Ok(())
}

 
 
 
pub(crate) fn set_no_particles_hidden(hidden: bool) -> anyhow::Result<()> {
    let bridge = NO_PARTICLES_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!hidden, "No-particles bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    env.set_static_field(
        &class,
        (&class, "HIDE_PARTICLES", "Z"),
        JValue::Bool(hidden.into()),
    )?;
    Ok(())
}

 
pub(crate) fn set_totem_pop_scale(scale: f32) -> anyhow::Result<()> {
    let bridge = TOTEM_POP_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(scale == 1.0, "Totem pop bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    env.set_static_field(&class, (&class, "SCALE", "F"), JValue::Float(scale))?;
    Ok(())
}

 
pub(crate) fn set_swing_echo(enabled: bool) -> anyhow::Result<()> {
    let bridge = SWING_ECHO_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!enabled, "Swing echo bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
     
     
    env.set_static_field(
        &class,
        (&class, "ENABLED", "Z"),
        JValue::Bool(enabled.into()),
    )
    .map_err(|error| describe_jni_error(&mut env, "swing echo toggle", error))?;
    Ok(())
}

 
 
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SwingEchoLook {
     
    pub frozen: bool,
     
    pub delay_seconds: f32,
     
    pub copies: i32,
     
    pub spacing: f32,
     
    pub hold_seconds: f32,
     
    pub fade_seconds: f32,
     
    pub start_alpha: f32,
     
    pub smoothness: f32,
}

 
 
pub(crate) fn set_swing_echo_look(look: SwingEchoLook) -> anyhow::Result<()> {
    let bridge = SWING_ECHO_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    let fields = [
        ("FROZEN", "Z", JValue::Bool(look.frozen.into())),
        ("DELAY_SECONDS", "F", JValue::Float(look.delay_seconds)),
        ("COPIES", "I", JValue::Int(look.copies)),
        ("SPACING", "F", JValue::Float(look.spacing)),
        ("HOLD_SECONDS", "F", JValue::Float(look.hold_seconds)),
        ("FADE_SECONDS", "F", JValue::Float(look.fade_seconds)),
        ("START_ALPHA", "F", JValue::Float(look.start_alpha)),
        ("SMOOTHNESS", "F", JValue::Float(look.smoothness)),
    ];
    for (name, signature, value) in fields {
         
        env.set_static_field(&class, (&class, name, signature), value)
            .map_err(|error| describe_jni_error(&mut env, "swing echo look", error))?;
    }
    Ok(())
}

 
pub(crate) fn set_hand_outline(enabled: bool) -> anyhow::Result<()> {
    let bridge = HAND_OUTLINE_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!enabled, "Hand outline bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
     
    env.set_static_field(
        &class,
        (&class, "ENABLED", "Z"),
        JValue::Bool(enabled.into()),
    )
    .map_err(|error| describe_jni_error(&mut env, "hand outline toggle", error))?;
    Ok(())
}

 
 
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HandOutlineLook {
     
    pub color: [f32; 4],
     
     
    pub thickness: f32,
     
    pub samples: i32,
}

 
 
pub(crate) fn set_hand_outline_look(look: HandOutlineLook) -> anyhow::Result<()> {
    let bridge = HAND_OUTLINE_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    let fields = [
        ("RED", "F", JValue::Float(look.color[0])),
        ("GREEN", "F", JValue::Float(look.color[1])),
        ("BLUE", "F", JValue::Float(look.color[2])),
        ("ALPHA", "F", JValue::Float(look.color[3])),
        ("THICKNESS", "F", JValue::Float(look.thickness)),
        ("SAMPLES", "I", JValue::Int(look.samples)),
    ];
    for (name, signature, value) in fields {
         
        env.set_static_field(&class, (&class, name, signature), value)
            .map_err(|error| describe_jni_error(&mut env, "hand outline look", error))?;
    }
    Ok(())
}

 
 
pub(crate) fn set_hand_aura(enabled: bool) -> anyhow::Result<()> {
    let bridge = HAND_AURA_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!enabled, "Hand aura bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
     
    env.set_static_field(
        &class,
        (&class, "ENABLED", "Z"),
        JValue::Bool(enabled.into()),
    )
    .map_err(|error| describe_jni_error(&mut env, "hand aura toggle", error))?;
    Ok(())
}

 
 
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HandAuraLook {
     
    pub color: [f32; 4],
     
     
    pub width: f32,
     
    pub main_hand: bool,
     
    pub off_hand: bool,
     
     
    pub echo_bleed: bool,
}

 
 
 
 
 
 
pub(crate) fn set_hand_aura_look(look: HandAuraLook) -> anyhow::Result<()> {
    let bridge = HAND_AURA_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    let fields = [
        ("RED", "F", JValue::Float(look.color[0])),
        ("GREEN", "F", JValue::Float(look.color[1])),
        ("BLUE", "F", JValue::Float(look.color[2])),
        ("ALPHA", "F", JValue::Float(look.color[3])),
        ("WIDTH", "F", JValue::Float(look.width)),
        ("MAIN_HAND", "Z", JValue::Bool(look.main_hand.into())),
        ("OFF_HAND", "Z", JValue::Bool(look.off_hand.into())),
        ("ECHO_BLEED", "Z", JValue::Bool(look.echo_bleed.into())),
    ];
    for (name, signature, value) in fields {
         
        env.set_static_field(&class, (&class, name, signature), value)
            .map_err(|error| describe_jni_error(&mut env, "hand aura look", error))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
     
     
     
     
     
     
    #[test]
    fn every_hand_aura_field_exists_on_the_bridge() {
        let bridge = include_str!("../java/InferedHandAuraBridge.java");
         
         
        let declared: Vec<&str> = bridge
            .lines()
            .filter_map(|line| line.split("public static volatile").nth(1))
            .filter_map(|declaration| declaration.split('=').next())
            .filter_map(|declaration| declaration.trim_end_matches(';').trim().rsplit(' ').next())
            .collect();

        for (writer, expected) in [("fn set_hand_aura_look", 8)] {
            let source = include_str!("camera_bridge.rs");
            let start = source
                .find(writer)
                .unwrap_or_else(|| panic!("the {writer} writer is in this file"));
            let end = start
                + source[start..]
                    .find("\n}\n")
                    .unwrap_or_else(|| panic!("{writer} ends"));
            let section = &source[start..end];

             
             
            let mut checked = 0;
            let mut rest = section;
            while let Some(open) = rest.find('"') {
                rest = &rest[open + 1..];
                let Some(close) = rest.find('"') else { break };
                let name = &rest[..close];
                rest = &rest[close + 1..];
                if name.len() > 1 && name.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
                    assert!(
                        declared.contains(&name),
                        "InferedHandAuraBridge declares no {name} field to push (has {declared:?})"
                    );
                    checked += 1;
                }
            }
            assert!(
                checked >= expected,
                "expected the whole look behind {writer}, found {checked}"
            );
        }
    }

     
     
    #[test]
    fn sky_hiders_hold_the_sky_until_the_last_one_releases() {
        use super::{next_sky_mask, SkyHider};

        let hidden = next_sky_mask(0, SkyHider::Atmosphere, true);
        assert_ne!(hidden, 0);
        assert_eq!(
            next_sky_mask(hidden, SkyHider::Atmosphere, true),
            hidden,
            "claiming twice is the same claim"
        );
        assert_eq!(next_sky_mask(hidden, SkyHider::Atmosphere, false), 0);

         
        assert_eq!(next_sky_mask(0, SkyHider::Atmosphere, false), 0);
    }

     
     
     
    #[test]
    fn the_world_tail_stays_on_while_any_pass_wants_it() {
        use super::{next_tail_mask, TAIL_ATMOSPHERE, TAIL_SMOKE};

        let smoke = next_tail_mask(0, TAIL_SMOKE, true);
        assert_ne!(smoke, 0);

         
        let both = next_tail_mask(smoke, TAIL_ATMOSPHERE, true);
        assert_ne!(both, 0);
        let sky_only = next_tail_mask(both, TAIL_SMOKE, false);
        assert_eq!(sky_only, TAIL_ATMOSPHERE);

         
        assert_eq!(next_tail_mask(sky_only, TAIL_ATMOSPHERE, true), sky_only);
        assert_eq!(next_tail_mask(sky_only, TAIL_ATMOSPHERE, false), 0);
        assert_eq!(next_tail_mask(0, TAIL_SMOKE, false), 0);
    }
}

 
 
pub(crate) fn set_explosion_suppression(suppressed: bool) -> anyhow::Result<()> {
    let bridge = EXPLOSION_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!suppressed, "Explosion bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    env.set_static_field(
        &class,
        (&class, "SUPPRESS_PARTICLES", "Z"),
        JValue::Bool(suppressed.into()),
    )?;
    env.set_static_field(
        &class,
        (&class, "SUPPRESS_SOUNDS", "Z"),
        JValue::Bool(suppressed.into()),
    )?;
    Ok(())
}

 
 
const MAIN_VIEW_FIELDS: [&str; 7] = [
    "MAIN_SCALE",
    "MAIN_X",
    "MAIN_Y",
    "MAIN_Z",
    "MAIN_PITCH",
    "MAIN_YAW",
    "MAIN_ROLL",
];
const OFF_VIEW_FIELDS: [&str; 7] = [
    "OFF_SCALE",
    "OFF_X",
    "OFF_Y",
    "OFF_Z",
    "OFF_PITCH",
    "OFF_YAW",
    "OFF_ROLL",
];

 
 
 
pub(crate) type ItemView = [f32; 7];

 
pub(crate) const ITEM_VIEW_IDENTITY: ItemView = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

 
 
 
 
 
 
 
pub(crate) fn set_item_view(
    main_hand: ItemView,
    off_hand: ItemView,
    enabled: bool,
) -> anyhow::Result<()> {
    let bridge = ITEM_SCALE_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!enabled, "Item scale bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    if !enabled {
        env.set_static_field(&class, (&class, "ENABLED", "Z"), JValue::Bool(0))?;
    }
    for (fields, view) in [(MAIN_VIEW_FIELDS, main_hand), (OFF_VIEW_FIELDS, off_hand)] {
        for (name, value) in fields.into_iter().zip(view) {
            env.set_static_field(&class, (&class, name, "F"), JValue::Float(value))?;
        }
    }
    if enabled {
        env.set_static_field(&class, (&class, "ENABLED", "Z"), JValue::Bool(1))?;
    }
    Ok(())
}

 
 
 
 
 
pub(crate) fn set_sky_tint(enabled: bool, red: u8, green: u8, blue: u8) -> anyhow::Result<()> {
    let bridge = SKY_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()).clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!enabled, "Custom sky bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    if !enabled {
        env.set_static_field(&class, (&class, "ENABLED", "Z"), JValue::Bool(0))?;
    }
    env.set_static_field(&class, (&class, "RED", "I"), JValue::Int(red.into()))?;
    env.set_static_field(&class, (&class, "GREEN", "I"), JValue::Int(green.into()))?;
    env.set_static_field(&class, (&class, "BLUE", "I"), JValue::Int(blue.into()))?;
    if enabled {
        env.set_static_field(&class, (&class, "ENABLED", "Z"), JValue::Bool(1))?;
    }
    Ok(())
}

 
 
 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkyHider {
     
    Atmosphere = 1 << 0,
}

 
static SKY_HIDERS: AtomicU8 = AtomicU8::new(0);

 
fn next_sky_mask(mask: u8, hider: SkyHider, hidden: bool) -> u8 {
    let bit = hider as u8;
    if hidden {
        mask | bit
    } else {
        mask & !bit
    }
}

 
 
 
 
 
pub(crate) fn set_sky_hidden(hider: SkyHider, hidden: bool) -> anyhow::Result<()> {
    let current = SKY_HIDERS.load(Ordering::SeqCst);
    let next = next_sky_mask(current, hider, hidden);
    if next == current {
        return Ok(());
    }
    set_sky_hide(next != 0)?;
    SKY_HIDERS.store(next, Ordering::SeqCst);
    Ok(())
}

 
 
 
fn set_sky_hide(hidden: bool) -> anyhow::Result<()> {
    let bridge = SKY_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()).clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!hidden, "Custom sky bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    env.set_static_field(
        &class,
        (&class, "HIDE_VANILLA", "Z"),
        JValue::Bool(hidden.into()),
    )?;
    Ok(())
}

 
 
 
 
 
 
pub(crate) fn set_shield_hide(
    main_hand: bool,
    off_hand: bool,
    enabled: bool,
) -> anyhow::Result<()> {
    let bridge = SHIELD_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let Some(bridge) = bridge else {
        anyhow::ensure!(!enabled, "Shield bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    if !enabled {
        env.set_static_field(&class, (&class, "ENABLED", "Z"), JValue::Bool(0))?;
    }
    env.set_static_field(
        &class,
        (&class, "MAIN_HAND", "Z"),
        JValue::Bool(main_hand.into()),
    )?;
    env.set_static_field(
        &class,
        (&class, "OFF_HAND", "Z"),
        JValue::Bool(off_hand.into()),
    )?;
    if enabled {
        env.set_static_field(&class, (&class, "ENABLED", "Z"), JValue::Bool(1))?;
    }
    Ok(())
}

pub(crate) fn update_scaffold_menu() -> anyhow::Result<()> {
    set_cosmetic_flag("FREELOOK", crate::module::misc::freelook::owns_mouse())?;
    set_cosmetic_flag(
        "MENU_OPEN",
        crate::graphic::input::GUI_OPEN.load(Ordering::Relaxed),
    )
}

pub(crate) fn glow_bridge() -> Option<GlobalRef> {
    GLOW_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

pub(crate) fn classic_chams_bridge() -> Option<GlobalRef> {
    CLASSIC_CHAMS_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

 
pub(crate) fn item_chams_bridge() -> Option<GlobalRef> {
    ITEM_CHAMS_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

 
pub(crate) fn skeleton_bridge() -> Option<GlobalRef> {
    SKELETON_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

 
 
 
 
 
const TAIL_SMOKE: u8 = 1 << 0;
const TAIL_ATMOSPHERE: u8 = 1 << 1;
const TAIL_REFLECTION: u8 = 1 << 2;
const TAIL_HAZE: u8 = 1 << 3;
static WORLD_TAIL: AtomicU8 = AtomicU8::new(0);

 
 
fn next_tail_mask(mask: u8, bit: u8, active: bool) -> u8 {
    if active {
        mask | bit
    } else {
        mask & !bit
    }
}

pub(crate) fn set_smoke_active(active: bool) -> anyhow::Result<()> {
    set_world_tail(TAIL_SMOKE, active)
}

 
 
pub(crate) fn set_atmosphere_active(active: bool) -> anyhow::Result<()> {
    set_world_tail(TAIL_ATMOSPHERE, active)
}

 
pub(crate) fn set_haze_active(active: bool) -> anyhow::Result<()> {
    set_world_tail(TAIL_HAZE, active)
}

 
 
pub(crate) fn set_reflection_active(active: bool) -> anyhow::Result<()> {
    set_world_tail(TAIL_REFLECTION, active)
}

fn set_world_tail(bit: u8, active: bool) -> anyhow::Result<()> {
    let previous = WORLD_TAIL.load(Ordering::SeqCst);
    let mask = next_tail_mask(previous, bit, active);
    let result = (|| -> anyhow::Result<()> {
        let Some(bridge) = smoke_bridge() else {
             
            anyhow::ensure!(mask == 0, "Smoke JNI bridge unavailable");
            WORLD_TAIL.store(mask, Ordering::SeqCst);
            return Ok(());
        };
        let mut env = mapping().get_env()?;
        let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
        let field = env.get_static_field_id(&class, "ACTIVE", "Z")?;
        env.set_static_field(class, field, JValue::Bool((mask != 0).into()))?;
        WORLD_TAIL.store(mask, Ordering::SeqCst);
        Ok(())
    })();
    if result.is_err() {
         
        WORLD_TAIL.store(previous, Ordering::SeqCst);
    }
    result
}

pub(crate) fn smoke_bridge() -> Option<GlobalRef> {
    SMOKE_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

pub fn set_freelook_no_clip(enabled: bool) -> anyhow::Result<()> {
    let slot = FREELOOK_BRIDGE.lock().unwrap_or_else(|p| p.into_inner());
    let Some(class) = slot.as_ref() else {
        anyhow::ensure!(!enabled, "Freelook bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    let local = env.new_local_ref(class.as_obj())?;
    let class = JClass::from(local);
    let field = env.get_static_field_id(&class, "NO_CLIP", "Z")?;
    env.set_static_field(&class, field, JValue::Bool(enabled.into()))?;
    Ok(())
}

fn bridge_slot() -> &'static Mutex<Option<GlobalRef>> {
    BRIDGE.get_or_init(|| Mutex::new(None))
}

fn level_bridge_slot() -> &'static Mutex<Option<GlobalRef>> {
    LEVEL_BRIDGE.get_or_init(|| Mutex::new(None))
}
fn xray_bridge_slot() -> &'static Mutex<Option<GlobalRef>> {
    XRAY_BRIDGE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn xray_bridge() -> Option<GlobalRef> {
    xray_bridge_slot()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

 
 
pub fn install() -> anyhow::Result<()> {
    let mut env = mapping().get_env()?;
    let loader = bridge_loader(&mut env)?;

    let cosmetics_class = match load_existing(&mut env, &loader, "InferedCosmeticsBridge") {
        Some(existing) => existing,
        None => env.define_class(
            "InferedCosmeticsBridge",
            loader.as_obj(),
            include_bytes!(env!("INFERED_COSMETICS_BRIDGE_CLASS")),
        )?,
    };
     
     
    let scaffold_result = (|| -> anyhow::Result<()> {
         
         
         
         
        for (name, bytes) in [
            (
                "InferedScaffoldLogic$Line",
                include_bytes!(env!("INFERED_SCAFFOLD_LINE_CLASS")).as_slice(),
            ),
            (
                "InferedScaffoldLogic$SupportCandidate",
                include_bytes!(env!("INFERED_SCAFFOLD_SUPPORT_CLASS")).as_slice(),
            ),
            (
                "InferedScaffoldLogic$FaceTarget",
                include_bytes!(env!("INFERED_SCAFFOLD_FACE_CLASS")).as_slice(),
            ),
            (
                "InferedScaffoldLogic",
                include_bytes!(env!("INFERED_SCAFFOLD_LOGIC_CLASS")).as_slice(),
            ),
        ] {
            if load_existing(&mut env, &loader, name).is_none() {
                env.define_class(name, loader.as_obj(), bytes)?;
            }
        }
        let scaffold_class = match load_existing(&mut env, &loader, "InferedScaffoldBridge") {
            Some(existing) => existing,
            None => env.define_class(
                "InferedScaffoldBridge",
                loader.as_obj(),
                include_bytes!(env!("INFERED_SCAFFOLD_BRIDGE_CLASS")),
            )?,
        };
        env.register_native_methods(
            &scaffold_class,
            &[NativeMethod {
                name: "armBodyNative".into(),
                sig: "(FF)Z".into(),
                fn_ptr: scaffold_arm_body as *mut c_void,
            }],
        )?;
        env.call_static_method(
            &scaffold_class,
            "configure",
            "(Ljava/lang/Object;)V",
            &[JValue::Object(minecraft().jni_ref().as_obj())],
        )?;
        *SCAFFOLD_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
            Some(env.new_global_ref(scaffold_class)?);
        Ok(())
    })();
    if let Err(error) = scaffold_result {
        if env.exception_check()? {
            env.exception_describe()?;
            env.exception_clear()?;
        }
        log::warn!("Scaffold unavailable; other bridges remain independent: {error}");
        SCAFFOLD_BRIDGE
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
    }
     
     
    let fast_scaffold_result = (|| -> anyhow::Result<()> {
        for (name, bytes) in [
            (
                "InferedFastScaffoldLogic$Line",
                include_bytes!(env!("INFERED_FAST_SCAFFOLD_LINE_CLASS")).as_slice(),
            ),
            (
                "InferedFastScaffoldLogic$SupportCandidate",
                include_bytes!(env!("INFERED_FAST_SCAFFOLD_SUPPORT_CLASS")).as_slice(),
            ),
            (
                "InferedFastScaffoldLogic$FaceTarget",
                include_bytes!(env!("INFERED_FAST_SCAFFOLD_FACE_CLASS")).as_slice(),
            ),
            (
                "InferedFastScaffoldLogic",
                include_bytes!(env!("INFERED_FAST_SCAFFOLD_LOGIC_CLASS")).as_slice(),
            ),
        ] {
            if load_existing(&mut env, &loader, name).is_none() {
                env.define_class(name, loader.as_obj(), bytes)?;
            }
        }
        let class = match load_existing(&mut env, &loader, "InferedFastScaffoldBridge") {
            Some(existing) => existing,
            None => env.define_class(
                "InferedFastScaffoldBridge",
                loader.as_obj(),
                include_bytes!(env!("INFERED_FAST_SCAFFOLD_BRIDGE_CLASS")),
            )?,
        };
        env.register_native_methods(
            &class,
            &[NativeMethod {
                name: "armBodyNative".into(),
                sig: "(FF)Z".into(),
                fn_ptr: fast_scaffold_arm_body as *mut c_void,
            }],
        )?;
        env.call_static_method(
            &class,
            "configure",
            "(Ljava/lang/Object;)V",
            &[JValue::Object(minecraft().jni_ref().as_obj())],
        )?;
        *FAST_SCAFFOLD_BRIDGE
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Some(env.new_global_ref(class)?);
        Ok(())
    })();
    if let Err(error) = fast_scaffold_result {
        if env.exception_check()? {
            env.exception_describe()?;
            env.exception_clear()?;
        }
        log::warn!("Fast Scaffold unavailable; original remains independent: {error}");
        FAST_SCAFFOLD_BRIDGE
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
    }
    let artwork = env.byte_array_from_slice(include_bytes!("../../logo's/cape.png"))?;
    env.call_static_method(
        &cosmetics_class,
        "configure",
        "(Ljava/lang/Object;[B)V",
        &[
            JValue::Object(minecraft().jni_ref().as_obj()),
            JValue::Object(&artwork),
        ],
    )?;
    *COSMETICS_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(cosmetics_class)?);

    let freelook_class = match load_existing(&mut env, &loader, "InferedFreelookBridge") {
        Some(existing) => existing,
        None => env.define_class(
            "InferedFreelookBridge",
            loader.as_obj(),
            include_bytes!(env!("INFERED_FREELOOK_BRIDGE_CLASS")),
        )?,
    };
    *FREELOOK_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(freelook_class)?);
    set_freelook_no_clip(false)?;

    let camera_class = match load_existing(&mut env, &loader, BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env
            .define_class(BRIDGE_CLASS, loader.as_obj(), BRIDGE_BYTECODE)
            .map_err(|error| describe_jni_error(&mut env, "camera bridge DefineClass", error))?,
    };
    let camera_methods = [NativeMethod {
        name: "afterCameraSetup".into(),
        sig: "(Ljava/lang/Object;)V".into(),
        fn_ptr: after_camera_setup as *mut c_void,
    }];
    env.register_native_methods(&camera_class, &camera_methods)
        .map_err(|error| describe_jni_error(&mut env, "camera bridge RegisterNatives", error))?;

    let level_class = match load_existing(&mut env, &loader, LEVEL_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env
            .define_class(LEVEL_BRIDGE_CLASS, loader.as_obj(), LEVEL_BRIDGE_BYTECODE)
            .map_err(|error| {
                describe_jni_error(&mut env, "level renderer bridge DefineClass", error)
            })?,
    };
    let level_methods = [
        NativeMethod {
            name: "afterCullPrepare".into(),
            sig: "(Ljava/lang/Object;)V".into(),
            fn_ptr: after_cull_prepare as *mut c_void,
        },
        NativeMethod {
            name: "onGraphInvalidate".into(),
            sig: "(Ljava/lang/Object;)V".into(),
            fn_ptr: on_graph_invalidate as *mut c_void,
        },
    ];
    env.register_native_methods(&level_class, &level_methods)
        .map_err(|error| {
            describe_jni_error(&mut env, "level renderer bridge RegisterNatives", error)
        })?;

    {
        let global = env.new_global_ref(camera_class)?;
        let mut slot = bridge_slot()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = Some(global);
    }
     
     
     
    let crystal_result = (|| -> anyhow::Result<()> {
        let crystal_class = match load_existing(&mut env, &loader, CRYSTAL_BRIDGE_CLASS) {
            Some(existing) => existing,
            None => env.define_class(
                CRYSTAL_BRIDGE_CLASS,
                loader.as_obj(),
                CRYSTAL_BRIDGE_BYTECODE,
            )?,
        };
        env.register_native_methods(
            &crystal_class,
            &[NativeMethod {
                name: "tick".into(),
                sig: "()V".into(),
                fn_ptr: crystal_input_phase as *mut c_void,
            }],
        )?;
        *CRYSTAL_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
            Some(env.new_global_ref(crystal_class)?);
        set_crystal_input_enabled(true)?;
        Ok(())
    })();
    if let Err(error) = crystal_result {
        if env.exception_check()? {
            env.exception_describe()?;
            env.exception_clear()?;
        }
        log::warn!("Crystal input phase unavailable; the macro keeps frame-tick ordering: {error}");
        CRYSTAL_BRIDGE
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        let _ = set_crystal_input_enabled(false);
    }
     
     
     
     
    let optimizer_result = (|| -> anyhow::Result<()> {
        let optimizer_class = match load_existing(&mut env, &loader, CRYSTAL_OPTIMIZER_BRIDGE_CLASS)
        {
            Some(existing) => existing,
            None => env.define_class(
                CRYSTAL_OPTIMIZER_BRIDGE_CLASS,
                loader.as_obj(),
                CRYSTAL_OPTIMIZER_BRIDGE_BYTECODE,
            )?,
        };
        env.register_native_methods(
            &optimizer_class,
            &[NativeMethod {
                name: "noteAttack".into(),
                sig: "(Ljava/lang/Object;)V".into(),
                fn_ptr: crate::module::combat::crystal_optimizer::note_attack as *mut c_void,
            }],
        )?;
        *CRYSTAL_OPTIMIZER_BRIDGE
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Some(env.new_global_ref(optimizer_class)?);
         
        set_crystal_optimizer_enabled(false)?;
        Ok(())
    })();
    if let Err(error) = optimizer_result {
        if env.exception_check()? {
            env.exception_describe()?;
            env.exception_clear()?;
        }
        log::warn!("Crystal optimizer unavailable; the module stays inert on this build: {error}");
        CRYSTAL_OPTIMIZER_BRIDGE
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        let _ = set_crystal_optimizer_enabled(false);
    }
    {
        let global = env.new_global_ref(level_class)?;
        let mut slot = level_bridge_slot()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = Some(global);
    }
    let smoke_class = match load_existing(&mut env, &loader, "InferedSmokeBridge") {
        Some(existing) => existing,
        None => env.define_class(
            "InferedSmokeBridge",
            loader.as_obj(),
            include_bytes!(env!("INFERED_SMOKE_BRIDGE_CLASS")),
        )?,
    };
    env.register_native_methods(
        &smoke_class,
        &[NativeMethod {
            name: "drawWorldRender".into(),
            sig: "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)V"
                .into(),
            fn_ptr: after_world_render as *mut c_void,
        }],
    )?;
    *SMOKE_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(smoke_class)?);
    set_smoke_active(false)?;
    let xray_class = match load_existing(&mut env, &loader, XRAY_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env.define_class(XRAY_BRIDGE_CLASS, loader.as_obj(), XRAY_BRIDGE_BYTECODE)?,
    };
    *xray_bridge_slot().lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(xray_class)?);

    let glow_class = match load_existing(&mut env, &loader, "InferedGlowBridge") {
        Some(existing) => existing,
        None => env.define_class(
            "InferedGlowBridge",
            loader.as_obj(),
            include_bytes!(env!("INFERED_GLOW_BRIDGE_CLASS")),
        )?,
    };
    *GLOW_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) = Some(env.new_global_ref(glow_class)?);

    let classic_chams_class = match load_existing(&mut env, &loader, "InferedClassicChamsBridge") {
        Some(existing) => existing,
        None => env.define_class(
            "InferedClassicChamsBridge",
            loader.as_obj(),
            include_bytes!(env!("INFERED_CLASSIC_CHAMS_BRIDGE_CLASS")),
        )?,
    };
    *CLASSIC_CHAMS_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = Some(env.new_global_ref(classic_chams_class)?);

    let item_chams_class = match load_existing(&mut env, &loader, "InferedItemChamsBridge") {
        Some(existing) => existing,
        None => env.define_class(
            "InferedItemChamsBridge",
            loader.as_obj(),
            include_bytes!(env!("INFERED_ITEM_CHAMS_BRIDGE_CLASS")),
        )?,
    };
    *ITEM_CHAMS_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(item_chams_class)?);

    let skeleton_class = match load_existing(&mut env, &loader, "InferedSkeletonBridge") {
        Some(existing) => existing,
        None => env.define_class(
            "InferedSkeletonBridge",
            loader.as_obj(),
            include_bytes!(env!("INFERED_SKELETON_BRIDGE_CLASS")),
        )?,
    };
    *SKELETON_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(skeleton_class)?);

     
     
     
    let explosion_class = match load_existing(&mut env, &loader, EXPLOSION_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env.define_class(
            EXPLOSION_BRIDGE_CLASS,
            loader.as_obj(),
            include_bytes!(env!("INFERED_EXPLOSION_BRIDGE_CLASS")),
        )?,
    };
    *EXPLOSION_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(explosion_class)?);
     
    set_explosion_suppression(false)?;

     
     
     
    let fire_class = match load_existing(&mut env, &loader, FIRE_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env.define_class(
            FIRE_BRIDGE_CLASS,
            loader.as_obj(),
            include_bytes!(env!("INFERED_FIRE_BRIDGE_CLASS")),
        )?,
    };
    *FIRE_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) = Some(env.new_global_ref(fire_class)?);
     
    set_fire_hidden(false)?;

     
     
    let crosshair_class = match load_existing(&mut env, &loader, CROSSHAIR_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env.define_class(
            CROSSHAIR_BRIDGE_CLASS,
            loader.as_obj(),
            include_bytes!(env!("INFERED_CROSSHAIR_BRIDGE_CLASS")),
        )?,
    };
    *CROSSHAIR_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(crosshair_class)?);
     
    set_crosshair_hidden(false)?;

     
     
    let title_logo_class = match load_existing(&mut env, &loader, TITLE_LOGO_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env.define_class(
            TITLE_LOGO_BRIDGE_CLASS,
            loader.as_obj(),
            include_bytes!(env!("INFERED_TITLE_LOGO_BRIDGE_CLASS")),
        )?,
    };
    *TITLE_LOGO_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(title_logo_class)?);
     
    set_title_logo_hidden(false)?;

     
     
     
    let no_particles_class = match load_existing(&mut env, &loader, NO_PARTICLES_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env.define_class(
            NO_PARTICLES_BRIDGE_CLASS,
            loader.as_obj(),
            include_bytes!(env!("INFERED_NO_PARTICLES_BRIDGE_CLASS")),
        )?,
    };
    *NO_PARTICLES_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = Some(env.new_global_ref(no_particles_class)?);
     
    set_no_particles_hidden(false)?;

     
     
     
     
     
    let item_scale_class = match load_existing(&mut env, &loader, ITEM_SCALE_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env.define_class(
            ITEM_SCALE_BRIDGE_CLASS,
            loader.as_obj(),
            include_bytes!(env!("INFERED_ITEM_SCALE_BRIDGE_CLASS")),
        )?,
    };
    *ITEM_SCALE_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(item_scale_class)?);
     
    set_item_view(ITEM_VIEW_IDENTITY, ITEM_VIEW_IDENTITY, false)?;

     
    let shield_class = match load_existing(&mut env, &loader, SHIELD_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env.define_class(
            SHIELD_BRIDGE_CLASS,
            loader.as_obj(),
            include_bytes!(env!("INFERED_SHIELD_BRIDGE_CLASS")),
        )?,
    };
    *SHIELD_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(shield_class)?);
     
    set_shield_hide(false, false, false)?;

     
     
     
     
    if load_existing(&mut env, &loader, "InferedSwingEchoBridge$Trail").is_none() {
        env.define_class(
            "InferedSwingEchoBridge$Trail",
            loader.as_obj(),
            include_bytes!(env!("INFERED_SWING_ECHO_TRAIL_CLASS")),
        )?;
    }
     
     
     
     
    if load_existing(&mut env, &loader, "InferedSwingEchoBridge$History").is_none() {
        env.define_class(
            "InferedSwingEchoBridge$History",
            loader.as_obj(),
            include_bytes!(env!("INFERED_SWING_ECHO_HISTORY_CLASS")),
        )?;
    }
    let swing_echo_class = match load_existing(&mut env, &loader, SWING_ECHO_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env.define_class(
            SWING_ECHO_BRIDGE_CLASS,
            loader.as_obj(),
            include_bytes!(env!("INFERED_SWING_ECHO_BRIDGE_CLASS")),
        )?,
    };
    *SWING_ECHO_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(swing_echo_class)?);
     
    set_swing_echo(false)?;

     
    let hand_outline_class = match load_existing(&mut env, &loader, HAND_OUTLINE_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env.define_class(
            HAND_OUTLINE_BRIDGE_CLASS,
            loader.as_obj(),
            include_bytes!(env!("INFERED_HAND_OUTLINE_BRIDGE_CLASS")),
        )?,
    };
    *HAND_OUTLINE_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = Some(env.new_global_ref(hand_outline_class)?);
     
    set_hand_outline(false)?;

     
     
     
    for (name, bytes) in [
        (
            "InferedHandAuraBridge$Gl",
            include_bytes!(env!("INFERED_HAND_AURA_GL_CLASS")).as_slice(),
        ),
        (
            "InferedHandAuraBridge$Gl$State",
            include_bytes!(env!("INFERED_HAND_AURA_GL_STATE_CLASS")).as_slice(),
        ),
         
         
        (
            "InferedHandAuraBridge$Look",
            include_bytes!(env!("INFERED_HAND_AURA_LOOK_CLASS")).as_slice(),
        ),
    ] {
        if load_existing(&mut env, &loader, name).is_none() {
            env.define_class(name, loader.as_obj(), bytes)?;
        }
    }
    let hand_aura_class = match load_existing(&mut env, &loader, HAND_AURA_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env.define_class(
            HAND_AURA_BRIDGE_CLASS,
            loader.as_obj(),
            include_bytes!(env!("INFERED_HAND_AURA_BRIDGE_CLASS")),
        )?,
    };
    *HAND_AURA_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(hand_aura_class)?);
     
    set_hand_aura(false)?;

     
     
    let totem_pop_class = match load_existing(&mut env, &loader, TOTEM_POP_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env.define_class(
            TOTEM_POP_BRIDGE_CLASS,
            loader.as_obj(),
            include_bytes!(env!("INFERED_TOTEM_POP_BRIDGE_CLASS")),
        )?,
    };
    *TOTEM_POP_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) =
        Some(env.new_global_ref(totem_pop_class)?);
     
    set_totem_pop_scale(1.0)?;

     
     
    let sky_class = match load_existing(&mut env, &loader, SKY_BRIDGE_CLASS) {
        Some(existing) => existing,
        None => env.define_class(
            SKY_BRIDGE_CLASS,
            loader.as_obj(),
            include_bytes!(env!("INFERED_SKY_BRIDGE_CLASS")),
        )?,
    };
     
     
    env.register_native_methods(
        &sky_class,
        &[NativeMethod {
            name: "drawSky".into(),
            sig: "()V".into(),
            fn_ptr: draw_sky as *mut c_void,
        }],
    )?;
    *SKY_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()) = Some(env.new_global_ref(sky_class)?);
     
     
    set_sky_tint(false, 255, 255, 255)?;
    SKY_HIDERS.store(0, Ordering::SeqCst);
    set_sky_hide(false)?;
    log::info!("[Glow] geometry bridge registered: Minecraft 1.21.4");
    log::info!("[Smoke] JNI bridge registered");
    log::info!("InferedCameraBridge and InferedLevelRendererBridge defined and registered");
    Ok(())
}

fn bridge_loader(env: &mut JNIEnv<'_>) -> anyhow::Result<GlobalRef> {
    if let Some(loader) = mapping().game_class_loader() {
        return Ok(loader);
    }
    let camera_class = mapping().resolve_class(env, "net/minecraft/class_4184")?;
    let loader = env
        .call_method(
            &camera_class,
            "getClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )?
        .l()?;
    if loader.is_null() {
        anyhow::bail!("camera bridge: Camera class has no usable class loader");
    }
    log::info!("camera bridge: captured loader from runtime Camera class");
    Ok(env.new_global_ref(loader)?)
}

fn load_existing<'a>(env: &mut JNIEnv<'a>, loader: &GlobalRef, name: &str) -> Option<JClass<'a>> {
    let name_obj = env.new_string(name).ok()?;
    let result = env.call_method(
        loader.as_obj(),
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[JValue::Object(&name_obj)],
    );
    match result.and_then(|value| value.l()) {
        Ok(class) if !class.is_null() => Some(JClass::from(class)),
        _ => {
            let _ = env.exception_clear();
            None
        }
    }
}

fn describe_jni_error(env: &mut JNIEnv, what: &str, error: jni::errors::Error) -> anyhow::Error {
    if env.exception_check().unwrap_or(false) {
        let detail = env
            .exception_occurred()
            .ok()
            .and_then(|throwable| {
                let _ = env.exception_clear();
                env.call_method(&throwable, "toString", "()Ljava/lang/String;", &[])
                    .ok()
            })
            .and_then(|value| value.l().ok())
            .and_then(|obj| {
                let string = JString::from(obj);
                let value = env.get_string(&string).ok()?.to_string_lossy().into_owned();
                Some(value)
            });
        if let Some(detail) = detail {
            return anyhow::anyhow!("{what} failed: {detail}");
        }
    }
    anyhow::anyhow!("{what} failed: {error}")
}

 
 
 
 
 
pub(crate) fn camera_frame_ran() -> bool {
    CAMERA_CALLBACK_LOGGED.load(Ordering::Relaxed)
}

 
 
 
 
 
 
 
 
 
 
 
 
 
unsafe extern "system" fn after_camera_setup(
    env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    camera: jni::sys::jobject,
) {
    if CAMERA_OVERRIDE_IN_PROGRESS.swap(true, Ordering::Acquire) {
        return;
    }
    let result = catch_unwind(AssertUnwindSafe(|| -> anyhow::Result<()> {
        if camera.is_null() {
            return Ok(());
        }
        let env = unsafe { JNIEnv::from_raw(env)? };
        let camera = unsafe { JObject::from_raw(camera) };
        if !CAMERA_CALLBACK_LOGGED.swap(true, Ordering::Relaxed) {
            log::info!("InferedCameraBridge camera-setup callback fired");
        }
        if freecam_enabled() && freecam_debug_bridge_allowed() {
            log::info!(
                "FREECAM DEBUG bridge: enabled=true, camera=0x{:x}",
                camera.as_raw() as usize
            );
        }
        if crate::RUNNING.load(Ordering::SeqCst) {
             
             
             
             
            if let Some(view) = crate::module::combat::rotation::restore_body() {
                let _ = mapping().call_method(
                    crate::mapping::MinecraftClassType::Camera,
                    &camera,
                    "setRotation",
                    &[JValue::Float(view.yaw), JValue::Float(view.pitch)],
                );
            }
            client().modules.camera_setup_object(&camera)?;
             
             
             
            crate::graphic::visual_silent::apply();
        }
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
            anyhow::bail!("camera bridge callback left a pending Java exception");
        }
        Ok(())
    }));

    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            log::warn!("InferedCameraBridge callback failed; camera left unchanged: {error}")
        }
        Err(_) => log::error!("InferedCameraBridge callback panicked; camera left unchanged"),
    }
    CAMERA_OVERRIDE_IN_PROGRESS.store(false, Ordering::Release);
}

fn freecam_enabled() -> bool {
    client()
        .modules
        .get(ModuleId::Freecam)
        .and_then(|handle| {
            handle
                .lock()
                .ok()
                .map(|module| module.get_module_data().enabled)
        })
        .unwrap_or(false)
}

 
 
 
 
 
 
 
fn culling_reveal_active() -> bool {
    client().modules.is_active()
        && (freecam_enabled()
            || crate::module::misc::freelook::render_reveal_active()
            || crate::module::render::xray::XrayModule::is_active())
}

 
 
 
 
 
 
 
 
 
 
 
 
fn set_partial_suppressed(suppressed: bool) -> anyhow::Result<()> {
    let bridge = level_bridge_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
        .ok_or_else(|| anyhow::anyhow!("level renderer bridge class unavailable"))?;
    let mut env = mapping().get_env()?;
    let local = env.new_local_ref(bridge.as_obj())?;
    let class = JClass::from(local);
    env.set_static_field(
        &class,
        (&class, "PARTIAL_SUPPRESSED", "Z"),
        JValue::Bool(if suppressed { 1 } else { 0 }),
    )
    .map_err(|error| describe_jni_error(&mut env, "PARTIAL_SUPPRESSED write", error))?;
    Ok(())
}

 
 
 
 
 
 
 
 
unsafe extern "system" fn after_cull_prepare(
    env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    level_renderer: jni::sys::jobject,
) {
    if CULL_OVERRIDE_IN_PROGRESS.swap(true, Ordering::Acquire) {
        return;
    }
    let result = catch_unwind(AssertUnwindSafe(|| -> anyhow::Result<()> {
        if level_renderer.is_null() {
            return Ok(());
        }
        let _env = unsafe { JNIEnv::from_raw(env)? };
        let renderer = unsafe { jni::objects::JObject::from_raw(level_renderer) };
        if culling_reveal_active() {
            crate::mapping::client::level_renderer::engage(&renderer)?;
        } else if crate::mapping::client::level_renderer::expansion_armed() {
            crate::mapping::client::level_renderer::restore_smart_cull(&renderer)?;
        }
         
         
         
        set_partial_suppressed(crate::mapping::client::level_renderer::partial_suppressed())?;
        if _env.exception_check().unwrap_or(false) {
            let _ = _env.exception_clear();
            anyhow::bail!("cull bridge callback left a pending Java exception");
        }
        Ok(())
    }));

    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => log::warn!(
            "InferedLevelRendererBridge cull callback failed; using game frustum: {error}"
        ),
        Err(_) => log::error!("InferedLevelRendererBridge cull callback panicked"),
    }
    CULL_OVERRIDE_IN_PROGRESS.store(false, Ordering::Release);
}

 
 
 
 
 
 
 
 
 
 
 
unsafe extern "system" fn on_graph_invalidate(
    env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    graph: jni::sys::jobject,
) {
    if GRAPH_INVALIDATE_IN_PROGRESS.swap(true, Ordering::Acquire) {
        return;
    }
    let result = catch_unwind(AssertUnwindSafe(|| -> anyhow::Result<()> {
        if graph.is_null() {
            return Ok(());
        }
        let env = unsafe { JNIEnv::from_raw(env)? };
        let graph = unsafe { jni::objects::JObject::from_raw(graph) };
        crate::mapping::client::level_renderer::on_graph_invalidate(&graph)?;
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
            anyhow::bail!("graph invalidate callback left a pending Java exception");
        }
        Ok(())
    }));

    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            log::warn!("InferedLevelRendererBridge graph-invalidate callback failed: {error}")
        }
        Err(_) => log::error!("InferedLevelRendererBridge graph-invalidate callback panicked"),
    }
    GRAPH_INVALIDATE_IN_PROGRESS.store(false, Ordering::Release);
}

 
pub(crate) fn configure_xray(ids: &[String], active: bool) -> anyhow::Result<()> {
    let Some(bridge) = xray_bridge() else {
        anyhow::ensure!(!active, "Xray bridge unavailable");
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    env.with_local_frame(32, |env| -> anyhow::Result<()> {
        let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
        if active {
            let registry = crate::mapping::chunk::block_registry()?;
            let map = env.new_object("java/util/IdentityHashMap", "()V", &[])?;
            let states = env
                .call_static_method(
                    "java/util/Collections",
                    "newSetFromMap",
                    "(Ljava/util/Map;)Ljava/util/Set;",
                    &[JValue::Object(&map)],
                )?
                .l()?;
            for id in ids {
                env.with_local_frame(16, |env| -> anyhow::Result<()> {
                    let text = env.new_string(id)?;
                    let key = mapping()
                        .call_static_method(
                            crate::mapping::MinecraftClassType::ResourceLocation,
                            "parse",
                            &[JValue::Object(&text)],
                        )?
                        .l()?;
                    let block = mapping()
                        .call_method(
                            crate::mapping::MinecraftClassType::Registry,
                            registry.as_obj(),
                            "getValue",
                            &[JValue::Object(&key)],
                        )?
                        .l()?;
                    anyhow::ensure!(!block.is_null(), "Xray block not registered: {id}");
                    let definition = mapping()
                        .call_method(
                            crate::mapping::MinecraftClassType::Block,
                            &block,
                            "getStateDefinition",
                            &[],
                        )?
                        .l()?;
                    let list = mapping()
                        .call_method(
                            crate::mapping::MinecraftClassType::StateDefinition,
                            &definition,
                            "getPossibleStates",
                            &[],
                        )?
                        .l()?;
                    let count = env.call_method(&list, "size", "()I", &[])?.i()?;
                    for i in 0..count {
                        let state = env
                            .call_method(&list, "get", "(I)Ljava/lang/Object;", &[JValue::Int(i)])?
                            .l()?;
                        env.call_method(
                            &states,
                            "add",
                            "(Ljava/lang/Object;)Z",
                            &[JValue::Object(&state)],
                        )?;
                        env.delete_local_ref(state)?;
                    }
                    Ok(())
                })?;
            }
            let invisible = mapping()
                .get_static_field(
                    crate::mapping::MinecraftClassType::RenderShape,
                    "INVISIBLE",
                    crate::mapping::FieldType::Object(
                        crate::mapping::MinecraftClassType::RenderShape,
                    ),
                )?
                .l()?;
            env.set_static_field(
                &class,
                (&class, "TARGET_STATES", "Ljava/util/Set;"),
                JValue::Object(&states),
            )?;
            env.set_static_field(
                &class,
                (&class, "INVISIBLE", "Ljava/lang/Object;"),
                JValue::Object(&invisible),
            )?;
        }
        env.set_static_field(&class, (&class, "ACTIVE", "Z"), JValue::Bool(active.into()))?;
        Ok(())
    })
}

pub(crate) fn rebuild_world_render() -> anyhow::Result<()> {
    use crate::mapping::{FieldType, MinecraftClassType};
    mapping().in_frame(|| {
        let renderer = mapping()
            .get_field(
                MinecraftClassType::Minecraft,
                minecraft().jni_ref().as_obj(),
                "levelRenderer",
                FieldType::Object(MinecraftClassType::LevelRenderer),
            )?
            .l()?;
        if !renderer.is_null() && minecraft().world()?.is_some() {
            mapping().call_method(
                MinecraftClassType::LevelRenderer,
                &renderer,
                "allChanged",
                &[],
            )?;
        }
        Ok(())
    })
}

 
 
 
 
 
 
 
 
unsafe extern "system" fn draw_sky(_raw_env: *mut jni::sys::JNIEnv, _class: jni::sys::jclass) {
    if !crate::RUNNING.load(Ordering::SeqCst) {
        return;
    }
    let result = catch_unwind(AssertUnwindSafe(|| -> anyhow::Result<()> {
        let env = mapping().get_env()?;
         
         
         
        let result = crate::graphic::atmosphere::render_sky_pass();
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
        result
    }));
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => log::error!("[Atmosphere] sky-pass render failed: {error:#}"),
        Err(_) => log::error!("[Atmosphere] sky-pass render panicked"),
    }
}

unsafe extern "system" fn after_world_render(
    raw_env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    _renderer: jni::sys::jobject,
    camera: jni::sys::jobject,
    view: jni::sys::jobject,
    projection: jni::sys::jobject,
) {
    if !crate::RUNNING.load(Ordering::SeqCst) {
        return;
    }
    let result = catch_unwind(AssertUnwindSafe(|| -> anyhow::Result<()> {
        let env = unsafe { JNIEnv::from_raw(raw_env)? };
        let camera = unsafe { JObject::from_raw(camera) };
        let view = unsafe { JObject::from_raw(view) };
        let projection = unsafe { JObject::from_raw(projection) };
         
         
        let cosmetics = COSMETICS_BRIDGE
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        if let Some(bridge) = cosmetics {
            let mut env = mapping().get_env()?;
            let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
            env.call_static_method(
                class,
                "render",
                "(Ljava/lang/Object;Ljava/lang/Object;)V",
                &[JValue::Object(&camera), JValue::Object(&view)],
            )?;
        }
         
         
         
         
        if !crate::jvmti::sky_drawn_in_sky_pass() {
            if let Err(error) = crate::graphic::atmosphere::render_world() {
                log::error!("[Atmosphere] render failed: {error:#}");
            }
        }
         
         
        if let Err(error) = crate::graphic::reflection::render_world(&camera, &view, &projection) {
             
             
            log::debug!("[Reflection] frame skipped: {error:#}");
        }
        if let Err(error) = crate::graphic::haze::render_world(&view, &projection) {
            log::debug!("[Haze] frame skipped: {error:#}");
        }
        let result = crate::graphic::smoke::render_world(&camera, &view, &projection);
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
            anyhow::bail!("Smoke callback left a Java exception");
        }
        result
    }));
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => crate::graphic::smoke::report_failure(&error.to_string()),
        Err(_) => crate::graphic::smoke::report_failure("render callback panicked"),
    }
}

fn freecam_debug_bridge_allowed() -> bool {
    let now = FREECAM_DEBUG_EPOCH
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis() as u64;
    let last = FREECAM_DEBUG_LAST_MS.load(Ordering::Relaxed);
    if now < last + 1000 {
        return false;
    }
    FREECAM_DEBUG_LAST_MS.store(now, Ordering::Relaxed);
    true
}

pub fn uninstall() {
     
     
     
    let _ = std::panic::catch_unwind(|| {
        let _ = crate::module::combat::rotation::restore_body();
    });
     
     
    if let Err(error) = set_crystal_input_enabled(false) {
        log::warn!("Crystal input phase cleanup: {error}");
    }
    CRYSTAL_INPUT_TICKS.store(0, Ordering::Relaxed);
     
     
     
    if let Err(error) = set_crystal_optimizer_enabled(false) {
        log::debug!("Crystal optimizer cleanup: {error}");
    }
    CRYSTAL_OPTIMIZER_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
    if let Err(error) = set_fast_scaffold_active(false) {
        log::warn!("Fast Scaffold cleanup: {error}");
    }
    FAST_SCAFFOLD_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
    if let Err(error) = set_scaffold_active(false) {
        log::warn!("Scaffold cleanup: {error}");
    }
    SCAFFOLD_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
    for flag in [
        "CAPE",
        "WINGS",
        "FREECAM",
        "SCAFFOLD",
        "MENU_OPEN",
        "FREELOOK",
    ] {
        if let Err(error) = set_cosmetic_flag(flag, false) {
            log::warn!("Cosmetics cleanup {flag}: {error}");
        }
    }
    if let Some(bridge) = COSMETICS_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take()
    {
        if let Ok(mut env) = mapping().get_env() {
            let result = env
                .new_local_ref(bridge.as_obj())
                .and_then(|local| env.call_static_method(JClass::from(local), "clear", "()V", &[]));
            if let Err(error) = result {
                log::warn!("Cosmetics resource cleanup: {error}");
                let _ = env.exception_clear();
            }
        }
    }
    let _ = configure_xray(&[], false);
    let _ = set_smoke_active(false);
    let _ = set_atmosphere_active(false);
    let _ = set_freelook_no_clip(false);
     
     
    let _ = set_explosion_suppression(false);
    EXPLOSION_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
     
    let _ = set_fire_hidden(false);
    FIRE_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()).take();
     
    let _ = set_crosshair_hidden(false);
    CROSSHAIR_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
     
    let _ = set_title_logo_hidden(false);
    TITLE_LOGO_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
     
    let _ = set_no_particles_hidden(false);
    NO_PARTICLES_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
     
     
    let _ = set_item_view(ITEM_VIEW_IDENTITY, ITEM_VIEW_IDENTITY, false);
    ITEM_SCALE_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
     
    let _ = set_swing_echo(false);
    SWING_ECHO_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
    let _ = set_hand_outline(false);
    HAND_OUTLINE_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
    let _ = set_hand_aura(false);
    HAND_AURA_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
     
    let _ = set_shield_hide(false, false, false);
    SHIELD_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
     
     
    let _ = set_sky_tint(false, 255, 255, 255);
    SKY_HIDERS.store(0, Ordering::SeqCst);
    let _ = set_sky_hide(false);
    SKY_BRIDGE.lock().unwrap_or_else(|p| p.into_inner()).take();
    FREELOOK_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
    CRYSTAL_BRIDGE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take();
    if let Err(error) = set_partial_suppressed(false) {
        log::debug!("PARTIAL_SUPPRESSED restore at teardown failed: {error}");
    }
    if let Err(error) = crate::mapping::client::level_renderer::restore_smart_cull_value() {
        log::debug!("smartCull restore at teardown failed: {error}");
    }
    for slot in [
        bridge_slot(),
        level_bridge_slot(),
        &SMOKE_BRIDGE,
        &GLOW_BRIDGE,
        &CLASSIC_CHAMS_BRIDGE,
        xray_bridge_slot(),
    ] {
        let mut slot = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(class) = slot.take() else {
            continue;
        };
        if let Ok(mut env) = mapping().get_env() {
            if let Ok(local) = env.new_local_ref(class.as_obj()) {
                if let Err(error) = env.unregister_native_methods(JClass::from(local)) {
                    log::debug!("bridge UnregisterNatives failed: {error}");
                    let _ = env.exception_clear();
                }
            }
        }
    }
}
