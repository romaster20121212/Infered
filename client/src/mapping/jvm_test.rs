 
 
 
 
 
 

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

use jni::objects::JValue;
use jni::{InitArgsBuilder, JNIVersion, JavaVM};

use crate::mapping::Mapping;

#[test]
fn scaffold_embedded_bridge_loads_without_helpers_on_game_classpath() {
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    let urls = env
        .new_object_array(0, "java/net/URL", jni::objects::JObject::null())
        .unwrap();
    let loader = env
        .new_object(
            "java/net/URLClassLoader",
            "([Ljava/net/URL;Ljava/lang/ClassLoader;)V",
            &[
                JValue::Object(&urls),
                JValue::Object(&jni::objects::JObject::null()),
            ],
        )
        .unwrap();
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
        env.define_class(name, &loader, bytes)
            .expect("embedded sibling must load in isolated game loader");
    }
    let bridge = env
        .define_class(
            "InferedScaffoldBridge",
            &loader,
            include_bytes!(env!("INFERED_SCAFFOLD_BRIDGE_CLASS")),
        )
        .unwrap();
     
    assert!(!env
        .get_static_field(&bridge, "ACTIVE", "Z")
        .unwrap()
        .z()
        .unwrap());
    let turn = env
        .call_static_method(
            &bridge,
            "angularTurn",
            "(FFFF)D",
            &[
                JValue::Float(179.0),
                JValue::Float(0.0),
                JValue::Float(-179.0),
                JValue::Float(0.0),
            ],
        )
        .unwrap()
        .d()
        .unwrap();
    assert!(turn < 3.0_f64.to_radians());
}

#[test]
fn fake_lag_keeps_movement_delayed_during_live_hits() {
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    let result = env.call_static_method("FakeLagNettyProbe", "run", "()V", &[]);
    if result.is_err() {
        env.exception_describe().unwrap();
        let _ = env.exception_clear();
    }
    result.expect("Fake Lag outbound-delay regression probe");
}

#[test]
fn scaffold_java_bridge_runs_real_aim_click_and_cleanup_regressions() {
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    let args = env
        .new_object_array(0, "java/lang/String", jni::objects::JObject::null())
        .unwrap();
    let result = env.call_static_method(
        "ScaffoldProbe",
        "main",
        "([Ljava/lang/String;)V",
        &[JValue::Object(&args)],
    );
    if result.is_err() {
        env.exception_describe().unwrap();
    }
    result.expect("actual scaffold Java bridge regression probe");
}

#[test]
fn fast_scaffold_embedded_classes_load_without_game_classpath_helpers() {
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    let urls = env
        .new_object_array(0, "java/net/URL", jni::objects::JObject::null())
        .unwrap();
    let loader = env
        .new_object(
            "java/net/URLClassLoader",
            "([Ljava/net/URL;Ljava/lang/ClassLoader;)V",
            &[
                JValue::Object(&urls),
                JValue::Object(&jni::objects::JObject::null()),
            ],
        )
        .unwrap();
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
        (
            "InferedFastScaffoldBridge",
            include_bytes!(env!("INFERED_FAST_SCAFFOLD_BRIDGE_CLASS")).as_slice(),
        ),
    ] {
        env.define_class(name, &loader, bytes)
            .expect("copy's embedded helpers load in isolated loader");
    }
    let class = env
        .call_method(
            &loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(
                &env.new_string("InferedFastScaffoldBridge").unwrap(),
            )],
        )
        .unwrap()
        .l()
        .unwrap();
    let class = jni::objects::JClass::from(class);
    assert!(!env
        .get_static_field(&class, "ACTIVE", "Z")
        .unwrap()
        .z()
        .unwrap());
}

#[test]
fn fast_scaffold_java_copy_runs_placement_and_cadence_regressions() {
     
     
    let output = Command::new("java")
        .arg("-cp")
        .arg(fixture_classes())
        .arg("FastScaffoldProbe")
        .output()
        .expect("launch Fast Scaffold regression probe");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{}", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn xray_no_argument_getters_preserve_vanilla_and_filter_selected_states() {
    let mut shape = crate::jvmti::XRAY_RENDER_TRANSFORM_PARAMS;
    shape.target_class = "XrayRenderProbe";
    shape.method = "renderShape";
    shape.descriptor = "()LXrayRenderProbe$Shape;";
    let occlusion = crate::jvmti::TransformParams {
        method: "canOcclude",
        descriptor: "()Z",
        bridge_method: "decideOcclusion",
        bridge_descriptor: "(Z)Z",
        ..shape
    };
    let source = std::fs::read(fixture_classes().join("XrayRenderProbe.class")).unwrap();
    let transforms = [shape, occlusion];
    let bytes = crate::jvmti::transform_class(&source, &transforms)
        .unwrap()
        .unwrap();
    assert!(crate::jvmti::transform_class(&bytes, &transforms)
        .unwrap()
        .is_none());
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    let loader = env
        .call_static_method(
            "java/lang/ClassLoader",
            "getSystemClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )
        .unwrap()
        .l()
        .unwrap();
    let class = env
        .define_class("XrayRenderProbe", &loader, &bytes)
        .expect("Xray getter must verify with debug LocalVariableTable and no local 1");
    let object = env.new_object(&class, "()V", &[]).unwrap();
    let bridge = env.find_class("InferedXrayBridge").unwrap();
    let invisible = env
        .get_static_field(
            "XrayRenderProbe$Shape",
            "INVISIBLE",
            "LXrayRenderProbe$Shape;",
        )
        .unwrap()
        .l()
        .unwrap();
    let model = env
        .get_static_field("XrayRenderProbe$Shape", "MODEL", "LXrayRenderProbe$Shape;")
        .unwrap()
        .l()
        .unwrap();
    env.set_static_field(
        &bridge,
        (&bridge, "INVISIBLE", "Ljava/lang/Object;"),
        JValue::Object(&invisible),
    )
    .unwrap();
    let map = env
        .new_object("java/util/IdentityHashMap", "()V", &[])
        .unwrap();
    let targets = env
        .call_static_method(
            "java/util/Collections",
            "newSetFromMap",
            "(Ljava/util/Map;)Ljava/util/Set;",
            &[JValue::Object(&map)],
        )
        .unwrap()
        .l()
        .unwrap();
    env.set_static_field(
        &bridge,
        (&bridge, "TARGET_STATES", "Ljava/util/Set;"),
        JValue::Object(&targets),
    )
    .unwrap();
    for active in [false, true, false, true] {
        env.set_static_field(
            &bridge,
            (&bridge, "ACTIVE", "Z"),
            JValue::Bool(active.into()),
        )
        .unwrap();
        let actual = env
            .call_method(&object, "renderShape", shape.descriptor, &[])
            .unwrap()
            .l()
            .unwrap();
        assert!(env
            .is_same_object(&actual, if active { &invisible } else { &model })
            .unwrap());
        assert_eq!(
            env.call_method(&object, "canOcclude", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            !active
        );
    }
    env.call_method(
        &targets,
        "add",
        "(Ljava/lang/Object;)Z",
        &[JValue::Object(&object)],
    )
    .unwrap();
    let actual = env
        .call_method(&object, "renderShape", shape.descriptor, &[])
        .unwrap()
        .l()
        .unwrap();
    assert!(env.is_same_object(&actual, &model).unwrap());
    assert_eq!(
        env.get_field(&object, "vanillaCalls", "I")
            .unwrap()
            .i()
            .unwrap(),
        5
    );
     
    let batch = crate::jvmti::TransformParams {
        target_class: "XrayPassProbe", method: "batch",
        descriptor: "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;ZLjava/lang/Object;)V",
        log_label: "XrayBatched", bridge_method: "shouldHide", bridge_descriptor: "(Ljava/lang/Object;)Z",
        replace_with_bridge_return: false, guard_flag: Some("ACTIVE"), ..shape
    };
    let liquid = crate::jvmti::TransformParams { method: "liquid",
        descriptor: "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)V",
        log_label: "XrayLiquid", ..batch };
    let block_entity = crate::jvmti::TransformParams {
        method: "blockEntity",
        descriptor: "(Ljava/lang/Object;FLjava/lang/Object;Ljava/lang/Object;)V",
        log_label: "XrayBlockEntity",
        bridge_method: "",
        bridge_descriptor: "",
        ..batch
    };
    let pass_params = [batch, liquid, block_entity];
    let source = std::fs::read(fixture_classes().join("XrayPassProbe.class")).unwrap();
    let patched = crate::jvmti::transform_class(&source, &pass_params)
        .unwrap()
        .unwrap();
    assert!(crate::jvmti::transform_class(&patched, &pass_params)
        .unwrap()
        .is_none());
    let pass_class = env
        .define_class("XrayPassProbe", &loader, &patched)
        .unwrap();
    let pass = env.new_object(&pass_class, "()V", &[]).unwrap();
    let unselected = env.new_object("java/lang/Object", "()V", &[]).unwrap();
    let unused = jni::objects::JObject::null();
    for (active, state, expected) in [
        (false, &unselected, 4),
        (true, &unselected, 4),
        (true, &object, 7),
        (false, &object, 11),
    ] {
        env.set_static_field(
            &bridge,
            (&bridge, "ACTIVE", "Z"),
            JValue::Bool(active.into()),
        )
        .unwrap();
        env.call_method(
            &pass,
            "batch",
            batch.descriptor,
            &[
                JValue::Object(state),
                JValue::Object(&unused),
                JValue::Object(&unused),
                JValue::Object(&unused),
                JValue::Object(&unused),
                JValue::Bool(1),
                JValue::Object(&unused),
            ],
        )
        .unwrap();
        env.call_method(
            &pass,
            "liquid",
            liquid.descriptor,
            &[
                JValue::Object(&unused),
                JValue::Object(&unused),
                JValue::Object(&unused),
                JValue::Object(state),
                JValue::Object(&unselected),
            ],
        )
        .unwrap();
        env.call_method(
            &pass,
            "blockEntity",
            block_entity.descriptor,
            &[
                JValue::Object(state),
                JValue::Float(0.5),
                JValue::Object(state),
                JValue::Object(state),
            ],
        )
        .unwrap();
        assert_eq!(
            env.get_field(&pass, "draws", "I").unwrap().i().unwrap(),
            expected
        );
    }
     
    env.call_method(&targets, "clear", "()V", &[]).unwrap();
    env.set_static_field(&bridge, (&bridge, "ACTIVE", "Z"), JValue::Bool(1))
        .unwrap();
    assert!(env
        .call_static_method(
            &bridge,
            "shouldHide",
            "(Ljava/lang/Object;)Z",
            &[JValue::Object(&object)]
        )
        .unwrap()
        .z()
        .unwrap());
    let actual = env
        .call_method(&object, "renderShape", shape.descriptor, &[])
        .unwrap()
        .l()
        .unwrap();
    assert!(env.is_same_object(&actual, &invisible).unwrap());
    env.set_static_field(&bridge, (&bridge, "ACTIVE", "Z"), JValue::Bool(0))
        .unwrap();
}

 
 
 
 
 
#[test]
fn crystal_optimizer_splice_hands_the_attacked_entity_to_the_bridge() {
    use crate::jvmti::{transform_class, PrependArg, PrependCall, TransformParams};
    use crate::mapping::MinecraftClassType;
    use jni::objects::JObject;

    let real = crate::jvmti::CRYSTAL_OPTIMIZER_TRANSFORM_PARAMS;
    assert_eq!(real.target_class, "net/minecraft/class_636");
    assert_eq!(real.method, "method_2918");
    assert_eq!(
        real.descriptor,
        "(Lnet/minecraft/class_1657;Lnet/minecraft/class_1297;)V"
    );
    assert_eq!(real.bridge_class, "InferedCrystalOptimizerBridge");
    assert_eq!(real.bridge_method, "beforeAttack");
    assert_eq!(real.bridge_descriptor, "(Ljava/lang/Object;)V");
    let real_locals = real
        .prepend_object_call
        .expect("the optimizer hook is a prologue splice")
        .arguments;
    assert_eq!(real_locals, &[PrependArg::Reference(2)]);

    let params = TransformParams {
        target_class: "CrystalOptimizerProbe",
        method: "attack",
        descriptor: "(Ljava/lang/Object;Ljava/lang/Object;)V",
        bridge_class: "CrystalOptimizerProbe",
        bridge_method: "noteAttack",
        bridge_descriptor: "(Ljava/lang/Object;)V",
        class_type: MinecraftClassType::MultiPlayerGameMode,
        log_label: "CrystalOptimizerProbe",
        replace_entire_body: false,
        replace_with_bridge_return: false,
        guard_flag: None,
         
        prepend_object_call: Some(PrependCall {
            arguments: &[PrependArg::Reference(2)],
            destination: None,
        }),
    };
    let source = std::fs::read(fixture_classes().join("CrystalOptimizerProbe.class")).unwrap();
    let bytes = transform_class(&source, &[params]).unwrap().unwrap();
    assert!(
        transform_class(&bytes, &[params]).unwrap().is_none(),
        "a hot reload of a class that already carries the splice must not add it twice"
    );

    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    let class = env
        .define_class("CrystalOptimizerProbe", &JObject::null(), &bytes)
        .expect("the spliced prologue must pass Java 21 -Xverify:all");
    let object = env.new_object(&class, "()V", &[]).unwrap();
    let player = env.new_string("player").unwrap();
    let target = env.new_string("crystal").unwrap();
    env.call_method(
        &object,
        "attack",
        params.descriptor,
        &[JValue::Object(&player), JValue::Object(&target)],
    )
    .unwrap();

    let seen = env
        .get_static_field(&class, "seen", "Ljava/lang/Object;")
        .unwrap()
        .l()
        .unwrap();
    assert!(
        env.is_same_object(&seen, &target).unwrap(),
        "the bridge must be handed the attacked entity, not the player"
    );
    let body = env
        .get_static_field(&class, "bodyTarget", "Ljava/lang/Object;")
        .unwrap()
        .l()
        .unwrap();
    assert!(
        env.is_same_object(&body, &target).unwrap(),
        "the attack itself must still run with its own arguments"
    );
    assert_eq!(
        env.get_static_field(&class, "bridged", "I")
            .unwrap()
            .i()
            .unwrap(),
        1,
        "the splice runs exactly once per attack"
    );
}

#[test]
fn smoke_render_tail_passes_hotspot_verification_and_preserves_argument_locals() {
    use jni::objects::JObject;
    let mut params = crate::jvmti::SMOKE_RENDER_TRANSFORM_PARAMS;
    params.target_class = "SmokeRenderProbe";
    params.method = "render";
    params.descriptor = "(Ljava/lang/Object;Ljava/lang/Object;ZLjava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)V";
    params.bridge_class = "SmokeRenderProbe";
    let source = std::fs::read(fixture_classes().join("SmokeRenderProbe.class")).unwrap();
    let bytes = crate::jvmti::transform_class(&source, &[params])
        .unwrap()
        .unwrap();
    assert!(crate::jvmti::transform_class(&bytes, &[params])
        .unwrap()
        .is_none());
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    let class = env
        .define_class("SmokeRenderProbe", &JObject::null(), &bytes)
        .expect("Smoke callback must pass Java 21 -Xverify:all");
    let object = env.new_object(&class, "()V", &[]).unwrap();
    let camera = env.new_string("camera").unwrap();
    let view = env.new_string("view").unwrap();
    let projection = env.new_string("projection").unwrap();
    for outline in [false, true] {
        env.call_method(
            &object,
            "render",
            params.descriptor,
            &[
                JValue::Object(&JObject::null()),
                JValue::Object(&JObject::null()),
                JValue::Bool(outline.into()),
                JValue::Object(&camera),
                JValue::Object(&JObject::null()),
                JValue::Object(&view),
                JValue::Object(&projection),
            ],
        )
        .unwrap();
        let captured = jni::objects::JObjectArray::from(
            env.get_static_field(&class, "captured", "[Ljava/lang/Object;")
                .unwrap()
                .l()
                .unwrap(),
        );
        for (i, expected) in [&object, camera.as_ref(), view.as_ref(), projection.as_ref()]
            .into_iter()
            .enumerate()
        {
            let actual = env.get_object_array_element(&captured, i as i32).unwrap();
            assert!(env.is_same_object(actual, expected).unwrap());
        }
    }
}

#[test]
fn item_scale_prologue_passes_the_hand_and_matrix_before_the_pass_body() {
    use crate::jvmti::{transform_class, PrependArg, TransformParams};
    use crate::mapping::MinecraftClassType;
    use jni::objects::JObject;
    let params = TransformParams {
        target_class: "ItemScaleProbe",
        method: "render",
        descriptor: "(Ljava/lang/Object;FFLjava/lang/Object;FLjava/lang/Object;FLjava/lang/Object;Ljava/lang/Object;I)V",
        bridge_class: "ItemScaleProbe",
        bridge_method: "apply",
        bridge_descriptor: "(Ljava/lang/Object;Ljava/lang/Object;)V",
        class_type: MinecraftClassType::ItemInHandRenderer,
        log_label: "ItemScaleProbe",
        replace_entire_body: false,
        replace_with_bridge_return: false,
        guard_flag: None,
         
         
        prepend_object_call: Some(crate::jvmti::PrependCall {
            arguments: &[PrependArg::Reference(4), PrependArg::Reference(8)],
            destination: None,
        }),
    };
    let source = std::fs::read(fixture_classes().join("ItemScaleProbe.class")).unwrap();
    let bytes = transform_class(&source, &[params]).unwrap().unwrap();
    assert!(
        transform_class(&bytes, &[params]).unwrap().is_none(),
        "idempotent hot-reload"
    );
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
     
     
     
    let loader_class = env.find_class("java/lang/ClassLoader").unwrap();
    let system_loader = env
        .call_static_method(
            &loader_class,
            "getSystemClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )
        .unwrap()
        .l()
        .unwrap();
    let class = env
        .define_class("ItemScaleProbe", &system_loader, &bytes)
        .expect("the spliced prologue must pass Java 21 -Xverify:all");
    let object = env.new_object(&class, "()V", &[]).unwrap();
    let hands = env.find_class("ItemScaleProbe$Hand").unwrap();
    let main_hand = env
        .get_static_field(&hands, "MAIN_HAND", "LItemScaleProbe$Hand;")
        .unwrap()
        .l()
        .unwrap();
    let off_hand = env
        .get_static_field(&hands, "OFF_HAND", "LItemScaleProbe$Hand;")
        .unwrap()
        .l()
        .unwrap();
    let matrix = env
        .get_static_field(&class, "MATRIX", "LItemScaleProbe$Matrix;")
        .unwrap()
        .l()
        .unwrap();
    let enabled = env.get_static_field_id(&class, "ENABLED", "Z").unwrap();
    let main_scale = env.get_static_field_id(&class, "MAIN_SCALE", "F").unwrap();
    let off_scale = env.get_static_field_id(&class, "OFF_SCALE", "F").unwrap();

    for (label, hand, hand_name, flag, main, off, expected) in [
         
         
         
         
         
        (
            "main",
            &main_hand,
            "MAIN_HAND",
            true,
            0.5_f32,
            0.25_f32,
            "translate(0.56,-0.52,-0.72)|scale(0.5,0.5,0.5)|translate(-0.56,0.52,0.72)|push|translate(1.5,2.0,7.0)|pop",
        ),
        (
            "off",
            &off_hand,
            "OFF_HAND",
            true,
            0.5_f32,
            0.25_f32,
            "translate(0.56,-0.52,-0.72)|scale(2.0,2.0,2.0)|translate(-0.56,0.52,0.72)|translate(-0.56,-0.52,-0.72)|scale(0.25,0.25,0.25)|translate(0.56,0.52,0.72)|push|translate(1.5,2.0,7.0)|pop",
        ),
         
        (
            "disabled",
            &main_hand,
            "MAIN_HAND",
            false,
            0.5_f32,
            0.25_f32,
            "translate(-0.56,-0.52,-0.72)|scale(4.0,4.0,4.0)|translate(0.56,0.52,0.72)|push|translate(1.5,2.0,7.0)|pop",
        ),
         
        (
            "vanilla scale",
            &main_hand,
            "MAIN_HAND",
            true,
            1.0_f32,
            0.25_f32,
            "push|translate(1.5,2.0,7.0)|pop",
        ),
    ] {
        env.call_static_method(&class, "reset", "()V", &[]).unwrap();
        env.set_static_field(&class, enabled, JValue::Bool(flag.into()))
            .unwrap();
        env.set_static_field(&class, main_scale, JValue::Float(main))
            .unwrap();
        env.set_static_field(&class, off_scale, JValue::Float(off))
            .unwrap();
        env.call_method(
            &object,
            "render",
            params.descriptor,
            &[
                JValue::Object(&JObject::null()),
                JValue::Float(0.5),
                JValue::Float(0.0),
                JValue::Object(hand),
                JValue::Float(2.0),
                JValue::Object(&JObject::null()),
                JValue::Float(1.5),
                JValue::Object(&matrix),
                JValue::Object(&JObject::null()),
                JValue::Int(7),
            ],
        )
        .unwrap();
        let trace = env
            .call_static_method(&class, "trace", "()Ljava/lang/String;", &[])
            .unwrap()
            .l()
            .unwrap();
        let trace = env
            .get_string(&jni::objects::JString::from(trace))
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(trace, expected, "matrix trace with the {label} hand");
        let body = env
            .get_static_field(&class, "body", "Ljava/lang/String;")
            .unwrap()
            .l()
            .unwrap();
        let body = env
            .get_string(&jni::objects::JString::from(body))
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            body,
            format!("render({hand_name},7)"),
            "the vanilla pass body must still run"
        );
    }
}

 
 
 
 
 
fn define_in_fixture_loader<'local>(
    env: &mut jni::JNIEnv<'local>,
    name: &str,
    bytes: &[u8],
) -> jni::objects::JClass<'local> {
    let path = env
        .new_string(fixture_classes().to_string_lossy().into_owned())
        .unwrap();
    let directory = env
        .new_object(
            "java/io/File",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&path)],
        )
        .unwrap();
    let uri = env
        .call_method(&directory, "toURI", "()Ljava/net/URI;", &[])
        .unwrap()
        .l()
        .unwrap();
    let url = env
        .call_method(&uri, "toURL", "()Ljava/net/URL;", &[])
        .unwrap()
        .l()
        .unwrap();
    let urls = env.new_object_array(1, "java/net/URL", &url).unwrap();
    let loader_class = env.find_class("java/lang/ClassLoader").unwrap();
    let parent = env
        .call_static_method(
            &loader_class,
            "getSystemClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )
        .unwrap()
        .l()
        .unwrap();
    let loader = env
        .new_object(
            "java/net/URLClassLoader",
            "([Ljava/net/URL;Ljava/lang/ClassLoader;)V",
            &[JValue::Object(&urls), JValue::Object(&parent)],
        )
        .unwrap();
    match env.define_class(name, &loader, bytes) {
        Ok(class) => class,
        Err(error) => {
            env.exception_describe().unwrap();
            panic!("{name} must verify with Java 21 -Xverify:all: {error}");
        }
    }
}

#[test]
fn invisible_shield_prologue_replaces_the_pass_stack_and_keeps_vanilla_untouched() {
    use crate::jvmti::{transform_class, PrependArg, PrependCall, TransformParams};
    use crate::mapping::MinecraftClassType;
    use jni::objects::JObject;
    let params = TransformParams {
        target_class: "ShieldHideProbe",
        method: "render",
        descriptor: "(Ljava/lang/Object;FFLjava/lang/Object;FLShieldHideProbe$Stack;FLjava/lang/Object;Ljava/lang/Object;I)V",
        bridge_class: "ShieldHideProbe",
        bridge_method: "hide",
        bridge_descriptor: "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        class_type: MinecraftClassType::ItemInHandRenderer,
        log_label: "ShieldHideProbe",
        replace_entire_body: false,
        replace_with_bridge_return: false,
        guard_flag: None,
         
         
        prepend_object_call: Some(PrependCall {
            arguments: &[PrependArg::Reference(4), PrependArg::Reference(6)],
            destination: Some(6),
        }),
    };
    let source = std::fs::read(fixture_classes().join("ShieldHideProbe.class")).unwrap();
    let bytes = transform_class(&source, &[params]).unwrap().unwrap();
    assert!(
        transform_class(&bytes, &[params]).unwrap().is_none(),
        "idempotent hot-reload"
    );
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    let class = define_in_fixture_loader(&mut env, "ShieldHideProbe", &bytes);
    let object = env.new_object(&class, "()V", &[]).unwrap();
    let hands = env.find_class("ShieldHideProbe$Hand").unwrap();
    let main_hand = env
        .get_static_field(&hands, "MAIN_HAND", "LShieldHideProbe$Hand;")
        .unwrap()
        .l()
        .unwrap();
    let off_hand = env
        .get_static_field(&hands, "OFF_HAND", "LShieldHideProbe$Hand;")
        .unwrap()
        .l()
        .unwrap();
     
     
    let mut stack = |name: &str| {
        env.get_static_field(&class, name, "LShieldHideProbe$Stack;")
            .unwrap()
            .l()
            .unwrap()
    };
    let shield = stack("SHIELD");
    let sword = stack("SWORD");
    let enabled = env.get_static_field_id(&class, "ENABLED", "Z").unwrap();
    let main_flag = env.get_static_field_id(&class, "MAIN_HAND", "Z").unwrap();
    let off_flag = env.get_static_field_id(&class, "OFF_HAND", "Z").unwrap();

    for (label, hand, stack, flag, main, off, expected) in [
         
         
        (
            "main shield",
            &main_hand,
            &shield,
            true,
            true,
            true,
            "EMPTY",
        ),
        ("off shield", &off_hand, &shield, true, true, true, "EMPTY"),
         
         
        (
            "main sword",
            &main_hand,
            &sword,
            true,
            true,
            true,
            "minecraft:sword",
        ),
        (
            "off hand hidden off",
            &off_hand,
            &shield,
            true,
            true,
            false,
            "minecraft:shield",
        ),
        (
            "main hand hidden off",
            &main_hand,
            &shield,
            true,
            false,
            true,
            "minecraft:shield",
        ),
        (
            "module off",
            &main_hand,
            &shield,
            false,
            true,
            true,
            "minecraft:shield",
        ),
    ] {
        env.call_static_method(&class, "reset", "()V", &[]).unwrap();
        env.set_static_field(&class, main_flag, JValue::Bool(main.into()))
            .unwrap();
        env.set_static_field(&class, off_flag, JValue::Bool(off.into()))
            .unwrap();
        env.set_static_field(&class, enabled, JValue::Bool(flag.into()))
            .unwrap();
        env.call_method(
            &object,
            "render",
            params.descriptor,
            &[
                JValue::Object(&JObject::null()),
                JValue::Float(0.5),
                JValue::Float(0.0),
                JValue::Object(hand),
                JValue::Float(2.0),
                JValue::Object(stack),
                JValue::Float(1.5),
                JValue::Object(&JObject::null()),
                JValue::Object(&JObject::null()),
                JValue::Int(7),
            ],
        )
        .unwrap();
        let drawn = env
            .call_static_method(&class, "drawn", "()Ljava/lang/String;", &[])
            .unwrap()
            .l()
            .unwrap();
        let drawn = env
            .get_string(&jni::objects::JString::from(drawn))
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(drawn, format!("{expected}|"), "with the {label}");
    }
}

#[test]
fn both_view_model_splices_compose_on_one_pass_method() {
    use crate::jvmti::{transform_class, PrependArg, PrependCall, TransformParams};
    use crate::mapping::MinecraftClassType;
    use jni::objects::JObject;
    let descriptor = "(Ljava/lang/Object;FFLjava/lang/Object;FLShieldHideProbe$Stack;FLjava/lang/Object;Ljava/lang/Object;I)V";
    let shield = TransformParams {
        target_class: "ShieldHideProbe",
        method: "render",
        descriptor,
        bridge_class: "ShieldHideProbe",
        bridge_method: "hide",
        bridge_descriptor: "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        class_type: MinecraftClassType::ItemInHandRenderer,
        log_label: "ShieldHideProbe",
        replace_entire_body: false,
        replace_with_bridge_return: false,
        guard_flag: None,
        prepend_object_call: Some(PrependCall {
            arguments: &[PrependArg::Reference(4), PrependArg::Reference(6)],
            destination: Some(6),
        }),
    };
    let scale = TransformParams {
        bridge_class: "ShieldHideProbe",
        bridge_method: "scale",
        bridge_descriptor: "(Ljava/lang/Object;Ljava/lang/Object;)V",
        log_label: "ShieldScaleProbe",
        prepend_object_call: Some(PrependCall {
            arguments: &[PrependArg::Reference(4), PrependArg::Reference(8)],
            destination: None,
        }),
        ..shield
    };
     
     
     
    let transforms = [shield, scale];
    let source = std::fs::read(fixture_classes().join("ShieldHideProbe.class")).unwrap();
    let bytes = transform_class(&source, &transforms).unwrap().unwrap();
    assert!(
        transform_class(&bytes, &transforms).unwrap().is_none(),
        "idempotent hot-reload"
    );
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    let class = define_in_fixture_loader(&mut env, "ShieldHideProbe", &bytes);
    let object = env.new_object(&class, "()V", &[]).unwrap();
    let hands = env.find_class("ShieldHideProbe$Hand").unwrap();
    let main_hand = env
        .get_static_field(&hands, "MAIN_HAND", "LShieldHideProbe$Hand;")
        .unwrap()
        .l()
        .unwrap();
    let shield_stack = env
        .get_static_field(&class, "SHIELD", "LShieldHideProbe$Stack;")
        .unwrap()
        .l()
        .unwrap();
    let matrix = env.new_object("java/lang/Object", "()V", &[]).unwrap();
    let enabled = env.get_static_field_id(&class, "ENABLED", "Z").unwrap();
    env.set_static_field(&class, enabled, JValue::Bool(1))
        .unwrap();
    env.call_method(
        &object,
        "render",
        descriptor,
        &[
            JValue::Object(&JObject::null()),
            JValue::Float(0.5),
            JValue::Float(0.0),
            JValue::Object(&main_hand),
            JValue::Float(2.0),
            JValue::Object(&shield_stack),
            JValue::Float(1.5),
            JValue::Object(&matrix),
            JValue::Object(&JObject::null()),
            JValue::Int(7),
        ],
    )
    .unwrap();
    let mut read = |method: &str| {
        let value = env
            .call_static_method(&class, method, "()Ljava/lang/String;", &[])
            .unwrap()
            .l()
            .unwrap();
        env.get_string(&jni::objects::JString::from(value))
            .unwrap()
            .to_string_lossy()
            .into_owned()
    };
    assert_eq!(read("drawn"), "EMPTY|", "the shield is still hidden");
    assert_eq!(
        read("scaled"),
        "MAIN_HAND/true;",
        "each prologue sees its own requested locals"
    );
}

#[test]
fn freelook_collision_guard_passes_hotspot_verification_and_skips_raycast_body() {
    use crate::jvmti::{transform_class, TransformParams};
    use crate::mapping::MinecraftClassType;
    use jni::objects::JObject;
    let params = |method| TransformParams {
        target_class: "CollisionProbe",
        method,
        descriptor: "(F)F",
        bridge_class: "CollisionProbe",
        bridge_method: "",
        bridge_descriptor: "",
        class_type: MinecraftClassType::Camera,
        log_label: "CollisionProbe",
        replace_entire_body: false,
        replace_with_bridge_return: false,
        guard_flag: Some("NO_CLIP"),
        prepend_object_call: None,
    };
    let source = std::fs::read(fixture_classes().join("CollisionProbe.class")).unwrap();
    let transforms = [params("clip"), params("simple")];
    let bytes = transform_class(&source, &transforms).unwrap().unwrap();
    assert!(
        transform_class(&bytes, &transforms).unwrap().is_none(),
        "idempotent hot-reload"
    );
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    let class = env
        .define_class("CollisionProbe", &JObject::null(), &bytes)
        .expect("float guard must pass -Xverify:all");
    let object = env.new_object(&class, "()V", &[]).unwrap();
    let field = env.get_static_field_id(&class, "NO_CLIP", "Z").unwrap();
    for guarded in [false, true, false] {
        env.set_static_field(&class, field, JValue::Bool(guarded.into()))
            .unwrap();
        for method in ["clip", "simple"] {
            let before = env
                .call_method(&object, "raycasts", "()I", &[])
                .unwrap()
                .i()
                .unwrap();
            let result = env
                .call_method(&object, method, "(F)F", &[JValue::Float(4.0)])
                .unwrap()
                .f()
                .unwrap();
            let after = env
                .call_method(&object, "raycasts", "()I", &[])
                .unwrap()
                .i()
                .unwrap();
            if guarded {
                assert_eq!(result, 4.0);
                assert_eq!(before, after, "no collision side effects");
            } else {
                assert!(result < 4.0);
                assert!(after > before, "vanilla collision restored");
            }
        }
    }
}

 
const SAMPLE: &str = "com/infered/fixture/Sample";

 
fn fixture_classes() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let sources_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/java");
         
         
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../target/jvm-fixtures")
            .join(std::process::id().to_string());
        std::fs::create_dir_all(&out).expect("create the fixture output directory");

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut sources = java_sources(&sources_root);
        sources.push(manifest.join("java/InferedXrayBridge.java"));
         
         
        sources.extend(java_sources(&manifest.join("java/stub")));
        for source in [
            "java/InferedCosmeticsBridge.java",
            "java/InferedScaffoldBridge.java",
            "java/InferedScaffoldLogic.java",
            "java/InferedChannelHandler.java",
            "tests/ScaffoldProbe.java",
            "java/InferedFastScaffoldBridge.java",
            "java/InferedFastScaffoldLogic.java",
            "tests/FastScaffoldProbe.java",
            "tests/FakeLagNettyProbe.java",
        ] {
            sources.push(manifest.join(source));
        }
        assert!(!sources.is_empty(), "no fixture .java sources found");

        let status = Command::new("javac")
            .arg("-g")
            .arg("-d")
            .arg(&out)
            .args(&sources)
            .status()
            .expect("`javac` (a JDK) must be available to run the JVM tests");
        assert!(status.success(), "fixture compilation failed");
        out
    })
}

 
fn java_sources(root: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "java") {
                sources.push(path);
            }
        }
    }
    sources
}

 
fn jvm() -> &'static JavaVM {
    static JVM: OnceLock<JavaVM> = OnceLock::new();
    JVM.get_or_init(|| {
        let classpath = format!("-Djava.class.path={}", fixture_classes().display());
        let args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option(&classpath)
             
             
             
            .option("-Xverify:all")
            .build()
            .expect("build the JVM init arguments");
        JavaVM::new(args).expect("create the in-process JVM")
    })
}

 
 
fn reflected_mapping() -> Mapping {
    jvm();  
    Mapping::new().expect("Mapping::new against the fixture JVM")
}

 
fn call_fixture_static(method: &str) {
    let mut env = jvm()
        .attach_current_thread_as_daemon()
        .expect("attach to the JVM");
    let class = env
        .find_class("net/minecraft/client/Minecraft")
        .expect("fixture Minecraft class");
    env.call_static_method(class, method, "()V", &[])
        .unwrap_or_else(|e| panic!("calling {method}: {e}"));
}

#[test]
fn chams_failure_retains_java_detail_and_leaves_the_callback_usable() {
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    env.throw_new("java/lang/IllegalStateException", "chams selection probe")
        .unwrap();
    assert!(env.exception_check().unwrap());
    let error = crate::graphic::chams::isolated_failure(
        &mut env,
        anyhow::anyhow!("player selection failed"),
    )
    .unwrap();
    assert!(format!("{error:#}").contains("chams selection probe"));
    assert!(!env.exception_check().unwrap());
    assert!(env.find_class("java/lang/Object").is_ok());
    let native_error = crate::graphic::chams::isolated_failure(
        &mut env,
        anyhow::anyhow!("vertex budget exceeded"),
    )
    .unwrap();
    assert_eq!(native_error.to_string(), "vertex budget exceeded");
}

#[test]
fn mapping_reflects_a_fixture_class() {
    let mapping = reflected_mapping();
    let class = mapping.get_class(SAMPLE).expect("Sample must reflect");
    let methods = class.method_names();
    for expected in ["value", "greet", "sum", "create"] {
        assert!(
            methods.iter().any(|m| m == expected),
            "reflected class is missing method `{expected}`",
        );
    }
}

#[test]
fn reflected_method_signatures_are_built_from_jni_types() {
    let mapping = reflected_mapping();
    let class = mapping.get_class(SAMPLE).unwrap();
    assert!(
        class
            .get_method_by_signature("greet", "(Ljava/lang/String;)Ljava/lang/String;")
            .is_ok(),
        "greet(String) signature should be reflected",
    );
    assert!(
        class.get_method_by_signature("sum", "(IJ)J").is_ok(),
        "sum(int, long) signature should be reflected",
    );
}

#[test]
fn an_overloaded_method_reflects_every_overload() {
    let mapping = reflected_mapping();
    let class = mapping.get_class(SAMPLE).unwrap();
    assert_eq!(
        class.get_methods("value").unwrap().len(),
        2,
        "value(int) and value(double) are distinct overloads",
    );
}

#[test]
fn overload_resolution_uses_reflected_signatures() {
    let mapping = reflected_mapping();
    let class = mapping.get_class(SAMPLE).unwrap();
    let chosen = class
        .get_method_by_args("value", &[JValue::Int(3)])
        .expect("an int argument must resolve an overload");
    assert_eq!(chosen.signature, "(I)I");
}

#[test]
fn get_class_caches_reflected_results() {
    let mapping = reflected_mapping();
    let first = mapping.get_class(SAMPLE).unwrap();
    let second = mapping.get_class(SAMPLE).unwrap();
    assert!(
        Arc::ptr_eq(&first, &second),
        "the second lookup must hit the cache",
    );
}

#[test]
fn resolving_a_missing_class_fails() {
    let mapping = reflected_mapping();
    let mut env = mapping.get_env().unwrap();
    assert!(
        mapping
            .resolve_class(&mut env, "totally/made/up/Class")
            .is_err(),
        "a class that does not exist must not resolve",
    );
}

 
 
 
fn shared_fixture_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
 
 
 
 
 
fn ensure_client_state() {
    static INIT: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = INIT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match crate::state::init() {
        Ok(()) | Err(crate::state::ClientError::AlreadyInitialized) => {}
        Err(error) => panic!("state::init failed against the fixture: {error}"),
    }
    for _ in 0..500 {
        if crate::state::try_client().is_some() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("client state never finished initializing");
}

 
 
#[test]
fn fishing_hook_wrapper_reads_the_bite_and_its_owner() {
    jvm();
    ensure_client_state();
    use crate::mapping::MappedObject;

    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    let hook_class = env
        .find_class("net/minecraft/world/entity/projectile/FishingHook")
        .expect("fixture FishingHook");
    let hook_object = env.new_object(hook_class, "()V", &[]).unwrap();
    let hook = crate::mapping::entity::FishingHook::new(env.new_global_ref(&hook_object).unwrap());

    assert!(!hook.is_biting().unwrap(), "a fresh bobber has no bite");
    env.set_field(&hook_object, "biting", "Z", JValue::Bool(1))
        .unwrap();
    assert!(hook.is_biting().unwrap(), "the bite flag must be readable");
    assert!(
        hook.player_owner().unwrap().is_none(),
        "a hook with no owner must report none"
    );

    let player_class = env
        .find_class("net/minecraft/world/entity/player/Player")
        .expect("fixture Player");
    let player_object = env.new_object(player_class, "()V", &[]).unwrap();
    env.call_method(
        &hook_object,
        "setPlayerOwner",
        "(Lnet/minecraft/world/entity/player/Player;)V",
        &[JValue::Object(&player_object)],
    )
    .unwrap();
    let owner =
        crate::mapping::entity::player::Player::new(env.new_global_ref(&player_object).unwrap());
    let found = hook
        .player_owner()
        .unwrap()
        .expect("the owner must be returned");
    assert!(
        found.is_same(&owner),
        "the owner must be the exact Player object"
    );
}

 
 
 
#[test]
fn weather_and_jump_delay_wrappers_drive_the_fixture() {
    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();
    call_fixture_static("enterWorld");
    let result = std::panic::catch_unwind(|| {
        use crate::mapping::MappedObject;
        use crate::module::Module;

        let world = crate::state::minecraft()
            .world()
            .unwrap()
            .expect("the fixture is in a world");
        world.set_rain_level(0.75).unwrap();
        world.set_thunder_level(0.25).unwrap();
        assert!((world.rain_level().unwrap() - 0.75).abs() < 1e-6);
        assert!((world.thunder_level().unwrap() - 0.25).abs() < 1e-6);

         
        let no_weather = crate::module::render::no_weather::NoWeatherModule::new();
        no_weather.on_start().unwrap();
        no_weather.on_tick().unwrap();
        assert_eq!(world.rain_level().unwrap(), 0.0);
        assert_eq!(world.thunder_level().unwrap(), 0.0);
        no_weather.on_stop().unwrap();
        assert!((world.rain_level().unwrap() - 0.75).abs() < 1e-6);
        assert!((world.thunder_level().unwrap() - 0.25).abs() < 1e-6);

         
         
        let no_jump_delay = crate::module::movement::no_jump_delay::NoJumpDelayModule::new();
        no_jump_delay.on_tick().unwrap();
        let player = crate::state::minecraft()
            .player()
            .unwrap()
            .expect("the fixture has a player");
        let mut env = jvm().attach_current_thread_as_daemon().unwrap();
        assert_eq!(
            env.get_field(player.jni_ref().as_obj(), "noJumpDelay", "I")
                .unwrap()
                .i()
                .unwrap(),
            0,
            "NoJumpDelay must clear the inherited cooldown field"
        );
    });
     
    call_fixture_static("leaveWorld");
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

#[test]
fn crosshair_reads_the_players_actual_attack_cooldown() {
    use crate::mapping::entity::PlayerRef;
    use crate::mapping::MappedObject;

    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();
    call_fixture_static("enterWorld");
    let result = std::panic::catch_unwind(|| {
        let player = crate::state::minecraft().player().unwrap().unwrap();
        let mut env = jvm().attach_current_thread_as_daemon().unwrap();
        let object = player.jni_ref().as_obj();
        env.set_field(object, "attackStrengthDelay", "F", JValue::Float(10.0))
            .unwrap();
        for (ticks, expected) in [(0, 0.0), (4, 0.4), (10, 1.0)] {
            env.set_field(object, "attackStrengthTicker", "I", JValue::Int(ticks))
                .unwrap();
            let actual = crate::graphic::crosshair::attack_cooldown(&player)
                .unwrap()
                .unwrap();
            assert!((actual - expected).abs() < 1e-6, "ticker {ticks}: {actual}");
        }
        env.set_field(object, "attackStrengthTicker", "I", JValue::Int(4))
            .unwrap();
        assert!(
            (player.attack_strength_scale().unwrap() - 0.45).abs() < 1e-6,
            "existing combat callers retain the half-tick offset"
        );
        env.set_field(object, "attackStrengthDelay", "F", JValue::Float(20.0))
            .unwrap();
        assert!(
            (crate::graphic::crosshair::attack_cooldown(&player)
                .unwrap()
                .unwrap()
                - 0.2)
                .abs()
                < 1e-6,
            "the displayed value follows the player's current attack speed"
        );
    });
    call_fixture_static("leaveWorld");
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

#[test]
fn performance_optimizers_follow_registry_and_slider_state() {
    use crate::module::performance::client_optimizations::{duration, modules, override_duration};
    use crate::module::{ModuleId, ModuleSetting};
    use std::time::Duration;

    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();
    let registry = &crate::state::client().modules;
    assert!(!registry.is_active());
    for module in modules() {
        registry.register(module);
    }
    let cases = [
        (
            ModuleId::EspSnapshotOptimizer,
            "Refresh ms",
            66.0,
            500.0,
            150.0,
        ),
        (ModuleId::BlockScanBudget, "Budget ms", 0.25, 2.0, 0.5),
        (
            ModuleId::NametagGearOptimizer,
            "Refresh ms",
            200.0,
            3000.0,
            1000.0,
        ),
        (
            ModuleId::RadarScanOptimizer,
            "Refresh ms",
            100.0,
            1000.0,
            250.0,
        ),
        (
            ModuleId::TrajectoryOptimizer,
            "Refresh ms",
            16.0,
            500.0,
            100.0,
        ),
    ];
    let fallback = Duration::from_millis(17);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for (id, name, min, max, _) in cases {
            assert_eq!(duration(id, name, fallback, min, max), fallback);
            let handle = registry.get(id).unwrap();
            handle
                .lock()
                .unwrap()
                .get_module_data_mut()
                .set_enabled(true);
        }
         
        assert_eq!(
            override_duration(cases[0].0, cases[0].1, cases[0].2, cases[0].3),
            None
        );
        registry.set_active(true);
        for (id, name, min, max, default) in cases {
            let expected = Duration::from_secs_f64(f64::from(default) / 1000.0);
            assert_eq!(override_duration(id, name, min, max), Some(expected));
            let handle = registry.get(id).unwrap();
            {
                let mut module = handle.lock().unwrap();
                let slider = module.get_module_data_mut().get_setting_mut(name).unwrap();
                assert!(matches!(slider, ModuleSetting::Slider { .. }));
                slider.set_slider_value(max * 2.0);
            }
            assert_eq!(
                override_duration(id, name, min, max),
                Some(Duration::from_secs_f64(f64::from(max) / 1000.0))
            );
            {
                let mut module = handle.lock().unwrap();
                module
                    .get_module_data_mut()
                    .get_setting_mut(name)
                    .unwrap()
                    .set_slider_value(f32::NAN);
            }
            assert_eq!(duration(id, name, fallback, min, max), fallback);
            handle
                .lock()
                .unwrap()
                .get_module_data_mut()
                .set_enabled(false);
            assert_eq!(override_duration(id, name, min, max), None);
        }
        registry.set_active(false);
    }));
    for (id, _, _, _, _) in cases {
        if let Some(handle) = registry.get(id) {
            handle
                .lock()
                .unwrap()
                .get_module_data_mut()
                .set_enabled(false);
        }
    }
    registry.set_active(false);
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

#[test]
fn practical_combat_modules_drive_fixture_actions() {
    use crate::mapping::{MappedObject, MinecraftClassType};
    use crate::module::combat::practical::modules;
    use crate::module::{Module, ModuleCategory, ModuleId};
    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();
    call_fixture_static("enterWorld");
    call_fixture_static("combatClearHotbar");
    let player = crate::state::minecraft().player().unwrap().unwrap();
    let mode = crate::state::minecraft().game_mode().unwrap().unwrap();
    let modules = modules();
    let ids = [
        ModuleId::AntiFireball,
        ModuleId::ShieldBreaker,
        ModuleId::AutoPotion,
        ModuleId::AutoCobweb,
        ModuleId::TotemRefill,
    ];
    for (module, id) in modules.iter().zip(ids) {
        assert_eq!(module.get_module_data().id, id);
        assert_eq!(module.get_module_data().category, ModuleCategory::Combat);
        assert!(!module.get_module_data().enabled);
    }
    let result = std::panic::catch_unwind(|| {
        let mut env = jvm().attach_current_thread_as_daemon().unwrap();
        let count = |env: &mut jni::JNIEnv, field: &str| {
            env.get_field(mode.jni_ref().as_obj(), field, "I")
                .unwrap()
                .i()
                .unwrap()
        };
        call_fixture_static("combatPickFireball");
        modules[0].on_tick().unwrap();
        modules[0].on_tick().unwrap();
        assert_eq!(
            count(&mut env, "attackCalls"),
            1,
            "cooldown prevents duplicate attacks"
        );
        call_fixture_static("combatPickFireball");
        call_fixture_static("combatUnpickableFireball");
        modules[0].anti_fireball(&player).unwrap();
        assert_eq!(count(&mut env, "attackCalls"), 1);
        call_fixture_static("combatPickShield");
        modules[0].anti_fireball(&player).unwrap();
        assert_eq!(count(&mut env, "attackCalls"), 1);
        call_fixture_static("combatAxe");
        call_fixture_static("combatShieldDown");
        modules[1].on_tick().unwrap();
        let inventory = player.get_inventory().unwrap();
        assert_eq!(inventory.get_selected().unwrap(), 0);
        call_fixture_static("combatPickShield");
        modules[1].on_tick().unwrap();
        assert_eq!(inventory.get_selected().unwrap(), 1);
        inventory.set_selected(5).unwrap();
        modules[1].on_tick().unwrap();
        modules[1].on_tick().unwrap();
        assert_eq!(
            inventory.get_selected().unwrap(),
            5,
            "manual weapon choice remains selected"
        );
        call_fixture_static("combatClearPick");
        modules[1].on_tick().unwrap();
        inventory.set_selected(0).unwrap();
        call_fixture_static("combatPickShield");
        modules[1].on_tick().unwrap();
        assert_eq!(inventory.get_selected().unwrap(), 1);
        call_fixture_static("combatClearPick");
        modules[1].on_tick().unwrap();
        assert_eq!(inventory.get_selected().unwrap(), 0);
        modules[1].shield_breaker(&player).unwrap();
        call_fixture_static("combatPotions");
        assert!(!inventory.get_item(3).unwrap().is_healing_potion().unwrap());
        assert!(inventory.get_item(4).unwrap().is_healing_potion().unwrap());
        call_fixture_static("combatHighHealth");
        modules[2].auto_potion(&player).unwrap();
        assert_eq!(count(&mut env, "useCalls"), 0);
        call_fixture_static("combatLowHealth");
        call_fixture_static("combatFlatLook");
        modules[2].auto_potion(&player).unwrap();
        assert_eq!(count(&mut env, "useCalls"), 0);
        call_fixture_static("combatDownLook");
        modules[2].on_tick().unwrap();
        modules[2].on_tick().unwrap();
        assert_eq!(count(&mut env, "useCalls"), 1);
        assert_eq!(inventory.get_selected().unwrap(), 4);
        assert_eq!(inventory.get_item(3).unwrap().count().unwrap(), 1);
        modules[2].on_stop().unwrap();
        assert_eq!(inventory.get_selected().unwrap(), 0);
        call_fixture_static("combatTotem");
        call_fixture_static("combatSprintOn");
        modules[4].totem_refill(&player).unwrap();
        assert_eq!(count(&mut env, "swapCalls"), 0);
        call_fixture_static("combatSprintOff");
        modules[4].on_tick().unwrap();
        assert_eq!(count(&mut env, "swapCalls"), 1);
        assert_eq!(
            inventory
                .get_item(8)
                .unwrap()
                .get_item()
                .unwrap()
                .registry_name()
                .unwrap(),
            "minecraft:totem_of_undying"
        );
        modules[4].totem_refill(&player).unwrap();
        assert_eq!(count(&mut env, "swapCalls"), 1);
        call_fixture_static("combatNearTarget");
        call_fixture_static("combatPickBlock");
        call_fixture_static("combatWeb");
        call_fixture_static("combatSolidOn");
        modules[3].auto_cobweb(&player).unwrap();
        assert_eq!(count(&mut env, "placeCalls"), 0);
        call_fixture_static("combatSolidOff");
        modules[3].on_tick().unwrap();
        assert_eq!(count(&mut env, "placeCalls"), 1);
        assert_eq!(inventory.get_selected().unwrap(), 2);
        inventory.set_selected(1).unwrap();
        modules[3].on_stop().unwrap();
        assert_eq!(
            inventory.get_selected().unwrap(),
            1,
            "manual slot change wins"
        );
        assert!(crate::state::mapping()
            .is_instance_of(MinecraftClassType::Player, player.jni_ref().as_obj())
            .unwrap());
    });
    for module in &modules {
        let _ = module.on_stop();
    }
    call_fixture_static("combatClearPick");
    call_fixture_static("leaveWorld");
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

#[test]
fn misc_option_utilities_restore_minecraft_values() {
    use crate::module::misc::option_utilities::modules;
    use crate::module::{Module, ModuleCategory, ModuleId};

    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();
    call_fixture_static("leaveWorld");
    let modules = modules();
    let options = crate::state::minecraft().options().unwrap();
    let fields = [
        "darknessEffectScale",
        "screenEffectScale",
        "glintStrength",
        "toggleCrouch",
        "toggleSprint",
    ];
    let doubles = [0.8, 0.6, 0.4];
    for (field, value) in fields[..3].iter().zip(doubles) {
        options
            .misc_option(field)
            .unwrap()
            .force_double(value)
            .unwrap();
    }
    for field in &fields[3..] {
        options
            .misc_option(field)
            .unwrap()
            .force_bool(false)
            .unwrap();
    }

    let result = std::panic::catch_unwind(|| {
        for module in &modules {
            assert_eq!(module.get_module_data().category, ModuleCategory::Misc);
            module.on_start().unwrap();
        }
        for (field, value) in fields[..3].iter().zip(doubles) {
            assert_eq!(
                options.misc_option(field).unwrap().get_double().unwrap(),
                value
            );
        }
        call_fixture_static("enterWorld");
        for module in &modules {
            module.on_start().unwrap();
            module.on_tick().unwrap();
        }
        for field in &fields[..3] {
            assert_eq!(
                options.misc_option(field).unwrap().get_double().unwrap(),
                0.0
            );
        }
        for field in &fields[3..] {
            assert!(options.misc_option(field).unwrap().get_bool().unwrap());
        }
         
        options
            .misc_option(fields[0])
            .unwrap()
            .force_double(0.3)
            .unwrap();
        modules[0].on_tick().unwrap();
        assert_eq!(
            options
                .misc_option(fields[0])
                .unwrap()
                .get_double()
                .unwrap(),
            0.0
        );
         
        options
            .misc_option(fields[1])
            .unwrap()
            .force_double(0.2)
            .unwrap();
        modules[1].on_stop().unwrap();
        assert_eq!(
            options
                .misc_option(fields[1])
                .unwrap()
                .get_double()
                .unwrap(),
            0.2
        );
        for module in [&modules[0], &modules[2], &modules[3]] {
            module.on_stop().unwrap();
        }
        assert_eq!(
            options
                .misc_option(fields[0])
                .unwrap()
                .get_double()
                .unwrap(),
            0.8
        );
        assert_eq!(
            options
                .misc_option(fields[2])
                .unwrap()
                .get_double()
                .unwrap(),
            0.4
        );
        assert!(!options.misc_option(fields[3]).unwrap().get_bool().unwrap());
        call_fixture_static("leaveWorld");
        modules[4].on_world_suspend().unwrap();
        assert!(!options.misc_option(fields[4]).unwrap().get_bool().unwrap());

         
        options
            .misc_option(fields[4])
            .unwrap()
            .force_bool(true)
            .unwrap();
        call_fixture_static("enterWorld");
        modules[4].on_start().unwrap();
        modules[4].on_stop().unwrap();
        assert!(options.misc_option(fields[4]).unwrap().get_bool().unwrap());
        assert_eq!(modules[4].get_module_data().id, ModuleId::ToggleSprint);

         
        options
            .misc_option(fields[4])
            .unwrap()
            .force_bool(false)
            .unwrap();
        let registry = crate::module::registry::ModuleRegistry::new();
        for module in crate::module::misc::option_utilities::modules() {
            registry.register(module);
        }
        for id in [
            ModuleId::NoDarknessEffect,
            ModuleId::NoScreenEffects,
            ModuleId::NoEnchantmentGlint,
            ModuleId::ToggleCrouch,
            ModuleId::ToggleSprint,
        ] {
            let handle = registry.get(id).expect("module registered");
            handle
                .lock()
                .unwrap()
                .get_module_data_mut()
                .set_enabled(true);
        }
        registry.set_active(true);
        for field in &fields[..3] {
            assert_eq!(
                options.misc_option(field).unwrap().get_double().unwrap(),
                0.0
            );
        }
        for field in &fields[3..] {
            assert!(options.misc_option(field).unwrap().get_bool().unwrap());
        }
        call_fixture_static("leaveWorld");
        registry.set_active(false);
        for (field, value) in fields[..3].iter().zip([0.8, 0.2, 0.4]) {
            assert_eq!(
                options.misc_option(field).unwrap().get_double().unwrap(),
                value
            );
        }
        for field in &fields[3..] {
            assert!(!options.misc_option(field).unwrap().get_bool().unwrap());
        }
    });
    for module in &modules {
        let _ = module.on_stop();
    }
    for (field, value) in fields[..3].iter().zip(doubles) {
        options
            .misc_option(field)
            .unwrap()
            .force_double(value)
            .unwrap();
    }
    for field in &fields[3..] {
        options
            .misc_option(field)
            .unwrap()
            .force_bool(false)
            .unwrap();
    }
    call_fixture_static("leaveWorld");
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

 
 
#[test]
fn silent_aura_blocks_camera_picked_mining_without_changing_bindings() {
    use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
    use crate::module::combat::rotation::{self, Rotation};
    use crate::module::combat::silent_aura::SilentAuraModule;
    use crate::module::Module;

    let _shared = shared_fixture_lock();
    let _rotation = rotation::test_guard();
    jvm();
    ensure_client_state();
    call_fixture_static("enterWorld");
    call_fixture_static("combatPickBlock");
    let was_active = crate::state::client().modules.is_active();
    crate::state::client().modules.set_active(false);
    let module = SilentAuraModule::new();
    let result = std::panic::catch_unwind(|| {
        let game = crate::state::minecraft();
        let mut env = crate::state::mapping().get_env().unwrap();
        let options = game.options().unwrap();
        let key = options
            .get_field(
                "keyAttack",
                FieldType::Object(MinecraftClassType::KeyMapping),
            )
            .unwrap()
            .l()
            .unwrap();
        let mode = game.game_mode().unwrap().unwrap();
        let counter = |name: &str| mode.get_field(name, FieldType::Int).unwrap().i().unwrap();
        let input = || {
            game.call_method("handleKeybinds", &[]).unwrap();
        };
        env.set_field(&key, "isDown", "Z", JValue::Bool(1)).unwrap();
        env.set_field(&key, "clickCount", "I", JValue::Int(2))
            .unwrap();
        options.key_up().unwrap().set_down(true).unwrap();
        game.set_field("missTime", FieldType::Int, JValue::Int(0))
            .unwrap();

         
        rotation::aim_silent(Rotation::new(0.0, 0.0), 7.0);
        module.on_input_phase().unwrap();
        input();
        assert_eq!(counter("startDigCalls"), 2);
        assert_eq!(counter("continueDigCalls"), 1);

         
         
        module.on_tick().unwrap();
        env.set_field(&key, "clickCount", "I", JValue::Int(3))
            .unwrap();
        module.on_input_phase().unwrap();
        assert_eq!(counter("abortDigCalls"), 1);
        assert_eq!(
            game.get_field("missTime", FieldType::Int)
                .unwrap()
                .i()
                .unwrap(),
            1
        );
         
        let player = game.player().unwrap().unwrap();
        let target = crate::mapping::entity::Entity::new(
            crate::state::mapping()
                .new_global_ref(
                    env.new_object("net/minecraft/world/entity/player/Player", "()V", &[])
                        .unwrap(),
                )
                .unwrap(),
        );
        mode.attack(&player, &target).unwrap();
        assert_eq!(counter("attackCalls"), 1);
        input();
        assert_eq!(counter("startDigCalls"), 2);
        assert_eq!(counter("continueDigCalls"), 1);
        assert_eq!(
            env.get_field(&key, "clickCount", "I").unwrap().i().unwrap(),
            0
        );
        assert!(env.get_field(&key, "isDown", "Z").unwrap().z().unwrap());
        assert!(options.key_up().unwrap().is_down().unwrap());

         
        rotation::clear_silent();
        module.on_input_phase().unwrap();
        input();
        assert_eq!(counter("continueDigCalls"), 2);
        assert_eq!(
            game.get_field("missTime", FieldType::Int)
                .unwrap()
                .i()
                .unwrap(),
            0
        );

         
        call_fixture_static("pickEntity");
        module.on_tick().unwrap();
        env.set_field(&key, "clickCount", "I", JValue::Int(1))
            .unwrap();
        module.on_input_phase().unwrap();
        assert_eq!(
            game.get_field("missTime", FieldType::Int)
                .unwrap()
                .i()
                .unwrap(),
            0
        );
        let before = game
            .get_field("attackCalls", FieldType::Int)
            .unwrap()
            .i()
            .unwrap();
        game.call_method("startAttack", &[]).unwrap();
        assert_eq!(
            game.get_field("attackCalls", FieldType::Int)
                .unwrap()
                .i()
                .unwrap(),
            before + 1
        );
    });
    module.on_stop().unwrap();
    if let Ok(options) = crate::state::minecraft().options() {
        options.key_up().unwrap().set_down(false).unwrap();
        let mut env = crate::state::mapping().get_env().unwrap();
        let key = options
            .get_field(
                "keyAttack",
                FieldType::Object(MinecraftClassType::KeyMapping),
            )
            .unwrap()
            .l()
            .unwrap();
        env.set_field(&key, "isDown", "Z", JValue::Bool(0)).unwrap();
        env.set_field(&key, "clickCount", "I", JValue::Int(0))
            .unwrap();
    }
    crate::state::minecraft()
        .set_field("missTime", FieldType::Int, JValue::Int(0))
        .unwrap();
    rotation::clear();
    call_fixture_static("leaveWorld");
    crate::state::client().modules.set_active(was_active);
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

#[test]
fn silent_aura_hands_back_rebased_yaw_and_resets_on_server_correction() {
    use crate::mapping::entity::EntityRef;
    use crate::mapping::MappedObject;
    use crate::module::combat::silent_aura::SilentAuraModule;
    use crate::module::Module;
    use crate::net::packet::captured::CapturedPacket;
    use crate::net::packet::serverbound_move_player::ServerboundMovePlayerPacket;
    use crate::net::packet::Packet;

    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();
    call_fixture_static("enterWorld");
    let mut module = SilentAuraModule::new();
    if let Some(crate::module::ModuleSetting::Toggle { value, .. }) =
        module.get_module_data_mut().get_setting_mut("Silent Aim")
    {
        *value = false;
    }
    let player = crate::state::minecraft().player().unwrap().unwrap();
    let movement = |yaw| {
        Packet::ServerboundMovePlayer(ServerboundMovePlayerPacket {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            y_rot: yaw,
            x_rot: 10.0,
            has_position: true,
            has_rotation: true,
            on_ground: true,
            horizontal_collision: false,
        })
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        player.set_rotation(179.0, 10.0).unwrap();
        module.on_start().unwrap();
        let mut packet = movement(-179.0);
        module.handle_packet(&mut packet);
        let Packet::ServerboundMovePlayer(snapshot) = packet else {
            unreachable!()
        };
        assert_eq!(snapshot.y_rot, 181.0);
        player.set_rotation(-179.0, 10.0).unwrap();
        module.on_stop().unwrap();
        assert_eq!(player.get_yaw().unwrap(), 181.0);
        assert_eq!(player.get_pitch().unwrap(), 10.0);
        assert_eq!(
            player.call_method("getYRot", &[]).unwrap().f().unwrap(),
            181.0
        );

        for rotation_only in [false, true] {
            module.on_start().unwrap();
            let captured = CapturedPacket {
                jni_ref: player.jni_ref().clone(),
            };
            let mut correction = if rotation_only {
                Packet::ClientboundPlayerRotation(captured)
            } else {
                Packet::ClientboundPlayerPosition(captured)
            };
            let original = correction.clone();
            module.handle_packet(&mut correction);
            assert_eq!(correction, original);
            let mut packet = movement(-100.0);
            module.handle_packet(&mut packet);
            let Packet::ServerboundMovePlayer(snapshot) = packet else {
                unreachable!()
            };
            assert_eq!(snapshot.y_rot, -100.0, "correction clears the old branch");
        }
    }));
    module.on_stop().unwrap();
    call_fixture_static("leaveWorld");
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

 
 
 
 
 
 
 
 
#[test]
fn a_silent_hit_tick_turns_the_body_and_restores_the_view() {
    use crate::graphic::input::FREE_KEY_W;
    use crate::mapping::entity::EntityRef;
    use crate::module::combat::rotation::{
        aim_body, aim_body_for, restore_body, restore_body_for, BodyAimOwner, Rotation,
    };

    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();
    call_fixture_static("enterWorld");
    let result = std::panic::catch_unwind(|| {
        let minecraft = crate::state::minecraft();
        let player = minecraft
            .player()
            .unwrap()
            .expect("the fixture has a player");
        let options = minecraft.options().unwrap();
        let forward = options.key_up().unwrap();

         
        let view = Rotation::new(45.0, 10.0);
        player.set_rotation(view.yaw, view.pitch).unwrap();
        forward.set_down(true).unwrap();
        crate::graphic::input::KEYS_DOWN
            .lock()
            .unwrap()
            .push(FREE_KEY_W);
        assert_eq!(restore_body(), None, "nothing has taken the body");

         
         
         
         
        let aim = Rotation::new(135.0, -30.0);
        aim_body(aim).unwrap();
        assert!((player.get_yaw().unwrap() - aim.yaw).abs() < 1e-6);
        assert!((player.get_pitch().unwrap() - aim.pitch).abs() < 1e-6);
        assert!(
            !forward.is_down().unwrap(),
            "the turn holds the player's forward key"
        );
        assert!(
            options.key_left().unwrap().is_down().unwrap(),
            "and presses the input that keeps the step on the player's heading"
        );

         
         
         
         
         
        let other = Rotation::new(-45.0, 5.0);
        aim_body(other).unwrap();
        assert!((player.get_yaw().unwrap() - other.yaw).abs() < 1e-6);
        assert!(
            options.key_right().unwrap().is_down().unwrap(),
            "the new aim needs the input on the other side"
        );
        assert!(
            !options.key_left().unwrap().is_down().unwrap(),
            "the first turn's key is handed back before the second writes"
        );
        assert!(
            !forward.is_down().unwrap(),
            "the frame's view is still the player's own, not the first aim"
        );

         
        let restored = restore_body().expect("the tick left a view to restore");
        assert!((restored.yaw - view.yaw).abs() < 1e-6);
        assert!((restored.pitch - view.pitch).abs() < 1e-6);
        assert!((player.get_yaw().unwrap() - view.yaw).abs() < 1e-6);
        assert!((player.get_pitch().unwrap() - view.pitch).abs() < 1e-6);
        assert!(forward.is_down().unwrap(), "the player's own W comes back");
        for key in [options.key_left(), options.key_right()]
            .into_iter()
            .flatten()
        {
            assert!(
                !key.is_down().unwrap(),
                "and the keys the turns pressed go away"
            );
        }
        assert_eq!(restore_body(), None, "a view is restored exactly once");

         
         
        aim_body(aim).unwrap();
        player.set_rotation(aim.yaw + 7.0, aim.pitch + 3.0).unwrap();
        {
            let mut keys = crate::graphic::input::KEYS_DOWN.lock().unwrap();
            keys.clear();
            keys.push(crate::graphic::input::FREE_KEY_S);
        }
        aim_body(other).unwrap();
        assert!(
            options.key_left().unwrap().is_down().unwrap()
                && !options.key_right().unwrap().is_down().unwrap(),
            "the second turn uses the newly held back key, not stale forward"
        );
        let moved_view = restore_body().expect("catch-up turn restores the view");
        assert!((moved_view.yaw - (view.yaw + 7.0)).abs() < 1e-6);
        assert!((moved_view.pitch - (view.pitch + 3.0)).abs() < 1e-6);
        assert!(options.key_back().unwrap().is_down().unwrap());
        assert!(!forward.is_down().unwrap());
        crate::graphic::input::KEYS_DOWN.lock().unwrap().clear();
        crate::graphic::input::KEYS_DOWN
            .lock()
            .unwrap()
            .push(FREE_KEY_W);
        options.key_back().unwrap().set_down(false).unwrap();
        forward.set_down(true).unwrap();
        player.set_rotation(view.yaw, view.pitch).unwrap();

         
         
         
        aim_body_for(BodyAimOwner::Scaffold, other).unwrap();
        assert!((player.get_yaw().unwrap() - other.yaw).abs() < 1e-6);
        assert_eq!(
            restore_body_for(BodyAimOwner::SilentAura),
            None,
            "the aura does not own Scaffold's turn"
        );
        assert!(
            (player.get_yaw().unwrap() - other.yaw).abs() < 1e-6,
            "a non-owner's restore leaves the turn and the body alone"
        );
        assert_eq!(
            restore_body_for(BodyAimOwner::FastScaffold),
            None,
            "Fast Scaffold cannot restore the original's pending turn"
        );
        let restored =
            restore_body_for(BodyAimOwner::Scaffold).expect("Scaffold restores the turn it made");
        assert!((restored.yaw - view.yaw).abs() < 1e-6);
        assert!((player.get_yaw().unwrap() - view.yaw).abs() < 1e-6);
        assert!(
            forward.is_down().unwrap(),
            "and the player's own W comes back"
        );
        aim_body_for(BodyAimOwner::FastScaffold, other).unwrap();
        assert_eq!(restore_body_for(BodyAimOwner::Scaffold), None);
        assert_eq!(restore_body_for(BodyAimOwner::SilentAura), None);
        assert!((player.get_yaw().unwrap() - other.yaw).abs() < 1e-6);
        let restored =
            restore_body_for(BodyAimOwner::FastScaffold).expect("copy restores only its own turn");
        assert!((restored.yaw - view.yaw).abs() < 1e-6);
        assert!((player.get_yaw().unwrap() - view.yaw).abs() < 1e-6);
        assert!(forward.is_down().unwrap());
    });
     
     
    let _ = restore_body();
    if let Ok(options) = crate::state::minecraft().options() {
        for key in [
            options.key_up(),
            options.key_back(),
            options.key_left(),
            options.key_right(),
        ]
        .into_iter()
        .flatten()
        {
            let _ = key.set_down(false);
        }
    }
    crate::graphic::input::KEYS_DOWN.lock().unwrap().clear();
    call_fixture_static("leaveWorld");
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

 
 
 
 
 
#[test]
fn visual_silent_draws_the_aim_on_the_model_and_hands_it_back() {
    use crate::graphic::visual_silent;
    use crate::mapping::entity::EntityRef;
    use crate::mapping::{FieldType, MappedObject};
    use crate::module::combat::rotation::{aim_silent, clear_silent, silent_rotation, Rotation};

    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();
    call_fixture_static("enterWorld");
    call_fixture_static("firstPerson");
    let result = std::panic::catch_unwind(|| {
        let minecraft = crate::state::minecraft();
        let player = minecraft
            .player()
            .unwrap()
            .expect("the fixture has a player");
        let field = |name: &str| {
            player
                .get_field(name, FieldType::Float)
                .unwrap()
                .f()
                .unwrap()
        };

         
         
        let view = Rotation::new(45.0, 10.0);
        player.set_rotation(view.yaw, view.pitch).unwrap();
        for name in ["yBodyRot", "yBodyRotO", "yHeadRot", "yHeadRotO"] {
            player
                .set_field(name, FieldType::Float, jni::objects::JValue::Float(30.0))
                .unwrap();
        }
        visual_silent::set_owner(visual_silent::SILENT_AURA, true);
         
         
        aim_silent(Rotation::new(135.0, -20.0), 100_000.0);
        let aim = silent_rotation().expect("a silent aim is on the wire");

         
         
        visual_silent::apply();
        assert!((player.get_yaw().unwrap() - view.yaw).abs() < 1e-6);
        assert!((player.get_pitch().unwrap() - view.pitch).abs() < 1e-6);

         
        call_fixture_static("thirdPerson");
        visual_silent::apply();
        assert!((player.get_yaw().unwrap() - aim.yaw).abs() < 1e-4);
        assert!((player.get_pitch().unwrap() - aim.pitch).abs() < 1e-4);
        for name in ["yBodyRot", "yBodyRotO", "yHeadRot", "yHeadRotO"] {
            assert!(
                (field(name) - aim.yaw).abs() < 1e-4,
                "{name} follows the aim: {}",
                field(name)
            );
        }
        assert!(
            (player.get_yaw().unwrap() - 45.0).abs() > 1.0,
            "the drawn pose is not the player's own view"
        );
        assert_eq!(
            silent_rotation(),
            Some(aim),
            "the projection never changes the aim that is on the wire"
        );

         
         
        visual_silent::restore();
        assert!((player.get_yaw().unwrap() - view.yaw).abs() < 1e-6);
        assert!((player.get_pitch().unwrap() - view.pitch).abs() < 1e-6);
        assert!((field("yRotO") - view.yaw).abs() < 1e-6);
        assert!((field("xRotO") - view.pitch).abs() < 1e-6);
        for name in ["yBodyRot", "yBodyRotO", "yHeadRot", "yHeadRotO"] {
            assert!((field(name) - 30.0).abs() < 1e-6, "{name} is handed back");
        }
         
        visual_silent::restore();
        assert!((player.get_yaw().unwrap() - view.yaw).abs() < 1e-6);

         
        visual_silent::set_owner(visual_silent::SILENT_AURA, false);
        visual_silent::apply();
        assert!((player.get_yaw().unwrap() - view.yaw).abs() < 1e-6);
        assert!((field("yBodyRot") - 30.0).abs() < 1e-6);
    });
     
     
    visual_silent::restore();
    visual_silent::set_owner(visual_silent::SILENT_AURA, false);
    clear_silent();
    call_fixture_static("firstPerson");
    call_fixture_static("leaveWorld");
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

 
 
 
#[test]
fn title_screen_splash_is_readable_and_replaceable() {
    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();
    call_fixture_static("openTitleScreen");
    let result = std::panic::catch_unwind(|| {
        let minecraft = crate::state::minecraft();
        assert!(minecraft.title_screen_open());
        assert_eq!(
            minecraft.title_splash().as_deref(),
            Some("Also try Terraria!")
        );

         
        minecraft.hide_title_splash().unwrap();
        assert_eq!(minecraft.title_splash(), None);
         
        minecraft.hide_title_splash().unwrap();

         
        minecraft.restore_title_splash().unwrap();
        assert_eq!(
            minecraft.title_splash().as_deref(),
            Some("Also try Terraria!")
        );
         
        minecraft.restore_title_splash().unwrap();
    });
     
    call_fixture_static("closeScreen");
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }

     
    let minecraft = crate::state::minecraft();
    assert!(!minecraft.title_screen_open());
    assert_eq!(minecraft.title_splash(), None);
    minecraft.hide_title_splash().unwrap();
    minecraft.restore_title_splash().unwrap();
}

#[test]
fn client_initializes_and_tracks_the_menu_and_world_states() {
    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();
    let minecraft = crate::state::minecraft();

     
    assert!(
        minecraft.player().unwrap().is_none(),
        "no player in the menu"
    );
    assert!(minecraft.world().unwrap().is_none(), "no world in the menu");
    assert!(minecraft.game_mode().unwrap().is_none());
    assert!(!minecraft.in_world());

     
    call_fixture_static("enterWorld");
    assert!(
        minecraft.player().unwrap().is_some(),
        "the player must be present once in a world",
    );
    assert!(minecraft.world().unwrap().is_some());
    assert!(minecraft.in_world());
    use crate::mapping::MappedObject;
    use crate::module::Module;
    let player = minecraft.player().unwrap().unwrap();
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();

     
     
    let camera_class = env.find_class("net/minecraft/client/Camera").unwrap();
    let object = env.new_object(camera_class, "()V", &[]).unwrap();
    let camera = crate::mapping::client::camera::Camera::new(env.new_global_ref(&object).unwrap());
    camera.setup_independent(35.0, -20.0, false, false).unwrap();
    assert_eq!(camera.yaw().unwrap(), 35.0);
    assert_eq!(camera.pitch().unwrap(), -20.0);
    let pivot = camera.position().unwrap();
    assert!(
        (pivot.x() - 2.0).abs() < 1e-6
            && (pivot.y() - 65.5).abs() < 1e-6
            && (pivot.z() - 3.0).abs() < 1e-6
    );
    camera.setup_independent(35.0, -20.0, true, false).unwrap();
    let orbit = camera.position().unwrap();
    let distance = ((orbit.x() - pivot.x()).powi(2)
        + (orbit.y() - pivot.y()).powi(2)
        + (orbit.z() - pivot.z()).powi(2))
    .sqrt();
    assert!(
        (distance - 1.5).abs() < 1e-6,
        "vanilla collision-limited orbit"
    );
    assert_eq!(
        env.get_field(&object, "requestedZoom", "F")
            .unwrap()
            .f()
            .unwrap(),
        2.0,
        "vanilla entity-scale orbit radius"
    );
    camera.setup_independent(35.0, -20.0, true, true).unwrap();
    assert_eq!(camera.yaw().unwrap(), 215.0);
    assert_eq!(camera.pitch().unwrap(), 20.0);
    let focused = env
        .get_field(
            &object,
            "entity",
            "Lnet/minecraft/world/entity/LivingEntity;",
        )
        .unwrap()
        .l()
        .unwrap();
    assert_eq!(
        env.call_method(&focused, "getYRot", "()F", &[])
            .unwrap()
            .f()
            .unwrap(),
        0.0,
        "camera does not rotate player"
    );

    use crate::mapping::entity::LivingEntityRef;
    assert_eq!(
        player.get_active_effects().unwrap(),
        vec![("night_vision".into(), 0, -1), ("speed".into(), 1, 1200)]
    );
     
    for _ in 0..50 {
        assert_eq!(player.get_active_effects().unwrap().len(), 2);
    }

    crate::state::client()
        .modules
        .register(crate::module::render::legit::radar());
    use crate::graphic::hud::RadarKind;
    assert_eq!(
        crate::graphic::hud::radar_contacts_for_test(),
        vec![
            ([5.0, 0.0, 0.0], RadarKind::Player),
            ([0.0, 0.0, 5.0], RadarKind::Mob),
            ([-5.0, 0.0, 0.0], RadarKind::Other)
        ]
    );
     
     
    let ctx = egui::Context::default();
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("radar_test"),
        ));
        crate::graphic::hud::draw_radar_at(ctx, &painter, egui::pos2(10.0, 10.0));
    });
    let self_index = output
        .shapes
        .iter()
        .position(|shape| matches!(shape.shape, egui::Shape::Path(_)))
        .unwrap();
    for color in [
        egui::Color32::from_rgb(255, 70, 80),
        egui::Color32::from_rgb(255, 210, 60),
        egui::Color32::from_rgb(70, 205, 255),
    ] {
        let (index, circle) = output
            .shapes
            .iter()
            .enumerate()
            .find_map(|(index, shape)| {
                if let egui::Shape::Circle(circle) = &shape.shape {
                    if circle.fill == color {
                        return Some((index, circle));
                    }
                }
                None
            })
            .expect("each entity kind must have a colored radar marker");
        assert!(
            index > self_index,
            "self marker must not cover nearby entities"
        );
        assert_eq!(circle.radius, 2.8);
        assert!((circle.center - egui::pos2(70.0, 70.0)).length() < 4.0);
        assert!(output.shapes.iter().skip(index + 1).any(|shape| matches!(&shape.shape,
            egui::Shape::Circle(outline) if outline.center == circle.center && outline.stroke.color == egui::Color32::WHITE)));
    }
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if text.galley.text() == "3 entities")));
    let radar_handle = crate::state::client()
        .modules
        .get(crate::module::ModuleId::Radar)
        .unwrap();
    let set_filter = |name: &str, enabled: bool| {
        let mut radar = radar_handle.lock().unwrap();
        let setting = radar
            .get_module_data_mut()
            .settings
            .iter_mut()
            .find(|s| s.name() == name)
            .unwrap();
        match setting {
            crate::module::ModuleSetting::Toggle { value, .. } => *value = enabled,
            _ => panic!("radar filter must be a toggle"),
        }
    };
    set_filter("Show Mobs", false);
    set_filter("Show Other Entities", false);
    assert_eq!(
        crate::graphic::hud::radar_contacts_for_test(),
        vec![([5.0, 0.0, 0.0], RadarKind::Player)]
    );
    set_filter("Show Mobs", true);
    assert_eq!(
        crate::graphic::hud::radar_contacts_for_test(),
        vec![
            ([5.0, 0.0, 0.0], RadarKind::Player),
            ([0.0, 0.0, 5.0], RadarKind::Mob)
        ]
    );
    set_filter("Show Other Entities", true);
    assert_eq!(crate::graphic::hud::radar_contacts_for_test().len(), 3);

     
     
     
    let entities = minecraft.world().unwrap().unwrap().get_entities().unwrap();
    let projectile = entities
        .iter()
        .find(|entity| entity.instance_of::<crate::mapping::entity::Projectile>())
        .expect("the fixture arrow must classify as a Projectile");
    assert!(!projectile.instance_of::<crate::mapping::entity::player::Player>());
    assert_eq!(
        entities
            .iter()
            .filter(|entity| !entity.instance_of::<crate::mapping::entity::Projectile>())
            .count(),
        6,
        "only the arrow classifies as a projectile"
    );

     
     
     
    assert_eq!(
        minecraft.options().unwrap().attack_indicator().unwrap(),
        crate::mapping::client::options::AttackIndicator::Crosshair,
        "the fixture's Attack Indicator defaults to the crosshair bar"
    );

    let fullbright = crate::module::render::fullbright::FullbrightModule::new();
    let gamma = minecraft.options().unwrap().gamma().unwrap();
    fullbright.on_start().unwrap();
    assert_eq!(gamma.get_double().unwrap(), 16.0);
    env.set_field(player.jni_ref().as_obj(), "inWater", "Z", JValue::Bool(1))
        .unwrap();
    fullbright.on_tick().unwrap();
    assert_eq!(gamma.get_double().unwrap(), 16.0);
    env.set_field(player.jni_ref().as_obj(), "inWater", "Z", JValue::Bool(0))
        .unwrap();
    fullbright.on_stop().unwrap();
    assert!((gamma.get_double().unwrap() - 0.7).abs() < 1e-9);

     
     
    call_fixture_static("pickBlock");
    assert!(minecraft.hit_block_in_reach().unwrap().is_some());
    let saved_range = env
        .call_method(
            player.jni_ref().as_obj(),
            "blockInteractionRange",
            "()D",
            &[],
        )
        .unwrap()
        .d()
        .unwrap();
    env.call_method(
        player.jni_ref().as_obj(),
        "setBlockInteractionRange",
        "(D)V",
        &[JValue::Double(3.0)],
    )
    .unwrap();
    assert!(minecraft.hit_block_in_reach().unwrap().is_none());
    env.call_method(
        player.jni_ref().as_obj(),
        "setBlockInteractionRange",
        "(D)V",
        &[JValue::Double(saved_range)],
    )
    .unwrap();
    call_fixture_static("pickFarBlock");
    assert!(minecraft.hit_block_in_reach().unwrap().is_none());
    call_fixture_static("pickMiss");
    assert!(minecraft.hit_block().unwrap().is_none());
    call_fixture_static("pickEntity");
    assert!(minecraft.hit_block().unwrap().is_none());
    assert_eq!(
        player
            .call_method("entityInteractionRange", &[])
            .unwrap()
            .d()
            .unwrap(),
        3.0
    );
    let reach = crate::module::combat::reach::ReachModule::new();
    reach.on_start().unwrap();
    reach.on_tick().unwrap();
    assert!(
        (player
            .call_method("entityInteractionRange", &[])
            .unwrap()
            .d()
            .unwrap()
            - 3.1)
            .abs()
            < 1e-6
    );
    reach.on_tick().unwrap();
    assert!(
        (player
            .call_method("entityInteractionRange", &[])
            .unwrap()
            .d()
            .unwrap()
            - 3.1)
            .abs()
            < 1e-6
    );
    player.call_method("syncAttributes", &[]).unwrap();
    assert_eq!(
        player
            .call_method("entityInteractionRange", &[])
            .unwrap()
            .d()
            .unwrap(),
        3.0
    );
    reach.on_tick().unwrap();
    assert!(
        (player
            .call_method("entityInteractionRange", &[])
            .unwrap()
            .d()
            .unwrap()
            - 3.1)
            .abs()
            < 1e-6
    );
    reach.on_stop().unwrap();
    assert_eq!(
        player
            .call_method("entityInteractionRange", &[])
            .unwrap()
            .d()
            .unwrap(),
        3.0
    );

    env.set_field(
        player.jni_ref().as_obj(),
        "fallDistance",
        "F",
        JValue::Float(15.0),
    )
    .unwrap();
    crate::module::movement::nofall::NoFallModule::new()
        .on_tick()
        .unwrap();
    assert_eq!(
        env.get_field(player.jni_ref().as_obj(), "fallDistance", "F")
            .unwrap()
            .f()
            .unwrap(),
        0.0
    );

     
     
    let packet_class = env
        .find_class("net/minecraft/network/protocol/game/ServerboundMovePlayerPacket")
        .unwrap();
    let nofall_registry = crate::module::registry::ModuleRegistry::new();
    nofall_registry.register(crate::module::movement::nofall::NoFallModule::new());
    nofall_registry
        .get(crate::module::ModuleId::NoFall)
        .unwrap()
        .lock()
        .unwrap()
        .get_module_data_mut()
        .set_enabled(true);
    nofall_registry.set_active(true);
    for (class_name, descriptor, args) in [
        (
            "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket$PosRot",
            "(DDDFFZZ)V",
            vec![
                JValue::Double(12.0),
                JValue::Double(64.0),
                JValue::Double(-9.0),
                JValue::Float(45.0),
                JValue::Float(20.0),
                JValue::Bool(0),
                JValue::Bool(1),
            ],
        ),
        (
            "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket$Pos",
            "(DDDZZ)V",
            vec![
                JValue::Double(12.0),
                JValue::Double(64.0),
                JValue::Double(-9.0),
                JValue::Bool(0),
                JValue::Bool(1),
            ],
        ),
        (
            "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket$Rot",
            "(FFZZ)V",
            vec![
                JValue::Float(45.0),
                JValue::Float(20.0),
                JValue::Bool(0),
                JValue::Bool(1),
            ],
        ),
        (
            "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket$StatusOnly",
            "(ZZ)V",
            vec![JValue::Bool(0), JValue::Bool(1)],
        ),
    ] {
        let class = env.find_class(class_name).unwrap();
        let original = env.new_object(class, descriptor, &args).unwrap();
        assert!(env.is_instance_of(&original, &packet_class).unwrap());
        let mut packet = crate::net::packet::Packet::from_outbound(&mut env, &original)
            .unwrap()
            .expect("movement packet must be recognized by outbound dispatch");
        let crate::net::packet::Packet::ServerboundMovePlayer(snapshot) = &packet else {
            unreachable!()
        };
        assert!(!snapshot.on_ground);
        assert!(snapshot.horizontal_collision);
        let snapshot = *snapshot;
        assert_eq!(
            nofall_registry.handle_packet(&mut packet),
            crate::net::packet::PacketAction::Forward
        );
        let crate::net::packet::Packet::ServerboundMovePlayer(rewritten) = packet else {
            unreachable!()
        };
        let rebuilt =
            unsafe { jni::objects::JObject::from_raw(rewritten.to_java(&mut env).unwrap()) };
        assert!(env.is_instance_of(&rebuilt, &packet_class).unwrap());
        assert!(env
            .call_method(&rebuilt, "isOnGround", "()Z", &[])
            .unwrap()
            .z()
            .unwrap());
        assert!(env
            .call_method(&rebuilt, "horizontalCollision", "()Z", &[])
            .unwrap()
            .z()
            .unwrap());
        assert_eq!(
            env.call_method(&rebuilt, "hasPosition", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            snapshot.has_position
        );
        assert_eq!(
            env.call_method(&rebuilt, "hasRotation", "()Z", &[])
                .unwrap()
                .z()
                .unwrap(),
            snapshot.has_rotation
        );
        if snapshot.has_position {
            assert_eq!(
                env.call_method(&rebuilt, "getX", "(D)D", &[JValue::Double(0.0)])
                    .unwrap()
                    .d()
                    .unwrap(),
                12.0
            );
            assert_eq!(
                env.call_method(&rebuilt, "getY", "(D)D", &[JValue::Double(0.0)])
                    .unwrap()
                    .d()
                    .unwrap(),
                64.0
            );
            assert_eq!(
                env.call_method(&rebuilt, "getZ", "(D)D", &[JValue::Double(0.0)])
                    .unwrap()
                    .d()
                    .unwrap(),
                -9.0
            );
        }
        if snapshot.has_rotation {
            assert_eq!(
                env.call_method(&rebuilt, "getYRot", "(F)F", &[JValue::Float(0.0)])
                    .unwrap()
                    .f()
                    .unwrap(),
                45.0
            );
            assert_eq!(
                env.call_method(&rebuilt, "getXRot", "(F)F", &[JValue::Float(0.0)])
                    .unwrap()
                    .f()
                    .unwrap(),
                20.0
            );
        }
    }
    nofall_registry.set_active(false);

     
     
    autoeat_regressions();

     
    call_fixture_static("leaveWorld");
    assert!(
        minecraft.player().unwrap().is_none(),
        "the player must be gone after leaving the world",
    );
    assert!(!minecraft.in_world());
}

fn autoeat_regressions() {
    use crate::module::{ModuleCategory, ModuleId};
    let modules = &crate::state::client().modules;
    modules.register(crate::module::world::autoeat::AutoEatModule::new());
    modules.register(crate::module::combat::autoclicker::AutoClickerModule::new());
    let eat = modules.get(ModuleId::AutoEat).unwrap();
    let clicker = modules.get(ModuleId::AutoClicker).unwrap();
    eat.lock().unwrap().get_module_data_mut().enabled = true;
    let invoke = |method: &str| {
        jvm()
            .attach_current_thread_as_daemon()
            .unwrap()
            .call_static_method("AutoEatProbe", method, "()V", &[])
            .unwrap();
    };
    let prepare = |scenario: i32| {
        jvm()
            .attach_current_thread_as_daemon()
            .unwrap()
            .call_static_method("AutoEatProbe", "prepare", "(I)V", &[JValue::Int(scenario)])
            .unwrap();
    };
    modules.set_active(true);

     
    assert_eq!(
        eat.lock().unwrap().get_module_data().category,
        ModuleCategory::World
    );
    crate::config::set_expanded(ModuleId::AutoEat, true);
    let ctx = egui::Context::default();
    let mut painted = Vec::new();
    for frame in 0..20 {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600.0, 1000.0),
                )),
                time: Some(frame as f64 / 30.0),
                ..Default::default()
            },
            |ctx| crate::graphic::menu::draw(ctx, 1.0, 0.0, 0.0, 0.0),
        );
        fn text(shape: &egui::Shape, output: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(text) => output.push(text.galley.job.text.clone()),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        text(shape, output);
                    }
                }
                _ => {}
            }
        }
        for shape in output.shapes {
            text(&shape.shape, &mut painted);
        }
    }
    assert!(
        painted.iter().any(|text| text == "AutoEat"),
        "AutoEat menu row"
    );
    assert!(
        painted.iter().any(|text| text.contains("Hunger Threshold")),
        "editable threshold in menu"
    );
    crate::config::set_expanded(ModuleId::AutoEat, false);

    for scenario in [0, 3, 5, 6, 7] {
        prepare(scenario);
        modules.tick();
        invoke("assertIdle");
        assert!(!crate::module::world::autoeat::is_eating());
    }
     
    for (scenario, enabled) in [(1, true), (2, true), (2, false)] {
        prepare(scenario);
        clicker.lock().unwrap().get_module_data_mut().enabled = enabled;
        modules.tick();
        invoke("assertEating");
        assert!(crate::module::world::autoeat::is_eating());
        assert_eq!(clicker.lock().unwrap().get_module_data().enabled, enabled);
        modules.tick();
        invoke("assertEating");  
        invoke("finishFood");
        modules.tick();
        invoke("assertRestored");
        assert!(!crate::module::world::autoeat::is_eating());
        assert_eq!(clicker.lock().unwrap().get_module_data().enabled, enabled);
        invoke(if enabled {
            "assertOneAttack"
        } else {
            "assertNoAttack"
        });
    }
    clicker.lock().unwrap().get_module_data_mut().enabled = false;
    prepare(4);
    modules.tick();
    invoke("assertRestored");  
    assert!(!crate::module::world::autoeat::is_eating());

    for cancel in 0..5 {
        prepare(2);
        modules.tick();
        invoke("assertEating");
        match cancel {
            0 => eat.lock().unwrap().on_stop().unwrap(),
            1 => {
                invoke("interrupt");
                modules.tick();
            }
            2 => {
                crate::graphic::input::GUI_OPEN.store(true, std::sync::atomic::Ordering::Relaxed);
                modules.tick();
                crate::graphic::input::GUI_OPEN.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            3 => modules.set_active(false),
            _ => {
                crate::state::minecraft()
                    .player()
                    .unwrap()
                    .unwrap()
                    .get_inventory()
                    .unwrap()
                    .set_selected(4)
                    .unwrap();
                modules.tick();
            }
        }
        invoke("assertRestored");
        assert!(!crate::module::world::autoeat::is_eating());
        modules.set_active(true);
    }
     
    prepare(1);
    eat.lock()
        .unwrap()
        .get_module_data_mut()
        .get_setting_mut("Hunger Threshold")
        .unwrap()
        .set_slider_value(8.0);
    modules.tick();
    invoke("assertIdle");
    eat.lock()
        .unwrap()
        .get_module_data_mut()
        .get_setting_mut("Hunger Threshold")
        .unwrap()
        .set_slider_value(14.0);
    clicker.lock().unwrap().get_module_data_mut().enabled = true;
    modules.tick();
    invoke("assertEating");
     
    clicker.lock().unwrap().get_module_data_mut().enabled = false;
    invoke("finishFood");
    modules.tick();
    invoke("assertRestored");
    invoke("assertNoAttack");
     
    prepare(2);
    let mut timeout_eat = crate::module::world::autoeat::AutoEatModule::new();
    timeout_eat.module.enabled = true;
    crate::module::Module::on_tick(&timeout_eat).unwrap();
    invoke("assertEating");
    timeout_eat.expire_session_for_test();
    crate::module::Module::on_tick(&timeout_eat).unwrap();
    invoke("assertRestored");
    assert!(!crate::module::world::autoeat::is_eating());

     
    prepare(2);
    modules.tick();
    invoke("assertEating");
    call_fixture_static("leaveWorld");
    modules.set_active(false);
    assert!(!crate::module::world::autoeat::is_eating());
    assert!(!crate::state::minecraft()
        .options()
        .unwrap()
        .key_use()
        .unwrap()
        .is_down()
        .unwrap());
    eat.lock().unwrap().get_module_data_mut().enabled = false;
}

 
 
 
 
 
 
 
 
#[test]
fn explosion_particle_gate_swallows_and_releases_the_spawn() {
    use crate::jvmti::{
        transform_class, EXPLOSION_ALWAYS_VISIBLE_TRANSFORM_PARAMS,
        EXPLOSION_FORCED_PARTICLE_TRANSFORM_PARAMS, EXPLOSION_SOUND_TRANSFORM_PARAMS,
        EXPLOSION_SUPPRESS_TRANSFORM_PARAMS,
    };
    use jni::objects::JObject;
    use jni::sys::jboolean;

    let jvm = jvm();
    let mut env = jvm.attach_current_thread_as_daemon().unwrap();

     
     
    let mut p_add = EXPLOSION_SUPPRESS_TRANSFORM_PARAMS;
    p_add.target_class = "ExplosionGateProbe";
    p_add.method = "addParticle";
    p_add.descriptor = "(Ljava/lang/Object;DDDDDD)V";
    p_add.bridge_class = "ExplosionGateProbe";

    let mut p_always = EXPLOSION_ALWAYS_VISIBLE_TRANSFORM_PARAMS;
    p_always.target_class = "ExplosionGateProbe";
    p_always.method = "addAlwaysVisibleParticle";
    p_always.descriptor = "(Ljava/lang/Object;DDDDDD)V";
    p_always.bridge_class = "ExplosionGateProbe";

    let mut p_forced = EXPLOSION_FORCED_PARTICLE_TRANSFORM_PARAMS;
    p_forced.target_class = "ExplosionGateProbe";
    p_forced.method = "addForcedParticle";
    p_forced.descriptor = "(Ljava/lang/Object;ZDDDDDD)V";
    p_forced.bridge_class = "ExplosionGateProbe";

    let mut p_sound = EXPLOSION_SOUND_TRANSFORM_PARAMS;
    p_sound.target_class = "ExplosionGateProbe";
    p_sound.method = "playLocalSound";
    p_sound.descriptor = "(DDDLjava/lang/Object;Ljava/lang/Object;FFZ)V";
    p_sound.bridge_class = "ExplosionGateProbe";

    let all_params = [p_add, p_always, p_forced, p_sound];

    let source = std::fs::read(fixture_classes().join("ExplosionGateProbe.class"))
        .expect("the gate fixture must be compiled");
    let transformed = transform_class(&source, &all_params)
        .expect("transform must not error")
        .expect("the explosion gates must be transformed");
    assert_ne!(transformed, source, "the gate must change the bytes");
    assert!(
        transform_class(&transformed, &all_params)
            .unwrap()
            .is_none(),
        "idempotent hot-reload"
    );

    let loader = env
        .call_static_method(
            "java/lang/ClassLoader",
            "getSystemClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )
        .expect("system class loader")
        .l()
        .map(JObject::from)
        .expect("system class loader non-null");
    let class = env
        .define_class("ExplosionGateProbe", &loader, &transformed)
        .expect("the explosion gate must pass -Xverify:all");
    let probe = JObject::from(
        env.new_object(&class, "()V", &[])
            .expect("instantiate probe"),
    );
    let particle = JObject::from(
        env.new_object("java/lang/Object", "()V", &[])
            .expect("particle stand-in"),
    );

    let invoke_all = |env: &mut jni::JNIEnv| {
        for method in ["addParticle", "addAlwaysVisibleParticle"] {
            env.call_method(
                &probe,
                method,
                "(Ljava/lang/Object;DDDDDD)V",
                &[
                    JValue::Object(&particle),
                    JValue::Double(1.0),
                    JValue::Double(2.0),
                    JValue::Double(3.0),
                    JValue::Double(0.5),
                    JValue::Double(0.5),
                    JValue::Double(0.5),
                ],
            )
            .expect("particle spawn must be callable");
        }
        env.call_method(
            &probe,
            "addForcedParticle",
            "(Ljava/lang/Object;ZDDDDDD)V",
            &[
                JValue::Object(&particle),
                JValue::Bool(jboolean::from(true)),
                JValue::Double(1.0),
                JValue::Double(2.0),
                JValue::Double(3.0),
                JValue::Double(0.5),
                JValue::Double(0.5),
                JValue::Double(0.5),
            ],
        )
        .expect("forced particle spawn must be callable");
        env.call_method(
            &probe,
            "playLocalSound",
            "(DDDLjava/lang/Object;Ljava/lang/Object;FFZ)V",
            &[
                JValue::Double(1.0),
                JValue::Double(2.0),
                JValue::Double(3.0),
                JValue::Object(&particle),
                JValue::Object(&particle),
                JValue::Float(4.0),
                JValue::Float(1.0),
                JValue::Bool(jboolean::from(false)),
            ],
        )
        .expect("local sound must be callable");
    };
    let set_gate = |env: &mut jni::JNIEnv, up: bool| {
        env.call_static_method(
            &class,
            "setSuppress",
            "(Z)V",
            &[JValue::Bool(jboolean::from(up))],
        )
        .expect("set the gate flag");
    };
    let spawned = |env: &mut jni::JNIEnv| {
        env.call_static_method(&class, "spawned", "()J", &[])
            .expect("read the spawn counter")
            .j()
            .unwrap()
    };
    let sounds = |env: &mut jni::JNIEnv| {
        env.call_static_method(&class, "soundsPlayed", "()J", &[])
            .expect("read the sound counter")
            .j()
            .unwrap()
    };

     
    set_gate(&mut env, false);
    invoke_all(&mut env);
    let after_clear = spawned(&mut env);
    let sounds_after_clear = sounds(&mut env);
    assert!(after_clear > 0, "the ungated particle bodies must spawn");
    assert!(sounds_after_clear > 0, "the ungated sound body must play");

     
    set_gate(&mut env, true);
    invoke_all(&mut env);
    assert_eq!(
        spawned(&mut env),
        after_clear,
        "the gate must swallow all three particle spawns"
    );
    assert_eq!(
        sounds(&mut env),
        sounds_after_clear,
        "the gate must swallow the local explosion sound"
    );

     
    set_gate(&mut env, false);
    invoke_all(&mut env);
    assert!(
        spawned(&mut env) > after_clear,
        "releasing the gate must let particles spawn again"
    );
    assert!(
        sounds(&mut env) > sounds_after_clear,
        "releasing the gate must let sounds play again"
    );
}

 
 
 
 
 
 
 
 
 
 
#[test]
fn prologue_guard_survives_hotspot_split_verifier() {
    use crate::jvmti::{transform_class, TransformParams};
    use crate::mapping::MinecraftClassType;
    use jni::objects::{JObject, JValue};
    use jni::sys::jboolean;

    let jvm = jvm();
    let mut env = jvm.attach_current_thread_as_daemon().unwrap();

    let params = TransformParams {
        target_class: "GuardProbe",
        method: "runPartial",
        descriptor: "(ZLnet/minecraft/class_4604;Ljava/util/List;Lnet/minecraft/class_243;Lit/unimi/dsi/fastutil/longs/LongOpenHashSet;)V",
        bridge_class: "GuardProbe",
        bridge_method: "",
        bridge_descriptor: "",
        class_type: MinecraftClassType::SectionOcclusionGraph,
        log_label: "GuardProbe",
        replace_entire_body: false,
        prepend_object_call: None,
        replace_with_bridge_return: false,
        guard_flag: Some("PARTIAL_SUPPRESSED"),
    };

    let source = std::fs::read(fixture_classes().join("GuardProbe.class")).expect("fixture");
    let transformed = transform_class(&source, &[params])
        .expect("transform must not error")
        .expect("the probe must be transformed");
    assert_ne!(
        transformed, source,
        "the prologue splice must change the bytes"
    );

     
     
     
     
    let string_cls = env.find_class("java/lang/String").unwrap();
    let loader = env
        .call_method(
            string_cls,
            "getClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )
        .expect("String's loader")
        .l()
        .map(JObject::from)
        .expect("ClassLoader non-null");
    let class = env
        .define_class("GuardProbe", &loader, &transformed)
        .expect("transformed GuardProbe must pass HotSpot's eager split verifier");

     
    let list = env.new_object("java/util/ArrayList", "()V", &[]).unwrap();
    let list_obj = JObject::from(list);
    for value in ["a", "b"] {
        let element = env.new_string(value).unwrap();
        let element = JObject::from(element);
        env.call_method(
            &list_obj,
            "add",
            "(Ljava/lang/Object;)Z",
            &[JValue::Object(&element)],
        )
        .unwrap();
    }

    let probe = match env.new_object(&class, "()V", &[]) {
        Ok(probe) => probe,
        Err(error) => {
             
             
            if env.exception_check().unwrap_or(false) {
                env.exception_describe().ok();
                env.exception_clear().ok();
            }
            panic!("instantiate GuardProbe: {error}");
        }
    };
    let probe_ref = JObject::from(probe);
     
     
     
     
    let frustum = env
        .new_object("net/minecraft/class_4604", "()V", &[])
        .expect("instantiate stub");
    let pos = env
        .new_object("net/minecraft/class_243", "()V", &[])
        .expect("instantiate stub");
    let long_set = env
        .new_object("it/unimi/dsi/fastutil/longs/LongOpenHashSet", "()V", &[])
        .expect("instantiate stub");

     
     
     
    let args = |flag: bool| {
        vec![
            JValue::Bool(jboolean::from(flag)),
            JValue::Object(&frustum),
            JValue::Object(&list_obj),
            JValue::Object(&pos),
            JValue::Object(&long_set),
        ]
    };
    env.call_method(
        &probe_ref,
        "runPartial",
        "(ZLnet/minecraft/class_4604;Ljava/util/List;Lnet/minecraft/class_243;Lit/unimi/dsi/fastutil/longs/LongOpenHashSet;)V",
        &args(false),
    )
    .expect("body path must execute with the guard clear");
    let sink_after_body = env
        .call_static_method(&class, "sink", "()J", &[])
        .expect("read the side-effect sink")
        .j()
        .unwrap();
    assert!(sink_after_body > 0, "the vanilla body must have run once");

     
     
    env.call_static_method(
        &class,
        "setPartial",
        "(Z)V",
        &[JValue::Bool(jboolean::from(true))],
    )
    .expect("raise PARTIAL_SUPPRESSED");
    env.call_method(
        &probe_ref,
        "runPartial",
        "(ZLnet/minecraft/class_4604;Ljava/util/List;Lnet/minecraft/class_243;Lit/unimi/dsi/fastutil/longs/LongOpenHashSet;)V",
        &args(false),
    )
    .expect("guarded RETURN path must execute with the flag set");
    let sink_after_suppressed = env
        .call_static_method(&class, "sink", "()J", &[])
        .expect("read the side-effect sink")
        .j()
        .unwrap();
    assert_eq!(
        sink_after_suppressed, sink_after_body,
        "the suppressed call must not modify the sink"
    );

     
    env.call_static_method(
        &class,
        "setPartial",
        "(Z)V",
        &[JValue::Bool(jboolean::from(false))],
    )
    .expect("lower PARTIAL_SUPPRESSED");
    env.call_method(
        &probe_ref,
        "runPartial",
        "(ZLnet/minecraft/class_4604;Ljava/util/List;Lnet/minecraft/class_243;Lit/unimi/dsi/fastutil/longs/LongOpenHashSet;)V",
        &args(true),
    )
    .expect("body path must execute again after lowering the flag");
    let sink_after_second_body = env
        .call_static_method(&class, "sink", "()J", &[])
        .expect("read the side-effect sink")
        .j()
        .unwrap();
    assert!(
        sink_after_second_body > sink_after_suppressed,
        "the second body call must add more to the sink than the suppressed call did"
    );
}

 
 
#[test]
fn pause_menu_hides_vanilla_buttons_and_runs_their_action() {
    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();

     
     
    let modules = &crate::state::client().modules;
    if modules.get(crate::module::ModuleId::CustomUi).is_none() {
        modules.register(crate::module::render::custom_ui::CustomUiModule::new());
    }
    let custom = modules
        .get(crate::module::ModuleId::CustomUi)
        .expect("Custom UI is registered");
    custom
        .lock()
        .unwrap()
        .get_module_data_mut()
        .set_enabled(true);

     
     
     
    call_fixture_static("enterWorld");
    call_fixture_static("resetPressProbe");
    call_fixture_static("openPauseMenu");
    crate::graphic::pause_menu::update();
    assert!(
        crate::graphic::pause_menu::wants_input(),
        "the pause menu must hand pointer input to the overlay"
    );
    assert_eq!(
        fixture_pause_buttons(),
        ((false, false), (0, 0)),
        "both vanilla buttons must be hidden before anything is clicked"
    );

     
     
     
    let ctx = egui::Context::default();
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));
    let target = egui::pos2(400.0, 120.0);
     
     
    for (time, pressed) in [(0.0, None), (0.05, Some(true)), (0.1, Some(false))] {
        let mut input = egui::RawInput::default();
        input.time = Some(time);
        input.screen_rect = Some(viewport);
        input.events.push(egui::Event::PointerMoved(target));
        if let Some(pressed) = pressed {
            input.events.push(egui::Event::PointerButton {
                pos: target,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
        }
        let _ = ctx.run(input, |ctx| crate::graphic::pause_menu::draw(ctx));
    }
     
     
     
    assert_eq!(
        fixture_pause_buttons(),
        ((false, false), (0, 0)),
        "a click must not run while the menu is still being painted"
    );
    crate::graphic::pause_menu::update();
    assert_eq!(
        fixture_pause_buttons(),
        ((true, true), (1, 0)),
        "the deferred click must press exactly the vanilla button under it, then restore visibility"
    );
    assert!(
        fixture_sibling_visible_at_press(),
        "the widgets must be visible again before the button's own action runs"
    );

     
     
    custom
        .lock()
        .unwrap()
        .get_module_data_mut()
        .set_enabled(false);
    crate::graphic::pause_menu::update();
    assert!(
        !crate::graphic::pause_menu::wants_input(),
        "the replacement must stand down when the module is switched off"
    );
    assert_eq!(
        fixture_pause_buttons().0,
        (true, true),
        "the vanilla buttons must be visible again"
    );

     
     
    call_fixture_static("closeScreen");
    call_fixture_static("leaveWorld");
    crate::graphic::pause_menu::update();
    assert!(
        !crate::graphic::pause_menu::wants_input(),
        "the replacement must stand down when the pause screen closes"
    );
}

 
 
 
 
#[test]
fn pause_menu_stands_down_when_the_world_is_left() {
    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();

    let modules = &crate::state::client().modules;
    if modules.get(crate::module::ModuleId::CustomUi).is_none() {
        modules.register(crate::module::render::custom_ui::CustomUiModule::new());
    }
    let custom = modules
        .get(crate::module::ModuleId::CustomUi)
        .expect("Custom UI is registered");
    custom
        .lock()
        .unwrap()
        .get_module_data_mut()
        .set_enabled(true);

    call_fixture_static("enterWorld");
    call_fixture_static("openPauseMenu");
    crate::graphic::pause_menu::update();
    assert!(
        crate::graphic::pause_menu::wants_input(),
        "the replacement must own the pause screen inside a world"
    );
    assert_eq!(
        fixture_pause_buttons().0,
        (false, false),
        "the vanilla buttons are hidden while the replacement draws them"
    );

     
     
     
    call_fixture_static("leaveWorld");
    custom
        .lock()
        .unwrap()
        .on_stop()
        .expect("the world-suspend path");
    crate::graphic::pause_menu::update();
    assert!(
        !crate::graphic::pause_menu::wants_input(),
        "the replacement must stand down once the world is gone"
    );
    assert_eq!(
        fixture_pause_buttons().0,
        (false, false),
        "a dead session's widgets must not be touched again"
    );

     
     
    call_fixture_static("closeScreen");
    crate::graphic::pause_menu::update();
    call_fixture_static("enterWorld");
    call_fixture_static("openPauseMenu");
    crate::graphic::pause_menu::update();
    assert!(
        crate::graphic::pause_menu::wants_input(),
        "the next pause screen must be replaced again"
    );
    assert_eq!(fixture_pause_buttons().0, (false, false));

    custom
        .lock()
        .unwrap()
        .get_module_data_mut()
        .set_enabled(false);
    crate::graphic::pause_menu::update();
    assert_eq!(
        fixture_pause_buttons().0,
        (true, true),
        "switching the module off inside a world restores the live buttons"
    );
    call_fixture_static("closeScreen");
    call_fixture_static("leaveWorld");
    crate::graphic::pause_menu::update();
    assert!(!crate::graphic::pause_menu::wants_input());
}

 
fn fixture_sibling_visible_at_press() -> bool {
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    let class = env
        .find_class("net/minecraft/client/Minecraft")
        .expect("fixture Minecraft");
    env.get_static_field(&class, "siblingVisibleAtPress", "Z")
        .expect("read the press probe")
        .z()
        .unwrap()
}

 
 
 
#[test]
fn a_display_module_draws_its_widget_without_custom_ui() {
    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();

    let modules = &crate::state::client().modules;
    if modules.get(crate::module::ModuleId::CustomUi).is_none() {
        modules.register(crate::module::render::custom_ui::CustomUiModule::new());
    }
    if modules.get(crate::module::ModuleId::Fps).is_none() {
        modules.register(crate::module::render::legit::fps());
    }
     
    modules
        .get(crate::module::ModuleId::CustomUi)
        .unwrap()
        .lock()
        .unwrap()
        .get_module_data_mut()
        .set_enabled(false);
    modules
        .get(crate::module::ModuleId::Fps)
        .unwrap()
        .lock()
        .unwrap()
        .get_module_data_mut()
        .set_enabled(true);

     
     
    assert!(
        hud_texts().iter().any(|text| text == "FPS"),
        "the FPS widget must appear while Custom UI is off"
    );

     
    modules
        .get(crate::module::ModuleId::Fps)
        .unwrap()
        .lock()
        .unwrap()
        .get_module_data_mut()
        .set_enabled(false);
    assert!(
        !hud_texts().iter().any(|text| text == "FPS"),
        "disabling the display module must remove its widget"
    );
}

 
fn hud_texts() -> Vec<String> {
    let ctx = egui::Context::default();
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(viewport),
            time: Some(1.0),
            ..Default::default()
        },
        |ctx| crate::graphic::hud::draw(ctx, true),
    );
    output
        .shapes
        .into_iter()
        .filter_map(|clipped| match clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

 
fn fixture_pause_buttons() -> ((bool, bool), (i32, i32)) {
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();
    let class = env
        .find_class("net/minecraft/client/Minecraft")
        .expect("fixture Minecraft");
    let visible = |env: &mut jni::JNIEnv, index: i32| {
        env.call_static_method(&class, "pauseButtonVisible", "(I)Z", &[JValue::Int(index)])
            .unwrap()
            .z()
            .unwrap()
    };
    let presses = |env: &mut jni::JNIEnv, index: i32| {
        env.call_static_method(&class, "pauseButtonPresses", "(I)I", &[JValue::Int(index)])
            .unwrap()
            .i()
            .unwrap()
    };
    (
        (visible(&mut env, 0), visible(&mut env, 1)),
        (presses(&mut env, 0), presses(&mut env, 1)),
    )
}

 
 
 
#[test]
fn custom_hotbar_tracks_the_selected_slot_and_its_switch() {
    use crate::module::ModuleId;

    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();

    let modules = &crate::state::client().modules;
    if modules.get(ModuleId::CustomUi).is_none() {
        modules.register(crate::module::render::custom_ui::CustomUiModule::new());
    }
    call_fixture_static("enterWorld");
    let custom = modules
        .get(ModuleId::CustomUi)
        .expect("Custom UI is registered");
    {
        let mut module = custom.lock().unwrap();
        module.get_module_data_mut().set_enabled(true);
        if let Some(crate::module::ModuleSetting::Toggle { value, .. }) =
            module.get_module_data_mut().get_setting_mut("Hotbar")
        {
            *value = true;
        }
    }

     
    let player = crate::state::minecraft().player().unwrap().unwrap();
    for slot in [6, 2, 8] {
        player.get_inventory().unwrap().set_selected(slot).unwrap();
        assert_eq!(
            crate::graphic::hotbar::snapshot()
                .expect("a snapshot inside a world")
                .selected,
            slot as usize
        );
    }

     
    let on = hud_texts();
    for number in 1..=9 {
        assert!(
            on.iter().any(|text| text == &number.to_string()),
            "slot number {number} must be painted while the custom hotbar is on"
        );
    }

     
     
    if let Some(crate::module::ModuleSetting::Toggle { value, .. }) = custom
        .lock()
        .unwrap()
        .get_module_data_mut()
        .get_setting_mut("Hotbar")
    {
        *value = false;
    }
    let off = hud_texts();
    let missing = (1..=9)
        .filter(|number| !off.iter().any(|text| text == &number.to_string()))
        .count();
    assert!(
        missing > 0,
        "switching the custom hotbar off must stop painting its slot numbers"
    );

    custom
        .lock()
        .unwrap()
        .get_module_data_mut()
        .set_enabled(false);
    call_fixture_static("leaveWorld");
}

 
 
 
#[test]
fn use_item_packet_snapshot_round_trips_and_carries_a_silent_aim() {
    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();
    let mut env = jvm().attach_current_thread_as_daemon().unwrap();

    let hand_class = env
        .find_class("net/minecraft/world/InteractionHand")
        .unwrap();
    let main_hand = env
        .get_static_field(
            &hand_class,
            "MAIN_HAND",
            "Lnet/minecraft/world/InteractionHand;",
        )
        .unwrap()
        .l()
        .unwrap();
    let class = env
        .find_class("net/minecraft/network/protocol/game/ServerboundUseItemPacket")
        .unwrap();
    let original = env
        .new_object(
            &class,
            "(Lnet/minecraft/world/InteractionHand;IFF)V",
            &[
                JValue::Object(&main_hand),
                JValue::Int(11),
                JValue::Float(30.0),
                JValue::Float(-10.0),
            ],
        )
        .unwrap();

    let mut packet = crate::net::packet::Packet::from_outbound(&mut env, &original)
        .unwrap()
        .expect("a use item packet must be recognized by outbound dispatch");
    let crate::net::packet::Packet::ServerboundUseItem(snapshot) = &mut packet else {
        panic!("expected a use item snapshot");
    };
    assert_eq!(snapshot.sequence, 11);
    assert!(snapshot.main_hand, "the hand must be read from the packet");
    snapshot.apply_aim(crate::module::combat::rotation::Rotation::new(-45.0, 90.0));

    let crate::net::packet::Packet::ServerboundUseItem(rewritten) = packet else {
        unreachable!()
    };
    let rebuilt = unsafe { jni::objects::JObject::from_raw(rewritten.to_java(&mut env).unwrap()) };
    assert!(env.is_instance_of(&rebuilt, &class).unwrap());
    assert_eq!(
        env.call_method(&rebuilt, "getYRot", "()F", &[])
            .unwrap()
            .f()
            .unwrap(),
        -45.0
    );
    assert_eq!(
        env.call_method(&rebuilt, "getXRot", "()F", &[])
            .unwrap()
            .f()
            .unwrap(),
        90.0
    );
    assert_eq!(
        env.call_method(&rebuilt, "getSequence", "()I", &[])
            .unwrap()
            .i()
            .unwrap(),
        11,
        "the interaction sequence must survive the rebuild"
    );
    let hand = env
        .call_method(
            &rebuilt,
            "getHand",
            "()Lnet/minecraft/world/InteractionHand;",
            &[],
        )
        .unwrap()
        .l()
        .unwrap();
    assert!(
        env.is_same_object(&hand, &main_hand).unwrap(),
        "the hand the use came from must survive the rebuild"
    );
}

 
 
 
#[test]
fn edit_ui_drags_a_hud_widget_into_its_new_slot() {
    let _shared = shared_fixture_lock();
    jvm();
    ensure_client_state();
    crate::config::reset_ui_state();

    struct CloseOnDrop;
    impl Drop for CloseOnDrop {
        fn drop(&mut self) {
            crate::module::render::custom_ui::set_editing(false);
            crate::config::reset_ui_state();
        }
    }
    let _reset = CloseOnDrop;

    crate::graphic::edit_ui::open();
    assert!(
        crate::graphic::edit_ui::is_open(),
        "the editor must open even though Custom UI is dormant"
    );
    assert!(
        crate::state::client()
            .modules
            .get(crate::module::ModuleId::CustomUi)
            .is_none()
            || !crate::module::render::custom_ui::is_enabled(),
        "the editor must not depend on the custom look being on"
    );

    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));
    let run = |time: f64, events: Vec<egui::Event>| {
        let mut input = egui::RawInput::default();
        input.time = Some(time);
        input.screen_rect = Some(screen);
        input.events = events;
        ctx.run(input, crate::graphic::edit_ui::draw)
    };
    let button = |pos: egui::Pos2, pressed: bool| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };

     
    for frame in 0..8 {
        run(frame as f64 * 0.1, Vec::new());
    }

    let radar =
        crate::graphic::hud::widget_target_pos(crate::graphic::hud::HudWidgetId::Radar, &screen);
    let grab = radar + egui::vec2(30.0, 30.0);
    let moved_to = grab + egui::vec2(48.0, 32.0);

    run(
        0.85,
        vec![egui::Event::PointerMoved(grab), button(grab, true)],
    );
    run(0.95, vec![egui::Event::PointerMoved(moved_to)]);
    run(1.05, vec![button(moved_to, false)]);

    let moved = crate::config::widget_pos(crate::graphic::hud::HudWidgetId::Radar)
        .expect("dragging a card must record the widget's new slot");
    assert!(
        moved[0] > radar.x && moved[1] > radar.y,
        "the recorded slot {moved:?} must follow the drag from {radar:?}"
    );
     
    let grid = crate::module::render::custom_ui::EditorStyle::read()
        .grid_size
        .max(1.0);
    assert!(
        (moved[0] / grid).round() * grid - moved[0] < 0.01,
        "slot {moved:?} must sit on the {grid}px grid"
    );
}
