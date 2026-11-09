use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const MAX_ACTIONS: usize = 1024;
pub const MAX_DEPTH: usize = 16;
pub const MAX_ITERATIONS: u32 = 10_000;
pub const MAX_RUN_DEPTH: usize = 8;
pub const MAX_DELAY_MS: u64 = 3_600_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Macro {
    pub format_version: u32,
    pub id: String,
    pub name: String,
    pub description: String,
    pub key: Option<String>,
    pub enabled: bool,
    pub cooldown_ms: u64,
    pub triggers: Vec<Trigger>,
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
    pub limits: Limits,
    pub variables: BTreeMap<String, Value>,
}
impl Default for Macro {
    fn default() -> Self {
        Self::new_template("", "")
    }
}
impl Macro {
    pub fn new_template(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            format_version: 1,
            id: id.into(),
            name: name.into(),
            description: String::new(),
            key: None,
            enabled: false,
            cooldown_ms: 0,
            triggers: vec![Trigger::Manual],
            conditions: Vec::new(),
            actions: Vec::new(),
            limits: Limits::default(),
            variables: BTreeMap::new(),
        }
    }
    pub fn cooldown(&self) -> u64 {
        self.cooldown_ms
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct Limits {
    pub max_runtime_ms: u64,
    pub max_iterations: u32,
    pub on_error: ErrorPolicy,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_runtime_ms: 60_000,
            max_iterations: 1_000,
            on_error: ErrorPolicy::Stop,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorPolicy {
    Stop,
    Continue,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyAction {
    Tap,
    Press,
    Release,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MovementKey {
    Forward,
    Back,
    Left,
    Right,
    Jump,
    Sneak,
    Sprint,
}
impl MovementKey {
    pub fn label(self) -> &'static str {
        match self {
            Self::Forward => "Forward",
            Self::Back => "Back",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Jump => "Jump",
            Self::Sneak => "Sneak",
            Self::Sprint => "Sprint",
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScreenName {
    Any,
    Chat,
    Inventory,
    Container,
    Crafting,
    Menu,
    Disconnected,
    Title,
    Unknown,
}
impl ScreenName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Chat => "chat",
            Self::Inventory => "inventory",
            Self::Container => "container",
            Self::Crafting => "crafting",
            Self::Menu => "menu",
            Self::Disconnected => "disconnected",
            Self::Title => "title",
            Self::Unknown => "unknown",
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionMode {
    Once,
    WhileTrue,
    BecomesFalse,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Contains,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    All {
        conditions: Vec<Condition>,
    },
    Any {
        conditions: Vec<Condition>,
    },
    Not {
        condition: Box<Condition>,
    },
    Compare {
        left: ValueRef,
        op: CompareOp,
        right: ValueRef,
    },
}
impl Condition {
    pub fn compare(left: ValueRef, op: CompareOp, right: ValueRef) -> Self {
        Self::Compare { left, op, right }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValueRef {
    Literal { value: Value },
    Var { name: String },
    State { path: StateKey },
}
impl ValueRef {
    pub fn literal(value: Value) -> Self {
        Self::Literal { value }
    }
    pub fn state(path: &str) -> Self {
        Self::State {
            path: StateKey::parse(path),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateKey {
    Health,
    MaxHealth,
    Food,
    Armor,
    Level,
    Slot,
    InWater,
    OnGround,
    Falling,
    Moving,
    Screen,
    World,
    X,
    Y,
    Z,
    NearbyPlayers,
    PlayerCount,
    NearestPlayerDistance,
    ItemCount(String),
    HasItem(String),
    Effect(String),
}
impl StateKey {
    pub fn parse(s: &str) -> Self {
        match s {
            "health" => Self::Health,
            "max_health" => Self::MaxHealth,
            "food" => Self::Food,
            "armor" => Self::Armor,
            "level" => Self::Level,
            "slot" => Self::Slot,
            "in_water" => Self::InWater,
            "on_ground" => Self::OnGround,
            "falling" => Self::Falling,
            "moving" => Self::Moving,
            "screen" => Self::Screen,
            "world" => Self::World,
            "x" => Self::X,
            "y" => Self::Y,
            "z" => Self::Z,
            "nearby_players" => Self::NearbyPlayers,
            "player_count" => Self::PlayerCount,
            "nearest_player_distance" => Self::NearestPlayerDistance,
            _ if s.starts_with("item_count:") || s.starts_with("item_count.") => {
                Self::ItemCount(s[11..].into())
            }
            _ if s.starts_with("has_item:") || s.starts_with("has_item.") => {
                Self::HasItem(s[9..].into())
            }
            _ if s.starts_with("effect:") || s.starts_with("effect.") => {
                Self::Effect(s[7..].into())
            }
            _ => Self::Effect(format!("__unknown_state__{s}")),
        }
    }
    fn string(&self) -> String {
        match self {
            Self::Health => "health".into(),
            Self::MaxHealth => "max_health".into(),
            Self::Food => "food".into(),
            Self::Armor => "armor".into(),
            Self::Level => "level".into(),
            Self::Slot => "slot".into(),
            Self::InWater => "in_water".into(),
            Self::OnGround => "on_ground".into(),
            Self::Falling => "falling".into(),
            Self::Moving => "moving".into(),
            Self::Screen => "screen".into(),
            Self::World => "world".into(),
            Self::X => "x".into(),
            Self::Y => "y".into(),
            Self::Z => "z".into(),
            Self::NearbyPlayers => "nearby_players".into(),
            Self::PlayerCount => "player_count".into(),
            Self::NearestPlayerDistance => "nearest_player_distance".into(),
            Self::ItemCount(s) => format!("item_count:{s}"),
            Self::HasItem(s) => format!("has_item:{s}"),
            Self::Effect(s) => format!("effect:{s}"),
        }
    }
}
impl Serialize for StateKey {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.string())
    }
}
impl<'de> Deserialize<'de> for StateKey {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = String::deserialize(d)?;
        let key = Self::parse(&value);
        match &key {
            Self::Effect(name) if name.starts_with("__unknown_state__") => Err(
                serde::de::Error::custom(format!("unknown state key {value:?}")),
            ),
            Self::Effect(name) | Self::HasItem(name) | Self::ItemCount(name) if name.is_empty() => {
                Err(serde::de::Error::custom("state key requires a name"))
            }
            _ => Ok(key),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Trigger {
    Manual,
    KeyPress {
        key: String,
    },
    KeyRelease {
        key: String,
    },
    MousePress {
        button: u8,
    },
    MouseRelease {
        button: u8,
    },
    Interval {
        every_ms: u64,
        start_delay_ms: u64,
    },
    AfterDelay {
        delay_ms: u64,
    },
    Condition {
        when: Condition,
        mode: ConditionMode,
        every_ms: u64,
    },
    WorldLoaded,
    WorldUnloaded,
    PlayerJoined {
        name: Option<String>,
    },
    PlayerLeft {
        name: Option<String>,
    },
    PlayerEnteredRange {
        range: f64,
        name: Option<String>,
    },
    PlayerLeftRange {
        range: f64,
        name: Option<String>,
    },
    PlayerDistanceChanged {
        by: f64,
        name: Option<String>,
    },
    HealthBelow {
        value: f64,
    },
    HungerBelow {
        value: f64,
    },
    SlotChanged,
    InventoryChanged,
    EquipmentChanged,
    LevelChanged,
    EffectGained {
        effect: Option<String>,
    },
    EffectLost {
        effect: Option<String>,
    },
    ScreenOpened {
        screen: ScreenName,
    },
    ScreenClosed {
        screen: ScreenName,
    },
    StartedMoving,
    StoppedMoving,
    EnteredWater,
    LeftWater,
    StartedFalling,
    Landed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Wait {
        duration_ms: u64,
    },
    WaitRandom {
        min_ms: u64,
        max_ms: u64,
    },
    WaitUntil {
        condition: Condition,
        timeout_ms: u64,
    },
    Key {
        key: String,
        action: KeyAction,
        hold_ms: u64,
    },
    Mouse {
        button: u8,
        action: KeyAction,
        hold_ms: u64,
    },
    ReleaseInputs,
    Hotbar {
        slot: u8,
    },
    CycleHotbar {
        steps: i32,
    },
    UseItem {
        hold_ms: u64,
    },
    Attack,
    Interact,
    PickBlock,
    Movement {
        key: MovementKey,
        action: KeyAction,
        hold_ms: u64,
    },
    OpenInventory,
    CloseScreen,
    Chat {
        message: String,
    },
    SetVar {
        name: String,
        value: Value,
    },
    If {
        condition: Condition,
        then: Vec<Action>,
        otherwise: Vec<Action>,
    },
    Repeat {
        times: u32,
        actions: Vec<Action>,
    },
    RepeatWhile {
        condition: Condition,
        max_iterations: u32,
        actions: Vec<Action>,
    },
    RepeatUntil {
        condition: Condition,
        max_iterations: u32,
        actions: Vec<Action>,
    },
    RunMacro {
        id: String,
    },
    Stop,
    StopAll,
    Log {
        message: String,
    },
}
