 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::branding;
use crate::graphic::fonts;
use crate::graphic::hud;
use crate::mapping::client::minecraft::Minecraft;
use crate::module::render::title_logo::{self, TitleLogoConfig};
use crate::state::try_minecraft;
use egui::{pos2, vec2, Color32, Context, Id, LayerId, Order, Pos2, Rect, Vec2};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

 
 
const MAX_SCREEN_SHARE: f32 = 0.42;

 
 
 
 
 
 
 
const LOGO_HEIGHT: f32 = 0.86;

 
const SPLASH_TEXT: &str = "Infered";
 
const SPLASH_RED: Color32 = Color32::from_rgb(232, 32, 32);
 
const SPLASH_SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 170);
 
 
 
const SPLASH_INSET: f32 = 6.0;
 
const SPLASH_TILT: f32 = -0.349_065_9;
 
 
 
const SPLASH_FONT_SIZE: f32 = 29.0;

 
 
static GATE: std::sync::Mutex<Option<bool>> = std::sync::Mutex::new(None);

 
 
static SPLASH_WARNED: AtomicBool = AtomicBool::new(false);

 
 
 
pub(crate) fn draw(ctx: &Context) {
    let cfg = title_logo::read_config();
    let minecraft = try_minecraft();
    let on_title = minecraft.is_some_and(Minecraft::title_screen_open);
    if !cfg.enabled || !on_title {
        release_splash();
        set_gate(false);
        return;
    }
     
     
    let Some(texture) = branding::title_logo_texture(ctx) else {
        release_splash();
        set_gate(false);
        return;
    };
    set_gate(true);

    let Some(minecraft) = minecraft else {
        return;
    };
    let Some(unit) = hud::gui_to_overlay() else {
         
         
        release_splash();
        return;
    };

    let (source_w, source_h) = branding::title_logo_size();
    let aspect = if source_w > 0 {
        source_h as f32 / source_w as f32
    } else {
        2.0 / 3.0
    };
    let time = ctx.input(|input| input.time);
    let breath = cfg.breath_at(time);
    let screen = ctx.screen_rect().size();
    let rect = logo_rect(screen, unit, &cfg, aspect, breath);

    let painter = ctx.layer_painter(LayerId::new(Order::Background, Id::new("title_logo")));
    painter.image(
        texture.id(),
        rect,
        Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
        Color32::WHITE,
    );

     
     
    hide_vanilla_splash(minecraft);
    draw_splash(ctx, &painter, rect, unit, time);

     
     
    ctx.request_repaint();
}

 
 
 
fn set_gate(hidden: bool) {
    let mut last = GATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if *last == Some(hidden) {
        return;
    }
    match crate::camera_bridge::set_title_logo_hidden(hidden) {
        Ok(()) => *last = Some(hidden),
        Err(error) => log::warn!("Title logo: could not move the vanilla logo gate: {error}"),
    }
}

 
 
 
fn hide_vanilla_splash(minecraft: &Minecraft) {
    match minecraft.hide_title_splash() {
        Ok(()) => {
            SPLASH_WARNED.store(false, Ordering::Relaxed);
        }
        Err(error) => warn_splash_once("hide", &error),
    }
}

 
fn warn_splash_once(action: &str, error: &anyhow::Error) {
    if !SPLASH_WARNED.swap(true, Ordering::Relaxed) {
        log::warn!("Title logo: could not {action} the vanilla splash: {error}");
    }
}

 
 
 
pub(crate) fn release_splash() {
    let Some(minecraft) = try_minecraft() else {
        return;
    };
    match minecraft.restore_title_splash() {
        Ok(()) => {
            SPLASH_WARNED.store(false, Ordering::Relaxed);
        }
        Err(error) => warn_splash_once("restore", &error),
    }
}

 
 
fn splash_anchor(logo: Rect, unit: f32) -> Pos2 {
    let unit = if unit.is_finite() && unit > 0.0 {
        unit
    } else {
        1.0
    };
    pos2(
        logo.right() + SPLASH_INSET * unit,
        logo.bottom() + SPLASH_INSET * unit,
    )
}

 
fn splash_scale(time: f64) -> f32 {
    let phase = (time.rem_euclid(1.0) * std::f64::consts::TAU) as f32;
    1.8 - phase.sin().abs() * 0.1
}

 
 
fn draw_splash(ctx: &Context, painter: &egui::Painter, logo: Rect, unit: f32, time: f64) {
    let anchor = splash_anchor(logo, unit);
     
    let font = fonts::lyrics_font(SPLASH_FONT_SIZE * (splash_scale(time) / 1.8) * unit);
     
    let shadow_offset = Vec2::splat(unit * 0.8);
    let shadow = ctx
        .fonts(|fonts| fonts.layout_no_wrap(SPLASH_TEXT.to_owned(), font.clone(), SPLASH_SHADOW));
    painter.add(tilted_text(
        anchor + shadow_offset,
        shadow,
        SPLASH_TILT,
        SPLASH_SHADOW,
    ));
    let line = ctx.fonts(|fonts| fonts.layout_no_wrap(SPLASH_TEXT.to_owned(), font, SPLASH_RED));
    painter.add(tilted_text(anchor, line, SPLASH_TILT, SPLASH_RED));
}

 
 
fn tilted_text(centre: Pos2, galley: Arc<egui::Galley>, angle: f32, color: Color32) -> egui::Shape {
    let half = galley.size() * 0.5;
    let offset = vec2(
        half.x * angle.cos() - half.y * angle.sin(),
        half.x * angle.sin() + half.y * angle.cos(),
    );
    egui::epaint::TextShape::new(centre - offset, galley, color)
        .with_angle(angle)
        .into()
}

 
 
 
 
 
pub(crate) fn logo_rect(
    screen: Vec2,
    unit: f32,
    cfg: &TitleLogoConfig,
    aspect: f32,
    breath: f32,
) -> Rect {
    let breath = if breath.is_finite() {
        breath.clamp(0.25, 4.0)
    } else {
        1.0
    };
    let unit = if unit.is_finite() && unit > 0.0 {
        unit
    } else {
        1.0
    };
    let mut width = cfg.width * unit * breath;
    let mut height = width * aspect * LOGO_HEIGHT;
     
    let max_height = screen.y * MAX_SCREEN_SHARE;
    if height > max_height && height > 0.0 {
        height = max_height;
        width = height / (aspect * LOGO_HEIGHT);
    }
    Rect::from_center_size(pos2(screen.x * 0.5, cfg.top * unit), vec2(width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::render::title_logo::{DEFAULT_BREATH, DEFAULT_TOP, DEFAULT_WIDTH};

    fn config() -> TitleLogoConfig {
        TitleLogoConfig {
            enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn the_logo_keeps_its_shape_and_sits_centred() {
        let screen = vec2(1280.0, 720.0);
        let rect = logo_rect(screen, 2.0, &config(), 2.0 / 3.0, 1.0);
        assert!((rect.center().x - screen.x * 0.5).abs() < 1e-4);
        assert!((rect.center().y - DEFAULT_TOP * 2.0).abs() < 1e-4);
        assert!((rect.width() - DEFAULT_WIDTH * 2.0).abs() < 1e-3);
         
        assert!((rect.height() - rect.width() * (2.0 / 3.0) * LOGO_HEIGHT).abs() < 1e-3);
        assert!(rect.height() < rect.width() * (2.0 / 3.0));
    }

    #[test]
    fn the_breath_grows_the_logo_around_its_centre() {
        let screen = vec2(1280.0, 720.0);
        let rest = logo_rect(screen, 2.0, &config(), 2.0 / 3.0, 1.0);
        let big = logo_rect(screen, 2.0, &config(), 2.0 / 3.0, 1.0 + DEFAULT_BREATH);
        let small = logo_rect(screen, 2.0, &config(), 2.0 / 3.0, 1.0 - DEFAULT_BREATH);
        assert!(big.width() > rest.width());
        assert!(small.width() < rest.width());
         
        assert!((big.center() - rest.center()).length() < 1e-4);
        assert!((small.center() - rest.center()).length() < 1e-4);
    }

    #[test]
    fn a_short_window_caps_the_logo_above_the_buttons() {
        let screen = vec2(960.0, 400.0);
        let cfg = TitleLogoConfig {
            width: 280.0,
            ..config()
        };
        let rect = logo_rect(screen, 2.0, &cfg, 2.0 / 3.0, 1.0);
        assert!(rect.height() <= screen.y * MAX_SCREEN_SHARE + 1e-3);
        assert!((rect.height() - rect.width() * (2.0 / 3.0) * LOGO_HEIGHT).abs() < 1e-3);
    }

    #[test]
    fn the_splash_hangs_off_the_logos_bottom_right_corner_and_pulses_twice_a_second() {
        let screen = vec2(1280.0, 720.0);
        let logo = logo_rect(screen, 2.0, &config(), 2.0 / 3.0, 1.0);
        let anchor = splash_anchor(logo, 2.0);
        assert!((anchor.x - (logo.right() + SPLASH_INSET * 2.0)).abs() < 1e-4);
        assert!((anchor.y - (logo.bottom() + SPLASH_INSET * 2.0)).abs() < 1e-4);
         
        assert!(anchor.x > logo.center().x && anchor.y > logo.center().y);
         
        assert!((splash_scale(0.0) - 1.8).abs() < 1e-5);
        assert!((splash_scale(0.25) - 1.7).abs() < 1e-4);
        assert!((splash_scale(1.25) - 1.7).abs() < 1e-4);
         
        let (lo, hi) = (0..400)
            .map(|i| splash_scale(i as f64 / 40.0))
            .fold((f32::MAX, f32::MIN), |(lo, hi), value| {
                (lo.min(value), hi.max(value))
            });
        assert!(lo >= 1.7 - 1e-4);
        assert!(hi <= 1.8 + 1e-4);
    }

     
     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn the_title_logo_preview_renders_the_artwork() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let (w, h) = (640usize, 360usize);
        let ctx = Context::default();
        fonts::install(&ctx);
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
         
         
        for frame in 0..2 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(w as f32, h as f32))),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(LayerId::new(
                        Order::Background,
                        Id::new("title_logo_preview"),
                    ));
                    let texture = branding::title_logo_texture(ctx).expect("the logo decodes");
                    let rect = logo_rect(vec2(w as f32, h as f32), 1.0, &config(), 2.0 / 3.0, 1.0);
                    painter.image(
                        texture.id(),
                        rect,
                        Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    draw_splash(ctx, &painter, rect, 1.0, 0.0);
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
        let image = rasterize(primitives, &textures, w, h, [0.06, 0.03, 0.03, 1.0]);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("title-logo.png");
        image.save(&path).expect("the title logo preview saves");

         
         
        let rect = logo_rect(vec2(w as f32, h as f32), 1.0, &config(), 2.0 / 3.0, 1.0);
        let inside = image
            .pixels()
            .enumerate()
            .filter(|(index, pixel)| {
                let (x, y) = (*index % w, *index / w);
                rect.contains(pos2(x as f32, y as f32)) && pixel[0] > 60 && pixel[0] > pixel[1] + 20
            })
            .count();
        assert!(
            inside > 500,
            "the logo should be visibly drawn, got {inside}"
        );
    }

    #[test]
    fn corrupt_inputs_still_produce_a_usable_rectangle() {
        let rect = logo_rect(vec2(800.0, 600.0), f32::NAN, &config(), 2.0 / 3.0, f32::NAN);
        assert!(rect.width().is_finite() && rect.width() > 0.0);
        assert!(rect.height().is_finite() && rect.height() > 0.0);
        let anchor = splash_anchor(
            logo_rect(vec2(800.0, 600.0), f32::NAN, &config(), 2.0 / 3.0, f32::NAN),
            f32::NAN,
        );
        assert!(anchor.x.is_finite() && anchor.y.is_finite());
    }
}
