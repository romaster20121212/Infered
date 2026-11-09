 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

pub mod packet;

use crate::mapping::{MappedObject, MinecraftClassType};
use crate::net::packet::{Packet, PacketAction};
use crate::state::{mapping, minecraft};
use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::sys::jobject;
use jni::{JNIEnv, NativeMethod};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

 
const HANDLER_BYTECODE: &[u8] = include_bytes!(env!("INFERED_CHANNEL_HANDLER_CLASS"));
 
const HANDLER_CLASS: &str = "InferedChannelHandler";
 
const PIPELINE_NAME: &str = "infered_handler";

 
 
 
const SIG_CHANNEL_FIELD: &str = "Lio/netty/channel/Channel;";
const SIG_PIPELINE: &str = "()Lio/netty/channel/ChannelPipeline;";
const SIG_PIPELINE_GET: &str = "(Ljava/lang/String;)Lio/netty/channel/ChannelHandler;";
const SIG_PIPELINE_REMOVE: &str = "(Ljava/lang/String;)Lio/netty/channel/ChannelHandler;";
const SIG_PIPELINE_ADD_BEFORE: &str = "(Ljava/lang/String;Ljava/lang/String;Lio/netty/channel/ChannelHandler;)Lio/netty/channel/ChannelPipeline;";

 
 
 
const MC_PACKET_HANDLER: &str = "packet_handler";

 
 
const MAX_DEFINE_ATTEMPTS: u32 = 20;

struct NetState {
     
    handler_class: Option<GlobalRef>,
     
    installed_on: Option<GlobalRef>,
     
    define_attempts: u32,
}

static STATE: Mutex<NetState> = Mutex::new(NetState {
    handler_class: None,
    installed_on: None,
    define_attempts: 0,
});

 
 
 
 
 
fn state() -> std::sync::MutexGuard<'static, NetState> {
    STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
 
 
 
 
 
 
static INJECTING: AtomicUsize = AtomicUsize::new(0);

 
 
 
 
 
 
 
pub(crate) fn send_packet(packet: jobject) -> anyhow::Result<()> {
    let packet = unsafe { JObject::from_raw(packet) };
    let mut env = mapping().get_env()?;
    let connection = current_connection(&mut env)?
        .ok_or_else(|| anyhow::anyhow!("no live connection to send through"))?;

    INJECTING.fetch_add(1, Ordering::Relaxed);
    let result = mapping().call_method(
        MinecraftClassType::Connection,
        connection.as_obj(),
        "send",
        &[JValue::Object(&packet)],
    );
    INJECTING.fetch_sub(1, Ordering::Relaxed);

    match result {
        Ok(_) => {
             
             
            clear_pending_exception();
            Ok(())
        }
        Err(error) => Err(error),
    }
}

 
 
static LAST_INSTALL_FAILURE: Mutex<Option<String>> = Mutex::new(None);

 
 
 
 
 
fn with_pending_java_detail(
    env: &mut JNIEnv,
    what: &str,
    error: jni::errors::Error,
) -> anyhow::Error {
    if !env.exception_check().unwrap_or(false) {
        return error.into();
    }
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
            let jstr = JString::from(obj);
            let text = env.get_string(&jstr).ok()?;
            Some(text.to_string_lossy().into_owned())
        });
    match detail {
        Some(text) => anyhow::anyhow!("{what}: Java exception was thrown: {text}"),
        None => anyhow::anyhow!("{what}: Java exception was thrown"),
    }
}

 
 
pub fn ensure_installed() {
    match ensure_installed_inner() {
        Ok(()) => {
             
            let mut last = LAST_INSTALL_FAILURE
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            if last.take().is_some() {
                log::warn!("net: install pass recovered — connection/pipeline is live again");
            }
        }
        Err(error) => {
            let detail = format!("{error:#}");
            let mut last = LAST_INSTALL_FAILURE
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            if last.as_deref() != Some(detail.as_str()) {
                log::debug!("net: install pass failed: {detail}");
                if log::log_enabled!(log::Level::Debug) {
                    *last = Some(detail);
                }
            }
        }
    }
    clear_pending_exception();
}

 
 
 
 
 
 
 
static LAST_CLEARED_JNI_DETAIL: Mutex<Option<String>> = Mutex::new(None);

 
 
fn clear_pending_exception() {
    if let Ok(mut env) = mapping().get_env() {
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
                    let jstr = JString::from(obj);
                    let text = env.get_string(&jstr).ok()?;
                    Some(text.to_string_lossy().into_owned())
                });
            if let Some(cause) = detail {
                let mut last = LAST_CLEARED_JNI_DETAIL
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                if last.as_deref() != Some(cause.as_str()) {
                    log::warn!("net: cleared a stray JNI exception (Java detail: {cause})");
                    if log::log_enabled!(log::Level::Warn) {
                        *last = Some(cause);
                    }
                }
            } else {
                let _ = env.exception_clear();
            }
        }
    }
}

fn ensure_installed_inner() -> anyhow::Result<()> {
     
    {
        let mut state = state();
        if state.handler_class.is_none() {
            if state.define_attempts >= MAX_DEFINE_ATTEMPTS {
                return Ok(());
            }
            match define_handler() {
                Ok(class) => {
                    log::info!("net: InferedChannelHandler defined and bound");
                    state.handler_class = Some(class);
                    state.define_attempts = 0;
                }
                Err(error) => {
                    state.define_attempts += 1;
                    log::warn!(
                        "net: handler definition failed (attempt {}/{}): {error}",
                        state.define_attempts,
                        MAX_DEFINE_ATTEMPTS
                    );
                    return Ok(());
                }
            }
        }
    }

    let mut env = mapping().get_env()?;

     
    let Some(connection) = current_connection(&mut env)? else {
        state().installed_on = None;
        return Ok(());
    };

     
    {
        let state = state();
        if let Some(previous) = &state.installed_on {
            if env.is_same_object(previous, &connection)? {
                return Ok(());
            }
        }
    }

    install_on(&mut env, &connection)?;
     
     
     
    let reinstalled = state().installed_on.replace(connection).is_some();
    if reinstalled {
        log::info!(
            "net: handler re-installed — the connection changed (server switch / reconnect)"
        );
    } else {
        log::info!("net: handler installed on the connection pipeline");
    }
    Ok(())
}

 
 
 
 
 
 
 
 
 
fn define_handler() -> anyhow::Result<GlobalRef> {
    let mut env = mapping().get_env()?;
    let loader = mapping()
        .game_class_loader()
        .ok_or_else(|| anyhow::anyhow!("game class loader not captured yet"))?;

    let class = match load_existing_handler(&mut env, &loader) {
        Some(existing) => existing,
        None => env
            .define_class(HANDLER_CLASS, loader.as_obj(), HANDLER_BYTECODE)
            .map_err(|error| describe_jni_error(&mut env, "DefineClass", error))?,
    };

    let methods = [
        NativeMethod {
            name: "onOutbound".into(),
            sig: "(Ljava/lang/Object;)Ljava/lang/Object;".into(),
            fn_ptr: dark_on_outbound as *mut c_void,
        },
        NativeMethod {
            name: "onInbound".into(),
            sig: "(Ljava/lang/Object;)Ljava/lang/Object;".into(),
            fn_ptr: dark_on_inbound as *mut c_void,
        },
    ];
    env.register_native_methods(&class, &methods)
        .map_err(|error| describe_jni_error(&mut env, "RegisterNatives", error))?;
    Ok(env.new_global_ref(class)?)
}

 
 
fn load_existing_handler<'a>(env: &mut JNIEnv<'a>, loader: &GlobalRef) -> Option<JClass<'a>> {
    let Ok(name) = env.new_string(HANDLER_CLASS) else {
        return None;
    };
    let result = env.call_method(
        loader.as_obj(),
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[JValue::Object(&name)],
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
                let jstr = JString::from(obj);
                let text = env.get_string(&jstr).ok()?;
                Some(text.to_string_lossy().into_owned())
            });
        if let Some(detail) = detail {
            return anyhow::anyhow!("{what} failed: {detail}");
        }
    }
    anyhow::anyhow!("{what} failed: {error}")
}

 
 
 
fn current_connection(env: &mut JNIEnv) -> anyhow::Result<Option<GlobalRef>> {
    env.with_local_frame(8, |_| -> anyhow::Result<Option<GlobalRef>> {
        let listener = mapping()
            .call_method(
                MinecraftClassType::Minecraft,
                minecraft().jni_ref().as_obj(),
                "getConnection",
                &[],
            )?
            .l()?;
        if listener.is_null() {
            return Ok(None);
        }
        let connection = mapping()
            .call_method(
                MinecraftClassType::ClientPacketListener,
                &listener,
                "getConnection",
                &[],
            )?
            .l()?;
        if connection.is_null() {
            return Ok(None);
        }
        Ok(Some(mapping().new_global_ref(connection)?))
    })
}

 
 
 
fn install_on(env: &mut JNIEnv, connection: &GlobalRef) -> anyhow::Result<()> {
    let handler_class = state()
        .handler_class
        .clone()
        .ok_or_else(|| anyhow::anyhow!("handler class missing"))?;

    env.with_local_frame(16, |env| -> anyhow::Result<()> {
        let pipeline = pipeline_of(env, connection)?;
        let name: JObject = env.new_string(PIPELINE_NAME)?.into();

         
        let existing = env
            .call_method(&pipeline, "get", SIG_PIPELINE_GET, &[JValue::Object(&name)])?
            .l()?;
        if !existing.is_null() {
            return Ok(());
        }

        let class = JClass::from(env.new_local_ref(handler_class.as_obj())?);
        let handler = env.new_object(&class, "()V", &[])?;
        let base: JObject = env.new_string(MC_PACKET_HANDLER)?.into();
        env.call_method(
            &pipeline,
            "addBefore",
            SIG_PIPELINE_ADD_BEFORE,
            &[
                JValue::Object(&base),
                JValue::Object(&name),
                JValue::Object(&handler),
            ],
        )?;
        Ok(())
    })
}

 
 
 
pub(crate) fn set_strict_outbound_delay(delay_ms: i32) -> anyhow::Result<()> {
    let Some(handler) = state().handler_class.clone() else {
        return Ok(());
    };
    let mut env = mapping().get_env()?;
    env.with_local_frame(4, |env| -> anyhow::Result<()> {
        let class = JClass::from(env.new_local_ref(handler.as_obj())?);
         
         
         
        let delay = if delay_ms > 0 {
            delay_ms.clamp(1, 1000)
        } else {
            0
        };
        env.set_static_field(
            &class,
            (&class, "OUTBOUND_DELAY_MS", "I"),
            JValue::Int(delay),
        )?;
        env.set_static_field(
            &class,
            (&class, "STRICT_OUTBOUND", "Z"),
            JValue::Bool(u8::from(delay > 0)),
        )?;
        Ok(())
    })
}

 
 
 
pub fn teardown() {
    if let Err(error) = set_strict_outbound_delay(0) {
        log::warn!("Fake Lag teardown: {error}");
    }
    let (connection, class) = {
        let mut state = state();
         
        state.define_attempts = 0;
        (state.installed_on.take(), state.handler_class.take())
    };

    if let (Some(connection), Ok(mut env)) = (connection, mapping().get_env()) {
        env.with_local_frame(16, |env| -> anyhow::Result<()> {
            let pipeline = pipeline_of(env, &connection)?;
            let name: JObject = env.new_string(PIPELINE_NAME)?.into();
            let existing = env
                .call_method(&pipeline, "get", SIG_PIPELINE_GET, &[JValue::Object(&name)])?
                .l()?;
            if !existing.is_null() {
                env.call_method(
                    &pipeline,
                    "remove",
                    SIG_PIPELINE_REMOVE,
                    &[JValue::Object(&name)],
                )?;
            }
            Ok(())
        })
        .unwrap_or_else(|error| log::debug!("net: pipeline cleanup failed: {error}"));
    }

    if let Some(class) = class {
        if let Ok(mut env) = mapping().get_env() {
            if let Ok(local) = env.new_local_ref(class.as_obj()) {
                let _ = env.unregister_native_methods(JClass::from(local));
            }
        }
    }
    clear_pending_exception();
}

 
 
fn pipeline_of<'a>(env: &mut JNIEnv<'a>, connection: &GlobalRef) -> anyhow::Result<JObject<'a>> {
    let channel_field = mapping().runtime_field_name(MinecraftClassType::Connection, "channel")?;
    let channel = env
        .get_field(connection.as_obj(), &channel_field, SIG_CHANNEL_FIELD)?
        .l()?;
    if channel.is_null() {
        return Err(anyhow::anyhow!("connection has no channel"));
    }
    Ok(env
        .call_method(&channel, "pipeline", SIG_PIPELINE, &[])?
        .l()?)
}

 

 
 
 
fn immediate_interaction(packet: &Packet) -> bool {
    matches!(
        packet,
        Packet::ServerboundInteract(_)
            | Packet::ServerboundUseItem(_)
            | Packet::ServerboundSwing(_)
    )
}

 
fn mark_immediate(env: &mut JNIEnv) {
    let Some(handler) = state().handler_class.clone() else {
        return;
    };
    let Ok(class) = env.new_local_ref(handler.as_obj()) else {
        return;
    };
    if let Err(error) = env.call_static_method(JClass::from(class), "sendImmediately", "()V", &[]) {
        log::debug!("net: immediate interaction mark failed: {error}");
    }
}

 
enum Dispatch {
     
    Forward,
     
    Replace(jobject),
     
     
    Drop,
}

 
unsafe extern "system" fn dark_on_outbound(
    env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    packet: jobject,
) -> jobject {
    catch_unwind(AssertUnwindSafe(|| {
        match unsafe { dispatch(env, packet, false) } {
            Ok(Dispatch::Replace(replacement)) => replacement,
            Ok(Dispatch::Drop) => std::ptr::null_mut(),
            Ok(Dispatch::Forward) | Err(_) => packet,
        }
    }))
    .unwrap_or(packet)
}

 
unsafe extern "system" fn dark_on_inbound(
    env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    packet: jobject,
) -> jobject {
    catch_unwind(AssertUnwindSafe(|| {
        match unsafe { dispatch(env, packet, true) } {
            Ok(Dispatch::Replace(replacement)) => replacement,
            Ok(Dispatch::Drop) => std::ptr::null_mut(),
            Ok(Dispatch::Forward) | Err(_) => packet,
        }
    }))
    .unwrap_or(packet)
}

 
 
 
 
 
unsafe fn dispatch(
    env: *mut jni::sys::JNIEnv,
    packet: jobject,
    inbound: bool,
) -> anyhow::Result<Dispatch> {
    if packet.is_null() {
        return Ok(Dispatch::Forward);
    }
    let mut env = unsafe { JNIEnv::from_raw(env)? };
    let packet_obj = unsafe { JObject::from_raw(packet) };
     
     
    if !inbound && INJECTING.load(Ordering::Relaxed) != 0 {
        if Packet::from_outbound(&mut env, &packet_obj)?
            .as_ref()
            .is_some_and(immediate_interaction)
        {
            mark_immediate(&mut env);
        }
        return Ok(Dispatch::Forward);
    }

    let built = if inbound {
        Packet::from_inbound(&mut env, &packet_obj)?
    } else {
        Packet::from_outbound(&mut env, &packet_obj)?
    };
    let Some(mut wrapped) = built else {
        return Ok(Dispatch::Forward);
    };
    if !inbound && immediate_interaction(&wrapped) {
        mark_immediate(&mut env);
    }
     
    if matches!(wrapped, Packet::ClientboundBlockUpdate(_)) {
        crate::graphic::esp::note_block_update(&packet_obj);
        if crate::module::combat::crystal_macro::awaiting_server_placement_confirmation() {
            if let Err(error) =
                crate::module::combat::crystal_macro::note_server_block_update(&packet_obj)
            {
                log::debug!("Crystal Macro: could not read server block update: {error}");
            }
        }
    }

    let original = wrapped.clone();
    match crate::state::client().modules.handle_packet(&mut wrapped) {
        PacketAction::Cancel => Ok(Dispatch::Drop),
        PacketAction::Forward if wrapped != original => {
            Ok(Dispatch::Replace(wrapped.to_java(&mut env)?))
        }
        PacketAction::Forward => Ok(Dispatch::Forward),
    }
}
