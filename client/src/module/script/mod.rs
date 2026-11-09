 
 
 
 
 
 
 
 
 
 
 

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use rhai::{Engine, Scope, AST};

use crate::module::{KeyboardKey, ModuleCategory, ModuleSetting};

pub mod api;
pub mod packet_api;

use api::{ScriptSettings, SettingPatch};

 
 
 
pub const MAX_CONSECUTIVE_FAILURES: u32 = 40;

 
 
#[derive(Debug, Default, Clone)]
pub struct Callbacks {
    pub on_start: Option<String>,
    pub on_stop: Option<String>,
    pub on_tick: Option<String>,
    pub on_render: Option<String>,
    pub on_packet: Option<String>,
    pub on_world_inactive: Option<String>,
}

 
 
 
 
 
#[derive(Debug)]
pub struct ScriptedModule {
    name: String,
    description: String,
    category: ModuleCategory,
    key_bind: KeyboardKey,
     
     
     
    live: Arc<ScriptSettings>,
    engine: Engine,
    scope: Mutex<Scope<'static>>,
    ast: AST,
     
    enabled: Arc<AtomicBool>,
    failures: Arc<AtomicU32>,
    errors: Arc<Mutex<Vec<String>>>,
     
     
    packet_slot: packet_api::PacketSlot,
}

impl ScriptedModule {
     
     
    pub fn new(
        name: String,
        description: String,
        category: ModuleCategory,
        key_bind: KeyboardKey,
        settings: Vec<ModuleSetting>,
        callbacks: &Callbacks,
    ) -> anyhow::Result<Self> {
        let live = Arc::new(ScriptSettings::new(settings));
        let enabled = Arc::new(AtomicBool::new(false));
        let packet_slot: packet_api::PacketSlot = Arc::new(Mutex::new(None));

        let mut engine = Engine::new();
         
         
        engine.set_max_operations(2_000_000);
        engine.set_max_call_levels(32);
        engine.set_max_string_size(64 * 1024);
        engine.set_max_array_size(1024);
        engine.set_max_map_size(1024);
        engine.set_max_modules(32);
        api::register(&mut engine, Arc::clone(&live));
        packet_api::register(&mut engine, Arc::clone(&packet_slot));

        let source = compose(callbacks);
        let ast = engine
            .compile(&source)
            .map_err(|error| anyhow::anyhow!("{error}\n--- script ---\n{}", indent(&source)))?;

         
         
        {
            let mut scope = Scope::new();
            scope.push_constant("enabled", Arc::clone(&enabled));
            engine
                .run_ast_with_scope(&mut scope, &ast)
                .map_err(|error| anyhow::anyhow!("{error}\n--- script ---\n{}", indent(&source)))?;
        }

        Ok(Self {
            name,
            description,
            category,
            key_bind,
            live,
            engine,
            scope: Mutex::new(Scope::new()),
            ast,
            enabled,
            failures: Arc::new(AtomicU32::new(0)),
            errors: Arc::new(Mutex::new(Vec::new())),
            packet_slot,
        })
    }

     

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn category(&self) -> ModuleCategory {
        self.category
    }

     

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

     
     
     
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

     

    pub fn key_bind(&self) -> KeyboardKey {
        self.key_bind
    }

    pub fn set_key_bind(&mut self, key: KeyboardKey) {
        self.key_bind = key;
    }

     

     
    pub fn settings(&self) -> Vec<ModuleSetting> {
        self.live.snapshot()
    }

     
    pub fn settings_handle(&self) -> Arc<ScriptSettings> {
        Arc::clone(&self.live)
    }

     
    pub fn saved_settings(&self) -> Vec<SettingPatch> {
        self.live.snapshot().iter().map(SettingPatch::of).collect()
    }

     
     
    pub fn apply_setting(&self, name: &str, value: &SettingPatch) {
        self.live.apply(name, value);
    }

     

     
    pub fn is_broken(&self) -> bool {
        self.failures.load(Ordering::SeqCst) >= MAX_CONSECUTIVE_FAILURES
    }

     
    pub fn errors(&self) -> Vec<String> {
        self.errors
            .lock()
            .map(|log| log.clone())
            .unwrap_or_default()
    }

    fn record_error(&self, where_: &str, error: &anyhow::Error) {
        let message = format!("{where_}: {error}");
        if let Ok(mut log) = self.errors.lock() {
            log.push(message.clone());
             
             
            if log.len() > 8 {
                let excess = log.len() - 8;
                log.drain(..excess);
            }
        }
        let failures = self.failures.fetch_add(1, Ordering::SeqCst) + 1;
        if failures == 1 || failures % MAX_CONSECUTIVE_FAILURES == 0 {
            log::warn!(
                "addon '{}': {where_} failed ({failures} in a row): {error}",
                self.name
            );
        }
    }

     

    fn call(&self, function: &str) -> anyhow::Result<()> {
        let mut scope = match self.scope.lock() {
            Ok(scope) => scope,
            Err(poisoned) => poisoned.into_inner(),
        };
        self.engine
            .call_fn::<()>(&mut scope, &self.ast, function, ())
            .map_err(|error| anyhow::anyhow!("{error}"))
    }

    fn call_guarded(&self, function: &str) -> anyhow::Result<()> {
        match self.call(function) {
            Ok(()) => {
                self.failures.store(0, Ordering::SeqCst);
                if let Ok(mut log) = self.errors.lock() {
                    log.clear();
                }
                Ok(())
            }
            Err(error) => {
                self.record_error(function, &error);
                Err(error)
            }
        }
    }

     
    fn defines(&self, function: &str) -> bool {
        self.ast
            .iter_functions()
            .any(|definition| definition.name == function)
    }

     

    pub fn start(&self) -> anyhow::Result<()> {
        if self.defines("on_start") {
            self.call_guarded("on_start")
        } else {
            Ok(())
        }
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        if self.defines("on_stop") {
            self.call_guarded("on_stop")
        } else {
            Ok(())
        }
    }

    pub fn tick(&self) -> anyhow::Result<()> {
        if self.is_broken() || !self.defines("on_tick") {
            return Ok(());
        }
        self.call_guarded("on_tick")
    }

    pub fn render(&self) {
        if self.is_broken() || !self.defines("on_render") {
            return;
        }
         
         
        if let Err(error) = self.call("on_render") {
            self.record_error("on_render", &error);
        }
    }

    pub fn world_inactive(&self) -> anyhow::Result<()> {
        if self.defines("on_world_inactive") {
            self.call_guarded("on_world_inactive")
        } else {
            Ok(())
        }
    }

     
     
    pub fn handle_packet(&self, packet: &mut crate::net::packet::Packet) -> bool {
        if self.is_broken() || !self.defines("on_packet") {
            return false;
        }
        packet_api::dispatch(self, packet)
    }

     
    pub(crate) fn packet_slot(&self) -> &packet_api::PacketSlot {
        &self.packet_slot
    }

     
    pub(crate) fn call_script(&self, function: &str) -> bool {
        self.call_guarded(function).is_ok()
    }
}

 
pub type ScriptedHandle = Arc<Mutex<ScriptedModule>>;

 
 
 
fn compose(callbacks: &Callbacks) -> String {
    let mut source = String::new();
    for (name, body) in [
        ("on_start", &callbacks.on_start),
        ("on_stop", &callbacks.on_stop),
        ("on_tick", &callbacks.on_tick),
        ("on_render", &callbacks.on_render),
        ("on_packet", &callbacks.on_packet),
        ("on_world_inactive", &callbacks.on_world_inactive),
    ] {
        let body = body.as_deref().unwrap_or("").trim_end();
        if body.is_empty() {
             
             
            source.push_str(&format!("fn {name}() {{}}\n\n"));
            continue;
        }
        source.push_str(&format!("fn {name}() {{\n{body}\n}}\n\n"));
    }
    source
}

fn indent(source: &str) -> String {
    source
        .lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

 
#[derive(Debug, Clone)]
pub struct ScriptedView {
    pub name: String,
    pub description: String,
    pub category: ModuleCategory,
    pub key_bind: KeyboardKey,
    pub enabled: bool,
    pub settings: Vec<ModuleSetting>,
    pub broken: bool,
    pub errors: Vec<String>,
}

 
pub fn view(module: &ScriptedModule) -> ScriptedView {
    ScriptedView {
        name: module.name().to_string(),
        description: module.description().to_string(),
        category: module.category(),
        key_bind: module.key_bind(),
        enabled: module.is_enabled(),
        settings: module.settings(),
        broken: module.is_broken(),
        errors: module.errors(),
    }
}
