 
 
 
 
 
 
 
 
 

use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use egui::{pos2, vec2, Align2, Color32, Context, FontId, Painter, Rect, Rounding, Stroke, Vec2};

use crate::graphic::esp::{View, V3};
use crate::graphic::theme;
use crate::module::render::target_info::{self as info, Config, TargetInfo};

 
const PANEL_W: f32 = 216.0;
 
const PAD: f32 = 8.0;

 
#[derive(Default)]
struct Motion {
    alpha: f32,
    health: f32,
    absorption: f32,
     
    appear: f32,
    id: Option<i32>,
    last_frame: Option<Instant>,
}

static MOTION: LazyLock<Mutex<Motion>> = LazyLock::new(|| Mutex::new(Motion::default()));

 
 
 
pub(crate) fn reset_motion() {
    if let Ok(mut motion) = MOTION.lock() {
        *motion = Motion::default();
    }
}

 
 
 
pub(crate) fn panel_size() -> Vec2 {
    panel_size_for(&info::read_config())
}

fn panel_size_for(config: &Config) -> Vec2 {
    let scale = config.scale;
     
     
    let effects = if config.show_effects { 5 } else { 0 };
    let mut rows = 0usize;
    if config.show_armor {
        rows += 1;
    }
    if config.show_held_item {
        rows += 2;
    }
    rows += effects;
    if config.show_visibility {
        rows += 1;
    }
    if config.show_last_seen {
        rows += 1;
    }
    if config.show_method {
        rows += 1;
    }
    let header = header_height(config);
    let bar = if config.show_health_bar { 12.0 } else { 0.0 };
    let line = line_height(config);
    let values = if config.show_health || config.show_percentage {
        line
    } else {
        0.0
    };
    let height = PAD * 2.0 + header + 4.0 + bar + values + 4.0 + rows as f32 * (line + 1.0);
    Vec2::new(PANEL_W * scale, height * scale)
}

 
 
pub(crate) fn draw(ctx: &Context, painter: &Painter) {
    let Some((target, active)) = info::display() else {
        return;
    };
    let config = info::read_config();
     
     
    if config.physical {
        return;
    }
    let frame = Instant::now();
    let motion = advance_motion(&target, &config, active, frame);
    if motion.alpha < 0.012 {
        return;
    }
    let alpha = motion.alpha;

    let size = panel_size_for(&config);
    let target_pos =
        crate::graphic::hud::widget_rendered_pos(ctx, crate::graphic::hud::HudWidgetId::TargetInfo);
    let screen = ctx.screen_rect();
    let max_y = (screen.bottom() - size.y - 2.0).max(screen.top() + 2.0);
    let origin = pos2(
        target_pos
            .x
            .min(screen.right() - size.x - 2.0)
            .max(screen.left() + 2.0),
        target_pos.y.min(max_y).max(screen.top() + 2.0),
    );

    let mut painter = painter.clone();
    painter.set_opacity(alpha);
    draw_at(
        ctx,
        &painter,
        origin,
        &target,
        &config,
        motion.health,
        motion.absorption,
        true,
        frame,
    );
}

 
struct MotionValues {
    alpha: f32,
    health: f32,
    absorption: f32,
    appear: f32,
}

 
 
 
fn advance_motion(
    target: &TargetInfo,
    config: &Config,
    active: bool,
    now: Instant,
) -> MotionValues {
    let mut motion = match MOTION.lock() {
        Ok(motion) => motion,
        Err(poisoned) => poisoned.into_inner(),
    };
    let dt = motion
        .last_frame
        .map(|last| now.duration_since(last).as_secs_f32().min(0.1))
        .unwrap_or(0.016);
    motion.last_frame = Some(now);

    let speed = config.animation_speed.max(0.1);
    let target_alpha = if active { 1.0 } else { 0.0 };
    let alpha_rate = 1.0 - (-dt * 10.0 * speed).exp();
    motion.alpha += (target_alpha - motion.alpha) * alpha_rate;
    if motion.alpha < 0.012 && !active {
        motion.alpha = 0.0;
    }

    if motion.id != Some(target.id) {
        motion.id = Some(target.id);
        motion.health = target.health_fraction();
        motion.absorption = target.absorption_fraction();
         
        motion.appear = 0.0;
    }
    let bar_rate = 1.0 - (-dt * 9.0 * speed).exp();
    motion.health += (target.health_fraction() - motion.health) * bar_rate;
    motion.absorption += (target.absorption_fraction() - motion.absorption) * bar_rate;
    let appear_rate = 1.0 - (-dt * 7.0 * speed).exp();
    motion.appear += ((if active { 1.0 } else { 0.0 }) - motion.appear) * appear_rate;

    MotionValues {
        alpha: motion.alpha.clamp(0.0, 1.0),
        health: motion.health,
        absorption: motion.absorption,
        appear: motion.appear.clamp(0.0, 1.0),
    }
}

 
 
pub(crate) fn draw_preview(ctx: &Context, painter: &Painter, origin: egui::Pos2) {
    let config = info::read_config();
    let now = Instant::now();
    let sample = TargetInfo {
        id: i32::MIN,
        name: "Alex".to_string(),
        entity_type: "RemotePlayer".to_string(),
        distance: 8.4,
        health: 14.0,
        max_health: 20.0,
        absorption: 4.0,
        armor: 12,
        armor_pieces: 3,
        main_hand: None,
        off_hand: None,
        effects: vec![
            ("speed".to_string(), 1, 1200),
            ("regeneration".to_string(), 0, 340),
        ],
        feet: [0.0, 64.0, 0.0],
        yaw: 90.0,
        height: 1.8,
        visible: true,
        invisible: false,
        is_player: true,
        skin_url: None,
        method: "Crosshair",
        first_seen: now,
        last_seen: now,
        last_damage: Some(6.0),
        last_damage_at: Some(now),
    };
    draw_at(
        ctx,
        painter,
        origin,
        &sample,
        &config,
        14.0 / 20.0,
        4.0 / 20.0,
        false,
        now,
    );
}

 

 
const ANCHOR_TICK: Duration = Duration::from_millis(50);
 
 
const ANCHOR_STALE: Duration = Duration::from_millis(500);

 
#[derive(Debug, Clone, Copy)]
struct AnchorSample {
    id: i32,
    feet: V3,
    yaw: f32,
    height: f64,
}

#[derive(Default)]
struct AnchorState {
    prev: Option<AnchorSample>,
    current: Option<AnchorSample>,
    at: Option<Instant>,
}

static ANCHOR: LazyLock<Mutex<AnchorState>> = LazyLock::new(|| Mutex::new(AnchorState::default()));

 
pub(crate) fn publish_anchor(id: i32, feet: V3, yaw: f32, height: f64, now: Instant) {
    let mut state = ANCHOR
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let sample = AnchorSample {
        id,
        feet,
        yaw,
        height,
    };
    state.prev = state
        .current
        .filter(|current| current.id == id)
        .or(Some(sample));
    state.current = Some(sample);
    state.at = Some(now);
}

 
pub(crate) fn clear_anchor() {
    if let Ok(mut state) = ANCHOR.lock() {
        *state = AnchorState::default();
    }
}

 
 
fn anchor_for(target: &TargetInfo, now: Instant) -> AnchorSample {
    let snapshot = AnchorSample {
        id: target.id,
        feet: V3 {
            x: target.feet[0],
            y: target.feet[1],
            z: target.feet[2],
        },
        yaw: target.yaw,
        height: target.height.max(0.6),
    };
    let state = ANCHOR
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (Some(current), Some(at)) = (state.current, state.at) else {
        return snapshot;
    };
    if current.id != target.id || now.saturating_duration_since(at) > ANCHOR_STALE {
        return snapshot;
    }
    let previous = state
        .prev
        .filter(|previous| previous.id == current.id)
        .unwrap_or(current);
    let t = (now.saturating_duration_since(at).as_secs_f32() / ANCHOR_TICK.as_secs_f32())
        .clamp(0.0, 1.0);
    AnchorSample {
        id: current.id,
        feet: previous.feet.lerp(current.feet, t as f64),
        yaw: lerp_angle(previous.yaw, current.yaw, t),
        height: previous.height + (current.height - previous.height) * t as f64,
    }
}

 
fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let mut delta = (to - from) % 360.0;
    if delta > 180.0 {
        delta -= 360.0;
    }
    if delta < -180.0 {
        delta += 360.0;
    }
    from + delta * t
}

 
 
fn side_vectors(yaw_degrees: f32) -> (V3, V3) {
    let yaw = (yaw_degrees as f64).to_radians();
    let left = V3 {
        x: yaw.cos(),
        y: 0.0,
        z: yaw.sin(),
    };
    (
        left,
        V3 {
            x: -left.x,
            y: 0.0,
            z: -left.z,
        },
    )
}

 
 
fn world_text(
    painter: &Painter,
    pos: egui::Pos2,
    align: Align2,
    text: String,
    font: FontId,
    color: Color32,
) {
    painter.text(
        pos + vec2(1.0, 1.0),
        align,
        text.clone(),
        font.clone(),
        Color32::from_black_alpha(170),
    );
    painter.text(pos, align, text, font, color);
}

fn pixels_per_block(view: &View, point: V3) -> Option<f32> {
    let depth = point.sub(view.cam).dot(view.fwd);
    if depth < 0.05 {
        return None;
    }
    Some((view.h as f64 * 0.5 / (view.tan_y * depth)) as f32)
}

 
 
 
pub(crate) fn draw_physical(ctx: &Context, painter: &Painter, view: &View, now: Instant) {
    let Some((target, active)) = info::display() else {
        return;
    };
    let config = info::read_config();
    if !config.enabled || !config.physical {
        return;
    }
    let motion = advance_motion(&target, &config, active, now);
    if motion.alpha < 0.012 || motion.appear < 0.01 {
        return;
    }
    let anchor = anchor_for(&target, now);
    let mut painter = painter.clone();
    painter.set_opacity(motion.alpha);
    draw_physical_at(ctx, &painter, view, &target, &config, &motion, anchor, now);
}

#[allow(clippy::too_many_arguments)]
fn draw_physical_at(
    ctx: &Context,
    painter: &Painter,
    view: &View,
    target: &TargetInfo,
    config: &Config,
    motion: &MotionValues,
    anchor: AnchorSample,
    now: Instant,
) {
    let appear = motion.appear.clamp(0.0, 1.0);
    let opacity = config.opacity;
    let scale = config.scale * (config.text_size / 11.0);
    let feet = anchor.feet;
    let chest = feet.y + anchor.height * 0.62;
    let (left, right) = side_vectors(anchor.yaw);
     
    let spread = (anchor.height * 0.34).max(0.48) * (0.45 + 0.55 * appear as f64);
    let rise = (1.0 - appear) as f64 * 0.18;
    let left_point = V3 {
        x: feet.x + left.x * spread,
        y: chest + rise,
        z: feet.z + left.z * spread,
    };
    let right_point = V3 {
        x: feet.x + right.x * spread,
        y: chest + rise,
        z: feet.z + right.z * spread,
    };
    let head_point = V3 {
        x: feet.x,
        y: feet.y + anchor.height + 0.30 + rise,
        z: feet.z,
    };
    if view
        .cam
        .sub(V3 {
            x: feet.x,
            y: chest,
            z: feet.z,
        })
        .length()
        < 0.9
    {
         
        return;
    }

     
    if let (Some(screen), Some(ppb)) =
        (view.project(left_point), pixels_per_block(view, left_point))
    {
        let font = FontId::proportional((ppb * 0.042 * scale).clamp(9.0, 20.0));
        let small = FontId::proportional((ppb * 0.036 * scale).clamp(8.0, 17.0));
        let bar_h = (ppb * 0.60 * scale * (0.55 + 0.45 * appear)).clamp(26.0, 210.0);
        let bar_w = (ppb * 0.055 * scale).clamp(3.5, 14.0);
        let track = Rect::from_center_size(screen, vec2(bar_w, bar_h));

        if config.show_health_bar {
            painter.rect_filled(
                track.expand(1.5),
                Rounding::same(bar_w),
                Color32::from_black_alpha((150.0 * opacity) as u8),
            );
            painter.rect_filled(
                track,
                Rounding::same(bar_w * 0.5),
                Color32::from_black_alpha(150),
            );
            let filled = bar_h * motion.health.clamp(0.0, 1.0);
            if filled > 0.5 {
                let fill = Rect::from_min_max(
                    pos2(track.left(), track.bottom() - filled),
                    pos2(track.right(), track.bottom()),
                );
                painter.rect_filled(
                    fill,
                    Rounding::same(bar_w * 0.5),
                    color(config.health_color, 1.0),
                );
            }
            if config.show_absorption && target.absorption > 0.0 {
                let top = track.bottom() - filled;
                let absorb = (bar_h * motion.absorption.clamp(0.0, 1.0)).min(top - track.top());
                if absorb > 0.5 {
                    painter.rect_filled(
                        Rect::from_min_max(
                            pos2(track.left(), top - absorb),
                            pos2(track.right(), top),
                        ),
                        Rounding::same(bar_w * 0.5),
                        Color32::from_rgb(245, 190, 70),
                    );
                }
            }
            painter.rect_stroke(
                track,
                Rounding::same(bar_w * 0.5),
                Stroke::new(1.0_f32, with_alpha(color(config.accent_color, 1.0), 0.45)),
            );
        }

        if config.show_health || config.show_percentage {
            let healthy = target.max_health > 0.0;
            let text = if !healthy {
                "—".to_string()
            } else if config.show_health && config.show_percentage {
                format!(
                    "{:.1}  ·  {:.0}%",
                    target.health,
                    target.health_fraction() * 100.0
                )
            } else if config.show_health {
                format!("{:.1} / {:.0}", target.health, target.max_health)
            } else {
                format!("{:.0}%", target.health_fraction() * 100.0)
            };
            world_text(
                painter,
                pos2(track.left() - 6.0, track.top() - 2.0),
                Align2::RIGHT_BOTTOM,
                text,
                font.clone(),
                if healthy {
                    theme::TEXT
                } else {
                    theme::TEXT_MUTED
                },
            );
            if config.show_damage {
                if let (Some(loss), Some(at)) = (target.last_damage, target.last_damage_at) {
                    let age = now.duration_since(at);
                    if age < info::damage_window() {
                        let fade = 1.0 - age.as_secs_f32() / info::damage_window().as_secs_f32();
                        world_text(
                            painter,
                            pos2(track.left() - 6.0, track.top() - 2.0 - font.size),
                            Align2::RIGHT_BOTTOM,
                            format!("-{loss:.1}"),
                            small.clone(),
                            with_alpha(theme::DANGER, fade),
                        );
                    }
                }
            }
        }

        if config.show_effects && !target.effects.is_empty() {
            let mut y = track.bottom() + 4.0;
            for (name, amplifier, duration) in target.effects.iter().take(3) {
                world_text(
                    painter,
                    pos2(track.left() - 6.0, y),
                    Align2::RIGHT_TOP,
                    format!(
                        "{} {}  {}",
                        pretty_effect(name),
                        roman(*amplifier + 1),
                        duration_label(*duration)
                    ),
                    small.clone(),
                    theme::TEXT_DIM,
                );
                y += small.size + 1.0;
            }
            if target.effects.len() > 3 {
                world_text(
                    painter,
                    pos2(track.left() - 6.0, y),
                    Align2::RIGHT_TOP,
                    format!("+{} more", target.effects.len() - 3),
                    small.clone(),
                    theme::TEXT_MUTED,
                );
            }
        }
    }

     
    if let (Some(screen), Some(ppb)) = (
        view.project(right_point),
        pixels_per_block(view, right_point),
    ) {
        let small = FontId::proportional((ppb * 0.036 * scale).clamp(8.0, 17.0));
        let icon = (ppb * 0.17 * scale * appear).clamp(14.0, 52.0);
        let mut y = screen.y - icon * 0.9;
        if config.show_held_item {
            for stack in [target.main_hand.as_ref(), target.off_hand.as_ref()]
                .into_iter()
                .flatten()
            {
                let rect = Rect::from_min_size(pos2(screen.x, y), Vec2::splat(icon));
                if let Some((texture, tint)) = crate::graphic::item_icon::icon(ctx, &stack.name) {
                    painter.rect_filled(
                        rect,
                        Rounding::same(3.0),
                        with_alpha(tint, 0.28 * opacity),
                    );
                    painter.image(
                        texture,
                        rect.shrink(icon * 0.08),
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                world_text(
                    painter,
                    pos2(rect.right() + 5.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    hand_label(Some(stack)),
                    small.clone(),
                    theme::TEXT,
                );
                y += icon + 3.0;
            }
        }
        if config.show_armor {
            world_text(
                painter,
                pos2(screen.x, y + 2.0),
                Align2::LEFT_TOP,
                if target.armor > 0 || target.armor_pieces > 0 {
                    format!("Armor {}  ·  {} pieces", target.armor, target.armor_pieces)
                } else {
                    "No armor".to_string()
                },
                small.clone(),
                theme::TEXT_DIM,
            );
            y += small.size + 2.0;
        }
        if config.show_distance {
            world_text(
                painter,
                pos2(screen.x, y + 2.0),
                Align2::LEFT_TOP,
                format!("{:.1} m", target.distance),
                small.clone(),
                theme::TEXT_MUTED,
            );
        }
    }

     
    if let (Some(screen), Some(ppb)) =
        (view.project(head_point), pixels_per_block(view, head_point))
    {
        let title = FontId::proportional((ppb * 0.046 * scale).clamp(10.0, 21.0));
        let small = FontId::proportional((ppb * 0.036 * scale).clamp(8.0, 17.0));
        if config.show_name {
            world_text(
                painter,
                screen,
                Align2::CENTER_BOTTOM,
                target.name.clone(),
                title,
                theme::TEXT,
            );
        }
        let mut y = screen.y + 1.0;
        if config.show_entity_type {
            world_text(
                painter,
                pos2(screen.x, y),
                Align2::CENTER_TOP,
                target.entity_type.clone(),
                small.clone(),
                with_alpha(color(config.accent_color, 1.0), 0.85),
            );
            y += small.size + 1.0;
        }
        if config.show_visibility && !target.visible {
            world_text(
                painter,
                pos2(screen.x, y),
                Align2::CENTER_TOP,
                if target.invisible {
                    "Invisible".to_string()
                } else {
                    "Blocked from view".to_string()
                },
                small,
                theme::WARN,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_at(
    ctx: &Context,
    painter: &Painter,
    origin: egui::Pos2,
    target: &TargetInfo,
    config: &Config,
    health: f32,
    absorption: f32,
    live: bool,
    now: Instant,
) {
    let s = config.scale;
    let font = FontId::proportional(config.text_size * s);
    let small = FontId::proportional((config.text_size - 2.0).max(7.0) * s);
    let title = FontId::proportional((config.text_size + 1.5) * s);
    let pad = PAD * s;
    let line = line_height(config) * s;
    let width = PANEL_W * s;

     
    let mut rows: Vec<(Option<&str>, String, Color32)> = Vec::new();
    if config.show_armor {
        rows.push((
            None,
            if target.armor > 0 || target.armor_pieces > 0 {
                format!("Armor {}  ·  {} pieces", target.armor, target.armor_pieces)
            } else {
                "Armor —".to_string()
            },
            theme::TEXT_DIM,
        ));
    }
    if config.show_held_item {
        rows.push((
            target.main_hand.as_ref().map(|stack| stack.name.as_str()),
            hand_label(target.main_hand.as_ref()),
            theme::TEXT,
        ));
        if let Some(off) = &target.off_hand {
            rows.push((
                Some(off.name.as_str()),
                format!("Off hand: {}", pretty_item(&off.name)),
                theme::TEXT_DIM,
            ));
        }
    }
    if config.show_effects {
        for (name, amplifier, duration) in target.effects.iter().take(4) {
            rows.push((
                None,
                format!(
                    "{} {}  ·  {}",
                    pretty_effect(name),
                    roman(*amplifier + 1),
                    duration_label(*duration)
                ),
                theme::TEXT_DIM,
            ));
        }
        if target.effects.len() > 4 {
            rows.push((
                None,
                format!("+{} more effects", target.effects.len() - 4),
                theme::TEXT_MUTED,
            ));
        }
    }
    if config.show_visibility {
        rows.push((
            None,
            if target.visible {
                "Visible to you".to_string()
            } else if target.invisible {
                "Invisible".to_string()
            } else {
                "Blocked from view".to_string()
            },
            if target.visible {
                theme::TEXT_DIM
            } else {
                theme::WARN
            },
        ));
    }
    if config.show_last_seen {
        let age = now.duration_since(target.last_seen);
        rows.push((
            None,
            format!("Last seen {}", short_duration(age)),
            theme::TEXT_MUTED,
        ));
    }
    if config.show_method {
        rows.push((
            None,
            format!("Source: {}", target.method),
            theme::TEXT_MUTED,
        ));
    }

    let header = header_height(config) * s;
    let bar_space = if config.show_health_bar {
        12.0 * s
    } else {
        0.0
    };
    let values_space = if config.show_health || config.show_percentage {
        line
    } else {
        0.0
    };
    let height = pad * 2.0
        + header
        + 4.0 * s
        + bar_space
        + values_space
        + if rows.is_empty() { 0.0 } else { 4.0 * s }
        + rows.len() as f32 * (line + 1.0 * s);
    let rect = Rect::from_min_size(origin, vec2(width, height));
    let radius = Rounding::same(config.corner_radius * s);

     
    painter.rect_filled(
        rect.translate(vec2(0.0, 2.0 * s)),
        radius,
        Color32::from_black_alpha((config.opacity * 60.0) as u8),
    );
    match config.background_style {
        1 => {
            painter.rect_filled(rect, radius, color(config.background_color, config.opacity));
        }
        2 => {}
        _ => {
            painter.rect_filled(rect, radius, color(config.background_color, config.opacity));
            let sheen = Rect::from_min_size(rect.min, vec2(rect.width(), rect.height() * 0.42));
            painter.rect_filled(
                sheen,
                Rounding {
                    nw: radius.nw,
                    ne: radius.ne,
                    sw: 0.0,
                    se: 0.0,
                },
                color(config.accent_color, 0.045 * config.opacity),
            );
        }
    }

    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0_f32, with_alpha(color(config.accent_color, 1.0), 0.34)),
    );
    painter.rect_filled(
        Rect::from_min_size(rect.min, vec2(2.0 * s, rect.height())),
        Rounding::same(1.0 * s),
        with_alpha(color(config.accent_color, 1.0), 0.9),
    );

    let mut y = rect.top() + pad;
    let text_x = if config.show_head {
        let head = Rect::from_min_size(pos2(rect.left() + pad, y), Vec2::splat(26.0 * s));
        draw_head(ctx, painter, head, target, config, live);
        head.right() + 8.0 * s
    } else {
        rect.left() + pad
    };

    let title_size = (config.text_size + 1.5) * s;
    let distance_width = if config.show_distance {
        config.text_size * 4.2 * s
    } else {
        0.0
    };
    if config.show_name {
        painter.text(
            pos2(text_x, y + 1.0 * s),
            Align2::LEFT_TOP,
            truncate(
                &target.name,
                fit_chars(rect.right() - pad - text_x - distance_width, title_size),
            ),
            title.clone(),
            theme::TEXT,
        );
    }
    if config.show_entity_type {
        painter.text(
            pos2(text_x, y + (config.text_size * 1.35 + 1.0) * s),
            Align2::LEFT_TOP,
            truncate(
                &target.entity_type,
                fit_chars(
                    rect.right() - pad - text_x,
                    (config.text_size - 2.0).max(7.0) * s,
                ),
            ),
            small.clone(),
            with_alpha(color(config.accent_color, 1.0), 0.85),
        );
    }
    if config.show_distance {
        painter.text(
            pos2(rect.right() - pad, y + 1.0 * s),
            Align2::RIGHT_TOP,
            format!("{:.1} m", target.distance),
            font.clone(),
            theme::TEXT,
        );
    }
    y += header + 4.0 * s;

    let max_health = target.max_health.max(0.0);
    let healthy = max_health > 0.0;
    if config.show_health_bar {
        let track =
            Rect::from_min_size(pos2(rect.left() + pad, y), vec2(width - pad * 2.0, 6.0 * s));
        painter.rect_filled(
            track,
            Rounding::same(3.0 * s),
            Color32::from_black_alpha(150),
        );
        let health_fill = (width - pad * 2.0) * health.clamp(0.0, 1.0);
        if config.health_bar_style == 1 {
            let segments = 10;
            let gap = 2.0 * s;
            let segment_w = (track.width() - gap * (segments - 1) as f32) / segments as f32;
            for index in 0..segments {
                let start = index as f32 * (segment_w + gap);
                let filled = start < health_fill;
                let segment = Rect::from_min_size(
                    pos2(track.left() + start, track.top()),
                    vec2(segment_w, track.height()),
                );
                painter.rect_filled(
                    segment,
                    Rounding::same(1.5 * s),
                    if filled {
                        color(config.health_color, 1.0)
                    } else {
                        with_alpha(theme::TEXT_MUTED, 0.18)
                    },
                );
            }
        } else {
            if healthy {
                painter.rect_filled(
                    Rect::from_min_size(track.min, vec2(health_fill, track.height())),
                    Rounding::same(3.0 * s),
                    color(config.health_color, 1.0),
                );
            }
        }
        if config.show_absorption && target.absorption > 0.0 && healthy {
            let start = health_fill;
            let end =
                ((health + absorption).clamp(0.0, 1.0) * (width - pad * 2.0)).max(start + 2.0 * s);
            painter.rect_filled(
                Rect::from_min_size(
                    pos2(track.left() + start, track.top() - 1.0 * s),
                    vec2(
                        (end - start).min(track.width() - start),
                        track.height() + 2.0 * s,
                    ),
                ),
                Rounding::same(2.0 * s),
                Color32::from_rgb(245, 190, 70),
            );
        }
        y += bar_space;
    }

    if config.show_health || config.show_percentage {
        let health_text = if healthy {
            if config.show_health && config.show_percentage {
                format!(
                    "{:.1} / {:.0}  ·  {:.0}%",
                    target.health,
                    max_health,
                    target.health_fraction() * 100.0
                )
            } else if config.show_health {
                format!("{:.1} / {:.0}", target.health, max_health)
            } else {
                format!("{:.0}%", target.health_fraction() * 100.0)
            }
        } else {
            "Health unavailable".to_string()
        };
        painter.text(
            pos2(rect.left() + pad, y),
            Align2::LEFT_TOP,
            health_text,
            font.clone(),
            if healthy {
                theme::TEXT
            } else {
                theme::TEXT_MUTED
            },
        );
        if config.show_absorption && target.absorption > 0.0 {
            painter.text(
                pos2(rect.right() - pad, y),
                Align2::RIGHT_TOP,
                format!("+{:.1}", target.absorption),
                font.clone(),
                Color32::from_rgb(245, 190, 70),
            );
        }
        if config.show_damage {
            if let (Some(loss), Some(at)) = (target.last_damage, target.last_damage_at) {
                let age = now.duration_since(at);
                if age < info::damage_window() {
                    let fade = 1.0 - age.as_secs_f32() / info::damage_window().as_secs_f32();
                    painter.text(
                        pos2(rect.right() - pad, y - 11.0 * s),
                        Align2::RIGHT_TOP,
                        format!("-{loss:.1}"),
                        small.clone(),
                        with_alpha(theme::DANGER, fade),
                    );
                }
            }
        }
        y += values_space;
    }

    if !rows.is_empty() {
        y += 4.0 * s;
    }
    let row_max = fit_chars(
        (width - pad * 2.0 - 15.0 * s).max(20.0),
        (config.text_size - 2.0).max(7.0) * s,
    );
    for (icon, text, color) in rows {
        let row_y = y;
        if let Some(item) = icon {
            let icon_rect = Rect::from_min_size(
                pos2(rect.left() + pad, row_y - 1.0 * s),
                Vec2::splat(11.0 * s),
            );
            if let Some((texture, tint)) = crate::graphic::item_icon::icon(ctx, item) {
                painter.rect_filled(icon_rect, Rounding::same(2.0 * s), with_alpha(tint, 0.22));
                painter.image(
                    texture,
                    icon_rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        let row_x = if icon.is_some() {
            rect.left() + pad + 15.0 * s
        } else {
            rect.left() + pad
        };
        painter.text(
            pos2(row_x, row_y),
            Align2::LEFT_TOP,
            truncate(&text, row_max),
            small.clone(),
            color,
        );
        y += line + 1.0 * s;
    }
}

fn draw_head(
    ctx: &Context,
    painter: &Painter,
    rect: Rect,
    target: &TargetInfo,
    config: &Config,
    live: bool,
) {
    if target.is_player {
        if live {
            if let Some(texture) = crate::graphic::esp::skin_texture(
                ctx,
                target.id,
                target.skin_url.as_deref(),
                &target.name,
            ) {
                crate::graphic::player_card::paint_head(painter, rect, texture);
                return;
            }
        }
        painter.rect_filled(
            rect,
            Rounding::same(4.0 * config.scale),
            with_alpha(color(config.accent_color, 1.0), 0.18),
        );
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            target
                .name
                .chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string(),
            FontId::proportional(15.0 * config.scale),
            theme::TEXT,
        );
    } else {
        painter.rect_filled(
            rect,
            Rounding::same(4.0 * config.scale),
            with_alpha(color(config.accent_color, 1.0), 0.16),
        );
        painter.circle_stroke(
            rect.center(),
            8.0 * config.scale,
            Stroke::new(1.4_f32, color(config.accent_color, 1.0)),
        );
        painter.circle_filled(
            rect.center(),
            2.4 * config.scale,
            color(config.accent_color, 1.0),
        );
    }
}

 
 
fn line_height(config: &Config) -> f32 {
    (config.text_size * 1.2 + 1.0).max(13.0)
}

 
 
fn header_height(config: &Config) -> f32 {
    if config.show_head {
        30.0_f32.max(config.text_size * 2.0 + 4.0)
    } else {
        18.0_f32.max(config.text_size * 1.3 + 2.0)
    }
}

fn hand_label(stack: Option<&crate::mapping::entity::player::GearStack>) -> String {
    match stack {
        Some(stack) => {
            let mut label = pretty_item(&stack.name);
            if stack.count > 1 {
                label.push_str(&format!(" ×{}", stack.count));
            }
            if stack.max_damage > 0 {
                let left = 1.0 - stack.damage as f32 / stack.max_damage as f32;
                label.push_str(&format!("  {:.0}%", left * 100.0));
            }
            label
        }
        None => "Empty hand".to_string(),
    }
}

fn pretty_item(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn pretty_effect(name: &str) -> String {
    pretty_item(name)
}

fn roman(value: i32) -> String {
    match value {
        1 => "I".to_string(),
        2 => "II".to_string(),
        3 => "III".to_string(),
        4 => "IV".to_string(),
        5 => "V".to_string(),
        _ => value.to_string(),
    }
}

fn duration_label(ticks: i32) -> String {
    if ticks <= 0 {
        return "0:00".to_string();
    }
    if ticks > 20 * 60 * 60 {
        return "∞".to_string();
    }
    let seconds = ticks / 20;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn short_duration(duration: Duration) -> String {
    let secs = duration.as_secs_f32();
    if secs < 1.0 {
        format!("{:.1}s ago", secs)
    } else if secs < 60.0 {
        format!("{:.0}s ago", secs)
    } else {
        format!("{:.0}m ago", secs / 60.0)
    }
}

fn color(value: [f32; 4], opacity: f32) -> Color32 {
    let channel = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(
        channel(value[0]),
        channel(value[1]),
        channel(value[2]),
        (channel(value[3]) as f32 * opacity).round() as u8,
    )
}

fn with_alpha(color: Color32, factor: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (color.a() as f32 * factor.clamp(0.0, 1.0)).round() as u8,
    )
}

 
 
fn fit_chars(available: f32, font_size: f32) -> usize {
    if font_size <= 0.0 {
        return 1;
    }
    (available / (font_size * 0.55)).floor().max(4.0) as usize
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

 
#[cfg(test)]
fn toggle(config: &Config, setting: &str) -> bool {
    match setting {
        "Name" => config.show_name,
        "Head" => config.show_head,
        "Health Bar" => config.show_health_bar,
        "Health" => config.show_health,
        "Percentage" => config.show_percentage,
        "Absorption" => config.show_absorption,
        "Distance" => config.show_distance,
        "Armor" => config.show_armor,
        "Held Item" => config.show_held_item,
        "Effects" => config.show_effects,
        "Damage" => config.show_damage,
        "Entity Type" => config.show_entity_type,
        "Visibility" => config.show_visibility,
        "Last Seen" => config.show_last_seen,
        "Method" => config.show_method,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_size_is_positive_and_scales() {
        let size = panel_size_for(&Config::default());
        assert!(size.x > 100.0 && size.y > 40.0);
        let mut large = Config::default();
        large.scale = 1.5;
        let scaled = panel_size_for(&large);
        assert!(scaled.x > size.x);
        assert!(scaled.y > size.y);
    }

    #[test]
    fn preview_paints_the_name_health_and_detail_rows() {
        let ctx = Context::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1280.0, 720.0),
                )),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("target_info_test"),
                ));
                draw_preview(ctx, &painter, egui::pos2(40.0, 40.0));
            },
        );
        let texts: Vec<String> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        for expected in [
            "Alex",
            "RemotePlayer",
            "14.0 / 20  ·  70%",
            "8.4 m",
            "Armor 12  ·  3 pieces",
            "Source: Crosshair",
            "Visible to you",
        ] {
            assert!(
                texts.iter().any(|text| text == expected),
                "missing {expected:?} in {texts:?}"
            );
        }
    }

    #[test]
    fn larger_text_grows_the_panel_instead_of_overlapping_rows() {
        let mut config = Config::default();
        let normal = panel_size_for(&config);
        config.text_size = 17.0;
        let large = panel_size_for(&config);
        assert!(large.y > normal.y + 8.0, "{} vs {}", large.y, normal.y);
    }

    #[test]
    fn every_display_toggle_maps_to_a_config_field() {
        let config = Config::default();
        for name in [
            "Name",
            "Head",
            "Health Bar",
            "Health",
            "Percentage",
            "Absorption",
            "Distance",
            "Armor",
            "Held Item",
            "Effects",
            "Damage",
            "Entity Type",
            "Visibility",
            "Last Seen",
            "Method",
        ] {
            assert!(toggle(&config, name), "{name} is not wired");
        }
    }

    fn physical_view_for(w: f32, h: f32) -> View {
        let tan_y = 0.34_f64;
        View {
            cam: V3 {
                x: 0.0,
                y: 65.9,
                z: 0.0,
            },
            fwd: V3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            right: V3 {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
            up: V3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            tan_x: tan_y * (w / h) as f64,
            tan_y,
            w,
            h,
        }
    }

    fn physical_view() -> View {
        physical_view_for(1280.0, 720.0)
    }

    fn physical_sample(yaw: f32) -> TargetInfo {
        use crate::mapping::entity::player::GearStack;
        TargetInfo {
            id: 7,
            name: "Alex".into(),
            entity_type: "Player".into(),
            distance: 8.4,
            health: 14.0,
            max_health: 20.0,
            absorption: 4.0,
            armor: 12,
            armor_pieces: 3,
            main_hand: Some(GearStack {
                name: "diamond_sword".into(),
                damage: 12,
                max_damage: 1561,
                count: 1,
            }),
            off_hand: None,
            effects: vec![("speed".into(), 1, 1200)],
            feet: [0.0, 64.0, 4.0],
            yaw,
            height: 1.8,
            visible: true,
            invisible: false,
            is_player: true,
            skin_url: None,
            method: "Crosshair",
            first_seen: Instant::now(),
            last_seen: Instant::now(),
            last_damage: None,
            last_damage_at: None,
        }
    }

     
    fn physical_output(ctx: &Context, yaw: f32, w: f32, h: f32) -> egui::FullOutput {
        let config = Config {
            enabled: true,
            physical: true,
            ..Config::default()
        };
        let target = physical_sample(yaw);
        let view = physical_view_for(w, h);
        let motion = MotionValues {
            alpha: 1.0,
            health: 14.0 / 20.0,
            absorption: 4.0 / 20.0,
            appear: 1.0,
        };
        let anchor = AnchorSample {
            id: target.id,
            feet: V3 {
                x: target.feet[0],
                y: target.feet[1],
                z: target.feet[2],
            },
            yaw,
            height: target.height,
        };
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(w, h))),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("target_info_physical"),
                ));
                draw_physical_at(
                    ctx,
                    &painter,
                    &view,
                    &target,
                    &config,
                    &motion,
                    anchor,
                    Instant::now(),
                );
            },
        )
    }

     
    fn physical_frame(yaw: f32) -> Vec<egui::epaint::ClippedShape> {
        let ctx = Context::default();
        physical_output(&ctx, yaw, 1280.0, 720.0).shapes
    }

    fn frame_texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn physical_draws_health_items_and_side_information() {
        let shapes = physical_frame(0.0);
        let texts = frame_texts(&shapes);
        for expected in [
            "Alex",
            "Player",
            "Armor 12  ·  3 pieces",
            "8.4 m",
            "Speed II  1:00",
        ] {
            assert!(
                texts.iter().any(|text| text.contains(expected)),
                "missing {expected:?} in {texts:?}"
            );
        }
        assert!(
            texts.iter().any(|text| text.contains("Diamond Sword")),
            "the held item is not named: {texts:?}"
        );
    }

    #[test]
    fn physical_turns_with_the_target() {
        let config = Config::default();
        let health_x = |yaw: f32| {
            physical_frame(yaw)
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect) if rect.fill == color(config.health_color, 1.0) => {
                        Some(rect.rect.center().x)
                    }
                    _ => None,
                })
                .expect("the health bar is drawn")
        };
        let facing = health_x(0.0);
        let turned = health_x(180.0);
        assert!(
            facing < 640.0,
            "facing south the health bar sits on the target's left, got {facing}"
        );
        assert!(
            turned > 640.0,
            "turning around must carry the bar to the other side, got {turned}"
        );
    }

    #[test]
    fn physical_slides_its_anchor_between_ticks_and_turns_the_short_way() {
        let now = Instant::now();
        clear_anchor();
        let target = physical_sample(0.0);
        let snapshot = anchor_for(&target, now);
        assert_eq!(snapshot.yaw, 0.0, "no anchor yet: the snapshot is used");

        publish_anchor(
            7,
            V3 {
                x: 0.0,
                y: 64.0,
                z: 4.0,
            },
            170.0,
            1.8,
            now,
        );
        let later = now + Duration::from_millis(25);
        publish_anchor(
            7,
            V3 {
                x: 1.0,
                y: 64.0,
                z: 4.0,
            },
            -170.0,
            1.8,
            later,
        );
        let midway = anchor_for(&target, later + Duration::from_millis(25));
        assert!(
            (midway.feet.x - 0.5).abs() < 0.01,
            "the anchor slides, got {}",
            midway.feet.x
        );
        assert!(
            midway.yaw.abs() > 179.0,
            "yaw must wrap the short way through 180°, got {}",
            midway.yaw
        );

         
        let stale = anchor_for(&target, later + Duration::from_secs(2));
        assert_eq!(stale.feet.x, target.feet[0]);
        let mut other = target.clone();
        other.id = 99;
        other.feet = [9.0, 64.0, 9.0];
        assert_eq!(anchor_for(&other, later).feet.x, 9.0);
        clear_anchor();
    }

     
     
    #[test]
    fn the_physical_preview_sheet_renders() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        const W: usize = 640;
        const H: usize = 520;
        let mut sheet =
            image::RgbaImage::from_pixel(W as u32 * 2, H as u32, image::Rgba([16, 18, 24, 255]));
        for (index, yaw) in [0.0_f32, 180.0].iter().enumerate() {
            let ctx = Context::default();
            let mut textures = HashMap::new();
            let output = physical_output(&ctx, *yaw, W as f32, H as f32);
            gather_textures(&mut textures, &output.textures_delta);
            let primitives = ctx.tessellate(output.shapes, 1.0);
            let cell = rasterize(primitives, &textures, W, H, [0.06, 0.07, 0.09, 1.0]);
            let lit = cell
                .pixels()
                .filter(|pixel| pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32 > 120)
                .count();
            assert!(lit > 800, "yaw {yaw} rendered almost nothing ({lit})");
            image::imageops::replace(&mut sheet, &cell, (index * W) as i64, 0);
        }
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/target-info-physical.png");
        sheet.save(&path).expect("the physical preview png saves");
    }

    #[test]
    fn durations_read_like_a_clock() {
        assert_eq!(duration_label(20 * 60), "1:00");
        assert_eq!(duration_label(20 * 5), "0:05");
        assert_eq!(duration_label(i32::MAX), "∞");
        assert_eq!(short_duration(Duration::from_millis(240)), "0.2s ago");
        assert_eq!(short_duration(Duration::from_secs(90)), "2m ago");
    }

    #[test]
    fn item_names_are_readable() {
        assert_eq!(pretty_item("diamond_sword"), "Diamond Sword");
        assert_eq!(pretty_effect("fire_resistance"), "Fire Resistance");
        assert_eq!(roman(2), "II");
    }
}
