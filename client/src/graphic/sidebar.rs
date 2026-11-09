 
 
 
 
 

use crate::graphic::anim::{self, Easing};
use crate::graphic::theme;
use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

 
const WIDE_H: f32 = 34.0;
 
const RIPPLE_TIME: f64 = 0.45;

 
static RIPPLES: LazyLock<Mutex<HashMap<Id, (Pos2, f64)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

 
 
 
pub fn pressable(ui: &mut Ui, label: &str, active: bool, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, WIDE_H), Sense::click());
    let ctx = ui.ctx().clone();
    let id = ui.id().with(label);

    let hover = anim::toggle(
        &ctx,
        id.with("hover"),
        response.hovered(),
        0.16,
        Easing::Out,
    );
    let press = anim::toggle(
        &ctx,
        id.with("press"),
        response.is_pointer_button_down_on(),
        0.07,
        Easing::Out,
    );
    let active_t = anim::toggle(&ctx, id.with("active"), active, 0.22, Easing::InOut);

    if response.clicked() {
        let origin = ctx
            .input(|i| i.pointer.interact_pos())
            .unwrap_or(rect.center());
        let now = ctx.input(|i| i.time);
        RIPPLES.lock().unwrap().insert(id, (origin, now));
    }

    let shape = Rect::from_center_size(rect.center(), rect.size() * (1.0 - 0.04 * press));
    let accent = theme::accent();

     
     
    let surface = theme::lerp_color(theme::SURFACE, theme::SURFACE_HOVER, hover);
    let tint = theme::lerp_color(surface, accent, 0.22);
    let fill = theme::lerp_color(surface, tint, active_t);
    let border = theme::lerp_color(theme::BORDER, accent, active_t * 0.8);
    let text = theme::lerp_color(
        theme::lerp_color(theme::TEXT_DIM, theme::TEXT, hover),
        accent,
        active_t,
    );

    let painter = ui.painter();
    painter.rect(
        shape,
        Rounding::same(theme::RADIUS_INNER),
        fill,
        Stroke::new(1.0_f32, border),
    );

     
    if active_t > 0.001 {
        let bar = Rect::from_center_size(
            Pos2::new(shape.left() + 1.0, shape.center().y),
            Vec2::new(2.0, shape.height() * 0.5 * active_t),
        );
        painter.rect_filled(bar, Rounding::same(1.0), accent);
    }

    painter.text(
        Pos2::new(shape.left() + 12.0, shape.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        text,
    );

    draw_ripple(ui, id, shape, accent);

    response
}

 
fn draw_ripple(ui: &Ui, id: Id, shape: Rect, accent: Color32) {
    let ctx = ui.ctx();
    let Some((origin, started)) = RIPPLES.lock().unwrap().get(&id).copied() else {
        return;
    };

    let t = ((ctx.input(|i| i.time) - started) / RIPPLE_TIME) as f32;
    if t >= 1.0 {
        RIPPLES.lock().unwrap().remove(&id);
        return;
    }

    let radius = shape.size().length() * 0.6 * Easing::Out.apply(t);
    ui.painter().with_clip_rect(shape).circle_filled(
        origin,
        radius,
        theme::with_alpha(accent, 0.28 * (1.0 - t)),
    );
    ctx.request_repaint();
}
