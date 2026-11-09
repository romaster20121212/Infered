use crate::state;
use anyhow::anyhow;
use jni::objects::{JClass, JObject, JString, JValue, JValueOwned};
use jni::JNIEnv;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

 
fn deserialize_methods<'de, D>(deserializer: D) -> Result<HashMap<String, Vec<Method>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct MethodsVisitor;

    impl<'de> Visitor<'de> for MethodsVisitor {
        type Value = HashMap<String, Vec<Method>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map of method names to methods or arrays of methods")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut methods = HashMap::new();

            while let Some(key) = map.next_key::<String>()? {
                 
                match map.next_value::<MethodOrVec>()? {
                    MethodOrVec::Single(method) => {
                        methods.insert(key, vec![method]);
                    }
                    MethodOrVec::Multiple(method_vec) => {
                        methods.insert(key, method_vec);
                    }
                }
            }

            Ok(methods)
        }
    }

    deserializer.deserialize_map(MethodsVisitor)
}

 
#[derive(Deserialize)]
#[serde(untagged)]
enum MethodOrVec {
    Single(Method),
    Multiple(Vec<Method>),
}

 
#[derive(Debug, Deserialize)]
pub struct MinecraftClass {
    pub name: String,
    #[serde(deserialize_with = "deserialize_methods", default)]
    methods: HashMap<String, Vec<Method>>,
    fields: HashMap<String, Field>,
}

 
#[derive(Debug, Deserialize)]
pub struct Method {
    pub name: String,
    pub signature: String,
     
     
     
    #[serde(skip)]
    pub id: OnceLock<MethodHandle>,
}

 
 
#[derive(Debug, Clone, Copy)]
pub struct MethodHandle(pub jni::sys::jmethodID);

unsafe impl Send for MethodHandle {}
unsafe impl Sync for MethodHandle {}

 
#[derive(Debug, Deserialize)]
pub struct Field {
    pub name: String,
}

 
#[derive(Debug, PartialEq)]
enum SignatureMatch {
    Exact,
    Compatible,
    Incompatible,
}

#[allow(dead_code)]
impl MinecraftClass {
     
     
     
    pub fn from_reflection(name: String, methods: HashMap<String, Vec<Method>>) -> Self {
        MinecraftClass {
            name,
            methods,
            fields: HashMap::new(),
        }
    }

    pub fn get_method(&self, name: &str) -> anyhow::Result<&Method> {
        self.methods
            .get(name)
            .and_then(|overloads| overloads.first())
            .ok_or_else(|| anyhow!("{} method not found", name))
    }

    pub fn get_methods(&self, name: &str) -> anyhow::Result<&Vec<Method>> {
        match self.methods.get(name) {
            Some(methods) => Ok(methods),
            None => Err(anyhow!("{} method not found", name)),
        }
    }

    pub fn get_method_by_signature(&self, name: &str, signature: &str) -> anyhow::Result<&Method> {
        let methods = self.get_methods(name)?;
        match methods.iter().find(|method| method.signature == signature) {
            Some(method) => Ok(method),
            None => Err(anyhow!(
                "{} method with signature {} not found",
                name,
                signature
            )),
        }
    }

    pub fn get_method_by_args(&self, name: &str, args: &[JValue]) -> anyhow::Result<&Method> {
        let methods = self.get_methods(name)?;

         
        if methods.len() == 1 {
            return Ok(&methods[0]);
        }

         
        let mut best_method = None;
        let mut best_match_quality = SignatureMatch::Incompatible;

        for method in methods {
            let match_quality = self.evaluate_signature_compatibility(&method.signature, args);

            if match_quality == SignatureMatch::Exact {
                 
                return Ok(method);
            }

            if match_quality == SignatureMatch::Compatible
                && best_match_quality != SignatureMatch::Exact
            {
                best_method = Some(method);
                best_match_quality = match_quality;
            }
        }

        match best_method {
            Some(method) => {
                log::debug!(
                    "Using compatible method '{}' with signature '{}' for args",
                    name,
                    method.signature
                );
                Ok(method)
            }
            None => {
                log::warn!(
                    "No compatible method found for '{}' with {} arguments, using first available method",
                    name, args.len()
                );
                Ok(&methods[0])
            }
        }
    }

     
    fn evaluate_signature_compatibility(
        &self,
        method_signature: &str,
        args: &[JValue],
    ) -> SignatureMatch {
        let param_types = match self.extract_parameter_types(method_signature) {
            Ok(types) => types,
            Err(_) => return SignatureMatch::Incompatible,
        };

         
        if param_types.len() != args.len() {
            return SignatureMatch::Incompatible;
        }

        let mut exact_matches = 0;
        let mut compatible_matches = 0;

         
        for (param_type, arg) in param_types.iter().zip(args.iter()) {
            match self.check_type_compatibility(param_type, arg) {
                SignatureMatch::Exact => exact_matches += 1,
                SignatureMatch::Compatible => compatible_matches += 1,
                SignatureMatch::Incompatible => return SignatureMatch::Incompatible,
            }
        }

        if exact_matches == args.len() {
            SignatureMatch::Exact
        } else if exact_matches + compatible_matches == args.len() {
            SignatureMatch::Compatible
        } else {
            SignatureMatch::Incompatible
        }
    }

     
     
     
     
    fn extract_parameter_types(&self, signature: &str) -> Result<Vec<String>, &'static str> {
        let start = signature
            .find('(')
            .ok_or("Invalid signature: missing opening parenthesis")?;
        let end = signature
            .find(')')
            .ok_or("Invalid signature: missing closing parenthesis")?;

        if start >= end {
            return Err("Invalid signature: malformed parentheses");
        }

        let params_str = &signature[start + 1..end];
        if params_str.is_empty() {
            return Ok(Vec::new());
        }

        let mut types = Vec::new();
        let mut chars = params_str.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                 
                'Z' | 'B' | 'C' | 'S' | 'I' | 'J' | 'F' | 'D' => {
                    types.push(ch.to_string());
                }
                 
                'L' => {
                    let mut object_type = String::from("L");
                    for ch in chars.by_ref() {
                        object_type.push(ch);
                        if ch == ';' {
                            break;
                        }
                    }
                    types.push(object_type);
                }
                 
                 
                 
                 
                '[' => {
                    let mut array_type = String::from("[");
                    while chars.peek() == Some(&'[') {
                        array_type.push('[');
                        chars.next();
                    }
                    match chars.next() {
                        Some(component @ ('Z' | 'B' | 'C' | 'S' | 'I' | 'J' | 'F' | 'D')) => {
                            array_type.push(component);
                        }
                        Some('L') => {
                            array_type.push('L');
                            let mut closed = false;
                            for ch in chars.by_ref() {
                                array_type.push(ch);
                                if ch == ';' {
                                    closed = true;
                                    break;
                                }
                            }
                            if !closed {
                                return Err("Invalid array type in signature");
                            }
                        }
                        _ => return Err("Invalid array type in signature"),
                    }
                    types.push(array_type);
                }
                _ => return Err("Unknown type character in signature"),
            }
        }

        Ok(types)
    }

     
    fn check_type_compatibility(&self, jni_type: &str, value: &JValue) -> SignatureMatch {
        match (jni_type, value) {
             
            ("Z", JValue::Bool(_)) => SignatureMatch::Exact,
            ("B", JValue::Byte(_)) => SignatureMatch::Exact,
            ("C", JValue::Char(_)) => SignatureMatch::Exact,
            ("S", JValue::Short(_)) => SignatureMatch::Exact,
            ("I", JValue::Int(_)) => SignatureMatch::Exact,
            ("J", JValue::Long(_)) => SignatureMatch::Exact,
            ("F", JValue::Float(_)) => SignatureMatch::Exact,
            ("D", JValue::Double(_)) => SignatureMatch::Exact,

             
            ("I", JValue::Byte(_) | JValue::Short(_) | JValue::Char(_)) => {
                SignatureMatch::Compatible
            }
            ("J", JValue::Byte(_) | JValue::Short(_) | JValue::Char(_) | JValue::Int(_)) => {
                SignatureMatch::Compatible
            }
            ("F", JValue::Byte(_) | JValue::Short(_) | JValue::Char(_) | JValue::Int(_)) => {
                SignatureMatch::Compatible
            }
            (
                "D",
                JValue::Byte(_)
                | JValue::Short(_)
                | JValue::Char(_)
                | JValue::Int(_)
                | JValue::Long(_)
                | JValue::Float(_),
            ) => SignatureMatch::Compatible,

             
            (jni_type, JValue::Object(obj))
                if jni_type.starts_with('L') && jni_type.ends_with(';') =>
            unsafe { self.check_object_type_compatibility(jni_type, obj) },

             
            (jni_type, JValue::Object(obj)) if jni_type.starts_with('[') => unsafe {
                self.check_array_type_compatibility(jni_type, obj)
            },

             
            (jni_type, JValue::Object(obj))
                if jni_type.starts_with('L') || jni_type.starts_with('[') =>
            {
                if obj.is_null() {
                    SignatureMatch::Compatible
                } else {
                    SignatureMatch::Incompatible
                }
            }

            _ => SignatureMatch::Incompatible,
        }
    }

     
    unsafe fn check_object_type_compatibility(
        &self,
        expected_type: &str,
        obj: &JObject,
    ) -> SignatureMatch {
         
        if obj.is_null() {
            return SignatureMatch::Compatible;
        }

         
         
        let expected_class_name = &expected_type[1..expected_type.len() - 1];

         
        if expected_class_name == "java/lang/Object" {
            return SignatureMatch::Compatible;
        }

         
        if let Ok(mut env) = state::env() {
             
            if let Ok(obj_class) = env.get_object_class(obj) {
                 
                 
                 
                 
                if let Ok(expected_class) =
                    state::mapping().resolve_class(&mut env, expected_class_name)
                {
                    if let Ok(same_class) = env.is_same_object(&obj_class, &expected_class) {
                        if same_class {
                            return SignatureMatch::Exact;
                        }
                    }

                     
                    if let Ok(is_instance) = env.is_instance_of(obj, &expected_class) {
                        if is_instance {
                            return SignatureMatch::Compatible;
                        }
                    }
                }

                 
                if let Ok(class_name) = self.get_class_name_from_object(&mut env, &obj_class) {
                    if self.are_compatible_types(&class_name, expected_class_name) {
                        return SignatureMatch::Compatible;
                    }
                }
            }
        }

        SignatureMatch::Incompatible
    }

     
    unsafe fn check_array_type_compatibility(
        &self,
        expected_type: &str,
        obj: &JObject,
    ) -> SignatureMatch {
         
        if obj.is_null() {
            return SignatureMatch::Compatible;
        }

        if let Ok(mut env) = state::env() {
             
            if let Ok(obj_class) = env.get_object_class(obj) {
                if let Ok(class_name) = self.get_class_name_from_object(&mut env, &obj_class) {
                     
                    if class_name.starts_with('[') {
                         
                        if class_name == expected_type {
                            return SignatureMatch::Exact;
                        }

                         
                         
                        if self.are_compatible_array_types(&class_name, expected_type) {
                            return SignatureMatch::Compatible;
                        }
                    }
                }
            }
        }

        SignatureMatch::Incompatible
    }

     
    unsafe fn get_class_name_from_object(
        &self,
        env: &mut JNIEnv,
        class: &JClass,
    ) -> Result<String, Box<dyn std::error::Error>> {
         
        let class_class = env.find_class("java/lang/Class")?;
        let get_name_method = env.get_method_id(&class_class, "getName", "()Ljava/lang/String;")?;

         
        let name_obj = env.call_method_unchecked(
            class,
            get_name_method,
            jni::signature::ReturnType::Object,
            &[],
        )?;

        if let JValueOwned::Object(name_str) = name_obj {
            let jstring = JString::from(name_str);
            let class_name = env.get_string(&jstring)?.to_str()?.to_string();

             
             
             
            let jni_name = class_name.replace('.', "/");
            Ok(jni_name)
        } else {
            Err("Failed to get class name as string".into())
        }
    }

     
    fn are_compatible_types(&self, actual_type: &str, expected_type: &str) -> bool {
         
        if actual_type == expected_type {
            return true;
        }

         
        match (actual_type, expected_type) {
             
            ("java/lang/String", "java/lang/CharSequence") => true,

             
             

             
            ("java/util/ArrayList", "java/util/List") => true,
            ("java/util/LinkedList", "java/util/List") => true,
            ("java/util/HashSet", "java/util/Set") => true,
            ("java/util/HashMap", "java/util/Map") => true,

             
            _ => false,
        }
    }

     
    fn are_compatible_array_types(&self, actual_type: &str, expected_type: &str) -> bool {
         
        if let (Some(actual_component), Some(expected_component)) = (
            self.extract_array_component_type(actual_type),
            self.extract_array_component_type(expected_type),
        ) {
             
            if actual_component.len() == 1 && expected_component.len() == 1 {
                return actual_component == expected_component;
            }

             
            if actual_component.starts_with('L') && expected_component.starts_with('L') {
                let actual_class = &actual_component[1..actual_component.len() - 1];
                let expected_class = &expected_component[1..expected_component.len() - 1];
                return self.are_compatible_types(actual_class, expected_class);
            }
        }

        false
    }

     
     
    fn extract_array_component_type(&self, array_type: &str) -> Option<String> {
        if array_type.starts_with('[') && array_type.len() > 1 {
            Some(array_type[1..].to_string())
        } else {
            None
        }
    }

     
    pub fn method_names(&self) -> Vec<String> {
        self.methods.keys().cloned().collect()
    }

    pub fn get_field(&self, name: &str) -> anyhow::Result<&Field> {
        match self.fields.get(name) {
            Some(fields) => Ok(fields),
            None => Err(anyhow!("{} field not found", name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_type_extraction() {
        let class = MinecraftClass {
            name: "TestClass".to_string(),
            methods: HashMap::new(),
            fields: HashMap::new(),
        };

         
        assert_eq!(
            class.extract_parameter_types("()V").unwrap(),
            Vec::<String>::new()
        );

        assert_eq!(class.extract_parameter_types("(I)V").unwrap(), vec!["I"]);

        assert_eq!(
            class
                .extract_parameter_types("(ILjava/lang/String;F)V")
                .unwrap(),
            vec!["I", "Ljava/lang/String;", "F"]
        );

         
        assert_eq!(class.extract_parameter_types("([I)V").unwrap(), vec!["[I"]);
         
        assert_eq!(
            class.extract_parameter_types("([[I)V").unwrap(),
            vec!["[[I"]
        );
        assert_eq!(
            class
                .extract_parameter_types("([[Ljava/lang/String;)V")
                .unwrap(),
            vec!["[[Ljava/lang/String;"]
        );
        assert_eq!(
            class
                .extract_parameter_types("([[I[[Ljava/lang/String;)V")
                .unwrap(),
            vec!["[[I", "[[Ljava/lang/String;"]
        );
         
        assert!(class.extract_parameter_types("([)V").is_err());
        assert!(class.extract_parameter_types("([L)V").is_err());
    }

    #[test]
    fn test_type_compatibility() {
        let class = MinecraftClass {
            name: "TestClass".to_string(),
            methods: HashMap::new(),
            fields: HashMap::new(),
        };

         
        assert_eq!(
            class.check_type_compatibility("I", &JValue::Int(42)),
            SignatureMatch::Exact
        );

         
        assert_eq!(
            class.check_type_compatibility("I", &JValue::Byte(42)),
            SignatureMatch::Compatible
        );

         
        assert_eq!(
            class.check_type_compatibility("I", &JValue::Double(42.0)),
            SignatureMatch::Incompatible
        );
    }

    #[test]
    fn methods_deserialize_from_a_single_object_or_an_array() {
        let json = r#"{
            "name": "Obf",
            "methods": {
                "single": { "name": "a", "signature": "()V" },
                "many": [
                    { "name": "b", "signature": "(I)V" },
                    { "name": "b", "signature": "(F)V" }
                ]
            },
            "fields": {}
        }"#;
        let class: MinecraftClass = serde_json::from_str(json).unwrap();
        assert_eq!(class.get_methods("single").unwrap().len(), 1);
        assert_eq!(class.get_methods("many").unwrap().len(), 2);
    }

    #[test]
    fn overload_resolution_prefers_an_exact_primitive_match() {
        let mut methods = HashMap::new();
        methods.insert(
            "foo".to_string(),
            vec![
                Method {
                    name: "foo".into(),
                    signature: "(D)V".into(),
                    id: OnceLock::new(),
                },
                Method {
                    name: "foo".into(),
                    signature: "(I)V".into(),
                    id: OnceLock::new(),
                },
            ],
        );
        let class = MinecraftClass {
            name: "T".into(),
            methods,
            fields: HashMap::new(),
        };

        let chosen = class.get_method_by_args("foo", &[JValue::Int(7)]).unwrap();
        assert_eq!(chosen.signature, "(I)V");
    }
}
