 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::graphic::block_group::{self, Cell};
use crate::graphic::notification::{Notification, NotificationType};
use crate::graphic::theme;
use crate::mapping::chunk::{
    block_registry, clear_palette_target_cache, clear_state_target_cache, palette_slot_target,
    BitStorage, LevelChunkSection, Palette,
};
use crate::mapping::client::camera::Camera;
use crate::mapping::client::world::{LevelChunk, World};
use crate::mapping::entity::mob::Mob;
use crate::mapping::entity::player::{GearStack, LocalPlayer, Player};
use crate::mapping::entity::{Entity, EntityRef, LivingEntityRef};
use crate::mapping::math::{BlockPos, Vec3};
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::module::render::box_esp::BoxTargets;
use crate::module::{ModuleId, ModuleSetting};
use crate::state::{client, mapping, minecraft};
use egui::{
    pos2, vec2, Align2, Color32, Context, FontId, LayerId, Order, Painter, Pos2, Rect, Rounding,
    Shape, Stroke,
};
use jni::objects::{GlobalRef, JObject, JValue};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock};
use std::time::{Duration, Instant};

 
 
const GATHER_INTERVAL: Duration = Duration::from_millis(66);
 
const CHEST_SCAN_INTERVAL: Duration = Duration::from_secs(2);
 
const GEAR_INTERVAL: Duration = Duration::from_millis(200);
 
const GEAR_STALE_AFTER: Duration = Duration::from_secs(10);
 
const GEAR_CACHE_CAPACITY: usize = 128;
 
 
const OUTLINE_REACH: f64 = 4.5;
 
const CHEST_CHUNK_RADIUS: i32 = 8;
 
 
const BLOCK_SCAN_INTERVAL: Duration = Duration::from_millis(250);
 
 
 
 
 
 
const BLOCK_SCAN_BUDGET: Duration = Duration::from_millis(2);
 
 
 
 
const BLOCK_PRIME_BUDGET: Duration = Duration::from_millis(6);
 
const BLOCK_CHANGES_MAX: usize = 4096;
 
 
 
 
const BLOCK_EVICT_MAX_PASSES_PER_TICK: u32 = 2;
 
 
 
 
const PALETTE_PRECHECK_MAX: i32 = 32;
 
 
 
const BLOCK_DISTANCE_LABEL_LIMIT: usize = 128;
 
 
 
const NEIGHBOUR_PROBE_LIMIT: usize = 1024;

 
const LINE_WIDTH: f32 = 1.6;
 
 
 
 
 
const BRACKET_FRACTION: f32 = 0.22;
const BRACKET_MIN: f32 = 4.0;
const BRACKET_MAX: f32 = 14.0;
 
const TARGET_STROKE: f32 = 1.0;
const BRACKET_STROKE: f32 = 2.0;
 
 
 
const ROUNDED_RADIUS_FRACTION: f32 = 0.18;
const ROUNDED_RADIUS_MIN: f32 = 4.0;
const ROUNDED_RADIUS_MAX: f32 = 18.0;
 
 
 
const CORNER_STEPS: usize = 7;
 
 
const PERIMETER_POINTS: usize = 4 * (CORNER_STEPS + 1);
 
 
 
const COMET_TAIL: f32 = 0.12;
const COMET_LAPS_PER_SECOND: f32 = 0.32;
 
static TARGET_CLOCK: LazyLock<Instant> = LazyLock::new(Instant::now);

 
const OUTLINE_FILL_ALPHA: u8 = 14;
 
const OUTLINE_EASE_RATE: f64 = 16.0;
 
const OUTLINE_STALE_AFTER: Duration = Duration::from_millis(250);
 
static OUTLINE_CLOCK: LazyLock<Instant> = LazyLock::new(Instant::now);

 

 
 
#[derive(Debug, Clone, Copy)]
pub(crate) struct V3 {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
}

impl V3 {
    pub(crate) fn sub(self, o: V3) -> V3 {
        V3 {
            x: self.x - o.x,
            y: self.y - o.y,
            z: self.z - o.z,
        }
    }

    pub(crate) fn dot(self, o: V3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub(crate) fn cross(self, o: V3) -> V3 {
        V3 {
            x: self.y * o.z - self.z * o.y,
            y: self.z * o.x - self.x * o.z,
            z: self.x * o.y - self.y * o.x,
        }
    }

    pub(crate) fn lerp(self, o: V3, t: f64) -> V3 {
        V3 {
            x: self.x + (o.x - self.x) * t,
            y: self.y + (o.y - self.y) * t,
            z: self.z + (o.z - self.z) * t,
        }
    }

    pub(crate) fn length(self) -> f64 {
        self.dot(self).sqrt()
    }
}

 
impl From<Vec3> for V3 {
    fn from(v: Vec3) -> V3 {
        V3 {
            x: v.x(),
            y: v.y(),
            z: v.z(),
        }
    }
}

 
pub(crate) struct View {
    pub(crate) cam: V3,
    pub(crate) fwd: V3,
    pub(crate) right: V3,
    pub(crate) up: V3,
    pub(crate) tan_x: f64,
    pub(crate) tan_y: f64,
    pub(crate) w: f32,
    pub(crate) h: f32,
}

impl View {
     
     
    pub(crate) fn project(&self, p: V3) -> Option<Pos2> {
        let r = p.sub(self.cam);
        let depth = r.dot(self.fwd);
        if depth < 0.05 {
            return None;
        }
        let ndc_x = (r.dot(self.right) / depth) / self.tan_x;
        let ndc_y = (r.dot(self.up) / depth) / self.tan_y;
        let sx = (ndc_x * 0.5 + 0.5) * self.w as f64;
        let sy = (0.5 - ndc_y * 0.5) * self.h as f64;
        Some(pos2(sx as f32, sy as f32))
    }
}

 
pub(crate) fn build_view(
    cam: V3,
    yaw_deg: f32,
    pitch_deg: f32,
    fov_deg: f64,
    w: f32,
    h: f32,
) -> View {
    let yaw = (yaw_deg as f64).to_radians();
    let pitch = (pitch_deg as f64).to_radians();
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let fwd = V3 {
        x: -sy * cp,
        y: -sp,
        z: cy * cp,
    };
     
     
    let right = V3 {
        x: cy,
        y: 0.0,
        z: sy,
    };
    build_view_from_basis(cam, fwd, fwd.cross(right), right, fov_deg, w, h)
}

 
 
 
 
pub(crate) fn build_view_from_basis(
    cam: V3,
    fwd: V3,
    up: V3,
    left: V3,
    fov_deg: f64,
    w: f32,
    h: f32,
) -> View {
    let right = V3 {
        x: -left.x,
        y: -left.y,
        z: -left.z,
    };
    let tan_y = (fov_deg.to_radians() * 0.5).tan();
    let tan_x = tan_y * (w / h) as f64;
    View {
        cam,
        fwd,
        right,
        up,
        tan_x,
        tan_y,
        w,
        h,
    }
}

 

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetKind {
    Player,
    Mob,
    Item,
    Projectile,
}

 
#[derive(Debug, Clone)]
struct EntityTarget {
    id: i32,
    kind: TargetKind,
     
    prev: V3,
     
    pos: V3,
    width: f64,
    height: f64,
    name: String,
    health: f32,
    max_health: f32,
    distance: f32,
    skin_url: Option<String>,
     
     
    player_jni: Option<GlobalRef>,
}

 
 
#[derive(Debug, Clone, Default)]
struct Gear {
     
    armor: [Option<GearStack>; 4],
    main_hand: Option<GearStack>,
    off_hand: Option<GearStack>,
}

impl Gear {
    fn read(player: &Player) -> Option<Gear> {
        let equipment = player.equipment().ok()?;
        Some(Gear {
            armor: equipment.armor,
            main_hand: equipment.main_hand,
            off_hand: equipment.off_hand,
        })
    }
}

 
#[derive(Debug, Clone, Copy)]
struct ChestTarget {
    pos: V3,
    is_spawner: bool,
     
    color: Option<Color32>,
}

 
struct EspState {
    camera: Option<Camera>,
    entities: Vec<EntityTarget>,
    chests: Vec<ChestTarget>,
     
    sweep: Option<BlockSweep>,
     
     
    block_primed: bool,
    spawners: Vec<ChestTarget>,
     
     
    outline_shown: Option<(V3, Instant)>,
    prev_gather: Option<Instant>,
    last_gather: Option<Instant>,
    last_chest_scan: Option<Instant>,
    last_spawner_scan: Option<Instant>,
    last_block_scan: Option<Instant>,
     
     
    last_logged_block_count: usize,
     
    fov: f64,
     
    target_fov: f64,
     
     
     
     
    partial_tick: f32,
     
    camera_logged: bool,
}

impl EspState {
    fn new() -> Self {
        EspState {
            camera: None,
            entities: Vec::new(),
            chests: Vec::new(),
            sweep: None,
            block_primed: false,
            spawners: Vec::new(),
            outline_shown: None,
            prev_gather: None,
            last_gather: None,
            last_chest_scan: None,
            last_spawner_scan: None,
            last_block_scan: None,
            last_logged_block_count: usize::MAX,
            fov: 70.0,
            target_fov: 70.0,
            partial_tick: 1.0,
            camera_logged: false,
        }
    }
}

fn state() -> &'static Mutex<EspState> {
    static STATE: OnceLock<Mutex<EspState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(EspState::new()))
}

 

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoxStyle {
     
    Brackets,
     
    Rounded,
}

impl BoxStyle {
     
    const OPTIONS: [&'static str; 2] = ["Brackets", "Rounded"];

    fn from_choice(index: usize) -> Self {
        match index {
            1 => BoxStyle::Rounded,
            _ => BoxStyle::Brackets,
        }
    }
}

 
 
#[derive(Debug, Clone, Copy)]
struct TargetLook {
    style: BoxStyle,
    animate: bool,
    glow: bool,
     
    time: f32,
     
    phase: f32,
}

impl TargetLook {
     
     
    #[cfg(test)]
    fn still(style: BoxStyle) -> Self {
        Self {
            style,
            animate: false,
            glow: false,
            time: 0.0,
            phase: 0.0,
        }
    }
}

 
#[derive(Clone, Copy)]
struct EntityCfg {
    enabled: bool,
    color: Color32,
    show_name: bool,
    show_distance: bool,
    show_health: bool,
     
    range: f32,
    style: BoxStyle,
     
    animate: bool,
     
    glow: bool,
}

impl EntityCfg {
     
     
    fn plain() -> Self {
        Self {
            enabled: false,
            color: Color32::WHITE,
            show_name: false,
            show_distance: false,
            show_health: false,
            range: 0.0,
            style: BoxStyle::Brackets,
            animate: true,
            glow: true,
        }
    }
}

 
#[derive(Clone, Copy)]
struct BoxCfg {
    enabled: bool,
    color: Color32,
    players: bool,
    mobs: bool,
     
    range: f32,
     
    thickness: f32,
     
    fill: bool,
    fill_opacity: f32,
}

impl BoxCfg {
    fn empty() -> Self {
        Self {
            enabled: false,
            color: Color32::WHITE,
            players: false,
            mobs: false,
            range: 0.0,
            thickness: 1.5,
            fill: false,
            fill_opacity: 0.0,
        }
    }
}

 
 
 
 
 
 
struct BlockProfile {
     
     
    source: ModuleId,
    color: Color32,
    show_distance: bool,
     
    range: f32,
     
    blocks: Vec<String>,
     
    ore_colors: bool,
     
    tracers: bool,
     
    shade: bool,
    shade_opacity: f32,
     
    notify: bool,
     
    notify_range: f32,
}

 
 
const BLOCK_NOTIFY_RANGE: f32 = 64.0;

 
const BLOCK_SHADE_OPACITY: f32 = 0.2;

impl BlockProfile {
     
     
    fn new(
        source: ModuleId,
        color: Color32,
        show_distance: bool,
        range: f32,
        blocks: Vec<String>,
        ore_colors: bool,
    ) -> Self {
        Self {
            source,
            color,
            show_distance,
            range,
            blocks,
            ore_colors,
            tracers: false,
            shade: false,
            shade_opacity: BLOCK_SHADE_OPACITY,
            notify: false,
            notify_range: BLOCK_NOTIFY_RANGE,
        }
    }

     
     
     
    fn with_alerts(mut self, data: &crate::module::ModuleData) -> Self {
        self.notify = toggle_setting(data, "Notify", true);
        self.notify_range = slider_setting(data, "Notify Range", BLOCK_NOTIFY_RANGE)
            .clamp(8.0, CHEST_CHUNK_RADIUS as f32 * 16.0);
        self
    }

     
     
     
     
     
     
    fn with_shared(mut self, data: &crate::module::ModuleData) -> Self {
        self.tracers = toggle_setting(data, "Tracers", false);
        self.shade = toggle_setting(data, "Shade", false);
        self.shade_opacity = slider_setting(data, "Shade Opacity", BLOCK_SHADE_OPACITY);
        self
    }
}

 
 
 
#[derive(Debug, Clone, Copy)]
struct BlockDraw {
    target: ChestTarget,
    color: Color32,
}

 
struct TracerCfg {
    enabled: bool,
    color: Color32,
    players: bool,
    mobs: bool,
     
    range: f32,
}

 
struct NametagCfg {
    enabled: bool,
    color: Color32,
    show_health: bool,
    show_distance: bool,
     
    range: f32,
     
    gear: bool,
     
    armor: bool,
     
    hands: bool,
     
    durability: bool,
}

impl NametagCfg {
     
     
    fn wants_gear(&self) -> bool {
        self.enabled && self.gear && (self.armor || self.hands)
    }
}

#[derive(Clone)]
struct OutlineCfg {
    enabled: bool,
    color: Color32,
    width: f32,
}

struct EspConfig {
    player: EntityCfg,
    player_hud_scale: f32,
    player_hud_x: f32,
    player_hud_y: f32,
    mob: EntityCfg,
    box_esp: BoxCfg,
     
     
    chest: Option<BlockProfile>,
     
    spawner: Option<BlockProfile>,
     
    blocks: Vec<BlockProfile>,
    tracer: TracerCfg,
    nametag: NametagCfg,
     
     
    infered: bool,
    item: EntityCfg,
    projectile: EntityCfg,
    outline: OutlineCfg,
     
    break_visual: OutlineCfg,
}

impl EspConfig {
    fn any_enabled(&self) -> bool {
        self.player.enabled
            || self.mob.enabled
            || self.box_esp.enabled
            || self.chest.is_some()
            || self.spawner.is_some()
            || !self.blocks.is_empty()
            || self.tracer.enabled
            || self.nametag.enabled
            || self.infered
            || self.item.enabled
            || self.projectile.enabled
            || self.outline.enabled
            || self.break_visual.enabled
    }

     
     
    fn want_players(&self) -> bool {
        self.player.enabled
            || (self.tracer.enabled && self.tracer.players)
            || self.nametag.enabled
            || self.infered
            || (self.box_esp.enabled && self.box_esp.players)
    }

     
    fn want_mobs(&self) -> bool {
        self.mob.enabled
            || (self.tracer.enabled && self.tracer.mobs)
            || (self.box_esp.enabled && self.box_esp.mobs)
    }

     
    fn want_items(&self) -> bool {
        self.item.enabled
    }

     
    fn want_projectiles(&self) -> bool {
        self.projectile.enabled
    }
}

pub(crate) fn color_setting(
    data: &crate::module::ModuleData,
    name: &str,
    fallback: Color32,
) -> Color32 {
    match data.get_setting(name) {
        Some(ModuleSetting::Color { value, .. }) => Color32::from_rgba_unmultiplied(
            (value[0] * 255.0) as u8,
            (value[1] * 255.0) as u8,
            (value[2] * 255.0) as u8,
            (value[3] * 255.0).max(40.0) as u8,
        ),
        _ => fallback,
    }
}

pub(crate) fn toggle_setting(data: &crate::module::ModuleData, name: &str, fallback: bool) -> bool {
    data.get_setting(name)
        .and_then(ModuleSetting::get_toggle_value)
        .unwrap_or(fallback)
}

pub(crate) fn slider_setting(data: &crate::module::ModuleData, name: &str, fallback: f32) -> f32 {
    data.get_setting(name)
        .and_then(ModuleSetting::get_slider_value)
        .unwrap_or(fallback)
}

 
 
pub(crate) fn choice_setting(data: &crate::module::ModuleData, name: &str) -> Option<usize> {
    match data.get_setting(name) {
        Some(ModuleSetting::Choice { value, .. }) => Some(*value),
        _ => None,
    }
}

 
fn entity_cfg(data: &crate::module::ModuleData, fallback_color: Color32) -> EntityCfg {
    EntityCfg {
        enabled: data.enabled,
        color: color_setting(data, "Color", fallback_color),
        show_name: toggle_setting(data, "Name", true),
        show_distance: toggle_setting(data, "Distance", true),
        show_health: toggle_setting(data, "Health", true),
        range: slider_setting(data, "Range", 64.0),
        style: BoxStyle::from_choice(choice_setting(data, "Style").unwrap_or(0)),
        animate: toggle_setting(data, "Animate", true),
        glow: toggle_setting(data, "Glow", true),
    }
}

 
fn read_config() -> EspConfig {
    let mut cfg = EspConfig {
        player: EntityCfg::plain(),
        player_hud_scale: 1.0,
        player_hud_x: 0.0,
        player_hud_y: 0.0,
        mob: EntityCfg::plain(),
        box_esp: BoxCfg::empty(),
        chest: None,
        spawner: None,
        blocks: Vec::new(),
        tracer: TracerCfg {
            enabled: false,
            color: Color32::WHITE,
            players: false,
            mobs: false,
            range: 0.0,
        },
        infered: false,
        nametag: NametagCfg {
            enabled: false,
            color: Color32::WHITE,
            show_health: false,
            show_distance: false,
            range: 0.0,
            gear: false,
            armor: false,
            hands: false,
            durability: false,
        },
        item: EntityCfg::plain(),
        projectile: EntityCfg::plain(),
        outline: OutlineCfg {
            enabled: false,
            color: Color32::from_rgb(255, 0, 0),
            width: 4.0,
        },
        break_visual: OutlineCfg {
            enabled: false,
            color: Color32::from_rgb(255, 64, 64),
            width: 0.0,
        },
    };

    let modules = &client().modules;

    if let Some(arc) = modules.get(ModuleId::PlayerEsp) {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            cfg.player = entity_cfg(data, Color32::from_rgb(255, 0, 0));
             
             
        }
    }
    if let Some(arc) = modules.get(ModuleId::MobEsp) {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            cfg.mob = entity_cfg(data, Color32::from_rgb(255, 0, 0));
        }
    }
    if let Some(arc) = modules.get(ModuleId::BoxEsp) {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            let targets = BoxTargets::from_choice(choice_setting(data, "Targets").unwrap_or(2));
            cfg.box_esp = BoxCfg {
                enabled: data.enabled,
                color: color_setting(data, "Color", Color32::from_rgb(255, 0, 0)),
                players: targets.players(),
                mobs: targets.mobs(),
                range: slider_setting(data, "Range", 64.0),
                thickness: slider_setting(data, "Thickness", 1.5).clamp(0.5, 6.0),
                fill: toggle_setting(data, "Fill", true),
                fill_opacity: slider_setting(data, "Fill Opacity", 0.16).clamp(0.0, 1.0),
            };
        }
    }
     
     
     
    if let Some(arc) = modules.get(ModuleId::ChestEsp) {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            if data.enabled {
                cfg.chest = Some(
                    BlockProfile::new(
                        ModuleId::ChestEsp,
                        color_setting(data, "Color", Color32::from_rgb(255, 0, 0)),
                        toggle_setting(data, "Distance", true),
                         
                         
                         
                        (CHEST_CHUNK_RADIUS * 16) as f32,
                        data.get_setting("Storage Blocks")
                            .and_then(ModuleSetting::get_block_list_value)
                            .map(|list| list.to_vec())
                            .unwrap_or_default(),
                        false,
                    )
                    .with_shared(data),
                );
            }
        }
    }
     
    if let Some(arc) = modules.get(ModuleId::BlockEsp) {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            if data.enabled {
                cfg.blocks.push(
                    BlockProfile::new(
                        ModuleId::BlockEsp,
                        color_setting(data, "Color", Color32::from_rgb(255, 0, 0)),
                        toggle_setting(data, "Distance", true),
                        slider_setting(data, "Range", 128.0),
                        data.get_setting("Blocks")
                            .and_then(ModuleSetting::get_block_list_value)
                            .map(|list| list.to_vec())
                            .unwrap_or_default(),
                         
                        false,
                    )
                    .with_shared(data),
                );
            }
        }
    }
     

    if let Some(arc) = modules.get(ModuleId::Tracers) {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            cfg.tracer = TracerCfg {
                enabled: data.enabled,

                color: color_setting(data, "Color", Color32::from_rgb(255, 0, 0)),
                players: toggle_setting(data, "Players", true),
                mobs: toggle_setting(data, "Mobs", false),
                range: slider_setting(data, "Range", 96.0),
            };
        }
    }
    if let Some(arc) = modules.get(ModuleId::ItemEsp) {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            cfg.item = EntityCfg {
                enabled: data.enabled,
                color: color_setting(data, "Color", Color32::from_rgb(255, 0, 0)),
                show_name: toggle_setting(data, "Name", true),
                show_distance: false,
                show_health: false,
                range: slider_setting(data, "Range", 48.0),
                ..EntityCfg::plain()
            };
        }
    }
    if let Some(arc) = modules.get(ModuleId::Projectiles) {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            cfg.projectile = EntityCfg {
                enabled: data.enabled,
                color: color_setting(data, "Color", Color32::from_rgb(255, 0, 0)),
                show_name: toggle_setting(data, "Name", true),
                show_distance: toggle_setting(data, "Distance", true),
                show_health: false,
                range: slider_setting(data, "Range", 96.0),
                ..EntityCfg::plain()
            };
        }
    }
     
     
    if let Some(arc) = modules.get(ModuleId::SpawnerFinder) {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            if data.enabled {
                cfg.spawner = Some(
                    BlockProfile::new(
                        ModuleId::SpawnerFinder,
                        color_setting(data, "Color", Color32::from_rgb(255, 0, 0)),
                        toggle_setting(data, "Distance", true),
                        (CHEST_CHUNK_RADIUS * 16) as f32,
                         
                        Vec::new(),
                        false,
                    )
                    .with_shared(data)
                    .with_alerts(data),
                );
            }
        }
    }
    if let Some(arc) = modules.get(ModuleId::BlockOutline) {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            cfg.outline = OutlineCfg {
                enabled: data.enabled,
                color: color_setting(data, "Color", Color32::from_rgb(255, 0, 0)),
                width: super::smoke::finite(slider_setting(data, "Width", 6.0), 6.0, 0.5, 12.0),
            };
        }
    }
    if let Some(arc) = modules.get(ModuleId::BreakVisualizer) {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            cfg.break_visual = OutlineCfg {
                enabled: data.enabled,
                color: color_setting(data, "Color", Color32::from_rgb(255, 64, 64)),
                width: 0.0,
            };
        }
    }
    if let Some(arc) = modules.get(ModuleId::Nametags) {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            cfg.nametag = NametagCfg {
                enabled: data.enabled,
                color: color_setting(data, "Color", Color32::from_rgb(255, 0, 0)),
                show_health: toggle_setting(data, "Health", true),
                show_distance: toggle_setting(data, "Distance", true),
                range: slider_setting(data, "Range", 64.0),
                gear: toggle_setting(data, "Gear", true),
                armor: toggle_setting(data, "Armor", true),
                hands: toggle_setting(data, "Hands", true),
                durability: toggle_setting(data, "Durability", true),
            };
            cfg.player_hud_scale = slider_setting(data, "HUD Scale", 1.06).clamp(0.6, 1.2);
            cfg.player_hud_x = slider_setting(data, "HUD X Offset", -95.0).clamp(-100.0, 100.0);
            cfg.player_hud_y = slider_setting(data, "HUD Y Offset", 0.0).clamp(-100.0, 100.0);
            cfg.player.show_health = cfg.nametag.show_health;
            cfg.player.show_distance = cfg.nametag.show_distance;
        }
    }
     
     
     
     
    if let Some(arc) = modules.get(ModuleId::InferedTag) {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            cfg.infered = data.enabled && toggle_setting(data, "Show Logo", true);
        }
    }

     
     

    cfg
}

 

 
 
fn overlay_layer() -> LayerId {
    LayerId::new(Order::Background, crate::graphic::hud::layer_id())
}

 
 
 
 
pub fn draw(ctx: &Context) {
     
     
     
     
     
     
    let pause_menu = crate::graphic::pause_menu::wants_input();
    if crate::graphic::input::minecraft_screen_open() && !pause_menu {
        return;
    }
    let cfg = read_config();
     
     
     
    let world_lyrics = crate::graphic::lyrics_world::WorldConfig::read();
    if !world_lyrics.enabled {
        crate::graphic::lyrics_world::reset();
    }
    let freecam = client().modules.is_active()
        && client()
            .modules
            .get(ModuleId::Freecam)
            .and_then(|h| h.try_lock().ok().map(|m| m.get_module_data().enabled))
            .unwrap_or(false);
    let mut state = state().lock().unwrap();

     
     
    if minecraft().world().ok().flatten().is_none() {
        state.sweep = None;
        state.block_primed = false;
        clear_block_memory();
    }

     
     
    let trail = crate::module::render::trail::read_config();
    let footprints = crate::module::render::footprints::read_config();
    let jump_circles = crate::module::render::jump_circles::read_config();
    let cubes = crate::module::render::floating_cubes::read_config();
    let aura = crate::graphic::aura::read_config();
    let death_effect = crate::module::render::death_effect::read_config();
    let hit_effect = crate::module::render::hit_effect::read_config();
    let trajectories = crate::module::render::trajectories::read_config();
    let crown = crate::graphic::crown::read_config();
    let target_info = crate::module::render::target_info::read_config();
    if !cfg.any_enabled()
        && !freecam
        && !trail.enabled
        && !footprints.enabled
        && !jump_circles.enabled
        && !cubes.enabled
        && !aura.enabled
        && !death_effect.enabled
        && !hit_effect.enabled
        && !trajectories.enabled
        && !crown.enabled
        && !(target_info.enabled && target_info.physical)
        && !world_lyrics.enabled
    {
        state.entities.clear();
        state.chests.clear();
        state.spawners.clear();
        crate::module::render::trajectories::clear();
        return;
    }

    let now = Instant::now();
    let gather_interval = crate::module::performance::client_optimizations::duration(
        ModuleId::EspSnapshotOptimizer,
        "Refresh ms",
        GATHER_INTERVAL,
        66.0,
        500.0,
    );
    if state.last_gather.is_none_or(|t| now - t >= gather_interval) {
        gather(&mut state, &cfg, now);
    }
     
     
    crate::module::render::trajectories::update(&trajectories);
     
    advance_blocks(&mut state, &cfg, now);

    let view = match read_view(&mut state, ctx) {
        Some(view) => view,
        None => return,
    };
    let t = interp_factor(&state, now);

    let painter = ctx.layer_painter(overlay_layer());
    crate::module::render::trail::draw(&painter, &view, now, &trail);
    crate::module::render::footprints::draw(&painter, &view, now, &footprints);
    crate::module::render::jump_circles::draw(&painter, &view, now, &jump_circles);
     
    crate::module::render::floating_cubes::draw(&painter, &view, &cubes);
    crate::graphic::aura::draw(&painter, &view, now, &aura);
    crate::graphic::lyrics_world::draw(ctx, &painter, &view, &world_lyrics);
    crate::module::render::death_effect::draw(&painter, &view, now, &death_effect);
    crate::module::render::trajectories::draw(&painter, &view, &trajectories);
    crate::module::render::hit_effect::draw(&painter, &view, now, &hit_effect);
    crate::graphic::crown::draw(&painter, &view, now, &crown);
     
    crate::graphic::target_info::draw_physical(ctx, &painter, &view, now);

     
     
     
    if freecam {
        if let Ok(Some(player)) = minecraft().player() {
             
             
            let partial = state.partial_tick;
            let label = mapping().in_frame(|| -> anyhow::Result<()> {
                let pos = interpolated_position(&player, partial)?;
                let name = local_player_name(&player);
                let height = player.call_method("getBbHeight", &[])?.f()? as f64;
                 
                let head = V3 {
                    x: pos.x,
                    y: pos.y + height + 0.5,
                    z: pos.z,
                };
                if let Some(anchor) = view.project(head) {
                    draw_label(
                        &painter,
                        anchor,
                        Align2::CENTER_BOTTOM,
                        &name,
                        Color32::WHITE,
                    );
                }
                Ok(())
            });
            if let Err(error) = label {
                log::debug!("Local player nametag: {error}");
            }
        }
    }

     
     
    draw_box_esp(&painter, &view, &state.entities, t, &cfg.box_esp);

    for entity in &state.entities {
        draw_entity(&painter, ctx, &view, entity, t, now, &cfg);
        if cfg.nametag.enabled {
            let editor_offset = crate::graphic::hud::player_esp_editor_offset(ctx);
            draw_player_hud(&painter, ctx, &view, entity, t, &cfg, editor_offset);
        } else if cfg.infered {
             
             
            draw_infered_marker(&painter, ctx, &view, entity, t);
        }
        if cfg.tracer.enabled {
            draw_tracer(&painter, &view, entity, t, &cfg.tracer);
        }
    }
     
     
     
     
     
     
     
    let mut blocks_by_profile: Vec<Vec<BlockDraw>> = Vec::with_capacity(cfg.blocks.len());
    if let Ok(memory) = block_memory().lock() {
        for profile in &cfg.blocks {
            let draws = memory
                .get(&profile.source)
                .into_iter()
                .flat_map(|targets| targets.values())
                .filter_map(|target| {
                     
                    let range = profile.range as f64;
                    let dx = target.pos.x + 0.5 - view.cam.x;
                    let dy = target.pos.y + 0.5 - view.cam.y;
                    let dz = target.pos.z + 0.5 - view.cam.z;
                    within_cylinder(dx, dy, dz, range, range * range).then(|| BlockDraw {
                        target: *target,
                        color: if profile.ore_colors {
                            target.color.unwrap_or(profile.color)
                        } else {
                            profile.color
                        },
                    })
                })
                .collect();
            blocks_by_profile.push(draws);
        }
    }
     
     
     
     
    let draws_for = |profile: &Option<BlockProfile>, targets: &[ChestTarget]| {
        profile
            .as_ref()
            .map(|profile| {
                targets
                    .iter()
                    .map(|target| BlockDraw {
                        target: *target,
                        color: profile.color,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let chest_draws = draws_for(&cfg.chest, &state.chests);
    let spawner_draws = draws_for(&cfg.spawner, &state.spawners);

    let total_blocks: usize = blocks_by_profile.iter().map(Vec::len).sum::<usize>()
        + chest_draws.len()
        + spawner_draws.len();
    let distance_label_budget = total_blocks <= BLOCK_DISTANCE_LABEL_LIMIT;
    for (profile, draws) in cfg.blocks.iter().zip(&blocks_by_profile) {
        draw_block_profile(&painter, &view, profile, draws, distance_label_budget);
    }
    if let Some(profile) = &cfg.chest {
        draw_block_profile(
            &painter,
            &view,
            profile,
            &chest_draws,
            distance_label_budget,
        );
    }
    if let Some(profile) = &cfg.spawner {
        draw_block_profile(
            &painter,
            &view,
            profile,
            &spawner_draws,
            distance_label_budget,
        );
    }
     
     
    if cfg.outline.enabled {
        let eye = minecraft()
            .player()
            .ok()
            .flatten()
            .and_then(|player| player.get_eye_position().ok());
        let hit = minecraft()
            .hit_block_in_reach()
            .ok()
            .flatten()
            .filter(|pos| {
                eye.is_some_and(|eye| {
                    let dx = pos.x() as f64 + 0.5 - eye.x();
                    let dy = pos.y() as f64 + 0.5 - eye.y();
                    let dz = pos.z() as f64 + 0.5 - eye.z();
                    dx * dx + dy * dy + dz * dz <= OUTLINE_REACH * OUTLINE_REACH
                })
            });
        let target = hit.as_ref().map(|pos| V3 {
            x: pos.x() as f64,
            y: pos.y() as f64,
            z: pos.z() as f64,
        });
        if let Some(pos) = ease_outline(&mut state, target, now) {
             
             
            let boxes = hit
                .as_ref()
                .and_then(|hit| {
                    minecraft()
                        .world()
                        .ok()
                        .flatten()?
                        .block_shape_boxes(hit)
                        .ok()
                })
                .filter(|boxes| !boxes.is_empty())
                .unwrap_or_else(|| vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]);
            draw_block_outline(
                &painter,
                &view,
                pos,
                &boxes,
                cfg.outline.color,
                cfg.outline.width,
                outline_time(now),
            );
        }
    } else {
        state.outline_shown = None;
    }
    if cfg.break_visual.enabled {
        draw_break_visual(&painter, &view, cfg.break_visual.color);
    }
}

 
 
 
 
fn draw_break_visual(painter: &Painter, view: &View, tint: Color32) {
    let Some(game_mode) = minecraft().game_mode().ok().flatten() else {
        return;
    };
    let Ok(Some(pos)) = game_mode.destroy_block_pos() else {
        return;
    };
    let progress = game_mode.destroy_progress().unwrap_or(0.0).clamp(0.0, 1.0) as f64;
    if progress <= 0.0 {
        return;
    }
     
     
    let half = 0.5 * progress;
    let centre = V3 {
        x: pos.x() as f64 + 0.5,
        y: pos.y() as f64 + 0.5,
        z: pos.z() as f64 + 0.5,
    };
    let min = V3 {
        x: centre.x - half,
        y: centre.y - half,
        z: centre.z - half,
    };
    let max = V3 {
        x: centre.x + half,
        y: centre.y + half,
        z: centre.z + half,
    };
    let body = fade(tint, 0.35);
    for face in box_faces(min, max) {
        let points: Option<Vec<Pos2>> = face.iter().map(|&p| view.project(p)).collect();
        if let Some(points) = points {
            painter.add(egui::Shape::convex_polygon(points, body, Stroke::NONE));
        }
    }
    let corners = box_corners(min, max);
    for &(a, c) in &EDGES {
        let (Some(pa), Some(pb)) = (view.project(corners[a]), view.project(corners[c])) else {
            continue;
        };
        painter.line_segment([pa, pb], Stroke::new(1.5_f32, tint));
    }
}

 
 
fn ease_outline(state: &mut EspState, target: Option<V3>, now: Instant) -> Option<V3> {
    let Some(target) = target else {
        state.outline_shown = None;
        return None;
    };
    let shown = match state.outline_shown {
        Some((from, last)) if now.duration_since(last) < OUTLINE_STALE_AFTER => {
            let dt = now.duration_since(last).as_secs_f64();
            from.lerp(target, 1.0 - (-OUTLINE_EASE_RATE * dt).exp())
        }
        _ => target,
    };
    state.outline_shown = Some((shown, now));
    Some(shown)
}

 
fn outline_time(now: Instant) -> f32 {
    now.duration_since(*OUTLINE_CLOCK).as_secs_f32()
}

 
fn frac(x: f32) -> f32 {
    x - x.floor()
}

 
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

 
fn liquid_hash(p: [f32; 3]) -> f32 {
    let q = [
        frac(p[0] * 0.1031),
        frac(p[1] * 0.1031),
        frac(p[2] * 0.1031),
    ];
    let d = q[0] * (q[1] + 33.33) + q[1] * (q[2] + 33.33) + q[2] * (q[0] + 33.33);
    frac(((q[0] + d) + (q[1] + d)) * (q[2] + d))
}

 
fn liquid_noise(p: [f32; 3]) -> f32 {
    let i = [p[0].floor(), p[1].floor(), p[2].floor()];
    let f = [0, 1, 2].map(|k| {
        let t = p[k] - i[k];
        t * t * (3.0 - 2.0 * t)
    });
    let h = |dx: f32, dy: f32, dz: f32| liquid_hash([i[0] + dx, i[1] + dy, i[2] + dz]);
    let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = mix(h(0.0, 0.0, 0.0), h(1.0, 0.0, 0.0), f[0]);
    let x10 = mix(h(0.0, 1.0, 0.0), h(1.0, 1.0, 0.0), f[0]);
    let x01 = mix(h(0.0, 0.0, 1.0), h(1.0, 0.0, 1.0), f[0]);
    let x11 = mix(h(0.0, 1.0, 1.0), h(1.0, 1.0, 1.0), f[0]);
    mix(mix(x00, x10, f[1]), mix(x01, x11, f[1]), f[2])
}

 
 
 
 
 
fn liquid_rgb(tint: Color32, p: V3, time: f32, light: f32) -> [f32; 3] {
    let (x, y) = (p.x as f32, p.y as f32);
    let flow = [
        x * 2.6,
        y * 2.6 - time * 1.8,
        p.z as f32 * 2.6 + time * 0.45,
    ];
    let detail = [
        flow[0] * 2.15 + 3.7,
        flow[1] * 2.15 - time * 2.4,
        flow[2] * 2.15 + 1.9,
    ];
    let n = 0.62 * liquid_noise(flow) + 0.38 * liquid_noise(detail);
    let wave = 0.5 + 0.5 * (y * 5.5 - time * 10.0 + n * 4.5).sin();
    let mix = smoothstep(0.20, 0.80, 0.65 * n + 0.35 * wave);
    let shimmer = 1.0 + 0.18 * (time * 10.0 - y * 4.0).sin();
    let dark = [14.0 / 255.0, 12.0 / 255.0, 13.0 / 255.0];
    let neon = [tint.r(), tint.g(), tint.b()].map(|c| c as f32 / 255.0);
    let glow = mix * mix * 0.45 * light;
    let mut rgb = [0.0; 3];
    for k in 0..3 {
        let base = dark[k] + (neon[k] - dark[k]) * mix;
        rgb[k] = (base * (0.35 + 0.65 * light * shimmer) + neon[k] * glow).clamp(0.0, 1.0);
    }
    rgb
}

 
 
 
 
 
 
pub(crate) fn liquid_sample(tint: Color32, point: [f32; 3], time: f32, light: f32) -> [f32; 3] {
    liquid_rgb(
        tint,
        V3 {
            x: point[0] as f64,
            y: point[1] as f64,
            z: point[2] as f64,
        },
        time,
        light,
    )
}

 
 
 
static PERM: LazyLock<[u8; 512]> = LazyLock::new(|| {
    let mut table: [u8; 256] = std::array::from_fn(|index| index as u8);
    let mut seed: u32 = 0x9E37_79B9;
    for index in (1..256).rev() {
         
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        table.swap(index, (seed % (index as u32 + 1)) as usize);
    }
    let mut perm = [0u8; 512];
    for (index, slot) in perm.iter_mut().enumerate() {
        *slot = table[index & 255];
    }
    perm
});

 
 
 
 
 
 
 
fn perlin(p: [f32; 3]) -> f32 {
    let ease = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let floor = [p[0].floor(), p[1].floor(), p[2].floor()];
    let f = [p[0] - floor[0], p[1] - floor[1], p[2] - floor[2]];
    let fade = [ease(f[0]), ease(f[1]), ease(f[2])];
    let perm = &PERM[..];
    let hash = |dx: i32, dy: i32, dz: i32| -> usize {
        let x = (floor[0] as i32 + dx) & 255;
        let y = (floor[1] as i32 + dy) & 255;
        let z = (floor[2] as i32 + dz) & 255;
        perm[(perm[(perm[x as usize] as usize + y as usize) & 255] as usize + z as usize) & 255]
            as usize
    };
     
    let gradient = |hash: usize, x: f32, y: f32, z: f32| match hash % 12 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        _ => -y - z,
    };
    let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let corner = |dx: i32, dy: i32, dz: i32| {
        gradient(
            hash(dx, dy, dz),
            f[0] - dx as f32,
            f[1] - dy as f32,
            f[2] - dz as f32,
        )
    };
    let x00 = mix(corner(0, 0, 0), corner(1, 0, 0), fade[0]);
    let x10 = mix(corner(0, 1, 0), corner(1, 1, 0), fade[0]);
    let x01 = mix(corner(0, 0, 1), corner(1, 0, 1), fade[0]);
    let x11 = mix(corner(0, 1, 1), corner(1, 1, 1), fade[0]);
    mix(mix(x00, x10, fade[1]), mix(x01, x11, fade[1]), fade[2]).clamp(-1.0, 1.0)
}

 
const OUTLINE_DARK: [f32; 3] = [14.0, 12.0, 13.0];

 
 
 
 
 
 
 
fn outline_mix(p: V3, time: f32) -> f32 {
    let drift = time * 0.18;
    let coarse = perlin([
        p.x as f32 * 1.15 + drift,
        p.y as f32 * 1.15 - drift * 0.6,
        p.z as f32 * 1.15 + 0.5,
    ]);
    let fine = perlin([
        p.x as f32 * 2.6 - drift * 1.3,
        p.y as f32 * 2.6 + drift * 0.9,
        p.z as f32 * 2.6 - 1.7,
    ]);
    smoothstep(-0.32, 0.34, 0.76 * coarse + 0.24 * fine)
}

 
 
 
 
 
 
 
fn outline_ramp(tint: Color32, mix: f32) -> Color32 {
    let mix = mix.clamp(0.0, 1.0);
    let dark = Color32::from_rgba_unmultiplied(
        OUTLINE_DARK[0] as u8,
        OUTLINE_DARK[1] as u8,
        OUTLINE_DARK[2] as u8,
        tint.a(),
    );
    let channel =
        |dark: u8, hot: u8| (dark as f32 + (hot as f32 - dark as f32) * mix).round() as u8;
    Color32::from_rgba_premultiplied(
        channel(dark.r(), tint.r()),
        channel(dark.g(), tint.g()),
        channel(dark.b(), tint.b()),
        tint.a(),
    )
}

 
fn outline_color(tint: Color32, p: V3, time: f32) -> Color32 {
    outline_ramp(tint, outline_mix(p, time))
}

 
fn box_faces(min: V3, max: V3) -> [[V3; 4]; 6] {
    let lo = [min.x, min.y, min.z];
    let hi = [max.x, max.y, max.z];
    let mut faces = [[V3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    }; 4]; 6];
    for axis in 0..3 {
        let (u, v) = match axis {
            0 => (1, 2),
            1 => (0, 2),
            _ => (0, 1),
        };
        for (side, plane) in [lo[axis], hi[axis]].into_iter().enumerate() {
            let corner = |cu: f64, cv: f64| {
                let mut p = [0.0; 3];
                p[axis] = plane;
                p[u] = cu;
                p[v] = cv;
                V3 {
                    x: p[0],
                    y: p[1],
                    z: p[2],
                }
            };
            faces[axis * 2 + side] = [
                corner(lo[u], lo[v]),
                corner(hi[u], lo[v]),
                corner(hi[u], hi[v]),
                corner(lo[u], hi[v]),
            ];
        }
    }
    faces
}

 
 
 
 
 
 
 
 
 
fn draw_block_outline(
    painter: &Painter,
    view: &View,
    origin: V3,
    boxes: &[[f64; 6]],
    tint: Color32,
    width: f32,
    time: f32,
) {
     
     
     
    const PIECE_PX: f32 = 5.0;
    for b in boxes {
        let min = V3 {
            x: origin.x + b[0],
            y: origin.y + b[1],
            z: origin.z + b[2],
        };
        let max = V3 {
            x: origin.x + b[3],
            y: origin.y + b[4],
            z: origin.z + b[5],
        };
        let corners = box_corners(min, max);
        for &(a, c) in &EDGES {
            let (pa, pb) = (corners[a], corners[c]);
            let (Some(sa), Some(sb)) = (view.project(pa), view.project(pb)) else {
                continue;
            };
            let pieces = ((sa.distance(sb) / PIECE_PX).ceil() as usize).clamp(1, 64);
            for piece in 0..pieces {
                let t0 = piece as f64 / pieces as f64;
                let t1 = (piece + 1) as f64 / pieces as f64;
                let (Some(p0), Some(p1)) =
                    (view.project(pa.lerp(pb, t0)), view.project(pa.lerp(pb, t1)))
                else {
                    continue;
                };
                let color = outline_color(tint, pa.lerp(pb, (t0 + t1) * 0.5), time);
                painter.line_segment([p0, p1], Stroke::new(width, color));
            }
        }
        for face in box_faces(min, max) {
            let points: Option<Vec<Pos2>> = face.iter().map(|&p| view.project(p)).collect();
            let Some(points) = points else {
                continue;
            };
            let centre = V3 {
                x: face.iter().map(|p| p.x).sum::<f64>() / 4.0,
                y: face.iter().map(|p| p.y).sum::<f64>() / 4.0,
                z: face.iter().map(|p| p.z).sum::<f64>() / 4.0,
            };
             
             
             
            let body = outline_color(tint, centre, time);
            let alpha = body.a().max(1) as f32;
            let fill = fade(body, OUTLINE_FILL_ALPHA as f32 / alpha);
            painter.add(egui::Shape::convex_polygon(points, fill, Stroke::NONE));
        }
    }
}

 
fn interp_factor(state: &EspState, now: Instant) -> f64 {
    match (state.prev_gather, state.last_gather) {
        (Some(prev), Some(last)) => {
            let span = (last - prev).as_secs_f64();
            if span <= 1e-4 {
                1.0
            } else {
                ((now - last).as_secs_f64() / span).clamp(0.0, 1.0)
            }
        }
        _ => 1.0,
    }
}

 

 
fn read_view(state: &mut EspState, ctx: &Context) -> Option<View> {
    if state.camera.is_none() {
        match minecraft()
            .game_renderer()
            .and_then(|gr| gr.get_main_camera())
        {
            Ok(camera) => state.camera = Some(camera),
            Err(e) => {
                log::debug!("ESP: camera unavailable: {e}");
                return None;
            }
        }
    }

    let camera = state.camera.clone()?;
    let rect = ctx.screen_rect();
    if rect.width() < 1.0 || rect.height() < 1.0 {
        return None;
    }

    let read = (|| -> anyhow::Result<(V3, V3, V3, V3)> {
        let (look, up, left) = camera.basis()?;
        Ok((
            camera.position()?.into(),
            look.into(),
            up.into(),
            left.into(),
        ))
    })();

    let (cam_pos, fwd, up, left) = match read {
        Ok(values) => values,
        Err(error) => {
            if !state.camera_logged {
                state.camera_logged = true;
                log::warn!("ESP: camera read failed: {error}");
            }
            return None;
        }
    };

     
     
    state.partial_tick = camera.partial_tick_time().unwrap_or(1.0);

    let fov_deg = resolve_fov(&camera, state, ctx);

    Some(build_view_from_basis(
        cam_pos,
        fwd,
        up,
        left,
        fov_deg,
        rect.width(),
        rect.height(),
    ))
}

 
 
 
 
 
 
 
 
fn resolve_fov(camera: &Camera, state: &mut EspState, _ctx: &Context) -> f64 {
     
     
     
     
     
     
     
    let rendered = game_fov(camera)
        .or_else(|| camera.fov().ok())
        .filter(|fov| fov.is_finite() && *fov > 0.0)
        .map(f64::from)
        .unwrap_or_else(formula_fov)
        .clamp(1.0, 179.0);

    state.target_fov = rendered;
    state.fov = rendered;
    rendered
}

 
 
fn game_fov(camera: &Camera) -> Option<f32> {
    let gr = minecraft().game_renderer().ok()?;
    let partial_tick = camera.partial_tick_time().ok()?;
    gr.get_fov(camera, partial_tick, true).ok()
}

 
 
 
 
 
 
 
 
fn formula_fov() -> f64 {
    let base = match read_option_fov() {
        Ok(fov) if fov.is_finite() && (1.0..=179.0).contains(&fov) => fov,
        _ => 70.0,
    };
    let raw = fov_modifier_raw();
    let scale = read_fov_effect_scale().unwrap_or(1.0).clamp(0.0, 1.0);
    let effective = 1.0 + (raw - 1.0) * scale;
    (base * effective).clamp(1.0, 179.0)
}

 
 
pub(crate) fn view_from_camera(ctx: &Context) -> Option<View> {
    let camera = minecraft()
        .game_renderer()
        .and_then(|gr| gr.get_main_camera())
        .ok()?;
    let rect = ctx.screen_rect();
    if rect.width() < 1.0 || rect.height() < 1.0 {
        return None;
    }
    let position = camera.position().ok()?;
    let (look, up, left) = camera.basis().ok()?;
    let fov = match camera.fov() {
        Ok(fov) if fov.is_finite() && fov > 0.0 => fov as f64,
        _ => formula_fov(),
    };
    Some(build_view_from_basis(
        position.into(),
        look.into(),
        up.into(),
        left.into(),
        fov.clamp(1.0, 179.0),
        rect.width(),
        rect.height(),
    ))
}

 
 
 
 
 
 
fn fov_modifier_raw() -> f64 {
    let player = match minecraft().player() {
        Ok(Some(player)) => player,
        _ => return 1.0,
    };

    let mut modifier = 1.0;
    if player.abilities.is_flying().unwrap_or(false) {
        modifier *= 1.1;
    }
    if player.is_sprinting().unwrap_or(false) {
        modifier *= 1.15;
    }
    modifier
}

 
fn read_option_fov() -> anyhow::Result<f64> {
    Ok(minecraft().options()?.fov()?.get_int()? as f64)
}

 
fn read_fov_effect_scale() -> anyhow::Result<f64> {
    minecraft().options()?.fov_effect_scale()?.get_double()
}

 

 
fn gather(state: &mut EspState, cfg: &EspConfig, now: Instant) {
    state.prev_gather = state.last_gather;
    state.last_gather = Some(now);

    let want_players = cfg.want_players();
    let want_mobs = cfg.want_mobs();
    let want_items = cfg.want_items();
    let want_projectiles = cfg.want_projectiles();
    if want_players || want_mobs || want_items || want_projectiles {
        let mut range = 0.0_f32;
        if cfg.player.enabled {
            range = range.max(cfg.player.range);
        }
        if cfg.mob.enabled {
            range = range.max(cfg.mob.range);
        }
        if cfg.box_esp.enabled {
            range = range.max(cfg.box_esp.range);
        }
        if cfg.tracer.enabled {
            range = range.max(cfg.tracer.range);
        }
        if cfg.nametag.enabled {
            range = range.max(cfg.nametag.range);
        }
        if cfg.infered {
            range = range.max(INFERED_MARKER_RANGE);
        }
        if cfg.item.enabled {
            range = range.max(cfg.item.range);
        }
        if cfg.projectile.enabled {
            range = range.max(cfg.projectile.range);
        }
        let range_sq = (range as f64) * (range as f64);

        let previous = std::mem::take(&mut state.entities);
        match gather_entities(
            &previous,
            range_sq,
            want_players,
            want_mobs,
            want_items,
            want_projectiles,
        ) {
            Ok(list) => state.entities = list,
            Err(e) => log::debug!("ESP: entity gather failed: {e}"),
        }
    } else {
        state.entities.clear();
    }

    if let Some(profile) = &cfg.chest {
        let due = state
            .last_chest_scan
            .is_none_or(|t| now - t >= CHEST_SCAN_INTERVAL);
        if due {
            state.last_chest_scan = Some(now);
            match gather_chests(&profile.blocks) {
                Ok(list) => state.chests = list,
                Err(e) => log::debug!("ESP: chest scan failed: {e}"),
            }
        }
    } else {
        state.chests.clear();
        state.last_chest_scan = None;
    }

    if cfg.spawner.is_some() {
        let due = state
            .last_spawner_scan
            .is_none_or(|t| now - t >= CHEST_SCAN_INTERVAL);
        if due {
            state.last_spawner_scan = Some(now);
            match gather_spawners() {
                Ok(list) => {
                     
                     
                    if let Some(profile) = &cfg.spawner {
                        announce_spawners(&list, profile);
                    }
                    state.spawners = list;
                }
                Err(e) => log::debug!("ESP: spawner scan failed: {e}"),
            }
        }
    } else {
        state.spawners.clear();
        state.last_spawner_scan = None;
    }
}

 
 
fn advance_blocks(state: &mut EspState, cfg: &EspConfig, now: Instant) {
    if !cfg.blocks.is_empty() {
         
         
         
        if sync_block_jobs(&cfg.blocks) {
             
             
            state.sweep = None;
            state.last_block_scan = None;
            state.block_primed = false;
        }

         
         
         
        let changed = take_block_changes();
        if !changed.is_empty() {
            forget_changed_blocks(&changed);
            if let Some(sweep) = state.sweep.as_mut() {
                sweep.work.forget_in_flight(&changed);
            }
            state.last_block_scan = None;
        }

        let due = state
            .last_block_scan
            .is_none_or(|t| now - t >= BLOCK_SCAN_INTERVAL);
        if state.sweep.is_some() || due {
            if state.sweep.is_none() {
                 
                 
                 
                request_block_scan(&cfg.blocks);
                match scan_center(state).map(BlockSweep::new) {
                    Some(Ok(sweep)) => state.sweep = Some(sweep),
                    Some(Err(error)) => {
                        log::debug!("ESP: block sweep could not start: {error}");
                    }
                    None => {}
                }
                 
                state.last_block_scan = Some(now);
            }
             
             
             
            let budget = if state.block_primed {
                BLOCK_SCAN_BUDGET
            } else {
                BLOCK_PRIME_BUDGET
            };
            let budget = crate::module::performance::client_optimizations::duration(
                ModuleId::BlockScanBudget,
                "Budget ms",
                budget,
                0.25,
                2.0,
            );
            let finished = state.sweep.as_mut().is_some_and(|sweep| sweep.step(budget));
            if finished {
                let sweep = state.sweep.take().expect("sweep just reported done");
                finish_sweep(state, sweep);
                state.block_primed = true;
            }
        }
    } else {
        let memory_empty = block_memory()
            .lock()
            .map(|memory| memory.values().all(|blocks| blocks.is_empty()))
            .unwrap_or(true);
        if state.sweep.is_some() || !memory_empty || !BLOCK_WORKER.jobs.lock().unwrap().is_empty() {
            state.sweep = None;
            clear_block_memory();
            state.last_block_scan = None;
            halt_block_scan();
        }
    }
}

 
 
#[allow(clippy::too_many_arguments)]
fn gather_entities(
    previous: &[EntityTarget],
    range_sq: f64,
    want_player: bool,
    want_mob: bool,
    want_item: bool,
    want_projectile: bool,
) -> anyhow::Result<Vec<EntityTarget>> {
    let mc = minecraft();
    let (Some(world), Some(player)) = (mc.world()?, mc.player()?) else {
        return Ok(Vec::new());
    };

    let local_id = player.id()?;
    let player_pos: V3 = player.get_position()?.into();

     
    let prev_pos: HashMap<i32, V3> = previous.iter().map(|e| (e.id, e.pos)).collect();

    let mut out: Vec<EntityTarget> = Vec::new();
    for entity in world.get_entities()? {
        if let Some(target) = process_entity(
            &entity,
            local_id,
            player_pos,
            range_sq,
            want_player,
            want_mob,
            want_item,
            want_projectile,
            &prev_pos,
        ) {
            out.push(target);
        }
    }

    Ok(out)
}

 
 
 
#[allow(clippy::too_many_arguments)]
fn process_entity(
    entity: &Entity,
    local_id: i32,
    player_pos: V3,
    range_sq: f64,
    want_player: bool,
    want_mob: bool,
    want_item: bool,
    want_projectile: bool,
    prev_pos: &HashMap<i32, V3>,
) -> Option<EntityTarget> {
     
     
    if let Ok(dist_sq) = entity.distance_to_sqr(player_pos.x, player_pos.y, player_pos.z) {
        if dist_sq > range_sq {
            return None;
        }
    }

    let kind = if want_player && entity.instance_of::<Player>() {
        TargetKind::Player
    } else if want_mob && entity.instance_of::<Mob>() {
        TargetKind::Mob
    } else if want_item && entity.instance_of::<crate::mapping::entity::ItemEntity>() {
        TargetKind::Item
    } else if want_projectile && entity.instance_of::<crate::mapping::entity::Projectile>() {
        TargetKind::Projectile
    } else {
        return None;
    };

    let id = entity.id().ok()?;
    if id == local_id {
        return None;
    }

    let pos: V3 = entity.get_position().ok()?.into();
    let dx = pos.x - player_pos.x;
    let dy = pos.y - player_pos.y;
    let dz = pos.z - player_pos.z;
    let distance = (dx * dx + dy * dy + dz * dz).sqrt() as f32;
    let width = entity.bb_width().ok()? as f64;
    let height = entity.bb_height().ok()? as f64;

    if kind == TargetKind::Player
        && !entity
            .as_living()
            .is_some_and(|living| living.is_alive().unwrap_or(false))
    {
        return None;
    }

    let name = read_name(entity);
    let (health, max_health) = read_health(entity).unwrap_or((0.0, 0.0));
    let skin_url = if kind == TargetKind::Player {
        Player::new(entity.jni_ref().clone())
            .player_info()
            .ok()
            .and_then(|info| info.skin_url().ok().flatten())
    } else {
        None
    };

    Some(EntityTarget {
        id,
        kind,
        prev: prev_pos.get(&id).copied().unwrap_or(pos),
        pos,
        width,
        height,
        name,
        health,
        max_health,
        distance,
        skin_url,
        player_jni: (kind == TargetKind::Player).then(|| entity.jni_ref().clone()),
    })
}

 
fn read_name(entity: &Entity) -> String {
    let name = entity
        .get_name()
        .and_then(|component| component.get_string())
        .unwrap_or_default();
    if name.chars().count() > 24 {
        name.chars().take(24).collect()
    } else {
        name
    }
}

 
fn read_health(entity: &Entity) -> Option<(f32, f32)> {
    let living = entity.as_living()?;
    Some((living.get_health().ok()?, living.get_max_health().ok()?))
}

 
 
 
 
 
 
 
static GEAR: OnceLock<Mutex<HashMap<i32, (Instant, Gear)>>> = OnceLock::new();

 
fn gear_for(player: &Player, gear_enabled: bool) -> Option<Gear> {
    if !gear_enabled {
        return None;
    }
    let id = player.id().ok()?;
    let cache = GEAR.get_or_init(|| Mutex::new(HashMap::new()));
     
     
    let cached = {
        let cache = cache.lock().ok()?;
        cache.get(&id).cloned()
    };
    let now = Instant::now();
    let interval = crate::module::performance::client_optimizations::duration(
        ModuleId::NametagGearOptimizer,
        "Refresh ms",
        GEAR_INTERVAL,
        200.0,
        3000.0,
    );
    if let Some((at, gear)) = &cached {
        if now - *at < interval {
            return Some(gear.clone());
        }
    }
    let previous = cached.map(|(_, gear)| gear);
    let gear = Gear::read(player).or(previous)?;
    if let Ok(mut cache) = cache.lock() {
        if cache.len() >= GEAR_CACHE_CAPACITY {
            cache.retain(|_, (at, _)| now - *at < GEAR_STALE_AFTER);
        }
        cache.insert(id, (now, gear.clone()));
    }
    Some(gear)
}

 
fn gather_chests(storage_blocks: &[String]) -> anyhow::Result<Vec<ChestTarget>> {
    let mc = minecraft();
    let (Some(world), Some(player)) = (mc.world()?, mc.player()?) else {
        return Ok(Vec::new());
    };

    let player_pos: V3 = player.get_position()?.into();
    let pcx = (player_pos.x / 16.0).floor() as i32;
    let pcz = (player_pos.z / 16.0).floor() as i32;

    let mut out: Vec<ChestTarget> = Vec::new();
    for cx in (pcx - CHEST_CHUNK_RADIUS)..=(pcx + CHEST_CHUNK_RADIUS) {
        for cz in (pcz - CHEST_CHUNK_RADIUS)..=(pcz + CHEST_CHUNK_RADIUS) {
            scan_chunk(&world, cx, cz, storage_blocks, &mut out)?;
        }
    }

    Ok(out)
}

 
fn scan_chunk(
    world: &World,
    cx: i32,
    cz: i32,
    storage_blocks: &[String],
    out: &mut Vec<ChestTarget>,
) -> anyhow::Result<()> {
    let Some(chunk) = world.get_chunk(cx, cz)? else {
        return Ok(());
    };

    for block_entity in chunk.get_block_entities()? {
        let Ok(pos) = block_entity.get_block_pos() else {
            continue;
        };
        let Ok(block_id) = world.block_registry_name(&pos) else {
            continue;
        };
        if !storage_blocks.iter().any(|selected| selected == &block_id) {
            continue;
        }
        out.push(ChestTarget {
            pos: V3 {
                x: pos.x() as f64,
                y: pos.y() as f64,
                z: pos.z() as f64,
            },
            is_spawner: false,
            color: None,
        });
    }
    Ok(())
}

 
 
fn gather_spawners() -> anyhow::Result<Vec<ChestTarget>> {
    let mc = minecraft();
    let (Some(world), Some(player)) = (mc.world()?, mc.player()?) else {
        return Ok(Vec::new());
    };

    let player_pos: V3 = player.get_position()?.into();
    let pcx = (player_pos.x / 16.0).floor() as i32;
    let pcz = (player_pos.z / 16.0).floor() as i32;

    let mut out: Vec<ChestTarget> = Vec::new();
    for cx in (pcx - CHEST_CHUNK_RADIUS)..=(pcx + CHEST_CHUNK_RADIUS) {
        for cz in (pcz - CHEST_CHUNK_RADIUS)..=(pcz + CHEST_CHUNK_RADIUS) {
            let Some(chunk) = world.get_chunk(cx, cz)? else {
                continue;
            };
            for block_entity in chunk.get_block_entities()? {
                if !block_entity.is_spawner() {
                    continue;
                }
                if let Ok(pos) = block_entity.get_block_pos() {
                    out.push(ChestTarget {
                        pos: V3 {
                            x: pos.x() as f64,
                            y: pos.y() as f64,
                            z: pos.z() as f64,
                        },
                        is_spawner: true,
                        color: None,
                    });
                }
            }
        }
    }
    Ok(out)
}

 
 
 
static ANNOUNCED_SPAWNERS: Mutex<Option<HashSet<(i32, i32, i32)>>> = Mutex::new(None);

 
fn spawner_distance(target: &ChestTarget, eye: [f64; 3]) -> f32 {
    let dx = target.pos.x - eye[0];
    let dy = target.pos.y - eye[1];
    let dz = target.pos.z - eye[2];
    (dx * dx + dy * dy + dz * dz).sqrt() as f32
}

 
 
 
 
 
 
 
 
fn spawners_to_announce(
    found: &[ChestTarget],
    announced: &mut HashSet<(i32, i32, i32)>,
    eye: [f64; 3],
    range: f32,
) -> Vec<((i32, i32, i32), f32)> {
    let mut fresh = Vec::new();
    for target in found {
         
         
        let key = (
            target.pos.x.floor() as i32,
            target.pos.y.floor() as i32,
            target.pos.z.floor() as i32,
        );
        if announced.contains(&key) {
            continue;
        }
        let distance = spawner_distance(target, eye);
        if distance > range {
            continue;
        }
        announced.insert(key);
        fresh.push((key, distance));
    }
    fresh.sort_by(|a, b| a.1.total_cmp(&b.1));
    fresh
}

 
 
 
 
 
 
fn announce_spawners(found: &[ChestTarget], profile: &BlockProfile) {
    if !profile.notify || found.is_empty() {
        return;
    }
     
     
    let Some(eye) = minecraft()
        .player()
        .ok()
        .flatten()
        .and_then(|player| player.get_position().ok())
        .map(|pos| [pos.x(), pos.y(), pos.z()])
    else {
        return;
    };
    let mut guard = ANNOUNCED_SPAWNERS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let announced = guard.get_or_insert_with(HashSet::new);
    let fresh = spawners_to_announce(found, announced, eye, profile.notify_range);
    let Some(&((x, y, z), nearest)) = fresh.first() else {
        return;
    };
     
     
    let (title, message) = if fresh.len() == 1 {
        (
            "Spawner found".to_string(),
            format!("{nearest:.0} blocks · {x} {y} {z}"),
        )
    } else {
        (
            format!("{} spawners found", fresh.len()),
            format!("Nearest {nearest:.0} blocks · {x} {y} {z}"),
        )
    };
    log::info!("Spawner Finder: {message}");
    Notification::send(NotificationType::Info, &title, &message);
}

 

 
type BlockMemory = HashMap<(i32, i32, i32), ChestTarget>;

 
 
 
 
 
 
static BLOCK_MEMORY: OnceLock<Mutex<HashMap<ModuleId, BlockMemory>>> = OnceLock::new();

 
 
const BLOCK_MEMORY_MAX: usize = 32768;

fn block_memory() -> &'static Mutex<HashMap<ModuleId, BlockMemory>> {
    BLOCK_MEMORY.get_or_init(|| Mutex::new(HashMap::new()))
}

 
 
 
 
 
 
 
static BLOCK_PENDING_EVICT: OnceLock<Mutex<HashMap<ModuleId, HashSet<(i32, i32)>>>> =
    OnceLock::new();

fn block_pending_evict() -> &'static Mutex<HashMap<ModuleId, HashSet<(i32, i32)>>> {
    BLOCK_PENDING_EVICT.get_or_init(|| Mutex::new(HashMap::new()))
}

 
 
fn clear_block_memory() {
    if let Ok(mut memory) = block_memory().lock() {
        memory.clear();
    }
    if let Ok(mut changes) = BLOCK_CHANGES.lock() {
        changes.clear();
    }
    if let Ok(mut pending) = block_pending_evict().lock() {
        pending.clear();
    }
}

 
 
static BLOCK_CHANGES: Mutex<Vec<(i32, i32, i32)>> = Mutex::new(Vec::new());

 
 
 
pub fn note_block_update(packet: &JObject) {
    if BLOCK_WORKER
        .jobs
        .lock()
        .map(|jobs| jobs.is_empty())
        .unwrap_or(true)
    {
        return;
    }
    let pos = mapping().in_frame(|| {
        let pos = mapping()
            .call_method(
                MinecraftClassType::ClientboundBlockUpdatePacket,
                packet,
                "getPos",
                &[],
            )?
            .l()?;
        BlockPos::read(&pos)
    });
    let Ok(pos) = pos else {
        log::debug!("ESP: block update position unreadable");
        return;
    };
    if let Ok(mut changes) = BLOCK_CHANGES.lock() {
        if changes.len() < BLOCK_CHANGES_MAX {
            changes.push((pos.x(), pos.y(), pos.z()));
        }
    }
}

 
fn take_block_changes() -> Vec<(i32, i32, i32)> {
    BLOCK_CHANGES
        .lock()
        .map(|mut changes| std::mem::take(&mut *changes))
        .unwrap_or_default()
}

 
 
fn forget_positions(memory: &mut BlockMemory, changed: &[(i32, i32, i32)]) -> usize {
    let mut removed = 0;
    for pos in changed {
        if memory.remove(pos).is_some() {
            removed += 1;
        }
    }
    removed
}

 
fn forget_changed_blocks(changed: &[(i32, i32, i32)]) {
    if let Ok(mut memory) = block_memory().lock() {
        for blocks in memory.values_mut() {
            forget_positions(blocks, changed);
        }
    }
}

 
 
fn clear_block_memory_for(source: ModuleId) {
    if let Ok(mut memory) = block_memory().lock() {
        memory.remove(&source);
    }
    if let Ok(mut pending) = block_pending_evict().lock() {
        pending.remove(&source);
    }
}

 
 
fn block_dist_sq(target: &ChestTarget, center: &V3) -> f64 {
    let dx = target.pos.x + 0.5 - center.x;
    let dy = target.pos.y + 0.5 - center.y;
    let dz = target.pos.z + 0.5 - center.z;
    dx * dx + dy * dy + dz * dz
}

 
 
 
static BLOCK_CACHE: OnceLock<Mutex<HashMap<String, Option<GlobalRef>>>> = OnceLock::new();

 
 
#[derive(Clone)]
struct BlockScanJob {
    source: ModuleId,
     
    range: f32,
     
    targets: Vec<GlobalRef>,
     
     
    blocks: Vec<String>,
     
     
    target_ids: Vec<String>,
     
    color: Color32,
    show_distance: bool,
     
    ore_colors: bool,
}

 
 
 
struct BlockWorker {
     
     
    jobs: Mutex<Vec<BlockScanJob>>,
}

static BLOCK_WORKER: BlockWorker = BlockWorker {
    jobs: Mutex::new(Vec::new()),
};

 
 
 
 
 
 
fn sync_block_jobs(profiles: &[BlockProfile]) -> bool {
    let mut reset: Vec<ModuleId> = Vec::new();
    {
        let mut jobs = BLOCK_WORKER.jobs.lock().unwrap();
        jobs.retain(|job| {
            let wanted = profiles.iter().any(|profile| profile.source == job.source);
            if !wanted {
                reset.push(job.source);
            }
            wanted
        });
        for profile in profiles {
            let changed = match jobs.iter().find(|job| job.source == profile.source) {
                Some(job) => job.blocks != profile.blocks,
                None => true,
            };
            if changed {
                reset.push(profile.source);
            }
        }
    }
    for source in reset.iter().copied() {
        clear_block_memory_for(source);
    }
    if !reset.is_empty() {
         
         
         
        clear_state_target_cache();
        clear_palette_target_cache();
    }
    !reset.is_empty()
}

 
 
 
fn request_block_scan(profiles: &[BlockProfile]) {
    let jobs = profiles
        .iter()
        .map(|profile| {
            let (targets, target_ids) = resolve_blocks(&profile.blocks);
            if targets.is_empty() && !profile.blocks.is_empty() {
                 
                 
                log::info!(
                    "{}: all {} configured block id(s) failed to resolve",
                    profile.source.display_name(),
                    profile.blocks.len()
                );
            }
            BlockScanJob {
                source: profile.source,
                range: profile.range,
                targets,
                blocks: profile.blocks.clone(),
                target_ids,
                color: profile.color,
                show_distance: profile.show_distance,
                ore_colors: profile.ore_colors,
            }
        })
        .collect();
    *BLOCK_WORKER.jobs.lock().unwrap() = jobs;
}

 
fn halt_block_scan() {
    BLOCK_WORKER.jobs.lock().unwrap().clear();
}

 
 
 
fn scan_center(state: &EspState) -> Option<V3> {
    state
        .camera
        .as_ref()
        .and_then(|camera| camera.position().ok())
        .map(|position| position.into())
        .or_else(|| {
            minecraft()
                .player()
                .ok()
                .flatten()
                .and_then(|player| player.get_position().ok())
                .map(|position| position.into())
        })
}

 
#[derive(Clone, Copy)]
struct SweepProfile {
    source: ModuleId,
     
    range: f32,
    range_sq: f64,
}

 
 
 
 
 
#[derive(Default)]
struct SweepRequest {
     
    ids: Vec<String>,
     
    blocks: Vec<GlobalRef>,
     
    owners: Vec<u32>,
     
    meta: Vec<SweepProfile>,
}

 
 
 
 
fn union_targets<T: Clone>(selections: &[Vec<(String, T)>]) -> (Vec<String>, Vec<T>, Vec<u32>) {
    let mut union: BTreeMap<String, (T, u32)> = BTreeMap::new();
    for (module_index, entries) in selections.iter().enumerate() {
        let bit = 1u32 << module_index;
        for (id, payload) in entries {
            let entry = union
                .entry(id.clone())
                .or_insert_with(|| (payload.clone(), 0));
            entry.1 |= bit;
        }
    }
    let mut ids = Vec::with_capacity(union.len());
    let mut payloads = Vec::with_capacity(union.len());
    let mut masks = Vec::with_capacity(union.len());
    for (id, (payload, mask)) in union {
        ids.push(id);
        payloads.push(payload);
        masks.push(mask);
    }
    (ids, payloads, masks)
}

 
fn build_sweep_request() -> SweepRequest {
    let jobs = BLOCK_WORKER.jobs.lock().unwrap().clone();
    let selections: Vec<Vec<(String, GlobalRef)>> = jobs
        .iter()
        .map(|job| {
            job.target_ids
                .iter()
                .cloned()
                .zip(job.targets.iter().cloned())
                .collect()
        })
        .collect();
    let (ids, blocks, owners) = union_targets(&selections);
    SweepRequest {
        ids,
        blocks,
        owners,
        meta: jobs
            .iter()
            .map(|job| SweepProfile {
                source: job.source,
                range: job.range,
                range_sq: (job.range as f64) * (job.range as f64),
            })
            .collect(),
    }
}

 
 
 
 
 
fn within_cylinder(dx: f64, dy: f64, dz: f64, range: f64, range_sq: f64) -> bool {
    dx * dx + dz * dz <= range_sq && dy.abs() <= range
}

 
 
 
 
 
 
fn finish_sweep(state: &mut EspState, sweep: BlockSweep) {
    let BlockSweep { work, .. } = sweep;
    let jobs = BLOCK_WORKER.jobs.lock().unwrap().clone();
    let mut remembered = 0usize;

    {
        let mut memory = block_memory().lock().unwrap();
        let mut pending = block_pending_evict().lock().unwrap();
        for (module_index, job) in jobs.iter().enumerate() {
            let bit = 1u32 << module_index;
            let memory = memory.entry(job.source).or_default();
            for (target, mask) in &work.found {
                if mask & bit == 0 {
                    continue;
                }
                let key = (
                    target.pos.x as i32,
                    target.pos.y as i32,
                    target.pos.z as i32,
                );
                if !memory.contains_key(&key) && memory.len() >= BLOCK_MEMORY_MAX {
                     
                     
                     
                     
                    if let Some((&far, _)) = memory.iter().max_by(|a, b| {
                        block_dist_sq(a.1, &work.center)
                            .partial_cmp(&block_dist_sq(b.1, &work.center))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    }) {
                        memory.remove(&far);
                    }
                }
                memory.insert(key, *target);
            }
             
             
             
             
             
             
             
            pending
                .entry(job.source)
                .or_default()
                .extend(work.verified.iter().copied());
            remembered += memory.len();
        }
    }

    if work.section_errors > 0 {
        log::warn!(
            "Block ESP: {} section(s) failed to read (skipped; see debug log)",
            work.section_errors
        );
    }
    log::debug!(
        "Block ESP sweep: modules={} radius={} chunks_loaded={} sections_scanned={} found={} verified={} center=({:.1}, {:.1}, {:.1})",
        jobs.len(),
        work.chunk_radius,
        work.loaded_chunks,
        work.sections_scanned,
        work.found.len(),
        work.verified.len(),
        work.center.x,
        work.center.y,
        work.center.z,
    );
    if remembered != state.last_logged_block_count {
        log::info!("Block ESP: {remembered} block(s) remembered");
        state.last_logged_block_count = remembered;
    }
}

 
 
 
 
fn resolve_blocks(ids: &[String]) -> (Vec<GlobalRef>, Vec<String>) {
    let cache = BLOCK_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut guard) = cache.lock() else {
        return (Vec::new(), Vec::new());
    };

    let mut resolved = Vec::new();
    let mut resolved_ids = Vec::new();
    for id in ids {
        let handle = guard.entry(id.clone()).or_insert_with(|| resolve_block(id));
        if let Some(global) = handle {
            resolved.push(global.clone());
            resolved_ids.push(id.clone());
        }
    }
    (resolved, resolved_ids)
}

 
 
 
 
 
 
 
pub(crate) fn resolve_block(id: &str) -> Option<GlobalRef> {
    let resolved = mapping().in_frame(|| {
        let env = mapping().get_env()?;
        let jid = env.new_string(id)?;
        let resource = mapping()
            .call_static_method(
                MinecraftClassType::ResourceLocation,
                "parse",
                &[JValue::Object(&jid)],
            )?
            .l()?;
        let registry = block_registry()?;
        let block = mapping()
            .call_method(
                MinecraftClassType::Registry,
                registry.as_obj(),
                "getValue",
                &[JValue::Object(&resource)],
            )?
            .l()?;
        if block.is_null() {
            anyhow::bail!("block '{id}' is not in the registry");
        }
        mapping().new_global_ref(block)
    });

    match resolved {
        Ok(handle) => Some(handle),
        Err(error) => {
            log::debug!("ESP: cannot resolve block '{id}': {error}");
            None
        }
    }
}

 
 
 
fn ring_offset(ring: i32, t: i32) -> (i32, i32) {
    let side = t / (2 * ring);
    let u = t % (2 * ring);
    match side {
        0 => (-ring + 1 + u, -ring),
        1 => (ring, -ring + 1 + u),
        2 => (ring - 1 - u, ring),
        _ => (-ring, ring - 1 - u),
    }
}

 
 
 
 
 
struct SweepCursor {
     
    pcx: i32,
    pcz: i32,
     
    chunk_radius: i32,
     
    sections_count: i32,
     
     
    ring: i32,
     
    ring_pos: usize,
     
    columns_opened: usize,
     
    columns_total: usize,
     
    current_column: Option<(i32, i32)>,
     
    section_cursor: i32,
     
    current_column_clean: bool,
}

 
 
trait SweepBackend {
     
     
    fn open_column(&mut self, cx: i32, cz: i32) -> bool;
     
     
    fn scan_next_section(&mut self, cx: i32, cz: i32, index: i32) -> bool;
     
    fn finish_column(&mut self, cx: i32, cz: i32, clean: bool);
}

impl SweepCursor {
    fn new(pcx: i32, pcz: i32, chunk_radius: i32, sections_count: i32) -> SweepCursor {
        let columns_total = (2 * chunk_radius as i64 + 1).pow(2).max(0) as usize;
        SweepCursor {
            pcx,
            pcz,
            chunk_radius,
            sections_count: sections_count.clamp(0, 4096),
            ring: 0,
            ring_pos: 0,
            columns_opened: 0,
            columns_total,
            current_column: None,
            section_cursor: 0,
            current_column_clean: true,
        }
    }

     
     
    fn next_column(&mut self) -> Option<(i32, i32)> {
        if self.columns_opened >= self.columns_total {
            return None;
        }
        let (dx, dz) = if self.ring == 0 {
            self.ring_pos += 1;
            self.ring = 1;
            (0, 0)
        } else {
            let size = 8 * self.ring as usize;
            let offset = ring_offset(self.ring, self.ring_pos as i32);
            self.ring_pos += 1;
            if self.ring_pos >= size {
                self.ring += 1;
                self.ring_pos = 0;
            }
            offset
        };
        self.columns_opened += 1;
        Some((self.pcx + dx, self.pcz + dz))
    }

     
     
     
     
     
    fn step(&mut self, budget: Duration, backend: &mut dyn SweepBackend) -> bool {
        let started = Instant::now();
        loop {
             
            if self.current_column.is_none() {
                let Some((cx, cz)) = self.next_column() else {
                    return true;  
                };
                if !backend.open_column(cx, cz) {
                     
                     
                    if started.elapsed() >= budget {
                        return self.done();
                    }
                    continue;
                }
                self.current_column = Some((cx, cz));
                self.section_cursor = 0;
                self.current_column_clean = true;
            }

             
            let (cx, cz) = self.current_column.expect("a column is open");
            if self.section_cursor < self.sections_count {
                let index = self.section_cursor;
                self.section_cursor += 1;
                let section_started = Instant::now();
                let section_clean = backend.scan_next_section(cx, cz, index);
                let section_cost = section_started.elapsed();
                if !section_clean {
                    self.current_column_clean = false;
                }
                 
                 
                 
                 
                if section_cost > budget {
                    static SLOW_SECTION_LOGGED: AtomicBool = AtomicBool::new(false);
                    if !SLOW_SECTION_LOGGED.swap(true, Ordering::Relaxed) {
                        log::warn!(
                            "Block ESP: one section cost {section_cost:?} (budget {budget:?}) — \
                             per-section work is over budget; see scan_section"
                        );
                    }
                }
            }

            if self.section_cursor >= self.sections_count {
                 
                backend.finish_column(cx, cz, self.current_column_clean);
                self.current_column = None;
            }

            if started.elapsed() >= budget {
                return self.done();
            }
        }
    }

     
    fn done(&self) -> bool {
        self.current_column.is_none() && self.columns_opened >= self.columns_total
    }
}

 
 
 
 
 
 
 
 
 
 
 
struct BlockSweep {
     
    cursor: SweepCursor,
     
    work: SweepWork,
}

 
 
 
 
struct SweepWork {
     
     
    center: V3,
    request: SweepRequest,
    min_y: i32,
     
    y_min: f64,
    y_max: f64,
     
    chunk_radius: i32,
     
    chunk: Option<LevelChunk>,
     
     
    found: Vec<(ChestTarget, u32)>,
     
    verified: Vec<(i32, i32)>,
     
     
    column_fresh: HashMap<(i32, i32, i32), u32>,
     
    evictions_this_tick: u32,
    loaded_chunks: u32,
    sections_scanned: u32,
    section_errors: u32,
}

impl SweepWork {
     
     
    fn forget_in_flight(&mut self, changed: &[(i32, i32, i32)]) {
        self.found.retain(|(target, _)| {
            let key = (
                target.pos.x as i32,
                target.pos.y as i32,
                target.pos.z as i32,
            );
            !changed.contains(&key)
        });
        self.column_fresh.retain(|pos, _| !changed.contains(pos));
    }
}

impl BlockSweep {
     
     
     
    fn new(center: V3) -> anyhow::Result<BlockSweep> {
        let mc = minecraft();
        let Some(world) = mc.world()? else {
            anyhow::bail!("no world loaded");
        };
        let request = build_sweep_request();
        if request.ids.is_empty() {
            anyhow::bail!("no block targets resolved");
        }

        let range = request
            .meta
            .iter()
            .map(|profile| profile.range)
            .fold(0.0_f32, f32::max);
        let pcx = (center.x / 16.0).floor() as i32;
        let pcz = (center.z / 16.0).floor() as i32;
        let chunk_radius = ((range / 16.0).ceil() as i32).clamp(1, 16);

        let min_y = world.get_min_y()?;
        let sections_count = world.sections_count()?;
         
         
         
         
        static RANGE_LOGGED: AtomicBool = AtomicBool::new(false);
        if !RANGE_LOGGED.swap(true, Ordering::Relaxed) {
            log::info!(
                "Block ESP: world min_y={min_y}, sections_count={sections_count}, \
                 world y span = [{}, {}], scan y span = [{}, {}]",
                min_y,
                min_y + sections_count * 16,
                center.y - range as f64,
                center.y + range as f64,
            );
        }

         
         
        Ok(BlockSweep {
            cursor: SweepCursor::new(pcx, pcz, chunk_radius, sections_count),
            work: SweepWork {
                center,
                request,
                min_y,
                y_min: center.y - range as f64,
                y_max: center.y + range as f64,
                chunk_radius,
                chunk: None,
                found: Vec::new(),
                verified: Vec::new(),
                column_fresh: HashMap::new(),
                evictions_this_tick: 0,
                loaded_chunks: 0,
                sections_scanned: 0,
                section_errors: 0,
            },
        })
    }

     
     
     
    fn step(&mut self, budget: Duration) -> bool {
        self.work.evictions_this_tick = 0;
        self.cursor.step(budget, &mut self.work)
    }
}

impl SweepBackend for SweepWork {
     
     
     
    fn open_column(&mut self, cx: i32, cz: i32) -> bool {
        let world = match minecraft().world() {
            Ok(world) => world,
            Err(error) => {
                if let Ok(env) = mapping().get_env() {
                    let _ = env.exception_clear();
                }
                log::debug!("Block ESP: world lookup failed: {error}");
                return false;
            }
        };
        let Some(world) = world else {
            return false;
        };
        match world.get_chunk(cx, cz) {
            Ok(Some(chunk)) => {
                self.chunk = Some(chunk);
                self.column_fresh.clear();
                self.loaded_chunks += 1;
                true
            }
            Ok(None) => false,
            Err(error) => {
                if let Ok(env) = mapping().get_env() {
                    let _ = env.exception_clear();
                }
                log::debug!("Block ESP: get_chunk({cx}, {cz}) failed: {error}");
                false
            }
        }
    }

     
     
     
     
    fn scan_next_section(&mut self, cx: i32, cz: i32, index: i32) -> bool {
        let base_y = self.min_y + index * 16;
         
         
         
        if (base_y as f64 + 16.0) < self.y_min || (base_y as f64) > self.y_max {
            return false;
        }
        let Some(chunk) = self.chunk.as_ref() else {
            return false;
        };
        let section = match chunk.get_section(index) {
            Ok(Some(section)) => section,
            Ok(None) => {
                 
                 
                return false;
            }
            Err(error) => {
                 
                 
                if let Ok(env) = mapping().get_env() {
                    let _ = env.exception_clear();
                }
                log::debug!("Block ESP: get_section({cx}, {index}, {cz}) failed: {error}");
                return false;
            }
        };
        let has_only_air = match section.has_only_air() {
            Ok(value) => value,
            Err(error) => {
                if let Ok(env) = mapping().get_env() {
                    let _ = env.exception_clear();
                }
                log::debug!("Block ESP: has_only_air({cx}, {index}, {cz}) failed: {error}");
                return false;
            }
        };
        if has_only_air {
            return true;
        }
         
         
         
        let mark = self.found.len();
        if let Err(e) = scan_section(
            cx,
            cz,
            base_y,
            &section,
            &self.request,
            self.center,
            &mut self.found,
        ) {
             
             
            let _ = mapping().get_env().map(|env| env.exception_clear());
            self.section_errors += 1;
            log::debug!("Block ESP: section ({cx}, {index}, {cz}) skipped: {e}");
            return false;
        }
         
         
         
        for (target, mask) in &self.found[mark..] {
            let key = (
                target.pos.x as i32,
                target.pos.y as i32,
                target.pos.z as i32,
            );
            *self.column_fresh.entry(key).or_insert(0) |= *mask;
        }
        self.sections_scanned += 1;
        true
    }

     
     
     
    fn finish_column(&mut self, cx: i32, cz: i32, clean: bool) {
        self.chunk = None;
        if clean {
            self.verified.push((cx, cz));
            self.evict_remembered(cx, cz);
        }
        self.column_fresh.clear();
    }
}

impl SweepWork {
     
     
     
     
     
     
     
     
    fn evict_remembered(&mut self, cx: i32, cz: i32) {
        let Ok(mut pending) = block_pending_evict().lock() else {
            return;
        };
        let Ok(mut memory) = block_memory().lock() else {
            return;
        };
        let fresh = &self.column_fresh;
        for (module_index, profile) in self.request.meta.iter().enumerate() {
            if self.evictions_this_tick >= BLOCK_EVICT_MAX_PASSES_PER_TICK {
                 
                 
                return;
            }
            let Some(queue) = pending.get_mut(&profile.source) else {
                continue;
            };
            if !queue.remove(&(cx, cz)) {
                 
                 
                continue;
            }
            let bit = 1u32 << module_index;
            let Some(blocks) = memory.get_mut(&profile.source) else {
                continue;
            };
             
             
             
            blocks.retain(|&(x, y, z), _target| {
                (x.div_euclid(16), z.div_euclid(16)) != (cx, cz)
                    || fresh.get(&(x, y, z)).is_some_and(|mask| mask & bit != 0)
            });
            self.evictions_this_tick += 1;
        }
    }
}

 
 
static FAST_PATH_LOGGED: AtomicBool = AtomicBool::new(false);
 
static FALLBACK_PATH_LOGGED: AtomicBool = AtomicBool::new(false);

 
 
 
 
 
 
 
 
 
fn scan_section(
    chunk_x: i32,
    chunk_z: i32,
    base_y: i32,
    section: &LevelChunkSection,
    request: &SweepRequest,
    center: V3,
    found: &mut Vec<(ChestTarget, u32)>,
) -> anyhow::Result<()> {
    let container = section.states()?;
    let data = container.data()?;
    let palette = Palette::new(data.palette()?);
    let storage = BitStorage::new(data.storage()?);

    let mut cache: HashMap<i32, Option<usize>> = HashMap::new();

     
     
     
    let single_state = palette.size()? <= 1;

     
     
     
    let mut emit = |i: usize, target_index: usize, found: &mut Vec<(ChestTarget, u32)>| {
        let (wx, wy, wz) = (
            chunk_x * 16 + (i & 15) as i32,
            base_y + ((i >> 8) & 15) as i32,
            chunk_z * 16 + ((i >> 4) & 15) as i32,
        );
        let dx = wx as f64 + 0.5 - center.x;
        let dy = wy as f64 + 0.5 - center.y;
        let dz = wz as f64 + 0.5 - center.z;
        let owners = request.owners[target_index];
        let id = request.ids[target_index].as_str();
        for (module_index, profile) in request.meta.iter().enumerate() {
            if owners & (1u32 << module_index) == 0 {
                continue;
            }
            if !within_cylinder(dx, dy, dz, profile.range as f64, profile.range_sq) {
                continue;
            }
            found.push((
                ChestTarget {
                    pos: V3 {
                        x: wx as f64,
                        y: wy as f64,
                        z: wz as f64,
                    },
                    is_spawner: false,
                    color: ore_color(Some(id)),
                },
                1u32 << module_index,
            ));
        }
    };

    match storage.unpack()? {
        Some(values) if !values.is_empty() => {
            if !FAST_PATH_LOGGED.swap(true, Ordering::Relaxed) {
                log::info!("Block ESP: bulk palette scan active");
            }
             
             
             
             
             
             
             
             
            let mut by_value: HashMap<i32, Vec<usize>> = HashMap::new();
            for (i, value) in values.into_iter().enumerate() {
                by_value.entry(value).or_default().push(i);
            }
            resolve_palette_values(&palette, &by_value, request, &mut cache, &mut emit, found)?;
        }
        Some(_) if single_state => {
             
             
             
            static SINGLE_STATE_LOGGED: AtomicBool = AtomicBool::new(false);
            if !SINGLE_STATE_LOGGED.swap(true, Ordering::Relaxed) {
                log::info!("Block ESP: single-state section path active");
            }
             
            let mut env = mapping().get_env()?;
            env.with_local_frame(64, |env| -> anyhow::Result<()> {
                if let Some(target_index) = palette_slot_target(
                    env,
                    &palette,
                    0,
                    &request.ids,
                    &request.blocks,
                    &mut cache,
                )? {
                    for i in 0..4096usize {
                        emit(i, target_index, found);
                    }
                }
                Ok(())
            })?;
        }
        _ => {
            if !FALLBACK_PATH_LOGGED.swap(true, Ordering::Relaxed) {
                log::info!(
                    "Block ESP: storage fast path unavailable — per-entry reads active (slow)"
                );
            }
             
             
             
             
            let size = storage.size()?.clamp(0, 4096) as usize;
            let mut by_value: HashMap<i32, Vec<usize>> = HashMap::new();
            for i in 0..size {
                let value = storage.get(i as i32)?;
                by_value.entry(value).or_default().push(i);
            }
            resolve_palette_values(&palette, &by_value, request, &mut cache, &mut emit, found)?;
        }
    }
    Ok(())
}

 
 
 
fn resolve_palette_values(
    palette: &Palette,
    by_value: &HashMap<i32, Vec<usize>>,
    request: &SweepRequest,
    cache: &mut HashMap<i32, Option<usize>>,
    mut emit: impl FnMut(usize, usize, &mut Vec<(ChestTarget, u32)>),
    found: &mut Vec<(ChestTarget, u32)>,
) -> anyhow::Result<()> {
    let values: Vec<i32> = by_value.keys().copied().collect();
    for batch in values.chunks(12) {
        let mut env = mapping().get_env()?;
        let result = env.with_local_frame(128, |env| -> anyhow::Result<()> {
            for value in batch {
                if let Some(target_index) =
                    palette_slot_target(env, palette, *value, &request.ids, &request.blocks, cache)?
                {
                    for &index in &by_value[value] {
                        emit(index, target_index, found);
                    }
                }
            }
            Ok(())
        });
         
         
         
        if let Err(error) = result {
            let _ = env.exception_clear();
            return Err(error);
        }
    }
    Ok(())
}

 

 
fn ore_color(id: Option<&str>) -> Option<Color32> {
    let id = id?;
    let id = id.strip_prefix("minecraft:").unwrap_or(id);
    let color = if id.contains("coal") {
        Color32::from_rgb(20, 20, 20)
    } else if id.contains("copper") {
        Color32::from_rgb(224, 118, 52)
    } else if id.contains("iron") {
        Color32::from_rgb(235, 235, 235)
    } else if id.contains("diamond") {
        Color32::from_rgb(40, 150, 255)
    } else if id.contains("ancient_debris") {
        Color32::from_rgb(125, 72, 38)
    } else if id.contains("redstone") {
        Color32::from_rgb(235, 35, 35)
    } else if id.contains("lapis") {
        Color32::from_rgb(35, 55, 150)
    } else if id.contains("emerald") {
        Color32::from_rgb(35, 210, 95)
    } else if id.contains("gold") {
        Color32::from_rgb(255, 205, 35)
    } else if id.contains("quartz") {
        Color32::from_rgb(245, 235, 220)
    } else {
        return None;
    };
    Some(color)
}

 
const EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0),  
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4),  
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),  
];

 
pub(crate) fn box_corners(min: V3, max: V3) -> [V3; 8] {
    [
        V3 {
            x: min.x,
            y: min.y,
            z: min.z,
        },
        V3 {
            x: max.x,
            y: min.y,
            z: min.z,
        },
        V3 {
            x: max.x,
            y: min.y,
            z: max.z,
        },
        V3 {
            x: min.x,
            y: min.y,
            z: max.z,
        },
        V3 {
            x: min.x,
            y: max.y,
            z: min.z,
        },
        V3 {
            x: max.x,
            y: max.y,
            z: min.z,
        },
        V3 {
            x: max.x,
            y: max.y,
            z: max.z,
        },
        V3 {
            x: min.x,
            y: max.y,
            z: max.z,
        },
    ]
}

 
 
pub(crate) fn draw_wire_box_width(
    painter: &Painter,
    view: &View,
    corners: &[V3; 8],
    color: Color32,
    width: f32,
) -> Option<Rect> {
    let projected: [Option<Pos2>; 8] = std::array::from_fn(|i| view.project(corners[i]));
    let stroke = Stroke::new(width, color);

    for &(a, b) in &EDGES {
        if let (Some(pa), Some(pb)) = (projected[a], projected[b]) {
            painter.line_segment([pa, pb], stroke);
        }
    }

    let mut bounds: Option<Rect> = None;
    for point in projected.into_iter().flatten() {
        bounds = Some(match bounds {
            Some(rect) => rect.union(Rect::from_min_max(point, point)),
            None => Rect::from_min_max(point, point),
        });
    }
    bounds
}

 
 
fn box_esp_bounds(entity: &EntityTarget, t: f64) -> (V3, V3) {
    let feet = entity.prev.lerp(entity.pos, t);
    let half = entity.width * 0.5;
    (
        V3 {
            x: feet.x - half,
            y: feet.y,
            z: feet.z - half,
        },
        V3 {
            x: feet.x + half,
            y: feet.y + entity.height,
            z: feet.z + half,
        },
    )
}

 
 
 
 
fn box_faces_facing(min: V3, max: V3, eye: V3) -> [bool; 6] {
    let axes = [
        (min.x, max.x, eye.x),
        (min.y, max.y, eye.y),
        (min.z, max.z, eye.z),
    ];
    let mut facing = [false; 6];
    for (axis, (lo, hi, eye)) in axes.into_iter().enumerate() {
        facing[axis * 2] = eye < lo;
        facing[axis * 2 + 1] = eye > hi;
    }
    facing
}

 
 
 
fn draw_box_esp(painter: &Painter, view: &View, entities: &[EntityTarget], t: f64, cfg: &BoxCfg) {
    if !cfg.enabled {
        return;
    }
    let thickness = cfg.thickness.clamp(0.5, 6.0);
    let fill = cfg
        .fill
        .then(|| fade(cfg.color, cfg.fill_opacity))
        .filter(|color| color.a() > 0);
    for entity in entities {
        let wanted = match entity.kind {
            TargetKind::Player => cfg.players,
            TargetKind::Mob => cfg.mobs,
            TargetKind::Item | TargetKind::Projectile => false,
        };
        if !wanted || entity.distance > cfg.range {
            continue;
        }
        let (min, max) = box_esp_bounds(entity, t);
        if let Some(fill) = fill {
            let facing = box_faces_facing(min, max, view.cam);
            for (index, face) in box_faces(min, max).into_iter().enumerate() {
                if !facing[index] {
                    continue;
                }
                let Some(points) = face
                    .iter()
                    .map(|&point| view.project(point))
                    .collect::<Option<Vec<Pos2>>>()
                else {
                    continue;
                };
                painter.add(Shape::convex_polygon(points, fill, Stroke::NONE));
            }
        }
        let corners = box_corners(min, max);
        draw_wire_box_width(painter, view, &corners, cfg.color, thickness);
    }
}

 
 
 
 
 
 
fn screen_bounds(view: &View, corners: &[V3; 8]) -> Option<Rect> {
    let mut bounds: Option<Rect> = None;
    for corner in corners {
        let Some(point) = view.project(*corner) else {
            continue;
        };
        bounds = Some(match bounds {
            Some(rect) => rect.union(Rect::from_min_max(point, point)),
            None => Rect::from_min_max(point, point),
        });
    }
    bounds
}

 
fn bracket_length(rect: Rect) -> f32 {
    (rect.width().min(rect.height()) * BRACKET_FRACTION).clamp(BRACKET_MIN, BRACKET_MAX)
}

 
 
 
 
 
 
 
fn fade(color: Color32, factor: f32) -> Color32 {
    let factor = factor.clamp(0.0, 1.0);
    let scale = |channel: u8| (channel as f32 * factor).round() as u8;
    Color32::from_rgba_premultiplied(
        scale(color.r()),
        scale(color.g()),
        scale(color.b()),
        scale(color.a()),
    )
}

 
 
 
 
 
fn lighten(color: Color32, amount: f32) -> Color32 {
    let amount = amount.clamp(0.0, 1.0);
     
     
     
     
    let alpha = color.a() as f32;
    let lift = |channel: u8| {
        (channel as f32 + (alpha - channel as f32) * amount)
            .round()
            .clamp(0.0, alpha) as u8
    };
    Color32::from_rgba_premultiplied(lift(color.r()), lift(color.g()), lift(color.b()), color.a())
}

 
fn breathe(time: f32, phase: f32) -> f32 {
    0.82 + 0.18 * (0.5 + 0.5 * (time * 2.1 + phase).sin())
}

 
 
fn target_phase(id: i32) -> f32 {
     
     
    let turns = (id.unsigned_abs() as f32 * 0.618_034).fract();
    turns * std::f32::consts::TAU
}

 
fn rounded_radius(rect: Rect) -> f32 {
    (rect.width().min(rect.height()) * ROUNDED_RADIUS_FRACTION)
        .clamp(ROUNDED_RADIUS_MIN, ROUNDED_RADIUS_MAX)
}

 
 
 
 
 
fn rounded_perimeter(rect: Rect, radius: f32) -> [Pos2; PERIMETER_POINTS] {
    let radius = radius.clamp(0.0, rect.width().min(rect.height()) * 0.5);
    let corners = [
        (
            pos2(rect.left() + radius, rect.top() + radius),
            std::f32::consts::PI,
        ),
        (
            pos2(rect.right() - radius, rect.top() + radius),
            1.5 * std::f32::consts::PI,
        ),
        (pos2(rect.right() - radius, rect.bottom() - radius), 0.0),
        (
            pos2(rect.left() + radius, rect.bottom() - radius),
            0.5 * std::f32::consts::PI,
        ),
    ];
    let mut points = [Pos2::ZERO; PERIMETER_POINTS];
    let mut index = 0;
    for (centre, start) in corners {
        for step in 0..=CORNER_STEPS {
            let angle = start + step as f32 / CORNER_STEPS as f32 * std::f32::consts::FRAC_PI_2;
            points[index] = pos2(
                centre.x + radius * angle.cos(),
                centre.y + radius * angle.sin(),
            );
            index += 1;
        }
    }
    points
}

 
 
 
 
 
 
fn draw_target_box(painter: &Painter, rect: Rect, color: Color32, look: TargetLook) {
    let pulse = if look.animate {
        breathe(look.time, look.phase)
    } else {
        1.0
    };
    match look.style {
        BoxStyle::Brackets => draw_bracket_box(painter, rect, color, look, pulse),
        BoxStyle::Rounded => draw_rounded_box(painter, rect, color, look, pulse),
    }
}

 
fn draw_bracket_box(painter: &Painter, rect: Rect, color: Color32, look: TargetLook, pulse: f32) {
    let rounding = Rounding::same(2.0);
    painter.rect_filled(rect, rounding, fade(color, 0.07 * pulse));
    if look.glow {
        painter.rect_stroke(
            rect,
            rounding,
            Stroke::new(TARGET_STROKE * 3.5, fade(color, 0.10 * pulse)),
        );
    }
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(TARGET_STROKE, fade(color, 0.55 * pulse)),
    );

    let length = bracket_length(rect);
    let stroke = Stroke::new(BRACKET_STROKE, fade(color, 0.95 * pulse));
    let halo = Stroke::new(BRACKET_STROKE * 2.6, fade(color, 0.12 * pulse));
    let (left, right) = (rect.left(), rect.right());
    let (top, bottom) = (rect.top(), rect.bottom());
    for (corner, horizontal) in [
        (pos2(left, top), 1.0),
        (pos2(right, top), -1.0),
        (pos2(left, bottom), 1.0),
        (pos2(right, bottom), -1.0),
    ] {
         
         
        let vertical = if corner.y < rect.center().y {
            1.0
        } else {
            -1.0
        };
        for (from, to) in [
            (corner, pos2(corner.x + horizontal * length, corner.y)),
            (corner, pos2(corner.x, corner.y + vertical * length)),
        ] {
            if look.glow {
                painter.line_segment([from, to], halo);
            }
            painter.line_segment([from, to], stroke);
        }
    }
}

 
 
fn draw_rounded_box(painter: &Painter, rect: Rect, color: Color32, look: TargetLook, pulse: f32) {
    let radius = rounded_radius(rect);
    let rounding = Rounding::same(radius);
    painter.rect_filled(rect, rounding, fade(color, 0.05 * pulse));
    if look.glow {
         
         
        painter.rect_stroke(
            rect,
            rounding,
            Stroke::new(6.0_f32, fade(color, 0.10 * pulse)),
        );
        painter.rect_stroke(
            rect,
            rounding,
            Stroke::new(2.8_f32, fade(color, 0.22 * pulse)),
        );
    }
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(1.6_f32, fade(color, 0.92 * pulse)),
    );

    if look.animate {
        draw_comet(painter, rect, radius, color, look, pulse);
    }
}

 
 
fn draw_comet(
    painter: &Painter,
    rect: Rect,
    radius: f32,
    color: Color32,
    look: TargetLook,
    pulse: f32,
) {
    let points = rounded_perimeter(rect, radius);
    let count = points.len();
    let head = (look.time * COMET_LAPS_PER_SECOND + look.phase / std::f32::consts::TAU)
        .rem_euclid(1.0)
        * count as f32;
    let steps = ((count as f32 * COMET_TAIL).round() as usize).clamp(4, count);
    let hot = lighten(color, 0.55);
    for offset in 0..steps {
        let index = (head as isize - offset as isize).rem_euclid(count as isize) as usize;
        let next = (index + 1) % count;
        let tail = offset as f32 / steps as f32;
         
         
        let tint = if tail < 0.3 {
            hot
        } else {
            lighten(color, 0.6 - 0.45 * tail)
        };
        let alpha = (1.0 - tail).powf(1.6) * pulse;
        let width = (3.2 - 2.0 * tail).max(1.0);
        painter.line_segment(
            [points[index], points[next]],
            Stroke::new(width, fade(tint, alpha)),
        );
    }

     
     
    let head_point = points[head as usize % count];
    painter.circle_filled(head_point, 4.2, fade(color, 0.28 * pulse));
    painter.circle_filled(head_point, 2.2, fade(hot, 0.95 * pulse));
}

 
 
fn draw_target_health(painter: &Painter, rect: Rect, fraction: f32) {
    let fraction = fraction.clamp(0.0, 1.0);
    let bar = Rect::from_min_max(
        pos2(rect.left() - 4.0, rect.top()),
        pos2(rect.left() - 1.5, rect.bottom()),
    );
    painter.rect_filled(bar, Rounding::same(1.0), Color32::from_black_alpha(150));
    let height = bar.height() * fraction;
    if height > 0.0 {
        let fill = Rect::from_min_max(pos2(bar.left(), bar.bottom() - height), bar.max);
        painter.rect_filled(fill, Rounding::same(1.0), health_color(fraction));
    }
}

 
 
fn draw_target_label(
    painter: &Painter,
    ctx: &Context,
    rect: Rect,
    entity: &EntityTarget,
    icfg: &EntityCfg,
    distance: f64,
) {
    let font = FontId::proportional(11.0);
    let name = if icfg.show_name {
        entity.name.trim().to_owned()
    } else {
        String::new()
    };
    let metres = if icfg.show_distance {
        format!("{distance:.0}m")
    } else {
        String::new()
    };
    if name.is_empty() && metres.is_empty() {
        return;
    }

    let distance_color = Color32::from_rgb(196, 199, 210);
    let (name_galley, metres_galley) = ctx.fonts(|fonts| {
        (
            fonts.layout_no_wrap(name.clone(), font.clone(), icfg.color),
            fonts.layout_no_wrap(metres.clone(), font.clone(), distance_color),
        )
    });
    let gap = if name.is_empty() || metres.is_empty() {
        0.0
    } else {
        5.0
    };
    let pad = 5.0;
    let width = name_galley.size().x + gap + metres_galley.size().x + pad * 2.0;
    let height = name_galley.size().y.max(metres_galley.size().y) + pad;
    let pill = Rect::from_center_size(
        pos2(rect.center().x, rect.top() - 4.0 - height * 0.5),
        vec2(width, height),
    );

    painter.rect_filled(
        pill,
        Rounding::same(height * 0.5),
        Color32::from_black_alpha(165),
    );
    let mut x = pill.left() + pad;
    if !name.is_empty() {
        let y = pill.center().y - name_galley.size().y * 0.5;
        painter.galley(pos2(x, y), name_galley.clone(), icfg.color);
        x += name_galley.size().x + gap;
    }
    if !metres.is_empty() {
        let y = pill.center().y - metres_galley.size().y * 0.5;
        painter.galley(pos2(x, y), metres_galley, distance_color);
    }
}

 
 
 
fn draw_entity(
    painter: &Painter,
    ctx: &Context,
    view: &View,
    entity: &EntityTarget,
    t: f64,
    now: Instant,
    cfg: &EspConfig,
) {
    let icfg = match entity.kind {
        TargetKind::Player => &cfg.player,
        TargetKind::Mob => &cfg.mob,
        TargetKind::Item => &cfg.item,
        TargetKind::Projectile => &cfg.projectile,
    };
     
     
    if !icfg.enabled {
        return;
    }

    let feet = entity.prev.lerp(entity.pos, t);
    let half = entity.width * 0.5;
    let corners = box_corners(
        V3 {
            x: feet.x - half,
            y: feet.y,
            z: feet.z - half,
        },
        V3 {
            x: feet.x + half,
            y: feet.y + entity.height,
            z: feet.z + half,
        },
    );

    let Some(mut rect) = screen_bounds(view, &corners) else {
        return;
    };
     
     
    if rect.width() < 6.0 {
        rect = rect.expand2(vec2(6.0 - rect.width(), 0.0) * 0.5);
    }
    if rect.height() < 8.0 {
        rect = rect.expand2(vec2(0.0, 8.0 - rect.height()) * 0.5);
    }

    draw_target_box(
        painter,
        rect,
        icfg.color,
        TargetLook {
            style: icfg.style,
            animate: icfg.animate,
            glow: icfg.glow,
            time: now.duration_since(*TARGET_CLOCK).as_secs_f32(),
            phase: target_phase(entity.id),
        },
    );
    if icfg.show_health && entity.max_health > 0.0 {
        draw_target_health(painter, rect, entity.health / entity.max_health);
    }
    draw_target_label(
        painter,
        ctx,
        rect,
        entity,
        icfg,
        feet.sub(view.cam).length(),
    );
}

type SkinPixels = (Vec<u8>, usize, usize);

 
 
 
 
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum PortraitSource {
    Skin(String),
    Avatar(String),
}

impl PortraitSource {
    fn for_player(url: Option<&str>, name: &str) -> Option<PortraitSource> {
        if let Some(url) = url.map(str::trim) {
            if url.starts_with("https://") || url.starts_with("http://") {
                return Some(PortraitSource::Skin(url.to_owned()));
            }
        }
        let name = name.trim();
        (!name.is_empty()).then(|| PortraitSource::Avatar(name.to_owned()))
    }
}

 
const PORTRAIT_RETRY_AFTER: std::time::Duration = std::time::Duration::from_secs(15);

static SKIN_PIXELS: OnceLock<Mutex<HashMap<i32, SkinPixels>>> = OnceLock::new();
static SKIN_REQUESTS: OnceLock<Mutex<HashSet<(i32, PortraitSource)>>> = OnceLock::new();
 
 
 
static SKIN_RETRY_AT: OnceLock<Mutex<HashMap<(i32, PortraitSource), std::time::Instant>>> =
    OnceLock::new();
static SKIN_TEXTURES: OnceLock<Mutex<HashMap<i32, egui::TextureHandle>>> = OnceLock::new();
 
 
static SKIN_WORKERS: OnceLock<Mutex<Vec<std::thread::JoinHandle<()>>>> = OnceLock::new();

 
 
 
pub(crate) fn shutdown_skin_loaders() {
    let Some(workers) = SKIN_WORKERS.get() else {
        return;
    };
    let handles = workers
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .drain(..)
        .collect::<Vec<_>>();
    for handle in handles {
        let _ = handle.join();
    }
}

 
 
 
pub(crate) fn skin_texture(
    ctx: &Context,
    id: i32,
    url: Option<&str>,
    name: &str,
) -> Option<egui::TextureId> {
    let textures = SKIN_TEXTURES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(texture) = textures.lock().ok()?.get(&id) {
        return Some(texture.id());
    }

    let pixels = SKIN_PIXELS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some((rgba, w, h)) = pixels.lock().ok()?.remove(&id) {
        let texture = ctx.load_texture(
            format!("player_esp_skin_{id}"),
            egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba),
            egui::TextureOptions::NEAREST,
        );
        let texture_id = texture.id();
        textures.lock().ok()?.insert(id, texture);
        return Some(texture_id);
    }

    let source = PortraitSource::for_player(url, name)?;
    let key = (id, source.clone());
    let retry_at = SKIN_RETRY_AT.get_or_init(|| Mutex::new(HashMap::new()));
    if retry_at
        .lock()
        .ok()?
        .get(&key)
        .is_some_and(|at| at.elapsed() < PORTRAIT_RETRY_AFTER)
    {
        return None;
    }

    let requests = SKIN_REQUESTS.get_or_init(|| Mutex::new(HashSet::new()));
    let should_start = requests.lock().ok()?.insert(key.clone());
    if should_start {
        let worker_key = key.clone();
        let worker = std::thread::spawn(move || {
            let head = match &worker_key.1 {
                PortraitSource::Skin(url) => crate::graphic::player_card::load_skin_head(url),
                PortraitSource::Avatar(name) => crate::graphic::player_card::load_avatar_head(name),
            };
            match head {
                Ok(head) => {
                    if let Ok(mut cache) = SKIN_PIXELS
                        .get_or_init(|| Mutex::new(HashMap::new()))
                        .lock()
                    {
                        cache.insert(worker_key.0, head);
                    }
                }
                Err(error) => {
                    log::debug!("Player ESP: portrait load failed for {worker_key:?}: {error:#}");
                     
                     
                    if let Some(requests) = SKIN_REQUESTS.get() {
                        if let Ok(mut requests) = requests.lock() {
                            requests.remove(&worker_key);
                        }
                    }
                    if let Ok(mut retries) = SKIN_RETRY_AT
                        .get_or_init(|| Mutex::new(HashMap::new()))
                        .lock()
                    {
                        retries.insert(worker_key, std::time::Instant::now());
                    }
                }
            }
        });
        SKIN_WORKERS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(worker);
    }
    None
}

 
 
 
const HUD_CARD_MIN_SCALE: f32 = 0.6;
const HUD_CARD_MAX_SCALE: f32 = 1.2;

 
 
 
const FRIEND_ACCENT: Color32 = Color32::from_rgb(96, 214, 138);

 
 
 
const FRIEND_BADGE_W: f32 = 36.0;

 
 
 
const INFERED_BADGE_W: f32 = 23.0;

 
 
 
 
 
 
 
 
 
fn nametag_card_geometry(scale: f32, friend: bool, infered: bool) -> (f32, f32) {
    let reserved =
        (if friend { FRIEND_BADGE_W } else { 0.0 }) + (if infered { INFERED_BADGE_W } else { 0.0 });
    let extra = reserved * scale;
    (170.0 * scale + extra, extra * 0.5)
}

fn hud_card_scale(distance: f32, range: f32, configured: f32) -> f32 {
    let configured = if configured.is_finite() {
        configured
    } else {
        1.0
    };
    let range = if range.is_finite() && range > 0.0 {
        range
    } else {
        64.0
    };
    let distance = if distance.is_finite() {
        distance.max(0.0)
    } else {
        range
    };
    let far = (distance / range).clamp(0.0, 1.0);
     
    let nudge = 1.03 - 0.06 * far;
    (configured * nudge).clamp(HUD_CARD_MIN_SCALE, HUD_CARD_MAX_SCALE)
}

 
 
 
 
fn draw_player_hud(
    painter: &Painter,
    ctx: &Context,
    view: &View,
    entity: &EntityTarget,
    t: f64,
    cfg: &EspConfig,
    editor_offset: egui::Vec2,
) {
    if entity.kind != TargetKind::Player {
        return;
    }
    let feet = entity.prev.lerp(entity.pos, t);
    if feet.sub(view.cam).length() > cfg.nametag.range as f64 {
        return;
    }
    let head = V3 {
        x: feet.x,
        y: feet.y + entity.height + 0.25,
        z: feet.z,
    };
    let Some(anchor) = view.project(head) else {
        return;
    };

     
     
     
    let distance = feet.sub(view.cam).length() as f32;
    let s = hud_card_scale(distance, cfg.nametag.range, cfg.player_hud_scale);
    let health_ratio = if entity.max_health > 0.0 {
        (entity.health / entity.max_health).clamp(0.0, 1.0)
    } else {
        0.0
    };

     
     
     
     
     
     
     
    let friend = crate::friends::is_friend(&entity.name);
    let infered = crate::module::render::infered_tag::is_infered(&entity.name);
    let (card_w, centre_nudge) = nametag_card_geometry(s, friend, infered);
    let mut card_h = 48.0 * s;
    let pad = 7.0 * s;
    let head = 36.0 * s;
    let radius = 9.0 * s;
     
    let gear = entity
        .player_jni
        .as_ref()
        .filter(|_| cfg.nametag.wants_gear())
        .and_then(|player| gear_for(&Player::new(player.clone()), true));
    let cells = gear
        .as_ref()
        .map(|gear| gear_cells(gear, &cfg.nametag))
        .unwrap_or_default();
    let gear_h = 22.0 * s;
    if !cells.is_empty() {
        card_h += gear_h + 4.0 * s;
    }
     
     
    let x = anchor.x + cfg.player_hud_x + editor_offset.x - centre_nudge;
    let y = anchor.y - card_h * 0.5 + cfg.player_hud_y + editor_offset.y;
    let card = Rect::from_min_size(pos2(x, y), vec2(card_w, card_h));
    let accent = theme::accent();
     
    let rim = if friend { FRIEND_ACCENT } else { accent };

     
    for (grow, alpha) in [(5.0_f32, 22_u8), (2.5, 40)] {
        painter.rect_filled(
            card.translate(vec2(0.0, 2.0 * s)).expand(grow * s),
            Rounding::same(radius + grow * s),
            Color32::from_black_alpha(alpha),
        );
    }
     
     
    painter.rect_filled(
        card,
        Rounding::same(radius),
        if friend {
            Color32::from_rgba_unmultiplied(12, 18, 14, 214)
        } else {
            Color32::from_rgba_unmultiplied(14, 11, 12, 214)
        },
    );
    painter.rect_filled(
        Rect::from_min_max(card.min, pos2(card.max.x, card.min.y + card_h * 0.5)),
        Rounding {
            nw: radius,
            ne: radius,
            sw: 0.0,
            se: 0.0,
        },
        Color32::from_white_alpha(7),
    );
    painter.rect_stroke(
        card,
        Rounding::same(radius),
        Stroke::new(
            1.0_f32,
            theme::with_alpha(rim, if friend { 0.7 } else { 0.45 }),
        ),
    );

     
     
     
     
    let head_rect = Rect::from_min_size(
        pos2(card.min.x + pad, card.min.y + 6.0 * s),
        vec2(head, head),
    );
    if let Some(texture) = skin_texture(ctx, entity.id, entity.skin_url.as_deref(), &entity.name) {
         
        crate::graphic::player_card::paint_head(painter, head_rect, texture);
    } else {
        painter.rect_filled(
            head_rect,
            Rounding::same(6.0 * s),
            theme::with_alpha(theme::ELEVATED, 0.9),
        );
        let initial = entity.name.chars().next().unwrap_or('?').to_string();
        painter.text(
            head_rect.center(),
            Align2::CENTER_CENTER,
            initial,
            FontId::proportional(17.0 * s),
            theme::with_alpha(accent, 0.9),
        );
    }
    painter.rect_stroke(
        head_rect,
        Rounding::same(6.0 * s),
        Stroke::new(
            1.0_f32,
            theme::with_alpha(rim, if friend { 0.6 } else { 0.35 }),
        ),
    );

     
    let text_x = head_rect.max.x + 8.0 * s;
    let text_right = card.max.x - pad;
    let text_w = text_right - text_x;

     
     
    let badge_font = FontId::proportional(7.5 * s);
    let badge = friend.then(|| {
        let galley = painter.layout_no_wrap("FRIEND".to_owned(), badge_font, FRIEND_ACCENT);
        let size = vec2(galley.size().x + 7.0 * s, 11.0 * s);
        let rect = Rect::from_min_size(pos2(card.max.x - pad - size.x, card.min.y + 5.0 * s), size);
        (rect, galley)
    });
     
     
    let logo_right = badge
        .as_ref()
        .map_or(card.max.x - pad, |(rect, _)| rect.min.x - 4.0 * s);
    let logo = infered.then(|| {
         
         
        let size = vec2(15.0 * s, 10.0 * s);
        Rect::from_min_size(pos2(logo_right - size.x, card.min.y + 7.0 * s), size)
    });
    let badge_w = badge
        .as_ref()
        .map_or(0.0, |(rect, _)| rect.width() + 4.0 * s)
        + logo.map_or(0.0, |rect| rect.width() + 4.0 * s);
    let name_font = FontId::proportional(12.0 * s);
    let name = fit_label(ctx, &entity.name, &name_font, (text_w - badge_w).max(12.0));
    painter.text(
        pos2(text_x, card.min.y + 6.0 * s),
        Align2::LEFT_TOP,
        name,
        name_font,
        if friend { FRIEND_ACCENT } else { theme::TEXT },
    );
    if let Some((rect, galley)) = badge {
        painter.rect_filled(
            rect,
            Rounding::same(rect.height() * 0.5),
            theme::with_alpha(FRIEND_ACCENT, 0.16),
        );
        painter.rect_stroke(
            rect,
            Rounding::same(rect.height() * 0.5),
            Stroke::new(1.0_f32, theme::with_alpha(FRIEND_ACCENT, 0.7)),
        );
        painter.galley(
            pos2(
                rect.center().x - galley.size().x * 0.5,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            FRIEND_ACCENT,
        );
    }
    if let Some(rect) = logo {
        match crate::graphic::branding::logo_texture(ctx) {
            Some(texture) => {
                painter.image(
                    texture,
                    rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    theme::brand(),
                );
            }
             
             
            None => {
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "INF",
                    FontId::proportional(5.5 * s),
                    theme::brand(),
                );
            }
        }
    }

    let stats_font = FontId::proportional(9.5 * s);
    let stats_y = card.min.y + 25.0 * s;
    if cfg.nametag.show_health {
        painter.text(
            pos2(text_x, stats_y),
            Align2::LEFT_CENTER,
            format!("{:.1} HP", entity.health),
            stats_font.clone(),
            player_health_color(health_ratio),
        );
    }
    if cfg.nametag.show_distance {
        painter.text(
            pos2(text_right, stats_y),
            Align2::RIGHT_CENTER,
            format!("{:.1}m", entity.distance),
            stats_font,
            theme::TEXT_DIM,
        );
    }

    if cfg.nametag.show_health {
        let track = Rect::from_min_max(
            pos2(text_x, card.min.y + 36.0 * s),
            pos2(text_right, card.min.y + 40.0 * s),
        );
        painter.rect_filled(
            track,
            Rounding::same(2.0 * s),
            Color32::from_white_alpha(26),
        );
        let fill = Rect::from_min_size(
            track.min,
            vec2(track.width() * health_ratio, track.height()),
        );
        if fill.width() > 0.0 {
            painter.rect_filled(
                fill,
                Rounding::same(2.0 * s),
                player_health_color(health_ratio),
            );
        }
    }

    if !cells.is_empty() {
        draw_gear_row(painter, ctx, &cells, card, s, &cfg.nametag);
    }
}

 
 
 
fn gear_cells<'a>(gear: &'a Gear, cfg: &NametagCfg) -> Vec<&'a GearStack> {
    let mut cells: Vec<&GearStack> = Vec::with_capacity(6);
    if cfg.armor {
        cells.extend(gear.armor.iter().flatten());
    }
    if cfg.hands {
        cells.extend(gear.main_hand.iter());
        cells.extend(gear.off_hand.iter());
    }
    cells
}

 
 
fn draw_gear_row(
    painter: &Painter,
    ctx: &Context,
    cells: &[&GearStack],
    card: Rect,
    s: f32,
    cfg: &NametagCfg,
) {
    let pad = 7.0 * s;
    let top = card.min.y + 48.0 * s;
    painter.line_segment(
        [pos2(card.min.x + pad, top), pos2(card.max.x - pad, top)],
        Stroke::new(1.0_f32, Color32::from_white_alpha(22)),
    );

    let icon = 16.0 * s;
    let gap = 4.0 * s;
    let row_top = top + 3.0 * s;
    let mut x = card.min.x + pad;
    for stack in cells {
        if x + icon > card.max.x - pad {
            break;
        }
        let tile = Rect::from_min_size(pos2(x, row_top), vec2(icon, icon));
        let durability = stack.durability();
        match crate::graphic::item_icon::icon(ctx, &stack.name) {
            Some((texture, tint)) => {
                painter.rect_filled(tile, Rounding::same(3.0 * s), theme::with_alpha(tint, 0.28));
                crate::graphic::player_card::paint_head(painter, tile, texture);
            }
            None => {
                painter.rect_filled(
                    tile,
                    Rounding::same(3.0 * s),
                    theme::with_alpha(theme::ELEVATED, 0.9),
                );
                painter.text(
                    tile.center(),
                    Align2::CENTER_CENTER,
                    item_initials(&stack.name),
                    FontId::proportional(7.0 * s),
                    theme::TEXT_DIM,
                );
            }
        }
         
        if let Some(ratio) = durability {
            let track = Rect::from_min_max(
                pos2(tile.min.x, tile.max.y + 1.5 * s),
                pos2(tile.max.x, tile.max.y + 3.5 * s),
            );
            painter.rect_filled(
                track,
                Rounding::same(1.0 * s),
                Color32::from_white_alpha(28),
            );
            let fill = Rect::from_min_size(track.min, vec2(track.width() * ratio, track.height()));
            if fill.width() > 0.0 {
                painter.rect_filled(fill, Rounding::same(1.0 * s), durability_color(ratio));
            }
            if cfg.durability {
                let left = stack.max_damage - stack.damage;
                painter.text(
                    pos2(tile.center().x, tile.max.y + 4.0 * s),
                    Align2::CENTER_TOP,
                    format!("{left}"),
                    FontId::proportional(6.5 * s),
                    durability_color(ratio),
                );
            }
        }
        x += icon + gap;
    }
}

 
fn durability_color(ratio: f32) -> Color32 {
    let ratio = ratio.clamp(0.0, 1.0);
    if ratio > 0.5 {
        Color32::from_rgb(95, 205, 105)
    } else if ratio > 0.2 {
        Color32::from_rgb(232, 197, 71)
    } else {
        Color32::from_rgb(226, 74, 74)
    }
}

 
fn item_initials(name: &str) -> String {
    let words: Vec<&str> = name.split('_').filter(|part| !part.is_empty()).collect();
    match words.len() {
        0 => "?".to_owned(),
        1 => words[0].chars().take(2).collect(),
        _ => words
            .iter()
            .take(2)
            .filter_map(|word| word.chars().next())
            .collect(),
    }
}

 
fn fit_label(ctx: &Context, text: &str, font: &FontId, max_w: f32) -> String {
    let width = |s: &str| {
        ctx.fonts(|f| {
            f.layout_no_wrap(s.to_owned(), font.clone(), Color32::WHITE)
                .size()
                .x
        })
    };
    if width(text) <= max_w {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while chars.len() > 1 {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>() + "…";
        if width(&candidate) <= max_w {
            return candidate;
        }
    }
    "…".to_owned()
}

fn player_health_color(ratio: f32) -> Color32 {
    Color32::from_rgb(
        (220.0 * (1.0 - ratio) + 55.0 * ratio) as u8,
        (70.0 + 170.0 * ratio) as u8,
        75,
    )
}

 
 
 
fn tracer_origin(width: f32, height: f32) -> Pos2 {
    pos2(width * 0.5, height * 0.5)
}

fn draw_tracer(painter: &Painter, view: &View, entity: &EntityTarget, t: f64, cfg: &TracerCfg) {
    let want = match entity.kind {
        TargetKind::Player => cfg.players,
        TargetKind::Mob => cfg.mobs,
        TargetKind::Item | TargetKind::Projectile => false,
    };
    if !want {
        return;
    }

    let feet = entity.prev.lerp(entity.pos, t);
    let center = V3 {
        x: feet.x,
        y: feet.y + entity.height * 0.5,
        z: feet.z,
    };
    if center.sub(view.cam).length() > cfg.range as f64 {
        return;
    }

    let Some(target) = view.project(center) else {
        return;
    };
    let origin = tracer_origin(view.w, view.h);
    painter.line_segment([origin, target], Stroke::new(LINE_WIDTH, cfg.color));
}

 
const INFERED_MARKER_RANGE: f32 = 64.0;

 
 
 
fn draw_infered_marker(
    painter: &Painter,
    ctx: &Context,
    view: &View,
    entity: &EntityTarget,
    t: f64,
) {
    if entity.kind != TargetKind::Player || entity.name.is_empty() {
        return;
    }
    if !crate::module::render::infered_tag::is_infered(&entity.name) {
        return;
    }
    let feet = entity.prev.lerp(entity.pos, t);
    if feet.sub(view.cam).length() > INFERED_MARKER_RANGE as f64 {
        return;
    }
    let head = V3 {
        x: feet.x,
        y: feet.y + entity.height + 0.32,
        z: feet.z,
    };
    let Some(anchor) = view.project(head) else {
        return;
    };
     
    let size = vec2(18.0, 12.0);
    let rect = Rect::from_min_size(pos2(anchor.x - size.x * 0.5, anchor.y - size.y), size);
    painter.rect_filled(
        rect.expand(2.5),
        Rounding::same(5.0),
        Color32::from_black_alpha(150),
    );
    match crate::graphic::branding::logo_texture(ctx) {
        Some(texture) => {
            painter.image(
                texture,
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                theme::brand(),
            );
        }
        None => {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "INF",
                FontId::proportional(7.0),
                theme::brand(),
            );
        }
    }
}

 
 
 
 
 
 
 
static PREVIOUS_POSITION: AtomicU8 = AtomicU8::new(PREVIOUS_POSITION_UNKNOWN);
const PREVIOUS_POSITION_UNKNOWN: u8 = 0;
const PREVIOUS_POSITION_SHORT: u8 = 1;
const PREVIOUS_POSITION_LONG: u8 = 2;
const PREVIOUS_POSITION_MISSING: u8 = 3;

 
 
fn read_double_fields(player: &LocalPlayer, fields: [&str; 3]) -> Option<V3> {
    Some(V3 {
        x: player
            .get_field(fields[0], FieldType::Double)
            .ok()?
            .d()
            .ok()?,
        y: player
            .get_field(fields[1], FieldType::Double)
            .ok()?
            .d()
            .ok()?,
        z: player
            .get_field(fields[2], FieldType::Double)
            .ok()?
            .d()
            .ok()?,
    })
}

 
fn previous_position(player: &LocalPlayer) -> Option<V3> {
    let mut mode = PREVIOUS_POSITION.load(Ordering::Relaxed);
    if mode == PREVIOUS_POSITION_UNKNOWN {
        mode = if read_double_fields(player, ["xo", "yo", "zo"]).is_some() {
            PREVIOUS_POSITION_SHORT
        } else if read_double_fields(player, ["xOld", "yOld", "zOld"]).is_some() {
            PREVIOUS_POSITION_LONG
        } else {
            PREVIOUS_POSITION_MISSING
        };
        PREVIOUS_POSITION.store(mode, Ordering::Relaxed);
    }
    match mode {
        PREVIOUS_POSITION_SHORT => read_double_fields(player, ["xo", "yo", "zo"]),
        PREVIOUS_POSITION_LONG => read_double_fields(player, ["xOld", "yOld", "zOld"]),
        _ => None,
    }
}

 
fn lerp_position(from: V3, to: V3, t: f64) -> V3 {
    V3 {
        x: from.x + (to.x - from.x) * t,
        y: from.y + (to.y - from.y) * t,
        z: from.z + (to.z - from.z) * t,
    }
}

 
 
 
 
 
 
 
 
 
fn interpolated_position(player: &LocalPlayer, partial: f32) -> anyhow::Result<V3> {
    let current: V3 = player.get_position()?.into();
    let t = if partial.is_finite() {
        partial.clamp(0.0, 1.0) as f64
    } else {
        1.0
    };
    Ok(match previous_position(player) {
        Some(previous) => lerp_position(previous, current, t),
        None => current,
    })
}

 
 
 
 
fn local_player_name(player: &LocalPlayer) -> String {
    let from_entity = player
        .get_name()
        .and_then(|component| component.get_string())
        .unwrap_or_default();
    if !from_entity.trim().is_empty() {
        return from_entity;
    }
    let from_profile = crate::mapping::profile::Profile::from_minecraft(minecraft())
        .and_then(|profile| profile.name())
        .unwrap_or_default();
    if from_profile.trim().is_empty() {
         
         
        static WARNED: AtomicBool = AtomicBool::new(false);
        if !WARNED.swap(true, Ordering::Relaxed) {
            log::warn!("Local player nametag: neither the entity nor the game profile gave a name");
        }
    }
    from_profile
}

fn draw_nametag(painter: &Painter, view: &View, entity: &EntityTarget, t: f64, cfg: &NametagCfg) {
    if entity.kind != TargetKind::Player || entity.name.is_empty() {
        return;
    }

    let feet = entity.prev.lerp(entity.pos, t);
    let head = V3 {
        x: feet.x,
        y: feet.y + entity.height + 0.45,
        z: feet.z,
    };
    if head.sub(view.cam).length() > cfg.range as f64 {
        return;
    }
    let Some(anchor) = view.project(head) else {
        return;
    };

    let mut label = entity.name.clone();
    if cfg.show_health && entity.max_health > 0.0 {
        label.push_str(&format!("  {:.0}\u{2764}", entity.health.max(0.0)));
    }
    if cfg.show_distance {
        label.push_str(&format!("  {:.0}m", feet.sub(view.cam).length()));
    }

    let galley = painter.layout_no_wrap(label, FontId::proportional(12.0), cfg.color);
    let pad = vec2(5.0, 2.5);
    let size = galley.size() + pad * 2.0;
     
    let rect = Rect::from_min_size(pos2(anchor.x - size.x * 0.5, anchor.y - size.y), size);
    painter.rect_filled(rect, Rounding::same(3.0), Color32::from_black_alpha(180));
    painter.galley(rect.min + pad, galley, cfg.color);
}

 
fn block_cell(pos: V3) -> Cell {
    (
        pos.x.floor() as i32,
        pos.y.floor() as i32,
        pos.z.floor() as i32,
    )
}

 
fn face_quad(face: block_group::Face) -> [V3; 4] {
    let (u, v) = match face.axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let mut base = [face.cell.0 as f64, face.cell.1 as f64, face.cell.2 as f64];
    if face.plus {
        base[face.axis] += 1.0;
    }
    [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(du, dv)| {
        let mut p = base;
        p[u] += du;
        p[v] += dv;
        V3 {
            x: p[0],
            y: p[1],
            z: p[2],
        }
    })
}

 
 
 
 
 
 
fn draw_block_profile(
    painter: &Painter,
    view: &View,
    profile: &BlockProfile,
    draws: &[BlockDraw],
    label_budget: bool,
) {
    if draws.is_empty() {
        return;
    }
    let colors: HashMap<Cell, Color32> = draws
        .iter()
        .map(|draw| (block_cell(draw.target.pos), draw.color))
        .collect();
    let cells: Vec<Cell> = draws
        .iter()
        .map(|draw| block_cell(draw.target.pos))
        .collect();
    let groups: Vec<Vec<Cell>> = block_group::components(&cells);
    let to_v3 = |p: [f64; 3]| V3 {
        x: p[0],
        y: p[1],
        z: p[2],
    };

    for group in &groups {
        let Some((min, max)) = block_group::bounds(group) else {
            continue;
        };
        let color = colors.get(&group[0]).copied().unwrap_or(profile.color);
        let set: HashSet<Cell> = group.iter().copied().collect();

         
        let mut screen: Option<Rect> = None;
        for edge in block_group::crease_edges(&set) {
            let (a, b) = edge.endpoints();
            let pa = view.project(to_v3(a));
            let pb = view.project(to_v3(b));
            if let (Some(pa), Some(pb)) = (pa, pb) {
                painter.line_segment([pa, pb], Stroke::new(LINE_WIDTH, color));
            }
            for point in [pa, pb].into_iter().flatten() {
                let rect = Rect::from_min_max(point, point);
                screen = Some(screen.map_or(rect, |bounds| bounds.union(rect)));
            }
        }

        if profile.shade {
            let fill = Color32::from_rgba_unmultiplied(
                color.r(),
                color.g(),
                color.b(),
                (profile.shade_opacity.clamp(0.0, 1.0) * 255.0) as u8,
            );
            for face in block_group::exposed_faces(&set) {
                let corners: Option<Vec<Pos2>> = face_quad(face)
                    .into_iter()
                    .map(|corner| view.project(corner))
                    .collect();
                if let Some(points) = corners {
                    painter.add(Shape::convex_polygon(points, fill, Stroke::NONE));
                }
            }
        }

        let center = to_v3([
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ]);
        if label_budget && profile.show_distance {
            if let Some(rect) = screen {
                let distance = center.sub(view.cam).length();
                draw_label(
                    painter,
                    pos2(rect.center().x, rect.bottom() + 3.0),
                    Align2::CENTER_TOP,
                    &format!("{distance:.0}m"),
                    color,
                );
            }
        }

        if profile.tracers && center.sub(view.cam).length() <= profile.range as f64 {
            if let Some(target) = view.project(center) {
                let origin = tracer_origin(view.w, view.h);
                painter.line_segment([origin, target], Stroke::new(LINE_WIDTH, color));
            }
        }
    }
}

 
fn health_color(fraction: f32) -> Color32 {
    let (r, g) = if fraction < 0.5 {
        (255.0, 255.0 * (fraction * 2.0))
    } else {
        (255.0 * (1.0 - (fraction - 0.5) * 2.0), 255.0)
    };
    Color32::from_rgb(r as u8, g as u8, 60)
}

 
fn draw_label(painter: &Painter, pos: Pos2, anchor: Align2, text: &str, color: Color32) {
    let font = FontId::proportional(11.0);
    painter.text(
        pos + vec2(0.8, 0.8),
        anchor,
        text,
        font.clone(),
        Color32::from_black_alpha(210),
    );
    painter.text(pos, anchor, text, font, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v3(x: f64, y: f64, z: f64) -> V3 {
        V3 { x, y, z }
    }

     
     
    #[test]
    fn the_tag_anchor_lerps_between_ticks() {
        let from = v3(0.0, 64.0, 0.0);
        let to = v3(1.0, 64.0, -2.0);
        let start = lerp_position(from, to, 0.0);
        assert!((start.x - 0.0).abs() < 1e-9 && (start.z - 0.0).abs() < 1e-9);
        let end = lerp_position(from, to, 1.0);
        assert!((end.x - 1.0).abs() < 1e-9 && (end.z + 2.0).abs() < 1e-9);
        let half = lerp_position(from, to, 0.5);
        assert!((half.x - 0.5).abs() < 1e-9, "x {}", half.x);
        assert!((half.z + 1.0).abs() < 1e-9, "z {}", half.z);
        assert!((half.y - 64.0).abs() < 1e-9);
    }

    #[test]
    fn forget_positions_drops_only_the_changed_blocks() {
        let target = ChestTarget {
            pos: v3(0.0, 0.0, 0.0),
            is_spawner: false,
            color: None,
        };
        let mut memory: BlockMemory = HashMap::new();
        memory.insert((1, 2, 3), target);
        memory.insert((4, 5, 6), target);

        let removed = forget_positions(&mut memory, &[(1, 2, 3), (9, 9, 9)]);

        assert_eq!(removed, 1);
        assert!(!memory.contains_key(&(1, 2, 3)));
        assert!(memory.contains_key(&(4, 5, 6)));
    }

     
     
    fn probe_outline(cam: V3, origin: V3, boxes: &[[f64; 6]]) -> (Vec<Color32>, usize) {
        let ctx = Context::default();
        let tint = Color32::from_rgb(255, 0, 0);
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(LayerId::new(
                Order::Background,
                egui::Id::new("outline_probe"),
            ));
            let view = build_view(cam, 0.0, 0.0, 70.0, 800.0, 600.0);
            draw_block_outline(&painter, &view, origin, boxes, tint, 6.0, 1.0);
        });
        let mut edge_colors = Vec::new();
        let mut fills = 0;
        for shape in output.shapes {
            match shape.shape {
                egui::Shape::LineSegment { stroke, .. } => {
                    assert_eq!(stroke.width, 6.0);
                    match stroke.color {
                        egui::epaint::ColorMode::Solid(c) => edge_colors.push(c),
                        _ => panic!("Outline edges must have a solid colour"),
                    }
                }
                egui::Shape::Path(path) => {
                    assert!(path.fill.a() <= OUTLINE_FILL_ALPHA);
                    fills += 1;
                }
                other => panic!("Unexpected block outline shape: {other:?}"),
            }
        }
        (edge_colors, fills)
    }

    #[test]
    fn block_outline_is_thick_noise_edges_over_a_faint_fill() {
        let (edge_colors, fills) = probe_outline(
            v3(0.0, 0.0, 0.0),
            v3(0.0, 0.0, 0.0),
            &[[-0.5, -0.5, 3.0, 0.5, 0.5, 4.0]],
        );
         
         
        assert!(
            edge_colors.len() >= 12 * 6,
            "only {} edge pieces",
            edge_colors.len()
        );
        assert_eq!(fills, 6);
        let first = edge_colors[0];
        assert!(
            edge_colors.iter().any(|c| *c != first),
            "edge colours must flow along the outline"
        );
         
         
        for color in &edge_colors {
            assert_eq!(color.a(), 255);
            assert!(
                color.r() >= 14 && color.g() <= 12,
                "{color:?} is off the ramp"
            );
        }
    }

    #[test]
    fn block_outline_follows_each_shape_box_not_the_full_cube() {
         
        let fence = [
            [0.375, 0.0, 0.375, 0.625, 1.0, 0.625],
            [0.0, 0.375, 0.4375, 1.0, 0.5625, 0.5625],
        ];
        let (edge_colors, fills) = probe_outline(v3(0.0, 0.0, 0.0), v3(0.0, 0.0, 3.0), &fence);
         
         
         
        assert_eq!(fills, 2 * 6);
        assert!(
            edge_colors.len() >= 2 * 12,
            "only {} edge pieces",
            edge_colors.len()
        );
    }

     
     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn block_outline_preview() {
        let (w, h) = (900.0_f32, 520.0_f32);
        let ctx = Context::default();
        let view = build_view(v3(0.8, 0.6, 0.0), 0.0, 6.0, 70.0, w, h);
        let tint = Color32::from_rgb(229, 28, 42);
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(w, h))),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(LayerId::new(
                    Order::Background,
                    egui::Id::new("block_outline_preview"),
                ));
                for x in [-1.6_f64, 0.0, 1.6] {
                    draw_block_outline(
                        &painter,
                        &view,
                        v3(x, 0.0, 5.0),
                        &[[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]],
                        tint,
                        2.0,
                        x as f32 * 4.0,
                    );
                }
            },
        );
        let mut textures = HashMap::new();
        crate::graphic::preview::gather_textures(&mut textures, &output.textures_delta);
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        let image = crate::graphic::preview::rasterize(
            primitives,
            &textures,
            w as usize,
            h as usize,
            [0.48, 0.58, 0.72, 1.0],
        );
         
         
        let hot = image
            .pixels()
            .filter(|p| p[0] > 120 && p[0] > p[2].saturating_add(60))
            .count();
        let cold = image.pixels().filter(|p| p[0] < 45 && p[2] < 50).count();
        assert!(hot > 200, "only {hot} hot pixels");
        assert!(cold > 200, "only {cold} black pixels");

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).unwrap();
        image.save(dir.join("block-outline.png")).unwrap();
    }

     
     
    #[test]
    fn perlin_noise_is_zero_on_the_lattice_and_smooth_between() {
        for point in [[0.0, 0.0, 0.0], [3.0, -2.0, 7.0], [-8.0, 4.0, 0.0]] {
            assert_eq!(
                perlin(point),
                0.0,
                "lattice points are the field's zero crossings"
            );
        }
        let mut previous = perlin([0.3, 0.7, 1.1]);
        for step in 1..200 {
            let p = [0.3 + step as f32 * 0.01, 0.7, 1.1];
            let value = perlin(p);
            assert!(value.abs() <= 1.0);
            assert!((value - previous).abs() < 0.35, "the field jumped at {p:?}");
            previous = value;
        }
    }

     
     
     
    #[test]
    fn the_outline_pattern_is_a_continuous_field() {
        let tint = Color32::from_rgb(255, 0, 0);
        let start = V3 {
            x: 12.0,
            y: 64.0,
            z: -30.0,
        };
        let mut previous = outline_color(tint, start, 1.0);
        let (mut darkest, mut lightest) = (255u8, 0u8);
        for step in 1..=320 {
            let p = V3 {
                x: start.x + step as f64 * 0.0125,
                y: start.y,
                z: start.z,
            };
            let color = outline_color(tint, p, 1.0);
            assert!(
                previous.r().abs_diff(color.r()) <= 24,
                "a 1/80-block step jumped {previous:?} -> {color:?}"
            );
            darkest = darkest.min(color.r());
            lightest = lightest.max(color.r());
            previous = color;
        }
        assert!(
            lightest - darkest > 120,
            "the sweep must cross from black to red: {darkest}..{lightest}"
        );
         
        let elsewhere = V3 {
            x: start.x + 7.0,
            ..start
        };
        assert_ne!(
            outline_color(tint, start, 1.0),
            outline_color(tint, elsewhere, 1.0)
        );
    }

     
    #[test]
    fn the_outline_ramp_runs_from_black_to_the_module_colour() {
        let tint = Color32::from_rgba_unmultiplied(255, 32, 48, 200);
        assert_eq!(
            outline_ramp(tint, 0.0),
            Color32::from_rgba_unmultiplied(14, 12, 13, 200)
        );
         
         
        assert_eq!(outline_ramp(tint, 1.0), tint);
        assert_eq!(outline_ramp(tint, -4.0), outline_ramp(tint, 0.0));
        assert_eq!(outline_ramp(tint, 4.0), outline_ramp(tint, 1.0));
    }

    #[test]
    fn outline_glides_toward_a_new_block_instead_of_snapping() {
        let mut state = EspState::new();
        let start = Instant::now();
        let first = ease_outline(&mut state, Some(v3(0.0, 0.0, 0.0)), start);
        assert_eq!(first.map(|p| p.x), Some(0.0));

        let moving = ease_outline(
            &mut state,
            Some(v3(1.0, 0.0, 0.0)),
            start + Duration::from_millis(16),
        )
        .expect("outline stays visible");
        assert!(moving.x > 0.0 && moving.x < 1.0, "moved to {}", moving.x);

        let mut now = start + Duration::from_millis(16);
        let mut settled = moving;
        for _ in 0..100 {
            now += Duration::from_millis(16);
            settled = ease_outline(&mut state, Some(v3(1.0, 0.0, 0.0)), now)
                .expect("outline stays visible");
        }
        assert!((settled.x - 1.0).abs() < 1e-3);
    }
    fn target(kind: TargetKind) -> EntityTarget {
        EntityTarget {
            id: 1,
            kind,
            prev: v3(0.0, 0.0, 3.0),
            pos: v3(0.0, 0.0, 3.0),
            width: 0.6,
            height: 1.8,
            name: "Alex".to_owned(),
            health: 12.0,
            max_health: 20.0,
            distance: 3.0,
            skin_url: None,
            player_jni: None,
        }
    }

    fn entity_cfg() -> EntityCfg {
        EntityCfg {
            enabled: true,
            color: Color32::from_rgb(255, 0, 0),
            show_name: true,
            show_distance: true,
            show_health: true,
            range: 64.0,
            style: BoxStyle::Brackets,
            animate: true,
            glow: true,
        }
    }

     
     
    fn probe_shapes(paint: impl FnOnce(&Painter, &Context)) -> Vec<egui::Shape> {
        let ctx = Context::default();
        let mut paint = Some(paint);
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(LayerId::new(
                Order::Background,
                egui::Id::new("target_probe"),
            ));
            if let Some(paint) = paint.take() {
                paint(&painter, ctx);
            }
        });
        output.shapes.into_iter().map(|shape| shape.shape).collect()
    }

     
     
    #[test]
    fn the_esp_draws_in_the_hud_layer_under_every_piece_of_ui() {
        assert_eq!(
            overlay_layer(),
            LayerId::new(Order::Background, crate::graphic::hud::layer_id())
        );
    }

     
     
    #[test]
    fn a_target_reads_as_one_screen_space_box() {
        let view = build_view(v3(0.0, 0.0, 0.0), 0.0, 0.0, 70.0, 800.0, 600.0);
        let corners = box_corners(v3(-0.3, 0.0, 2.7), v3(0.3, 1.8, 3.3));
        let rect = screen_bounds(&view, &corners).expect("the box is in front of the camera");
        assert!(rect.height() > rect.width(), "a player is taller than wide");

        let xs: Vec<f32> = corners
            .iter()
            .filter_map(|corner| view.project(*corner))
            .map(|point| point.x)
            .collect();
        let left = xs.iter().cloned().fold(f32::INFINITY, f32::min);
        let right = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!((rect.left() - left).abs() < 1e-3);
        assert!((rect.right() - right).abs() < 1e-3);
    }

     
    #[test]
    fn a_target_behind_the_camera_has_no_box() {
        let view = build_view(v3(0.0, 0.0, 0.0), 0.0, 0.0, 70.0, 800.0, 600.0);
        let corners = box_corners(v3(-0.3, 0.0, -5.0), v3(0.3, 1.8, -4.4));
        assert!(screen_bounds(&view, &corners).is_none());
    }

    fn box_cfg(color: Color32, fill: bool) -> BoxCfg {
        BoxCfg {
            enabled: true,
            color,
            players: true,
            mobs: false,
            range: 64.0,
            thickness: 1.5,
            fill,
            fill_opacity: 0.2,
        }
    }

    fn edge_count(shapes: &[egui::Shape]) -> usize {
        shapes
            .iter()
            .filter(|shape| matches!(shape, egui::Shape::LineSegment { .. }))
            .count()
    }

     
     
    #[test]
    fn box_esp_projects_all_twelve_edges_of_a_hitbox() {
        let view = build_view(v3(0.0, 0.0, 0.0), 0.0, 0.0, 70.0, 800.0, 600.0);
        let shapes = probe_shapes(|painter, _ctx| {
            draw_box_esp(
                painter,
                &view,
                &[target(TargetKind::Player)],
                1.0,
                &box_cfg(Color32::from_rgb(255, 0, 0), false),
            );
        });
        assert_eq!(edge_count(&shapes), 12, "a 3D box has twelve edges");
    }

     
     
    #[test]
    fn box_esp_skips_unselected_targets_and_out_of_range_entities() {
        let view = build_view(v3(0.0, 0.0, 0.0), 0.0, 0.0, 70.0, 800.0, 600.0);
        let mut mob = target(TargetKind::Mob);
        mob.id = 2;
        let mut far = target(TargetKind::Player);
        far.id = 3;
        far.distance = 100.0;
        let cfg = box_cfg(Color32::from_rgb(255, 0, 0), false);
        let shapes = probe_shapes(|painter, _ctx| {
            draw_box_esp(
                painter,
                &view,
                &[target(TargetKind::Player), mob, far],
                1.0,
                &cfg,
            );
        });
        assert_eq!(edge_count(&shapes), 12, "only the in-range player is boxed");
    }

     
    #[test]
    fn box_esp_draws_nothing_behind_the_camera() {
        let view = build_view(v3(0.0, 0.0, 0.0), 0.0, 0.0, 70.0, 800.0, 600.0);
        let mut behind = target(TargetKind::Player);
        behind.prev = v3(0.0, 0.0, -6.0);
        behind.pos = v3(0.0, 0.0, -6.0);
        let shapes = probe_shapes(|painter, _ctx| {
            draw_box_esp(
                painter,
                &view,
                &[behind],
                1.0,
                &box_cfg(Color32::from_rgb(255, 0, 0), true),
            );
        });
        assert!(
            shapes.is_empty(),
            "nothing may be painted behind the camera"
        );
    }

     
     
    #[test]
    fn a_filled_box_paints_only_camera_facing_faces() {
        let min = v3(1.0, 1.0, 1.0);
        let max = v3(2.0, 2.0, 2.0);
        let facing = box_faces_facing(min, max, v3(0.0, 0.0, 0.0));
        assert_eq!(facing.iter().filter(|seen| **seen).count(), 3);
        assert!(
            facing[0] && facing[2] && facing[4],
            "the near faces point at the camera"
        );

         
        let behind = box_faces_facing(min, max, v3(4.0, 4.0, 4.0));
        assert!(behind[1] && behind[3] && behind[5]);

         
        let inside = box_faces_facing(min, max, v3(1.5, 1.5, 1.5));
        assert!(inside.iter().all(|seen| !seen));
    }

     
     
    #[test]
    fn a_3d_box_matches_the_hitbox_it_wraps() {
        let mut entity = target(TargetKind::Player);
        entity.prev = v3(0.0, 0.0, 2.0);
        entity.pos = v3(1.0, 0.0, 4.0);
        let (min, max) = box_esp_bounds(&entity, 0.5);
        assert!((min.x - 0.2).abs() < 1e-9 && (max.x - 0.8).abs() < 1e-9);
        assert!((min.z - 2.7).abs() < 1e-9 && (max.z - 3.3).abs() < 1e-9);
        assert!((max.y - min.y - 1.8).abs() < 1e-9);
        assert!(min.y.abs() < 1e-9);
    }

     
     
    #[test]
    fn brackets_stay_readable_on_any_box() {
        let small = Rect::from_min_max(pos2(0.0, 0.0), pos2(4.0, 6.0));
        let huge = Rect::from_min_max(pos2(0.0, 0.0), pos2(900.0, 1200.0));
        let mid = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 50.0));
        assert_eq!(bracket_length(small), BRACKET_MIN);
        assert_eq!(bracket_length(huge), BRACKET_MAX);
        assert!((bracket_length(mid) - 50.0 * BRACKET_FRACTION).abs() < 1e-6);
         
        let tall = Rect::from_min_max(pos2(0.0, 0.0), pos2(20.0, 500.0));
        assert!((bracket_length(tall) - 20.0 * BRACKET_FRACTION).abs() < 1e-6);
    }

     
     
    #[test]
    fn a_fainter_part_of_a_box_keeps_its_colour() {
         
         
         
        let hue_kept = |before: Color32, after: Color32| {
            let ratio = |channel: u8, alpha: u8| channel as f32 / alpha.max(1) as f32;
            [
                (before.r(), after.r()),
                (before.g(), after.g()),
                (before.b(), after.b()),
            ]
            .iter()
            .all(|&(a, b)| (ratio(a, before.a()) - ratio(b, after.a())).abs() < 0.02)
        };

        let opaque = Color32::from_rgb(200, 40, 80);
        let faded = fade(opaque, 0.55);
        assert_eq!(faded.a(), (255.0_f32 * 0.55).round() as u8);
        assert!(hue_kept(opaque, faded), "{opaque:?} -> {faded:?}");

        let translucent = Color32::from_rgba_unmultiplied(200, 40, 80, 200);
        let ghosted = fade(translucent, 0.55);
        assert!(
            hue_kept(translucent, ghosted),
            "{translucent:?} -> {ghosted:?}"
        );

        assert_eq!(fade(opaque, 1.0), opaque);
        assert_eq!(fade(opaque, 0.0).a(), 0);
        assert_eq!(fade(opaque, 5.0), opaque, "factors above one clamp");
    }

     
     
    #[test]
    fn the_target_box_is_brackets_over_a_faint_wash() {
        let rect = Rect::from_min_max(pos2(100.0, 100.0), pos2(140.0, 220.0));
        let shapes = probe_shapes(|painter, _ctx| {
            draw_target_box(
                painter,
                rect,
                Color32::from_rgb(255, 0, 0),
                TargetLook::still(BoxStyle::Brackets),
            );
        });

        let (mut washes, mut borders, mut brackets) = (0, 0, 0);
        for shape in shapes {
            match shape {
                egui::Shape::Rect(rect) if rect.fill != Color32::TRANSPARENT => {
                    washes += 1;
                    assert!(rect.fill.a() <= 26, "the wash must not hide the model");
                }
                egui::Shape::Rect(rect) => {
                    borders += 1;
                    assert!((rect.stroke.width - TARGET_STROKE).abs() < 1e-6);
                }
                egui::Shape::LineSegment { stroke, .. } => {
                    brackets += 1;
                    assert!((stroke.width - BRACKET_STROKE).abs() < 1e-6);
                }
                other => panic!("unexpected target shape: {other:?}"),
            }
        }
        assert_eq!((washes, borders, brackets), (1, 1, 8));
    }

     
     
    #[test]
    fn the_style_choice_picks_the_box_look() {
        assert_eq!(BoxStyle::OPTIONS.len(), 2);
        assert_eq!(BoxStyle::from_choice(0), BoxStyle::Brackets);
        assert_eq!(BoxStyle::from_choice(1), BoxStyle::Rounded);
        assert_eq!(BoxStyle::from_choice(7), BoxStyle::Brackets);
    }

     
     
     
    #[test]
    fn the_rounded_frame_cuts_its_corners() {
        let rect = Rect::from_min_max(pos2(100.0, 100.0), pos2(220.0, 400.0));
        let radius = rounded_radius(rect);
        assert_eq!(radius, ROUNDED_RADIUS_MAX, "a tall box caps its radius");
        let mid = Rect::from_min_max(pos2(0.0, 0.0), pos2(60.0, 200.0));
        assert!((rounded_radius(mid) - 60.0 * ROUNDED_RADIUS_FRACTION).abs() < 1e-6);
        let tiny = Rect::from_min_max(pos2(0.0, 0.0), pos2(8.0, 8.0));
        let huge = Rect::from_min_max(pos2(0.0, 0.0), pos2(900.0, 900.0));
        assert_eq!(rounded_radius(tiny), ROUNDED_RADIUS_MIN);
        assert_eq!(rounded_radius(huge), ROUNDED_RADIUS_MAX);

        let points = rounded_perimeter(rect, radius);
        let corners = [
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
         
         
        let clearance = radius * (2f32.sqrt() - 1.0) - 0.01;
        let mut perimeter = 0.0;
        for index in 0..points.len() {
            let point = points[index];
            assert!(rect.contains(point), "{point:?} left the box");
            let nearest = corners
                .iter()
                .map(|corner| corner.distance(point))
                .fold(f32::INFINITY, f32::min);
            assert!(
                nearest >= clearance,
                "{point:?} is {nearest} from a corner: the corner is not cut by {radius}"
            );
            let step = point.distance(points[(index + 1) % points.len()]);
            assert!(step > 0.0, "two perimeter points coincide at {index}");
            perimeter += step;
        }
         
        let sharp = 2.0 * (rect.width() + rect.height());
        assert!(
            perimeter > sharp * 0.85 && perimeter < sharp,
            "perimeter {perimeter} vs sharp {sharp}"
        );
    }

     
     
    #[test]
    fn the_rounded_style_animates_a_comet_around_the_frame() {
        let rect = Rect::from_min_max(pos2(100.0, 100.0), pos2(160.0, 300.0));
        let color = Color32::from_rgb(255, 60, 60);
        let rect_count = |shapes: &[egui::Shape]| {
            shapes
                .iter()
                .filter(|shape| matches!(shape, egui::Shape::Rect(_)))
                .count()
        };
        let segments = |shapes: &[egui::Shape]| {
            shapes
                .iter()
                .filter(|shape| matches!(shape, egui::Shape::LineSegment { .. }))
                .count()
        };

        let still = probe_shapes(|painter, _ctx| {
            draw_target_box(painter, rect, color, TargetLook::still(BoxStyle::Rounded));
        });
        assert_eq!(rect_count(&still), 2, "the wash and the frame");
        assert_eq!(
            segments(&still),
            0,
            "nothing animates while the box is still"
        );

        let moving = |time: f32| {
            probe_shapes(move |painter, _ctx| {
                draw_target_box(
                    painter,
                    rect,
                    color,
                    TargetLook {
                        style: BoxStyle::Rounded,
                        animate: true,
                        glow: true,
                        time,
                        phase: 0.0,
                    },
                );
            })
        };
        let first = moving(0.0);
        assert_eq!(rect_count(&first), 4, "wash, two halo passes and the frame");
        let perimeter = rounded_perimeter(rect, rounded_radius(rect)).len() as f32;
        let expected = ((perimeter * COMET_TAIL).round() as usize).clamp(4, perimeter as usize);
        assert_eq!(
            segments(&first),
            expected,
            "the comet covers its whole tail"
        );

         
        let head = |shapes: &[egui::Shape]| {
            shapes.iter().find_map(|shape| match shape {
                egui::Shape::LineSegment { points, .. } => Some(points[0].lerp(points[1], 0.5)),
                _ => None,
            })
        };
        let start = head(&first).expect("the comet is drawn");
        let later = head(&moving(1.0)).expect("the comet is drawn");
        assert!(
            start.distance(later) > 20.0,
            "the comet does not travel: {start:?} -> {later:?}"
        );
         
        let static_box = probe_shapes(|painter, _ctx| {
            draw_target_box(
                painter,
                rect,
                color,
                TargetLook {
                    style: BoxStyle::Rounded,
                    animate: false,
                    glow: true,
                    time: 4.0,
                    phase: 1.0,
                },
            );
        });
        assert_eq!(segments(&static_box), 0);
    }

     
     
    #[test]
    fn the_box_pulse_breathes_within_bounds() {
        let (mut min, mut max) = (f32::INFINITY, f32::NEG_INFINITY);
        for step in 0..200 {
            let value = breathe(step as f32 * 0.07, 0.4);
            assert!(
                (0.82..=1.0).contains(&value),
                "pulse left its range: {value}"
            );
            min = min.min(value);
            max = max.max(value);
        }
        assert!(max - min > 0.1, "the box does not breathe: {min}..{max}");
    }

     
     
    #[test]
    fn each_target_breathes_on_its_own_beat() {
        let phases: Vec<f32> = (0..8).map(target_phase).collect();
        assert_eq!(phases[3], target_phase(3));
        for phase in &phases {
            assert!((0.0..std::f32::consts::TAU).contains(phase), "{phase}");
        }
        let spread = phases
            .windows(2)
            .map(|pair| (pair[0] - pair[1]).abs())
            .fold(0.0_f32, f32::max);
        assert!(spread > 0.5, "the phases bunch together: {phases:?}");
    }

     
     
    #[test]
    fn the_comet_head_is_a_lightened_target_colour() {
        let color = Color32::from_rgb(200, 40, 80);
        let hot = lighten(color, 0.55);
        assert_eq!(hot.a(), 255, "opacity is untouched");
        assert!(
            hot.r() > color.r() && hot.g() > color.g() && hot.b() > color.b(),
            "{color:?} -> {hot:?}"
        );
        assert_eq!(lighten(color, 0.0), color);
        assert_eq!(lighten(color, 1.0), Color32::WHITE);

         
         
        let ghost = lighten(Color32::from_rgba_unmultiplied(200, 40, 80, 160), 0.55);
        assert_eq!(ghost.a(), 160);
        assert!(ghost.r() <= ghost.a() && ghost.g() <= ghost.a() && ghost.b() <= ghost.a());
        assert_eq!(lighten(Color32::TRANSPARENT, 1.0), Color32::TRANSPARENT);
    }

     
    #[test]
    fn the_health_bar_hugs_the_box_and_drains_downward() {
        let rect = Rect::from_min_max(pos2(100.0, 100.0), pos2(140.0, 220.0));
        let bar = |fraction: f32| {
            let shapes = probe_shapes(|painter, _ctx| {
                draw_target_health(painter, rect, fraction);
            });
            let mut rects = shapes.into_iter().filter_map(|shape| match shape {
                egui::Shape::Rect(rect) => Some(rect.rect),
                _ => None,
            });
            let track = rects.next().expect("track");
            (track, rects.next())
        };

        let (track, full) = bar(1.0);
        let full = full.expect("a full bar has a fill");
        assert!(
            track.right() <= rect.left(),
            "the bar stays clear of the box"
        );
        assert!(track.height() > 100.0, "it spans the whole box");
        assert!((full.height() - track.height()).abs() < 0.01);

        let (track, half) = bar(0.5);
        let half = half.expect("a half bar has a fill");
        assert!((half.height() - track.height() * 0.5).abs() < 0.5);
        assert!(
            (half.bottom() - track.bottom()).abs() < 0.01,
            "the bar drains from the top down"
        );
        assert!(bar(0.0).1.is_none(), "no fill at zero health");
    }

     
     
    #[test]
    fn the_label_is_one_pill_above_the_box() {
        let rect = Rect::from_min_max(pos2(100.0, 100.0), pos2(140.0, 220.0));
        let entity = target(TargetKind::Player);
        let cfg = entity_cfg();
        let shapes = probe_shapes(|painter, ctx| {
            draw_target_label(painter, ctx, rect, &entity, &cfg, 12.4);
        });

        let (mut texts, mut pill_bottom) = (0, f32::NEG_INFINITY);
        for shape in shapes {
            match shape {
                egui::Shape::Rect(rect) if rect.fill.a() > 0 => {
                    pill_bottom = pill_bottom.max(rect.rect.bottom());
                }
                egui::Shape::Text(text) => {
                    texts += 1;
                    assert!(text.pos.y < rect.top(), "the label sits above the box");
                }
                other => panic!("unexpected label shape: {other:?}"),
            }
        }
        assert_eq!(texts, 2, "the name and the distance");
        assert!(pill_bottom > 0.0 && pill_bottom <= rect.top());
    }

     
     
     
    #[test]
    #[ignore = "writes a preview into screenshots/"]
    fn target_preview() {
        let (w, h) = (720usize, 980usize);
        let ctx = Context::default();
        let cfg = entity_cfg();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(w as f32, h as f32))),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(LayerId::new(
                    Order::Background,
                    egui::Id::new("target_preview"),
                ));
                let mut player = target(TargetKind::Player);
                player.name = "Alex".to_owned();
                let mut mob = target(TargetKind::Mob);
                mob.name = "Zombie".to_owned();
                let mut zombie_cfg = entity_cfg();
                zombie_cfg.color = Color32::from_rgb(120, 220, 140);
                let mut rounded = entity_cfg();
                rounded.style = BoxStyle::Rounded;
                let mut rounded_mob = zombie_cfg;
                rounded_mob.style = BoxStyle::Rounded;

                 
                let grid = [
                    (190.0, 250.0, 160.0, 300.0, 1.0, &player, &cfg),
                    (530.0, 250.0, 160.0, 300.0, 0.55, &player, &rounded),
                    (190.0, 740.0, 160.0, 300.0, 0.82, &mob, &zombie_cfg),
                    (530.0, 740.0, 110.0, 150.0, 0.72, &mob, &rounded_mob),
                ];
                for (index, (cx, cy, width, height, health, entity, icfg)) in
                    grid.into_iter().enumerate()
                {
                    let rect = Rect::from_center_size(pos2(cx, cy), vec2(width, height));
                    draw_target_box(
                        &painter,
                        rect,
                        icfg.color,
                        TargetLook {
                            style: icfg.style,
                            animate: icfg.animate,
                            glow: icfg.glow,
                             
                             
                            time: 0.9 + index as f32 * 0.7,
                            phase: index as f32 * 1.3,
                        },
                    );
                    draw_target_health(&painter, rect, health);
                    draw_target_label(&painter, ctx, rect, entity, icfg, 7.5);
                }
            },
        );

        let mut textures = HashMap::new();
        crate::graphic::preview::gather_textures(&mut textures, &output.textures_delta);
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        let image = crate::graphic::preview::rasterize(
            primitives,
            &textures,
            w,
            h,
            [0.055, 0.06, 0.07, 1.0],
        );
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../screenshots");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("esp-targets.png");
        image.save(&path).unwrap();
         
         
         
        let crop = image::imageops::crop_imm(&image, 90, 80, 540, 190).to_image();
        let close = dir.join("esp-targets-close.png");
        crop.save(&close).unwrap();
         
        let rounded = image::imageops::crop_imm(&image, 430, 80, 200, 360).to_image();
        rounded.save(dir.join("esp-targets-rounded.png")).unwrap();
         
         
        let comet = image::imageops::crop_imm(&image, 455, 655, 160, 200).to_image();
        let comet = image::imageops::resize(&comet, 480, 600, image::imageops::FilterType::Nearest);
        comet.save(dir.join("esp-targets-comet.png")).unwrap();
         
         
        let label = image::imageops::crop_imm(&image, 100, 130, 200, 40).to_image();
        let zoom = image::imageops::resize(&label, 600, 120, image::imageops::FilterType::Nearest);
        let zoom_path = dir.join("esp-targets-label.png");
        zoom.save(&zoom_path).unwrap();
        eprintln!("wrote {} and {}", path.display(), close.display());
    }

    #[test]
    fn v3_length_and_dot() {
        let a = v3(3.0, 4.0, 0.0);
        assert_eq!(a.length(), 5.0);
        assert_eq!(a.dot(a), 25.0);
    }

    #[test]
    fn v3_cross_of_x_and_y_is_z() {
        let z = v3(1.0, 0.0, 0.0).cross(v3(0.0, 1.0, 0.0));
        assert!(z.sub(v3(0.0, 0.0, 1.0)).length() < 1e-9);
    }

    #[test]
    fn v3_lerp_finds_the_midpoint() {
        let m = v3(0.0, 0.0, 0.0).lerp(v3(10.0, 20.0, -4.0), 0.5);
        assert_eq!((m.x, m.y, m.z), (5.0, 10.0, -2.0));
    }

    #[test]
    fn tracer_starts_at_screen_center_at_every_resolution() {
        for (width, height) in [(1920.0, 1080.0), (854.0, 480.0), (3440.0, 1440.0)] {
            assert_eq!(
                tracer_origin(width, height),
                pos2(width / 2.0, height / 2.0)
            );
        }
    }

    #[test]
    fn a_point_dead_ahead_projects_to_the_screen_centre() {
         
        let view = build_view(v3(0.0, 0.0, 0.0), 0.0, 0.0, 70.0, 1920.0, 1080.0);
        let projected = view
            .project(v3(0.0, 0.0, 10.0))
            .expect("a point straight ahead must project");
        assert!((projected.x - 960.0).abs() < 1.0);
        assert!((projected.y - 540.0).abs() < 1.0);
    }

    #[test]
    fn a_point_behind_the_camera_does_not_project() {
        let view = build_view(v3(0.0, 0.0, 0.0), 0.0, 0.0, 70.0, 1920.0, 1080.0);
        assert!(view.project(v3(0.0, 0.0, -10.0)).is_none());
    }

    #[test]
    fn packed_word_index_never_exceeds_the_raw_array() {
         
         
        let word_count = 256usize;
        let values_per_word = 4096usize.div_ceil(word_count);
        for index in 0..4096usize {
            assert!(index / values_per_word < word_count);
        }

         
        let word_count = 342usize;
        let values_per_word = 4096usize.div_ceil(word_count);
        for index in 0..4096usize {
            assert!(index / values_per_word < word_count);
        }
    }

    #[test]
    fn box_corners_span_min_to_max() {
        let corners = box_corners(v3(0.0, 0.0, 0.0), v3(1.0, 2.0, 3.0));
        assert_eq!(corners.len(), 8);
        assert_eq!((corners[0].x, corners[0].y, corners[0].z), (0.0, 0.0, 0.0));
        assert_eq!((corners[6].x, corners[6].y, corners[6].z), (1.0, 2.0, 3.0));
    }

    #[test]
    fn section_y_intersection_includes_deepslate_underground() {
         
         
        let center_y = 80.0;
        let range = 64.0;
        let y_min = center_y - range;
        let y_max = center_y + range;

        let section_intersects = |base_y: i32| -> bool {
            let top = base_y as f64 + 16.0;
            let bottom = base_y as f64;
            !(top < y_min || bottom > y_max)
        };

         
        assert!(section_intersects(16));
         
        assert!(section_intersects(64));
         
        assert!(!section_intersects(-64));

         
        let range_large = 128.0;
        let y_min_large = center_y - range_large;
        let y_max_large = center_y + range_large;
        let section_intersects_large = |base_y: i32| -> bool {
            let top = base_y as f64 + 16.0;
            let bottom = base_y as f64;
            !(top < y_min_large || bottom > y_max_large)
        };
         
        assert!(section_intersects_large(-64));
    }

    #[test]
    fn div_euclid_handles_negative_chunk_coordinates() {
        assert_eq!(5i32.div_euclid(16), 0);
        assert_eq!(0i32.div_euclid(16), 0);
        assert_eq!((-1i32).div_euclid(16), -1);
        assert_eq!((-15i32).div_euclid(16), -1);
        assert_eq!((-16i32).div_euclid(16), -1);
        assert_eq!((-17i32).div_euclid(16), -2);
    }

    #[test]
    fn bit_storage_unpack_words_decodes_palette_indices() {
        use crate::mapping::chunk::BitStorage;
        let mut words = vec![0u64; 256];
         
         
        words[0] = 7u64 | (12u64 << 4);
        let jlong_words: Vec<jni::sys::jlong> =
            words.into_iter().map(|w| w as jni::sys::jlong).collect();

        let unpacked = BitStorage::unpack_words(&jlong_words, 4);
        assert_eq!(unpacked.len(), 4096);
        assert_eq!(unpacked[0], 7);
        assert_eq!(unpacked[1], 12);
        assert_eq!(unpacked[2], 0);
    }

    #[test]
    fn deduplication_keys_by_position() {
        let mut map = HashMap::new();
        let target1 = ChestTarget {
            pos: V3 {
                x: 10.0,
                y: 20.0,
                z: -5.0,
            },
            is_spawner: false,
            color: None,
        };
        let target2 = ChestTarget {
            pos: V3 {
                x: 10.0,
                y: 20.0,
                z: -5.0,
            },
            is_spawner: false,
            color: None,
        };

        let key1 = (
            target1.pos.x as i32,
            target1.pos.y as i32,
            target1.pos.z as i32,
        );
        let key2 = (
            target2.pos.x as i32,
            target2.pos.y as i32,
            target2.pos.z as i32,
        );
        map.insert(key1, target1);
        map.insert(key2, target2);

        assert_eq!(map.len(), 1);
    }

    #[test]
    fn union_targets_dedupes_and_masks_owners() {
         
        let selections: Vec<Vec<(String, u32)>> = vec![
            vec![
                ("minecraft:diamond_ore".to_string(), 1),
                ("minecraft:spawner".to_string(), 2),
            ],
            vec![
                ("minecraft:diamond_ore".to_string(), 3),
                ("minecraft:chest".to_string(), 4),
            ],
        ];
        let (ids, payloads, masks) = union_targets(&selections);
        assert_eq!(
            ids,
            vec![
                "minecraft:chest".to_string(),
                "minecraft:diamond_ore".to_string(),
                "minecraft:spawner".to_string(),
            ]
        );
         
        assert_eq!(payloads, vec![4, 1, 2]);
         
         
        assert_eq!(masks, vec![0b10, 0b11, 0b01]);
    }

    #[test]
    fn cylinder_range_reaches_deep_blocks_at_full_horizontal_distance() {
         
         
         
         
        assert!(within_cylinder(100.0, -100.0, 0.0, 113.0, 113.0 * 113.0));
        assert!(!within_cylinder(100.0, -100.0, 0.0, 99.9, 99.9 * 99.9));
         
        assert!(within_cylinder(128.0, 0.0, 0.0, 128.0, 128.0 * 128.0));
        assert!(!within_cylinder(128.1, 0.0, 0.0, 128.0, 128.0 * 128.0));
         
        assert!(within_cylinder(0.0, -128.0, 0.0, 128.0, 128.0 * 128.0));
        assert!(!within_cylinder(0.0, -128.1, 0.0, 128.0, 128.0 * 128.0));
    }

    #[test]
    fn portrait_prefers_the_resolved_skin_and_falls_back_to_the_username() {
        let url = "https://textures.minecraft.net/texture/abc";
        assert_eq!(
            PortraitSource::for_player(Some(url), "Alex"),
            Some(PortraitSource::Skin(url.to_owned()))
        );
         
         
        assert_eq!(
            PortraitSource::for_player(None, " Alex "),
            Some(PortraitSource::Avatar("Alex".to_owned()))
        );
        assert_eq!(
            PortraitSource::for_player(Some(""), "Alex"),
            Some(PortraitSource::Avatar("Alex".to_owned()))
        );
        assert_eq!(
            PortraitSource::for_player(Some("not a url"), "Alex"),
            Some(PortraitSource::Avatar("Alex".to_owned()))
        );
         
        assert_eq!(PortraitSource::for_player(None, "   "), None);
    }

     
     
     
     
    #[test]
    fn nametags_reserve_room_for_each_badge() {
        assert_eq!(nametag_card_geometry(1.0, false, false), (170.0, 0.0));
        let (width, nudge) = nametag_card_geometry(1.0, true, false);
        assert!((width - (170.0 + FRIEND_BADGE_W)).abs() < f32::EPSILON);
        assert!((nudge - FRIEND_BADGE_W * 0.5).abs() < f32::EPSILON);

        let (width, nudge) = nametag_card_geometry(1.0, false, true);
        assert!((width - (170.0 + INFERED_BADGE_W)).abs() < f32::EPSILON);
        assert!((nudge - INFERED_BADGE_W * 0.5).abs() < f32::EPSILON);

         
        let (width, nudge) = nametag_card_geometry(1.0, true, true);
        assert!((width - (170.0 + FRIEND_BADGE_W + INFERED_BADGE_W)).abs() < f32::EPSILON);
        assert!((nudge - (FRIEND_BADGE_W + INFERED_BADGE_W) * 0.5).abs() < f32::EPSILON);

         
        let (wide, wide_nudge) = nametag_card_geometry(1.2, true, true);
        assert!((wide - (170.0 + FRIEND_BADGE_W + INFERED_BADGE_W) * 1.2).abs() < 0.001);
        assert!((wide_nudge - (FRIEND_BADGE_W + INFERED_BADGE_W) * 0.6).abs() < 0.001);
    }

     
     
    #[test]
    fn nametag_cards_keep_an_almost_constant_size_across_the_range() {
        let near = hud_card_scale(0.5, 64.0, 1.06);
        let mid = hud_card_scale(32.0, 64.0, 1.06);
        let far = hud_card_scale(64.0, 64.0, 1.06);
        assert!((near - mid).abs() < 0.1, "near {near} vs mid {mid}");
        assert!((mid - far).abs() < 0.1, "mid {mid} vs far {far}");
        assert!(near <= HUD_CARD_MAX_SCALE && far >= HUD_CARD_MIN_SCALE);

         
         
        assert_eq!(hud_card_scale(0.0, 64.0, 1.6), HUD_CARD_MAX_SCALE);
         
        assert!(hud_card_scale(64.0, 64.0, 0.6) <= 0.61);
         
        let fallback = hud_card_scale(f32::NAN, f32::NAN, 1.0);
        assert!(fallback.is_finite());
        assert!((HUD_CARD_MIN_SCALE..=1.06).contains(&fallback));
    }

    fn nametag_cfg(armor: bool, hands: bool) -> NametagCfg {
        NametagCfg {
            enabled: true,
            color: Color32::WHITE,
            show_health: true,
            show_distance: true,
            range: 64.0,
            gear: true,
            armor,
            hands,
            durability: true,
        }
    }

    fn stack(name: &str) -> GearStack {
        GearStack {
            name: name.to_owned(),
            damage: 0,
            max_damage: 0,
            count: 1,
        }
    }

    #[test]
    fn gear_row_orders_armor_before_hands_and_skips_empty_slots() {
        let gear = Gear {
            armor: [
                Some(stack("netherite_helmet")),
                None,
                Some(stack("diamond_leggings")),
                None,
            ],
            main_hand: Some(stack("diamond_sword")),
            off_hand: Some(stack("totem_of_undying")),
        };
        let names = |cfg: &NametagCfg| {
            gear_cells(&gear, cfg)
                .iter()
                .map(|cell| cell.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(&nametag_cfg(true, true)),
            [
                "netherite_helmet",
                "diamond_leggings",
                "diamond_sword",
                "totem_of_undying"
            ]
        );
         
        assert_eq!(
            names(&nametag_cfg(true, false)),
            ["netherite_helmet", "diamond_leggings"]
        );
        assert_eq!(
            names(&nametag_cfg(false, true)),
            ["diamond_sword", "totem_of_undying"]
        );
        assert!(names(&nametag_cfg(false, false)).is_empty());
    }

    #[test]
    fn durability_reads_green_then_amber_then_red() {
        assert_eq!(durability_color(1.0), Color32::from_rgb(95, 205, 105));
        assert_eq!(durability_color(0.51), Color32::from_rgb(95, 205, 105));
        assert_eq!(durability_color(0.5), Color32::from_rgb(232, 197, 71));
        assert_eq!(durability_color(0.21), Color32::from_rgb(232, 197, 71));
        assert_eq!(durability_color(0.2), Color32::from_rgb(226, 74, 74));
        assert_eq!(durability_color(0.0), Color32::from_rgb(226, 74, 74));
         
        assert_eq!(durability_color(2.0), Color32::from_rgb(95, 205, 105));
        assert_eq!(durability_color(-1.0), Color32::from_rgb(226, 74, 74));
    }

    #[test]
    fn item_initials_stay_short_for_the_no_icon_tile() {
        assert_eq!(item_initials("netherite_helmet"), "nh");
        assert_eq!(item_initials("shield"), "sh");
        assert_eq!(item_initials("totem_of_undying"), "to");
        assert_eq!(item_initials(""), "?");
         
        assert_eq!(item_initials("__elytra__"), "el");
    }

     
     
     
    #[test]
    fn touching_blocks_always_draw_as_one_group() {
        let target = |x: f64| ChestTarget {
            pos: V3 { x, y: 0.0, z: 0.0 },
            is_spawner: false,
            color: None,
        };
         
        let view = build_view(v3(0.5, 0.5, -6.0), 0.0, 0.0, 70.0, 800.0, 600.0);
        let profile = BlockProfile::new(
            ModuleId::ChestEsp,
            Color32::from_rgb(255, 0, 0),
            false,
            128.0,
            Vec::new(),
            false,
        );
        let draws = vec![
            BlockDraw {
                target: target(0.0),
                color: profile.color,
            },
            BlockDraw {
                target: target(1.0),
                color: profile.color,
            },
        ];
        let ctx = Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(LayerId::new(
                Order::Background,
                egui::Id::new("block_group_probe"),
            ));
            draw_block_profile(&painter, &view, &profile, &draws, false);
        });
        let segments = output
            .shapes
            .iter()
            .filter(|clipped| matches!(clipped.shape, egui::Shape::LineSegment { .. }))
            .count();

        let merged: HashSet<Cell> = [(0, 0, 0), (1, 0, 0)].into_iter().collect();
        let lone: HashSet<Cell> = [(0, 0, 0)].into_iter().collect();
        assert!(segments > 0, "nothing was drawn");
        assert_eq!(
            segments,
            block_group::crease_edges(&merged).len(),
            "the pair must draw as one merged outline"
        );
        assert_ne!(
            segments,
            2 * block_group::crease_edges(&lone).len(),
            "the blocks must not draw as two separate boxes"
        );
    }

    fn spawner(x: f64, y: f64, z: f64) -> ChestTarget {
        ChestTarget {
            pos: V3 { x, y, z },
            is_spawner: true,
            color: None,
        }
    }

     
     
     
    #[test]
    fn only_new_spawners_inside_the_alert_range_are_announced() {
        let eye = [0.0, 64.0, 0.0];
        let found = [
            spawner(30.0, 64.0, 0.0),
            spawner(-10.0, 64.0, 0.0),
            spawner(200.0, 64.0, 0.0),
        ];
        let mut announced = HashSet::new();

        let fresh = spawners_to_announce(&found, &mut announced, eye, 64.0);
        assert_eq!(fresh.len(), 2, "the far spawner waits until it is close");
        assert_eq!(fresh[0].0, (-10, 64, 0), "the nearest is announced first");
        assert!((fresh[0].1 - 10.0).abs() < 1e-3);
        assert_eq!(fresh[1].0, (30, 64, 0));
        assert!(
            !announced.contains(&(200, 64, 0)),
            "an out-of-range spawner must not be marked as seen"
        );

         
        assert!(spawners_to_announce(&found, &mut announced, eye, 64.0).is_empty());

         
        let closer = spawners_to_announce(&found, &mut announced, [150.0, 64.0, 0.0], 64.0);
        assert_eq!(closer.len(), 1);
        assert_eq!(closer[0].0, (200, 64, 0));
    }

     
     
    #[test]
    fn a_spawner_is_keyed_by_the_block_it_sits_in() {
        let mut announced = HashSet::new();
        let found = [spawner(-0.5, 64.0, 12.9), spawner(-0.2, 64.0, 12.1)];
        let fresh = spawners_to_announce(&found, &mut announced, [0.0, 64.0, 13.0], 8.0);
        assert_eq!(fresh.len(), 1, "both samples are the same block");
        assert_eq!(fresh[0].0, (-1, 64, 12));
        assert!(announced.contains(&(-1, 64, 12)));
    }

     
     
    #[test]
    fn every_block_scanner_offers_shade_and_tracers() {
        use crate::module::render::{block_esp::BlockEspModule, chest_esp::ChestEspModule};
        use crate::module::Module;

        let scanners: Vec<Box<dyn Module>> = vec![
            Box::new(BlockEspModule::new()),
            Box::new(ChestEspModule::new()),
            Box::new(crate::module::render::esp_modules::spawner_finder()),
        ];
        for scanner in scanners {
            let data = scanner.get_module_data();
            for name in ["Tracers", "Shade", "Shade Opacity"] {
                assert!(
                    data.get_setting(name).is_some(),
                    "{:?} is missing {name}",
                    data.id
                );
            }
            assert!(
                data.get_setting("Connected").is_none(),
                "{:?} still offers grouping as a setting",
                data.id
            );
        }
    }
}
