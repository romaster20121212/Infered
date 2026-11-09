 
 
 
 
 
 
 
 
 
 
 
 

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use infered_macros::{load_library, save, Library, Macro, ScreenName, Trigger};
use serde_json::Value as Json;

use super::conds;
use super::host::{Host, Snapshot};
use super::run::{Ownership, Rng, Run, TICK_MS};

 
pub const SCAN_INTERVAL: Duration = Duration::from_millis(1_000);

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub enabled: bool,
    pub stop_in_menus: bool,
    pub stop_on_world_change: bool,
    pub stop_on_death: bool,
}

impl Default for Policy {
    fn default() -> Self {
        Policy {
            enabled: false,
            stop_in_menus: true,
            stop_on_world_change: true,
            stop_on_death: true,
        }
    }
}

 
#[derive(Debug, Clone)]
struct Loaded {
    file_name: String,
    macro_: Arc<Macro>,
    modified: Option<SystemTime>,
}

 
#[derive(Debug, Clone)]
struct MacroState {
    enabled: bool,
    armed_ms: u64,
    last_end_ms: u64,
     
    triggers: Vec<TriggerMemory>,
}

impl MacroState {
    fn new(macro_: &Macro, now_ms: u64) -> MacroState {
        MacroState {
            enabled: macro_.enabled,
            armed_ms: now_ms,
            last_end_ms: 0,
            triggers: macro_
                .triggers
                .iter()
                .map(|_| TriggerMemory::default())
                .collect(),
        }
    }

    fn reset_session(&mut self, macro_: &Macro, now_ms: u64) {
        self.armed_ms = now_ms;
        self.last_end_ms = 0;
        self.triggers = macro_
            .triggers
            .iter()
            .map(|trigger| TriggerMemory {
                next_ms: match trigger {
                    Trigger::Interval { start_delay_ms, .. } => now_ms + start_delay_ms,
                    _ => 0,
                },
                ..TriggerMemory::default()
            })
            .collect();
    }
}

 
#[derive(Debug, Clone, Default)]
struct TriggerMemory {
     
    last: bool,
     
    next_ms: u64,
     
    fired: bool,
}

 
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    Key { glfw: i32, pressed: bool },
    Mouse { button: i32, pressed: bool },
}

 
#[derive(Debug, Clone, PartialEq)]
pub struct RunStatus {
    pub id: String,
    pub name: String,
    pub status: String,
    pub error: Option<String>,
    pub elapsed_ms: u64,
}

 
#[derive(Debug, Clone, PartialEq)]
pub struct MacroControl {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub running: bool,
    pub status: String,
}

 
pub struct Engine {
    pub dir: PathBuf,
    policy: Policy,
    library: Vec<Loaded>,
    broken: usize,
    fingerprint: Option<u64>,
    last_scan: Option<Instant>,
    now_ms: u64,
    active: bool,
    session: u64,
    world_loaded_pending: bool,
    states: HashMap<String, MacroState>,
    runs: HashMap<String, Run>,
    ownership: Ownership,
    rng: Rng,
    events: Vec<InputEvent>,
    previous: Snapshot,
    stop_all: bool,
     
    pub last_error: Option<String>,
}

impl Engine {
    pub fn new(dir: PathBuf) -> Engine {
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0x1234_5678);
        Engine {
            dir,
            policy: Policy::default(),
            library: Vec::new(),
            broken: 0,
            fingerprint: None,
            last_scan: None,
            now_ms: 0,
            active: false,
            session: 0,
            world_loaded_pending: false,
            states: HashMap::new(),
            runs: HashMap::new(),
            ownership: Ownership::default(),
            rng: Rng::new(seed),
            events: Vec::new(),
            previous: Snapshot::default(),
            stop_all: false,
            last_error: None,
        }
    }

     
     
    pub fn set_policy(&mut self, host: &mut dyn Host, policy: Policy) {
        let was_enabled = self.policy.enabled;
        self.policy = policy;
        if was_enabled && !policy.enabled {
            self.stop_everything(host, "macro system disabled");
        }
    }

     
    pub fn maintain(&mut self, host: &mut dyn Host, force: bool) {
        let now = Instant::now();
        let due = force
            || self
                .last_scan
                .is_none_or(|last| now.duration_since(last) >= SCAN_INTERVAL);
        if !due {
            return;
        }
        self.last_scan = Some(now);
        self.reload(host);
    }

     
     
    pub fn reload(&mut self, host: &mut dyn Host) {
        let library = load_library(&self.dir);
        let fingerprint = fingerprint(&library);
        if self.fingerprint == Some(fingerprint) {
            return;
        }
        self.fingerprint = Some(fingerprint);
        self.apply_library(host, library);
    }

    fn apply_library(&mut self, host: &mut dyn Host, library: Library) {
        let mut entries = Vec::new();
        let mut present = HashSet::new();
        for file in library.macros {
            let id = file.macro_.id.clone();
            present.insert(id.clone());
            let state = self
                .states
                .entry(id.clone())
                .or_insert_with(|| MacroState::new(&file.macro_, self.now_ms));
            state.enabled = file.macro_.enabled;
            if state.triggers.len() != file.macro_.triggers.len() {
                state.reset_session(&file.macro_, self.now_ms);
            }
            if !state.enabled {
                if let Some(mut run) = self.runs.remove(&id) {
                    run.release_inputs(host, &mut self.ownership);
                    run.finished = true;
                }
            }
            entries.push(Loaded {
                file_name: file.file_name,
                macro_: Arc::new(file.macro_),
                modified: file.modified,
            });
        }

        let removed: Vec<String> = self
            .runs
            .keys()
            .filter(|id| !present.contains(*id))
            .cloned()
            .collect();
        for id in removed {
            if let Some(mut run) = self.runs.remove(&id) {
                run.release_inputs(host, &mut self.ownership);
                log::info!("macro {} stopped: its file was removed", run.name);
            }
        }
        self.states.retain(|id, _| present.contains(id));
        self.library = entries;
        self.broken = library.broken.len();
        if self.broken > 0 {
            log::warn!(
                "{} macro file(s) in {} could not be loaded",
                self.broken,
                self.dir.display()
            );
        }
    }

     
     
    pub fn update_session(&mut self, host: &mut dyn Host, in_world: bool, alive: bool) {
        let active = in_world && alive;
        if active == self.active {
            return;
        }
        if active {
            self.active = true;
            self.session = self.session.wrapping_add(1);
            self.world_loaded_pending = true;
            let now = self.now_ms;
            let macros: Vec<Arc<Macro>> =
                self.library.iter().map(|l| Arc::clone(&l.macro_)).collect();
            for macro_ in macros {
                if let Some(state) = self.states.get_mut(&macro_.id) {
                    state.reset_session(&macro_, now);
                    state.enabled = macro_.enabled;
                }
            }
            self.previous = Snapshot::default();
        } else {
            let died = in_world;
            let reset = if died {
                self.policy.stop_on_death
            } else {
                self.policy.stop_on_world_change
            };
            self.stop_everything(
                host,
                if died {
                    "player died"
                } else {
                    "world unloaded"
                },
            );
            self.active = false;
            self.previous = Snapshot::default();
            if reset {
                let now = self.now_ms;
                let macros: Vec<Arc<Macro>> =
                    self.library.iter().map(|l| Arc::clone(&l.macro_)).collect();
                for macro_ in macros {
                    if let Some(state) = self.states.get_mut(&macro_.id) {
                        state.reset_session(&macro_, now);
                    }
                }
            }
        }
    }

     
    pub fn tick(&mut self, host: &mut dyn Host) {
        if !self.active {
             
             
            self.events.clear();
            return;
        }
        if !self.policy.enabled {
            self.events.clear();
            if !self.runs.is_empty() {
                self.stop_everything(host, "macro system disabled");
            }
            return;
        }

        let snapshot = host.snapshot();
        if !snapshot.available {
            self.stop_everything(host, "no live player");
            self.events.clear();
            return;
        }

        if self.policy.stop_in_menus && snapshot.screen.is_some() {
            if !self.runs.is_empty() {
                self.stop_everything(host, "a screen opened");
            }
            self.events.clear();
            self.previous = snapshot;
            return;
        }

        self.now_ms = self.now_ms.wrapping_add(TICK_MS);
        let events = std::mem::take(&mut self.events);
        let world_loaded = std::mem::take(&mut self.world_loaded_pending);

         
        let now = self.now_ms;
        let previous = &self.previous;
        let macros: Vec<Arc<Macro>> = self
            .library
            .iter()
            .map(|loaded| Arc::clone(&loaded.macro_))
            .collect();
        for macro_ in &macros {
            let id = macro_.id.as_str();
            if self.runs.contains_key(id) {
                continue;
            }
            let Some(state) = self.states.get_mut(id) else {
                continue;
            };
            if !state.enabled || now < state.last_end_ms.saturating_add(macro_.cooldown()) {
                continue;
            }
            if !conds::all_hold(&macro_.conditions, &macro_.variables, &snapshot) {
                continue;
            }
            if state.triggers.len() != macro_.triggers.len() {
                state.triggers = macro_
                    .triggers
                    .iter()
                    .map(|_| TriggerMemory::default())
                    .collect();
            }
            let armed_ms = state.armed_ms;
            let Some(event_vars) = trigger_for(
                macro_,
                &mut state.triggers,
                previous,
                &snapshot,
                &events,
                now,
                armed_ms,
                world_loaded,
            ) else {
                continue;
            };
            let run = Run::new(macro_, event_vars, now);
            log::info!("macro '{}' started: {}", run.name, run.id);
            self.runs.insert(id.to_string(), run);
        }

         
        let catalog: BTreeMap<String, Arc<Macro>> = self
            .library
            .iter()
            .map(|loaded| (loaded.macro_.id.clone(), Arc::clone(&loaded.macro_)))
            .collect();
        let ids: Vec<String> = self.runs.keys().cloned().collect();
        {
            let Engine {
                runs,
                ownership,
                rng,
                now_ms,
                ..
            } = self;
            for id in &ids {
                let Some(run) = runs.get_mut(id) else {
                    continue;
                };
                run.advance(host, &snapshot, *now_ms, rng, ownership, &catalog);
            }
        }

        let finished: Vec<String> = self
            .runs
            .iter()
            .filter(|(_, run)| run.finished)
            .map(|(id, _)| id.clone())
            .collect();
        for id in finished {
            if let Some(mut run) = self.runs.remove(&id) {
                run.release_inputs(host, &mut self.ownership);
                if let Some(state) = self.states.get_mut(&id) {
                    state.last_end_ms = self.now_ms;
                }
                if run.stop_all {
                    self.stop_all = true;
                }
                if let Some(error) = &run.error {
                    log::warn!("macro '{}' stopped: {error}", run.name);
                    self.last_error = Some(format!("{}: {error}", run.name));
                } else {
                    log::info!("macro '{}' finished", run.name);
                }
            }
        }

        if self.stop_all {
            self.stop_all = false;
            self.stop_everything(host, "stop_all action");
        }

        self.previous = snapshot;
    }

     
    pub fn stop_everything(&mut self, host: &mut dyn Host, reason: &str) {
        if self.runs.is_empty() {
            return;
        }
        let ids: Vec<String> = self.runs.keys().cloned().collect();
        for id in ids {
            if let Some(mut run) = self.runs.remove(&id) {
                run.release_inputs(host, &mut self.ownership);
                if let Some(state) = self.states.get_mut(&id) {
                    state.last_end_ms = self.now_ms;
                }
                log::info!("macro '{}' stopped: {reason}", run.name);
            }
        }
    }

     
     
    pub fn push_event(&mut self, event: InputEvent) {
        if self.events.len() < 256 {
            self.events.push(event);
        }
    }

     
    pub fn set_macro_enabled(
        &mut self,
        host: &mut dyn Host,
        id: &str,
        enabled: bool,
    ) -> Result<(), String> {
        let Some(index) = self.library.iter().position(|l| l.macro_.id == id) else {
            return Err(format!("macro {id} is not loaded"));
        };
        let mut macro_ = (*self.library[index].macro_).clone();
        if macro_.enabled == enabled {
            return Ok(());
        }
        macro_.enabled = enabled;
        let file_name = save(&self.dir, Some(&self.library[index].file_name), &macro_)?;
        self.library[index].file_name = file_name;
        self.library[index].macro_ = Arc::new(macro_.clone());
        self.library[index].modified = Some(SystemTime::now());
        self.fingerprint = None;
        if let Some(state) = self.states.get_mut(id) {
            state.enabled = enabled;
        }
        if !enabled {
            if let Some(mut run) = self.runs.remove(id) {
                run.release_inputs(host, &mut self.ownership);
            }
        }
        Ok(())
    }

     
    pub fn invalidate_scan(&mut self) {
        self.fingerprint = None;
        self.last_scan = None;
    }

     
    pub fn controls(&self) -> Vec<MacroControl> {
        self.library
            .iter()
            .map(|loaded| {
                let id = &loaded.macro_.id;
                let run = self.runs.get(id);
                MacroControl {
                    id: id.clone(),
                    name: loaded.macro_.name.clone(),
                    enabled: self
                        .states
                        .get(id)
                        .map(|state| state.enabled)
                        .unwrap_or(loaded.macro_.enabled),
                    running: run.is_some(),
                    status: run.map(|run| run.status.clone()).unwrap_or_default(),
                }
            })
            .collect()
    }

     
    pub fn running(&self) -> Vec<RunStatus> {
        let mut runs: Vec<RunStatus> = self
            .runs
            .values()
            .map(|run| RunStatus {
                id: run.id.clone(),
                name: run.name.clone(),
                status: run.status.clone(),
                error: run.error.clone(),
                elapsed_ms: self.now_ms.saturating_sub(run.started_ms),
            })
            .collect();
        runs.sort_by(|a, b| a.name.cmp(&b.name));
        runs
    }

    pub fn enabled(&self) -> bool {
        self.policy.enabled
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn broken_files(&self) -> usize {
        self.broken
    }

    pub fn macro_count(&self) -> usize {
        self.library.len()
    }

     
    pub fn shutdown(&mut self, host: &mut dyn Host) {
        self.stop_everything(host, "client shutdown");
        self.events.clear();
        self.active = false;
    }

     
    pub fn directory(&self) -> &Path {
        &self.dir
    }

     
    pub fn held_inputs(&self) -> Vec<(String, String)> {
        self.ownership
            .held()
            .into_iter()
            .map(|(held, owner)| (format!("{held:?}"), owner))
            .collect()
    }

     
    pub fn macros(&self) -> Vec<Arc<Macro>> {
        self.library.iter().map(|l| Arc::clone(&l.macro_)).collect()
    }
}

 
 
 
fn fingerprint(library: &Library) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for file in &library.macros {
        file.file_name.hash(&mut hasher);
        file.macro_.id.hash(&mut hasher);
        file.macro_.enabled.hash(&mut hasher);
        file.macro_.name.hash(&mut hasher);
        file.macro_.actions.len().hash(&mut hasher);
        if let Some(modified) = file.modified {
            modified
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
                .hash(&mut hasher);
        }
    }
    for broken in &library.broken {
        broken.file_name.hash(&mut hasher);
    }
    0x6D61_6372_6F73 ^ hasher.finish()
}

 
fn nearest_matching<'a>(
    players: &'a [super::host::PlayerSample],
    name: &Option<String>,
) -> Option<&'a super::host::PlayerSample> {
    players.iter().find(|player| {
        name.as_ref()
            .is_none_or(|filter| player.name.eq_ignore_ascii_case(filter))
    })
}

fn contains_player(players: &[super::host::PlayerSample], name: &str) -> bool {
    players
        .iter()
        .any(|player| player.name.eq_ignore_ascii_case(name))
}

 
 
#[allow(clippy::too_many_arguments)]
fn trigger_for(
    macro_: &Macro,
    memories: &mut [TriggerMemory],
    previous: &Snapshot,
    snapshot: &Snapshot,
    events: &[InputEvent],
    now_ms: u64,
    armed_ms: u64,
    world_loaded: bool,
) -> Option<BTreeMap<String, Json>> {
    for (index, trigger) in macro_.triggers.iter().enumerate() {
        let Some(memory) = memories.get_mut(index) else {
            continue;
        };
        if let Some(vars) = fires(
            trigger,
            memory,
            macro_,
            now_ms,
            previous,
            snapshot,
            events,
            world_loaded,
            armed_ms,
        ) {
            return Some(vars);
        }
    }
    None
}

 
 
#[allow(clippy::too_many_arguments)]
fn fires(
    trigger: &Trigger,
    memory: &mut TriggerMemory,
    macro_: &Macro,
    now_ms: u64,
    previous: &Snapshot,
    snapshot: &Snapshot,
    events: &[InputEvent],
    world_loaded: bool,
    armed_ms: u64,
) -> Option<BTreeMap<String, Json>> {
    let mut vars = BTreeMap::new();
    let fire = match trigger {
        Trigger::Manual => {
            let key = macro_.key.as_ref()?;
            let spec = infered_macros::find_key(key)?;
            let pressed = events.iter().any(|event| {
                matches!(event, InputEvent::Key { glfw, pressed } if *glfw == spec.glfw && *pressed)
            });
            if pressed {
                vars.insert("event.key".into(), Json::String(spec.name.into()));
            }
            pressed
        }
        Trigger::KeyPress { key } | Trigger::KeyRelease { key } => {
            let Some(spec) = infered_macros::find_key(key) else {
                return None;
            };
            let want = matches!(trigger, Trigger::KeyPress { .. });
            let hit = events.iter().any(|event| {
                matches!(event, InputEvent::Key { glfw, pressed } if *glfw == spec.glfw && *pressed == want)
            });
            if hit {
                vars.insert("event.key".into(), Json::String(spec.name.into()));
            }
            hit
        }
        Trigger::MousePress { button } | Trigger::MouseRelease { button } => {
            let want = matches!(trigger, Trigger::MousePress { .. });
            let hit = events.iter().any(|event| {
                matches!(event, InputEvent::Mouse { button: pressed_button, pressed } if pressed_button == &(*button as i32) && *pressed == want)
            });
            if hit {
                vars.insert("event.button".into(), Json::from(*button));
            }
            hit
        }
        Trigger::Interval {
            every_ms,
            start_delay_ms,
        } => {
            if memory.next_ms == 0 {
                memory.next_ms = armed_ms + start_delay_ms;
            }
            if now_ms >= memory.next_ms.max(armed_ms) {
                memory.next_ms = now_ms + every_ms;
                true
            } else {
                false
            }
        }
        Trigger::AfterDelay { delay_ms } => {
            if !memory.fired && now_ms >= armed_ms + delay_ms {
                memory.fired = true;
                true
            } else {
                false
            }
        }
        Trigger::Condition {
            when,
            mode,
            every_ms,
        } => {
            let holds = conds::evaluate(when, &macro_.variables, snapshot);
            let fire = match mode {
                infered_macros::ConditionMode::Once => !memory.last && holds,
                infered_macros::ConditionMode::WhileTrue => {
                    if holds && now_ms >= memory.next_ms {
                        memory.next_ms = now_ms + every_ms;
                        true
                    } else {
                        false
                    }
                }
                infered_macros::ConditionMode::BecomesFalse => memory.last && !holds,
            };
            memory.last = holds;
            fire
        }
        Trigger::WorldLoaded => world_loaded,
         
         
        Trigger::WorldUnloaded => false,
        Trigger::PlayerJoined { name } => {
            if !previous.available {
                false
            } else {
                let joined = snapshot.players.iter().find(|player| {
                    name.as_ref()
                        .is_none_or(|filter| player.name.eq_ignore_ascii_case(filter))
                        && !contains_player(&previous.players, &player.name)
                });
                match joined {
                    Some(player) => {
                        vars.insert("event.player".into(), Json::String(player.name.clone()));
                        vars.insert("event.distance".into(), Json::from(player.distance));
                        true
                    }
                    None => false,
                }
            }
        }
        Trigger::PlayerLeft { name } => {
            if !previous.available {
                false
            } else {
                let left = previous.players.iter().find(|player| {
                    name.as_ref()
                        .is_none_or(|filter| player.name.eq_ignore_ascii_case(filter))
                        && !contains_player(&snapshot.players, &player.name)
                });
                match left {
                    Some(player) => {
                        vars.insert("event.player".into(), Json::String(player.name.clone()));
                        true
                    }
                    None => false,
                }
            }
        }
        Trigger::PlayerEnteredRange { range, name } => {
            if !previous.available {
                false
            } else {
                let now = nearest_matching(&snapshot.players, name);
                let before = nearest_matching(&previous.players, name);
                match now {
                    Some(player)
                        if player.distance <= *range
                            && before.is_none_or(|old| old.distance > *range) =>
                    {
                        vars.insert("event.player".into(), Json::String(player.name.clone()));
                        vars.insert("event.distance".into(), Json::from(player.distance));
                        true
                    }
                    _ => false,
                }
            }
        }
        Trigger::PlayerLeftRange { range, name } => {
            if !previous.available {
                false
            } else {
                let before = nearest_matching(&previous.players, name);
                let now = nearest_matching(&snapshot.players, name);
                match before {
                    Some(player)
                        if player.distance <= *range
                            && now.is_none_or(|current| current.distance > *range) =>
                    {
                        vars.insert("event.player".into(), Json::String(player.name.clone()));
                        vars.insert("event.distance".into(), Json::from(player.distance));
                        true
                    }
                    _ => false,
                }
            }
        }
        Trigger::PlayerDistanceChanged { by, name } => {
            if !previous.available {
                false
            } else {
                let changed = snapshot.players.iter().find_map(|player| {
                    if name
                        .as_ref()
                        .is_some_and(|filter| !player.name.eq_ignore_ascii_case(filter))
                    {
                        return None;
                    }
                    let before = previous
                        .players
                        .iter()
                        .find(|old| old.name.eq_ignore_ascii_case(&player.name))?;
                    if (player.distance - before.distance).abs() >= *by {
                        Some(player)
                    } else {
                        None
                    }
                });
                match changed {
                    Some(player) => {
                        vars.insert("event.player".into(), Json::String(player.name.clone()));
                        vars.insert("event.distance".into(), Json::from(player.distance));
                        true
                    }
                    None => false,
                }
            }
        }
        Trigger::HealthBelow { value } => {
             
             
            let low = snapshot.health < *value;
            let crossed = low && (!previous.available || previous.health >= *value);
            if crossed {
                vars.insert("event.value".into(), Json::from(snapshot.health));
            }
            crossed
        }
        Trigger::HungerBelow { value } => {
            let low = snapshot.food < *value;
            let crossed = low && (!previous.available || previous.food >= *value);
            if crossed {
                vars.insert("event.value".into(), Json::from(snapshot.food));
            }
            crossed
        }
        Trigger::SlotChanged => previous.available && previous.slot != snapshot.slot,
        Trigger::InventoryChanged => {
            previous.available && previous.inventory_hash != snapshot.inventory_hash
        }
        Trigger::EquipmentChanged => {
            previous.available && previous.equipment_hash != snapshot.equipment_hash
        }
        Trigger::LevelChanged => previous.available && previous.level != snapshot.level,
        Trigger::EffectGained { effect } | Trigger::EffectLost { effect } => {
            if !previous.available {
                false
            } else {
                let gained = matches!(trigger, Trigger::EffectGained { .. });
                let before: BTreeSet<&str> = previous
                    .effects
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect();
                let now: BTreeSet<&str> = snapshot
                    .effects
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect();
                let changed = if gained {
                    now.difference(&before).next().copied()
                } else {
                    before.difference(&now).next().copied()
                };
                let matched = changed.filter(|name| {
                    effect
                        .as_ref()
                        .is_none_or(|filter| filter.eq_ignore_ascii_case(name))
                });
                if let Some(name) = matched {
                    vars.insert("event.effect".into(), Json::String(name.to_string()));
                    true
                } else {
                    false
                }
            }
        }
        Trigger::ScreenOpened { screen } => {
            let opened = previous.screen.is_none() && snapshot.screen.is_some();
            let matches = screen_matches(*screen, snapshot.screen);
            if opened && matches {
                vars.insert(
                    "event.screen".into(),
                    Json::String(
                        snapshot
                            .screen
                            .map(ScreenName::as_str)
                            .unwrap_or("none")
                            .into(),
                    ),
                );
            }
            opened && matches
        }
        Trigger::ScreenClosed { screen } => {
            let closed = previous.screen.is_some() && snapshot.screen.is_none();
            let matches = screen_matches(*screen, previous.screen);
            if closed && matches {
                vars.insert(
                    "event.screen".into(),
                    Json::String(
                        previous
                            .screen
                            .map(ScreenName::as_str)
                            .unwrap_or("none")
                            .into(),
                    ),
                );
            }
            closed && matches
        }
        Trigger::StartedMoving => edge(previous, snapshot, |snapshot| snapshot.moving).is_some(),
        Trigger::StoppedMoving => edge(previous, snapshot, |snapshot| !snapshot.moving).is_some(),
        Trigger::EnteredWater => edge(previous, snapshot, |snapshot| snapshot.in_water).is_some(),
        Trigger::LeftWater => edge(previous, snapshot, |snapshot| !snapshot.in_water).is_some(),
        Trigger::StartedFalling => edge(previous, snapshot, |snapshot| snapshot.falling).is_some(),
        Trigger::Landed => edge(previous, snapshot, |snapshot| {
            !snapshot.falling && snapshot.on_ground
        })
        .is_some(),
    };
    fire.then_some(vars)
}

 
fn edge(
    previous: &Snapshot,
    snapshot: &Snapshot,
    predicate: impl Fn(&Snapshot) -> bool,
) -> Option<bool> {
    if !previous.available || !snapshot.available {
        return None;
    }
    (predicate(snapshot) && !predicate(previous)).then_some(true)
}

fn screen_matches(wanted: ScreenName, actual: Option<ScreenName>) -> bool {
    match (wanted, actual) {
        (ScreenName::Any, Some(_)) => true,
        (wanted, Some(actual)) => wanted == actual,
        (_, None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::test_host::FakeHost;
    use infered_macros::Condition;
    use infered_macros::{Action, Trigger};

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Scratch {
            let path = std::env::temp_dir().join(format!(
                "infered-engine-{label}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let _ = std::fs::remove_dir_all(&path);
            Scratch(path)
        }

        fn dir(&self) -> PathBuf {
            self.0.join("macros")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn engine_with(macros: Vec<Macro>, label: &str) -> (Engine, Scratch) {
        let scratch = Scratch::new(label);
        for macro_ in macros {
            save(&scratch.dir(), None, &macro_).unwrap();
        }
        let mut engine = Engine::new(scratch.dir());
        engine.set_policy(
            &mut FakeHost::default(),
            Policy {
                enabled: true,
                ..Policy::default()
            },
        );
        engine.reload(&mut FakeHost::default());
        (engine, scratch)
    }

    fn enter(engine: &mut Engine, host: &mut FakeHost) {
        engine.update_session(host, true, true);
    }

    fn macro_with(id: &str, triggers: Vec<Trigger>, actions: Vec<Action>) -> Macro {
        let mut macro_ = Macro::new_template(id, id.replace('_', " "));
        macro_.enabled = true;
        macro_.triggers = triggers;
        macro_.actions = actions;
        macro_
    }

    #[test]
    fn a_key_trigger_starts_a_run_with_its_event_value() {
        let macro_ = macro_with(
            "keyed",
            vec![Trigger::KeyPress { key: "F8".into() }],
            vec![Action::Chat {
                message: "hello".into(),
            }],
        );
        let (mut engine, _scratch) = engine_with(vec![macro_], "key");
        let mut host = FakeHost::default();
        enter(&mut engine, &mut host);
        engine.push_event(InputEvent::Key {
            glfw: 297,
            pressed: true,
        });
        engine.tick(&mut host);
        assert_eq!(host.chats, vec!["hello"]);
    }

    #[test]
    fn no_trigger_fires_without_its_conditions() {
        let mut macro_ = macro_with(
            "gated",
            vec![Trigger::KeyPress { key: "F8".into() }],
            vec![Action::Chat {
                message: "hello".into(),
            }],
        );
        macro_.conditions = vec![Condition::compare(
            infered_macros::ValueRef::state("health"),
            infered_macros::CompareOp::Lt,
            infered_macros::ValueRef::literal(Json::from(5.0)),
        )];
        let (mut engine, _scratch) = engine_with(vec![macro_], "gated");
        let mut host = FakeHost::default();
        host.snapshot.health = 20.0;
        enter(&mut engine, &mut host);
        engine.push_event(InputEvent::Key {
            glfw: 297,
            pressed: true,
        });
        engine.tick(&mut host);
        assert!(host.chats.is_empty());

        host.snapshot.health = 2.0;
        engine.push_event(InputEvent::Key {
            glfw: 297,
            pressed: true,
        });
        engine.tick(&mut host);
        assert_eq!(host.chats, vec!["hello"]);
    }

    #[test]
    fn a_health_threshold_fires_once_per_crossing() {
        let macro_ = macro_with(
            "low",
            vec![Trigger::HealthBelow { value: 10.0 }],
            vec![Action::Log {
                message: "low".into(),
            }],
        );
        let (mut engine, _scratch) = engine_with(vec![macro_], "health");
        let mut host = FakeHost::default();
        host.snapshot.health = 20.0;
        enter(&mut engine, &mut host);
        engine.tick(&mut host);
        assert!(host.logs.is_empty());

        host.snapshot.health = 8.0;
        engine.tick(&mut host);
        assert_eq!(host.logs, vec!["low: low"]);
        assert!(
            engine.running().is_empty(),
            "the log macro already finished"
        );

         
        engine.tick(&mut host);
         
        host.snapshot.health = 20.0;
        engine.tick(&mut host);
        host.snapshot.health = 9.0;
        engine.tick(&mut host);
        assert_eq!(host.logs.len(), 2);
    }

    #[test]
    fn an_interval_trigger_repeats_on_its_schedule() {
        let macro_ = macro_with(
            "tick",
            vec![Trigger::Interval {
                every_ms: 500,
                start_delay_ms: 0,
            }],
            vec![Action::Attack],
        );
        let (mut engine, _scratch) = engine_with(vec![macro_], "interval");
        let mut host = FakeHost::default();
        enter(&mut engine, &mut host);
        for _ in 0..40 {
            engine.tick(&mut host);
        }
         
         
        assert!((3..=5).contains(&host.attacks), "{} attacks", host.attacks);
    }

    #[test]
    fn a_player_entering_range_passes_the_name_to_the_run() {
        let macro_ = macro_with(
            "near",
            vec![Trigger::PlayerEnteredRange {
                range: 8.0,
                name: None,
            }],
            vec![Action::Chat {
                message: "hi".into(),
            }],
        );
        let (mut engine, _scratch) = engine_with(vec![macro_], "range");
        let mut host = FakeHost::default();
        host.snapshot.players = Arc::new(vec![crate::macros::host::PlayerSample {
            name: "Alex".into(),
            distance: 40.0,
        }]);
        enter(&mut engine, &mut host);
        engine.tick(&mut host);
        assert!(host.chats.is_empty());

        host.snapshot.players = Arc::new(vec![crate::macros::host::PlayerSample {
            name: "Alex".into(),
            distance: 5.0,
        }]);
        engine.tick(&mut host);
        assert_eq!(host.chats, vec!["hi"]);
    }

    #[test]
    fn leaving_the_world_stops_runs_and_releases_inputs() {
        let macro_ = macro_with(
            "walk",
            vec![Trigger::Manual],
            vec![
                Action::Movement {
                    key: infered_macros::MovementKey::Forward,
                    action: infered_macros::KeyAction::Press,
                    hold_ms: 0,
                },
                Action::Wait { duration_ms: 5_000 },
            ],
        );
        let mut macro_ = macro_;
        macro_.key = Some("F8".into());
        let (mut engine, _scratch) = engine_with(vec![macro_], "leave");
        let mut host = FakeHost::default();
        enter(&mut engine, &mut host);
        engine.push_event(InputEvent::Key {
            glfw: 297,
            pressed: true,
        });
        engine.tick(&mut host);
        assert_eq!(
            host.held,
            vec![(infered_macros::MovementKey::Forward, true)]
        );

        engine.update_session(&mut host, false, false);
        assert!(engine.running().is_empty());
        assert_eq!(
            host.held.last(),
            Some(&(infered_macros::MovementKey::Forward, false))
        );
        assert!(engine.held_inputs().is_empty());
    }

    #[test]
    fn the_screen_switch_stops_macros_in_menus() {
        let macro_ = macro_with(
            "walk",
            vec![Trigger::Interval {
                every_ms: 100,
                start_delay_ms: 0,
            }],
            vec![Action::Wait { duration_ms: 2_000 }],
        );
        let (mut engine, _scratch) = engine_with(vec![macro_], "screen");
        let mut host = FakeHost::default();
        enter(&mut engine, &mut host);
        engine.tick(&mut host);
        assert_eq!(engine.running().len(), 1);

        host.snapshot.screen = Some(ScreenName::Inventory);
        engine.tick(&mut host);
        assert!(engine.running().is_empty());
    }

    #[test]
    fn disabling_a_macro_on_disk_stops_and_removes_it() {
        let mut macro_ = macro_with(
            "long",
            vec![Trigger::Interval {
                every_ms: 100,
                start_delay_ms: 0,
            }],
            vec![Action::Wait { duration_ms: 5_000 }],
        );
        let (mut engine, scratch) = engine_with(vec![macro_.clone()], "disable");
        let mut host = FakeHost::default();
        enter(&mut engine, &mut host);
        engine.tick(&mut host);
        assert_eq!(engine.running().len(), 1);

        macro_.enabled = false;
        save(&scratch.dir(), Some("long.macro"), &macro_).unwrap();
        engine.invalidate_scan();
        engine.reload(&mut host);
        assert!(engine.running().is_empty());
        assert!(engine.controls().iter().all(|control| !control.enabled));
    }

    #[test]
    fn the_engine_can_toggle_a_macro_and_writes_the_file() {
        let macro_ = macro_with("toggled", vec![Trigger::Manual], vec![Action::Stop]);
        let (mut engine, scratch) = engine_with(vec![macro_], "toggle");
        let mut host = FakeHost::default();
        engine
            .set_macro_enabled(&mut host, "toggled", false)
            .unwrap();
        assert!(
            !infered_macros::read(&scratch.dir(), "toggled.macro")
                .unwrap()
                .enabled
        );
        assert!(engine.controls()[0].enabled == false);
        engine
            .set_macro_enabled(&mut host, "toggled", true)
            .unwrap();
        assert!(
            infered_macros::read(&scratch.dir(), "toggled.macro")
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn two_macros_cannot_fight_over_one_input() {
        let first = macro_with(
            "first",
            vec![Trigger::Interval {
                every_ms: 100,
                start_delay_ms: 0,
            }],
            vec![
                Action::Movement {
                    key: infered_macros::MovementKey::Forward,
                    action: infered_macros::KeyAction::Press,
                    hold_ms: 0,
                },
                Action::Wait { duration_ms: 5_000 },
            ],
        );
        let second = macro_with(
            "second",
            vec![Trigger::Interval {
                every_ms: 200,
                start_delay_ms: 50,
            }],
            vec![
                Action::Movement {
                    key: infered_macros::MovementKey::Forward,
                    action: infered_macros::KeyAction::Press,
                    hold_ms: 0,
                },
                Action::Wait { duration_ms: 5_000 },
            ],
        );
        let (mut engine, _scratch) = engine_with(vec![first, second], "conflict");
        let mut host = FakeHost::default();
        enter(&mut engine, &mut host);
        engine.tick(&mut host);
        engine.tick(&mut host);
        assert_eq!(engine.running().len(), 1, "the second run must stand down");
        assert!(engine.last_error.is_some());
    }

    #[test]
    fn stop_all_ends_every_run() {
        let one = macro_with(
            "one",
            vec![Trigger::KeyPress { key: "F8".into() }],
            vec![Action::Wait { duration_ms: 5_000 }],
        );
        let two = macro_with(
            "two",
            vec![Trigger::KeyPress { key: "F9".into() }],
            vec![Action::StopAll],
        );
        let (mut engine, _scratch) = engine_with(vec![one, two], "stopall");
        let mut host = FakeHost::default();
        enter(&mut engine, &mut host);
        engine.push_event(InputEvent::Key {
            glfw: 297,
            pressed: true,
        });
        engine.tick(&mut host);
        assert_eq!(engine.running().len(), 1);
        engine.push_event(InputEvent::Key {
            glfw: 298,
            pressed: true,
        });
        engine.tick(&mut host);
        assert!(engine.running().is_empty());
    }

    #[test]
    fn a_broken_file_leaves_the_rest_of_the_library_working() {
        let macro_ = macro_with("good", vec![Trigger::Manual], vec![Action::Stop]);
        let (mut engine, scratch) = engine_with(vec![macro_], "broken");
        std::fs::write(scratch.dir().join("bad.macro"), "{ nope").unwrap();
        engine.invalidate_scan();
        engine.reload(&mut FakeHost::default());
        assert_eq!(engine.macro_count(), 1);
        assert_eq!(engine.broken_files(), 1);
    }

    #[test]
    fn policy_off_stops_everything() {
        let macro_ = macro_with(
            "long",
            vec![Trigger::Manual],
            vec![Action::Wait { duration_ms: 5_000 }],
        );
        let mut macro_ = macro_;
        macro_.key = Some("F8".into());
        let (mut engine, _scratch) = engine_with(vec![macro_], "policy");
        let mut host = FakeHost::default();
        enter(&mut engine, &mut host);
        engine.push_event(InputEvent::Key {
            glfw: 297,
            pressed: true,
        });
        engine.tick(&mut host);
        assert_eq!(engine.running().len(), 1);
        engine.set_policy(
            &mut host,
            Policy {
                enabled: false,
                ..Policy::default()
            },
        );
        assert!(engine.running().is_empty());
    }

    #[test]
    fn world_unloaded_never_starts_a_run() {
        let macro_ = macro_with(
            "bye",
            vec![Trigger::WorldUnloaded],
            vec![Action::Chat {
                message: "bye".into(),
            }],
        );
        let (mut engine, _scratch) = engine_with(vec![macro_], "unload");
        let mut host = FakeHost::default();
        enter(&mut engine, &mut host);
        engine.tick(&mut host);
        engine.update_session(&mut host, false, false);
        engine.tick(&mut host);
        assert!(host.chats.is_empty());
    }
}
