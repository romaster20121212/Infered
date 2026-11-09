 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::entity::EntityRef;
use crate::mapping::{FieldType, MinecraftClassType};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::{mapping, minecraft};
use jni::objects::{GlobalRef, JString};
use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

 
pub const ROSTER_FILE: &str = "infered_roster.txt";
 
 
pub const MAX_NAME: usize = 32;
 
pub const MAX_SERVER: usize = 96;

 
const REFRESH_INTERVAL: Duration = Duration::from_secs(1);
 
const BEACON_INTERVAL: Duration = Duration::from_secs(3);
 
const PRESENCE_TTL: Duration = Duration::from_secs(10);
 
const RECV_POLL: Duration = Duration::from_millis(250);
 
const IDLE_POLL: Duration = Duration::from_millis(250);
 
const RETRY_DELAY: Duration = Duration::from_secs(5);

 
 
const GROUP: Ipv4Addr = Ipv4Addr::new(239, 255, 77, 31);
const DISCOVERY_PORT: u16 = 47831;

 
 
const WIRE_PREFIX: &str = "INFERED/1";

 
const ROSTER_TEMPLATE: &str = "\
# Infered Tag roster
#
# One player name per line. Players listed here get the Infered logo beside
# their nametag. Matching is case-insensitive; blank lines and lines starting
# with '#' are ignored.
#
# This file stays on your computer. Nothing about it is ever sent to the
# Minecraft server. Players on your local network are detected automatically
# and do not need to be listed here.
";

 
 
static SHOW_LOGO: AtomicBool = AtomicBool::new(false);
 
static DETECT: AtomicBool = AtomicBool::new(false);
 
static BROADCAST: AtomicBool = AtomicBool::new(false);
 
static RUNNING: AtomicBool = AtomicBool::new(false);
 
 
static WORKER: AtomicBool = AtomicBool::new(false);

 
#[derive(Clone, Debug, PartialEq, Eq)]
struct Presence {
     
    server: String,
     
    name: String,
}

 
 
#[derive(Debug, Default)]
struct RosterState {
     
    file_names: HashSet<String>,
     
    signature: Option<Signature>,
     
     
    loaded: bool,
     
    checked: Option<Instant>,
     
    peers: HashMap<String, Instant>,
     
    presence: Option<Presence>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Signature {
    modified: Option<SystemTime>,
    length: u64,
}

static ROSTER: LazyLock<Mutex<RosterState>> = LazyLock::new(|| Mutex::new(RosterState::default()));

fn lock() -> MutexGuard<'static, RosterState> {
    ROSTER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
fn instance_id() -> u64 {
    static ID: LazyLock<u64> = LazyLock::new(|| {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs() ^ u64::from(elapsed.subsec_nanos()))
            .unwrap_or(0);
        (u64::from(std::process::id()) << 32) ^ nanos
    });
    *ID
}

 

 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Beacon,
    Reply,
}

impl Kind {
    fn field(self) -> &'static str {
        match self {
            Kind::Beacon => "B",
            Kind::Reply => "R",
        }
    }

    fn parse(field: &str) -> Option<Self> {
        match field {
            "B" => Some(Kind::Beacon),
            "R" => Some(Kind::Reply),
            _ => None,
        }
    }
}

 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Wire<'a> {
    kind: Kind,
    instance: u64,
    server: &'a str,
    name: &'a str,
}

 
 
fn valid_field(text: &str, max: usize) -> bool {
    !text.is_empty()
        && text.chars().count() <= max
        && !text.chars().any(|c| c.is_control() || c == '\t')
}

 
fn encode_wire(kind: Kind, instance: u64, server: &str, name: &str) -> Option<String> {
    if !valid_field(server, MAX_SERVER) || !valid_field(name, MAX_NAME) {
        return None;
    }
    Some(format!(
        "{WIRE_PREFIX}\t{}\t{instance:016x}\t{server}\t{name}",
        kind.field()
    ))
}

 
 
fn parse_wire(text: &str) -> Option<Wire<'_>> {
    let mut fields = text.split('\t');
    if fields.next()? != WIRE_PREFIX {
        return None;
    }
    let kind = Kind::parse(fields.next()?)?;
    let instance = u64::from_str_radix(fields.next()?, 16).ok()?;
    let server = fields.next()?;
    let name = fields.next()?;
    if fields.next().is_some() {
        return None;
    }
    if !valid_field(server, MAX_SERVER) || !valid_field(name, MAX_NAME) {
        return None;
    }
    Some(Wire {
        kind,
        instance,
        server,
        name,
    })
}

 
 
fn normalize_server(address: &str) -> String {
    let mut text = address.trim().to_ascii_lowercase();
    if text.ends_with(":25565") {
        text.truncate(text.len() - ":25565".len());
    }
    while text.ends_with('.') {
        text.pop();
    }
    text
}

 

 
 
#[cfg(test)]
static ROSTER_PATH_OVERRIDE: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));

 
pub fn roster_path() -> PathBuf {
    #[cfg(test)]
    if let Some(path) = ROSTER_PATH_OVERRIDE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
    {
        return path;
    }
    crate::config::base_dir().join(ROSTER_FILE)
}

 
 
 
 
 
fn parse_roster(text: &str) -> HashSet<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter(|line| line.chars().count() <= MAX_NAME)
        .map(|line| line.to_lowercase())
        .collect()
}

fn signature_of(path: &Path) -> Option<Signature> {
    let meta = std::fs::metadata(path).ok()?;
    Some(Signature {
        modified: meta.modified().ok(),
        length: meta.len(),
    })
}

 
 
 
 
 
fn refresh_from(path: &Path, state: &mut RosterState) -> bool {
    let signature = signature_of(path);
    if state.loaded && signature == state.signature {
        return false;
    }
    let names = std::fs::read_to_string(path)
        .map(|text| parse_roster(&text))
        .unwrap_or_default();
    state.file_names = names;
    state.signature = signature;
    state.loaded = true;
    true
}

 
 
 
fn learn(state: &mut RosterState, server: &str, name: &str, now: Instant) -> bool {
    let Some(presence) = state.presence.as_ref() else {
        return false;
    };
    if !presence.server.eq_ignore_ascii_case(server) {
        return false;
    }
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return false;
    }
    let name = name.to_lowercase();
    let fresh = state
        .peers
        .insert(name.clone(), now + PRESENCE_TTL)
        .is_none_or(|previous| previous <= now);
    if fresh {
        log::info!("[InferedTag] detected {name} on {server}");
    }
    fresh
}

 
fn expire(state: &mut RosterState, now: Instant) -> bool {
    if !state.peers.values().any(|deadline| *deadline <= now) {
        return false;
    }
    state.peers.retain(|name, deadline| {
        let alive = *deadline > now;
        if !alive {
            log::info!("[InferedTag] lost {name}: no beacon for {PRESENCE_TTL:?}");
        }
        alive
    });
    true
}

 
 
pub fn ensure_roster_file() {
    let path = roster_path();
    if path.exists() {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(error) = std::fs::write(&path, ROSTER_TEMPLATE) {
        log::debug!("[InferedTag] could not create {}: {error}", path.display());
    }
}

 
 
fn refresh(force: bool) {
    let now = Instant::now();
    {
        let mut state = lock();
        if !force
            && state
                .checked
                .is_some_and(|checked| now.saturating_duration_since(checked) < REFRESH_INTERVAL)
        {
            return;
        }
        state.checked = Some(now);
        refresh_from(&roster_path(), &mut state);
        expire(&mut state, now);
    }
}

 
fn clear() {
    let mut state = lock();
    state.file_names.clear();
    state.signature = None;
    state.loaded = false;
    state.checked = None;
    state.peers.clear();
    state.presence = None;
}

 
 
pub fn is_infered(name: &str) -> bool {
    if !SHOW_LOGO.load(Ordering::Relaxed) {
        return false;
    }
    let state = lock();
    if state.file_names.is_empty() && state.peers.is_empty() {
        return false;
    }
    let name = name.trim().to_lowercase();
    if state.file_names.contains(&name) {
        return true;
    }
    state
        .peers
        .get(&name)
        .is_some_and(|deadline| *deadline > Instant::now())
}

 

 
fn current_server_address() -> Option<String> {
    let server: GlobalRef = minecraft().current_server().ok()??;
    mapping()
        .in_frame(|| -> anyhow::Result<String> {
            let ip = mapping()
                .get_field(
                    MinecraftClassType::ServerData,
                    server.as_obj(),
                    "ip",
                    FieldType::String,
                )?
                .l()?;
            let ip = JString::from(ip);
            let text = mapping().get_env()?.get_string(&ip)?;
            Ok(String::from(text))
        })
        .ok()
}

 
 
fn current_presence() -> Option<Presence> {
    let minecraft = minecraft();
    if !minecraft.in_world() {
        return None;
    }
    let server = normalize_server(&current_server_address()?);
    if server.is_empty() {
        return None;
    }
    let name = minecraft
        .player()
        .ok()
        .flatten()?
        .get_name()
        .ok()?
        .get_string()
        .ok()?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some(Presence { server, name })
}

 
 
fn publish_presence() {
    let presence = current_presence();
    let mut state = lock();
    let server_changed = match (state.presence.as_ref(), presence.as_ref()) {
        (Some(old), Some(new)) => !old.server.eq_ignore_ascii_case(&new.server),
        (Some(_), None) => true,
        _ => false,
    };
    if server_changed {
        state.peers.clear();
    }
     
     
    let announce = presence.as_ref().is_some_and(|new| {
        state
            .presence
            .as_ref()
            .is_none_or(|old| !old.server.eq_ignore_ascii_case(&new.server))
    });
    if announce {
        if let Some(new) = presence.as_ref() {
            log::info!(
                "[InferedTag] advertising {} on {}, waiting for peers",
                new.name,
                new.server
            );
        }
    }
    state.presence = presence;
}

 

 
fn beacon() -> Option<String> {
    let state = lock();
    let presence = state.presence.as_ref()?;
    encode_wire(
        Kind::Beacon,
        instance_id(),
        &presence.server,
        &presence.name,
    )
}

 
 
fn roster_replies() -> Vec<String> {
    let state = lock();
    let Some(presence) = state.presence.as_ref() else {
        return Vec::new();
    };
    let now = Instant::now();
    let mut names: Vec<&str> = Vec::new();
    names.extend(state.file_names.iter().map(String::as_str));
    names.extend(
        state
            .peers
            .iter()
            .filter(|(_, deadline)| **deadline > now)
            .map(|(name, _)| name.as_str()),
    );
    names.sort_unstable();
    names.dedup();
    names.truncate(63);
     
     
    names.insert(0, presence.name.as_str());
    names
        .into_iter()
        .filter_map(|name| encode_wire(Kind::Reply, instance_id(), &presence.server, name))
        .collect()
}

 
 
fn accept(text: &str, from: SocketAddr, socket: &UdpSocket) {
    if !DETECT.load(Ordering::Relaxed) {
        return;
    }
    let Some(wire) = parse_wire(text) else {
        return;
    };
    if wire.instance == instance_id() {
        return;
    }
    let now = Instant::now();
    {
        let mut state = lock();
        learn(&mut state, wire.server, wire.name, now);
    }
    if wire.kind != Kind::Beacon || !BROADCAST.load(Ordering::Relaxed) {
        return;
    }
    for reply in roster_replies() {
        if socket.send_to(reply.as_bytes(), from).is_err() {
            break;
        }
    }
}

 
 
 
 
 
 
 
fn open_socket() -> std::io::Result<UdpSocket> {
    let primary = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, DISCOVERY_PORT)).and_then(|socket| {
        socket.join_multicast_v4(&GROUP, &Ipv4Addr::UNSPECIFIED)?;
        socket.set_multicast_loop_v4(true)?;
        socket.set_multicast_ttl_v4(1)?;
        Ok(socket)
    });
    let socket = match primary {
        Ok(socket) => socket,
        Err(error) => {
            log::debug!("[InferedTag] rendezvous port busy ({error}); using an ephemeral port");
            UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?
        }
    };
     
     
    let _ = socket.set_multicast_ttl_v4(1);
    socket.set_read_timeout(Some(RECV_POLL))?;
    Ok(socket)
}

 
 
 
fn pump(socket: &UdpSocket) -> bool {
    let target = SocketAddrV4::new(GROUP, DISCOVERY_PORT);
    let mut buf = [0u8; 1024];
    let mut last_beacon: Option<Instant> = None;
    loop {
        if !RUNNING.load(Ordering::Relaxed) {
            return true;
        }
        let now = Instant::now();
        if BROADCAST.load(Ordering::Relaxed)
            && last_beacon.is_none_or(|sent| now.duration_since(sent) >= BEACON_INTERVAL)
        {
            if let Some(text) = beacon() {
                if let Err(error) = socket.send_to(text.as_bytes(), target) {
                    log::debug!("[InferedTag] beacon send failed: {error}");
                    return false;
                }
            }
            last_beacon = Some(now);
        }
        match socket.recv_from(&mut buf) {
            Ok((len, from)) => {
                if let Ok(text) = std::str::from_utf8(&buf[..len]) {
                    accept(text, from, socket);
                }
            }
            Err(error) => match error.kind() {
                ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted => {}
                _ => {
                    log::debug!("[InferedTag] beacon receive failed: {error}");
                    return false;
                }
            },
        }
    }
}

 
 
fn worker() {
    let mut socket: Option<UdpSocket> = None;
    let mut logged: Option<String> = None;
    loop {
        if !RUNNING.load(Ordering::Relaxed) {
            socket = None;
            std::thread::sleep(IDLE_POLL);
            continue;
        }
        if socket.is_none() {
            match open_socket() {
                Ok(opened) => {
                    logged = None;
                    socket = Some(opened);
                }
                Err(error) => {
                     
                     
                    let text = error.to_string();
                    if logged.as_deref() != Some(text.as_str()) {
                        log::debug!("[InferedTag] LAN discovery unavailable: {text}");
                        logged = Some(text);
                    }
                    std::thread::sleep(RETRY_DELAY);
                    continue;
                }
            }
        }
        if let Some(open) = socket.as_ref() {
            if !pump(open) {
                socket = None;
            }
        }
    }
}

 
fn ensure_worker() {
    if WORKER.swap(true, Ordering::SeqCst) {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("infered-tag-presence".to_string())
        .spawn(worker);
    if let Err(error) = spawned {
        WORKER.store(false, Ordering::SeqCst);
        log::debug!("[InferedTag] could not start the LAN discovery worker: {error}");
    }
}

 
 
#[derive(Debug)]
pub struct InferedTagModule {
    pub module: ModuleData,
}

impl InferedTagModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::InferedTag,
                description: "Draws the Infered logo beside other Infered players found on your \
                              local network. Detection uses a LAN-only beacon: nothing is sent \
                              to the Minecraft server and the client brand stays vanilla. Names \
                              in infered_roster.txt are badged too."
                    .into(),
                category: ModuleCategory::Visuals,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![
                    ModuleSetting::Toggle {
                        name: "Show Logo".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Auto-Detect".to_string(),
                        value: true,
                    },
                    ModuleSetting::Toggle {
                        name: "Broadcast".to_string(),
                        value: true,
                    },
                ],
            },
        }
    }

    fn toggle(&self, name: &str, fallback: bool) -> bool {
        self.module
            .get_setting(name)
            .and_then(ModuleSetting::get_toggle_value)
            .unwrap_or(fallback)
    }

    fn show_logo(&self) -> bool {
        self.toggle("Show Logo", true)
    }
}

impl Module for InferedTagModule {
    fn on_start(&self) -> anyhow::Result<()> {
        SHOW_LOGO.store(self.show_logo(), Ordering::Relaxed);
        DETECT.store(self.toggle("Auto-Detect", true), Ordering::Relaxed);
        BROADCAST.store(self.toggle("Broadcast", true), Ordering::Relaxed);
        ensure_roster_file();
        clear();
        refresh(true);
        publish_presence();
        ensure_worker();
        RUNNING.store(true, Ordering::SeqCst);
        log::info!("[InferedTag] enabled: listening for Infered beacons on the local network");
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        RUNNING.store(false, Ordering::SeqCst);
        SHOW_LOGO.store(false, Ordering::Relaxed);
        DETECT.store(false, Ordering::Relaxed);
        BROADCAST.store(false, Ordering::Relaxed);
        clear();
        Ok(())
    }

    fn on_world_inactive(&self) -> anyhow::Result<()> {
         
         
         
        SHOW_LOGO.store(false, Ordering::Relaxed);
        RUNNING.store(false, Ordering::SeqCst);
        clear();
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        SHOW_LOGO.store(self.show_logo(), Ordering::Relaxed);
        DETECT.store(self.toggle("Auto-Detect", true), Ordering::Relaxed);
        BROADCAST.store(self.toggle("Broadcast", true), Ordering::Relaxed);
        RUNNING.store(true, Ordering::SeqCst);
        ensure_worker();
        publish_presence();
        refresh(false);
        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}

#[cfg(test)]
mod tests {
    use super::*;

     
    struct ScratchDir(PathBuf);

    impl ScratchDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "infered_roster_{name}_{}_{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|elapsed| elapsed.subsec_nanos())
                    .unwrap_or(0)
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

     
     
    fn globals() -> MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn presence(server: &str, name: &str) -> Presence {
        Presence {
            server: server.to_string(),
            name: name.to_string(),
        }
    }

     
     
    #[test]
    fn the_roster_format_ignores_comments_and_normalises_names() {
        let names = parse_roster(
            "\
# header
Notch

   alex
\tSTEVE\t
# trailing comment
玩家
",
        );
        assert_eq!(names.len(), 4);
        assert!(names.contains("notch"));
        assert!(names.contains("alex"));
        assert!(names.contains("steve"));
        assert!(names.contains("玩家"));
         
         
        assert!(!names.contains("Notch"));
    }

     
     
    #[test]
    fn over_long_names_are_skipped() {
        let long = "a".repeat(MAX_NAME + 1);
        let exact = "b".repeat(MAX_NAME);
        let names = parse_roster(&format!("{long}\n{exact}\n"));
        assert!(!names.contains(&long));
        assert!(names.contains(&exact));
        assert!(parse_roster("").is_empty());
    }

     
     
    #[test]
    fn refreshing_follows_the_file() {
        let scratch = ScratchDir::new("refresh");
        let path = scratch.0.join(ROSTER_FILE);
        let mut state = RosterState::default();

         
        assert!(refresh_from(&path, &mut state));
        assert!(state.file_names.is_empty());

        std::fs::write(&path, "Notch\n").unwrap();
        assert!(refresh_from(&path, &mut state));
        assert!(state.file_names.contains("notch"));

         
        assert!(!refresh_from(&path, &mut state), "no edit, no reload");

         
        std::fs::write(&path, "alex\n").unwrap();
        assert!(refresh_from(&path, &mut state));
        assert!(!state.file_names.contains("notch"));
        assert!(state.file_names.contains("alex"));
    }

     
     
    #[test]
    fn beacons_round_trip() {
        let text = encode_wire(Kind::Beacon, 0x1234_abcd, "play.example.net", "玩家").unwrap();
        let wire = parse_wire(&text).unwrap();
        assert_eq!(wire.kind, Kind::Beacon);
        assert_eq!(wire.instance, 0x1234_abcd);
        assert_eq!(wire.server, "play.example.net");
        assert_eq!(wire.name, "玩家");

        let reply = encode_wire(Kind::Reply, 1, "example.net", "Notch").unwrap();
        assert_eq!(parse_wire(&reply).unwrap().kind, Kind::Reply);
    }

     
     
    #[test]
    fn malformed_datagrams_are_rejected() {
        assert!(parse_wire("").is_none());
        assert!(parse_wire("INFERED/1").is_none());
        assert!(parse_wire("INFERED/2\tB\t0000000000000001\tsrv\tname").is_none());
        assert!(parse_wire("INFERED/1\tX\t0000000000000001\tsrv\tname").is_none());
        assert!(parse_wire("INFERED/1\tB\tzzzz\tsrv\tname").is_none());
        assert!(parse_wire("INFERED/1\tB\t0000000000000001\tsrv").is_none());
        assert!(parse_wire("INFERED/1\tB\t0000000000000001\tsrv\tname\textra").is_none());
        assert!(parse_wire("INFERED/1\tB\t0000000000000001\t\tsrv\tname").is_none());
         
        assert!(parse_wire("INFERED/1\tB\t0000000000000001\tsrv\tna\nme").is_none());
         
        let long = "a".repeat(MAX_NAME + 1);
        assert!(encode_wire(Kind::Beacon, 1, "srv", &long).is_none());
    }

     
     
    #[test]
    fn server_keys_are_normalised() {
        assert_eq!(normalize_server("Example.NET"), "example.net");
        assert_eq!(normalize_server(" example.net:25565 "), "example.net");
        assert_eq!(normalize_server("example.net."), "example.net");
        assert_eq!(normalize_server("example.net:25566"), "example.net:25566");
        assert_ne!(
            normalize_server("example.net:25566"),
            normalize_server("example.net")
        );
    }

     
     
    #[test]
    fn peers_are_keyed_by_server_and_expire() {
        let now = Instant::now();
        let mut state = RosterState {
            presence: Some(presence("example.net", "me")),
            ..RosterState::default()
        };

        assert!(learn(&mut state, "Example.NET", "Friend", now));
        assert!(state.peers.contains_key("friend"), "names are lowercased");
        assert!(!learn(&mut state, "other.net", "Stranger", now));
        assert!(!state.peers.contains_key("stranger"));
        assert!(!learn(&mut state, "example.net", "   ", now));

         
        let deadline = *state.peers.get("friend").unwrap();
        assert!(!expire(&mut state, deadline - Duration::from_millis(1)));
        assert!(expire(&mut state, deadline + Duration::from_millis(1)));
        assert!(state.peers.is_empty());
    }

     
    #[test]
    fn a_heartbeat_keeps_a_peer_alive() {
        let now = Instant::now();
        let mut state = RosterState {
            presence: Some(presence("example.net", "me")),
            ..RosterState::default()
        };
        learn(&mut state, "example.net", "Friend", now);
        let first = *state.peers.get("friend").unwrap();
        learn(
            &mut state,
            "example.net",
            "Friend",
            now + Duration::from_secs(5),
        );
        let second = *state.peers.get("friend").unwrap();
        assert!(second > first);
    }

     
    #[test]
    fn no_beacon_without_a_server() {
        let _guard = globals();
        {
            let mut state = lock();
            state.presence = None;
            state.peers.clear();
            state.file_names.clear();
        }
        assert!(beacon().is_none());
        assert!(roster_replies().is_empty());
        clear();
    }

     
     
    #[test]
    fn the_badge_follows_the_roster_and_the_switch() {
        let _guard = globals();
        {
            let mut state = lock();
            state.file_names = parse_roster("Notch\n");
            state.presence = Some(presence("example.net", "me"));
            state.peers.clear();
            learn(&mut state, "example.net", "Alex", Instant::now());
        }
        SHOW_LOGO.store(true, Ordering::Relaxed);
        assert!(is_infered("Notch"), "roster file names count");
        assert!(is_infered("alex"), "discovered peers count");
        assert!(is_infered("ALEX"));
        assert!(!is_infered("Steve"));

        SHOW_LOGO.store(false, Ordering::Relaxed);
        assert!(!is_infered("Notch"), "the switch hides every badge");
        assert!(!is_infered("alex"));
        clear();
    }

     
    #[test]
    fn expired_peers_stop_being_badged() {
        let _guard = globals();
        {
            let mut state = lock();
            state.file_names.clear();
            state.presence = Some(presence("example.net", "me"));
            state
                .peers
                .insert("alex".to_string(), Instant::now() - Duration::from_secs(1));
        }
        SHOW_LOGO.store(true, Ordering::Relaxed);
        assert!(!is_infered("alex"));
        clear();
    }

     
     
     
    #[test]
    fn the_worker_learns_a_beacon_and_answers_with_the_roster() {
        let _guard = globals();
         
         
         
        let scratch = ScratchDir::new("worker");
        let roster = scratch.0.join(ROSTER_FILE);
        std::fs::write(&roster, "# seeded by the test\nNotch\n").unwrap();
        *ROSTER_PATH_OVERRIDE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(roster);
        {
            let mut state = lock();
            state.file_names.clear();
            state.signature = None;
            state.loaded = false;
            state.peers.clear();
            state.presence = Some(presence("example.net", "me"));
        }
         
         
        refresh(true);
        SHOW_LOGO.store(true, Ordering::Relaxed);
        DETECT.store(true, Ordering::Relaxed);
        BROADCAST.store(true, Ordering::Relaxed);
        RUNNING.store(true, Ordering::SeqCst);
        ensure_worker();

        let Ok(peer) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) else {
            RUNNING.store(false, Ordering::SeqCst);
            return;
        };
        peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let beacon = encode_wire(Kind::Beacon, 0xDEAD_BEEF_u64, "example.net", "Friend")
            .expect("encode a beacon");
        let target = SocketAddrV4::new(GROUP, DISCOVERY_PORT);
        if peer.send_to(beacon.as_bytes(), target).is_err() {
            RUNNING.store(false, Ordering::SeqCst);
            return;
        }

         
         
         
         
        let mut buf = [0u8; 1024];
        let mut replied: Vec<String> = Vec::new();
        'announce: for _ in 0..25 {
            let _ = peer.send_to(beacon.as_bytes(), target);
            peer.set_read_timeout(Some(Duration::from_millis(100)))
                .unwrap();
            for _ in 0..4 {
                match peer.recv_from(&mut buf) {
                    Ok((len, _)) => {
                        let reply = std::str::from_utf8(&buf[..len]).unwrap();
                        let wire = parse_wire(reply).expect("the reply is a roster entry");
                        assert_eq!(wire.kind, Kind::Reply);
                         
                         
                         
                        if wire.instance != instance_id() {
                            continue;
                        }
                        replied.push(wire.name.to_string());
                        if replied.iter().any(|name| name == "notch") {
                            break 'announce;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
        if replied.is_empty() {
             
             
             
            eprintln!("skipped: no beacon reached the worker");
        } else {
            assert!(
                replied.iter().any(|name| name == "notch"),
                "the roster file is shared: {replied:?}"
            );
            assert!(
                is_infered("friend"),
                "the worker learns the player from the beacon"
            );
        }

        RUNNING.store(false, Ordering::SeqCst);
        SHOW_LOGO.store(false, Ordering::Relaxed);
        DETECT.store(false, Ordering::Relaxed);
        BROADCAST.store(false, Ordering::Relaxed);
        clear();
        *ROSTER_PATH_OVERRIDE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }

     
     
    #[test]
    fn the_module_is_local_only() {
        let module = InferedTagModule::new();
        assert_eq!(module.module.id, ModuleId::InferedTag);
        assert!(!module.module.enabled);
        assert!(module.show_logo());
        assert!(module.toggle("Auto-Detect", true));
        assert!(module.toggle("Broadcast", true));
        let settings: Vec<&str> = module
            .module
            .settings
            .iter()
            .map(ModuleSetting::name)
            .collect();
        assert_eq!(settings, vec!["Show Logo", "Auto-Detect", "Broadcast"]);
        assert!(module.module.description.contains("LAN-only"));
        assert!(module.module.description.contains("nothing is sent"));
    }
}
