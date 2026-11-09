 
 
 
 
 
 
 
 
 
 

use egui::{Color32, Context, Margin, Rounding, Stroke, Style, Vec2, Visuals};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

 

 
pub const DEFAULT_ACCENT: [u8; 3] = [161, 18, 23];
 
 
pub const LEGACY_ACCENT: [u8; 3] = [225, 47, 51];
 
pub const DEFAULT_BASE: [u8; 3] = [12, 10, 10];

 
const fn pack(rgb: [u8; 3]) -> u32 {
    ((rgb[0] as u32) << 16) | ((rgb[1] as u32) << 8) | rgb[2] as u32
}

fn unpack(value: u32) -> Color32 {
    Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

static ACCENT_RGB: AtomicU32 = AtomicU32::new(pack(DEFAULT_ACCENT));
static BASE_RGB: AtomicU32 = AtomicU32::new(pack(DEFAULT_BASE));
 
static STYLE_DIRTY: AtomicBool = AtomicBool::new(false);

 
pub fn accent() -> Color32 {
    unpack(ACCENT_RGB.load(Ordering::Relaxed))
}

 
 
pub fn accent_dim() -> Color32 {
    scale(accent(), 0.54)
}

 
pub fn brand() -> Color32 {
    accent()
}

 
pub fn base() -> Color32 {
    unpack(BASE_RGB.load(Ordering::Relaxed))
}

 
pub fn accent_rgb() -> [u8; 3] {
    unpack_rgb(ACCENT_RGB.load(Ordering::Relaxed))
}

 
pub fn base_rgb() -> [u8; 3] {
    unpack_rgb(BASE_RGB.load(Ordering::Relaxed))
}

 
pub fn set_accent(rgb: [u8; 3]) {
    ACCENT_RGB.store(pack(rgb), Ordering::Relaxed);
    STYLE_DIRTY.store(true, Ordering::Relaxed);
}

 
pub fn set_base(rgb: [u8; 3]) {
    BASE_RGB.store(pack(rgb), Ordering::Relaxed);
    STYLE_DIRTY.store(true, Ordering::Relaxed);
}

 
pub fn reset_colors() {
    set_accent(DEFAULT_ACCENT);
    set_base(DEFAULT_BASE);
}

 
 
pub fn take_style_dirty() -> bool {
    STYLE_DIRTY.swap(false, Ordering::Relaxed)
}

 
 
pub fn mark_dirty() {
    STYLE_DIRTY.store(true, Ordering::Relaxed);
}

fn unpack_rgb(value: u32) -> [u8; 3] {
    [(value >> 16) as u8, (value >> 8) as u8, value as u8]
}

 
fn scale(color: Color32, factor: f32) -> Color32 {
    let mix = |c: u8| (c as f32 * factor).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(mix(color.r()), mix(color.g()), mix(color.b()), color.a())
}

 
 
pub const SURFACE: Color32 = Color32::from_rgb(19, 15, 15);
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(28, 20, 20);
pub const ELEVATED: Color32 = Color32::from_rgb(22, 17, 17);

 
pub const BORDER: Color32 = Color32::from_rgb(40, 32, 32);

 
pub const TEXT: Color32 = Color32::from_rgb(240, 235, 235);
pub const TEXT_DIM: Color32 = Color32::from_rgb(157, 145, 145);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(97, 88, 88);

 
pub const WARN: Color32 = Color32::from_rgb(241, 143, 63);
pub const DANGER: Color32 = Color32::from_rgb(255, 70, 70);
 
pub const SUCCESS: Color32 = Color32::from_rgb(88, 196, 122);

 
 
 
 
 

 
pub const PAUSE_BUTTON: Color32 = Color32::from_rgb(116, 18, 24);
 
pub const PAUSE_BUTTON_HOVER: Color32 = Color32::from_rgb(158, 30, 36);
 
pub const PAUSE_BUTTON_ACTIVE: Color32 = Color32::from_rgb(88, 12, 17);
 
pub const PAUSE_BUTTON_BORDER: Color32 = Color32::from_rgb(226, 84, 90);
pub const PAUSE_BUTTON_BORDER_HOVER: Color32 = Color32::from_rgb(250, 158, 162);
 
pub const PAUSE_GLOW: Color32 = Color32::from_rgb(218, 48, 54);

 

 
pub const RADIUS: f32 = 7.0;
 
pub const RADIUS_INNER: f32 = 4.0;

 

 
 
 
 
 
pub fn apply(ctx: &Context) {
    let accent = accent();
    let base = base();

    let mut style = Style::default();
    let mut v = Visuals::dark();

    let window_rounding = RADIUS;
    let inner_rounding = RADIUS_INNER;

    v.window_fill = base;
    v.panel_fill = base;
    v.window_stroke = Stroke::new(1.0_f32, BORDER);
    v.window_rounding = Rounding::same(window_rounding);
    v.window_shadow = egui::epaint::Shadow {
        offset: Vec2::new(0.0, 6.0),
        blur: 24.0,
        spread: 0.0,
        color: Color32::from_black_alpha(140),
    };
    v.popup_shadow = v.window_shadow;

    v.selection.bg_fill = accent;
    v.selection.stroke = Stroke::NONE;
    v.hyperlink_color = accent;

    let w = &mut v.widgets;

    w.noninteractive.bg_fill = SURFACE;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_DIM);

    w.inactive.bg_fill = SURFACE;
    w.inactive.weak_bg_fill = SURFACE;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_DIM);
    w.inactive.rounding = Rounding::same(inner_rounding);
    w.inactive.expansion = 0.0;

    w.hovered.bg_fill = SURFACE_HOVER;
    w.hovered.weak_bg_fill = SURFACE_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT);
    w.hovered.rounding = Rounding::same(inner_rounding);
    w.hovered.expansion = 1.0;

    w.active.bg_fill = accent;
    w.active.weak_bg_fill = accent;
    w.active.bg_stroke = Stroke::NONE;
    w.active.fg_stroke = Stroke::new(1.0_f32, Color32::BLACK);
    w.active.rounding = Rounding::same(inner_rounding);
    w.active.expansion = -1.0;

    w.open.bg_fill = SURFACE_HOVER;
    w.open.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.open.fg_stroke = Stroke::new(1.0_f32, TEXT);

    style.visuals = v;

    style.spacing.item_spacing = Vec2::new(6.0, 6.0);
    style.spacing.window_margin = Margin::same(0.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    style.spacing.interact_size.y = 16.0;
    style.spacing.slider_width = 100.0;

     
     
    style.animation_time = 0.18;

    ctx.set_style(style);
}

 

 
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

 
 
 
 
 
pub fn category_accent(_category: crate::module::ModuleCategory) -> Color32 {
    accent()
}

 
pub fn with_alpha(color: Color32, factor: f32) -> Color32 {
    let a = (color.a() as f32 * factor.clamp(0.0, 1.0)).round() as u8;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

     
     
    fn palette_lock() -> MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

     
    #[test]
    fn packing_round_trips_every_channel() {
        for rgb in [[0, 0, 0], [255, 255, 255], [225, 47, 51], [1, 128, 254]] {
            assert_eq!(unpack_rgb(pack(rgb)), rgb);
        }
    }

     
    #[test]
    fn a_color_change_marks_the_style_dirty_once() {
        let _guard = palette_lock();
        set_accent([10, 20, 30]);
        assert!(take_style_dirty());
        assert!(!take_style_dirty());
        assert_eq!(accent_rgb(), [10, 20, 30]);
        reset_colors();
        assert_eq!(accent_rgb(), DEFAULT_ACCENT);
        assert_eq!(base_rgb(), DEFAULT_BASE);
        take_style_dirty();
    }

     
     
    #[test]
    fn no_category_has_a_colour_of_its_own() {
        use crate::module::ModuleCategory;
        let _guard = palette_lock();
        let shared = accent();
        for category in [
            ModuleCategory::Combat,
            ModuleCategory::Movement,
            ModuleCategory::Render,
            ModuleCategory::Player,
            ModuleCategory::World,
            ModuleCategory::Performance,
            ModuleCategory::Misc,
            ModuleCategory::Search,
        ] {
            assert_eq!(
                category_accent(category),
                shared,
                "{category:?} still has its own colour"
            );
        }
    }
}
