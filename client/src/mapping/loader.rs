 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::JNIEnv;

 
 
const MINECRAFT_CLASS: &str = "net.minecraft.client.Minecraft";

 
 
const GET_INSTANCE_SIG: &str = "()Lnet/minecraft/client/Minecraft;";

 
 
 
const INTERMEDIARY_MC_CLASS: &str = "net.minecraft.class_310";

 
 
const INTERMEDIARY_GET_INSTANCE_SIG: &str = "()Lnet/minecraft/class_310;";

 
const RENDER_THREAD: &str = "Render thread";

 
enum LoaderProbe {
     
     
    Live(GlobalRef),
     
     
     
    RenderThread(GlobalRef),
     
    Unrelated,
}

 
 
 
 
 
pub fn discover_game_loader(env: &mut JNIEnv) -> Option<GlobalRef> {
    scan_threads(env, MINECRAFT_CLASS, GET_INSTANCE_SIG)
}

 
 
 
 
 
pub fn discover_intermediary_loader(env: &mut JNIEnv) -> Option<GlobalRef> {
    scan_threads(env, INTERMEDIARY_MC_CLASS, INTERMEDIARY_GET_INSTANCE_SIG)
}

 
 
fn scan_threads(env: &mut JNIEnv, mc_class: &str, get_instance_sig: &str) -> Option<GlobalRef> {
    match scan_threads_all(env, mc_class, get_instance_sig) {
        Ok(loader) => loader,
        Err(_) => {
            let _ = env.exception_clear();
            None
        }
    }
}

 
 
fn scan_threads_all(
    env: &mut JNIEnv,
    mc_class: &str,
    get_instance_sig: &str,
) -> anyhow::Result<Option<GlobalRef>> {
    let thread_class = env.find_class("java/lang/Thread")?;
    let traces = env
        .call_static_method(&thread_class, "getAllStackTraces", "()Ljava/util/Map;", &[])?
        .l()?;
    let threads = env
        .call_method(&traces, "keySet", "()Ljava/util/Set;", &[])?
        .l()?;
    let iter = env
        .call_method(&threads, "iterator", "()Ljava/util/Iterator;", &[])?
        .l()?;

    let mut render_thread_loader: Option<GlobalRef> = None;

    while env.call_method(&iter, "hasNext", "()Z", &[])?.z()? {
         
         
        let probe = env.with_local_frame(32, |env| -> anyhow::Result<LoaderProbe> {
            let thread = env
                .call_method(&iter, "next", "()Ljava/lang/Object;", &[])?
                .l()?;
            let loader = env
                .call_method(
                    &thread,
                    "getContextClassLoader",
                    "()Ljava/lang/ClassLoader;",
                    &[],
                )?
                .l()?;
            if loader.is_null() {
                return Ok(LoaderProbe::Unrelated);
            }
            probe_loader(env, &thread, &loader, mc_class, get_instance_sig)
        })?;

        match probe {
            LoaderProbe::Live(loader) => return Ok(Some(loader)),
            LoaderProbe::RenderThread(loader) => render_thread_loader = Some(loader),
            LoaderProbe::Unrelated => {}
        }
    }

    Ok(render_thread_loader)
}

 
 
fn probe_loader(
    env: &mut JNIEnv,
    thread: &JObject,
    loader: &JObject,
    mc_class: &str,
    get_instance_sig: &str,
) -> anyhow::Result<LoaderProbe> {
    let class = match load_class(env, loader, mc_class) {
        Some(class) => class,
        None => return Ok(LoaderProbe::Unrelated),
    };

    let instance = match env.call_static_method(&class, "getInstance", get_instance_sig, &[]) {
        Ok(value) => value.l().unwrap_or_else(|_| JObject::null()),
        Err(_) => {
            let _ = env.exception_clear();
            JObject::null()
        }
    };

    if !instance.is_null() {
        return Ok(LoaderProbe::Live(env.new_global_ref(loader)?));
    }
    if thread_name(env, thread)?.as_deref() == Some(RENDER_THREAD) {
        return Ok(LoaderProbe::RenderThread(env.new_global_ref(loader)?));
    }
    Ok(LoaderProbe::Unrelated)
}

 
 
fn load_class<'a>(env: &mut JNIEnv<'a>, loader: &JObject, binary_name: &str) -> Option<JClass<'a>> {
    let name: JObject = env.new_string(binary_name).ok()?.into();
    let result = env.call_method(
        loader,
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

 
fn thread_name(env: &mut JNIEnv, thread: &JObject) -> anyhow::Result<Option<String>> {
    let name = env
        .call_method(thread, "getName", "()Ljava/lang/String;", &[])?
        .l()?;
    Ok(env
        .get_string(&JString::from(name))?
        .to_str()
        .ok()
        .map(str::to_owned))
}
