 
 
 
 
 
 
 
 
 

use crate::graphic::notification::{Notification, NotificationType};
use crate::graphic::{sidebar, theme};
use crate::module::ModuleId;
use egui::{Align2, Context, Frame, Id, Margin, Order, RichText, Rounding, Stroke, Vec2};
use std::sync::atomic::{AtomicBool, Ordering};

 
static OPEN: AtomicBool = AtomicBool::new(false);

 
const WIDTH: f32 = 400.0;
 
const TOAST_SECONDS: f32 = 3.0;

 
pub fn is_open() -> bool {
    OPEN.load(Ordering::Relaxed)
}

 
pub fn open() {
    OPEN.store(true, Ordering::Relaxed);
}

 
pub fn close() {
    OPEN.store(false, Ordering::Relaxed);
}

 
pub fn draw(ctx: &Context) {
    if !is_open() {
        return;
    }
     
    ctx.request_repaint();

    let mut info = false;
    let mut warning = false;
    let mut alert = false;
    let mut tester = false;

    egui::Area::new(Id::new("dev_test_popup"))
        .order(Order::Foreground)
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, 40.0))
        .show(ctx, |ui| {
            Frame::none()
                .fill(theme::SURFACE)
                .stroke(Stroke::new(1.0_f32, theme::BORDER))
                .rounding(Rounding::same(theme::RADIUS))
                .shadow(ui.style().visuals.window_shadow)
                .inner_margin(Margin::same(16.0))
                .show(ui, |ui| {
                    ui.set_width(WIDTH);
                    ui.spacing_mut().item_spacing.y = 8.0;

                    ui.label(
                        RichText::new("Dev build")
                            .size(16.0)
                            .strong()
                            .color(theme::TEXT),
                    );
                    ui.label(
                        RichText::new(
                            "Test popup — the overlay is live in this build. \
                             Every button below is a wired-up trigger.",
                        )
                        .size(11.5)
                        .color(theme::TEXT_DIM),
                    );

                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("NOTIFICATIONS")
                            .size(10.0)
                            .strong()
                            .color(theme::accent()),
                    );
                    ui.horizontal(|ui| {
                        info = sidebar::pressable(ui, "Info", false, 88.0).clicked();
                        warning = sidebar::pressable(ui, "Warning", false, 96.0).clicked();
                        alert = sidebar::pressable(ui, "Alert", false, 88.0).clicked();
                    });

                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("MODULE TESTER")
                            .size(10.0)
                            .strong()
                            .color(theme::accent()),
                    );
                    ui.label(
                        RichText::new(
                            "Disables every module and brings them up one at a time. \
                             Y = works, Page Down = broken and why; the broken ones \
                             land in dev_overview.md.",
                        )
                        .size(11.0)
                        .color(theme::TEXT_MUTED),
                    );
                    ui.horizontal(|ui| {
                        tester = sidebar::pressable(ui, "Start tester", true, 128.0).clicked();
                        if sidebar::pressable(ui, "Close", false, 88.0).clicked() {
                            close();
                        }
                    });
                });
        });

    if info || warning || alert {
        let (kind, label) = if info {
            (NotificationType::Info, "info")
        } else if warning {
            (NotificationType::Warning, "warning")
        } else {
            (NotificationType::Alert, "alert")
        };
        Notification::send_with_time(
            kind,
            "Dev build",
            &format!("{label} notification fired from the test popup"),
            TOAST_SECONDS,
        );
    }
    if tester {
         
         
         
        close();
        crate::graphic::input::request_module_toggle(ModuleId::ModuleTester);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

     
    #[test]
    fn opening_and_closing_follow_the_module() {
        close();
        assert!(!is_open());
        open();
        assert!(is_open());
        close();
        assert!(!is_open());
    }
}
