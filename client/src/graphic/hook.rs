 
 
 
 
 

use crate::graphic::platform;
use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::{gl, state, RUNNING};
use ilhook::x64::{CallbackOption, HookFlags, HookPoint, HookType, Hooker, Registers};
use log::info;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Instant;

static LAST_TICK: AtomicI32 = AtomicI32::new(0);
static GL_LOADED: AtomicBool = AtomicBool::new(false);
 
static INACTIVE_RENDER_CLEAN: AtomicBool = AtomicBool::new(false);

 
static IDLE_WARN_AT: AtomicU64 = AtomicU64::new(0);
 
static FRAME_CLOCK: OnceLock<Instant> = OnceLock::new();

 
 
 
 
 
fn warn_idle(reason: &str) {
    let now = FRAME_CLOCK.get_or_init(Instant::now).elapsed().as_secs();
    let last = IDLE_WARN_AT.load(Ordering::Relaxed);
    if now < last + 5 {
        return;
    }
    IDLE_WARN_AT.store(now, Ordering::Relaxed);
    log::warn!("modules are not ticking: {reason}");
}

 
 
pub struct HookHandle(HookPoint);

unsafe impl Send for HookHandle {}
unsafe impl Sync for HookHandle {}

 
 
static GLOBAL_HOOK: OnceLock<Mutex<Option<HookHandle>>> = OnceLock::new();

 
 
 
fn global_hook() -> MutexGuard<'static, Option<HookHandle>> {
    GLOBAL_HOOK
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

 

 
unsafe extern "win64" fn swap_buffers_hook(_registers: *mut Registers, _user_data: usize) {
    on_frame();
}

 
unsafe fn on_frame() {
     
     
    if !RUNNING.load(Ordering::SeqCst) {
        return;
    }

     
     
     
     
    crate::graphic::visual_silent::restore();

     
    if !GL_LOADED.load(Ordering::Relaxed) {
        gl::load_with(platform::gl_proc_address);
        GL_LOADED.store(true, Ordering::Relaxed);
    }

    check_tick();
     
     
    crate::graphic::input::init();
    crate::graphic::input::update_freelook_hold();
    if let Err(error) = crate::camera_bridge::update_scaffold_menu() {
        log::debug!("Scaffold input gate: {error}");
    }
     
     
    crate::module::combat::rotation::update();
    state::client().modules.render();
    crate::graphic::input::flush_deferred_mouse_actions();
     
     
     
    crate::macros::frame();
     
     
    crate::graphic::pause_menu::update();
     
     
    crate::graphic::motion_blur::apply();
    render_overlay();
     
    crate::graphic::saturation::apply();
}

 
unsafe fn render_overlay() {
     
     
     
    crate::graphic::input::init();

    crate::graphic::ui_engine::render_egui_ui();
}

 
 
fn check_tick() {
     
    if let Err(error) = state::env() {
        warn_idle(&format!(
            "this thread cannot be attached to the JVM: {error}"
        ));
        return;
    }

     
     
     
     
    let mc = state::minecraft();
    let in_world = matches!(mc.world(), Ok(Some(_)));
    let player = match mc.player() {
        Ok(Some(player)) => Some(player),
        Ok(None) | Err(_) => None,
    };
     
     
     
    if player.is_some() && !in_world {
        warn_idle("the player exists but no world is loaded");
    }
    let alive = player
        .as_ref()
        .is_some_and(|player| player.is_alive().unwrap_or(false));
    state::client().modules.set_active(in_world && alive);
    if !in_world || !alive {
         
         
        if let Some(handle) = state::client()
            .modules
            .get(crate::module::ModuleId::Freelook)
        {
            if let Ok(module) = handle.lock() {
                let _ = module.on_world_inactive();
            }
        }
        if !INACTIVE_RENDER_CLEAN.swap(true, Ordering::Relaxed) {
            state::client().modules.restore_inactive_render_state();
        }
        return;
    }
    INACTIVE_RENDER_CLEAN.store(false, Ordering::Relaxed);
    let Some(player) = player else {
        return;
    };

    let tick_count = match player.get_tick_count() {
        Ok(count) => count,
        Err(error) => {
            warn_idle(&format!("reading the player's tick count failed: {error}"));
            return;
        }
    };

     
     
     
     
     
     
     
    if tick_count != LAST_TICK.load(Ordering::Relaxed) {
        LAST_TICK.store(tick_count, Ordering::Relaxed);
         
        crate::net::ensure_installed();
         
        crate::macros::tick();
        state::client().modules.tick();
    }
}

 

 
 
fn store_hook(hook: HookPoint, label: &str) {
    let mut guard = global_hook();
    if guard.is_some() {
        info!("Replacing the existing buffer-swap hook.");
    }
    *guard = Some(HookHandle(hook));
    info!(">>> HOOK ACTIVE ON: {} <<<", label);
}

 
fn hooker_for(target_addr: usize) -> Hooker {
    Hooker::new(
        target_addr,
        HookType::JmpBack(swap_buffers_hook),
        CallbackOption::None,
        0,
        HookFlags::empty(),
    )
}

 
 
pub fn install_hooks() -> anyhow::Result<()> {
    for target in platform::frame_hook_targets() {
         
         
        let result = unsafe { hooker_for(target.address).hook() };
        match result {
            Ok(hook) => {
                store_hook(hook, &target.label);
                return Ok(());
            }
            Err(e) => info!("hook install failed on {}: {e:?}", target.label),
        }
    }
    Err(anyhow::anyhow!(
        "no buffer-swap target could be hooked on this platform"
    ))
}

 
pub fn uninstall_hooks() {
    let mut guard = global_hook();
    if guard.take().is_some() {
        info!("Buffer-swap hook removed; memory restored.");
    } else {
        info!("No active hook to remove.");
    }
}
