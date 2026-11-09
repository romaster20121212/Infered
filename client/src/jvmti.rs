 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::config;
use crate::mapping::MinecraftClassType;
use crate::state::mapping;
use jni::objects::{JObject, JString, JValue};
use jni::sys::{jint, JNI_GetCreatedJavaVMs, JavaVM as RawJavaVM, JNI_OK};
use ristretto_classfile::attributes::{Attribute, Instruction, StackFrame, VerificationType};
use ristretto_classfile::{ClassFile, Constant, MethodAccessFlags};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;

extern "C" {
    fn dark_jvmti_class_hook_diagnose(
        vm: *mut RawJavaVM,
        callback: extern "C" fn(
            *const c_char,
            jint,
            u8,
            jint,
            *const u8,
            *mut *mut u8,
            *mut jint,
        ) -> i32,
        statuses: *mut i32,
        status_count: i32,
        values: *mut i32,
        value_count: i32,
        pointers: *mut usize,
        pointer_count: i32,
    ) -> i32;
    fn dark_jvmti_class_hook_uninstall();
    fn dark_jvmti_retransform_diagnose(
        target_class: jni::sys::jclass,
        statuses: *mut i32,
        status_count: i32,
        values: *mut i32,
        value_count: i32,
        pointers: *mut usize,
        pointer_count: i32,
    ) -> i32;
    #[link_name = "dark_jvmti_status_name"]
    fn dark_jvmti_status_name_ptr(status: i32) -> *const c_char;
}

const CAMERA_TARGET_CLASS: &str = "net/minecraft/class_4184";
const CAMERA_SETUP_METHOD: &str = "method_19321";
const CAMERA_SETUP_DESCRIPTOR: &str = "(Lnet/minecraft/class_1922;Lnet/minecraft/class_1297;ZZF)V";
const BRIDGE_CLASS: &str = "InferedCameraBridge";
const BRIDGE_METHOD: &str = "afterCameraSetup";
const BRIDGE_DESCRIPTOR: &str = "(Ljava/lang/Object;)V";

 
 
 
 
 
 
const LEVEL_RENDERER_TARGET_CLASS: &str = "net/minecraft/class_761";

 
 
const BLOCK_STATE_BASE_TARGET_CLASS: &str = "net/minecraft/class_4970$class_4971";
const BLOCK_STATE_BASE_RENDER_SHAPE_METHOD: &str = "method_26217";
const BLOCK_STATE_BASE_RENDER_SHAPE_DESCRIPTOR: &str = "()Lnet/minecraft/class_2464;";
const XRAY_BRIDGE_CLASS: &str = "InferedXrayBridge";
const XRAY_BRIDGE_METHOD: &str = "decideRenderShape";
const XRAY_BRIDGE_DESCRIPTOR: &str = "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;";
const LEVEL_RENDERER_CULL_METHOD: &str = "method_32133";
const LEVEL_RENDERER_CULL_DESCRIPTOR: &str =
    "(Lnet/minecraft/class_243;Lorg/joml/Matrix4f;Lorg/joml/Matrix4f;)V";
const LEVEL_RENDERER_BRIDGE: &str = "InferedLevelRendererBridge";
const LEVEL_RENDERER_BRIDGE_METHOD: &str = "afterCullPrepare";
const LEVEL_RENDERER_BRIDGE_DESCRIPTOR: &str = "(Ljava/lang/Object;)V";

 
 
 
 
 
 
 
 
 
 
 
const GRAPH_TARGET_CLASS: &str = "net/minecraft/class_8679";
const GRAPH_INVALIDATE_METHOD: &str = "method_52817";
const GRAPH_INVALIDATE_DESCRIPTOR: &str = "()V";
const GRAPH_BRIDGE: &str = "InferedLevelRendererBridge";
const GRAPH_BRIDGE_METHOD: &str = "onGraphInvalidate";
const GRAPH_BRIDGE_DESCRIPTOR: &str = "(Ljava/lang/Object;)V";

 
 
 
 
 
 
 
 
 
 
 
const GRAPH_RUN_PARTIAL_METHOD: &str = "method_52835";
const GRAPH_RUN_PARTIAL_DESCRIPTOR: &str =
    "(ZLnet/minecraft/class_4604;Ljava/util/List;Lnet/minecraft/class_243;Lit/unimi/dsi/fastutil/longs/LongOpenHashSet;)V";
const GRAPH_GUARD_FLAG: &str = "PARTIAL_SUPPRESSED";

static mut CLASS_HOOK_REGISTERED: bool = false;

pub fn install() -> anyhow::Result<()> {
    log::info!("JVMTI Camera hook installation started");
    log::info!("CFLH diagnostic mode = {}", diagnostic_mode());

    let mut raw_vm: *mut RawJavaVM = std::ptr::null_mut();
    let mut count = 0;
    let status = unsafe { JNI_GetCreatedJavaVMs(&mut raw_vm, 1, &mut count) };
    if status != JNI_OK || count == 0 || raw_vm.is_null() {
        anyhow::bail!("JVMTI Camera hook: no live JVM (status={status}, count={count})");
    }
    log::info!("JVMTI live JVM acquired");
    log_runtime_properties()?;
     
     
    crate::camera_bridge::install()?;
    install_class_hook(raw_vm);
    Ok(())
}

fn install_class_hook(raw_vm: *mut RawJavaVM) {
    let mut statuses = [-1_i32; 7];
    let mut values = [-1_i32; 4];
    let mut pointers = [0_usize; 10];
    let result = unsafe {
        dark_jvmti_class_hook_diagnose(
            raw_vm,
            class_file_load_hook_callback,
            statuses.as_mut_ptr(),
            statuses.len() as i32,
            values.as_mut_ptr(),
            values.len() as i32,
            pointers.as_mut_ptr(),
            pointers.len() as i32,
        )
    };
    let operations = [
        "ClassHook GetEnv(JVMTI_VERSION_1_2)",
        "ClassHook GetCapabilities(before)",
        "ClassHook AddCapabilities(can_retransform_classes)",
        "ClassHook AddCapabilities(can_generate_all_class_hook_events)",
        "ClassHook GetCapabilities(after)",
        "ClassHook SetEventCallbacks",
        "ClassHook SetEventNotificationMode(ClassFileLoadHook)",
    ];
    for (operation, operation_status) in operations.iter().zip(statuses) {
        log_jvmti_status(operation, operation_status);
    }
    log::info!("ClassHook diagnostic return status={result}");
    log::info!(
        "ClassHook capabilities: can_retransform_classes={}, can_generate_all_class_hook_events={} (before), {} / {} (after)",
        capability_value(values[0]),
        capability_value(values[1]),
        capability_value(values[2]),
        capability_value(values[3])
    );
    if statuses[5] == 0 && statuses[6] == 0 {
        unsafe {
            CLASS_HOOK_REGISTERED = true;
        }
        log::info!("ClassFileLoadHook callback installed and enabled");
        run_retransform(CAMERA_TRANSFORM_PARAMS);
        run_retransform(CAMERA_COLLISION_TRANSFORM_PARAMS);
        run_retransform(LEVEL_RENDERER_TRANSFORM_PARAMS);
        run_retransform(SMOKE_RENDER_TRANSFORM_PARAMS);
        run_retransform(EXPLOSION_SUPPRESS_TRANSFORM_PARAMS);
        run_retransform(EXPLOSION_ALWAYS_VISIBLE_TRANSFORM_PARAMS);
        run_retransform(EXPLOSION_FORCED_PARTICLE_TRANSFORM_PARAMS);
        run_retransform(EXPLOSION_SOUND_TRANSFORM_PARAMS);
        run_retransform(TOTEM_POP_TRANSFORM_PARAMS);
         
         
         
        run_retransform(SWING_ECHO_TRANSFORM_PARAMS);
        run_retransform(HAND_OUTLINE_TRANSFORM_PARAMS);
        run_retransform(HAND_AURA_AFTER_TRANSFORM_PARAMS);
        run_retransform(ITEM_SCALE_TRANSFORM_PARAMS);
        run_retransform(SKY_COLOR_TRANSFORM_PARAMS);
        run_retransform(SKY_DISC_HIDE_TRANSFORM_PARAMS);
        run_retransform(SUN_MOON_STARS_HIDE_TRANSFORM_PARAMS);
        run_retransform(DARK_DISC_HIDE_TRANSFORM_PARAMS);
        run_retransform(SUNRISE_HIDE_TRANSFORM_PARAMS);
        run_retransform(CLOUDS_HIDE_TRANSFORM_PARAMS);
        run_retransform(SHIELD_TRANSFORM_PARAMS);
        run_retransform(FIRE_HIDE_TRANSFORM_PARAMS);
        run_retransform(CROSSHAIR_HIDE_TRANSFORM_PARAMS);
        run_retransform(TITLE_LOGO_HIDE_TRANSFORM_PARAMS);
        run_retransform(TITLE_LOGO_OFFSET_HIDE_TRANSFORM_PARAMS);
        run_retransform(NO_PARTICLES_TRANSFORM_PARAMS);
        if crate::camera_bridge::crystal_optimizer_bridge_defined() {
            run_retransform(CRYSTAL_OPTIMIZER_TRANSFORM_PARAMS);
        }
        if crate::camera_bridge::scaffold_bridge().is_some() {
            run_retransform(SCAFFOLD_INPUT_TRANSFORM_PARAMS);
            run_retransform(SCAFFOLD_EDGE_TRANSFORM_PARAMS);
        }
        run_retransform(GRAPH_TRANSFORM_PARAMS);
        run_retransform(GRAPH_RUN_PARTIAL_TRANSFORM_PARAMS);
        run_retransform(XRAY_RENDER_TRANSFORM_PARAMS);
        run_retransform(XRAY_OCCLUSION_TRANSFORM_PARAMS);
        run_retransform(XRAY_BATCHED_TRANSFORM_PARAMS);
        run_retransform(XRAY_LIQUID_TRANSFORM_PARAMS);
        run_retransform(XRAY_BLOCK_ENTITY_TRANSFORM_PARAMS);
    }
}

 
 
#[derive(Clone, Copy)]
pub(crate) struct PrependCall {
     
     
    pub(crate) arguments: &'static [PrependArg],
     
     
     
    pub(crate) destination: Option<u8>,
}

 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrependArg {
     
    Reference(u8),
     
    Float(u8),
     
    Int(u8),
}

impl PrependArg {
     
    pub(crate) const fn slot(self) -> u8 {
        match self {
            PrependArg::Reference(slot) | PrependArg::Float(slot) | PrependArg::Int(slot) => slot,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TransformParams {
    pub(crate) target_class: &'static str,
    pub(crate) method: &'static str,
    pub(crate) descriptor: &'static str,
    pub(crate) bridge_class: &'static str,
    pub(crate) bridge_method: &'static str,
    pub(crate) bridge_descriptor: &'static str,
    pub(crate) class_type: MinecraftClassType,
    pub(crate) log_label: &'static str,
     
     
     
     
     
     
    pub(crate) replace_entire_body: bool,
     
     
     
    pub(crate) replace_with_bridge_return: bool,
     
     
     
     
     
    pub(crate) guard_flag: Option<&'static str>,
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
     
    pub(crate) prepend_object_call: Option<PrependCall>,
}

const CAMERA_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: CAMERA_TARGET_CLASS,
    method: CAMERA_SETUP_METHOD,
    descriptor: CAMERA_SETUP_DESCRIPTOR,
    bridge_class: BRIDGE_CLASS,
    bridge_method: BRIDGE_METHOD,
    bridge_descriptor: BRIDGE_DESCRIPTOR,
    class_type: MinecraftClassType::Camera,
    log_label: "Camera",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: None,
    prepend_object_call: None,
};

 
const CAMERA_COLLISION_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: CAMERA_TARGET_CLASS,
    method: "method_19318",
    descriptor: "(F)F",
    bridge_class: "InferedFreelookBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::Camera,
    log_label: "CameraCollision",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("NO_CLIP"),
    prepend_object_call: None,
};

const LEVEL_RENDERER_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: LEVEL_RENDERER_TARGET_CLASS,
    method: LEVEL_RENDERER_CULL_METHOD,
    descriptor: LEVEL_RENDERER_CULL_DESCRIPTOR,
    bridge_class: LEVEL_RENDERER_BRIDGE,
    bridge_method: LEVEL_RENDERER_BRIDGE_METHOD,
    bridge_descriptor: LEVEL_RENDERER_BRIDGE_DESCRIPTOR,
    class_type: MinecraftClassType::LevelRenderer,
    log_label: "LevelRenderer",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: None,
    prepend_object_call: None,
};

pub(crate) const SMOKE_RENDER_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: LEVEL_RENDERER_TARGET_CLASS,
    method: "method_22710",
    descriptor: "(Lnet/minecraft/class_9922;Lnet/minecraft/class_9779;ZLnet/minecraft/class_4184;Lnet/minecraft/class_757;Lorg/joml/Matrix4f;Lorg/joml/Matrix4f;)V",
    bridge_class: "InferedSmokeBridge",
    bridge_method: "afterWorldRender",
    bridge_descriptor: "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)V",
    class_type: MinecraftClassType::LevelRenderer,
    log_label: "SmokeWorldRender",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: None,
    prepend_object_call: None,
};

 
 
 
 
 
 
pub(crate) const EXPLOSION_SUPPRESS_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_638",
    method: "method_8406",
    descriptor: "(Lnet/minecraft/class_2394;DDDDDD)V",
    bridge_class: "InferedExplosionBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::Level,
    log_label: "ExplosionParticleSuppress",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("SUPPRESS_PARTICLES"),
    prepend_object_call: None,
};

 
 
pub(crate) const EXPLOSION_ALWAYS_VISIBLE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_638",
    method: "method_8494",
    descriptor: "(Lnet/minecraft/class_2394;DDDDDD)V",
    bridge_class: "InferedExplosionBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::Level,
    log_label: "ExplosionAlwaysVisibleSuppress",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("SUPPRESS_PARTICLES"),
    prepend_object_call: None,
};

 
 
pub(crate) const EXPLOSION_FORCED_PARTICLE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_638",
    method: "method_17452",
    descriptor: "(Lnet/minecraft/class_2394;ZDDDDDD)V",
    bridge_class: "InferedExplosionBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::Level,
    log_label: "ExplosionForcedSuppress",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("SUPPRESS_PARTICLES"),
    prepend_object_call: None,
};

 
 
 
pub(crate) const EXPLOSION_SOUND_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_638",
    method: "method_8486",
    descriptor: "(DDDLnet/minecraft/class_3414;Lnet/minecraft/class_3419;FFZ)V",
    bridge_class: "InferedExplosionBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::Level,
    log_label: "ExplosionSoundSuppress",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("SUPPRESS_SOUNDS"),
    prepend_object_call: None,
};

 
 
 
 
pub(crate) const FIRE_HIDE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_4603",
    method: "method_23070",
    descriptor: "(Lnet/minecraft/class_4587;Lnet/minecraft/class_4597;)V",
    bridge_class: "InferedFireBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::ScreenEffectRenderer,
    log_label: "InvisFire",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("SUPPRESS_FIRE"),
    prepend_object_call: None,
};

 
 
 
 
pub(crate) const CROSSHAIR_HIDE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_329",
    method: "method_1736",
    descriptor: "(Lnet/minecraft/class_332;Lnet/minecraft/class_9779;)V",
    bridge_class: "InferedCrosshairBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::Gui,
    log_label: "CrosshairHide",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("HIDE_VANILLA"),
    prepend_object_call: None,
};

 
 
 
 
 
pub(crate) const TITLE_LOGO_HIDE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_8020",
    method: "method_48209",
    descriptor: "(Lnet/minecraft/class_332;IF)V",
    bridge_class: "InferedTitleLogoBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::LogoRenderer,
    log_label: "TitleLogoHide",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("HIDE_VANILLA"),
    prepend_object_call: None,
};

 
 
pub(crate) const TITLE_LOGO_OFFSET_HIDE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_8020",
    method: "method_48210",
    descriptor: "(Lnet/minecraft/class_332;IFI)V",
    bridge_class: "InferedTitleLogoBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::LogoRenderer,
    log_label: "TitleLogoOffsetHide",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("HIDE_VANILLA"),
    prepend_object_call: None,
};

 
 
 
 
 
pub(crate) const NO_PARTICLES_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_702",
    method: "method_3058",
    descriptor: "(Lnet/minecraft/class_703;)V",
    bridge_class: "InferedNoParticlesBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::ParticleEngine,
    log_label: "NoParticles",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("HIDE_PARTICLES"),
    prepend_object_call: None,
};

 
const CRYSTAL_OPTIMIZER_TARGET_CLASS: &str = "net/minecraft/class_636";

 
 
 
const CRYSTAL_OPTIMIZER_METHOD: &str = "method_2918";
const CRYSTAL_OPTIMIZER_DESCRIPTOR: &str =
    "(Lnet/minecraft/class_1657;Lnet/minecraft/class_1297;)V";

 
 
const CRYSTAL_OPTIMIZER_LOCALS: &[PrependArg] = &[PrependArg::Reference(2)];

 
 
 
 
 
pub(crate) const CRYSTAL_OPTIMIZER_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: CRYSTAL_OPTIMIZER_TARGET_CLASS,
    method: CRYSTAL_OPTIMIZER_METHOD,
    descriptor: CRYSTAL_OPTIMIZER_DESCRIPTOR,
    bridge_class: "InferedCrystalOptimizerBridge",
    bridge_method: "beforeAttack",
    bridge_descriptor: "(Ljava/lang/Object;)V",
    class_type: MinecraftClassType::MultiPlayerGameMode,
    log_label: "CrystalOptimizer",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: None,
    prepend_object_call: Some(PrependCall {
        arguments: CRYSTAL_OPTIMIZER_LOCALS,
        destination: None,
    }),
};

 
 
 
 
 
 
 
 
 
 
 
 
 
 
const SCAFFOLD_INPUT_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_310",
    method: "method_1574",
    descriptor: "()V",
    bridge_class: "InferedScaffoldBridge",
    bridge_method: "runInputPhase",
    bridge_descriptor: "(Ljava/lang/Object;)V",
    class_type: MinecraftClassType::Minecraft,
    log_label: "ScaffoldInputPhase",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: None,
    prepend_object_call: None,
};

 
 
 
 
 
const SCAFFOLD_EDGE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_1657",
    method: "method_18796",
    descriptor: "(Lnet/minecraft/class_243;Lnet/minecraft/class_1313;)Lnet/minecraft/class_243;",
    bridge_class: "InferedScaffoldBridge",
    bridge_method: "safeWalk",
    bridge_descriptor: "(Ljava/lang/Object;)Z",
    class_type: MinecraftClassType::Player,
    log_label: "ScaffoldEdgeSafety",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: None,
    prepend_object_call: None,
};

const TOTEM_POP_TARGET_CLASS: &str = "net/minecraft/class_757";
const TOTEM_POP_METHOD: &str = "method_3171";
const TOTEM_POP_DESCRIPTOR: &str = "(Lnet/minecraft/class_332;F)V";

 
 
pub(crate) const TOTEM_POP_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: TOTEM_POP_TARGET_CLASS,
    method: TOTEM_POP_METHOD,
    descriptor: TOTEM_POP_DESCRIPTOR,
    bridge_class: "InferedTotemPopBridge",
    bridge_method: "scale",
    bridge_descriptor: "(Ljava/lang/Object;FFF)V",
    class_type: MinecraftClassType::GameRenderer,
    log_label: "TotemPop",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: None,
    prepend_object_call: None,
};

const GRAPH_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: GRAPH_TARGET_CLASS,
    method: GRAPH_INVALIDATE_METHOD,
    descriptor: GRAPH_INVALIDATE_DESCRIPTOR,
    bridge_class: GRAPH_BRIDGE,
    bridge_method: GRAPH_BRIDGE_METHOD,
    bridge_descriptor: GRAPH_BRIDGE_DESCRIPTOR,
    class_type: MinecraftClassType::SectionOcclusionGraph,
    log_label: "SectionOcclusionGraph",
    replace_entire_body: true,
    replace_with_bridge_return: false,
    guard_flag: None,
    prepend_object_call: None,
};

const GRAPH_RUN_PARTIAL_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: GRAPH_TARGET_CLASS,
    method: GRAPH_RUN_PARTIAL_METHOD,
    descriptor: GRAPH_RUN_PARTIAL_DESCRIPTOR,
    bridge_class: GRAPH_BRIDGE,
    bridge_method: GRAPH_BRIDGE_METHOD,
    bridge_descriptor: GRAPH_BRIDGE_DESCRIPTOR,
    class_type: MinecraftClassType::SectionOcclusionGraph,
    log_label: "SectionOcclusionGraphPartial",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some(GRAPH_GUARD_FLAG),
    prepend_object_call: None,
};

 
 
const SHIELD_ITEM_LOCAL: u8 = 6;

 
 
const SHIELD_LOCALS: &[PrependArg] = &[PrependArg::Reference(4), PrependArg::Reference(6)];

 
 
 
 
pub(crate) const SHIELD_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: ITEM_SCALE_TARGET_CLASS,
    method: ITEM_SCALE_METHOD,
    descriptor: ITEM_SCALE_DESCRIPTOR,
    bridge_class: "InferedShieldBridge",
    bridge_method: "hide",
    bridge_descriptor: "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
    class_type: MinecraftClassType::ItemInHandRenderer,
    log_label: "InvisibleShield",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: None,
    prepend_object_call: Some(PrependCall {
        arguments: SHIELD_LOCALS,
        destination: Some(SHIELD_ITEM_LOCAL),
    }),
};

 
const ITEM_SCALE_TARGET_CLASS: &str = "net/minecraft/class_759";

 
 
 
 
 
 
 
 
const ITEM_SCALE_METHOD: &str = "method_3228";
const ITEM_SCALE_DESCRIPTOR: &str = "(Lnet/minecraft/class_742;FFLnet/minecraft/class_1268;FLnet/minecraft/class_1799;FLnet/minecraft/class_4587;Lnet/minecraft/class_4597;I)V";

 
 
const ITEM_SCALE_LOCALS: &[PrependArg] = &[PrependArg::Reference(4), PrependArg::Reference(8)];

 
 
 
 
 
pub(crate) const ITEM_SCALE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: ITEM_SCALE_TARGET_CLASS,
    method: ITEM_SCALE_METHOD,
    descriptor: ITEM_SCALE_DESCRIPTOR,
    bridge_class: "InferedItemScaleBridge",
    bridge_method: "apply",
    bridge_descriptor: "(Ljava/lang/Object;Ljava/lang/Object;)V",
    class_type: MinecraftClassType::ItemInHandRenderer,
    log_label: "ItemScale",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: None,
    prepend_object_call: Some(PrependCall {
        arguments: ITEM_SCALE_LOCALS,
        destination: None,
    }),
};

 
 
const SWING_ECHO_LOCALS: &[PrependArg] = &[
    PrependArg::Reference(0),
    PrependArg::Reference(1),
    PrependArg::Float(2),
    PrependArg::Float(3),
    PrependArg::Reference(4),
    PrependArg::Float(5),
    PrependArg::Reference(6),
    PrependArg::Float(7),
    PrependArg::Reference(8),
    PrependArg::Reference(9),
    PrependArg::Int(10),
];

const SWING_ECHO_BRIDGE_DESCRIPTOR: &str = "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)V";

 
 
 
pub(crate) const SWING_ECHO_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: ITEM_SCALE_TARGET_CLASS,
    method: ITEM_SCALE_METHOD,
    descriptor: ITEM_SCALE_DESCRIPTOR,
    bridge_class: "InferedSwingEchoBridge",
    bridge_method: "apply",
    bridge_descriptor: SWING_ECHO_BRIDGE_DESCRIPTOR,
    class_type: MinecraftClassType::ItemInHandRenderer,
    log_label: "SwingEcho",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: None,
    prepend_object_call: Some(PrependCall {
        arguments: SWING_ECHO_LOCALS,
        destination: None,
    }),
};

 
 
 
pub(crate) const HAND_OUTLINE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: ITEM_SCALE_TARGET_CLASS,
    method: ITEM_SCALE_METHOD,
    descriptor: ITEM_SCALE_DESCRIPTOR,
    bridge_class: "InferedHandOutlineBridge",
    bridge_method: "apply",
    bridge_descriptor: SWING_ECHO_BRIDGE_DESCRIPTOR,
    class_type: MinecraftClassType::ItemInHandRenderer,
    log_label: "HandOutline",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: None,
    prepend_object_call: Some(PrependCall {
        arguments: SWING_ECHO_LOCALS,
        destination: None,
    }),
};

 
 
 
 
 
 
const HANDS_RENDERED_METHOD: &str = "method_22976";
const HANDS_RENDERED_DESCRIPTOR: &str =
    "(FLnet/minecraft/class_4587;Lnet/minecraft/class_4597$class_4598;Lnet/minecraft/class_746;I)V";

 
 
 
 
 
 
 
 
 
pub(crate) const HAND_AURA_AFTER_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: ITEM_SCALE_TARGET_CLASS,
    method: HANDS_RENDERED_METHOD,
    descriptor: HANDS_RENDERED_DESCRIPTOR,
    bridge_class: "InferedHandAuraBridge",
    bridge_method: "afterHands",
    bridge_descriptor: "(Ljava/lang/Object;)V",
    class_type: MinecraftClassType::ItemInHandRenderer,
    log_label: "HandAuraAfter",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: None,
    prepend_object_call: None,
};

pub(crate) const XRAY_RENDER_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: BLOCK_STATE_BASE_TARGET_CLASS,
    method: BLOCK_STATE_BASE_RENDER_SHAPE_METHOD,
    descriptor: BLOCK_STATE_BASE_RENDER_SHAPE_DESCRIPTOR,
    bridge_class: XRAY_BRIDGE_CLASS,
    bridge_method: XRAY_BRIDGE_METHOD,
    bridge_descriptor: XRAY_BRIDGE_DESCRIPTOR,
    class_type: MinecraftClassType::BlockStateBase,
    log_label: "XrayRenderShape",
    replace_entire_body: false,
    replace_with_bridge_return: true,
    guard_flag: None,
    prepend_object_call: None,
};

const XRAY_OCCLUSION_TRANSFORM_PARAMS: TransformParams = TransformParams {
    method: "method_26225",
    descriptor: "()Z",
    bridge_method: "decideOcclusion",
    bridge_descriptor: "(Z)Z",
    log_label: "XrayOcclusion",
    ..XRAY_RENDER_TRANSFORM_PARAMS
};

const XRAY_BATCHED_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_776",
    method: "method_3355",
    descriptor: "(Lnet/minecraft/class_2680;Lnet/minecraft/class_2338;Lnet/minecraft/class_1920;Lnet/minecraft/class_4587;Lnet/minecraft/class_4588;ZLnet/minecraft/class_5819;)V",
    class_type: MinecraftClassType::BlockRenderDispatcher,
    log_label: "XrayBatched",
    bridge_method: "shouldHide",
    bridge_descriptor: "(Ljava/lang/Object;)Z",
    replace_with_bridge_return: false,
    guard_flag: Some("ACTIVE"),
    ..XRAY_RENDER_TRANSFORM_PARAMS
};

const XRAY_LIQUID_TRANSFORM_PARAMS: TransformParams = TransformParams {
    method: "method_3352",
    descriptor: "(Lnet/minecraft/class_2338;Lnet/minecraft/class_1920;Lnet/minecraft/class_4588;Lnet/minecraft/class_2680;Lnet/minecraft/class_3610;)V",
    log_label: "XrayLiquid",
    ..XRAY_BATCHED_TRANSFORM_PARAMS
};

const XRAY_BLOCK_ENTITY_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_824",
    method: "method_3555",
    descriptor:
        "(Lnet/minecraft/class_2586;FLnet/minecraft/class_4587;Lnet/minecraft/class_4597;)V",
    class_type: MinecraftClassType::BlockEntityRenderDispatcher,
    log_label: "XrayBlockEntity",
    bridge_method: "",
    bridge_descriptor: "",
    ..XRAY_BATCHED_TRANSFORM_PARAMS
};

 
 
 
 
 
pub(crate) const SKY_COLOR_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: "net/minecraft/class_638",
    method: "method_23777",
    descriptor: "(Lnet/minecraft/class_243;F)I",
    bridge_class: "InferedSkyBridge",
    bridge_method: "tint",
    bridge_descriptor: "(ILjava/lang/Object;)I",
    class_type: MinecraftClassType::Level,
    log_label: "CustomSky",
    replace_entire_body: false,
    replace_with_bridge_return: true,
    guard_flag: None,
    prepend_object_call: None,
};

 
 
 
 
 
 
 
pub(crate) const SKY_RENDERER_TARGET_CLASS: &str = "net/minecraft/class_9975";

 
 
 
 
 
 
 
 
 
pub(crate) const SKY_DISC_HIDE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: SKY_RENDERER_TARGET_CLASS,
    method: "method_62302",
    descriptor: "(FFF)V",
    bridge_class: "InferedSkyBridge",
    bridge_method: "drawSky",
    bridge_descriptor: "()V",
    class_type: MinecraftClassType::SkyRenderer,
    log_label: "SkyDiscHide",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("HIDE_VANILLA"),
    prepend_object_call: Some(PrependCall {
        arguments: &[],
        destination: None,
    }),
};

 
 
 
pub(crate) const DARK_DISC_HIDE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: SKY_RENDERER_TARGET_CLASS,
    method: "method_62305",
    descriptor: "(Lnet/minecraft/class_4587;)V",
    bridge_class: "InferedSkyBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::SkyRenderer,
    log_label: "SkyDarkDiscHide",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("HIDE_VANILLA"),
    prepend_object_call: None,
};

 
pub(crate) const SUNRISE_HIDE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: SKY_RENDERER_TARGET_CLASS,
    method: "method_62306",
    descriptor: "(Lnet/minecraft/class_4587;Lnet/minecraft/class_4597$class_4598;FI)V",
    bridge_class: "InferedSkyBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::SkyRenderer,
    log_label: "SkySunriseHide",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("HIDE_VANILLA"),
    prepend_object_call: None,
};

 
 
 
pub(crate) const CLOUDS_HIDE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: LEVEL_RENDERER_TARGET_CLASS,
    method: "method_62204",
    descriptor: "(Lnet/minecraft/class_9909;Lorg/joml/Matrix4f;Lorg/joml/Matrix4f;Lnet/minecraft/class_4063;Lnet/minecraft/class_243;FIF)V",
    bridge_class: "InferedSkyBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::LevelRenderer,
    log_label: "SkyCloudsHide",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("HIDE_VANILLA"),
    prepend_object_call: None,
};

 
 
pub(crate) const SUN_MOON_STARS_HIDE_TRANSFORM_PARAMS: TransformParams = TransformParams {
    target_class: SKY_RENDERER_TARGET_CLASS,
    method: "method_62307",
    descriptor: "(Lnet/minecraft/class_4587;Lnet/minecraft/class_4597$class_4598;FIFFLnet/minecraft/class_9958;)V",
    bridge_class: "InferedSkyBridge",
    bridge_method: "",
    bridge_descriptor: "",
    class_type: MinecraftClassType::SkyRenderer,
    log_label: "SunMoonStarsHide",
    replace_entire_body: false,
    replace_with_bridge_return: false,
    guard_flag: Some("HIDE_VANILLA"),
    prepend_object_call: None,
};

 
const JVMTI_OK: i32 = 0;

 
 
 
 
static INSTALLED_TRANSFORMS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
static PREPARED_TRANSFORMS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

 
 
 
pub(crate) fn sky_drawn_in_sky_pass() -> bool {
    transform_installed(SKY_DISC_HIDE_TRANSFORM_PARAMS.log_label)
}

pub(crate) fn transform_installed(label: &str) -> bool {
    INSTALLED_TRANSFORMS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .contains(&label)
}

fn mark_transform_installed(label: &'static str) {
    let mut installed = INSTALLED_TRANSFORMS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if !installed.contains(&label) {
        installed.push(label);
    }
}

fn mark_transform_rejected(label: &'static str) {
    INSTALLED_TRANSFORMS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .retain(|entry| *entry != label);
    log::error!(
        "{label} transform is NOT installed; anything relying on it will fall back to vanilla behaviour"
    );
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
pub(crate) fn crystal_input_hook_installed() -> bool {
    crate::camera_bridge::crystal_bridge().is_some()
        && (transform_installed(SCAFFOLD_INPUT_TRANSFORM_PARAMS.log_label)
            || crate::camera_bridge::crystal_input_phase_ran())
}

 
 
 
 
 
 
 
 
pub(crate) fn camera_frame_hook_installed() -> bool {
    crate::camera_bridge::camera_frame_ran()
        || transform_installed(CAMERA_TRANSFORM_PARAMS.log_label)
}

pub(crate) fn graph_hooks_installed() -> bool {
    transform_installed(GRAPH_TRANSFORM_PARAMS.log_label)
        && transform_installed(GRAPH_RUN_PARTIAL_TRANSFORM_PARAMS.log_label)
}

fn run_retransform(params: TransformParams) {
    let label = params.log_label;
    log::info!(
        "JVMTI {label} retransformation target={}",
        params.target_class
    );

    let mut env = match mapping().get_env() {
        Ok(env) => env,
        Err(error) => {
            log::error!("JNI GetEnv for retransformation target failed: {error}");
            return;
        }
    };
    let runtime_name = match mapping().runtime_class_name(params.class_type) {
        Ok(name) => name,
        Err(error) => {
            log::error!("{label} runtime class-name resolution failed: {error}");
            return;
        }
    };
    if runtime_name != params.target_class {
        log::error!(
            "{label} runtime class resolved unexpectedly as {runtime_name}, expected {}",
            params.target_class
        );
        return;
    }
    let target = match mapping().resolve_class(&mut env, &runtime_name) {
        Ok(class) => class,
        Err(error) => {
            log::error!("JNI resolve_class({runtime_name}) failed: {error}");
            let _ = env.exception_clear();
            return;
        }
    };
    log::info!("{label} class resolved successfully: {runtime_name}");
    log::info!(
        "{label} retransform target: runtime_name={runtime_name}, jni_class=0x{:x}",
        target.as_raw() as usize
    );
    log_current_thread(&mut env);

    let mut statuses = [-1_i32; 2];
    let mut values = [-1_i32; 3];
    let mut pointers = [0_usize; 4];
    PREPARED_TRANSFORMS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .retain(|entry| *entry != label);
    let result = unsafe {
        dark_jvmti_retransform_diagnose(
            target.as_raw(),
            statuses.as_mut_ptr(),
            statuses.len() as i32,
            values.as_mut_ptr(),
            values.len() as i32,
            pointers.as_mut_ptr(),
            pointers.len() as i32,
        )
    };
    log_jvmti_status(&format!("IsModifiableClass({label})"), statuses[0]);
    log::info!(
        "JVMTI IsModifiableClass({label}): result={}, capabilities before retransform: can_retransform_classes={}, can_generate_all_class_hook_events={}",
        capability_value(values[0]),
        capability_value(values[1]),
        capability_value(values[2])
    );
    log_jvmti_status(&format!("RetransformClasses({label})"), statuses[1]);
    log::info!(
        "JVMTI RetransformClasses({label}) return status={result} ({})",
        jvmti_status_name(result)
    );
    if result != JVMTI_OK {
         
         
         
         
         
         
         
        log::error!(
            "{label} RE-TRANSFORM REJECTED by the VM ({}): {}",
            jvmti_status_name(result),
            describe_pending_exception(&mut env)
        );
        mark_transform_rejected(label);
        return;
    }
    if !PREPARED_TRANSFORMS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .contains(&label)
    {
        mark_transform_rejected(label);
        return;
    }
    mark_transform_installed(label);
    log::info!(
        "{label} ClassFileLoadHook completed; replacement bytes splice {}.{} {}",
        params.target_class,
        params.method,
        params.descriptor
    );
}

 
 
fn describe_pending_exception(env: &mut jni::JNIEnv) -> String {
    let Ok(throwable) = env.exception_occurred() else {
        return "<none pending>".to_string();
    };
    let _ = env.exception_clear();
    env.call_method(&throwable, "toString", "()Ljava/lang/String;", &[])
        .and_then(|value| value.l())
        .and_then(|string| {
            env.get_string(&JString::from(string))
                .map(|value| value.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|_| "<pending exception could not be read>".to_string())
}

 
 
fn log_current_thread(env: &mut jni::JNIEnv) {
    let (name, id) = match env
        .find_class("java/lang/Thread")
        .and_then(|class| {
            env.call_static_method(&class, "currentThread", "()Ljava/lang/Thread;", &[])
        })
        .and_then(|value| value.l())
    {
        Ok(thread) => {
            let name = env
                .call_method(&thread, "getName", "()Ljava/lang/String;", &[])
                .and_then(|value| value.l())
                .and_then(|string| {
                    env.get_string(&JString::from(string))
                        .map(|value| value.to_string_lossy().into_owned())
                })
                .ok();
            let id = env
                .call_method(&thread, "getId", "()J", &[])
                .and_then(|value| value.j())
                .ok();
            (name, id)
        }
        Err(_) => (None, None),
    };
    log::info!(
        "retransform caller thread: name={}, id={}",
        name.as_deref().unwrap_or("<unknown>"),
        id.map(|value| value.to_string())
            .unwrap_or_else(|| "<unknown>".to_owned())
    );
}

 
 
 
 
 
 
 
fn base_transforms() -> &'static [TransformParams] {
    &[
        CAMERA_TRANSFORM_PARAMS,
        CAMERA_COLLISION_TRANSFORM_PARAMS,
        LEVEL_RENDERER_TRANSFORM_PARAMS,
        SMOKE_RENDER_TRANSFORM_PARAMS,
        EXPLOSION_SUPPRESS_TRANSFORM_PARAMS,
        EXPLOSION_ALWAYS_VISIBLE_TRANSFORM_PARAMS,
        EXPLOSION_FORCED_PARTICLE_TRANSFORM_PARAMS,
        EXPLOSION_SOUND_TRANSFORM_PARAMS,
        FIRE_HIDE_TRANSFORM_PARAMS,
        CROSSHAIR_HIDE_TRANSFORM_PARAMS,
        TITLE_LOGO_HIDE_TRANSFORM_PARAMS,
        TITLE_LOGO_OFFSET_HIDE_TRANSFORM_PARAMS,
        NO_PARTICLES_TRANSFORM_PARAMS,
        TOTEM_POP_TRANSFORM_PARAMS,
        SWING_ECHO_TRANSFORM_PARAMS,
        HAND_OUTLINE_TRANSFORM_PARAMS,
        HAND_AURA_AFTER_TRANSFORM_PARAMS,
         
        ITEM_SCALE_TRANSFORM_PARAMS,
        SKY_COLOR_TRANSFORM_PARAMS,
        SKY_DISC_HIDE_TRANSFORM_PARAMS,
        SUN_MOON_STARS_HIDE_TRANSFORM_PARAMS,
        DARK_DISC_HIDE_TRANSFORM_PARAMS,
        SUNRISE_HIDE_TRANSFORM_PARAMS,
        CLOUDS_HIDE_TRANSFORM_PARAMS,
        SHIELD_TRANSFORM_PARAMS,
        GRAPH_TRANSFORM_PARAMS,
        GRAPH_RUN_PARTIAL_TRANSFORM_PARAMS,
        XRAY_RENDER_TRANSFORM_PARAMS,
        XRAY_OCCLUSION_TRANSFORM_PARAMS,
        XRAY_BATCHED_TRANSFORM_PARAMS,
        XRAY_LIQUID_TRANSFORM_PARAMS,
        XRAY_BLOCK_ENTITY_TRANSFORM_PARAMS,
    ]
}

extern "C" fn class_file_load_hook_callback(
    class_name: *const c_char,
    class_name_length: jint,
    class_being_redefined_nonnull: u8,
    class_data_len: jint,
    class_data: *const u8,
    replacement: *mut *mut u8,
    replacement_len: *mut jint,
) -> i32 {
    unsafe {
        if !replacement.is_null() {
            *replacement = std::ptr::null_mut();
        }
        if !replacement_len.is_null() {
            *replacement_len = 0;
        }
    }
    let name = if class_name.is_null() || class_name_length <= 0 {
        "<unnamed>".to_owned()
    } else {
        let bytes = unsafe {
            std::slice::from_raw_parts(class_name as *const u8, class_name_length as usize)
        };
        String::from_utf8_lossy(bytes).into_owned()
    };
    if name.contains("class_759") {
        log::info!(
            "ClassFileLoadHook raw item-pass event: name={name}, redefinition={}, class_data_len={class_data_len}",
            class_being_redefined_nonnull != 0
        );
    }
    let mut transforms: Vec<TransformParams> = base_transforms().to_vec();
     
     
     
     
     
    if crate::camera_bridge::scaffold_bridge().is_some() {
        transforms.push(SCAFFOLD_INPUT_TRANSFORM_PARAMS);
        transforms.push(SCAFFOLD_EDGE_TRANSFORM_PARAMS);
    }
     
     
    if crate::camera_bridge::crystal_optimizer_bridge_defined() {
        transforms.push(CRYSTAL_OPTIMIZER_TRANSFORM_PARAMS);
    }
    let params: Vec<TransformParams> = transforms
        .into_iter()
        .filter(|params| params.target_class == name)
        .collect();
    if params.is_empty() {
        return 0;
    };
    if class_data.is_null() || class_data_len <= 0 {
        log::warn!(
            "{} class bytes unavailable for transformation",
            params[0].log_label
        );
        return 0;
    }

    let event_kind = if class_being_redefined_nonnull == 0 {
        "initial load"
    } else {
        "redefinition/retransformation"
    };
    log::info!(
        "ClassFileLoadHook callback fired: class_name={}, redefinition={}, class_data_len={}, event={}",
        name,
        class_being_redefined_nonnull != 0,
        class_data_len,
        event_kind
    );

    let transform_result = catch_unwind(AssertUnwindSafe(|| {
        transform_class(
            unsafe { std::slice::from_raw_parts(class_data, class_data_len as usize) },
            &params,
        )
    }));
    match transform_result {
        Ok(Ok(Some(bytes))) => {
            let mut prepared = PREPARED_TRANSFORMS
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            for param in &params {
                if !prepared.contains(&param.log_label) {
                    prepared.push(param.log_label);
                }
            }
            drop(prepared);
            let len = bytes.len();
            let boxed = bytes.into_boxed_slice();
            let ptr = Box::into_raw(boxed) as *mut u8;
            unsafe {
                if !replacement.is_null() {
                    *replacement = ptr;
                }
                if !replacement_len.is_null() {
                    *replacement_len = len as jint;
                }
            }
            log::info!(
                "{} replacement bytes supplied: len={len}",
                params[0].log_label
            );
            1
        }
        Ok(Ok(None)) => 0,
        Ok(Err(error)) => {
            log::warn!(
                "{} transformation skipped; original class preserved: {error}",
                params[0].log_label
            );
            0
        }
        Err(_) => {
            log::error!(
                "{} transformation panicked; original class preserved",
                params[0].log_label
            );
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn dark_free_transformed_bytes(ptr: *mut u8, len: jint) {
    if ptr.is_null() || len <= 0 {
        return;
    }
    unsafe {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            ptr,
            len as usize,
        )));
    }
}

 
 
 
 
 
fn diagnostic_mode() -> String {
    let path = config::base_dir().join("cflh_mode.txt");
    let value = std::fs::read_to_string(&path)
        .map(|contents| contents.trim().to_ascii_lowercase())
        .unwrap_or_default();
    match value.as_str() {
        "freeze" | "roundtrip" | "tail3" | "dump" | "pop" => value,
        _ => "normal".to_owned(),
    }
}

fn is_bridge_method(
    pool: &ristretto_classfile::ConstantPool<'_>,
    index: u16,
    params: &TransformParams,
) -> bool {
    let Some(Constant::MethodRef {
        class_index,
        name_and_type_index,
    }) = pool.get(index)
    else {
        return false;
    };
    let Some(Constant::Class(name_index)) = pool.get(*class_index) else {
        return false;
    };
    let Some(Constant::NameAndType {
        name_index: method_name_index,
        descriptor_index,
    }) = pool.get(*name_and_type_index)
    else {
        return false;
    };
    let owner = pool
        .try_get_utf8(*name_index)
        .map(|value| value.to_string())
        .ok();
    let name = pool
        .try_get_utf8(*method_name_index)
        .map(|value| value.to_string())
        .ok();
    let descriptor = pool
        .try_get_utf8(*descriptor_index)
        .map(|value| value.to_string())
        .ok();
    owner.as_deref() == Some(params.bridge_class)
        && name.as_deref() == Some(params.bridge_method)
        && descriptor.as_deref() == Some(params.bridge_descriptor)
}

fn terminal_stackframe_offset(frames: &[StackFrame]) -> Option<usize> {
    let mut offset = -1i64;
    for frame in frames {
        offset += i64::from(frame.offset_delta()) + 1;
    }
    (offset >= 0).then_some(offset as usize)
}

 
 
 
 
 
 
 
fn patch_terminal_stackmap(
    class_file: &mut ClassFile<'_>,
    method_index: usize,
    terminal_instruction: usize,
    mode: &str,
    require_terminal_frame: bool,
) -> anyhow::Result<bool> {
    for attribute in &mut class_file.methods[method_index].attributes {
        let Attribute::Code { attributes, .. } = attribute else {
            continue;
        };
        for code_attribute in attributes.iter_mut() {
            let Attribute::StackMapTable { frames, .. } = code_attribute else {
                continue;
            };
            if frames.is_empty() {
                continue;
            }
            let last_offset = terminal_stackframe_offset(frames)
                .ok_or_else(|| anyhow::anyhow!("cannot compute terminal frame offset"))?;
            if last_offset != terminal_instruction {
                 
                 
                 
                 
                 
                if !require_terminal_frame {
                    return Ok(false);
                }
                anyhow::bail!(
                    "CFLH diagnostic mode={mode}: terminal StackMapTable frame@{last_offset} does not match terminal instruction `{terminal_instruction}`"
                );
            }
            let delta = frames
                .last()
                .map(StackFrame::offset_delta)
                .unwrap_or_default();
            frames.pop();
            frames.push(StackFrame::FullFrame {
                frame_type: 255,
                offset_delta: delta,
                locals: vec![VerificationType::Object {
                    cpool_index: class_file.this_class,
                }],
                stack: vec![],
            });
            log::info!(
                "CFLH diagnostic mode={mode}: Camera.setup StackMapTable terminal frame rewritten @instruction {last_offset}: locals=[this #{}]",
                class_file.this_class
            );
            return Ok(true);
        }
    }
    Ok(false)
}

 
 
 
 
fn preserve_smoke_argument_frames(
    class_file: &mut ClassFile<'_>,
    method_index: usize,
    params: &TransformParams,
) -> anyhow::Result<()> {
    let camera = if params.target_class == LEVEL_RENDERER_TARGET_CLASS {
        "net/minecraft/class_4184"
    } else {
        "java/lang/Object"
    };
    let matrix = if params.target_class == LEVEL_RENDERER_TARGET_CLASS {
        "org/joml/Matrix4f"
    } else {
        "java/lang/Object"
    };
    let camera = class_file.constant_pool.add_class(camera)?;
    let matrix = class_file.constant_pool.add_class(matrix)?;
    for attribute in &mut class_file.methods[method_index].attributes {
        if let Attribute::Code {
            code, attributes, ..
        } = attribute
        {
             
            for instruction in code.iter() {
                if matches!(
                    instruction,
                    Instruction::Astore(4 | 6 | 7)
                        | Instruction::Istore(4 | 6 | 7)
                        | Instruction::Fstore(4 | 6 | 7)
                        | Instruction::Lstore(4 | 6 | 7)
                        | Instruction::Dstore(4 | 6 | 7)
                ) {
                    anyhow::bail!("Smoke render argument slot overwritten by target bytecode");
                }
            }
            for attribute in attributes {
                if let Attribute::StackMapTable { frames, .. } = attribute {
                    for frame in frames {
                        if let StackFrame::FullFrame { locals, .. } = frame {
                            locals.resize(locals.len().max(8), VerificationType::Top);
                            for (slot, index) in [(4, camera), (6, matrix), (7, matrix)] {
                                locals[slot] = VerificationType::Object { cpool_index: index };
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

 
 
 
 
fn replace_with_bridge_returning_object(
    class_file: &mut ClassFile<'_>,
    method_index: usize,
    params: &TransformParams,
) -> anyhow::Result<()> {
    for attribute in &mut class_file.methods[method_index].attributes {
        let Attribute::Code {
            code,
            max_stack,
            exception_table,
            attributes: code_attributes,
            ..
        } = attribute
        else {
            continue;
        };
        let bridge_class = class_file.constant_pool.add_class(params.bridge_class)?;
        let bridge_ref = class_file.constant_pool.add_method_ref(
            bridge_class,
            params.bridge_method,
            params.bridge_descriptor,
        )?;
         
         
         
         
        anyhow::ensure!(
            exception_table.is_empty(),
            "Xray getter has exception handlers"
        );
        let returns_int = params.descriptor.ends_with(")I");
        let return_op = if params.descriptor == "()Z" || returns_int {
            Instruction::Ireturn
        } else {
            Instruction::Areturn
        };
        anyhow::ensure!(
            code.last() == Some(&return_op),
            "Xray getter has unexpected return"
        );
        let return_count = code
            .iter()
            .filter(|instruction| {
                matches!(instruction, Instruction::Ireturn | Instruction::Areturn)
            })
            .count();
        anyhow::ensure!(
            return_count == 1,
            "Xray getter has {return_count} returns, expected only the terminal one"
        );
        for instruction in code.iter() {
            anyhow::ensure!(
                !matches!(
                    instruction,
                    Instruction::Tableswitch(_) | Instruction::Lookupswitch(_)
                ),
                "Xray getter contains a switch"
            );
        }
        code.pop();
        if params.descriptor == "()Z" {
            code.extend([Instruction::Invokestatic(bridge_ref), Instruction::Ireturn]);
        } else if returns_int {
             
            code.extend([
                Instruction::Aload_0,
                Instruction::Invokestatic(bridge_ref),
                Instruction::Ireturn,
            ]);
        } else {
            let return_class = params
                .descriptor
                .strip_prefix("()L")
                .and_then(|s| s.strip_suffix(';'))
                .ok_or_else(|| anyhow::anyhow!("Xray getter must return an object"))?;
            let cast = class_file.constant_pool.add_class(return_class)?;
            code.extend([
                Instruction::Aload_0,
                Instruction::Swap,
                Instruction::Invokestatic(bridge_ref),
                Instruction::Checkcast(cast),
                Instruction::Areturn,
            ]);
        }
        *max_stack = (*max_stack).max(2);
         
         
         
        code_attributes.retain(|a| {
            !matches!(
                a,
                Attribute::LocalVariableTable { .. }
                    | Attribute::LocalVariableTypeTable { .. }
                    | Attribute::LineNumberTable { .. }
            )
        });
        log::info!(
            "CFLH diagnostic mode=normal: {}.{} body replaced with bridge call: max_stack={max_stack}",
            params.target_class,
            params.method
        );
        return Ok(());
    }
    anyhow::bail!("{} target method has no Code attribute", params.log_label)
}

 
 
 
 
 
 
 
 
const GUARD_PREFIX_INSTRUCTIONS: u16 = 3;

 
 
 
 
 
 
 
const GUARD_PREFIX_BYTES: u16 = 7;

 
fn is_guard_field(
    pool: &ristretto_classfile::ConstantPool<'_>,
    index: u16,
    params: &TransformParams,
) -> bool {
    let Some(Constant::FieldRef {
        class_index,
        name_and_type_index,
    }) = pool.get(index)
    else {
        return false;
    };
    let Some(Constant::Class(name_index)) = pool.get(*class_index) else {
        return false;
    };
    let Some(Constant::NameAndType {
        name_index: field_name_index,
        descriptor_index,
    }) = pool.get(*name_and_type_index)
    else {
        return false;
    };
    let owner = pool
        .try_get_utf8(*name_index)
        .map(|value| value.to_string())
        .ok();
    let field = pool
        .try_get_utf8(*field_name_index)
        .map(|value| value.to_string())
        .ok();
    let descriptor = pool
        .try_get_utf8(*descriptor_index)
        .map(|value| value.to_string())
        .ok();
    owner.as_deref() == Some(params.bridge_class)
        && field.as_deref() == params.guard_flag
        && descriptor.as_deref() == Some("Z")
}

 
 
 
fn already_injected(
    class_file: &ClassFile<'_>,
    method_index: usize,
    params: &TransformParams,
) -> anyhow::Result<bool> {
    let code = class_file.methods[method_index]
        .attributes
        .iter()
        .find_map(|attribute| {
            let Attribute::Code { code, .. } = attribute else {
                return None;
            };
            Some(code)
        })
        .ok_or_else(|| {
            anyhow::anyhow!("{} target method has no Code attribute", params.log_label)
        })?;
    if params.replace_with_bridge_return || params.guard_flag.is_some() {
        Ok(code.iter().any(|instruction| {
            matches!(
                instruction,
                Instruction::Getstatic(index)
                    if is_guard_field(&class_file.constant_pool, *index, params)
            ) || matches!(
                instruction,
                Instruction::Invokestatic(index)
                    if is_bridge_method(&class_file.constant_pool, *index, params)
            )
        }))
    } else {
        Ok(code.iter().any(|instruction| {
            matches!(
                instruction,
                Instruction::Invokestatic(index)
                    if is_bridge_method(&class_file.constant_pool, *index, params)
            )
        }))
    }
}

 
 
 
fn shift_absolute_target(instruction: &mut Instruction, shift: u16) {
    macro_rules! shift_u16 {
        ($($variant:ident),* $(,)?) => {
            match instruction {
                $(Instruction::$variant(target) => {
                    *target = target.saturating_add(shift);
                })*
                _ => {}
            }
        };
    }
    shift_u16!(
        Ifeq, Ifne, Iflt, Ifge, Ifgt, Ifle, If_icmpeq, If_icmpne, If_icmplt, If_icmpge, If_icmpgt,
        If_icmple, If_acmpeq, If_acmpne, Ifnull, Ifnonnull, Goto, Jsr,
    );
    match instruction {
        Instruction::Goto_w(target) | Instruction::Jsr_w(target) => {
            *target = target.saturating_add(i32::from(shift));
        }
        _ => {}
    }
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
fn shift_code_attribute_offsets(attribute: &mut Attribute, index_shift: u16, byte_shift: u16) {
    match attribute {
        Attribute::LineNumberTable { line_numbers, .. } => {
            for line in line_numbers {
                line.start_pc = line.start_pc.saturating_add(index_shift);
            }
        }
        Attribute::LocalVariableTable { variables, .. } => {
            for variable in variables {
                variable.start_pc = variable.start_pc.saturating_add(byte_shift);
            }
        }
        Attribute::LocalVariableTypeTable { variable_types, .. } => {
            for variable in variable_types {
                variable.start_pc = variable.start_pc.saturating_add(byte_shift);
            }
        }
        _ => {}
    }
}

 
 
 
 
 
 
 
 
 
fn drop_debug_tables_unless_shift_is_flat(
    code: &[Instruction],
    code_attributes: &mut Vec<Attribute>,
    byte_shift: u16,
) {
    if byte_shift % 4 == 0 {
        return;
    }
    if !code.iter().any(|instruction| {
        matches!(
            instruction,
            Instruction::Tableswitch(_) | Instruction::Lookupswitch(_)
        )
    }) {
        return;
    }
    code_attributes.retain(|a| {
        !matches!(
            a,
            Attribute::LocalVariableTable { .. } | Attribute::LocalVariableTypeTable { .. }
        )
    });
}

 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
fn transform_prepend_guard(
    class_file: &mut ClassFile<'_>,
    method_index: usize,
    params: &TransformParams,
) -> anyhow::Result<(usize, u16, u16)> {
    let label = params.log_label;
    let float_identity = params.descriptor == "(F)F";
    let selected_state_guard = matches!(params.log_label, "XrayBatched" | "XrayLiquid");
    let shift = if float_identity || selected_state_guard {
        4
    } else {
        GUARD_PREFIX_INSTRUCTIONS
    };
    let byte_shift = if float_identity {
        8
    } else if selected_state_guard {
        if params.log_label == "XrayLiquid" {
            9
        } else {
            8
        }
    } else {
        GUARD_PREFIX_BYTES
    };
    let static_method = class_file.methods[method_index]
        .access_flags
        .contains(MethodAccessFlags::STATIC);
    let bridge_class = class_file.constant_pool.add_class(params.bridge_class)?;
    let flag_ref = class_file.constant_pool.add_field_ref(
        bridge_class,
        params.guard_flag.expect("guard"),
        "Z",
    )?;
    let stackmap_name = class_file.constant_pool.add_utf8("StackMapTable")?;
    let mut frame_locals = Vec::with_capacity(6);
    if !static_method {
        frame_locals.push(VerificationType::Object {
            cpool_index: class_file.this_class,
        });
    }
     
     
    let descriptor = params.descriptor.as_bytes();
    let mut cursor = 1;
    anyhow::ensure!(
        descriptor.first() == Some(&b'('),
        "invalid guard descriptor"
    );
    while descriptor.get(cursor) != Some(&b')') {
        let start = cursor;
        let mut array = false;
        while descriptor.get(cursor) == Some(&b'[') {
            array = true;
            cursor += 1;
        }
        let kind = *descriptor
            .get(cursor)
            .ok_or_else(|| anyhow::anyhow!("incomplete guard descriptor"))?;
        cursor += 1;
        let object = if kind == b'L' {
            let name_start = cursor;
            while descriptor.get(cursor).is_some_and(|b| *b != b';') {
                cursor += 1;
            }
            anyhow::ensure!(
                descriptor.get(cursor) == Some(&b';'),
                "unterminated guard object"
            );
            let name = &params.descriptor[name_start..cursor];
            cursor += 1;
            Some(name)
        } else {
            None
        };
        let verification = if array || object.is_some() {
            let name = if array {
                &params.descriptor[start..cursor]
            } else {
                object.unwrap()
            };
            VerificationType::Object {
                cpool_index: class_file.constant_pool.add_class(name)?,
            }
        } else {
            match kind {
                b'Z' | b'B' | b'C' | b'S' | b'I' => VerificationType::Integer,
                b'F' => VerificationType::Float,
                b'J' => VerificationType::Long,
                b'D' => VerificationType::Double,
                _ => anyhow::bail!("invalid guard argument descriptor"),
            }
        };
        frame_locals.push(verification);
    }
    let selected_method = if selected_state_guard {
        Some(class_file.constant_pool.add_method_ref(
            bridge_class,
            params.bridge_method,
            params.bridge_descriptor,
        )?)
    } else {
        None
    };
    for attribute in &mut class_file.methods[method_index].attributes {
        let Attribute::Code {
            code,
            max_stack,
            max_locals,
            exception_table,
            attributes: code_attributes,
            ..
        } = attribute
        else {
            continue;
        };
        let original_code_len = code.len();
        drop_debug_tables_unless_shift_is_flat(code.as_slice(), code_attributes, byte_shift);
        for instruction in code.iter_mut() {
            shift_absolute_target(instruction, shift);
        }
        for entry in exception_table.iter_mut() {
             
             
            entry.range_pc.start = entry.range_pc.start.saturating_add(shift);
            entry.range_pc.end = entry.range_pc.end.saturating_add(shift);
            entry.handler_pc = entry.handler_pc.saturating_add(shift);
        }
        for code_attribute in code_attributes.iter_mut() {
            shift_code_attribute_offsets(code_attribute, shift, byte_shift);
        }
        let mut prefix = if let Some(method) = selected_method {
            vec![
                if params.log_label == "XrayLiquid" {
                    Instruction::Aload(4)
                } else {
                    Instruction::Aload_1
                },
                Instruction::Invokestatic(method),
                Instruction::Ifeq(shift),
                Instruction::Return,
            ]
        } else {
            vec![Instruction::Getstatic(flag_ref), Instruction::Ifeq(shift)]
        };
        if !selected_state_guard {
            if float_identity {
                prefix.extend([Instruction::Fload_1, Instruction::Freturn]);
            } else {
                prefix.push(Instruction::Return);
            }
        }
        code.splice(0..0, prefix);
        *max_stack = (*max_stack).max(1);
        let mut stackmap_frame_count = None;
        let mut subsumed_first_frame = false;
        let mut frame_added = false;
        for code_attribute in code_attributes.iter_mut() {
            let Attribute::StackMapTable { frames, .. } = code_attribute else {
                continue;
            };
            stackmap_frame_count = Some(frames.len());
            if let Some(first_delta) = frames.first().map(StackFrame::offset_delta) {
                if first_delta == 0 {
                     
                     
                     
                    frames.remove(0);
                    subsumed_first_frame = true;
                } else {
                    decrement_offset_delta(&mut frames[0], first_delta);
                }
            }
            frames.insert(
                0,
                StackFrame::FullFrame {
                    frame_type: 255,
                    offset_delta: shift,
                    locals: frame_locals.clone(),
                    stack: vec![],
                },
            );
            frame_added = true;
        }
        if !frame_added {
            code_attributes.push(Attribute::StackMapTable {
                name_index: stackmap_name,
                frames: vec![StackFrame::FullFrame {
                    frame_type: 255,
                    offset_delta: shift,
                    locals: frame_locals.clone(),
                    stack: vec![],
                }],
            });
            frame_added = true;
        }
        log::info!(
            "CFLH diagnostic mode=normal: {label}.{} prologue guard spliced: instructions {original_code_len} -> {}, max_stack={max_stack}, max_locals={max_locals}, exception_handlers={}, stackmap_prepended={}, stackmap_frames_before={:?}, first_frame={}",
            params.method,
            code.len(),
            exception_table.len(),
            frame_added,
            stackmap_frame_count,
            if subsumed_first_frame {
                "subsumed"
            } else {
                "delta-1"
            }
        );
        return Ok((code.len(), *max_stack, *max_locals));
    }
    Err(anyhow::anyhow!(
        "{label} target method has no Code attribute"
    ))
}

 
 
 
 
fn decrement_offset_delta(frame: &mut StackFrame, delta: u16) {
    *frame = leading_frame_with_delta(frame, delta - 1);
}

 
 
 
 
 
 
 
fn shift_first_frame_delta(frame: &mut StackFrame, shift: u16) {
    let new_delta = frame.offset_delta().saturating_add(shift);
    *frame = leading_frame_with_delta(frame, new_delta);
}

 
 
 
 
 
fn leading_frame_with_delta(frame: &StackFrame, new_delta: u16) -> StackFrame {
    match frame.clone() {
        StackFrame::SameFrame { .. } | StackFrame::SameFrameExtended { .. } => {
            StackFrame::SameFrameExtended {
                frame_type: 251,
                offset_delta: new_delta,
            }
        }
        StackFrame::SameLocals1StackItemFrame { stack, .. }
        | StackFrame::SameLocals1StackItemFrameExtended { stack, .. } => {
            StackFrame::SameLocals1StackItemFrameExtended {
                frame_type: 247,
                offset_delta: new_delta,
                stack,
            }
        }
        StackFrame::ChopFrame { frame_type, .. } => StackFrame::ChopFrame {
            frame_type,
            offset_delta: new_delta,
        },
        StackFrame::AppendFrame {
            frame_type, locals, ..
        } => StackFrame::AppendFrame {
            frame_type,
            offset_delta: new_delta,
            locals,
        },
        StackFrame::FullFrame {
            frame_type,
            locals,
            stack,
            ..
        } => StackFrame::FullFrame {
            frame_type,
            offset_delta: new_delta,
            locals,
            stack,
        },
    }
}

 
 
 
fn descriptor_class_name(descriptor_type: &str) -> &str {
    descriptor_type
        .strip_prefix('L')
        .and_then(|name| name.strip_suffix(';'))
        .unwrap_or(descriptor_type)
}

 
 
 
 
fn descriptor_local_type<'descriptor>(
    descriptor: &'descriptor str,
    slot: u8,
    is_static: bool,
) -> anyhow::Result<&'descriptor str> {
    let bytes = descriptor.as_bytes();
    anyhow::ensure!(bytes.first() == Some(&b'('), "invalid method descriptor");
    let mut index: u8 = if is_static { 0 } else { 1 };
    let mut cursor = 1_usize;
    while bytes.get(cursor) != Some(&b')') {
        let start = cursor;
        while bytes.get(cursor) == Some(&b'[') {
            cursor += 1;
        }
        let kind = *bytes
            .get(cursor)
            .ok_or_else(|| anyhow::anyhow!("truncated method descriptor"))?;
        cursor += 1;
        let width = if kind == b'L' {
            while bytes.get(cursor).is_some_and(|byte| *byte != b';') {
                cursor += 1;
            }
            anyhow::ensure!(bytes.get(cursor) == Some(&b';'), "unterminated object type");
            cursor += 1;
            1
        } else if kind == b'J' || kind == b'D' {
            2
        } else {
            1
        };
        if index == slot {
            let local = &descriptor[start..cursor];
            anyhow::ensure!(
                local.starts_with('L') || local.starts_with('['),
                "local at slot {slot} is not a reference type"
            );
            return Ok(local);
        }
        index = index.saturating_add(width);
    }
    anyhow::bail!("descriptor has no local at slot {slot}")
}

 
 
 
 
 
 
 
 
 
 
 
 
 
fn transform_prepend_object_call(
    class_file: &mut ClassFile<'_>,
    method_index: usize,
    params: &TransformParams,
    call: PrependCall,
) -> anyhow::Result<(usize, u16, u16)> {
    let label = params.log_label;
    anyhow::ensure!(
        !call.arguments.is_empty() || call.destination.is_none(),
        "{label} prepend call needs at least one local to bind its destination to"
    );
    let is_static = class_file.methods[method_index]
        .access_flags
        .contains(MethodAccessFlags::STATIC);
     
     
     
     
    let mut instructions = 1u16;
    let mut bytes = 3u16;
    for argument in call.arguments.iter() {
        match argument {
            PrependArg::Reference(_) => {
                instructions += 1;
                bytes += 2;
            }
            PrependArg::Float(_) | PrependArg::Int(_) => {
                instructions += 2;
                bytes += 5;
            }
        }
    }
    let uses_float = call
        .arguments
        .iter()
        .any(|argument| matches!(argument, PrependArg::Float(_)));
    let uses_int = call
        .arguments
        .iter()
        .any(|argument| matches!(argument, PrependArg::Int(_)));
    let box_float = if uses_float {
        let float_class = class_file.constant_pool.add_class("java/lang/Float")?;
        Some(class_file.constant_pool.add_method_ref(
            float_class,
            "valueOf",
            "(F)Ljava/lang/Float;",
        )?)
    } else {
        None
    };
    let box_int = if uses_int {
        let int_class = class_file.constant_pool.add_class("java/lang/Integer")?;
        Some(class_file.constant_pool.add_method_ref(
            int_class,
            "valueOf",
            "(I)Ljava/lang/Integer;",
        )?)
    } else {
        None
    };
    let checkcast = match call.destination {
        Some(destination) => {
            instructions += 2;
            bytes += 5;
             
             
             
            let local = descriptor_local_type(params.descriptor, destination, is_static)?;
            Some(
                class_file
                    .constant_pool
                    .add_class(descriptor_class_name(local))?,
            )
        }
        None => None,
    };
    let bridge_class = class_file.constant_pool.add_class(params.bridge_class)?;
    let bridge = class_file.constant_pool.add_method_ref(
        bridge_class,
        params.bridge_method,
        params.bridge_descriptor,
    )?;
    let mut spliced = 0usize;
    let mut final_code_len = 0usize;
    let mut final_max_stack = 0u16;
    let mut final_max_locals = 0u16;
    for attribute in &mut class_file.methods[method_index].attributes {
        let Attribute::Code {
            code,
            max_stack,
            max_locals,
            exception_table,
            attributes: code_attributes,
            ..
        } = attribute
        else {
            continue;
        };
        spliced += 1;
        drop_debug_tables_unless_shift_is_flat(code.as_slice(), code_attributes, bytes);
        for instruction in code.iter_mut() {
            shift_absolute_target(instruction, instructions);
        }
        for entry in exception_table.iter_mut() {
            entry.range_pc.start = entry.range_pc.start.saturating_add(instructions);
            entry.range_pc.end = entry.range_pc.end.saturating_add(instructions);
            entry.handler_pc = entry.handler_pc.saturating_add(instructions);
        }
        for code_attribute in code_attributes.iter_mut() {
            shift_code_attribute_offsets(code_attribute, instructions, bytes);
        }
        for code_attribute in code_attributes.iter_mut() {
            if let Attribute::StackMapTable { frames, .. } = code_attribute {
                if let Some(first) = frames.first_mut() {
                    shift_first_frame_delta(first, instructions);
                }
            }
        }
        let mut prologue: Vec<Instruction> = Vec::new();
        for argument in call.arguments.iter() {
            match (argument, box_float, box_int) {
                (PrependArg::Reference(slot), _, _) => {
                    prologue.push(Instruction::Aload(*slot));
                }
                (PrependArg::Float(slot), Some(box_float), _) => {
                    prologue.push(Instruction::Fload(*slot));
                    prologue.push(Instruction::Invokestatic(box_float));
                }
                (PrependArg::Int(slot), _, Some(box_int)) => {
                    prologue.push(Instruction::Iload(*slot));
                    prologue.push(Instruction::Invokestatic(box_int));
                }
                _ => anyhow::bail!("{label} prepend boxing constant was not resolved"),
            }
        }
        prologue.push(Instruction::Invokestatic(bridge));
        if let (Some(index), Some(destination)) = (checkcast, call.destination) {
            prologue.push(Instruction::Checkcast(index));
            prologue.push(Instruction::Astore(destination));
        }
        code.splice(0..0, prologue);
         
         
        let depth = call.arguments.len().max(1);
        *max_stack = (*max_stack).max(u16::try_from(depth).unwrap_or(u16::MAX));
        final_code_len = code.len();
        final_max_stack = *max_stack;
        final_max_locals = *max_locals;
    }
    anyhow::ensure!(
        spliced == 1,
        "{label} target method must have exactly one Code attribute, found {spliced}"
    );
    log::info!(
        "CFLH diagnostic mode=normal: {label}.{} prologue call spliced (args={:?}, destination={:?}): instructions shift by {instructions}, bytes by {bytes}, max_stack={final_max_stack}, max_locals={final_max_locals}",
        params.method,
        call.arguments,
        call.destination
    );
    Ok((final_code_len, final_max_stack, final_max_locals))
}

 
 
 
 
fn patch_scaffold_calls(
    bytes: &[u8],
    params: &[&TransformParams],
) -> anyhow::Result<Option<Vec<u8>>> {
    use ristretto_classfile::byte_reader::ByteReader;
    use ristretto_classfile::ConstantPool;
    let mut reader = ByteReader::new(bytes);
    reader.set_position(8);  
    let mut pool = ConstantPool::from_bytes(&mut reader)?;
    let pool_end = reader.position();
    reader.set_position(pool_end + 6);  
    let interfaces = reader.read_u16()?;
    reader.set_position(reader.position() + usize::from(interfaces) * 2);
    let fields = reader.read_u16()?;
    for _ in 0..fields {
        reader.set_position(reader.position() + 6);
        let attributes = reader.read_u16()?;
        for _ in 0..attributes {
            reader.read_u16()?;
            let length = reader.read_u32()? as usize;
            reader.set_position(reader.position() + length);
        }
    }
    let methods = reader.read_u16()?;
    let mut changes = Vec::new();
    let mut counts = vec![0usize; params.len()];
    let mut existing = vec![false; params.len()];
    for _ in 0..methods {
        reader.read_u16()?;
        let name = pool.try_get_utf8(reader.read_u16()?)?.to_string();
        let descriptor = pool.try_get_utf8(reader.read_u16()?)?.to_string();
        let attributes = reader.read_u16()?;
        for _ in 0..attributes {
            let attr_name = pool.try_get_utf8(reader.read_u16()?)?.to_string();
            let attr_length = reader.read_u32()? as usize;
            let start = reader.position();
            if attr_name == "Code" {
                reader.read_u16()?;
                reader.read_u16()?;
                let length = reader.read_u32()? as usize;
                let code_start = reader.position();
                let code_bytes = bytes
                    .get(code_start..code_start + length)
                    .ok_or_else(|| anyhow::anyhow!("Truncated scaffold Code attribute"))?;
                let mut code = ByteReader::new(code_bytes);
                while code.remaining() > 0 {
                    let offset = code.position();
                    let instruction = Instruction::from_bytes(&mut code)?;
                    for (i, params) in params.iter().enumerate() {
                        if name != params.method || descriptor != params.descriptor {
                            continue;
                        }
                        if let Instruction::Invokestatic(index) = instruction {
                            existing[i] |= is_bridge_method(&pool, index, params);
                        }
                        let (owner, method, original_descriptor) = match params.log_label {
                            "ScaffoldEdgeSafety" => {
                                ("net/minecraft/class_1657", "method_21825", "()Z")
                            }
                            "TotemPop" => ("net/minecraft/class_4587", "method_22905", "(FFF)V"),
                            _ => ("net/minecraft/class_310", "method_1508", "()V"),
                        };
                        let index = match instruction {
                            Instruction::Invokevirtual(index)
                            | Instruction::Invokespecial(index) => index,
                            _ => continue,
                        };
                        let Constant::MethodRef {
                            class_index,
                            name_and_type_index,
                        } = pool.get(index).unwrap()
                        else {
                            continue;
                        };
                        let Constant::Class(class_name) = pool.get(*class_index).unwrap() else {
                            continue;
                        };
                        let Constant::NameAndType {
                            name_index,
                            descriptor_index,
                        } = pool.get(*name_and_type_index).unwrap()
                        else {
                            continue;
                        };
                        if pool.try_get_utf8(*class_name)?.to_string() != owner
                            || pool.try_get_utf8(*name_index)?.to_string() != method
                            || pool.try_get_utf8(*descriptor_index)?.to_string()
                                != original_descriptor
                        {
                            continue;
                        }
                        anyhow::ensure!(
                            code.position() - offset == 3,
                            "Scaffold replacement must preserve call width"
                        );
                        let bridge_class = pool.add_class(params.bridge_class)?;
                        let bridge = pool.add_method_ref(
                            bridge_class,
                            params.bridge_method,
                            params.bridge_descriptor,
                        )?;
                        changes.push((code_start + offset, bridge));
                        counts[i] += 1;
                    }
                }
            }
            reader.set_position(start + attr_length);
        }
    }
    for (i, params) in params.iter().enumerate() {
        anyhow::ensure!(
            (existing[i] && counts[i] == 0) || (!existing[i] && counts[i] == 1),
            "{} requires exactly one call replacement (found {}, existing={})",
            params.log_label,
            counts[i],
            existing[i]
        );
    }
    if changes.is_empty() {
        return Ok(None);
    }
    let mut result = bytes[..8].to_vec();
    pool.to_bytes(&mut result)?;
    let new_pool_end = result.len();
    result.extend_from_slice(&bytes[pool_end..]);
    for (offset, bridge) in changes {
        let offset = offset - pool_end + new_pool_end;
        result[offset] = 0xb8;  
        result[offset + 1..offset + 3].copy_from_slice(&bridge.to_be_bytes());
    }
    ClassFile::from_bytes(&result)?.verify()?;
    Ok(Some(result))
}

pub(crate) fn transform_class(
    bytes: &[u8],
    params_list: &[TransformParams],
) -> anyhow::Result<Option<Vec<u8>>> {
    let mut class_file = ClassFile::from_bytes(bytes)
        .map_err(|error| anyhow::anyhow!("class parse failed: {error}"))?;
    let class_name = class_file.class_name()?.to_string();
    let matched: Vec<&TransformParams> = params_list
        .iter()
        .filter(|params| {
            params.target_class == class_name || params.target_class == class_name.replace('.', "/")
        })
        .collect();
    if matched.is_empty() {
        return Ok(None);
    }

    let mode = diagnostic_mode();
    if mode == "freeze" || mode == "dump" {
        if mode == "dump" {
            let path = config::base_dir().join(format!(
                "{}_{}.class",
                matched[0].target_class.replace('/', "_"),
                bytes.len()
            ));
            match std::fs::write(&path, bytes) {
                Ok(()) => log::info!(
                    "CFLH diagnostic mode=dump: saved {} live bytes to {}",
                    bytes.len(),
                    path.display()
                ),
                Err(error) => {
                    log::warn!("CFLH diagnostic mode=dump: failed to save live bytes: {error}")
                }
            }
        }
        log::info!("CFLH diagnostic mode={mode}: passing original class bytes through untouched");
        return Ok(None);
    }

    if mode == "normal"
        && matched.iter().all(|p| {
            matches!(
                p.log_label,
                "ScaffoldInputPhase" | "ScaffoldEdgeSafety" | "TotemPop"
            )
        })
    {
        return patch_scaffold_calls(bytes, &matched);
    }

    let mut transformed_any = false;
    let mut final_code_len = 0_usize;
    let mut final_max_stack = 0_u16;
    let mut final_max_locals = 0_u16;
    for params in matched {
        let label = params.log_label;
        log::info!("{label} transformation started: bytes={}", bytes.len());
        let method_index = class_file
            .methods
            .iter()
            .position(|method| {
                class_file
                    .constant_pool
                    .try_get_utf8(method.name_index)
                    .map(|name| name.to_string())
                    .ok()
                    .as_deref()
                    == Some(params.method)
                    && class_file
                        .constant_pool
                        .try_get_utf8(method.descriptor_index)
                        .map(|desc| desc.to_string())
                        .ok()
                        .as_deref()
                        == Some(params.descriptor)
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "{label} target method not found: {}{}",
                    params.method,
                    params.descriptor
                )
            })?;
        log::info!(
            "{label} target method found: {}::{}{}",
            params.target_class,
            params.method,
            params.descriptor
        );
        if already_injected(&class_file, method_index, params)? {
            log::info!("{label} already injected; original {label} bytes preserved");
            continue;
        }
        if mode == "roundtrip" {
            log::info!(
                "CFLH diagnostic mode=roundtrip: re-serializing {label} without bytecode edits"
            );
            continue;
        }

        if matches!(
            params.log_label,
            "ScaffoldInputPhase" | "ScaffoldEdgeSafety"
        ) {
            anyhow::ensure!(
                mode == "normal",
                "Scaffold hooks require normal transform mode"
            );
            let (original_owner, original_method) = ("net/minecraft/class_310", "method_1508");
            let owner = class_file.constant_pool.add_class(params.bridge_class)?;
            let bridge = class_file.constant_pool.add_method_ref(
                owner,
                params.bridge_method,
                params.bridge_descriptor,
            )?;
            let pool = &class_file.constant_pool;
            let original_refs: Vec<u16> = (1..=pool.len())
                .filter_map(|index| {
                    let constant = pool.get(index as u16)?;
                    let Constant::MethodRef {
                        class_index,
                        name_and_type_index,
                    } = constant
                    else {
                        return None;
                    };
                    let Constant::Class(owner_name) = pool.get(*class_index)? else {
                        return None;
                    };
                    let Constant::NameAndType {
                        name_index,
                        descriptor_index,
                    } = pool.get(*name_and_type_index)?
                    else {
                        return None;
                    };
                    (pool.try_get_utf8(*owner_name).ok()?.to_string() == original_owner
                        && pool.try_get_utf8(*name_index).ok()?.to_string() == original_method
                        && pool.try_get_utf8(*descriptor_index).ok()?.to_string() == "()V")
                        .then_some(index as u16)
                })
                .collect();
            let mut replaced = 0;
            for attribute in &mut class_file.methods[method_index].attributes {
                if let Attribute::Code { code, .. } = attribute {
                    for instruction in code {
                        if matches!(instruction, Instruction::Invokevirtual(index) | Instruction::Invokespecial(index) if original_refs.contains(index))
                        {
                             
                             
                            *instruction = Instruction::Invokestatic(bridge);
                            replaced += 1;
                        }
                    }
                }
            }
            anyhow::ensure!(
                replaced == 1,
                "Expected exactly one vanilla {original_method} call for {label}, found {replaced}"
            );
            transformed_any = true;
            continue;
        }

        if params.replace_with_bridge_return {
            if mode != "normal" {
                log::info!("CFLH diagnostic mode={mode}: {label} bridge-return transform skipped");
                continue;
            }
            replace_with_bridge_returning_object(&mut class_file, method_index, params)?;
            if let Some(Attribute::Code {
                code,
                max_stack,
                max_locals,
                ..
            }) = class_file.methods[method_index]
                .attributes
                .iter()
                .find(|a| matches!(a, Attribute::Code { .. }))
            {
                final_code_len = code.len();
                final_max_stack = *max_stack;
                final_max_locals = *max_locals;
            }
            transformed_any = true;
            continue;
        }
        if params.guard_flag.is_some() {
            if mode != "normal" {
                log::info!("CFLH diagnostic mode={mode}: {label} guard transform skipped");
                continue;
            }
            let (mut code_len, mut max_stack, mut max_locals) =
                transform_prepend_guard(&mut class_file, method_index, params)?;
             
             
             
             
             
            if let Some(call) = params.prepend_object_call {
                let (len, stack, locals) =
                    transform_prepend_object_call(&mut class_file, method_index, params, call)?;
                code_len = len;
                max_stack = stack;
                max_locals = locals;
            }
            final_code_len = code_len;
            final_max_stack = max_stack;
            final_max_locals = max_locals;
            transformed_any = true;
            continue;
        }
        if let Some(call) = params.prepend_object_call {
            if mode != "normal" {
                log::info!("CFLH diagnostic mode={mode}: {label} prepend-call transform skipped");
                continue;
            }
            let (code_len, max_stack, max_locals) =
                transform_prepend_object_call(&mut class_file, method_index, params, call)?;
            final_code_len = code_len;
            final_max_stack = max_stack;
            final_max_locals = max_locals;
            transformed_any = true;
            continue;
        }

         
         
         
         
        let minimal_mutation = mode == "tail3" || mode == "pop";
        let bridge_ref = if minimal_mutation {
            None
        } else {
            let bridge_class = class_file.constant_pool.add_class(params.bridge_class)?;
            Some(class_file.constant_pool.add_method_ref(
                bridge_class,
                params.bridge_method,
                params.bridge_descriptor,
            )?)
        };
        let mut saw_code = false;
        if params.replace_entire_body {
             
             
             
             
             
             
             
            let bridge_ref = bridge_ref.ok_or_else(|| {
                anyhow::anyhow!("{label} whole-body replace requires a bridge reference")
            })?;
            for attribute in &mut class_file.methods[method_index].attributes {
                let Attribute::Code {
                    code,
                    max_stack,
                    max_locals,
                    exception_table,
                    attributes: code_attributes,
                    ..
                } = attribute
                else {
                    continue;
                };
                saw_code = true;
                if !exception_table.is_empty() {
                    return Err(anyhow::anyhow!(
                        "{label} whole-body replace: target method must not have exception handlers"
                    ));
                }
                let original_code_len = code.len();
                 
                 
                 
                 
                 
                 
                 
                code_attributes.clear();
                code.clear();
                code.push(Instruction::Aload_0);
                code.push(Instruction::Invokestatic(bridge_ref));
                code.push(Instruction::Return);
                *max_stack = 1;
                final_code_len = code.len();
                final_max_stack = *max_stack;
                final_max_locals = *max_locals;
                log::info!(
                    "CFLH diagnostic mode={mode}: {label}.{} whole body replaced: instructions {original_code_len} -> {}, max_stack={max_stack}, max_locals={max_locals}, exception_handlers=0 (must be branch-free)",
                    params.method,
                    code.len()
                );
                transformed_any = true;
                break;
            }
            if !saw_code {
                return Err(anyhow::anyhow!(
                    "{label} target method has no Code attribute"
                ));
            }
        } else {
            let mut terminal_instruction = 0_usize;
            for attribute in &mut class_file.methods[method_index].attributes {
                let Attribute::Code {
                    code,
                    max_stack,
                    max_locals,
                    exception_table,
                    ..
                } = attribute
                else {
                    continue;
                };
                saw_code = true;
                let original_code_len = code.len();
                terminal_instruction = original_code_len - 1;
                let last_index = code.len().checked_sub(1).ok_or_else(|| {
                    anyhow::anyhow!("{label} target method has an empty Code attribute")
                })?;
                if !matches!(code.last(), Some(Instruction::Return)) {
                    return Err(anyhow::anyhow!(
                        "{label} target method terminal instruction is not a void RETURN"
                    ));
                }
                log::info!(
                    "CFLH diagnostic mode={mode}: {label}.{} Code before edit: instructions={original_code_len}, max_stack={max_stack}, max_locals={max_locals}, exception_handlers={}, terminal={:?}, constant_pool_entries={}",
                    params.method,
                    exception_table.len(),
                    code.last(),
                    class_file.constant_pool.len()
                );
                 
                 
                 
                 
                 
                code[last_index] = Instruction::Aload_0;
                if minimal_mutation {
                    code.push(Instruction::Pop);
                } else if let Some(bridge_ref) = bridge_ref {
                    if params.log_label == "SmokeWorldRender" {
                         
                         
                        code.push(Instruction::Aload(4));
                        code.push(Instruction::Aload(6));
                        code.push(Instruction::Aload(7));
                    }
                    code.push(Instruction::Invokestatic(bridge_ref));
                }
                code.push(Instruction::Return);
                *max_stack = (*max_stack).max(if params.log_label == "SmokeWorldRender" {
                    4
                } else {
                    1
                });
                final_code_len = code.len();
                final_max_stack = *max_stack;
                final_max_locals = *max_locals;
                log::info!(
                    "CFLH diagnostic mode={mode}: {label}.{} transformed tail: instructions {original_code_len} -> {}, max_stack={max_stack}, max_locals={max_locals}, exception_handlers={}",
                    params.method,
                    code.len(),
                    exception_table.len()
                );
                transformed_any = true;
                break;
            }
            if !saw_code {
                return Err(anyhow::anyhow!(
                    "{label} target method has no Code attribute"
                ));
            }
            if params.log_label == "SmokeWorldRender" {
                preserve_smoke_argument_frames(&mut class_file, method_index, params)?;
            } else {
                 
                 
                let require_terminal_frame = params.log_label != "HandAuraAfter";
                patch_terminal_stackmap(
                    &mut class_file,
                    method_index,
                    terminal_instruction,
                    &mode,
                    require_terminal_frame,
                )?;
            }
        }
    }
    if !transformed_any {
        return Ok(None);
    }
    class_file
        .verify()
        .map_err(|error| anyhow::anyhow!("transformed class verification failed: {error}"))?;
    let mut output = Vec::new();
    class_file
        .to_bytes(&mut output)
        .map_err(|error| anyhow::anyhow!("transformed class write failed: {error}"))?;
    let reparsed = ClassFile::from_bytes(&output)
        .map_err(|error| anyhow::anyhow!("serialized class parse failed: {error}"))?;
    reparsed
        .verify()
        .map_err(|error| anyhow::anyhow!("serialized class verification failed: {error}"))?;
    log::info!(
        "transformation verified: class={} ({} transforms applied), original_bytes={}, transformed_bytes={}, transformed_instructions={}, transformed_max_stack={}, transformed_max_locals={}",
        class_name,
        params_list.len(),
        bytes.len(),
        output.len(),
        final_code_len,
        final_max_stack,
        final_max_locals
    );
    Ok(Some(output))
}

 
 
fn jvmti_status_name(status: i32) -> String {
    let ptr = unsafe { dark_jvmti_status_name_ptr(status) };
    if ptr.is_null() {
        return "JVMTI_ERROR_UNKNOWN".to_owned();
    }
    unsafe { std::ffi::CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

fn log_jvmti_status(operation: &str, status: i32) {
    if status == 0 {
        log::info!("JVMTI {operation} succeeded: status=0 (JVMTI_ERROR_NONE)");
    } else {
        log::warn!(
            "JVMTI {operation} returned status={status} ({})",
            jvmti_status_name(status)
        );
    }
}

fn capability_value(value: i32) -> &'static str {
    match value {
        0 => "false",
        1 => "true",
        _ => "unknown",
    }
}

fn log_runtime_properties() -> anyhow::Result<()> {
    let mut env = mapping().get_env()?;
    let system = env.find_class("java/lang/System")?;
    for property in [
        "java.version",
        "java.vendor",
        "java.vm.name",
        "java.vm.version",
    ] {
        let key = env.new_string(property)?;
        let key_obj = JObject::from(key);
        let value = env
            .call_static_method(
                &system,
                "getProperty",
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[JValue::Object(&key_obj)],
            )?
            .l()?;
        if value.is_null() {
            log::warn!("Java runtime property {property}=<null>");
        } else {
            let value = env.get_string(&JString::from(value))?.to_str()?.to_owned();
            log::info!("Java runtime {property}={value}");
        }
    }
    Ok(())
}

pub fn uninstall() {
    let registered = unsafe { CLASS_HOOK_REGISTERED };
    if registered {
        unsafe {
            dark_jvmti_class_hook_uninstall();
        }
        unsafe {
            CLASS_HOOK_REGISTERED = false;
        }
        log::info!("ClassFileLoadHook callback removed");
    }
}

#[cfg(test)]
mod transform_tests {
    use super::*;

     
     
    const GRAPH_FIXTURE: &[u8] = include_bytes!("../tests/data/class_8679.class");

    #[test]
    fn scaffold_input_hook_replaces_only_keybind_call_and_preserves_metadata() {
        let source = include_bytes!("../tests/data/class_310.class");
        let params = SCAFFOLD_INPUT_TRANSFORM_PARAMS;
        let bytes = transform_class(source, &[params]).unwrap().unwrap();
        assert!(transform_class(&bytes, &[params]).unwrap().is_none());
        let before = ClassFile::from_bytes(source).unwrap();
        let after = ClassFile::from_bytes(&bytes).unwrap();
        after.verify().unwrap();
        let index = method_index(&after, params.method, params.descriptor);
        for (a, b) in before.methods[index]
            .attributes
            .iter()
            .zip(&after.methods[index].attributes)
        {
            if let (
                Attribute::Code {
                    code: ac,
                    max_stack: ast,
                    max_locals: al,
                    exception_table: ae,
                    attributes: aa,
                    ..
                },
                Attribute::Code {
                    code: bc,
                    max_stack: bst,
                    max_locals: bl,
                    exception_table: be,
                    attributes: ba,
                    ..
                },
            ) = (a, b)
            {
                assert_eq!(ac.len(), bc.len());
                let changed: Vec<_> = ac.iter().zip(bc).filter(|(a, b)| a != b).collect();
                assert_eq!(changed.len(), 1);
                assert!(matches!(changed[0].1, Instruction::Invokestatic(_)));
                assert_eq!((ast, al, ae, aa), (bst, bl, be, ba));
            }
        }
        let dir = crate::config::base_dir().join(".freebuff/scaffold-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("class_310.class"), bytes).unwrap();
    }

    #[test]
    fn scaffold_edge_hook_changes_one_call_and_keeps_other_methods_identical() {
        let source = include_bytes!("../tests/data/class_1657.class");
        let params = SCAFFOLD_EDGE_TRANSFORM_PARAMS;
        let bytes = transform_class(source, &[params]).unwrap().unwrap();
        assert!(transform_class(&bytes, &[params]).unwrap().is_none());
        let before = ClassFile::from_bytes(source).unwrap();
        let after = ClassFile::from_bytes(&bytes).unwrap();
        after.verify().unwrap();
        let index = method_index(&after, params.method, params.descriptor);
        for (i, (a, b)) in before.methods.iter().zip(&after.methods).enumerate() {
            if i != index {
                assert_eq!(a, b, "Scaffold changed another Player method");
            }
        }
        let changed = match (
            &before.methods[index].attributes[0],
            &after.methods[index].attributes[0],
        ) {
            (
                Attribute::Code {
                    code: a,
                    max_stack: sa,
                    max_locals: la,
                    attributes: aa,
                    exception_table: ea,
                    ..
                },
                Attribute::Code {
                    code: b,
                    max_stack: sb,
                    max_locals: lb,
                    attributes: ab,
                    exception_table: eb,
                    ..
                },
            ) => {
                assert_eq!((sa, la, aa, ea), (sb, lb, ab, eb));
                assert_eq!(a.len(), b.len());
                a.iter().zip(b).filter(|(x, y)| x != y).count()
            }
            _ => panic!("Missing edge decision Code"),
        };
        assert_eq!(changed, 1);
        let dir = crate::config::base_dir().join(".freebuff/scaffold-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("class_1657.class"), bytes).unwrap();
    }

     
     
     
     
    fn mapping_entry<'a>(json: &'a str, mojang: &str) -> Option<(String, &'a str)> {
        let key = format!("\"{mojang}\": {{");
        let start = json.find(&key)? + key.len();
         
         
        let end = json[start..]
            .find("\n  \"")
            .map_or(json.len(), |offset| start + offset);
        let block = &json[start..end];
        let name = "\"name\": \"";
        let at = block.find(name)? + name.len();
        let end = block[at..].find('"')? + at;
        Some((block[at..end].to_string(), block))
    }

     
     
    fn declares(block: &str, method: &str, descriptor: &str) -> bool {
        let name = format!("\"name\": \"{method}\"");
        let signature = format!("\"signature\": \"{descriptor}\"");
        let mut from = 0;
        while let Some(found) = block[from..].find(&name) {
            let at = from + found;
            let window = &block[at..block.len().min(at + 256)];
            if window.contains(&signature) {
                return true;
            }
            from = at + name.len();
        }
        false
    }

     
     
     
     
     
     
    #[test]
    fn every_transform_targets_the_intermediary_class_the_mapping_names() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../mappings_intermediary.json");
        let json =
            std::fs::read_to_string(path).expect("the bundled intermediary mapping is readable");

        let optional = [
            SCAFFOLD_INPUT_TRANSFORM_PARAMS,
            SCAFFOLD_EDGE_TRANSFORM_PARAMS,
            CRYSTAL_OPTIMIZER_TRANSFORM_PARAMS,
        ];
        let transforms = base_transforms().iter().chain(optional.iter());
        for params in transforms {
            let mojang = params.class_type.get_name();
            let (runtime, block) = mapping_entry(&json, mojang)
                .unwrap_or_else(|| panic!("{mojang} is not in mappings_intermediary.json"));
            assert_eq!(
                params.target_class, runtime,
                "{} targets {mojang} by the wrong name",
                params.log_label
            );

             
             
            assert!(
                declares(block, params.method, params.descriptor),
                "{}: {runtime} has no {} {}",
                params.log_label,
                params.method,
                params.descriptor
            );
        }
    }

    #[test]
    fn scaffold_does_not_transform_local_player_movement_or_input_methods() {
        let source = include_bytes!("../tests/data/class_746.class");
        for method in ["method_5773", "method_6007"] {
            assert!(
                transform_class(source, &[SCAFFOLD_INPUT_TRANSFORM_PARAMS])
                    .unwrap()
                    .is_none(),
                "Scaffold must not intercept LocalPlayer::{method}"
            );
        }
    }

    #[test]
    fn xray_filters_exact_1_21_4_state_getters_without_loading_argument_local_one() {
        let source = include_bytes!("../tests/data/class_4970$class_4971.class");
        let params = [
            XRAY_RENDER_TRANSFORM_PARAMS,
            XRAY_OCCLUSION_TRANSFORM_PARAMS,
        ];
        let bytes = transform_class(source, &params).unwrap().unwrap();
        assert!(transform_class(&bytes, &params).unwrap().is_none());
        let class = ClassFile::from_bytes(&bytes).unwrap();
        class.verify().unwrap();
        for param in &params {
            let index = method_index(&class, param.method, param.descriptor);
            for attribute in &class.methods[index].attributes {
                if let Attribute::Code {
                    code, max_locals, ..
                } = attribute
                {
                    assert_eq!(*max_locals, 1, "a no-argument getter only has this");
                    assert!(!code.iter().any(|i| matches!(i, Instruction::Aload_1)));
                    assert!(code
                        .iter()
                        .any(|i| matches!(i, Instruction::Invokestatic(_))));
                }
            }
        }
    }

    #[test]
    fn xray_filters_exact_1_21_4_model_fluid_and_block_entity_passes() {
        let sources: &[(&[u8], &[TransformParams])] = &[
            (
                include_bytes!("../tests/data/class_776.class"),
                &[XRAY_BATCHED_TRANSFORM_PARAMS, XRAY_LIQUID_TRANSFORM_PARAMS],
            ),
            (
                include_bytes!("../tests/data/class_824.class"),
                &[XRAY_BLOCK_ENTITY_TRANSFORM_PARAMS],
            ),
        ];
        for (source, params) in sources {
            let original = ClassFile::from_bytes(source).unwrap();
            let bytes = transform_class(source, params).unwrap().unwrap();
            assert!(transform_class(&bytes, params).unwrap().is_none());
            let class = ClassFile::from_bytes(&bytes).unwrap();
            class.verify().unwrap();
            for param in *params {
                let index = method_index(&class, param.method, param.descriptor);
                let (code, _, _, handlers, frames) = code_snapshot(&class, index);
                let old_index = method_index(&original, param.method, param.descriptor);
                let (_, _, _, old_handlers, _) = code_snapshot(&original, old_index);
                assert_eq!(
                    handlers, old_handlers,
                    "original exception handlers preserved"
                );
                let (instructions, byte_shift) = match param.log_label {
                    "XrayBatched" => {
                        assert!(matches!(
                            &code[..4],
                            [
                                Instruction::Aload_1,
                                Instruction::Invokestatic(_),
                                Instruction::Ifeq(4),
                                Instruction::Return
                            ]
                        ));
                        (4, 8)
                    }
                    "XrayLiquid" => {
                        assert!(matches!(
                            &code[..4],
                            [
                                Instruction::Aload(4),
                                Instruction::Invokestatic(_),
                                Instruction::Ifeq(4),
                                Instruction::Return
                            ]
                        ));
                        (4, 9)
                    }
                    _ => {
                        assert!(matches!(
                            &code[..3],
                            [
                                Instruction::Getstatic(_),
                                Instruction::Ifeq(3),
                                Instruction::Return
                            ]
                        ));
                        (3, 7)
                    }
                };
                assert_eq!(frames[0].offset_delta(), instructions);
                let before = debug_table_start_pcs(&original, old_index);
                let after = debug_table_start_pcs(&class, index);
                for ((name, old_pcs), (new_name, new_pcs)) in before.iter().zip(&after) {
                    assert_eq!(name, new_name);
                    let shift = if name == "LineNumberTable" {
                        instructions
                    } else {
                        byte_shift
                    };
                    assert_eq!(
                        *new_pcs,
                        old_pcs.iter().map(|pc| pc + shift).collect::<Vec<_>>()
                    );
                }
            }
        }
    }

    #[test]
    fn smoke_hook_transforms_exact_1_21_4_world_renderer_idempotently() {
        let source = include_bytes!("../tests/data/class_761.class");
        let transforms = [
            LEVEL_RENDERER_TRANSFORM_PARAMS,
            SMOKE_RENDER_TRANSFORM_PARAMS,
        ];
        let bytes = transform_class(source, &transforms).unwrap().unwrap();
        assert!(transform_class(&bytes, &transforms).unwrap().is_none());
        let class_file = ClassFile::from_bytes(&bytes).unwrap();
        class_file.verify().unwrap();
        let index = method_index(
            &class_file,
            SMOKE_RENDER_TRANSFORM_PARAMS.method,
            SMOKE_RENDER_TRANSFORM_PARAMS.descriptor,
        );
        for attribute in &class_file.methods[index].attributes {
            if let Attribute::Code { code, .. } = attribute {
                assert!(matches!(
                    &code[code.len() - 6..],
                    [
                        Instruction::Aload_0,
                        Instruction::Aload(4),
                        Instruction::Aload(6),
                        Instruction::Aload(7),
                        Instruction::Invokestatic(_),
                        Instruction::Return
                    ]
                ));
            }
        }
        let dir = std::path::Path::new("../target/smoke-verification");
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("class_761_transformed.class"), bytes).unwrap();
    }

     
     
     
    #[test]
    fn cloud_guard_transforms_exact_1_21_4_level_renderer_idempotently() {
        let source = include_bytes!("../tests/data/class_761.class");
        let transforms = [CLOUDS_HIDE_TRANSFORM_PARAMS];
        let bytes = transform_class(source, &transforms).unwrap().unwrap();
        assert!(
            transform_class(&bytes, &transforms).unwrap().is_none(),
            "idempotent hot-reload"
        );
        let class_file = ClassFile::from_bytes(&bytes).unwrap();
        class_file.verify().unwrap();
        let index = method_index(
            &class_file,
            CLOUDS_HIDE_TRANSFORM_PARAMS.method,
            CLOUDS_HIDE_TRANSFORM_PARAMS.descriptor,
        );
        let (code, _, _, _, frames) = code_snapshot(&class_file, index);
        assert!(matches!(
            &code[..3],
            [
                Instruction::Getstatic(_),
                Instruction::Ifeq(3),
                Instruction::Return
            ]
        ));
        assert_eq!(frames[0].offset_delta(), 3);
    }

     
     
     
     
    #[test]
    fn a_guard_and_a_prepend_call_compose_draw_first_then_skip() {
        const PARAMS: TransformParams = TransformParams {
            target_class: "net/minecraft/class_761",
            method: "method_32133",
            descriptor: "(Lnet/minecraft/class_243;Lorg/joml/Matrix4f;Lorg/joml/Matrix4f;)V",
            bridge_class: "InferedSkyBridge",
            bridge_method: "drawSky",
            bridge_descriptor: "()V",
            class_type: MinecraftClassType::LevelRenderer,
            log_label: "SkyGuardDrawComposition",
            replace_entire_body: false,
            replace_with_bridge_return: false,
            guard_flag: Some("HIDE_VANILLA"),
            prepend_object_call: Some(PrependCall {
                arguments: &[],
                destination: None,
            }),
        };

        let source = include_bytes!("../tests/data/class_761.class");
        let bytes = transform_class(source, &[PARAMS]).unwrap().unwrap();
        assert!(
            transform_class(&bytes, &[PARAMS]).unwrap().is_none(),
            "idempotent hot-reload"
        );
        let class_file = ClassFile::from_bytes(&bytes).unwrap();
        class_file.verify().unwrap();
        let index = method_index(&class_file, PARAMS.method, PARAMS.descriptor);
        let (code, _, _, _, frames) = code_snapshot(&class_file, index);
        assert!(
            matches!(code[0], Instruction::Invokestatic(_)),
            "the draw call is the first thing the method does: {:?}",
            &code[..5]
        );
        assert!(matches!(code[1], Instruction::Getstatic(_)));
        assert!(
            matches!(code[2], Instruction::Ifeq(4)),
            "the guard jumps over its own RETURN, to the vanilla body: {:?}",
            &code[..6]
        );
        assert!(matches!(code[3], Instruction::Return));
        assert!(
            matches!(code[4], Instruction::Aload_0),
            "the body follows both prologues: {:?}",
            &code[..6]
        );
        assert_eq!(
            frames[0].offset_delta(),
            4,
            "the guard's frame follows the draw call"
        );
    }

     
     
     
    #[test]
    fn crosshair_guard_transforms_exact_1_21_4_gui_idempotently() {
        let source = include_bytes!("../tests/data/class_329.class");
        let transforms = [CROSSHAIR_HIDE_TRANSFORM_PARAMS];
        let bytes = transform_class(source, &transforms).unwrap().unwrap();
        assert!(
            transform_class(&bytes, &transforms).unwrap().is_none(),
            "idempotent hot-reload"
        );
        let class_file = ClassFile::from_bytes(&bytes).unwrap();
        class_file.verify().unwrap();
        let index = method_index(
            &class_file,
            CROSSHAIR_HIDE_TRANSFORM_PARAMS.method,
            CROSSHAIR_HIDE_TRANSFORM_PARAMS.descriptor,
        );
        let (code, _, _, _, _) = code_snapshot(&class_file, index);
        assert!(matches!(
            &code[..3],
            [
                Instruction::Getstatic(_),
                Instruction::Ifeq(3),
                Instruction::Return
            ]
        ));
        assert!(
            code.len() > 20,
            "the vanilla reticle body must still follow the guard"
        );
    }

     
    fn branch_target(instruction: &Instruction) -> Option<u16> {
        Some(match instruction {
            Instruction::Ifeq(target)
            | Instruction::Ifne(target)
            | Instruction::Iflt(target)
            | Instruction::Ifge(target)
            | Instruction::Ifgt(target)
            | Instruction::Ifle(target)
            | Instruction::If_icmpeq(target)
            | Instruction::If_icmpne(target)
            | Instruction::If_icmplt(target)
            | Instruction::If_icmpge(target)
            | Instruction::If_icmpgt(target)
            | Instruction::If_icmple(target)
            | Instruction::If_acmpeq(target)
            | Instruction::If_acmpne(target)
            | Instruction::Ifnull(target)
            | Instruction::Ifnonnull(target)
            | Instruction::Goto(target)
            | Instruction::Jsr(target) => *target,
            _ => return None,
        })
    }

     
     
     
    #[test]
    fn totem_pop_hook_swaps_the_scale_call_for_the_bridge_idempotently() {
        let source = include_bytes!("../tests/data/class_757.class");
        let transforms = [TOTEM_POP_TRANSFORM_PARAMS];
        let bytes = transform_class(source, &transforms).unwrap().unwrap();
        assert!(
            transform_class(&bytes, &transforms).unwrap().is_none(),
            "idempotent hot-reload"
        );
        let class_file = ClassFile::from_bytes(&bytes).unwrap();
        class_file.verify().unwrap();
        let index = method_index(
            &class_file,
            TOTEM_POP_TRANSFORM_PARAMS.method,
            TOTEM_POP_TRANSFORM_PARAMS.descriptor,
        );
        let (code, _, _, _, _) = code_snapshot(&class_file, index);
        let bridge_calls: Vec<u16> = code
            .iter()
            .filter_map(|instruction| match instruction {
                Instruction::Invokestatic(index) => Some(*index),
                _ => None,
            })
            .collect();
        assert_eq!(bridge_calls.len(), 1, "exactly one scale call is routed");
        assert!(is_bridge_method(
            &class_file.constant_pool,
            bridge_calls[0],
            &TOTEM_POP_TRANSFORM_PARAMS,
        ));
        assert!(
            !code
                .iter()
                .any(|instruction| matches!(instruction, Instruction::Invokevirtual(_))),
            "the vanilla scale call must be gone"
        );
    }

     
     
     
     
     
     
     
     
    #[test]
    fn item_scale_hook_transforms_exact_1_21_4_item_in_hand_renderer_idempotently() {
        let source = include_bytes!("../tests/data/class_759.class");
        let original = ClassFile::from_bytes(source).unwrap();
        let original_index = method_index(
            &original,
            ITEM_SCALE_TRANSFORM_PARAMS.method,
            ITEM_SCALE_TRANSFORM_PARAMS.descriptor,
        );
        let (original_code, _, _, original_handlers, original_frames) =
            code_snapshot(&original, original_index);
        assert!(
            !original_frames.is_empty(),
            "the real item pass must carry a StackMapTable to move"
        );
        let before_debug = debug_table_start_pcs(&original, original_index);

        let transforms = [ITEM_SCALE_TRANSFORM_PARAMS];
        let bytes = transform_class(source, &transforms).unwrap().unwrap();
        assert!(
            transform_class(&bytes, &transforms).unwrap().is_none(),
            "idempotent hot-reload"
        );
        let class_file = ClassFile::from_bytes(&bytes).unwrap();
        class_file.verify().unwrap();
        let index = method_index(
            &class_file,
            ITEM_SCALE_TRANSFORM_PARAMS.method,
            ITEM_SCALE_TRANSFORM_PARAMS.descriptor,
        );
        let (code, _, _, handlers, frames) = code_snapshot(&class_file, index);

         
         
        assert!(matches!(
            &code[..4],
            [
                Instruction::Aload(4),
                Instruction::Aload(8),
                Instruction::Invokestatic(_),
                Instruction::Aload_1
            ]
        ));
        assert_eq!(code.len(), original_code.len() + 3);
        assert_eq!(
            handlers, original_handlers,
            "exception handlers must survive"
        );

         
         
         
        let mut branches = 0;
        for (offset, before) in original_code.iter().enumerate() {
            let after = &code[offset + 3];
            if let Some(target) = branch_target(before) {
                branches += 1;
                assert_eq!(
                    branch_target(after),
                    Some(target + 3),
                    "the branch at instruction {offset} must move with the body"
                );
                continue;
            }
            assert_eq!(
                before, after,
                "instruction {offset} must be preserved verbatim"
            );
        }
        assert!(
            branches > 1,
            "the real item pass branches, or this proves nothing"
        );

        let after_debug = debug_table_start_pcs(&class_file, index);
         
         
         
        assert!(
            after_debug
                .iter()
                .all(|(name, _)| name != "LocalVariableTable"),
            "a flat byte shift cannot move this switch-bearing method's LVT"
        );
        let before_lnt = &before_debug
            .iter()
            .find(|(name, _)| name == "LineNumberTable")
            .expect("fixture has a LineNumberTable")
            .1;
        let after_lnt = &after_debug
            .iter()
            .find(|(name, _)| name == "LineNumberTable")
            .expect("LineNumberTable survives")
            .1;
        assert_eq!(
            *after_lnt,
            before_lnt.iter().map(|pc| pc + 3).collect::<Vec<_>>(),
            "LineNumberTable must shift by its instruction offset space"
        );

         
         
        assert_eq!(frames.len(), original_frames.len());
        assert_eq!(
            frames[0].offset_delta(),
            original_frames[0].offset_delta() + 3
        );
        for (before, after) in original_frames.iter().skip(1).zip(frames.iter().skip(1)) {
            assert_eq!(
                before.offset_delta(),
                after.offset_delta(),
                "later frame deltas are stated relative to their predecessor"
            );
        }

        let dir = std::path::Path::new("../target/item-scale-verification");
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("class_759_transformed.class"), bytes).unwrap();
    }

     
     
     
     
     
     
     
    #[test]
    fn invisible_shield_hook_replaces_the_real_item_stack_local_and_keeps_the_body() {
        let source = include_bytes!("../tests/data/class_759.class");
        let original = ClassFile::from_bytes(source).unwrap();
        let index = method_index(
            &original,
            SHIELD_TRANSFORM_PARAMS.method,
            SHIELD_TRANSFORM_PARAMS.descriptor,
        );
        let (original_code, _, _, original_handlers, original_frames) =
            code_snapshot(&original, index);
        assert!(
            !original_frames.is_empty(),
            "the real item pass must carry a StackMapTable to move"
        );

        let transforms = [ITEM_SCALE_TRANSFORM_PARAMS, SHIELD_TRANSFORM_PARAMS];
        let bytes = transform_class(source, &transforms).unwrap().unwrap();
        assert!(
            transform_class(&bytes, &transforms).unwrap().is_none(),
            "idempotent hot-reload"
        );
        let class_file = ClassFile::from_bytes(&bytes).unwrap();
        class_file.verify().unwrap();
        let (code, _, _, handlers, frames) = code_snapshot(&class_file, index);

         
        const SHIELD_PROLOGUE: usize = 5;
         
        const SCALE_PROLOGUE: usize = 3;
        assert!(matches!(
            &code[..SHIELD_PROLOGUE + SCALE_PROLOGUE + 1],
            [
                Instruction::Aload(4),
                Instruction::Aload(6),
                Instruction::Invokestatic(_),
                Instruction::Checkcast(_),
                Instruction::Astore(6),
                Instruction::Aload(4),
                Instruction::Aload(8),
                Instruction::Invokestatic(_),
                Instruction::Aload_1,
            ]
        ));
        let cast = match &code[3] {
            Instruction::Checkcast(cast) => *cast,
            other => panic!("expected the shield prologue's CHECKCAST, found {other:?}"),
        };
        assert_eq!(
            class_file
                .constant_pool
                .try_get_class(cast)
                .unwrap()
                .to_string(),
            "net/minecraft/class_1799",
            "the local must be restored to the ItemStack the pass declares"
        );

        let shift = SHIELD_PROLOGUE + SCALE_PROLOGUE;
         
        let shift_delta = shift as u16;
        assert_eq!(code.len(), original_code.len() + shift);
        assert_eq!(
            handlers, original_handlers,
            "exception handlers must survive"
        );
        let mut branches = 0;
        for (offset, before) in original_code.iter().enumerate() {
            let after = &code[offset + shift];
            if let Some(target) = branch_target(before) {
                branches += 1;
                assert_eq!(
                    branch_target(after),
                    Some(target + shift_delta),
                    "the branch at instruction {offset} must move with the body"
                );
                continue;
            }
            assert_eq!(
                before, after,
                "instruction {offset} must be preserved verbatim"
            );
        }
        assert!(
            branches > 1,
            "the real item pass branches, or this proves nothing"
        );
        assert_eq!(frames.len(), original_frames.len());
        assert_eq!(
            frames[0].offset_delta(),
            original_frames[0].offset_delta() + shift_delta
        );
        for (before, after) in original_frames.iter().skip(1).zip(frames.iter().skip(1)) {
            assert_eq!(before.offset_delta(), after.offset_delta());
        }

         
         
         
         
         
         
        let before_debug = debug_table_start_pcs(&original, index);
        let after_debug = debug_table_start_pcs(&class_file, index);
        assert!(
            after_debug
                .iter()
                .all(|(name, _)| name != "LocalVariableTable"),
            "a flat byte shift cannot move this switch-bearing method's LVT"
        );
        let before_lnt = &before_debug
            .iter()
            .find(|(name, _)| name == "LineNumberTable")
            .expect("fixture has a LineNumberTable")
            .1;
        let after_lnt = &after_debug
            .iter()
            .find(|(name, _)| name == "LineNumberTable")
            .expect("LineNumberTable survives")
            .1;
        assert_eq!(
            *after_lnt,
            before_lnt
                .iter()
                .map(|pc| pc + shift_delta)
                .collect::<Vec<_>>(),
            "LineNumberTable must shift by its instruction offset space"
        );
    }

     
     
     
     
     
    #[test]
    fn all_item_pass_hooks_reparse_together_on_the_real_class() {
        let source = include_bytes!("../tests/data/class_759.class");
        let transforms = [
            ITEM_SCALE_TRANSFORM_PARAMS,
            SWING_ECHO_TRANSFORM_PARAMS,
            HAND_OUTLINE_TRANSFORM_PARAMS,
            HAND_AURA_AFTER_TRANSFORM_PARAMS,
            SHIELD_TRANSFORM_PARAMS,
        ];
        let bytes = transform_class(source, &transforms)
            .unwrap()
            .expect("the item pass matches the hooks");
        ClassFile::from_bytes(&bytes).expect("the combined pass reparses");
    }

     
     
     
     
    #[test]
    fn hand_aura_after_hook_splices_the_terminal_return_of_the_two_hand_render() {
        let source = include_bytes!("../tests/data/class_759.class");
        let original = ClassFile::from_bytes(source).unwrap();
        let original_index = method_index(
            &original,
            HAND_AURA_AFTER_TRANSFORM_PARAMS.method,
            HAND_AURA_AFTER_TRANSFORM_PARAMS.descriptor,
        );
        let (original_code, _, _, original_handlers, original_frames) =
            code_snapshot(&original, original_index);
        assert_eq!(
            original_code.last(),
            Some(&Instruction::Return),
            "the two-hand render must end in the RETURN the hook replaces"
        );
        assert_eq!(
            original_code
                .iter()
                .filter(|instruction| matches!(instruction, Instruction::Return))
                .count(),
            1,
            "a second RETURN would skip the aura"
        );

        let transforms = [HAND_AURA_AFTER_TRANSFORM_PARAMS];
        let bytes = transform_class(source, &transforms).unwrap().unwrap();
        assert!(
            transform_class(&bytes, &transforms).unwrap().is_none(),
            "idempotent hot-reload"
        );
        let class_file = ClassFile::from_bytes(&bytes).unwrap();
        class_file.verify().unwrap();
        let index = method_index(
            &class_file,
            HAND_AURA_AFTER_TRANSFORM_PARAMS.method,
            HAND_AURA_AFTER_TRANSFORM_PARAMS.descriptor,
        );
        let (code, _, _, handlers, frames) = code_snapshot(&class_file, index);

         
         
        let tail = code.len() - 3;
        assert_eq!(code[..tail], original_code[..original_code.len() - 1]);
        assert!(matches!(
            &code[tail..],
            [
                Instruction::Aload_0,
                Instruction::Invokestatic(_),
                Instruction::Return
            ]
        ));
        assert_eq!(
            handlers, original_handlers,
            "exception handlers must survive"
        );
         
         
        assert_eq!(frames.len(), original_frames.len());
        for (before, after) in original_frames.iter().zip(frames.iter()) {
            assert_eq!(before.offset_delta(), after.offset_delta());
        }
    }

     
     
     
     
     
     
    #[test]
    fn the_invisible_shield_hook_locals_are_the_real_item_pass_slots() {
        let class_file =
            ClassFile::from_bytes(include_bytes!("../tests/data/class_759.class")).unwrap();
        let index = method_index(
            &class_file,
            SHIELD_TRANSFORM_PARAMS.method,
            SHIELD_TRANSFORM_PARAMS.descriptor,
        );
        let mut locals = Vec::new();
        for attribute in &class_file.methods[index].attributes {
            let Attribute::Code { attributes, .. } = attribute else {
                continue;
            };
            for code_attribute in attributes {
                let Attribute::LocalVariableTable { variables, .. } = code_attribute else {
                    continue;
                };
                for variable in variables {
                    locals.push((
                        variable.index,
                        class_file
                            .constant_pool
                            .try_get_utf8(variable.descriptor_index)
                            .unwrap()
                            .to_string(),
                    ));
                }
            }
        }
        let declared = |slot: u8| {
            let matches: Vec<&str> = locals
                .iter()
                .filter(|(index, _)| *index == u16::from(slot))
                .map(|(_, descriptor)| descriptor.as_str())
                .collect();
            assert!(!matches.is_empty(), "slot {slot} must be a declared local");
            matches
        };
        for descriptor in declared(SHIELD_LOCALS[0].slot()) {
            assert_eq!(descriptor, "Lnet/minecraft/class_1268;", "the hand local");
        }
        for descriptor in declared(SHIELD_ITEM_LOCAL) {
            assert_eq!(descriptor, "Lnet/minecraft/class_1799;", "the item local");
        }
         
         
        assert_eq!(
            descriptor_local_type(SHIELD_TRANSFORM_PARAMS.descriptor, SHIELD_ITEM_LOCAL, false)
                .unwrap(),
            "Lnet/minecraft/class_1799;"
        );
    }

     
     
     
     
     
     
    #[test]
    fn the_item_scale_off_hand_keys_on_the_real_interaction_hand_constant_names() {
        let class_file =
            ClassFile::from_bytes(include_bytes!("../tests/data/class_1268.class")).unwrap();
        assert_eq!(
            class_file.class_name().unwrap().to_string(),
            "net/minecraft/class_1268"
        );
        let strings: Vec<String> = (1..=class_file.constant_pool.len())
            .filter_map(|index| {
                class_file
                    .constant_pool
                    .try_get_utf8(index as u16)
                    .map(|value| value.to_string())
                    .ok()
            })
            .collect();
        for name in ["MAIN_HAND", "OFF_HAND"] {
            assert!(
                strings.iter().any(|value| value == name),
                "the InteractionHand enum literal {name} is gone"
            );
        }
         
        assert!(ITEM_SCALE_DESCRIPTOR.contains("Lnet/minecraft/class_1268;"));
    }

    fn method_index(class_file: &ClassFile<'_>, name: &str, descriptor: &str) -> usize {
        class_file
            .methods
            .iter()
            .position(|method| {
                class_file
                    .constant_pool
                    .try_get_utf8(method.name_index)
                    .map(|v| v.to_string())
                    .ok()
                    .as_deref()
                    == Some(name)
                    && class_file
                        .constant_pool
                        .try_get_utf8(method.descriptor_index)
                        .map(|v| v.to_string())
                        .ok()
                        .as_deref()
                        == Some(descriptor)
            })
            .unwrap()
    }

     
    #[allow(clippy::type_complexity)]
    fn code_snapshot(
        class_file: &ClassFile<'_>,
        method_index: usize,
    ) -> (Vec<Instruction>, u16, u16, usize, Vec<StackFrame>) {
        for attribute in &class_file.methods[method_index].attributes {
            if let Attribute::Code {
                code,
                max_stack,
                max_locals,
                exception_table,
                attributes: code_attributes,
                ..
            } = attribute
            {
                let mut frames = Vec::new();
                for code_attribute in code_attributes {
                    if let Attribute::StackMapTable { frames: table, .. } = code_attribute {
                        frames = table.clone();
                        break;
                    }
                }
                return (
                    code.clone(),
                    *max_stack,
                    *max_locals,
                    exception_table.len(),
                    frames,
                );
            }
        }
        panic!("method has no Code attribute")
    }

     
     
     
     
    fn debug_table_start_pcs(
        class_file: &ClassFile<'_>,
        method_index: usize,
    ) -> Vec<(String, Vec<u16>)> {
        let mut out: Vec<(String, Vec<u16>)> = Vec::new();

        for attribute in &class_file.methods[method_index].attributes {
            let Attribute::Code {
                attributes: code_attributes,
                ..
            } = attribute
            else {
                continue;
            };
            for code_attribute in code_attributes {
                let table = match code_attribute {
                    Attribute::LineNumberTable { line_numbers, .. } => (
                        "LineNumberTable".to_string(),
                        line_numbers.iter().map(|line| line.start_pc).collect(),
                    ),
                    Attribute::LocalVariableTable { variables, .. } => (
                        "LocalVariableTable".to_string(),
                        variables.iter().map(|entry| entry.start_pc).collect(),
                    ),
                    Attribute::LocalVariableTypeTable { variable_types, .. } => (
                        "LocalVariableTypeTable".to_string(),
                        variable_types.iter().map(|entry| entry.start_pc).collect(),
                    ),
                    _ => continue,
                };
                out.push(table);
            }
        }
        out
    }

     
     
     
     
     
     
     
     
     
     
    #[test]
    fn guard_prologue_shifts_debug_tables_in_the_right_offset_space() {
        let original = ClassFile::from_bytes(GRAPH_FIXTURE).unwrap();
        let original_idx = method_index(
            &original,
            GRAPH_RUN_PARTIAL_METHOD,
            GRAPH_RUN_PARTIAL_DESCRIPTOR,
        );
        let before = debug_table_start_pcs(&original, original_idx);
        for required in ["LocalVariableTable", "LineNumberTable"] {
            assert!(
                before.iter().any(|(name, _)| name == required),
                "fixture must carry a {required}, otherwise this test cannot regress"
            );
        }

        let transformed =
            crate::jvmti::transform_class(GRAPH_FIXTURE, &[GRAPH_RUN_PARTIAL_TRANSFORM_PARAMS])
                .unwrap()
                .unwrap();
        let class_file = ClassFile::from_bytes(&transformed).unwrap();
        let idx = method_index(
            &class_file,
            GRAPH_RUN_PARTIAL_METHOD,
            GRAPH_RUN_PARTIAL_DESCRIPTOR,
        );
        let after = debug_table_start_pcs(&class_file, idx);

        assert_eq!(
            before.len(),
            after.len(),
            "the splice must not add or drop a debug table"
        );
        for ((before_name, before_pcs), (after_name, after_pcs)) in before.iter().zip(after.iter())
        {
            assert_eq!(before_name, after_name, "debug table order changed");
            assert_eq!(
                before_pcs.len(),
                after_pcs.len(),
                "{before_name} entry count changed"
            );
             
             
             
            let expected_shift = match before_name.as_str() {
                "LineNumberTable" => u32::from(GUARD_PREFIX_INSTRUCTIONS),
                "LocalVariableTable" | "LocalVariableTypeTable" => u32::from(GUARD_PREFIX_BYTES),
                other => panic!("unexpected debug table {other}"),
            };
            for (before_pc, after_pc) in before_pcs.iter().zip(after_pcs.iter()) {
                assert_eq!(
                    u32::from(*after_pc),
                    u32::from(*before_pc) + expected_shift,
                    "{before_name} start_pc must move by {expected_shift} in its own offset space \
                     (index shift {GUARD_PREFIX_INSTRUCTIONS}, byte shift {GUARD_PREFIX_BYTES})"
                );
            }
        }
    }

    #[test]
    fn fixture_parses_run_partial_exactly() {
        let class_file = ClassFile::from_bytes(GRAPH_FIXTURE).unwrap();
        assert_eq!(
            class_file.class_name().unwrap().to_string(),
            "net/minecraft/class_8679"
        );
        let idx = method_index(
            &class_file,
            GRAPH_RUN_PARTIAL_METHOD,
            GRAPH_RUN_PARTIAL_DESCRIPTOR,
        );
        let (code, _, max_locals, handlers, frames) = code_snapshot(&class_file, idx);
         
         
         
        assert_eq!(code.len(), 60, "expected the known 60-instruction body");
        assert_eq!(max_locals, 10);
        assert_eq!(handlers, 0);
        assert_eq!(frames.first().map(StackFrame::offset_delta), Some(15));
    }

    #[test]
    fn guard_transform_is_idempotent() {
         
         
         
        let first =
            crate::jvmti::transform_class(GRAPH_FIXTURE, &[GRAPH_RUN_PARTIAL_TRANSFORM_PARAMS]);
        let transformed = first
            .expect("transform must not err")
            .expect("must transform");
        let second =
            crate::jvmti::transform_class(&transformed, &[GRAPH_RUN_PARTIAL_TRANSFORM_PARAMS]);
        assert!(
            second.expect("second pass must not err").is_none(),
            "already-injected marker must suppress a second splice"
        );
    }

    #[test]
    fn guard_prologue_structure_and_verifies() {
        let result =
            crate::jvmti::transform_class(GRAPH_FIXTURE, &[GRAPH_RUN_PARTIAL_TRANSFORM_PARAMS]);
        let transformed = result.unwrap().unwrap();
        let class_file = ClassFile::from_bytes(&transformed).unwrap();
        class_file.verify().expect("transformed class must verify");

        let idx = method_index(
            &class_file,
            GRAPH_RUN_PARTIAL_METHOD,
            GRAPH_RUN_PARTIAL_DESCRIPTOR,
        );
        let (code, max_stack, _, _, frames) = code_snapshot(&class_file, idx);

         
        assert_eq!(
            code.len(),
            63,
            "original 60 shifted by GUARD_PREFIX_INSTRUCTIONS=3"
        );
        let Instruction::Getstatic(flag_ref) = code[0] else {
            panic!("expected GETSTATIC at index 0");
        };
        assert!(matches!(
            class_file.constant_pool.get(flag_ref),
            Some(Constant::FieldRef { .. })
        ));
        let Instruction::Ifeq(target) = code[1] else {
            panic!("expected IFEQ at index 1");
        };
        assert_eq!(
            target, 3,
            "vanilla branch must land on the first original instruction"
        );
        let Instruction::Return = code[2] else {
            panic!("expected RETURN at index 2");
        };
         
        assert_eq!(
            code[3],
            Instruction::Aload_0,
            "original first instruction preserved"
        );
        assert!(max_stack >= 1);

         
         
        assert!(
            frames.len() >= 2,
            "one prefixed frame plus at least the original head frame"
        );
        let head = frames.first().unwrap();
        assert_eq!(
            head.offset_delta(),
            3,
            "entry frame must cover the first original instruction"
        );
        let StackFrame::FullFrame { locals, stack, .. } = head else {
            panic!("head frame must be a FullFrame");
        };
        assert!(stack.is_empty(), "branch edge must carry an empty stack");
        assert!(
            matches!(locals.first(), Some(VerificationType::Object { cpool_index }) if *cpool_index == class_file.this_class)
        );
        assert_eq!(locals.len(), 6, "this + 5 parameters");

         
         
         
         
        let absolute_offsets: Vec<u64> = {
            let mut acc = 0u64;
            frames
                .iter()
                .map(|frame| {
                    acc += u64::from(frame.offset_delta()) + 1;
                    acc - 1
                })
                .collect()
        };
        assert_eq!(absolute_offsets.first(), Some(&3), "head frame at index 3");
         
         
         
         
        for pair in absolute_offsets.windows(2) {
            assert!(
                pair[0] < pair[1],
                "stackmap offsets must be strictly ascending"
            );
        }

         
         
         
         
         
         
         
        let dump = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/class_8679_guarded.class");
        std::fs::write(&dump, &transformed).unwrap();
    }
}
