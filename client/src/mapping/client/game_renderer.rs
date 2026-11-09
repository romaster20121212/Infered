 

use crate::mapping::client::camera::Camera;
use crate::mapping::MappedObject;
use crate::state::mapping;
use jni::objects::GlobalRef;

use jni::objects::JValue;

 
#[derive(Debug, MappedObject)]
#[mapped(class = GameRenderer)]
pub struct GameRenderer {
    jni_ref: GlobalRef,
}

impl GameRenderer {
     
    pub fn new(jni_ref: GlobalRef) -> GameRenderer {
        GameRenderer { jni_ref }
    }

     
    pub fn get_main_camera(&self) -> anyhow::Result<Camera> {
        self.in_frame(|| {
            let camera = self.call_method("getMainCamera", &[])?.l()?;
            Ok(Camera::new(mapping().new_global_ref(camera)?))
        })
    }

     
     
     
     
     
     
     
     
     
    pub fn get_fov(
        &self,
        camera: &Camera,
        partial_tick: f32,
        use_fov_setting: bool,
    ) -> anyhow::Result<f32> {
        self.in_frame(|| {
            let fov = self
                .call_method(
                    "getFov",
                    &[
                        JValue::Object(camera.jni_ref().as_obj()),
                        JValue::Float(partial_tick),
                        JValue::Bool(use_fov_setting as u8),
                    ],
                )?
                .f()?;
            Ok(fov)
        })
    }
}
