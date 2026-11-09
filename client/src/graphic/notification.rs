 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::anim::{self, Easing};
use crate::graphic::theme;
use egui::{
    Align2, Color32, Context, FontId, Id, LayerId, Order, Painter, Pos2, Rect, Rounding, Stroke,
    Vec2,
};
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

 
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NotificationType {
    Info,
    Warning,
    Alert,
}

impl NotificationType {
     
    fn color(self) -> Color32 {
        match self {
            NotificationType::Info => theme::accent(),
            NotificationType::Warning => theme::WARN,
            NotificationType::Alert => theme::DANGER,
        }
    }

     
    fn icon(self) -> &'static str {
        match self {
            NotificationType::Info => "i",
            NotificationType::Warning => "!",
            NotificationType::Alert => "x",
        }
    }

     
    fn tag(self) -> &'static str {
        match self {
            NotificationType::Info => "INFO",
            NotificationType::Warning => "WARNING",
            NotificationType::Alert => "ALERT",
        }
    }
}

 
pub struct Notification {
     
    id: u64,
    notif_type: NotificationType,
    title: String,
    message: String,
    spawn_time: Instant,
    duration: f32,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

lazy_static! {
    static ref QUEUE: Mutex<Vec<Notification>> = Mutex::new(Vec::new());
}

impl Notification {
     
    pub fn send(notif_type: NotificationType, title: &str, message: &str) {
        Self::send_with_time(notif_type, title, message, 3.0);
    }

     
    pub fn send_with_time(notif_type: NotificationType, title: &str, message: &str, duration: f32) {
        let mut queue = QUEUE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if queue.len() >= MAX_VISIBLE {
            let overflow = queue.len() + 1 - MAX_VISIBLE;
            queue.drain(..overflow);
        }
        queue.push(Notification {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            notif_type,
            title: title.to_string(),
            message: message.to_string(),
            spawn_time: Instant::now(),
            duration,
        });
    }
}

 
const WIDTH: f32 = 306.0;
const HEIGHT: f32 = 68.0;
const MARGIN: f32 = 16.0;
const GAP: f32 = 10.0;
 
const RADIUS: f32 = 10.0;
 
const SLIDE: f32 = 0.32;
 
 
 
const MAX_VISIBLE: usize = 6;
 
 
const EXIT_SLIP: f32 = 26.0;

 
pub fn draw(ctx: &Context) {
    let mut queue = QUEUE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let now = Instant::now();
    queue.retain(|n| now.duration_since(n.spawn_time).as_secs_f32() < n.duration);
     
    if !crate::graphic::settings::notifications_enabled() {
        queue.clear();
    }
    if queue.is_empty() {
        return;
    }

     
    let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("notifications")));
    let screen_w = ctx.screen_rect().width();
    let rest_x = screen_w - MARGIN - WIDTH;

     
    for (index, n) in queue.iter().rev().enumerate() {
        let elapsed = now.duration_since(n.spawn_time).as_secs_f32();
        let remaining = (n.duration - elapsed).max(0.0);

         
        let enter = Easing::Out.apply((elapsed / SLIDE).clamp(0.0, 1.0));
        let exit = Easing::In.apply((remaining / SLIDE).clamp(0.0, 1.0));
        let opacity = (enter * exit).clamp(0.0, 1.0);
        let x = rest_x + (1.0 - enter) * (WIDTH + MARGIN) + (1.0 - exit) * EXIT_SLIP;

         
        let target_y = MARGIN + index as f32 * (HEIGHT + GAP);
        let y = anim::ease_to(
            ctx,
            Id::new("notif_y").with(n.id),
            target_y,
            0.22,
            Easing::Out,
        );

        let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(WIDTH, HEIGHT));
        draw_card(
            &painter,
            rect,
            n,
            (remaining / n.duration).clamp(0.0, 1.0),
            opacity,
        );
    }
}

 
 
 
fn layout_line(
    painter: &Painter,
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::single_section(
        text.to_owned(),
        egui::TextFormat {
            font_id: font,
            color,
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping {
        max_width: width.max(1.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    painter.layout_job(job)
}

 
 
fn draw_card(painter: &Painter, rect: Rect, n: &Notification, remaining: f32, opacity: f32) {
    let accent = n.notif_type.color();
    let radius = Rounding::same(RADIUS);
     
    let fade = |color: Color32| theme::with_alpha(color, opacity);

     
     
     
    for step in 1..=10 {
        let grow = step as f32;
        painter.rect_filled(
            rect.expand(grow),
            Rounding::same(RADIUS + grow),
            fade(theme::with_alpha(accent, 0.022)),
        );
    }

     
    painter.rect_filled(
        rect,
        radius,
        fade(Color32::from_rgba_unmultiplied(14, 12, 13, 246)),
    );
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(
            1.0_f32,
            fade(theme::lerp_color(theme::BORDER, accent, 0.30)),
        ),
    );

     
     
    painter.rect_stroke(
        rect.shrink(1.0),
        Rounding::same(RADIUS - 1.0),
        Stroke::new(1.0_f32, fade(theme::with_alpha(accent, 0.06))),
    );
    painter.rect_filled(
        Rect::from_min_size(
            rect.min + Vec2::new(12.0, 1.0),
            Vec2::new(rect.width() - 24.0, 1.0),
        ),
        Rounding::same(1.0),
        fade(theme::with_alpha(theme::TEXT, 0.035)),
    );

     
    painter.rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height())),
        Rounding {
            nw: RADIUS,
            sw: RADIUS,
            ne: 0.0,
            se: 0.0,
        },
        fade(accent),
    );
    painter.rect_filled(
        Rect::from_min_size(
            rect.min + Vec2::new(3.0, 14.0),
            Vec2::new(1.0, rect.height() - 28.0),
        ),
        Rounding::same(0.5),
        fade(theme::with_alpha(accent, 0.22)),
    );

     
    let badge = Rect::from_center_size(
        Pos2::new(rect.min.x + 38.0, rect.center().y - 3.0),
        Vec2::splat(30.0),
    );
     
    painter.circle_filled(badge.center(), 22.0, fade(theme::with_alpha(accent, 0.05)));
    painter.rect_filled(
        badge,
        Rounding::same(9.0),
        fade(theme::with_alpha(accent, 0.15)),
    );
    painter.rect_stroke(
        badge,
        Rounding::same(9.0),
        Stroke::new(1.0_f32, fade(theme::with_alpha(accent, 0.42))),
    );
    painter.text(
        badge.center(),
        Align2::CENTER_CENTER,
        n.notif_type.icon(),
        FontId::proportional(15.0),
        fade(accent),
    );

    let text_x = rect.min.x + 62.0;

     
    painter.text(
        Pos2::new(rect.right() - 14.0, rect.min.y + 15.0),
        Align2::RIGHT_TOP,
        n.notif_type.tag(),
        FontId::proportional(9.0),
        fade(theme::with_alpha(accent, 0.9)),
    );

     
    let title = layout_line(
        painter,
        &n.title,
        FontId::proportional(15.0),
        fade(theme::TEXT),
        rect.right() - 82.0 - text_x,
    );
     
    let title_y = if n.message.trim().is_empty() {
        rect.center().y - title.size().y * 0.5 - 2.0
    } else {
        rect.min.y + 12.0
    };
    painter.galley(Pos2::new(text_x, title_y), title, fade(theme::TEXT));

    if !n.message.trim().is_empty() {
         
         
        let message = layout_line(
            painter,
            &n.message,
            FontId::proportional(11.5),
            fade(theme::TEXT_DIM),
            rect.right() - 14.0 - text_x,
        );
        painter.galley(
            Pos2::new(text_x, rect.min.y + 35.0),
            message,
            fade(theme::TEXT_DIM),
        );
    }

     
     
    let track = Rect::from_min_size(
        Pos2::new(rect.min.x + 14.0, rect.bottom() - 9.0),
        Vec2::new(rect.width() - 28.0, 2.5),
    );
    painter.rect_filled(
        track,
        Rounding::same(1.25),
        fade(theme::with_alpha(theme::TEXT, 0.08)),
    );
    if remaining > 0.0 {
        let filled = Rect::from_min_size(
            track.min,
            Vec2::new((track.width() * remaining).max(3.0), track.height()),
        );
        painter.rect_filled(filled, Rounding::same(1.25), fade(accent));
         
         
        let head = Pos2::new(filled.right(), filled.center().y);
        painter.circle_filled(head, 4.0, fade(theme::with_alpha(accent, 0.10)));
        painter.circle_filled(head, 1.7, fade(accent));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(notif_type: NotificationType, title: &str, message: &str) -> Notification {
        Notification {
            id: 0,
            notif_type,
            title: title.to_string(),
            message: message.to_string(),
            spawn_time: Instant::now(),
            duration: 4.0,
        }
    }

     
    #[test]
    fn every_severity_has_its_own_tag() {
        assert_eq!(NotificationType::Info.tag(), "INFO");
        assert_eq!(NotificationType::Warning.tag(), "WARNING");
        assert_eq!(NotificationType::Alert.tag(), "ALERT");
    }

     
     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn notification_cards_preview() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let cards = [
            card(
                NotificationType::Info,
                "Spawner found",
                "24 blocks · 118 64 -204",
            ),
            card(
                NotificationType::Info,
                "4 spawners found",
                "Nearest 5 blocks · 118 64 -204",
            ),
            card(
                NotificationType::Warning,
                "A message that runs on",
                "This line is far too long for a toast card and has to be cut",
            ),
            card(NotificationType::Alert, "Panic", "The client is unloading"),
        ];
        let (w, h) = (
            340usize,
            (cards.len() as f32 * (HEIGHT + GAP) + 30.0) as usize,
        );
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut shapes = Vec::new();
        let mut laid_out_at = 1.0;
         
        for _ in 0..2 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(w as f32, h as f32),
                    )),
                    time: Some(1.0),
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx
                        .layer_painter(LayerId::new(Order::Background, Id::new("toast_preview")));
                    for (index, n) in cards.iter().enumerate() {
                        let rect = Rect::from_min_size(
                            Pos2::new(17.0, 15.0 + index as f32 * (HEIGHT + GAP)),
                            Vec2::new(WIDTH, HEIGHT),
                        );
                        draw_card(&painter, rect, n, 0.6, 1.0);
                    }
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            shapes = output.shapes;
            laid_out_at = output.pixels_per_point;
        }
        let primitives = ctx.tessellate(shapes, laid_out_at);
        let sheet = rasterize(primitives, &textures, w, h, [0.05, 0.04, 0.04, 1.0]);
        let image = image::imageops::resize(
            &sheet,
            w as u32 * 2,
            h as u32 * 2,
            image::imageops::FilterType::Nearest,
        );
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).expect("the screenshots directory exists");
        let path = dir.join("notifications.png");
        image.save(&path).expect("the notification preview saves");
        println!("notifications: {path:?}");
    }
}
