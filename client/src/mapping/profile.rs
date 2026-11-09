 
 
 
 
 
 
 
 
 

use crate::mapping::client::minecraft::Minecraft;
use crate::mapping::MappedObject;
use crate::state::mapping;
use anyhow::anyhow;
use jni::objects::{JByteArray, JObject, JValue};
use jni::JNIEnv;

 
#[derive(Debug, Clone)]
pub struct Profile {
    jni_ref: jni::objects::GlobalRef,
}

 
 
 
macro_rules! jni {
    ($env:expr, $call:expr) => {
        match $call {
            Ok(value) => value,
            Err(error) => {
                let _ = $env.exception_clear();
                return Err(anyhow::anyhow!("JNI call failed: {error}"));
            }
        }
    };
}

impl Profile {
     
    pub fn from_minecraft(minecraft: &Minecraft) -> anyhow::Result<Profile> {
        let obj = minecraft.in_frame(|| {
            minecraft
                .call_method("getGameProfile", &[])?
                .l()
                .map_err(|e| anyhow!("getGameProfile: {e}"))
        })?;
        Ok(Profile {
            jni_ref: mapping().new_global_ref(obj)?,
        })
    }

     
    pub fn name(&self) -> anyhow::Result<String> {
        mapping().in_frame(|| {
            let mut env = mapping().get_env()?;
            let name = jni!(
                env,
                env.call_method(
                    self.jni_ref.as_obj(),
                    "getName",
                    "()Ljava/lang/String;",
                    &[]
                )
            )
            .l()?;
            mapping().get_string(name)
        })
    }

     
     
    pub fn skin_url(&self) -> anyhow::Result<Option<String>> {
        mapping().in_frame(|| {
            let mut env = mapping().get_env()?;

            let props = jni!(
                env,
                env.call_method(
                    self.jni_ref.as_obj(),
                    "getProperties",
                    "()Lcom/mojang/authlib/properties/PropertyMap;",
                    &[]
                )
            )
            .l()?;
            if props.is_null() {
                return Ok(None);
            }

            let key: JObject = jni!(env, env.new_string("textures")).into();
            let textures = jni!(
                env,
                env.call_method(
                    &props,
                    "get",
                    "(Ljava/lang/String;)Ljava/util/Collection;",
                    &[JValue::Object(&key)]
                )
            )
            .l()?;
            if textures.is_null() {
                return Ok(None);
            }

            let property = first_property(&mut env, &textures)?;
            let Some(property) = property else {
                return Ok(None);
            };

            let encoded = jni!(
                env,
                env.call_method(&property, "getValue", "()Ljava/lang/String;", &[])
            )
            .l()?;
            let base64 = mapping().get_string(encoded)?;

            let json = decode_base64(&base64)
                .ok_or_else(|| anyhow!("malformed base64 in textures property"))?;
            let parsed: serde_json::Value = serde_json::from_slice(&json)?;
            Ok(parsed["textures"]["SKIN"]["url"]
                .as_str()
                .map(str::to_owned))
        })
    }
}

 
 
 
#[allow(clippy::needless_lifetimes)]
fn first_property<'a>(
    env: &mut JNIEnv<'a>,
    collection: &JObject<'a>,
) -> anyhow::Result<Option<JObject<'a>>> {
    let iterator = jni!(
        env,
        env.call_method(collection, "iterator", "()Ljava/util/Iterator;", &[])
    )
    .l()?;
    if iterator.is_null() {
        return Ok(None);
    }
    let has_next = jni!(env, env.call_method(&iterator, "hasNext", "()Z", &[])).z()?;
    if !has_next {
        return Ok(None);
    }
    let property = jni!(
        env,
        env.call_method(&iterator, "next", "()Ljava/lang/Object;", &[])
    )
    .l()?;
    Ok((!property.is_null()).then_some(property))
}

 
const CONNECT_TIMEOUT_MS: i32 = 3_000;
 
const READ_TIMEOUT_MS: i32 = 5_000;

 
 
 
 
 
 
 
 
pub fn fetch_url(url: &str) -> anyhow::Result<Vec<u8>> {
    mapping().in_frame(|| {
        let mut env = mapping().get_env()?;
        let url_class = match env.find_class("java/net/URL") {
            Ok(class) => class,
            Err(error) => {
                let _ = env.exception_clear();
                return Err(anyhow::anyhow!("java.net.URL unavailable: {error}"));
            }
        };

        let spec: JObject = jni!(env, env.new_string(url)).into();
        let jurl = jni!(
            env,
            env.new_object(
                &url_class,
                "(Ljava/lang/String;)V",
                &[JValue::Object(&spec)]
            )
        );

         
         
        let connection = jni!(
            env,
            env.call_method(&jurl, "openConnection", "()Ljava/net/URLConnection;", &[])
        )
        .l()?;
        for (setter, millis) in [
            ("setConnectTimeout", CONNECT_TIMEOUT_MS),
            ("setReadTimeout", READ_TIMEOUT_MS),
        ] {
            jni!(
                env,
                env.call_method(&connection, setter, "(I)V", &[JValue::Int(millis)])
            );
        }

        let stream = jni!(
            env,
            env.call_method(
                &connection,
                "getInputStream",
                "()Ljava/io/InputStream;",
                &[]
            )
        )
        .l()?;
        let bytes = jni!(env, env.call_method(&stream, "readAllBytes", "()[B", &[])).l()?;
        let raw = jni!(env, env.convert_byte_array(JByteArray::from(bytes)));
         
        if env.call_method(&stream, "close", "()V", &[]).is_err() {
            let _ = env.exception_clear();
        }
        Ok(raw)
    })
}

 
 
pub fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut accumulator: u32 = 0;
    let mut bits: u32 = 0;
    for c in input.chars() {
        let value = match c {
            'A'..='Z' => c as u8 - b'A',
            'a'..='z' => c as u8 - b'a' + 26,
            '0'..='9' => c as u8 - b'0' + 52,
            '+' => 62,
            '/' => 63,
            '=' => break,
            c if c.is_whitespace() => continue,
            _ => return None,
        };
        accumulator = (accumulator << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((accumulator >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::decode_base64;

    #[test]
    fn decodes_the_empty_string() {
        assert_eq!(decode_base64(""), Some(vec![]));
    }

    #[test]
    fn decodes_padded_payloads() {
        assert_eq!(decode_base64("Zm9v"), Some(b"foo".to_vec()));
        assert_eq!(decode_base64("Zm9vYg=="), Some(b"foob".to_vec()));
        assert_eq!(decode_base64("Zm9vYmE="), Some(b"fooba".to_vec()));
        assert_eq!(decode_base64("Zm9vYmFy"), Some(b"foobar".to_vec()));
    }

    #[test]
    fn decodes_with_and_without_padding() {
        assert_eq!(decode_base64("aGVsbG8="), Some(b"hello".to_vec()));
        assert_eq!(decode_base64("aGVsbG8"), Some(b"hello".to_vec()));
    }

    #[test]
    fn ignores_whitespace() {
        assert_eq!(decode_base64("Zm9v\nIGJh\tcg=="), Some(b"foo bar".to_vec()));
    }

    #[test]
    fn rejects_junk() {
        assert_eq!(decode_base64("$$$$"), None);
        assert_eq!(decode_base64("Zm9v!"), None);
    }
}
