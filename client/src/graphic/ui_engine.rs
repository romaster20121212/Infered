use crate::graphic::input::{GUI_OPEN, MOUSE_STATE, PENDING_EVENTS};
use egui::Context;
use egui_glow::Painter;
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

pub struct EguiState {
    pub ctx: Context,
    pub painter: Painter,
    pub last_left_down: bool,
    pub last_right_down: bool,
    pub last_mouse_pos: egui::Pos2,
}

lazy_static! {
    pub static ref EGUI_STATE: Mutex<Option<EguiState>> = Mutex::new(None);
}

 
 
 
static OVERLAY_SCALE_BITS: AtomicU32 = AtomicU32::new(0x3f80_0000);

 
pub fn overlay_scale() -> f32 {
    f32::from_bits(OVERLAY_SCALE_BITS.load(Ordering::Relaxed))
}

 
 
 
 
 
fn elapsed_seconds() -> f64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}

pub fn gather_egui_inputs(
    state: &mut EguiState,
    screen_width: f32,
    screen_height: f32,
    scale_factor: f32,
) -> egui::RawInput {
    let mut raw_input = egui::RawInput {
        time: Some(elapsed_seconds()),
        ..Default::default()
    };

    let viewport_info = egui::ViewportInfo {
        native_pixels_per_point: Some(scale_factor),
        ..Default::default()
    };

    raw_input
        .viewports
        .insert(egui::ViewportId::ROOT, viewport_info);

    let logical_width = screen_width / scale_factor;
    let logical_height = screen_height / scale_factor;

    raw_input.screen_rect = Some(egui::Rect::from_min_max(
        egui::pos2(0.0, 0.0),
        egui::pos2(logical_width, logical_height),
    ));

     
     
     
     
     
     
     
     
    let interactive = GUI_OPEN.load(Ordering::Relaxed)
        || crate::graphic::pause_menu::wants_input()
        || crate::graphic::edit_ui::is_open()
        || crate::graphic::input::dev_takes_input()
        || crate::graphic::hud::top_bar_wants_input();
    if !interactive {
         
         
        if let Ok(mut queue) = PENDING_EVENTS.lock() {
            queue.clear();
        }
        return raw_input;
    }

     
    if let Ok(mut queue) = PENDING_EVENTS.lock() {
        raw_input.events.append(&mut queue);
    }

    let mut mouse = MOUSE_STATE.lock().unwrap();
    let current_pos = egui::pos2(
        (mouse.x as f32) / scale_factor,
        (mouse.y as f32) / scale_factor,
    );

    if current_pos != state.last_mouse_pos {
        raw_input
            .events
            .push(egui::Event::PointerMoved(current_pos));
        state.last_mouse_pos = current_pos;
    }

    if mouse.left_down != state.last_left_down {
        raw_input.events.push(egui::Event::PointerButton {
            pos: current_pos,
            button: egui::PointerButton::Primary,
            pressed: mouse.left_down,
            modifiers: Default::default(),
        });
        state.last_left_down = mouse.left_down;
    }

    if mouse.right_down != state.last_right_down {
        raw_input.events.push(egui::Event::PointerButton {
            pos: current_pos,
            button: egui::PointerButton::Secondary,
            pressed: mouse.right_down,
            modifiers: Default::default(),
        });
        state.last_right_down = mouse.right_down;
    }

     
    let (scroll_x, scroll_y) = (mouse.scroll_x, mouse.scroll_y);
    mouse.scroll_x = 0.0;
    mouse.scroll_y = 0.0;
    if scroll_x != 0.0 || scroll_y != 0.0 {
        raw_input.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(scroll_x, scroll_y),
            modifiers: Default::default(),
        });
    }

    raw_input
}

pub unsafe fn render_egui_ui() {
    let mut viewport = [0; 4];
    crate::gl::GetIntegerv(crate::gl::VIEWPORT, viewport.as_mut_ptr());
    let screen_width = viewport[2] as f32;
    let screen_height = viewport[3] as f32;

    let scale_factor = (screen_height / 720.0).max(1.0).floor();
    OVERLAY_SCALE_BITS.store(scale_factor.to_bits(), Ordering::Relaxed);

    let mut state_guard = EGUI_STATE.lock().unwrap();

    if state_guard.is_none() {
        let gl = glow::Context::from_loader_function(|s| {
            crate::graphic::platform::gl_proc_address(s) as *const _
        });
        let gl = std::sync::Arc::new(gl);
        let ctx = egui::Context::default();

         
         
        crate::graphic::fonts::install(&ctx);

         
         
        crate::graphic::theme::apply(&ctx);

        unsafe {
            crate::gl::PixelStorei(crate::gl::UNPACK_ALIGNMENT, 1);
            crate::gl::PixelStorei(crate::gl::UNPACK_ROW_LENGTH, 0);
            crate::gl::PixelStorei(crate::gl::UNPACK_SKIP_PIXELS, 0);
            crate::gl::PixelStorei(crate::gl::UNPACK_SKIP_ROWS, 0);
        }

        let painter = egui_glow::Painter::new(gl, "", None, false).unwrap();

        *state_guard = Some(EguiState {
            ctx,
            painter,
            last_left_down: false,
            last_right_down: false,
            last_mouse_pos: egui::pos2(0.0, 0.0),
        });
    }

    let state = state_guard.as_mut().unwrap();
    let raw_input = gather_egui_inputs(state, screen_width, screen_height, scale_factor);

    let ctx = state.ctx.clone();

    let full_output = ctx.run(raw_input, |ctx| {
        crate::graphic::gui::render_all(ctx);
    });

    let clipped_primitives = state
        .ctx
        .tessellate(full_output.shapes, full_output.pixels_per_point);

     
     
     
    unsafe {
        let mut last_active_texture = 0;
        crate::gl::GetIntegerv(crate::gl::ACTIVE_TEXTURE, &mut last_active_texture);

        let mut last_texture_unit0 = 0;
        let mut last_sampler_unit0 = 0;
        crate::gl::ActiveTexture(crate::gl::TEXTURE0);
        crate::gl::GetIntegerv(crate::gl::TEXTURE_BINDING_2D, &mut last_texture_unit0);
        crate::gl::GetIntegerv(crate::gl::SAMPLER_BINDING, &mut last_sampler_unit0);

        let mut last_texture_active = last_texture_unit0;
        let mut last_sampler_active = last_sampler_unit0;
        if last_active_texture != crate::gl::TEXTURE0 as i32 {
            crate::gl::ActiveTexture(last_active_texture as u32);
            crate::gl::GetIntegerv(crate::gl::TEXTURE_BINDING_2D, &mut last_texture_active);
            crate::gl::GetIntegerv(crate::gl::SAMPLER_BINDING, &mut last_sampler_active);
        }

        let mut last_array_buffer = 0;
        let mut last_element_array_buffer = 0;
        let mut last_vertex_array = 0;
        let mut last_program = 0;
        let mut last_pixel_unpack_buffer = 0;
        let mut last_framebuffer = 0;
        let mut last_read_framebuffer = 0;
        let mut last_draw_framebuffer = 0;
        let mut last_viewport = [0; 4];
        let mut last_scissor_box = [0; 4];
        crate::gl::GetIntegerv(crate::gl::ARRAY_BUFFER_BINDING, &mut last_array_buffer);
        crate::gl::GetIntegerv(
            crate::gl::ELEMENT_ARRAY_BUFFER_BINDING,
            &mut last_element_array_buffer,
        );
        crate::gl::GetIntegerv(crate::gl::VERTEX_ARRAY_BINDING, &mut last_vertex_array);
        crate::gl::GetIntegerv(crate::gl::CURRENT_PROGRAM, &mut last_program);
        crate::gl::GetIntegerv(
            crate::gl::PIXEL_UNPACK_BUFFER_BINDING,
            &mut last_pixel_unpack_buffer,
        );
        crate::gl::GetIntegerv(crate::gl::FRAMEBUFFER_BINDING, &mut last_framebuffer);
        crate::gl::GetIntegerv(
            crate::gl::READ_FRAMEBUFFER_BINDING,
            &mut last_read_framebuffer,
        );
        crate::gl::GetIntegerv(
            crate::gl::DRAW_FRAMEBUFFER_BINDING,
            &mut last_draw_framebuffer,
        );
        crate::gl::GetIntegerv(crate::gl::VIEWPORT, last_viewport.as_mut_ptr());
        crate::gl::GetIntegerv(crate::gl::SCISSOR_BOX, last_scissor_box.as_mut_ptr());

        let mut unpack_alignment = 0;
        let mut unpack_row_length = 0;
        let mut unpack_skip_pixels = 0;
        let mut unpack_skip_rows = 0;
        crate::gl::GetIntegerv(crate::gl::UNPACK_ALIGNMENT, &mut unpack_alignment);
        crate::gl::GetIntegerv(crate::gl::UNPACK_ROW_LENGTH, &mut unpack_row_length);
        crate::gl::GetIntegerv(crate::gl::UNPACK_SKIP_PIXELS, &mut unpack_skip_pixels);
        crate::gl::GetIntegerv(crate::gl::UNPACK_SKIP_ROWS, &mut unpack_skip_rows);

        let mut blend_src_rgb = 0;
        let mut blend_src_alpha = 0;
        let mut blend_dst_rgb = 0;
        let mut blend_dst_alpha = 0;
        let mut blend_eq_rgb = 0;
        let mut blend_eq_alpha = 0;
        let mut depth_func = 0;
        let mut cull_face = 0;
        let mut front_face = 0;
        crate::gl::GetIntegerv(crate::gl::BLEND_SRC_RGB, &mut blend_src_rgb);
        crate::gl::GetIntegerv(crate::gl::BLEND_SRC_ALPHA, &mut blend_src_alpha);
        crate::gl::GetIntegerv(crate::gl::BLEND_DST_RGB, &mut blend_dst_rgb);
        crate::gl::GetIntegerv(crate::gl::BLEND_DST_ALPHA, &mut blend_dst_alpha);
        crate::gl::GetIntegerv(crate::gl::BLEND_EQUATION_RGB, &mut blend_eq_rgb);
        crate::gl::GetIntegerv(crate::gl::BLEND_EQUATION_ALPHA, &mut blend_eq_alpha);
        crate::gl::GetIntegerv(crate::gl::DEPTH_FUNC, &mut depth_func);
        crate::gl::GetIntegerv(crate::gl::CULL_FACE_MODE, &mut cull_face);
        crate::gl::GetIntegerv(crate::gl::FRONT_FACE, &mut front_face);

        let mut blend_enabled = 0;
        let mut cull_enabled = 0;
        let mut depth_enabled = 0;
        let mut scissor_enabled = 0;
        let mut stencil_enabled = 0;
        let mut depth_mask = 0;
        crate::gl::GetBooleanv(crate::gl::BLEND, &mut blend_enabled);
        crate::gl::GetBooleanv(crate::gl::CULL_FACE, &mut cull_enabled);
        crate::gl::GetBooleanv(crate::gl::DEPTH_TEST, &mut depth_enabled);
        crate::gl::GetBooleanv(crate::gl::SCISSOR_TEST, &mut scissor_enabled);
        crate::gl::GetBooleanv(crate::gl::STENCIL_TEST, &mut stencil_enabled);
        crate::gl::GetBooleanv(crate::gl::DEPTH_WRITEMASK, &mut depth_mask);

        crate::gl::BindBuffer(crate::gl::PIXEL_UNPACK_BUFFER, 0);
        crate::gl::ActiveTexture(crate::gl::TEXTURE0);
        crate::gl::BindSampler(0, 0);
        crate::gl::PixelStorei(crate::gl::UNPACK_ALIGNMENT, 1);
        crate::gl::PixelStorei(crate::gl::UNPACK_ROW_LENGTH, 0);
        crate::gl::PixelStorei(crate::gl::UNPACK_SKIP_PIXELS, 0);
        crate::gl::PixelStorei(crate::gl::UNPACK_SKIP_ROWS, 0);
        crate::gl::Disable(crate::gl::CULL_FACE);
        crate::gl::Disable(crate::gl::DEPTH_TEST);
        crate::gl::Enable(crate::gl::BLEND);

        state.painter.paint_and_update_textures(
            [screen_width as u32, screen_height as u32],
            full_output.pixels_per_point,
            &clipped_primitives,
            &full_output.textures_delta,
        );

         
        crate::gl::ActiveTexture(crate::gl::TEXTURE0);
        crate::gl::BindTexture(crate::gl::TEXTURE_2D, last_texture_unit0 as u32);
        crate::gl::BindSampler(0, last_sampler_unit0 as u32);
        if last_active_texture != crate::gl::TEXTURE0 as i32 {
            crate::gl::ActiveTexture(last_active_texture as u32);
            crate::gl::BindTexture(crate::gl::TEXTURE_2D, last_texture_active as u32);
            crate::gl::BindSampler(0, last_sampler_active as u32);
        }
        crate::gl::BindBuffer(
            crate::gl::PIXEL_UNPACK_BUFFER,
            last_pixel_unpack_buffer as u32,
        );
        crate::gl::PixelStorei(crate::gl::UNPACK_ALIGNMENT, unpack_alignment);
        crate::gl::PixelStorei(crate::gl::UNPACK_ROW_LENGTH, unpack_row_length);
        crate::gl::PixelStorei(crate::gl::UNPACK_SKIP_PIXELS, unpack_skip_pixels);
        crate::gl::PixelStorei(crate::gl::UNPACK_SKIP_ROWS, unpack_skip_rows);
        crate::gl::BindBuffer(crate::gl::ARRAY_BUFFER, last_array_buffer as u32);
        crate::gl::BindBuffer(
            crate::gl::ELEMENT_ARRAY_BUFFER,
            last_element_array_buffer as u32,
        );
        crate::gl::BindVertexArray(last_vertex_array as u32);
        crate::gl::UseProgram(last_program as u32);
        crate::gl::BindFramebuffer(crate::gl::FRAMEBUFFER, last_framebuffer as u32);
        crate::gl::BindFramebuffer(crate::gl::READ_FRAMEBUFFER, last_read_framebuffer as u32);
        crate::gl::BindFramebuffer(crate::gl::DRAW_FRAMEBUFFER, last_draw_framebuffer as u32);
        crate::gl::Viewport(
            last_viewport[0],
            last_viewport[1],
            last_viewport[2],
            last_viewport[3],
        );
        crate::gl::Scissor(
            last_scissor_box[0],
            last_scissor_box[1],
            last_scissor_box[2],
            last_scissor_box[3],
        );
        crate::gl::BlendFuncSeparate(
            blend_src_rgb as u32,
            blend_dst_rgb as u32,
            blend_src_alpha as u32,
            blend_dst_alpha as u32,
        );
        crate::gl::BlendEquationSeparate(blend_eq_rgb as u32, blend_eq_alpha as u32);
        crate::gl::DepthFunc(depth_func as u32);
        crate::gl::DepthMask(depth_mask);
        crate::gl::CullFace(cull_face as u32);
        crate::gl::FrontFace(front_face as u32);
        if blend_enabled != 0 {
            crate::gl::Enable(crate::gl::BLEND);
        } else {
            crate::gl::Disable(crate::gl::BLEND);
        }
        if cull_enabled != 0 {
            crate::gl::Enable(crate::gl::CULL_FACE);
        } else {
            crate::gl::Disable(crate::gl::CULL_FACE);
        }
        if depth_enabled != 0 {
            crate::gl::Enable(crate::gl::DEPTH_TEST);
        } else {
            crate::gl::Disable(crate::gl::DEPTH_TEST);
        }
        if scissor_enabled != 0 {
            crate::gl::Enable(crate::gl::SCISSOR_TEST);
        } else {
            crate::gl::Disable(crate::gl::SCISSOR_TEST);
        }
        if stencil_enabled != 0 {
            crate::gl::Enable(crate::gl::STENCIL_TEST);
        } else {
            crate::gl::Disable(crate::gl::STENCIL_TEST);
        }
        crate::gl::ActiveTexture(last_active_texture as u32);
    }
}

pub fn call_panic() {
     
     
     
    crate::RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
    crate::graphic::input::close_gui_for_panic();
     
     
    crate::module::render::custom_ui::set_editing(false);
    crate::graphic::input::set_editor_cursor(false);
    if let Ok(mut events) = PENDING_EVENTS.lock() {
        events.clear();
    }

     
     
     
     
    crate::config::save();

     
    let world_active = crate::state::client().modules.is_active();
    for handle in crate::state::client().modules.handles() {
        let Ok(mut module) = handle.lock() else {
            continue;
        };
        if module.get_module_data().enabled {
            module.get_module_data_mut().set_enabled(false);
            if world_active {
                if let Err(e) = module.on_stop() {
                    log::error!(
                        "Failed to stop module {} on panic: {}",
                        module.get_module_data().name(),
                        e
                    );
                }
            }
        }
    }
     
     
     
     
}
