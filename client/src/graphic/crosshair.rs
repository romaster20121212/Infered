 
 
 
 
 
 

use crate::graphic::input::GUI_OPEN;
use crate::mapping::client::minecraft::Minecraft;
use crate::mapping::client::options::AttackIndicator;
use crate::mapping::entity::PlayerRef;
use crate::module::render::crosshair::{self, CooldownStyle, CrosshairConfig};
use crate::state::try_minecraft;
use egui::{
    Color32, Context, Id, LayerId, Mesh, Order, Painter, Pos2, Rect, Rounding, Shape, Stroke, Vec2,
};
use std::sync::atomic::Ordering;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

 
 
 
pub(crate) fn draw(ctx: &Context) {
    let cfg = crosshair::read_config();
    if !cfg.enabled || !crosshair::active() || GUI_OPEN.load(Ordering::Relaxed) {
        return;
    }
    let Some(minecraft) = try_minecraft() else {
        return;
    };
    if !minecraft.screen_is_null() {
        return;
    }
    if minecraft
        .options()
        .ok()
        .and_then(|options| options.hide_gui().ok())
        .unwrap_or(false)
    {
        return;
    }
    if !matches!(minecraft.world(), Ok(Some(_))) {
        return;
    }
    let Some(player) = minecraft.player().ok().flatten() else {
        return;
    };

     
     
     
     
     
    let strength = if wants_crosshair_indicator(&minecraft) {
        attack_cooldown(&player).ok().flatten()
    } else {
        None
    };
    let cooldown = strength.filter(|strength| *strength < 1.0);

     
    let flash = ready_flash(strength, Instant::now());

    let painter = ctx.layer_painter(LayerId::new(
        Order::Background,
        Id::new("infered_crosshair"),
    ));
    paint(&painter, ctx.screen_rect().center(), &cfg, cooldown, flash);
}

 
 
pub(crate) fn attack_cooldown(player: &impl PlayerRef) -> anyhow::Result<Option<f32>> {
    let strength = player.attack_strength_scale_at(0.0)?;
    Ok(strength.is_finite().then(|| strength.clamp(0.0, 1.0)))
}

 
const READY_FLASH: Duration = Duration::from_millis(380);
 
const READY_PULSE: f32 = 5.0;

#[derive(Default)]
struct ReadyState {
     
    charging: bool,
     
    at: Option<Instant>,
}

static READY: LazyLock<Mutex<ReadyState>> = LazyLock::new(|| Mutex::new(ReadyState::default()));

 
fn ready_flash(strength: Option<f32>, now: Instant) -> Option<f32> {
    let mut state = READY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    flash_from(&mut state, strength, now)
}

 
fn flash_from(state: &mut ReadyState, strength: Option<f32>, now: Instant) -> Option<f32> {
    let Some(strength) = strength.filter(|strength| strength.is_finite()) else {
        *state = ReadyState::default();
        return None;
    };
    if strength < 1.0 {
        state.charging = true;
        state.at = None;
        return None;
    }
    if state.charging {
        state.charging = false;
        state.at = Some(now);
    }
    let at = state.at?;
    let age = now.saturating_duration_since(at);
    if age >= READY_FLASH {
        state.at = None;
        return None;
    }
    let fade = 1.0 - age.as_secs_f32() / READY_FLASH.as_secs_f32();
    Some(fade.clamp(0.0, 1.0).powf(1.5))
}

 
 
 
fn wants_crosshair_indicator(minecraft: &Minecraft) -> bool {
    match minecraft
        .options()
        .map(|options| options.attack_indicator())
    {
        Ok(Ok(AttackIndicator::Off | AttackIndicator::Hotbar)) => false,
        _ => true,
    }
}

 
 
 
pub(crate) fn paint(
    painter: &Painter,
    center: Pos2,
    cfg: &CrosshairConfig,
    cooldown: Option<f32>,
    flash: Option<f32>,
) {
    for shape in reticle_shapes(center, cfg) {
        painter.add(shape);
    }
    match cfg.cooldown_style {
        CooldownStyle::Bar => {
            if let Some(strength) = cooldown {
                for shape in bar_shapes(center, cfg, strength) {
                    painter.add(shape);
                }
            }
        }
        CooldownStyle::Ring => {
            if let Some(strength) = cooldown {
                for shape in ring_shapes(center, cfg, strength, None) {
                    painter.add(shape);
                }
            } else if let Some(fade) = flash {
                for shape in ring_shapes(center, cfg, 0.0, Some(fade)) {
                    painter.add(shape);
                }
            }
        }
    }
}

 

 
const ARM_LENGTH: f32 = 6.4;

 
 
fn ring_inner_radius(cfg: &CrosshairConfig) -> f32 {
    (cfg.gap + ARM_LENGTH) * cfg.size + (6.5 * cfg.size).max(4.0)
}

fn ring_thickness(cfg: &CrosshairConfig) -> f32 {
    (3.2 * cfg.size).max(2.5)
}

 
 
 
fn ring_shapes(
    center: Pos2,
    cfg: &CrosshairConfig,
    strength: f32,
    flash: Option<f32>,
) -> Vec<Shape> {
    let strength = if strength.is_finite() {
        strength.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let inner = ring_inner_radius(cfg);
    let thickness = ring_thickness(cfg);
    let mid = inner + thickness * 0.5;
    let mut shapes = Vec::new();

     
    shapes.push(Shape::circle_stroke(
        center,
        mid,
        Stroke::new(thickness, Color32::from_black_alpha(175)),
    ));
    if cfg.outline {
        shapes.push(Shape::circle_stroke(
            center,
            mid + thickness * 0.5 + 0.5,
            Stroke::new(1.0_f32, Color32::from_white_alpha(45)),
        ));
    }

    if let Some(fade) = flash {
        let fade = fade.clamp(0.0, 1.0);
         
         
        let pulse = mid + (1.0 - fade) * READY_PULSE * cfg.size;
        shapes.push(Shape::circle_stroke(
            center,
            pulse,
            Stroke::new(thickness + 5.0, tint(cfg.color, (75.0 * fade) as u8)),
        ));
        shapes.push(Shape::circle_stroke(
            center,
            pulse,
            Stroke::new(thickness + 1.5, tint(cfg.color, (250.0 * fade) as u8)),
        ));
        return shapes;
    }

    if strength <= 0.002 {
        return shapes;
    }

     
     
    let color = mix_white(cfg.color, 0.35 * strength);
    let sweep = strength * std::f32::consts::TAU;
    if cfg.glow {
        shapes.push(arc_ribbon(
            center,
            inner - 3.5,
            inner + thickness + 3.5,
            sweep,
            tint(color, 48),
        ));
    }
    shapes.push(arc_ribbon(center, inner, inner + thickness, sweep, color));

    let start = center + Vec2::angled(-std::f32::consts::FRAC_PI_2) * mid;
    shapes.push(Shape::circle_filled(start, thickness * 0.5, color));
    let tip = center + Vec2::angled(-std::f32::consts::FRAC_PI_2 + sweep) * mid;
    shapes.push(Shape::circle_filled(
        tip,
        thickness * 0.55,
        mix_white(color, 0.55),
    ));
    shapes
}

 
 
fn arc_ribbon(center: Pos2, inner: f32, outer: f32, sweep: f32, color: Color32) -> Shape {
    const STEP: f32 = std::f32::consts::TAU / 96.0;
    let segments = ((sweep / STEP).ceil() as usize).max(2);
    let mut mesh = Mesh::default();
    for index in 0..=segments {
        let angle = -std::f32::consts::FRAC_PI_2 + sweep * index as f32 / segments as f32;
        let direction = Vec2::angled(angle);
        mesh.colored_vertex(center + direction * inner, color);
        mesh.colored_vertex(center + direction * outer, color);
    }
    for index in 0..segments as u32 {
        let base = index * 2;
        mesh.add_triangle(base, base + 1, base + 3);
        mesh.add_triangle(base, base + 3, base + 2);
    }
    Shape::mesh(mesh)
}

 
fn mix_white(color: Color32, amount: f32) -> Color32 {
    let amount = amount.clamp(0.0, 1.0);
    let blend = |channel: u8| (channel as f32 + (255.0 - channel as f32) * amount).round() as u8;
    Color32::from_rgba_unmultiplied(
        blend(color.r()),
        blend(color.g()),
        blend(color.b()),
        color.a(),
    )
}

 

 
 
fn bar_shapes(center: Pos2, cfg: &CrosshairConfig, strength: f32) -> Vec<Shape> {
    let size = cfg.size;
    let strength = if strength.is_finite() {
        strength.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let width = 16.0 * size;
    let height = (3.0 * size).max(2.0);
     
    let bottom = center.y + (cfg.gap + 6.4) * size;
    let track = Rect::from_min_size(
        Pos2::new(center.x - width * 0.5, bottom + 3.0 * size),
        Vec2::new(width, height),
    );
    let rounding = Rounding::same(height * 0.5);
    let mut shapes = Vec::new();
    if cfg.outline {
         
         
        shapes.push(Shape::rect_stroke(
            track.expand(1.0),
            rounding,
            Stroke::new(1.0_f32, Color32::from_white_alpha(80)),
        ));
    }
    shapes.push(Shape::rect_filled(
        track,
        rounding,
        Color32::from_black_alpha(150),
    ));
    let filled = width * strength;
    if filled > 0.5 {
        shapes.push(Shape::rect_filled(
            Rect::from_min_size(track.min, Vec2::new(filled, height)),
            rounding,
            cfg.color,
        ));
    }
    shapes
}

fn reticle_shapes(center: Pos2, cfg: &CrosshairConfig) -> Vec<Shape> {
    let size = cfg.size;
    let gap = cfg.gap * size;
    let length = 6.4 * size;
    let half = (cfg.thickness * size).max(0.5) * 0.5;
    let directions = [
        Vec2::new(1.0, 0.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, -1.0),
    ];
    let mut shapes = Vec::new();

    if cfg.glow {
        let glow = tint(cfg.color, 34);
        for direction in directions {
            shapes.push(Shape::convex_polygon(
                arm(center, direction, gap - 1.5, length + 3.0, half + 2.2),
                glow,
                Stroke::NONE,
            ));
        }
    }

    if cfg.outline {
        let ink = Color32::from_black_alpha(190);
        for direction in directions {
            shapes.push(Shape::convex_polygon(
                arm(center, direction, gap - 1.0, length + 1.6, half + 1.1),
                ink,
                Stroke::NONE,
            ));
        }
    }

    for direction in directions {
        shapes.push(Shape::convex_polygon(
            arm(center, direction, gap, length, half),
            cfg.color,
            Stroke::NONE,
        ));
        let tip = center + direction * (gap + length);
        shapes.push(Shape::circle_filled(
            tip,
            (half * 0.9).max(0.8),
            tint(cfg.color, 235),
        ));
    }

    if cfg.dot {
        let radius = 2.6 * size;
        if cfg.outline {
            shapes.push(diamond(
                center,
                radius + 1.2,
                Color32::from_black_alpha(190),
            ));
        }
        shapes.push(diamond(center, radius, cfg.color));
    }

    shapes
}

 
fn arm(center: Pos2, direction: Vec2, gap: f32, length: f32, half: f32) -> Vec<Pos2> {
    let perpendicular = Vec2::new(-direction.y, direction.x);
    let shoulder = gap + length * 0.62;
    vec![
        center + direction * gap + perpendicular * half,
        center + direction * shoulder + perpendicular * half,
        center + direction * (gap + length),
        center + direction * shoulder - perpendicular * half,
        center + direction * gap - perpendicular * half,
    ]
}

fn diamond(center: Pos2, radius: f32, fill: Color32) -> Shape {
    Shape::convex_polygon(
        vec![
            center + Vec2::new(0.0, -radius),
            center + Vec2::new(radius, 0.0),
            center + Vec2::new(0.0, radius),
            center + Vec2::new(-radius, 0.0),
        ],
        fill,
        Stroke::NONE,
    )
}

fn tint(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> CrosshairConfig {
        CrosshairConfig {
            enabled: true,
            ..CrosshairConfig::default()
        }
    }

    fn points(shape: &Shape) -> Vec<Pos2> {
        match shape {
            Shape::Path(path) => path.points.clone(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn the_default_reticle_has_four_spear_arms_and_a_center_diamond() {
        let shapes = reticle_shapes(Pos2::new(100.0, 50.0), &cfg());
         
         
        assert_eq!(shapes.len(), 4 + 4 + 8 + 2);
        let colored = shapes
            .iter()
            .filter(|shape| match shape {
                Shape::Path(path) => path.fill == cfg().color,
                _ => false,
            })
            .count();
        assert_eq!(
            colored, 5,
            "four arms and the centre diamond use the setting"
        );
    }

    #[test]
    fn every_shape_stays_inside_the_reticles_bounds() {
        let center = Pos2::new(200.0, 120.0);
        let cfg = CrosshairConfig {
            size: 2.0,
            gap: 10.0,
            ..cfg()
        };
        let shapes = reticle_shapes(center, &cfg);
        let reach = (cfg.gap + 6.4) * cfg.size + 4.0;
        for shape in shapes {
            for point in points(&shape) {
                assert!((point.x - center.x).abs() <= reach);
                assert!((point.y - center.y).abs() <= reach);
            }
        }
    }

    #[test]
    fn glow_outline_and_dot_are_independent() {
        let center = Pos2::new(0.0, 0.0);
        let plain = CrosshairConfig {
            glow: false,
            outline: false,
            dot: false,
            ..cfg()
        };
        assert_eq!(reticle_shapes(center, &plain).len(), 8);
        let with_dot = CrosshairConfig { dot: true, ..plain };
        assert_eq!(reticle_shapes(center, &with_dot).len(), 9);
        let with_outline = CrosshairConfig {
            outline: true,
            ..with_dot
        };
        assert_eq!(reticle_shapes(center, &with_outline).len(), 14);
        let with_glow = CrosshairConfig {
            glow: true,
            ..with_outline
        };
        assert_eq!(reticle_shapes(center, &with_glow).len(), 18);
    }

    fn rects(shapes: &[Shape]) -> Vec<Rect> {
        shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::Rect(rect) => Some(rect.rect),
                _ => None,
            })
            .collect()
    }

     
     
     
    #[test]
    fn the_cooldown_bar_fills_with_the_attack_strength() {
        let center = Pos2::new(100.0, 100.0);
        let cfg = CrosshairConfig {
            outline: false,
            ..cfg()
        };

        let half = rects(&bar_shapes(center, &cfg, 0.5));
        assert_eq!(half.len(), 2, "track and fill");
        assert_eq!(half[0].width(), 16.0);
        assert!((half[1].width() - 8.0).abs() < 0.01);
        assert_eq!(half[1].min, half[0].min);
        assert_eq!(half[1].height(), half[0].height());

        let full = rects(&bar_shapes(center, &cfg, 1.0));
        assert_eq!(full.len(), 2);
        assert!((full[1].width() - full[0].width()).abs() < 0.01);

        assert_eq!(rects(&bar_shapes(center, &cfg, 0.0)).len(), 1);
        for strength in [-3.0, f32::NAN, 9.0] {
            assert!(
                rects(&bar_shapes(center, &cfg, strength))
                    .iter()
                    .all(|rect| rect.width() <= 16.0 + 0.01),
                "a corrupt strength cannot paint outside the track"
            );
        }
    }

     
     
    #[test]
    fn the_cooldown_bar_hangs_below_the_reticle() {
        for (size, gap) in [(0.5, 0.0), (1.0, 3.5), (2.0, 10.0)] {
            let center = Pos2::new(200.0, 120.0);
            let cfg = CrosshairConfig { size, gap, ..cfg() };
            let track = rects(&bar_shapes(center, &cfg, 0.5))[0];
            let tip = center.y + (cfg.gap + 6.4) * cfg.size;
            assert!(
                track.min.y > tip,
                "the bar must start below the bottom arm (size {size})"
            );
            assert!(
                track.min.x < center.x && track.max.x > center.x,
                "the bar is centred"
            );
            assert_eq!(bar_shapes(center, &cfg, 0.5).len(), 3, "outline on");
        }
    }

    fn arc_points(shape: &Shape) -> Vec<Pos2> {
        match shape {
            Shape::Mesh(mesh) => mesh.vertices.iter().map(|vertex| vertex.pos).collect(),
            _ => Vec::new(),
        }
    }

     
    #[test]
    fn the_cooldown_ring_fills_clockwise_with_the_attack_strength() {
        let center = Pos2::new(200.0, 120.0);
        let cfg = CrosshairConfig {
            glow: false,
            outline: false,
            ..cfg()
        };
        let start = -std::f32::consts::FRAC_PI_2;
        let tau = std::f32::consts::TAU;
        for strength in [0.25_f32, 0.5, 0.8] {
            let shapes = ring_shapes(center, &cfg, strength, None);
             
            assert_eq!(shapes.len(), 4, "strength {strength}");
            let points = arc_points(&shapes[1]);
            assert!(!points.is_empty(), "the arc is a mesh");
            let span = points
                .iter()
                .map(|point| ((*point - center).angle() - start).rem_euclid(tau))
                .fold(0.0_f32, f32::max);
            assert!(
                (span - strength * tau).abs() < 0.06,
                "strength {strength} swept {span} rad"
            );
             
            let first = points[0];
            assert!(
                (first.x - center.x).abs() < 0.5 && first.y < center.y,
                "the arc starts at the top, got {first:?}"
            );
        }

         
        for strength in [0.0, -2.0, f32::NAN, f32::INFINITY] {
            assert_eq!(ring_shapes(center, &cfg, strength, None).len(), 1);
        }
    }

     
    #[test]
    fn the_cooldown_ring_sits_outside_the_reticle() {
        for (size, gap) in [(0.5, 0.0), (1.0, 3.5), (2.0, 10.0)] {
            let center = Pos2::new(80.0, 60.0);
            let cfg = CrosshairConfig {
                size,
                gap,
                outline: false,
                ..cfg()
            };
            let reach = (cfg.gap + 6.4) * cfg.size;
            let shapes = ring_shapes(center, &cfg, 1.0, None);
            let inner = ring_inner_radius(&cfg);
            assert!(
                inner > reach,
                "the ring must clear the arm tips (size {size}, gap {gap})"
            );
            for shape in shapes {
                for point in points(&shape).into_iter().chain(arc_points(&shape)) {
                    let radius = (point - center).length();
                    assert!(
                        radius > reach,
                        "a ring vertex at {radius} is inside the reticle reach {reach}"
                    );
                }
                if let Shape::Circle(circle) = &shape {
                     
                     
                    let offset = (circle.center - center).length();
                    let nearest = if offset < 0.01 {
                        circle.radius
                    } else {
                        offset - circle.radius
                    };
                    assert!(nearest > reach, "a ring circle reaches the reticle arms");
                }
            }
        }
    }

     
     
    #[test]
    fn the_cooldown_ring_warms_towards_white_as_it_completes() {
        let center = Pos2::new(0.0, 0.0);
        let cfg = CrosshairConfig {
            glow: false,
            outline: false,
            ..cfg()
        };
        let color_at = |strength: f32| match &ring_shapes(center, &cfg, strength, None)[1] {
            Shape::Mesh(mesh) => mesh.vertices[0].color,
            other => panic!("expected the arc mesh, got {other:?}"),
        };
        let early = color_at(0.05);
        let late = color_at(0.95);
        assert!(
            late.r() >= early.r() && late.g() > early.g() && late.b() > early.b(),
            "the arc should brighten as it fills: {early:?} -> {late:?}"
        );
    }

     
     
    #[test]
    fn the_completed_ring_flashes_once_and_fades() {
        let now = Instant::now();
        let mut state = ReadyState::default();
        assert_eq!(
            flash_from(&mut state, Some(0.5), now),
            None,
            "while charging"
        );
        let done = now + Duration::from_millis(500);
        let flash = flash_from(&mut state, Some(1.0), done).expect("the completion flashes");
        assert!(
            flash > 0.9,
            "the flash starts at full strength, got {flash}"
        );
        let later =
            flash_from(&mut state, Some(1.0), done + READY_FLASH / 2).expect("still fading");
        assert!(later < flash && later > 0.0, "fades, got {later}");
        assert_eq!(
            flash_from(
                &mut state,
                Some(1.0),
                done + READY_FLASH + Duration::from_millis(1)
            ),
            None
        );
        assert_eq!(
            flash_from(&mut state, Some(1.0), done + READY_FLASH * 4),
            None,
            "an idle frame does not start another flash"
        );

        assert_eq!(flash_from(&mut state, Some(0.5), done), None);
        assert_eq!(flash_from(&mut state, None, done), None);
        assert_eq!(
            flash_from(&mut state, Some(1.0), done),
            None,
            "a missing reading does not count as completion"
        );

         
        let cfg = cfg();
        let shapes = ring_shapes(Pos2::new(10.0, 10.0), &cfg, 0.0, Some(0.6));
        assert!(shapes.iter().all(|shape| matches!(shape, Shape::Circle(_))));
    }

     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn crosshair_preview() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;
        use std::path::Path;

        let (w, h) = (200usize, 200usize);
        let mut sheet = image::RgbaImage::new(w as u32 * 3, h as u32);
        for (index, (strength, flash)) in [
            (Some(0.33_f32), None),
            (Some(0.75), None),
            (None, Some(0.85)),
        ]
        .iter()
        .enumerate()
        {
            let ctx = Context::default();
            let mut textures = HashMap::new();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(w as f32, h as f32),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    let painter =
                        ctx.layer_painter(LayerId::new(Order::Background, Id::new("preview")));
                    paint(
                        &painter,
                        Pos2::new(w as f32 * 0.5, h as f32 * 0.5),
                        &cfg(),
                        *strength,
                        *flash,
                    );
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            let primitives = ctx.tessellate(output.shapes, 1.0);
            let cell = rasterize(primitives, &textures, w, h, [0.05, 0.04, 0.04, 1.0]);
            image::imageops::replace(&mut sheet, &cell, (index * w) as i64, 0);
        }
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("crosshair-cooldown.png");
        sheet.save(&path).expect("the crosshair preview saves");
        println!("crosshair preview: {path:?}");
    }

    #[test]
    fn the_center_gap_stays_open_for_aiming() {
        let center = Pos2::new(50.0, 50.0);
        let shapes = reticle_shapes(center, &cfg());
        for shape in shapes {
            for point in points(&shape) {
                let distance = ((point.x - center.x).powi(2) + (point.y - center.y).powi(2)).sqrt();
                assert!(
                    distance >= 1.0,
                    "the reticle must not fill its own centre, got {distance}"
                );
            }
        }
    }
}
