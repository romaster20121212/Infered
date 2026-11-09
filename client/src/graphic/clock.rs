 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::theme;
use crate::module::{ModuleId, ModuleSetting};
use egui::{Align2, Color32, Context, FontId, Painter, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

 
pub(crate) const DIAL: f32 = 96.0;
 
const PILL_H: f32 = 16.0;
 
const DATE_H: f32 = 13.0;
 
const PILL_GAP: f32 = 2.0;
 
const FACE_INSET: f32 = 3.0;
 
const HOUR_TICK: f32 = 5.0;
const MINUTE_TICK: f32 = 2.4;
const QUARTER_TICK: f32 = 6.0;
 
const NUMERAL_RADIUS: f32 = 0.66;

 
 
const HOUR_HAND: (f32, f32, f32) = (0.58, 0.10, 2.7);
const MINUTE_HAND: (f32, f32, f32) = (0.88, 0.12, 1.9);
const SECOND_HAND: (f32, f32) = (0.90, 0.24);
 
const SECOND_DISC: (f32, f32) = (0.72, 2.0);

 
 
const PILL_TIME: f32 = 10.0;
const PILL_TAG: f32 = 7.5;
const NUMERAL_SIZE: f32 = 10.5;
const DIGITAL_TIME: f32 = 21.0;
const DIGITAL_SMALL: f32 = 12.0;
const DIGITAL_DATE: f32 = 8.5;
const DIGITAL_TAG: f32 = 7.5;
const DIGITAL_PAD: f32 = 8.0;

 
const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
 
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

 
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Face {
    Analog,
    Digital,
}

 
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Zone {
     
    Local,
     
    Utc,
}

 
 
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClockConfig {
    pub face: Face,
    pub hour24: bool,
    pub seconds: bool,
    pub date: bool,
    pub zone: Zone,
     
    pub smooth: bool,
    pub scale: f32,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            face: Face::Analog,
            hour24: true,
            seconds: true,
            date: false,
            zone: Zone::Local,
            smooth: true,
            scale: 1.0,
        }
    }
}

impl ClockConfig {
     
     
    pub(crate) fn read() -> Self {
        let mut config = Self::default();
        let Some(handle) =
            crate::state::try_client().and_then(|client| client.modules.get(ModuleId::Clock))
        else {
            return config;
        };
        let Ok(module) = handle.lock() else {
            return config;
        };
        let data = module.get_module_data();
        let toggle = |name: &str| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_toggle_value)
        };
        let slider = |name: &str| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_slider_value)
        };
        let choice = |name: &str| match data.get_setting(name) {
            Some(ModuleSetting::Choice { value, .. }) => Some(*value),
            _ => None,
        };

        if let Some(value) = choice("Style") {
            config.face = if value == 0 {
                Face::Analog
            } else {
                Face::Digital
            };
        }
        if let Some(value) = choice("Timezone") {
            config.zone = if value == 0 { Zone::Local } else { Zone::Utc };
        }
        config.hour24 = toggle("24 Hour").unwrap_or(config.hour24);
        config.seconds = toggle("Seconds").unwrap_or(config.seconds);
        config.date = toggle("Show Date").unwrap_or(config.date);
        config.smooth = toggle("Smooth Seconds").unwrap_or(config.smooth);
        config.scale = slider("Scale").unwrap_or(config.scale).clamp(0.6, 2.0);
        config
    }

     
    pub(crate) fn offset_minutes(&self) -> i32 {
        match self.zone {
            Zone::Utc => 0,
            Zone::Local => local_offset_minutes(),
        }
    }

    fn radius(&self) -> f32 {
        DIAL * self.scale * 0.5
    }
}

 
 
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClockTime {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
     
    pub fraction: f32,
     
    pub weekday: usize,
    pub day: u32,
     
    pub month: u32,
     
    pub offset_minutes: i32,
}

impl ClockTime {
     
    fn time_text(&self, hour24: bool, seconds: bool) -> String {
        let parts = self.time_parts(hour24, seconds);
        format!("{}{}{}", parts.main, parts.seconds, parts.suffix)
    }

     
     
     
    fn time_parts(&self, hour24: bool, seconds: bool) -> TimeParts {
        let (main, suffix) = if hour24 {
            (
                format!("{:02}:{:02}", self.hour, self.minute),
                String::new(),
            )
        } else {
            let hour = match self.hour % 12 {
                0 => 12,
                hour => hour,
            };
            let suffix = if self.hour < 12 { " AM" } else { " PM" };
            (format!("{hour}:{:02}", self.minute), suffix.to_string())
        };
        let seconds = if seconds {
            format!(":{:02}", self.second)
        } else {
            String::new()
        };
        TimeParts {
            main,
            seconds,
            suffix,
        }
    }

     
    fn date_text(&self) -> String {
        format!(
            "{} {} {}",
            WEEKDAYS[self.weekday.min(6)],
            self.day,
            MONTHS[(self.month as usize).clamp(1, 12) - 1]
        )
    }

     
     
    fn zone_text(&self) -> String {
        if self.offset_minutes == 0 {
            return "UTC".to_string();
        }
        let sign = if self.offset_minutes < 0 { '-' } else { '+' };
        let minutes = self.offset_minutes.unsigned_abs();
        let (hours, minutes) = (minutes / 60, minutes % 60);
        if minutes == 0 {
            format!("UTC{sign}{hours}")
        } else {
            format!("UTC{sign}{hours}:{minutes:02}")
        }
    }
}

 
struct TimeParts {
    main: String,
    seconds: String,
    suffix: String,
}

 
 
pub(crate) fn reading(unix_secs: i64, fraction: f32, offset_minutes: i32) -> ClockTime {
    let local = unix_secs + offset_minutes as i64 * 60;
    let days = local.div_euclid(86_400);
    let of_day = local.rem_euclid(86_400);
    let (_, month, day) = civil_from_days(days);
    ClockTime {
        hour: (of_day / 3_600) as u32,
        minute: (of_day % 3_600 / 60) as u32,
        second: (of_day % 60) as u32,
        fraction: fraction.clamp(0.0, 1.0),
         
        weekday: (days + 3).rem_euclid(7) as usize,
        day,
        month,
        offset_minutes,
    }
}

 
 
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let of_era = z - era * 146_097;
    let year_of_era = (of_era - of_era / 1_460 + of_era / 36_524 - of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

 
pub(crate) fn now(config: &ClockConfig) -> ClockTime {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let mut time = reading(
        elapsed.as_secs() as i64,
        elapsed.subsec_nanos() as f32 / 1e9,
        config.offset_minutes(),
    );
    if !config.smooth {
        time.fraction = 0.0;
    }
    time
}

 
 
pub(crate) fn sample() -> ClockTime {
    ClockTime {
        hour: 10,
        minute: 10,
        second: 35,
        fraction: 0.4,
        weekday: 0,
        day: 26,
        month: 10,
        offset_minutes: 0,
    }
}

 
 
 
fn local_offset_minutes() -> i32 {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    const REFRESH: Duration = Duration::from_secs(30);
    static CACHE: Mutex<Option<(Instant, i32)>> = Mutex::new(None);

    let Ok(mut cache) = CACHE.lock() else {
        return 0;
    };
    if let Some((taken, offset)) = *cache {
        if taken.elapsed() < REFRESH {
            return offset;
        }
    }
    let offset = java_offset_minutes().unwrap_or(0);
    *cache = Some((Instant::now(), offset));
    offset
}

 
 
fn java_offset_minutes() -> Option<i32> {
    crate::state::try_client()?;
    let mapping = crate::state::mapping();
    let offset = mapping.in_frame(|| -> anyhow::Result<i32> {
        let mut env = mapping.get_env()?;
        let millis = env
            .call_static_method("java/lang/System", "currentTimeMillis", "()J", &[])?
            .j()?;
        let zone = env
            .call_static_method(
                "java/util/TimeZone",
                "getDefault",
                "()Ljava/util/TimeZone;",
                &[],
            )?
            .l()?;
        let offset = env
            .call_method(
                &zone,
                "getOffset",
                "(J)I",
                &[jni::objects::JValue::Long(millis)],
            )?
            .i()?;
        Ok(offset / 60_000)
    });
    match offset {
        Ok(minutes) => Some(minutes),
        Err(error) => {
            log::debug!("clock: the local time zone is unavailable ({error:#})");
            None
        }
    }
}

 
 
pub(crate) fn hour_angle(time: &ClockTime) -> f32 {
    let hour = (time.hour % 12) as f32;
    (hour + time.minute as f32 / 60.0 + time.second as f32 / 3_600.0) * 30.0
}

 
pub(crate) fn minute_angle(time: &ClockTime) -> f32 {
    (time.minute as f32 + time.second as f32 / 60.0) * 6.0
}

 
 
pub(crate) fn second_angle(time: &ClockTime) -> f32 {
    (time.second as f32 + time.fraction.clamp(0.0, 1.0)) * 6.0
}

 
 
 
fn accent_text() -> Color32 {
    theme::lerp_color(theme::accent(), theme::TEXT, 0.30)
}

 
 
fn at_angle(degrees: f32, radius: f32) -> Vec2 {
    let radians = degrees.rem_euclid(360.0).to_radians();
    egui::vec2(radians.sin() * radius, -radians.cos() * radius)
}

 
 
fn hand(center: Pos2, degrees: f32, tip: f32, heel: f32, half: f32, fill: Color32) -> Shape {
    let dir = at_angle(degrees, 1.0);
    let side = egui::vec2(-dir.y, dir.x) * half;
    let waist = center + dir * (half * 1.1);
    Shape::convex_polygon(
        vec![
            center + dir * tip,
            waist + side,
            center - dir * heel,
            waist - side,
        ],
        fill,
        Stroke::NONE,
    )
}

 
pub(crate) fn size(ctx: &Context, config: &ClockConfig) -> Vec2 {
    match config.face {
        Face::Analog => {
            let scale = config.scale;
            let panel = PILL_H + if config.date { DATE_H } else { 0.0 };
            Vec2::new(DIAL, DIAL + PILL_GAP + panel) * scale
        }
        Face::Digital => digital_card(ctx, config).size,
    }
}

 
pub(crate) fn draw(painter: &Painter, origin: Pos2, config: &ClockConfig, time: &ClockTime) {
    match config.face {
        Face::Analog => draw_analog(painter, origin, config, time),
        Face::Digital => draw_digital(painter, origin, config, time),
    }
}

 
 
pub(crate) fn draw_preview(painter: &Painter, origin: Pos2, config: &ClockConfig) {
    draw(painter, origin, config, &sample());
}

 

fn draw_analog(painter: &Painter, origin: Pos2, config: &ClockConfig, time: &ClockTime) {
    let scale = config.scale;
    let radius = config.radius();
    let center = origin + Vec2::splat(radius);
    let face = radius - FACE_INSET * scale;
    let accent = theme::accent();

     
    painter.circle_filled(
        center + egui::vec2(0.0, 2.0 * scale),
        radius,
        Color32::from_black_alpha(60),
    );
    painter.circle_filled(center, radius, theme::with_alpha(theme::base(), 0.94));
    painter.circle_stroke(
        center,
        radius - 0.6 * scale,
        Stroke::new(1.2 * scale, theme::BORDER),
    );
    painter.circle_filled(center, face, theme::with_alpha(theme::SURFACE, 0.72));
    painter.circle_stroke(
        center,
        face,
        Stroke::new(0.8 * scale, theme::with_alpha(theme::BORDER, 0.9)),
    );

     
     
    let tick_outer = face - 1.5 * scale;
    for step in 0..60 {
        let degrees = step as f32 * 6.0;
        let (length, width, alpha) = if step % 15 == 0 {
            (QUARTER_TICK, 1.8, 1.0)
        } else if step % 5 == 0 {
            (HOUR_TICK, 1.6, 0.9)
        } else {
            (MINUTE_TICK, 1.0, 0.45)
        };
        let direction = at_angle(degrees, 1.0);
        painter.line_segment(
            [
                center + direction * (tick_outer - length * scale),
                center + direction * tick_outer,
            ],
            Stroke::new(width * scale, theme::with_alpha(theme::TEXT_DIM, alpha)),
        );
    }

     
     
    let numeral_font = FontId::monospace(NUMERAL_SIZE * scale);
    for (label, degrees) in [("12", 0.0), ("3", 90.0), ("6", 180.0), ("9", 270.0)] {
        painter.text(
            center + at_angle(degrees, face * NUMERAL_RADIUS),
            Align2::CENTER_CENTER,
            label,
            numeral_font.clone(),
            theme::with_alpha(theme::TEXT, 0.86),
        );
    }

     
     
    for (degrees, (tip, heel, half)) in [
        (hour_angle(time), HOUR_HAND),
        (minute_angle(time), MINUTE_HAND),
    ] {
        painter.add(hand(
            center,
            degrees,
            face * tip,
            face * heel,
            half * scale,
            theme::TEXT,
        ));
    }
    if config.seconds {
        let degrees = second_angle(time);
        let (tip, heel) = SECOND_HAND;
        let direction = at_angle(degrees, 1.0);
        painter.line_segment(
            [
                center - direction * (face * heel),
                center + direction * (face * tip),
            ],
            Stroke::new(1.1 * scale, accent),
        );
        painter.circle_filled(
            center + direction * (face * tip * SECOND_DISC.0),
            SECOND_DISC.1 * scale,
            accent,
        );
    }

     
    painter.circle_filled(center, 3.0 * scale, theme::base());
    painter.circle_stroke(center, 3.0 * scale, Stroke::new(1.0 * scale, theme::BORDER));
    painter.circle_filled(center, 1.4 * scale, accent);

     
    let panel = Rect::from_min_size(
        origin + egui::vec2(0.0, DIAL * scale + PILL_GAP * scale),
        egui::vec2(
            DIAL * scale,
            (PILL_H + if config.date { DATE_H } else { 0.0 }) * scale,
        ),
    );
    painter.rect_filled(
        panel,
        Rounding::same(3.0 * scale),
        Color32::from_black_alpha(170),
    );
    painter.rect_filled(
        Rect::from_min_size(panel.min, egui::vec2(2.0 * scale, PILL_H * scale)),
        Rounding::same(1.0 * scale),
        accent,
    );
    let time_row = Rect::from_min_size(panel.min, egui::vec2(panel.width(), PILL_H * scale));
    let time_font = FontId::monospace(PILL_TIME * scale);
    let tag_font = FontId::monospace(PILL_TAG * scale);
    let readout = time.time_text(config.hour24, config.seconds);
    let tag = time.zone_text();
    let time_width = measure(painter.ctx(), &readout, &time_font);
    let tag_width = measure(painter.ctx(), &tag, &tag_font);
     
     
     
    if time_width + tag_width + 14.0 * scale <= panel.width() {
        painter.text(
            egui::pos2(time_row.min.x + 6.0 * scale, time_row.center().y),
            Align2::LEFT_CENTER,
            readout,
            time_font,
            theme::TEXT,
        );
        painter.text(
            egui::pos2(time_row.max.x - 6.0 * scale, time_row.center().y),
            Align2::RIGHT_CENTER,
            tag,
            tag_font,
            accent_text(),
        );
    } else {
        painter.text(
            time_row.center() + egui::vec2(1.0 * scale, 0.0),
            Align2::CENTER_CENTER,
            readout,
            time_font,
            theme::TEXT,
        );
    }
    if config.date {
        painter.text(
            egui::pos2(panel.center().x, panel.max.y - DATE_H * scale * 0.5),
            Align2::CENTER_CENTER,
            time.date_text(),
            FontId::proportional(8.0 * scale),
            theme::with_alpha(theme::TEXT_DIM, 0.9),
        );
    }
}

 

 
 
struct DigitalCard {
    size: Vec2,
    pad: f32,
    gap: f32,
    time: FontId,
    small: FontId,
    date: FontId,
    tag: FontId,
}

fn digital_card(ctx: &Context, config: &ClockConfig) -> DigitalCard {
    let scale = config.scale;
    let pad = DIGITAL_PAD * scale;
    let gap = 8.0 * scale;
    let time = FontId::monospace(DIGITAL_TIME * scale);
    let small = FontId::monospace(DIGITAL_SMALL * scale);
    let date = FontId::proportional(DIGITAL_DATE * scale);
    let tag = FontId::monospace(DIGITAL_TAG * scale);

     
     
    let sample = TimeParts {
        main: "88:88".to_string(),
        seconds: if config.seconds {
            ":88".into()
        } else {
            String::new()
        },
        suffix: if config.hour24 {
            String::new()
        } else {
            " PM".into()
        },
    };
    let row = measure(ctx, &sample.main, &time)
        + measure(ctx, &sample.seconds, &small)
        + measure(ctx, &sample.suffix, &small);
    let zone = sample_zone(config);
    let tag_width = measure(ctx, &zone, &tag) + 2.0 * 4.0 * scale;
    let date_width = measure(ctx, "Wed 26 Oct", &date);

    let width = row + gap + tag_width;
    let width = if config.date {
        width.max(pad * 2.0 + date_width)
    } else {
        width
    };
    let height = pad * 2.0
        + DIGITAL_TIME * scale * 1.05
        + if config.date {
            2.0 * scale + DIGITAL_DATE * scale * 1.25
        } else {
            0.0
        };
    DigitalCard {
        size: Vec2::new(width + pad * 2.0, height),
        pad,
        gap,
        time,
        small,
        date,
        tag,
    }
}

 
fn sample_zone(config: &ClockConfig) -> String {
    match config.zone {
        Zone::Utc => "UTC".to_string(),
         
         
        Zone::Local => reading(0, 0.0, config.offset_minutes()).zone_text(),
    }
}

fn draw_digital(painter: &Painter, origin: Pos2, config: &ClockConfig, time: &ClockTime) {
    let scale = config.scale;
    let card = digital_card(painter.ctx(), config);
    let rect = Rect::from_min_size(origin, card.size);
    let accent = theme::accent();

    painter.rect_filled(
        rect.translate(egui::vec2(0.0, 2.0 * scale)),
        Rounding::same(4.0 * scale),
        Color32::from_black_alpha(70),
    );
    painter.rect_filled(
        rect,
        Rounding::same(4.0 * scale),
        Color32::from_black_alpha(178),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(4.0 * scale),
        Stroke::new(1.0 * scale, theme::with_alpha(theme::BORDER, 0.85)),
    );
     
     
    painter.rect_filled(
        Rect::from_min_size(
            rect.min,
            egui::vec2(2.0 * scale, rect.height() - 8.0 * scale),
        ),
        Rounding::same(1.0 * scale),
        accent,
    );

    let parts = time.time_parts(config.hour24, config.seconds);
    let line = rect.min.y + card.pad + DIGITAL_TIME * scale * 0.525;
    let mut x = rect.min.x + card.pad + 4.0 * scale;
    painter.text(
        egui::pos2(x, line),
        Align2::LEFT_CENTER,
        parts.main.as_str(),
        card.time.clone(),
        theme::TEXT,
    );
    x += measure(painter.ctx(), &parts.main, &card.time);
    if !parts.seconds.is_empty() {
        painter.text(
            egui::pos2(x, line + DIGITAL_TIME * scale * 0.10),
            Align2::LEFT_CENTER,
            parts.seconds.as_str(),
            card.small.clone(),
            accent,
        );
        x += measure(painter.ctx(), &parts.seconds, &card.small);
    }
    if !parts.suffix.is_empty() {
        painter.text(
            egui::pos2(x, line + DIGITAL_TIME * scale * 0.10),
            Align2::LEFT_CENTER,
            parts.suffix,
            card.small.clone(),
            theme::with_alpha(theme::TEXT_DIM, 0.95),
        );
    }

     
    let tag = time.zone_text();
    let tag_width = measure(painter.ctx(), &tag, &card.tag) + 8.0 * scale;
    let tag_height = DIGITAL_TAG * scale + 5.0 * scale;
    let badge = Rect::from_min_size(
        egui::pos2(rect.max.x - card.pad - tag_width, line - tag_height * 0.5),
        egui::vec2(tag_width, tag_height),
    );
    painter.rect_filled(
        badge,
        Rounding::same(3.0 * scale),
        theme::with_alpha(accent, 0.14),
    );
    painter.rect_stroke(
        badge,
        Rounding::same(3.0 * scale),
        Stroke::new(1.0 * scale, theme::with_alpha(accent, 0.40)),
    );
    painter.text(
        badge.center(),
        Align2::CENTER_CENTER,
        tag,
        card.tag.clone(),
        accent_text(),
    );

    if config.date {
        painter.text(
            egui::pos2(
                rect.min.x + card.pad + 4.0 * scale,
                rect.max.y - card.pad - DIGITAL_DATE * scale * 0.5,
            ),
            Align2::LEFT_CENTER,
            time.date_text(),
            card.date.clone(),
            theme::with_alpha(theme::TEXT_DIM, 0.92),
        );
    }
}

 
fn measure(ctx: &Context, text: &str, font: &FontId) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    ctx.fonts(|fonts| {
        fonts
            .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Color32, LayerId, Order};

     
    const MONDAY: i64 = 1_792_972_800;

    fn painted(config: &ClockConfig, time: &ClockTime) -> (Vec<PaintedText>, Vec<Shape>, Rect) {
        let ctx = Context::default();
        let origin = egui::pos2(24.0, 18.0);
        let mut texts = Vec::new();
        let mut shapes = Vec::new();
        let mut bounds = Rect::ZERO;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(520.0, 420.0))),
                ..Default::default()
            },
            |ctx| {
                let painter =
                    ctx.layer_painter(LayerId::new(Order::Background, egui::Id::new("clock")));
                bounds = Rect::from_min_size(origin, size(ctx, config));
                draw(&painter, origin, config, time);
            },
        );
        for clipped in &output.shapes {
            shapes.push(clipped.shape.clone());
            if let egui::Shape::Text(text) = &clipped.shape {
                texts.push(PaintedText {
                    text: text.galley.text().to_owned(),
                    color: text.override_text_color.unwrap_or(text.fallback_color),
                    rect: Rect::from_min_size(text.pos, text.galley.size()),
                });
            }
        }
        (texts, shapes, bounds)
    }

     
    struct PaintedText {
        text: String,
        color: Color32,
        rect: Rect,
    }

     
    fn hand_tips(shapes: &[Shape], center: Pos2) -> Vec<(f32, f32)> {
        let mut tips = Vec::new();
        for shape in shapes {
            let egui::Shape::Path(path) = shape else {
                continue;
            };
            if path.fill != theme::TEXT {
                continue;
            }
            let Some(tip) = path
                .points
                .iter()
                .copied()
                .max_by(|a, b| (*a - center).length().total_cmp(&(*b - center).length()))
            else {
                continue;
            };
            let offset = tip - center;
            tips.push((
                offset.length(),
                offset.x.atan2(-offset.y).to_degrees().rem_euclid(360.0),
            ));
        }
        tips
    }

    #[test]
    fn a_unix_stamp_reads_as_a_wall_clock() {
        let time = reading(0, 0.0, 0);
        assert_eq!((time.hour, time.minute, time.second), (0, 0, 0));
        assert_eq!((time.day, time.month), (1, 1));
        assert_eq!(WEEKDAYS[time.weekday], "Thu", "1970-01-01 was a Thursday");

        assert_eq!(reading(0, 0.75, 0).fraction, 0.75);

        let late = reading(MONDAY + 23 * 3_600 + 59 * 60 + 59, 0.0, 0);
        assert_eq!((late.hour, late.minute, late.second), (23, 59, 59));
        assert_eq!((late.day, late.month), (26, 10));
        assert_eq!(WEEKDAYS[late.weekday], "Mon");
         
        let midnight = reading(MONDAY + 86_400, 0.0, 0);
        assert_eq!((midnight.hour, midnight.day, midnight.month), (0, 27, 10));
        assert_eq!(WEEKDAYS[midnight.weekday], "Tue");
    }

    #[test]
    fn the_zone_shifts_the_time_and_rolls_the_date_with_it() {
        let stamp = MONDAY + 23 * 3_600 + 30 * 60;
        let ahead = reading(stamp, 0.0, 120);
        assert_eq!((ahead.hour, ahead.minute), (1, 30));
        assert_eq!((ahead.day, WEEKDAYS[ahead.weekday]), (27, "Tue"));

        let behind = reading(stamp, 0.0, -90);
        assert_eq!((behind.hour, behind.minute), (22, 0));
        assert_eq!((behind.day, WEEKDAYS[behind.weekday]), (26, "Mon"));

         
        let india = reading(MONDAY + 3_600, 0.0, 330);
        assert_eq!((india.hour, india.minute), (6, 30));
    }

    #[test]
    fn the_zone_badge_spells_the_offset_out() {
        assert_eq!(reading(0, 0.0, 0).zone_text(), "UTC");
        assert_eq!(reading(0, 0.0, 120).zone_text(), "UTC+2");
        assert_eq!(reading(0, 0.0, -240).zone_text(), "UTC-4");
        assert_eq!(reading(0, 0.0, 330).zone_text(), "UTC+5:30");
        assert_eq!(reading(0, 0.0, -270).zone_text(), "UTC-4:30");
    }

    #[test]
    fn the_hands_follow_the_reading() {
        let midnight = reading(MONDAY, 0.0, 0);
        assert_eq!(hour_angle(&midnight), 0.0);
        assert_eq!(minute_angle(&midnight), 0.0);
        assert_eq!(second_angle(&midnight), 0.0);

         
        let three = ClockTime {
            hour: 3,
            ..midnight
        };
        assert_eq!(hour_angle(&three), 90.0);
        assert_eq!(minute_angle(&three), 0.0);

         
        let half_six = ClockTime {
            hour: 6,
            minute: 30,
            ..midnight
        };
        assert_eq!(hour_angle(&half_six), 195.0);
        assert_eq!(minute_angle(&half_six), 180.0);

         
        let quarter = ClockTime {
            hour: 16,
            minute: 25,
            second: 30,
            ..midnight
        };
        assert!((hour_angle(&quarter) - 132.75).abs() < 1e-3);
        assert_eq!(minute_angle(&quarter), 153.0);
        assert_eq!(second_angle(&quarter), 180.0);

         
        let sweeping = ClockTime {
            second: 10,
            fraction: 0.5,
            ..midnight
        };
        assert_eq!(second_angle(&sweeping), 63.0);
    }

    #[test]
    fn the_digital_readout_respects_the_switches() {
        let time = ClockTime {
            hour: 13,
            minute: 5,
            second: 9,
            ..reading(MONDAY, 0.0, 0)
        };
        assert_eq!(time.time_text(true, true), "13:05:09");
        assert_eq!(time.time_text(true, false), "13:05");
        assert_eq!(time.time_text(false, true), "1:05:09 PM");
        assert_eq!(time.time_text(false, false), "1:05 PM");

         
        let midnight = ClockTime {
            hour: 0,
            minute: 0,
            second: 0,
            ..time
        };
        assert_eq!(midnight.time_text(false, true), "12:00:00 AM");
        let noon = ClockTime {
            hour: 12,
            ..midnight
        };
        assert_eq!(noon.time_text(false, false), "12:00 PM");
        assert_eq!(noon.time_text(true, false), "12:00");
    }

    #[test]
    fn the_date_row_names_the_weekday_month_and_day() {
        assert_eq!(reading(MONDAY, 0.0, 0).date_text(), "Mon 26 Oct");
        assert_eq!(
            reading(MONDAY + 86_400 * 2, 0.0, 0).date_text(),
            "Wed 28 Oct"
        );
        assert_eq!(
            reading(MONDAY + 86_400 * 31, 0.0, 0).date_text(),
            "Thu 26 Nov"
        );
         
        assert_eq!(
            reading(MONDAY + 86_400 * 6, 0.0, 0).date_text(),
            "Sun 1 Nov"
        );
    }

    #[test]
    fn the_widget_is_the_face_plus_its_readout() {
        let ctx = Context::default();
         
        let _ = ctx.run(egui::RawInput::default(), |_| {});
        let plain = ClockConfig::default();
        assert_eq!(
            size(&ctx, &plain),
            Vec2::new(DIAL, DIAL + PILL_GAP + PILL_H)
        );
        let dated = ClockConfig {
            date: true,
            ..plain
        };
        assert_eq!(
            size(&ctx, &dated),
            Vec2::new(DIAL, DIAL + PILL_GAP + PILL_H + DATE_H)
        );
         
        let big = ClockConfig {
            scale: 1.5,
            ..plain
        };
        let scaled = size(&ctx, &big);
        assert!((scaled.x - DIAL * 1.5).abs() < 1e-3);
        assert!((scaled.y - (DIAL + PILL_GAP + PILL_H) * 1.5).abs() < 1e-3);
         
        let digital = ClockConfig {
            face: Face::Digital,
            ..plain
        };
        let card = size(&ctx, &digital);
        assert!(card.x > card.y);
        assert!(card.x > 120.0 && card.y > 30.0);
    }

    #[test]
    fn the_analog_face_paints_the_dial_hands_and_readout() {
        let config = ClockConfig {
            seconds: true,
            ..ClockConfig::default()
        };
        let time = sample();
        let (texts, shapes, bounds) = painted(&config, &time);
        let labels: Vec<&str> = texts.iter().map(|text| text.text.as_str()).collect();
        for numeral in ["12", "3", "6", "9"] {
            assert!(labels.contains(&numeral), "missing {numeral}: {labels:?}");
        }
        assert!(
            labels.contains(&"10:10:35"),
            "missing the readout: {labels:?}"
        );
        assert!(labels.contains(&"UTC"), "missing the zone: {labels:?}");

         
        let lines = shapes
            .iter()
            .filter(|shape| matches!(shape, egui::Shape::LineSegment { .. }))
            .count();
        assert_eq!(lines, 61, "expected 60 ticks and the second hand");

         
         
        let center = bounds.min + Vec2::splat(config.radius());
        let mut tips = hand_tips(&shapes, center);
        tips.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert_eq!(tips.len(), 2, "hour and minute hands only");
        let (hour, minute) = (tips[0], tips[1]);
        assert!(
            (hour.1 - hour_angle(&time)).abs() < 0.5,
            "hour hand at {}°",
            hour.1
        );
        assert!(
            (minute.1 - minute_angle(&time)).abs() < 0.5,
            "minute hand at {}°",
            minute.1
        );
        assert!(
            minute.0 > hour.0 + 10.0,
            "the minute hand reaches past the hour hand"
        );
    }

    #[test]
    fn the_second_hand_is_accent_coloured_and_switchable() {
        let config = ClockConfig::default();
        let (_, shapes, _) = painted(&config, &sample());
        let accent_lines = shapes
            .iter()
            .filter(|shape| {
                let egui::Shape::LineSegment { stroke, .. } = shape else {
                    return false;
                };
                stroke.color == egui::epaint::ColorMode::Solid(theme::accent())
            })
            .count();
        assert_eq!(accent_lines, 1, "the sweep hand is the accent line");

        let without = ClockConfig {
            seconds: false,
            ..config
        };
        let (_, shapes, _) = painted(&without, &sample());
        let accent_lines = shapes
            .iter()
            .filter(|shape| match shape {
                egui::Shape::LineSegment { stroke, .. } => {
                    stroke.color == egui::epaint::ColorMode::Solid(theme::accent())
                }
                _ => false,
            })
            .count();
        assert_eq!(accent_lines, 0, "no sweep hand when seconds are off");
    }

    #[test]
    fn the_digital_card_paints_the_time_seconds_and_date() {
        let config = ClockConfig {
            face: Face::Digital,
            hour24: true,
            seconds: true,
            date: true,
            ..ClockConfig::default()
        };
        let (texts, _, bounds) = painted(&config, &sample());
        let labels: Vec<&str> = texts.iter().map(|text| text.text.as_str()).collect();
        assert!(
            labels.contains(&"10:10"),
            "missing the big reading: {labels:?}"
        );
        assert!(labels.contains(&":35"), "missing the seconds: {labels:?}");
        assert!(
            labels.contains(&"Mon 26 Oct"),
            "missing the date: {labels:?}"
        );
        assert!(
            labels.contains(&"UTC"),
            "missing the zone badge: {labels:?}"
        );
         
        let seconds = texts.iter().find(|text| text.text == ":35").unwrap();
        assert_eq!(seconds.color, theme::accent());
        let digits = texts.iter().find(|text| text.text == "10:10").unwrap();
        assert_eq!(digits.color, theme::TEXT);
        assert!(bounds.width() > 120.0);
    }

    #[test]
    fn every_readout_fits_its_own_plate() {
         
         
        let faces = [Face::Analog, Face::Digital];
        for face in faces {
            for hour24 in [true, false] {
                for seconds in [true, false] {
                    for date in [true, false] {
                        for offset in [0, 120, -240, 330, -270, 765] {
                            let config = ClockConfig {
                                face,
                                hour24,
                                seconds,
                                date,
                                ..ClockConfig::default()
                            };
                            let time = reading(MONDAY + 23 * 3_600 + 59 * 60 + 59, 0.7, offset);
                            let (texts, _, bounds) = painted(&config, &time);
                            for text in &texts {
                                let inside = bounds.expand(0.5).contains_rect(text.rect);
                                assert!(
                                    inside,
                                    "{:?} overflowed {bounds:?} by {:?}",
                                    text.text, text.rect
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_pill_keeps_the_time_when_the_zone_will_not_fit() {
         
         
         
        let config = ClockConfig {
            hour24: false,
            seconds: true,
            ..ClockConfig::default()
        };
        let time = reading(MONDAY + 23 * 3_600 + 59 * 60 + 59, 0.0, 765);
        let (texts, _, bounds) = painted(&config, &time);
        let labels: Vec<&str> = texts.iter().map(|text| text.text.as_str()).collect();
        assert!(labels.contains(&"12:44:59 PM"), "{labels:?}");
        assert!(
            !labels.iter().any(|label| label.starts_with("UTC")),
            "the badge was dropped, not clipped: {labels:?}"
        );
        for text in &texts {
            assert!(bounds.contains_rect(text.rect), "{} spilled", text.text);
        }
         
        let brief = ClockConfig {
            seconds: false,
            ..config
        };
        let close = reading(MONDAY + 11 * 3_600 + 59 * 60 + 59, 0.0, 120);
        let (texts, _, _) = painted(&brief, &close);
        let labels: Vec<&str> = texts.iter().map(|text| text.text.as_str()).collect();
        assert!(labels.contains(&"1:59 PM"), "{labels:?}");
        assert!(labels.contains(&"UTC+2"), "{labels:?}");
    }

     
     
     
    #[test]
    fn the_clock_preview_renders_both_faces() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let variants = [
            (ClockConfig::default(), sample()),
            (
                ClockConfig {
                    hour24: false,
                    date: true,
                    ..ClockConfig::default()
                },
                reading(MONDAY + 21 * 3_600 + 7 * 60 + 12, 0.6, 120),
            ),
            (
                ClockConfig {
                    face: Face::Digital,
                    ..ClockConfig::default()
                },
                sample(),
            ),
            (
                ClockConfig {
                    face: Face::Digital,
                    hour24: false,
                    date: true,
                    ..ClockConfig::default()
                },
                reading(MONDAY + 21 * 3_600 + 7 * 60 + 12, 0.6, -240),
            ),
            (
                ClockConfig {
                    face: Face::Digital,
                    seconds: false,
                    date: true,
                    ..ClockConfig::default()
                },
                reading(MONDAY + 7 * 3_600 + 4 * 60, 0.0, 0),
            ),
        ];
        let width = 620usize;
        let height = 440usize;
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        for frame in 0..4 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    let painter =
                        ctx.layer_painter(LayerId::new(Order::Background, egui::Id::new("clock")));
                    let mut at = egui::pos2(14.0, 14.0);
                    let mut tallest = 0.0_f32;
                    for (index, (config, time)) in variants.iter().enumerate() {
                        draw(&painter, at, config, time);
                        tallest = tallest.max(size(ctx, config).y);
                        if index % 2 == 1 {
                            at = egui::pos2(14.0, at.y + tallest + 14.0);
                            tallest = 0.0;
                        } else {
                            at.x += 210.0;
                        }
                    }
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
        let backdrop = [0.055, 0.055, 0.07, 1.0];
        let image = rasterize(primitives, &textures, width, height, backdrop);
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/clock-preview.png");
        image.save(&path).expect("the clock preview png saves");
        let lit = image
            .pixels()
            .filter(|pixel| {
                pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32 > 130 && pixel[3] > 128
            })
            .count();
        println!("clock preview: {lit} lit pixels -> {path:?}");
        assert!(lit > 4000, "only {lit} lit pixels — the faces did not draw");
    }
}
