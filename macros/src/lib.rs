pub mod model;
pub mod validate;
pub use model::*;

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct MacroFile {
    pub file_name: String,
    pub macro_: Macro,
    pub modified: Option<SystemTime>,
}
#[derive(Debug, Clone)]
pub struct BrokenFile {
    pub file_name: String,
    pub error: String,
}
#[derive(Debug, Default)]
pub struct Library {
    pub macros: Vec<MacroFile>,
    pub broken: Vec<BrokenFile>,
}

pub fn macros_dir(base: &Path) -> PathBuf {
    base.join("macros")
}
pub fn toggle_label(name: &str) -> String {
    name.trim().chars().take(40).collect()
}
pub fn duration(ms: u64) -> String {
    if ms >= 1000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{ms}ms")
    }
}
pub fn action_summary(action: &Action) -> String {
    kind_of(action).serde_name().replace('_', " ")
}
pub fn trigger_summary(trigger: &Trigger) -> String {
    serde_json::to_value(trigger)
        .ok()
        .and_then(|v| {
            v.get("type")
                .and_then(|x| x.as_str())
                .map(|x| x.replace('_', " "))
        })
        .unwrap_or_default()
}
pub fn condition_summary(condition: &Condition) -> String {
    serde_json::to_value(condition)
        .ok()
        .and_then(|v| v.get("type").and_then(|x| x.as_str()).map(str::to_owned))
        .unwrap_or_default()
}
pub use validate::validate;

pub fn parse_text(text: &str) -> Result<Macro, String> {
    if text.len() > 1_048_576 {
        return Err("macro file is too large".into());
    }
    let doc: Macro = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let issues = validate(&doc);
    if !issues.is_empty() {
        return Err(issues
            .iter()
            .map(|i| format!("{}: {}", i.path, i.message))
            .collect::<Vec<_>>()
            .join("; "));
    }
    Ok(doc)
}
fn path_for(dir: &Path, file: &str) -> Result<PathBuf, String> {
    let path = Path::new(file);
    if file.is_empty()
        || path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
        || path.extension().and_then(|x| x.to_str()) != Some("macro")
    {
        return Err("invalid macro file name".into());
    }
    Ok(dir.join(path))
}
pub fn read(dir: &Path, file: &str) -> Result<Macro, String> {
    let path = path_for(dir, file)?;
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_text(&text)
}
pub fn load_library(dir: &Path) -> Library {
    let mut out = Library::default();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.file_name());
    let mut ids = HashSet::new();
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if path_for(dir, &name).is_err() || !entry.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        match read(dir, &name) {
            Ok(doc) if ids.insert(doc.id.clone()) => out.macros.push(MacroFile {
                file_name: name,
                macro_: doc,
                modified: entry.metadata().ok().and_then(|m| m.modified().ok()),
            }),
            Ok(_) => out.broken.push(BrokenFile {
                file_name: name,
                error: "duplicate macro id".into(),
            }),
            Err(error) => out.broken.push(BrokenFile {
                file_name: name,
                error,
            }),
        }
    }
    out
}
fn slug(name: &str) -> String {
    let s: String = name
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(48)
        .collect();
    if s.trim_matches('_').is_empty() {
        "macro".into()
    } else {
        s
    }
}
fn unique_name(dir: &Path, id: &str) -> String {
    let base = slug(id);
    for i in 0.. {
        let n = if i == 0 {
            format!("{base}.macro")
        } else {
            format!("{base}_{i}.macro")
        };
        if !dir.join(&n).exists() {
            return n;
        }
    }
    unreachable!()
}
pub fn new_template(dir: &Path, name: &str) -> Macro {
    let file = unique_name(dir, name);
    Macro::new_template(file.trim_end_matches(".macro"), name)
}
pub fn save(dir: &Path, file: Option<&str>, doc: &Macro) -> Result<String, String> {
    let issues = validate(doc);
    if !issues.is_empty() {
        return Err(issues
            .iter()
            .map(|i| format!("{}: {}", i.path, i.message))
            .collect::<Vec<_>>()
            .join("; "));
    }
    if load_library(dir)
        .macros
        .iter()
        .any(|entry| entry.macro_.id == doc.id && Some(entry.file_name.as_str()) != file)
    {
        return Err("macro id already exists".into());
    }
    let text = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?;
    let (_, name) = save_text(dir, file, &text)?;
    Ok(name)
}
pub fn save_text(dir: &Path, file: Option<&str>, text: &str) -> Result<(Macro, String), String> {
    let doc = parse_text(text)?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let name = file
        .map(str::to_owned)
        .unwrap_or_else(|| unique_name(dir, &doc.id));
    let path = path_for(dir, &name)?;
    if load_library(dir)
        .macros
        .iter()
        .any(|entry| entry.macro_.id == doc.id && entry.file_name != name)
    {
        return Err("macro id already exists".into());
    }
    if file.is_none() {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        f.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    } else {
        std::fs::write(path, text).map_err(|e| e.to_string())?;
    }
    Ok((doc, name))
}
pub fn duplicate(dir: &Path, file: &str) -> Result<String, String> {
    let mut doc = read(dir, file)?;
    let name = unique_name(dir, &format!("{}_copy", doc.id));
    doc.id = name.trim_end_matches(".macro").to_owned();
    doc.name = format!("{} Copy", doc.name);
    doc.enabled = false;
    save(dir, None, &doc)
}
pub fn delete(dir: &Path, file: &str) -> Result<(), String> {
    std::fs::remove_file(path_for(dir, file)?).map_err(|e| e.to_string())
}
pub fn import(dir: &Path, path: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let doc = parse_text(&text)?;
    if load_library(dir)
        .macros
        .iter()
        .any(|m| m.macro_.id == doc.id)
    {
        return Err("macro id already exists".into());
    }
    save(dir, None, &doc)
}
pub fn export(dir: &Path, file: &str, destination: &Path) -> Result<PathBuf, String> {
    let text = std::fs::read_to_string(path_for(dir, file)?).map_err(|e| e.to_string())?;
    parse_text(&text)?;
    std::fs::write(destination, text).map_err(|e| e.to_string())?;
    Ok(destination.to_path_buf())
}

#[derive(Debug, Clone, Copy)]
pub struct KeySpec {
    pub name: &'static str,
    pub glfw: i32,
    pub translation: &'static str,
}
pub fn find_key(key: &str) -> Option<KeySpec> {
    KEYS.iter()
        .copied()
        .find(|item| item.name.eq_ignore_ascii_case(key))
}
const KEYS: &[KeySpec] = &[
    KeySpec {
        name: "B",
        glfw: 66,
        translation: "key.keyboard.b",
    },
    KeySpec {
        name: "C",
        glfw: 67,
        translation: "key.keyboard.c",
    },
    KeySpec {
        name: "E",
        glfw: 69,
        translation: "key.keyboard.e",
    },
    KeySpec {
        name: "F",
        glfw: 70,
        translation: "key.keyboard.f",
    },
    KeySpec {
        name: "G",
        glfw: 71,
        translation: "key.keyboard.g",
    },
    KeySpec {
        name: "H",
        glfw: 72,
        translation: "key.keyboard.h",
    },
    KeySpec {
        name: "I",
        glfw: 73,
        translation: "key.keyboard.i",
    },
    KeySpec {
        name: "J",
        glfw: 74,
        translation: "key.keyboard.j",
    },
    KeySpec {
        name: "K",
        glfw: 75,
        translation: "key.keyboard.k",
    },
    KeySpec {
        name: "L",
        glfw: 76,
        translation: "key.keyboard.l",
    },
    KeySpec {
        name: "M",
        glfw: 77,
        translation: "key.keyboard.m",
    },
    KeySpec {
        name: "N",
        glfw: 78,
        translation: "key.keyboard.n",
    },
    KeySpec {
        name: "O",
        glfw: 79,
        translation: "key.keyboard.o",
    },
    KeySpec {
        name: "P",
        glfw: 80,
        translation: "key.keyboard.p",
    },
    KeySpec {
        name: "Q",
        glfw: 81,
        translation: "key.keyboard.q",
    },
    KeySpec {
        name: "R",
        glfw: 82,
        translation: "key.keyboard.r",
    },
    KeySpec {
        name: "T",
        glfw: 84,
        translation: "key.keyboard.t",
    },
    KeySpec {
        name: "U",
        glfw: 85,
        translation: "key.keyboard.u",
    },
    KeySpec {
        name: "V",
        glfw: 86,
        translation: "key.keyboard.v",
    },
    KeySpec {
        name: "X",
        glfw: 88,
        translation: "key.keyboard.x",
    },
    KeySpec {
        name: "Y",
        glfw: 89,
        translation: "key.keyboard.y",
    },
    KeySpec {
        name: "Z",
        glfw: 90,
        translation: "key.keyboard.z",
    },
    KeySpec {
        name: "0",
        glfw: 48,
        translation: "key.keyboard.0",
    },
    KeySpec {
        name: "1",
        glfw: 49,
        translation: "key.keyboard.1",
    },
    KeySpec {
        name: "2",
        glfw: 50,
        translation: "key.keyboard.2",
    },
    KeySpec {
        name: "3",
        glfw: 51,
        translation: "key.keyboard.3",
    },
    KeySpec {
        name: "4",
        glfw: 52,
        translation: "key.keyboard.4",
    },
    KeySpec {
        name: "5",
        glfw: 53,
        translation: "key.keyboard.5",
    },
    KeySpec {
        name: "6",
        glfw: 54,
        translation: "key.keyboard.6",
    },
    KeySpec {
        name: "7",
        glfw: 55,
        translation: "key.keyboard.7",
    },
    KeySpec {
        name: "8",
        glfw: 56,
        translation: "key.keyboard.8",
    },
    KeySpec {
        name: "9",
        glfw: 57,
        translation: "key.keyboard.9",
    },
    KeySpec {
        name: "W",
        glfw: 87,
        translation: "key.keyboard.w",
    },
    KeySpec {
        name: "A",
        glfw: 65,
        translation: "key.keyboard.a",
    },
    KeySpec {
        name: "S",
        glfw: 83,
        translation: "key.keyboard.s",
    },
    KeySpec {
        name: "D",
        glfw: 68,
        translation: "key.keyboard.d",
    },
    KeySpec {
        name: "Space",
        glfw: 32,
        translation: "key.keyboard.space",
    },
    KeySpec {
        name: "Left Shift",
        glfw: 340,
        translation: "key.keyboard.left.shift",
    },
    KeySpec {
        name: "Left Control",
        glfw: 341,
        translation: "key.keyboard.left.control",
    },
    KeySpec {
        name: "F1",
        glfw: 290,
        translation: "key.keyboard.f1",
    },
    KeySpec {
        name: "F2",
        glfw: 291,
        translation: "key.keyboard.f2",
    },
    KeySpec {
        name: "F3",
        glfw: 292,
        translation: "key.keyboard.f3",
    },
    KeySpec {
        name: "F4",
        glfw: 293,
        translation: "key.keyboard.f4",
    },
    KeySpec {
        name: "F5",
        glfw: 294,
        translation: "key.keyboard.f5",
    },
    KeySpec {
        name: "F6",
        glfw: 295,
        translation: "key.keyboard.f6",
    },
    KeySpec {
        name: "F7",
        glfw: 296,
        translation: "key.keyboard.f7",
    },
    KeySpec {
        name: "F8",
        glfw: 297,
        translation: "key.keyboard.f8",
    },
    KeySpec {
        name: "F9",
        glfw: 298,
        translation: "key.keyboard.f9",
    },
    KeySpec {
        name: "F10",
        glfw: 299,
        translation: "key.keyboard.f10",
    },
    KeySpec {
        name: "F11",
        glfw: 300,
        translation: "key.keyboard.f11",
    },
    KeySpec {
        name: "F12",
        glfw: 301,
        translation: "key.keyboard.f12",
    },
];

macro_rules! kinds { ($( $name:ident => $tag:literal => $pattern:pat => $default:expr ),+ $(,)?) => {
    #[derive(Debug,Clone,Copy)] pub enum ActionKind { $( $name ),+ }
    impl ActionKind { pub const ALL: &'static [Self] = &[$(Self::$name),+]; pub fn serde_name(self)->&'static str { match self { $(Self::$name => $tag),+ } } pub fn default_action(self)->Action { match self { $(Self::$name => $default),+ } } }
    fn kind_of(action:&Action)->ActionKind { match action { $( $pattern => ActionKind::$name ),+ } }
} }
fn default_condition() -> Condition {
    Condition::compare(
        ValueRef::state("health"),
        CompareOp::Lt,
        ValueRef::literal(serde_json::json!(10.0)),
    )
}
kinds! {
 Wait=>"wait"=>Action::Wait{..}=>Action::Wait{duration_ms:500},
 WaitRandom=>"wait_random"=>Action::WaitRandom{..}=>Action::WaitRandom{min_ms:500,max_ms:1000},
 WaitUntil=>"wait_until"=>Action::WaitUntil{..}=>Action::WaitUntil{condition:default_condition(),timeout_ms:5000},
 Key=>"key"=>Action::Key{..}=>Action::Key{key:"F8".into(),action:KeyAction::Tap,hold_ms:0},
 Mouse=>"mouse"=>Action::Mouse{..}=>Action::Mouse{button:1,action:KeyAction::Tap,hold_ms:0},
 ReleaseInputs=>"release_inputs"=>Action::ReleaseInputs=>Action::ReleaseInputs,
 Hotbar=>"hotbar"=>Action::Hotbar{..}=>Action::Hotbar{slot:1},
 CycleHotbar=>"cycle_hotbar"=>Action::CycleHotbar{..}=>Action::CycleHotbar{steps:1},
 UseItem=>"use_item"=>Action::UseItem{..}=>Action::UseItem{hold_ms:50},
 Attack=>"attack"=>Action::Attack=>Action::Attack,
 Interact=>"interact"=>Action::Interact=>Action::Interact,
 PickBlock=>"pick_block"=>Action::PickBlock=>Action::PickBlock,
 Movement=>"movement"=>Action::Movement{..}=>Action::Movement{key:MovementKey::Forward,action:KeyAction::Tap,hold_ms:50},
 OpenInventory=>"open_inventory"=>Action::OpenInventory=>Action::OpenInventory,
 CloseScreen=>"close_screen"=>Action::CloseScreen=>Action::CloseScreen,
 Chat=>"chat"=>Action::Chat{..}=>Action::Chat{message:String::new()},
 SetVar=>"set_var"=>Action::SetVar{..}=>Action::SetVar{name:"value".into(),value:serde_json::Value::Null},
 If=>"if"=>Action::If{..}=>Action::If{condition:default_condition(),then:vec![],otherwise:vec![]},
 Repeat=>"repeat"=>Action::Repeat{..}=>Action::Repeat{times:2,actions:vec![]},
 RepeatWhile=>"repeat_while"=>Action::RepeatWhile{..}=>Action::RepeatWhile{condition:default_condition(),max_iterations:10,actions:vec![]},
 RepeatUntil=>"repeat_until"=>Action::RepeatUntil{..}=>Action::RepeatUntil{condition:default_condition(),max_iterations:10,actions:vec![]},
 RunMacro=>"run_macro"=>Action::RunMacro{..}=>Action::RunMacro{id:String::new()},
 Stop=>"stop"=>Action::Stop=>Action::Stop,
 StopAll=>"stop_all"=>Action::StopAll=>Action::StopAll,
 Log=>"log"=>Action::Log{..}=>Action::Log{message:String::new()}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_action_defaults_round_trip() {
        for kind in ActionKind::ALL {
            let value = serde_json::to_value(kind.default_action()).unwrap();
            assert_eq!(value["type"], kind.serde_name());
            serde_json::from_value::<Action>(value).unwrap();
        }
    }

    #[test]
    fn state_keys_match_engine_examples_and_reject_unknown_names() {
        for (raw, expected) in [
            ("effect:speed", StateKey::Effect("speed".into())),
            (
                "has_item:minecraft:golden_apple",
                StateKey::HasItem("minecraft:golden_apple".into()),
            ),
            (
                "item_count:minecraft:stone",
                StateKey::ItemCount("minecraft:stone".into()),
            ),
        ] {
            let parsed: StateKey = serde_json::from_value(serde_json::json!(raw)).unwrap();
            assert_eq!(parsed, expected);
            assert_eq!(serde_json::to_value(parsed).unwrap(), raw);
        }
        assert!(serde_json::from_value::<StateKey>(serde_json::json!("typo")).is_err());
    }

    #[test]
    fn validates_limits_and_nested_actions() {
        let mut doc = Macro::new_template("test", "Test");
        doc.actions = vec![Action::WaitRandom {
            min_ms: 2,
            max_ms: 1,
        }];
        assert!(!validate(&doc).is_empty());
        doc.actions = vec![Action::Repeat {
            times: 1,
            actions: vec![Action::Attack],
        }];
        assert!(validate(&doc).is_empty());
        let text = serde_json::to_string(&doc).unwrap();
        assert_eq!(parse_text(&text).unwrap().id, "test");
    }

    #[test]
    fn checks_file_names_and_common_keys() {
        assert!(path_for(Path::new("."), "../outside.macro").is_err());
        assert!(path_for(Path::new("."), "valid.macro").is_ok());
        assert_eq!(find_key("F8").unwrap().glfw, 297);
        assert_eq!(find_key("q").unwrap().glfw, 81);
    }
}
