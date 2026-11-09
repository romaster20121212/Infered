 
 
 
 
 
 
 

use crate::mapping::class::{Method, MinecraftClass};
use crate::mapping::Mapping;
use jni::objects::{JObject, JObjectArray, JString};
use jni::JNIEnv;
use std::collections::HashMap;

 
 
pub fn reflect_class(mapping: &Mapping, class_name: &str) -> anyhow::Result<MinecraftClass> {
    let mut env = mapping.get_env()?;

    let jclass: JObject = mapping.resolve_class(&mut env, class_name)?.into();

    let mut methods: HashMap<String, Vec<Method>> = HashMap::new();

     
     
    for accessor in ["getMethods", "getDeclaredMethods"] {
        if let Err(error) = collect_methods(&mut env, &jclass, accessor, &mut methods) {
             
             
             
            let _ = env.exception_clear();
            log::debug!("reflect: {accessor} on {class_name} did not fully enumerate: {error}");
        }
    }

    Ok(MinecraftClass::from_reflection(
        class_name.to_owned(),
        methods,
    ))
}

 
 
fn collect_methods(
    env: &mut JNIEnv,
    jclass: &JObject,
    accessor: &str,
    out: &mut HashMap<String, Vec<Method>>,
) -> anyhow::Result<()> {
    let array = env
        .call_method(jclass, accessor, "()[Ljava/lang/reflect/Method;", &[])?
        .l()?;
    let array = JObjectArray::from(array);
    let count = env.get_array_length(&array)?;

    for index in 0..count {
         
         
        let described = env.with_local_frame(64, |env| -> anyhow::Result<(String, String)> {
            let method = env.get_object_array_element(&array, index)?;
            describe_method(env, &method)
        });
        let (name, signature) = match described {
            Ok(method) => method,
             
             
             
            Err(_) => {
                let _ = env.exception_clear();
                continue;
            }
        };

        let overloads = out.entry(name.clone()).or_default();
        if !overloads.iter().any(|m| m.signature == signature) {
            overloads.push(Method {
                name,
                signature,
                id: std::sync::OnceLock::new(),
            });
        }
    }
    Ok(())
}

 
fn describe_method(env: &mut JNIEnv, method: &JObject) -> anyhow::Result<(String, String)> {
    let name = call_string(env, method, "getName")?;

    let params = env
        .call_method(method, "getParameterTypes", "()[Ljava/lang/Class;", &[])?
        .l()?;
    let params = JObjectArray::from(params);
    let param_count = env.get_array_length(&params)?;

    let mut signature = String::from("(");
    for index in 0..param_count {
        let param = env.get_object_array_element(&params, index)?;
        signature.push_str(&type_descriptor(env, &param)?);
    }
    signature.push(')');

    let return_type = env
        .call_method(method, "getReturnType", "()Ljava/lang/Class;", &[])?
        .l()?;
    signature.push_str(&type_descriptor(env, &return_type)?);

    Ok((name, signature))
}

 
fn type_descriptor(env: &mut JNIEnv, class: &JObject) -> anyhow::Result<String> {
    let name = call_string(env, class, "getName")?;
    Ok(match name.as_str() {
        "boolean" => "Z".to_owned(),
        "byte" => "B".to_owned(),
        "char" => "C".to_owned(),
        "short" => "S".to_owned(),
        "int" => "I".to_owned(),
        "long" => "J".to_owned(),
        "float" => "F".to_owned(),
        "double" => "D".to_owned(),
        "void" => "V".to_owned(),
         
        array if array.starts_with('[') => array.replace('.', "/"),
        object => format!("L{};", object.replace('.', "/")),
    })
}

 
fn call_string(env: &mut JNIEnv, obj: &JObject, method: &str) -> anyhow::Result<String> {
    let value = env
        .call_method(obj, method, "()Ljava/lang/String;", &[])?
        .l()?;
    Ok(env.get_string(&JString::from(value))?.to_str()?.to_owned())
}

 
 
 
pub fn class_runtime_name(env: &mut JNIEnv, class: &JObject) -> anyhow::Result<String> {
    call_string(env, class, "getName")
}
