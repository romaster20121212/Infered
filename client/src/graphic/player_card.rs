 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::anim::{self, Easing, SpringCfg};
use crate::graphic::theme::{self, BORDER, ELEVATED, RADIUS_INNER, SURFACE, SURFACE_HOVER, TEXT};
use crate::graphic::transparency::{self, Layer};
use crate::graphic::{friends as friends_window, settings};
use crate::mapping::profile::{self, Profile};
use crate::state::minecraft;
use anyhow::anyhow;
use egui::{
    pos2, vec2, Align2, Color32, Context, FontId, Id, LayerId, Order, Painter, Rect, Rounding,
    Stroke, Vec2,
};
use image::GenericImageView;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

thread_local! {
     
     
    static IS_FETCH_WORKER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

 
const HEAD: f32 = 42.0;
 
const GAP: f32 = 10.0;
 
const PAD: f32 = 10.0;
 
const CARD_RADIUS: f32 = 5.0;

 
const MENU_ITEM: f32 = 52.0;
 
const MENU_PAD: f32 = 6.0;
 
const MENU_GAP: f32 = 6.0;
 
const MENU_W: f32 = MENU_PAD * 2.0 + MENU_ITEM * 2.0 + MENU_GAP;
 
const MENU_H: f32 = MENU_PAD * 2.0 + MENU_ITEM;
 
const MENU_RISE: f32 = 8.0;
 
const MENU_GAP_Y: f32 = 8.0;

 
const HEAD_OUT: usize = 48;
 
 
 
 
 
 
const HEAD_ROUNDING_FRACTION: f32 = 9.0 / 48.0;
 
const FACE: (u32, u32) = (8, 8);
const HAT: (u32, u32) = (40, 8);
 
const FACE_SIZE: u32 = 8;

 
static NAME: OnceLock<Mutex<Option<String>>> = OnceLock::new();
 
type HeadPixels = (Vec<u8>, usize, usize);
static HEAD_RGBA: OnceLock<Mutex<Option<HeadPixels>>> = OnceLock::new();
 
static HEAD_TEX: OnceLock<Mutex<Option<egui::TextureHandle>>> = OnceLock::new();
 
static MENU_OPEN: AtomicBool = AtomicBool::new(false);
 
static LOADING: AtomicBool = AtomicBool::new(false);
 
static DONE: AtomicBool = AtomicBool::new(false);
 
static FETCH: OnceLock<Mutex<Option<std::thread::JoinHandle<()>>>> = OnceLock::new();

 
 
fn lock_slot<T>(cell: &'static OnceLock<Mutex<Option<T>>>) -> MutexGuard<'static, Option<T>> {
    cell.get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
fn player_name() -> Option<String> {
    lock_slot(&NAME).clone()
}

 
 
 
 
 
 
 
pub(crate) fn paint_head(painter: &Painter, rect: Rect, texture: egui::TextureId) {
    painter.add(egui::epaint::RectShape {
        rect,
        rounding: Rounding::same(rect.width() * HEAD_ROUNDING_FRACTION),
        fill: Color32::WHITE,
        stroke: Stroke::NONE,
        blur_width: 0.0,
        fill_texture_id: texture,
        uv: Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
    });
}

 
fn head_texture(ctx: &Context) -> Option<egui::TextureId> {
    let rgba = lock_slot(&HEAD_RGBA).clone();
    let (rgba, w, h) = rgba?;
    let mut tex = lock_slot(&HEAD_TEX);
    if tex.is_none() {
        let image = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
        *tex = Some(ctx.load_texture(
            "player_card_skin_head",
            image,
            egui::TextureOptions::NEAREST,
        ));
    }
    tex.as_ref().map(|handle| handle.id())
}

 
 
pub(crate) fn close_menu() {
    MENU_OPEN.store(false, Ordering::Relaxed);
}

 
 
pub fn draw(ctx: &Context, progress: f32) {
    ensure_loading(progress);
    if progress <= 0.001 {
        MENU_OPEN.store(false, Ordering::Relaxed);
        return;
    }

    let appear = anim::spring(
        ctx,
        Id::new("player_card_appear"),
        progress,
        SpringCfg::PANEL,
    );
    if appear <= 0.01 {
        return;
    }
     
    let appear = appear.min(1.0);

    let screen = ctx.screen_rect();
    let font = FontId::proportional(15.0);
    let painter = ctx.layer_painter(LayerId::new(
        Order::Foreground,
        Id::new("player_card_layer"),
    ));

    let name = player_name();
    let label = name.clone().unwrap_or_else(|| "…".to_owned());
    let galley = painter.layout_no_wrap(label, font.clone(), Color32::WHITE);
    let name_w = galley.size().x.max(1.0);

    let card_w = PAD + HEAD + GAP + name_w + 12.0;
    let card_h = HEAD + 8.0;

     
    let x_off = (1.0 - appear) * 24.0;
    let y_off = (1.0 - appear) * -8.0;
    let rect = Rect::from_min_size(
        pos2(
            screen.max.x - 10.0 - card_w - x_off,
            screen.min.y + 10.0 + y_off,
        ),
        vec2(card_w, card_h),
    );

    let hovered = ctx
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|pointer| rect.contains(pointer));
     
    let primary_clicked = ctx.input(|i| i.pointer.primary_clicked());
    if primary_clicked
        && ctx
            .input(|i| i.pointer.interact_pos())
            .is_some_and(|pointer| rect.contains(pointer))
    {
        MENU_OPEN.fetch_xor(true, Ordering::Relaxed);
    }
     
     
    if MENU_OPEN.load(Ordering::Relaxed) && primary_clicked {
        let menu = dropdown_rect(rect);
        if let Some(pointer) = ctx.input(|i| i.pointer.interact_pos()) {
            if !rect.contains(pointer) && !menu.contains(pointer) {
                MENU_OPEN.store(false, Ordering::Relaxed);
            }
        }
    }
    if hovered {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let hover = anim::spring(
        ctx,
        Id::new("player_card_hover"),
        hovered as u8 as f32,
        SpringCfg::SNAPPY,
    );

    let grow = 1.0 + 0.03 * hover;
    let draw_rect = Rect::from_center_size(rect.center(), rect.size() * grow);

    let fill = theme::lerp_color(SURFACE, ELEVATED, hover);
    let border = theme::lerp_color(BORDER, theme::accent(), hover);
    painter.rect(draw_rect, CARD_RADIUS, fill, Stroke::new(1.0_f32, border));

     
    if hover > 0.0 {
        painter.rect_filled(
            Rect::from_min_size(
                pos2(draw_rect.left() + 4.0, draw_rect.bottom() - 2.0),
                vec2((draw_rect.width() - 8.0) * hover, 1.5),
            ),
            egui::Rounding::ZERO,
            theme::accent(),
        );
    }

     
     
    let head_scale = 1.0 + 0.08 * hover;
    let head_size = HEAD * head_scale;
    let head_rect = Rect::from_min_size(
        pos2(
            draw_rect.left() + PAD,
            draw_rect.center().y - head_size * 0.5,
        ),
        vec2(head_size, head_size),
    );
    match head_texture(ctx) {
        Some(texture) => paint_head(&painter, head_rect, texture),
        None => {
            painter.rect_filled(head_rect, RADIUS_INNER, SURFACE_HOVER);
            painter.rect_stroke(head_rect, RADIUS_INNER, Stroke::new(1.0_f32, BORDER));
            let initial = name
                .as_deref()
                .and_then(|n| n.chars().next())
                .map(|c| c.to_string())
                .unwrap_or_else(|| "?".to_owned());
            painter.text(
                head_rect.center(),
                Align2::CENTER_CENTER,
                initial,
                FontId::proportional(17.0),
                theme::accent(),
            );
        }
    }

    let name_color = theme::lerp_color(TEXT, Color32::WHITE, hover);
    let name_pos = pos2(
        draw_rect.left() + PAD + HEAD + GAP,
        draw_rect.center().y - galley.size().y * 0.5,
    );
    painter.galley_with_override_text_color(name_pos, galley, name_color);

    dropdown(ctx, rect);
}

 
fn dropdown_rect(card: Rect) -> Rect {
    Rect::from_min_size(
        pos2(card.right() - MENU_W, card.bottom() + MENU_GAP_Y),
        vec2(MENU_W, MENU_H),
    )
}

 
#[derive(Clone, Copy)]
enum MenuIcon {
    Friends,
    Settings,
}

 
#[derive(Clone, Copy)]
struct MenuEntry {
    label: &'static str,
    icon: MenuIcon,
}

const MENU_ENTRIES: [MenuEntry; 2] = [
    MenuEntry {
        label: "Friends",
        icon: MenuIcon::Friends,
    },
    MenuEntry {
        label: "Settings",
        icon: MenuIcon::Settings,
    },
];

 
 
 
fn dropdown(ctx: &Context, card: Rect) {
    let open = MENU_OPEN.load(Ordering::Relaxed);
    let t = anim::toggle(
        ctx,
        Id::new("player_card_dropdown"),
        open,
        0.24,
        Easing::OutBack,
    );
    if !open && t <= 0.001 {
        return;
    }
    ctx.request_repaint();

    let target = dropdown_rect(card);
    let slide = (1.0 - t.clamp(0.0, 1.0)) * -MENU_RISE;
    let alpha = t.clamp(0.0, 1.0);

    egui::Area::new(Id::new("player_card_dropdown_area"))
        .order(Order::Tooltip)
        .fixed_pos(pos2(target.left(), target.top() + slide))
        .show(ctx, |ui| {
            ui.set_opacity(alpha);
             
             
            let (rect, background) =
                ui.allocate_exact_size(vec2(MENU_W, MENU_H), egui::Sense::click());
            if background.clicked() {
                MENU_OPEN.store(false, Ordering::Relaxed);
            }

            let painter = ui.painter();
            let round = Rounding::same(9.0);
            painter.rect(
                rect,
                round,
                transparency::surface(1.0, Layer::Card),
                Stroke::new(1.0_f32, theme::BORDER),
            );
            transparency::edge(painter, rect, round, Layer::Card, alpha);

            for (index, entry) in MENU_ENTRIES.iter().enumerate() {
                let tile = Rect::from_min_size(
                    pos2(
                        rect.left() + MENU_PAD + index as f32 * (MENU_ITEM + MENU_GAP),
                        rect.top() + MENU_PAD,
                    ),
                    Vec2::splat(MENU_ITEM),
                );
                let response = ui.interact(
                    tile,
                    Id::new("player_card_menu_item").with(entry.label),
                    egui::Sense::click(),
                );
                let hover = anim::toggle(
                    ctx,
                    Id::new("player_card_menu_hover").with(entry.label),
                    response.hovered(),
                    0.14,
                    Easing::Out,
                );
                let press = anim::toggle(
                    ctx,
                    Id::new("player_card_menu_press").with(entry.label),
                    response.is_pointer_button_down_on(),
                    0.08,
                    Easing::Out,
                );
                paint_menu_item(painter, tile, entry.icon, entry.label, hover, press);

                if response.clicked() {
                    match entry.icon {
                        MenuIcon::Friends => friends_window::set_open(true),
                        MenuIcon::Settings => settings::toggle_open(),
                    }
                    MENU_OPEN.store(false, Ordering::Relaxed);
                }
            }
        });
}

 
 
fn paint_menu_item(
    painter: &Painter,
    rect: Rect,
    icon: MenuIcon,
    label: &str,
    hover: f32,
    press: f32,
) {
    let shape = Rect::from_center_size(
        rect.center(),
        rect.size() * (1.0 + 0.03 * hover - 0.04 * press),
    );
    let fill = theme::lerp_color(SURFACE, SURFACE_HOVER, hover);
    let border = theme::lerp_color(BORDER, theme::accent(), hover);
    painter.rect(
        shape,
        Rounding::same(theme::RADIUS + 2.0),
        fill,
        Stroke::new(1.0_f32, border),
    );

    let icon_color = theme::lerp_color(theme::TEXT_DIM, theme::accent(), hover);
    let icon_rect =
        Rect::from_center_size(pos2(shape.center().x, shape.top() + 19.0), vec2(22.0, 22.0));
    match icon {
        MenuIcon::Friends => paint_friends_icon(painter, icon_rect, icon_color),
        MenuIcon::Settings => paint_settings_icon(painter, icon_rect, icon_color),
    }

    painter.text(
        pos2(shape.center().x, shape.bottom() - 10.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(8.5),
        theme::lerp_color(theme::TEXT_DIM, theme::TEXT, hover),
    );
}

 
fn paint_friends_icon(painter: &Painter, rect: Rect, color: Color32) {
    let w = rect.width();
    let h = rect.height();
    let head = w * 0.15;
    let stroke = Stroke::new(1.5_f32, color);
     
    painter.circle_stroke(
        pos2(rect.left() + w * 0.40, rect.top() + h * 0.33),
        head,
        stroke,
    );
    painter.add(egui::Shape::line(
        vec![
            pos2(rect.left() + w * 0.20, rect.top() + h * 0.80),
            pos2(rect.left() + w * 0.28, rect.top() + h * 0.58),
            pos2(rect.left() + w * 0.52, rect.top() + h * 0.58),
            pos2(rect.left() + w * 0.60, rect.top() + h * 0.80),
        ],
        stroke,
    ));
     
    painter.circle_stroke(
        pos2(rect.left() + w * 0.72, rect.top() + h * 0.40),
        head * 0.85,
        Stroke::new(1.3_f32, color),
    );
    painter.add(egui::Shape::line(
        vec![
            pos2(rect.left() + w * 0.60, rect.top() + h * 0.80),
            pos2(rect.left() + w * 0.66, rect.top() + h * 0.62),
            pos2(rect.left() + w * 0.82, rect.top() + h * 0.62),
            pos2(rect.left() + w * 0.88, rect.top() + h * 0.80),
        ],
        Stroke::new(1.3_f32, color),
    ));
}

 
fn paint_settings_icon(painter: &Painter, rect: Rect, color: Color32) {
    let w = rect.width();
    let h = rect.height();
    let stroke = Stroke::new(1.4_f32, color);
    for (index, knob) in [0.40_f32, 0.72, 0.28].into_iter().enumerate() {
        let y = rect.top() + h * (0.28 + index as f32 * 0.22);
        painter.line_segment(
            [
                pos2(rect.left() + w * 0.14, y),
                pos2(rect.right() - w * 0.14, y),
            ],
            stroke,
        );
        painter.circle_filled(pos2(rect.left() + w * knob, y), 2.4, color);
    }
}

 
 
fn ensure_loading(progress: f32) {
    if progress <= 0.001 || DONE.load(Ordering::Relaxed) {
        return;
    }
    if LOADING.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = std::thread::spawn(|| {
        IS_FETCH_WORKER.set(true);
        if let Err(error) = load_profile() {
            log::warn!("player card: {error:#}");
        }
        DONE.store(true, Ordering::SeqCst);
        LOADING.store(false, Ordering::SeqCst);
    });
    *lock_slot(&FETCH) = Some(handle);
}

 
 
 
 
 
 
 
 
pub fn shutdown() {
    if IS_FETCH_WORKER.get() {
        return;
    }
    if let Some(handle) = lock_slot(&FETCH).take() {
        let _ = handle.join();
    }
}

 
 
 
 
 
 
fn load_profile() -> anyhow::Result<()> {
    let profile = Profile::from_minecraft(minecraft())?;
    let player_name = profile.name()?;
    if !player_name.is_empty() {
        *lock_slot(&NAME) = Some(player_name.clone());
    }

    let head = match profile.skin_url() {
        Ok(Some(url)) => fetch_and_render_skin(&url).ok(),
        Ok(None) => None,
        Err(error) => {
            log::warn!("player card: reading skin url: {error:#}");
            None
        }
    }
    .or_else(|| load_avatar_head(&player_name).ok())
    .or_else(|| fetch_and_render_avatar(&player_name, "https://minotar.net/helm/{}/64.png").ok());

    if let Some(head) = head {
        *lock_slot(&HEAD_RGBA) = Some(head);
    }
    Ok(())
}

 
fn fetch_and_render_skin(url: &str) -> anyhow::Result<(Vec<u8>, usize, usize)> {
    load_skin_head(url)
}

 
 
 
 
pub(crate) fn load_skin_head(url: &str) -> anyhow::Result<(Vec<u8>, usize, usize)> {
    let png = profile::fetch_url(url)?;
    render_skin_head(&png)
}

 
 
pub(crate) fn load_skin_image(url: &str) -> anyhow::Result<(Vec<u8>, usize, usize)> {
    let png = profile::fetch_url(url)?;
    let image =
        image::load_from_memory(&png).map_err(|error| anyhow!("decoding skin PNG: {error}"))?;
    let (w, h) = image.dimensions();
    if w < 64 || h < 32 {
        return Err(anyhow!("unexpected skin dimensions {w}x{h}"));
    }
    Ok((image.to_rgba8().into_raw(), w as usize, h as usize))
}

 
 
 
 
 
 
pub(crate) fn load_avatar_head(name: &str) -> anyhow::Result<(Vec<u8>, usize, usize)> {
    fetch_and_render_avatar(name, AVATAR_TEMPLATE)
}

 
 
 
const AVATAR_TEMPLATE: &str = "https://mc-heads.net/avatar/{}/64";

 
fn fetch_and_render_avatar(name: &str, template: &str) -> anyhow::Result<(Vec<u8>, usize, usize)> {
    if name.is_empty() {
        return Err(anyhow!("no username to resolve a head for"));
    }
    let url = template.replace("{}", name);
    let png = profile::fetch_url(&url)?;
    render_avatar_head(&png)
}

 
 
 
fn render_skin_head(png: &[u8]) -> anyhow::Result<(Vec<u8>, usize, usize)> {
    let skin =
        image::load_from_memory(png).map_err(|error| anyhow!("decoding skin PNG: {error}"))?;
    let (w, h) = skin.dimensions();
    if w < 64 || h < 16 {
        return Err(anyhow!("unexpected skin dimensions {w}x{h}"));
    }
    Ok((compose_head(&skin.to_rgba8()), HEAD_OUT, HEAD_OUT))
}

 
 
fn render_avatar_head(png: &[u8]) -> anyhow::Result<(Vec<u8>, usize, usize)> {
    let image =
        image::load_from_memory(png).map_err(|error| anyhow!("decoding avatar PNG: {error}"))?;
    let (w, h) = image.dimensions();
    if w < 8 || h < 8 {
        return Err(anyhow!("unexpected avatar dimensions {w}x{h}"));
    }
    Ok((scale_head(&image.to_rgba8(), w, h), HEAD_OUT, HEAD_OUT))
}

 
fn scale_head(src: &image::RgbaImage, w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEAD_OUT * HEAD_OUT * 4);
    for oy in 0..HEAD_OUT as u32 {
        for ox in 0..HEAD_OUT as u32 {
            let sx = ox * w / HEAD_OUT as u32;
            let sy = oy * h / HEAD_OUT as u32;
            out.extend_from_slice(&src.get_pixel(sx, sy).0);
        }
    }
    out
}

 
fn compose_head(skin: &image::RgbaImage) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEAD_OUT * HEAD_OUT * 4);
    let map = |s: u32| FACE.0 + s * FACE_SIZE / HEAD_OUT as u32;
    for oy in 0..HEAD_OUT as u32 {
        for ox in 0..HEAD_OUT as u32 {
            let sx = map(ox);
            let sy = map(oy);
            let face = skin.get_pixel(sx, sy).0;
            let hat = skin
                .get_pixel(HAT.0 + (sx - FACE.0), HAT.1 + (sy - FACE.1))
                .0;
            let rgba = if hat[3] > 0 { hat } else { face };
            out.extend_from_slice(&rgba);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        compose_head, dropdown_rect, paint_head, render_avatar_head, render_skin_head, scale_head,
        HEAD_OUT, HEAD_ROUNDING_FRACTION, MENU_H, MENU_W,
    };
    use image::{Rgba, RgbaImage};

     
     
    fn sample_skin() -> RgbaImage {
        let mut skin = RgbaImage::new(64, 64);
        for y in 8..16 {
            for x in 8..16 {
                skin.put_pixel(x, y, Rgba([230, 30, 30, 255]));
            }
        }
        for y in 8..16 {
            for x in 40..48 {
                skin.put_pixel(x, y, Rgba([20, 20, 240, 128]));
            }
        }
        for x in 40..48 {
            skin.put_pixel(x, 8, Rgba([20, 20, 240, 0]));
        }
        skin
    }

    fn px_at(head: &[u8], x: usize, y: usize) -> (u8, u8, u8, u8) {
        let i = (y * HEAD_OUT + x) * 4;
        (head[i], head[i + 1], head[i + 2], head[i + 3])
    }

    #[test]
    fn blends_hat_over_face() {
        let head = compose_head(&sample_skin());
         
        assert_eq!(px_at(&head, 24, 24), (20, 20, 240, 128));
         
        assert_eq!(px_at(&head, 15, 0), (230, 30, 30, 255));
    }

     
     
     
    #[test]
    fn keeps_corner_pixels_unmasked_so_the_renderer_rounds_the_head() {
        let head = compose_head(&sample_skin());
        assert_eq!(px_at(&head, 0, 0).3, 255);
        assert_eq!(px_at(&head, 1, 1).3, 255);
    }

     
     
     
     
    #[test]
    fn head_is_painted_as_one_rounded_textured_rect() {
        let ctx = egui::Context::default();
        let texture = ctx.load_texture(
            "head_shape_test",
            egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255, 0, 0, 255]),
            egui::TextureOptions::NEAREST,
        );
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(42.0, 42.0));
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("head_shape_test"),
            ));
            paint_head(&painter, rect, texture.id());
        });
        let shapes: Vec<egui::Shape> = output.shapes.into_iter().map(|c| c.shape).collect();
        assert_eq!(shapes.len(), 1, "the head is a single shape");
        let egui::Shape::Rect(head) = &shapes[0] else {
            panic!(
                "expected the head drawn as a Rect shape, got {:?}",
                shapes[0]
            );
        };
        assert_eq!(head.rect, rect);
        assert_eq!(
            head.rounding,
            egui::Rounding::same(42.0 * HEAD_ROUNDING_FRACTION)
        );
        assert!(
            head.rounding != egui::Rounding::ZERO,
            "the head must be rounded"
        );
        assert_eq!(head.fill_texture_id, texture.id());
    }

     
     
    #[test]
    fn the_dropdown_hangs_below_the_card_and_matches_its_right_edge() {
        let card = egui::Rect::from_min_size(egui::pos2(1000.0, 10.0), egui::vec2(180.0, 50.0));
        let menu = dropdown_rect(card);
        assert_eq!(menu.right(), card.right());
        assert!(menu.top() > card.bottom());
        assert_eq!(menu.size(), egui::vec2(MENU_W, MENU_H));
        assert_eq!(MENU_W, 122.0);
    }

    #[test]
    fn outputs_the_expected_size() {
        let head = compose_head(&sample_skin());
        assert_eq!(head.len(), HEAD_OUT * HEAD_OUT * 4);
    }

    #[test]
    fn works_with_legacy_64x32_skins() {
        let mut skin = RgbaImage::new(64, 32);
        for y in 8..16 {
            for x in 40..48 {
                skin.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        let head = compose_head(&skin);
        assert_eq!(px_at(&head, 24, 24), (255, 255, 255, 255));
    }

    #[test]
    fn rejects_non_png_data() {
        assert!(render_skin_head(b"not a png").is_err());
        assert!(render_avatar_head(b"not a png").is_err());
    }

    #[test]
    fn scales_any_size_avatar() {
         
        let mut avatar = RgbaImage::new(32, 32);
        for y in 8..24 {
            for x in 8..24 {
                avatar.put_pixel(x, y, Rgba([10, 200, 90, 255]));
            }
        }
        let head = scale_head(&avatar, 32, 32);
        assert_eq!(head.len(), HEAD_OUT * HEAD_OUT * 4);
        assert_eq!(px_at(&head, 24, 24), (10, 200, 90, 255));
         
        assert_eq!(px_at(&head, 0, 0).3, 0);
         
        assert_eq!(px_at(&head, 24, 4).3, 0);
    }
}
