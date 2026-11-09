 
 
 
 
 
 
 
 
 
 
 

use crate::config;
use crate::friends::{self as registry, Friend, FriendMode, PLAYER_MODULES};
use crate::graphic::ambience;
use crate::graphic::anim::{self, Easing};
use crate::graphic::settings;
use crate::graphic::sidebar;
use crate::graphic::theme;
use crate::graphic::transparency::{self, Layer};
use egui::{
    pos2, vec2, Align, Align2, Color32, Context, FontId, Frame, Id, Layout, Margin, Order, Pos2,
    Rect, Response, RichText, Rounding, Sense, Stroke, Ui, Vec2,
};
use std::f32::consts::{FRAC_PI_2, TAU};
use std::sync::atomic::{AtomicBool, Ordering};

 
const WIDTH: f32 = 340.0;
 
const RISE: f32 = 14.0;
 
const OPEN_TIME: f32 = 0.26;
 
const LIST_H: f32 = 168.0;
 
const ROW_H: f32 = 38.0;
 
const REMOVE_W: f32 = 26.0;
 
const ROW_GAP: f32 = 6.0;
 
const ROW_STAGGER: f32 = 0.04;
 
const ROW_REVEAL_SPAN: f32 = 0.55;
 
const ROW_SLIDE: f32 = 14.0;
 
const ADD_FLASH_TIME: f32 = 0.8;

 
static OPEN: AtomicBool = AtomicBool::new(false);

 
pub fn is_open() -> bool {
    OPEN.load(Ordering::Relaxed)
}

 
 
pub fn set_open(open: bool) {
    OPEN.store(open, Ordering::Relaxed);
    if open {
        settings::close();
        crate::graphic::configs::close();
    }
}

 
pub fn close() {
    OPEN.store(false, Ordering::Relaxed);
}

 
 
pub fn progress(ctx: &Context, gui_progress: f32) -> f32 {
    let target = is_open() && gui_progress > 0.0;
    anim::toggle(
        ctx,
        Id::new("friends_window"),
        target,
        OPEN_TIME,
        Easing::Out,
    )
}

 
pub fn is_visible(progress: f32) -> bool {
    progress > 0.001
}

 
pub fn draw(ctx: &Context, p: f32) {
    if !is_visible(p) {
        return;
    }
     
    ctx.request_repaint();

    egui::Area::new(Id::new("clickgui_friends"))
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
                    ui.add_space(4.0);
                    add_row(ui);
                    ui.add_space(4.0);
                    body(ui, p);
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

 
 
fn header(ui: &mut Ui, p: f32) {
    let mut title = None;
    ui.horizontal(|ui| {
        let response = ui.label(
            RichText::new("Friends")
                .size(16.0)
                .strong()
                .color(theme::TEXT),
        );
        title = Some(response.rect);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if sidebar::pressable(ui, "Close", false, 64.0).clicked() {
                close();
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
        pos2(title.left(), title.bottom() + 2.0),
        vec2(title.width().max(48.0) * t, 1.5),
    );
    painter.rect_filled(
        line,
        Rounding::ZERO,
        theme::with_alpha(theme::accent(), 0.72),
    );
    let head = line.right_center();
    painter.circle_filled(head, 4.5, theme::with_alpha(theme::accent(), 0.16));
    painter.circle_filled(head, 1.8, theme::accent());
}

 
fn add_row(ui: &mut Ui) {
    let input_id = Id::new("friends_add_input");
    let mut input = ui
        .ctx()
        .data(|data| data.get_temp::<String>(input_id).unwrap_or_default());

    let mut submitted = false;
    ui.horizontal(|ui| {
        let button_w = 52.0;
        let field_w = (ui.available_width() - button_w - 6.0).max(80.0);
         
         
        let edit = egui::TextEdit::singleline(&mut input)
            .hint_text("Username")
            .desired_width((field_w - 16.0).max(40.0))
            .margin(Margin::symmetric(8.0, 7.0));
        let response = ui.add_sized(vec2(field_w, 32.0), edit);
        submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if action_button(ui, "Add", button_w, 32.0) {
            submitted = true;
        }
    });

    if submitted {
        let name = input.trim().to_owned();
        if registry::add(&name) {
            config::save();
            select(&name);
             
             
             
            let now = ui.ctx().input(|i| i.time);
            ui.ctx().data_mut(|data| {
                data.insert_temp(Id::new("friends_last_added"), (name.clone(), now));
            });
            input.clear();
        }
    }

    ui.ctx().data_mut(|data| data.insert_temp(input_id, input));
}

 
fn body(ui: &mut Ui, p: f32) {
    list(ui, p);
    ui.add_space(2.0);
    let friend = selected_friend();
     
     
     
    let detail_t = anim::toggle(
        ui.ctx(),
        Id::new("friends_detail_open"),
        friend.is_some(),
        0.24,
        Easing::Out,
    );
    let Some(friend) = friend else {
        return;
    };
    ui.scope(|ui| {
        ui.set_opacity(detail_t);
        section(ui, "Selected friend");
        detail(ui, &friend);
    });
}

 
fn list(ui: &mut Ui, p: f32) {
    let friends = registry::list();
    section(ui, "Your friends");
    if friends.is_empty() {
        ui.label(
            RichText::new("No friends yet — add a username above.")
                .size(11.5)
                .color(theme::TEXT_MUTED),
        );
        return;
    }

    let mut select_name = None;
    let mut remove_name = None;
    egui::ScrollArea::vertical()
        .id_salt("friends_list")
        .max_height(LIST_H)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for (index, friend) in friends.iter().enumerate() {
                let selected = is_selected(&friend.name);
                let reveal = row_reveal(p, index);
                ui.horizontal(|ui| {
                    let row_w = (ui.available_width() - REMOVE_W - ROW_GAP).max(80.0);
                    if friend_row(ui, friend, selected, row_w, reveal).clicked() {
                        select_name = Some(friend.name.clone());
                    }
                    if remove_button(ui, &friend.name).clicked() {
                        remove_name = Some(friend.name.clone());
                    }
                });
            }
        });

    if let Some(name) = select_name {
        select(&name);
    }
    if let Some(name) = remove_name {
        registry::remove(&name);
        if is_selected(&name) {
            select("");
        }
        config::save();
    }
}

 
 
fn row_reveal(open: f32, index: usize) -> f32 {
    let delay = index.min(6) as f32 * ROW_STAGGER;
    Easing::Out.apply(((open - delay) / ROW_REVEAL_SPAN).clamp(0.0, 1.0))
}

 
fn name_phase(name: &str) -> f32 {
    let mut hash: u32 = 2_166_136_261;
    for byte in name.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    (hash % 1_000) as f32 / 1_000.0 * TAU
}

 
fn rotate_about(point: Pos2, center: Pos2, angle: f32) -> Pos2 {
    let offset = point - center;
    let (sin, cos) = angle.sin_cos();
    center
        + vec2(
            offset.x * cos - offset.y * sin,
            offset.x * sin + offset.y * cos,
        )
}

 
fn friend_row(ui: &mut Ui, friend: &Friend, selected: bool, width: f32, reveal: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::click());
    let ctx = ui.ctx().clone();
    let hover = anim::toggle(
        &ctx,
        Id::new("friend_row_hover").with(&friend.name),
        response.hovered(),
        0.14,
        Easing::Out,
    );
    let selected_t = anim::toggle(
        &ctx,
        Id::new("friend_row_selected").with(&friend.name),
        selected,
        0.24,
        Easing::OutBack,
    )
    .clamp(0.0, 1.0);
    let now = ctx.input(|i| i.time);
    let accent = theme::accent();

    let fill = theme::lerp_color(theme::SURFACE, theme::SURFACE_HOVER, hover);
    let fill = theme::lerp_color(fill, accent, 0.16 * selected_t);
    let border = theme::lerp_color(theme::BORDER, accent, selected_t);
    let painter = ui.painter();

     
     
    if selected_t > 0.001 {
        painter.rect_filled(
            rect.expand(1.5),
            Rounding::same(9.0),
            theme::with_alpha(accent, 0.035 * selected_t),
        );
    }
    painter.rect(
        rect,
        Rounding::same(8.0),
        fill,
        Stroke::new(1.0_f32, border),
    );

     
     
    let content = painter.with_clip_rect(rect);
    let slide = (1.0 - reveal) * ROW_SLIDE;
    let activity = hover.max(selected_t);

     
     
    if hover > 0.005 {
        let phase = name_phase(&friend.name) / TAU;
        let sweep = ((now as f32 * 0.5 + phase) % 1.0 + 1.0) % 1.0;
        let band_w = 34.0;
        let x = rect.left() - band_w + (rect.width() + band_w * 2.0) * sweep;
        for (scale, alpha) in [(1.0, 0.018), (0.55, 0.03), (0.25, 0.04)] {
            let w = band_w * scale;
            content.rect_filled(
                Rect::from_min_size(
                    pos2(x + (band_w - w) * 0.5, rect.top() + 2.0),
                    vec2(w, rect.height() - 4.0),
                ),
                Rounding::same(6.0),
                theme::with_alpha(accent, alpha * hover),
            );
        }
    }

     
    if selected_t > 0.001 {
        let bar_h = (rect.height() - 16.0) * selected_t;
        let bar =
            Rect::from_center_size(pos2(rect.left() + 2.2, rect.center().y), vec2(2.5, bar_h));
        painter.rect_filled(
            bar.expand(1.5),
            Rounding::same(2.0),
            theme::with_alpha(accent, 0.10 * selected_t),
        );
        painter.rect_filled(bar, Rounding::same(1.5), accent);
    }

    let tile_side = 24.0 * (1.0 + 0.06 * activity);
    let tile = Rect::from_center_size(
        pos2(rect.left() + 21.0 + slide, rect.center().y),
        Vec2::splat(tile_side),
    );
    content.rect_filled(
        tile,
        Rounding::same(6.0 * tile_side / 24.0),
        theme::with_alpha(accent, 0.18 + 0.08 * activity),
    );
    content.text(
        tile.center(),
        Align2::CENTER_CENTER,
        initial(&friend.name),
        FontId::proportional(12.0 + 0.6 * activity),
        accent,
    );

     
     
    if activity > 0.01 {
        let phase = now as f32 * 1.7 + name_phase(&friend.name);
        let radius = 15.5 + 0.7 * (now as f32 * 3.1).sin();
        let points: Vec<Pos2> = (0..=14)
            .map(|step| {
                let angle = phase + TAU * 0.72 * step as f32 / 14.0;
                tile.center() + vec2(angle.cos(), angle.sin()) * radius
            })
            .collect();
        content.add(egui::Shape::line(
            points,
            Stroke::new(1.2_f32, theme::with_alpha(accent, 0.32 * activity)),
        ));
    }

    let text_x = tile.right() + 9.0;
    content.text(
        pos2(text_x, rect.center().y - 7.0),
        Align2::LEFT_CENTER,
        &friend.name,
        FontId::proportional(13.0),
        theme::TEXT,
    );
    content.text(
        pos2(text_x, rect.center().y + 8.0),
        Align2::LEFT_CENTER,
        friend.summary(),
        FontId::proportional(9.5),
        theme::TEXT_DIM,
    );

     
     
    if let Some((added, at)) =
        ctx.data(|data| data.get_temp::<(String, f64)>(Id::new("friends_last_added")))
    {
        if added.eq_ignore_ascii_case(&friend.name) {
            let age = (now - at) as f32;
            if (0.0..ADD_FLASH_TIME).contains(&age) {
                let t = age / ADD_FLASH_TIME;
                let expand = Easing::Out.apply(t);
                painter.rect_stroke(
                    rect.expand(4.0 * expand),
                    Rounding::same(8.0 + 3.0 * expand),
                    Stroke::new(1.5_f32, theme::with_alpha(accent, (1.0 - t).powi(2) * 0.7)),
                );
                ctx.request_repaint();
            }
        }
    }

    response
}

 
 
fn remove_button(ui: &mut Ui, name: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(REMOVE_W), Sense::click());
    let hover = anim::toggle(
        ui.ctx(),
        Id::new("friend_remove_hover").with(name),
        response.hovered(),
        0.12,
        Easing::Out,
    );
    let press = anim::toggle(
        ui.ctx(),
        Id::new("friend_remove_press").with(name),
        response.is_pointer_button_down_on(),
        0.07,
        Easing::Out,
    );
    let fill = theme::lerp_color(theme::SURFACE, theme::SURFACE_HOVER, hover);
    let color = theme::lerp_color(theme::TEXT_MUTED, theme::DANGER, hover);
    let border = theme::lerp_color(theme::BORDER, theme::DANGER, hover * 0.45);
    let painter = ui.painter();
    painter.rect(
        rect,
        Rounding::same(7.0),
        fill,
        Stroke::new(1.0_f32, border),
    );
    if hover > 0.001 {
        painter.circle_filled(
            rect.center(),
            9.0 + 2.5 * hover,
            theme::with_alpha(theme::DANGER, 0.08 * hover),
        );
    }

    let angle = (0.65 * hover + 0.35 * press) * FRAC_PI_2;
    let scale = 1.0 + 0.15 * hover - 0.22 * press;
    let glyph = Rect::from_center_size(rect.center(), Vec2::splat(8.0 * scale));
    let stroke = Stroke::new(1.4_f32 + 0.4 * hover, color);
    painter.line_segment(
        [
            rotate_about(glyph.left_top(), rect.center(), angle),
            rotate_about(glyph.right_bottom(), rect.center(), angle),
        ],
        stroke,
    );
    painter.line_segment(
        [
            rotate_about(glyph.right_top(), rect.center(), angle),
            rotate_about(glyph.left_bottom(), rect.center(), angle),
        ],
        stroke,
    );
    response
}

 
fn action_button(ui: &mut Ui, label: &str, width: f32, height: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let hover = anim::toggle(
        ui.ctx(),
        Id::new("friends_action_hover").with(label),
        response.hovered(),
        0.14,
        Easing::Out,
    );
    let press = anim::toggle(
        ui.ctx(),
        Id::new("friends_action_press").with(label),
        response.is_pointer_button_down_on(),
        0.07,
        Easing::Out,
    );
    let fill = theme::lerp_color(theme::accent(), theme::accent_dim(), 0.35 * press);
    let border = theme::lerp_color(theme::BORDER, Color32::WHITE, hover * 0.5);
    let shape = Rect::from_center_size(rect.center(), rect.size() * (1.0 - 0.04 * press));
    let painter = ui.painter();
    painter.rect(
        shape,
        Rounding::same(7.0),
        fill,
        Stroke::new(1.0_f32, border),
    );
     
     
    if hover > 0.001 {
        let inset = 7.0;
        painter.rect_filled(
            Rect::from_min_size(
                pos2(shape.left() + inset, shape.bottom() - 2.0),
                vec2((shape.width() - inset * 2.0) * hover, 1.2),
            ),
            Rounding::ZERO,
            Color32::from_white_alpha((70.0 * hover).round() as u8),
        );
    }
    painter.text(
        shape.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(12.5),
        Color32::BLACK,
    );
    response.clicked()
}

 
 
fn mode_selector(ui: &mut Ui, mode: FriendMode) -> Option<FriendMode> {
    let mut chosen = None;
    ui.horizontal(|ui| {
        let width = ((ui.available_width() - 6.0) / 2.0).max(60.0);
        if sidebar::pressable(ui, "Easy", mode == FriendMode::Easy, width).clicked() {
            chosen = Some(FriendMode::Easy);
        }
        if sidebar::pressable(ui, "Advanced", mode == FriendMode::Advanced, width).clicked() {
            chosen = Some(FriendMode::Advanced);
        }
    });
    chosen
}

 
fn detail(ui: &mut Ui, friend: &Friend) {
    let mut clear = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(&friend.name)
                .size(14.0)
                .strong()
                .color(theme::TEXT),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if sidebar::pressable(ui, "Done", false, 48.0).clicked() {
                clear = true;
            }
        });
    });

    if let Some(mode) = mode_selector(ui, friend.mode) {
        registry::set_mode(&friend.name, mode);
        config::save();
    }

    match friend.mode {
        FriendMode::Easy => {
            let mut protected = friend.protected;
            if toggle_row(
                ui,
                "Ignore all player modules",
                Id::new("friends_easy").with(&friend.name),
                &mut protected,
            ) {
                registry::set_protected(&friend.name, protected);
                config::save();
            }
            ui.label(
                RichText::new(
                    "Easy mode covers every module that can act on another player, so you never \
                     have to tick them one by one.",
                )
                .size(10.0)
                .color(theme::TEXT_MUTED),
            );
        }
        FriendMode::Advanced => {
            for module in PLAYER_MODULES {
                let mut exempt = friend.exempt.contains(&module);
                if toggle_row(
                    ui,
                    module.display_name(),
                    Id::new("friends_module")
                        .with(&friend.name)
                        .with(module.display_name()),
                    &mut exempt,
                ) {
                    registry::set_exempt(&friend.name, module, exempt);
                    config::save();
                }
            }
            ui.label(
                RichText::new("Checked modules leave this friend alone. Only modules that can affect other players are listed.")
                    .size(10.0)
                    .color(theme::TEXT_MUTED),
            );
        }
    }

    if clear {
        select("");
    }
}

 
fn toggle_row(ui: &mut Ui, label: &str, id: Id, value: &mut bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.0).color(theme::TEXT));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if settings::switch(ui, id, value) {
                changed = true;
            }
        });
    });
    changed
}

 
fn section(ui: &mut Ui, title: &str) {
    ui.label(
        RichText::new(title.to_uppercase())
            .size(10.0)
            .strong()
            .color(theme::accent()),
    );
}

 
fn initial(name: &str) -> String {
    name.chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_owned())
}

 
 
static SELECTED: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

fn select(name: &str) {
    *SELECTED.lock().unwrap() = (!name.is_empty()).then(|| name.to_owned());
}

fn selected() -> Option<String> {
    SELECTED.lock().unwrap().clone()
}

fn is_selected(name: &str) -> bool {
    selected().is_some_and(|selected| selected.eq_ignore_ascii_case(name))
}

fn selected_friend() -> Option<Friend> {
    let name = selected()?;
    registry::list()
        .into_iter()
        .find(|friend| friend.name.eq_ignore_ascii_case(&name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, RawInput, Rect, Vec2};

    #[test]
    fn friend_rows_cascade_in_and_all_settle_when_open() {
        assert_eq!(row_reveal(0.0, 0), 0.0);
        assert_eq!(row_reveal(1.0, 0), 1.0);
        assert_eq!(row_reveal(1.0, 20), 1.0);
         
        assert!(row_reveal(0.2, 0) > row_reveal(0.2, 4));
         
        assert!(row_reveal(0.5, 2) >= row_reveal(0.3, 2));
    }

    #[test]
    fn avatar_orbit_phases_are_stable_and_distinct() {
        let notch = name_phase("Notch");
        assert_eq!(notch, name_phase("Notch"));
        assert!((0.0..TAU).contains(&notch));
        assert_ne!(notch, name_phase("Jeb_"));
    }

    #[test]
    fn remove_button_spins_around_its_center() {
        let center = Pos2::new(10.0, 10.0);
        let point = rotate_about(Pos2::new(14.0, 10.0), center, FRAC_PI_2);
        assert!((point.x - center.x).abs() < 1e-5);
        assert!((point.y - center.y - 4.0).abs() < 1e-5);
    }

    #[test]
    fn a_selected_friend_row_draws_its_orbit_and_add_flash() {
        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let output = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ctx.data_mut(|data| {
                        data.insert_temp(Id::new("friends_last_added"), ("Notch".to_owned(), 0.0));
                    });
                    let friend = crate::friends::Friend {
                        name: "Notch".into(),
                        mode: crate::friends::FriendMode::Easy,
                        protected: true,
                        exempt: Vec::new(),
                    };
                    let response = friend_row(ui, &friend, true, 260.0, 1.0);
                    assert!(response.rect.width() > 0.0);
                });
            },
        );
        assert!(!output.shapes.is_empty(), "the row should paint");
    }

    #[test]
    fn the_window_is_visible_until_its_fade_finishes() {
        assert!(is_visible(1.0));
        assert!(is_visible(0.2));
        assert!(!is_visible(0.0));
    }

    #[test]
    fn opening_friends_closes_settings() {
         
        let _guard = settings::window_lock();
        settings::toggle_open();
        assert!(settings::is_open());
        set_open(true);
        assert!(is_open());
        assert!(!settings::is_open());
        close();
        assert!(!is_open());
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
            if let Some(rect) = ctx.memory(|memory| memory.area_rect(Id::new("clickgui_friends"))) {
                last_area = Some(rect);
            }
        }

        let bounds = last_area.expect("the friends window was never laid out");
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
