 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::config;
use crate::graphic::ambience;
use crate::graphic::anim::{self, Easing};
use crate::graphic::branding;
use crate::graphic::sidebar;
use crate::graphic::theme;
use crate::graphic::transparency::{self, Layer};
use egui::{
    Align, Align2, Color32, Context, FontId, Frame, Id, Layout, Margin, Order, Pos2, Rect,
    RichText, Rounding, Sense, Stroke, Ui, Vec2,
};
use std::sync::atomic::{AtomicBool, Ordering};

 
const WIDTH: f32 = 348.0;
 
const RISE: f32 = 14.0;
 
const OPEN_TIME: f32 = 0.26;
 
 
const GROUP_STAGGER: f32 = 0.14;
 
 
const PREVIEW_RATE: f64 = std::f64::consts::PI / 1.6;
 
const ROW_H: f32 = 30.0;
 
const SWITCH_W: f32 = 34.0;
const SWITCH_H: f32 = 18.0;
const KNOB: f32 = 14.0;

 
static OPEN: AtomicBool = AtomicBool::new(false);
 
static NOTIFICATIONS: AtomicBool = AtomicBool::new(true);
 
static PENDING_SAVE: AtomicBool = AtomicBool::new(false);

 
pub fn is_open() -> bool {
    OPEN.load(Ordering::Relaxed)
}

 
 
pub fn toggle_open() {
    let was_open = OPEN.fetch_xor(true, Ordering::Relaxed);
    if !was_open {
        crate::graphic::friends::close();
        crate::graphic::configs::close();
    }
}

 
pub fn close() {
    OPEN.store(false, Ordering::Relaxed);
}

 
 
 
#[cfg(test)]
pub(crate) fn window_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
 
 
pub fn sync_with_menu(gui_open: bool) {
    if !gui_open {
        close();
    }
}

 
pub fn notifications_enabled() -> bool {
    NOTIFICATIONS.load(Ordering::Relaxed)
}

 
pub fn set_notifications_enabled(on: bool) {
    NOTIFICATIONS.store(on, Ordering::Relaxed);
}

 
fn mark_changed() {
    PENDING_SAVE.store(true, Ordering::Relaxed);
}

 
 
 
 
pub fn progress(ctx: &Context, gui_progress: f32) -> f32 {
    let target = is_open() && gui_progress > 0.0;
    anim::toggle(
        ctx,
        Id::new("settings_window"),
        target,
        OPEN_TIME,
        Easing::Out,
    )
}

 
pub fn is_visible(progress: f32) -> bool {
    progress > 0.001
}

 
pub fn draw(ctx: &Context, p: f32) {
    if is_visible(p) {
         
         
        ctx.request_repaint();

         
         
        egui::Area::new(Id::new("clickgui_settings"))
            .order(Order::Foreground)
            .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, RISE * (1.0 - p)))
            .show(ctx, |ui| {
                ui.set_opacity(p);
                let window = Frame::none()
                     
                     
                    .fill(transparency::surface(0.93, Layer::Window))
                    .stroke(Stroke::new(1.0_f32, theme::BORDER))
                    .rounding(Rounding::same(theme::RADIUS))
                    .shadow(ui.style().visuals.window_shadow)
                    .inner_margin(Margin::same(14.0))
                    .show(ui, |ui| {
                        ui.set_width(WIDTH);
                        ui.spacing_mut().item_spacing.y = 6.0;
                        header(ui, p);
                        ui.add_space(2.0);
                        staggered(ui, reveal(p, 0), |ui| group(ui, "Appearance", appearance));
                        staggered(ui, reveal(p, 1), |ui| group(ui, "Motion", motion));
                        staggered(ui, reveal(p, 2), |ui| group(ui, "Interface", interface));
                        ui.add_space(2.0);
                        staggered(ui, reveal(p, 3), footer);
                    });
                transparency::edge(
                    ui.painter(),
                    window.response.rect,
                    Rounding::same(theme::RADIUS),
                    Layer::Window,
                    0.9 * p,
                );
                ambience::panel_light(
                    ui.painter(),
                    window.response.rect,
                    theme::RADIUS,
                    theme::accent(),
                    0.8 * p,
                );
            });
    }

     
     
    if PENDING_SAVE.load(Ordering::Relaxed) && !ctx.input(|i| i.pointer.any_down()) {
        PENDING_SAVE.store(false, Ordering::Relaxed);
        config::save();
    }
}

 
 
fn reveal(p: f32, index: usize) -> f32 {
    let delay = index as f32 * GROUP_STAGGER;
    Easing::Out.apply(((p - delay) / (1.0 - delay)).clamp(0.0, 1.0))
}

 
 
fn staggered(ui: &mut Ui, reveal: f32, add_contents: impl FnOnce(&mut Ui)) {
    ui.scope(|ui| {
        ui.set_opacity(reveal);
        ui.add_space((1.0 - reveal) * 8.0);
        add_contents(ui);
    });
}

 
 
fn header(ui: &mut Ui, p: f32) {
    let mut title = None;
    ui.horizontal(|ui| {
        if let Some(icon) = branding::settings_icon_texture(ui.ctx()) {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
            ui.painter().image(
                icon.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                theme::brand(),
            );
            ui.add_space(2.0);
        }
        let response = ui.label(
            RichText::new("Settings")
                .size(16.0)
                .strong()
                .color(theme::TEXT),
        );
        title = Some(response.rect);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if sidebar::pressable(ui, "Close", false, 56.0).clicked() {
                OPEN.store(false, Ordering::Relaxed);
            }
        });
    });

    let Some(title) = title else {
        return;
    };
    let t = Easing::Out.apply(p).clamp(0.0, 1.0);
    if t <= 0.001 {
        return;
    }
    let painter = ui.painter();
    let line = Rect::from_min_size(
        Pos2::new(title.left(), title.bottom() + 1.0),
        Vec2::new(title.width().max(48.0) * t, 1.5),
    );
    painter.rect_filled(
        line,
        Rounding::ZERO,
        theme::with_alpha(theme::accent(), 0.72),
    );
    let head = line.right_center();
    painter.circle_filled(head, 4.0, theme::with_alpha(theme::accent(), 0.16));
    painter.circle_filled(head, 1.6, theme::accent());
}

 
fn group(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    Frame::none()
        .fill(theme::SURFACE)
        .stroke(Stroke::new(1.0_f32, theme::BORDER))
        .rounding(Rounding::same(theme::RADIUS))
        .inner_margin(Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(
                RichText::new(title.to_uppercase())
                    .size(10.0)
                    .strong()
                    .color(theme::accent()),
            );
            ui.add_space(2.0);
            add(ui);
        });
}

 
fn appearance(ui: &mut Ui) {
    preview(ui);

    let mut accent = theme::accent_rgb();
    row(ui, "Accent", |ui| {
        if colour_swatch(ui, &mut accent).changed() {
            theme::set_accent(accent);
            mark_changed();
        }
    });

    let mut base = theme::base_rgb();
    row(ui, "Background", |ui| {
        if colour_swatch(ui, &mut base).changed() {
            theme::set_base(base);
            mark_changed();
        }
    });
}

 
fn motion(ui: &mut Ui) {
    let mut speed = anim::speed();
    if slider(
        ui,
        "Speed",
        &mut speed,
        anim::SPEED_MIN,
        anim::SPEED_MAX,
        "x",
        2,
    ) {
        anim::set_speed(speed);
        mark_changed();
    }
    speed_bars(ui);
}

 
fn interface(ui: &mut Ui) {
     
     
     
    let mut percent = (transparency::strength() * 100.0).round();
    if slider(ui, "Transparency", &mut percent, 0.0, 100.0, "%", 0) {
        transparency::set_strength(percent / 100.0);
        mark_changed();
    }

     
    let mut layout = crate::graphic::menu_sidebar::style();
    row(ui, "Menu Layout", |ui| {
        if layout_switch(ui, Id::new("settings_menu_layout"), &mut layout) {
            crate::graphic::menu_sidebar::set_style(layout);
            mark_changed();
        }
    });

     
    let names = crate::graphic::fonts::ui_font_names();
    let mut chosen = crate::graphic::fonts::ui_font();
    row(ui, "Font", |ui| {
        let before = chosen;
        egui::ComboBox::from_id_salt(Id::new("settings_ui_font"))
            .selected_text(names[chosen])
            .width(120.0)
            .show_ui(ui, |ui| {
                for (index, name) in names.iter().enumerate() {
                    ui.selectable_value(&mut chosen, index, *name);
                }
            });
        if chosen != before {
            crate::graphic::fonts::set_ui_font(chosen);
            crate::graphic::fonts::apply_ui_font(ui.ctx());
            mark_changed();
        }
    });

     
     
    let mut gradient = crate::graphic::module_gradient::is_on();
    row(ui, "Module Gradient", |ui| {
        if switch(ui, Id::new("settings_module_gradient"), &mut gradient) {
            crate::graphic::module_gradient::set_on(gradient);
            mark_changed();
        }
    });

    let mut on = notifications_enabled();
    row(ui, "Notifications", |ui| {
        if switch(ui, Id::new("settings_notifications"), &mut on) {
            set_notifications_enabled(on);
            mark_changed();
        }
    });
}

 
fn colour_swatch(ui: &mut Ui, rgb: &mut [u8; 3]) -> egui::Response {
    ui.scope(|ui| {
        let rounding = Rounding::same(theme::RADIUS_INNER);
        ui.spacing_mut().interact_size = Vec2::splat(22.0);
        let visuals = ui.visuals_mut();
        visuals.widgets.inactive.rounding = rounding;
        visuals.widgets.hovered.rounding = rounding;
        visuals.widgets.active.rounding = rounding;
        visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        visuals.widgets.hovered.bg_stroke =
            Stroke::new(1.0_f32, theme::with_alpha(theme::TEXT, 0.25));
        ui.color_edit_button_srgb(rgb)
    })
    .inner
}

 
 
fn layout_switch(ui: &mut Ui, id: Id, style: &mut crate::graphic::menu_sidebar::MenuStyle) -> bool {
    use crate::graphic::menu_sidebar::MenuStyle;

    const ORDER: [MenuStyle; 2] = [MenuStyle::Panels, MenuStyle::Sidebar];
    const LABELS: [&str; 2] = ["Panels", "Sidebar"];

    let (rect, response) = ui.allocate_exact_size(Vec2::new(204.0, 22.0), Sense::click());
    let mut changed = false;
    if response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let segment =
                ((pointer.x - rect.left()) / rect.width() * 2.0).clamp(0.0, 1.999) as usize;
            let next = ORDER[segment];
            if next != *style {
                *style = next;
                changed = true;
            }
        }
    }

    let selected = ORDER.iter().position(|s| s == style).unwrap_or(0);
     
     
    let at_sidebar = anim::toggle(ui.ctx(), id, selected == 1, 0.20, Easing::InOut);
    let slide = at_sidebar;
    let hover = anim::toggle(
        ui.ctx(),
        id.with("hover"),
        response.hovered(),
        0.14,
        Easing::Out,
    );

    let inner = rect.shrink(2.0);
    let segment = inner.width() / 2.0;
    let thumb = Rect::from_min_size(
        Pos2::new(inner.left() + segment * slide, inner.top()),
        Vec2::new(segment, inner.height()),
    );

    let round = Rounding::same(theme::RADIUS_INNER);
    let font = FontId::proportional(10.0);
    let painter = ui.painter();
    painter.rect(
        rect,
        round,
        theme::lerp_color(theme::SURFACE, theme::SURFACE_HOVER, hover),
        Stroke::new(1.0_f32, theme::BORDER),
    );

    let labels = LABELS;
    for (index, label) in labels.iter().enumerate() {
        let centre = Pos2::new(
            inner.left() + segment * (index as f32 + 0.5),
            rect.center().y,
        );
        painter.text(
            centre,
            Align2::CENTER_CENTER,
            *label,
            font.clone(),
            theme::TEXT_DIM,
        );
    }
    painter.rect_filled(
        thumb,
        Rounding::same(theme::RADIUS_INNER - 1.0),
        theme::with_alpha(theme::accent(), 0.80),
    );
    let centre = Pos2::new(
        inner.left() + segment * (selected as f32 + 0.5),
        rect.center().y,
    );
    painter.text(
        centre,
        Align2::CENTER_CENTER,
        labels[selected],
        font,
        theme::TEXT,
    );

    changed
}

 
fn footer(ui: &mut Ui) {
    if sidebar::pressable(ui, "Reset Settings", false, ui.available_width()).clicked() {
        theme::reset_colors();
        anim::set_speed(1.0);
        set_notifications_enabled(true);
        transparency::set_strength(0.0);
        crate::graphic::menu_sidebar::set_style(crate::graphic::menu_sidebar::MenuStyle::Panels);
        mark_changed();
    }
}

 
fn row(ui: &mut Ui, label: &str, control: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), ROW_H),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.label(RichText::new(label).size(12.0).color(theme::TEXT_DIM));
            ui.with_layout(Layout::right_to_left(Align::Center), control);
        },
    );
}

 
 
fn slider(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    suffix: &str,
    decimals: usize,
) -> bool {
    let mut changed = false;
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), ROW_H),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.label(RichText::new(label).size(12.0).color(theme::TEXT_DIM));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let response = ui.add(
                    egui::Slider::new(value, min..=max)
                        .suffix(suffix)
                        .fixed_decimals(decimals)
                        .show_value(true),
                );
                changed = response.changed();
            });
        },
    );
    changed
}

 
 
fn preview(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::hover());

    let time = ui.ctx().input(|i| i.time);
    let on = (time * anim::speed() as f64 * PREVIEW_RATE).sin() >= 0.0;
    let t = anim::toggle(
        ui.ctx(),
        Id::new("settings_preview"),
        on,
        0.5,
        Easing::InOut,
    )
    .clamp(0.0, 1.0);

    let inner = rect.shrink(3.0);
    let half = inner.width() * 0.5;
    let thumb = Rect::from_min_size(
        Pos2::new(inner.left() + half * (1.0 - t), inner.top()),
        Vec2::new(half, inner.height()),
    );

    let enabled_c = Pos2::new(inner.left() + half * 0.5, rect.center().y);
    let idle_c = Pos2::new(inner.left() + half * 1.5, rect.center().y);
    let font = FontId::proportional(11.5);

    let round = Rounding::same(theme::RADIUS_INNER);
    let painter = ui.painter();
    painter.rect(
        rect,
        round,
        theme::SURFACE_HOVER,
        Stroke::new(1.0_f32, theme::BORDER),
    );
    painter.text(
        enabled_c,
        Align2::CENTER_CENTER,
        "Enabled",
        font.clone(),
        theme::TEXT_MUTED,
    );
    painter.text(
        idle_c,
        Align2::CENTER_CENTER,
        "Idle",
        font.clone(),
        theme::TEXT_MUTED,
    );

    painter.rect_filled(
        thumb,
        Rounding::same(theme::RADIUS_INNER - 1.0),
        theme::lerp_color(theme::SURFACE, theme::accent(), t),
    );

     
     
    let text = theme::lerp_color(theme::TEXT_DIM, theme::TEXT, t);
    let over = painter.with_clip_rect(thumb);
    over.text(
        enabled_c,
        Align2::CENTER_CENTER,
        "Enabled",
        font.clone(),
        text,
    );
    over.text(idle_c, Align2::CENTER_CENTER, "Idle", font, text);
}

 
 
fn speed_bars(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 14.0), Sense::hover());
    let time = ui.ctx().input(|i| i.time) as f32;
    let speed = anim::speed();

    const BARS: usize = 28;
    const GAP: f32 = 2.0;
    let bar_w = (rect.width() - GAP * (BARS - 1) as f32) / BARS as f32;

    let painter = ui.painter();
    for i in 0..BARS {
        let wave = 0.5 + 0.5 * (time * speed * 5.0 + i as f32 * 0.55).sin();
        let h = 2.0 + wave * (rect.height() - 2.0);
        let x = rect.left() + i as f32 * (bar_w + GAP);
        let bar = Rect::from_min_max(
            Pos2::new(x, rect.bottom() - h),
            Pos2::new(x + bar_w, rect.bottom()),
        );
        painter.rect_filled(
            bar,
            Rounding::same(1.0),
            theme::with_alpha(
                theme::lerp_color(theme::SURFACE_HOVER, theme::accent(), wave),
                0.35 + 0.55 * wave,
            ),
        );
    }
}

 
 
pub(crate) fn switch(ui: &mut Ui, id: Id, on: &mut bool) -> bool {
    let size = Vec2::new(SWITCH_W, SWITCH_H);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let flipped = response.clicked();
    if flipped {
        *on = !*on;
    }

    let ctx = ui.ctx().clone();
    let t = anim::toggle(&ctx, id, *on, 0.22, Easing::InOut);
    let hover = anim::toggle(
        &ctx,
        id.with("hover"),
        response.hovered(),
        0.14,
        Easing::Out,
    );

    let travel = size.x - 4.0 - KNOB;
    let knob = Rect::from_min_size(
        Pos2::new(rect.left() + 2.0 + travel * t, rect.center().y - KNOB * 0.5),
        Vec2::splat(KNOB),
    );

    let painter = ui.painter();
    painter.rect(
        rect,
        Rounding::same(size.y * 0.5),
        theme::lerp_color(theme::SURFACE_HOVER, theme::accent(), t),
        Stroke::new(
            1.0_f32,
            theme::lerp_color(theme::BORDER, theme::accent(), 0.4 + 0.6 * hover),
        ),
    );
    painter.rect_filled(
        knob,
        Rounding::same(theme::RADIUS_INNER),
        theme::lerp_color(theme::TEXT_MUTED, Color32::WHITE, t),
    );

    if hover > 0.01 {
        ctx.request_repaint();
    }
    flipped
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, RawInput, Rect, Vec2};

     
     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn settings_window_preview() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let (width, height) = (760usize, 820usize);
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        for frame in 0..40 {
            let output = ctx.run(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some(2.0 + frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| draw(ctx, 1.0),
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
        let world = [0.20, 0.22, 0.26, 1.0];
        let image = rasterize(primitives, &textures, width, height, world);
        let flat = image::RgbaImage::from_fn(width as u32, height as u32, |x, y| {
            let pixel = image.get_pixel(x, y);
            let a = pixel[3] as f32 / 255.0;
            let over = |channel: usize| {
                ((pixel[channel] as f32 / 255.0 * a + world[channel] * (1.0 - a)) * 255.0).round()
                    as u8
            };
            image::Rgba([over(0), over(1), over(2), 255])
        });
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings-window.png");
        flat.save(&path).expect("the settings preview saves");
        println!("settings window: {path:?}");
    }

     
     
    #[test]
    fn closing_the_menu_closes_the_window() {
        let _guard = window_lock();
        toggle_open();
        assert!(is_open());
        sync_with_menu(true);
        assert!(is_open(), "an open menu leaves the window alone");
        sync_with_menu(false);
        assert!(!is_open(), "closing the menu closes the window");
         
        sync_with_menu(false);
        assert!(!is_open());
    }

    #[test]
    fn the_window_is_visible_until_its_fade_finishes() {
        assert!(is_visible(1.0));
        assert!(is_visible(0.2));
        assert!(!is_visible(0.0));
    }

     
     
    #[test]
    fn the_window_is_centered_on_screen() {
        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0));
        let mut last_area = None;
        for frame in 0..20 {
            let _ = ctx.run(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| draw(ctx, 1.0),
            );
             
             
            if let Some(rect) = ctx.memory(|memory| memory.area_rect(Id::new("clickgui_settings")))
            {
                last_area = Some(rect);
            }
        }

        let bounds = last_area.expect("the settings window was never laid out");
        let centre = bounds.center();
        assert!(
            (centre.x - screen.center().x).abs() < 1.0,
            "window centre x {} != screen centre {}",
            centre.x,
            screen.center().x
        );
        assert!(
            (centre.y - screen.center().y).abs() < 1.0,
            "window centre y {} != screen centre {}",
            centre.y,
            screen.center().y
        );
    }
}
