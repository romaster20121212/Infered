 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::esp::{View, V3};
use crate::graphic::fonts;
use crate::graphic::lyrics::{self, LyricLine, LyricsState};
use crate::graphic::now_playing;
use crate::module::{ModuleId, ModuleSetting};
use egui::{pos2, vec2, Align2, Color32, Context, FontId, Painter, Pos2, Rect};
use std::f32::consts::PI;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

 
const MAX_DISTANCE: f64 = 64.0;
 
const FADE_START: f64 = 0.6;
 
const GLYPH_HEIGHT: f64 = 0.34;
 
const GLYPH_MAX_WIDTH: f64 = 8.0;
 
const FADE_IN_SECONDS: f32 = 0.22;
 
const FADE_OUT_FRACTION: f32 = 0.30;
 
const FADE_OUT_MIN: f32 = 0.25;
const FADE_OUT_MAX: f32 = 0.90;

 
#[derive(Debug, Clone, Copy)]
pub(crate) struct WorldConfig {
    pub(crate) enabled: bool,
    pub(crate) sky: bool,
    also_hud: bool,
    distance: f64,
    height: f64,
    size: f32,
    glow: f32,
    highlight: Color32,
    text: Color32,
    word_highlight: bool,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sky: false,
            also_hud: false,
            distance: 7.0,
            height: 0.4,
            size: 1.0,
            glow: 0.8,
            highlight: Color32::WHITE,
            text: Color32::WHITE,
            word_highlight: true,
        }
    }
}

impl WorldConfig {
     
     
     
    pub(crate) fn read() -> Self {
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
                .and_then(ModuleSetting::get_toggle_value)
                .unwrap_or(default)
        };
        let slider = |name: &str, default: f32| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_slider_value)
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
        let sky = data.enabled && toggle("Sky Lyrics", false);
        Self {
            enabled: data.enabled && toggle("World Space", false) && !sky,
            sky,
            also_hud: toggle("Also Show HUD", false),
            distance: slider("World Distance", 7.0).clamp(2.0, 20.0) as f64,
            height: slider("World Height", 0.4).clamp(-1.0, 3.0) as f64,
            size: slider("World Size", 1.0).clamp(0.5, 2.5),
            glow: slider("Glow Strength", 0.8).clamp(0.0, 1.0),
            highlight: color("Highlight Color", Color32::WHITE),
            text: color("Text Color", Color32::WHITE),
            word_highlight: toggle("Word Highlight", true),
        }
    }
}

 
 
 
pub(crate) fn replaces_hud() -> bool {
    let config = WorldConfig::read();
    if config.also_hud
        || !(config.enabled || (config.sky && crate::graphic::atmosphere::sky_available()))
    {
        return false;
    }
    let Some(playback) = now_playing::current() else {
        return false;
    };
    matches!(
        lyrics::state_for(&playback.title, &playback.artist),
        LyricsState::Ready(_)
    )
}

 
#[derive(Debug, Clone, Copy)]
struct Anchor {
    line: usize,
    pos: V3,
}

#[derive(Default)]
struct State {
    track: Option<(String, String)>,
    anchor: Option<Anchor>,
}

fn state() -> MutexGuard<'static, State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE
        .get_or_init(|| Mutex::new(State::default()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
 
pub(crate) fn reset() {
    *state() = State::default();
}

pub(crate) const SKY_WIDTH: usize = 1024;
pub(crate) const SKY_HEIGHT: usize = 128;

#[derive(Clone)]
pub(crate) struct SkyFrame {
    pub pixels: Arc<[u8]>,
    glyph_edges: Arc<[f32]>,
    pub revision: u64,
    pub alpha: f32,
    pub progress: f32,
    pub text: [f32; 3],
    pub highlight: [f32; 3],
    pub word_highlight: bool,
}

#[derive(Default)]
struct SkyState {
    text: String,
    line_key: Option<(String, String, Duration)>,
    revision: u64,
    frame: Option<SkyFrame>,
}

fn sky_state() -> MutexGuard<'static, SkyState> {
    static SKY: OnceLock<Mutex<SkyState>> = OnceLock::new();
    SKY.get_or_init(|| Mutex::new(SkyState::default()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) fn sky_frame() -> Option<SkyFrame> {
    sky_state().frame.clone()
}

fn sky_progress(line: &LyricLine, at: Duration, glyph_edges: &[f32]) -> f32 {
    let mut offset = 0;
    let mut progress = 0.0;
    for (index, word) in line.words.iter().enumerate() {
        if index > 0 {
            offset += 1;
        }
        let end = offset + word.text.chars().count();
        let (Some(&start_x), Some(&end_x)) = (glyph_edges.get(offset), glyph_edges.get(end)) else {
            break;
        };
        if at < word.start {
            break;
        }
        progress = start_x + (end_x - start_x) * lyrics::word_progress(word, at);
        offset = end;
    }
    progress.clamp(0.0, 1.0)
}

fn sky_bitmap(ctx: &Context, text: &str) -> (Vec<u8>, Vec<f32>) {
    let text: String = text.chars().map(|ch| if ch.is_control() { ' ' } else { ch }).collect();
    let (galley, atlas) = ctx.fonts(|fonts| {
        let galley = fonts.layout_no_wrap(text, fonts::lyrics_font(44.0), Color32::WHITE);
        (galley, fonts.image())
    });
    let mut pixels = vec![0_u8; SKY_WIDTH * SKY_HEIGHT];
    let width = galley.size().x;
    if !width.is_finite() || width <= 0.0 {
        return (pixels, vec![0.0]);
    }
    let mut glyph_edges: Vec<f32> = galley.rows.iter()
        .flat_map(|row| row.glyphs.iter().map(|glyph| (glyph.pos.x / width).clamp(0.0, 1.0)))
        .collect();
    glyph_edges.push(1.0);
    let scale_x = (SKY_WIDTH as f32 * 0.90) / width;
    let left = SKY_WIDTH as f32 * 0.05;
    let top = (SKY_HEIGHT as f32 - galley.size().y) * 0.5;
    for row in &galley.rows {
        for glyph in &row.glyphs {
            let uv = glyph.uv_rect;
            if uv.is_nothing() || uv.size.x <= 0.0 || uv.size.y <= 0.0 {
                continue;
            }
            let x0 = left + (glyph.pos.x + uv.offset.x) * scale_x;
            let y0 = top + glyph.pos.y + uv.offset.y;
            let x1 = x0 + uv.size.x * scale_x;
            let y1 = y0 + uv.size.y;
            let min_x = (x0.floor() as i32).clamp(0, SKY_WIDTH as i32) as usize;
            let max_x = (x1.ceil() as i32).clamp(0, SKY_WIDTH as i32) as usize;
            let min_y = (y0.floor() as i32).clamp(0, SKY_HEIGHT as i32) as usize;
            let max_y = (y1.ceil() as i32).clamp(0, SKY_HEIGHT as i32) as usize;
            let source_width = usize::from(uv.max[0] - uv.min[0]);
            let source_height = usize::from(uv.max[1] - uv.min[1]);
            for y in min_y..max_y {
                let sy = (((y as f32 + 0.5 - y0) / uv.size.y)
                    .clamp(0.0, 0.9999) * source_height as f32) as usize;
                for x in min_x..max_x {
                    let sx = (((x as f32 + 0.5 - x0) / (uv.size.x * scale_x))
                        .clamp(0.0, 0.9999) * source_width as f32) as usize;
                    let index = (usize::from(uv.min[1]) + sy) * atlas.size[0]
                        + usize::from(uv.min[0]) + sx;
                    if let Some(coverage) = atlas.pixels.get(index) {
                        let alpha = (coverage.clamp(0.0, 1.0) * 255.0).round() as u8;
                        let pixel = &mut pixels[y * SKY_WIDTH + x];
                        *pixel = (*pixel).max(alpha);
                    }
                }
            }
        }
    }
    (pixels, glyph_edges)
}

pub(crate) fn prepare_sky(ctx: &Context) {
    let cfg = WorldConfig::read();
    if !cfg.sky
        || !crate::state::client().modules.is_active()
        || !crate::graphic::atmosphere::sky_available()
        || crate::state::minecraft().world().ok().flatten().is_none()
    {
        sky_state().frame = None;
        return;
    }
    lyrics::ensure_worker();
    now_playing::ensure_poller();
    let now = Instant::now();
    let active = now_playing::current().and_then(|playback| {
        let LyricsState::Ready(lines) = lyrics::state_for(&playback.title, &playback.artist) else {
            return None;
        };
        let at = playback.position_at(now);
        let line = lines.get(visible_line(&lines, at)?)?.clone();
        let key = (playback.title, playback.artist, line.start);
        Some((line, at, key))
    });
    let (text, alpha) = match &active {
        Some((line, at, _)) => (line_text(line), line_envelope(line, *at).alpha()),
        None => (String::new(), 0.0),
    };
    let key = active.as_ref().map(|(_, _, key)| key.clone());
    let mut state = sky_state();
    if state.text != text || state.line_key != key || state.frame.is_none() {
        state.revision = state.revision.wrapping_add(1);
        let (pixels, glyph_edges) = sky_bitmap(ctx, &text);
        state.text = text;
        state.line_key = key;
        state.frame = Some(SkyFrame {
            pixels: pixels.into(),
            glyph_edges: glyph_edges.into(),
            revision: state.revision,
            alpha,
            progress: 0.0,
            text: [0.0; 3],
            highlight: [0.0; 3],
            word_highlight: cfg.word_highlight,
        });
    }
    if let Some(frame) = state.frame.as_mut() {
        frame.alpha = alpha;
        frame.progress = active.as_ref().map_or(0.0, |(line, at, _)| {
            sky_progress(line, *at, &frame.glyph_edges)
        });
        frame.text = [cfg.text.r() as f32 / 255.0, cfg.text.g() as f32 / 255.0, cfg.text.b() as f32 / 255.0];
        frame.highlight = [cfg.highlight.r() as f32 / 255.0, cfg.highlight.g() as f32 / 255.0, cfg.highlight.b() as f32 / 255.0];
        frame.word_highlight = cfg.word_highlight;
    }
}

static CLOCK: LazyLock<Instant> = LazyLock::new(Instant::now);

fn animation_time(now: Instant) -> f32 {
    now.duration_since(*CLOCK).as_secs_f32()
}

 
 
pub(crate) fn draw(ctx: &Context, painter: &Painter, view: &View, cfg: &WorldConfig) {
    if !cfg.enabled {
        reset();
        return;
    }
    lyrics::ensure_worker();
    now_playing::ensure_poller();

    let now = Instant::now();
    let Some(playback) = now_playing::current() else {
        reset();
        return;
    };
    let key = (playback.title.clone(), playback.artist.clone());

     
     
    {
        let mut state = state();
        if state.track.as_ref() != Some(&key) {
            state.track = Some(key.clone());
            state.anchor = None;
        }
    }

    let LyricsState::Ready(lines) = lyrics::state_for(&key.0, &key.1) else {
        return;
    };
    let at = playback.position_at(now);

     
     
    let Some(index) = visible_line(&lines, at) else {
        let mut state = state();
        state.anchor = None;
        return;
    };
    let Some(line) = lines.get(index) else {
        return;
    };

     
    ctx.request_repaint();

    let envelope = line_envelope(line, at);
    let pos = {
        let mut state = state();
        match state.anchor {
            Some(anchor) if anchor.line == index => anchor.pos,
            _ => {
                 
                 
                let pos = anchor_point(view, cfg.distance, cfg.height);
                state.anchor = Some(Anchor { line: index, pos });
                pos
            }
        }
    };

    if envelope.alpha() <= 0.002 {
        return;
    }
    draw_line(
        ctx,
        painter,
        view,
        pos,
        line,
        at,
        envelope,
        animation_time(now),
        cfg,
    );
}

 
 
 
fn visible_line(lines: &[LyricLine], at: Duration) -> Option<usize> {
    let index = lines
        .partition_point(|line| line.start <= at)
        .checked_sub(1)?;
    let line = lines.get(index)?;
    (line.end > line.start && at < line.end).then_some(index)
}

 
 
#[derive(Debug, Clone, Copy, PartialEq)]
struct Envelope {
    enter: f32,
    exit: f32,
}

impl Envelope {
    fn alpha(self) -> f32 {
        self.enter.min(self.exit)
    }
}

 
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

 
 
fn line_envelope(line: &LyricLine, at: Duration) -> Envelope {
    let span = line.end.saturating_sub(line.start).as_secs_f32();
    if span <= 0.0 || at < line.start || at >= line.end {
        return Envelope {
            enter: 0.0,
            exit: 0.0,
        };
    }
    let t = at.saturating_sub(line.start).as_secs_f32();
    let fade_in = FADE_IN_SECONDS.min(span * 0.35).max(0.01);
    let fade_out = (span * FADE_OUT_FRACTION)
        .clamp(FADE_OUT_MIN, FADE_OUT_MAX)
        .min(span * 0.65)
        .max(0.01);
    Envelope {
        enter: smoothstep(t / fade_in),
        exit: smoothstep((span - t) / fade_out),
    }
}

 
 
fn anchor_point(view: &View, distance: f64, height: f64) -> V3 {
    let (fx, fz) = horizontal(view.fwd, (0.0, 1.0));
    V3 {
        x: view.cam.x + fx * distance,
        y: view.cam.y + height,
        z: view.cam.z + fz * distance,
    }
}

 
 
fn horizontal(v: V3, fallback: (f64, f64)) -> (f64, f64) {
    let length = (v.x * v.x + v.z * v.z).sqrt();
    if length > 1e-6 {
        (v.x / length, v.z / length)
    } else {
        fallback
    }
}

 
 
fn distance_fade(depth: f64) -> f32 {
    if depth >= MAX_DISTANCE {
        0.0
    } else if depth <= MAX_DISTANCE * FADE_START {
        1.0
    } else {
        let span = MAX_DISTANCE * (1.0 - FADE_START);
        ((MAX_DISTANCE - depth) / span).clamp(0.0, 1.0) as f32
    }
}

 
fn pixels_per_unit(depth: f64, tan_y: f64, height: f32) -> f64 {
    if depth <= 0.0 || tan_y <= 0.0 || height <= 0.0 {
        return 0.0;
    }
    (height as f64 * 0.5) / (tan_y * depth)
}

fn line_text(line: &LyricLine) -> String {
    if line.words.is_empty() {
        "♪".to_owned()
    } else {
        line.words
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn text_width(ctx: &Context, text: &str, font: &FontId) -> f32 {
    ctx.fonts(|fonts| {
        fonts
            .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x
    })
}

fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    let alpha = (color.a() as f32 * alpha.clamp(0.0, 1.0)).round() as u8;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

 
 
#[allow(clippy::too_many_arguments)]
fn draw_line(
    ctx: &Context,
    painter: &Painter,
    view: &View,
    anchor: V3,
    line: &LyricLine,
    at: Duration,
    envelope: Envelope,
    time: f32,
    cfg: &WorldConfig,
) {
     
     
     
    let phase = time * 0.8 + line.start.as_secs_f32();
    let pos = V3 {
        x: anchor.x + (phase * 0.6).sin() as f64 * 0.03,
        y: anchor.y + (phase * 0.9).sin() as f64 * 0.05,
        z: anchor.z,
    };
    let Some(center) = view.project(pos) else {
        return;
    };
    let depth = pos.sub(view.cam).dot(view.fwd);
    let units = pixels_per_unit(depth, view.tan_y, view.h);
    if units <= 0.0 {
        return;
    }
    let alpha = envelope.alpha() * distance_fade(depth);
    if alpha <= 0.01 {
        return;
    }

    let s = cfg.size;
    let max_px = (GLYPH_MAX_WIDTH * s as f64 * units) as f32;
     
    let scale = 0.90 + 0.10 * envelope.enter;
    let mut font = fonts::lyrics_font(
        ((GLYPH_HEIGHT * s as f64 * units) as f32 * scale)
            .clamp(4.0, 110.0)
            .max(4.0),
    );
    let text = line_text(line);
    let width = text_width(ctx, &text, &font);
    if width > max_px && width > 0.0 {
        font = fonts::lyrics_font((font.size * max_px / width).max(4.0));
    }

     
     
    let lift = (1.0 - envelope.exit).powf(1.4) * font.size * 0.45;
    let center = center - vec2(0.0, lift);

    if cfg.word_highlight && !line.words.is_empty() {
        draw_words(ctx, painter, center, line, at, &font, cfg, alpha);
    } else {
        draw_centered_text(painter, center, &text, &font, cfg, alpha);
    }
}

 
fn draw_centered_text(
    painter: &Painter,
    center: Pos2,
    text: &str,
    font: &FontId,
    cfg: &WorldConfig,
    alpha: f32,
) {
    painter.text(
        center + vec2(1.2, 1.6),
        Align2::CENTER_CENTER,
        text,
        font.clone(),
        with_alpha(Color32::BLACK, 0.40 * alpha),
    );
    if cfg.glow > 0.0 {
        let halo = with_alpha(cfg.highlight, 0.10 * cfg.glow * alpha);
        for (dx, dy) in [(-1.4, 0.0), (1.4, 0.0), (0.0, -1.4), (0.0, 1.4)] {
            painter.text(
                center + vec2(dx, dy),
                Align2::CENTER_CENTER,
                text,
                font.clone(),
                halo,
            );
        }
    }
    painter.text(
        center,
        Align2::CENTER_CENTER,
        text,
        font.clone(),
        with_alpha(cfg.text, alpha),
    );
}

 
 
 
#[allow(clippy::too_many_arguments)]
fn draw_words(
    ctx: &Context,
    painter: &Painter,
    center: Pos2,
    line: &LyricLine,
    at: Duration,
    font: &FontId,
    cfg: &WorldConfig,
    alpha: f32,
) {
    let gap = text_width(ctx, " ", font);
    let active = lyrics::active_word_index(line, at);
    let growth = 0.12 + 0.14 * cfg.glow;
    let sizes: Vec<f32> = line
        .words
        .iter()
        .enumerate()
        .map(|(index, word)| {
            if active == Some(index) {
                let bell = (PI * lyrics::word_progress(word, at)).sin();
                font.size * (1.0 + growth * bell)
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
    let mut x = center.x - total / 2.0;

    for (index, word) in line.words.iter().enumerate() {
        let width = widths[index];
        let size = sizes[index];
        let word_font = FontId::new(size, font.family.clone());
        let progress = lyrics::word_progress(word, at);
        let lift = if active == Some(index) {
            -(PI * progress).sin() * font.size * 0.16
        } else {
            0.0
        };
        let y = center.y + lift;

        painter.text(
            pos2(x + 1.2, y + 1.6),
            Align2::LEFT_CENTER,
            &word.text,
            word_font.clone(),
            with_alpha(Color32::BLACK, 0.40 * alpha),
        );

        if active == Some(index) && cfg.glow > 0.0 {
            let halo = with_alpha(cfg.highlight, 0.13 * cfg.glow * alpha);
            for (dx, dy) in [(-1.3, 0.0), (1.3, 0.0), (0.0, -1.3), (0.0, 1.3)] {
                painter.text(
                    pos2(x + dx, y + dy),
                    Align2::LEFT_CENTER,
                    &word.text,
                    word_font.clone(),
                    halo,
                );
            }
        }

        painter.text(
            pos2(x, y),
            Align2::LEFT_CENTER,
            &word.text,
            word_font.clone(),
            with_alpha(cfg.text, alpha),
        );

        let fill = if active == Some(index) {
            progress
        } else if progress >= 1.0 {
            1.0
        } else {
            0.0
        };
        if fill > 0.0 {
            let clip = Rect::from_min_max(
                pos2(x - 1.0, y - size),
                pos2(x + width * fill + 1.0, y + size),
            );
            painter.with_clip_rect(clip).text(
                pos2(x, y),
                Align2::LEFT_CENTER,
                &word.text,
                word_font.clone(),
                with_alpha(cfg.highlight, alpha),
            );
        }

        x += width + gap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v3(x: f64, y: f64, z: f64) -> V3 {
        V3 { x, y, z }
    }

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    fn line(start: u64, end: u64) -> LyricLine {
        LyricLine {
            start: ms(start),
            end: ms(end),
            words: vec![lyrics::LyricWord {
                text: "hello".into(),
                start: ms(start),
                end: ms(end),
            }],
        }
    }

    #[test]
    fn the_anchor_sits_centered_ahead_and_lifted() {
        let view = View {
            cam: v3(10.0, 64.0, -4.0),
            fwd: v3(0.0, 0.0, 1.0),
            right: v3(-1.0, 0.0, 0.0),
            up: v3(0.0, 1.0, 0.0),
            tan_x: 1.24,
            tan_y: 0.70,
            w: 1280.0,
            h: 720.0,
        };
        let anchor = anchor_point(&view, 7.0, 0.4);
        assert!((anchor.x - 10.0).abs() < 1e-9);
        assert!((anchor.y - 64.4).abs() < 1e-9);
        assert!((anchor.z - 3.0).abs() < 1e-9);
    }

    #[test]
    fn the_anchor_follows_horizontal_facing_not_pitch() {
        let view = View {
            cam: v3(0.0, 64.0, 0.0),
            fwd: v3(0.0, 1.0, 0.0),
            right: v3(-1.0, 0.0, 0.0),
            up: v3(0.0, 1.0, 0.0),
            tan_x: 1.24,
            tan_y: 0.70,
            w: 1280.0,
            h: 720.0,
        };
        let anchor = anchor_point(&view, 7.0, 0.0);
        assert!((anchor.y - 64.0).abs() < 1e-9);
        assert!((anchor.z - 7.0).abs() < 1e-9);
    }

    #[test]
    fn projected_size_halves_when_distance_doubles() {
        let near = pixels_per_unit(7.0, 0.7, 720.0);
        let far = pixels_per_unit(14.0, 0.7, 720.0);
        assert!((near / far - 2.0).abs() < 1e-9);
    }

    #[test]
    fn distance_fade_is_full_near_and_zero_far() {
        assert_eq!(distance_fade(4.0), 1.0);
        assert_eq!(distance_fade(MAX_DISTANCE), 0.0);
        let mid = distance_fade(MAX_DISTANCE * 0.8);
        assert!(mid > 0.0 && mid < 1.0);
    }

    #[test]
    fn a_line_is_visible_only_inside_its_own_window() {
        let lines = [line(0, 2_000), line(2_000, 4_000)];
        assert_eq!(visible_line(&lines, ms(0)), Some(0));
        assert_eq!(visible_line(&lines, ms(1_999)), Some(0));
         
         
        assert_eq!(visible_line(&lines, ms(2_000)), Some(1));
        assert_eq!(visible_line(&lines, ms(3_999)), Some(1));
         
        assert_eq!(visible_line(&lines, ms(4_000)), None);
    }

    #[test]
    fn a_gap_between_lines_renders_nothing() {
        let lines = [line(0, 1_000), line(4_000, 6_000)];
        assert_eq!(visible_line(&lines, ms(2_500)), None);
        assert_eq!(visible_line(&lines, ms(5_000)), Some(1));
    }

    #[test]
    fn a_line_fades_in_and_is_gone_before_it_ends() {
        let line = line(0, 4_000);
        assert_eq!(line_envelope(&line, ms(0)).alpha(), 0.0);
        assert!(
            line_envelope(&line, ms(120)).alpha() > 0.5,
            "the line should be well into its fade-in"
        );
        assert_eq!(
            line_envelope(&line, ms(2_000)).alpha(),
            1.0,
            "the middle of the line is fully visible"
        );
        assert!(
            line_envelope(&line, ms(3_990)).alpha() < 0.05,
            "the fade-out has essentially finished before the line ends"
        );
        assert_eq!(line_envelope(&line, ms(4_000)).alpha(), 0.0);
    }

    #[test]
    fn a_very_short_line_still_fades_both_ends() {
        let line = line(0, 300);
        assert_eq!(line_envelope(&line, ms(0)).alpha(), 0.0);
        assert_eq!(line_envelope(&line, ms(300)).alpha(), 0.0);
        assert!(line_envelope(&line, ms(150)).alpha() > 0.5);
    }

    #[test]
    fn instrumental_lines_show_a_note() {
        let line = LyricLine {
            start: Duration::ZERO,
            end: Duration::from_secs(3),
            words: Vec::new(),
        };
        assert_eq!(line_text(&line), "♪");
    }

    #[test]
    fn sky_bitmap_fills_the_band_and_handles_empty_text() {
        let ctx = Context::default();
        fonts::install(&ctx);
        let mut pixels = Vec::new();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            assert!(sky_bitmap(ctx, "").0.iter().all(|pixel| *pixel == 0));
            pixels = sky_bitmap(ctx, "hello sky").0;
        });
        assert_eq!(pixels.len(), SKY_WIDTH * SKY_HEIGHT);
        let ink: Vec<usize> = pixels.iter().enumerate()
            .filter_map(|(index, alpha)| (*alpha > 0).then_some(index % SKY_WIDTH))
            .collect();
        assert!(!ink.is_empty());
        let min = *ink.iter().min().unwrap();
        let max = *ink.iter().max().unwrap();
        assert!(max - min > SKY_WIDTH * 3 / 4);
    }

    #[test]
    fn sky_progress_tracks_words_and_clamps_at_the_end() {
        let line = LyricLine { start: ms(0), end: ms(2000), words: vec![
            lyrics::LyricWord { text: "one".into(), start: ms(0), end: ms(1000) },
            lyrics::LyricWord { text: "two".into(), start: ms(1000), end: ms(2000) },
        ]};
        let edges = [0.0, 0.1, 0.2, 0.3, 0.4, 0.6, 0.8, 1.0];
        assert_eq!(sky_progress(&line, ms(0), &edges), 0.0);
        assert!((sky_progress(&line, ms(1000), &edges) - 0.4).abs() < 1e-6);
        assert!((sky_progress(&line, ms(1500), &edges) - 0.7).abs() < 1e-6);
        assert_eq!(sky_progress(&line, ms(2000), &edges), 1.0);
    }

    #[test]
    fn the_world_pass_draws_words_without_a_panel() {
        let ctx = Context::default();
        fonts::install(&ctx);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 720.0));
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("lyrics_world_test"),
                ));
                let view = View {
                    cam: v3(0.0, 64.0, 0.0),
                    fwd: v3(0.0, 0.0, 1.0),
                    right: v3(-1.0, 0.0, 0.0),
                    up: v3(0.0, 1.0, 0.0),
                    tan_x: 1.24,
                    tan_y: 0.70,
                    w: 1280.0,
                    h: 720.0,
                };
                let cfg = WorldConfig {
                    enabled: true,
                    ..WorldConfig::default()
                };
                let line = line(0, 3_000);
                draw_line(
                    ctx,
                    &painter,
                    &view,
                    anchor_point(&view, 7.0, 0.4),
                    &line,
                    ms(800),
                    line_envelope(&line, ms(800)),
                    1.0,
                    &cfg,
                );
            },
        );
        let texts = output
            .shapes
            .iter()
            .filter(|shape| matches!(shape.shape, egui::Shape::Text(_)))
            .count();
        let rects = output
            .shapes
            .iter()
            .filter(|shape| matches!(shape.shape, egui::Shape::Rect(_)))
            .count();
        assert!(texts > 0, "the line should emit text");
        assert_eq!(rects, 0, "world lyrics are words only, never a panel");
    }
}
