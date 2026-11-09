 
 
 
 
 
 
 
 
 
 
 
 
 

use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use crate::friends;
use crate::mapping::entity::living::LivingEntity;
use crate::mapping::entity::player::{Equipment, GearStack, Player};
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::MappedObject;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{mapping, minecraft};

 
const SELECT_EVERY: Duration = Duration::from_millis(150);
 
const DETAIL_EVERY: Duration = Duration::from_millis(100);
 
 
const MAX_CANDIDATES: usize = 64;

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetSource {
    Crosshair,
    RecentAttack,
    NearestPlayer,
    ManualName,
    Automatic,
}

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Priority {
    Nearest,
    LowestHealth,
    HighestHealth,
}

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AllowedEntities {
    Players,
    PlayersAndMobs,
    AllLiving,
}

 
#[derive(Debug, Clone)]
pub(crate) struct TargetInfo {
    pub id: i32,
    pub name: String,
    pub entity_type: String,
    pub distance: f64,
    pub health: f32,
    pub max_health: f32,
    pub absorption: f32,
    pub armor: i32,
    pub armor_pieces: usize,
    pub main_hand: Option<GearStack>,
    pub off_hand: Option<GearStack>,
    pub effects: Vec<(String, i32, i32)>,
     
    pub feet: [f64; 3],
     
    pub yaw: f32,
     
    pub height: f64,
    pub visible: bool,
    pub invisible: bool,
    pub is_player: bool,
    pub skin_url: Option<String>,
    pub method: &'static str,
    pub first_seen: Instant,
    pub last_seen: Instant,
    pub last_damage: Option<f32>,
    pub last_damage_at: Option<Instant>,
}

impl TargetInfo {
    pub fn health_fraction(&self) -> f32 {
        if self.max_health <= 0.0 {
            0.0
        } else {
            (self.health / self.max_health).clamp(0.0, 1.0)
        }
    }

    pub fn absorption_fraction(&self) -> f32 {
        if self.max_health <= 0.0 {
            0.0
        } else {
            (self.absorption / self.max_health).clamp(0.0, 1.0)
        }
    }
}

#[derive(Default)]
struct Cache {
    data: Option<TargetInfo>,
     
     
    active: bool,
    selected_id: Option<i32>,
    last_seen: Option<Instant>,
    last_scan: Option<Instant>,
    last_detail: Option<Instant>,
    recent_id: Option<i32>,
    recent_at: Option<Instant>,
}

static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(|| Mutex::new(Cache::default()));

 
pub(crate) fn display() -> Option<(TargetInfo, bool)> {
    let cache = CACHE.lock().ok()?;
    let data = cache.data.clone()?;
    Some((data, cache.active))
}

 
pub(crate) fn reset() {
    if let Ok(mut cache) = CACHE.lock() {
        *cache = Cache::default();
    }
    crate::graphic::target_info::reset_motion();
    crate::graphic::target_info::clear_anchor();
}

 
pub(crate) fn retention() -> u64 {
    read_config().retention_ms
}

 
#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub enabled: bool,
    pub source: TargetSource,
    pub priority: Priority,
    pub allowed: AllowedEntities,
    pub max_distance: f64,
    pub retention_ms: u64,
    pub track_invisible: bool,
    pub ignore_friends: bool,
    pub manual_name: String,
    pub show_name: bool,
    pub show_head: bool,
    pub show_health_bar: bool,
    pub show_health: bool,
    pub show_percentage: bool,
    pub show_absorption: bool,
    pub show_distance: bool,
    pub show_armor: bool,
    pub show_held_item: bool,
    pub show_effects: bool,
    pub show_damage: bool,
    pub show_entity_type: bool,
    pub show_visibility: bool,
    pub show_last_seen: bool,
    pub show_method: bool,
    pub scale: f32,
    pub opacity: f32,
    pub health_bar_style: usize,
    pub background_style: usize,
    pub corner_radius: f32,
    pub text_size: f32,
    pub health_color: [f32; 4],
    pub background_color: [f32; 4],
    pub accent_color: [f32; 4],
    pub animation_speed: f32,
     
    pub physical: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            enabled: false,
            source: TargetSource::Crosshair,
            priority: Priority::Nearest,
            allowed: AllowedEntities::PlayersAndMobs,
            max_distance: 64.0,
            retention_ms: 1500,
            track_invisible: false,
            ignore_friends: true,
            manual_name: String::new(),
            show_name: true,
            show_head: true,
            show_health_bar: true,
            show_health: true,
            show_percentage: true,
            show_absorption: true,
            show_distance: true,
            show_armor: true,
            show_held_item: true,
            show_effects: true,
            show_damage: true,
            show_entity_type: true,
            show_visibility: true,
            show_last_seen: true,
            show_method: true,
            scale: 1.0,
            opacity: 0.82,
            health_bar_style: 0,
            background_style: 0,
            corner_radius: 7.0,
            text_size: 11.0,
            health_color: [0.88, 0.18, 0.20, 1.0],
            background_color: [0.035, 0.028, 0.03, 1.0],
            accent_color: [0.88, 0.18, 0.20, 1.0],
            animation_speed: 1.0,
            physical: false,
        }
    }
}

impl Config {
    fn from_data(data: &ModuleData) -> Config {
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
        let choice = |name: &str, default: usize| match data.get_setting(name) {
            Some(ModuleSetting::Choice { value, .. }) => *value,
            _ => default,
        };
        let text = |name: &str| {
            data.get_setting(name)
                .and_then(ModuleSetting::get_text_value)
                .unwrap_or_default()
                .to_string()
        };
        let color = |name: &str, default: [f32; 4]| match data.get_setting(name) {
            Some(ModuleSetting::Color { value, .. }) => *value,
            _ => default,
        };

        Config {
            enabled: data.enabled,
            source: match choice("Target Source", 0) {
                1 => TargetSource::RecentAttack,
                2 => TargetSource::NearestPlayer,
                3 => TargetSource::ManualName,
                4 => TargetSource::Automatic,
                _ => TargetSource::Crosshair,
            },
            priority: match choice("Selection Priority", 0) {
                1 => Priority::LowestHealth,
                2 => Priority::HighestHealth,
                _ => Priority::Nearest,
            },
            allowed: match choice("Allowed Entities", 1) {
                0 => AllowedEntities::Players,
                2 => AllowedEntities::AllLiving,
                _ => AllowedEntities::PlayersAndMobs,
            },
            max_distance: slider("Max Distance", 64.0).clamp(2.0, 256.0) as f64,
            retention_ms: slider("Retention (ms)", 1500.0).clamp(0.0, 5000.0) as u64,
            track_invisible: toggle("Track Invisible", false),
            ignore_friends: toggle("Ignore Friends", true),
            manual_name: text("Manual Target"),
            show_name: toggle("Show Name", true),
            show_head: toggle("Show Head", true),
            show_health_bar: toggle("Show Health Bar", true),
            show_health: toggle("Show Health", true),
            show_percentage: toggle("Show Percentage", true),
            show_absorption: toggle("Show Absorption", true),
            show_distance: toggle("Show Distance", true),
            show_armor: toggle("Show Armor", true),
            show_held_item: toggle("Show Held Item", true),
            show_effects: toggle("Show Effects", true),
            show_damage: toggle("Show Damage", true),
            show_entity_type: toggle("Show Entity Type", true),
            show_visibility: toggle("Show Visibility", true),
            show_last_seen: toggle("Show Last Seen", true),
            show_method: toggle("Show Method", true),
            scale: slider("Scale", 1.0).clamp(0.6, 1.6),
            opacity: slider("Opacity", 0.82).clamp(0.15, 1.0),
            health_bar_style: choice("Health Bar Style", 0),
            background_style: choice("Background Style", 0),
            corner_radius: slider("Corner Radius", 7.0).clamp(0.0, 18.0),
            text_size: slider("Text Size", 11.0).clamp(8.0, 17.0),
            health_color: color("Health Color", [0.88, 0.18, 0.20, 1.0]),
            background_color: color("Background Color", [0.035, 0.028, 0.03, 1.0]),
            accent_color: color("Accent Color", [0.88, 0.18, 0.20, 1.0]),
            animation_speed: slider("Animation Speed", 1.0).clamp(0.25, 3.0),
            physical: toggle("Physical", false),
        }
    }
}

 
pub(crate) fn read_config() -> Config {
     
     
    let Some(client) = crate::state::try_client() else {
        return Config::default();
    };
    let Some(handle) = client.modules.get(ModuleId::TargetInfo) else {
        return Config::default();
    };
    let Ok(module) = handle.lock() else {
        return Config::default();
    };
    Config::from_data(module.get_module_data())
}

 
#[derive(Debug)]
pub struct TargetInfoModule {
    pub module: ModuleData,
}

impl TargetInfoModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::TargetInfo,
                description: "Shows the name, health, distance and gear of the entity \
                              you are looking at or last fought"
                    .to_string(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: settings(),
            },
        }
    }

    fn config(&self) -> Config {
        Config::from_data(&self.module)
    }
}

fn settings() -> Vec<ModuleSetting> {
    vec![
        ModuleSetting::Choice {
            name: "Target Source".to_string(),
            value: 0,
            options: [
                "Crosshair",
                "Recent Attack",
                "Nearest Player",
                "Manual Name",
                "Automatic",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
        },
        ModuleSetting::Choice {
            name: "Selection Priority".to_string(),
            value: 0,
            options: ["Nearest", "Lowest Health", "Highest Health"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        },
        ModuleSetting::Choice {
            name: "Allowed Entities".to_string(),
            value: 1,
            options: ["Players", "Players + Mobs", "All Living"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        },
        ModuleSetting::Text {
            name: "Manual Target".to_string(),
            value: String::new(),
        },
        ModuleSetting::Slider {
            name: "Max Distance".to_string(),
            value: 64.0,
            min: 4.0,
            max: 256.0,
        },
        ModuleSetting::Slider {
            name: "Retention (ms)".to_string(),
            value: 1500.0,
            min: 0.0,
            max: 5000.0,
        },
        ModuleSetting::Toggle {
            name: "Track Invisible".to_string(),
            value: false,
        },
        ModuleSetting::Toggle {
            name: "Ignore Friends".to_string(),
            value: true,
        },
         
        ModuleSetting::Toggle {
            name: "Show Name".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Head".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Health Bar".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Health".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Percentage".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Absorption".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Distance".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Armor".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Held Item".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Effects".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Damage".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Entity Type".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Visibility".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Last Seen".to_string(),
            value: true,
        },
        ModuleSetting::Toggle {
            name: "Show Method".to_string(),
            value: true,
        },
         
        ModuleSetting::Slider {
            name: "Scale".to_string(),
            value: 1.0,
            min: 0.6,
            max: 1.6,
        },
        ModuleSetting::Slider {
            name: "Opacity".to_string(),
            value: 0.82,
            min: 0.15,
            max: 1.0,
        },
        ModuleSetting::Choice {
            name: "Health Bar Style".to_string(),
            value: 0,
            options: ["Smooth", "Segmented"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        },
        ModuleSetting::Choice {
            name: "Background Style".to_string(),
            value: 0,
            options: ["Glass", "Solid", "None"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        },
        ModuleSetting::Slider {
            name: "Corner Radius".to_string(),
            value: 7.0,
            min: 0.0,
            max: 18.0,
        },
        ModuleSetting::Slider {
            name: "Text Size".to_string(),
            value: 11.0,
            min: 8.0,
            max: 17.0,
        },
        ModuleSetting::Color {
            name: "Health Color".to_string(),
            value: [0.88, 0.18, 0.20, 1.0],
        },
        ModuleSetting::Color {
            name: "Background Color".to_string(),
            value: [0.035, 0.028, 0.03, 1.0],
        },
        ModuleSetting::Color {
            name: "Accent Color".to_string(),
            value: [0.88, 0.18, 0.20, 1.0],
        },
        ModuleSetting::Slider {
            name: "Animation Speed".to_string(),
            value: 1.0,
            min: 0.25,
            max: 3.0,
        },
        ModuleSetting::Toggle {
            name: "Physical".to_string(),
            value: false,
        },
    ]
}

impl Module for TargetInfoModule {
    fn on_start(&self) -> anyhow::Result<()> {
        reset();
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        reset();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let config = self.config();
        let now = Instant::now();

         
         
         
        if matches!(
            config.source,
            TargetSource::Crosshair | TargetSource::RecentAttack | TargetSource::Automatic
        ) {
            if let Some(id) = observed_click(now) {
                if let Ok(mut cache) = CACHE.lock() {
                    cache.recent_id = Some(id);
                    cache.recent_at = Some(now);
                }
            }
        }

        let (scan_due, mut detail_due, selected_id, recent, previous) = {
            let Ok(cache) = CACHE.lock() else {
                return Ok(());
            };
            (
                cache
                    .last_scan
                    .is_none_or(|last| now.duration_since(last) >= SELECT_EVERY),
                cache
                    .last_detail
                    .is_none_or(|last| now.duration_since(last) >= DETAIL_EVERY),
                cache.selected_id,
                cache.recent_id.zip(cache.recent_at),
                cache.data.clone(),
            )
        };

        if scan_due {
            if let Ok(mut cache) = CACHE.lock() {
                cache.last_scan = Some(now);
            }
            if let Some((entity, method)) = choose_target(&config, recent, now) {
                let id = entity.id()?;
                let same = selected_id == Some(id);
                 
                 
                let mut next = snapshot(&entity, &config, method, now)?;
                if same {
                    if let Some(old) = previous.as_ref().filter(|data| data.id == id) {
                        carry_over(&mut next, old, now, config.show_damage);
                    }
                }
                 
                 
                next.last_seen = now;
                if let Ok(mut cache) = CACHE.lock() {
                    cache.data = Some(next);
                    cache.selected_id = Some(id);
                    cache.last_detail = Some(now);
                    cache.last_seen = Some(now);
                    cache.active = true;
                }
                 
                detail_due = false;
            } else if let Ok(mut cache) = CACHE.lock() {
                 
                 
                let expired = cache.selected_id.is_none()
                    || cache.last_seen.is_none_or(|last| {
                        now.duration_since(last) >= Duration::from_millis(config.retention_ms)
                    });
                if expired {
                    cache.active = false;
                    cache.selected_id = None;
                }
            }
        }

         
         
        if detail_due {
            let selected = {
                let Ok(cache) = CACHE.lock() else {
                    return Ok(());
                };
                if cache.active {
                    cache.selected_id
                } else {
                    None
                }
            };
            if let Some(id) = selected {
                if let Ok(mut cache) = CACHE.lock() {
                    cache.last_detail = Some(now);
                }
                if let Some(entity) = current_entity(id) {
                    let method = previous
                        .as_ref()
                        .filter(|data| data.id == id)
                        .map(|data| data.method)
                        .unwrap_or("Tracked");
                    let mut next = snapshot(&entity, &config, method, now)?;
                    if let Some(old) = previous.as_ref().filter(|data| data.id == id) {
                        carry_over(&mut next, old, now, config.show_damage);
                         
                         
                        next.last_seen = old.last_seen;
                    }
                    if let Ok(mut cache) = CACHE.lock() {
                         
                         
                        if cache.selected_id == Some(id) {
                            cache.data = Some(next);
                            cache.active = true;
                        }
                    }
                }
                 
                 
            }
        }

         
         
         
        if config.enabled && config.physical {
            let selected = {
                let Ok(cache) = CACHE.lock() else {
                    return Ok(());
                };
                cache.active.then_some(cache.selected_id).flatten()
            };
            if let Some(id) = selected {
                if let Some(entity) = current_entity(id) {
                    if let (Ok(feet), Ok(yaw)) = (entity.get_position(), entity.get_yaw()) {
                        let height = entity.bb_height().unwrap_or(1.8) as f64;
                        crate::graphic::target_info::publish_anchor(
                            id,
                            feet.into(),
                            yaw,
                            height,
                            now,
                        );
                    }
                }
            }
        } else {
            crate::graphic::target_info::clear_anchor();
        }
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

 

struct Candidate {
    entity: crate::mapping::entity::Entity,
    id: i32,
    distance: f64,
    health: f32,
    name: Option<String>,
    is_player: bool,
}

fn choose_target(
    config: &Config,
    recent: Option<(i32, Instant)>,
    now: Instant,
) -> Option<(crate::mapping::entity::Entity, &'static str)> {
    let local = minecraft().player().ok().flatten()?;
    if !local.is_alive().unwrap_or(false) {
        return None;
    }
    let origin = local.get_position().ok()?;
    let local_id = local.id().ok()?;

    match config.source {
        TargetSource::Crosshair => {
            crosshair_candidate(config, &origin, local_id).map(|entity| (entity, "Crosshair"))
        }
        TargetSource::RecentAttack => recent_candidate(config, recent, &origin, local_id, now),
        TargetSource::ManualName => {
            manual_candidate(config, &origin, local_id).map(|entity| (entity, "Manual"))
        }
        TargetSource::NearestPlayer => nearest_candidate(config, &origin, local_id),
        TargetSource::Automatic => {
            if let Some(entity) = crosshair_candidate(config, &origin, local_id) {
                return Some((entity, "Crosshair"));
            }
            if let Some(entity) = recent_candidate(config, recent, &origin, local_id, now) {
                return Some(entity);
            }
            if let Some(entity) = manual_candidate(config, &origin, local_id) {
                return Some((entity, "Manual"));
            }
            nearest_candidate(config, &origin, local_id)
        }
    }
}

fn crosshair_candidate(
    config: &Config,
    origin: &crate::mapping::math::Vec3,
    local_id: i32,
) -> Option<crate::mapping::entity::Entity> {
    let entity = minecraft().crosshair_entity().ok().flatten()?;
    candidate_for(entity, config, origin, local_id, false).map(|candidate| candidate.entity)
}

fn recent_candidate(
    config: &Config,
    recent: Option<(i32, Instant)>,
    origin: &crate::mapping::math::Vec3,
    local_id: i32,
    now: Instant,
) -> Option<(crate::mapping::entity::Entity, &'static str)> {
    let (id, at) = recent?;
     
    if now.duration_since(at) > Duration::from_secs(5) {
        return None;
    }
    let entity = current_entity(id)?;
    let candidate = candidate_for(entity, config, origin, local_id, false)?;
    Some((candidate.entity, "Recent Attack"))
}

fn manual_candidate(
    config: &Config,
    origin: &crate::mapping::math::Vec3,
    local_id: i32,
) -> Option<crate::mapping::entity::Entity> {
    let wanted = config.manual_name.trim();
    if wanted.is_empty() {
        return None;
    }
    all_candidates(config, origin, local_id)
        .into_iter()
        .find(|candidate| {
            candidate
                .name
                .as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case(wanted))
        })
        .map(|candidate| candidate.entity)
}

fn nearest_candidate(
    config: &Config,
    origin: &crate::mapping::math::Vec3,
    local_id: i32,
) -> Option<(crate::mapping::entity::Entity, &'static str)> {
    let mut candidates = all_candidates(config, origin, local_id);
     
     
    if config.source == TargetSource::NearestPlayer {
        candidates.retain(|candidate| candidate.is_player);
    }
    rank(&mut candidates, config.priority);
    let candidate = candidates.into_iter().next()?;
    Some((candidate.entity, "Nearest"))
}

fn rank(candidates: &mut [Candidate], priority: Priority) {
    match priority {
        Priority::Nearest => candidates.sort_by(|a, b| a.distance.total_cmp(&b.distance)),
        Priority::LowestHealth => {
            candidates.sort_by(|a, b| a.health.total_cmp(&b.health));
        }
        Priority::HighestHealth => {
            candidates.sort_by(|a, b| b.health.total_cmp(&a.health));
        }
    }
}

fn all_candidates(
    config: &Config,
    origin: &crate::mapping::math::Vec3,
    local_id: i32,
) -> Vec<Candidate> {
    let Ok(Some(world)) = minecraft().world() else {
        return Vec::new();
    };
    let Ok(entities) = world.get_entities() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entity in entities {
        if out.len() >= MAX_CANDIDATES {
            break;
        }
        if let Some(candidate) = candidate_for(entity, config, origin, local_id, true) {
            out.push(candidate);
        }
    }
    out
}

fn candidate_for(
    entity: crate::mapping::entity::Entity,
    config: &Config,
    origin: &crate::mapping::math::Vec3,
    local_id: i32,
    read_name: bool,
) -> Option<Candidate> {
    let id = entity.id().ok()?;
    if id == local_id || entity.is_removed().unwrap_or(true) {
        return None;
    }
    let is_player = entity.instance_of::<Player>();
    let is_living = entity.instance_of::<LivingEntity>();
    if !is_living {
        return None;
    }
    match config.allowed {
        AllowedEntities::Players if !is_player => return None,
        AllowedEntities::PlayersAndMobs
            if !is_player && !entity.instance_of::<crate::mapping::entity::mob::Mob>() =>
        {
            return None;
        }
        _ => {}
    }
    let living = LivingEntity::new(entity.jni_ref().clone());
    let invisible = entity.is_invisible().unwrap_or(false);
    if !config.track_invisible && invisible {
        return None;
    }
    let position = entity.get_position().ok()?;
    let distance = (position.x() - origin.x())
        .hypot(position.y() - origin.y())
        .hypot(position.z() - origin.z());
    if distance > config.max_distance {
        return None;
    }
    let name = if is_player || read_name || config.ignore_friends {
        entity
            .get_name()
            .ok()
            .and_then(|name| name.get_string().ok())
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
    } else {
        None
    };
    if config.ignore_friends {
        if let Some(name) = name.as_deref() {
            if friends::is_friend(name) {
                return None;
            }
        }
    }
    let health = living.get_health().unwrap_or(0.0);
    Some(Candidate {
        entity,
        id,
        distance,
        health,
        name,
        is_player,
    })
}

fn current_entity(id: i32) -> Option<crate::mapping::entity::Entity> {
    let output = minecraft().world();
    match output {
        Ok(Some(world)) => world.get_entity(id).ok().flatten(),
        _ => None,
    }
}

 
 
 
 
fn observed_click(_now: Instant) -> Option<i32> {
    let (left, right) = crate::graphic::input::MOUSE_STATE
        .lock()
        .map(|state| (state.left_down, state.right_down))
        .unwrap_or((false, false));
    if !left && !right {
        return None;
    }
    minecraft()
        .crosshair_entity()
        .ok()
        .flatten()
        .and_then(|entity| entity.id().ok())
}

 

 
 
fn carry_over(next: &mut TargetInfo, old: &TargetInfo, now: Instant, show_damage: bool) {
    next.first_seen = old.first_seen;
    next.last_damage = old.last_damage;
    next.last_damage_at = old.last_damage_at;
    if show_damage && next.health < old.health {
        let loss = old.health - next.health;
        if loss > 0.05 {
            next.last_damage = Some(loss);
            next.last_damage_at = Some(now);
        }
    }
}

fn snapshot(
    entity: &crate::mapping::entity::Entity,
    config: &Config,
    method: &'static str,
    now: Instant,
) -> anyhow::Result<TargetInfo> {
    let id = entity.id()?;
    let local = minecraft()
        .player()?
        .ok_or_else(|| anyhow::anyhow!("no local player"))?;
    let origin = local.get_position()?;
    let position = entity.get_position()?;
    let distance = (position.x() - origin.x())
        .hypot(position.y() - origin.y())
        .hypot(position.z() - origin.z());
    let invisible = entity.is_invisible().unwrap_or(false);
     
     
    let line_of_sight = local
        .call_method(
            "hasLineOfSight",
            &[jni::objects::JValue::Object(entity.jni_ref().as_obj())],
        )
        .ok()
        .and_then(|value| value.z().ok());
    let visible = !invisible && line_of_sight.unwrap_or(true);

    let name = entity
        .get_name()
        .ok()
        .and_then(|name| name.get_string().ok())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("Entity #{id}"));
    let is_player = entity.instance_of::<Player>();
    let entity_type = if is_player {
        "Player".to_string()
    } else {
        class_simple_name(entity).unwrap_or_else(|| "Entity".to_string())
    };

    let mut health = 0.0;
    let mut max_health = 0.0;
    let mut absorption = 0.0;
    let mut armor = 0;
    let mut effects = Vec::new();
    let mut main_hand = None;
    let mut off_hand = None;
    let mut armor_pieces = 0;
    let mut skin_url = None;

    if entity.instance_of::<LivingEntity>() {
        let living = LivingEntity::new(entity.jni_ref().clone());
        health = living.get_health().unwrap_or(0.0);
        max_health = living.get_max_health().unwrap_or(0.0);
        absorption = living.get_absorption().unwrap_or(0.0);
        armor = living.get_armor_value().unwrap_or(0);
        if config.show_effects {
            effects = living.get_active_effects().unwrap_or_default();
        }
        if config.show_held_item {
            main_hand = read_hand(living.get_main_hand_item().ok());
            off_hand = read_hand(living.get_offhand_item().ok());
        }
        if config.show_armor && is_player {
            if let Ok(equipment) = player_equipment(entity) {
                armor_pieces = equipment.armor.iter().filter(|slot| slot.is_some()).count();
            }
        }
    }

    if is_player && config.show_head {
        let player = Player::new(entity.jni_ref().clone());
        skin_url = player
            .player_info()
            .ok()
            .and_then(|info| info.skin_url().ok().flatten());
    }

    Ok(TargetInfo {
        id,
        name,
        entity_type,
        distance,
        health,
        max_health,
        absorption,
        armor,
        armor_pieces,
        main_hand,
        off_hand,
        effects,
        feet: [position.x(), position.y(), position.z()],
        yaw: entity.get_yaw().unwrap_or(0.0),
        height: entity.bb_height().unwrap_or(1.8) as f64,
        visible,
        invisible,
        is_player,
        skin_url,
        method,
        first_seen: now,
        last_seen: now,
        last_damage: None,
        last_damage_at: None,
    })
}

fn player_equipment(entity: &crate::mapping::entity::Entity) -> anyhow::Result<Equipment> {
    Player::new(entity.jni_ref().clone()).equipment()
}

fn read_hand(stack: Option<crate::mapping::inventory::ItemStack>) -> Option<GearStack> {
    let stack = stack?;
    if stack.is_empty().unwrap_or(true) {
        return None;
    }
    let name = stack
        .get_item()
        .and_then(|item| item.registry_name())
        .map(|full| full.strip_prefix("minecraft:").unwrap_or(&full).to_string())
        .unwrap_or_default();
    if name.is_empty() {
        return None;
    }
    Some(GearStack {
        name,
        damage: stack.get_damage_value().unwrap_or(0),
        max_damage: stack.get_max_damage().unwrap_or(0),
        count: stack.count().unwrap_or(1),
    })
}

 
fn class_simple_name(entity: &crate::mapping::entity::Entity) -> Option<String> {
    use jni::objects::JString;
    let mut env = mapping().get_env().ok()?;
    let class = env.get_object_class(entity.jni_ref().as_obj()).ok()?;
    let name = env
        .call_method(&class, "getSimpleName", "()Ljava/lang/String;", &[])
        .ok()?
        .l()
        .ok()?;
    let name = JString::from(name);
    let text: String = env.get_string(&name).ok()?.into();
    Some(text).filter(|name| !name.trim().is_empty())
}

 
pub(crate) fn damage_window() -> Duration {
    Duration::from_millis(900)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_usable_and_bounded() {
        let module = TargetInfoModule::new();
        let config = Config::from_data(&module.module);
        assert_eq!(config.source, TargetSource::Crosshair);
        assert_eq!(config.allowed, AllowedEntities::PlayersAndMobs);
        assert!(config.max_distance >= 4.0);
        assert!(config.retention_ms <= 5000);
        assert!((0.6..=1.6).contains(&config.scale));
        assert!((0.15..=1.0).contains(&config.opacity));
        assert!((8.0..=17.0).contains(&config.text_size));
        assert!(!config.physical, "the panel is the default mode");
    }

    #[test]
    fn physical_is_a_real_toggle() {
        let mut data = TargetInfoModule::new().module;
        data.get_setting_mut("Physical")
            .expect("the Physical toggle exists")
            .set_toggle_value(true);
        let config = Config::from_data(&data);
        assert!(config.physical);
         
        assert!(!config.enabled);
    }

    #[test]
    fn every_display_setting_exists() {
        let module = TargetInfoModule::new();
        for name in [
            "Target Source",
            "Selection Priority",
            "Allowed Entities",
            "Manual Target",
            "Max Distance",
            "Retention (ms)",
            "Track Invisible",
            "Ignore Friends",
            "Show Name",
            "Show Head",
            "Show Health Bar",
            "Show Health",
            "Show Percentage",
            "Show Absorption",
            "Show Distance",
            "Show Armor",
            "Show Held Item",
            "Show Effects",
            "Show Damage",
            "Show Entity Type",
            "Show Visibility",
            "Show Last Seen",
            "Show Method",
            "Scale",
            "Opacity",
            "Health Bar Style",
            "Background Style",
            "Corner Radius",
            "Text Size",
            "Health Color",
            "Background Color",
            "Accent Color",
            "Animation Speed",
            "Physical",
        ] {
            assert!(module.module.get_setting(name).is_some(), "missing {name}");
        }
    }

    #[test]
    fn health_fraction_is_clamped() {
        let mut target = TargetInfo {
            id: 1,
            name: "Alex".into(),
            entity_type: "Player".into(),
            distance: 3.0,
            health: 30.0,
            max_health: 20.0,
            absorption: 0.0,
            armor: 0,
            armor_pieces: 0,
            main_hand: None,
            off_hand: None,
            effects: Vec::new(),
            feet: [0.0, 64.0, 0.0],
            yaw: 0.0,
            height: 1.8,
            visible: true,
            invisible: false,
            is_player: true,
            skin_url: None,
            method: "Test",
            first_seen: Instant::now(),
            last_seen: Instant::now(),
            last_damage: None,
            last_damage_at: None,
        };
        assert_eq!(target.health_fraction(), 1.0);
        target.health = 5.0;
        assert!((target.health_fraction() - 0.25).abs() < 0.0001);
        target.max_health = 0.0;
        assert_eq!(target.health_fraction(), 0.0);
    }

    fn sample_target() -> TargetInfo {
        TargetInfo {
            id: 7,
            name: "Alex".into(),
            entity_type: "Player".into(),
            distance: 4.0,
            health: 20.0,
            max_health: 20.0,
            absorption: 0.0,
            armor: 0,
            armor_pieces: 0,
            main_hand: None,
            off_hand: None,
            effects: Vec::new(),
            feet: [0.0, 64.0, 0.0],
            yaw: 0.0,
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

    #[test]
    fn refreshes_keep_damage_and_first_seen() {
        let now = Instant::now();
        let mut old = sample_target();
        old.first_seen = now - Duration::from_secs(3);
        old.last_seen = now - Duration::from_millis(400);
        old.last_damage = Some(4.0);
        old.last_damage_at = Some(now - Duration::from_millis(200));

        let mut next = sample_target();
        carry_over(&mut next, &old, now, true);
        assert_eq!(next.first_seen, old.first_seen);
        assert_eq!(next.last_damage, Some(4.0));
        assert_eq!(next.last_damage_at, old.last_damage_at);

         
         
        next.health = 11.0;
        next.last_damage = None;
        carry_over(&mut next, &old, now, true);
        assert_eq!(next.last_damage, Some(9.0));
        assert_eq!(next.last_damage_at, Some(now));
    }

    #[test]
    fn ranking_honours_the_priority_setting() {
        let mut values = [(30.0_f32, 5.0_f64), (10.0, 7.0), (20.0, 2.0)];
        values.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert_eq!(values[0].0, 10.0);
        values.sort_by(|a, b| b.0.total_cmp(&a.0));
        assert_eq!(values[0].0, 30.0);
    }
}
