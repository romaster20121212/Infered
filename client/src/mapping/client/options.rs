 

use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JObject, JString, JValue};

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Perspective {
    FirstPerson,
    ThirdPersonBack,
    ThirdPersonFront,
}

 
 
 
 
 
 
 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackIndicator {
     
    Off,
     
    Crosshair,
     
    Hotbar,
}

 
#[derive(Debug, MappedObject)]
#[mapped(class = Options)]
pub struct Options {
    jni_ref: GlobalRef,
}

impl Options {
     
    pub fn new(jni_ref: GlobalRef) -> Options {
        Options { jni_ref }
    }

     
     
    pub fn hide_gui(&self) -> anyhow::Result<bool> {
        self.in_frame(|| Ok(self.get_field("hideGui", FieldType::Boolean)?.z()?))
    }

    pub fn perspective(&self) -> anyhow::Result<Perspective> {
        self.in_frame(|| {
            let value = self.call_method("getCameraType", &[])?.l()?;
            let ordinal = mapping()
                .get_env()?
                .call_method(&value, "ordinal", "()I", &[])?
                .i()?;
            match ordinal {
                0 => Ok(Perspective::FirstPerson),
                1 => Ok(Perspective::ThirdPersonBack),
                2 => Ok(Perspective::ThirdPersonFront),
                _ => anyhow::bail!("Unknown camera perspective {ordinal}"),
            }
        })
    }

     
     
     
     
     
    pub fn attack_indicator(&self) -> anyhow::Result<AttackIndicator> {
        self.in_frame(|| {
            let option = self
                .get_field(
                    "attackIndicator",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
                .l()?;
            let value = mapping()
                .call_method(MinecraftClassType::OptionInstance, &option, "get", &[])?
                .l()?;
            let mut env = mapping().get_env()?;
            let name = env
                .call_method(&value, "name", "()Ljava/lang/String;", &[])?
                .l()?;
            let name = env
                .get_string(&JString::from(name))?
                .to_string_lossy()
                .into_owned();
            Ok(match name.as_str() {
                "OFF" => AttackIndicator::Off,
                "HOTBAR" => AttackIndicator::Hotbar,
                 
                 
                _ => AttackIndicator::Crosshair,
            })
        })
    }

     
    pub(crate) fn misc_option(&self, name: &str) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field(name, FieldType::Object(MinecraftClassType::OptionInstance))?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

     
    pub fn fov(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field("fov", FieldType::Object(MinecraftClassType::OptionInstance))?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

     
     
     
    pub fn fov_effect_scale(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field(
                    "fovEffectScale",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

     
     
    pub fn entity_distance_scaling(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field(
                    "entityDistanceScaling",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

     
    pub fn gamma(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field(
                    "gamma",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

     
    pub fn sensitivity(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field(
                    "sensitivity",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

     
    pub fn key_use(&self) -> anyhow::Result<KeyMapping> {
        self.in_frame(|| {
            let key = self
                .get_field("keyUse", FieldType::Object(MinecraftClassType::KeyMapping))?
                .l()?;
            Ok(KeyMapping::new(mapping().new_global_ref(key)?))
        })
    }

     
    pub fn key_attack(&self) -> anyhow::Result<KeyMapping> {
        self.in_frame(|| {
            let key = self
                .get_field(
                    "keyAttack",
                    FieldType::Object(MinecraftClassType::KeyMapping),
                )?
                .l()?;
            Ok(KeyMapping::new(mapping().new_global_ref(key)?))
        })
    }

     
    pub fn key_up(&self) -> anyhow::Result<KeyMapping> {
        self.in_frame(|| {
            let key = self
                .get_field("keyUp", FieldType::Object(MinecraftClassType::KeyMapping))?
                .l()?;
            Ok(KeyMapping::new(mapping().new_global_ref(key)?))
        })
    }

     
    pub fn key_back(&self) -> anyhow::Result<KeyMapping> {
        self.in_frame(|| {
            let key = self
                .get_field("keyDown", FieldType::Object(MinecraftClassType::KeyMapping))?
                .l()?;
            Ok(KeyMapping::new(mapping().new_global_ref(key)?))
        })
    }

     
    pub fn key_left(&self) -> anyhow::Result<KeyMapping> {
        self.in_frame(|| {
            let key = self
                .get_field("keyLeft", FieldType::Object(MinecraftClassType::KeyMapping))?
                .l()?;
            Ok(KeyMapping::new(mapping().new_global_ref(key)?))
        })
    }

     
    pub fn key_right(&self) -> anyhow::Result<KeyMapping> {
        self.in_frame(|| {
            let key = self
                .get_field(
                    "keyRight",
                    FieldType::Object(MinecraftClassType::KeyMapping),
                )?
                .l()?;
            Ok(KeyMapping::new(mapping().new_global_ref(key)?))
        })
    }

     
    pub fn key_sprint(&self) -> anyhow::Result<KeyMapping> {
        self.in_frame(|| {
            let key = self
                .get_field(
                    "keySprint",
                    FieldType::Object(MinecraftClassType::KeyMapping),
                )?
                .l()?;
            Ok(KeyMapping::new(mapping().new_global_ref(key)?))
        })
    }

     
    pub fn key_jump(&self) -> anyhow::Result<KeyMapping> {
        self.in_frame(|| {
            let key = self
                .get_field("keyJump", FieldType::Object(MinecraftClassType::KeyMapping))?
                .l()?;
            Ok(KeyMapping::new(mapping().new_global_ref(key)?))
        })
    }

     
    pub fn key_shift(&self) -> anyhow::Result<KeyMapping> {
        self.in_frame(|| {
            let key = self
                .get_field(
                    "keyShift",
                    FieldType::Object(MinecraftClassType::KeyMapping),
                )?
                .l()?;
            Ok(KeyMapping::new(mapping().new_global_ref(key)?))
        })
    }

     
    pub fn render_distance(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field(
                    "renderDistance",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

     
    pub fn biome_blend_radius(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field(
                    "biomeBlendRadius",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

     
    pub fn ambient_occlusion(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field(
                    "ambientOcclusion",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

     
    pub fn menu_background_blurriness(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field(
                    "menuBackgroundBlurriness",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

     
    pub fn entity_shadows(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field(
                    "entityShadows",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

     
     
    pub fn framerate_limit(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field(
                    "framerateLimit",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

     
    pub fn enable_vsync(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field(
                    "enableVsync",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

     
     
     
     
     
    pub fn server_render_distance(&self) -> anyhow::Result<i32> {
        self.in_frame(|| {
            Ok(self
                .get_field("serverRenderDistance", FieldType::Int)?
                .i()?)
        })
    }

     
     
     
     
     
    pub fn set_server_render_distance(&self, chunks: i32) -> anyhow::Result<()> {
        self.in_frame(|| {
            self.call_method("setServerRenderDistance", &[JValue::Int(chunks)])?;
            Ok(())
        })
    }
}

 
#[derive(Debug, MappedObject)]
#[mapped(class = OptionInstance)]
pub struct OptionInstance {
    jni_ref: GlobalRef,
}

impl OptionInstance {
     
    pub fn new(jni_ref: GlobalRef) -> OptionInstance {
        OptionInstance { jni_ref }
    }

     
     
    pub fn get_int(&self) -> anyhow::Result<i32> {
        self.in_frame(|| {
            let value = self.call_method("get", &[])?.l()?;
            Ok(mapping()
                .call_method(MinecraftClassType::Integer, &value, "intValue", &[])?
                .i()?)
        })
    }

     
     
    pub fn get_double(&self) -> anyhow::Result<f64> {
        self.in_frame(|| {
            let value = self.call_method("get", &[])?.l()?;
            Ok(mapping()
                .call_method(MinecraftClassType::Double, &value, "doubleValue", &[])?
                .d()?)
        })
    }

     
     
     
     
     
     
    pub fn force_double(&self, value: f64) -> anyhow::Result<()> {
        self.in_frame(|| {
            let mut env = mapping().get_env()?;
            let double_class = env.find_class("java/lang/Double")?;
            let boxed = env.new_object(&double_class, "(D)V", &[JValue::Double(value)])?;
            self.set_field(
                "value",
                FieldType::Object(MinecraftClassType::Object),
                JValue::Object(&boxed),
            )
        })
    }

     
     
    pub fn get_bool(&self) -> anyhow::Result<bool> {
        self.in_frame(|| {
            let value = self.call_method("get", &[])?.l()?;
            Ok(mapping()
                .call_method(MinecraftClassType::Boolean, &value, "booleanValue", &[])?
                .z()?)
        })
    }

     
     
     
    pub fn force_bool(&self, value: bool) -> anyhow::Result<()> {
        self.in_frame(|| {
            let mut env = mapping().get_env()?;
            let boolean_class = env.find_class("java/lang/Boolean")?;
            let boxed = env.new_object(&boolean_class, "(Z)V", &[JValue::Bool(u8::from(value))])?;
            self.set_field(
                "value",
                FieldType::Object(MinecraftClassType::Object),
                JValue::Object(&boxed),
            )
        })
    }

     
     
     
     
     
     
    pub fn force_int(&self, value: i32) -> anyhow::Result<()> {
        self.in_frame(|| {
            let mut env = mapping().get_env()?;
            let integer_class = env.find_class("java/lang/Integer")?;
            let boxed = env.new_object(&integer_class, "(I)V", &[JValue::Int(value)])?;
            self.set_field(
                "value",
                FieldType::Object(MinecraftClassType::Object),
                JValue::Object(&boxed),
            )
        })
    }
}

 
#[derive(Debug, MappedObject)]
#[mapped(class = KeyMapping)]
pub struct KeyMapping {
    jni_ref: GlobalRef,
}

impl KeyMapping {
     
    pub fn new(jni_ref: GlobalRef) -> KeyMapping {
        KeyMapping { jni_ref }
    }

    pub fn is_down(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isDown", &[])?.z()?)
    }

     
     
     
    pub fn set_down(&self, down: bool) -> anyhow::Result<()> {
        self.set_field(
            "isDown",
            FieldType::Boolean,
            JValue::Bool(if down { 1 } else { 0 }),
        )
    }

     
     
     
     
    pub fn key_name(&self) -> anyhow::Result<String> {
        mapping().in_frame(|| {
            let mut env = mapping().get_env()?;
            let key = mapping()
                .get_field(
                    MinecraftClassType::KeyMapping,
                    self.jni_ref.as_obj(),
                    "key",
                    FieldType::Object(MinecraftClassType::InputConstantsKey),
                )?
                .l()?;
            let name = mapping()
                .call_method(MinecraftClassType::InputConstantsKey, &key, "getName", &[])?
                .l()?;
            Ok(env
                .get_string(&JString::from(name))?
                .to_string_lossy()
                .into_owned())
        })
    }

     
     
     
    pub fn key_value(&self) -> anyhow::Result<i32> {
        mapping().in_frame(|| {
            let key = mapping()
                .get_field(
                    MinecraftClassType::KeyMapping,
                    self.jni_ref.as_obj(),
                    "key",
                    FieldType::Object(MinecraftClassType::InputConstantsKey),
                )?
                .l()?;
            Ok(mapping()
                .call_method(MinecraftClassType::InputConstantsKey, &key, "getValue", &[])?
                .i()?)
        })
    }

     
     
     
     
    pub fn tap(&self) -> anyhow::Result<()> {
        mapping().in_frame(|| {
            let key = mapping()
                .get_field(
                    MinecraftClassType::KeyMapping,
                    self.jni_ref.as_obj(),
                    "key",
                    FieldType::Object(MinecraftClassType::InputConstantsKey),
                )?
                .l()?;
            mapping().call_static_method(
                MinecraftClassType::KeyMapping,
                "click",
                &[JValue::Object(&key)],
            )?;
            Ok(())
        })
    }

     
     
     
    pub fn click_named(name: &str) -> anyhow::Result<()> {
        mapping().in_frame(|| {
            let name: JObject = mapping().get_env()?.new_string(name)?.into();
            let key = mapping()
                .call_static_method(
                    MinecraftClassType::InputConstants,
                    "getKey",
                    &[JValue::Object(&name)],
                )?
                .l()?;
            mapping().call_static_method(
                MinecraftClassType::KeyMapping,
                "click",
                &[JValue::Object(&key)],
            )?;
            Ok(())
        })
    }
}
