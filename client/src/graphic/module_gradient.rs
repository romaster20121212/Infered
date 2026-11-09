 
 
 
 
 
 
 
 
 
 
 
 
 

use egui::{pos2, Color32, Mesh, Painter, Rect, Shape};
use std::sync::atomic::{AtomicBool, Ordering};

 
static ENABLED: AtomicBool = AtomicBool::new(false);

 
 
 
 
const SHADE: f32 = 0.5;

 
const BODY: f32 = 0.92;

 
pub(crate) fn is_on() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

 
pub(crate) fn set_on(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

 
 
 
pub(crate) fn seed(name: &str) -> f32 {
     
    let mut hash: u32 = 2_166_136_261;
    for byte in name.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16_777_619);
    }
    (hash % 997) as f32 * 0.037
}

 
 
 
 
 
pub(crate) fn paint(
    painter: &Painter,
    rect: Rect,
    tint: Color32,
    seed: f32,
    time: f32,
    emphasis: f32,
) -> bool {
    if !is_on()
        || !time.is_finite()
        || !seed.is_finite()
        || !emphasis.is_finite()
        || emphasis <= 0.001
        || rect.width() < 4.0
        || rect.height() < 2.0
    {
        return false;
    }
    let emphasis = emphasis.clamp(0.0, 1.0);
     
     
    let cols = ((rect.width() / 7.0) as usize).clamp(4, 40);
    let rows = ((rect.height() / 7.0) as usize).clamp(2, 8);
     
     
    let aspect = rect.width() / rect.height().max(1.0);
     
     
    let light = crate::graphic::smoke_effects::glow_pulse(time);

    let mut mesh = Mesh::default();
    for row in 0..=rows {
        let v = row as f32 / rows as f32;
        for col in 0..=cols {
            let u = col as f32 / cols as f32;
            let point = [u * aspect * 2.4 + seed, v * 1.5, seed * 0.5 + 3.0];
            let [r, g, b] = crate::graphic::esp::liquid_sample(tint, point, time, light);
            let channel = |value: f32| (value * SHADE * 255.0).clamp(0.0, 255.0).round() as u8;
            mesh.colored_vertex(
                pos2(
                    rect.left() + u * rect.width(),
                    rect.top() + v * rect.height(),
                ),
                Color32::from_rgba_unmultiplied(
                    channel(r),
                    channel(g),
                    channel(b),
                    (255.0 * emphasis * BODY).round() as u8,
                ),
            );
        }
    }
    let stride = cols as u32 + 1;
    for row in 0..rows as u32 {
        for col in 0..cols as u32 {
            let i = row * stride + col;
            mesh.add_triangle(i, i + 1, i + stride);
            mesh.add_triangle(i + 1, i + stride + 1, i + stride);
        }
    }
    painter.add(Shape::mesh(mesh));
     
    painter.ctx().request_repaint();
    true
}

 
 
pub(crate) fn text_shadow() -> Color32 {
    Color32::from_black_alpha(160)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Id, LayerId, Order};

     
     
    fn guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

     
     
    #[test]
    fn the_setting_is_off_until_it_is_switched_on() {
        let _guard = guard();
        set_on(false);
        assert!(!is_on());
        set_on(true);
        assert!(is_on());
        set_on(false);
        assert!(!is_on());
    }

     
     
     
    #[test]
    fn each_module_gets_its_own_slice_of_the_material() {
        let sample = |name: &str, col: usize| {
            let point = [col as f32 * 0.31, seed(name), 3.0];
            crate::graphic::esp::liquid_sample(
                Color32::from_rgb(220, 40, 40),
                point,
                1.25,
                crate::graphic::smoke_effects::glow_pulse(1.25),
            )
        };
        assert_eq!(sample("KillAura", 3), sample("KillAura", 3));
        assert_ne!(sample("KillAura", 3), sample("Sprint", 3));
         
        for step in 0..64 {
            let s = seed(&format!("module {step}"));
            assert!(
                s.is_finite() && (0.0..40.0).contains(&s),
                "seed {s} is out of range"
            );
        }
    }

     
     
    #[test]
    fn nothing_is_painted_unless_the_effect_should_show() {
        let _guard = guard();
        let ctx = Context::default();
        let painter = ctx.layer_painter(LayerId::new(Order::Background, Id::new("gradient")));
        let rect = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(168.0, 24.0));
        let tint = Color32::from_rgb(220, 40, 40);

        set_on(false);
        assert!(!paint(&painter, rect, tint, seed("KillAura"), 1.0, 1.0));

        set_on(true);
         
         
        assert!(!paint(&painter, rect, tint, seed("KillAura"), 1.0, 0.0));
        assert!(!paint(
            &painter,
            rect,
            tint,
            seed("KillAura"),
            f32::NAN,
            1.0
        ));
        let tiny = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(2.0, 1.0));
        assert!(!paint(&painter, tiny, tint, seed("KillAura"), 1.0, 1.0));
         
        assert!(paint(&painter, rect, tint, seed("KillAura"), 1.0, 1.0));
        set_on(false);
    }
}
