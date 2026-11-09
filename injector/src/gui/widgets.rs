 
 
 
 
 
 

use egui::{
    epaint::Mesh, pos2, vec2, Align, Align2, Color32, CursorIcon, FontId, Painter, Pos2, Rect,
    Response, RichText, Rounding, Sense, Shape, Stroke, TextureHandle, Ui, Vec2,
};

use super::theme::{lerp_color, pulse, with_alpha, Palette, TITLE_BAR_HEIGHT};
use crate::app::InjectorApp;

 
 
 
 
 
pub fn title_bar(
    ui: &mut Ui,
    sidebar_icon: &TextureHandle,
    minimize_icon: &TextureHandle,
    close_icon: &TextureHandle,
) -> bool {
    let (bar, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), TITLE_BAR_HEIGHT),
        Sense::hover(),
    );
    let ctx = ui.ctx().clone();

     
    let drag = ui.interact(bar, ui.id().with("bar_drag"), Sense::drag());
    if drag.drag_started() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

     
     
    let radius = 15.0;
    let addons_center = pos2(bar.left() + 22.0, bar.center().y);
    let addons_rect = Rect::from_center_size(addons_center, Vec2::splat(radius * 2.0));
    let addons_resp = ui.allocate_rect(addons_rect, Sense::click());
    paint_png_button(ui, &addons_resp, addons_center, sidebar_icon);

     
    let gap = 8.0;
    let cy = bar.center().y;
    let right = bar.right() - 12.0;
    let close_c = pos2(right - radius, cy);
    let min_c = pos2(close_c.x - radius * 2.0 - gap, cy);

    let min_rect = Rect::from_center_size(min_c, Vec2::splat(radius * 2.0));
    let min_resp = ui.allocate_rect(min_rect, Sense::click());
    paint_png_button(ui, &min_resp, min_c, minimize_icon);
    if min_resp.clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }

    let close_rect = Rect::from_center_size(close_c, Vec2::splat(radius * 2.0));
    let close_resp = ui.allocate_rect(close_rect, Sense::click());
    paint_png_button(ui, &close_resp, close_c, close_icon);
    if close_resp.clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    addons_resp.clicked()
}

 
pub fn paint_png_button(ui: &mut Ui, resp: &Response, center: Pos2, texture: &TextureHandle) {
    let hover =
        ui.ctx()
            .animate_bool_with_time(resp.id.with("png_button_hover"), resp.hovered(), 0.20);
     
     
    let scale = 1.0 + 0.20 * hover;
    let painter = ui.painter();
    let icon_size = 25.0 * scale;
    let icon_rect = Rect::from_center_size(center, Vec2::splat(icon_size));
    painter.image(
        texture.id(),
        icon_rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        with_alpha(Color32::WHITE, 0.92 + 0.08 * hover),
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
}

 
 
 
pub fn logo_banner(ui: &mut Ui, textlogo: &TextureHandle) {
    let time = ui.input(|i| i.time);
     
    let (slot, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 140.0), Sense::hover());
    let tex = textlogo.size_vec2();
    let scale = 1.0 + 0.025 * (2.0 * pulse(time, 3.4) - 1.0);
    let width = 340.0 * scale;
    let rect = Rect::from_center_size(slot.center(), vec2(width, width * tex.y / tex.x));

    ui.painter().image(
        textlogo.id(),
        rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );

     
    let line_w = 110.0;
    let line_y = slot.bottom() - 2.0;
    let line = Rect::from_center_size(
        pos2(slot.center().x, line_y),
        Vec2::new(line_w * (0.9 + 0.1 * pulse(time, 3.4)), 2.0),
    );
    ui.painter().rect_filled(
        line,
        Rounding::same(1.0),
        with_alpha(Palette::ACCENT, 0.35 + 0.3 * pulse(time, 3.4)),
    );

     
    let attr_y = slot.bottom() + 4.0;
    ui.painter().text(
        pos2(slot.center().x, attr_y),
        Align2::CENTER_TOP,
        "by thefinalblow",
        FontId::proportional(10.0),
        Color32::RED,
    );
}

 
 
 
pub fn process_picker(ui: &mut Ui, app: &mut InjectorApp, open: &mut bool) {
    let width = 300.0;
    let busy = app.is_busy();
    if busy {
        *open = false;
    }
    let (rect, response) = ui.allocate_exact_size(vec2(width, 34.0), Sense::click());
    let response = response.on_hover_text("Choose which Minecraft to inject into");
    let id = response.id;
    let hover =
        ui.ctx()
            .animate_bool_with_time(id.with("hover"), response.hovered() && !busy, 0.14);

    let selected = app.selected_process().map(|p| (p.pid, p.info.clone()));
    let running = !app.processes().is_empty();
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Rounding::same(10.0),
        with_alpha(Palette::CARD, 0.6 + 0.25 * hover),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(10.0),
        Stroke::new(
            1.0_f32,
            with_alpha(
                if *open || hover > 0.5 {
                    Palette::ACCENT
                } else {
                    Palette::TEXT_DIM
                },
                0.18 + 0.28 * hover + if *open { 0.2 } else { 0.0 },
            ),
        ),
    );

     
    let dot_color = if selected.is_some() {
        Palette::OK
    } else {
        with_alpha(Palette::TEXT_DIM, 0.5)
    };
    painter.circle_filled(pos2(rect.left() + 14.0, rect.center().y), 3.5, dot_color);

    let label = match &selected {
        Some((pid, info)) => format!("Minecraft {info}  ·  pid {pid}"),
        None if running => "select a Minecraft".to_string(),
        None => "no Minecraft running".to_string(),
    };
    painter.text(
        pos2(rect.left() + 26.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        with_alpha(Palette::TEXT, 0.75 + 0.2 * hover),
    );

     
    let ink = with_alpha(Palette::TEXT_DIM, 0.85 + 0.15 * hover);
    let chevron = pos2(
        rect.right() - 14.0,
        rect.center().y + if *open { 1.2 } else { -1.2 },
    );
    let down = if *open { -1.0_f32 } else { 1.0 };
    painter.line_segment(
        [
            chevron + vec2(-3.2, -1.4 * down),
            chevron + vec2(0.0, 1.6 * down),
        ],
        Stroke::new(1.2_f32, ink),
    );
    painter.line_segment(
        [
            chevron + vec2(3.2, -1.4 * down),
            chevron + vec2(0.0, 1.6 * down),
        ],
        Stroke::new(1.2_f32, ink),
    );

    if response.hovered() && !busy {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if response.clicked() && !busy {
        *open = !*open;
        if *open {
             
            app.scan();
        }
    }
    if !*open {
        return;
    }

    let ctx = ui.ctx().clone();
    let mut chosen = None;
    let menu = egui::Area::new(id.with("menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(rect.left(), rect.bottom() + 4.0))
        .show(&ctx, |ui| {
            egui::Frame::none()
                .fill(Palette::BG)
                .stroke(Stroke::new(1.0_f32, with_alpha(Palette::ACCENT, 0.30)))
                .rounding(Rounding::same(10.0))
                .inner_margin(egui::Margin::same(5.0))
                .show(ui, |ui| {
                    ui.set_width(width);
                    ui.spacing_mut().item_spacing.y = 3.0;
                    if app.processes().is_empty() {
                        let (row, _) =
                            ui.allocate_exact_size(vec2(width - 10.0, 34.0), Sense::hover());
                        ui.painter().text(
                            pos2(row.left() + 10.0, row.center().y),
                            Align2::LEFT_CENTER,
                            "no Minecraft running — start the game",
                            FontId::proportional(10.0),
                            with_alpha(Palette::TEXT_DIM, 0.9),
                        );
                    }
                    for proc in app.processes() {
                        let (row, response) =
                            ui.allocate_exact_size(vec2(width - 10.0, 34.0), Sense::click());
                        let active = selected.as_ref().is_some_and(|(pid, _)| *pid == proc.pid);
                        if response.hovered() {
                            ui.painter().rect_filled(
                                row,
                                Rounding::same(7.0),
                                with_alpha(Palette::ACCENT, 0.14),
                            );
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        }
                        if active {
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    pos2(row.left() + 1.0, row.center().y - 5.0),
                                    vec2(2.0, 10.0),
                                ),
                                Rounding::same(1.0),
                                Palette::ACCENT,
                            );
                        }
                        let text_color = if active {
                            Palette::TEXT
                        } else {
                            with_alpha(Palette::TEXT_DIM, 0.95)
                        };
                        ui.painter().text(
                            pos2(row.left() + 12.0, row.center().y - 6.0),
                            Align2::LEFT_CENTER,
                            format!("Minecraft {}", proc.info),
                            FontId::proportional(10.5),
                            text_color,
                        );
                        ui.painter().text(
                            pos2(row.left() + 12.0, row.center().y + 7.0),
                            Align2::LEFT_CENTER,
                            format!("pid {}", proc.pid),
                            FontId::proportional(8.5),
                            with_alpha(Palette::TEXT_DIM, 0.6),
                        );
                        if response.clicked() {
                            chosen = Some(proc.pid);
                        }
                    }
                });
        });

    if ctx.input(|input| input.pointer.any_pressed()) {
        let pointer = ctx.pointer_interact_pos();
        let inside =
            pointer.is_some_and(|point| menu.response.rect.contains(point) || rect.contains(point));
        if !inside {
            *open = false;
        }
    }
    if let Some(pid) = chosen {
        app.select(pid);
        *open = false;
    }
}

 
 
 
 
 
 
 
 
 
pub fn inject_button(ui: &mut Ui, app: &InjectorApp) -> Response {
    let busy = app.is_busy();
    let time = ui.input(|i| i.time);
    let size = Vec2::new(300.0, 52.0);
     
     
    let (slot, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), size.y), Sense::hover());
    let rect = Rect::from_center_size(slot.center(), size);
    let response = ui.allocate_rect(rect, Sense::click());
    let id = response.id;

    let hover =
        ui.ctx()
            .animate_bool_with_time(id.with("hover"), response.hovered() && !busy, 0.15);
    let press = ui.ctx().animate_bool_with_time(
        id.with("press"),
        response.is_pointer_button_down_on() && !busy,
        0.08,
    );

    let painter = ui.painter();
    let rounding = Rounding::same(14.0);

     
    let scale = 1.0 + 0.008 * pulse(time, 2.6) + 0.035 * hover;
    let body_rect = Rect::from_center_size(rect.center(), size * scale).shrink(1.5 * press);

     
    let breathe = 0.55 + 0.45 * pulse(time, 2.6);
    let glow = (0.35 + 0.45 * breathe) * (1.0 + 0.9 * hover) * if busy { 0.7 } else { 1.0 };
    let flash = 0.35 * press;
    for (spread, alpha) in [(22.0, 0.07), (14.0, 0.12), (8.0, 0.20), (3.0, 0.30)] {
        painter.rect_filled(
            body_rect.expand(spread),
            Rounding::same(24.0),
            with_alpha(
                if press > 0.1 {
                    Palette::ACCENT_BRIGHT
                } else {
                    Palette::ACCENT
                },
                (alpha * glow + flash * alpha).min(0.65),
            ),
        );
    }

     
    let outline_alpha = 0.45 + 0.35 * pulse(time, 2.6) + 0.2 * hover;
    painter.rect_stroke(
        body_rect.expand(1.0),
        Rounding::same(15.0),
        egui::Stroke::new(1.4_f32, with_alpha(Palette::ACCENT_BRIGHT, outline_alpha)),
    );

     
    let runner_speed = if busy { 0.55 } else { 0.22 } * (1.0 + 1.5 * hover) as f64;
    let runners = 3;
    for k in 0..runners {
        let t0 = (time * runner_speed + k as f64 / runners as f64).rem_euclid(1.0);
         
        for (i, step) in [0.0, 0.010, 0.020, 0.030].iter().enumerate() {
            let tt = (t0 - step).rem_euclid(1.0);
            let p = rounded_rect_point(body_rect, 14.0, tt as f32);
            let fade = 1.0 - i as f32 / 4.0;
            let twinkle = 0.65 + 0.35 * ((time * 7.0 + k as f64).sin() as f32).abs();
            painter.circle_filled(
                p,
                1.2 + 1.4 * fade,
                with_alpha(Color32::WHITE, (0.9 * fade * twinkle).min(1.0)),
            );
        }
    }

     
    let mut body = lerp_color(Palette::ACCENT, Palette::ACCENT_BRIGHT, 0.30 * hover);
    body = lerp_color(body, Palette::ACCENT_DEEP, press * 0.55);
    if busy {
        body = lerp_color(body, Palette::PANEL, 0.30);
    }
    painter.rect_filled(body_rect, rounding, body);

     
    let inset = body_rect.shrink(2.0);
    let gloss_h = inset.height() * 0.42;
    let gloss = Rect::from_min_size(inset.min, Vec2::new(inset.width(), gloss_h));
    painter.rect_filled(
        gloss,
        Rounding {
            nw: 12.0,
            ne: 12.0,
            sw: 8.0,
            se: 8.0,
        },
        with_alpha(Color32::WHITE, 0.13 + 0.09 * hover),
    );
    let gloss2_h = inset.height() * 0.22;
    let gloss2 = Rect::from_min_size(inset.min, Vec2::new(inset.width(), gloss2_h));
    painter.rect_filled(
        gloss2,
        Rounding {
            nw: 12.0,
            ne: 12.0,
            sw: 6.0,
            se: 6.0,
        },
        with_alpha(Color32::WHITE, 0.09 + 0.07 * hover),
    );

     
    let shade = Rect::from_min_size(
        pos2(inset.left(), inset.center().y),
        Vec2::new(inset.width(), inset.height() * 0.55),
    );
    painter.rect_filled(
        shade,
        Rounding {
            nw: 8.0,
            ne: 8.0,
            sw: 12.0,
            se: 12.0,
        },
        with_alpha(Color32::BLACK, 0.16),
    );

     
    if busy {
        hazard_stripes(painter, body_rect.shrink(3.0), time);
    }

     
    if busy {
        sweep(painter, body_rect, time, 1.4, 0.10);
    } else {
        sweep(painter, body_rect, time, 3.6, 0.05);
    }

     
    painter.line_segment(
        [
            pos2(body_rect.left() + 16.0, body_rect.top() + 1.5),
            pos2(body_rect.right() - 16.0, body_rect.top() + 1.5),
        ],
        egui::Stroke::new(1.2_f32, with_alpha(Color32::WHITE, 0.30 + 0.15 * hover)),
    );
    painter.rect_stroke(
        body_rect.shrink(1.0),
        Rounding::same(13.0),
        egui::Stroke::new(1.0_f32, with_alpha(Color32::BLACK, 0.25)),
    );

     
    let label = if busy {
        format!("INJECTING{}", ".".repeat(1 + ((time * 2.5) % 3.0) as usize))
    } else {
         
         
         
        "INJECT".to_owned()
    };
    let text_alpha = if busy {
        0.75 + 0.25 * pulse(time, 1.2)
    } else {
        1.0
    };
    painter.text(
        body_rect.center() + vec2(0.0, 1.5),
        Align2::CENTER_CENTER,
        &label,
        FontId::proportional(17.0),
        with_alpha(Color32::BLACK, 0.45),
    );
    painter.text(
        body_rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(17.0),
        with_alpha(Color32::WHITE, text_alpha),
    );
    if busy {
        response
    } else {
        response.on_hover_cursor(CursorIcon::PointingHand)
    }
}

 
 
fn rounded_rect_point(rect: Rect, r: f32, t: f32) -> Pos2 {
    use std::f32::consts::FRAC_PI_2;
    let r = r.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let straight_x = rect.width() - 2.0 * r;
    let straight_y = rect.height() - 2.0 * r;
    let arc = FRAC_PI_2 * r;
    let total = 2.0 * straight_x + 2.0 * straight_y + 4.0 * arc;
    let mut d = t.rem_euclid(1.0) * total;

     
    if d < straight_x {
        return pos2(rect.left() + r + d, rect.top());
    }
    d -= straight_x;
     
    if d < arc {
        let a = -FRAC_PI_2 + d / r;
        return pos2(rect.right() - r + a.cos() * r, rect.top() + r + a.sin() * r);
    }
    d -= arc;
     
    if d < straight_y {
        return pos2(rect.right(), rect.top() + r + d);
    }
    d -= straight_y;
     
    if d < arc {
        let a = d / r;
        return pos2(
            rect.right() - r + a.cos() * r,
            rect.bottom() - r + a.sin() * r,
        );
    }
    d -= arc;
     
    if d < straight_x {
        return pos2(rect.right() - r - d, rect.bottom());
    }
    d -= straight_x;
     
    if d < arc {
        let a = FRAC_PI_2 + d / r;
        return pos2(
            rect.left() + r + a.cos() * r,
            rect.bottom() - r + a.sin() * r,
        );
    }
    d -= arc;
     
    if d < straight_y {
        return pos2(rect.left(), rect.bottom() - r - d);
    }
    d -= straight_y;
     
    let a = std::f32::consts::PI + d / r;
    pos2(rect.left() + r + a.cos() * r, rect.top() + r + a.sin() * r)
}

 
 
fn hazard_stripes(painter: &Painter, rect: Rect, time: f64) {
    let clip = painter.with_clip_rect(rect);
    let spacing = 26.0_f32;
    let period = 1.4_f64;
    let offset = ((time % period) / period) as f32 * spacing * 2.0;
    let stroke = egui::Stroke::new(9.0_f32, with_alpha(Color32::WHITE, 0.055));
    let reach = rect.width() + rect.height() + 80.0;
    let mut x = -reach;
    while x < reach {
        let cx = rect.left() + x + offset;
        clip.line_segment(
            [
                pos2(cx, rect.bottom() + 24.0),
                pos2(cx + rect.height() + 48.0, rect.top() - 24.0),
            ],
            stroke,
        );
        x += spacing * 2.0;
    }
}

 
pub(super) fn sweep(painter: &Painter, rect: Rect, time: f64, period_secs: f64, strength: f32) {
    let band_w = (rect.width() * 0.35).max(40.0);
    let progress = ((time % period_secs) / period_secs) as f32;
    let x = rect.left() - band_w + (rect.width() + 2.0 * band_w) * progress;
    let band = Rect::from_min_size(pos2(x, rect.top()), Vec2::new(band_w, rect.height()));

    let mid = with_alpha(Color32::WHITE, strength);
    let mut mesh = Mesh::default();
    mesh.colored_vertex(pos2(band.left(), rect.top()), Color32::TRANSPARENT);
    mesh.colored_vertex(pos2(band.center().x, rect.top()), mid);
    mesh.colored_vertex(pos2(band.right(), rect.top()), Color32::TRANSPARENT);
    mesh.colored_vertex(pos2(band.left(), rect.bottom()), Color32::TRANSPARENT);
    mesh.colored_vertex(pos2(band.center().x, rect.bottom()), mid);
    mesh.colored_vertex(pos2(band.right(), rect.bottom()), Color32::TRANSPARENT);
    mesh.add_triangle(0, 1, 3);
    mesh.add_triangle(1, 4, 3);
    mesh.add_triangle(1, 2, 4);
    mesh.add_triangle(2, 5, 4);
    painter.with_clip_rect(rect).add(Shape::mesh(mesh));
}

 

 
 
pub(super) fn ease_out_back(t: f32) -> f32 {
    const C1: f32 = 1.701_58;
    let t = t.clamp(0.0, 1.0) - 1.0;
    1.0 + (C1 + 1.0) * t * t * t + C1 * t * t
}

 
pub(super) fn elided(
    ui: &Ui,
    text: &str,
    size: f32,
    color: Color32,
    max_width: f32,
) -> std::sync::Arc<egui::Galley> {
    egui::WidgetText::from(RichText::new(text).size(size).color(color)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        max_width,
        egui::TextStyle::Body,
    )
}

 
const TRACKING: f32 = 0.16;

 
 
pub(super) fn tracked_width(painter: &Painter, text: &str, size: f32) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let font = FontId::proportional(size);
    let glyphs: f32 = text
        .chars()
        .map(|ch| {
            painter
                .layout_no_wrap(ch.to_string(), font.clone(), Color32::WHITE)
                .size()
                .x
        })
        .sum();
    glyphs + size * TRACKING * (text.chars().count() - 1) as f32
}

 
 
 
pub(super) fn tracked(
    painter: &Painter,
    text: &str,
    size: f32,
    color: Color32,
    align: Align,
    anchor_x: f32,
    centre_y: f32,
) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let font = FontId::proportional(size);
    let galleys: Vec<_> = text
        .chars()
        .map(|ch| painter.layout_no_wrap(ch.to_string(), font.clone(), color))
        .collect();
    let tracking = size * TRACKING;
    let total =
        galleys.iter().map(|g| g.size().x).sum::<f32>() + tracking * (galleys.len() - 1) as f32;
    let height = galleys.iter().map(|g| g.size().y).fold(0.0_f32, f32::max);
    let mut x = match align {
        Align::Min => anchor_x,
        Align::Center => anchor_x - total * 0.5,
        Align::Max => anchor_x - total,
    };
    let y = centre_y - height * 0.5;
    for galley in galleys {
        let advance = galley.size().x;
        painter.galley(pos2(x, y), galley, color);
        x += advance + tracking;
    }
    total
}

 
 
pub(super) fn radial_glow(
    painter: &Painter,
    centre: Pos2,
    radius: f32,
    color: Color32,
    alpha: f32,
) {
    if alpha <= 0.001 || radius <= 0.0 {
        return;
    }
    const SEGMENTS: usize = 24;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(centre, with_alpha(color, alpha));
    let rim = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0);
    for i in 0..=SEGMENTS {
        let angle = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(centre + vec2(angle.cos(), angle.sin()) * radius, rim);
    }
    for i in 0..SEGMENTS {
        mesh.add_triangle(0, 1 + i as u32, 2 + i as u32);
    }
    painter.add(Shape::mesh(mesh));
}

 
 
 
const SHOULDER: f32 = 10.0;

 
 
 
 
 
 
 
pub(super) fn top_light_rounded(painter: &Painter, rect: Rect, radius: f32, strength: f32) {
    if !strength.is_finite() || strength <= 0.0 || rect.width() <= 2.0 || rect.height() <= 2.0 {
        return;
    }
    let radius = radius
        .min(rect.width() * 0.5)
        .min(rect.height() * 0.5)
        .max(0.0);
    let path = top_light_path(rect, radius);
    let last = path.len().saturating_sub(1);
    if last == 0 {
        return;
    }

     
     
     
    let fall = (radius + 8.0).min(rect.height() * 0.5).max(6.0);
    let mut mesh = Mesh::default();
    for (point, profile) in &path {
        mesh.colored_vertex(
            *point,
            with_alpha(Color32::WHITE, strength * (0.35 + 0.65 * profile)),
        );
        mesh.colored_vertex(
            pos2(point.x, point.y + fall),
            with_alpha(Color32::WHITE, 0.0),
        );
    }
    for index in 0..last as u32 {
        let base = index * 2;
        mesh.add_triangle(base, base + 1, base + 3);
        mesh.add_triangle(base, base + 3, base + 2);
    }
    painter.add(Shape::mesh(mesh));

     
     
     
    let ink = (strength * 1.6).min(1.0);
    for pair in path.windows(2) {
        let profile = (pair[0].1 + pair[1].1) * 0.5;
        if profile <= 0.001 {
            continue;
        }
        painter.line_segment(
            [pair[0].0, pair[1].0],
            Stroke::new(
                1.0_f32,
                with_alpha(Color32::WHITE, ink * (0.25 + 0.75 * profile)),
            ),
        );
    }
}

 
 
 
 
fn top_light_path(rect: Rect, radius: f32) -> Vec<(Pos2, f32)> {
    let shoulder = SHOULDER.min((rect.height() - radius).max(0.0));
     
    const LEG_STEPS: usize = 4;

    let mut path: Vec<(Pos2, f32)> = Vec::with_capacity(2 * LEG_STEPS + 26);
    for step in (1..=LEG_STEPS).rev() {
        let t = step as f32 / LEG_STEPS as f32;
        path.push((
            pos2(rect.left(), rect.top() + radius + shoulder * t),
            shoulder_fade(t),
        ));
    }
    path.extend(top_contour(rect, radius).into_iter().map(|p| (p, 1.0)));
    for step in 1..=LEG_STEPS {
        let t = step as f32 / LEG_STEPS as f32;
        path.push((
            pos2(rect.right(), rect.top() + radius + shoulder * t),
            shoulder_fade(t),
        ));
    }
    path
}

 
fn shoulder_fade(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - t * t * (3.0 - 2.0 * t)
}

 
 
fn top_contour(rect: Rect, radius: f32) -> Vec<Pos2> {
    if radius <= 0.5 {
        return vec![rect.left_top(), rect.right_top()];
    }
     
     
    let steps = ((radius * 0.9).round() as usize).clamp(3, 12);
    let mut points = Vec::with_capacity(2 * steps + 2);
    let left = pos2(rect.left() + radius, rect.top() + radius);
    for step in 0..=steps {
        let angle =
            std::f32::consts::PI + std::f32::consts::FRAC_PI_2 * (step as f32 / steps as f32);
        points.push(pos2(
            left.x + radius * angle.cos(),
            left.y + radius * angle.sin(),
        ));
    }
    let right = pos2(rect.right() - radius, rect.top() + radius);
    for step in 1..=steps {
        let angle =
            std::f32::consts::PI * 1.5 + std::f32::consts::FRAC_PI_2 * (step as f32 / steps as f32);
        points.push(pos2(
            right.x + radius * angle.cos(),
            right.y + radius * angle.sin(),
        ));
    }
    points
}

 
 
pub(super) fn count_up(value: usize, entered: f64) -> usize {
    let t = ((entered - 0.10) / 0.46).clamp(0.0, 1.0);
    (value as f32 * super::ease_out_cubic(t as f32)).round() as usize
}

 
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PillIcon {
    None,
    Plus,
}

 
pub(super) struct PillStyle<'a> {
    pub label: &'a str,
    pub icon: PillIcon,
     
    pub emphasis: f32,
}

 
 
 
 
pub(super) fn glossy_pill(
    painter: &Painter,
    rect: Rect,
    style: PillStyle<'_>,
    hover: f32,
    press: f32,
    now: f64,
) {
    let emphasis = style.emphasis.clamp(0.0, 1.0);
    let breathe = 0.55 + 0.45 * pulse(now, 3.0);
    if emphasis > 0.01 {
        for (spread, alpha) in [(10.0_f32, 0.045_f32), (5.0, 0.07), (2.0, 0.10)] {
            painter.rect_filled(
                rect.expand(spread * (0.5 + 0.5 * hover)),
                Rounding::same(14.0),
                with_alpha(
                    Palette::ACCENT,
                    alpha * emphasis * (0.45 + 0.55 * hover) * (0.6 + 0.4 * breathe),
                ),
            );
        }
    }

    let body = rect
        .translate(vec2(0.0, 1.0 * press))
        .shrink2(vec2(0.0, 1.0 * press));
    let base = lerp_color(
        with_alpha(Palette::CARD, 0.85),
        Palette::ACCENT_DEEP,
        emphasis,
    );
    painter.rect_filled(
        body,
        Rounding::same(9.0),
        lerp_color(base, Palette::ACCENT, (0.30 + 0.50 * hover) * emphasis),
    );
    painter.rect_filled(
        Rect::from_min_size(
            body.min + vec2(1.0, 1.0),
            vec2(body.width() - 2.0, body.height() * 0.45),
        ),
        Rounding {
            nw: 8.0,
            ne: 8.0,
            sw: 4.0,
            se: 4.0,
        },
        with_alpha(Color32::WHITE, 0.02 + (0.09 + 0.06 * hover) * emphasis),
    );
    painter.rect_stroke(
        body,
        Rounding::same(9.0),
        Stroke::new(
            1.0_f32,
            with_alpha(
                lerp_color(Palette::TEXT_DIM, Palette::ACCENT_BRIGHT, emphasis),
                0.22 + 0.35 * hover,
            ),
        ),
    );
    if emphasis > 0.5 {
        sweep(painter, body, now, 2.8, 0.05 * emphasis);
    }

    let ink = with_alpha(
        if emphasis > 0.6 {
            Color32::WHITE
        } else {
            Palette::TEXT
        },
        0.78 + 0.22 * hover,
    );
    match style.icon {
        PillIcon::None => {
            tracked(
                painter,
                style.label,
                7.5,
                ink,
                Align::Center,
                body.center().x,
                body.center().y,
            );
        }
        PillIcon::Plus => {
            let plus = pos2(body.left() + 18.0, body.center().y);
            let angle = 0.22 * hover;
            let (sin, cos) = angle.sin_cos();
            let rotate = |v: Vec2| vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
            let len = 4.6 + 0.5 * hover;
            for axis in [vec2(len, 0.0), vec2(0.0, len)] {
                painter.line_segment(
                    [plus - rotate(axis), plus + rotate(axis)],
                    Stroke::new(1.6_f32, ink),
                );
            }
            tracked(
                painter,
                style.label,
                7.5,
                ink,
                Align::Min,
                plus.x + 12.0,
                body.center().y,
            );
        }
    }
}

 
 
 
pub(super) fn paint_toggle(painter: &Painter, rect: Rect, on: f32, hover: f32) {
    let rounding = Rounding::same(rect.height() * 0.5);
    painter.rect_filled(
        rect.expand(2.5),
        Rounding::same(14.0),
        with_alpha(Palette::ACCENT, 0.12 * on),
    );
    painter.rect_filled(
        rect,
        rounding,
        lerp_color(
            lerp_color(Palette::CARD_HOVER, Palette::ACCENT_DEEP, on),
            Palette::ACCENT,
            0.25 * hover * on,
        ),
    );
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(
            1.0_f32,
            with_alpha(
                if on > 0.5 {
                    Palette::ACCENT_BRIGHT
                } else {
                    Palette::TEXT_DIM
                },
                0.25 + 0.30 * hover,
            ),
        ),
    );
    let radius = rect.height() * 0.5 - 3.5 + 0.4 * hover;
    let travel = rect.width() - 8.0 - 2.0 * (rect.height() * 0.5 - 3.5);
    let knob = pos2(rect.left() + 4.0 + radius + travel * on, rect.center().y);
    painter.circle_filled(
        knob + vec2(0.0, 1.0),
        radius,
        with_alpha(Color32::BLACK, 0.30),
    );
    painter.circle_filled(knob, radius, with_alpha(Palette::TEXT, 0.92));
}

 
 
pub(super) fn paint_delete(painter: &Painter, rect: Rect, hover: f32, alpha: f32) {
    if alpha <= 0.01 {
        return;
    }
    let tint = lerp_color(Palette::TEXT_DIM, Palette::ERR, hover);
    let ink = with_alpha(tint, alpha * (0.55 + 0.45 * hover));
    painter.rect_filled(
        rect,
        Rounding::same(8.0),
        with_alpha(Palette::ERR, 0.12 * hover * alpha),
    );
    let c = rect.center();
     
    painter.line_segment(
        [c + vec2(-6.0, -4.0), c + vec2(6.0, -4.0)],
        Stroke::new(1.35_f32, ink),
    );
    painter.line_segment(
        [c + vec2(-2.6, -6.2), c + vec2(2.6, -6.2)],
        Stroke::new(1.35_f32, ink),
    );
    painter.line_segment(
        [c + vec2(-2.6, -6.2), c + vec2(-2.6, -4.0)],
        Stroke::new(1.35_f32, ink),
    );
    painter.line_segment(
        [c + vec2(2.6, -6.2), c + vec2(2.6, -4.0)],
        Stroke::new(1.35_f32, ink),
    );
     
    painter.rect_stroke(
        Rect::from_min_max(c + vec2(-4.8, -2.0), c + vec2(4.8, 6.4)),
        Rounding {
            nw: 1.5,
            ne: 1.5,
            sw: 2.5,
            se: 2.5,
        },
        Stroke::new(1.35_f32, ink),
    );
    for dx in [-1.8, 1.8] {
        painter.line_segment(
            [c + vec2(dx, 0.6), c + vec2(dx, 4.0)],
            Stroke::new(1.0_f32, ink),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

     
     
     
     
    #[test]
    fn the_top_light_follows_the_corner_arcs() {
        let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(200.0, 60.0));
        let radius = 12.0;
        let points = top_contour(rect, radius);
        assert!(points.len() > 10, "the arcs are sampled, not skipped");

         
        let start = points[0];
        let end = *points.last().unwrap();
        assert!(
            (start.x - rect.left()).abs() < 0.01 && (start.y - (rect.top() + radius)).abs() < 0.01
        );
        assert!(
            (end.x - rect.right()).abs() < 0.01 && (end.y - (rect.top() + radius)).abs() < 0.01
        );

         
         
        let left_c = pos2(rect.left() + radius, rect.top() + radius);
        let right_c = pos2(rect.right() - radius, rect.top() + radius);
        for point in &points {
            let on_top = (point.y - rect.top()).abs() < 0.01
                && point.x >= left_c.x - 0.01
                && point.x <= right_c.x + 0.01;
            let from_left = ((point.x - left_c.x).powi(2) + (point.y - left_c.y).powi(2)).sqrt();
            let from_right = ((point.x - right_c.x).powi(2) + (point.y - right_c.y).powi(2)).sqrt();
            assert!(
                on_top || (from_left - radius).abs() < 0.05 || (from_right - radius).abs() < 0.05,
                "{point:?} is off the surface's border"
            );
        }

         
         
        let min_x = points.iter().map(|p| p.x).fold(f32::MAX, f32::min);
        let max_x = points.iter().map(|p| p.x).fold(f32::MIN, f32::max);
        assert!((min_x - rect.left()).abs() < 0.01 && (max_x - rect.right()).abs() < 0.01);
    }

     
     
     
     
    #[test]
    fn the_top_light_is_full_strength_round_the_corners() {
        let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(200.0, 60.0));
        let radius = 12.0;
        let path = top_light_path(rect, radius);

        for (point, profile) in &path {
             
             
            if point.y <= rect.top() + radius + 0.01 {
                assert!(
                    (*profile - 1.0).abs() < 0.001,
                    "{point:?} on the border is only lit at {profile}"
                );
            }
        }

         
        let start = path[0];
        let end = *path.last().unwrap();
        assert!(
            start.1 < 0.001 && end.1 < 0.001,
            "the legs fade out at the sides"
        );
        assert!(
            start.0.y > rect.top() + radius,
            "the leg runs down the side"
        );
        assert!(end.0.y > rect.top() + radius);
        assert!(path
            .iter()
            .any(|(_, profile)| (*profile - 1.0).abs() < 0.001));
    }

     
    #[test]
    fn a_square_corner_has_a_straight_top_edge() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(40.0, 20.0));
        assert_eq!(
            top_contour(rect, 0.0),
            vec![rect.left_top(), rect.right_top()]
        );
    }

    #[test]
    fn the_entrance_easing_starts_at_zero_overshoots_and_settles() {
        assert_eq!(ease_out_back(0.0), 0.0);
        assert!((ease_out_back(1.0) - 1.0).abs() < 0.001);
        assert!(
            ease_out_back(0.6) > 1.0,
            "the entrance should overshoot slightly before settling"
        );
    }
}
