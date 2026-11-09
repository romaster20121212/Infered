use crate::mapping::class::{Method, MethodHandle, MinecraftClass};
pub use crate::mapping::class_type::MinecraftClassType;
use crate::mapping::minecraft_version::MinecraftVersion;
pub use crate::mapping::object::MappedObject;
use dashmap::DashMap;
use jni::objects::{
    GlobalRef, JClass, JMethodID, JObject, JObjectArray, JString, JValue, JValueOwned,
};
use jni::signature::{Primitive, ReturnType};
use jni::sys::{jsize, jvalue, JNI_GetCreatedJavaVMs, JNI_OK};
use jni::{JNIEnv, JavaVM};
use log::info;
pub use mapping_derive::MappedObject;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

pub mod block;
pub mod block_entity;
pub mod chunk;
pub mod class;
pub mod class_type;
pub mod client;
pub mod component;
pub mod entity;
pub mod inventory;
pub mod java;
mod loader;
pub mod math;
mod minecraft_version;
pub mod object;
pub mod profile;
mod reflect;
mod renames;

#[cfg(test)]
mod jvm_test;

 
#[derive(Debug, Deserialize)]
struct MappingFile {
    version: MinecraftVersion,
    classes: HashMap<String, MinecraftClass>,
}

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
     
    Obfuscated,
     
     
    Reflected,
     
     
     
    Intermediary,
}

 
 
#[derive(Debug)]
pub struct Mapping {
     
    jvm: JavaVM,
    mode: Mode,
    version: MinecraftVersion,
     
     
     
    classes: DashMap<String, Arc<MinecraftClass>>,
     
     
     
     
     
     
     
    class_loader: RwLock<Option<GlobalRef>>,
     
     
    class_handles: DashMap<String, Option<GlobalRef>>,
     
     
     
     
     
     
    class_chain: DashMap<String, Arc<Vec<Arc<MinecraftClass>>>>,
}

#[allow(dead_code)]
pub enum FieldType {
    Boolean,
    Byte,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
    String,
    LongArray,
    Object(MinecraftClassType),
}

impl FieldType {
    pub fn get_signature(&self) -> anyhow::Result<String> {
        Ok(match self {
            FieldType::Boolean => String::from("Z"),
            FieldType::Byte => String::from("B"),
            FieldType::Char => String::from("C"),
            FieldType::Short => String::from("S"),
            FieldType::Int => String::from("I"),
            FieldType::Long => String::from("J"),
            FieldType::Float => String::from("F"),
            FieldType::Double => String::from("D"),
            FieldType::String => String::from("Ljava/lang/String;"),
            FieldType::LongArray => String::from("[J"),
            FieldType::Object(class_type) => {
                format!(
                    "L{};",
                    crate::state::mapping().runtime_class_name(*class_type)?
                )
            }
        })
    }
}

 
 
 
 
 
fn probe_unobfuscated(env: &mut JNIEnv) -> bool {
    let found = env.find_class("net/minecraft/client/Minecraft").is_ok();
    if !found {
        let _ = env.exception_clear();
    }
    found
}

 
fn acquire_jvm() -> anyhow::Result<JavaVM> {
    let mut raw: *mut jni::sys::JavaVM = std::ptr::null_mut();
    let mut count: jsize = 0;

     
    unsafe {
        if JNI_GetCreatedJavaVMs(&mut raw, 1, &mut count) != JNI_OK || count == 0 {
            return Err(anyhow::anyhow!("no JVM found in this process"));
        }
        Ok(JavaVM::from_raw(raw)?)
    }
}

 
 
 
fn load_bundled_classes(
    mappings_json: &str,
) -> anyhow::Result<(MinecraftVersion, DashMap<String, Arc<MinecraftClass>>)> {
    let mut file: MappingFile = serde_json::from_str(mappings_json)?;

     
     
    let java_classes: HashMap<String, MinecraftClass> =
        serde_json::from_str(include_str!("../../../java_mappings.json"))?;
    file.classes.extend(java_classes);

    let classes = file
        .classes
        .into_iter()
        .map(|(name, class)| (name, Arc::new(class)))
        .collect();
    Ok((file.version, classes))
}

#[allow(dead_code)]
impl Mapping {
    pub fn new() -> anyhow::Result<Mapping> {
        let jvm = acquire_jvm()?;
        let mut env = jvm.attach_current_thread_as_daemon()?;

         
         
         
         
        let game_loader = loader::discover_game_loader(&mut env);

         
         
         
         
        let reflected = game_loader.is_some() || probe_unobfuscated(&mut env);

        if reflected {
            match game_loader {
                Some(_) => info!(
                    "Modded/unobfuscated Minecraft detected — routing class \
                     resolution through the game class loader"
                ),
                None => info!("Unobfuscated Minecraft detected — using runtime reflection mapping"),
            }
            return Ok(Mapping {
                jvm,
                mode: Mode::Reflected,
                version: MinecraftVersion::LATEST,
                classes: DashMap::new(),
                class_loader: RwLock::new(game_loader),
                class_handles: DashMap::new(),
                class_chain: DashMap::new(),
            });
        }

         
         
         
         
        if let Some(intermediary_loader) = loader::discover_intermediary_loader(&mut env) {
            info!("Fabric intermediary Minecraft detected — using bundled intermediary mappings");
            let (version, classes) =
                load_bundled_classes(include_str!("../../../mappings_intermediary.json"))?;
            return Ok(Mapping {
                jvm,
                mode: Mode::Intermediary,
                version,
                classes,
                class_loader: RwLock::new(Some(intermediary_loader)),
                class_handles: DashMap::new(),
                class_chain: DashMap::new(),
            });
        }

        info!("Obfuscated Minecraft detected — using bundled Mojmap mappings");
        let (version, classes) = load_bundled_classes(include_str!("../../../mappings.json"))?;

        Ok(Mapping {
            jvm,
            mode: Mode::Obfuscated,
            version,
            classes,
            class_loader: RwLock::new(None),
            class_handles: DashMap::new(),
            class_chain: DashMap::new(),
        })
    }

     
     
    pub fn get_env(&self) -> anyhow::Result<JNIEnv<'_>> {
        Ok(self.jvm.attach_current_thread_as_daemon()?)
    }

     
     
     
     
    pub fn in_frame<T>(&self, f: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<T> {
        let mut env = self.get_env()?;
        env.with_local_frame(16, |_| f())
    }

     
     
     
     
    pub fn teardown(&self) {
        self.classes.clear();
        self.class_handles.clear();
        self.class_chain.clear();
        if let Ok(mut loader) = self.class_loader.write() {
            *loader = None;
        }
    }

    pub fn get_version(&self) -> MinecraftVersion {
        self.version
    }

     
     
    pub fn get_class(&self, name: &str) -> anyhow::Result<Arc<MinecraftClass>> {
        let name = renames::resolve_class(name, self.version);
        if let Some(class) = self.classes.get(name) {
            return Ok(Arc::clone(class.value()));
        }

        match self.mode {
            Mode::Obfuscated | Mode::Intermediary => {
                Err(anyhow::anyhow!("{} java class not found", name))
            }
            Mode::Reflected => {
                let class = Arc::new(reflect::reflect_class(self, name)?);
                self.classes.insert(name.to_owned(), Arc::clone(&class));
                Ok(class)
            }
        }
    }

     
    fn loader(&self) -> Option<GlobalRef> {
        self.class_loader.read().ok().and_then(|slot| slot.clone())
    }

     
     
    pub fn game_class_loader(&self) -> Option<GlobalRef> {
        self.loader()
    }

     
     
     
     
     
     
     
    fn class_chain(&self, name: &str) -> anyhow::Result<Arc<Vec<Arc<MinecraftClass>>>> {
        let key = renames::resolve_class(name, self.version).to_owned();
        if self.mode == Mode::Reflected {
            return Ok(Arc::new(vec![self.get_class(&key)?]));
        }
        if let Some(chain) = self.class_chain.get(&key) {
            return Ok(Arc::clone(chain.value()));
        }
        let chain = self.build_class_chain(&key)?;
        self.class_chain.insert(key, Arc::clone(&chain));
        Ok(chain)
    }

     
     
     
    fn build_class_chain(&self, key: &str) -> anyhow::Result<Arc<Vec<Arc<MinecraftClass>>>> {
        let mut env = self.get_env()?;
        let mut chain = Vec::new();
        let mut pending = vec![key.to_owned()];
        let mut visited: HashSet<String> = HashSet::new();

        while let Some(current) = pending.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            let class = match self.classes.get(&current) {
                Some(entry) => Arc::clone(entry.value()),
                 
                 
                None => continue,
            };

            let jclass = match self.resolve_class(&mut env, &class.name) {
                Ok(jclass) => jclass,
                 
                Err(_) => continue,
            };
            chain.push(class);

            for ancestor in self.ancestor_names(&mut env, &jclass) {
                pending.push(ancestor);
            }
        }

        if chain.is_empty() {
            return Err(anyhow::anyhow!(
                "class {} has no JSON entry to resolve its hierarchy from",
                key
            ));
        }
        Ok(Arc::new(chain))
    }

     
     
     
    fn ancestor_names(&self, env: &mut JNIEnv, jclass: &JClass) -> Vec<String> {
        let mut names = Vec::new();

        let super_class = env
            .call_method(jclass, "getSuperclass", "()Ljava/lang/Class;", &[])
            .and_then(|value| value.l());
        match super_class {
            Ok(super_handle) if !super_handle.is_null() => {
                if let Some(name) = self.class_name_of(env, &super_handle) {
                    names.push(name);
                }
            }
            _ => {
                let _ = env.exception_clear();
            }
        }

        if let Ok(array) = env
            .call_method(jclass, "getInterfaces", "()[Ljava/lang/Class;", &[])
            .and_then(|value| value.l())
        {
            let array = JObjectArray::from(array);
            if let Ok(count) = env.get_array_length(&array) {
                for index in 0..count {
                    match env.get_object_array_element(&array, index) {
                        Ok(interface_handle) => {
                            if let Some(name) = self.class_name_of(env, &interface_handle) {
                                names.push(name);
                            }
                        }
                        Err(_) => {
                            let _ = env.exception_clear();
                        }
                    }
                }
            }
        } else {
            let _ = env.exception_clear();
        }

        names
    }

     
     
    fn mapped_name_of_runtime(&self, runtime_name: &str) -> Option<String> {
        self.classes
            .iter()
            .find(|entry| entry.value().name == runtime_name)
            .map(|entry| entry.key().clone())
    }

     
     
    fn class_name_of(&self, env: &mut JNIEnv, class: &JObject) -> Option<String> {
        let runtime_name = reflect::class_runtime_name(env, class).ok()?;
        let jni_name = runtime_name.replace('.', "/");
        self.mapped_name_of_runtime(&jni_name)
    }

     
     
     
     
     
     
     
     
     
     
     
    pub(crate) fn resolve_class<'a>(
        &self,
        env: &mut JNIEnv<'a>,
        jni_name: &str,
    ) -> anyhow::Result<JClass<'a>> {
        if let Some(cached) = self.class_handles.get(jni_name) {
            return match cached.value() {
                Some(handle) => Ok(JClass::from(env.new_local_ref(handle.as_obj())?)),
                None => Err(anyhow::anyhow!("Class {} not present at runtime", jni_name)),
            };
        }

        let resolved = self.lookup_class(env, jni_name);
        let handle = match &resolved {
            Ok(jclass) => env.new_global_ref(jclass).ok(),
            Err(_) => None,
        };
        if handle.is_none() {
            log::warn!(
                "Mapping: class '{}' could not be resolved at runtime",
                jni_name
            );
        }
        self.class_handles.insert(jni_name.to_owned(), handle);
        resolved
    }

     
     
    fn lookup_class<'a>(&self, env: &mut JNIEnv<'a>, jni_name: &str) -> anyhow::Result<JClass<'a>> {
        if let Some(loader) = self.loader() {
            let binary_name = jni_name.replace('/', ".");
            let name: JObject = env.new_string(binary_name)?.into();
            return match env.call_method(
                loader.as_obj(),
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[JValue::Object(&name)],
            ) {
                Ok(value) => Ok(JClass::from(value.l()?)),
                Err(_) => {
                    let _ = env.exception_clear();
                    Err(anyhow::anyhow!("Class {} not found at runtime", jni_name))
                }
            };
        }

         
         
        match env.find_class(jni_name) {
            Ok(jclass) => {
                self.capture_class_loader(env, &jclass);
                Ok(jclass)
            }
            Err(_) => {
                let _ = env.exception_clear();
                Err(anyhow::anyhow!("Class {} not found at runtime", jni_name))
            }
        }
    }

     
    fn capture_class_loader(&self, env: &mut JNIEnv, jclass: &JClass) {
        if self.loader().is_some() {
            return;
        }
        let loader = env.call_method(jclass, "getClassLoader", "()Ljava/lang/ClassLoader;", &[]);
        match loader.and_then(|value| value.l()) {
            Ok(obj) if !obj.is_null() => {
                if let (Ok(global), Ok(mut slot)) =
                    (env.new_global_ref(obj), self.class_loader.write())
                {
                    *slot = Some(global);
                }
            }
            _ => {
                let _ = env.exception_clear();
            }
        }
    }

     
    pub(crate) fn runtime_class_name(
        &self,
        class_type: MinecraftClassType,
    ) -> anyhow::Result<String> {
        match self.mode {
            Mode::Reflected => Ok(class_type.get_name().to_owned()),
            Mode::Obfuscated | Mode::Intermediary => {
                Ok(self.get_class(class_type.get_name())?.name.clone())
            }
        }
    }

     
    pub(crate) fn runtime_field_name(
        &self,
        class_type: MinecraftClassType,
        field: &str,
    ) -> anyhow::Result<String> {
        let field = renames::resolve_member(class_type, field, self.version);
        match self.mode {
            Mode::Reflected => Ok(field.to_owned()),
            Mode::Obfuscated | Mode::Intermediary => {
                for class in self.class_chain(class_type.get_name())?.iter() {
                    if let Ok(entry) = class.get_field(field) {
                        return Ok(entry.name.clone());
                    }
                }
                Err(anyhow::anyhow!(
                    "field {} not found on {} or any superclass/interface",
                    field,
                    class_type.get_name()
                ))
            }
        }
    }

     
     
     
     
     
    fn find_method<'a>(
        &self,
        chain: &'a [Arc<MinecraftClass>],
        method_name: &str,
        args: &[JValue],
    ) -> anyhow::Result<&'a Method> {
        for class in chain {
            if let Ok(method) = class.get_method_by_args(method_name, args) {
                 
                 
                 
                 
                 
                 
                if signature_arg_count(&method.signature) == args.len() {
                    return Ok(method);
                }
            }
        }
        Err(anyhow::anyhow!(
            "method {} not found on {} or any superclass/interface",
            method_name,
            chain
                .first()
                .map(|c| c.name.as_str())
                .unwrap_or("<unknown>")
        ))
    }

     
     
    fn find_class_by_obfuscated_name(&self, obfuscated_name: &str) -> Option<String> {
        self.classes
            .iter()
            .find(|entry| entry.value().name == obfuscated_name)
            .map(|entry| entry.key().clone())
    }

    fn translate_type_descriptor(&self, descriptor: &mut &str) -> String {
        let mut array_brackets = String::new();
        while descriptor.starts_with('[') {
            array_brackets.push_str("[]");
            *descriptor = &descriptor[1..];
        }

        let type_name = if let Some(stripped) = descriptor.strip_prefix('L') {
            if let Some(end_index) = stripped.find(';') {
                let obfuscated_name = &stripped[..end_index];
                let deobfuscated_name = self
                    .find_class_by_obfuscated_name(obfuscated_name)
                    .unwrap_or_else(|| obfuscated_name.to_owned());

                *descriptor = &stripped[end_index + 1..];
                deobfuscated_name
            } else {
                let rest = descriptor.to_string();
                *descriptor = "";
                rest
            }
        } else {
            let (primitive, rest) = descriptor.split_at(1);
            *descriptor = rest;
            match primitive {
                "Z" => "boolean".to_string(),
                "B" => "byte".to_string(),
                "C" => "char".to_string(),
                "S" => "short".to_string(),
                "I" => "int".to_string(),
                "J" => "long".to_string(),
                "F" => "float".to_string(),
                "D" => "double".to_string(),
                "V" => "void".to_string(),
                _ => primitive.to_string(),
            }
        };

        format!("{}{}", type_name, array_brackets)
    }

    fn translate_signature(&self, signature: &str) -> String {
        if let (Some(params_start), Some(params_end)) = (signature.find('('), signature.find(')')) {
            let mut params_str = &signature[params_start + 1..params_end];
            let mut return_type_str = &signature[params_end + 1..];

            let mut translated_params = Vec::new();
            while !params_str.is_empty() {
                translated_params.push(self.translate_type_descriptor(&mut params_str));
            }

            let translated_return = self.translate_type_descriptor(&mut return_type_str);

            format!(
                "({}) -> {}",
                translated_params.join(", "),
                translated_return
            )
        } else {
            signature.to_string()
        }
    }

    pub fn call_static_method(
        &'_ self,
        class_type: MinecraftClassType,
        method_name: &str,
        args: &[JValue],
    ) -> anyhow::Result<JValueOwned<'_>> {
        let mut env = self.get_env()?;

        let class_name = class_type.get_name();
        let chain = self.class_chain(class_name)?;
        let method_name = renames::resolve_member(class_type, method_name, self.version);
        let method = self.find_method(&chain, method_name, args)?;
        let jclass = self.resolve_class(&mut env, &chain[0].name)?;
        match env.call_static_method(jclass, &method.name, &method.signature, args) {
            Ok(value) => Ok(value),
            Err(_) => {
                let _ = env.exception_clear();
                let translated_signature = self.translate_signature(&method.signature);
                Err(anyhow::anyhow!(
                    "Error calling static method {} ({}) in class {} ({}) with signature {} ({})",
                    method_name,
                    method.name,
                    class_name,
                    chain[0].name,
                    translated_signature,
                    method.signature
                ))
            }
        }
    }

    pub fn call_method(
        &'_ self,
        class_type: MinecraftClassType,
        instance: &JObject,
        method_name: &str,
        args: &[JValue],
    ) -> anyhow::Result<JValueOwned<'_>> {
        let mut env = self.get_env()?;

        let class_name = class_type.get_name();
        let chain = self.class_chain(class_name)?;
        let method_name = renames::resolve_member(class_type, method_name, self.version);
        let method = self.find_method(&chain, method_name, args)?;

         
         
         
        if signature_arg_count(&method.signature) != args.len() {
            return Err(anyhow::anyhow!(
                "Argument count mismatch calling {} ({}) — signature {}",
                method_name,
                method.name,
                method.signature
            ));
        }

        let method_id = self.resolve_method_id(&mut env, class_type, method)?;
        let return_type = parse_return_type(&method.signature);
        let jni_args: Vec<jvalue> = args.iter().map(|arg| arg.as_jni()).collect();

        match unsafe { env.call_method_unchecked(instance, method_id, return_type, &jni_args) } {
            Ok(value) => Ok(value),
            Err(_) => {
                let _ = env.exception_clear();
                let translated_signature = self.translate_signature(&method.signature);
                Err(anyhow::anyhow!(
                    "Error calling method {} ({}) in class {} ({}) with signature {} ({})",
                    method_name,
                    method.name,
                    class_name,
                    chain[0].name,
                    translated_signature,
                    method.signature
                ))
            }
        }
    }

     
     
     
     
     
     
    pub fn call_method_batch(
        &'_ self,
        class_type: MinecraftClassType,
        instance: &JObject,
        method_name: &str,
        args_list: &[Vec<JValue>],
    ) -> anyhow::Result<()> {
        let mut env = self.get_env()?;

        let class_name = class_type.get_name();
        let chain = self.class_chain(class_name)?;
        let method_name = renames::resolve_member(class_type, method_name, self.version);
        let method = if args_list.is_empty() {
            return Ok(());
        } else {
            self.find_method(&chain, method_name, &args_list[0])?
        };

         
         
        if signature_arg_count(&method.signature) != args_list[0].len() {
            return Err(anyhow::anyhow!(
                "Argument count mismatch calling {} ({}) — signature {}",
                method_name,
                method.name,
                method.signature
            ));
        }

        let method_id = self.resolve_method_id(&mut env, class_type, method)?;
        let return_type = parse_return_type(&method.signature);
        let translated_signature = self.translate_signature(&method.signature);

         
         
        let capacity = (64 + args_list.len() * 2).min(16384) as i32;
        if env.push_local_frame(capacity).is_err() {
            let _ = env.exception_clear();
            return Err(anyhow::anyhow!(
                "Failed to push local frame for {} ({}) in class {} ({}) with signature {} ({})",
                method_name,
                method.name,
                class_name,
                chain[0].name,
                translated_signature,
                method.signature
            ));
        }

        let mut failed = false;
        for args in args_list {
            let jni_args: Vec<jvalue> = args.iter().map(|arg| arg.as_jni()).collect();
             
             
             
             
             
            if unsafe {
                env.call_method_unchecked(instance, method_id, return_type.clone(), &jni_args)
            }
            .is_err()
            {
                let _ = env.exception_clear();
                failed = true;
                break;
            }
        }

         
         
         
         
        let _ = unsafe { env.pop_local_frame(&JObject::null()) };

        if failed {
            Err(anyhow::anyhow!(
                "Error calling method {} ({}) in class {} ({}) with signature {} ({})",
                method_name,
                method.name,
                class_name,
                chain[0].name,
                translated_signature,
                method.signature
            ))
        } else {
            Ok(())
        }
    }

     
     
     
     
     
     
    pub(crate) fn resolve_named_method_id(
        &self,
        env: &mut JNIEnv,
        class_type: MinecraftClassType,
        method_name: &str,
    ) -> anyhow::Result<jni::sys::jmethodID> {
        let chain = self.class_chain(class_type.get_name())?;
        let method = chain
            .iter()
            .find_map(|class| class.get_method(method_name).ok())
            .ok_or_else(|| anyhow::anyhow!("method {} not found on {}", method_name, class_type))?;
        Ok(self.resolve_method_id(env, class_type, method)?.into_raw())
    }

    fn resolve_method_id(
        &self,
        env: &mut JNIEnv,
        class_type: MinecraftClassType,
        method: &Method,
    ) -> anyhow::Result<JMethodID> {
        if let Some(handle) = method.id.get() {
            return Ok(unsafe { JMethodID::from_raw(handle.0) });
        }

        let class_name = self.runtime_class_name(class_type)?;
        let jclass = self.resolve_class(env, &class_name)?;
        let id = match env.get_method_id(&jclass, &method.name, &method.signature) {
            Ok(id) => id,
            Err(_) => {
                let _ = env.exception_clear();
                return Err(anyhow::anyhow!(
                    "Method {} {} not found on class {}",
                    method.name,
                    method.signature,
                    class_name
                ));
            }
        };
        let _ = method.id.set(MethodHandle(id.into_raw()));
        Ok(id)
    }

    pub fn get_static_field(
        &'_ self,
        class_type: MinecraftClassType,
        field_name: &str,
        field_type: FieldType,
    ) -> anyhow::Result<JValueOwned<'_>> {
        let mut env = self.get_env()?;

        let class_name = self.runtime_class_name(class_type)?;
        let jclass = self.resolve_class(&mut env, &class_name)?;
        let runtime_field = self.runtime_field_name(class_type, field_name)?;
        match env.get_static_field(jclass, &runtime_field, field_type.get_signature()?) {
            Ok(value) => Ok(value),
            Err(_) => {
                let _ = env.exception_clear();
                Err(anyhow::anyhow!(
                    "Error getting static field {} ({}) from class {}",
                    field_name,
                    runtime_field,
                    class_type.get_name()
                ))
            }
        }
    }

    pub fn get_field(
        &'_ self,
        class_type: MinecraftClassType,
        instance: &JObject,
        field_name: &str,
        field_type: FieldType,
    ) -> anyhow::Result<JValueOwned<'_>> {
        let mut env = self.get_env()?;

        let runtime_field = self.runtime_field_name(class_type, field_name)?;
        match env.get_field(instance, &runtime_field, field_type.get_signature()?) {
            Ok(value) => Ok(value),
            Err(_) => {
                let _ = env.exception_clear();
                Err(anyhow::anyhow!(
                    "Error getting field {} ({}) from class {}",
                    field_name,
                    runtime_field,
                    class_type.get_name()
                ))
            }
        }
    }

    pub fn set_field(
        &self,
        class_type: MinecraftClassType,
        instance: &JObject,
        field_name: &str,
        field_type: FieldType,
        value: JValue,
    ) -> anyhow::Result<()> {
        let mut env = self.get_env()?;

        let runtime_field = self.runtime_field_name(class_type, field_name)?;
        match env.set_field(instance, &runtime_field, field_type.get_signature()?, value) {
            Ok(_) => Ok(()),
            Err(_) => {
                let _ = env.exception_clear();
                Err(anyhow::anyhow!(
                    "Error setting field {} ({}) in class {}",
                    field_name,
                    runtime_field,
                    class_type.get_name()
                ))
            }
        }
    }

    pub fn new_global_ref(&self, obj: JObject) -> anyhow::Result<GlobalRef> {
        let env = self.get_env()?;
        Ok(env.new_global_ref(obj)?)
    }

    pub fn get_string(&self, obj: JObject) -> anyhow::Result<String> {
        let env = self.get_env()?;
        let jstring = JString::from(obj);
        unsafe {
            let value = env
                .get_string_unchecked(jstring.as_ref())?
                .to_str()?
                .to_string();
            Ok(value)
        }
    }

    pub fn is_instance_of(
        &self,
        class_type: MinecraftClassType,
        instance: &JObject,
    ) -> anyhow::Result<bool> {
        let mut env = self.get_env()?;
        let class_name = self.runtime_class_name(class_type)?;
        let jclass = self.resolve_class(&mut env, &class_name)?;
        Ok(env.is_instance_of(instance, &jclass)?)
    }
}

 
 
fn signature_arg_count(signature: &str) -> usize {
    let params = match (signature.find('('), signature.find(')')) {
        (Some(open), Some(close)) if open < close => &signature[open + 1..close],
         
        _ => return usize::MAX,
    };

    let mut count = 0;
    let mut chars = params.chars();
    while let Some(ch) = chars.next() {
        match ch {
             
            '[' => continue,
             
            'L' => {
                for inner in chars.by_ref() {
                    if inner == ';' {
                        break;
                    }
                }
                count += 1;
            }
             
            _ => count += 1,
        }
    }
    count
}

 
fn parse_return_type(signature: &str) -> ReturnType {
    let return_descriptor = signature.rsplit(')').next().unwrap_or("V");
    match return_descriptor.chars().next() {
        Some('Z') => ReturnType::Primitive(Primitive::Boolean),
        Some('B') => ReturnType::Primitive(Primitive::Byte),
        Some('C') => ReturnType::Primitive(Primitive::Char),
        Some('S') => ReturnType::Primitive(Primitive::Short),
        Some('I') => ReturnType::Primitive(Primitive::Int),
        Some('J') => ReturnType::Primitive(Primitive::Long),
        Some('F') => ReturnType::Primitive(Primitive::Float),
        Some('D') => ReturnType::Primitive(Primitive::Double),
        Some('[') => ReturnType::Array,
        Some('L') => ReturnType::Object,
        _ => ReturnType::Primitive(Primitive::Void),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arg_count_counts_primitives_and_objects() {
        assert_eq!(signature_arg_count("()V"), 0);
        assert_eq!(signature_arg_count("(I)V"), 1);
        assert_eq!(signature_arg_count("(ILjava/lang/String;F)V"), 3);
        assert_eq!(signature_arg_count("([ILjava/lang/String;)V"), 2);
    }

    #[test]
    fn arg_count_of_an_unparseable_signature_never_matches() {
        assert_eq!(signature_arg_count("garbage"), usize::MAX);
    }

    #[test]
    fn return_type_is_parsed_from_the_descriptor() {
        assert!(matches!(
            parse_return_type("()V"),
            ReturnType::Primitive(Primitive::Void)
        ));
        assert!(matches!(
            parse_return_type("(I)I"),
            ReturnType::Primitive(Primitive::Int)
        ));
        assert!(matches!(
            parse_return_type("()Z"),
            ReturnType::Primitive(Primitive::Boolean)
        ));
        assert!(matches!(
            parse_return_type("()Ljava/lang/String;"),
            ReturnType::Object
        ));
        assert!(matches!(parse_return_type("()[I"), ReturnType::Array));
    }

    #[test]
    fn primitive_field_types_produce_jni_signatures() {
        assert_eq!(FieldType::Boolean.get_signature().unwrap(), "Z");
        assert_eq!(FieldType::Int.get_signature().unwrap(), "I");
        assert_eq!(FieldType::Long.get_signature().unwrap(), "J");
        assert_eq!(FieldType::Double.get_signature().unwrap(), "D");
        assert_eq!(
            FieldType::String.get_signature().unwrap(),
            "Ljava/lang/String;"
        );
    }

    #[test]
    fn the_bundled_java_mappings_supplement_parses() {
        let java: HashMap<String, MinecraftClass> =
            serde_json::from_str(include_str!("../../../java_mappings.json"))
                .expect("java_mappings.json must be valid");
        assert!(!java.is_empty());
    }

    #[test]
    fn the_bundled_intermediary_mappings_parse_and_translate_the_core() {
        let (version, classes) =
            load_bundled_classes(include_str!("../../../mappings_intermediary.json"))
                .expect("mappings_intermediary.json must be a valid MappingFile");

        assert_eq!(version.to_string(), "1.21.4");

        let minecraft = classes
            .get("net/minecraft/client/Minecraft")
            .expect("Minecraft must be mapped");
        assert_eq!(minecraft.name, "net/minecraft/class_310");

        let get_instance = minecraft
            .get_methods("getInstance")
            .expect("getInstance must be mapped")
            .first()
            .expect("getInstance has an entry");
        assert_eq!(get_instance.name, "method_1551");
        assert_eq!(get_instance.signature, "()Lnet/minecraft/class_310;");

        assert_eq!(
            minecraft.get_field("crosshairPickEntity").unwrap().name,
            "field_1692",
        );
        let entity = classes.get("net/minecraft/world/entity/Entity").unwrap();
        let removed = entity.get_methods("isRemoved").unwrap().first().unwrap();
        assert_eq!(removed.name, "method_31481");
        assert_eq!(removed.signature, "()Z");
        let living = classes
            .get("net/minecraft/world/entity/LivingEntity")
            .unwrap();
        assert!(living
            .get_methods("hasLineOfSight")
            .unwrap()
            .iter()
            .any(|method| {
                method.name == "method_6057" && method.signature == "(Lnet/minecraft/class_1297;)Z"
            }));

        assert!(entity
            .get_methods("getPosition")
            .unwrap()
            .iter()
            .any(|m| m.name == "method_30950" && m.signature == "(F)Lnet/minecraft/class_243;"));
        let camera = classes.get("net/minecraft/client/Camera").unwrap();
        assert!(camera
            .get_methods("getPartialTickTime")
            .unwrap()
            .iter()
            .any(|m| m.name == "method_55437" && m.signature == "()F"));
        let renderer = classes
            .get("net/minecraft/client/renderer/LevelRenderer")
            .unwrap();
        assert!(renderer
            .get_methods("getLightColor")
            .unwrap()
            .iter()
            .any(|m| m.name == "method_23794"
                && m.signature == "(Lnet/minecraft/class_1920;Lnet/minecraft/class_2338;)I"));
        let world = classes
            .get("net/minecraft/client/multiplayer/ClientLevel")
            .unwrap();
        assert!(world
            .get_methods("getSkyDarken")
            .unwrap()
            .iter()
            .any(|m| m.name == "method_23783" && m.signature == "(F)F"));
        assert!(minecraft
            .get_methods("isPaused")
            .unwrap()
            .iter()
            .any(|m| m.name == "method_1493" && m.signature == "()Z"));

        let connection = classes.get("net/minecraft/network/Connection").unwrap();
        assert_eq!(connection.get_field("channel").unwrap().name, "field_11651");
        let move_packet = classes
            .get("net/minecraft/network/protocol/game/ServerboundMovePlayerPacket")
            .unwrap();
        assert_eq!(
            move_packet.get_methods("horizontalCollision").unwrap()[0].name,
            "method_61225"
        );
        let hit = classes.get("net/minecraft/world/phys/HitResult").unwrap();
        assert_eq!(
            hit.get_methods("getType").unwrap()[0].signature,
            "()Lnet/minecraft/class_239$class_240;"
        );
        let player = classes
            .get("net/minecraft/world/entity/player/Player")
            .unwrap();
        assert_eq!(
            player.get_methods("blockInteractionRange").unwrap()[0].name,
            "method_55754"
        );
        assert_eq!(
            player.get_methods("entityInteractionRange").unwrap()[0].name,
            "method_55755"
        );

         
         
         
        let living = classes
            .get("net/minecraft/world/entity/LivingEntity")
            .unwrap();
        assert!(living
            .get_methods("isFallFlying")
            .unwrap()
            .iter()
            .any(|m| m.name == "method_6128" && m.signature == "()Z"));
         
        assert!(living
            .get_methods("getArmorValue")
            .unwrap()
            .iter()
            .any(|m| m.name == "method_6096" && m.signature == "()I"));
        let entity = classes.get("net/minecraft/world/entity/Entity").unwrap();
        assert_eq!(entity.get_field("fallDistance").unwrap().name, "field_6017");

         
         
         
        let block_state = classes
            .get("net/minecraft/world/level/block/state/BlockBehaviour$BlockStateBase")
            .expect("BlockStateBase must be mapped");
        assert!(block_state
            .get_methods("getCollisionShape")
            .expect("getCollisionShape must be mapped")
            .iter()
            .any(|m| m.name == "method_26220"
                && m.signature
                    == "(Lnet/minecraft/class_1922;Lnet/minecraft/class_2338;)Lnet/minecraft/class_265;"));

         
         
         
         
        let client_level = classes
            .get("net/minecraft/client/multiplayer/ClientLevel")
            .expect("ClientLevel must be mapped");
        for (moj_name, gate) in [
            (
                "addParticle",
                crate::jvmti::EXPLOSION_SUPPRESS_TRANSFORM_PARAMS,
            ),
            (
                "addAlwaysVisibleParticle",
                crate::jvmti::EXPLOSION_ALWAYS_VISIBLE_TRANSFORM_PARAMS,
            ),
            (
                "addAlwaysVisibleParticle",
                crate::jvmti::EXPLOSION_FORCED_PARTICLE_TRANSFORM_PARAMS,
            ),
            (
                "playLocalSound",
                crate::jvmti::EXPLOSION_SOUND_TRANSFORM_PARAMS,
            ),
        ] {
            assert_eq!(client_level.name, gate.target_class);
            assert!(
                client_level
                    .get_methods(moj_name)
                    .expect("explosion-gated method must be mapped")
                    .iter()
                    .any(|method| method.name == gate.method
                        && method.signature == gate.descriptor),
                "the explosion gate target {} {} is not mapped on {}",
                gate.method,
                gate.descriptor,
                moj_name
            );
        }

         
         
        let sound_events = classes
            .get("net/minecraft/sounds/SoundEvents")
            .expect("SoundEvents must be mapped");
        assert!(
            sound_events.get_field("GENERIC_EXPLODE").is_ok(),
            "SoundEvents.GENERIC_EXPLODE must be mapped"
        );
        let sound_source = classes
            .get("net/minecraft/sounds/SoundSource")
            .expect("SoundSource must be mapped");
        assert!(
            sound_source.get_field("BLOCKS").is_ok(),
            "SoundSource.BLOCKS must be mapped"
        );

         
        let explode = classes
            .get("net/minecraft/network/protocol/game/ClientboundExplodePacket")
            .expect("ClientboundExplodePacket must be mapped");
        assert!(explode
            .get_methods("center")
            .expect("center must be mapped")
            .iter()
            .any(|method| method.signature == "()Lnet/minecraft/class_243;"));

         
         
         
        let item_scale = crate::jvmti::ITEM_SCALE_TRANSFORM_PARAMS;
        let view_model = classes
            .get("net/minecraft/client/renderer/ItemInHandRenderer")
            .expect("ItemInHandRenderer must be mapped");
        assert_eq!(view_model.name, item_scale.target_class);
        assert!(
            view_model
                .get_methods("renderArmWithItem")
                .expect("renderArmWithItem must be mapped")
                .iter()
                .any(|method| method.name == item_scale.method
                    && method.signature == item_scale.descriptor),
            "the item scale target {} {} is not mapped",
            item_scale.method,
            item_scale.descriptor
        );

         
         
         
        let sky = crate::jvmti::SKY_COLOR_TRANSFORM_PARAMS;
        let level = classes
            .get("net/minecraft/client/multiplayer/ClientLevel")
            .expect("ClientLevel must be mapped");
        assert_eq!(level.name, sky.target_class);
        assert!(
            level
                .get_methods("getSkyColor")
                .expect("getSkyColor must be mapped")
                .iter()
                .any(|method| method.name == sky.method && method.signature == sky.descriptor),
            "the sky target {} {} is not mapped",
            sky.method,
            sky.descriptor
        );

         
         
         
         
        let listener = classes
            .get("net/minecraft/client/multiplayer/ClientPacketListener")
            .expect("ClientPacketListener must be mapped");
        for (name, intermediary) in [
            ("sendChat", "method_45729"),
            ("sendCommand", "method_45730"),
        ] {
            assert!(
                listener
                    .get_methods(name)
                    .unwrap_or_else(|_| panic!("{name} must be mapped"))
                    .iter()
                    .any(|method| method.name == intermediary
                        && method.signature == "(Ljava/lang/String;)V"),
                "{name} must be {intermediary} (Ljava/lang/String;)V"
            );
        }

         
         
        let local_player = classes
            .get("net/minecraft/client/player/LocalPlayer")
            .expect("LocalPlayer must be mapped");
        assert!(local_player
            .get_methods("drop")
            .expect("LocalPlayer.drop must be mapped")
            .iter()
            .any(|method| method.name == "method_7290" && method.signature == "(Z)Z"));

        let window = classes
            .get("com/mojang/blaze3d/platform/Window")
            .expect("Window must be mapped");
        assert_eq!(window.name, "net/minecraft/class_1041");
    }
}
