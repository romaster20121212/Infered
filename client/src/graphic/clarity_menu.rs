 
 
 
 
 
 
 
 
 

use crate::graphic::anim::{self, Easing};
use crate::graphic::theme::with_alpha;
use crate::graphic::{branding, configs, friends, settings};
use egui::{pos2, vec2, Align2, Color32, Context, FontId, Id, Order, Pos2, Rect, Sense, Stroke};
use std::sync::atomic::{AtomicBool, Ordering};

 
const BRAND: &str = "Sahjow";
 
const LABELS: [&str; 4] = ["Modules", "Friends", "Settings", "Configs"];
 
const LOGO_H: f32 = 100.0;
const LOGO_W: f32 = LOGO_H * 1.5;
 
const LOGO_CENTER_SHARE: f32 = 0.36;
 
const LOGO_GAP: f32 = 40.0;
const BUTTON_W: f32 = 260.0;
const BUTTON_H: f32 = 50.0;
const BUTTON_GAP: f32 = 12.0;
const BUTTON_RADIUS: f32 = 12.0;

 
 
static MODULES_VIEW: AtomicBool = AtomicBool::new(false);

 
pub fn modules_view() -> bool {
    MODULES_VIEW.load(Ordering::Relaxed)
}

 
pub fn sync_with_menu(gui_open: bool) {
    if !gui_open {
        MODULES_VIEW.store(false, Ordering::Relaxed);
    }
}

 
 
 
pub fn draw(ctx: &Context, progress: f32, modal: f32) {
    let alpha = (progress.clamp(0.0, 1.0) * (1.0 - modal.clamp(0.0, 1.0))).clamp(0.0, 1.0);
    if alpha <= 0.001 {
        return;
    }
     
    ctx.request_repaint();
    let screen = ctx.screen_rect();
    let center_x = screen.center().x;

    egui::Area::new(Id::new("clarity_menu"))
        .order(Order::Middle)
        .fixed_pos(Pos2::ZERO)
        .interactable(true)
        .show(ctx, |ui| {
            let painter = ui.painter().clone();

             
            let logo_center = pos2(center_x, screen.height() * LOGO_CENTER_SHARE);
            painter.circle_filled(
                logo_center,
                LOGO_H * 0.85,
                with_alpha(Color32::WHITE, 0.06 * alpha),
            );
            if let Some(texture) = branding::title_logo_texture(ctx) {
                let logo_rect = Rect::from_center_size(logo_center, vec2(LOGO_W, LOGO_H));
                painter.image(
                    texture.id(),
                    logo_rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    with_alpha(Color32::WHITE, alpha),
                );
            }

             
            let brand_x = center_x + LOGO_W / 2.0 + 22.0;
            painter.text(
                pos2(brand_x, logo_center.y),
                Align2::LEFT_CENTER,
                BRAND,
                FontId::proportional(34.0),
                with_alpha(Color32::WHITE, 0.95 * alpha),
            );

             
            let top = logo_center.y + LOGO_H / 2.0 + LOGO_GAP;
            for (index, label) in LABELS.iter().enumerate() {
                let rect = Rect::from_center_size(
                    pos2(
                        center_x,
                        top + index as f32 * (BUTTON_H + BUTTON_GAP) + BUTTON_H / 2.0,
                    ),
                    vec2(BUTTON_W, BUTTON_H),
                );
                let response =
                    ui.interact(rect, Id::new(("clarity_button", index)), Sense::click());
                let hover = anim::toggle(
                    ctx,
                    Id::new(("clarity_hover", index)),
                    response.hovered(),
                    0.14,
                    Easing::Out,
                );

                if hover > 0.01 {
                    painter.rect_filled(
                        rect,
                        BUTTON_RADIUS,
                        with_alpha(Color32::WHITE, 0.07 * hover * alpha),
                    );
                    painter.rect_stroke(
                        rect,
                        BUTTON_RADIUS,
                        Stroke::new(1.0_f32, with_alpha(Color32::WHITE, 0.18 * hover * alpha)),
                    );
                }

                let text_alpha = alpha * (0.62 + 0.38 * hover);
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    *label,
                    FontId::proportional(18.0 + 0.6 * hover),
                    with_alpha(Color32::WHITE, text_alpha),
                );
                let underline = 26.0 * hover;
                if underline > 0.5 {
                    let y = rect.bottom() - 11.0;
                    painter.line_segment(
                        [
                            pos2(rect.center().x - underline, y),
                            pos2(rect.center().x + underline, y),
                        ],
                        Stroke::new(1.5_f32, with_alpha(Color32::WHITE, 0.85 * hover * alpha)),
                    );
                }

                if response.clicked() {
                    match index {
                        0 => MODULES_VIEW.store(true, Ordering::Relaxed),
                        1 => friends::set_open(true),
                        2 => {
                            if !settings::is_open() {
                                settings::toggle_open();
                            }
                        }
                        _ => {
                            if !configs::is_open() {
                                configs::toggle_open();
                            }
                        }
                    }
                }
            }
        });
}
