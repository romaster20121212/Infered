 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 
 

use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

 
 
pub const PLAYER_MODULES: [(&str, &str); 7] = [
    ("KillAura", "KillAura"),
    ("SilentAura", "SilentAura"),
    ("Aimbot", "Aimbot"),
    ("TriggerBot", "TriggerBot"),
    ("AutoMace", "AutoMace"),
    ("SilentMace", "Silent Mace"),
    ("HitSelect", "HitSelect"),
];

 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendMode {
     
    Easy,
     
    Advanced,
}

impl FriendMode {
     
    pub fn as_str(self) -> &'static str {
        match self {
            FriendMode::Easy => "Easy",
            FriendMode::Advanced => "Advanced",
        }
    }

     
    pub fn label(self) -> &'static str {
        self.as_str()
    }

    fn from_json(value: Option<&Value>) -> Self {
        match value.and_then(Value::as_str) {
            Some("Advanced") => FriendMode::Advanced,
            _ => FriendMode::Easy,
        }
    }
}

 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriendView {
    pub name: String,
    pub mode: FriendMode,
     
    pub protected: bool,
     
    pub exempt: Vec<String>,
}

impl FriendView {
     
     
    pub fn is_protected(&self) -> bool {
        match self.mode {
            FriendMode::Easy => self.protected,
            FriendMode::Advanced => PLAYER_MODULES
                .iter()
                .all(|(module, _)| self.exempt.iter().any(|entry| entry == module)),
        }
    }

     
    pub fn ignores(&self, module: &str) -> bool {
        if !PLAYER_MODULES.iter().any(|(id, _)| *id == module) {
            return false;
        }
        match self.mode {
            FriendMode::Easy => self.protected,
            FriendMode::Advanced => self.exempt.iter().any(|entry| entry == module),
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
                    .filter(|module| PLAYER_MODULES.iter().any(|(id, _)| *id == module.as_str()))
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

 
 
 
 
 
pub fn read(live: &Path) -> Result<Vec<FriendView>, String> {
    let Some(root) = read_root(live)? else {
        return Ok(Vec::new());
    };
    let Some(friends) = root.get("friends") else {
        return Ok(Vec::new());
    };
    let friends = friends
        .as_array()
        .ok_or_else(|| "the live config's friend list is not an array".to_string())?;
    let mut views: Vec<FriendView> = friends.iter().filter_map(parse_friend).collect();
    views.sort_by_cached_key(|friend| friend.name.to_lowercase());
    Ok(views)
}

 
 
 
pub fn add(live: &Path, name: &str) -> Result<(), String> {
    let name = validate_name(name)?;
    edit(live, |friends| {
        if friends.iter().any(|friend| name_matches(friend, &name)) {
            return Err(format!("{name} is already on the friend list"));
        }
        friends.push(serde_json::json!({
            "name": name,
            "mode": FriendMode::Easy.as_str(),
            "protected": true,
            "exempt": [],
        }));
        sort_friends(friends);
        Ok(())
    })
}

 
pub fn remove(live: &Path, name: &str) -> Result<(), String> {
    edit(live, |friends| {
        friends.retain(|friend| !name_matches(friend, name.trim()));
        Ok(())
    })
}

 
 
pub fn set_mode(live: &Path, name: &str, mode: FriendMode) -> Result<(), String> {
    edit(live, |friends| {
        let friend = find_friend(friends, name)
            .ok_or_else(|| format!("{} is not on the friend list", name.trim()))?;
        if FriendMode::from_json(friend.get("mode")) == mode {
            return Ok(());
        }
        let protected = friend
            .get("protected")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let exempt = string_list(friend.get("exempt"));
        match mode {
            FriendMode::Easy => {
                 
                 
                let all = PLAYER_MODULES
                    .iter()
                    .all(|(module, _)| exempt.iter().any(|entry| entry == module));
                friend.insert("protected".to_string(), Value::Bool(all));
            }
            FriendMode::Advanced => {
                let seeded = if protected {
                    PLAYER_MODULES
                        .iter()
                        .map(|(module, _)| Value::String((*module).to_string()))
                        .collect()
                } else {
                    Vec::new()
                };
                friend.insert("exempt".to_string(), Value::Array(seeded));
            }
        }
        friend.insert("mode".to_string(), Value::String(mode.as_str().to_string()));
        Ok(())
    })
}

 
pub fn set_protected(live: &Path, name: &str, protected: bool) -> Result<(), String> {
    edit(live, |friends| {
        let friend = find_friend(friends, name)
            .ok_or_else(|| format!("{} is not on the friend list", name.trim()))?;
        friend.insert("protected".to_string(), Value::Bool(protected));
        Ok(())
    })
}

 
pub fn set_exempt(live: &Path, name: &str, module: &str, exempt: bool) -> Result<(), String> {
    if !PLAYER_MODULES.iter().any(|(id, _)| *id == module) {
        return Err(format!("{module} cannot affect another player"));
    }
    edit(live, |friends| {
        let friend = find_friend(friends, name)
            .ok_or_else(|| format!("{} is not on the friend list", name.trim()))?;
        let mut entries = string_list(friend.get("exempt"));
        entries.retain(|entry| entry != module);
        if exempt {
            entries.push(module.to_string());
             
            entries.sort_by_key(|entry| {
                PLAYER_MODULES
                    .iter()
                    .position(|(id, _)| id == entry)
                    .unwrap_or(usize::MAX)
            });
        }
        friend.insert(
            "exempt".to_string(),
            Value::Array(entries.into_iter().map(Value::String).collect()),
        );
        Ok(())
    })
}

 
fn validate_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("enter a username".to_string());
    }
    if name.chars().count() > 32 {
        return Err("usernames can be at most 32 characters".to_string());
    }
    Ok(name.to_string())
}

fn parse_friend(entry: &Value) -> Option<FriendView> {
    let name = entry.get("name")?.as_str()?.trim();
    if name.is_empty() {
        return None;
    }
    let mode = FriendMode::from_json(entry.get("mode"));
    Some(FriendView {
        name: name.to_string(),
        mode,
        protected: entry
            .get("protected")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        exempt: string_list(entry.get("exempt")),
    })
}

fn name_matches(entry: &Value, name: &str) -> bool {
    entry
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|saved| saved.trim().eq_ignore_ascii_case(name.trim()))
}

fn find_friend<'a>(friends: &'a mut [Value], name: &str) -> Option<&'a mut Map<String, Value>> {
    friends.iter_mut().find_map(|entry| {
        let friend = entry.as_object_mut()?;
        let saved = friend.get("name").and_then(Value::as_str)?;
        if saved.trim().eq_ignore_ascii_case(name.trim()) {
            Some(friend)
        } else {
            None
        }
    })
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn sort_friends(friends: &mut [Value]) {
    friends.sort_by(|left, right| {
        let left = left
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_lowercase();
        let right = right
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_lowercase();
        left.cmp(&right)
    });
}

 
fn read_root(live: &Path) -> Result<Option<Value>, String> {
    match fs::read_to_string(live) {
        Ok(text) => {
            let value: Value = serde_json::from_str(&text)
                .map_err(|_| "the live config could not be parsed".to_string())?;
            if value.is_object() {
                Ok(Some(value))
            } else {
                Err("the live config is not a JSON object".to_string())
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("could not read the live config: {error}")),
    }
}

 
 
 
fn edit<F>(live: &Path, change: F) -> Result<(), String>
where
    F: FnOnce(&mut Vec<Value>) -> Result<(), String>,
{
    let mut root = match read_root(live)? {
        Some(root) => root,
        None => serde_json::json!({ "modules": [], "friends": [] }),
    };
    let friends = friends_mut(&mut root)?;
    change(friends)?;
    write_config(live, &root)
}

fn friends_mut(root: &mut Value) -> Result<&mut Vec<Value>, String> {
    let object = root
        .as_object_mut()
        .ok_or_else(|| "the live config is not a JSON object".to_string())?;
    if object.get("friends").is_none() {
        object.insert("friends".to_string(), Value::Array(Vec::new()));
    }
    match object.get_mut("friends") {
        Some(Value::Array(friends)) => Ok(friends),
        Some(_) => Err("the live config's friend list is not an array".to_string()),
        None => Err("the live config's friend list could not be created".to_string()),
    }
}

fn write_config(live: &Path, root: &Value) -> Result<(), String> {
    let json = serde_json::to_string_pretty(root)
        .map_err(|error| format!("could not serialize the config: {error}"))?;
    fs::write(live, json).map_err(|error| format!("could not write the live config: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

     
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("infered-friends-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Scratch(path)
        }

        fn live(&self) -> PathBuf {
            self.0.join("infered_config.json")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const CONFIG: &str = r#"{
        "modules": [{ "id": "Sprint", "enabled": true }],
        "ui": { "accent": [1, 2, 3] },
        "friends": [
            { "name": "steve", "mode": "Easy", "protected": false, "exempt": [] },
            { "name": "Alex", "mode": "Advanced", "protected": true, "exempt": ["KillAura"] }
        ]
    }"#;

    #[test]
    fn reads_the_clients_friend_shape_and_sorts_names() {
        let scratch = Scratch::new("read");
        fs::write(scratch.live(), CONFIG).unwrap();

        let friends = read(&scratch.live()).unwrap();
        assert_eq!(
            friends.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
            ["Alex", "steve"]
        );
        assert_eq!(friends[0].mode, FriendMode::Advanced);
        assert_eq!(friends[0].exempt, ["KillAura"]);
        assert!(friends[0].protected, "the saved flag is still readable");
        assert!(!friends[1].protected);
        assert_eq!(friends[0].summary(), "1 module ignored");
        assert!(!friends[1].is_protected());
    }

    #[test]
    fn adding_a_friend_writes_the_client_defaults_and_keeps_the_rest() {
        let scratch = Scratch::new("add");
        fs::write(scratch.live(), CONFIG).unwrap();

        add(&scratch.live(), "  Notch  ").unwrap();
        let saved: Value =
            serde_json::from_str(&fs::read_to_string(scratch.live()).unwrap()).unwrap();
        let names: Vec<&str> = saved["friends"]
            .as_array()
            .unwrap()
            .iter()
            .map(|friend| friend["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["Alex", "Notch", "steve"], "the list stays sorted");
        let notch = &saved["friends"][1];
        assert_eq!(notch["name"], "Notch", "the name is trimmed");
        assert_eq!(notch["mode"], "Easy");
        assert_eq!(notch["protected"], true);
        assert_eq!(notch["exempt"], serde_json::json!([]));
        assert_eq!(saved["ui"]["accent"], serde_json::json!([1, 2, 3]));
        assert_eq!(saved["modules"][0]["enabled"], true);
    }

    #[test]
    fn duplicate_names_are_refused_case_insensitively() {
        let scratch = Scratch::new("duplicate");
        fs::write(scratch.live(), CONFIG).unwrap();

        let error = add(&scratch.live(), "ALEX").unwrap_err();
        assert!(error.contains("already"), "got {error}");
        assert_eq!(read(&scratch.live()).unwrap().len(), 2);
    }

    #[test]
    fn invalid_names_are_refused() {
        let scratch = Scratch::new("invalid");
        assert!(add(&scratch.live(), "   ").is_err());
        assert!(add(&scratch.live(), &"x".repeat(33)).is_err());
        assert!(!scratch.live().exists(), "a refused add writes nothing");
    }

    #[test]
    fn removing_a_friend_is_case_insensitive() {
        let scratch = Scratch::new("remove");
        fs::write(scratch.live(), CONFIG).unwrap();

        remove(&scratch.live(), "STEVE").unwrap();
        let friends = read(&scratch.live()).unwrap();
        assert_eq!(friends.len(), 1);
        assert_eq!(friends[0].name, "Alex");
    }

    #[test]
    fn switching_modes_seeds_the_same_way_as_the_in_game_window() {
        let scratch = Scratch::new("mode");
        fs::write(scratch.live(), CONFIG).unwrap();

         
        set_mode(&scratch.live(), "Alex", FriendMode::Easy).unwrap();
        let alex = &read(&scratch.live()).unwrap()[0];
        assert_eq!(alex.mode, FriendMode::Easy);
        assert!(!alex.protected);

         
        set_mode(&scratch.live(), "STEVE", FriendMode::Advanced).unwrap();
        let steve = read(&scratch.live())
            .unwrap()
            .into_iter()
            .find(|friend| friend.name == "steve")
            .unwrap();
        assert_eq!(steve.mode, FriendMode::Advanced);
        assert!(steve.exempt.is_empty());

         
         
        set_protected(&scratch.live(), "steve", true).unwrap();
        set_mode(&scratch.live(), "steve", FriendMode::Easy).unwrap();
        let steve = read(&scratch.live())
            .unwrap()
            .into_iter()
            .find(|friend| friend.name == "steve")
            .unwrap();
        assert!(!steve.protected);

         
         
        set_mode(&scratch.live(), "steve", FriendMode::Advanced).unwrap();
        for (module, _) in PLAYER_MODULES {
            set_exempt(&scratch.live(), "steve", module, true).unwrap();
        }
        set_mode(&scratch.live(), "steve", FriendMode::Easy).unwrap();
        let steve = read(&scratch.live())
            .unwrap()
            .into_iter()
            .find(|friend| friend.name == "steve")
            .unwrap();
        assert!(steve.protected);
    }

    #[test]
    fn advanced_mode_seeds_every_player_module_when_protected() {
        let scratch = Scratch::new("seed");
        fs::write(
            scratch.live(),
            r#"{"friends":[{"name":"Alex","mode":"Easy","protected":true,"exempt":[]}]}"#,
        )
        .unwrap();

        set_mode(&scratch.live(), "Alex", FriendMode::Advanced).unwrap();
        let alex = &read(&scratch.live()).unwrap()[0];
        assert_eq!(
            alex.exempt,
            PLAYER_MODULES
                .iter()
                .map(|(module, _)| module.to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn exemptions_toggle_in_module_order_and_reject_other_modules() {
        let scratch = Scratch::new("exempt");
        fs::write(scratch.live(), CONFIG).unwrap();

        set_exempt(&scratch.live(), "Alex", "TriggerBot", true).unwrap();
        set_exempt(&scratch.live(), "Alex", "KillAura", false).unwrap();
        let alex = &read(&scratch.live()).unwrap()[0];
        assert_eq!(alex.exempt, ["TriggerBot"]);

        let error = set_exempt(&scratch.live(), "Alex", "Sprint", true).unwrap_err();
        assert!(error.contains("cannot affect"), "got {error}");
    }

    #[test]
    fn a_missing_config_starts_with_a_module_and_friend_list() {
        let scratch = Scratch::new("fresh");
        add(&scratch.live(), "Alex").unwrap();

        let saved: Value =
            serde_json::from_str(&fs::read_to_string(scratch.live()).unwrap()).unwrap();
        assert_eq!(saved["modules"], serde_json::json!([]));
        assert_eq!(saved["friends"][0]["name"], "Alex");
    }

    #[test]
    fn a_damaged_config_is_refused_and_left_alone() {
        let scratch = Scratch::new("damaged");
        fs::write(scratch.live(), "{ not json").unwrap();

        assert!(add(&scratch.live(), "Alex").is_err());
        assert!(read(&scratch.live()).is_err());
        assert_eq!(fs::read_to_string(scratch.live()).unwrap(), "{ not json");
    }
}
