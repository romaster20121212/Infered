 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::theme;
use egui::{Color32, Painter, Pos2, Rect, Rounding};
use std::sync::atomic::{AtomicU32, Ordering};

 
 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
     
     
    Window,
     
    Panel,
     
    Pane,
     
    Card,
     
     
    Chip,
}

impl Layer {
     
    fn give(self) -> f32 {
        match self {
            Layer::Window => 0.72,
            Layer::Panel => 0.62,
            Layer::Pane => 0.55,
            Layer::Card => 0.50,
            Layer::Chip => 0.34,
        }
    }

     
    fn fog(self) -> f32 {
        match self {
            Layer::Window => 1.0,
            Layer::Panel => 0.85,
            Layer::Pane => 0.70,
            Layer::Card => 0.60,
            Layer::Chip => 0.40,
        }
    }
}

 
 
static STRENGTH_BITS: AtomicU32 = AtomicU32::new(0);

 
pub fn strength() -> f32 {
    clamp_strength(f32::from_bits(STRENGTH_BITS.load(Ordering::Relaxed)))
}

 
 
pub fn set_strength(value: f32) {
    STRENGTH_BITS.store(clamp_strength(value).to_bits(), Ordering::Relaxed);
}

 
fn clamp_strength(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

 
pub fn active() -> bool {
    strength() > 0.001
}

 
 
const MIST: f32 = 0.16;

 
const FOG: Color32 = Color32::from_rgb(238, 240, 245);

 
fn keep_at(layer: Layer, strength: f32) -> f32 {
    1.0 - strength * layer.give()
}

 
fn mist_at(layer: Layer, strength: f32) -> f32 {
    strength * layer.fog() * MIST
}

 
pub fn keep(layer: Layer) -> f32 {
    keep_at(layer, strength())
}

 
 
 
 
 
 
fn fill_at(color: Color32, alpha: f32, layer: Layer, strength: f32) -> Color32 {
    let amount = mist_at(layer, strength);
    let mix = |channel: u8, fog: u8| {
        (channel as f32 + (fog as f32 - channel as f32) * amount).round() as u8
    };
    let alpha = (alpha.clamp(0.0, 1.0) * keep_at(layer, strength) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(
        mix(color.r(), FOG.r()),
        mix(color.g(), FOG.g()),
        mix(color.b(), FOG.b()),
        alpha,
    )
}

 
fn surface_at(alpha: f32, layer: Layer, strength: f32) -> Color32 {
    fill_at(theme::base(), alpha, layer, strength)
}

 
 
pub fn fill(color: Color32, alpha: f32, layer: Layer) -> Color32 {
    fill_at(color, alpha, layer, strength())
}

 
 
pub fn surface(alpha: f32, layer: Layer) -> Color32 {
    surface_at(alpha, layer, strength())
}

 
 
 
 
fn wash_at(strength: f32) -> f32 {
    1.0 - 0.45 * strength
}

 
pub fn wash() -> f32 {
    wash_at(strength())
}

 
 
 
 
 
 
pub fn edge(painter: &Painter, rect: Rect, rounding: Rounding, layer: Layer, alpha: f32) {
    edge_at(painter, rect, rounding, layer, alpha, strength());
}

 
 
fn edge_at(
    painter: &Painter,
    rect: Rect,
    rounding: Rounding,
    layer: Layer,
    alpha: f32,
    strength: f32,
) {
    let t = strength * layer.fog() * alpha.clamp(0.0, 1.0);
    if t <= 0.001 || rect.width() < 2.0 || rect.height() < 2.0 {
        return;
    }

     
    let left = rect.left() + rounding.nw.max(2.0);
    let right = rect.right() - rounding.ne.max(2.0);
    if right > left {
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(left, rect.top()),
                Pos2::new(right, rect.top() + 1.0),
            ),
            Rounding::ZERO,
            Color32::from_white_alpha((26.0 * t).round() as u8),
        );
    }

     
    let bottom_left = rect.left() + rounding.sw.max(2.0);
    let bottom_right = rect.right() - rounding.se.max(2.0);
    if bottom_right > bottom_left {
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(bottom_left, rect.bottom() - 1.0),
                Pos2::new(bottom_right, rect.bottom()),
            ),
            Rounding::ZERO,
            Color32::from_black_alpha((30.0 * t).round() as u8),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Vec2;

     
     
    #[test]
    fn the_default_is_solid() {
        assert_eq!(keep_at(Layer::Window, 0.0), 1.0);
        assert_eq!(wash_at(0.0), 1.0);
        assert_eq!(mist_at(Layer::Panel, 0.0), 0.0);
        assert_eq!(
            surface_at(0.9, Layer::Panel, 0.0),
            theme::with_alpha(theme::base(), 0.9)
        );
    }

    #[test]
    fn every_surface_thins_but_not_equally() {
         
         
        let keeps = [
            keep_at(Layer::Chip, 1.0),
            keep_at(Layer::Card, 1.0),
            keep_at(Layer::Pane, 1.0),
            keep_at(Layer::Panel, 1.0),
            keep_at(Layer::Window, 1.0),
        ];
        assert!(
            keeps.windows(2).all(|pair| pair[0] > pair[1]),
            "layers must thin unevenly, from chip to window: {keeps:?}"
        );
        assert!(keeps[4] > 0.2, "a window never becomes a hole: {keeps:?}");

         
         
        let (window, panel, solid) = (
            surface_at(0.9, Layer::Window, 1.0).a(),
            surface_at(0.9, Layer::Panel, 1.0).a(),
            surface_at(0.9, Layer::Panel, 0.0).a(),
        );
        assert!(window < panel && panel < solid, "{window} {panel} {solid}");
    }

     
     
    #[test]
    fn fog_is_light_in_the_fill_not_over_the_content() {
        let plain = fill_at(theme::base(), 1.0, Layer::Panel, 0.0);
        let fogged = fill_at(theme::base(), 1.0, Layer::Panel, 1.0);
         
         
        let brightness = |color: Color32| color.r() as f32 / color.a().max(1) as f32;
        assert!(
            brightness(fogged) > brightness(plain),
            "the fog lightens the body: {fogged:?} vs {plain:?}"
        );
         
        assert!(
            mist_at(Layer::Chip, 1.0) < mist_at(Layer::Window, 1.0),
            "a HUD chip carries the least fog"
        );
    }

    #[test]
    fn strength_folds_into_range() {
        assert_eq!(clamp_strength(4.0), 1.0);
        assert_eq!(clamp_strength(-2.0), 0.0);
        assert_eq!(clamp_strength(f32::NAN), 0.0);
        assert_eq!(clamp_strength(0.35), 0.35);
    }

    #[test]
    fn the_backdrop_lifts_as_the_interface_thins() {
        let opaque = wash_at(0.0);
        let half = wash_at(0.5);
        let thin = wash_at(1.0);
        assert!(opaque > half && half > thin, "{opaque} {half} {thin}");
        assert!(thin > 0.4, "the menu always keeps some wash: {thin}");
    }

     
     
    #[test]
    fn the_edge_paints_only_when_something_is_thin() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use egui::{Context, RawInput};
        use std::collections::HashMap;

        let render = |strength: f32| {
            let ctx = Context::default();
            let mut textures = HashMap::new();
            let mut primitives = Vec::new();
            for _ in 0..2 {
                let output = ctx.run(
                    RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(60.0, 40.0))),
                        ..Default::default()
                    },
                    |ctx| {
                        let painter = ctx.layer_painter(egui::LayerId::new(
                            egui::Order::Foreground,
                            egui::Id::new("fog_test"),
                        ));
                        edge_at(
                            &painter,
                            Rect::from_min_size(Pos2::ZERO, Vec2::new(60.0, 40.0)),
                            Rounding::same(6.0),
                            Layer::Panel,
                            1.0,
                            strength,
                        );
                    },
                );
                gather_textures(&mut textures, &output.textures_delta);
                primitives = ctx.tessellate(output.shapes, 1.0);
            }
            let image = rasterize(primitives, &textures, 60, 40, [0.0, 0.0, 0.0, 1.0]);
             
            image.get_pixel(30, 0)[0]
        };

        assert_eq!(render(0.0), 0, "no setting, no edge light");
        assert!(render(1.0) > 0, "a thinned surface carries a lit rim");
    }
}
