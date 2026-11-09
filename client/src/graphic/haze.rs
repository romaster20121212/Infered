 
 
 
 
 
 
 
 
 
 
 
 

use super::smoke::{inverse, matrix, GlGuard};
use crate::gl;
use crate::mapping::MappedObject;
use crate::state::{mapping, minecraft};
use anyhow::{ensure, Context};
use jni::objects::{JClass, JObject, JValue};
use std::ffi::CString;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

 
pub const MOTE_COUNT: usize = 24;

 
 
const SHADER_NAME: &str = "haze";

const VERTEX: &str = r#"#version 150
in vec3 Position;
void main() {
    gl_Position = vec4(Position.xy, 1.0, 1.0);
}
"#;

 
 
const FRAGMENT: &str = r#"#version 150
uniform sampler2D u_depth;
uniform vec2 u_viewport_origin;
uniform vec2 u_resolution;
uniform mat4 u_inv_proj;
uniform mat4 u_inv_view;
uniform float u_density;
uniform float u_darken;
uniform float u_glow;
uniform vec3 u_fog_color;
uniform vec3 u_mote_color;
uniform int u_mote_count;
uniform vec4 u_motes[24];
out vec4 fragColor;

void main() {
    vec2 uv = (gl_FragCoord.xy - u_viewport_origin) / u_resolution;
    float raw = texture(u_depth, uv).r;

    // Rebuild this pixel's camera-relative position from its depth.
    vec4 ndc = vec4(uv * 2.0 - 1.0, raw * 2.0 - 1.0, 1.0);
    vec4 view_pos = u_inv_proj * ndc;
    view_pos /= view_pos.w;
    vec3 world = (u_inv_view * view_pos).xyz;

    // Sky pixels have no world depth. They only take the darkening, never the
    // fog, so the sky itself stays readable.
    if (raw >= 1.0) {
        fragColor = vec4(0.0, 0.0, 0.0, u_darken);
        return;
    }
    float dist = length(world);

    // Thin near the camera, dense far away.
    float fog = 1.0 - exp(-dist * u_density);

    // Each mote adds a soft halo. The fog picks up that light within the halo,
    // weighted by how much fog lies between the camera and this point.
    vec3 glow = vec3(0.0);
    for (int i = 0; i < u_mote_count; i++) {
        vec3 mote = u_motes[i].xyz;
        float radius = u_motes[i].w;
        float d = distance(world, mote);
        glow += u_mote_color * exp(-(d * d) / (radius * radius + 0.0001));
    }
    glow *= u_glow * fog;

    float alpha = clamp(u_darken + fog * (1.0 - u_darken), 0.0, 1.0);
    vec3 rgb = u_fog_color * fog * (1.0 - u_darken) + glow;
    fragColor = vec4(rgb, alpha);
}
"#;

 
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mote {
    pub pos: [f32; 3],
    pub radius: f32,
}

 
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HazeStyle {
    pub density: f32,
    pub darken: f32,
    pub glow: f32,
    pub fog_color: [f32; 3],
    pub mote_color: [f32; 3],
}

static STYLE: Mutex<Option<HazeStyle>> = Mutex::new(None);
static CLOCK: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
static GPU: Mutex<Option<Gpu>> = Mutex::new(None);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
pub(crate) fn set_style(style: HazeStyle) {
    *lock(&STYLE) = Some(style);
}

 
pub(crate) fn clear_style() {
    *lock(&STYLE) = None;
}

 
 
fn mote_offsets() -> [[f32; 3]; MOTE_COUNT] {
    let mut out = [[0.0; 3]; MOTE_COUNT];
    for (i, slot) in out.iter_mut().enumerate() {
        let f = i as f32 + 1.0;
        let x = (f * 12.9898).sin() * 43758.5453;
        let y = (f * 78.233).sin() * 12345.6789;
        let z = (f * 37.719).sin() * 9876.5432;
         
         
        *slot = [
            x.rem_euclid(1.0) * 2.0 - 1.0,
            y.rem_euclid(1.0) * 2.0 - 1.0,
            z.rem_euclid(1.0) * 2.0 - 1.0,
        ];
    }
    out
}

 
 
 
pub fn mote_positions(time: f32) -> [Mote; MOTE_COUNT] {
    let mut out = [Mote {
        pos: [0.0; 3],
        radius: 0.0,
    }; MOTE_COUNT];
    for (i, (slot, o)) in out.iter_mut().zip(mote_offsets()).enumerate() {
        let phase = i as f32 * 0.7;
        let drift = [
            (time * 0.35 + phase).sin() * 0.6,
            (time * 0.25 + phase * 1.3).sin() * 0.4,
            (time * 0.3 + phase * 0.9).cos() * 0.6,
        ];
        let len = (o[0] * o[0] + o[1] * o[1] + o[2] * o[2]).sqrt().max(0.0001);
        let dist = 4.0 + (o[0].abs() + o[1].abs() + o[2].abs()) * 4.0;
        slot.pos = [
            o[0] / len * dist + drift[0],
            o[1] / len * dist + drift[1],
            o[2] / len * dist + drift[2],
        ];
        slot.radius = 2.0 + (phase.sin() * 0.5 + 0.5);
    }
    out
}

 
 
 
unsafe fn load_program() -> anyhow::Result<u32> {
    let bridge = crate::camera_bridge::smoke_bridge().context("Smoke bridge unavailable")?;
    let mut env = mapping().get_env()?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    let name = env.new_string(SHADER_NAME)?;
    let vertex = env.new_string(VERTEX)?;
    let fragment = env.new_string(FRAGMENT)?;
    let result = env.call_static_method(
        class,
        "reloadShader",
        "(Ljava/lang/Object;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        &[
            JValue::Object(minecraft().jni_ref().as_obj()),
            JValue::Object(&name),
            JValue::Object(&vertex),
            JValue::Object(&fragment),
        ],
    );
    let program = match result {
        Ok(value) => value.i()? as u32,
        Err(error) => {
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            anyhow::bail!("ShaderLoader.reloadShader failed for haze: {error}");
        }
    };
    ensure!(
        program != 0 && gl::IsProgram(program) != 0,
        "ShaderLoader returned an invalid haze program"
    );
    Ok(program)
}

 
struct Gpu {
    program: u32,
    vao: u32,
    vbo: u32,
    depth: u32,
    dimensions: [i32; 2],
    uniforms: [i32; 12],
}

 
const U_DEPTH: usize = 0;
const U_VIEWPORT_ORIGIN: usize = 1;
const U_RESOLUTION: usize = 2;
const U_INV_PROJ: usize = 3;
const U_INV_VIEW: usize = 4;
const U_DENSITY: usize = 5;
const U_DARKEN: usize = 6;
const U_GLOW: usize = 7;
const U_FOG_COLOR: usize = 8;
const U_MOTE_COLOR: usize = 9;
const U_MOTE_COUNT: usize = 10;
const U_MOTES: usize = 11;

const UNIFORM_NAMES: [&str; 12] = [
    "u_depth",
    "u_viewport_origin",
    "u_resolution",
    "u_inv_proj",
    "u_inv_view",
    "u_density",
    "u_darken",
    "u_glow",
    "u_fog_color",
    "u_mote_color",
    "u_mote_count",
    "u_motes[0]",
];

impl Gpu {
     
     
    unsafe fn is_live(&self) -> bool {
        if gl::IsProgram(self.program) == 0 {
            return false;
        }
        let name = CString::new("u_motes[0]").expect("static uniform name");
        gl::GetUniformLocation(self.program, name.as_ptr()) == self.uniforms[U_MOTES]
    }

    unsafe fn destroy(self) {
        gl::DeleteVertexArrays(1, &self.vao);
        gl::DeleteBuffers(1, &self.vbo);
        gl::DeleteTextures(1, &self.depth);
    }

    unsafe fn create() -> anyhow::Result<Self> {
        let program = load_program()?;
        let mut uniforms = [0_i32; 12];
        for (slot, name) in uniforms.iter_mut().zip(UNIFORM_NAMES) {
            let c_name = CString::new(name)?;
            *slot = gl::GetUniformLocation(program, c_name.as_ptr());
            ensure!(*slot >= 0, "haze shader has no {name} uniform");
        }
        let position_name = CString::new("Position")?;
        let position = gl::GetAttribLocation(program, position_name.as_ptr());
        ensure!(position >= 0, "haze shader has no Position attribute");

        let mut vao = 0;
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);
        let mut vbo = 0;
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        let quad: [f32; 18] = [
            -1., -1., 0., 1., -1., 0., 1., 1., 0., -1., -1., 0., 1., 1., 0., -1., 1., 0.,
        ];
        gl::BufferData(
            gl::ARRAY_BUFFER,
            std::mem::size_of_val(&quad) as isize,
            quad.as_ptr().cast(),
            gl::STATIC_DRAW,
        );
        gl::VertexAttribPointer(
            position as u32,
            3,
            gl::FLOAT,
            gl::FALSE,
            0,
            std::ptr::null(),
        );
        gl::EnableVertexAttribArray(position as u32);
        gl::BindVertexArray(0);

        let mut depth = 0;
        gl::GenTextures(1, &mut depth);

        Ok(Self {
            program,
            vao,
            vbo,
            depth,
            dimensions: [0, 0],
            uniforms,
        })
    }
}

 
 
 
pub(crate) fn render_world(view: &JObject, projection: &JObject) -> anyhow::Result<()> {
    let style = { *lock(&STYLE) };
    let Some(style) = style else {
        return Ok(());
    };
    let view = matrix(view)?;
    let projection = matrix(projection)?;
    let inv_proj = inverse(&projection).context("singular projection matrix")?;
    let inv_view = inverse(&view).context("singular view matrix")?;

    let time = CLOCK.get_or_init(Instant::now).elapsed().as_secs_f32();
    let motes = mote_positions(time);

    let mut gpu = lock(&GPU);
    unsafe {
         
        if !gl::GetIntegerv::is_loaded() {
            gl::load_with(super::platform::gl_proc_address);
        }
        let guard = GlGuard::capture();
        if !gpu.as_ref().is_some_and(|gpu| gpu.is_live()) {
            if let Some(stale) = gpu.take() {
                stale.destroy();
            }
            *gpu = Some(Gpu::create()?);
        }
        let gpu = gpu.as_mut().context("haze GPU state missing")?;
        gpu.draw(&style, &motes, &inv_proj, &inv_view, &guard)?;
    }
    Ok(())
}

impl Gpu {
     
     
    unsafe fn draw(
        &mut self,
        style: &HazeStyle,
        motes: &[Mote; MOTE_COUNT],
        inv_proj: &[f32; 16],
        inv_view: &[f32; 16],
        guard: &GlGuard,
    ) -> anyhow::Result<()> {
        let [x, y, w, h] = guard.viewport();
        ensure!(
            w > 0 && h > 0 && guard.draw_fbo() != 0,
            "world framebuffer unavailable"
        );
        let mut samples = 0;
        gl::GetIntegerv(gl::SAMPLES, &mut samples);
        ensure!(
            samples == 0,
            "multisampled world depth is not supported by haze"
        );

         
         
        gl::BindFramebuffer(gl::READ_FRAMEBUFFER, guard.draw_fbo() as u32);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, self.depth);
        if self.dimensions != [w, h] {
            gl::CopyTexImage2D(gl::TEXTURE_2D, 0, gl::DEPTH_COMPONENT24, x, y, w, h, 0);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            self.dimensions = [w, h];
        } else {
            gl::CopyTexSubImage2D(gl::TEXTURE_2D, 0, 0, 0, x, y, w, h);
        }

        gl::UseProgram(self.program);
        let u = self.uniforms;
        gl::Uniform1i(u[U_DEPTH], 0);
        gl::Uniform2f(u[U_VIEWPORT_ORIGIN], x as f32, y as f32);
        gl::Uniform2f(u[U_RESOLUTION], w as f32, h as f32);
        gl::UniformMatrix4fv(u[U_INV_PROJ], 1, gl::FALSE, inv_proj.as_ptr());
        gl::UniformMatrix4fv(u[U_INV_VIEW], 1, gl::FALSE, inv_view.as_ptr());
        gl::Uniform1f(u[U_DENSITY], style.density);
        gl::Uniform1f(u[U_DARKEN], style.darken);
        gl::Uniform1f(u[U_GLOW], style.glow);
        let [fr, fg, fb] = style.fog_color;
        gl::Uniform3f(u[U_FOG_COLOR], fr, fg, fb);
        let [mr, mg, mb] = style.mote_color;
        gl::Uniform3f(u[U_MOTE_COLOR], mr, mg, mb);
        gl::Uniform1i(u[U_MOTE_COUNT], MOTE_COUNT as i32);
        let packed: [[f32; 4]; MOTE_COUNT] =
            motes.map(|m| [m.pos[0], m.pos[1], m.pos[2], m.radius]);
        gl::Uniform4fv(u[U_MOTES], MOTE_COUNT as i32, packed.as_ptr().cast());

         
         
        gl::Disable(gl::DEPTH_TEST);
        gl::DepthMask(gl::FALSE);
        gl::Enable(gl::BLEND);
        gl::BlendFuncSeparate(gl::ONE, gl::ONE_MINUS_SRC_ALPHA, gl::ZERO, gl::ONE);
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::SCISSOR_TEST);
        gl::Disable(gl::STENCIL_TEST);

        gl::BindVertexArray(self.vao);
        gl::DrawArrays(gl::TRIANGLES, 0, 6);
        gl::BindVertexArray(0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mote_positions_stay_in_shell() {
        for (i, mote) in mote_positions(1.5).iter().enumerate() {
            let d = (mote.pos[0].powi(2) + mote.pos[1].powi(2) + mote.pos[2].powi(2)).sqrt();
            assert!(d > 2.0 && d < 24.0, "mote {i} at distance {d}");
            assert!(mote.radius > 0.0, "mote {i} has no light radius");
        }
    }

    #[test]
    fn style_round_trips() {
        let style = HazeStyle {
            density: 0.02,
            darken: 0.1,
            glow: 1.0,
            fog_color: [0.8, 0.8, 0.9],
            mote_color: [1.0, 0.7, 0.3],
        };
        set_style(style);
        assert_eq!(*lock(&STYLE), Some(style));
        clear_style();
        assert_eq!(*lock(&STYLE), None);
    }
}
