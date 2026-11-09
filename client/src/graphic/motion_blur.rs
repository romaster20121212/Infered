 
 
 
 
 
 
 
 
 
 

use super::smoke::GlGuard;
use crate::gl;
use crate::module::{ModuleData, ModuleId};
use crate::state::client;
use anyhow::bail;
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub(super) const VERTEX: &str = r#"#version 150
out vec2 v_uv;
void main() {
    // One oversized triangle covers the whole viewport; no vertex buffer needed.
    vec2 p = vec2(-1.0) + vec2(float(gl_VertexID == 1) * 4.0, float(gl_VertexID == 2) * 4.0);
    v_uv = p * 0.5 + 0.5;
    gl_Position = vec4(p, 0.0, 1.0);
}
"#;

const FRAGMENT: &str = r#"#version 150
uniform sampler2D u_history;
uniform vec2 u_shift;
uniform float u_alpha;
in vec2 v_uv;
out vec4 frag;
void main() {
    frag = vec4(texture(u_history, v_uv + u_shift).rgb, u_alpha);
}
"#;

 
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
     
    pub strength: f32,
     
    pub smear: f32,
     
    pub angle: f32,
}

impl Config {
    fn read(data: &ModuleData) -> Self {
        let slider = |name: &str, fallback: f32| {
            data.get_setting(name)
                .and_then(|setting| setting.get_slider_value())
                .filter(|value| value.is_finite())
                .unwrap_or(fallback)
        };
        Self {
            strength: slider("Strength", 0.55).clamp(0.0, 0.95),
            smear: slider("Smear", 4.0).clamp(0.0, 32.0),
            angle: slider("Direction", 0.0),
        }
    }
}

struct Gpu {
    program: u32,
    vao: u32,
    texture: u32,
    sampler_loc: i32,
    shift_loc: i32,
    alpha_loc: i32,
    width: i32,
    height: i32,
     
    primed: bool,
}

 
static GPU: Mutex<Option<Gpu>> = Mutex::new(None);
static FAILED: AtomicBool = AtomicBool::new(false);

 
fn config() -> Option<Config> {
    let handle = client().modules.get(ModuleId::MotionBlur)?;
    let module = handle.try_lock().ok()?;
    let data = module.get_module_data();
    data.enabled.then(|| Config::read(data))
}

 
 
 
pub(crate) unsafe fn apply() {
    if FAILED.load(Ordering::Relaxed) {
        return;
    }
    let mut slot = GPU.lock().unwrap_or_else(|error| error.into_inner());
    let cfg = if client().modules.is_active() {
        config()
    } else {
        None
    };
    let Some(cfg) = cfg else {
         
        if let Some(gpu) = slot.as_mut() {
            gpu.primed = false;
        }
        return;
    };

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
                log::warn!("[MotionBlur] disabled: {error:#}");
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

    if gpu.primed {
        let angle = cfg.angle.to_radians();
        let shift_x = angle.cos() * cfg.smear / width as f32;
        let shift_y = angle.sin() * cfg.smear / height as f32;

        gl::UseProgram(gpu.program);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, gpu.texture);
        gl::Uniform1i(gpu.sampler_loc, 0);
        gl::Uniform2f(gpu.shift_loc, shift_x, shift_y);
        gl::Uniform1f(gpu.alpha_loc, cfg.strength);

        gl::Enable(gl::BLEND);
         
        gl::BlendFuncSeparate(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA, gl::ZERO, gl::ONE);
        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::SCISSOR_TEST);
        gl::Disable(gl::STENCIL_TEST);
        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);

        gl::BindVertexArray(gpu.vao);
        gl::DrawArrays(gl::TRIANGLES, 0, 3);
        gl::BindVertexArray(0);
    }

     
    gl::BindTexture(gl::TEXTURE_2D, gpu.texture);
    gl::CopyTexSubImage2D(gl::TEXTURE_2D, 0, 0, 0, vx, vy, width, height);
    gpu.primed = true;
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
            bail!("motion blur program failed to link");
        }

        let sampler_loc = uniform(program, "u_history")?;
        let shift_loc = uniform(program, "u_shift")?;
        let alpha_loc = uniform(program, "u_alpha")?;

         
         
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
            shift_loc,
            alpha_loc,
            width,
            height,
            primed: false,
        })
    }

     
     
    unsafe fn resize(&mut self, width: i32, height: i32) {
        gl::BindTexture(gl::TEXTURE_2D, self.texture);
        allocate(self.texture, width, height);
        self.width = width;
        self.height = height;
        self.primed = false;
    }
}

pub(super) unsafe fn allocate(texture: u32, width: i32, height: i32) {
    gl::BindTexture(gl::TEXTURE_2D, texture);
    gl::TexImage2D(
        gl::TEXTURE_2D,
        0,
        gl::RGBA8 as i32,
        width,
        height,
        0,
        gl::RGBA,
        gl::UNSIGNED_BYTE,
        std::ptr::null(),
    );
}

pub(super) unsafe fn compile(kind: u32, source: &str) -> anyhow::Result<u32> {
    let shader = gl::CreateShader(kind);
    let text = CString::new(source)?;
    gl::ShaderSource(shader, 1, &text.as_ptr(), std::ptr::null());
    gl::CompileShader(shader);
    let mut compiled = 0;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut compiled);
    if compiled == 0 {
        gl::DeleteShader(shader);
        bail!("post-process shader failed to compile");
    }
    Ok(shader)
}

pub(super) unsafe fn uniform(program: u32, name: &str) -> anyhow::Result<i32> {
    let name_c = CString::new(name)?;
    let location = gl::GetUniformLocation(program, name_c.as_ptr());
    if location < 0 {
        bail!("post-process shader has no {name} uniform");
    }
    Ok(location)
}
