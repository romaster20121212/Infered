 
 
 
 
 
 
 
 
 
 
 
 
 
 

use egui::{
    pos2, vec2, Align, Align2, Color32, CursorIcon, FontId, Id, Layout, Rect, Rounding, ScrollArea,
    Sense, Shape, Stroke, TextureHandle, Ui, UiBuilder, Vec2,
};

use super::theme::{lerp_color, pulse, with_alpha, Palette};
use super::widgets::{
    count_up, ease_out_back, elided, glossy_pill, paint_delete, paint_toggle, radial_glow, sweep,
    top_light_rounded, tracked, PillIcon, PillStyle,
};
use super::{ease_out_cubic, InjectorGui};
use crate::app::AddonInfo;

 
const HEADER_H: f32 = 44.0;
 
const CARD_H: f32 = 56.0;
 
const CARD_GAP: f32 = 8.0;
 
const PILL_SIZE: Vec2 = Vec2::new(124.0, 28.0);

 
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Toggle,
    Delete,
}

 
pub(super) fn page(gui: &mut InjectorGui, ui: &mut Ui, now: f64) {
    let addons: Vec<AddonInfo> = gui.app.addons().to_vec();
    let active = addons.iter().filter(|addon| addon.enabled).count();
    let elapsed = (now - gui.addons_opened_at).max(0.0);
    let (area, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());

    let mut load = header(
        ui,
        area,
        &gui.addons_icon,
        addons.len(),
        active,
        now,
        elapsed,
    );

     
     
     
    let mut top = area.top() + HEADER_H + 10.0;
    if let Some((message, ok)) = gui.addon_notice.clone() {
        let strip = Rect::from_min_size(pos2(area.left(), top), vec2(area.width(), 24.0));
        notice(ui, strip, &message, ok);
        top += 30.0;
    }
    let content = Rect::from_min_max(pos2(area.left(), top), area.max);

    if addons.is_empty() {
        load |= empty(ui, content.shrink2(vec2(0.0, 2.0)), now, elapsed);
    } else {
        let mut action: Option<(String, bool, Action)> = None;
        let mut list = ui.new_child(
            UiBuilder::new()
                .max_rect(content)
                .layout(Layout::top_down(Align::Min)),
        );
        list.set_clip_rect(content);
        ScrollArea::vertical()
            .id_salt("addons_list")
            .auto_shrink([false, false])
            .max_height(content.height())
            .show(&mut list, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, CARD_GAP);
                for (index, addon) in addons.iter().enumerate() {
                    if let Some(clicked) = card(ui, addon, index, now, gui.addons_opened_at) {
                        action = Some((addon.file_name.clone(), addon.enabled, clicked));
                    }
                }
                ui.add_space(6.0);
            });

        if let Some((file_name, enabled, action)) = action {
            match action {
                Action::Toggle => {
                    let _ = gui.app.set_addon_enabled(&file_name, !enabled);
                }
                Action::Delete => {
                    let _ = gui.app.delete_addon(&file_name);
                }
            }
        }
    }

    if load {
        gui.load_addon_from_picker();
         
         
        if matches!(&gui.addon_notice, Some((_, true))) {
            gui.addons_opened_at = now;
        }
    }
}

 

 
 
fn header(
    ui: &Ui,
    area: Rect,
    icon: &TextureHandle,
    total: usize,
    active: usize,
    now: f64,
    elapsed: f64,
) -> bool {
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
        "ADDONS",
        16.0,
        Palette::TEXT,
        Align::Min,
        text_x,
        area.top() + 13.0,
    );

     
     
    let summary = format!(
        "{} LOADED  ·  {} ACTIVE",
        count_up(total, elapsed),
        count_up(active, elapsed)
    );
    let status = if active > 0 {
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

     
    let pill = Rect::from_min_size(
        pos2(area.right() - 2.0 - PILL_SIZE.x, area.top() + 8.0),
        PILL_SIZE,
    );
    let response = ui.interact(pill, Id::new("addons_load_pill"), Sense::click());
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.16);
    let press = ui.ctx().animate_bool_with_time(
        response.id.with("press"),
        response.is_pointer_button_down_on(),
        0.08,
    );
    let appear = ease_out_back(((elapsed - 0.10) / 0.34).clamp(0.0, 1.0) as f32);
    let drawn = pill.translate(vec2(16.0 * (1.0 - appear), 0.0));
    let mut pill_painter = painter.clone();
    pill_painter.set_opacity(draw * appear.clamp(0.0, 1.0));
    glossy_pill(
        &pill_painter,
        drawn,
        PillStyle {
            label: "LOAD ADDON",
            icon: PillIcon::Plus,
            emphasis: 1.0,
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

 
 
fn notice(ui: &Ui, rect: Rect, message: &str, ok: bool) {
    let color = if ok { Palette::OK } else { Palette::ERR };
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(8.0), with_alpha(color, 0.07));
    painter.rect_stroke(
        rect,
        Rounding::same(8.0),
        Stroke::new(1.0_f32, with_alpha(color, 0.28)),
    );
    painter.circle_filled(
        pos2(rect.left() + 13.0, rect.center().y),
        2.6,
        with_alpha(color, 0.95),
    );
    let galley = elided(
        ui,
        message,
        10.0,
        with_alpha(Palette::TEXT, 0.90),
        (rect.width() - 130.0).max(40.0),
    );
    painter.galley(
        pos2(rect.left() + 24.0, rect.center().y - galley.size().y * 0.5),
        galley,
        Palette::TEXT,
    );
    tracked(
        painter,
        if ok { "LOADED" } else { "FAILED" },
        7.0,
        with_alpha(color, 0.85),
        Align::Max,
        rect.right() - 12.0,
        rect.center().y,
    );
}

 

 
 
 
fn empty(ui: &Ui, rect: Rect, now: f64, elapsed: f64) -> bool {
    let appear = ease_out_cubic(((elapsed - 0.10) / 0.40).clamp(0.0, 1.0) as f32);
    let id = Id::new("addons_empty");
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
                0.12 + 0.30 * hover,
            ),
        ),
    );

     
    let arm = 14.0;
    let ink = Stroke::new(1.6_f32, with_alpha(Palette::ACCENT, 0.25 + 0.55 * hover));
    for (sx, sy) in [(-1.0_f32, -1.0_f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let corner = pos2(
            rect.center().x + sx * (rect.width() * 0.5 - 10.0),
            rect.center().y + sy * (rect.height() * 0.5 - 10.0),
        );
        painter.line_segment([corner, corner + vec2(-sx * arm, 0.0)], ink);
        painter.line_segment([corner, corner + vec2(0.0, -sy * arm)], ink);
    }

     
    let centre = pos2(rect.center().x, rect.top() + rect.height() * 0.38);
    let float = 2.5 * (2.0 * pulse(now, 3.2) - 1.0);
    radial_glow(
        &painter,
        centre + vec2(0.0, float),
        62.0,
        Palette::ACCENT,
        0.045 + 0.06 * hover,
    );
    let well = Rect::from_center_size(centre + vec2(0.0, float), Vec2::splat(46.0 + 3.0 * hover));
    painter.rect_filled(
        well,
        Rounding::same(12.0),
        with_alpha(Palette::ACCENT, 0.07 + 0.06 * hover),
    );
    painter.rect_stroke(
        well,
        Rounding::same(12.0),
        Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.30 + 0.35 * hover)),
    );
    let angle = 0.25 * hover;
    let (sin, cos) = angle.sin_cos();
    let rotate = |v: Vec2| vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
    let len = 8.0 + 0.6 * hover;
    let plus_ink = Stroke::new(1.8_f32, with_alpha(Palette::TEXT, 0.60 + 0.30 * hover));
    for axis in [vec2(len, 0.0), vec2(0.0, len)] {
        let c = well.center();
        painter.line_segment([c - rotate(axis), c + rotate(axis)], plus_ink);
    }

    painter.text(
        pos2(centre.x, centre.y + 52.0),
        Align2::CENTER_CENTER,
        "No addons yet",
        FontId::proportional(13.0),
        with_alpha(Palette::TEXT, 0.78 + 0.22 * hover),
    );
    painter.text(
        pos2(centre.x, centre.y + 70.0),
        Align2::CENTER_CENTER,
        "Click to load an .inf, or drop one in the addons folder",
        FontId::proportional(9.5),
        with_alpha(Palette::TEXT_DIM, 0.62 + 0.20 * hover),
    );

     
     
    let cta = Rect::from_center_size(
        pos2(centre.x, rect.bottom() - 28.0 + 1.0 * press),
        vec2(150.0, 26.0),
    );
    painter.rect_filled(
        cta,
        Rounding::same(8.0),
        with_alpha(Palette::ACCENT, 0.10 + 0.30 * hover),
    );
    painter.rect_stroke(
        cta,
        Rounding::same(8.0),
        Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.30 + 0.50 * hover)),
    );
    tracked(
        &painter,
        "BROWSE FILES",
        7.5,
        with_alpha(
            if hover > 0.4 {
                Color32::WHITE
            } else {
                Palette::TEXT
            },
            0.80 + 0.20 * hover,
        ),
        Align::Center,
        cta.center().x,
        cta.center().y,
    );

    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response.clicked()
}

 

 
 
fn card(ui: &mut Ui, addon: &AddonInfo, index: usize, now: f64, opened_at: f64) -> Option<Action> {
     
    let t = ((now - opened_at - index as f64 * 0.05) / 0.40).clamp(0.0, 1.0) as f32;
    if t <= 0.0 {
        ui.ctx().request_repaint();
        return None;
    }
    let appear = ease_out_back(t);
    let fade = ease_out_cubic((t / 0.62).clamp(0.0, 1.0));

    let (slot, _) = ui.allocate_exact_size(vec2(ui.available_width(), CARD_H), Sense::hover());
    let settled = Rect::from_min_size(slot.min, vec2(slot.width(), CARD_H));
    let id = Id::new(("addon_row", addon.file_name.as_str()));
    let response = ui.interact(settled, id, Sense::click());
     
     
    let hover =
        ui.ctx()
            .animate_bool_with_time(id.with("hover"), ui.rect_contains_pointer(settled), 0.16);
    let press = ui.ctx().animate_bool_with_time(
        id.with("press"),
        response.is_pointer_button_down_on(),
        0.09,
    );
    let on = ui
        .ctx()
        .animate_bool_with_time(id.with("on"), addon.enabled, 0.24);

     
    let delete = Rect::from_center_size(
        pos2(settled.right() - 14.0 - 13.0, settled.center().y),
        Vec2::splat(26.0),
    );
    let toggle = Rect::from_center_size(
        pos2(delete.left() - 10.0 - 20.0, settled.center().y),
        vec2(40.0, 22.0),
    );
    let toggle_response = ui.interact(toggle, id.with("toggle"), Sense::click());
    let delete_response = ui.interact(delete, id.with("delete"), Sense::click());
    let toggle_hover = ui.ctx().animate_bool_with_time(
        toggle_response.id.with("hover"),
        toggle_response.hovered(),
        0.12,
    );
    let delete_hover = ui.ctx().animate_bool_with_time(
        delete_response.id.with("hover"),
        delete_response.hovered(),
        0.12,
    );

     
    let rect = settled
        .translate(vec2(-22.0 * (1.0 - appear), 9.0 * (1.0 - appear)))
        .translate(vec2(0.0, -2.0 * hover + 1.0 * press));
    let mut painter = ui.painter().clone();
    painter.set_opacity(fade);

     
    painter.rect_filled(
        rect.translate(vec2(0.0, 3.0)).expand(1.0),
        Rounding::same(14.0),
        with_alpha(Color32::BLACK, 0.30 + 0.10 * hover),
    );
    if hover > 0.01 {
        for (spread, alpha) in [(5.0_f32, 0.040_f32), (2.0, 0.060)] {
            painter.rect_filled(
                rect.expand(spread),
                Rounding::same(15.0),
                with_alpha(Palette::ACCENT, alpha * hover),
            );
        }
    }
    painter.rect_filled(
        rect,
        Rounding::same(11.0),
        lerp_color(Palette::CARD, Palette::CARD_HOVER, hover),
    );
    top_light_rounded(&painter, rect, 11.0, 0.06 + 0.03 * hover);
    if press > 0.01 {
        painter.rect_filled(
            rect,
            Rounding::same(11.0),
            with_alpha(Palette::ACCENT_BRIGHT, 0.05 * press),
        );
    }
    if hover > 0.01 {
        if let Some(pointer) = ui.ctx().pointer_hover_pos() {
            let focus = pos2(
                pointer.x.clamp(rect.left() + 10.0, rect.right() - 10.0),
                pointer.y.clamp(rect.top() + 8.0, rect.bottom() - 8.0),
            );
            radial_glow(
                &painter.with_clip_rect(rect.shrink(1.0)),
                focus,
                88.0,
                Palette::ACCENT,
                0.06 * hover,
            );
        }
    }

    let accent = if addon.enabled {
        Palette::ACCENT
    } else {
        Palette::TEXT_DIM
    };
    painter.line_segment(
        [
            pos2(rect.left() + 14.0, rect.top() + 1.0),
            pos2(rect.right() - 14.0, rect.top() + 1.0),
        ],
        Stroke::new(1.0_f32, with_alpha(Color32::WHITE, 0.05 + 0.03 * hover)),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(11.0),
        Stroke::new(
            1.0_f32,
            with_alpha(accent, 0.14 + 0.30 * hover + 0.16 * press),
        ),
    );
     
    painter.rect_filled(
        Rect::from_center_size(
            pos2(rect.left() + 1.5, rect.center().y),
            vec2(2.5, 16.0 + 20.0 * hover),
        ),
        Rounding::same(1.5),
        with_alpha(
            accent,
            (if addon.enabled { 0.30 } else { 0.16 }) + 0.55 * hover,
        ),
    );

     
    let well = Rect::from_center_size(
        pos2(rect.left() + 14.0 + 19.0, rect.center().y),
        Vec2::splat(38.0 + 0.8 * hover),
    );
    painter.rect_filled(
        well,
        Rounding::same(10.0),
        with_alpha(
            lerp_color(Color32::WHITE, Palette::ACCENT, on),
            0.055 + 0.04 * hover,
        ),
    );
    painter.rect_stroke(
        well,
        Rounding::same(10.0),
        Stroke::new(
            1.0_f32,
            with_alpha(
                lerp_color(Palette::TEXT_DIM, Palette::ACCENT, on),
                0.20 + 0.30 * hover,
            ),
        ),
    );
    addon_chip(&painter, well, on, hover, now);

     
    let text_x = well.right() + 12.0;
    let text_right = toggle.left() - 14.0;
    let width = (text_right - text_x).max(30.0);
    let title = elided(
        ui,
        &addon.title,
        12.5,
        with_alpha(Palette::TEXT, 0.86 + 0.14 * hover),
        width,
    );
    painter.galley(
        pos2(text_x, rect.center().y - 8.0 - title.size().y * 0.5),
        title,
        Palette::TEXT,
    );
    let detail_text = if addon.description.trim().is_empty() {
        addon.file_name.as_str()
    } else {
        addon.description.as_str()
    };
    let detail = elided(
        ui,
        detail_text,
        10.0,
        with_alpha(Palette::TEXT_DIM, 0.62 + 0.18 * hover),
        width,
    );
    painter.galley(
        pos2(text_x, rect.center().y + 8.0 - detail.size().y * 0.5),
        detail,
        Palette::TEXT_DIM,
    );

    paint_toggle(&painter, toggle, on, toggle_hover);
    paint_delete(&painter, delete, delete_hover, 0.25 + 0.75 * hover);
     
    sweep(
        &painter,
        rect.shrink(1.0),
        now + index as f64 * 1.3,
        6.8,
        0.022,
    );

    if response.hovered() || toggle_response.hovered() || delete_response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    if delete_response.clicked() {
        Some(Action::Delete)
    } else if response.clicked() || toggle_response.clicked() {
        Some(Action::Toggle)
    } else {
        None
    }
}

 
 
fn addon_chip(painter: &egui::Painter, well: Rect, on: f32, hover: f32, now: f64) {
    let centre = well.center();
    let scale = well.width() / 38.0;
    let tint = lerp_color(Palette::TEXT_DIM, Palette::ACCENT_BRIGHT, on);
    radial_glow(
        painter,
        centre,
        20.0 * scale,
        Palette::ACCENT,
        0.045 * on + 0.05 * hover,
    );

    let body = Rect::from_center_size(centre, Vec2::splat(18.0 * scale));
    let rounding = Rounding::same(4.0 * scale);
    painter.rect_filled(
        body,
        rounding,
        with_alpha(tint, 0.08 + 0.06 * on + 0.05 * hover),
    );
    painter.rect_stroke(
        body,
        rounding,
        Stroke::new(1.1_f32 * scale, with_alpha(tint, 0.55 + 0.30 * hover)),
    );

    let ink = Stroke::new(1.1_f32 * scale, with_alpha(tint, 0.52 + 0.28 * hover));
    let pin = (2.6 + 1.3 * hover) * scale;
    for i in 0..3 {
        let f = (i as f32 + 1.0) / 4.0;
        let x = body.left() + body.width() * f;
        let y = body.top() + body.height() * f;
        painter.line_segment([pos2(x, body.top() - pin), pos2(x, body.top())], ink);
        painter.line_segment([pos2(x, body.bottom()), pos2(x, body.bottom() + pin)], ink);
        painter.line_segment([pos2(body.left() - pin, y), pos2(body.left(), y)], ink);
        painter.line_segment([pos2(body.right(), y), pos2(body.right() + pin, y)], ink);
    }

    let breathe = 0.5 + 0.5 * (now as f32 * 2.2).sin();
    let radius = (4.2 + 0.5 * breathe * (0.3 + on) + 0.5 * hover) * scale;
    painter.add(Shape::convex_polygon(
        vec![
            centre + vec2(0.0, -radius),
            centre + vec2(radius, 0.0),
            centre + vec2(0.0, radius),
            centre + vec2(-radius, 0.0),
        ],
        with_alpha(tint, 0.70 + 0.25 * on),
        Stroke::new(0.0_f32, Color32::TRANSPARENT),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Pos2, RawInput};

     
    fn painted(mut paint: impl FnMut(&mut Ui)) -> (Vec<String>, Vec<Shape>) {
        let ctx = Context::default();
        crate::gui::theme::apply(&ctx);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(670.0, 278.0));
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
                    .show(ctx, |ui| paint(ui));
            },
        );
        let mut texts = Vec::new();
        let mut shapes = Vec::new();
        for clipped in output.shapes {
            match clipped.shape {
                Shape::Text(text) => texts.push(text.galley.text().to_owned()),
                other => shapes.push(other),
            }
        }
        (texts, shapes)
    }

    fn addon(title: &str, description: &str, enabled: bool) -> AddonInfo {
        AddonInfo {
            file_name: format!("{}.inf", title.to_lowercase().replace(' ', "_")),
            title: title.to_owned(),
            description: description.to_owned(),
            enabled,
        }
    }

    fn icon(ctx: &Context) -> TextureHandle {
        ctx.load_texture(
            "test_addons_icon",
            egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255, 255, 255, 255]),
            Default::default(),
        )
    }

    #[test]
    fn the_header_shows_the_title_counts_and_load_button() {
        let (texts, _) = painted(|ui| {
            let ctx = ui.ctx();
            let icon = icon(ctx);
            let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(670.0, 278.0));
            let _ = header(ui, area, &icon, 3, 2, 5.0, 10.0);
        });
        let joined = texts.join("");
        for expected in ["ADDONS", "3 LOADED  ·  2 ACTIVE", "LOAD ADDON"] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }
    }

    #[test]
    fn a_card_paints_its_title_description_switch_and_chip() {
        let (texts, shapes) = painted(|ui| {
            let addon = addon("Bunny Hop", "Jump higher than the server likes", true);
            let _ = card(ui, &addon, 0, 5.0, 0.0);
        });
        let joined = texts.join("");
        assert!(joined.contains("Bunny Hop"), "got {joined:?}");
        assert!(
            joined.contains("Jump higher than the server likes"),
            "got {joined:?}"
        );
         
        assert!(
            shapes.iter().any(|shape| matches!(shape, Shape::Circle(_))),
            "the card must paint its switch"
        );
        assert!(
            shapes
                .iter()
                .any(|shape| matches!(shape, Shape::Path(path) if path.closed && path.fill != Color32::TRANSPARENT)),
            "the chip's diamond core must be filled"
        );
    }

    #[test]
    fn a_card_without_a_description_shows_its_file_name() {
        let (texts, _) = painted(|ui| {
            let addon = addon("Quiet Pack", "   ", false);
            let _ = card(ui, &addon, 0, 5.0, 0.0);
        });
        assert!(texts.join("").contains("quiet_pack.inf"), "got {texts:?}");
    }

    #[test]
    fn the_empty_shelf_explains_how_to_load() {
        let (texts, _) = painted(|ui| {
            let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(670.0, 210.0));
            let _ = empty(ui, area, 5.0, 10.0);
        });
        let joined = texts.join("");
        for expected in ["No addons yet", "Click to load an .inf", "BROWSE FILES"] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }
    }

     
     
    #[test]
    fn the_page_lists_the_shelf_and_its_summary() {
        let mut gui = InjectorGui::stub_gui();
        gui.drawer_page = super::super::DrawerPage::Addons;
        gui.addons_opened_at = -10.0;
        gui.app.stub_addons(vec![
            addon("Bunny Hop", "Jump higher", true),
            addon("Auto Fish", "Reels them in", false),
        ]);
        let (texts, _) = painted(|ui| page(&mut gui, ui, 10.0));
        let joined = texts.join("");
        for expected in ["ADDONS", "2 LOADED  ·  1 ACTIVE", "Bunny Hop", "Auto Fish"] {
            assert!(
                joined.contains(expected),
                "missing {expected:?} in {joined:?}"
            );
        }
    }
}
