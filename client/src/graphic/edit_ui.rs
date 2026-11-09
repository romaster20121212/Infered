 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::anim::{self, Easing};
use crate::graphic::{hud, theme};
use crate::module::render::custom_ui::{self, EditorStyle, UiStyle};
use egui::{
    Align2, Color32, Context, FontId, Id, LayerId, Order, Rect, Rounding, Sense, Stroke, Vec2,
};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering as AtomicOrdering};

 
const HEADER_H: f32 = 24.0;
 
const CARD_PAD: f32 = 6.0;
 
const MIN_CARD_W: f32 = 124.0;
 
const EDITOR_REVEAL_SECS: f32 = 0.28;
 
const HINT_W: f32 = 520.0;
const HINT_H: f32 = 34.0;
const DONE_W: f32 = 64.0;
const DONE_H: f32 = 22.0;

 
 
static DRAGGED: AtomicU8 = AtomicU8::new(0);
 
 
static SYNCED: AtomicBool = AtomicBool::new(false);

 

 
 
 
 
 
pub fn is_open() -> bool {
    custom_ui::EDITOR_OPEN.load(AtomicOrdering::Relaxed)
}

 
 
pub fn open() {
    if !is_open() {
        custom_ui::set_editing(true);
    }
}

 
pub fn close() {
    if is_open() {
        custom_ui::set_editing(false);
        crate::config::save();
    }
}

 
pub fn toggle() {
    if is_open() {
        close();
    } else {
        open();
    }
}

 
 
 
 
pub fn sync() {
    let open = is_open();
    if SYNCED.swap(open, AtomicOrdering::Relaxed) != open {
        crate::graphic::input::set_editor_cursor(open);
    }
}

 

 
 
pub fn draw(ctx: &Context) {
    let open = custom_ui::editor_open();
    let style = UiStyle::read();
    let editor = EditorStyle::read();
    let progress = if editor.animate {
        anim::toggle(
            ctx,
            Id::new("edit_ui_reveal"),
            open,
            EDITOR_REVEAL_SECS,
            Easing::Out,
        )
    } else if open {
        1.0
    } else {
        0.0
    };
    if progress <= 0.001 {
        return;
    }

     
     
    ctx.request_repaint();

    let screen = ctx.screen_rect();
    let tint = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("edit_ui_tint")));
    draw_backdrop(&tint, screen, &style, &editor, progress);

     
    let raise = DRAGGED.swap(0, AtomicOrdering::Relaxed);
    let mut order: Vec<hud::HudWidgetId> = hud::HudWidgetId::all().into_iter().collect();
    if let Some(index) = raise.checked_sub(1) {
        if let Some(id) = hud::HudWidgetId::all().get(index as usize).copied() {
            order.retain(|candidate| *candidate != id);
            order.push(id);
        }
    }
    let count = hud::HudWidgetId::all().len();
    for id in order {
        draw_widget(ctx, id, &style, &editor, progress, id.index(), count, open);
    }

     
    if editor.show_hint {
        draw_hint(ctx, &editor, &style, progress);
    }
}

 
 
 
 
 
 
fn draw_backdrop(
    painter: &egui::Painter,
    screen: Rect,
    style: &UiStyle,
    editor: &EditorStyle,
    progress: f32,
) {
     
     
    let wash = (editor.dim * 0.35 * progress).clamp(0.0, 1.0);
    painter.rect_filled(
        screen,
        Rounding::ZERO,
        Color32::from_rgba_unmultiplied(
            style.background.r(),
            style.background.g(),
            style.background.b(),
            (wash * 255.0).round() as u8,
        ),
    );
}

 
 
 
 
fn draw_hint(ctx: &Context, editor: &EditorStyle, style: &UiStyle, progress: f32) {
    let slide = (1.0 - Easing::Out.apply(progress)) * -12.0;
    egui::Area::new(Id::new("edit_ui_hint"))
        .order(Order::Tooltip)
        .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 20.0 + slide))
        .show(ctx, |ui| {
            ui.set_opacity(progress);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(HINT_W, HINT_H), Sense::hover());
            let painter = ui.painter().clone();
            let now = ctx.input(|input| input.time);
            hud::draw_top_bar_chip(
                &painter,
                rect,
                theme::RADIUS,
                progress,
                hud::comet_head(now, 7),
            );

             
            let pulse = 0.5 + 0.5 * ((now as f32) * 1.8).sin();
            painter.circle_filled(
                egui::pos2(rect.left() + 15.0, rect.center().y),
                3.4,
                theme::with_alpha(style.accent, 0.7 + 0.3 * pulse),
            );
            painter.text(
                egui::pos2(rect.left() + 25.0, rect.center().y),
                Align2::LEFT_CENTER,
                "EDIT UI",
                FontId::proportional(10.5),
                style.text,
            );

            let message = if editor.lock_widgets {
                "Layout locked"
            } else {
                "Drag any widget to move it"
            };
            painter.text(
                egui::pos2(rect.left() + 86.0, rect.center().y),
                Align2::LEFT_CENTER,
                message,
                FontId::proportional(11.5),
                theme::TEXT_DIM,
            );

            painter.text(
                egui::pos2(rect.right() - DONE_W - 22.0, rect.center().y),
                Align2::RIGHT_CENTER,
                "Esc closes",
                FontId::proportional(9.5),
                theme::with_alpha(style.accent, 0.85),
            );

            let done = Rect::from_min_size(
                egui::pos2(rect.right() - DONE_W - 10.0, rect.center().y - DONE_H * 0.5),
                Vec2::new(DONE_W, DONE_H),
            );
            let response = ui.interact(done, Id::new("edit_ui_done"), Sense::click());
            let hover = anim::toggle(
                ui.ctx(),
                Id::new("edit_ui_done_hover"),
                response.hovered(),
                0.12,
                Easing::Out,
            );
            painter.rect_filled(
                done,
                Rounding::same(6.0),
                theme::lerp_color(theme::SURFACE, style.accent, 0.16 + 0.34 * hover),
            );
            painter.rect_stroke(
                done,
                Rounding::same(6.0),
                Stroke::new(
                    1.0_f32,
                    theme::lerp_color(theme::BORDER, style.accent, hover),
                ),
            );
            painter.text(
                done.center(),
                Align2::CENTER_CENTER,
                "DONE",
                FontId::proportional(10.5),
                theme::lerp_color(theme::TEXT_DIM, style.text, hover),
            );

            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                close();
            }
        });
}

 
fn card_size(content: Vec2, editor: &EditorStyle) -> Vec2 {
    let header = if editor.show_labels { HEADER_H } else { 0.0 };
    let width = content
        .x
        .max(if editor.show_labels { MIN_CARD_W } else { 0.0 });
    Vec2::new(width + CARD_PAD * 2.0, content.y + header + CARD_PAD * 2.0)
}

 
#[allow(clippy::too_many_arguments)]
fn draw_widget(
    ctx: &Context,
    id: hud::HudWidgetId,
    style: &UiStyle,
    editor: &EditorStyle,
    progress: f32,
    index: usize,
    count: usize,
    interactive: bool,
) {
    let content = hud::widget_content_size(id, ctx);
    let pos = hud::widget_target_pos(id, &ctx.screen_rect());
    let pose = style.reveal_at(progress, index, count);
    if !pose.is_visible() {
        return;
    }
    let size = card_size(content, editor);
    let screen = ctx.screen_rect();
     
     
    let anchored = id.anchors_bottom();
    let card_pos = if anchored {
        pos - Vec2::new(0.0, size.y - CARD_PAD)
    } else {
        pos
    };

    egui::Area::new(Id::new("edit_ui_box").with(id))
        .movable(false)
        .current_pos(card_pos + pose.offset)
        .order(Order::Tooltip)
        .show(ctx, |ui| {
            ui.set_opacity(pose.alpha);
            let sense = if interactive && !editor.lock_widgets {
                Sense::drag()
            } else {
                Sense::hover()
            };
            let (card, response) = ui.allocate_exact_size(size, sense);

            let hover = if interactive && editor.highlight_hovered {
                anim::toggle(
                    ui.ctx(),
                    Id::new("edit_ui_hover").with(id),
                    response.hovered(),
                    0.12,
                    Easing::Out,
                )
            } else {
                0.0
            };
            let active = interactive && response.dragged();
            let lift = if editor.hover_lift {
                hover * 1.6 + if active { 1.8 } else { 0.0 }
            } else {
                0.0
            };

            if response.hovered() && interactive {
                ui.ctx().set_cursor_icon(if editor.lock_widgets {
                    egui::CursorIcon::NotAllowed
                } else if active {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::Grab
                });
            }

            if response.dragged() && interactive {
                DRAGGED.store(id.index() as u8 + 1, AtomicOrdering::Relaxed);
                let delta = ui.ctx().input(|input| input.pointer.delta());
                if delta != Vec2::ZERO {
                     
                     
                    let next = snap(card_pos + delta, size, &screen, editor);
                    let stored = if anchored {
                        next + Vec2::new(0.0, size.y - CARD_PAD)
                    } else {
                        next
                    };
                    crate::config::set_widget_pos(id, [stored.x, stored.y]);
                }
            }

            let visual = card.translate(Vec2::new(0.0, -lift));
            let round = Rounding::same(style.radius.max(3.0));
            let painter = ui.painter();

            if active && editor.show_guides {
                let guide = Stroke::new(1.0_f32, theme::with_alpha(style.accent, 0.55));
                painter.line_segment(
                    [
                        egui::pos2(card.left(), screen.top()),
                        egui::pos2(card.left(), screen.bottom()),
                    ],
                    guide,
                );
                painter.line_segment(
                    [
                        egui::pos2(screen.left(), card.top()),
                        egui::pos2(screen.right(), card.top()),
                    ],
                    guide,
                );
            }

             
             
            if active {
                for step in 0..3 {
                    painter.rect_stroke(
                        visual.expand(3.0 + step as f32 * 3.0),
                        round,
                        Stroke::new(
                            2.5_f32,
                            theme::with_alpha(style.accent, 0.12 - step as f32 * 0.035),
                        ),
                    );
                }
            }

            painter.rect_filled(
                visual.translate(Vec2::new(0.0, 3.0)),
                round,
                Color32::from_black_alpha(80),
            );
            painter.rect_filled(
                visual,
                round,
                theme::with_alpha(theme::base(), (editor.card_opacity * 0.9).max(0.55)),
            );
            let tint = if active {
                0.16
            } else if hover > 0.0 {
                0.08 + 0.06 * hover
            } else {
                0.04
            };
            let surface = theme::lerp_color(theme::SURFACE, style.accent, tint);
            painter.rect_filled(
                visual,
                round,
                theme::with_alpha(surface, editor.card_opacity),
            );
            let border = if active {
                style.accent
            } else if hover > 0.0 {
                theme::lerp_color(theme::BORDER, style.accent, hover)
            } else {
                theme::BORDER
            };
            painter.rect_stroke(visual, round, Stroke::new(1.0_f32, border));

            if editor.show_labels {
                let header_rect =
                    Rect::from_min_size(visual.min, Vec2::new(visual.width(), HEADER_H));
                painter.rect_filled(
                    header_rect,
                    Rounding {
                        nw: round.nw,
                        ne: round.ne,
                        ..Rounding::ZERO
                    },
                    theme::with_alpha(theme::ELEVATED, editor.card_opacity),
                );
                 
                 
                painter.line_segment(
                    [
                        egui::pos2(header_rect.left(), header_rect.bottom()),
                        egui::pos2(header_rect.right(), header_rect.bottom()),
                    ],
                    Stroke::new(1.0_f32, theme::with_alpha(theme::BORDER, 0.9)),
                );
                painter.rect_filled(
                    Rect::from_min_size(
                        header_rect.min + egui::vec2(7.0, 7.0),
                        Vec2::new(3.0, HEADER_H - 14.0),
                    ),
                    Rounding::same(1.5),
                    theme::with_alpha(style.accent, 0.55 + 0.45 * hover),
                );

                painter.text(
                    egui::pos2(header_rect.left() + 17.0, header_rect.center().y),
                    Align2::LEFT_CENTER,
                    style.label(id.display_name()),
                    FontId::proportional(11.0),
                    if active || hover > 0.0 {
                        style.text
                    } else {
                        theme::TEXT_DIM
                    },
                );
                if editor.show_sizes {
                    painter.text(
                        egui::pos2(header_rect.right() - 24.0, header_rect.center().y),
                        Align2::RIGHT_CENTER,
                        format!("{:.0}×{:.0}", content.x, content.y),
                        FontId::proportional(9.0),
                        theme::with_alpha(style.accent, 0.75),
                    );
                }

                 
                let grip = theme::with_alpha(
                    theme::TEXT_DIM,
                    if editor.lock_widgets {
                        0.18
                    } else {
                        0.35 + 0.5 * hover
                    },
                );
                for column in 0..2 {
                    for row in 0..3 {
                        painter.circle_filled(
                            egui::pos2(
                                header_rect.right() - 12.0 + column as f32 * 4.0,
                                header_rect.center().y - 4.0 + row as f32 * 4.0,
                            ),
                            1.0,
                            grip,
                        );
                    }
                }
            }

            let header = if editor.show_labels { HEADER_H } else { 0.0 };
            let preview = Rect::from_min_size(
                egui::pos2(visual.left() + CARD_PAD, visual.top() + header + CARD_PAD),
                content,
            );
             
             
            let preview_painter = ui.painter().with_clip_rect(visual.shrink(1.0));
            hud::draw_widget_preview(ctx, &preview_painter, id, preview);
        });
}

 
 
fn snap(mut pos: egui::Pos2, size: Vec2, screen: &Rect, editor: &EditorStyle) -> egui::Pos2 {
    let step = editor.grid_size.max(1.0);
    if editor.snap_to_grid {
        pos.x = (pos.x / step).round() * step;
        pos.y = (pos.y / step).round() * step;
    }

    if editor.snap_to_edges {
        let distance = editor.snap_distance.max(0.0);
        let left = screen.left() + 10.0;
        let top = screen.top() + 10.0;
        let right = screen.right() - 10.0 - size.x;
        let bottom = screen.bottom() - 10.0 - size.y;
        if (pos.x - left).abs() < distance {
            pos.x = left;
        } else if (pos.x - right).abs() < distance {
            pos.x = right;
        }
        if (pos.y - top).abs() < distance {
            pos.y = top;
        } else if (pos.y - bottom).abs() < distance {
            pos.y = bottom;
        }
    }

    let max_x = (screen.right() - size.x).max(screen.left());
    let max_y = (screen.bottom() - size.y).max(screen.top());
    pos.x = pos.x.clamp(screen.left(), max_x);
    pos.y = pos.y.clamp(screen.top(), max_y);
    pos
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    fn editor() -> EditorStyle {
        EditorStyle {
            snap_to_grid: true,
            grid_size: 8.0,
            snap_to_edges: true,
            snap_distance: 9.0,
            ..EditorStyle::default()
        }
    }

    #[test]
    fn grid_snapping_rounds_to_the_nearest_cell() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(1920.0, 1080.0));
        let size = Vec2::new(100.0, 40.0);
        assert_eq!(
            snap(pos2(103.0, 61.0), size, &screen, &editor()),
            pos2(104.0, 64.0)
        );
    }

    #[test]
    fn edge_snapping_wins_over_an_almost_margin_position() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(1920.0, 1080.0));
        let size = Vec2::new(100.0, 40.0);
        let snapped = snap(pos2(7.0, 12.0), size, &screen, &editor());
         
        assert_eq!(snapped.x, 10.0);
        assert_eq!(snapped.y, 10.0);
    }

    #[test]
    fn cards_cannot_be_dragged_off_screen() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(800.0, 600.0));
        let size = Vec2::new(200.0, 100.0);
        let snapped = snap(pos2(900.0, 700.0), size, &screen, &editor());
        assert_eq!(snapped, pos2(600.0, 500.0));
    }

    #[test]
    fn a_wider_snap_distance_catches_a_card_further_away() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(800.0, 600.0));
        let size = Vec2::new(100.0, 40.0);
        let strict = EditorStyle {
            snap_distance: 4.0,
            snap_to_grid: false,
            ..editor()
        };
        assert_eq!(snap(pos2(20.0, 200.0), size, &screen, &strict).x, 20.0);

        let loose = EditorStyle {
            snap_distance: 12.0,
            snap_to_grid: false,
            ..strict
        };
        assert_eq!(snap(pos2(20.0, 200.0), size, &screen, &loose).x, 10.0);
    }

    #[test]
    fn with_animation_off_the_editor_is_immediately_fully_open() {
        let style = UiStyle {
            reveal_distance: 40.0,
            stagger: 0.2,
            ..UiStyle::default()
        };
        let pose = style.reveal_at(1.0, 11, hud::HudWidgetId::all().len());
        assert_eq!(pose.alpha, 1.0);
        assert_eq!(pose.offset, Vec2::ZERO);
    }

    #[test]
    fn a_card_always_leaves_room_for_its_title_and_padding() {
        let editor = EditorStyle::default();
        let size = card_size(Vec2::new(20.0, 12.0), &editor);
        assert_eq!(size.x, MIN_CARD_W + CARD_PAD * 2.0);
        assert_eq!(size.y, 12.0 + HEADER_H + CARD_PAD * 2.0);

         
        let bare = EditorStyle {
            show_labels: false,
            ..editor
        };
        assert_eq!(
            card_size(Vec2::new(20.0, 12.0), &bare),
            Vec2::new(20.0 + CARD_PAD * 2.0, 12.0 + CARD_PAD * 2.0)
        );
    }

    #[test]
    fn the_backdrop_is_one_light_veil_and_nothing_else() {
        let ctx = Context::default();
        let screen = Rect::from_min_size(pos2(0.0, 0.0), Vec2::new(1280.0, 720.0));
        let style = UiStyle::default();
        let editor = EditorStyle::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                let painter =
                    ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("backdrop_test")));
                draw_backdrop(&painter, screen, &style, &editor, 1.0);
            },
        );
         
         
        assert_eq!(output.shapes.len(), 1, "{} shapes", output.shapes.len());
        let egui::Shape::Rect(veil) = &output.shapes[0].shape else {
            panic!("the backdrop is a single filled rect");
        };
        assert_eq!(veil.rect, screen);
        assert!(
            veil.fill.a() <= 80,
            "the veil is still a blackout: alpha {}",
            veil.fill.a()
        );
    }
}
