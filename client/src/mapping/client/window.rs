use crate::mapping::{MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValueOwned};
use jni::sys::jlong;

#[derive(Debug, MappedObject)]
#[mapped(class = Window)]
pub struct Window {
    jni_ref: GlobalRef,
}

impl Window {
    pub fn new(minecraft: &GlobalRef) -> anyhow::Result<Window> {
        mapping().in_frame(|| {
            let window_obj = mapping()
                .call_method(
                    MinecraftClassType::Minecraft,
                    minecraft.as_obj(),
                    "getWindow",
                    &[],
                )?
                .l()?;
            Ok(Window {
                jni_ref: mapping().new_global_ref(window_obj)?,
            })
        })
    }

     
     
    pub fn get_window(&self) -> anyhow::Result<jlong> {
        Ok(self.call_method("handle", &[])?.j()?)
    }

     
     
     
     
     
     
     
     
    pub fn gui_scale(&self) -> anyhow::Result<f64> {
        let value = self.call_method("getGuiScale", &[])?;
        match value {
            JValueOwned::Int(scale) => Ok(f64::from(scale)),
            JValueOwned::Double(scale) => Ok(scale),
            JValueOwned::Float(scale) => Ok(f64::from(scale)),
            _ => anyhow::bail!("Window.getGuiScale returned an unexpected type"),
        }
    }

     
     
    pub fn gui_scaled_width(&self) -> anyhow::Result<i32> {
        Ok(self.call_method("getGuiScaledWidth", &[])?.i()?)
    }

     
    pub fn gui_scaled_height(&self) -> anyhow::Result<i32> {
        Ok(self.call_method("getGuiScaledHeight", &[])?.i()?)
    }
}
