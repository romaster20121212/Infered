 
 
 
 
 
 
 
 
 
 
 
 
 
 

use super::chams_warped::{self, Settings};
use super::chams_warped_shader as shader;
use crate::gl;
use crate::mapping::MappedObject;
use crate::state::mapping;
use anyhow::{ensure, Context};
use jni::objects::{JClass, JValue};
use std::ffi::CString;

 
 
 
 
const MASK_UNIT: u32 = gl::TEXTURE4;
const GLOW_UNIT: u32 = gl::TEXTURE5;
const SCENE_UNIT: u32 = gl::TEXTURE6;
const MASK_SAMPLER: i32 = 4;
const GLOW_SAMPLER: i32 = 5;
const SCENE_SAMPLER: i32 = 6;

 
#[derive(Default)]
struct Target {
    fbo: u32,
    texture: u32,
    width: i32,
    height: i32,
}

impl Target {
     
     
    unsafe fn ensure(&mut self, width: i32, height: i32) -> anyhow::Result<()> {
        if self.fbo != 0 && self.width == width && self.height == height {
            return Ok(());
        }
        self.release();
        gl::GenTextures(1, &mut self.texture);
        gl::BindTexture(gl::TEXTURE_2D, self.texture);
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
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::GenFramebuffers(1, &mut self.fbo);
        gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            self.texture,
            0,
        );
        let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
        ensure!(
            status == gl::FRAMEBUFFER_COMPLETE,
            "Outline Glow target {width}x{height} is incomplete (0x{status:04X})"
        );
        self.width = width;
        self.height = height;
        Ok(())
    }

    unsafe fn release(&mut self) {
        if self.fbo != 0 {
            gl::DeleteFramebuffers(1, &self.fbo);
        }
        if self.texture != 0 {
            gl::DeleteTextures(1, &self.texture);
        }
        *self = Target::default();
    }
}

fn uniform(program: u32, name: &str) -> i32 {
    let Ok(name) = CString::new(name) else {
        return -1;
    };
    unsafe { gl::GetUniformLocation(program, name.as_ptr()) }
}

 
 
 
 
 
 
 
unsafe fn load(
    env: &mut jni::JNIEnv,
    name: &str,
    vertex: &str,
    fragment: &str,
) -> anyhow::Result<u32> {
    let bridge = crate::camera_bridge::smoke_bridge().context("Outline Glow bridge unavailable")?;
    let class = JClass::from(env.new_local_ref(bridge.as_obj())?);
    let key = env.new_string(name)?;
    let vertex = env.new_string(vertex)?;
    let fragment = env.new_string(fragment)?;
    let result = env.call_static_method(
        class,
        "reloadShader",
        "(Ljava/lang/Object;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        &[
            JValue::Object(crate::state::minecraft().jni_ref().as_obj()),
            JValue::Object(&key),
            JValue::Object(&vertex),
            JValue::Object(&fragment),
        ],
    );
    let program = match result {
        Ok(value) => value.i()? as u32,
        Err(error) => {
            let detail = super::smoke::java_exception(env);
            anyhow::bail!("{name} shader failed to load: {error}; {detail}");
        }
    };
    ensure!(
        program != 0 && gl::IsProgram(program) != 0,
        "{name} shader returned an invalid GL program"
    );
    Ok(program)
}

 
pub(crate) struct Frame<'a> {
    pub world_fbo: i32,
    pub viewport: [i32; 4],
     
    pub depth: u32,
    pub view: &'a [f32; 16],
    pub projection: &'a [f32; 16],
    pub inverse: &'a [f32; 16],
}

pub(crate) struct WarpedPass {
    mask_program: u32,
    mask_position: u32,
    mask_view: i32,
    mask_projection: i32,
    mask_tint: i32,
    mask_occlude: i32,
    mask_depth: i32,
    mask_inverse: i32,
    mask_viewport: i32,
    mask_vao: u32,
    mask_vbo: u32,
    down_program: u32,
    down_mask: i32,
    down_texel: i32,
    down_spread: i32,
    blur_program: u32,
    blur_source: i32,
    blur_direction: i32,
    blur_sigma: i32,
    composite_program: u32,
    composite: CompositeUniforms,
    quad_vao: u32,
    mask: Target,
    glow_a: Target,
    glow_b: Target,
    scene: Target,
}

 
 
 
struct CompositeUniforms {
    mask: i32,
    glow: i32,
    scene: i32,
    texel: i32,
    glow_texel: i32,
    px_scale: i32,
    time: i32,
    outline: i32,
    intensity: i32,
    fill: i32,
    tint_amount: i32,
    aspect: i32,
    accent: i32,
    accent2: i32,
    thin_inner: i32,
    line_count: i32,
    refraction: i32,
    dispersion: i32,
    thickness: i32,
    bass: i32,
    beat_age: i32,
    flash: i32,
    mode: i32,
}

impl CompositeUniforms {
    unsafe fn new(program: u32) -> Self {
        Self {
            mask: uniform(program, "Mask"),
            glow: uniform(program, "Glow"),
            scene: uniform(program, "Scene"),
            texel: uniform(program, "texel"),
            glow_texel: uniform(program, "glowTexel"),
            px_scale: uniform(program, "pxScale"),
            time: uniform(program, "time"),
            outline: uniform(program, "outline"),
            intensity: uniform(program, "intensity"),
            fill: uniform(program, "fill"),
            tint_amount: uniform(program, "tintAmount"),
            aspect: uniform(program, "aspect"),
            accent: uniform(program, "accent"),
            accent2: uniform(program, "accent2"),
            thin_inner: uniform(program, "thinInner"),
            line_count: uniform(program, "lineCount"),
            refraction: uniform(program, "refraction"),
            dispersion: uniform(program, "dispersion"),
            thickness: uniform(program, "thickness"),
            bass: uniform(program, "bass"),
            beat_age: uniform(program, "beatAge[0]"),
            flash: uniform(program, "flash"),
            mode: uniform(program, "Mode"),
        }
    }
}

 
unsafe fn set1f(location: i32, value: f32) {
    if location >= 0 {
        gl::Uniform1f(location, value);
    }
}

unsafe fn set1i(location: i32, value: i32) {
    if location >= 0 {
        gl::Uniform1i(location, value);
    }
}

unsafe fn set2f(location: i32, x: f32, y: f32) {
    if location >= 0 {
        gl::Uniform2f(location, x, y);
    }
}

unsafe fn set3f(location: i32, value: [f32; 3]) {
    if location >= 0 {
        gl::Uniform3f(location, value[0], value[1], value[2]);
    }
}

impl WarpedPass {
    pub(crate) unsafe fn create() -> anyhow::Result<Self> {
        let mut env = mapping().get_env()?;
        let mask_program = load(
            &mut env,
            "chams_outline_mask",
            shader::MASK_VERTEX,
            shader::MASK_FRAGMENT,
        )?;
        let down_program = load(
            &mut env,
            "chams_outline_down",
            shader::POST_VERTEX,
            shader::DOWN_FRAGMENT,
        )?;
        let blur_program = load(
            &mut env,
            "chams_outline_blur",
            shader::POST_VERTEX,
            shader::BLUR_FRAGMENT,
        )?;
        let composite_program = load(
            &mut env,
            "chams_outline_glow",
            shader::POST_VERTEX,
            shader::COMPOSITE_FRAGMENT,
        )?;

        let position = gl::GetAttribLocation(mask_program, c"Position".as_ptr());
        ensure!(position >= 0, "Outline Glow mask has no Position attribute");

        let mut mask_vao = 0;
        let mut mask_vbo = 0;
        gl::GenVertexArrays(1, &mut mask_vao);
        gl::GenBuffers(1, &mut mask_vbo);
         
         
        let mut quad_vao = 0;
        gl::GenVertexArrays(1, &mut quad_vao);

        log::info!("[Chams] Outline Glow shaders ready (mask, down, blur, 11 looks)");
        Ok(Self {
            mask_program,
            mask_position: position as u32,
            mask_view: uniform(mask_program, "View"),
            mask_projection: uniform(mask_program, "Projection"),
            mask_tint: uniform(mask_program, "Tint"),
            mask_occlude: uniform(mask_program, "Occlude"),
            mask_depth: uniform(mask_program, "SceneDepth"),
            mask_inverse: uniform(mask_program, "InverseProjection"),
            mask_viewport: uniform(mask_program, "Viewport"),
            mask_vao,
            mask_vbo,
            down_program,
            down_mask: uniform(down_program, "Mask"),
            down_texel: uniform(down_program, "srcTexel"),
            down_spread: uniform(down_program, "spread"),
            blur_program,
            blur_source: uniform(blur_program, "Source"),
            blur_direction: uniform(blur_program, "direction"),
            blur_sigma: uniform(blur_program, "sigma"),
            composite_program,
            composite: CompositeUniforms::new(composite_program),
            quad_vao,
            mask: Target::default(),
            glow_a: Target::default(),
            glow_b: Target::default(),
            scene: Target::default(),
        })
    }

     
     
     
    pub(crate) unsafe fn release(&mut self) {
        for target in [
            &mut self.mask,
            &mut self.glow_a,
            &mut self.glow_b,
            &mut self.scene,
        ] {
            target.release();
        }
        for array in [&mut self.mask_vao, &mut self.quad_vao] {
            if *array != 0 {
                gl::DeleteVertexArrays(1, array);
                *array = 0;
            }
        }
        if self.mask_vbo != 0 {
            gl::DeleteBuffers(1, &self.mask_vbo);
            self.mask_vbo = 0;
        }
    }

     
     
     
    pub(crate) unsafe fn render(
        &mut self,
        frame: &Frame,
        vertices: &[f32],
        settings: &Settings,
        look: &super::chams::Config,
        time: f32,
    ) -> anyhow::Result<()> {
        let previous = Saved::capture();
        let result = self.run(frame, vertices, settings, look, time);
        previous.restore();
        result
    }

    #[allow(clippy::too_many_lines)]
    unsafe fn run(
        &mut self,
        frame: &Frame,
        vertices: &[f32],
        settings: &Settings,
        look: &super::chams::Config,
        time: f32,
    ) -> anyhow::Result<()> {
        let [vx, vy, vw, vh] = frame.viewport;
        ensure!(vw > 0 && vh > 0, "Outline Glow has no viewport");
        let px = vh as f32 / chams_warped::REFERENCE_HEIGHT;
        let sigma = chams_warped::blur_sigma(settings, px);
        let div = chams_warped::glow_divisor(sigma);
        let glow_width = (vw / div).max(1);
        let glow_height = (vh / div).max(1);

        self.mask.ensure(vw, vh)?;
        self.glow_a.ensure(glow_width, glow_height)?;
        self.glow_b.ensure(glow_width, glow_height)?;
        let wants_scene = chams_warped::wants_scene(settings.mode);
        let scene_div = chams_warped::scene_divisor(settings);
        let scene_width = (vw / scene_div).max(1);
        let scene_height = (vh / scene_div).max(1);
        if wants_scene {
             
             
             
             
            self.scene.ensure(scene_width, scene_height)?;
            gl::Disable(gl::SCISSOR_TEST);
            gl::BindFramebuffer(gl::READ_FRAMEBUFFER, frame.world_fbo as u32);
            gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.scene.fbo);
            gl::BlitFramebuffer(
                vx,
                vy,
                vx + vw,
                vy + vh,
                0,
                0,
                scene_width,
                scene_height,
                gl::COLOR_BUFFER_BIT,
                gl::LINEAR,
            );
        }

         
        gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.mask.fbo);
        gl::Viewport(0, 0, vw, vh);
        gl::Disable(gl::BLEND);
        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::SCISSOR_TEST);
        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        gl::ClearColor(0.0, 0.0, 0.0, 0.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
        gl::UseProgram(self.mask_program);
        gl::BindVertexArray(self.mask_vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, self.mask_vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            std::mem::size_of_val(vertices) as isize,
            vertices.as_ptr().cast(),
            gl::STREAM_DRAW,
        );
        gl::EnableVertexAttribArray(self.mask_position);
        gl::VertexAttribPointer(
            self.mask_position,
            3,
            gl::FLOAT,
            gl::FALSE,
            12,
            std::ptr::null(),
        );
        if self.mask_view >= 0 {
            gl::UniformMatrix4fv(self.mask_view, 1, gl::FALSE, frame.view.as_ptr());
        }
        if self.mask_projection >= 0 {
            gl::UniformMatrix4fv(
                self.mask_projection,
                1,
                gl::FALSE,
                frame.projection.as_ptr(),
            );
        }
        if self.mask_inverse >= 0 {
            gl::UniformMatrix4fv(self.mask_inverse, 1, gl::FALSE, frame.inverse.as_ptr());
        }
        set3f(
            self.mask_tint,
            [look.color[0], look.color[1], look.color[2]],
        );
         
        set1i(self.mask_occlude, if look.through { 0 } else { 1 });
        set1i(self.mask_depth, MASK_SAMPLER);
         
         
         
        if self.mask_viewport >= 0 {
            gl::Uniform4f(self.mask_viewport, 0.0, 0.0, vw as f32, vh as f32);
        }
        gl::ActiveTexture(MASK_UNIT);
        gl::BindTexture(gl::TEXTURE_2D, frame.depth);
        gl::DrawArrays(gl::TRIANGLES, 0, (vertices.len() / 3) as i32);
        gl::DisableVertexAttribArray(self.mask_position);

         
        gl::UseProgram(self.down_program);
        gl::BindVertexArray(self.quad_vao);
        gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.glow_a.fbo);
        gl::Viewport(0, 0, glow_width, glow_height);
        set1i(self.down_mask, MASK_SAMPLER);
        set2f(
            self.down_texel,
            1.0 / self.mask.width as f32,
            1.0 / self.mask.height as f32,
        );
        set1f(self.down_spread, if div == 2 { 0.5 } else { 1.0 });
        gl::ActiveTexture(MASK_UNIT);
        gl::BindTexture(gl::TEXTURE_2D, self.mask.texture);
        gl::DrawArrays(gl::TRIANGLES, 0, 3);

        let sigma = (sigma / div as f32).min(16.0);
        gl::UseProgram(self.blur_program);
        set1i(self.blur_source, MASK_SAMPLER);
        set1f(self.blur_sigma, sigma);
        gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.glow_b.fbo);
        gl::Viewport(0, 0, glow_width, glow_height);
        set2f(self.blur_direction, 1.0 / glow_width as f32, 0.0);
        gl::ActiveTexture(MASK_UNIT);
        gl::BindTexture(gl::TEXTURE_2D, self.glow_a.texture);
        gl::DrawArrays(gl::TRIANGLES, 0, 3);
        gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.glow_a.fbo);
        set2f(self.blur_direction, 0.0, 1.0 / glow_height as f32);
        gl::BindTexture(gl::TEXTURE_2D, self.glow_b.texture);
        gl::DrawArrays(gl::TRIANGLES, 0, 3);

         
        gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, frame.world_fbo as u32);
        gl::Viewport(vx, vy, vw, vh);
        gl::UseProgram(self.composite_program);
        gl::BindVertexArray(self.quad_vao);
        gl::Enable(gl::BLEND);
        gl::BlendEquationSeparate(gl::FUNC_ADD, gl::FUNC_ADD);
        gl::BlendFuncSeparate(
            gl::ONE,
            gl::ONE_MINUS_SRC_ALPHA,
            gl::ONE,
            gl::ONE_MINUS_SRC_ALPHA,
        );
        let u = &self.composite;
        set1i(u.mask, MASK_SAMPLER);
        set1i(u.glow, GLOW_SAMPLER);
        set1i(u.scene, SCENE_SAMPLER);
        set2f(u.texel, 1.0 / vw as f32, 1.0 / vh as f32);
        set2f(
            u.glow_texel,
            1.0 / glow_width as f32,
            1.0 / glow_height as f32,
        );
        set1f(u.px_scale, px);
        let speed = chams_warped::clamp(settings.speed, 0.0, 5.0);
        set1f(u.time, time * speed);
        set1f(u.outline, if settings.outline { 1.0 } else { 0.0 });
        set1f(
            u.intensity,
            chams_warped::clamp(settings.intensity, 0.0, 3.0),
        );
        set1f(u.fill, chams_warped::clamp01(settings.fill_opacity));
        set1f(u.tint_amount, chams_warped::clamp01(settings.tint_amount));
        set1f(u.aspect, vw as f32 / vh as f32);
        let tint = [look.color[0], look.color[1], look.color[2]];
        set3f(u.accent, tint);
         
         
         
        set3f(
            u.accent2,
            [
                tint[0] * 0.55 + 0.35,
                tint[1] * 0.55 + 0.25,
                tint[2] * 0.55 + 0.45,
            ],
        );
        set1f(u.thin_inner, if settings.hairline { 1.0 } else { 0.0 });
        set1f(u.line_count, chams_warped::clamp(settings.lines, 2.0, 16.0));
        set1f(
            u.refraction,
            chams_warped::clamp(settings.refraction, 0.0, 3.0),
        );
        set1f(u.dispersion, chams_warped::clamp01(settings.dispersion));
        set1f(
            u.thickness,
            chams_warped::clamp(settings.thickness, 4.0, 40.0),
        );
         
         
        let pulse = chams_warped::beat(settings, time as f64, false, 0.0);
        set1f(u.bass, chams_warped::clamp(pulse - 1.0, 0.0, 1.0));
        set1f(u.flash, chams_warped::clamp((pulse - 1.0) * 1.4, 0.0, 1.0));
        if u.beat_age >= 0 {
            let ages = chams_warped::beat_ages(time);
            gl::Uniform1fv(u.beat_age, 4, ages.as_ptr());
        }
        set1i(u.mode, settings.mode as i32);
         
        if settings.mode == 6 {
            set1f(u.tint_amount, chams_warped::clamp01(settings.glass_tint));
        }
        if settings.mode == 9 {
             
            set1f(u.fill, chams_warped::clamp01(settings.cel_opacity));
        }
        gl::ActiveTexture(MASK_UNIT);
        gl::BindTexture(gl::TEXTURE_2D, self.mask.texture);
        gl::ActiveTexture(GLOW_UNIT);
        gl::BindTexture(gl::TEXTURE_2D, self.glow_a.texture);
        gl::ActiveTexture(SCENE_UNIT);
        gl::BindTexture(
            gl::TEXTURE_2D,
            if wants_scene {
                self.scene.texture
            } else {
                self.mask.texture
            },
        );
        gl::DrawArrays(gl::TRIANGLES, 0, 3);
        gl::Disable(gl::BLEND);
        Ok(())
    }
}

 
 
 
struct Saved {
    read_fbo: i32,
    draw_fbo: i32,
    viewport: [i32; 4],
    program: i32,
    vao: i32,
    array: i32,
    active: u32,
    units: [(u32, i32); 3],
    blend: bool,
    depth_test: bool,
    cull: bool,
    scissor: bool,
    color_mask: [u8; 4],
    clear: [f32; 4],
}

impl Saved {
    unsafe fn capture() -> Self {
        let get = |name| {
            let mut value = 0;
            gl::GetIntegerv(name, &mut value);
            value
        };
        let active = get(gl::ACTIVE_TEXTURE) as u32;
        let units = [MASK_UNIT, GLOW_UNIT, SCENE_UNIT].map(|unit| {
            gl::ActiveTexture(unit);
            let mut texture = 0;
            gl::GetIntegerv(gl::TEXTURE_BINDING_2D, &mut texture);
            (unit, texture)
        });
        gl::ActiveTexture(active);
        let mut viewport = [0; 4];
        gl::GetIntegerv(gl::VIEWPORT, viewport.as_mut_ptr());
        let mut color_mask = [0; 4];
        gl::GetBooleanv(gl::COLOR_WRITEMASK, color_mask.as_mut_ptr());
        let mut clear = [0.0; 4];
        gl::GetFloatv(gl::COLOR_CLEAR_VALUE, clear.as_mut_ptr());
        Self {
            read_fbo: get(gl::READ_FRAMEBUFFER_BINDING),
            draw_fbo: get(gl::DRAW_FRAMEBUFFER_BINDING),
            viewport,
            program: get(gl::CURRENT_PROGRAM),
            vao: get(gl::VERTEX_ARRAY_BINDING),
            array: get(gl::ARRAY_BUFFER_BINDING),
            active,
            units,
            blend: gl::IsEnabled(gl::BLEND) != 0,
            depth_test: gl::IsEnabled(gl::DEPTH_TEST) != 0,
            cull: gl::IsEnabled(gl::CULL_FACE) != 0,
            scissor: gl::IsEnabled(gl::SCISSOR_TEST) != 0,
            color_mask,
            clear,
        }
    }

    unsafe fn restore(&self) {
        gl::BindFramebuffer(gl::READ_FRAMEBUFFER, self.read_fbo as u32);
        gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.draw_fbo as u32);
        gl::Viewport(
            self.viewport[0],
            self.viewport[1],
            self.viewport[2],
            self.viewport[3],
        );
        gl::UseProgram(self.program as u32);
        gl::BindVertexArray(self.vao as u32);
        gl::BindBuffer(gl::ARRAY_BUFFER, self.array as u32);
        for (unit, texture) in self.units {
            gl::ActiveTexture(unit);
            gl::BindTexture(gl::TEXTURE_2D, texture as u32);
        }
        gl::ActiveTexture(self.active);
        for (name, on) in [
            (gl::BLEND, self.blend),
            (gl::DEPTH_TEST, self.depth_test),
            (gl::CULL_FACE, self.cull),
            (gl::SCISSOR_TEST, self.scissor),
        ] {
            if on {
                gl::Enable(name);
            } else {
                gl::Disable(name);
            }
        }
        let mask = self.color_mask;
        gl::ColorMask(mask[0], mask[1], mask[2], mask[3]);
        let clear = self.clear;
        gl::ClearColor(clear[0], clear[1], clear[2], clear[3]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

     
     
     
    #[test]
    fn the_pass_never_borrows_a_vanilla_texture_unit() {
        for unit in [MASK_UNIT, GLOW_UNIT, SCENE_UNIT] {
            assert_ne!(unit, gl::TEXTURE0);
            assert_ne!(unit, gl::TEXTURE1);
            assert_ne!(unit, gl::TEXTURE2);
             
            assert_ne!(unit, crate::graphic::smoke::ITEM_ATLAS_UNIT);
        }
        for sampler in [MASK_SAMPLER, GLOW_SAMPLER, SCENE_SAMPLER] {
            assert!(sampler >= 3);
        }
        assert_eq!(MASK_UNIT, gl::TEXTURE0 + MASK_SAMPLER as u32);
        assert_eq!(GLOW_UNIT, gl::TEXTURE0 + GLOW_SAMPLER as u32);
        assert_eq!(SCENE_UNIT, gl::TEXTURE0 + SCENE_SAMPLER as u32);
    }

     
    #[test]
    fn the_composite_reads_the_three_samplers() {
        for sampler in ["texture(Mask,", "textureLod(Glow,", "textureLod(Scene,"] {
            assert!(
                shader::COMPOSITE_FRAGMENT.contains(sampler),
                "the composite never reads {sampler}"
            );
        }
    }
}
