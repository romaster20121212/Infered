 
 
 
 
 
 
 
 
 

pub mod conds;
pub mod engine;
pub mod host;
pub mod module;
pub mod run;
mod ui;

#[cfg(test)]
mod test_host;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use infered_macros::{macros_dir, toggle_label};
use log::{info, warn};

use crate::config;
use crate::mapping::entity::LivingEntityRef;
use engine::{Engine, InputEvent, MacroControl, Policy, RunStatus};
use host::RealHost;

 
 
struct Runtime {
    engine: Engine,
    host: RealHost,
}

static RUNTIME: OnceLock<Mutex<Option<Runtime>>> = OnceLock::new();
static INPUT_QUEUE: OnceLock<Mutex<Vec<InputEvent>>> = OnceLock::new();

 
#[derive(Debug, Clone, Default)]
pub struct Board {
    pub enabled: bool,
    pub in_world: bool,
    pub loaded: usize,
    pub broken: usize,
    pub running: Vec<RunStatus>,
    pub error: Option<String>,
    pub directory: PathBuf,
}

static BOARD: Mutex<Board> = Mutex::new(Board {
    enabled: false,
    in_world: false,
    loaded: 0,
    broken: 0,
    running: Vec::new(),
    error: None,
    directory: PathBuf::new(),
});

fn runtime() -> MutexGuard<'static, Option<Runtime>> {
    RUNTIME
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn queue() -> MutexGuard<'static, Vec<InputEvent>> {
    INPUT_QUEUE
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn board_slot() -> MutexGuard<'static, Board> {
    BOARD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 
 
pub fn directory() -> PathBuf {
    macros_dir(&config::base_dir())
}

 
 
pub fn init() {
    let dir = directory();
    if let Err(error) = std::fs::create_dir_all(&dir) {
        warn!("macros: could not create {}: {error}", dir.display());
    }
    info!("macros: library directory {}", dir.display());
    *runtime() = Some(Runtime {
        engine: Engine::new(dir.clone()),
        host: RealHost::new(),
    });
    let mut board = board_slot();
    board.directory = dir;
    board.enabled = false;
}

 
 
pub fn shutdown() {
    let mut guard = runtime();
    if let Some(runtime) = guard.as_mut() {
        runtime.engine.shutdown(&mut runtime.host);
    }
    *guard = None;
    queue().clear();
    let mut board = board_slot();
    *board = Board {
        directory: board.directory.clone(),
        ..Board::default()
    };
}

 
 
pub fn push_event(event: InputEvent) {
    let mut events = queue();
    if events.len() < 256 {
        events.push(event);
    }
}

 
 
pub fn frame() {
    let policy = module::policy();
    let mc = crate::state::minecraft();
    let in_world = matches!(mc.world(), Ok(Some(_)));
    let alive = mc
        .player()
        .ok()
        .flatten()
        .is_some_and(|player| player.is_alive().unwrap_or(false));

    let mut guard = runtime();
    let Some(runtime) = guard.as_mut() else {
        return;
    };
    runtime.engine.set_policy(&mut runtime.host, policy);
    runtime
        .engine
        .update_session(&mut runtime.host, in_world, alive);
    runtime.engine.maintain(&mut runtime.host, false);
    module::sync(&mut runtime.engine, &mut runtime.host);
    let board = Board {
        enabled: runtime.engine.enabled(),
        in_world: runtime.engine.is_active(),
        loaded: runtime.engine.macro_count(),
        broken: runtime.engine.broken_files(),
        running: runtime.engine.running(),
        error: runtime.engine.last_error.clone(),
        directory: runtime.engine.directory().to_path_buf(),
    };
    drop(guard);
    *board_slot() = board;
}

 
pub fn tick() {
    let mut guard = runtime();
    let Some(runtime) = guard.as_mut() else {
        return;
    };
    {
        let mut events = queue();
        for event in events.drain(..) {
            runtime.engine.push_event(event);
        }
    }
    runtime.engine.tick(&mut runtime.host);
    let board = Board {
        enabled: runtime.engine.enabled(),
        in_world: runtime.engine.is_active(),
        loaded: runtime.engine.macro_count(),
        broken: runtime.engine.broken_files(),
        running: runtime.engine.running(),
        error: runtime.engine.last_error.clone(),
        directory: runtime.engine.directory().to_path_buf(),
    };
    drop(guard);
    *board_slot() = board;
}

 
pub fn stop_all() {
    let mut guard = runtime();
    if let Some(runtime) = guard.as_mut() {
        runtime
            .engine
            .stop_everything(&mut runtime.host, "stopped from the HUD");
    }
}

 
pub fn board() -> Board {
    board_slot().clone()
}

 
pub fn controls() -> Vec<MacroControl> {
    runtime()
        .as_ref()
        .map(|runtime| runtime.engine.controls())
        .unwrap_or_default()
}

 
 
pub fn rescan() {
    let mut guard = runtime();
    if let Some(runtime) = guard.as_mut() {
        runtime.engine.invalidate_scan();
        runtime.engine.maintain(&mut runtime.host, true);
    }
}

 
pub fn draw_hud(ctx: &egui::Context) {
    let Some(handle) = crate::state::client()
        .modules
        .get(crate::module::ModuleId::Macros)
    else {
        return;
    };
    let Ok(module) = handle.lock() else {
        return;
    };
    let data = module.get_module_data();
    let hud_on = data
        .get_setting("Status HUD")
        .and_then(|setting| setting.get_toggle_value())
        .unwrap_or(true);
    let enabled = data.enabled;
    drop(module);
    if !enabled || !hud_on {
        return;
    }
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        crate::graphic::hud::layer_id(),
    ));
    ui::draw(ctx, &painter, &board());
}

 
pub fn setting_label(name: &str) -> String {
    format!("{}{}", module::MACRO_SETTING_PREFIX, toggle_label(name))
}

 
pub fn has_dynamic_switches() -> bool {
    runtime()
        .as_ref()
        .is_some_and(|runtime| !runtime.engine.controls().is_empty())
}

 
pub fn describe() -> String {
    let board = board();
    format!(
        "{} macros ({} broken), {} running, enabled={}, world={}",
        board.loaded,
        board.broken,
        board.running.len(),
        board.enabled,
        board.in_world
    )
}

 
pub fn current_policy() -> Policy {
    runtime()
        .as_ref()
        .map(|runtime| Policy {
            enabled: runtime.engine.enabled(),
            ..Policy::default()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_labels_carry_the_prefix() {
        let label = setting_label("Farm");
        assert!(module::is_dynamic_setting_name(&label));
        assert!(label.ends_with("Farm"));
    }

    #[test]
    fn board_starts_empty() {
        let board = board();
        assert!(board.running.is_empty());
    }

    #[test]
    fn shutdown_is_idempotent() {
        shutdown();
        shutdown();
    }
}
