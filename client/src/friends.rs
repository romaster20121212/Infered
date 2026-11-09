 
 
 
 
 
 
 
 
 
 
 
 
 
 

use crate::mapping::entity::player::Player;
use crate::mapping::entity::{Entity, EntityRef};
use crate::mapping::MappedObject;
use crate::module::ModuleId;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

 
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum FriendMode {
     
    #[default]
    Easy,
     
    Advanced,
}

 
 
 
 
 
pub const PLAYER_MODULES: [ModuleId; 10] = [
    ModuleId::KillAura,
    ModuleId::SilentAura,
    ModuleId::Aimbot,
    ModuleId::TriggerBot,
    ModuleId::AutoMace,
    ModuleId::SilentMace,
    ModuleId::SilentAssist,
    ModuleId::HitSelect,
    ModuleId::ShieldBreaker,
    ModuleId::AutoCobweb,
];

 
pub fn module_affects_players(module: ModuleId) -> bool {
    PLAYER_MODULES.contains(&module)
}

 
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Friend {
    pub name: String,
    #[serde(default)]
    pub mode: FriendMode,
     
    #[serde(default = "default_protected")]
    pub protected: bool,
     
    #[serde(default)]
    pub exempt: Vec<ModuleId>,
}

 
 
fn default_protected() -> bool {
    true
}

impl Friend {
    fn new(name: String) -> Self {
        Self {
            name,
            mode: FriendMode::Easy,
            protected: true,
            exempt: Vec::new(),
        }
    }

     
    pub fn ignores(&self, module: ModuleId) -> bool {
        if !module_affects_players(module) {
            return false;
        }
        match self.mode {
            FriendMode::Easy => self.protected,
            FriendMode::Advanced => self.exempt.contains(&module),
        }
    }

     
    pub fn summary(&self) -> String {
        match self.mode {
            FriendMode::Easy if self.protected => "All player modules ignored".to_owned(),
            FriendMode::Easy => "All player modules active".to_owned(),
            FriendMode::Advanced => {
                let count = self
                    .exempt
                    .iter()
                    .filter(|module| module_affects_players(**module))
                    .count();
                match count {
                    0 => "No modules ignored".to_owned(),
                    1 => "1 module ignored".to_owned(),
                    count => format!("{count} modules ignored"),
                }
            }
        }
    }
}

 
 
static FRIENDS: LazyLock<Mutex<Vec<Friend>>> = LazyLock::new(|| Mutex::new(Vec::new()));

 
static ANY: AtomicBool = AtomicBool::new(false);

fn lock() -> std::sync::MutexGuard<'static, Vec<Friend>> {
    FRIENDS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
pub fn list() -> Vec<Friend> {
    lock().clone()
}

 
pub fn replace(friends: Vec<Friend>) {
    let mut guard = lock();
    *guard = friends;
    guard.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    ANY.store(!guard.is_empty(), Ordering::Relaxed);
}

 
 
pub fn add(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 32 {
        return false;
    }
    let mut guard = lock();
    if guard
        .iter()
        .any(|friend| friend.name.eq_ignore_ascii_case(name))
    {
        return false;
    }
    guard.push(Friend::new(name.to_owned()));
    guard.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    ANY.store(true, Ordering::Relaxed);
    true
}

 
pub fn remove(name: &str) {
    let mut guard = lock();
    guard.retain(|friend| !friend.name.eq_ignore_ascii_case(name));
    ANY.store(!guard.is_empty(), Ordering::Relaxed);
}

 
 
 
pub fn set_mode(name: &str, mode: FriendMode) {
    let mut guard = lock();
    let Some(friend) = guard
        .iter_mut()
        .find(|friend| friend.name.eq_ignore_ascii_case(name))
    else {
        return;
    };
    if friend.mode == mode {
        return;
    }
    match mode {
        FriendMode::Easy => {
             
             
             
            friend.protected = PLAYER_MODULES
                .iter()
                .all(|module| friend.exempt.contains(module));
        }
        FriendMode::Advanced => {
            friend.exempt = if friend.protected {
                PLAYER_MODULES.to_vec()
            } else {
                Vec::new()
            };
        }
    }
    friend.mode = mode;
}

 
pub fn set_protected(name: &str, protected: bool) {
    let mut guard = lock();
    if let Some(friend) = guard
        .iter_mut()
        .find(|friend| friend.name.eq_ignore_ascii_case(name))
    {
        friend.protected = protected;
    }
}

 
pub fn set_exempt(name: &str, module: ModuleId, exempt: bool) {
    if !module_affects_players(module) {
        return;
    }
    let mut guard = lock();
    let Some(friend) = guard
        .iter_mut()
        .find(|friend| friend.name.eq_ignore_ascii_case(name))
    else {
        return;
    };
    friend.exempt.retain(|entry| *entry != module);
    if exempt {
         
        friend.exempt.push(module);
        friend
            .exempt
            .sort_by_key(|module| PLAYER_MODULES.iter().position(|m| m == module));
    }
}

 
pub fn is_friend(name: &str) -> bool {
    lock()
        .iter()
        .any(|friend| friend.name.eq_ignore_ascii_case(name.trim()))
}

 
pub fn ignores(name: &str, module: ModuleId) -> bool {
    if !module_affects_players(module) {
        return false;
    }
    lock()
        .iter()
        .find(|friend| friend.name.eq_ignore_ascii_case(name.trim()))
        .is_some_and(|friend| friend.ignores(module))
}

 
 
 
 
 
 
pub fn ignores_entity(entity: &Entity, module: ModuleId) -> bool {
    if !ANY.load(Ordering::Relaxed) || !module_affects_players(module) {
        return false;
    }
    if !entity.instance_of::<Player>() {
        return false;
    }
    let Ok(name) = entity.get_name().and_then(|name| name.get_string()) else {
        return false;
    };
    ignores(&name, module)
}

#[cfg(test)]
mod tests {
    use super::*;

     
     
     
    fn reset() -> std::sync::MutexGuard<'static, ()> {
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let guard = TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        replace(Vec::new());
        guard
    }

    #[test]
    fn a_new_friend_is_protected_in_easy_mode() {
        let _guard = reset();
        assert!(add("Notch"));
        let friend = list().pop().unwrap();
        assert_eq!(friend.name, "Notch");
        assert_eq!(friend.mode, FriendMode::Easy);
        assert!(friend.protected);
        for module in PLAYER_MODULES {
            assert!(ignores("Notch", module));
        }
    }

    #[test]
    fn names_are_matched_case_insensitively_and_never_duplicated() {
        let _guard = reset();
        assert!(add("Notch"));
        assert!(!add("notch"));
        assert!(!add("  NOTCH  "));
        assert!(is_friend("nOtCh"));
        remove("NOTCH");
        assert!(!is_friend("Notch"));
    }

    #[test]
    fn empty_and_overlong_names_are_rejected() {
        let _guard = reset();
        assert!(!add(""));
        assert!(!add("   "));
        assert!(!add(&"a".repeat(33)));
        assert!(list().is_empty());
    }

    #[test]
    fn only_player_modules_can_be_exempted() {
        let _guard = reset();
        assert!(add("Notch"));
        set_protected("Notch", false);
        set_mode("Notch", FriendMode::Advanced);
        set_exempt("Notch", ModuleId::Fly, true);
        set_exempt("Notch", ModuleId::Sprint, true);
        assert!(!ignores("Notch", ModuleId::Fly));
        assert!(!ignores("Notch", ModuleId::Sprint));
        assert!(!ignores("Notch", ModuleId::KillAura));
    }

    #[test]
    fn advanced_mode_exempts_only_the_checked_modules() {
        let _guard = reset();
        assert!(add("Notch"));
        set_mode("Notch", FriendMode::Advanced);
         
        assert!(ignores("Notch", ModuleId::KillAura));
        assert!(ignores("Notch", ModuleId::TriggerBot));

        set_exempt("Notch", ModuleId::KillAura, false);
        assert!(!ignores("Notch", ModuleId::KillAura));
        assert!(ignores("Notch", ModuleId::Aimbot));

         
         
        set_mode("Notch", FriendMode::Easy);
        assert!(!ignores("Notch", ModuleId::KillAura));
        set_mode("Notch", FriendMode::Advanced);
        for module in PLAYER_MODULES {
            assert!(!ignores("Notch", module));
        }
    }

    #[test]
    fn unknown_friends_are_never_ignored() {
        let _guard = reset();
        assert!(!ignores("Nobody", ModuleId::KillAura));
        assert!(!module_affects_players(ModuleId::Sprint));
        assert!(module_affects_players(ModuleId::KillAura));
    }

    #[test]
    fn summaries_reflect_the_mode_and_selection() {
        let _guard = reset();
        assert!(add("Notch"));
        assert_eq!(list()[0].summary(), "All player modules ignored");
        set_protected("Notch", false);
        assert_eq!(list()[0].summary(), "All player modules active");
        set_mode("Notch", FriendMode::Advanced);
        assert_eq!(list()[0].summary(), "No modules ignored");
        set_exempt("Notch", ModuleId::Aimbot, true);
        assert_eq!(list()[0].summary(), "1 module ignored");
    }

    #[test]
    fn the_list_survives_a_serialization_round_trip() {
        let _guard = reset();
        assert!(add("Notch"));
        set_protected("Notch", false);
        set_mode("Notch", FriendMode::Advanced);
        set_exempt("Notch", ModuleId::TriggerBot, true);
        let json = serde_json::to_string(&list()).unwrap();
        let restored: Vec<Friend> = serde_json::from_str(&json).unwrap();
        replace(restored);
        assert!(ignores("Notch", ModuleId::TriggerBot));
        assert!(!ignores("Notch", ModuleId::KillAura));
    }
}
