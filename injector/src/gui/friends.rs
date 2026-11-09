 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use egui::{
    pos2, vec2, Align, Align2, Color32, CursorIcon, FontId, Id, Layout, Margin, Pos2, Rect,
    Rounding, ScrollArea, Sense, Shape, Stroke, TextEdit, TextureHandle, Ui, UiBuilder, Vec2,
};

use super::theme::{lerp_color, pulse, with_alpha, Palette};
use super::widgets::{
    count_up, ease_out_back, elided, glossy_pill, paint_delete, paint_toggle, radial_glow, sweep,
    top_light_rounded, tracked, PillIcon, PillStyle,
};
use super::{ease_out_cubic, InjectorGui};
use crate::friends::{FriendMode, FriendView, PLAYER_MODULES};

 
const HEADER_H: f32 = 44.0;
 
const ROW_H: f32 = 44.0;

 
 
enum Action {
    Expand(String, bool),
    Remove(String),
    Mode(String, FriendMode),
    Protected(String, bool),
    Exempt(String, String, bool),
}

 

 
pub(super) fn page(gui: &mut InjectorGui, ui: &mut Ui) {
    let now = ui.input(|input| input.time);
    let elapsed = (now - gui.friends_opened_at).max(0.0);
    let eased = ease_out_cubic((elapsed / 0.34).clamp(0.0, 1.0) as f32);

     
     
    let (area, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    let total = gui.friends_list.len();
    let protected = gui
        .friends_list
        .iter()
        .filter(|friend| friend.is_protected())
        .count();
    header(ui, area, &gui.friends_icon, total, protected, now, elapsed);

    let top = area.top() + HEADER_H + 10.0;
    let content_rect = Rect::from_min_max(pos2(area.left(), top), area.max);
    let mut content = ui.new_child(
        UiBuilder::new()
            .max_rect(content_rect)
            .layout(Layout::top_down(Align::Min)),
    );
    content.set_opacity(eased);
    content.spacing_mut().item_spacing = vec2(6.0, 6.0);

    let field_id = add_bar(gui, &mut content, now, elapsed);
    if let Some((message, ok)) = gui.friend_notice.clone() {
        notice_strip(&mut content, &message, ok);
    }
    list_header(&mut content, total, protected);

    if total == 0 {
        if empty_friends(&mut content, now, gui.friends_opened_at) {
            content
                .ctx()
                .memory_mut(|memory| memory.request_focus(field_id));
        }
        return;
    }

     
     
    let friends = gui.friends_list.clone();
    let opened_at = gui.friends_opened_at;
    let expanded = gui.friends_expanded.clone();
    let mut actions = Vec::new();
    ScrollArea::vertical()
        .id_salt("friends_list")
        .auto_shrink([false, false])
        .max_height(content.available_height())
        .show(&mut content, |ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 8.0);
            for (index, friend) in friends.iter().enumerate() {
                friend_card(
                    ui,
                    friend,
                    index,
                    expanded.contains(&friend.name),
                    now,
                    opened_at,
                    &mut actions,
                );
            }
            ui.add_space(4.0);
        });

    apply(gui, actions);
}

 

 
 
fn header(
    ui: &Ui,
    area: Rect,
    icon: &TextureHandle,
    total: usize,
    protected: usize,
    now: f64,
    elapsed: f64,
) {
    let draw = ease_out_cubic((elapsed / 0.42).clamp(0.0, 1.0) as f32);
    let mut painter = ui.painter().clone();
    painter.set_opacity(draw);

     
    let mark = Rect::from_min_size(pos2(area.left() + 1.0, area.top() + 2.0), Vec2::splat(28.0));
    radial_glow(
        &painter,
        mark.center(),
        22.0,
        Palette::ACCENT,
        0.10 + 0.05 * pulse(now, 3.6),
    );
    painter.image(
        icon.id(),
        mark.shrink(3.0),
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );

    let text_x = mark.right() + 10.0;
    tracked(
        &painter,
        "FRIENDS",
        16.0,
        Palette::TEXT,
        Align::Min,
        text_x,
        area.top() + 13.0,
    );

    let summary = format!(
        "{} FRIENDS  ·  {} PROTECTED",
        count_up(total, elapsed),
        count_up(protected, elapsed)
    );
    let status = if protected > 0 {
        Palette::OK
    } else {
        Palette::ACCENT
    };
    painter.circle_filled(
        pos2(text_x + 2.0, area.top() + 30.0),
        2.0,
        with_alpha(status, 0.9),
    );
    tracked(
        &painter,
        &summary,
        7.5,
        with_alpha(Palette::TEXT_DIM, 0.85),
        Align::Min,
        text_x + 10.0,
        area.top() + 30.0,
    );

     
    let y = area.top() + HEADER_H;
    let full = (area.width() - 2.0).max(0.0);
    painter.rect_filled(
        Rect::from_min_size(pos2(area.left() + 1.0, y), vec2(full, 1.0)),
        Rounding::same(1.0),
        with_alpha(Palette::TEXT_DIM, 0.11),
    );
    painter.rect_filled(
        Rect::from_min_size(pos2(area.left() + 1.0, y - 0.25), vec2(full * draw, 1.5)),
        Rounding::same(1.0),
        with_alpha(Palette::ACCENT, 0.34 + 0.24 * pulse(now, 3.4)),
    );
    painter.circle_filled(
        pos2(area.left() + 1.0 + full * draw, y),
        2.0,
        with_alpha(Palette::ACCENT_BRIGHT, 0.9 * (1.0 - draw)),
    );
}

 

 
 
fn add_bar(gui: &mut InjectorGui, ui: &mut Ui, now: f64, elapsed: f64) -> Id {
    let mut submit = false;
    let mut field_id = Id::new("friend_name_field");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let button_w = 116.0;
        let field_w = (ui.available_width() - button_w - 8.0).max(140.0);
        let (shell, _) = ui.allocate_exact_size(vec2(field_w, 28.0), Sense::hover());
        let painter = ui.painter().clone();
        let hovered = ui.rect_contains_pointer(shell);
        let clear = (!gui.friend_name.is_empty()).then(|| {
            Rect::from_center_size(
                pos2(shell.right() - 14.0, shell.center().y),
                Vec2::splat(18.0),
            )
        });
        let clear_hot = clear.is_some_and(|clear| ui.rect_contains_pointer(clear));

        painter.rect_filled(
            shell,
            Rounding::same(8.0),
            with_alpha(Palette::CARD, if hovered { 0.78 } else { 0.58 }),
        );
        painter.rect_stroke(
            shell,
            Rounding::same(8.0),
            Stroke::new(
                1.0_f32,
                with_alpha(
                    Palette::TEXT_DIM,
                    0.14 + 0.10 * if hovered { 1.0 } else { 0.0 },
                ),
            ),
        );
        person_plus(
            &painter,
            pos2(shell.left() + 13.0, shell.center().y),
            with_alpha(Palette::TEXT_DIM, 0.78),
        );

        let right = shell.right() - if clear.is_some() { 28.0 } else { 8.0 };
        let edit_rect = Rect::from_min_max(
            pos2(shell.left() + 26.0, shell.top() + 3.0),
            pos2(right, shell.bottom() - 3.0),
        );
        let response = ui.put(
            edit_rect,
            TextEdit::singleline(&mut gui.friend_name)
                .frame(false)
                .hint_text("Add a username")
                .margin(Margin::ZERO),
        );
        field_id = response.id;
        submit = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if response.has_focus() {
            painter.rect_stroke(
                shell,
                Rounding::same(8.0),
                Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.55)),
            );
        }
        if let Some(clear) = clear {
            let clear_response = ui.interact(clear, Id::new("friend_name_clear"), Sense::click());
            if clear_hot {
                painter.circle_filled(clear.center(), 8.0, with_alpha(Palette::TEXT_DIM, 0.16));
            }
            let ink = with_alpha(Palette::TEXT, if clear_hot { 0.95 } else { 0.60 });
            for (a, b) in [
                (vec2(-3.0, -3.0), vec2(3.0, 3.0)),
                (vec2(3.0, -3.0), vec2(-3.0, 3.0)),
            ] {
                painter.line_segment(
                    [clear.center() + a, clear.center() + b],
                    Stroke::new(1.2_f32, ink),
                );
            }
            if clear_response.clicked() {
                gui.friend_name.clear();
                clear_response.request_focus();
            }
            if clear_hot {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
        }

         
        let (slot, _) = ui.allocate_exact_size(vec2(button_w, 28.0), Sense::hover());
        let pill = ui.interact(slot, Id::new("friends_add_pill"), Sense::click());
        let pill_hover =
            ui.ctx()
                .animate_bool_with_time(pill.id.with("hover"), pill.hovered(), 0.16);
        let press = ui.ctx().animate_bool_with_time(
            pill.id.with("press"),
            pill.is_pointer_button_down_on(),
            0.08,
        );
        let appear = ease_out_back(((elapsed - 0.10) / 0.34).clamp(0.0, 1.0) as f32);
        let drawn = slot.translate(vec2(14.0 * (1.0 - appear), 0.0));
        glossy_pill(
            ui.painter(),
            drawn,
            PillStyle {
                label: "ADD FRIEND",
                icon: PillIcon::Plus,
                emphasis: 1.0,
            },
            pill_hover,
            press,
            now,
        );
        if pill.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if pill.clicked() {
            submit = true;
        }
    });
    if submit {
        add_friend(gui);
    }
    field_id
}

 
fn person_plus(painter: &egui::Painter, centre: Pos2, color: Color32) {
    let stroke = Stroke::new(1.15_f32, color);
    painter.circle_stroke(centre + vec2(-1.5, -3.4), 2.2, stroke);
    arc_stroke(painter, centre + vec2(-1.5, 5.6), 4.2, 200.0, 140.0, stroke);
    let plus = centre + vec2(5.4, -4.6);
    painter.line_segment(
        [plus + vec2(-2.2, 0.0), plus + vec2(2.2, 0.0)],
        Stroke::new(1.2_f32, color),
    );
    painter.line_segment(
        [plus + vec2(0.0, -2.2), plus + vec2(0.0, 2.2)],
        Stroke::new(1.2_f32, color),
    );
}

fn add_friend(gui: &mut InjectorGui) {
    let name = gui.friend_name.trim().to_string();
    match gui.app.add_friend(&name) {
        Ok(()) => {
            gui.friend_name.clear();
            gui.friend_notice = Some((format!("Added {name}."), true));
            refresh(gui);
        }
        Err(error) => gui.friend_notice = Some((error, false)),
    }
}

 
fn refresh(gui: &mut InjectorGui) {
    match gui.app.friends() {
        Ok(friends) => gui.friends_list = friends,
        Err(error) => {
            gui.friends_list.clear();
            gui.friend_notice = Some((error, false));
        }
    }
}

 
fn notice_strip(ui: &mut Ui, message: &str, ok: bool) {
    let color = if ok { Palette::OK } else { Palette::ERR };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(8.0), with_alpha(color, 0.07));
    painter.rect_stroke(
        rect,
        Rounding::same(8.0),
        Stroke::new(1.0_f32, with_alpha(color, 0.28)),
    );
    painter.circle_filled(
        pos2(rect.left() + 11.0, rect.center().y),
        2.4,
        with_alpha(color, 0.95),
    );
    let galley = elided(
        ui,
        message,
        9.5,
        with_alpha(Palette::TEXT, 0.90),
        (rect.width() - 84.0).max(30.0),
    );
    painter.galley(
        pos2(rect.left() + 20.0, rect.center().y - galley.size().y * 0.5),
        galley,
        Palette::TEXT,
    );
    tracked(
        painter,
        if ok { "OK" } else { "FAILED" },
        6.5,
        with_alpha(color, 0.85),
        Align::Max,
        rect.right() - 10.0,
        rect.center().y,
    );
}

 
fn list_header(ui: &mut Ui, total: usize, protected: usize) {
    let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    let painter = ui.painter();
    let title = "ON THE LIST";
    let title_w = tracked(
        painter,
        title,
        7.0,
        with_alpha(Palette::TEXT, 0.78),
        Align::Min,
        head.left() + 2.0,
        head.center().y,
    );
    let summary = match total {
        0 => "nobody added yet".to_string(),
        1 => format!("1 friend  ·  {protected} protected"),
        total => format!("{total} friends  ·  {protected} protected"),
    };
    painter.text(
        pos2(head.right() - 2.0, head.center().y),
        Align2::RIGHT_CENTER,
        summary,
        FontId::proportional(9.0),
        with_alpha(Palette::TEXT_DIM, 0.70),
    );
    let line_x = head.left() + 2.0 + title_w + 8.0;
    if line_x < head.right() - 130.0 {
        painter.line_segment(
            [
                pos2(line_x, head.center().y),
                pos2(head.right() - 128.0, head.center().y),
            ],
            Stroke::new(1.0_f32, with_alpha(Palette::TEXT_DIM, 0.09)),
        );
    }
}

 

 
 
 
fn empty_friends(ui: &mut Ui, now: f64, opened_at: f64) -> bool {
    let elapsed = (now - opened_at).max(0.0);
    let appear = ease_out_cubic(((elapsed - 0.08) / 0.36).clamp(0.0, 1.0) as f32);
    let height = (ui.available_height() - 2.0).max(96.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let id = Id::new("friends_empty");
    let response = ui.interact(rect, id, Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), response.hovered(), 0.20);
    let press = ui.ctx().animate_bool_with_time(
        id.with("press"),
        response.is_pointer_button_down_on(),
        0.09,
    );

    let mut painter = ui.painter().clone();
    painter.set_opacity(appear);
    painter.rect_filled(
        rect,
        Rounding::same(12.0),
        with_alpha(Color32::WHITE, 0.018 + 0.012 * hover),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(12.0),
        Stroke::new(
            1.0_f32,
            with_alpha(
                if hover > 0.01 {
                    Palette::ACCENT
                } else {
                    Palette::TEXT_DIM
                },
                0.12 + 0.28 * hover,
            ),
        ),
    );
    let arm = 12.0;
    let ink = Stroke::new(1.5_f32, with_alpha(Palette::ACCENT, 0.22 + 0.50 * hover));
    for (sx, sy) in [(-1.0_f32, -1.0_f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let corner = pos2(
            rect.center().x + sx * (rect.width() * 0.5 - 9.0),
            rect.center().y + sy * (rect.height() * 0.5 - 9.0),
        );
        painter.line_segment([corner, corner + vec2(-sx * arm, 0.0)], ink);
        painter.line_segment([corner, corner + vec2(0.0, -sy * arm)], ink);
    }

     
    let centre = pos2(rect.center().x, rect.top() + height * 0.32);
    let float = 2.5 * (2.0 * pulse(now, 3.2) - 1.0);
    radial_glow(
        &painter,
        centre + vec2(0.0, float),
        54.0,
        Palette::ACCENT,
        0.045 + 0.055 * hover,
    );
    let well = Rect::from_center_size(centre + vec2(0.0, float), Vec2::splat(42.0 + 3.0 * hover));
    painter.rect_filled(
        well,
        Rounding::same(11.0),
        with_alpha(Palette::ACCENT, 0.07 + 0.05 * hover),
    );
    painter.rect_stroke(
        well,
        Rounding::same(11.0),
        Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.30 + 0.32 * hover)),
    );
    person_plus(
        &painter,
        well.center() + vec2(1.0, 1.0),
        with_alpha(Palette::TEXT, 0.72 + 0.20 * hover),
    );

    painter.text(
        pos2(centre.x, centre.y + 44.0),
        Align2::CENTER_CENTER,
        "No friends yet",
        FontId::proportional(12.5),
        with_alpha(Palette::TEXT, 0.78 + 0.22 * hover),
    );
    painter.text(
        pos2(centre.x, centre.y + 59.0),
        Align2::CENTER_CENTER,
        "Add a username above and the client leaves them alone",
        FontId::proportional(9.0),
        with_alpha(Palette::TEXT_DIM, 0.62 + 0.18 * hover),
    );
    let cta = Rect::from_center_size(
        pos2(centre.x, rect.bottom() - 23.0 + 1.0 * press),
        vec2(132.0, 24.0),
    );
    glossy_pill(
        &painter,
        cta,
        PillStyle {
            label: "ADD A USERNAME",
            icon: PillIcon::Plus,
            emphasis: 0.95,
        },
        hover,
        press,
        now,
    );

    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response.clicked()
}

 

 
 
 
fn friend_card(
    ui: &mut Ui,
    friend: &FriendView,
    index: usize,
    expanded: bool,
    now: f64,
    opened_at: f64,
    actions: &mut Vec<Action>,
) {
     
    let t = ((now - opened_at - index as f64 * 0.045) / 0.38).clamp(0.0, 1.0) as f32;
    if t <= 0.0 {
        ui.ctx().request_repaint();
        let _ = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::hover());
        return;
    }
    let appear = ease_out_back(t);
    let fade = ease_out_cubic((t / 0.60).clamp(0.0, 1.0));

    let (slot, _) = ui.allocate_exact_size(
        vec2((ui.available_width() - 2.0).max(80.0), ROW_H),
        Sense::hover(),
    );
    let id = ui.id().with(("friend", friend.name.as_str()));
    let row = ui.interact(slot, id, Sense::click());
    let hover =
        ui.ctx()
            .animate_bool_with_time(id.with("hover"), ui.rect_contains_pointer(slot), 0.16);
    let press =
        ui.ctx()
            .animate_bool_with_time(id.with("press"), row.is_pointer_button_down_on(), 0.09);

    let remove_rect =
        Rect::from_center_size(pos2(slot.right() - 13.0, slot.center().y), vec2(22.0, 22.0));
    let remove = ui.interact(remove_rect, id.with("remove"), Sense::click());
    let remove_hover =
        ui.ctx()
            .animate_bool_with_time(id.with("remove_hover"), remove.hovered(), 0.12);

    let rect = slot
        .translate(vec2(-18.0 * (1.0 - appear), 7.0 * (1.0 - appear)))
        .translate(vec2(0.0, -1.5 * hover + 1.0 * press));
    let mut painter = ui.painter().clone();
    painter.set_opacity(fade);

    let protected = friend.is_protected();
    let accent = if protected {
        Palette::OK
    } else {
        Palette::TEXT_DIM
    };

     
    painter.rect_filled(
        rect.translate(vec2(0.0, 2.5)).expand(0.5),
        Rounding::same(13.0),
        with_alpha(Color32::BLACK, 0.28 + 0.10 * hover),
    );
    if hover > 0.01 {
        for (spread, alpha) in [(4.0_f32, 0.038_f32), (1.5, 0.055)] {
            painter.rect_filled(
                rect.expand(spread),
                Rounding::same(14.0),
                with_alpha(Palette::ACCENT, alpha * hover),
            );
        }
    }
    let base = lerp_color(
        Palette::CARD,
        Palette::ACCENT_DEEP,
        if expanded { 0.20 } else { 0.0 },
    );
    painter.rect_filled(
        rect,
        Rounding::same(10.0),
        lerp_color(base, Palette::CARD_HOVER, hover),
    );
    top_light_rounded(&painter, rect, 10.0, 0.055 + 0.025 * hover);
    if press > 0.01 {
        painter.rect_filled(
            rect,
            Rounding::same(10.0),
            with_alpha(Palette::ACCENT_BRIGHT, 0.05 * press),
        );
    }
    if hover > 0.01 {
        if let Some(pointer) = ui.ctx().pointer_hover_pos() {
            let focus = pos2(
                pointer.x.clamp(rect.left() + 10.0, rect.right() - 10.0),
                pointer.y.clamp(rect.top() + 7.0, rect.bottom() - 7.0),
            );
            radial_glow(
                &painter.with_clip_rect(rect.shrink(1.0)),
                focus,
                74.0,
                Palette::ACCENT,
                0.05 * hover,
            );
        }
    }
    painter.rect_stroke(
        rect,
        Rounding::same(10.0),
        Stroke::new(
            1.0_f32,
            with_alpha(
                if expanded { Palette::ACCENT } else { accent },
                if expanded { 0.42 } else { 0.13 + 0.28 * hover },
            ),
        ),
    );
    painter.line_segment(
        [
            pos2(rect.left() + 12.0, rect.top() + 1.0),
            pos2(rect.right() - 12.0, rect.top() + 1.0),
        ],
        Stroke::new(1.0_f32, with_alpha(Color32::WHITE, 0.05 + 0.03 * hover)),
    );
     
    painter.rect_filled(
        Rect::from_center_size(
            pos2(rect.left() + 1.5, rect.center().y),
            vec2(2.5, if expanded { 24.0 } else { 12.0 + 16.0 * hover }),
        ),
        Rounding::same(1.5),
        with_alpha(accent, if expanded { 0.75 } else { 0.22 + 0.50 * hover }),
    );

     
    let well = Rect::from_center_size(
        pos2(rect.left() + 11.0 + 14.0, rect.center().y),
        Vec2::splat(28.0 + 0.6 * hover),
    );
    avatar(&painter, well, protected, hover, now);

    let badge_right = rect.right() - 48.0;
    let mode_label = friend.mode.label().to_ascii_uppercase();
    let mode_color = with_alpha(
        if protected {
            Palette::OK
        } else {
            Palette::TEXT_DIM
        },
        0.9,
    );
    let mode = painter.layout_no_wrap(mode_label, FontId::proportional(7.5), mode_color);
    let chip = Rect::from_min_size(
        pos2(badge_right - mode.size().x - 12.0, rect.center().y - 7.5),
        vec2(mode.size().x + 12.0, 15.0),
    );
    painter.rect_filled(chip, Rounding::same(4.0), with_alpha(mode_color, 0.10));
    painter.galley(
        pos2(chip.left() + 6.0, chip.center().y - mode.size().y * 0.5),
        mode,
        mode_color,
    );

    let name_x = well.right() + 10.0;
    let name = elided(
        ui,
        &friend.name,
        11.5,
        with_alpha(Palette::TEXT, 0.88 + 0.12 * hover),
        (chip.left() - 10.0 - name_x).max(40.0),
    );
    painter.galley(
        pos2(name_x, rect.center().y - 7.5 - name.size().y * 0.5),
        name,
        Palette::TEXT,
    );
    let summary = elided(
        ui,
        &friend.summary(),
        8.5,
        with_alpha(
            if protected {
                Palette::OK
            } else {
                Palette::TEXT_DIM
            },
            0.62 + 0.20 * hover,
        ),
        (chip.left() - 10.0 - name_x).max(40.0),
    );
    painter.galley(
        pos2(name_x, rect.center().y + 7.5 - summary.size().y * 0.5),
        summary,
        Palette::TEXT_DIM,
    );

     
    let open = ui
        .ctx()
        .animate_bool_with_time(id.with("chevron"), expanded, 0.16);
    let chevron = pos2(rect.right() - 34.0, rect.center().y);
    painter.circle_filled(
        chevron,
        9.0 + 0.6 * hover,
        with_alpha(
            if expanded {
                Palette::ACCENT
            } else {
                Palette::TEXT_DIM
            },
            if expanded { 0.16 } else { 0.05 + 0.08 * hover },
        ),
    );
    painter.circle_stroke(
        chevron,
        9.0 + 0.6 * hover,
        Stroke::new(
            1.0_f32,
            with_alpha(
                if expanded {
                    Palette::ACCENT
                } else {
                    Palette::TEXT_DIM
                },
                0.22 + 0.28 * hover,
            ),
        ),
    );
    let ink = Stroke::new(
        1.25_f32,
        with_alpha(
            if expanded {
                Palette::ACCENT_BRIGHT
            } else {
                Palette::TEXT
            },
            0.55 + 0.35 * hover,
        ),
    );
    let end_y = -1.6 + 3.2 * open;
    let tip_y = 1.6 - 3.2 * open;
    painter.line_segment(
        [
            pos2(chevron.x - 3.2, chevron.y + end_y),
            pos2(chevron.x, chevron.y + tip_y),
        ],
        ink,
    );
    painter.line_segment(
        [
            pos2(chevron.x, chevron.y + tip_y),
            pos2(chevron.x + 3.2, chevron.y + end_y),
        ],
        ink,
    );

    paint_delete(&painter, remove_rect, remove_hover, 0.25 + 0.75 * hover);
    sweep(
        &painter,
        rect.shrink(1.0),
        now + index as f64 * 1.2,
        6.6,
        0.018,
    );

    if row.hovered() || remove.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if remove.clicked() {
        actions.push(Action::Remove(friend.name.clone()));
    } else if row.clicked() {
        actions.push(Action::Expand(friend.name.clone(), !expanded));
    }

    if expanded {
        settings(ui, friend, id, actions, now);
    }
}

 
 
fn avatar(painter: &egui::Painter, well: Rect, protected: bool, hover: f32, now: f64) {
    let centre = well.center();
    let accent = if protected {
        Palette::OK
    } else {
        Palette::TEXT_DIM
    };
    radial_glow(
        painter,
        centre,
        18.0,
        if protected {
            Palette::OK
        } else {
            Palette::ACCENT
        },
        0.035 * if protected { 1.0 } else { 0.0 } + 0.04 * hover,
    );
    painter.rect_filled(
        well,
        Rounding::same(8.0),
        with_alpha(accent, 0.07 + 0.05 * hover),
    );
    painter.rect_stroke(
        well,
        Rounding::same(8.0),
        Stroke::new(1.0_f32, with_alpha(accent, 0.24 + 0.30 * hover)),
    );
    let ink = with_alpha(accent, 0.72 + 0.20 * hover);
    let stroke = Stroke::new(1.2_f32, ink);
    painter.circle_stroke(centre + vec2(0.0, -3.6), 2.8, stroke);
    arc_stroke(painter, centre + vec2(0.0, 6.4), 5.4, 200.0, 140.0, stroke);
    if protected {
        let beat = 0.5 + 0.5 * pulse(now, 3.4);
        let badge = pos2(well.right() - 2.5, well.bottom() - 2.5);
        painter.circle_filled(badge, 3.4, with_alpha(Palette::OK, 0.85 + 0.15 * beat));
        let tick = Stroke::new(1.0_f32, with_alpha(Palette::BG, 0.9));
        painter.line_segment([badge + vec2(-1.7, 0.0), badge + vec2(-0.4, 1.3)], tick);
        painter.line_segment([badge + vec2(-0.4, 1.3), badge + vec2(1.8, -1.4)], tick);
    }
}

 

 
 
fn settings(ui: &mut Ui, friend: &FriendView, id: Id, actions: &mut Vec<Action>, now: f64) {
    ui.add_space(1.0);
    let outer = ui.available_width();
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        let panel = egui::Frame::none()
            .fill(with_alpha(Palette::CARD, 0.44))
            .stroke(Stroke::new(1.0_f32, with_alpha(Palette::TEXT_DIM, 0.10)))
            .rounding(Rounding::same(9.0))
            .inner_margin(Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                 
                 
                ui.vertical(|ui| {
                    ui.set_width((outer - 42.0).max(140.0));
                    ui.spacing_mut().item_spacing = vec2(6.0, 5.0);

                    ui.horizontal(|ui| {
                        let (label, _) = ui.allocate_exact_size(vec2(160.0, 20.0), Sense::hover());
                        tracked(
                            ui.painter(),
                            "EXEMPTIONS",
                            6.5,
                            with_alpha(Palette::TEXT_DIM, 0.72),
                            Align::Min,
                            label.left() + 2.0,
                            label.center().y,
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if let Some(mode) = mode_switch(ui, friend.mode, id.with("mode"), now) {
                                actions.push(Action::Mode(friend.name.clone(), mode));
                            }
                        });
                    });

                    match friend.mode {
                        FriendMode::Easy => {
                            if let Some(on) = switch_row(
                                ui,
                                id.with("protected"),
                                "Ignore all player modules",
                                friend.protected,
                            ) {
                                actions.push(Action::Protected(friend.name.clone(), on));
                            }
                        }
                        FriendMode::Advanced => {
                             
                             
                             
                             
                            let rows = PLAYER_MODULES.len().div_ceil(2);
                            let (grid, _) = ui.allocate_exact_size(
                                vec2(ui.available_width(), rows as f32 * 26.0),
                                Sense::hover(),
                            );
                            let gap = 10.0;
                            let col_w = ((grid.width() - gap) * 0.5).max(60.0);
                            for (index, &(module, label)) in PLAYER_MODULES.iter().enumerate() {
                                let column = index % 2;
                                let row = index / 2;
                                let cell = Rect::from_min_size(
                                    pos2(
                                        grid.left() + column as f32 * (col_w + gap),
                                        grid.top() + row as f32 * 26.0,
                                    ),
                                    vec2(col_w, 24.0),
                                );
                                if let Some(on) = switch_cell(
                                    ui,
                                    cell,
                                    id.with(("exempt", module)),
                                    label,
                                    friend.ignores(module),
                                ) {
                                    actions.push(Action::Exempt(
                                        friend.name.clone(),
                                        module.to_string(),
                                        on,
                                    ));
                                }
                            }
                        }
                    }
                });
            });

         
         
        let rect = panel.response.rect;
        let painter = ui.painter();
        painter.line_segment(
            [
                pos2(rect.left() + 9.0, rect.top()),
                pos2(rect.right() - 9.0, rect.top()),
            ],
            Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.22)),
        );
        painter.rect_filled(
            Rect::from_center_size(pos2(rect.left() + 1.5, rect.top() + 20.0), vec2(2.5, 22.0)),
            Rounding::same(1.5),
            with_alpha(Palette::ACCENT, 0.45),
        );
    });
}

 
 
fn mode_switch(ui: &mut Ui, current: FriendMode, id: Id, now: f64) -> Option<FriendMode> {
    let (rect, _) = ui.allocate_exact_size(vec2(138.0, 22.0), Sense::hover());
    let slide =
        ui.ctx()
            .animate_bool_with_time(id.with("advanced"), current == FriendMode::Advanced, 0.18);
    let half = (rect.width() - 6.0) * 0.5;
    let easy = Rect::from_min_size(
        pos2(rect.left() + 3.0, rect.top()),
        vec2(half, rect.height()),
    );
    let advanced = Rect::from_min_size(
        pos2(rect.left() + 3.0 + half, rect.top()),
        vec2(half, rect.height()),
    );
    let easy_response = ui.interact(easy, id.with("easy"), Sense::click());
    let advanced_response = ui.interact(advanced, id.with("advanced_tab"), Sense::click());
    if easy_response.hovered() || advanced_response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(6.0), with_alpha(Palette::CARD, 0.72));
    painter.rect_stroke(
        rect,
        Rounding::same(6.0),
        Stroke::new(1.0_f32, with_alpha(Palette::TEXT_DIM, 0.14)),
    );
    let pill = Rect::from_min_size(
        pos2(rect.left() + 3.0 + half * slide, rect.top() + 3.0),
        vec2(half, rect.height() - 6.0),
    );
    painter.rect_filled(
        pill,
        Rounding::same(4.0),
        with_alpha(Palette::ACCENT, 0.15 + 0.05 * pulse(now, 3.0)),
    );
    painter.rect_stroke(
        pill,
        Rounding::same(4.0),
        Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.40)),
    );
    for (mode, area, label) in [
        (FriendMode::Easy, easy, "EASY"),
        (FriendMode::Advanced, advanced, "ADVANCED"),
    ] {
        tracked(
            painter,
            label,
            6.5,
            if mode == current {
                with_alpha(Palette::TEXT, 0.95)
            } else {
                with_alpha(Palette::TEXT_DIM, 0.80)
            },
            Align::Center,
            area.center().x,
            area.center().y,
        );
    }
    if advanced_response.clicked() {
        Some(FriendMode::Advanced)
    } else if easy_response.clicked() {
        Some(FriendMode::Easy)
    } else {
        None
    }
}

 
 
fn switch_row(ui: &mut Ui, id: Id, label: &str, value: bool) -> Option<bool> {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    switch_cell(ui, rect, id, label, value)
}

 
 
fn switch_cell(ui: &mut Ui, rect: Rect, id: Id, label: &str, value: bool) -> Option<bool> {
    let response = ui.interact(rect, id, Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), response.hovered(), 0.12);
    let on = ui.ctx().animate_bool_with_time(id.with("on"), value, 0.20);
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Rounding::same(6.0),
        with_alpha(Color32::WHITE, 0.024 * hover),
    );
    let toggle = Rect::from_min_size(
        pos2(rect.right() - 4.0 - 34.0, rect.center().y - 8.0),
        vec2(34.0, 16.0),
    );
    paint_toggle(painter, toggle, on, hover);
    let galley = elided(
        ui,
        label,
        9.5,
        with_alpha(
            if value {
                Palette::TEXT
            } else {
                Palette::TEXT_DIM
            },
            0.62 + 0.30 * hover + 0.08 * on,
        ),
        (rect.width() - 52.0).max(30.0),
    );
    painter.galley(
        pos2(rect.left() + 8.0, rect.center().y - galley.size().y * 0.5),
        galley,
        Palette::TEXT,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if response.clicked() {
        Some(!value)
    } else {
        None
    }
}

 
fn arc_stroke(
    painter: &egui::Painter,
    centre: Pos2,
    radius: f32,
    start_deg: f32,
    sweep_deg: f32,
    stroke: Stroke,
) {
    const SEGMENTS: usize = 14;
    let mut points = Vec::with_capacity(SEGMENTS + 1);
    for step in 0..=SEGMENTS {
        let angle = (start_deg + sweep_deg * step as f32 / SEGMENTS as f32).to_radians();
        points.push(centre + vec2(angle.cos(), angle.sin()) * radius);
    }
    painter.add(Shape::line(points, stroke));
}

 
 
fn apply(gui: &mut InjectorGui, actions: Vec<Action>) {
    let mut changed = false;
    for action in actions {
        match action {
            Action::Expand(name, open) => {
                if open {
                    gui.friends_expanded.insert(name);
                } else {
                    gui.friends_expanded.remove(&name);
                }
            }
            Action::Remove(name) => match gui.app.remove_friend(&name) {
                Ok(()) => {
                    gui.friends_expanded.remove(&name);
                    gui.friend_notice = Some((format!("Removed {name}."), true));
                    changed = true;
                }
                Err(error) => gui.friend_notice = Some((error, false)),
            },
            Action::Mode(name, mode) => match gui.app.set_friend_mode(&name, mode) {
                Ok(()) => changed = true,
                Err(error) => gui.friend_notice = Some((error, false)),
            },
            Action::Protected(name, protected) => {
                match gui.app.set_friend_protected(&name, protected) {
                    Ok(()) => changed = true,
                    Err(error) => gui.friend_notice = Some((error, false)),
                }
            }
            Action::Exempt(name, module, exempt) => {
                match gui.app.set_friend_exempt(&name, &module, exempt) {
                    Ok(()) => changed = true,
                    Err(error) => gui.friend_notice = Some((error, false)),
                }
            }
        }
    }
    if changed {
        refresh(gui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::friends::FriendView;
    use egui::{Context, Pos2, RawInput};

    fn friend(name: &str, mode: FriendMode, protected: bool, exempt: Vec<&str>) -> FriendView {
        FriendView {
            name: name.to_string(),
            mode,
            protected,
            exempt: exempt.into_iter().map(str::to_string).collect(),
        }
    }

    fn gui_with(friends: Vec<FriendView>, expanded: &[&str]) -> InjectorGui {
        let mut gui = InjectorGui::stub_gui();
        gui.drawer_page = super::super::DrawerPage::Friends;
        gui.friends_list = friends;
        gui.friends_expanded = expanded.iter().map(|name| name.to_string()).collect();
        gui
    }

     
    fn painted(gui: &mut InjectorGui) -> Vec<String> {
        let ctx = Context::default();
        super::super::theme::apply(&ctx);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(702.0, 322.0));
        let _ = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |_| {},
        );
        let output = ctx.run(
            RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Palette::BG))
                    .show(ctx, |ui| page(gui, ui));
            },
        );
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_page_shows_the_add_field_and_every_friend() {
        let mut gui = gui_with(
            vec![
                friend("Alex", FriendMode::Easy, true, vec![]),
                friend("Steve", FriendMode::Advanced, false, vec!["KillAura"]),
            ],
            &[],
        );
        let texts = painted(&mut gui);
         
        let joined = texts.join("");
        for expected in [
            "FRIENDS",
            "ADD FRIEND",
            "Alex",
            "Steve",
            "EASY",
            "ADVANCED",
            "2 friends  ·  1 protected",
        ] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }
    }

    #[test]
    fn expanding_a_friend_shows_the_mode_and_easy_switch() {
        let mut gui = gui_with(
            vec![friend("Alex", FriendMode::Easy, true, vec![])],
            &["Alex"],
        );
        let texts = painted(&mut gui);
        let joined = texts.join("");
        for expected in [
            "EXEMPTIONS",
            "Ignore all player modules",
            "All player modules ignored",
        ] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }
    }

    #[test]
    fn advanced_mode_lists_each_player_module() {
        let mut gui = gui_with(
            vec![friend("Alex", FriendMode::Advanced, true, vec!["KillAura"])],
            &["Alex"],
        );
        let texts = painted(&mut gui);
        let joined = texts.join("");
        for (_, label) in PLAYER_MODULES {
            assert!(joined.contains(label), "missing {label:?} in {joined:?}");
        }
        assert!(joined.contains("1 module ignored"), "got {joined:?}");
    }

    #[test]
    fn an_empty_list_explains_how_to_add_a_friend() {
        let mut gui = gui_with(Vec::new(), &[]);
        let texts = painted(&mut gui);
        let joined = texts.join("");
        for expected in [
            "No friends yet",
            "nobody added yet",
            "Add a username above and the client leaves them alone",
        ] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }
    }
}
