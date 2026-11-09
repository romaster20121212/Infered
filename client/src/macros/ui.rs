 
 
 
 
 

use egui::{pos2, Align2, Color32, FontId, Painter, Rect, Rounding, Stroke, Vec2};

use super::Board;
use crate::graphic::theme;

const PAD: f32 = 8.0;
const WIDTH: f32 = 236.0;
const ROW_H: f32 = 14.0;
const MAX_ROWS: usize = 5;

 
pub(crate) fn draw(ctx: &egui::Context, painter: &Painter, board: &Board) {
    if board.loaded == 0 && board.running.is_empty() {
        return;
    }
    let screen = ctx.screen_rect();
    let rows = board.running.len().min(MAX_ROWS);
    let height = 30.0_f32 + rows as f32 * ROW_H + 6.0;
    let rect = Rect::from_min_size(
        pos2(screen.center().x - WIDTH * 0.5, screen.top() + 62.0),
        Vec2::new(WIDTH, height),
    );

    painter.rect_filled(
        rect,
        Rounding::same(7.0),
        theme::with_alpha(theme::base(), 0.78),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(7.0),
        Stroke::new(1.0_f32, theme::with_alpha(theme::accent(), 0.34)),
    );
    painter.rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(2.0, rect.height())),
        Rounding::same(1.0),
        theme::accent(),
    );

    let title = if !board.enabled {
        "MACROS OFF".to_string()
    } else if board.running.is_empty() {
        format!("MACROS · {} LOADED", board.loaded)
    } else {
        format!("MACROS RUNNING · {}", board.running.len())
    };
    painter.text(
        pos2(rect.left() + PAD, rect.top() + 9.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(10.0),
        theme::accent(),
    );

     
    let detail = if let Some(error) = &board.error {
        error.clone()
    } else if board.broken > 0 {
        format!("{} file(s) could not be parsed", board.broken)
    } else if !board.enabled {
        "enable the Macros module to play back".to_string()
    } else if !board.in_world {
        "waiting for a world".to_string()
    } else if board.running.is_empty() {
        "idle".to_string()
    } else {
        "stop from the ClickGUI or disable the module".to_string()
    };
    painter.text(
        pos2(rect.left() + PAD, rect.top() + 22.0),
        Align2::LEFT_TOP,
        truncate(&detail, 46),
        FontId::proportional(9.0),
        theme::with_alpha(theme::TEXT_DIM, 0.82),
    );

    for (index, run) in board.running.iter().take(MAX_ROWS).enumerate() {
        let y = rect.top() + 38.0 + index as f32 * ROW_H;
        let color = if run.error.is_some() {
            theme::DANGER
        } else {
            Color32::WHITE
        };
        painter.circle_filled(
            pos2(rect.left() + PAD + 2.0, y + 5.0),
            2.0,
            if run.error.is_some() {
                theme::DANGER
            } else {
                theme::accent()
            },
        );
        painter.text(
            pos2(rect.left() + PAD + 9.0, y),
            Align2::LEFT_TOP,
            truncate(&run.name, 24),
            FontId::proportional(9.5),
            color,
        );
        painter.text(
            pos2(rect.right() - PAD, y),
            Align2::RIGHT_TOP,
            truncate(
                if let Some(error) = &run.error {
                    error
                } else {
                    &run.status
                },
                26,
            ),
            FontId::proportional(9.0),
            theme::with_alpha(theme::TEXT_DIM, 0.86),
        );
    }

    if board.running.len() > MAX_ROWS {
        painter.text(
            pos2(rect.right() - PAD, rect.bottom() - 5.0),
            Align2::RIGHT_BOTTOM,
            format!("+{} more", board.running.len() - MAX_ROWS),
            FontId::proportional(8.5),
            theme::with_alpha(theme::TEXT_DIM, 0.7),
        );
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
