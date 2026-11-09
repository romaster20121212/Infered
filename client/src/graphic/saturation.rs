 
 
 
 
 
 
 
 

use super::motion_blur::{allocate, compile, uniform, VERTEX};
use super::smoke::GlGuard;
use crate::gl;
use crate::module::{ModuleData, ModuleId};
use crate::state::client;
use anyhow::bail;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

const FRAGMENT: &str = r#"#version 150
uniform sampler2D u_frame;
uniform float u_saturation;
in vec2 v_uv;
out vec4 frag;
void main() {
    vec4 c = texture(u_frame, v_uv);
    float luma = dot(c.rgb, vec3(0.2126, 0.7152, 0.0722));
    frag = vec4(clamp(mix(vec3(luma), c.rgb, u_saturation), 0.0, 1.0), c.a);
}
"#;

 
const NEUTRAL: f32 = 1.0;

struct Gpu {
    program: u32,
    vao: u32,
    texture: u32,
    sampler_loc: i32,
    saturation_loc: i32,
    width: i32,
    height: i32,
}

 
static GPU: Mutex<Option<Gpu>> = Mutex::new(None);
static FAILED: AtomicBool = AtomicBool::new(false);

 
fn saturation() -> Option<f32> {
    let handle = client().modules.get(ModuleId::Saturation)?;
    let module = handle.try_lock().ok()?;
    let data = module.get_module_data();
    data.enabled.then(|| read(data))
}

fn read(data: &ModuleData) -> f32 {
    data.get_setting("Saturation")
        .and_then(|setting| setting.get_slider_value())
        .filter(|value| value.is_finite())
        .unwrap_or(NEUTRAL)
        .clamp(0.0, 2.0)
}

 
 
pub(crate) unsafe fn apply() {
    if FAILED.load(Ordering::Relaxed) {
        return;
    }
    let factor = if client().modules.is_active() {
        saturation()
    } else {
        None
    };
    let Some(factor) = factor else {
        return;
    };
     
    if (factor - NEUTRAL).abs() < 1e-4 {
        return;
    }

    let mut slot = GPU.lock().unwrap_or_else(|error| error.into_inner());
    let _guard = GlGuard::capture();
    let mut viewport = [0i32; 4];
    gl::GetIntegerv(gl::VIEWPORT, viewport.as_mut_ptr());
    let [vx, vy, width, height] = viewport;
    if width <= 0 || height <= 0 {
        return;
    }

    let gpu = match slot.as_mut() {
        Some(gpu) => gpu,
        None => match Gpu::create(width, height) {
            Ok(gpu) => slot.insert(gpu),
            Err(error) => {
                log::warn!("[Saturation] disabled: {error:#}");
                FAILED.store(true, Ordering::Relaxed);
                return;
            }
        },
    };
    if gpu.width != width || gpu.height != height {
        gpu.resize(width, height);
    }

    let mut draw_fbo = 0i32;
    gl::GetIntegerv(gl::DRAW_FRAMEBUFFER_BINDING, &mut draw_fbo);
     
    gl::BindFramebuffer(gl::READ_FRAMEBUFFER, draw_fbo as u32);
    gl::BindTexture(gl::TEXTURE_2D, gpu.texture);
    gl::CopyTexSubImage2D(gl::TEXTURE_2D, 0, 0, 0, vx, vy, width, height);

    gl::UseProgram(gpu.program);
    gl::ActiveTexture(gl::TEXTURE0);
    gl::BindTexture(gl::TEXTURE_2D, gpu.texture);
    gl::Uniform1i(gpu.sampler_loc, 0);
    gl::Uniform1f(gpu.saturation_loc, factor);

     
    gl::Disable(gl::BLEND);
    gl::Disable(gl::DEPTH_TEST);
    gl::Disable(gl::CULL_FACE);
    gl::Disable(gl::SCISSOR_TEST);
    gl::Disable(gl::STENCIL_TEST);
    gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);

    gl::BindVertexArray(gpu.vao);
    gl::DrawArrays(gl::TRIANGLES, 0, 3);
    gl::BindVertexArray(0);
}

impl Gpu {
    unsafe fn create(width: i32, height: i32) -> anyhow::Result<Self> {
        let vertex = compile(gl::VERTEX_SHADER, VERTEX)?;
        let fragment = compile(gl::FRAGMENT_SHADER, FRAGMENT)?;
        let program = gl::CreateProgram();
        gl::AttachShader(program, vertex);
        gl::AttachShader(program, fragment);
        gl::LinkProgram(program);
        gl::DeleteShader(vertex);
        gl::DeleteShader(fragment);
        let mut linked = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut linked);
        if linked == 0 {
            gl::DeleteProgram(program);
            bail!("saturation program failed to link");
        }

        let sampler_loc = uniform(program, "u_frame")?;
        let saturation_loc = uniform(program, "u_saturation")?;

         
         
        let mut vao = 0;
        gl::GenVertexArrays(1, &mut vao);

        let mut texture = 0;
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        allocate(texture, width, height);

        Ok(Self {
            program,
            vao,
            texture,
            sampler_loc,
            saturation_loc,
            width,
            height,
        })
    }

     
    unsafe fn resize(&mut self, width: i32, height: i32) {
        allocate(self.texture, width, height);
        self.width = width;
        self.height = height;
    }
}
