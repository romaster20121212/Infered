 
 
 
 
 
 
 
 
 
 

use std::sync::atomic::Ordering;

use jni::objects::GlobalRef;
use rhai::Dynamic;

use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::{Entity, EntityRef, LivingEntityRef, PlayerRef};
use crate::mapping::math::BlockPos;
use crate::mapping::MappedObject;
use crate::state::minecraft;

 
 
 
 
 
pub fn register(engine: &mut rhai::Engine, settings: std::sync::Arc<ScriptSettings>) {
    register_logging(engine);
    register_player(engine);
    register_world(engine);
    register_combat(engine);
    register_settings(engine, settings);
}

fn register_logging(engine: &mut rhai::Engine) {
    engine
        .register_fn("print", |message: &str| log::info!("[addon] {message}"))
        .register_fn("debug", |message: &str| log::debug!("[addon] {message}"))
        .register_fn("warn", |message: &str| log::warn!("[addon] {message}"))
        .register_fn("error", |message: &str| log::error!("[addon] {message}"));
}

fn register_player(engine: &mut rhai::Engine) {
     
    engine.register_fn("in_world", || {
        minecraft()
            .player()
            .ok()
            .flatten()
            .map(|player| player.is_alive().unwrap_or(false))
            .unwrap_or(false)
    });
    engine.register_fn("player_exists", || {
        minecraft().player().ok().flatten().is_some()
    });
    engine.register_fn("screen_open", || minecraft().current_screen().is_open());

    engine.register_fn("px", || {
        player_f64(|player| player.get_position().map(|v| v.x()))
    });
    engine.register_fn("py", || {
        player_f64(|player| player.get_position().map(|v| v.y()))
    });
    engine.register_fn("pz", || {
        player_f64(|player| player.get_position().map(|v| v.z()))
    });
    engine.register_fn("eye_x", || {
        player_f64(|player| player.get_eye_position().map(|v| v.x()))
    });
    engine.register_fn("eye_y", || {
        player_f64(|player| player.get_eye_position().map(|v| v.y()))
    });
    engine.register_fn("eye_z", || {
        player_f64(|player| player.get_eye_position().map(|v| v.z()))
    });
    engine.register_fn("yaw", || {
        player_f64(|player| player.get_yaw().map(|v| v as f64))
    });
    engine.register_fn("pitch", || {
        player_f64(|player| player.get_pitch().map(|v| v as f64))
    });
    engine.register_fn("velocity_x", || {
        player_f64(|player| player.get_delta_movement().map(|v| v.x()))
    });
    engine.register_fn("velocity_y", || {
        player_f64(|player| player.get_delta_movement().map(|v| v.y()))
    });
    engine.register_fn("velocity_z", || {
        player_f64(|player| player.get_delta_movement().map(|v| v.z()))
    });

    engine.register_fn("on_ground", || player_bool(|player| player.on_ground()));
    engine.register_fn("is_sprinting", || {
        player_bool(|player| player.is_sprinting())
    });
    engine.register_fn("is_alive", || player_bool(|player| player.is_alive()));
    engine.register_fn("in_water", || player_bool(|player| player.is_in_water()));
    engine.register_fn("in_lava", || player_bool(|player| player.is_in_lava()));
    engine.register_fn("on_climbable", || {
        player_bool(|player| player.on_climbable())
    });
    engine.register_fn("sneaking", || {
        player_bool(|player| player.is_shift_key_down())
    });
    engine.register_fn("moving", || player_bool(|player| player.has_move_input()));
    engine.register_fn("jump_held", || player_bool(|player| player.is_jumping()));
    engine.register_fn("using_item", || {
        player_bool(|player| player.is_using_item())
    });
    engine.register_fn("blocking", || player_bool(|player| player.is_blocking()));
    engine.register_fn("health", || {
        player_f64(|player| player.get_health().map(|v| v as f64))
    });
    engine.register_fn("max_health", || {
        player_f64(|player| player.get_max_health().map(|v| v as f64))
    });
    engine.register_fn("fall_distance", || {
        player_f64(|player| player.get_fall_distance())
    });
    engine.register_fn("tick_count", || {
        minecraft()
            .player()
            .ok()
            .flatten()
            .and_then(|player| player.get_tick_count().ok())
            .map(|value| value as f64)
            .unwrap_or(0.0)
    });
    engine.register_fn("attack_cooldown", || {
        player_f64(|player| player.attack_strength_scale().map(|v| v as f64))
    });
    engine.register_fn("speed", || {
        player_f64(|player| player.get_speed().map(|v| v as f64))
    });
    engine.register_fn("move_forward", || {
        minecraft()
            .player()
            .ok()
            .flatten()
            .and_then(|player| player.move_input().ok().map(|(f, _)| f as f64))
            .unwrap_or(0.0)
    });
    engine.register_fn("move_strafe", || {
        minecraft()
            .player()
            .ok()
            .flatten()
            .and_then(|player| player.move_input().ok().map(|(_, s)| s as f64))
            .unwrap_or(0.0)
    });
    engine.register_fn("has_move_input", || {
        player_bool(|player| player.has_move_input())
    });

     
    engine.register_fn("set_sprinting", |value: bool| {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.set_sprinting(value);
        }
    });
    engine.register_fn("set_rotation", |yaw: f64, pitch: f64| {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.set_rotation(yaw as f32, pitch as f32);
        }
    });
    engine.register_fn("set_velocity", |x: f64, y: f64, z: f64| {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.set_delta_movement(x, y, z);
        }
    });
    engine.register_fn("set_position", |x: f64, y: f64, z: f64| {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.set_pos(x, y, z);
        }
    });
    engine.register_fn("jump", || {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.jump_from_ground();
        }
    });
    engine.register_fn("reset_fall_distance", || {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.reset_fall_distance();
        }
    });
    engine.register_fn("set_no_physics", |value: bool| {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.set_no_physics(value);
        }
    });
    engine.register_fn("set_invulnerable", |value: bool| {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.set_invulnerable(value);
        }
    });
    engine.register_fn("set_flying", |value: bool| {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.abilities.fly(value);
        }
    });
    engine.register_fn("set_flying_speed", |speed: f64| {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.abilities.set_flying_speed(speed as f32);
        }
    });
    engine.register_fn("add_attribute", |attribute: &str, id: &str, amount: f64| {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.add_attribute_modifier(attribute, id, amount);
        }
    });
    engine.register_fn("remove_attribute", |attribute: &str, id: &str| {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.remove_attribute_modifier(attribute, id);
        }
    });
    engine.register_fn("attribute", |attribute: &str| {
        minecraft()
            .player()
            .ok()
            .flatten()
            .and_then(|player| {
                player
                    .get_attribute_value(attribute)
                    .ok()
                    .flatten()
                    .map(|value| value as f64)
            })
            .unwrap_or(0.0)
    });

     
    engine.register_fn("held_slot", || {
        minecraft()
            .player()
            .ok()
            .flatten()
            .and_then(|player| {
                player
                    .get_inventory()
                    .ok()
                    .and_then(|inventory| inventory.get_selected().ok())
            })
            .unwrap_or(0) as f64
    });
}

fn register_world(engine: &mut rhai::Engine) {
    engine.register_fn("entities", || {
        let Ok(Some(world)) = minecraft().world() else {
            return Vec::<EntityView>::new();
        };
        let Ok(entities) = world.get_entities() else {
            return Vec::new();
        };
        let self_id = minecraft()
            .player()
            .ok()
            .flatten()
            .and_then(|p| p.id().ok());
        entities
            .into_iter()
            .filter(|entity| Some(entity.id().unwrap_or(-1)) != self_id)
            .filter_map(|entity| EntityView::from_entity(&entity))
            .collect()
    });

    engine.register_fn("block_at", |x: f64, y: f64, z: f64| {
        let Ok(Some(world)) = minecraft().world() else {
            return String::new();
        };
        let pos = BlockPos::new(x as i32, y as i32, z as i32);
        world
            .block_registry_name(&pos)
            .unwrap_or_default()
            .to_string()
    });
    engine.register_fn("is_air", |x: f64, y: f64, z: f64| {
        let Ok(Some(world)) = minecraft().world() else {
            return true;
        };
        let pos = BlockPos::new(x as i32, y as i32, z as i32);
        world.is_block_air(&pos).unwrap_or(true)
    });
    engine.register_fn("min_y", || {
        minecraft()
            .world()
            .ok()
            .flatten()
            .and_then(|world| world.get_min_y().ok())
            .unwrap_or(0) as f64
    });
    engine.register_fn("add_particles", |x: f64, y: f64, z: f64| {
        let Ok(Some(world)) = minecraft().world() else {
            return;
        };
        let spawns = vec![(x, y, z, 0.0, 0.0, 0.0)];
        let _ = world.add_dust_particles(255, 255, 255, 1.0, &spawns);
    });
    engine.register_fn("time", || {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    });
}

fn register_combat(engine: &mut rhai::Engine) {
    engine.register_fn("attack", || {
        let _ = minecraft().start_attack();
    });
    engine.register_fn("swing", || {
        if let Ok(Some(player)) = minecraft().player() {
            let _ = player.swing();
        }
    });
    engine.register_fn("attack_entity", |target: EntityView| {
        let (Ok(player), Ok(game_mode)) = (minecraft().player(), minecraft().game_mode()) else {
            return;
        };
        let (Some(player), Some(game_mode)) = (player, game_mode) else {
            return;
        };
        let handle = target.handle.lock().ok().and_then(|handle| handle.clone());
        let Some(handle) = handle else {
            return;
        };
        let entity = Entity::new(handle);
        let _ = game_mode.attack(&player, &entity);
    });
    engine.register_fn("use_item", || {
        let (Ok(player), Ok(game_mode)) = (minecraft().player(), minecraft().game_mode()) else {
            return;
        };
        if let (Some(player), Some(game_mode)) = (player, game_mode) {
            let _ = game_mode.use_item(&player);
        }
    });
    engine.register_fn("gui_open", || {
        crate::graphic::input::GUI_OPEN.load(Ordering::Relaxed)
    });
}

fn register_settings(engine: &mut rhai::Engine, settings: std::sync::Arc<ScriptSettings>) {
    let toggles = std::sync::Arc::clone(&settings);
    engine.register_fn("setting_toggle", move |name: &str| toggles.toggle(name));
    let sliders = std::sync::Arc::clone(&settings);
    engine.register_fn("setting_slider", move |name: &str| sliders.slider(name));
    let choices = std::sync::Arc::clone(&settings);
    engine.register_fn("setting_choice", move |name: &str| choices.choice(name));
    let colors = std::sync::Arc::clone(&settings);
    engine.register_fn("setting_color", move |name: &str| colors.color(name));
}

 
 
#[derive(Debug, Default)]
pub struct ScriptSettings {
    inner: std::sync::Mutex<Vec<crate::module::ModuleSetting>>,
}

impl ScriptSettings {
    pub fn new(settings: Vec<crate::module::ModuleSetting>) -> Self {
        Self {
            inner: std::sync::Mutex::new(settings),
        }
    }

     
     
    pub fn apply(&self, name: &str, value: &SettingPatch) {
        let Ok(mut settings) = self.inner.lock() else {
            return;
        };
        let Some(setting) = settings.iter_mut().find(|setting| setting.name() == name) else {
            return;
        };
        value.apply(setting);
    }

    pub fn snapshot(&self) -> Vec<crate::module::ModuleSetting> {
        self.inner
            .lock()
            .map(|settings| settings.clone())
            .unwrap_or_default()
    }

    pub fn replace(&self, settings: Vec<crate::module::ModuleSetting>) {
        if let Ok(mut current) = self.inner.lock() {
            *current = settings;
        }
    }

    fn toggle(&self, name: &str) -> bool {
        self.inner
            .lock()
            .ok()
            .and_then(|settings| {
                settings
                    .iter()
                    .find(|setting| setting.name() == name)
                    .and_then(|setting| setting.get_toggle_value())
            })
            .unwrap_or(false)
    }

    fn slider(&self, name: &str) -> f64 {
        self.inner
            .lock()
            .ok()
            .and_then(|settings| {
                settings
                    .iter()
                    .find(|setting| setting.name() == name)
                    .and_then(|setting| setting.get_slider_value())
            })
            .unwrap_or(0.0) as f64
    }

    fn choice(&self, name: &str) -> f64 {
        self.inner
            .lock()
            .ok()
            .and_then(|settings| {
                settings
                    .iter()
                    .find(|setting| setting.name() == name)
                    .and_then(|setting| match setting {
                        crate::module::ModuleSetting::Choice { value, .. } => Some(*value),
                        _ => None,
                    })
            })
            .unwrap_or(0) as f64
    }

    fn color(&self, name: &str) -> Dynamic {
        let rgba = self
            .inner
            .lock()
            .ok()
            .and_then(|settings| {
                settings
                    .iter()
                    .find(|setting| setting.name() == name)
                    .and_then(|setting| match setting {
                        crate::module::ModuleSetting::Color { value, .. } => Some(*value),
                        _ => None,
                    })
            })
            .unwrap_or([1.0, 1.0, 1.0, 1.0]);
        Dynamic::from_array(
            rgba.iter()
                .map(|component| Dynamic::from_float(*component as f64))
                .collect(),
        )
    }
}

 
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum SettingPatch {
    Toggle(bool),
    Slider(f32),
    Choice(usize),
    Color([f32; 4]),
    BlockList(Vec<String>),
    Text(String),
}

impl SettingPatch {
     
    pub fn of(setting: &crate::module::ModuleSetting) -> SettingPatch {
        use crate::module::ModuleSetting;
        match setting {
            ModuleSetting::Toggle { value, .. } => SettingPatch::Toggle(*value),
            ModuleSetting::Slider { value, .. } => SettingPatch::Slider(*value),
            ModuleSetting::Choice { value, .. } => SettingPatch::Choice(*value),
            ModuleSetting::Color { value, .. } => SettingPatch::Color(*value),
            ModuleSetting::BlockList { value, .. } => SettingPatch::BlockList(value.clone()),
            ModuleSetting::Text { value, .. } => SettingPatch::Text(value.clone()),
        }
    }

    fn apply(&self, setting: &mut crate::module::ModuleSetting) {
        use crate::module::ModuleSetting;
        match (self, setting) {
            (SettingPatch::Toggle(value), ModuleSetting::Toggle { value: live, .. }) => {
                *live = *value
            }
            (
                SettingPatch::Slider(value),
                ModuleSetting::Slider {
                    value: live,
                    min,
                    max,
                    ..
                },
            ) => *live = value.clamp(*min, *max),
            (
                SettingPatch::Choice(value),
                ModuleSetting::Choice {
                    value: live,
                    options,
                    ..
                },
            ) if *value < options.len() => *live = *value,
            (SettingPatch::Color(value), ModuleSetting::Color { value: live, .. }) => {
                *live = *value
            }
            (SettingPatch::BlockList(value), ModuleSetting::BlockList { value: live, .. }) => {
                *live = value.clone()
            }
            (SettingPatch::Text(value), ModuleSetting::Text { value: live, .. }) => {
                *live = value.clone()
            }
            _ => {}
        }
    }
}

 
 
#[derive(Debug, Clone)]
pub struct EntityView {
     
     
    pub handle: std::sync::Arc<std::sync::Mutex<Option<GlobalRef>>>,
    pub id: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub eye_y: f64,
    pub health: f64,
    pub max_health: f64,
    pub width: f64,
    pub height: f64,
    pub distance: f64,
    pub yaw: f64,
    pub pitch: f64,
    pub on_ground: bool,
    pub alive: bool,
    pub name: String,
}

impl EntityView {
    fn from_entity(entity: &Entity) -> Option<Self> {
        let position = entity.get_position().ok()?;
        let living = entity.as_living();
        let (health, max_health, alive) = match &living {
            Some(living) => (
                living.get_health().unwrap_or(0.0) as f64,
                living.get_max_health().unwrap_or(0.0) as f64,
                living.is_alive().unwrap_or(false),
            ),
            None => (0.0, 0.0, true),
        };
        let eye_y = living
            .as_ref()
            .and_then(|living| living.get_eye_position().ok())
            .map(|eye| eye.y())
            .unwrap_or(position.y());
        let name = living
            .as_ref()
            .and_then(|living| living.get_name().ok())
            .and_then(|component| component.get_string().ok())
            .unwrap_or_default();
        let distance = match minecraft().player().ok().flatten() {
            Some(player) => player
                .distance_to_sqr(position.x(), position.y(), position.z())
                .unwrap_or(f64::MAX)
                .sqrt(),
            None => f64::MAX,
        };
        Some(Self {
            handle: std::sync::Arc::new(std::sync::Mutex::new(Some(entity.jni_ref().clone()))),
            id: entity.id().unwrap_or(-1) as f64,
            x: position.x(),
            y: position.y(),
            z: position.z(),
            eye_y,
            health,
            max_health,
            width: entity.bb_width().unwrap_or(0.0) as f64,
            height: entity.bb_height().unwrap_or(0.0) as f64,
            distance,
            yaw: entity.get_yaw().unwrap_or(0.0) as f64,
            pitch: entity.get_pitch().unwrap_or(0.0) as f64,
            on_ground: entity.on_ground().unwrap_or(false),
            alive,
            name,
        })
    }
}

fn player_bool(read: impl Fn(LocalPlayer) -> anyhow::Result<bool>) -> bool {
    minecraft()
        .player()
        .ok()
        .flatten()
        .and_then(|player| read(player).ok())
        .unwrap_or(false)
}

fn player_f64(read: impl Fn(LocalPlayer) -> anyhow::Result<f64>) -> f64 {
    minecraft()
        .player()
        .ok()
        .flatten()
        .and_then(|player| read(player).ok())
        .unwrap_or(0.0)
}
