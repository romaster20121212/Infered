 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::anim::{self, Easing, SpringCfg};
use crate::graphic::theme;
use egui::{pos2, vec2, Align2, FontId, Id, Rect, Rounding, Sense, Stroke, Ui};

 
const SWITCH_ROW_H: f32 = 21.0;
const SLIDER_ROW_H: f32 = 36.0;
const CHOICE_ROW_H: f32 = 19.0;
const ACTION_ROW_H: f32 = 22.0;
 
const SWITCH_W: f32 = 32.0;
const SWITCH_H: f32 = 16.0;
const KNOB_R: f32 = 5.8;
 
const TRACK_H: f32 = 4.0;
const SLIDER_KNOB_R: f32 = 5.2;
 
const CHOICE_GAP: f32 = 2.0;
 
const MIN_PILL_W: f32 = 52.0;

 
 
pub(crate) const MAX_PILLS: usize = 3;

 
fn reaction(ui: &Ui, id: Id, triggered: bool, duration: f32) -> Option<f32> {
    let now = ui.input(|input| input.time);
    if triggered {
        ui.data_mut(|data| data.insert_temp(id, now));
    }
    let start = ui.data(|data| data.get_temp::<f64>(id))?;
    let progress = ((now - start) as f32 * anim::speed() / duration).clamp(0.0, 1.0);
    if now < start || progress >= 1.0 {
        ui.data_mut(|data| data.remove::<f64>(id));
        return None;
    }
    ui.ctx().request_repaint();
    Some(progress)
}

 
 
pub(crate) fn switch_row(ui: &mut Ui, name: &str, value: &mut bool, scale: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), SWITCH_ROW_H * scale),
        Sense::click(),
    );
    let mut changed = false;
    if response.clicked() {
        *value = !*value;
        changed = true;
    }

    let ctx = ui.ctx().clone();
    let id = response.id.with("ctrl_switch");
    let on = anim::toggle(&ctx, id.with("on"), *value, 0.18, Easing::InOut);
    let hover = anim::toggle(
        &ctx,
        id.with("hover"),
        response.hovered(),
        0.12,
        Easing::Out,
    );
     
     
    let knob = anim::spring(
        &ctx,
        id.with("knob"),
        if *value { 1.0 } else { 0.0 },
        SpringCfg::SNAPPY,
    )
    .clamp(0.0, 1.0);

    let burst = reaction(ui, id.with("burst"), changed, 0.42);
    let painter = ui.painter();
    painter.text(
        pos2(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(12.0 * scale),
        theme::lerp_color(theme::TEXT_DIM, theme::TEXT, hover.max(on)),
    );

    let (track_w, track_h) = (SWITCH_W * scale, SWITCH_H * scale);
    let track = Rect::from_center_size(
        pos2(rect.right() - track_w * 0.5, rect.center().y),
        vec2(track_w, track_h),
    );
    painter.rect(
        track,
        Rounding::same(track_h * 0.5),
        theme::lerp_color(
            theme::ELEVATED,
            theme::with_alpha(theme::accent(), 0.82),
            on,
        ),
        Stroke::new(
            1.0_f32,
            theme::lerp_color(theme::BORDER, theme::with_alpha(theme::accent(), 0.9), on),
        ),
    );
    let knob_x = track.left() + track_h * 0.5 + knob * (track_w - track_h);
    if hover > 0.01 {
        painter.circle_stroke(
            pos2(knob_x, track.center().y),
            (KNOB_R + 2.6) * scale,
            Stroke::new(1.0_f32, theme::with_alpha(theme::accent(), 0.32 * hover)),
        );
    }
    let knob_pos = pos2(knob_x, track.center().y);
    if let Some(progress) = burst {
        let fade = (1.0 - progress).powi(2);
        painter.circle_stroke(
            knob_pos,
            (KNOB_R + 2.0 + progress * 6.0) * scale,
            Stroke::new(scale, theme::with_alpha(theme::accent(), 0.6 * fade)),
        );
        for sign in [-1.0, 1.0] {
            let offset = (7.0 + progress * 5.0) * scale * sign;
            painter.circle_filled(
                knob_pos + vec2(offset, -4.0 * scale),
                (1.4 - progress * 0.8) * scale,
                theme::with_alpha(theme::accent(), fade),
            );
        }
    }
    painter.circle_filled(
        knob_pos,
        KNOB_R * scale,
        theme::lerp_color(theme::TEXT_MUTED, theme::TEXT, on),
    );
    if hover > 0.01 || response.is_pointer_button_down_on() {
        ui.ctx().request_repaint();
    }
    changed
}

 
 
 
pub(crate) fn slider_row(
    ui: &mut Ui,
    name: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    scale: f32,
) -> bool {
    let track_h = TRACK_H * scale;
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), SLIDER_ROW_H * scale),
        Sense::click_and_drag(),
    );
    let track = Rect::from_min_size(
        pos2(rect.left(), rect.bottom() - track_h - 3.0 * scale),
        vec2(rect.width(), track_h),
    );

    let mut changed = false;
    if response.clicked() || response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let next = value_at(pointer.x, track, min, max);
            if (next - *value).abs() > f32::EPSILON {
                *value = next;
                changed = true;
            }
        }
    }

    let ctx = ui.ctx().clone();
    let id = response.id.with("ctrl_slider");
    let hover = anim::toggle(
        &ctx,
        id.with("hover"),
        response.hovered(),
        0.12,
        Easing::Out,
    );
    let fill = anim::ease_to(
        &ctx,
        id.with("fill"),
        fraction(*value, min, max),
        0.16,
        Easing::Out,
    );
    let active = if response.dragged() { 1.0 } else { hover };
    let spark = reaction(ui, id.with("spark"), changed, 0.36);

    let painter = ui.painter();
    painter.text(
        pos2(rect.left(), rect.top() + 6.0 * scale),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(11.5 * scale),
        theme::lerp_color(theme::TEXT_DIM, theme::TEXT, hover),
    );
     
     
    let value_text = format!("{value:.2}");
    let value_font = FontId::proportional(10.0 * scale);
    let value_w = ui
        .fonts(|fonts| {
            fonts
                .layout_no_wrap(value_text.clone(), value_font.clone(), theme::TEXT)
                .size()
                .x
        })
        .max(20.0 * scale);
    let chip = Rect::from_min_size(
        pos2(
            rect.right() - value_w - 8.0 * scale,
            rect.top() + 0.5 * scale,
        ),
        vec2(value_w + 8.0 * scale, 14.0 * scale),
    );
    painter.rect_filled(
        chip,
        Rounding::same(4.0 * scale),
        theme::with_alpha(theme::accent(), 0.10 + 0.10 * active),
    );
    painter.text(
        chip.center(),
        Align2::CENTER_CENTER,
        value_text,
        value_font,
        theme::lerp_color(theme::accent_dim(), theme::accent(), 0.4 + 0.6 * active),
    );

    painter.rect(
        track,
        Rounding::same(track_h * 0.5),
        theme::ELEVATED,
        Stroke::new(1.0_f32, theme::with_alpha(theme::BORDER, 0.9)),
    );
     
    for step in 1..4 {
        let x = track.left() + track.width() * step as f32 / 4.0;
        painter.line_segment(
            [
                pos2(x, track.center().y - 1.5 * scale),
                pos2(x, track.center().y + 1.5 * scale),
            ],
            Stroke::new(1.0_f32, theme::with_alpha(theme::BORDER, 0.9)),
        );
    }
    let filled = Rect::from_min_size(track.left_top(), vec2(track.width() * fill, track.height()));
    if fill > 0.001 {
        painter.rect_filled(
            filled,
            Rounding::same(track_h * 0.5),
            theme::lerp_color(
                theme::with_alpha(theme::accent(), 0.85),
                theme::accent(),
                active,
            ),
        );
    }

    let knob = pos2(filled.right(), track.center().y);
    if let Some(progress) = spark {
        let fade = (1.0 - progress).powi(2);
        let center = knob + vec2(0.0, -(5.0 + progress * 7.0) * scale);
        let radius = (2.0 + progress) * scale;
        let color = theme::with_alpha(theme::accent(), 0.8 * fade);
        painter.line_segment(
            [center - vec2(radius, 0.0), center + vec2(radius, 0.0)],
            Stroke::new(scale, color),
        );
        painter.line_segment(
            [center - vec2(0.0, radius), center + vec2(0.0, radius)],
            Stroke::new(scale, color),
        );
    }
    let knob_r = SLIDER_KNOB_R * scale;
    if active > 0.01 {
        painter.circle_filled(
            knob,
            knob_r + 4.0 * scale,
            theme::with_alpha(theme::accent(), 0.12 * active),
        );
    }
    painter.circle_filled(knob, knob_r * (1.0 + 0.12 * active), theme::TEXT);
    painter.circle_filled(
        knob,
        (knob_r - 2.0 * scale).max(1.0),
        theme::lerp_color(theme::accent(), theme::TEXT, 0.35),
    );

    if active > 0.01 || response.dragged() {
        ui.ctx().request_repaint();
    }
    changed
}

 
 
pub(crate) fn choice_row(
    ui: &mut Ui,
    name: &str,
    value: &mut usize,
    options: &[String],
    scale: f32,
) -> bool {
    let width = ui.available_width();
    let gap = CHOICE_GAP * scale;
    let pill_h = CHOICE_ROW_H * scale;
    let per_row = pills_per_row(width, options.len(), scale);
    let rows = options.len().div_ceil(per_row.max(1)).max(1);
    let pills_h = rows as f32 * pill_h + (rows - 1) as f32 * gap;
    let (rect, _) = ui.allocate_exact_size(vec2(width, pills_h + 20.0 * scale), Sense::hover());
    let painter = ui.painter().clone();
    painter.text(
        pos2(rect.left(), rect.top() + 6.0 * scale),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(11.5 * scale),
        theme::TEXT_DIM,
    );

    let area = Rect::from_min_size(
        pos2(rect.left(), rect.top() + 17.0 * scale),
        vec2(rect.width(), pills_h),
    );
    if options.is_empty() {
        return false;
    }
    let pills = pills_for(area, options.len(), scale);
    let mut changed = false;
    for (index, pill) in pills.iter().enumerate() {
        let response = ui.interact(
            *pill,
            ui.id().with("ctrl_choice").with(name).with(index),
            Sense::click(),
        );
        if response.clicked() && *value != index {
            *value = index;
            changed = true;
        }
    }

    let ctx = ui.ctx().clone();
    let flash = reaction(
        ui,
        ui.id().with("ctrl_choice_flash").with(name),
        changed,
        0.38,
    );
    let slide = anim::ease_to(
        &ctx,
        ui.id().with("ctrl_choice_slide").with(name),
        (*value).min(options.len() - 1) as f32,
        0.18,
        Easing::Out,
    );
    painter.rect_filled(area, Rounding::same(5.0 * scale), theme::SURFACE_HOVER);
    let selected = (*value).min(pills.len() - 1);
    let from_index = (slide.floor() as usize).min(pills.len() - 1);
    let to_index = (from_index + 1).min(pills.len() - 1);
    let step = (slide - from_index as f32).clamp(0.0, 1.0);
    let highlight = Rect::from_min_size(
        pills[from_index].min.lerp(pills[to_index].min, step),
        pills[selected].size(),
    );
    painter.rect_filled(
        highlight,
        Rounding::same(4.0 * scale),
        theme::with_alpha(theme::accent(), 0.28),
    );
    painter.rect_stroke(
        highlight,
        Rounding::same(4.0 * scale),
        Stroke::new(1.0_f32, theme::with_alpha(theme::accent(), 0.6)),
    );
    if let Some(progress) = flash {
        let fade = (1.0 - progress).powi(2);
        let width = highlight.width() * Easing::Out.apply(progress);
        let line = Rect::from_min_size(
            pos2(
                highlight.center().x - width * 0.5,
                highlight.bottom() - 2.0 * scale,
            ),
            vec2(width, scale),
        );
        painter.rect_filled(
            line,
            Rounding::same(scale),
            theme::with_alpha(theme::accent(), fade),
        );
    }
    for (index, (option, pill)) in options.iter().zip(pills).enumerate() {
        let selected = 1.0 - (index as f32 - slide).abs().clamp(0.0, 1.0);
        painter.text(
            pill.center(),
            Align2::CENTER_CENTER,
            option,
            FontId::proportional(10.0 * scale),
            theme::lerp_color(theme::TEXT_DIM, theme::TEXT, selected),
        );
    }
    if (slide - *value as f32).abs() > 0.001 {
        ui.ctx().request_repaint();
    }
    changed
}

 
 
 
pub(crate) fn action_row(ui: &mut Ui, caption: &str, scale: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), ACTION_ROW_H * scale),
        Sense::click(),
    );
    let ctx = ui.ctx().clone();
    let id = response.id.with("ctrl_action");
    let hover = anim::toggle(
        &ctx,
        id.with("hover"),
        response.hovered(),
        0.12,
        Easing::Out,
    );
     
    let held = if response.is_pointer_button_down_on() {
        1.0
    } else {
        0.0
    };
    let lit = hover.max(held);
    let flash = reaction(ui, id.with("flash"), response.clicked(), 0.4);

    let painter = ui.painter();
    painter.rect(
        rect,
        Rounding::same(5.0 * scale),
        theme::lerp_color(
            theme::ELEVATED,
            theme::with_alpha(theme::accent(), 0.16),
            lit,
        ),
        Stroke::new(
            1.0_f32,
            theme::lerp_color(theme::BORDER, theme::with_alpha(theme::accent(), 0.7), lit),
        ),
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        caption,
        FontId::proportional(11.5 * scale),
        theme::lerp_color(theme::TEXT_DIM, theme::TEXT, lit),
    );
    if let Some(progress) = flash {
        let fade = (1.0 - progress).powi(2);
        let width = rect.width() * Easing::Out.apply(progress);
        painter.rect_filled(
            Rect::from_min_size(
                pos2(rect.center().x - width * 0.5, rect.bottom() - 2.0 * scale),
                vec2(width, scale),
            ),
            Rounding::same(scale),
            theme::with_alpha(theme::accent(), fade),
        );
    }
    if lit > 0.01 || response.clicked() {
        ui.ctx().request_repaint();
    }
    response.clicked()
}

 
fn pills_per_row(width: f32, count: usize, scale: f32) -> usize {
    if count == 0 {
        return 1;
    }
    let gap = CHOICE_GAP * scale;
    ((width + gap) / (MIN_PILL_W * scale + gap))
        .floor()
        .clamp(1.0, count as f32) as usize
}

 
fn pills_for(area: Rect, count: usize, scale: f32) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    let gap = CHOICE_GAP * scale;
    let pill_h = CHOICE_ROW_H * scale;
    let per_row = pills_per_row(area.width(), count, scale);
    let width = ((area.width() - gap * (per_row - 1) as f32) / per_row as f32).max(1.0);
    (0..count)
        .map(|index| {
            let row = index / per_row;
            let column = index % per_row;
            Rect::from_min_size(
                pos2(
                    area.left() + column as f32 * (width + gap),
                    area.top() + row as f32 * (pill_h + gap),
                ),
                vec2(width, pill_h),
            )
        })
        .collect()
}

 
 
fn fraction(value: f32, min: f32, max: f32) -> f32 {
    if (max - min).abs() < f32::EPSILON {
        return 0.0;
    }
    ((value - min) / (max - min)).clamp(0.0, 1.0)
}

 
fn value_at(x: f32, track: Rect, min: f32, max: f32) -> f32 {
    if track.width() <= 0.0 {
        return min;
    }
    let t = ((x - track.left()) / track.width()).clamp(0.0, 1.0);
    min + (max - min) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Pos2};

    #[test]
    fn reactions_expire_and_follow_the_menu_speed() {
        let ctx = Context::default();
        let id = Id::new("reaction_test");
        let speed = anim::speed();
        let mut samples = Vec::new();
        for (time, triggered) in [
            (10.0, true),
            (10.0 + 0.2 / speed as f64, false),
            (10.0 + 0.5 / speed as f64, false),
            (11.0 + 0.5 / speed as f64, false),
        ] {
            let _ = ctx.run(
                egui::RawInput {
                    time: Some(time),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| samples.push(reaction(ui, id, triggered, 0.4)));
                },
            );
        }
        assert_eq!(samples[0], Some(0.0));
        assert!((samples[1].unwrap() - 0.5).abs() < 0.001);
        assert_eq!(samples[2], None);
        assert_eq!(samples[3], None);
    }

    #[test]
    fn control_interactions_paint_short_lived_reactions() {
        use egui::{Event, PointerButton, RawInput};

        for control in 0..4 {
            let ctx = Context::default();
            let mut toggle = false;
            let mut slider = 0.0;
            let mut choice = 0;
            let options = vec!["One".into(), "Two".into()];
            let pos = std::cell::Cell::new(Pos2::ZERO);
            let changed = std::cell::Cell::new(false);
            let mut draw = |time, events| {
                ctx.run(
                    RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 200.0))),
                        time: Some(time),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            let start = ui.next_widget_position();
                            let width = ui.available_width();
                            pos.set(match control {
                                0 => start + vec2(10.0, 10.0),
                                1 => start + vec2(width * 0.7, 25.0),
                                2 => start + vec2(width * 0.75, 25.0),
                                _ => start + vec2(width * 0.5, 10.0),
                            });
                            changed.set(match control {
                                0 => switch_row(ui, "Toggle", &mut toggle, 1.0),
                                1 => slider_row(ui, "Slider", &mut slider, 0.0, 1.0, 1.0),
                                2 => choice_row(ui, "Choice", &mut choice, &options, 1.0),
                                _ => action_row(ui, "Action", 1.0),
                            });
                        });
                    },
                )
            };
            let _ = draw(1.0, vec![]);
            let target = pos.get();
            let _ = draw(
                1.1,
                vec![
                    Event::PointerMoved(target),
                    Event::PointerButton {
                        pos: target,
                        button: PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                ],
            );
            let output = draw(
                1.2,
                vec![Event::PointerButton {
                    pos: target,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                }],
            );
            assert!(changed.get(), "control {control} did not react to a click");
            let active_shapes = output.shapes.len();
            let output = draw(3.0, vec![]);
            assert!(
                active_shapes > output.shapes.len(),
                "control {control}: {active_shapes} vs {}",
                output.shapes.len()
            );
            match control {
                0 => assert!(toggle),
                1 => assert!(slider > 0.6),
                2 => assert_eq!(choice, 1),
                _ => {}
            }
        }
    }

     
    #[test]
    fn only_short_option_lists_use_pills() {
        assert!(MAX_PILLS >= 2, "two-way choices need pills");
        assert!(MAX_PILLS < 8, "a long list must not be squeezed into pills");
    }

     
    #[test]
    fn dragging_the_track_sets_the_value() {
        let track = Rect::from_min_size(pos2(10.0, 0.0), vec2(100.0, 4.0));
        assert_eq!(value_at(10.0, track, 0.0, 10.0), 0.0);
        assert!((value_at(60.0, track, 0.0, 10.0) - 5.0).abs() < 1e-5);
        assert_eq!(value_at(110.0, track, 0.0, 10.0), 10.0);
        assert_eq!(value_at(-40.0, track, 0.0, 10.0), 0.0);
        let thin = Rect::from_min_size(pos2(0.0, 0.0), vec2(0.0, 4.0));
        assert!(value_at(0.5, thin, 0.0, 1.0).is_finite());
         
        assert!((fraction(5.0, 10.0, 0.0) - 0.5).abs() < 1e-5);
        assert_eq!(fraction(3.0, 4.0, 4.0), 0.0);
    }

     
    #[test]
    fn pills_split_the_row_and_wrap_instead_of_cramping() {
        let area = Rect::from_min_size(Pos2::ZERO, vec2(128.0, 19.0));
        let two = pills_for(area, 2, 1.0);
        assert_eq!(two.len(), 2);
        assert!((two[0].width() - 63.0).abs() < 1e-5);
        assert!((two[1].left() - (two[0].right() + CHOICE_GAP)).abs() < 1e-5);

        let three = pills_for(Rect::from_min_size(Pos2::ZERO, vec2(128.0, 40.0)), 3, 1.0);
        assert_eq!(pills_per_row(128.0, 3, 1.0), 2);
        assert!(three[0].width() >= MIN_PILL_W);
        assert!((three[2].top() - three[1].bottom() - CHOICE_GAP).abs() < 1e-5);
        assert!(pills_per_row(400.0, 3, 1.0) == 3);
        assert!(pills_for(area, 0, 1.0).is_empty());

         
        let big = pills_for(Rect::from_min_size(Pos2::ZERO, vec2(256.0, 80.0)), 3, 2.0);
        assert_eq!(pills_per_row(256.0, 3, 2.0), 2);
        assert!((big[0].height() - CHOICE_ROW_H * 2.0).abs() < 1e-5);
    }

     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn controls_preview() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let (w, h) = (172usize, 250usize);
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        for frame in 0..3 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(w as f32, h as f32))),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .frame(
                            egui::Frame::none()
                                .fill(theme::SURFACE)
                                .inner_margin(egui::Margin::same(12.0)),
                        )
                        .show(ctx, |ui| {
                            ui.set_width(148.0);
                            ui.spacing_mut().item_spacing.y = 6.0;
                            for (name, on, value, choice) in
                                [("Name", true, 64.0, 1usize), ("Range", false, 12.5, 0)]
                            {
                                let mut on = on;
                                let mut value = value;
                                let mut choice = choice;
                                switch_row(ui, name, &mut on, 1.0);
                                slider_row(ui, "Distance", &mut value, 0.0, 128.0, 1.0);
                                let options: Vec<String> = ["Box", "Rounded", "Outline"]
                                    .iter()
                                    .map(|s| s.to_string())
                                    .collect();
                                choice_row(ui, "Style", &mut choice, &options, 1.0);
                                ui.add_space(10.0);
                            }
                        });
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
        let image = rasterize(primitives, &textures, w, h, [0.07, 0.06, 0.06, 1.0]);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("module-controls.png");
        image.save(&path).expect("the controls preview saves");
        println!("module controls: {path:?}");
    }
}
