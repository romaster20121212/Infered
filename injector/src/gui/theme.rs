 

use egui::{
    Color32, Context, FontDefinitions, FontFamily, Frame, Margin, Rounding, Stroke, TextStyle, Vec2,
};

 
pub const TITLE_BAR_HEIGHT: f32 = 38.0;

 
pub struct Palette;

impl Palette {
     
    pub const BG: Color32 = Color32::from_rgb(0x0a, 0x08, 0x09);
     
    pub const PANEL: Color32 = Color32::from_rgb(0x0e, 0x0b, 0x0c);
     
    pub const CARD: Color32 = Color32::from_rgb(0x18, 0x11, 0x12);
     
    pub const CARD_HOVER: Color32 = Color32::from_rgb(0x23, 0x16, 0x18);
     
    pub const ACCENT: Color32 = Color32::from_rgb(0xe1, 0x2f, 0x33);
     
    pub const ACCENT_BRIGHT: Color32 = Color32::from_rgb(0xff, 0x55, 0x59);
     
    pub const ACCENT_DEEP: Color32 = Color32::from_rgb(0x8f, 0x16, 0x1c);
     
    pub const TEXT: Color32 = Color32::from_rgb(0xf2, 0xec, 0xec);
     
    pub const TEXT_DIM: Color32 = Color32::from_rgb(0x9c, 0x8f, 0x90);
     
    pub const OK: Color32 = Color32::from_rgb(0x4c, 0xd9, 0x77);
     
    pub const ERR: Color32 = Color32::from_rgb(0xff, 0x52, 0x52);
}

 
pub fn lerp_color(from: Color32, to: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| -> u8 { (a as f32 + (b as f32 - a as f32) * t).round() as u8 };
    Color32::from_rgba_unmultiplied(
        mix(from.r(), to.r()),
        mix(from.g(), to.g()),
        mix(from.b(), to.b()),
        mix(from.a(), to.a()),
    )
}

 
pub fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    color.gamma_multiply(alpha.clamp(0.0, 1.0))
}

 
pub fn pulse(time: f64, period: f64) -> f32 {
    (0.5 + 0.5 * (time * std::f64::consts::TAU / period).sin()) as f32
}

 
pub fn apply(ctx: &Context) {
     
     
    let fonts = FontDefinitions::default();
    ctx.set_fonts(fonts);
    let mut style = (*ctx.style()).clone();
    for text_style in [
        TextStyle::Small,
        TextStyle::Body,
        TextStyle::Button,
        TextStyle::Heading,
        TextStyle::Monospace,
    ] {
        style
            .text_styles
            .entry(text_style)
            .and_modify(|font| font.family = FontFamily::Monospace);
    }
    let v = &mut style.visuals;

    v.dark_mode = true;
    v.panel_fill = Palette::BG;
    v.window_fill = Palette::BG;
    v.extreme_bg_color = Palette::PANEL;
    v.override_text_color = Some(Palette::TEXT);
    v.selection.bg_fill = Palette::ACCENT.linear_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0_f32, Palette::ACCENT);
    v.hyperlink_color = Palette::ACCENT;
    v.window_rounding = Rounding::same(12.0);

    let w = &mut v.widgets;
    w.noninteractive.rounding = Rounding::same(10.0);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, Palette::TEXT_DIM);
    w.inactive.rounding = Rounding::same(10.0);
    w.inactive.bg_fill = Palette::CARD;
    w.inactive.weak_bg_fill = Palette::CARD;
    w.inactive.fg_stroke = Stroke::new(1.0_f32, Palette::TEXT);
    w.hovered.rounding = Rounding::same(10.0);
    w.hovered.bg_fill = Palette::CARD_HOVER;
    w.hovered.weak_bg_fill = Palette::CARD_HOVER;
    w.hovered.fg_stroke = Stroke::new(1.0_f32, Palette::TEXT);
    w.active.rounding = Rounding::same(10.0);
    w.active.bg_fill = Palette::ACCENT;
    w.active.weak_bg_fill = Palette::ACCENT;

    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 8.0);
    style.spacing.icon_width_inner = 18.0;

    ctx.set_style(style);
}

 
pub fn central_frame() -> Frame {
    Frame::none()
        .fill(Palette::BG)
        .inner_margin(Margin::symmetric(18.0, 14.0))
}

 
 
pub fn title_bar_frame() -> Frame {
    Frame::none()
        .fill(Palette::BG)
        .inner_margin(Margin::symmetric(0.0, 0.0))
}
