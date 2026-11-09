 
 
 
 
 
 
 
 
 
 
 
 
 

use std::collections::BTreeMap;
use std::sync::Arc;

use infered_macros::{find_key, MovementKey, ScreenName};

use crate::mapping::client::screen::Screen;
use crate::mapping::entity::player::{LocalPlayer, Player};
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::MappedObject;
use crate::state::minecraft;

 
 
pub const EXPENSIVE_EVERY: u32 = 8;

 
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerSample {
    pub name: String,
     
    pub distance: f64,
}

 
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
     
     
    pub available: bool,
    pub in_world: bool,
    pub health: f64,
    pub max_health: f64,
    pub food: f64,
    pub armor: f64,
    pub level: i32,
     
    pub slot: i32,
    pub in_water: bool,
    pub on_ground: bool,
    pub falling: bool,
     
    pub moving: bool,
     
    pub screen: Option<ScreenName>,
    pub x: f64,
    pub y: f64,
    pub z: f64,
     
    pub effects: Arc<Vec<(String, i32)>>,
     
    pub item_counts: Arc<BTreeMap<String, i64>>,
     
    pub equipment_hash: u64,
     
    pub inventory_hash: u64,
     
    pub players: Arc<Vec<PlayerSample>>,
}

impl Snapshot {
    pub const MAX_PLAYERS: usize = 48;

     
     
    pub fn value(&self, key: &infered_macros::StateKey) -> serde_json::Value {
        use infered_macros::StateKey;
        use serde_json::json;
        if !self.available {
            return serde_json::Value::Null;
        }
        match key {
            StateKey::Health => json!(self.health),
            StateKey::MaxHealth => json!(self.max_health),
            StateKey::Food => json!(self.food),
            StateKey::Armor => json!(self.armor),
            StateKey::Level => json!(self.level),
            StateKey::Slot => json!(self.slot + 1),
            StateKey::InWater => json!(self.in_water),
            StateKey::OnGround => json!(self.on_ground),
            StateKey::Falling => json!(self.falling),
            StateKey::Moving => json!(self.moving),
            StateKey::Screen => match self.screen {
                Some(screen) => json!(screen.as_str()),
                None => json!("none"),
            },
            StateKey::World => json!(self.in_world),
            StateKey::X => json!(self.x),
            StateKey::Y => json!(self.y),
            StateKey::Z => json!(self.z),
            StateKey::NearbyPlayers => {
                json!(self
                    .players
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>())
            }
            StateKey::PlayerCount => json!(self.players.len()),
            StateKey::NearestPlayerDistance => match self.players.first() {
                Some(player) => json!(player.distance),
                None => serde_json::Value::Null,
            },
            StateKey::ItemCount(id) => json!(self.item_counts.get(id).copied().unwrap_or(0)),
            StateKey::HasItem(id) => json!(self.item_counts.get(id).copied().unwrap_or(0) > 0),
            StateKey::Effect(name) => {
                match self.effects.iter().find(|(effect, _)| effect == name) {
                     
                     
                    Some((_, amplifier)) => json!(amplifier + 1),
                    None => serde_json::Value::Null,
                }
            }
        }
    }
}

 
pub trait Host {
     
    fn snapshot(&mut self) -> Snapshot;

     
    fn tap_key(&mut self, key: &str) -> anyhow::Result<()>;
     
    fn set_movement(&mut self, key: MovementKey, down: bool) -> anyhow::Result<()>;
     
    fn set_use(&mut self, down: bool) -> anyhow::Result<()>;
    fn attack(&mut self) -> anyhow::Result<()>;
    fn interact(&mut self) -> anyhow::Result<()>;
    fn pick_block(&mut self) -> anyhow::Result<()>;
     
    fn hotbar(&mut self, slot: u8) -> anyhow::Result<()>;
    fn selected_slot(&mut self) -> anyhow::Result<i32>;
    fn open_inventory(&mut self) -> anyhow::Result<()>;
    fn close_screen(&mut self) -> anyhow::Result<()>;
    fn send_chat(&mut self, message: &str) -> anyhow::Result<()>;

    fn log(&mut self, message: &str) {
        log::info!("[Macros] {message}");
    }
}

 
#[derive(Debug, Default)]
pub struct RealHost {
    ticks: u32,
    sampled: bool,
    effects: Arc<Vec<(String, i32)>>,
    item_counts: Arc<BTreeMap<String, i64>>,
    equipment_hash: u64,
    inventory_hash: u64,
    players: Arc<Vec<PlayerSample>>,
}

impl RealHost {
    pub fn new() -> RealHost {
        RealHost::default()
    }

     
     
    fn sample_expensive(&mut self, player: &LocalPlayer) {
        self.ticks = self.ticks.wrapping_add(1);
        if self.sampled && self.ticks % EXPENSIVE_EVERY != 0 {
            return;
        }
        self.sampled = true;

        if let Ok(effects) = player.get_active_effects() {
            self.effects = Arc::new(
                effects
                    .into_iter()
                    .map(|(name, amplifier, _duration)| (name, amplifier))
                    .collect(),
            );
        }

        let (counts, inventory_hash, equipment_hash) = scan_inventory(player);
        self.item_counts = Arc::new(counts);
        self.inventory_hash = inventory_hash;
        self.equipment_hash = equipment_hash;
        self.players = Arc::new(scan_players(player));
    }
}

impl Host for RealHost {
    fn snapshot(&mut self) -> Snapshot {
        let mc = minecraft();
        let Ok(Some(player)) = mc.player() else {
            return Snapshot::default();
        };
        let mut snapshot = Snapshot {
            available: true,
            in_world: matches!(mc.world(), Ok(Some(_))),
            screen: screen_name(),
            ..Snapshot::default()
        };
        snapshot.health = player.get_health().unwrap_or(0.0) as f64;
        snapshot.max_health = player.get_max_health().unwrap_or(20.0) as f64;
        snapshot.food = player.food_level().unwrap_or(20) as f64;
        snapshot.armor = player.get_armor_value().unwrap_or(0) as f64;
        snapshot.level = player.experience_level().unwrap_or(0);
        snapshot.slot = player
            .get_inventory()
            .and_then(|inventory| inventory.get_selected())
            .unwrap_or(0)
            .clamp(0, 8);
        snapshot.in_water = player.is_in_water().unwrap_or(false);
        snapshot.on_ground = player.on_ground().unwrap_or(true);
        if let Ok(position) = player.get_position() {
            snapshot.x = position.x();
            snapshot.y = position.y();
            snapshot.z = position.z();
        }
        snapshot.moving = player
            .get_delta_movement()
            .map(|velocity| velocity.x().hypot(velocity.z()) > 0.02)
            .unwrap_or(false);
        snapshot.falling = !snapshot.on_ground && player.get_fall_distance().unwrap_or(0.0) > 0.1;

        self.sample_expensive(&player);
        snapshot.effects = Arc::clone(&self.effects);
        snapshot.item_counts = Arc::clone(&self.item_counts);
        snapshot.equipment_hash = self.equipment_hash;
        snapshot.inventory_hash = self.inventory_hash;
        snapshot.players = Arc::clone(&self.players);
        snapshot
    }

    fn tap_key(&mut self, key: &str) -> anyhow::Result<()> {
        let Some(spec) = find_key(key) else {
            anyhow::bail!("{key:?} is not a supported key");
        };
        crate::mapping::client::options::KeyMapping::click_named(spec.translation)
    }

    fn set_movement(&mut self, key: MovementKey, down: bool) -> anyhow::Result<()> {
        let options = minecraft().options()?;
        let binding = match key {
            MovementKey::Forward => options.key_up()?,
            MovementKey::Back => options.key_back()?,
            MovementKey::Left => options.key_left()?,
            MovementKey::Right => options.key_right()?,
            MovementKey::Jump => options.key_jump()?,
            MovementKey::Sneak => options.key_shift()?,
            MovementKey::Sprint => options.key_sprint()?,
        };
        binding.set_down(down)
    }

    fn set_use(&mut self, down: bool) -> anyhow::Result<()> {
        minecraft().options()?.key_use()?.set_down(down)
    }

    fn attack(&mut self) -> anyhow::Result<()> {
        minecraft().start_attack()
    }

    fn interact(&mut self) -> anyhow::Result<()> {
        minecraft().start_use_item()
    }

    fn pick_block(&mut self) -> anyhow::Result<()> {
        minecraft().pick_block()
    }

    fn hotbar(&mut self, slot: u8) -> anyhow::Result<()> {
        if !(1..=9).contains(&slot) {
            anyhow::bail!("hotbar slot {slot} is outside 1-9");
        }
        crate::mapping::client::options::KeyMapping::click_named(&format!("key.keyboard.{slot}"))
    }

    fn selected_slot(&mut self) -> anyhow::Result<i32> {
        let player = minecraft()
            .player()?
            .ok_or_else(|| anyhow::anyhow!("no player"))?;
        player.get_inventory()?.get_selected()
    }

    fn open_inventory(&mut self) -> anyhow::Result<()> {
        let player = minecraft()
            .player()?
            .ok_or_else(|| anyhow::anyhow!("no player"))?;
        minecraft().open_inventory_screen(&player)
    }

    fn close_screen(&mut self) -> anyhow::Result<()> {
        minecraft().close_screen()
    }

    fn send_chat(&mut self, message: &str) -> anyhow::Result<()> {
        minecraft().send_chat(message)
    }
}

 
 
 
pub fn screen_name() -> Option<ScreenName> {
    match minecraft().current_screen() {
        Screen::None => None,
        Screen::Chat => Some(ScreenName::Chat),
        Screen::Inventory => Some(ScreenName::Inventory),
        Screen::Container => Some(ScreenName::Container),
        Screen::Crafting => Some(ScreenName::Crafting),
        Screen::Menu => Some(ScreenName::Menu),
        Screen::Disconnected => Some(ScreenName::Disconnected),
        Screen::Unknown => {
            if minecraft().title_screen_open() {
                Some(ScreenName::Title)
            } else {
                Some(ScreenName::Unknown)
            }
        }
    }
}

 
 
 
fn scan_inventory(player: &LocalPlayer) -> (BTreeMap<String, i64>, u64, u64) {
    use std::hash::{Hash, Hasher};

    let mut counts = BTreeMap::new();
    let mut inventory_hasher = std::collections::hash_map::DefaultHasher::new();
    let mut equipment_hasher = std::collections::hash_map::DefaultHasher::new();
    let Ok(inventory) = player.get_inventory() else {
        return (counts, 0, 0);
    };
    for slot in 0..=40 {
        let Ok(stack) = inventory.get_item(slot) else {
            continue;
        };
        if stack.is_empty().unwrap_or(true) {
            continue;
        }
        let Ok(item) = stack.get_item() else {
            continue;
        };
        let Ok(name) = item.registry_name() else {
            continue;
        };
        let count = stack.count().unwrap_or(1).max(0) as i64;
        *counts.entry(name.clone()).or_insert(0) += count;
        (slot, &name, count).hash(&mut inventory_hasher);
        if (36..=40).contains(&slot) {
            (slot, &name, count).hash(&mut equipment_hasher);
        }
    }
    (counts, inventory_hasher.finish(), equipment_hasher.finish())
}

 
fn scan_players(player: &LocalPlayer) -> Vec<PlayerSample> {
    let mc = minecraft();
    let Ok(Some(world)) = mc.world() else {
        return Vec::new();
    };
    let Ok(local_id) = player.id() else {
        return Vec::new();
    };
    let Ok(origin) = player.get_position() else {
        return Vec::new();
    };
    let Ok(entities) = world.get_entities() else {
        return Vec::new();
    };
    let mut players = Vec::new();
    for entity in entities {
        if !entity.instance_of::<Player>() {
            continue;
        }
        let Ok(id) = entity.id() else {
            continue;
        };
        if id == local_id {
            continue;
        }
        let Ok(position) = entity.get_position() else {
            continue;
        };
        let name = entity
            .get_name()
            .ok()
            .and_then(|name| name.get_string().ok())
            .unwrap_or_default();
        if name.trim().is_empty() {
            continue;
        }
        let distance = (position.x() - origin.x())
            .hypot(position.z() - origin.z())
            .hypot(position.y() - origin.y());
        players.push(PlayerSample { name, distance });
    }
    players.sort_by(|a, b| a.distance.total_cmp(&b.distance));
    players.truncate(Snapshot::MAX_PLAYERS);
    players
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unavailable_snapshot_resolves_to_null() {
        let snapshot = Snapshot::default();
        assert_eq!(
            snapshot.value(&infered_macros::StateKey::Health),
            serde_json::Value::Null
        );
        assert!(!snapshot.available);
    }

    #[test]
    fn slots_are_reported_one_based_and_effects_as_levels() {
        let snapshot = Snapshot {
            available: true,
            in_world: true,
            slot: 0,
            effects: Arc::new(vec![("speed".into(), 1)]),
            item_counts: Arc::new(BTreeMap::from([("minecraft:stone".into(), 12)])),
            players: Arc::new(vec![PlayerSample {
                name: "Alex".into(),
                distance: 3.5,
            }]),
            ..Snapshot::default()
        };
        assert_eq!(snapshot.value(&infered_macros::StateKey::Slot), 1);
        assert_eq!(
            snapshot.value(&infered_macros::StateKey::Effect("speed".into())),
            2
        );
        assert_eq!(
            snapshot.value(&infered_macros::StateKey::ItemCount(
                "minecraft:stone".into()
            )),
            12
        );
        assert_eq!(
            snapshot.value(&infered_macros::StateKey::NearestPlayerDistance),
            3.5
        );
        assert_eq!(
            snapshot.value(&infered_macros::StateKey::NearbyPlayers),
            serde_json::json!(["Alex"])
        );
    }
}
