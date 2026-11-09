 
 
 
 
 
 
 
 
 

use crate::graphic::hud::{self, dimmed, fit_text, text_width, HudWidgetId};
use crate::graphic::lyrics::{self, LyricLine, LyricWord, LyricsState};
use crate::graphic::now_playing;
use crate::module::{ModuleId, ModuleSetting};
use egui::{Align2, Color32, Context, FontId, Id, Painter, Pos2, Rect, Rounding, Vec2};
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Duration;

 
const PAD: f32 = 10.0;
 
const TITLE_ROW: f32 = 16.0;
 
const ROW: f32 = 26.0;
 
const FONT_IDLE: f32 = 15.0;
const FONT_FOCUS: f32 = 20.0;
const DEFAULT_HIGHLIGHT: Color32 = Color32::WHITE;
 
const SPARK_LIFE: f32 = 0.7;
 
const SPARKS_PER_WORD: u64 = 7;

 
#[derive(Clone, Copy)]
struct LyricsStyle {
    scale: f32,
    radius: f32,
    opacity: f32,
    width: f32,
    lines_above: usize,
    lines_below: usize,
    slide_speed: f32,
    glow: f32,
    highlight: Color32,
    text: Color32,
    show_background: bool,
    word_highlight: bool,
    show_title: bool,
}

impl Default for LyricsStyle {
    fn default() -> Self {
        Self {
            scale: 1.0,
            radius: 8.0,
            opacity: 0.55,
            width: 360.0,
            lines_above: 2,
            lines_below: 2,
            slide_speed: 8.0,
            glow: 0.8,
            highlight: DEFAULT_HIGHLIGHT,
            text: Color32::WHITE,
            show_background: true,
            word_highlight: true,
            show_title: true,
        }
    }
}

impl LyricsStyle {
     
     
    fn read() -> Self {
        let fallback = Self::default();
        let Some(handle) = crate::state::client().modules.get(ModuleId::Lyrics) else {
            return fallback;
        };
        let Ok(module) = handle.lock() else {
            return fallback;
        };
        let data = module.get_module_data();
        let toggle = |name: &str, default: bool| {
            data.get_setting(name)
                .and_then(|setting| setting.get_toggle_value())
                .unwrap_or(default)
        };
        let slider = |name: &str, default: f32| {
            data.get_setting(name)
                .and_then(|setting| setting.get_slider_value())
                .filter(|value| value.is_finite())
                .unwrap_or(default)
        };
        let color = |name: &str, default: Color32| match data.get_setting(name) {
            Some(ModuleSetting::Color { value, .. }) => {
                let channel = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                Color32::from_rgb(channel(value[0]), channel(value[1]), channel(value[2]))
            }
            _ => default,
        };
        Self {
            scale: slider("Scale", 1.0).clamp(0.6, 2.0),
            radius: slider("Corner Radius", 8.0).clamp(0.0, 16.0),
            opacity: slider("Background Opacity", 0.55).clamp(0.0, 1.0),
            width: slider("Max Width", 360.0).clamp(160.0, 700.0),
            lines_above: slider("Lines Above", 2.0).round().clamp(0.0, 4.0) as usize,
            lines_below: slider("Lines Below", 2.0).round().clamp(0.0, 4.0) as usize,
            slide_speed: slider("Slide Speed", 8.0).clamp(1.0, 20.0),
            glow: slider("Glow Strength", 0.8).clamp(0.0, 1.0),
            highlight: color("Highlight Color", DEFAULT_HIGHLIGHT),
            text: color("Text Color", Color32::WHITE),
            show_background: toggle("Show Background", true),
            word_highlight: toggle("Word Highlight", true),
            show_title: toggle("Show Title", true),
        }
    }
}

 
 
fn card_size(style: &LyricsStyle) -> Vec2 {
    let s = style.scale;
    let title = if style.show_title { TITLE_ROW * s } else { 0.0 };
    let rows = (style.lines_above + 1 + style.lines_below) as f32;
    Vec2::new(style.width * s, PAD * 2.0 * s + title + rows * ROW * s)
}

 
pub(crate) fn default_size() -> Vec2 {
    card_size(&LyricsStyle::default())
}

 
pub(crate) struct LyricsCard {
    title: String,
    body: CardBody,
}

enum CardBody {
    Message(String),
    Lines {
        lines: Arc<[LyricLine]>,
        elapsed: Duration,
    },
}

 
pub(crate) fn preview_card() -> LyricsCard {
    LyricsCard {
        title: "Lyrics".into(),
        body: CardBody::Message("♪ Synced lyrics appear here ♪".into()),
    }
}

 
fn live_card() -> LyricsCard {
    let Some(playback) = now_playing::current() else {
        return LyricsCard {
            title: "Lyrics".into(),
            body: CardBody::Message("No song playing".into()),
        };
    };
    let body = match lyrics::state_for(&playback.title, &playback.artist) {
        LyricsState::Ready(lines) => CardBody::Lines {
            lines,
            elapsed: playback.position_at(std::time::Instant::now()),
        },
        LyricsState::Missing => CardBody::Message("No synced lyrics for this song".into()),
        LyricsState::Loading | LyricsState::Idle => CardBody::Message("Looking up lyrics…".into()),
    };
    LyricsCard {
        title: playback.title.clone(),
        body,
    }
}

 
pub(crate) fn draw_widget(ctx: &Context, painter: &Painter) {
    lyrics::ensure_worker();
    let origin = hud::widget_rendered_pos(ctx, HudWidgetId::Lyrics);
    let style = LyricsStyle::read();
    draw_card(ctx, painter, origin, &live_card(), &style);
}

 
pub(crate) fn draw_preview(ctx: &Context, painter: &Painter, origin: Pos2) {
    draw_card(ctx, painter, origin, &preview_card(), &LyricsStyle::read());
}

 
pub(crate) fn saved_size() -> Vec2 {
    card_size(&LyricsStyle::read())
}

 
fn draw_card(
    ctx: &Context,
    painter: &Painter,
    origin: Pos2,
    card: &LyricsCard,
    style: &LyricsStyle,
) {
    let s = style.scale;
    let rect = Rect::from_min_size(origin, card_size(style));
    if style.show_background {
        let alpha = (style.opacity * 255.0).round().clamp(0.0, 255.0) as u8;
        painter.rect_filled(
            rect,
            Rounding::same(style.radius * s),
            Color32::from_black_alpha(alpha),
        );
    }

    let mut top = rect.min.y + PAD * s;
    if style.show_title {
        let font = FontId::proportional(11.0 * s);
        let title = fit_text(ctx, &card.title, &font, rect.width() - PAD * 2.0 * s);
        painter.text(
            egui::pos2(rect.center().x, top),
            Align2::CENTER_TOP,
            title,
            font,
            dimmed(style.text, 0.6),
        );
        top += TITLE_ROW * s;
    }

    let body = Rect::from_min_max(
        egui::pos2(rect.min.x + PAD * s, top),
        egui::pos2(rect.max.x - PAD * s, rect.max.y - PAD * s),
    );
    match &card.body {
        CardBody::Message(text) => {
             
            ctx.request_repaint();
            let time = ctx.input(|input| input.time) as f32;
            let breathe = 0.5 + 0.5 * (time * 2.2).sin();
            let font = FontId::proportional(12.0 * s);
            let text = fit_text(ctx, text, &font, body.width());
            painter.text(
                body.center(),
                Align2::CENTER_CENTER,
                text,
                font,
                dimmed(style.text, 0.45 + 0.35 * breathe),
            );
        }
        CardBody::Lines { lines, elapsed } => {
            draw_lines(
                ctx,
                &painter.with_clip_rect(body),
                body,
                lines,
                *elapsed,
                style,
            );
            draw_progress(painter, rect, lines, *elapsed, style);
        }
    }
}

 
 
 
fn advance_scroll(ctx: &Context, target: f32, speed: f32) -> f32 {
    let id = Id::new("lyrics_card_scroll");
    let dt = ctx.input(|input| input.stable_dt).clamp(0.0, 0.1);
    let mut position = ctx
        .data_mut(|data| data.get_temp::<f32>(id))
        .unwrap_or(target);
    if (target - position).abs() > 6.0 {
        position = target;
    }
    position += (target - position) * (1.0 - (-speed * dt).exp());
    if (target - position).abs() > 0.001 {
        ctx.request_repaint();
    }
    ctx.data_mut(|data| data.insert_temp(id, position));
    position
}

 
 
fn draw_lines(
    ctx: &Context,
    painter: &Painter,
    area: Rect,
    lines: &[LyricLine],
    elapsed: Duration,
    style: &LyricsStyle,
) {
    let s = style.scale;
    let row = ROW * s;
    let active = lyrics::active_line_index(lines, elapsed);
     
     
    if active.is_some() {
        ctx.request_repaint();
    }
    let target = active.map_or(-1.0, |index| index as f32);
    let scroll = advance_scroll(ctx, target, style.slide_speed);
    let base = scroll.floor();
    let frac = scroll - base;
    let reach = style.lines_above.max(style.lines_below) as f32 + 1.0;
    let center_x = area.center().x;
    let center_y = area.center().y;

    for k in -(reach as i64) - 2..=(reach as i64) + 2 {
        let slot = k as f32 - frac;
        if slot.abs() > reach + 0.5 {
            continue;
        }
        let index = base as i64 + k;
        let Some(line) = usize::try_from(index).ok().and_then(|i| lines.get(i)) else {
            continue;
        };
         
        let focus = (1.0 - slot.abs()).clamp(0.0, 1.0);
        let fade = (1.0 - 0.22 * slot.abs()).clamp(0.25, 1.0);
        let is_active = active == usize::try_from(index).ok();
         
        let pop = if is_active {
            entry_pop(elapsed, line.start)
        } else {
            0.0
        };
        let size = FONT_IDLE + (FONT_FOCUS - FONT_IDLE) * focus + pop;
        let color = dimmed(mix(dimmed(style.text, 0.45), style.text, focus), fade);
        let y = center_y + slot * row;

        if line.words.is_empty() {
            let font = FontId::proportional(size * s);
            painter.text(
                egui::pos2(center_x, y),
                Align2::CENTER_CENTER,
                "♪",
                font,
                color,
            );
            continue;
        }

        let text = line
            .words
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let font = fitted_font(ctx, &text, size * s, area.width());
        if is_active && style.word_highlight {
            draw_words(
                ctx, painter, center_x, y, line, elapsed, &font, style, color,
            );
        } else {
            painter.text(
                egui::pos2(center_x, y),
                Align2::CENTER_CENTER,
                text,
                font,
                color,
            );
        }
    }
}

 
 
 
 
fn draw_words(
    ctx: &Context,
    painter: &Painter,
    center_x: f32,
    y: f32,
    line: &LyricLine,
    at: Duration,
    font: &FontId,
    style: &LyricsStyle,
    unsung: Color32,
) {
    let gap = text_width(ctx, " ", font);
    let active = lyrics::active_word_index(line, at);
    let s = style.scale;
     
     
    let growth = 0.2 + 0.2 * style.glow;
    let sizes: Vec<f32> = line
        .words
        .iter()
        .enumerate()
        .map(|(index, word)| {
            if active == Some(index) {
                let bell = (PI * lyrics::word_progress(word, at)).sin();
                font.size * (1.0 + growth * ease(bell))
            } else {
                font.size
            }
        })
        .collect();
     
     
    let widths: Vec<f32> = line
        .words
        .iter()
        .zip(&sizes)
        .map(|(word, size)| text_width(ctx, &word.text, &FontId::new(*size, font.family.clone())))
        .collect();
    let total = widths.iter().sum::<f32>() + gap * widths.len().saturating_sub(1) as f32;
    let mut x = center_x - total / 2.0;

    for (index, word) in line.words.iter().enumerate() {
        let width = widths[index];
        let word_font = FontId::new(sizes[index], font.family.clone());
        let progress = lyrics::word_progress(word, at);

        if active == Some(index) {
             
            let lift = -(PI * progress).sin() * 4.0 * s * (0.5 + style.glow);
            let rect = Rect::from_min_size(
                egui::pos2(x, y + lift - word_font.size * 0.8),
                Vec2::new(width, word_font.size * 1.6),
            );
            draw_sweeping_word(
                painter, rect, &word.text, &word_font, progress, style, unsung,
            );
        } else {
            let fill = if progress >= 1.0 {
                style.highlight
            } else {
                unsung
            };
            painter.text(
                egui::pos2(x, y),
                Align2::LEFT_CENTER,
                &word.text,
                word_font,
                fill,
            );
        }

        draw_sparks(
            painter,
            egui::pos2(x + width / 2.0, y),
            line,
            index,
            word,
            at,
            style,
        );
        x += width + gap;
    }
}

 
 
 
fn draw_sweeping_word(
    painter: &Painter,
    rect: Rect,
    text: &str,
    font: &FontId,
    progress: f32,
    style: &LyricsStyle,
    unsung: Color32,
) {
    let s = style.scale;
    let origin = rect.left_center();
    let eased = ease(progress);

    if style.glow > 0.0 {
        let halo = dimmed(style.highlight, 0.22 * style.glow);
        for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
            painter.text(
                origin + Vec2::new(dx * 1.5 * s, dy * 1.5 * s),
                Align2::LEFT_CENTER,
                text,
                font.clone(),
                halo,
            );
        }
    }

    painter.text(origin, Align2::LEFT_CENTER, text, font.clone(), unsung);
    if eased <= 0.0 {
        return;
    }

    let edge = origin.x + rect.width() * eased;
    let wipe = painter.with_clip_rect(Rect::from_min_max(
        egui::pos2(origin.x - 2.0 * s, rect.min.y),
        egui::pos2(edge, rect.max.y),
    ));
    wipe.text(
        origin,
        Align2::LEFT_CENTER,
        text,
        font.clone(),
        style.highlight,
    );

    if progress < 1.0 && style.glow > 0.0 {
         
        for layer in 0..3 {
            let spread = (3 - layer) as f32 * 2.0 * s;
            let strip = Rect::from_min_max(
                egui::pos2(edge - spread, rect.min.y),
                egui::pos2(edge + spread, rect.max.y),
            );
            painter.rect_filled(
                strip,
                Rounding::same(0.0),
                dimmed(style.highlight, 0.16 * style.glow),
            );
        }
    }
}

 
 
 
fn draw_sparks(
    painter: &Painter,
    origin: Pos2,
    line: &LyricLine,
    index: usize,
    word: &LyricWord,
    at: Duration,
    style: &LyricsStyle,
) {
    if at < word.end || style.glow <= 0.0 {
        return;
    }
    let age = (at - word.end).as_secs_f32();
    if age >= SPARK_LIFE {
        return;
    }
    let s = style.scale;
    let life = age / SPARK_LIFE;
    let fade = (1.0 - life).powi(2) * style.glow;
    let base = (line.start.as_millis() as u64) ^ ((index as u64 + 1) << 32);

    for spark in 0..SPARKS_PER_WORD {
        let seed = base.wrapping_add(spark.wrapping_mul(0x9E37_79B9));
         
        let angle = -PI / 2.0 + (unit(seed) - 0.5) * PI * 1.1;
        let speed = (24.0 + 26.0 * unit(seed ^ 0xA5A5)) * s;
        let distance = speed * age;
        let drop = 40.0 * s * age * age;
        let pos = origin + Vec2::new(angle.cos() * distance, angle.sin() * distance + drop);
        let radius = (1.6 + 1.4 * unit(seed ^ 0x5A5A)) * s * (1.0 - 0.5 * life);
        painter.circle_filled(pos, radius, dimmed(style.highlight, fade));
    }
}

 
 
fn draw_progress(
    painter: &Painter,
    rect: Rect,
    lines: &[LyricLine],
    elapsed: Duration,
    style: &LyricsStyle,
) {
    let Some(index) = lyrics::active_line_index(lines, elapsed) else {
        return;
    };
    let line = &lines[index];
    let span = line.end.saturating_sub(line.start).as_secs_f32();
    if span <= 0.0 {
        return;
    }
    let progress = (elapsed.saturating_sub(line.start).as_secs_f32() / span).clamp(0.0, 1.0);

    let s = style.scale;
    let thickness = 0.75 * s;
    let inset = style.radius * s;
    let left = rect.min.x + inset;
    let right = rect.max.x - inset;
    let y = rect.max.y - 1.5 * s;
    let track = Rect::from_min_max(
        egui::pos2(left, y - thickness),
        egui::pos2(right, y + thickness),
    );
    painter.rect_filled(track, Rounding::same(thickness), dimmed(style.text, 0.12));

    let head_x = left + (right - left) * progress;
    if head_x > left {
        let fill = Rect::from_min_max(
            egui::pos2(left, y - thickness),
            egui::pos2(head_x, y + thickness),
        );
        painter.rect_filled(
            fill,
            Rounding::same(thickness),
            dimmed(style.highlight, 0.9),
        );
        let head = Rect::from_min_max(
            egui::pos2(head_x - 4.0 * s, y - 1.5 * s),
            egui::pos2(head_x, y + 1.5 * s),
        );
        painter.rect_filled(
            head,
            Rounding::same(1.5 * s),
            dimmed(style.highlight, 0.35 * style.glow),
        );
    }
}

 
 
fn entry_pop(elapsed: Duration, line_start: Duration) -> f32 {
    let age = elapsed.saturating_sub(line_start).as_secs_f32();
    3.0 * (-5.0 * age).exp()
}

 
fn unit(seed: u64) -> f32 {
    let mut x = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

 
 
fn fitted_font(ctx: &Context, text: &str, size: f32, max_width: f32) -> FontId {
    let font = FontId::proportional(size);
    let width = text_width(ctx, text, &font);
    if width <= max_width || width <= 0.0 {
        font
    } else {
        FontId::proportional((size * max_width / width).max(size * 0.6))
    }
}

 
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
        lerp(a.a(), b.a()),
    )
}

 
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_noise_is_in_range_deterministic_and_varied() {
        let mut distinct = std::collections::HashSet::new();
        for seed in 0..256u64 {
            let value = unit(seed);
            assert!((0.0..1.0).contains(&value));
            assert_eq!(value, unit(seed));
            distinct.insert(value.to_bits());
        }
        assert!(distinct.len() > 200);
    }

    #[test]
    fn entry_pop_springs_up_then_settles() {
        let start = Duration::from_secs(10);
        let fresh = entry_pop(start + Duration::from_millis(50), start);
        let later = entry_pop(start + Duration::from_millis(500), start);
        assert!(fresh > later);
        assert!(entry_pop(start + Duration::from_secs(3), start) < 0.01);
         
        assert_eq!(entry_pop(start, start + Duration::from_secs(1)), 3.0);
    }
}
