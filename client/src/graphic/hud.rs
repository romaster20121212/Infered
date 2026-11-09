 
 
 
 

use crate::graphic::anim::{self, Easing};
use crate::graphic::now_playing;
use crate::graphic::theme;
use crate::graphic::transparency::{self, Layer};
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::inventory::{self, Inventory};
use crate::mapping::MappedObject;
use crate::module::render::custom_ui::{self, UiStyle};
use egui::{
    Align2, Color32, Context, FontId, Id, LayerId, Order, Painter, Pos2, Rect, Rounding, Stroke,
    TextureHandle, Vec2,
};

 
const MARGIN: f32 = 10.0;

 
 
 
 
 
 
 

 
const HOTBAR_WIDTH: f64 = 182.0;
const HOTBAR_HEIGHT: f64 = 22.0;
const HOTBAR_SLOT_INSET: f64 = 3.0;
const HOTBAR_SLOT: f64 = 20.0;

 
#[derive(Clone, Copy, Debug, PartialEq)]
struct GuiMetrics {
     
     
    scale: f64,
     
    scaled_width: i32,
    scaled_height: i32,
}

 
 
 
 
fn ui_scale(metrics: GuiMetrics) -> Option<f64> {
    let physical_height = f64::from(metrics.scaled_height) * metrics.scale;
    (physical_height > 0.0).then(|| (physical_height / 720.0).max(1.0).floor())
}

 
 
fn hotbar_first_slot() -> Option<Rect> {
    hotbar_first_slot_in(gui_metrics()?)
}

 
 
 
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HotbarFrame {
     
    pub bar: Rect,
     
    pub first_slot: Rect,
     
    pub to_hud: f32,
}

 
 
pub(crate) fn hotbar_frame() -> Option<HotbarFrame> {
    hotbar_frame_in(gui_metrics()?)
}

 
 
fn hotbar_frame_in(metrics: GuiMetrics) -> Option<HotbarFrame> {
    let to_hud = (metrics.scale / ui_scale(metrics)?) as f32;
    if !to_hud.is_finite() || to_hud <= 0.0 {
        return None;
    }
    let left = f64::from(metrics.scaled_width) * 0.5 - HOTBAR_WIDTH * 0.5;
    let top = f64::from(metrics.scaled_height) - HOTBAR_HEIGHT;
    Some(HotbarFrame {
        bar: Rect::from_min_size(
            egui::pos2(left as f32 * to_hud, top as f32 * to_hud),
            Vec2::new(HOTBAR_WIDTH as f32 * to_hud, HOTBAR_HEIGHT as f32 * to_hud),
        ),
        first_slot: hotbar_first_slot_in(metrics)?,
        to_hud,
    })
}

 
 
fn hotbar_first_slot_in(metrics: GuiMetrics) -> Option<Rect> {
    let to_hud = (metrics.scale / ui_scale(metrics)?) as f32;
    if !to_hud.is_finite() || to_hud <= 0.0 {
        return None;
    }
    let left = f64::from(metrics.scaled_width) * 0.5 - HOTBAR_WIDTH * 0.5;
    let top = f64::from(metrics.scaled_height) - HOTBAR_HEIGHT;
    Some(Rect::from_min_size(
        egui::pos2(
            (left + HOTBAR_SLOT_INSET) as f32 * to_hud,
            (top + HOTBAR_SLOT_INSET) as f32 * to_hud,
        ),
        Vec2::splat(HOTBAR_SLOT as f32 * to_hud),
    ))
}

 
 
 
 
 
 
pub(crate) fn gui_to_overlay() -> Option<f32> {
    let metrics = gui_metrics()?;
    let factor = (metrics.scale / ui_scale(metrics)?) as f32;
    (factor.is_finite() && factor > 0.0).then_some(factor)
}

 
 
fn gui_metrics() -> Option<GuiMetrics> {
    const REFRESH: std::time::Duration = std::time::Duration::from_secs(1);
    static METRICS: std::sync::Mutex<Option<(GuiMetrics, std::time::Instant)>> =
        std::sync::Mutex::new(None);

    if let Ok(cache) = METRICS.lock() {
        if let Some((metrics, at)) = cache.as_ref() {
            if at.elapsed() < REFRESH {
                return Some(*metrics);
            }
        }
    }

     
     
     
    let window = &crate::state::try_minecraft()?.window;
    let metrics = GuiMetrics {
        scale: window.gui_scale().ok()?,
        scaled_width: window.gui_scaled_width().ok()?,
        scaled_height: window.gui_scaled_height().ok()?,
    };
    if let Ok(mut cache) = METRICS.lock() {
        *cache = Some((metrics, std::time::Instant::now()));
    }
    Some(metrics)
}

 
 
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum HudWidgetId {
    Radar,
    KeyStrokes,
    Fps,
    Coords,
    Compass,
    Clock,
    ArmorStatus,
    Status,
    PotionStatus,
    TotemCounter,
    PlayerEsp,
    Spotify,
    Lyrics,
    Speed,
    Session,
    Ping,
    TargetInfo,
}

impl HudWidgetId {
     
    pub fn display_name(self) -> &'static str {
        match self {
            HudWidgetId::Radar => "Radar",
            HudWidgetId::KeyStrokes => "KeyStrokes",
            HudWidgetId::Fps => "FPS",
            HudWidgetId::Coords => "Coords",
            HudWidgetId::Compass => "Compass",
            HudWidgetId::Clock => "Clock",
            HudWidgetId::ArmorStatus => "Armor Status",
            HudWidgetId::Status => "Status",
            HudWidgetId::PotionStatus => "Potion Status",
            HudWidgetId::TotemCounter => "Totem Counter",
            HudWidgetId::PlayerEsp => "Nametags",
            HudWidgetId::Spotify => "Spotify",
            HudWidgetId::Lyrics => "Lyrics",
            HudWidgetId::Speed => "Speed",
            HudWidgetId::Session => "Session",
            HudWidgetId::Ping => "Ping",
            HudWidgetId::TargetInfo => "Target Info",
        }
    }

     
    pub fn all() -> [HudWidgetId; 17] {
        [
            HudWidgetId::Radar,
            HudWidgetId::KeyStrokes,
            HudWidgetId::Fps,
            HudWidgetId::Coords,
            HudWidgetId::Compass,
            HudWidgetId::Clock,
            HudWidgetId::ArmorStatus,
            HudWidgetId::Status,
            HudWidgetId::PotionStatus,
            HudWidgetId::TotemCounter,
            HudWidgetId::PlayerEsp,
            HudWidgetId::Spotify,
            HudWidgetId::Lyrics,
            HudWidgetId::Speed,
            HudWidgetId::Session,
            HudWidgetId::Ping,
            HudWidgetId::TargetInfo,
        ]
    }

     
     
     
    pub fn anchors_bottom(self) -> bool {
        matches!(self, HudWidgetId::PotionStatus)
    }

     
    pub fn index(self) -> usize {
        Self::all()
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0)
    }

     
     
     
    pub fn default_pos(self, screen: &Rect) -> Pos2 {
        let h = screen.height();
        match self {
            HudWidgetId::Radar => egui::pos2(MARGIN, MARGIN),
            HudWidgetId::KeyStrokes => egui::pos2(screen.width() * 0.5 - 49.0, h - MARGIN - 98.0),
            HudWidgetId::Fps => egui::pos2(MARGIN, h - MARGIN - 18.0),
            HudWidgetId::Coords => egui::pos2(MARGIN, h - MARGIN - 39.0),
             
             
            HudWidgetId::Compass => egui::pos2(MARGIN, MARGIN + 120.0 + 6.0),
             
             
             
            HudWidgetId::Clock => egui::pos2(MARGIN + 120.0 + 6.0, MARGIN),
             
             
             
            HudWidgetId::ArmorStatus => egui::pos2(
                screen.width() - MARGIN - ARMOR_W,
                h * 0.5 - armor_height() * 0.5,
            ),
             
             
            HudWidgetId::Status => egui::pos2(MARGIN, h * 0.5 - 34.0),
             
             
            HudWidgetId::PotionStatus => egui::pos2(MARGIN, h - MARGIN - 133.0),
             
             
             
            HudWidgetId::Speed => {
                egui::pos2(MARGIN, h - MARGIN - 60.0 - crate::graphic::speedometer::H)
            }
             
             
            HudWidgetId::Session => egui::pos2(MARGIN, MARGIN + 120.0 + 6.0 + 110.0 + 6.0),
             
             
            HudWidgetId::Ping => egui::pos2(MARGIN + 120.0 + 6.0, MARGIN + 138.0),
            HudWidgetId::TotemCounter => totem_counter_pos(screen),
            HudWidgetId::Spotify => {
                let size = spotify_card_size(&SpotifyStyle::default());
                egui::pos2(screen.width() - MARGIN - size.x, h - MARGIN - size.y)
            }
            HudWidgetId::Lyrics => {
                let size = crate::graphic::lyrics_card::default_size();
                egui::pos2(screen.width() - MARGIN - size.x, h - MARGIN - size.y)
            }
            HudWidgetId::PlayerEsp => {
                egui::pos2(screen.width() * 0.5 - 85.0, screen.height() * 0.5 - 22.0)
            }
            HudWidgetId::TargetInfo => {
                let size = crate::graphic::target_info::panel_size();
                egui::pos2(screen.width() - MARGIN - size.x, MARGIN + 96.0)
            }
        }
    }
}

 
 
pub(crate) fn widget_target_pos(id: HudWidgetId, screen: &Rect) -> Pos2 {
    crate::config::widget_pos(id)
        .map(|p| egui::pos2(p[0], p[1]))
        .unwrap_or_else(|| id.default_pos(screen))
}

 
 
 
 
 
 
pub(crate) fn player_esp_editor_offset(ctx: &Context) -> Vec2 {
    let screen = ctx.screen_rect();
    let default = HudWidgetId::PlayerEsp.default_pos(&screen);
    let target = widget_target_pos(HudWidgetId::PlayerEsp, &screen);
    target - default
}

pub(crate) fn widget_rendered_pos(ctx: &Context, id: HudWidgetId) -> Pos2 {
    let screen = ctx.screen_rect();
    let target = widget_target_pos(id, &screen);
    let settled = anim::spring_pos(
        ctx,
        Id::new("widget_pos").with(id),
        target,
        anim::SpringCfg::PANEL,
    );
    settled + custom_ui::UiStyle::reveal_pose(ctx, id.index(), HudWidgetId::all().len()).offset
}

 
 
 
 
 
 
 
pub(crate) fn layer_id() -> Id {
    Id::new("hud_layer")
}

 
 
 
 
 
pub fn draw(ctx: &Context, watermark_visible: bool) {
    let painter = ctx.layer_painter(LayerId::new(Order::Background, layer_id()));
    if let Some(list) = module_list_config() {
        if list.enabled {
            let top = if list.show_logo {
                if watermark_visible {
                    draw_watermark(
                        ctx,
                        &painter,
                        list.logo_size,
                        list.logo_opacity,
                        list.right_margin,
                    )
                } else {
                    MARGIN + list.logo_size + 6.0
                }
            } else {
                MARGIN
            };
            draw_arraylist(ctx, &painter, top + list.top_offset, &list);
        }
    }
    draw_widgets(ctx, &painter);
    if widget_enabled(crate::module::ModuleId::TargetInfo) {
        draw_widget_faded(ctx, &painter, HudWidgetId::TargetInfo, |painter| {
            crate::graphic::target_info::draw(ctx, painter)
        });
    }
    draw_auto_mace_status(ctx, &painter);
    crate::macros::draw_hud(ctx);
     
     
    crate::graphic::hotbar::draw(ctx, &painter);
    draw_top_bar(ctx, &painter);
}

 
 
 
 
fn draw_auto_mace_status(ctx: &Context, painter: &Painter) {
    let Some(line) = crate::module::combat::auto_mace::hud_line() else {
        return;
    };
    let font = FontId::proportional(11.0);
    let screen = ctx.screen_rect();
    let galley =
        ctx.fonts(|fonts| fonts.layout_no_wrap(line.clone(), font.clone(), Color32::WHITE));
    let size = Vec2::new(galley.size().x + 18.0, galley.size().y + 10.0);
    let rect = Rect::from_min_size(
        egui::pos2(screen.center().x - size.x * 0.5, MARGIN + 36.0),
        size,
    );
    painter.rect_filled(
        rect,
        Rounding::same(5.0),
        theme::with_alpha(theme::base(), 0.72),
    );
    painter.rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(2.0, rect.height())),
        Rounding::ZERO,
        theme::accent(),
    );
    painter.text(
        egui::pos2(rect.min.x + 9.0, rect.center().y),
        Align2::LEFT_CENTER,
        line,
        font,
        Color32::WHITE,
    );
}

 
 
fn widget_enabled(id: crate::module::ModuleId) -> bool {
    crate::state::client()
        .modules
        .get(id)
        .and_then(|handle| {
            let module = handle.lock().ok()?;
            Some(module.get_module_data().enabled)
        })
        .unwrap_or(false)
}

 
#[derive(Clone, Copy, PartialEq, Eq)]
enum ModuleListSort {
    Length,
    Name,
    Category,
}

 
 
 
struct ModuleListConfig {
    enabled: bool,
    show_logo: bool,
    logo_size: f32,
    logo_opacity: f32,
    show_background: bool,
    background_opacity: f32,
    show_border: bool,
    accent_bar: bool,
    show_dots: bool,
    show_count: bool,
    lowercase: bool,
    text_color: Color32,
    text_size: f32,
    row_height: f32,
    row_spacing: f32,
    right_margin: f32,
    top_offset: f32,
    sort: ModuleListSort,
}

fn module_list_config() -> Option<ModuleListConfig> {
    let handle = crate::state::client()
        .modules
        .get(crate::module::ModuleId::ModuleList)?;
    let module = handle.lock().ok()?;
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
        Some(crate::module::ModuleSetting::Color { value, .. }) => {
            let channel = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            Color32::from_rgb(channel(value[0]), channel(value[1]), channel(value[2]))
        }
        _ => default,
    };
    let sort = match data.get_setting("Sort") {
        Some(crate::module::ModuleSetting::Choice { value, .. }) => match value {
            1 => ModuleListSort::Name,
            2 => ModuleListSort::Category,
            _ => ModuleListSort::Length,
        },
        _ => ModuleListSort::Length,
    };
    Some(ModuleListConfig {
        enabled: data.enabled,
        show_logo: toggle("Show Logo", true),
        logo_size: slider("Logo Size", 135.0).clamp(60.0, 220.0),
        logo_opacity: slider("Logo Opacity", 1.0).clamp(0.0, 1.0),
        show_background: toggle("Show Background", true),
        background_opacity: slider("Background Opacity", 0.76).clamp(0.0, 1.0),
        show_border: toggle("Show Border", true),
        accent_bar: toggle("Accent Bar", true),
        show_dots: toggle("Show Dots", false),
        show_count: toggle("Show Count", false),
        lowercase: toggle("Lowercase", false),
        text_color: color("Text Color", theme::TEXT),
        text_size: slider("Text Size", 14.0).clamp(8.0, 24.0),
        row_height: slider("Row Height", 20.0).clamp(14.0, 32.0),
        row_spacing: slider("Row Spacing", 2.0).clamp(0.0, 10.0),
        right_margin: slider("Right Margin", MARGIN).clamp(0.0, 40.0),
        top_offset: slider("Top Offset", 0.0).clamp(0.0, 200.0),
        sort,
    })
}

 

 
#[derive(Default)]
struct WidgetCache {
    effects: Vec<EffectStatus>,
     
     
    effects_motion: PotionMotion,
     
    armor: [Option<ArmorPiece>; 4],
     
     
    armor_motion: ArmorMotion,
     
     
     
    armor_opened: bool,
    radar: Vec<([f64; 3], RadarKind)>,
     
    totems: i32,
     
    totem_roll: TotemRoll,
    last_effects: Option<std::time::Instant>,
    last_armor: Option<std::time::Instant>,
    last_radar: Option<std::time::Instant>,
    last_totems: Option<std::time::Instant>,
     
    fps: f32,
    last_frame: Option<std::time::Instant>,
     
     
    speed_sample: Option<([f64; 3], std::time::Instant)>,
    speed: f64,
     
     
    speed_peak: f64,
     
    ping: Option<(Option<i32>, std::time::Instant)>,
     
    ping_history: Vec<u32>,
}

static WIDGET_CACHE: std::sync::Mutex<Option<WidgetCache>> = std::sync::Mutex::new(None);

fn cache() -> std::sync::MutexGuard<'static, Option<WidgetCache>> {
    WIDGET_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
 
 
fn draw_widgets(ctx: &Context, painter: &Painter) {
     
    let now = std::time::Instant::now();
    {
        let mut cache = cache();
        let cache = cache.get_or_insert_with(WidgetCache::default);
        if let Some(last) = cache.last_frame {
            let dt = now.duration_since(last).as_secs_f32();
            if dt > 0.0 {
                let instant = 1.0 / dt;
                cache.fps = if cache.fps <= 0.0 {
                    instant
                } else {
                    cache.fps * 0.9 + instant * 0.1
                };
            }
        }
        cache.last_frame = Some(now);
    }

     
     
     
     

    if widget_enabled(crate::module::ModuleId::Radar) {
        refresh_radar(&mut cache());
        draw_widget_faded(ctx, painter, HudWidgetId::Radar, |painter| {
            draw_radar(ctx, painter)
        });
    } else if let Some(cache) = cache().as_mut() {
        cache.radar.clear();
        cache.last_radar = None;
    }
    if widget_enabled(crate::module::ModuleId::KeyStrokes) {
        draw_widget_faded(ctx, painter, HudWidgetId::KeyStrokes, |painter| {
            draw_key_strokes(ctx, painter)
        });
    }

    if widget_enabled(crate::module::ModuleId::Fps) {
        let fps = cache().as_ref().map(|c| c.fps).unwrap_or(0.0);
        draw_widget_faded(ctx, painter, HudWidgetId::Fps, |painter| {
            draw_text_widget(
                ctx,
                painter,
                HudWidgetId::Fps,
                vec![StatRow::new("FPS", (fps.round() as i32).to_string())],
            )
        });
    }
    if widget_enabled(crate::module::ModuleId::Coords) {
        if let Some(pos) = player_position() {
            draw_widget_faded(ctx, painter, HudWidgetId::Coords, |painter| {
                draw_text_widget(
                    ctx,
                    painter,
                    HudWidgetId::Coords,
                    vec![StatRow::new(
                        "XYZ",
                        format!("{:.1} / {:.1} / {:.1}", pos[0], pos[1], pos[2]),
                    )],
                )
            });
        }
    }
    if widget_enabled(crate::module::ModuleId::Compass) {
        draw_widget_faded(ctx, painter, HudWidgetId::Compass, |painter| {
            draw_compass(ctx, painter)
        });
    }
    if widget_enabled(crate::module::ModuleId::Clock) {
         
         
        let config = crate::graphic::clock::ClockConfig::read();
        let time = crate::graphic::clock::now(&config);
        draw_widget_faded(ctx, painter, HudWidgetId::Clock, |painter| {
            crate::graphic::clock::draw(
                painter,
                widget_rendered_pos(ctx, HudWidgetId::Clock),
                &config,
                &time,
            )
        });
    }
    if widget_enabled(crate::module::ModuleId::Speedometer) {
        refresh_speed(&mut cache());
        let (speed, peak) = {
            let cache = cache();
            cache
                .as_ref()
                .map(|c| (c.speed, c.speed_peak))
                .unwrap_or((0.0, 0.0))
        };
        draw_widget_faded(ctx, painter, HudWidgetId::Speed, |painter| {
            crate::graphic::speedometer::draw(
                painter,
                widget_rendered_pos(ctx, HudWidgetId::Speed),
                speed,
                peak,
            )
        });
    }
    if widget_enabled(crate::module::ModuleId::Session) {
        draw_widget_faded(ctx, painter, HudWidgetId::Session, |painter| {
            draw_text_widget(
                ctx,
                painter,
                HudWidgetId::Session,
                vec![StatRow::new("Session", session_uptime())],
            )
        });
    }
    if widget_enabled(crate::module::ModuleId::Ping) {
        let mut cache = cache();
        let ping = refresh_ping(&mut cache);
        let history = cache
            .as_ref()
            .map(|c| c.ping_history.as_slice())
            .unwrap_or(&[]);
        draw_widget_faded(ctx, painter, HudWidgetId::Ping, |painter| {
            crate::graphic::ping::draw(
                painter,
                widget_rendered_pos(ctx, HudWidgetId::Ping),
                ping.map(|ms| ms.max(0) as u32),
                history,
            )
        });
    } else if let Some(cache) = cache().as_mut() {
         
         
        cache.ping = None;
        cache.ping_history.clear();
    }
    if widget_enabled(crate::module::ModuleId::SpotifyNowPlaying) {
        now_playing::ensure_poller();
        draw_widget_faded(ctx, painter, HudWidgetId::Spotify, |painter| {
            draw_spotify_widget(ctx, painter)
        });
    }
    if widget_enabled(crate::module::ModuleId::Lyrics)
        && !crate::graphic::lyrics_world::replaces_hud()
    {
        draw_widget_faded(ctx, painter, HudWidgetId::Lyrics, |painter| {
            crate::graphic::lyrics_card::draw_widget(ctx, painter)
        });
    }
    if widget_enabled(crate::module::ModuleId::ArmorStatus) {
        refresh_armor(&mut cache());
        let armor = cache()
            .as_ref()
            .map(|c| c.armor.clone())
            .unwrap_or_default();
        draw_widget_faded(ctx, painter, HudWidgetId::ArmorStatus, |painter| {
            draw_armor_status(ctx, painter, armor)
        });
    } else if let Some(state) = cache().as_mut() {
         
        state.armor_opened = false;
    }
    if widget_enabled(crate::module::ModuleId::StatusHud) {
        draw_widget_faded(ctx, painter, HudWidgetId::Status, |painter| {
            draw_status_card(ctx, painter)
        });
    }
    if widget_enabled(crate::module::ModuleId::TotemCounter) {
        refresh_totems(&mut cache());
         
         
        let now = ctx.input(|input| input.time);
        let (count, motion) = {
            let mut guard = cache();
            let entry = guard.get_or_insert_with(WidgetCache::default);
            let count = entry.totems;
            entry.totem_roll.advance(count, now);
            (count, TotemMotion::read(&entry.totem_roll, now))
        };
        draw_widget_faded(ctx, painter, HudWidgetId::TotemCounter, |painter| {
            draw_totem_counter(ctx, painter, count, motion)
        });
    }
    if widget_enabled(crate::module::ModuleId::PotionStatus) {
        refresh_effects(&mut cache());
        let dt = ctx.input(|input| input.stable_dt).clamp(0.0, 0.1);
        let (effects, motion) = {
            let mut guard = cache();
            let entry = guard.get_or_insert_with(WidgetCache::default);
            entry.effects_motion.advance(&entry.effects, dt);
            (entry.effects.clone(), entry.effects_motion)
        };
        let anchor = widget_rendered_pos(ctx, HudWidgetId::PotionStatus);
        draw_widget_faded(ctx, painter, HudWidgetId::PotionStatus, |painter| {
            draw_potion_card(painter, anchor, &effects, motion);
        });
    }
}

 
 
fn draw_widget_faded(
    ctx: &Context,
    painter: &Painter,
    id: HudWidgetId,
    draw: impl FnOnce(&Painter),
) {
    let pose = UiStyle::reveal_pose(ctx, id.index(), HudWidgetId::all().len());
    if !pose.is_visible() {
        return;
    }
    let mut faded = painter.clone();
    faded.multiply_opacity(pose.alpha);
    draw(&faded);
}

 
 
 
 
 
pub(crate) struct StatRow {
    label: String,
    value: String,
}

impl StatRow {
    fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

 
const STAT_GAP: f32 = 10.0;
 
const STAT_LABEL_ALPHA: f32 = 0.62;

 
 
fn draw_text_widget(ctx: &Context, painter: &Painter, id: HudWidgetId, rows: Vec<StatRow>) {
    if rows.is_empty() {
        return;
    }
    let origin = widget_rendered_pos(ctx, id);
    draw_text_widget_at(ctx, painter, origin, rows);
}

 
 
 
 
 
 
 
 
pub(crate) fn draw_text_widget_at(
    ctx: &Context,
    painter: &Painter,
    origin: Pos2,
    rows: Vec<StatRow>,
) {
    let style = UiStyle::hud_style();
    let font = FontId::proportional(style.text_size * style.scale);
    let line = style.line_height();
    let pad = style.padding * style.scale;
    let columns = stat_columns(ctx, &style, &font, &rows);
    let width = pad * 2.0 + columns.total(style.scale);
    let label_color = theme::with_alpha(style.text, STAT_LABEL_ALPHA);
    for (i, row) in rows.iter().enumerate() {
        let y = origin.y - i as f32 * line;
        let rect = Rect::from_min_size(egui::pos2(origin.x, y), Vec2::new(width, line));
        paint_panel(painter, rect, &style);
        let label = style.label(&row.label);
        let value = style.label(&row.value);
        let label_pos = egui::pos2(rect.min.x + pad, rect.center().y);
        let value_pos = egui::pos2(rect.max.x - pad, rect.center().y);
        if style.text_shadow {
            let shadow = Color32::from_black_alpha(150);
            painter.text(
                label_pos + egui::vec2(1.0, 1.0),
                Align2::LEFT_CENTER,
                &label,
                font.clone(),
                shadow,
            );
            painter.text(
                value_pos + egui::vec2(1.0, 1.0),
                Align2::RIGHT_CENTER,
                &value,
                font.clone(),
                shadow,
            );
        }
        painter.text(
            label_pos,
            Align2::LEFT_CENTER,
            label,
            font.clone(),
            label_color,
        );
        if !row.value.is_empty() {
            painter.text(
                value_pos,
                Align2::RIGHT_CENTER,
                value,
                font.clone(),
                style.text,
            );
        }
    }
}

 
 
 
 
fn stat_columns(ctx: &Context, style: &UiStyle, font: &FontId, rows: &[StatRow]) -> StatColumns {
    let measure = |text: &str| {
        ctx.fonts(|f| {
            f.layout_no_wrap(style.label(text), font.clone(), style.text)
                .size()
                .x
        })
    };
    StatColumns {
        label: rows
            .iter()
            .map(|row| measure(&row.label))
            .fold(0.0, f32::max),
        value: rows
            .iter()
            .map(|row| measure(&row.value))
            .fold(0.0, f32::max),
    }
}

 
struct StatColumns {
    label: f32,
    value: f32,
}

impl StatColumns {
     
     
     
    fn total(&self, scale: f32) -> f32 {
        if self.value <= 0.0 {
            self.label
        } else {
            self.label + STAT_GAP * scale + self.value
        }
    }
}

 
 
 
 
 
 
 
 
 
 
pub(crate) fn paint_panel(painter: &Painter, rect: Rect, style: &UiStyle) {
    let round = style.rounding(rect);
    let now = painter.ctx().input(|input| input.time) as f32;
    let pulse = if style.glow && style.glow_pulse {
        0.55 + 0.45 * ((now * style.pulse_speed * std::f32::consts::TAU).sin() * 0.5 + 0.5)
    } else {
        1.0
    };

    if style.shadow_alpha > 0.0 {
        painter.rect_filled(
            rect.translate(egui::vec2(0.0, style.shadow_offset)),
            round,
            Color32::from_black_alpha((style.shadow_alpha * 110.0).round() as u8),
        );
    }
    if style.glow && style.glow_strength > 0.0 {
        painter.rect_filled(
            rect.expand(2.0),
            style.rounding(rect.expand(4.0)),
            theme::with_alpha(style.accent, style.glow_strength * 0.10 * pulse),
        );
        painter.ctx().request_repaint();
    }

    let outline = rounded_outline(rect, round.nw);
    if style.background.a() > 0 {
         
         
         
        painter.rect_filled(rect, round, style.background);
        if style.panel_gradient {
            let opacity = style.background.a() as f32 / 255.0;
            let top = theme::lerp_color(theme::SURFACE, style.accent, style.gradient_strength);
            let bottom = theme::lerp_color(theme::SURFACE, Color32::BLACK, 0.45);
            paint_vertical_gradient(
                painter,
                rect,
                &outline,
                theme::with_alpha(top, opacity),
                theme::with_alpha(bottom, opacity),
            );
        }
    }
    if style.border_width > 0.0 && style.border_alpha > 0.0 {
        painter.rect_stroke(
            rect,
            round,
            Stroke::new(
                style.border_width,
                theme::with_alpha(theme::BORDER, style.border_alpha),
            ),
        );
    }
    if style.comets && style.comet_speed > 0.0 {
         
         
        let seed = (rect.left() * 0.013 + rect.top() * 0.017).rem_euclid(1.0);
        let head = (now * style.comet_speed * anim::speed() + seed).rem_euclid(1.0);
        paint_border_comet(
            painter,
            &outline,
            head,
            style.border_alpha.max(0.35),
            style.accent,
            style.comet_tail,
        );
        painter.ctx().request_repaint();
    }
}

 

 
 
#[derive(Clone, Copy, Debug)]
struct StatusConfig {
    name: bool,
    health: bool,
    armor: bool,
    food: bool,
    numbers: bool,
    width: f32,
    bar_height: f32,
}

impl Default for StatusConfig {
    fn default() -> Self {
        Self {
            name: true,
            health: true,
            armor: true,
            food: true,
            numbers: true,
            width: 148.0,
            bar_height: 6.0,
        }
    }
}

impl StatusConfig {
    fn read() -> Self {
        let stock = Self::default();
         
         
        let Some(client) = crate::state::try_client() else {
            return stock;
        };
        let Some(handle) = client.modules.get(crate::module::ModuleId::StatusHud) else {
            return stock;
        };
        let Ok(module) = handle.lock() else {
            return stock;
        };
        let data = module.get_module_data();
        let toggle = |name: &str, fallback: bool| {
            data.get_setting(name)
                .and_then(crate::module::ModuleSetting::get_toggle_value)
                .unwrap_or(fallback)
        };
        let slider = |name: &str, fallback: f32| {
            data.get_setting(name)
                .and_then(crate::module::ModuleSetting::get_slider_value)
                .filter(|value| value.is_finite())
                .unwrap_or(fallback)
        };
        Self {
            name: toggle("Name", stock.name),
            health: toggle("Health", stock.health),
            armor: toggle("Armor", stock.armor),
            food: toggle("Food", stock.food),
            numbers: toggle("Numbers", stock.numbers),
            width: slider("Width", stock.width).clamp(110.0, 240.0),
            bar_height: slider("Bar Height", stock.bar_height).clamp(4.0, 12.0),
        }
    }

    fn rows(&self) -> usize {
        self.health as usize + self.armor as usize + self.food as usize
    }
}

 
 
struct StatusBar {
    label: &'static str,
    fraction: f32,
    color: Color32,
    value: String,
}

 
 
fn bar_fraction(value: f32, max: f32) -> f32 {
    if !value.is_finite() || !max.is_finite() || max <= 0.0 {
        return 0.0;
    }
    (value / max).clamp(0.0, 1.0)
}

 
 
fn health_color(fraction: f32) -> Color32 {
    let f = fraction.clamp(0.0, 1.0);
    if f >= 0.5 {
        theme::lerp_color(theme::WARN, theme::SUCCESS, (f - 0.5) * 2.0)
    } else {
        theme::lerp_color(theme::DANGER, theme::WARN, f * 2.0)
    }
}

 
fn status_row_height(cfg: &StatusConfig, style: &UiStyle) -> f32 {
    (cfg.bar_height * style.scale).max(9.0 * style.scale) + 6.0 * style.scale
}

 
fn status_card_size(cfg: &StatusConfig, style: &UiStyle) -> Vec2 {
    let pad = style.padding * style.scale;
    let header = if cfg.name {
        style.text_size * style.scale * 1.15
    } else {
        0.0
    };
    Vec2::new(
        cfg.width * style.scale,
        pad * 2.0 + header + cfg.rows() as f32 * status_row_height(cfg, style),
    )
}

 
 
fn status_bars(
    cfg: &StatusConfig,
    health: f32,
    max_health: f32,
    armor: i32,
    food: i32,
) -> Vec<StatusBar> {
    let mut bars = Vec::new();
    if cfg.health {
        bars.push(StatusBar {
            label: "HP",
            fraction: bar_fraction(health, max_health),
            color: health_color(bar_fraction(health, max_health)),
            value: format!("{:.0}/{:.0}", health.max(0.0), max_health.max(1.0)),
        });
    }
    if cfg.armor {
        bars.push(StatusBar {
            label: "ARM",
            fraction: bar_fraction(armor as f32, 30.0),
            color: theme::category_accent(crate::module::ModuleCategory::Render),
            value: armor.to_string(),
        });
    }
    if cfg.food {
        bars.push(StatusBar {
            label: "FOOD",
            fraction: bar_fraction(food as f32, 20.0),
            color: theme::WARN,
            value: food.to_string(),
        });
    }
    bars
}

 
 
fn draw_status_card(ctx: &Context, painter: &Painter) {
    let Some(player) = crate::state::minecraft().player().ok().flatten() else {
        return;
    };
    let cfg = StatusConfig::read();
    let bars = status_bars(
        &cfg,
        player.get_health().unwrap_or(0.0),
        player.get_max_health().unwrap_or(20.0),
        player.get_armor_value().unwrap_or(0),
        player.food_level().unwrap_or(0),
    );
    let name = player
        .get_name()
        .and_then(|component| component.get_string())
        .unwrap_or_default();
    draw_status_card_at(
        ctx,
        painter,
        widget_rendered_pos(ctx, HudWidgetId::Status),
        &cfg,
        &name,
        &bars,
    );
}

 
 
fn draw_status_card_at(
    ctx: &Context,
    painter: &Painter,
    origin: Pos2,
    cfg: &StatusConfig,
    name: &str,
    bars: &[StatusBar],
) {
    let style = UiStyle::hud_style();
    let size = status_card_size(cfg, &style);
    let rect = Rect::from_min_size(origin, size);
    paint_panel(painter, rect, &style);

    let pad = style.padding * style.scale;
    let bar_h = (cfg.bar_height * style.scale).max(9.0 * style.scale);
    let mut y = rect.min.y + pad;

    if cfg.name && !name.is_empty() {
        let header = style.text_size * style.scale * 1.15;
        painter.circle_filled(
            egui::pos2(rect.min.x + pad + 2.0 * style.scale, y + header * 0.5),
            2.0 * style.scale,
            style.accent,
        );
        painter.text(
            egui::pos2(rect.min.x + pad + 7.0 * style.scale, y + header * 0.5),
            Align2::LEFT_CENTER,
            name,
            FontId::proportional(style.text_size * 0.92 * style.scale),
            style.text,
        );
        y += header;
    }

    let row_h = status_row_height(cfg, &style);
    let label_w = 26.0 * style.scale;
    let value_w = if cfg.numbers { 36.0 * style.scale } else { 0.0 };
    for (index, bar) in bars.iter().enumerate() {
        let centre = y + row_h * 0.5;
        painter.text(
            egui::pos2(rect.min.x + pad, centre),
            Align2::LEFT_CENTER,
            bar.label,
            FontId::proportional(8.5 * style.scale),
            theme::with_alpha(style.text, 0.62),
        );
        let track = Rect::from_min_max(
            egui::pos2(rect.min.x + pad + label_w, centre - bar_h * 0.5),
            egui::pos2(rect.max.x - pad - value_w, centre + bar_h * 0.5),
        );
        painter.rect_filled(
            track,
            Rounding::same(bar_h * 0.5),
            theme::with_alpha(theme::SURFACE_HOVER, 0.9),
        );
        let fill = anim::ease_to(
            ctx,
            Id::new("status_fill").with(index),
            bar.fraction,
            0.22,
            anim::Easing::Out,
        );
        if fill > 0.001 {
            let filled = Rect::from_min_size(
                track.min,
                Vec2::new(track.width() * fill.clamp(0.0, 1.0), track.height()),
            );
            painter.rect_filled(filled, Rounding::same(bar_h * 0.5), bar.color);
             
             
            if filled.width() > bar_h {
                painter.circle_filled(
                    egui::pos2(filled.max.x - bar_h * 0.5, filled.center().y),
                    bar_h * 0.5,
                    theme::lerp_color(bar.color, Color32::WHITE, 0.35),
                );
            }
            ctx.request_repaint();
        }
        if cfg.numbers {
            painter.text(
                egui::pos2(rect.max.x - pad, centre),
                Align2::RIGHT_CENTER,
                &bar.value,
                FontId::proportional(9.5 * style.scale),
                theme::with_alpha(style.text, 0.9),
            );
        }
        y += row_h;
    }
}

 
 
fn draw_status_preview(ctx: &Context, painter: &Painter, origin: Pos2) {
    let cfg = StatusConfig::read();
    let bars = status_bars(&cfg, 18.0, 20.0, 15, 17);
    draw_status_card_at(ctx, painter, origin, &cfg, "Player", &bars);
}

 

 
const ARMOR_W: f32 = 176.0;
 
const ARMOR_HEADER: f32 = 19.0;
 
const ARMOR_ROW: f32 = 25.0;
const ARMOR_PAD: f32 = 7.0;
 
 
const ARMOR_ART: f32 = 16.0;
 
const ARMOR_BAR: f32 = 2.5;
 
 
const ARMOR_EASE: f32 = 9.0;
 
const ARMOR_CRITICAL: f32 = 0.2;
 
const ARMOR_APPEAR: f32 = 0.26;
const ARMOR_STAGGER: f32 = 0.05;

 
fn armor_height() -> f32 {
    ARMOR_HEADER + ARMOR_ROW * 4.0 + ARMOR_PAD
}

 
 
 
#[derive(Clone, Copy, Debug)]
struct ArmorMotion {
    shown: [f32; 4],
    age: f32,
}

impl Default for ArmorMotion {
    fn default() -> Self {
        Self::fresh()
    }
}

impl ArmorMotion {
     
    fn fresh() -> Self {
        Self {
            shown: [0.0; 4],
            age: 0.0,
        }
    }

     
     
    fn settled(pieces: &[Option<ArmorPiece>; 4]) -> Self {
        let mut shown = [0.0; 4];
        for (slot, piece) in pieces.iter().enumerate() {
            shown[slot] = piece
                .as_ref()
                .map_or(0.0, |piece| piece.durability.clamp(0.0, 1.0));
        }
        Self {
            shown,
            age: ARMOR_APPEAR * 8.0,
        }
    }

     
     
    fn advance(&mut self, pieces: &[Option<ArmorPiece>; 4], dt: f32) {
        let k = 1.0 - (-dt * ARMOR_EASE).exp();
        for slot in 0..4 {
            let target = pieces[slot]
                .as_ref()
                .map_or(0.0, |piece| piece.durability.clamp(0.0, 1.0));
            self.shown[slot] += (target - self.shown[slot]) * k;
        }
        self.age += dt;
    }

     
     
     
    fn intro(&self, row: usize) -> f32 {
        let t = ((self.age - row as f32 * ARMOR_STAGGER) / ARMOR_APPEAR).clamp(0.0, 1.0);
        anim::Easing::OutBack.apply(t)
    }
}

 
 
#[derive(Clone, Debug, PartialEq)]
struct ArmorPiece {
     
    name: String,
     
    id: String,
     
    durability: f32,
}

 
fn draw_armor_status(ctx: &Context, painter: &Painter, pieces: [Option<ArmorPiece>; 4]) {
     
     
    let dt = ctx.input(|input| input.stable_dt).clamp(0.0, 0.25);
    let motion = {
        let mut guard = cache();
        let state = guard.get_or_insert_with(WidgetCache::default);
        if pieces.iter().all(Option::is_none) {
             
             
            state.armor_opened = false;
            state.armor_motion = ArmorMotion::settled(&pieces);
            return;
        }
        if !state.armor_opened {
            state.armor_opened = true;
            state.armor_motion = ArmorMotion::fresh();
        }
        state.armor_motion.advance(&pieces, dt);
        state.armor_motion
    };
    draw_armor_card_at(
        ctx,
        painter,
        widget_rendered_pos(ctx, HudWidgetId::ArmorStatus),
        &pieces,
        &motion,
    );
}

 
 
fn draw_armor_preview(ctx: &Context, painter: &Painter, origin: Pos2) {
    let piece = |id: &str, durability: f32| {
        Some(ArmorPiece {
            name: title_case(id),
            id: id.to_string(),
            durability,
        })
    };
     
    let pieces = [
        piece("diamond_boots", 0.47),
        piece("diamond_leggings", 0.64),
        piece("diamond_chestplate", 0.82),
        piece("diamond_helmet", 1.0),
    ];
    draw_armor_card_at(
        ctx,
        painter,
        origin,
        &pieces,
        &ArmorMotion::settled(&pieces),
    );
}

 
 
fn draw_armor_card_at(
    ctx: &Context,
    painter: &Painter,
    origin: Pos2,
    pieces: &[Option<ArmorPiece>; 4],
    motion: &ArmorMotion,
) {
    let style = UiStyle::hud_style();
    let s = style.scale;
    let now = ctx.input(|input| input.time) as f32;
    let rect = Rect::from_min_size(origin, Vec2::new(ARMOR_W, armor_height()) * s);
    paint_panel(painter, rect, &style);

    let pad = ARMOR_PAD * s;
    let worn = pieces.iter().filter(|piece| piece.is_some()).count();
    let header = rect.top() + ARMOR_HEADER * 0.5 * s;
    let label = FontId::proportional((7.5 * s).max(6.5));
    painter.text(
        egui::pos2(rect.left() + pad, header),
        Align2::LEFT_CENTER,
        "ARMOR",
        label.clone(),
        theme::with_alpha(theme::TEXT_DIM, 0.9),
    );
    painter.text(
        egui::pos2(rect.right() - pad, header),
        Align2::RIGHT_CENTER,
        format!("{worn}/4"),
        FontId::monospace((7.5 * s).max(6.5)),
        theme::with_alpha(
            if worn == 4 {
                style.accent
            } else {
                theme::TEXT_MUTED
            },
            0.95,
        ),
    );
    let rule = rect.top() + (ARMOR_HEADER - 3.0) * s;
    painter.line_segment(
        [
            egui::pos2(rect.left() + pad, rule),
            egui::pos2(rect.right() - pad, rule),
        ],
        Stroke::new(1.0 * s, theme::with_alpha(theme::TEXT, 0.06)),
    );

    for row in 0..4 {
         
        let slot = 3 - row;
        let intro = motion.intro(row);
        let alpha = intro.clamp(0.0, 1.0);
        if alpha <= 0.01 {
            continue;
        }
        let top = rect.top() + (ARMOR_HEADER + ARMOR_ROW * row as f32) * s;
        let row_rect = Rect::from_min_size(
            egui::pos2(rect.left() + pad, top + (1.0 - alpha) * 8.0 * s),
            Vec2::new(rect.width() - pad * 2.0, ARMOR_ROW * s),
        );
        armor_row(
            ctx,
            painter,
            row_rect,
            slot,
            pieces[slot].as_ref(),
            motion.shown[slot],
            alpha,
            &style,
            now,
        );
    }
}

 
 
 
#[allow(clippy::too_many_arguments)]
fn armor_row(
    ctx: &Context,
    painter: &Painter,
    row: Rect,
    slot: usize,
    piece: Option<&ArmorPiece>,
    shown: f32,
    alpha: f32,
    style: &UiStyle,
    now: f32,
) {
    let s = style.scale;
    let fade = |color: Color32, factor: f32| theme::with_alpha(color, factor * alpha);
     
     
    let bob = ((now * 1.6 + slot as f32 * 0.7).sin() * 0.5).round() * s * alpha;
    let art = Rect::from_center_size(
        egui::pos2(row.left() + ARMOR_ART * 0.5 * s, row.center().y + bob),
        Vec2::splat(ARMOR_ART * s),
    );
    let text_left = art.right() + 6.0 * s;
    let line = row.top() + 2.5 * s;
    let bar_top = row.top() + 16.0 * s;
    let track = Rect::from_min_size(
        egui::pos2(text_left, bar_top),
        Vec2::new((row.right() - text_left).max(1.0), ARMOR_BAR * s),
    );
    let radius = Rounding::same(ARMOR_BAR * 0.5 * s);
    let name_font = FontId::proportional((9.0 * s).max(6.5));
    let value_font = FontId::monospace((8.5 * s).max(6.5));

    let Some(piece) = piece else {
         
         
        painter.rect_filled(art, Rounding::same(3.0 * s), fade(theme::ELEVATED, 0.5));
        painter.text(
            art.center(),
            Align2::CENTER_CENTER,
            "–",
            FontId::proportional((9.0 * s).max(6.5)),
            fade(theme::TEXT_MUTED, 0.7),
        );
        painter.text(
            egui::pos2(text_left, line),
            Align2::LEFT_TOP,
            "Empty",
            name_font,
            fade(theme::TEXT_MUTED, 0.8),
        );
        painter.rect_filled(track, radius, fade(theme::TEXT, 0.04));
        return;
    };

    let shown = shown.clamp(0.0, 1.0);
    let left = piece.durability.clamp(0.0, 1.0);
    let critical = left <= ARMOR_CRITICAL;
    let pulse = if critical {
        0.55 + 0.45 * (now * 6.0).sin().mul_add(0.5, 0.5)
    } else {
        1.0
    };
     
     
    let damage = (shown - left).max(0.0);
     
     
    let colour = if critical {
        theme::DANGER
    } else {
        theme::lerp_color(style.accent, theme::DANGER, damage * 6.0)
    };

     
     
     
     
    if critical {
        painter.circle_filled(
            art.center(),
            art.width() * 0.78,
            fade(theme::DANGER, 0.12 * pulse),
        );
    }
    painter.rect_filled(
        art.expand(1.5 * s),
        Rounding::same(4.0 * s),
        fade(theme::ELEVATED, if critical { 0.35 } else { 0.55 }),
    );
    match crate::graphic::item_icon::icon(ctx, &piece.id) {
        Some((texture, _)) => crate::graphic::item_icon::paint(painter, art, texture, alpha),
        None => {
             
             
            let initial: String = piece
                .id
                .chars()
                .next()
                .map(|c| c.to_ascii_uppercase())
                .into_iter()
                .collect();
            painter.rect_filled(art, Rounding::same(3.0 * s), fade(theme::ELEVATED, 0.8));
            painter.text(
                art.center(),
                Align2::CENTER_CENTER,
                initial,
                FontId::proportional((9.0 * s).max(6.5)),
                fade(theme::TEXT_DIM, 0.9),
            );
        }
    }

     
    painter.text(
        egui::pos2(text_left, line),
        Align2::LEFT_TOP,
        &piece.name,
        name_font,
        fade(
            theme::lerp_color(theme::TEXT, theme::DANGER, damage * 2.5),
            0.92,
        ),
    );
     
    painter.text(
        egui::pos2(row.right(), line),
        Align2::RIGHT_TOP,
        format!("{:.0}%", shown.clamp(0.0, 1.0) * 100.0),
        value_font,
        if critical {
            fade(theme::DANGER, pulse)
        } else {
            fade(
                theme::lerp_color(theme::TEXT, theme::DANGER, damage * 2.0),
                0.9,
            )
        },
    );

     
    painter.rect_filled(track, radius, fade(theme::TEXT, 0.07));
    if damage > 0.001 {
         
        let ghost = Rect::from_min_size(
            egui::pos2(track.left() + track.width() * left, track.top()),
            Vec2::new(track.width() * damage, track.height()),
        );
        painter.rect_filled(ghost, radius, fade(theme::DANGER, 0.5));
    }
    if shown > 0.001 {
        let filled = Rect::from_min_size(
            track.min,
            Vec2::new(track.width() * shown.clamp(0.0, 1.0), track.height()),
        );
        painter.rect_filled(filled, radius, fade(colour, 0.92));
         
         
        if filled.width() > 1.5 * s {
            let cap = Rect::from_min_size(
                egui::pos2(filled.right() - 1.5 * s, track.top() - 0.5 * s),
                Vec2::new(1.5 * s, track.height() + s),
            );
            painter.rect_filled(
                cap,
                Rounding::same(0.75 * s),
                fade(
                    theme::lerp_color(colour, Color32::WHITE, 0.65),
                    0.95 * pulse,
                ),
            );
        }
    }
}

 

 
 
 
const TOTEM_W: f32 = 74.0;
const TOTEM_H: f32 = 40.0;
 
 
const TOTEM_ART: f32 = 32.0;
 
 
const TOTEM_PAD: f32 = 4.0;
 
const TOTEM_TEXT_GAP: f32 = 6.0;
 
const TOTEM_GAP: f32 = 4.0;
 
 
const TOTEM_FONT: f32 = 17.0;
const TOTEM_NUMBER_Y: f32 = -8.0;
const TOTEM_CAPTION: f32 = 6.5;
const TOTEM_CAPTION_Y: f32 = 12.0;
 
const TOTEM_POP: f32 = 0.14;
 
const TOTEM_ITEM: &str = "totem_of_undying";
 
const TOTEM_PREVIEW_COUNT: i32 = 12;
 
const TOTEM_ROLL: f64 = 0.42;
 
 
const TOTEM_REFRESH: std::time::Duration = std::time::Duration::from_millis(200);

 
 
 
 
 
 
const TOTEM_PNG: &[u8] = include_bytes!("../../assets/totem/totem_of_undying.png");

 
 
 
 
fn totem_counter_pos(screen: &Rect) -> Pos2 {
    let style = UiStyle::hud_style();
    let size = Vec2::new(TOTEM_W, TOTEM_H) * style.scale;
     
     
    let fallback = egui::pos2(
        screen.width() * 0.5 - 158.0,
        screen.height() - 17.0 - size.y,
    );
    let Some(slot) = hotbar_first_slot() else {
        return fallback;
    };
    let anchor = egui::pos2(
        slot.min.x - TOTEM_GAP - size.x,
        slot.center().y - size.y * 0.5,
    );
     
     
    if anchor.x < MARGIN {
        fallback
    } else {
        anchor
    }
}

 
 
fn module_toggle(id: crate::module::ModuleId, name: &str, default: bool) -> bool {
    crate::state::client()
        .modules
        .get(id)
        .and_then(|handle| {
            let module = handle.lock().ok()?;
            let setting = module.get_module_data().get_setting(name)?;
            setting.get_toggle_value()
        })
        .unwrap_or(default)
}

 
 
 
fn totem_slots() -> impl Iterator<Item = i32> {
    (0..36).chain(std::iter::once(Inventory::SLOT_OFFHAND))
}

 
 
 
fn count_totems(inventory: &Inventory) -> Option<i32> {
    let totem = inventory::totem_of_undying().ok()?;
    let mut total = 0;
    for slot in totem_slots() {
        let Ok(stack) = inventory.get_item(slot) else {
            continue;
        };
        if stack.is_empty().unwrap_or(true) {
            continue;
        }
        let Ok(item) = stack.get_item() else {
            continue;
        };
        if item.is_same(&totem) {
            total += stack.count().unwrap_or(0);
        }
    }
    Some(total)
}

 
 
fn refresh_totems(cache: &mut Option<WidgetCache>) {
    let cache = cache.get_or_insert_with(WidgetCache::default);
    if cache
        .last_totems
        .is_some_and(|at| at.elapsed() < TOTEM_REFRESH)
    {
        return;
    }
    cache.last_totems = Some(std::time::Instant::now());
    let Some(player) = crate::state::minecraft().player().ok().flatten() else {
        cache.totems = 0;
        return;
    };
     
     
    let Ok(inventory) = player.get_inventory() else {
        return;
    };
    if let Some(total) = count_totems(&inventory) {
        cache.totems = total;
    }
}

 
 
#[derive(Default, Clone, Copy)]
struct TotemRoll {
     
     
    shown: f32,
     
    from: f32,
     
    target: i32,
     
    started: f64,
     
     
    loss: bool,
}

impl TotemRoll {
     
    fn advance(&mut self, count: i32, now: f64) {
        if count != self.target {
            self.from = self.shown;
            self.target = count;
            self.started = now;
            self.loss = count < self.from.round() as i32;
        }
        let t = ((now - self.started) / TOTEM_ROLL).clamp(0.0, 1.0) as f32;
        self.shown = self.from + (self.target as f32 - self.from) * Easing::Out.apply(t);
    }
}

 
 
 
#[derive(Clone, Copy)]
struct TotemMotion {
    value: f32,
    t: f32,
    loss: bool,
}

impl TotemMotion {
     
     
    fn settled(count: i32) -> Self {
        Self {
            value: count as f32,
            t: 1.0,
            loss: false,
        }
    }

     
    fn read(roll: &TotemRoll, now: f64) -> Self {
        Self {
            value: roll.shown,
            t: ((now - roll.started) / TOTEM_ROLL).clamp(0.0, 1.0) as f32,
            loss: roll.loss,
        }
    }

     
    fn live(&self) -> f32 {
        1.0 - self.t
    }

     
     
    fn pop(&self) -> f32 {
        self.live() * self.live()
    }
}

 
 
 
fn totem_art(ctx: &Context) -> Option<egui::TextureId> {
    if let Some((texture, _)) = crate::graphic::item_icon::icon(ctx, TOTEM_ITEM) {
        return Some(texture);
    }
    bundled_totem_art(ctx)
}

 
 
 
fn bundled_totem_art(ctx: &Context) -> Option<egui::TextureId> {
    let id = Id::new("infered_totem_counter_art");
    if let Some(handle) = ctx.data(|data| data.get_temp::<TextureHandle>(id)) {
        return Some(handle.id());
    }
    let image = image::load_from_memory(TOTEM_PNG).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    let handle = ctx.load_texture(
        "totem_counter_art",
        egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], image.as_raw()),
        egui::TextureOptions::NEAREST,
    );
    ctx.data_mut(|data| data.insert_temp(id, handle.clone()));
    Some(handle.id())
}

 
 
fn draw_totem_counter(ctx: &Context, painter: &Painter, count: i32, motion: TotemMotion) {
    if count <= 0
        && !module_toggle(
            crate::module::ModuleId::TotemCounter,
            "Show When Empty",
            true,
        )
    {
        return;
    }
    let origin = widget_rendered_pos(ctx, HudWidgetId::TotemCounter);
    let art = totem_art(ctx);
    draw_totem_counter_at(painter, origin, count, art, motion);
}

 
 
 
 
fn draw_totem_counter_at(
    painter: &Painter,
    origin: Pos2,
    count: i32,
    art: Option<egui::TextureId>,
    motion: TotemMotion,
) {
    let style = UiStyle::hud_style();
    let s = style.scale;
    let now = painter.ctx().input(|input| input.time) as f32;
    let rect = Rect::from_min_size(origin, Vec2::new(TOTEM_W, TOTEM_H) * s);
    let empty = count <= 0;
     
     
     
    let mood = if empty { theme::DANGER } else { style.accent };
    let pop = motion.pop();

    paint_panel(painter, rect, &style);

     
    let art_side = TOTEM_ART * s;
    let plate = Rect::from_center_size(
        egui::pos2(rect.min.x + TOTEM_PAD * s + art_side * 0.5, rect.center().y),
        Vec2::splat(art_side),
    );
    let breathe = 0.5 + 0.5 * (now * 2.1).sin();
     
     
     
    painter.circle_filled(
        plate.center(),
        art_side * 0.70,
        theme::with_alpha(mood, 0.05 + 0.04 * breathe + 0.13 * pop),
    );
    painter.rect_filled(
        plate,
        Rounding::same(4.0 * s),
        theme::with_alpha(theme::ELEVATED, 0.55),
    );
     
     
    let bob = ((now * 2.2).sin() * 0.5).round() * s;
    let art_rect = Rect::from_center_size(
        plate.center() + egui::vec2(0.0, bob),
        Vec2::splat(art_side * (1.0 + TOTEM_POP * pop)),
    );
    match art {
        Some(texture) => {
            crate::graphic::item_icon::paint(
                painter,
                art_rect,
                texture,
                if empty { 0.55 } else { 1.0 },
            );
        }
        None => {
            painter.text(
                art_rect.center(),
                Align2::CENTER_CENTER,
                "T",
                FontId::proportional(9.0 * s),
                style.accent,
            );
        }
    }

     
     
     
     
    let text_x = plate.max.x + TOTEM_TEXT_GAP * s;
    let number = motion.value.round().max(0.0) as i32;
    let rest = totem_count_color(count, &style);
    let flash = if motion.loss {
        theme::DANGER
    } else {
        style.accent
    };
    painter.text(
        egui::pos2(text_x, rect.center().y + TOTEM_NUMBER_Y * s),
        Align2::LEFT_CENTER,
        number.to_string(),
        FontId::proportional((TOTEM_FONT * s * (1.0 + TOTEM_POP * pop)).max(8.0)),
        theme::lerp_color(rest, flash, 0.85 * pop),
    );

     
     
     
    let caption_y = rect.center().y + TOTEM_CAPTION_Y * s;
    let caption = |text: &str, color: Color32, alpha: f32| {
        if alpha <= 0.01 {
            return;
        }
        painter.text(
            egui::pos2(text_x, caption_y),
            Align2::LEFT_CENTER,
            text,
            FontId::proportional((TOTEM_CAPTION * s).max(6.0)),
            theme::with_alpha(color, alpha),
        );
    };
    if empty && motion.t < 1.0 {
         
         
        let out = ((0.5 - motion.t) / 0.5).clamp(0.0, 1.0);
        let in_ = ((motion.t - 0.5) / 0.5).clamp(0.0, 1.0);
        caption("TOTEMS", theme::TEXT_DIM, 0.85 * out);
        caption("EMPTY", theme::DANGER, in_);
    } else if empty {
        caption("EMPTY", theme::DANGER, 1.0);
    } else {
        caption("TOTEMS", theme::TEXT_DIM, 0.85);
    }
}

 
 
fn totem_count_color(count: i32, style: &UiStyle) -> Color32 {
    if count <= 0 {
        theme::DANGER
    } else {
        style.text
    }
}

 

 
const SPOTIFY_WIDTH: f32 = 272.0;
 
const SPOTIFY_ART: f32 = 46.0;
 
const SPOTIFY_GREEN: Color32 = Color32::from_rgb(0x1D, 0xB9, 0x54);
 
const SPOTIFY_TITLE_LINE: f32 = 1.22;
const SPOTIFY_ARTIST_LINE: f32 = 1.3;
 
const SPOTIFY_ART_GAP: f32 = 10.0;
 
const SPOTIFY_BAR_GAP: f32 = 9.0;
 
const SPOTIFY_MIN_CONTENT: f32 = 14.0;

 
#[derive(Clone, Copy)]
struct SpotifyStyle {
     
    scale: f32,
    width: f32,
    radius: f32,
    opacity: f32,
    padding: f32,
    art_size: f32,
    art_radius: f32,
    title_size: f32,
    artist_size: f32,
    progress_height: f32,
    accent: Color32,
    text: Color32,
    show_background: bool,
    show_border: bool,
    show_title: bool,
    show_art: bool,
    show_artist: bool,
    show_progress: bool,
    show_time: bool,
}

impl Default for SpotifyStyle {
    fn default() -> Self {
        Self {
            scale: 1.0,
            width: SPOTIFY_WIDTH,
            radius: 8.0,
            opacity: 0.62,
            padding: 10.0,
            art_size: SPOTIFY_ART,
            art_radius: 6.0,
            title_size: 13.5,
            artist_size: 10.5,
            progress_height: 3.0,
            accent: SPOTIFY_GREEN,
            text: theme::TEXT,
            show_background: true,
            show_border: true,
            show_title: true,
            show_art: true,
            show_artist: true,
            show_progress: true,
            show_time: true,
        }
    }
}

impl SpotifyStyle {
     
     
    fn read() -> Self {
        let fallback = Self::default();
        let Some(handle) = crate::state::client()
            .modules
            .get(crate::module::ModuleId::SpotifyNowPlaying)
        else {
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
            Some(crate::module::ModuleSetting::Color { value, .. }) => {
                let channel = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                Color32::from_rgb(channel(value[0]), channel(value[1]), channel(value[2]))
            }
            _ => default,
        };
        Self {
            scale: slider("Scale", 1.0).clamp(0.6, 2.0),
            width: slider("Width", SPOTIFY_WIDTH).clamp(160.0, 420.0),
            radius: slider("Corner Radius", 8.0).clamp(0.0, 24.0),
            opacity: slider("Background Opacity", 0.62).clamp(0.0, 1.0),
            padding: slider("Padding", 10.0).clamp(0.0, 20.0),
            art_size: slider("Art Size", SPOTIFY_ART).clamp(24.0, 72.0),
            art_radius: slider("Art Corner Radius", 6.0).clamp(0.0, 36.0),
            title_size: slider("Title Size", 13.5).clamp(8.0, 22.0),
            artist_size: slider("Artist Size", 10.5).clamp(7.0, 18.0),
            progress_height: slider("Progress Height", 3.0).clamp(1.0, 8.0),
            accent: color("Accent Color", SPOTIFY_GREEN),
            text: color("Text Color", theme::TEXT),
            show_background: toggle("Show Background", true),
            show_border: toggle("Show Border", true),
            show_title: toggle("Show Title", true),
            show_art: toggle("Show Album Art", true),
            show_artist: toggle("Show Artist", true),
            show_progress: toggle("Show Progress Bar", true),
            show_time: toggle("Show Time", true),
        }
    }
}

 
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SpotifyClockRow {
    Title,
    Artist,
    None,
}

 
struct SpotifyRows {
    title_y: f32,
    artist_y: f32,
     
    time_y: f32,
     
    bar_y: f32,
     
    content: f32,
     
    height: f32,
    clock_row: SpotifyClockRow,
}

 
 
fn spotify_rows(style: &SpotifyStyle) -> SpotifyRows {
    let pad = style.padding;
    let title_line = if style.show_title {
        style.title_size * SPOTIFY_TITLE_LINE
    } else {
        0.0
    };
    let artist_line = if style.show_artist {
        style.artist_size * SPOTIFY_ARTIST_LINE
    } else {
        0.0
    };
    let title_gap = if style.show_title && style.show_artist {
        1.0
    } else {
        0.0
    };
    let bar_height = if style.show_progress {
        style.progress_height
    } else {
        0.0
    };
    let bar_gap = if style.show_progress && (style.show_title || style.show_artist) {
        SPOTIFY_BAR_GAP
    } else {
        0.0
    };
    let text = title_line + title_gap + artist_line + bar_gap + bar_height;
    let art = if style.show_art { style.art_size } else { 0.0 };
    let content = text.max(art).max(SPOTIFY_MIN_CONTENT);

    let title_y = pad;
    let artist_y = title_y + title_line + title_gap;
    let time_y = if style.show_artist {
        artist_y + artist_line * 0.5
    } else if style.show_title {
        title_y + title_line * 0.5
    } else {
        pad + content * 0.5
    };
    let clock_row = if !style.show_time {
        SpotifyClockRow::None
    } else if style.show_artist {
        SpotifyClockRow::Artist
    } else if style.show_title {
        SpotifyClockRow::Title
    } else {
        SpotifyClockRow::None
    };

    SpotifyRows {
        title_y,
        artist_y,
        time_y,
        bar_y: pad + content - bar_height,
        content,
        height: content + pad * 2.0,
        clock_row,
    }
}

 
pub(crate) fn dimmed(color: Color32, factor: f32) -> Color32 {
    let alpha = (color.a() as f32 * factor).round() as u8;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

 
fn spotify_card_size(style: &SpotifyStyle) -> Vec2 {
    Vec2::new(style.width, spotify_rows(style).height) * style.scale
}

 
 
 
static SPOTIFY_ART_TEX: std::sync::Mutex<Option<(u64, Option<TextureHandle>)>> =
    std::sync::Mutex::new(None);

 
const SPOTIFY_ENTER_SECS: f32 = 0.55;
 
const SPOTIFY_MARQUEE_SPEED: f32 = 22.0;
 
const SPOTIFY_MARQUEE_PAUSE: f32 = 2.0;
 
const SPOTIFY_IDLE_ALPHA: f32 = 0.55;
 
const SPOTIFY_PROGRESS_SPRING: anim::SpringCfg = anim::SpringCfg {
    stiffness: 180.0,
    damping: 27.0,
};

 
static SPOTIFY_TRACK: std::sync::Mutex<Option<(u64, f64)>> = std::sync::Mutex::new(None);

 
#[derive(Clone, Copy)]
struct SpotifyMotion {
     
    enter: f32,
     
    art_pop: f32,
     
    alpha: f32,
     
    playing: f32,
     
    progress: f32,
     
    since_change: f32,
}

impl SpotifyMotion {
     
    fn settled(progress: f32) -> Self {
        Self {
            enter: 1.0,
            art_pop: 1.0,
            alpha: 1.0,
            playing: 1.0,
            progress,
            since_change: 1000.0,
        }
    }
}

 
fn spotify_enter(since: f32) -> f32 {
    Easing::Out.apply(since / SPOTIFY_ENTER_SECS)
}

 
fn spotify_art_pop(since: f32) -> f32 {
    Easing::OutBack.apply(since / (SPOTIFY_ENTER_SECS * 1.3))
}

 
fn track_fraction(elapsed: std::time::Duration, duration: std::time::Duration) -> f32 {
    if duration.is_zero() {
        0.0
    } else {
        (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
    }
}

 
fn spotify_clock(elapsed: std::time::Duration, duration: std::time::Duration) -> String {
    let total = if duration.is_zero() {
        "--:--".to_string()
    } else {
        now_playing::format_clock(duration)
    };
    format!("{} / {}", now_playing::format_clock(elapsed), total)
}

 
 
fn marquee_offset(overflow: f32, elapsed: f32, speed: f32) -> f32 {
    if overflow <= 0.0 || speed <= 0.0 {
        return 0.0;
    }
    let rest = SPOTIFY_MARQUEE_PAUSE;
    let scroll = overflow / speed;
    let period = 2.0 * (rest + scroll);
    let t = elapsed.max(0.0).rem_euclid(period);
    if t < rest {
        0.0
    } else if t < rest + scroll {
        overflow * (t - rest) / scroll
    } else if t < 2.0 * rest + scroll {
        overflow
    } else {
        overflow * (1.0 - (t - 2.0 * rest - scroll) / scroll)
    }
}

 
 
fn track_first_seen(generation: u64, now: f64) -> f64 {
    let mut track = SPOTIFY_TRACK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match *track {
        Some((seen, at)) if seen == generation => at,
        _ => {
            *track = Some((generation, now));
            now
        }
    }
}

 
 
struct SpotifyCard {
    title: String,
    artist: String,
    elapsed: std::time::Duration,
    duration: std::time::Duration,
    art: Option<egui::TextureId>,
    motion: SpotifyMotion,
}

 
fn spotify_art_texture(ctx: &Context, playback: &now_playing::Playback) -> Option<egui::TextureId> {
    let bytes = playback.art.as_ref()?;
    let mut cache = SPOTIFY_ART_TEX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if cache.as_ref().map(|(generation, _)| *generation) != Some(playback.art_generation) {
        let texture = image::load_from_memory(&bytes[..]).ok().map(|image| {
            let size = [image.width() as usize, image.height() as usize];
            let rgba = image.to_rgba8();
            let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            ctx.load_texture("infered_spotify_art", color, egui::TextureOptions::LINEAR)
        });
        *cache = Some((playback.art_generation, texture));
    }
    cache
        .as_ref()
        .and_then(|(_, texture)| texture.as_ref().map(|handle| handle.id()))
}

 
pub(crate) fn text_width(ctx: &Context, text: &str, font: &FontId) -> f32 {
    ctx.fonts(|f| {
        f.layout_no_wrap(text.to_string(), font.clone(), theme::TEXT)
            .size()
            .x
    })
}

 
pub(crate) fn fit_text(ctx: &Context, text: &str, font: &FontId, max_width: f32) -> String {
    if text_width(ctx, text, font) <= max_width {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while chars.pop().is_some() {
        let candidate: String = chars.iter().collect::<String>() + "…";
        if text_width(ctx, &candidate, font) <= max_width {
            return candidate;
        }
    }
    "…".to_string()
}

 
 
 
 
fn rounded_image_mesh(rect: Rect, radius: f32, tint: Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return mesh;
    }
    let outline = rounded_outline(rect, radius);
    if outline.len() < 3 {
        return mesh;
    }
    let uv = |point: Pos2| {
        egui::pos2(
            ((point.x - rect.min.x) / rect.width()).clamp(0.0, 1.0),
            ((point.y - rect.min.y) / rect.height()).clamp(0.0, 1.0),
        )
    };
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.center(),
        uv: uv(rect.center()),
        color: tint,
    });
    for point in &outline {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: *point,
            uv: uv(*point),
            color: tint,
        });
    }
    let count = outline.len() as u32;
    for index in 0..count {
        mesh.add_triangle(0, 1 + index, 1 + (index + 1) % count);
    }
    mesh
}

 
fn paint_rounded_image(
    painter: &Painter,
    texture: egui::TextureId,
    rect: Rect,
    radius: f32,
    tint: Color32,
) {
    let mut mesh = rounded_image_mesh(rect, radius, tint);
    if mesh.is_empty() {
        return;
    }
    mesh.texture_id = texture;
    painter.add(egui::Shape::mesh(mesh));
}

 
 
fn draw_spotify_card(
    ctx: &Context,
    painter: &Painter,
    origin: Pos2,
    card: &SpotifyCard,
    style: &SpotifyStyle,
) {
    let s = style.scale;
    let rows = spotify_rows(style);
    let m = card.motion;
    let rect = Rect::from_min_size(origin, spotify_card_size(style));
    let round = Rounding::same((style.radius * s).min(rect.height() * 0.5));

     
     
    if style.show_background {
        painter.rect_filled(
            rect.translate(egui::vec2(0.0, 2.0 * s)),
            round,
            Color32::from_black_alpha(
                (55.0 * m.alpha * style.opacity).round().clamp(0.0, 255.0) as u8
            ),
        );
        let outline = rounded_outline(rect, style.radius * s);
        let top = theme::lerp_color(theme::SURFACE, style.accent, 0.10);
        let bottom = theme::lerp_color(theme::SURFACE, Color32::BLACK, 0.55);
        let fade = style.opacity * m.alpha;
        paint_vertical_gradient(
            painter,
            rect,
            &outline,
            theme::with_alpha(top, fade),
            theme::with_alpha(bottom, fade),
        );
    }
    if style.show_border {
        painter.rect_stroke(
            rect,
            round,
            Stroke::new(1.0_f32, theme::with_alpha(theme::BORDER, m.alpha)),
        );
    }

    let pad = style.padding * s;
    let text_right = rect.max.x - pad;
    let mut text_x = rect.min.x + pad;
    if style.show_art {
        let art_size = style.art_size * s;
        let slot = Rect::from_min_size(rect.min + Vec2::splat(pad), Vec2::splat(art_size));
         
        let art_rect = Rect::from_center_size(
            slot.center(),
            Vec2::splat(art_size * (0.9 + 0.1 * m.art_pop)),
        );
        let radius = (style.art_radius * s).min(art_size * 0.5);
        let art_alpha = (m.alpha * m.enter).clamp(0.0, 1.0);
        match card.art {
            Some(texture) => paint_rounded_image(
                painter,
                texture,
                art_rect,
                radius,
                Color32::from_white_alpha((255.0 * art_alpha).round() as u8),
            ),
            None => {
                 
                painter.rect_filled(
                    art_rect,
                    Rounding::same(radius),
                    dimmed(Color32::from_white_alpha(22), art_alpha),
                );
                let ink = dimmed(style.accent, art_alpha * 0.8);
                let head = art_rect.center() + egui::vec2(-art_size * 0.10, art_size * 0.16);
                painter.circle_filled(head, art_size * 0.12, ink);
                painter.line_segment(
                    [
                        head + egui::vec2(art_size * 0.11, 0.0),
                        head + egui::vec2(art_size * 0.11, -art_size * 0.34),
                    ],
                    Stroke::new((art_size * 0.055).max(1.0), ink),
                );
            }
        }
        if style.show_border {
            painter.rect_stroke(
                art_rect,
                Rounding::same(radius),
                Stroke::new(1.0_f32, Color32::from_white_alpha((26.0 * art_alpha) as u8)),
            );
        }
        text_x = slot.max.x + SPOTIFY_ART_GAP * s;
    }

    let column = (text_right - text_x).max(0.0);
    let text_alpha = (m.enter * m.alpha).clamp(0.0, 1.0);
    let primary = dimmed(style.text, text_alpha);
    let secondary = dimmed(style.text, text_alpha * 0.58);
    let slide = (1.0 - m.enter) * 6.0 * s;

     
     
    let clock_font = FontId::monospace(style.artist_size * 0.9 * s);
    let clock = if rows.clock_row == SpotifyClockRow::None {
        None
    } else {
        Some(spotify_clock(card.elapsed, card.duration))
    };
    let clock_width = clock
        .as_ref()
        .map(|text| text_width(ctx, text, &clock_font))
        .unwrap_or(0.0);
    let clock_gap = if clock.is_some() { 10.0 * s } else { 0.0 };

    if style.show_title {
        let font = FontId::proportional(style.title_size * s);
        let reserved = if rows.clock_row == SpotifyClockRow::Title {
            clock_width + clock_gap
        } else {
            0.0
        };
        let width = (column - reserved).max(0.0);
        let x = text_x + slide;
        let y = rect.min.y + rows.title_y * s;
        let overflow = text_width(ctx, &card.title, &font) - width;
        if overflow > 0.5 && width > 0.0 {
             
            let clip =
                Rect::from_min_max(egui::pos2(x, rect.min.y), egui::pos2(x + width, rect.max.y));
            let offset = marquee_offset(overflow, m.since_change, SPOTIFY_MARQUEE_SPEED * s);
            painter.with_clip_rect(clip).text(
                egui::pos2(x - offset, y),
                Align2::LEFT_TOP,
                card.title.as_str(),
                font,
                primary,
            );
        } else {
            painter.text(
                egui::pos2(x, y),
                Align2::LEFT_TOP,
                card.title.as_str(),
                font,
                primary,
            );
        }
    }

    if style.show_artist {
        let font = FontId::proportional(style.artist_size * s);
        let reserved = if rows.clock_row == SpotifyClockRow::Artist {
            clock_width + clock_gap
        } else {
            0.0
        };
        let artist = fit_text(ctx, &card.artist, &font, (column - reserved).max(0.0));
        painter.text(
            egui::pos2(text_x + slide, rect.min.y + rows.artist_y * s),
            Align2::LEFT_TOP,
            artist,
            font,
            secondary,
        );
    }

    if let Some(clock) = &clock {
        painter.text(
            egui::pos2(text_right, rect.min.y + rows.time_y * s),
            Align2::RIGHT_CENTER,
            clock,
            clock_font,
            dimmed(style.text, text_alpha * 0.45),
        );
    }

    if style.show_progress {
        let height = (style.progress_height * s).max(1.0);
        let half = height * 0.5;
         
         
        let bar = Rect::from_min_max(
            egui::pos2(text_x, rect.min.y + rows.bar_y * s),
            egui::pos2(text_right, rect.min.y + rows.bar_y * s + height),
        );
        painter.rect_filled(
            bar,
            Rounding::same(half),
            dimmed(Color32::from_white_alpha(26), m.alpha),
        );
        let played = bar.width() * m.progress.clamp(0.0, 1.0);
        if played > 0.0 {
            let fill = Rect::from_min_size(
                bar.min,
                Vec2::new(played.max(height).min(bar.width()), height),
            );
             
            let strength = 0.62 + 0.38 * m.playing;
            painter.rect_filled(
                fill,
                Rounding::same(half),
                dimmed(style.accent, m.alpha * strength),
            );
        }
    }
}

 
 
fn draw_spotify_widget(ctx: &Context, painter: &Painter) {
    let origin = widget_rendered_pos(ctx, HudWidgetId::Spotify);
    let style = SpotifyStyle::read();
    let playback = now_playing::current();
    let elapsed = playback
        .as_ref()
        .map(|p| p.position_at(std::time::Instant::now()))
        .unwrap_or(std::time::Duration::ZERO);
    let duration = playback
        .as_ref()
        .map(|p| p.duration)
        .unwrap_or(std::time::Duration::ZERO);
    let generation = playback.as_ref().map(|p| p.art_generation).unwrap_or(0);

    let now = ctx.input(|i| i.time);
    let since_change = ((now - track_first_seen(generation, now)) as f32 * anim::speed()).max(0.0);
    let live = playback.is_some();
    let motion = SpotifyMotion {
        enter: spotify_enter(since_change),
        art_pop: spotify_art_pop(since_change),
        alpha: SPOTIFY_IDLE_ALPHA
            + (1.0 - SPOTIFY_IDLE_ALPHA)
                * anim::toggle(ctx, Id::new("spotify_live"), live, 0.4, Easing::InOut),
        playing: anim::toggle(
            ctx,
            Id::new("spotify_playing"),
            playback.as_ref().is_some_and(|p| p.playing),
            0.3,
            Easing::InOut,
        ),
        progress: anim::spring(
            ctx,
            Id::new(("spotify_progress", generation)),
            track_fraction(elapsed, duration),
            SPOTIFY_PROGRESS_SPRING,
        )
        .clamp(0.0, 1.0),
        since_change,
    };

    let card = match playback {
        Some(playback) => SpotifyCard {
            title: playback.title.clone(),
            artist: playback.artist.clone(),
            elapsed,
            duration,
            art: spotify_art_texture(ctx, &playback),
            motion,
        },
        None => SpotifyCard {
            title: "Nothing playing".into(),
            artist: "Spotify".into(),
            elapsed,
            duration,
            art: None,
            motion,
        },
    };
    draw_spotify_card(ctx, painter, origin, &card, &style);
}

 
 
const MAX_SAMPLE_SPEED: f64 = 100.0;

 
 
 
 
fn refresh_speed(cache: &mut Option<WidgetCache>) {
    let cache = cache.get_or_insert_with(WidgetCache::default);
    let now = std::time::Instant::now();
    let Some(pos) = player_position() else {
        cache.speed_sample = None;
        cache.speed = 0.0;
        cache.speed_peak = 0.0;
        return;
    };
    let Some((last, at)) = cache.speed_sample else {
        cache.speed_sample = Some((pos, now));
        return;
    };
    let dt = now.duration_since(at).as_secs_f64();
    if dt < 0.1 {
        return;
    }
    let dx = pos[0] - last[0];
    let dz = pos[2] - last[2];
    let instant = ((dx * dx + dz * dz).sqrt() / dt).min(MAX_SAMPLE_SPEED);
     
     
    cache.speed = if cache.speed <= 0.0 {
        instant
    } else {
        cache.speed * 0.8 + instant * 0.2
    };
     
     
    cache.speed_peak = cache.speed_peak.max(cache.speed).min(MAX_SAMPLE_SPEED);
    cache.speed_sample = Some((pos, now));
}

 
fn session_uptime() -> String {
    static START: std::sync::LazyLock<std::time::Instant> =
        std::sync::LazyLock::new(std::time::Instant::now);
    format_uptime(START.elapsed().as_secs())
}

 
fn format_uptime(seconds: u64) -> String {
    format!(
        "{}:{:02}:{:02}",
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60
    )
}

 
 
fn refresh_ping(cache: &mut Option<WidgetCache>) -> Option<i32> {
    const REFRESH: std::time::Duration = std::time::Duration::from_secs(1);
    let cache = cache.get_or_insert_with(WidgetCache::default);
    if let Some((ping, at)) = cache.ping {
        if at.elapsed() < REFRESH {
            return ping;
        }
    }
    let ping = crate::state::minecraft()
        .player()
        .ok()
        .flatten()
        .and_then(|player| player.player_info().ok())
        .and_then(|info| info.latency().ok())
        .filter(|latency| *latency > 0);
    cache.ping = Some((ping, std::time::Instant::now()));
    match ping {
        Some(ms) => {
            cache.ping_history.push(ms.max(0) as u32);
            let excess = cache
                .ping_history
                .len()
                .saturating_sub(crate::graphic::ping::SPARK_SAMPLES);
            if excess > 0 {
                cache.ping_history.drain(..excess);
            }
        }
         
         
        None => cache.ping_history.clear(),
    }
    ping
}

 
fn player_position() -> Option<[f64; 3]> {
    let player = crate::state::minecraft().player().ok()??;
    let pos = player.get_position().ok()?;
    Some([pos.x(), pos.y(), pos.z()])
}

 
fn player_yaw() -> Option<f32> {
    let player = crate::state::minecraft().player().ok()??;
    player.get_yaw().ok()
}

 
 
fn view_yaw() -> Option<f32> {
    crate::state::minecraft()
        .game_renderer()
        .and_then(|renderer| renderer.get_main_camera())
        .and_then(|camera| camera.yaw())
        .ok()
        .or_else(player_yaw)
}

 
fn draw_compass(ctx: &Context, painter: &Painter) {
    let Some(yaw) = view_yaw() else {
        return;
    };
    let origin = widget_rendered_pos(ctx, HudWidgetId::Compass);
    crate::graphic::compass::draw(painter, origin, yaw, compass_direction(yaw));
}

 
fn compass_direction(yaw: f32) -> &'static str {
    let yaw = yaw.rem_euclid(360.0);
    match yaw {
        337.5..=360.0 | 0.0..=22.5 => "South",
        22.5..=67.5 => "South West",
        67.5..=112.5 => "West",
        112.5..=157.5 => "North West",
        157.5..=202.5 => "North",
        202.5..=247.5 => "North East",
        247.5..=292.5 => "East",
        _ => "South East",
    }
}

 
fn roman(n: i32) -> String {
    match n {
        1 => "I".to_string(),
        2 => "II".to_string(),
        3 => "III".to_string(),
        4 => "IV".to_string(),
        5 => "V".to_string(),
        other => other.to_string(),
    }
}

fn title_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for (i, word) in name.split('_').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

 
#[derive(Clone, Debug, PartialEq)]
struct EffectStatus {
     
    id: String,
     
    name: String,
    amplifier: i32,
     
    duration: i32,
     
     
     
    longest: i32,
}

impl EffectStatus {
    fn infinite(&self) -> bool {
        self.duration < 0
    }

     
    fn level(&self) -> String {
        roman(self.amplifier.saturating_add(1))
    }

    fn fraction(&self) -> f32 {
        if self.infinite() {
            return 1.0;
        }
        if self.longest <= 0 {
            return 0.0;
        }
        (self.duration as f32 / self.longest as f32).clamp(0.0, 1.0)
    }

    fn time_text(&self) -> String {
        if self.infinite() {
            return "∞".to_string();
        }
        clock_text(self.duration)
    }

     
    fn expiring(&self) -> bool {
        !self.infinite() && self.duration <= 200
    }
}

fn clock_text(ticks: i32) -> String {
    let seconds = ticks.max(0).saturating_add(19) / 20;
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

 
 
 
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct PotionMotion {
    age: f32,
    signature: u64,
}

impl PotionMotion {
    fn advance(&mut self, effects: &[EffectStatus], dt: f32) {
        let signature = effect_signature(effects);
        if signature != self.signature {
            self.signature = signature;
            self.age = 0.0;
        }
        self.age = (self.age + dt.max(0.0)).min(8.0);
    }

     
    fn intro(&self, row: usize) -> f32 {
        let start = row as f32 * 0.045;
        let t = ((self.age - start) / 0.24).clamp(0.0, 1.0);
        anim::Easing::OutBack.apply(t)
    }

    fn settled(effects: &[EffectStatus]) -> Self {
        Self {
            age: 8.0,
            signature: effect_signature(effects),
        }
    }

    fn animating(&self) -> bool {
        self.age < 1.4
    }
}

 
fn effect_signature(effects: &[EffectStatus]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for effect in effects {
        effect.id.hash(&mut hasher);
        effect.amplifier.hash(&mut hasher);
    }
    hasher.finish()
}

 
 
fn sort_effect_rows(rows: &mut [EffectStatus]) {
    let life = |effect: &EffectStatus| {
        if effect.infinite() {
            i64::MAX
        } else {
            i64::from(effect.longest)
        }
    };
    rows.sort_by(|a, b| life(b).cmp(&life(a)).then_with(|| a.name.cmp(&b.name)));
}

const POTION_W: f32 = 198.0;
const POTION_HEADER_H: f32 = 22.0;
const POTION_ROW_H: f32 = 27.0;
const POTION_PAD: f32 = 7.0;
const POTION_EMPTY_H: f32 = 34.0;
const POTION_TILE: f32 = 19.0;
const POTION_MAX_ROWS: usize = 8;

fn potion_card_height(count: usize, extra: bool, scale: f32) -> f32 {
    if count == 0 {
        return (POTION_HEADER_H + POTION_EMPTY_H + POTION_PAD) * scale;
    }
    let rows = count.min(POTION_MAX_ROWS) as f32 + if extra { 1.0 } else { 0.0 };
    (POTION_HEADER_H + POTION_PAD + rows * POTION_ROW_H) * scale
}

fn potion_card_size(count: usize, extra: bool, scale: f32) -> Vec2 {
    Vec2::new(POTION_W * scale, potion_card_height(count, extra, scale))
}

 
 
fn potion_preview_effects() -> Vec<EffectStatus> {
    let make = |id: &str, amplifier: i32, duration: i32, longest: i32| EffectStatus {
        id: id.to_string(),
        name: title_case(id),
        amplifier,
        duration,
        longest,
    };
    let mut rows = vec![
        make("night_vision", 0, -1, -1),
        make("fire_resistance", 0, 1680, 3600),
        make("regeneration", 1, 620, 1200),
        make("speed", 1, 240, 1200),
    ];
    sort_effect_rows(&mut rows);
    rows
}

 
 
 
fn draw_potion_card(
    painter: &Painter,
    anchor: Pos2,
    effects: &[EffectStatus],
    motion: PotionMotion,
) {
    let style = UiStyle::hud_style();
    let s = style.scale;
    let extra = effects.len() > POTION_MAX_ROWS;
    let size = potion_card_size(effects.len(), extra, s);
    let rect = Rect::from_min_size(egui::pos2(anchor.x, anchor.y - size.y), size);
    paint_panel(painter, rect, &style);

    let pad = POTION_PAD * s;
    let header_h = POTION_HEADER_H * s;
    let accent = style.accent;

     
    let pip = egui::pos2(rect.left() + pad + 1.6 * s, rect.top() + header_h * 0.5);
    painter.circle_filled(pip, 1.8 * s, theme::with_alpha(accent, 0.95));
    painter.text(
        egui::pos2(pip.x + 7.0 * s, pip.y),
        Align2::LEFT_CENTER,
        "POTIONS",
        FontId::proportional(8.5 * s),
        theme::with_alpha(style.text, 0.78),
    );
    if !effects.is_empty() {
        let label = effects.len().to_string();
        let chip_w = (10.0 + 5.0 * label.chars().count() as f32) * s;
        let chip = Rect::from_center_size(
            egui::pos2(rect.right() - pad - chip_w * 0.5, pip.y),
            Vec2::new(chip_w, 13.0 * s),
        );
        painter.rect_filled(
            chip,
            Rounding::same(6.5 * s),
            theme::with_alpha(accent, 0.16),
        );
        painter.rect_stroke(
            chip,
            Rounding::same(6.5 * s),
            Stroke::new(0.8_f32, theme::with_alpha(accent, 0.38)),
        );
        painter.text(
            chip.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::monospace(8.0 * s),
            theme::with_alpha(accent, 0.96),
        );
    }

    let rule_y = rect.top() + header_h;
    painter.line_segment(
        [
            egui::pos2(rect.left() + pad, rule_y),
            egui::pos2(rect.right() - pad, rule_y),
        ],
        Stroke::new(0.8_f32, theme::with_alpha(theme::BORDER, 0.62)),
    );
    painter.line_segment(
        [
            egui::pos2(rect.left() + pad, rule_y),
            egui::pos2(rect.left() + pad + 22.0 * s, rule_y),
        ],
        Stroke::new(1.3_f32, theme::with_alpha(accent, 0.55)),
    );

    if effects.is_empty() {
        let empty = Rect::from_min_size(
            egui::pos2(rect.left() + pad, rect.top() + header_h),
            Vec2::new(rect.width() - pad * 2.0, POTION_EMPTY_H * s),
        );
        let tile = Rect::from_center_size(
            egui::pos2(
                empty.left() + POTION_TILE * 0.5 * s,
                empty.center().y - 2.0 * s,
            ),
            Vec2::splat(POTION_TILE * s),
        );
        crate::graphic::potion_icon::paint(painter, tile, "potion", 0.5);
        painter.text(
            egui::pos2(tile.right() + 8.0 * s, empty.center().y - 6.0 * s),
            Align2::LEFT_CENTER,
            "No active effects",
            FontId::proportional(9.5 * s),
            theme::with_alpha(style.text, 0.55),
        );
        painter.text(
            egui::pos2(tile.right() + 8.0 * s, empty.center().y + 6.0 * s),
            Align2::LEFT_CENTER,
            "Potions appear here",
            FontId::proportional(7.5 * s),
            theme::with_alpha(style.text, 0.30),
        );
        return;
    }

    for (index, effect) in effects.iter().take(POTION_MAX_ROWS).enumerate() {
        let top = rect.top() + header_h + index as f32 * POTION_ROW_H * s;
        let row = Rect::from_min_size(
            egui::pos2(rect.left(), top),
            Vec2::new(rect.width(), POTION_ROW_H * s),
        );
        let intro = motion.intro(index);
        let alpha = intro.clamp(0.0, 1.0);
        let slide = (1.0 - intro) * 5.0 * s;
        draw_potion_row(
            painter,
            row.translate(egui::vec2(0.0, slide)),
            effect,
            &style,
            alpha,
        );
    }

    if extra {
        let top = rect.top() + header_h + POTION_MAX_ROWS as f32 * POTION_ROW_H * s;
        let left = rect.left() + pad + 2.0 * s;
        for index in 0..3 {
            painter.circle_filled(
                egui::pos2(left + index as f32 * 4.2 * s, top + 13.5 * s),
                1.3 * s,
                theme::with_alpha(accent, 0.55),
            );
        }
        painter.text(
            egui::pos2(left + 15.0 * s, top + 13.5 * s),
            Align2::LEFT_CENTER,
            format!("+{} more", effects.len() - POTION_MAX_ROWS),
            FontId::proportional(8.5 * s),
            theme::with_alpha(style.text, 0.42),
        );
    }

    if motion.animating() || effects.iter().any(EffectStatus::expiring) {
        painter.ctx().request_repaint();
    }
}

fn draw_potion_row(
    painter: &Painter,
    row: Rect,
    effect: &EffectStatus,
    style: &UiStyle,
    alpha: f32,
) {
    let s = style.scale;
    let accent = crate::graphic::potion_icon::color(&effect.id);
    let now = painter.ctx().input(|input| input.time) as f32;
    let pulse = 0.55 + 0.45 * ((now * 5.0).sin() * 0.5 + 0.5);
    let urgent = effect.expiring();
    let danger = theme::lerp_color(accent, theme::DANGER, 0.7);

    let tile_size = POTION_TILE * s;
    let tile = Rect::from_min_size(
        egui::pos2(
            row.left() + POTION_PAD * s,
            row.top() + (POTION_ROW_H * s - tile_size) * 0.5,
        ),
        Vec2::splat(tile_size),
    );
    if urgent {
        painter.rect_filled(
            tile.expand(2.5 * s),
            Rounding::same(6.0 * s),
            theme::with_alpha(theme::DANGER, 0.14 * pulse * alpha),
        );
    }
    crate::graphic::potion_icon::paint(painter, tile, &effect.id, alpha);

    let text_left = tile.right() + 7.0 * s;
    let name_font = FontId::proportional(9.5 * s);
    let name_top = row.top() + 3.0 * s;
    painter.text(
        egui::pos2(text_left, name_top),
        Align2::LEFT_TOP,
        &effect.name,
        name_font.clone(),
        theme::with_alpha(style.text, 0.92 * alpha),
    );

    let time = effect.time_text();
     
    let time_font = FontId::monospace(if effect.infinite() { 11.0 } else { 8.5 } * s);
    let time_width = painter.ctx().fonts(|fonts| {
        fonts
            .layout_no_wrap(time.clone(), time_font.clone(), Color32::WHITE)
            .size()
            .x
    });
    painter.text(
        egui::pos2(row.right() - POTION_PAD * s, name_top + 4.6 * s),
        Align2::RIGHT_CENTER,
        time,
        time_font,
        if urgent {
            theme::with_alpha(danger, (0.62 + 0.38 * pulse) * alpha)
        } else {
            theme::with_alpha(accent, 0.85 * alpha)
        },
    );

     
     
    if effect.amplifier > 0 {
        let name_width = painter.ctx().fonts(|fonts| {
            fonts
                .layout_no_wrap(effect.name.clone(), name_font.clone(), Color32::WHITE)
                .size()
                .x
        });
        let level = effect.level();
        let level_width = (level.chars().count() as f32 * 4.6 + 6.0) * s;
        let pill = Rect::from_min_size(
            egui::pos2(text_left + name_width + 4.0 * s, name_top + 1.0 * s),
            Vec2::new(level_width, 9.5 * s),
        );
        let time_left = row.right() - POTION_PAD * s - time_width;
        if pill.right() < time_left - 3.0 * s {
            painter.rect_filled(
                pill,
                Rounding::same(4.0 * s),
                theme::with_alpha(accent, 0.17 * alpha),
            );
            painter.text(
                pill.center(),
                Align2::CENTER_CENTER,
                level,
                FontId::monospace(7.0 * s),
                theme::with_alpha(accent, alpha),
            );
        }
    }

    let radius = Rounding::same(1.3 * s);
    let track = Rect::from_min_size(
        egui::pos2(text_left, row.top() + 18.0 * s),
        Vec2::new(row.right() - POTION_PAD * s - text_left, 2.6 * s),
    );
    painter.rect_filled(track, radius, theme::with_alpha(style.text, 0.07 * alpha));

    let fraction = effect.fraction();
    let fill_width = track.width() * fraction;
    if fill_width > 0.5 {
        let fill = Rect::from_min_size(
            track.min,
            Vec2::new(fill_width.max(radius.nw * 2.0), track.height()),
        );
        painter.rect_filled(
            fill,
            radius,
            if effect.infinite() {
                theme::with_alpha(accent, 0.30 * alpha)
            } else if urgent {
                theme::with_alpha(danger, (0.62 + 0.38 * pulse) * alpha)
            } else {
                theme::with_alpha(accent, 0.88 * alpha)
            },
        );
         
         
        if track.height() > 1.5 {
            painter.line_segment(
                [
                    egui::pos2(fill.left() + radius.nw, fill.top() + track.height() * 0.3),
                    egui::pos2(fill.right() - radius.ne, fill.top() + track.height() * 0.3),
                ],
                Stroke::new(0.6_f32 * s, Color32::from_white_alpha((44.0 * alpha) as u8)),
            );
        }
        if !effect.infinite() && fraction > 0.06 {
            painter.circle_filled(
                egui::pos2(fill.right(), fill.center().y),
                1.5 * s,
                theme::with_alpha(theme::lerp_color(accent, Color32::WHITE, 0.55), alpha),
            );
        }
    }
}

 
 
fn refresh_effects(cache: &mut Option<WidgetCache>) {
    let cache = cache.get_or_insert_with(WidgetCache::default);
    let fresh = cache
        .last_effects
        .is_none_or(|at| at.elapsed() >= std::time::Duration::from_millis(250));
    if !fresh {
        return;
    }
    cache.last_effects = Some(std::time::Instant::now());
    let active = match crate::state::minecraft().player() {
        Ok(Some(player)) => match player.get_active_effects() {
            Ok(effects) => effects,
            Err(error) => {
                log::debug!("Potion Status: {error:#}");
                Vec::new()
            }
        },
        _ => Vec::new(),
    };
    let previous = std::mem::take(&mut cache.effects);
    let mut next: Vec<EffectStatus> = active
        .into_iter()
        .map(|(raw, amplifier, duration)| {
            let id = raw
                .strip_prefix("minecraft:")
                .unwrap_or(&raw)
                .to_ascii_lowercase();
            let longest = previous
                .iter()
                .find(|effect| effect.id == id && effect.amplifier == amplifier)
                .map(|effect| effect.longest.max(duration))
                .unwrap_or(duration);
            EffectStatus {
                name: title_case(&id),
                id,
                amplifier,
                duration,
                longest,
            }
        })
        .collect();
    sort_effect_rows(&mut next);
    cache.effects = next;
}

 
 
fn refresh_armor(cache: &mut Option<WidgetCache>) {
    let cache = cache.get_or_insert_with(WidgetCache::default);
    let fresh = cache
        .last_armor
        .is_none_or(|at| at.elapsed() >= std::time::Duration::from_millis(250));
    if !fresh {
        return;
    }
    cache.last_armor = Some(std::time::Instant::now());
    let mut slots: [Option<ArmorPiece>; 4] = Default::default();
    if let Some(player) = crate::state::minecraft().player().ok().flatten() {
        if let Ok(inventory) = player.get_inventory() {
            for (i, slot) in (36..40).enumerate() {
                let Ok(stack) = inventory.get_item(slot) else {
                    continue;
                };
                if stack.is_empty().unwrap_or(true) {
                    continue;
                }
                let Ok(full) = stack.get_item().and_then(|item| item.registry_name()) else {
                    continue;
                };
                let id = full.strip_prefix("minecraft:").unwrap_or(&full).to_string();
                 
                let durability = match (stack.get_damage_value(), stack.get_max_damage()) {
                    (Ok(damage), Ok(max)) if max > 0 => 1.0 - damage as f32 / max as f32,
                    _ => 1.0,
                };
                slots[i] = Some(ArmorPiece {
                    name: title_case(&id),
                    id,
                    durability: durability.clamp(0.0, 1.0),
                });
            }
        }
    }
    cache.armor = slots;
}

 
fn look_vector(yaw: f32, pitch: f32) -> (f64, f64, f64) {
    let (yaw_r, pitch_r) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    let x = -yaw_r.sin() * pitch_r.cos();
    let y = -pitch_r.sin();
    let z = yaw_r.cos() * pitch_r.cos();
    (x, y, z)
}

 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RadarKind {
    Player,
    Mob,
    Other,
}

fn radar_settings() -> (f64, bool, bool) {
    crate::state::client()
        .modules
        .get(crate::module::ModuleId::Radar)
        .and_then(|handle| {
            let module = handle.lock().ok()?;
            let data = module.get_module_data();
            let range = data
                .get_setting("Range")
                .and_then(|setting| setting.get_slider_value())
                .unwrap_or(64.0);
            let mobs = data
                .get_setting("Show Mobs")
                .and_then(|setting| setting.get_toggle_value())
                .unwrap_or(true);
            let other = data
                .get_setting("Show Other Entities")
                .and_then(|s| s.get_toggle_value())
                .unwrap_or(true);
            let range = if range.is_finite() {
                range.clamp(16.0, 128.0)
            } else {
                64.0
            };
            Some((f64::from(range), mobs, other))
        })
        .unwrap_or((64.0, true, true))
}

 
 
fn refresh_radar(cache: &mut Option<WidgetCache>) {
    let cache = cache.get_or_insert_with(WidgetCache::default);
    let mc = crate::state::minecraft();
    let (Some(player), Some(world)) = (mc.player().ok().flatten(), mc.world().ok().flatten())
    else {
        cache.radar.clear();
        cache.last_radar = None;
        return;
    };
    let interval = crate::module::performance::client_optimizations::duration(
        crate::module::ModuleId::RadarScanOptimizer,
        "Refresh ms",
        std::time::Duration::from_millis(100),
        100.0,
        1000.0,
    );
    if cache.last_radar.is_some_and(|at| at.elapsed() < interval) {
        return;
    }
    cache.last_radar = Some(std::time::Instant::now());
    cache.radar.clear();
    let (Ok(self_id), Ok(pos)) = (player.id(), player.get_position()) else {
        return;
    };
    let (range, show_mobs, show_other) = radar_settings();
    let entities = match world.get_entities() {
        Ok(entities) => entities,
        Err(error) => {
            log::debug!("Radar entity scan failed: {error:#}");
            return;
        }
    };
    for entity in &entities {
        let Ok(id) = entity.id() else { continue };
        if id == self_id {
            continue;
        }
        let kind = if entity.instance_of::<crate::mapping::entity::player::Player>() {
            RadarKind::Player
        } else if entity.instance_of::<crate::mapping::entity::mob::Mob>() {
            if !show_mobs {
                continue;
            }
            RadarKind::Mob
        } else {
            if !show_other {
                continue;
            }
            RadarKind::Other
        };
        if entity.instance_of::<crate::mapping::entity::living::LivingEntity>() {
            let alive = crate::state::mapping().in_frame(|| {
                Ok(!crate::state::mapping()
                    .call_method(
                        crate::mapping::MinecraftClassType::LivingEntity,
                        entity.jni_ref().as_obj(),
                        "isDeadOrDying",
                        &[],
                    )?
                    .z()?)
            });
            match alive {
                Ok(true) => {}
                Ok(false) => continue,
                Err(error) => {
                    log::debug!("Radar living contact {id}: {error:#}");
                    continue;
                }
            }
        }
        let Ok(entity_pos) = entity.get_position() else {
            continue;
        };
        let dx = entity_pos.x() - pos.x();
        let dz = entity_pos.z() - pos.z();
         
         
        if dx * dx + dz * dz > (range + 4.0).powi(2) {
            continue;
        }
        cache
            .radar
            .push(([entity_pos.x(), entity_pos.y(), entity_pos.z()], kind));
    }
}

#[cfg(test)]
pub(crate) fn radar_contacts_for_test() -> Vec<([f64; 3], RadarKind)> {
    let mut cache = cache();
    *cache = None;
    refresh_radar(&mut cache);
    cache.as_ref().unwrap().radar.clone()
}

 
 
fn radar_offset(dx: f64, dz: f64, yaw: f32, range: f64, radius: f32) -> Option<Vec2> {
    if !dx.is_finite()
        || !dz.is_finite()
        || !yaw.is_finite()
        || !range.is_finite()
        || range <= 0.0
        || dx.hypot(dz) > range
    {
        return None;
    }
    let (sin, cos) = f64::from(yaw).to_radians().sin_cos();
    let right = -dx * cos - dz * sin;
    let forward = -dx * sin + dz * cos;
    let scale = f64::from(radius) / range;
    Some(egui::vec2(
        (right * scale) as f32,
        (-forward * scale) as f32,
    ))
}

fn radar_kind_visible(kind: RadarKind, mobs: bool, other: bool) -> bool {
    match kind {
        RadarKind::Player => true,
        RadarKind::Mob => mobs,
        RadarKind::Other => other,
    }
}

fn draw_radar_contact(painter: &Painter, contact: Pos2, kind: RadarKind) {
    let color = match kind {
        RadarKind::Player => Color32::from_rgb(255, 70, 80),
        RadarKind::Mob => Color32::from_rgb(255, 210, 60),
        RadarKind::Other => Color32::from_rgb(70, 205, 255),
    };
    painter.circle_filled(contact, 3.5, Color32::BLACK);
    painter.circle_filled(contact, 2.8, color);
    painter.circle_stroke(contact, 2.8, Stroke::new(0.6_f32, Color32::WHITE));
}

 
fn draw_radar(ctx: &Context, painter: &Painter) {
    let origin = widget_rendered_pos(ctx, HudWidgetId::Radar);
    draw_radar_at(ctx, painter, origin);
}

 
pub(crate) fn draw_radar_at(_ctx: &Context, painter: &Painter, origin: Pos2) {
    const RADIUS: f32 = 60.0;
    const PLOT_RADIUS: f32 = 40.0;
    let center = origin + Vec2::splat(RADIUS);
    let (range, show_mobs, show_other) = radar_settings();
    let yaw = view_yaw().unwrap_or(0.0);

    painter.circle_filled(center, RADIUS, theme::with_alpha(theme::base(), 0.92));
    painter.circle_stroke(center, RADIUS - 0.5, Stroke::new(1.0_f32, theme::BORDER));
    painter.circle_filled(center, PLOT_RADIUS, theme::with_alpha(theme::SURFACE, 0.85));
    for fraction in [0.5, 1.0] {
        painter.circle_stroke(
            center,
            PLOT_RADIUS * fraction,
            Stroke::new(0.8_f32, theme::with_alpha(theme::TEXT_MUTED, 0.45)),
        );
    }
    for axis in [egui::vec2(PLOT_RADIUS, 0.0), egui::vec2(0.0, PLOT_RADIUS)] {
        painter.line_segment(
            [center - axis, center + axis],
            Stroke::new(0.6_f32, theme::with_alpha(theme::TEXT_MUTED, 0.35)),
        );
    }
     
    for (label, dx, dz) in [
        ("N", 0.0, -1.0),
        ("E", 1.0, 0.0),
        ("S", 0.0, 1.0),
        ("W", -1.0, 0.0),
    ] {
        if let Some(offset) = radar_offset(dx, dz, yaw, 1.0, 51.0) {
            painter.text(
                center + offset,
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(9.0),
                if label == "N" {
                    theme::accent()
                } else {
                    theme::TEXT_DIM
                },
            );
        }
    }

     
    painter.circle_filled(center, 3.0, theme::SURFACE);
    painter.add(egui::Shape::convex_polygon(
        vec![
            center + egui::vec2(0.0, -4.0),
            center + egui::vec2(2.5, 2.5),
            center + egui::vec2(-2.5, 2.5),
        ],
        theme::TEXT,
        Stroke::NONE,
    ));
    let dots = cache()
        .as_ref()
        .map(|c| c.radar.clone())
        .unwrap_or_default();
    let mut contact_count = 0;
    if let Some(player) = player_position() {
        for (position, kind) in dots {
            if !radar_kind_visible(kind, show_mobs, show_other) {
                continue;
            }
            let Some(offset) = radar_offset(
                position[0] - player[0],
                position[2] - player[2],
                yaw,
                range,
                PLOT_RADIUS,
            ) else {
                continue;
            };
            let contact = center + offset;
            draw_radar_contact(painter, contact, kind);
            contact_count += 1;
        }
    }

    painter.text(
        center + egui::vec2(0.0, 44.0),
        Align2::CENTER_CENTER,
        format!("{contact_count} entities"),
        FontId::proportional(8.0),
        theme::TEXT_DIM,
    );
    painter.text(
        center + egui::vec2(0.0, 30.0),
        Align2::CENTER_CENTER,
        format!("{range:.0}m"),
        FontId::proportional(9.0),
        theme::TEXT_MUTED,
    );
}

 
 
fn draw_key_strokes(ctx: &Context, painter: &Painter) {
    let origin = widget_rendered_pos(ctx, HudWidgetId::KeyStrokes);
    draw_key_strokes_at(ctx, painter, origin);
}

 
pub(crate) fn draw_key_strokes_at(_ctx: &Context, painter: &Painter, origin: Pos2) {
    let style = UiStyle::hud_style();
    let s = style.scale;
    let size = 30.0 * s;
    let gap = 4.0 * s;
    let base_x = origin.x;
    let base_y = origin.y;
    let round = Rounding::same((style.radius * s).min(size * 0.3));

    let keys: [(i32, &str, f32, f32); 4] = [
        (87, "W", size + gap, 0.0),
        (65, "A", 0.0, size + gap),
        (83, "S", size + gap, size + gap),
        (68, "D", (size + gap) * 2.0, size + gap),
    ];
    for (code, label, dx, dy) in keys {
        let rect = Rect::from_min_size(egui::pos2(base_x + dx, base_y + dy), Vec2::splat(size));
        let down = crate::graphic::input::key_down(code);
        paint_key_cap(painter, rect, &style, down, round);
        if style.text_shadow {
            painter.text(
                rect.center() + egui::vec2(1.0, 1.0),
                Align2::CENTER_CENTER,
                style.label(label),
                FontId::proportional(14.0 * s),
                Color32::from_black_alpha(150),
            );
        }
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            style.label(label),
            FontId::proportional(14.0 * s),
            if down { theme::base() } else { style.text },
        );
    }

    let mouse_row_y = base_y + (size + gap) * 2.0;
    for (left, label) in [(true, "LMB"), (false, "RMB")] {
        let down = crate::graphic::input::MOUSE_STATE
            .lock()
            .map(|mouse| {
                if left {
                    mouse.left_down
                } else {
                    mouse.right_down
                }
            })
            .unwrap_or(false);
        let cps = crate::graphic::input::cps(left);
        let x = if left {
            base_x
        } else {
            base_x + (size + gap) + gap
        };
        let rect = Rect::from_min_size(egui::pos2(x, mouse_row_y), Vec2::new(size + gap, size));
        paint_key_cap(painter, rect, &style, down, round);
        if style.text_shadow {
            painter.text(
                rect.center() + egui::vec2(1.0, 1.0),
                Align2::CENTER_CENTER,
                format!("{}\n{cps}", style.label(label)),
                FontId::proportional(11.0 * s),
                Color32::from_black_alpha(150),
            );
        }
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            format!("{}\n{cps}", style.label(label)),
            FontId::proportional(11.0 * s),
            if down { theme::base() } else { style.text },
        );
    }
}

 
 
fn paint_key_cap(painter: &Painter, rect: Rect, style: &UiStyle, down: bool, round: Rounding) {
    if down {
        painter.rect_filled(rect, round, style.accent);
    } else {
        painter.rect_filled(rect, round, style.background);
        if style.panel_gradient && style.background.a() > 0 {
            let outline = rounded_outline(rect, round.nw);
            let opacity = style.background.a() as f32 / 255.0;
            let top = theme::lerp_color(theme::SURFACE, style.accent, style.gradient_strength);
            let bottom = theme::lerp_color(theme::SURFACE, Color32::BLACK, 0.45);
            paint_vertical_gradient(
                painter,
                rect,
                &outline,
                theme::with_alpha(top, opacity),
                theme::with_alpha(bottom, opacity),
            );
        }
    }

    if style.border_width > 0.0 && style.border_alpha > 0.0 {
        painter.rect_stroke(
            rect,
            round,
            Stroke::new(
                style.border_width,
                if down {
                    theme::with_alpha(style.accent, 0.85)
                } else {
                    theme::with_alpha(theme::BORDER, style.border_alpha)
                },
            ),
        );
    }
}

 
const LOGO_H: f32 = 135.0;

 
 
const LOGO_STRETCH: f32 = 1.25;

fn watermark_rect(screen: Rect, source_size: (u32, u32), height: f32, margin: f32) -> Rect {
    let aspect = source_size.0 as f32 / source_size.1.max(1) as f32;
    let size = Vec2::new(height * aspect * LOGO_STRETCH, height);
    Rect::from_min_size(
        screen.right_top() + Vec2::new(-margin - size.x, MARGIN),
        size,
    )
}

 
 
 
 
fn draw_watermark(ctx: &Context, painter: &Painter, height: f32, opacity: f32, margin: f32) -> f32 {
    let panel = watermark_rect(
        ctx.screen_rect(),
        crate::graphic::branding::logo_size(),
        height,
        margin,
    );
    let tint = theme::with_alpha(theme::brand(), opacity);

    match crate::graphic::branding::logo_texture(ctx) {
        Some(texture) => {
            painter.add(egui::epaint::RectShape {
                rect: panel,
                rounding: Rounding::same(3.0),
                fill: tint,
                stroke: Stroke::NONE,
                blur_width: 0.0,
                fill_texture_id: texture,
                uv: Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            });
        }
        None => {
             
             
             
            painter.text(
                panel.center(),
                Align2::CENTER_CENTER,
                "Infered",
                FontId::proportional((height * 0.1).max(10.0)),
                tint,
            );
        }
    }

    panel.bottom() + 6.0
}

 
 
 
fn draw_arraylist(ctx: &Context, painter: &Painter, top: f32, cfg: &ModuleListConfig) {
    let row_h = cfg.row_height;
    let pad_x = (cfg.text_size * 0.7).max(6.0);
    let radius = Rounding::same((row_h * 0.35).min(7.0));
    let font = FontId::proportional(cfg.text_size.min(row_h - 4.0).max(6.0));

     
     
    let snapshot: Vec<(String, bool, crate::module::ModuleCategory)> = crate::state::client()
        .modules
        .handles()
        .iter()
        .filter_map(|m| {
            let module = m.lock().ok()?;
            let data = module.get_module_data();
            if data.id == crate::module::ModuleId::ModuleList {
                return None;
            }
            Some((data.name().to_string(), data.enabled, data.category))
        })
        .collect();

    let enabled_count = snapshot.iter().filter(|(_, enabled, _)| *enabled).count();

     
     
    let mut rows: Vec<(String, f32, f32, crate::module::ModuleCategory)> = Vec::new();  
    for (name, enabled, category) in snapshot {
        let factor = anim::toggle(
            ctx,
            Id::new("arraylist").with(&name),
            enabled,
            0.22,
            Easing::Out,
        );
        if factor <= 0.001 {
            continue;
        }
        let label = if cfg.lowercase {
            name.to_lowercase()
        } else {
            name
        };
        let width = ctx.fonts(|f| {
            f.layout_no_wrap(label.clone(), font.clone(), cfg.text_color)
                .size()
                .x
        });
        rows.push((label, factor, width, category));
    }
    if rows.is_empty() {
        return;
    }

    match cfg.sort {
         
        ModuleListSort::Length => rows.sort_by(|a, b| b.2.total_cmp(&a.2)),
        ModuleListSort::Name => rows.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase())),
        ModuleListSort::Category => rows.sort_by(|a, b| {
            a.3.display_name()
                .cmp(b.3.display_name())
                .then(b.2.total_cmp(&a.2))
        }),
    }

    let screen_w = ctx.screen_rect().width();
    let mut y = top;

    if cfg.show_count && enabled_count > 0 {
         
         
        let label = format!("{enabled_count} ENABLED");
        let font = FontId::proportional(9.5);
        let width = ctx.fonts(|f| {
            f.layout_no_wrap(label.clone(), font.clone(), theme::accent())
                .size()
                .x
        });
        let chip = Rect::from_min_size(
            egui::pos2(screen_w - cfg.right_margin - width - 14.0, y),
            Vec2::new(width + 14.0, 14.0),
        );
        painter.rect_filled(
            chip,
            Rounding::same(4.0),
            theme::with_alpha(theme::accent(), 0.16),
        );
        painter.rect_filled(
            Rect::from_min_size(chip.min, Vec2::new(2.0, chip.height())),
            Rounding {
                nw: 4.0,
                sw: 4.0,
                ..Rounding::ZERO
            },
            theme::with_alpha(theme::accent(), 0.85),
        );
        painter.text(
            egui::pos2(chip.right() - 7.0, chip.center().y),
            Align2::RIGHT_CENTER,
            label,
            font,
            theme::with_alpha(theme::accent(), 0.95),
        );
        y += 17.0;
    }

    for (name, factor, text_w, category) in &rows {
         
         
        let tint = theme::category_accent(*category);
         
        let eased = *factor;
        let lead = if cfg.show_dots { 8.0 } else { 0.0 };
        let row_w = text_w + pad_x * 2.0 + lead;
         
        let x = screen_w - cfg.right_margin - row_w * eased;
        let rect = Rect::from_min_size(egui::pos2(x, y), Vec2::new(row_w, row_h));

        if cfg.show_background {
            let alpha = (cfg.background_opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
            painter.rect_filled(
                rect,
                radius,
                theme::with_alpha(Color32::from_black_alpha(alpha), eased),
            );
        }
        if cfg.show_border {
            painter.rect_stroke(
                rect,
                radius,
                Stroke::new(1.0_f32, theme::with_alpha(theme::BORDER, eased)),
            );
        }

         
        if cfg.accent_bar {
            let tab = Rect::from_min_size(
                rect.right_top() - Vec2::new(3.0, 0.0),
                Vec2::new(3.0, row_h),
            );
            painter.rect_filled(tab, Rounding::same(2.0), theme::with_alpha(tint, eased));
        }

        if cfg.show_dots {
            painter.circle_filled(
                egui::pos2(rect.min.x + pad_x * 0.5 + 2.0, rect.center().y),
                2.0,
                theme::with_alpha(tint, eased),
            );
        }

        painter.text(
            egui::pos2(rect.min.x + pad_x + lead, rect.center().y),
            Align2::LEFT_CENTER,
            name,
            font.clone(),
            theme::with_alpha(cfg.text_color, eased),
        );

        y += row_h + cfg.row_spacing;
    }
}

 

 
 
const TOP_BAR_HEIGHT: f32 = 26.0;
const TOP_BAR_MIN_HEIGHT: f32 = 18.0;
const TOP_BAR_MAX_HEIGHT: f32 = 40.0;

 
 
 
 
struct TopBarConfig {
    enabled: bool,
    height: f32,
    show_brand: bool,
    show_wordmark: bool,
    show_time: bool,
    show_date: bool,
    show_fps: bool,
    show_ping: bool,
    show_direction: bool,
    show_coords: bool,
    show_speed: bool,
    show_module_count: bool,
}

 
 
 
 
 
#[derive(Clone, Debug)]
struct TopBarMetrics {
    height: f32,
     
    pad: f32,
     
    icon: f32,
     
    gap: f32,
     
    spacing: f32,
     
    font: FontId,
    wordmark: FontId,
}

impl TopBarMetrics {
    fn for_height(height: f32) -> Self {
        let height = height.clamp(TOP_BAR_MIN_HEIGHT, TOP_BAR_MAX_HEIGHT);
        Self {
            height,
            pad: (height * 0.52).clamp(9.0, 22.0),
            icon: (height * 0.56).clamp(11.0, 20.0),
            gap: (height * 0.20).clamp(4.0, 9.0),
            spacing: (height * 0.30).clamp(5.0, 12.0),
            font: FontId::proportional((height * 0.50).clamp(10.0, 17.0)),
            wordmark: FontId::proportional((height * 0.46).clamp(9.0, 15.0)),
        }
    }

     
    fn radius(&self) -> f32 {
        self.height * 0.5
    }

     
    fn logo(&self) -> Vec2 {
        let (w, h) = top_bar_logo_size();
        let side = (self.height * 0.56).clamp(11.0, 22.0);
        let aspect = w as f32 / h.max(1) as f32;
        Vec2::new(side * aspect, side)
    }
}

fn top_bar_config() -> Option<TopBarConfig> {
    let handle = crate::state::client()
        .modules
        .get(crate::module::ModuleId::TopBar)?;
    let module = handle.lock().ok()?;
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
    Some(TopBarConfig {
        enabled: data.enabled,
        height: slider("Bar Height", TOP_BAR_HEIGHT).clamp(TOP_BAR_MIN_HEIGHT, TOP_BAR_MAX_HEIGHT),
        show_brand: toggle("Show Brand", true),
        show_wordmark: toggle("Show Wordmark", true),
        show_time: toggle("Show Time", true),
        show_date: toggle("Show Date", false),
        show_fps: toggle("Show FPS", true),
        show_ping: toggle("Show Ping", true),
        show_direction: toggle("Show Direction", true),
        show_coords: toggle("Show Coordinates", false),
        show_speed: toggle("Show Speed", false),
        show_module_count: toggle("Show Module Count", true),
    })
}

 
fn enabled_module_count() -> usize {
    crate::state::client()
        .modules
        .handles()
        .iter()
        .filter(|handle| {
            handle
                .lock()
                .map(|module| module.get_module_data().enabled)
                .unwrap_or(false)
        })
        .count()
}

 
fn unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

 
fn short_clock() -> String {
    let day_secs = unix_seconds() % 86_400;
    format!("{:02}:{:02}", day_secs / 3600, (day_secs % 3600) / 60)
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

 
fn short_date() -> String {
    let (month, day) = civil_from_days((unix_seconds() / 86_400) as i64);
    format!("{} {day}", MONTHS[(month.clamp(1, 12) - 1) as usize])
}

 
 
 
 
 
fn civil_from_days(days: i64) -> (u32, u32) {
     
    let doe = (days + 719_468).rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (month as u32, day as u32)
}

 

 
 
static TOP_BAR_LOGO_PNG: &[u8] = include_bytes!("../../assets/infered_brand_logo.png");
static TOP_BAR_LOGO_TEX: std::sync::OnceLock<std::sync::Mutex<Option<TextureHandle>>> =
    std::sync::OnceLock::new();
static TOP_BAR_LOGO_SIZE: std::sync::OnceLock<(u32, u32)> = std::sync::OnceLock::new();

 
fn top_bar_logo_size() -> (u32, u32) {
    if let Some(size) = TOP_BAR_LOGO_SIZE.get() {
        return *size;
    }
    match image::load_from_memory(TOP_BAR_LOGO_PNG) {
        Ok(image) => {
            let size = (image.width(), image.height());
            let _ = TOP_BAR_LOGO_SIZE.set(size);
            size
        }
        Err(_) => (1254, 1254),
    }
}

 
 
fn top_bar_logo_texture(ctx: &Context) -> Option<egui::TextureId> {
    let lock = TOP_BAR_LOGO_TEX.get_or_init(|| std::sync::Mutex::new(None));
    let mut tex = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if tex.is_none() {
        let image = image::load_from_memory(TOP_BAR_LOGO_PNG).ok()?;
        let size = (image.width(), image.height());
        let _ = TOP_BAR_LOGO_SIZE.set(size);
        let mask = crate::graphic::branding::brand_mask(&image);
        *tex = Some(ctx.load_texture(
            "infered_top_bar_logo_mask",
            mask,
            egui::TextureOptions::LINEAR,
        ));
    }
    tex.as_ref().map(|handle| handle.id())
}

 
 
const TOP_BAR_SLIDE: anim::SpringCfg = anim::SpringCfg {
    stiffness: 240.0,
    damping: 26.0,
};
const TOP_BAR_LAYOUT: anim::SpringCfg = anim::SpringCfg {
    stiffness: 420.0,
    damping: 40.0,
};

 
#[derive(Clone, Copy, PartialEq, Eq)]
enum TopBarIcon {
    Clock,
    Date,
    Fps,
    Ping,
    Direction,
    Coords,
    Speed,
    Mods,
}

impl TopBarIcon {
     
     
     
    fn color(self) -> Color32 {
        match self {
            TopBarIcon::Fps | TopBarIcon::Speed => theme::accent(),
            TopBarIcon::Clock
            | TopBarIcon::Date
            | TopBarIcon::Ping
            | TopBarIcon::Direction
            | TopBarIcon::Coords
            | TopBarIcon::Mods => theme::TEXT_DIM,
        }
    }

     
     
    fn key(self) -> &'static str {
        match self {
            TopBarIcon::Clock => "clock",
            TopBarIcon::Date => "date",
            TopBarIcon::Fps => "fps",
            TopBarIcon::Ping => "ping",
            TopBarIcon::Direction => "direction",
            TopBarIcon::Coords => "coords",
            TopBarIcon::Speed => "speed",
            TopBarIcon::Mods => "mods",
        }
    }
}

 
 
struct TopBarBubble {
    icon: TopBarIcon,
    text: String,
    tint: Color32,
    width: f32,
}

impl TopBarBubble {
     
    fn tinted(mut self, tint: Color32) -> Self {
        self.tint = tint;
        self
    }
}

fn measure_bubble(
    ctx: &Context,
    metrics: &TopBarMetrics,
    icon: TopBarIcon,
    text: String,
) -> TopBarBubble {
    let text_w = ctx.fonts(|f| {
        f.layout_no_wrap(text.clone(), metrics.font.clone(), theme::TEXT)
            .size()
            .x
    });
    TopBarBubble {
        icon,
        text,
        tint: icon.color(),
        width: metrics.pad * 2.0 + metrics.icon + metrics.gap + text_w,
    }
}

 
 
static TOP_BAR_DIRECTION: std::sync::Mutex<Option<(Option<String>, std::time::Instant)>> =
    std::sync::Mutex::new(None);

fn top_bar_direction() -> Option<String> {
    let mut guard = TOP_BAR_DIRECTION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((value, at)) = guard.as_ref() {
        if at.elapsed() < std::time::Duration::from_millis(250) {
            return value.clone();
        }
    }
    let value = player_yaw().map(compass_direction).map(str::to_string);
    guard.replace((value.clone(), std::time::Instant::now()));
    value
}

 
 
static TOP_BAR_FPS: std::sync::Mutex<Option<(i32, std::time::Instant)>> =
    std::sync::Mutex::new(None);

fn top_bar_fps() -> i32 {
    let live = cache().as_ref().map(|c| c.fps).unwrap_or(0.0).round() as i32;
    let mut guard = TOP_BAR_FPS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match guard.as_ref() {
        Some((value, at)) if at.elapsed() < std::time::Duration::from_secs(1) => *value,
        _ => {
            guard.replace((live, std::time::Instant::now()));
            live
        }
    }
}

 
 
static TOP_BAR_POSITION: std::sync::Mutex<Option<(Option<[f64; 3]>, std::time::Instant)>> =
    std::sync::Mutex::new(None);

fn top_bar_position() -> Option<[f64; 3]> {
    let mut guard = TOP_BAR_POSITION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((value, at)) = guard.as_ref() {
        if at.elapsed() < std::time::Duration::from_millis(200) {
            return *value;
        }
    }
    let value = player_position();
    guard.replace((value, std::time::Instant::now()));
    value
}

 
 
 
fn top_bar_bubbles(
    ctx: &Context,
    cfg: &TopBarConfig,
    metrics: &TopBarMetrics,
) -> Vec<TopBarBubble> {
    let mut bubbles = Vec::new();
    if cfg.show_time {
        bubbles.push(measure_bubble(
            ctx,
            metrics,
            TopBarIcon::Clock,
            short_clock(),
        ));
    }
    if cfg.show_date {
        bubbles.push(measure_bubble(ctx, metrics, TopBarIcon::Date, short_date()));
    }
    if cfg.show_fps {
        bubbles.push(measure_bubble(
            ctx,
            metrics,
            TopBarIcon::Fps,
            format!("{}", top_bar_fps()),
        ));
    }
    if cfg.show_ping {
         
         
        let (text, tint) = match refresh_ping(&mut cache()) {
            Some(ms) if ms <= 60 => (format!("{ms} ms"), theme::SUCCESS),
            Some(ms) if ms <= 150 => (format!("{ms} ms"), theme::TEXT_DIM),
            Some(ms) => (format!("{ms} ms"), theme::WARN),
            None => ("-- ms".to_string(), theme::TEXT_MUTED),
        };
        bubbles.push(measure_bubble(ctx, metrics, TopBarIcon::Ping, text).tinted(tint));
    }
    if cfg.show_direction {
        if let Some(direction) = top_bar_direction() {
            bubbles.push(measure_bubble(
                ctx,
                metrics,
                TopBarIcon::Direction,
                direction,
            ));
        }
    }
    if cfg.show_coords {
        if let Some(pos) = top_bar_position() {
            bubbles.push(measure_bubble(
                ctx,
                metrics,
                TopBarIcon::Coords,
                format!(
                    "{} {} {}",
                    pos[0].round() as i64,
                    pos[1].round() as i64,
                    pos[2].round() as i64
                ),
            ));
        }
    }
    if cfg.show_speed {
        refresh_speed(&mut cache());
        let speed = cache().as_ref().map(|cache| cache.speed).unwrap_or(0.0);
        bubbles.push(measure_bubble(
            ctx,
            metrics,
            TopBarIcon::Speed,
            format!("{speed:.1} b/s"),
        ));
    }
    if cfg.show_module_count {
        bubbles.push(measure_bubble(
            ctx,
            metrics,
            TopBarIcon::Mods,
            format!("{}", enabled_module_count()),
        ));
    }
    bubbles
}

 
 
fn brand_chip_width(ctx: &Context, cfg: &TopBarConfig, metrics: &TopBarMetrics, logo: Vec2) -> f32 {
    let mut width = metrics.pad * 2.0 + logo.x;
    if cfg.show_wordmark {
        width += metrics.gap + wordmark_width(ctx, metrics);
    }
    width
}

fn wordmark_width(ctx: &Context, metrics: &TopBarMetrics) -> f32 {
    ctx.fonts(|f| {
        f.layout_no_wrap("Infered".to_string(), metrics.wordmark.clone(), theme::TEXT)
            .size()
            .x
    })
}

 
 
 
 
fn draw_top_bar(ctx: &Context, painter: &Painter) {
    let Some(cfg) = top_bar_config() else {
        return;
    };
     
     
    if crate::graphic::edit_ui::is_open() {
        top_bar_reveal_elapsed(ctx, false);
        return;
    }
    let screen = ctx.screen_rect();
    let menu_open = crate::graphic::input::GUI_OPEN.load(std::sync::atomic::Ordering::Relaxed);
    let visible = cfg.enabled && !menu_open;
    let elapsed = top_bar_reveal_elapsed(ctx, visible);

     
     
     
    let metrics = TopBarMetrics::for_height(cfg.height);
    let chip_h = metrics.height;
    let bubbles = top_bar_bubbles(ctx, &cfg, &metrics);

    let brand = cfg.show_brand.then(|| metrics.logo());
    let brand_w = brand.map(|size| brand_chip_width(ctx, &cfg, &metrics, size));

    let cell_count = brand.is_some() as usize + bubbles.len();
    let width = brand_w.unwrap_or(0.0)
        + bubbles.iter().map(|bubble| bubble.width).sum::<f32>()
        + metrics.spacing * cell_count.saturating_sub(1) as f32;
    let left = (screen.width() - width) * 0.5;

    let rest_y = screen.top() + MARGIN;
    let hide_y = screen.top() - chip_h - MARGIN;
    let y = anim::spring(
        ctx,
        Id::new("top_bar_y"),
        if cfg.enabled && !menu_open {
            rest_y
        } else {
            hide_y
        },
        TOP_BAR_SLIDE,
    );
    let x = anim::spring(ctx, Id::new("top_bar_left"), left, TOP_BAR_LAYOUT);
    let w = anim::spring(ctx, Id::new("top_bar_width"), width, TOP_BAR_LAYOUT);

     
     
     
    let row = Rect::from_min_size(egui::pos2(x, y), Vec2::new(w.max(4.0), chip_h));
    let off_screen = row.bottom() <= screen.top()
        || row.right() <= screen.left()
        || row.left() >= screen.right();
    if off_screen {
        return;
    }

    ctx.request_repaint();
    let layout = lay_out_top_bar(&metrics, brand_w, &bubbles, egui::pos2(x, y));
    let pointer = ctx.input(|input| input.pointer.hover_pos());
    paint_top_bar_row(
        ctx,
        painter,
        &TopBarRow {
            cfg: &cfg,
            metrics: &metrics,
            layout: &layout,
            brand,
            bubbles: &bubbles,
            visible,
            elapsed,
            pointer,
        },
    );
}

 
 
struct TopBarLayout {
    brand: Option<Rect>,
    bubbles: Vec<Rect>,
}

fn lay_out_top_bar(
    metrics: &TopBarMetrics,
    brand_width: Option<f32>,
    bubbles: &[TopBarBubble],
    origin: Pos2,
) -> TopBarLayout {
    let height = metrics.height;
    let mut cx = origin.x;
    let mut layout = TopBarLayout {
        brand: None,
        bubbles: Vec::with_capacity(bubbles.len()),
    };
    if let Some(width) = brand_width {
        layout.brand = Some(Rect::from_min_size(
            egui::pos2(cx, origin.y),
            Vec2::new(width, height),
        ));
        cx += width + metrics.spacing;
    }
    for bubble in bubbles {
        layout.bubbles.push(Rect::from_min_size(
            egui::pos2(cx, origin.y),
            Vec2::new(bubble.width, height),
        ));
        cx += bubble.width + metrics.spacing;
    }
    layout
}

 
struct TopBarRow<'a> {
    cfg: &'a TopBarConfig,
    metrics: &'a TopBarMetrics,
    layout: &'a TopBarLayout,
    brand: Option<Vec2>,
    bubbles: &'a [TopBarBubble],
    visible: bool,
    elapsed: f32,
     
    pointer: Option<Pos2>,
}

 
fn paint_top_bar_row(ctx: &Context, painter: &Painter, row: &TopBarRow<'_>) {
    let metrics = row.metrics;
    let radius = metrics.radius();
     
    let now = ctx.input(|input| input.time);
    let hovered =
        |rect: Rect, alpha: f32| alpha > 0.5 && row.pointer.is_some_and(|pos| rect.contains(pos));

     
     
    if let (Some(chip), Some(logo)) = (row.layout.brand, row.brand) {
        let (alpha, dy) = top_bar_cell_pose(row.visible, row.elapsed, 0);
        if alpha > 0.005 {
            let chip = chip.translate(egui::vec2(0.0, dy));
            draw_top_bar_chip(painter, chip, radius, alpha, comet_head(now, 0));
            if hovered(chip, alpha) {
                hover_wash(painter, chip, radius, alpha);
            }
             
             
            paint_horizontal_gradient(
                painter,
                Rect::from_min_size(
                    egui::pos2(chip.left() + metrics.pad, chip.bottom() - 2.0),
                    Vec2::new(chip.width() - metrics.pad * 2.0, 1.5),
                ),
                theme::with_alpha(theme::accent(), alpha),
                theme::with_alpha(theme::accent(), 0.0),
            );
            let mark = Rect::from_center_size(
                egui::pos2(chip.left() + metrics.pad + logo.x * 0.5, chip.center().y),
                logo,
            );
            if let Some(texture) = top_bar_logo_texture(ctx) {
                painter.add(egui::epaint::RectShape {
                    rect: mark,
                    rounding: Rounding::ZERO,
                    fill: theme::with_alpha(theme::brand(), alpha),
                    stroke: Stroke::NONE,
                    blur_width: 0.0,
                    fill_texture_id: texture,
                    uv: Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                });
            }
            if row.cfg.show_wordmark {
                painter.text(
                    egui::pos2(mark.right() + metrics.gap, chip.center().y),
                    Align2::LEFT_CENTER,
                    "Infered",
                    metrics.wordmark.clone(),
                    theme::with_alpha(theme::TEXT, 0.92 * alpha),
                );
            }
        }
    }

     
    let first = row.layout.brand.is_some() as usize;
    for (index, (bubble, rect)) in row.bubbles.iter().zip(&row.layout.bubbles).enumerate() {
        let cell = first + index;
        let (alpha, dy) = top_bar_cell_pose(row.visible, row.elapsed, cell);
        if alpha <= 0.005 {
            continue;
        }
        let rect = rect.translate(egui::vec2(0.0, dy));
        draw_top_bar_chip(painter, rect, radius, alpha, comet_head(now, cell));
        if hovered(rect, alpha) {
            hover_wash(painter, rect, radius, alpha);
        }
        let icon_center = egui::pos2(
            rect.left() + metrics.pad + metrics.icon * 0.5,
            rect.center().y,
        );
        draw_top_bar_icon(
            painter,
            bubble.icon,
            icon_center,
            theme::with_alpha(bubble.tint, alpha),
            metrics.icon,
        );
         
         
        draw_switching_text(
            ctx,
            painter,
            Id::new("top_bar_value").with(bubble.icon.key()),
            rect,
            egui::pos2(
                icon_center.x + metrics.icon * 0.5 + metrics.gap,
                rect.center().y,
            ),
            &bubble.text,
            theme::with_alpha(theme::TEXT, alpha),
            metrics.font.clone(),
        );
    }
}

 
 
fn hover_wash(painter: &Painter, rect: Rect, radius: f32, alpha: f32) {
    painter.rect_filled(
        rect,
        Rounding::same(radius),
        theme::with_alpha(theme::accent(), 0.10 * alpha),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(radius),
        Stroke::new(1.0_f32, theme::with_alpha(theme::accent(), 0.55 * alpha)),
    );
}

 
const SWITCH_DURATION: f64 = 0.2;

 
#[derive(Clone)]
struct TextSwitch {
    from: String,
    to: String,
    changed_at: f64,
}

 
 
 
 
fn switch_pose(t: f32, rise: f32) -> (f32, f32, f32, f32) {
    let e = Easing::Out.apply(t);
    (-rise * e, 1.0 - e, rise * (1.0 - e), e)
}

 
 
#[allow(clippy::too_many_arguments)]
fn draw_switching_text(
    ctx: &Context,
    painter: &Painter,
    id: Id,
    clip: Rect,
    pos: Pos2,
    text: &str,
    color: Color32,
    font: FontId,
) {
    let now = ctx.input(|i| i.time);
    let mut state = ctx
        .data(|d| d.get_temp::<TextSwitch>(id))
        .unwrap_or(TextSwitch {
            from: text.to_string(),
            to: text.to_string(),
            changed_at: now - SWITCH_DURATION,
        });
    if state.to != text {
        state = TextSwitch {
            from: state.to.clone(),
            to: text.to_string(),
            changed_at: now,
        };
    }
    let t = ((now - state.changed_at) / SWITCH_DURATION).clamp(0.0, 1.0) as f32;
    let (old_dy, old_alpha, new_dy, new_alpha) = switch_pose(t, font.size * 0.65);
    let clipped = painter.with_clip_rect(clip);
    if t < 1.0 {
        clipped.text(
            pos + egui::vec2(0.0, old_dy),
            Align2::LEFT_CENTER,
            &state.from,
            font.clone(),
            theme::with_alpha(color, old_alpha),
        );
        ctx.request_repaint();
    }
    clipped.text(
        pos + egui::vec2(0.0, new_dy),
        Align2::LEFT_CENTER,
        &state.to,
        font,
        theme::with_alpha(color, new_alpha),
    );
    ctx.data_mut(|d| d.insert_temp(id, state));
}

#[cfg(test)]
mod switch_tests {
    use super::*;

     
    const RISE: f32 = 8.0;

    #[test]
    fn switch_starts_on_the_old_value_and_settles_on_the_new_one() {
        let (old_dy, old_alpha, new_dy, new_alpha) = switch_pose(0.0, RISE);
        assert_eq!((old_dy, old_alpha), (0.0, 1.0));
        assert_eq!((new_dy, new_alpha), (RISE, 0.0));

        let (old_dy, old_alpha, new_dy, new_alpha) = switch_pose(1.0, RISE);
        assert_eq!((old_dy, old_alpha), (-RISE, 0.0));
        assert_eq!((new_dy, new_alpha), (0.0, 1.0));
    }

    #[test]
    fn a_bigger_bar_makes_bigger_moves() {
         
         
        let small = TopBarMetrics::for_height(18.0);
        let large = TopBarMetrics::for_height(40.0);
        assert!(large.font.size > small.font.size);
        assert!(large.icon > small.icon);
        assert!(large.pad > small.pad);
        assert!(large.spacing > small.spacing);
        assert_eq!(
            large.radius(),
            20.0,
            "a capsule is half as round as it is tall"
        );
        assert_eq!(TopBarMetrics::for_height(4.0).height, TOP_BAR_MIN_HEIGHT);
        assert_eq!(TopBarMetrics::for_height(400.0).height, TOP_BAR_MAX_HEIGHT);
    }
}

 
const TOP_BAR_REVEAL_STAGGER: f32 = 0.06;
 
const TOP_BAR_REVEAL_SECS: f32 = 0.42;
 
const TOP_BAR_REVEAL_DROP: f32 = 12.0;
 
const TOP_BAR_COMET_SPEED: f64 = 0.16;
 
const TOP_BAR_COMET_TAIL: f32 = 0.3;
 
const TOP_BAR_CORNER_STEPS: usize = 6;
 
 
 
 
const TOP_BAR_EDGE_STEP: f32 = 1.0;

 
#[derive(Clone, Copy)]
struct TopBarReveal {
    visible: bool,
    changed_at: f64,
}

 
 
fn top_bar_reveal_elapsed(ctx: &Context, visible: bool) -> f32 {
    let id = Id::new("top_bar_reveal");
    let now = ctx.input(|i| i.time);
    let mut state = ctx
        .data(|d| d.get_temp::<TopBarReveal>(id))
        .unwrap_or(TopBarReveal {
            visible: false,
            changed_at: now - 10.0,
        });
    if state.visible != visible {
        state = TopBarReveal {
            visible,
            changed_at: now,
        };
    }
    ctx.data_mut(|d| d.insert_temp(id, state));
    ((now - state.changed_at) as f32 * anim::speed()).max(0.0)
}

 
 
 
fn top_bar_cell_pose(visible: bool, elapsed: f32, index: usize) -> (f32, f32) {
    let k =
        ((elapsed - index as f32 * TOP_BAR_REVEAL_STAGGER) / TOP_BAR_REVEAL_SECS).clamp(0.0, 1.0);
    let pop = if visible {
        Easing::OutBack.apply(k)
    } else {
        1.0 - Easing::In.apply(k)
    };
    (pop.clamp(0.0, 1.0), (1.0 - pop) * -TOP_BAR_REVEAL_DROP)
}

 
pub(crate) fn comet_head(now: f64, index: usize) -> f32 {
    (now * anim::speed() as f64 * TOP_BAR_COMET_SPEED + index as f64 * 0.31).rem_euclid(1.0) as f32
}

 
 
 
fn rounded_outline(rect: Rect, radius: f32) -> Vec<Pos2> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let r = radius.clamp(0.0, rect.width().min(rect.height()) * 0.5);
    let corners = [
        (egui::pos2(rect.left() + r, rect.top() + r), PI),
        (egui::pos2(rect.right() - r, rect.top() + r), PI * 1.5),
        (egui::pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (egui::pos2(rect.left() + r, rect.bottom() - r), FRAC_PI_2),
    ];
    let mut points = Vec::with_capacity(4 * (TOP_BAR_CORNER_STEPS + 1));
    for (center, start) in corners {
        for step in 0..=TOP_BAR_CORNER_STEPS {
            let angle = start + FRAC_PI_2 * step as f32 / TOP_BAR_CORNER_STEPS as f32;
            points.push(center + egui::vec2(angle.cos(), angle.sin()) * r);
        }
    }
    points
}

 
 
 
 
fn rounded_comet_outline(rect: Rect, radius: f32) -> Vec<Pos2> {
    let corner_outline = rounded_outline(rect, radius);
    let mut points = Vec::with_capacity(corner_outline.len() * 4);
    for (index, from) in corner_outline.iter().enumerate() {
        let to = corner_outline[(index + 1) % corner_outline.len()];
        points.push(*from);
        push_edge_points(&mut points, *from, to, TOP_BAR_EDGE_STEP);
    }
    points
}

 
 
 
fn push_edge_points(points: &mut Vec<Pos2>, from: Pos2, to: Pos2, step: f32) {
    let length = from.distance(to);
    if !step.is_finite() || step <= 0.0 || length <= step {
        return;
    }
    let count = (length / step).ceil() as usize;
    for i in 1..count {
        let t = i as f32 / count as f32;
        points.push(from + (to - from) * t);
    }
}

 
 
fn comet_intensity(head: f32, pos: f32, tail: f32) -> f32 {
    let behind = (head - pos).rem_euclid(1.0);
    if behind >= tail {
        return 0.0;
    }
    let fade = 1.0 - behind / tail;
    fade * fade
}

 
 
fn paint_vertical_gradient(
    painter: &Painter,
    rect: Rect,
    outline: &[Pos2],
    top: Color32,
    bottom: Color32,
) {
    let color_at = |y: f32| {
        let t = (y - rect.top()) / rect.height().max(1.0);
        theme::lerp_color(top, bottom, t)
    };
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.center(), color_at(rect.center().y));
    for point in outline {
        mesh.colored_vertex(*point, color_at(point.y));
    }
    let n = outline.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(egui::Shape::mesh(mesh));
}

 
fn paint_horizontal_gradient(painter: &Painter, rect: Rect, left: Color32, right: Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), left);
    mesh.colored_vertex(rect.right_top(), right);
    mesh.colored_vertex(rect.right_bottom(), right);
    mesh.colored_vertex(rect.left_bottom(), left);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

 
 
fn paint_border_comet(
    painter: &Painter,
    outline: &[Pos2],
    head: f32,
    alpha: f32,
    accent: Color32,
    tail: f32,
) {
    let n = outline.len();
    if n < 2 {
        return;
    }
    let lengths: Vec<f32> = (0..n)
        .map(|i| outline[i].distance(outline[(i + 1) % n]))
        .collect();
    let total: f32 = lengths.iter().sum();
    if total <= 0.0 {
        return;
    }
    let mut along = 0.0;
    for (i, length) in lengths.iter().enumerate() {
        let mid = (along + length * 0.5) / total;
        along += length;
        let light = comet_intensity(head, mid, tail);
        if light <= 0.0 {
            continue;
        }
        let core = theme::lerp_color(accent, Color32::WHITE, light.powi(3) * 0.6);
        let color = theme::with_alpha(core, (light * alpha).clamp(0.0, 1.0));
        painter.line_segment(
            [outline[i], outline[(i + 1) % n]],
            Stroke::new(1.5_f32, color),
        );
    }
}

 
 
 
pub(crate) fn draw_top_bar_chip(painter: &Painter, rect: Rect, radius: f32, alpha: f32, head: f32) {
    let round = Rounding::same(radius);
    painter.rect_filled(
        rect.translate(egui::vec2(0.0, 2.0)),
        round,
        Color32::from_black_alpha((60.0 * alpha).round() as u8),
    );

    let outline = rounded_outline(rect, radius);
    let comet_outline = rounded_comet_outline(rect, radius);
    let top = theme::lerp_color(theme::SURFACE, theme::accent(), 0.12);
    let bottom = theme::lerp_color(theme::SURFACE, Color32::BLACK, 0.4);
     
     
    let body = 0.94;
    paint_vertical_gradient(
        painter,
        rect,
        &outline,
        transparency::fill(top, body * alpha, Layer::Chip),
        transparency::fill(bottom, body * alpha, Layer::Chip),
    );
    painter.rect_stroke(
        rect,
        round,
        Stroke::new(1.0_f32, theme::with_alpha(theme::BORDER, alpha)),
    );
     
    transparency::edge(painter, rect, round, Layer::Chip, alpha);
    paint_border_comet(
        painter,
        &comet_outline,
        head,
        alpha,
        theme::accent(),
        TOP_BAR_COMET_TAIL,
    );
}

#[cfg(test)]
mod top_bar_style_tests {
    use super::*;

     
    fn preview_config() -> TopBarConfig {
        TopBarConfig {
            enabled: true,
            height: TOP_BAR_HEIGHT,
            show_brand: true,
            show_wordmark: true,
            show_time: true,
            show_date: true,
            show_fps: true,
            show_ping: true,
            show_direction: true,
            show_coords: true,
            show_speed: true,
            show_module_count: true,
        }
    }

     
     
    fn preview_readings(cfg: &TopBarConfig) -> Vec<(TopBarIcon, &'static str, Color32)> {
        let mut readings = Vec::new();
        if cfg.show_time {
            readings.push((TopBarIcon::Clock, "14:07", theme::TEXT_DIM));
        }
        if cfg.show_date {
            readings.push((TopBarIcon::Date, "Oct 25", theme::TEXT_DIM));
        }
        if cfg.show_fps {
            readings.push((TopBarIcon::Fps, "240", theme::accent()));
        }
        if cfg.show_ping {
            readings.push((TopBarIcon::Ping, "24 ms", theme::SUCCESS));
        }
        if cfg.show_direction {
            readings.push((TopBarIcon::Direction, "North West", theme::TEXT_DIM));
        }
        if cfg.show_coords {
            readings.push((TopBarIcon::Coords, "118 64 -204", theme::TEXT_DIM));
        }
        if cfg.show_speed {
            readings.push((TopBarIcon::Speed, "5.6 b/s", theme::accent()));
        }
        if cfg.show_module_count {
            readings.push((TopBarIcon::Mods, "14", theme::TEXT_DIM));
        }
        readings
    }

    fn preview_bubbles(
        ctx: &Context,
        metrics: &TopBarMetrics,
        cfg: &TopBarConfig,
    ) -> Vec<TopBarBubble> {
        preview_readings(cfg)
            .into_iter()
            .map(|(icon, text, tint)| {
                measure_bubble(ctx, metrics, icon, text.to_string()).tinted(tint)
            })
            .collect()
    }

    #[test]
    fn the_calendar_day_comes_out_of_the_epoch_days() {
        assert_eq!(civil_from_days(0), (1, 1), "1970-01-01");
        assert_eq!(civil_from_days(19_782), (2, 29), "2024-02-29");
        assert_eq!(civil_from_days(20_000), (10, 4), "2024-10-04");
        assert_eq!(civil_from_days(20_751), (10, 25), "2026-10-25");
    }

     
     
     
    #[test]
    fn the_row_lays_its_capsules_out_in_order_without_overlapping() {
        let ctx = Context::default();
         
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 200.0))),
                ..Default::default()
            },
            |_| {},
        );
        let cfg = preview_config();
        let metrics = TopBarMetrics::for_height(cfg.height);
        let bubbles = preview_bubbles(&ctx, &metrics, &cfg);
        let logo = metrics.logo();
        let brand_w = brand_chip_width(&ctx, &cfg, &metrics, logo);
        let origin = egui::pos2(40.0, 12.0);
        let layout = lay_out_top_bar(&metrics, Some(brand_w), &bubbles, origin);

        let mut cells = vec![layout.brand.expect("the brand chip is switched on")];
        cells.extend(layout.bubbles.iter().copied());
        assert_eq!(cells[0].min, origin, "the row starts at its origin");
        for pair in cells.windows(2) {
            assert!(
                pair[0].right() <= pair[1].left() + 1e-3,
                "capsules overlap: {pair:?}"
            );
            assert!(
                (pair[1].left() - pair[0].right() - metrics.spacing).abs() < 1e-3,
                "a capsule is not where the spacing puts it: {pair:?}"
            );
        }
        for rect in &cells {
            assert!((rect.height() - cfg.height).abs() < 1e-3);
            assert!(rect.width() > cfg.height * 0.5, "a capsule fits its type");
        }
         
         
        let mut without = preview_config();
        without.show_wordmark = false;
        let narrow = brand_chip_width(&ctx, &without, &metrics, logo);
        assert!(narrow < brand_w);

         
        let short = measure_bubble(&ctx, &metrics, TopBarIcon::Clock, "1".to_string());
        let long = measure_bubble(&ctx, &metrics, TopBarIcon::Clock, "12:34".to_string());
        assert!(long.width > short.width);
    }

     
     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn top_bar_preview() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let cfg = preview_config();
        let (w, h) = (1440usize, 132usize);
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut shapes = Vec::new();
        let mut laid_out_at = 1.0;
         
         
        for _ in 0..2 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(w as f32, h as f32),
                    )),
                    time: Some(4.5),
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx
                        .layer_painter(LayerId::new(Order::Background, Id::new("top_bar_preview")));
                    for (row, height) in [TOP_BAR_HEIGHT, TOP_BAR_MAX_HEIGHT].iter().enumerate() {
                        let metrics = TopBarMetrics::for_height(*height);
                        let bubbles = preview_bubbles(ctx, &metrics, &cfg);
                        let logo = metrics.logo();
                        let brand_w = brand_chip_width(ctx, &cfg, &metrics, logo);
                        let origin = egui::pos2(18.0, 14.0 + row as f32 * (*height + 14.0));
                        let layout = lay_out_top_bar(&metrics, Some(brand_w), &bubbles, origin);
                        paint_top_bar_row(
                            ctx,
                            &painter,
                            &TopBarRow {
                                cfg: &cfg,
                                metrics: &metrics,
                                layout: &layout,
                                brand: Some(logo),
                                bubbles: &bubbles,
                                visible: true,
                                elapsed: 10.0,
                                pointer: None,
                            },
                        );
                    }
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            shapes = output.shapes;
            laid_out_at = output.pixels_per_point;
        }
        let primitives = ctx.tessellate(shapes, laid_out_at);
        let sheet = rasterize(primitives, &textures, w, h, [0.06, 0.05, 0.05, 1.0]);
        let image = image::imageops::resize(
            &sheet,
            w as u32 * 2,
            h as u32 * 2,
            image::imageops::FilterType::Nearest,
        );
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).expect("the screenshots directory exists");
        let path = dir.join("top-bar.png");
        image.save(&path).expect("the top bar preview saves");
        println!("top bar: {path:?}");
    }

    #[test]
    fn outline_is_a_closed_loop_inside_the_chip() {
        let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), Vec2::new(120.0, 16.0));
        let outline = rounded_outline(rect, 7.0);
        assert_eq!(outline.len(), 4 * (TOP_BAR_CORNER_STEPS + 1));
        for point in &outline {
            assert!(
                rect.expand(0.001).contains(*point),
                "{point:?} outside {rect:?}"
            );
        }
         
        assert!(outline.iter().any(|p| (p.x - rect.left()).abs() < 1e-3));
        assert!(outline.iter().any(|p| (p.x - rect.right()).abs() < 1e-3));
        assert!(outline.iter().any(|p| (p.y - rect.top()).abs() < 1e-3));
        assert!(outline.iter().any(|p| (p.y - rect.bottom()).abs() < 1e-3));
    }

    #[test]
    fn straight_edges_are_subdivided_for_a_smooth_comet() {
        let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), Vec2::new(120.0, 16.0));
        let outline = rounded_comet_outline(rect, 7.0);
        assert!(outline.len() > 4 * (TOP_BAR_CORNER_STEPS + 1));
        let longest = (0..outline.len())
            .map(|i| outline[i].distance(outline[(i + 1) % outline.len()]))
            .fold(0.0_f32, f32::max);
        assert!(
            longest <= TOP_BAR_EDGE_STEP + 1e-3,
            "an unsplit straight edge would stall the comet: {longest}px"
        );

         
         
        let on_top_edge = outline
            .iter()
            .filter(|point| (point.y - rect.top()).abs() < 1e-3)
            .count();
        assert!(
            on_top_edge > 10,
            "only {on_top_edge} points on the top edge"
        );
    }

    #[test]
    fn comet_is_brightest_at_its_head_and_dark_past_its_tail() {
        assert_eq!(comet_intensity(0.5, 0.5, 0.3), 1.0);
        assert_eq!(comet_intensity(0.5, 0.5 - 0.31, 0.3), 0.0);
        assert_eq!(comet_intensity(0.5, 0.9, 0.3), 0.0);
         
        assert!(comet_intensity(0.1, 0.9, 0.3) > 0.0);
        assert!(comet_intensity(0.5, 0.45, 0.3) > comet_intensity(0.5, 0.4, 0.3));
    }

    #[test]
    fn reveal_pops_in_from_above_and_settles_at_rest() {
         
        assert_eq!(top_bar_cell_pose(true, 10.0, 0), (1.0, 0.0));
         
        let (alpha, dy) = top_bar_cell_pose(true, 0.0, 3);
        assert!(alpha < 1e-3, "alpha {alpha}");
        assert!((dy + TOP_BAR_REVEAL_DROP).abs() < 1e-3, "dy {dy}");
         
        let (alpha, _) = top_bar_cell_pose(false, 10.0, 0);
        assert!(alpha < 1e-3, "alpha {alpha}");
    }
}

 
fn draw_top_bar_icon(painter: &Painter, icon: TopBarIcon, center: Pos2, color: Color32, size: f32) {
    let h = size * 0.5;
     
     
    let stroke = Stroke::new((size * 0.13).clamp(1.2, 2.4), color);
    match icon {
        TopBarIcon::Clock => {
            let r = h * 0.82;
            painter.circle_stroke(center, r, stroke);
            painter.line_segment([center, center + egui::vec2(0.0, -r * 0.55)], stroke);
            painter.line_segment([center, center + egui::vec2(r * 0.45, r * 0.08)], stroke);
        }
         
        TopBarIcon::Date => {
            let body = Rect::from_center_size(
                center + egui::vec2(0.0, size * 0.05),
                egui::vec2(size * 0.74, size * 0.64),
            );
            painter.rect_stroke(body, Rounding::same(size * 0.12), stroke);
            let rule = body.top() + body.height() * 0.34;
            painter.line_segment(
                [
                    egui::pos2(body.left(), rule),
                    egui::pos2(body.right(), rule),
                ],
                stroke,
            );
            for dx in [-0.24_f32, 0.24] {
                let x = center.x + body.width() * dx;
                painter.line_segment(
                    [
                        egui::pos2(x, body.top() - size * 0.10),
                        egui::pos2(x, body.top() + size * 0.05),
                    ],
                    stroke,
                );
            }
            painter.rect_filled(
                Rect::from_center_size(
                    egui::pos2(center.x, body.top() + body.height() * 0.68),
                    Vec2::splat(size * 0.13),
                ),
                Rounding::same(size * 0.04),
                color,
            );
        }
         
        TopBarIcon::Ping => {
            let base = center.y + h * 0.85;
            let bar = size * 0.15;
            for (index, tall) in [0.4_f32, 0.68, 1.0].iter().enumerate() {
                let x = center.x + (index as f32 - 1.0) * size * 0.30;
                painter.rect_filled(
                    Rect::from_min_max(
                        egui::pos2(x - bar * 0.5, base - size * 0.85 * tall),
                        egui::pos2(x + bar * 0.5, base),
                    ),
                    Rounding::same(bar * 0.45),
                    color,
                );
            }
        }
        TopBarIcon::Fps => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    center + egui::vec2(0.05 * h, -h),
                    center + egui::vec2(-0.55 * h, 0.05 * h),
                    center + egui::vec2(-0.1 * h, 0.05 * h),
                    center + egui::vec2(-0.05 * h, h),
                    center + egui::vec2(0.55 * h, -0.05 * h),
                    center + egui::vec2(0.1 * h, -0.05 * h),
                ],
                color,
                Stroke::NONE,
            ));
        }
        TopBarIcon::Direction => {
            painter.line_segment(
                [
                    center + egui::vec2(0.0, h * 0.55),
                    center + egui::vec2(0.0, -h * 0.6),
                ],
                stroke,
            );
            painter.add(egui::Shape::convex_polygon(
                vec![
                    center + egui::vec2(-h * 0.42, -h * 0.1),
                    center + egui::vec2(h * 0.42, -h * 0.1),
                    center + egui::vec2(0.0, -h * 0.9),
                ],
                color,
                Stroke::NONE,
            ));
        }
         
        TopBarIcon::Coords => {
            let r = h * 0.52;
            painter.circle_stroke(center, r, stroke);
            for dir in [
                egui::vec2(0.0, -1.0),
                egui::vec2(0.0, 1.0),
                egui::vec2(-1.0, 0.0),
                egui::vec2(1.0, 0.0),
            ] {
                painter.line_segment(
                    [
                        center + dir * (r + stroke.width * 1.2),
                        center + dir * (h * 0.98),
                    ],
                    stroke,
                );
            }
            painter.circle_filled(center, (size * 0.07).max(1.2), color);
        }
         
        TopBarIcon::Speed => {
            let r = h * 0.8;
            let mut previous: Option<Pos2> = None;
            for step in 0..=12 {
                let angle = std::f32::consts::PI * (1.0 + step as f32 / 12.0);
                let point = center + egui::vec2(angle.cos() * r, angle.sin() * r);
                if let Some(from) = previous {
                    painter.line_segment([from, point], stroke);
                }
                previous = Some(point);
            }
            let hub = center + egui::vec2(0.0, r * 0.12);
            painter.line_segment([hub, hub + egui::vec2(r * 0.6, -r * 0.6)], stroke);
            painter.circle_filled(hub, (size * 0.08).max(1.3), color);
        }
        TopBarIcon::Mods => {
            let s = h * 0.46;
            for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                painter.rect_filled(
                    Rect::from_center_size(center + egui::vec2(dx * s, dy * s), egui::vec2(s, s)),
                    Rounding::same(1.3),
                    color,
                );
            }
        }
    }
}

 
 
pub(crate) fn top_bar_wants_input() -> bool {
    top_bar_config().is_some_and(|cfg| cfg.enabled)
        && crate::state::try_minecraft().is_some()
        && crate::graphic::input::minecraft_screen_open()
}

 

fn preview_rows(id: HudWidgetId) -> Vec<StatRow> {
    match id {
        HudWidgetId::Fps => vec![StatRow::new("FPS", "120")],
        HudWidgetId::Coords => vec![StatRow::new("XYZ", "100.0 / 64.0 / 100.0")],
         
         
        HudWidgetId::Clock => Vec::new(),
         
         
        HudWidgetId::Speed => Vec::new(),
        HudWidgetId::Session => vec![StatRow::new("Session", "0:42:13")],
         
        HudWidgetId::Ping => Vec::new(),
        HudWidgetId::TargetInfo => Vec::new(),
        HudWidgetId::ArmorStatus => Vec::new(),
         
         
        HudWidgetId::PotionStatus => Vec::new(),
        HudWidgetId::Radar
        | HudWidgetId::Compass
        | HudWidgetId::KeyStrokes
        | HudWidgetId::Status
        | HudWidgetId::TotemCounter
        | HudWidgetId::Spotify
        | HudWidgetId::Lyrics => Vec::new(),
        HudWidgetId::PlayerEsp => vec![StatRow::new("PlayerName", "12.4m")],
    }
}

 
 
pub(crate) fn widget_content_size(id: HudWidgetId, ctx: &Context) -> Vec2 {
    let style = UiStyle::hud_style();
    match id {
        HudWidgetId::Radar => Vec2::new(120.0, 120.0),
        HudWidgetId::Compass => crate::graphic::compass::SIZE,
        HudWidgetId::Clock => {
            crate::graphic::clock::size(ctx, &crate::graphic::clock::ClockConfig::read())
        }
        HudWidgetId::Speed => crate::graphic::speedometer::size(style.scale),
        HudWidgetId::Ping => crate::graphic::ping::size(style.scale),
        HudWidgetId::KeyStrokes => Vec2::splat(98.0) * style.scale,
        HudWidgetId::TotemCounter => Vec2::new(TOTEM_W, TOTEM_H) * style.scale,
        HudWidgetId::Status => status_card_size(&StatusConfig::read(), &style),
        HudWidgetId::PotionStatus => {
            let effects = potion_preview_effects();
            potion_card_size(effects.len(), false, style.scale)
        }
        HudWidgetId::ArmorStatus => Vec2::new(ARMOR_W, armor_height()) * style.scale,
        HudWidgetId::PlayerEsp => Vec2::new(198.0, 52.0),
        HudWidgetId::TargetInfo => crate::graphic::target_info::panel_size(),
        HudWidgetId::Spotify => spotify_card_size(&SpotifyStyle::read()),
        HudWidgetId::Lyrics => crate::graphic::lyrics_card::saved_size(),
        _ => {
            let rows = preview_rows(id);
            let line = style.line_height();
            let font = FontId::proportional(style.text_size * style.scale);
            let columns = stat_columns(ctx, &style, &font, &rows);
            let width = columns.total(style.scale) + style.padding * style.scale * 2.0;
            Vec2::new(width, line * rows.len().max(1) as f32)
        }
    }
}

 
 
 
pub(crate) fn draw_widget_preview(ctx: &Context, painter: &Painter, id: HudWidgetId, rect: Rect) {
    match id {
        HudWidgetId::Radar => draw_radar_at(ctx, painter, rect.min),
        HudWidgetId::Compass => {
             
             
            crate::graphic::compass::draw(painter, rect.min, 240.0, "North East");
        }
        HudWidgetId::KeyStrokes => draw_key_strokes_at(ctx, painter, rect.min),
        HudWidgetId::Spotify => draw_spotify_card(
            ctx,
            painter,
            rect.min,
            &SpotifyCard {
                title: "Song Title".into(),
                artist: "Artist Name".into(),
                elapsed: std::time::Duration::from_secs(83),
                duration: std::time::Duration::from_secs(225),
                art: None,
                motion: SpotifyMotion::settled(83.0 / 225.0),
            },
            &SpotifyStyle::read(),
        ),
        HudWidgetId::Lyrics => crate::graphic::lyrics_card::draw_preview(ctx, painter, rect.min),
        HudWidgetId::Clock => crate::graphic::clock::draw_preview(
            painter,
            rect.min,
            &crate::graphic::clock::ClockConfig::read(),
        ),
        HudWidgetId::Speed => crate::graphic::speedometer::draw_preview(painter, rect.min),
        HudWidgetId::Ping => crate::graphic::ping::draw_preview(painter, rect.min),
        HudWidgetId::TotemCounter => {
             
             
            draw_totem_counter_at(
                painter,
                rect.min,
                TOTEM_PREVIEW_COUNT,
                totem_art(ctx),
                TotemMotion::settled(TOTEM_PREVIEW_COUNT),
            );
        }
        HudWidgetId::Status => draw_status_preview(ctx, painter, rect.min),
        HudWidgetId::ArmorStatus => draw_armor_preview(ctx, painter, rect.min),
        HudWidgetId::PlayerEsp => {
            painter.rect_filled(rect, Rounding::same(5.0), Color32::from_black_alpha(190));
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Player ESP",
                FontId::proportional(12.0),
                theme::TEXT,
            );
        }
        HudWidgetId::TargetInfo => {
            crate::graphic::target_info::draw_preview(ctx, painter, rect.min);
        }
        HudWidgetId::PotionStatus => {
            let effects = potion_preview_effects();
            let motion = PotionMotion::settled(&effects);
            draw_potion_card(
                painter,
                egui::pos2(rect.min.x, rect.max.y),
                &effects,
                motion,
            );
        }
        _ => {
            let style = UiStyle::hud_style();
            let origin = egui::pos2(rect.min.x, rect.max.y - style.line_height());
            draw_text_widget_at(ctx, painter, origin, preview_rows(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spotify_entrance_settles_and_the_art_overshoots_once() {
        assert_eq!(spotify_enter(0.0), 0.0);
        assert!((spotify_enter(SPOTIFY_ENTER_SECS) - 1.0).abs() < 1e-6);
        assert_eq!(spotify_enter(60.0), 1.0);
        assert!(spotify_enter(SPOTIFY_ENTER_SECS * 0.3) > 0.5);
        assert!(spotify_art_pop(0.0).abs() < 1e-6);
        assert!((spotify_art_pop(60.0) - 1.0).abs() < 1e-6);
        let peak = (0..=100)
            .map(|i| spotify_art_pop(i as f32 / 100.0 * SPOTIFY_ENTER_SECS * 1.3))
            .fold(0.0_f32, f32::max);
        assert!(peak > 1.0, "art should overshoot, peaked at {peak}");
    }

    #[test]
    fn marquee_rests_at_both_ends_and_scrolls_between() {
         
        let at = |t: f32| marquee_offset(100.0, t, 50.0);
        assert_eq!(at(0.0), 0.0);
        assert_eq!(at(SPOTIFY_MARQUEE_PAUSE - 0.1), 0.0);
        assert!((at(SPOTIFY_MARQUEE_PAUSE + 1.0) - 50.0).abs() < 1e-3);
        assert_eq!(at(4.0), 100.0);
        assert!((at(7.0) - 50.0).abs() < 1e-3);
        assert!(at(8.1) < 1e-2, "cycle wraps back to the start");
        assert_eq!(marquee_offset(-5.0, 3.0, 50.0), 0.0);
    }

    #[test]
    fn uptime_reads_as_hours_minutes_and_seconds() {
        assert_eq!(format_uptime(0), "0:00:00");
        assert_eq!(format_uptime(59), "0:00:59");
        assert_eq!(format_uptime(61), "0:01:01");
        assert_eq!(format_uptime(3_725), "1:02:05");
        assert_eq!(format_uptime(86_400), "24:00:00");
    }

    #[test]
    fn every_text_widget_has_a_preview_row() {
         
         
        for id in [HudWidgetId::Session, HudWidgetId::Fps, HudWidgetId::Coords] {
            assert!(!preview_rows(id).is_empty(), "{id:?} has no preview");
        }
    }

    #[test]
    fn track_fraction_stays_in_range() {
        let secs = std::time::Duration::from_secs;
        assert_eq!(track_fraction(secs(30), std::time::Duration::ZERO), 0.0);
        assert_eq!(track_fraction(secs(30), secs(60)), 0.5);
        assert_eq!(track_fraction(secs(90), secs(60)), 1.0);
    }

    #[test]
    fn spotify_clock_pairs_elapsed_and_length() {
        let secs = std::time::Duration::from_secs;
        assert_eq!(spotify_clock(secs(83), secs(225)), "1:23 / 3:45");
        assert_eq!(
            spotify_clock(secs(83), std::time::Duration::ZERO),
            "1:23 / --:--"
        );
        assert_eq!(spotify_clock(secs(3725), secs(4000)), "1:02:05 / 1:06:40");
    }

    #[test]
    fn spotify_card_fits_its_rows_and_the_bar_inside_the_padding() {
        let style = SpotifyStyle::default();
        let rows = spotify_rows(&style);
        assert_eq!(rows.title_y, style.padding);
        assert!(
            (rows.time_y - (rows.artist_y + style.artist_size * SPOTIFY_ARTIST_LINE * 0.5)).abs()
                < 1e-4
        );
        assert_eq!(rows.clock_row, SpotifyClockRow::Artist);
        assert!((rows.height - (rows.content + style.padding * 2.0)).abs() < 1e-4);
        assert!(
            rows.content >= style.art_size,
            "art must never overflow the card"
        );
        let bar_bottom = rows.bar_y + style.progress_height;
        assert!(bar_bottom <= style.padding + rows.content + 1e-4);
        assert!(bar_bottom + style.padding <= rows.height + 1e-4);
        assert!(rows.artist_y >= rows.title_y + style.title_size);

         
        let flat = SpotifyStyle {
            show_art: false,
            ..style
        };
        assert!(spotify_rows(&flat).height < rows.height);

         
        let empty = SpotifyStyle {
            show_art: false,
            show_title: false,
            show_artist: false,
            show_progress: false,
            ..style
        };
        let empty = spotify_rows(&empty);
        assert_eq!(empty.content, SPOTIFY_MIN_CONTENT);
        assert_eq!(empty.clock_row, SpotifyClockRow::None);
        assert!((empty.height - (SPOTIFY_MIN_CONTENT + style.padding * 2.0)).abs() < 1e-4);

         
        let title_only = SpotifyStyle {
            show_artist: false,
            ..style
        };
        let title_only = spotify_rows(&title_only);
        assert_eq!(title_only.clock_row, SpotifyClockRow::Title);
        assert!(
            (title_only.time_y
                - (title_only.title_y + style.title_size * SPOTIFY_TITLE_LINE * 0.5))
                .abs()
                < 1e-4
        );
    }

    #[test]
    fn rounded_art_mesh_cuts_its_corners_and_maps_the_texture() {
        let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), Vec2::new(46.0, 46.0));
        let mesh = rounded_image_mesh(rect, 8.0, Color32::WHITE);
        let ring = 4 * (TOP_BAR_CORNER_STEPS + 1);
        assert_eq!(mesh.vertices.len(), 1 + ring);
        assert_eq!(mesh.indices.len(), 3 * ring);

        let centre = mesh.vertices[0];
        assert!((centre.uv.x - 0.5).abs() < 1e-4 && (centre.uv.y - 0.5).abs() < 1e-4);
        for vertex in &mesh.vertices {
            assert!(vertex.pos.x >= rect.min.x - 1e-3 && vertex.pos.x <= rect.max.x + 1e-3);
            assert!(vertex.pos.y >= rect.min.y - 1e-3 && vertex.pos.y <= rect.max.y + 1e-3);
            assert!((0.0..=1.0).contains(&vertex.uv.x));
            assert!((0.0..=1.0).contains(&vertex.uv.y));
             
            let on_x = (vertex.pos.x - rect.min.x).abs() < 1e-3
                || (vertex.pos.x - rect.max.x).abs() < 1e-3;
            let on_y = (vertex.pos.y - rect.min.y).abs() < 1e-3
                || (vertex.pos.y - rect.max.y).abs() < 1e-3;
            assert!(
                !(on_x && on_y),
                "vertex at a sharp corner: {:?}",
                vertex.pos
            );
        }
         
        for triangle in mesh.indices.chunks(3) {
            assert_eq!(triangle[0], 0);
            assert!(triangle[1] > 0 && triangle[2] > 0);
        }

         
        let empty = rounded_image_mesh(
            Rect::from_min_size(Pos2::ZERO, Vec2::ZERO),
            4.0,
            Color32::WHITE,
        );
        assert!(empty.is_empty());
    }

    #[test]
    fn smaller_watermark_keeps_its_corner_and_arraylist_spacing() {
        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0));
        let mut bottom = 0.0;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                let painter =
                    ctx.layer_painter(LayerId::new(Order::Background, Id::new("watermark_test")));
                bottom = draw_watermark(ctx, &painter, LOGO_H, 1.0, MARGIN);
            },
        );
        let expected = watermark_rect(
            screen,
            crate::graphic::branding::logo_size(),
            LOGO_H,
            MARGIN,
        );
        assert_eq!(expected.height(), LOGO_H);
        assert_eq!(expected.right(), screen.right() - MARGIN);
        assert_eq!(expected.top(), screen.top() + MARGIN);
        assert_eq!(bottom, expected.bottom() + 6.0);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Rect(rect) if rect.rect == expected
                && rect.uv == Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0))
                && rect.fill == theme::brand()
        )));
        assert!((LOGO_H / 180.0 - 3.0 / 4.0).abs() < 1e-6);
    }

    #[test]
    fn module_list_logo_geometry_follows_its_settings() {
        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0));
        let size = LOGO_H * 0.5;
        let margin = 32.0;
        let mut bottom = 0.0;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                let painter =
                    ctx.layer_painter(LayerId::new(Order::Background, Id::new("logo_test")));
                bottom = draw_watermark(ctx, &painter, size, 0.5, margin);
            },
        );
        let expected = watermark_rect(screen, crate::graphic::branding::logo_size(), size, margin);
        assert_eq!(expected.height(), size);
        assert_eq!(expected.right(), screen.right() - margin);
        assert_eq!(bottom, expected.bottom() + 6.0);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Rect(rect) if rect.rect == expected
                && rect.fill == theme::with_alpha(theme::brand(), 0.5)
        )));
    }

    fn assert_offset(dx: f64, dz: f64, yaw: f32, expected: Vec2) {
        let actual = radar_offset(dx, dz, yaw, 10.0, 40.0).unwrap();
        assert!(
            (actual - expected).length() < 1e-4,
            "yaw {yaw}, ({dx}, {dz}): expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn potion_rows_show_levels_and_tick_durations() {
        let effect = EffectStatus {
            id: "speed".into(),
            name: "Speed".into(),
            amplifier: 1,
            duration: 1200,
            longest: 1200,
        };
        assert_eq!(effect.level(), "II");
        assert_eq!(effect.time_text(), "1:00");
        assert!((effect.fraction() - 1.0).abs() < 1e-6);
        assert!(!effect.expiring());

        let draining = EffectStatus {
            duration: 300,
            ..effect.clone()
        };
        assert_eq!(draining.time_text(), "0:15");
        assert!((draining.fraction() - 0.25).abs() < 1e-6);
        assert!(!draining.expiring());

        let urgent = EffectStatus {
            duration: 100,
            ..effect.clone()
        };
        assert!(urgent.expiring());

        let infinite = EffectStatus {
            duration: -1,
            longest: -1,
            ..effect.clone()
        };
        assert_eq!(infinite.time_text(), "∞");
        assert!((infinite.fraction() - 1.0).abs() < 1e-6);
        assert!(!infinite.expiring());

        let nearly_over = EffectStatus {
            duration: 1,
            ..effect.clone()
        };
        assert_eq!(nearly_over.time_text(), "0:01");

        let broken = EffectStatus {
            longest: 0,
            duration: 40,
            ..effect
        };
        assert_eq!(broken.fraction(), 0.0);
    }

    #[test]
    fn potion_rows_sort_longest_first_and_keep_infinite_on_top() {
        let make = |id: &str, duration: i32, longest: i32| EffectStatus {
            id: id.into(),
            name: title_case(id),
            amplifier: 0,
            duration,
            longest,
        };
        let mut rows = vec![
            make("speed", 120, 120),
            make("night_vision", -1, -1),
            make("fire_resistance", 900, 1800),
            make("regeneration", 300, 600),
        ];
        sort_effect_rows(&mut rows);
        let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(
            ids,
            ["night_vision", "fire_resistance", "regeneration", "speed"]
        );
    }

    #[test]
    fn potion_motion_replays_only_when_the_effects_change() {
        let make = |id: &str, amplifier: i32, duration: i32| EffectStatus {
            id: id.into(),
            name: title_case(id),
            amplifier,
            duration,
            longest: 1200,
        };
        let mut motion = PotionMotion::default();
        let rows = vec![make("speed", 1, 1200)];
        motion.advance(&rows, 0.5);
        assert!((motion.age - 0.5).abs() < 1e-6);
        assert!(motion.intro(0) > 0.9);

         
        motion.advance(&vec![make("speed", 1, 1000)], 0.1);
        assert!((motion.age - 0.6).abs() < 1e-6);

         
        motion.advance(&vec![make("speed", 2, 1000)], 0.1);
        assert!(motion.age < 0.11);
        assert!(motion.intro(2) < 0.5);
        assert_eq!(motion.intro(3), 0.0);
        assert!(motion.animating());
    }

    #[test]
    fn potion_card_grows_upward_and_caps_its_rows() {
        assert!(potion_card_height(1, false, 1.0) < potion_card_height(4, false, 1.0));
         
        assert_eq!(
            potion_card_height(POTION_MAX_ROWS, false, 1.0),
            potion_card_height(POTION_MAX_ROWS + 5, false, 1.0)
        );
        assert!(
            potion_card_height(POTION_MAX_ROWS, true, 1.0)
                > potion_card_height(POTION_MAX_ROWS, false, 1.0)
        );
        assert_eq!(potion_card_size(0, false, 2.0).x, POTION_W * 2.0);
        let preview = potion_preview_effects();
        assert_eq!(preview.len(), 4);
        assert!(preview[0].infinite());
        assert_eq!(preview[0].id, "night_vision");
    }

     
     
    #[test]
    fn the_potion_card_preview_renders_tinted_icon_rows() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::{HashMap, HashSet};

        let (width, height) = (240usize, 200usize);
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        let effects = potion_preview_effects();
        let motion = PotionMotion::settled(&effects);
        for frame in 0..4 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx
                        .layer_painter(LayerId::new(Order::Background, Id::new("potion_preview")));
                    draw_potion_card(&painter, egui::pos2(20.0, 190.0), &effects, motion);
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
        let image = rasterize(
            primitives.clone(),
            &textures,
            width,
            height,
            [0.06, 0.06, 0.075, 1.0],
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/potion-card-preview.png");
        image.save(&path).expect("the potion preview saves");

        let lit = image
            .pixels()
            .filter(|pixel| {
                pixel[3] > 128 && pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32 > 120
            })
            .count();
        assert!(lit > 2000, "the card should be visibly lit, got {lit}");

         
         
        let mut hues = HashSet::new();
        for pixel in image.pixels() {
            if pixel[3] < 160 {
                continue;
            }
            let (r, g, b) = (pixel[0], pixel[1], pixel[2]);
            let high = r.max(g).max(b) as i32;
            let low = r.min(g).min(b) as i32;
            if high - low > 40 {
                hues.insert((r / 32, g / 32, b / 32));
            }
        }
        assert!(
            hues.len() >= 4,
            "expected distinct effect accents, got {hues:?}"
        );

         
        for frame in 0..3 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(LayerId::new(
                        Order::Background,
                        Id::new("potion_empty_preview"),
                    ));
                    draw_potion_card(
                        &painter,
                        egui::pos2(20.0, 190.0),
                        &[],
                        PotionMotion::default(),
                    );
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
        let empty = rasterize(
            primitives,
            &textures,
            width,
            height,
            [0.06, 0.06, 0.075, 1.0],
        );
        empty
            .save(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../target/potion-card-empty.png"),
            )
            .expect("the empty potion preview saves");
    }

    #[test]
    fn radar_cardinal_headings_keep_forward_up_and_right_right() {
        for (yaw, forward, right) in [
            (0.0, (0.0, 5.0), (-5.0, 0.0)),
            (90.0, (-5.0, 0.0), (0.0, -5.0)),
            (180.0, (0.0, -5.0), (5.0, 0.0)),
            (-90.0, (5.0, 0.0), (0.0, 5.0)),
        ] {
            assert_offset(forward.0, forward.1, yaw, egui::vec2(0.0, -20.0));
            assert_offset(-forward.0, -forward.1, yaw, egui::vec2(0.0, 20.0));
            assert_offset(right.0, right.1, yaw, egui::vec2(20.0, 0.0));
            assert_offset(-right.0, -right.1, yaw, egui::vec2(-20.0, 0.0));
        }
    }

    #[test]
    fn radar_diagonal_headings_and_wrapped_yaw_preserve_distance() {
        for yaw in [-405.0, -45.0, 45.0, 135.0, 405.0] {
            let (sin, cos) = f64::from(yaw).to_radians().sin_cos();
            assert_offset(-5.0 * sin, 5.0 * cos, yaw, egui::vec2(0.0, -20.0));
            let offset = radar_offset(3.0, 4.0, yaw, 10.0, 40.0).unwrap();
            assert!((offset.length() - 20.0).abs() < 1e-4);
        }
    }

    #[test]
    fn radar_clips_to_a_circle_and_rejects_invalid_input() {
        assert!(radar_offset(8.0, 8.0, 0.0, 10.0, 40.0).is_none());
        assert_eq!(radar_offset(0.0, 0.0, 0.0, 10.0, 40.0), Some(Vec2::ZERO));
        assert_offset(6.0, 8.0, 0.0, egui::vec2(-24.0, -32.0));
        for (dx, dz, yaw, range) in [
            (f64::NAN, 0.0, 0.0, 10.0),
            (0.0, f64::INFINITY, 0.0, 10.0),
            (0.0, 0.0, f32::NAN, 10.0),
            (0.0, 0.0, 0.0, 0.0),
            (0.0, 0.0, 0.0, -10.0),
            (0.0, 0.0, 0.0, f64::NAN),
        ] {
            assert!(radar_offset(dx, dz, yaw, range, 40.0).is_none());
        }
    }

    #[test]
    fn hud_look_vector_matches_minecraft_yaw_and_pitch() {
        for yaw in [0.0, 90.0, 180.0, -90.0, 45.0] {
            let look = look_vector(yaw, 0.0);
            let offset = radar_offset(look.0, look.2, yaw, 2.0, 40.0).unwrap();
            assert!((offset - egui::vec2(0.0, -20.0)).length() < 1e-4);
        }
        let down = look_vector(0.0, 90.0);
        assert!((down.1 + 1.0).abs() < 1e-9);
        assert!(down.0.abs() < 1e-9 && down.2.abs() < 1e-9);
    }

    #[test]
    fn totem_slots_cover_the_carried_inventory_only() {
        let slots: Vec<i32> = totem_slots().collect();
         
        assert_eq!(
            slots,
            (0..36).chain(std::iter::once(40)).collect::<Vec<_>>()
        );
        assert_eq!(slots.len(), 37);
        let mut unique = slots.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), slots.len(), "slots must not repeat");
         
        for slot in 36..40 {
            assert!(!slots.contains(&slot), "armor slot {slot} must be skipped");
        }
        assert!(slots.contains(&Inventory::SLOT_OFFHAND));
    }

    #[test]
    fn totem_count_turns_to_danger_only_when_empty() {
        let style = UiStyle::default();
        assert_eq!(totem_count_color(0, &style), theme::DANGER);
        assert_eq!(totem_count_color(-1, &style), theme::DANGER);
        assert_eq!(totem_count_color(1, &style), style.text);
        assert_eq!(totem_count_color(2304, &style), style.text);
    }

     
     
     
     
    #[test]
    fn hotbar_slot_one_lands_where_vanilla_draws_it() {
        let hd = hotbar_first_slot_in(GuiMetrics {
            scale: 3.0,
            scaled_width: 640,
            scaled_height: 360,
        })
        .unwrap();
        assert!((hd.min.x - 696.0).abs() < 1e-3, "was {}", hd.min.x);
        assert!((hd.width() - 60.0).abs() < 1e-3);
         
        assert!((hd.min.y - ((360.0 - 22.0 + 3.0) * 3.0)).abs() < 1e-3);

         
         
         
        let uhd = hotbar_first_slot_in(GuiMetrics {
            scale: 4.0,
            scaled_width: 960,
            scaled_height: 540,
        })
        .unwrap();
        assert!((uhd.min.x - (960.0 / 2.0 - 91.0 + 3.0) * 4.0 / 3.0).abs() < 1e-3);
        assert!((uhd.width() - 20.0 * 4.0 / 3.0).abs() < 1e-3);

         
        for metrics in [
            GuiMetrics {
                scale: 0.0,
                scaled_width: 640,
                scaled_height: 360,
            },
            GuiMetrics {
                scale: f64::NAN,
                scaled_width: 640,
                scaled_height: 360,
            },
            GuiMetrics {
                scale: 3.0,
                scaled_width: 640,
                scaled_height: 0,
            },
        ] {
            assert!(hotbar_first_slot_in(metrics).is_none());
        }
    }

    #[test]
    fn totem_counter_sits_just_left_of_the_first_slot() {
         
         
        let slot = hotbar_first_slot_in(GuiMetrics {
            scale: 3.0,
            scaled_width: 640,
            scaled_height: 360,
        })
        .unwrap();
        let panel = Rect::from_min_size(
            egui::pos2(
                slot.min.x - TOTEM_GAP - TOTEM_W,
                slot.center().y - TOTEM_H * 0.5,
            ),
            Vec2::new(TOTEM_W, TOTEM_H),
        );
        assert!((panel.max.x + TOTEM_GAP - slot.min.x).abs() < 1e-3);
        assert!((panel.center().y - slot.center().y).abs() < 1e-3);
    }

     
     
    #[test]
    fn totem_counter_paints_the_art_and_the_count_inside_its_panel() {
        let ctx = Context::default();
        let origin = egui::pos2(100.0, 40.0);
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0))),
                ..Default::default()
            },
            |ctx| {
                let painter =
                    ctx.layer_painter(LayerId::new(Order::Background, Id::new("totem_test")));
                let handle = ctx.load_texture(
                    "totem_test_icon",
                    egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255, 255, 255, 255]),
                    egui::TextureOptions::NEAREST,
                );
                draw_totem_counter_at(
                    &painter,
                    origin,
                    12,
                    Some(handle.id()),
                    TotemMotion::settled(12),
                );
                 
                draw_totem_counter_at(
                    &painter,
                    origin + Vec2::new(0.0, 48.0),
                    0,
                    None,
                    TotemMotion::settled(0),
                );
            },
        );

        let style = UiStyle::default();
        let panel_fill = style.background;
        let mut panels = Vec::new();
        let mut tiles = Vec::new();
        let mut texts = Vec::new();
        for shape in &output.shapes {
            match &shape.shape {
                egui::Shape::Rect(rect) if rect.fill == panel_fill => panels.push(rect.rect),
                 
                egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default() => {
                    tiles.push(mesh.clone())
                }
                egui::Shape::Text(text) => texts.push((
                    text.galley.text().to_owned(),
                    text.fallback_color,
                    text.pos,
                    text.galley.size().x,
                )),
                _ => {}
            }
        }

        let panel = Rect::from_min_size(origin, Vec2::new(TOTEM_W, TOTEM_H));
        assert_eq!(panels.len(), 2, "one panel per counter drawn");
        assert!(panels.contains(&panel));
        assert!(panels.contains(&(panel.translate(Vec2::new(0.0, 48.0)))));

         
         
        assert_eq!(tiles.len(), 1, "only the textured counter draws art");
        let art = &tiles[0];
        let texels = crate::graphic::item_icon::ICON;
        assert_eq!(art.vertices.len(), texels * texels * 4);
        let mut bounds = Rect::NOTHING;
        for vertex in &art.vertices {
            bounds = bounds.union(Rect::from_min_size(vertex.pos, Vec2::ZERO));
        }
        let plate = Rect::from_center_size(
            egui::pos2(panel.min.x + TOTEM_PAD + TOTEM_ART * 0.5, panel.center().y),
            Vec2::splat(TOTEM_ART),
        );
         
         
        assert!(
            (bounds.center().x - plate.center().x).abs() < 0.01
                && (bounds.center().y - plate.center().y).abs() <= 2.0,
            "the art must sit on the plate: {bounds:?} vs {plate:?}"
        );
        assert!((bounds.width() - TOTEM_ART).abs() < 0.01);
         
         
        for vertex in &art.vertices {
            let cell = vertex.uv.x * texels as f32 - 0.5;
            assert!(
                (cell - cell.round()).abs() < 1e-4,
                "uv {vertex:?} is off centre"
            );
        }

         
        let text_left = plate.max.x + TOTEM_TEXT_GAP;
        let find = |label: &str| {
            texts
                .iter()
                .find(|(text, _, _, _)| text == label)
                .unwrap_or_else(|| panic!("no {label} text drawn"))
        };
        let (_, color, pos, _) = find("12");
        assert_eq!(*color, style.text, "a carried count is ordinary HUD text");
        assert!(
            (pos.x - text_left).abs() < 1e-3,
            "the count starts at the copy column"
        );
        assert!(pos.x > plate.max.x, "the count overlaps the art");

        let caption = find("TOTEMS");
        assert!(caption.2.y > pos.y, "the caption sits under the number");

        let (_, color, pos, _) = find("0");
        assert_eq!(*color, theme::DANGER);
        let empty_caption = find("EMPTY");
        assert_eq!(empty_caption.1, theme::DANGER);
        assert!((empty_caption.2.x - pos.x).abs() < 1e-3);

         
         
        let (label, _, pos, width) = find("T");
        assert_eq!(label, "T");
        assert!((pos.x + width * 0.5 - plate.center().x).abs() < 1e-3);
    }

     
     
    #[test]
    fn the_totem_number_rolls_to_the_new_count() {
        let mut roll = TotemRoll::default();
        roll.advance(12, 10.0);
        assert_eq!(roll.shown, 0.0, "the first count is where the roll starts");

        roll.advance(12, 10.0 + TOTEM_ROLL * 0.5);
        let middle = roll.shown;
        assert!(
            middle > 0.0 && middle < 12.0,
            "half way through a gain, the number is between the two counts: {middle}"
        );
        assert!(!roll.loss, "gaining totems is not a loss");

        roll.advance(12, 10.0 + TOTEM_ROLL);
        assert!((roll.shown - 12.0).abs() < 1e-3, "the roll lands exactly");

        let landed_at = 11.0;
        roll.advance(11, landed_at);
        assert!(roll.loss, "giving one back is a loss");
        roll.advance(11, landed_at + TOTEM_ROLL * 0.5);
        assert!(roll.shown <= 12.0 && roll.shown >= 11.0);
        roll.advance(11, landed_at + TOTEM_ROLL);
        assert!((roll.shown - 11.0).abs() < 1e-3);
    }

     
     
     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn totem_counter_preview() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

         
        let (w, h) = (472usize, 118usize);
        let mut columns: Vec<(i32, TotemMotion, String)> = Vec::new();
        for step in 0..5 {
            let t = step as f32 / 4.0;
            columns.push((
                9,
                TotemMotion {
                    value: 3.0 + 6.0 * Easing::Out.apply(t),
                    t,
                    loss: false,
                },
                format!("roll {:.2}s", t * TOTEM_ROLL as f32),
            ));
        }
        columns.push((12, TotemMotion::settled(12), "stocked".into()));
        columns.push((1, TotemMotion::settled(1), "last one".into()));
        columns.push((
            13,
            TotemMotion {
                value: 12.4,
                t: 0.18,
                loss: false,
            },
            "gained".into(),
        ));
        columns.push((
            12,
            TotemMotion {
                value: 12.6,
                t: 0.18,
                loss: true,
            },
            "lost one".into(),
        ));
        columns.push((
            0,
            TotemMotion {
                value: 0.0,
                t: 0.55,
                loss: true,
            },
            "out".into(),
        ));

        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut shapes = Vec::new();
        let mut laid_out_at = 1.0;
         
        for _ in 0..2 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(w as f32, h as f32),
                    )),
                    time: Some(1.25),
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx
                        .layer_painter(LayerId::new(Order::Background, Id::new("totem_preview")));
                    let art = bundled_totem_art(ctx);
                    for (index, (count, motion, label)) in columns.iter().enumerate() {
                        let origin = egui::pos2(
                            8.0 + (index % 5) as f32 * 94.0,
                            8.0 + (index / 5) as f32 * 56.0,
                        );
                        draw_totem_counter_at(&painter, origin, *count, art, *motion);
                        painter.text(
                            origin + egui::vec2(2.0, TOTEM_H + 3.0),
                            Align2::LEFT_TOP,
                            label,
                            FontId::proportional(7.5),
                            theme::TEXT_MUTED,
                        );
                    }
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            shapes = output.shapes;
            laid_out_at = output.pixels_per_point;
        }
        let primitives = ctx.tessellate(shapes, laid_out_at);
        let sheet = rasterize(primitives, &textures, w, h, [0.05, 0.04, 0.04, 1.0]);
         
        let image = image::imageops::resize(
            &sheet,
            w as u32 * 2,
            h as u32 * 2,
            image::imageops::FilterType::Nearest,
        );
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).expect("the screenshots directory exists");
        let path = dir.join("totem-counter.png");
        image.save(&path).expect("the totem preview saves");
        println!("totem counter: {path:?}");
    }

     
     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn armor_status_preview() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let piece = |id: &str, durability: f32| {
            Some(ArmorPiece {
                name: title_case(id),
                id: id.to_string(),
                durability,
            })
        };
         
        let worn = [
            piece("netherite_boots", 0.47),
            piece("diamond_leggings", 0.18),
            piece("diamond_chestplate", 0.82),
            piece("diamond_helmet", 1.0),
        ];
        let hit = [
            worn[0].clone(),
            worn[1].clone(),
            piece("diamond_chestplate", 0.42),
            piece("diamond_helmet", 0.66),
        ];
        let partial = [worn[0].clone(), None, worn[2].clone(), worn[3].clone()];

        let origin = |index: usize| {
            egui::pos2(
                10.0 + (index % 2) as f32 * (ARMOR_W + 12.0),
                10.0 + (index / 2) as f32 * (armor_height() + 16.0),
            )
        };
        let (w, h) = (
            (ARMOR_W * 2.0 + 34.0) as usize,
            ((armor_height() + 16.0) * 2.0 + 14.0) as usize,
        );

        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut shapes = Vec::new();
        let mut laid_out_at = 1.0;
         
         
        for _ in 0..2 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(w as f32, h as f32),
                    )),
                    time: Some(1.25),
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(LayerId::new(
                        Order::Background,
                        Id::new("armor_preview_sheet"),
                    ));
                    draw_armor_card_at(
                        ctx,
                        &painter,
                        origin(0),
                        &worn,
                        &ArmorMotion::settled(&worn),
                    );
                     
                     
                    let mut struck = ArmorMotion::settled(&worn);
                    struck.advance(&hit, 0.055);
                    draw_armor_card_at(ctx, &painter, origin(1), &hit, &struck);
                     
                    let entering = ArmorMotion {
                        shown: [0.2, 0.55, 0.32, 0.94],
                        age: ARMOR_APPEAR + ARMOR_STAGGER * 1.25,
                    };
                    draw_armor_card_at(ctx, &painter, origin(2), &partial, &entering);
                     
                    let empty = [None, None, None, None];
                    draw_armor_card_at(
                        ctx,
                        &painter,
                        origin(3),
                        &empty,
                        &ArmorMotion::settled(&empty),
                    );
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            shapes = output.shapes;
            laid_out_at = output.pixels_per_point;
        }
        let primitives = ctx.tessellate(shapes, laid_out_at);
        let sheet = rasterize(primitives, &textures, w, h, [0.05, 0.04, 0.04, 1.0]);
         
        let image = image::imageops::resize(
            &sheet,
            w as u32 * 2,
            h as u32 * 2,
            image::imageops::FilterType::Nearest,
        );
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).expect("the screenshots directory exists");
        let path = dir.join("armor-status.png");
        image.save(&path).expect("the armor preview saves");
        println!("armor status: {path:?}");
    }

     
     
    #[test]
    fn a_settled_counter_does_not_animate() {
        let motion = TotemMotion::settled(7);
        assert_eq!(motion.value, 7.0);
        assert_eq!(motion.live(), 0.0);
        assert_eq!(motion.pop(), 0.0);
    }

    #[test]
    fn every_widget_has_a_default_slot_on_screen() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0));
        let ctx = Context::default();
        let ids = HudWidgetId::all();
        assert_eq!(ids.len(), 17);

         
         
        let mut measured = Vec::new();
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                for id in ids {
                     
                     
                     
                    if matches!(id, HudWidgetId::Spotify | HudWidgetId::Lyrics) {
                        continue;
                    }
                    measured.push((id, widget_content_size(id, ctx), id.default_pos(&screen)));
                }
            },
        );

         
        assert_eq!(measured.len(), ids.len() - 2);
        for (id, size, pos) in measured {
            assert!(pos.x >= 0.0 && pos.x + size.x <= screen.width(), "{id:?} x");
             
             
            let top = if id.anchors_bottom() {
                pos.y - size.y
            } else {
                pos.y
            };
            assert!(top >= 0.0 && top + size.y <= screen.height(), "{id:?} y");
            assert!(!id.display_name().is_empty());
        }
    }

     
     
    #[test]
    fn status_bars_clamp_and_colour_by_state() {
        assert!((bar_fraction(10.0, 20.0) - 0.5).abs() < 1e-6);
        assert_eq!(bar_fraction(40.0, 20.0), 1.0);
        assert_eq!(bar_fraction(-2.0, 20.0), 0.0);
         
        assert_eq!(bar_fraction(10.0, 0.0), 0.0);
        assert_eq!(bar_fraction(f32::NAN, 20.0), 0.0);

        assert_eq!(health_color(0.0), theme::DANGER);
        assert_eq!(health_color(1.0), theme::SUCCESS);
        assert_eq!(health_color(-1.0), theme::DANGER);
        assert_eq!(health_color(4.0), theme::SUCCESS);
         
        let middle = health_color(0.5);
        assert_ne!(middle, theme::DANGER);
        assert_ne!(middle, theme::SUCCESS);
    }

     
     
    #[test]
    fn status_card_rows_follow_the_switches() {
        let style = UiStyle::default();
        let full = StatusConfig::default();
        let bars = status_bars(&full, 18.0, 20.0, 15, 17);
        assert_eq!(bars.len(), 3);
        assert_eq!(
            bars.iter().map(|bar| bar.label).collect::<Vec<_>>(),
            vec!["HP", "ARM", "FOOD"]
        );
        assert_eq!(bars[0].value, "18/20");
        assert_eq!(bars[1].fraction, 0.5);
        let full_size = status_card_size(&full, &style);

        let mut health_only = full;
        health_only.armor = false;
        health_only.food = false;
        let bars = status_bars(&health_only, 18.0, 20.0, 15, 17);
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].label, "HP");
        let small = status_card_size(&health_only, &style);
        assert!(small.y < full_size.y);
        assert!((small.x - full_size.x).abs() < 1e-6, "width is a setting");

        health_only.name = false;
        assert!(status_card_size(&health_only, &style).y < small.y);
    }

     
     
    #[test]
    fn the_status_card_preview_renders_lit_bars() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let (width, height) = (240usize, 140usize);
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        let cfg = StatusConfig::default();
        let bars = status_bars(&cfg, 18.0, 20.0, 15, 17);
        for frame in 0..4 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx
                        .layer_painter(LayerId::new(Order::Background, Id::new("status_preview")));
                    draw_status_card_at(
                        ctx,
                        &painter,
                        egui::pos2(24.0, 28.0),
                        &cfg,
                        "Steve",
                        &bars,
                    );
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            primitives = ctx.tessellate(output.shapes, 1.0);
        }
        let image = rasterize(
            primitives,
            &textures,
            width,
            height,
            [0.06, 0.06, 0.075, 1.0],
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/status-card-preview.png");
        image.save(&path).expect("the preview png saves");
        let lit = image
            .pixels()
            .filter(|pixel| {
                pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32 > 120 && pixel[3] > 128
            })
            .count();
        assert!(
            lit > 400,
            "the card should be visibly lit, got {lit} pixels"
        );
    }

     
     
     
    #[test]
    fn the_stat_column_preview_has_no_side_rail() {
        use crate::graphic::preview::{gather_textures, rasterize};
        use std::collections::HashMap;

        let (width, height) = (300usize, 200usize);
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = Context::default();
        let mut textures = HashMap::new();
        let mut primitives = Vec::new();
        let mut texts = Vec::new();
        for frame in 0..3 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(LayerId::new(
                        Order::Background,
                        Id::new("stat_column_preview"),
                    ));
                    draw_text_widget_at(
                        ctx,
                        &painter,
                        egui::pos2(16.0, 26.0),
                        preview_rows(HudWidgetId::Coords),
                    );
                    draw_text_widget_at(
                        ctx,
                        &painter,
                        egui::pos2(16.0, 60.0),
                        preview_rows(HudWidgetId::Session),
                    );
                },
            );
            gather_textures(&mut textures, &output.textures_delta);
            texts = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                    _ => None,
                })
                .collect();
            primitives = ctx.tessellate(output.shapes, 1.0);
        }

        let image = rasterize(
            primitives,
            &textures,
            width,
            height,
            [0.06, 0.06, 0.075, 1.0],
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/hud-stats-preview.png");
        image.save(&path).expect("the stat column preview saves");

         
         
        for expected in ["XYZ", "Session", "0:42:13"] {
            assert!(
                texts.iter().any(|text| text == expected),
                "missing {expected:?} in {texts:?}"
            );
        }
        assert!(
            !texts.iter().any(|text| text == "XYZ: 100.0 / 64.0 / 100.0"),
            "the label and value must not be one run"
        );
    }

     
    struct ArmorFrame {
         
        texts: Vec<(String, Pos2)>,
         
        icons: usize,
         
        rects: Vec<(Rect, Color32)>,
    }

     
     
    fn armor_frames(frames: usize, draw: impl Fn(&Context, &Painter)) -> ArmorFrame {
        let (width, height) = (260usize, 220usize);
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
        let ctx = Context::default();
        let (mut texts, mut icons, mut rects) = (Vec::new(), 0usize, Vec::new());
        for frame in 0..frames {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 / 60.0),
                    predicted_dt: 1.0 / 60.0,
                    ..Default::default()
                },
                |ctx| {
                    let painter =
                        ctx.layer_painter(LayerId::new(Order::Background, Id::new("armor_test")));
                    draw(ctx, &painter);
                },
            );
            texts = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some((text.galley.text().to_owned(), text.pos)),
                    _ => None,
                })
                .collect();
            icons = output
                .shapes
                .iter()
                .filter(|clipped| {
                    matches!(&clipped.shape, egui::Shape::Mesh(mesh)
                        if mesh.texture_id != egui::TextureId::default())
                })
                .count();
            rects = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect) => Some((rect.rect, rect.fill)),
                    _ => None,
                })
                .collect();
        }
        ArmorFrame {
            texts,
            icons,
            rects,
        }
    }

     
     
     
    #[test]
    fn the_armor_card_paints_icons_names_and_durability() {
        let origin = egui::pos2(12.0, 12.0);
        let frame = armor_frames(40, |ctx, painter| draw_armor_preview(ctx, painter, origin));
        let ArmorFrame {
            texts,
            icons,
            rects,
        } = &frame;

        let style = UiStyle::hud_style();
        let card = Rect::from_min_size(origin, Vec2::new(ARMOR_W, armor_height()) * style.scale);
        assert!(
            rects
                .iter()
                .any(|(rect, fill)| *rect == card && *fill == style.background),
            "the card is one panel of the menu's own fill: {card:?}"
        );
        assert_eq!(*icons, 4, "one bundled item texture per worn piece");

        let at = |label: &str| {
            texts
                .iter()
                .find(|(text, _)| text == label)
                .map(|(_, pos)| *pos)
                .unwrap_or_else(|| panic!("no {label:?} drawn, got {texts:?}"))
        };
        for label in [
            "ARMOR",
            "4/4",
            "Diamond Helmet",
            "Diamond Chestplate",
            "Diamond Leggings",
            "Diamond Boots",
        ] {
            at(label);
        }
        for value in ["100%", "82%", "64%", "47%"] {
            at(value);
        }
         
        assert!(at("Diamond Helmet").y < at("Diamond Chestplate").y);
        assert!(at("Diamond Chestplate").y < at("Diamond Leggings").y);
        assert!(at("Diamond Leggings").y < at("Diamond Boots").y);
         
        let tracks = rects
            .iter()
            .filter(|(_, fill)| *fill == theme::with_alpha(theme::TEXT, 0.07))
            .count();
        assert_eq!(tracks, 4, "one durability track per row");
    }

     
     
     
     
    #[test]
    fn the_armor_bar_slides_to_a_new_durability_instead_of_jumping() {
        let origin = egui::pos2(12.0, 12.0);
        let piece = |durability: f32| {
            [
                Some(ArmorPiece {
                    name: "Diamond Helmet".into(),
                    id: "diamond_helmet".into(),
                    durability,
                }),
                None,
                None,
                None,
            ]
        };
        let percent = |texts: &Vec<(String, Pos2)>| {
            texts
                .iter()
                .find_map(|(text, _)| text.strip_suffix('%')?.parse::<f32>().ok())
                .expect("the row shows a percentage")
        };
        let danger_trail = theme::with_alpha(theme::DANGER, 0.5);

         
         
        let whole = piece(1.0);
        let frame = armor_frames(40, |ctx, painter| {
            draw_armor_card_at(ctx, painter, origin, &whole, &ArmorMotion::settled(&whole))
        });
        assert_eq!(percent(&frame.texts), 100.0, "a whole piece reads 100%");
        assert!(
            !frame.rects.iter().any(|(_, fill)| *fill == danger_trail),
            "a settled bar carries no damage trail"
        );

         
         
        let hit = piece(0.5);
        let mut motion = ArmorMotion::settled(&whole);
        motion.advance(&hit, 1.0 / 60.0);
        let frame = armor_frames(1, |ctx, painter| {
            draw_armor_card_at(ctx, painter, origin, &hit, &motion)
        });
        let shown = percent(&frame.texts);
        assert!(
            (50.0..100.0).contains(&shown),
            "the bar lags behind the hit instead of snapping, read {shown}"
        );
        assert!(
            frame.rects.iter().any(|(_, fill)| *fill == danger_trail),
            "the damage the bar is still travelling over is left in red"
        );

         
        for _ in 0..60 {
            motion.advance(&hit, 1.0 / 60.0);
        }
        let frame = armor_frames(1, |ctx, painter| {
            draw_armor_card_at(ctx, painter, origin, &hit, &motion)
        });
        assert_eq!(percent(&frame.texts), 50.0, "the bar finishes the slide");
    }

     
     
    #[test]
    fn the_armor_card_keeps_a_shape_for_empty_slots() {
        let origin = egui::pos2(12.0, 12.0);
        let pieces = [
            Some(ArmorPiece {
                name: "Diamond Boots".into(),
                id: "diamond_boots".into(),
                durability: 1.0,
            }),
            None,
            None,
            None,
        ];
        let frame = armor_frames(20, |ctx, painter| {
            draw_armor_card_at(
                ctx,
                painter,
                origin,
                &pieces,
                &ArmorMotion::settled(&pieces),
            )
        });
        let ArmorFrame {
            texts,
            icons,
            rects,
        } = &frame;
        assert_eq!(*icons, 1, "only the worn piece gets its texture");
        assert_eq!(
            texts.iter().filter(|(text, _)| text == "Empty").count(),
            3,
            "every free slot is named"
        );
        assert!(
            texts.iter().any(|(text, _)| text == "1/4"),
            "the header counts the slots in use"
        );
        let style = UiStyle::hud_style();
         
        assert_eq!(
            rects
                .iter()
                .filter(|(_, fill)| *fill == theme::with_alpha(theme::TEXT, 0.04))
                .count(),
            3
        );
        assert_eq!(
            rects
                .iter()
                .filter(|(_, fill)| *fill == theme::with_alpha(theme::TEXT, 0.07))
                .count(),
            1
        );
        assert!(
            widget_content_size(HudWidgetId::ArmorStatus, &Context::default())
                == Vec2::new(ARMOR_W, armor_height()) * style.scale,
            "Edit-UI sizes its drag box to the card"
        );
    }

     
     
    #[test]
    fn the_status_card_paints_labels_values_and_filled_tracks() {
        let ctx = Context::default();
        let cfg = StatusConfig::default();
        let bars = status_bars(&cfg, 18.0, 20.0, 15, 17);
        let origin = egui::pos2(30.0, 220.0);
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0))),
                ..Default::default()
            },
            |ctx| {
                let painter =
                    ctx.layer_painter(LayerId::new(Order::Background, Id::new("status_test")));
                draw_status_card_at(ctx, &painter, origin, &cfg, "Steve", &bars);
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
        for expected in ["Steve", "HP", "ARM", "FOOD", "18/20", "17"] {
            assert!(
                texts.iter().any(|text| text == expected),
                "missing {expected:?} in {texts:?}"
            );
        }

         
         
        let card = Rect::from_min_size(origin, status_card_size(&cfg, &UiStyle::default()));
        let filled = output.shapes.iter().filter(|clipped| {
            matches!(&clipped.shape, egui::Shape::Rect(rect)
                if rect.fill == health_color(0.9) && card.contains_rect(rect.rect))
        });
        assert!(
            filled.count() >= 1,
            "the health fill must be drawn in its state colour"
        );
    }
}
