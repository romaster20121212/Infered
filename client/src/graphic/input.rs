 
 
 
 
 
 

use crate::state::{client, minecraft};
use libloading::Library;
use log::info;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

 

 
 
 
 
pub static PENDING_EVENTS: Mutex<Vec<egui::Event>> = Mutex::new(Vec::new());

fn push_event(event: egui::Event) {
    if let Ok(mut queue) = PENDING_EVENTS.lock() {
        queue.push(event);
    }
}

fn translate_mods(mods: i32) -> egui::Modifiers {
    let ctrl = mods & 0x2 != 0;
    egui::Modifiers {
        alt: mods & 0x4 != 0,
        ctrl,
        shift: mods & 0x1 != 0,
        mac_cmd: false,
         
        command: ctrl,
    }
}

 
 
fn translate_glfw_key(key: i32) -> Option<egui::Key> {
    use egui::Key::*;
    Some(match key {
        32 => Space,
        39 => Quote,
        44 => Comma,
        45 => Minus,
        46 => Period,
        47 => Slash,
        48 => Num0,
        49 => Num1,
        50 => Num2,
        51 => Num3,
        52 => Num4,
        53 => Num5,
        54 => Num6,
        55 => Num7,
        56 => Num8,
        57 => Num9,
        59 => Semicolon,
        61 => Equals,
        65 => A,
        66 => B,
        67 => C,
        68 => D,
        69 => E,
        70 => F,
        71 => G,
        72 => H,
        73 => I,
        74 => J,
        75 => K,
        76 => L,
        77 => M,
        78 => N,
        79 => O,
        80 => P,
        81 => Q,
        82 => R,
        83 => S,
        84 => T,
        85 => U,
        86 => V,
        87 => W,
        88 => X,
        89 => Y,
        90 => Z,
        91 => OpenBracket,
        92 => Backslash,
        93 => CloseBracket,
        96 => Backtick,
        256 => Escape,
        257 => Enter,
        258 => Tab,
        259 => Backspace,
        260 => Insert,
        261 => Delete,
        262 => ArrowRight,
        263 => ArrowLeft,
        264 => ArrowDown,
        265 => ArrowUp,
        266 => PageUp,
        267 => PageDown,
        268 => Home,
        269 => End,
         
        320 => Num0,
        321 => Num1,
        322 => Num2,
        323 => Num3,
        324 => Num4,
        325 => Num5,
        326 => Num6,
        327 => Num7,
        328 => Num8,
        329 => Num9,
        330 => Period,  
        331 => Slash,   
        333 => Minus,   
        334 => Plus,    
        335 => Enter,   
        336 => Equals,  
        290..=301 => match key - 289 {
            1 => F1,
            2 => F2,
            3 => F3,
            4 => F4,
            5 => F5,
            6 => F6,
            7 => F7,
            8 => F8,
            9 => F9,
            10 => F10,
            11 => F11,
            12 => F12,
            _ => return None,
        },
        _ => return None,
    })
}

 

 
 
pub static GUI_OPEN: AtomicBool = AtomicBool::new(false);

 
 
 
static CLOSE_GUI_REQUESTED: AtomicBool = AtomicBool::new(false);

 
pub fn request_gui_close() {
    CLOSE_GUI_REQUESTED.store(true, Ordering::Relaxed);
}

 
 
static PANIC_RECAPTURE_CURSOR: AtomicBool = AtomicBool::new(false);

 
 
pub fn close_gui_for_panic() {
    if GUI_OPEN.swap(false, Ordering::SeqCst) {
        PANIC_RECAPTURE_CURSOR.store(true, Ordering::SeqCst);
    }
}

 
 
static PENDING_MODULE_TOGGLES: Mutex<Vec<crate::module::ModuleId>> = Mutex::new(Vec::new());

 
pub fn request_module_toggle(id: crate::module::ModuleId) {
    if let Ok(mut queue) = PENDING_MODULE_TOGGLES.lock() {
        queue.push(id);
    }
}

 
 
 
pub fn close_gui_if_requested() {
    if CLOSE_GUI_REQUESTED.swap(false, Ordering::Relaxed) && GUI_OPEN.load(Ordering::Relaxed) {
        toggle_gui();
    }
    if TOGGLE_GUI_REQUESTED.swap(false, Ordering::Relaxed) {
        toggle_gui();
    }
    let queued = PENDING_MODULE_TOGGLES
        .lock()
        .map(|mut queue| std::mem::take(&mut *queue))
        .unwrap_or_default();
    for id in queued {
        toggle_module(id);
    }
}

 
pub static LAST_KEY_PRESSED: AtomicI32 = AtomicI32::new(-1);

 
 
pub static USER_CURSOR_MOVED: AtomicU64 = AtomicU64::new(0);

 
 
 
 
 
 
 
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MiddleClick {
    pub ctrl: bool,
    pub shift: bool,
}

 
const GLFW_MOD_SHIFT: i32 = 0x0001;
const GLFW_MOD_CONTROL: i32 = 0x0002;

 
 
 
const MAX_MIDDLE_CLICKS: usize = 8;
static MIDDLE_CLICKS: Mutex<Vec<MiddleClick>> = Mutex::new(Vec::new());

impl MiddleClick {
     
    pub fn from_mods(mods: i32) -> Self {
        Self {
            ctrl: mods & GLFW_MOD_CONTROL != 0,
            shift: mods & GLFW_MOD_SHIFT != 0,
        }
    }
}

 
fn push_middle_click(mods: i32) {
    if let Ok(mut queue) = MIDDLE_CLICKS.lock() {
        if queue.len() < MAX_MIDDLE_CLICKS {
            queue.push(MiddleClick::from_mods(mods));
        }
    }
}

 
pub fn take_middle_click() -> Option<MiddleClick> {
    MIDDLE_CLICKS.lock().ok().and_then(|mut queue| {
        if queue.is_empty() {
            None
        } else {
            Some(queue.remove(0))
        }
    })
}

 
 
pub fn clear_middle_clicks() {
    if let Ok(mut queue) = MIDDLE_CLICKS.lock() {
        queue.clear();
    }
}

 
 
static TOGGLE_GUI_REQUESTED: AtomicBool = AtomicBool::new(false);

 
pub fn request_gui_toggle() {
    TOGGLE_GUI_REQUESTED.store(true, Ordering::Relaxed);
}

 
 
 
 
static PLAYER_ACTION_ACTIVE: AtomicBool = AtomicBool::new(false);
 
static FREELOOK_HANDOFF_PENDING: AtomicBool = AtomicBool::new(false);
 
static FREELOOK_HANDOFF_HOLDING: AtomicBool = AtomicBool::new(false);
 
static USER_MOUSE_BUTTONS: AtomicU8 = AtomicU8::new(0);
 
static DEFERRED_MOUSE_BUTTONS: AtomicU8 = AtomicU8::new(0);
 
static DEFERRED_MOUSE_ACTIONS: Mutex<Vec<DeferredMouseAction>> = Mutex::new(Vec::new());

fn freelook_should_defer_action(owns_freelook: bool, menu_open: bool, freecam: bool) -> bool {
    owns_freelook && !menu_open && !freecam
}

fn freelook_handoff_pending(
    user_action_active: bool,
    held_button: bool,
    queued_buttons: u8,
) -> bool {
    user_action_active || held_button || queued_buttons != 0
}

#[derive(Clone, Copy)]
struct DeferredMouseAction {
    window: usize,
    button: i32,
    action: i32,
    mods: i32,
}
 
static INJECTED_MOUSE_BUTTONS: AtomicU8 = AtomicU8::new(0);

 
 
 
static INJECTED_MOUSE_MOVES: AtomicU64 = AtomicU64::new(0);

 
 
 
 
pub fn note_injected_mouse_move() {
     
     
     
    INJECTED_MOUSE_MOVES.store(1, Ordering::Relaxed);
}

 
 
fn injected_mouse_move_echo() -> bool {
    INJECTED_MOUSE_MOVES.swap(0, Ordering::Relaxed) != 0
}

 
 
pub fn note_injected_mouse_press() {
    INJECTED_MOUSE_BUTTONS.fetch_or(1, Ordering::Relaxed);
}

 
 
pub fn note_injected_right_mouse_press() {
    INJECTED_MOUSE_BUTTONS.fetch_or(2, Ordering::Relaxed);
}

 
 
pub fn note_injected_middle_mouse_press() {
    INJECTED_MOUSE_BUTTONS.fetch_or(4, Ordering::Relaxed);
}

 
pub fn note_injected_mouse_release() {
    INJECTED_MOUSE_BUTTONS.fetch_and(!1, Ordering::Relaxed);
}

 
pub fn user_action_active() -> bool {
    PLAYER_ACTION_ACTIVE.load(Ordering::Acquire)
}

 
pub fn player_action_active() -> bool {
    user_action_active() || FREELOOK_HANDOFF_PENDING.load(Ordering::Acquire)
}

fn injected_mouse_button_event(button: i32, action: i32) -> bool {
    let mask = match button {
        GLFW_MOUSE_BUTTON_LEFT => 1,
        GLFW_MOUSE_BUTTON_RIGHT => 2,
        GLFW_MOUSE_BUTTON_MIDDLE => 4,
        _ => return false,
    };
    match action {
        GLFW_PRESS => INJECTED_MOUSE_BUTTONS.load(Ordering::Relaxed) & mask != 0,
        GLFW_RELEASE => INJECTED_MOUSE_BUTTONS.fetch_and(!mask, Ordering::Relaxed) & mask != 0,
        _ => false,
    }
}

fn player_movement_key(key: i32) -> bool {
    matches!(
        key,
        FREE_KEY_W
            | FREE_KEY_A
            | FREE_KEY_S
            | FREE_KEY_D
            | FREE_KEY_SPACE
            | FREE_KEY_SHIFT
            | FREE_KEY_CTRL
    )
}

 
 
 
 
 
 
fn camera_handoff_active(user_mouse_buttons: u8) -> bool {
    user_mouse_buttons != 0
}

fn refresh_player_action_active() {
    PLAYER_ACTION_ACTIVE.store(
        camera_handoff_active(USER_MOUSE_BUTTONS.load(Ordering::Relaxed)),
        Ordering::Release,
    );
}

 
pub static KEYS_DOWN: Mutex<Vec<i32>> = Mutex::new(Vec::new());

 
 
pub static CLICK_TIMES_LEFT: Mutex<Vec<u128>> = Mutex::new(Vec::new());
pub static CLICK_TIMES_RIGHT: Mutex<Vec<u128>> = Mutex::new(Vec::new());

 
fn record_click(store: &Mutex<Vec<u128>>) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    if let Ok(mut times) = store.lock() {
        times.retain(|t| now.saturating_sub(*t) <= 1000);
        times.push(now);
    }
}

 
pub fn cps(left: bool) -> u32 {
    let store = if left {
        &CLICK_TIMES_LEFT
    } else {
        &CLICK_TIMES_RIGHT
    };
    store.lock().map(|times| times.len() as u32).unwrap_or(0)
}

 
pub fn key_down(key: i32) -> bool {
    KEYS_DOWN
        .lock()
        .map(|keys| keys.contains(&key))
        .unwrap_or(false)
}

 
pub static MOUSE_STATE: Mutex<MouseState> = Mutex::new(MouseState::NEW);

#[derive(Default, Clone, Copy)]
pub struct MouseState {
    pub x: f64,
    pub y: f64,
    pub left_down: bool,
    pub right_down: bool,
    pub left_clicked: bool,
    pub right_clicked: bool,
     
     
    pub scroll_x: f32,
    pub scroll_y: f32,
}

impl MouseState {
    const NEW: MouseState = MouseState {
        x: 0.0,
        y: 0.0,
        left_down: false,
        right_down: false,
        left_clicked: false,
        right_clicked: false,
        scroll_x: 0.0,
        scroll_y: 0.0,
    };
}

 

const GLFW_RELEASE: i32 = 0;
const GLFW_PRESS: i32 = 1;
const GLFW_REPEAT: i32 = 2;
const GLFW_MOUSE_BUTTON_LEFT: i32 = 0;
const GLFW_MOUSE_BUTTON_RIGHT: i32 = 1;
const GLFW_MOUSE_BUTTON_MIDDLE: i32 = 2;
const GLFW_KEY_RIGHT_SHIFT: i32 = 344;
const GLFW_KEY_ESCAPE: i32 = 256;

 

 
 
pub const FREE_KEY_W: i32 = 87;
pub const FREE_KEY_A: i32 = 65;
pub const FREE_KEY_S: i32 = 83;
pub const FREE_KEY_D: i32 = 68;
pub const FREE_KEY_SPACE: i32 = 32;
pub const FREE_KEY_SHIFT: i32 = 340;
pub const FREE_KEY_CTRL: i32 = 341;

const GLFW_CURSOR: i32 = 0x0003_3001;
const GLFW_CURSOR_NORMAL: i32 = 0x0003_4001;
const GLFW_CURSOR_DISABLED: i32 = 0x0003_4003;
const GLFW_FOCUSED: i32 = 0x0002_0001;

 

type MouseButtonFun = extern "C" fn(*mut c_void, i32, i32, i32);
type CursorPosFun = extern "C" fn(*mut c_void, f64, f64);
type KeyFun = extern "C" fn(*mut c_void, i32, i32, i32, i32);
type ScrollFun = extern "C" fn(*mut c_void, f64, f64);
type CharFun = extern "C" fn(*mut c_void, u32);

type GetCurrentContext = extern "C" fn() -> *mut c_void;
type SetMouseButtonCallback = extern "C" fn(*mut c_void, MouseButtonFun) -> *mut c_void;
type SetCursorPosCallback = extern "C" fn(*mut c_void, CursorPosFun) -> *mut c_void;
type SetKeyCallback = extern "C" fn(*mut c_void, KeyFun) -> *mut c_void;
type SetScrollCallback = extern "C" fn(*mut c_void, ScrollFun) -> *mut c_void;
type SetCharCallback = extern "C" fn(*mut c_void, CharFun) -> *mut c_void;
type SetInputMode = extern "C" fn(*mut c_void, i32, i32);
type SetCursorPos = extern "C" fn(*mut c_void, f64, f64);
type GetWindowAttrib = extern "C" fn(*mut c_void, i32) -> i32;

 

 
 
 
struct GlfwHooks {
    library: Library,
    window: *mut c_void,
    original_mouse_button: *mut c_void,
    original_cursor_pos: *mut c_void,
    original_key: *mut c_void,
    original_scroll: *mut c_void,
    original_char: *mut c_void,
    set_input_mode: SetInputMode,
    set_cursor_pos: SetCursorPos,
    get_window_attrib: GetWindowAttrib,
}

 
 
unsafe impl Send for GlfwHooks {}
unsafe impl Sync for GlfwHooks {}

static HOOKS: OnceLock<GlfwHooks> = OnceLock::new();

 
 
static CURSOR_LOCK_X: AtomicU64 = AtomicU64::new(0);
static CURSOR_LOCK_Y: AtomicU64 = AtomicU64::new(0);

 
 
 
pub static FREECAM_LOOK_DELTA: Mutex<(f64, f64)> = Mutex::new((0.0, 0.0));
static FREELOOK_LOOK_DELTA: Mutex<(f64, f64)> = Mutex::new((0.0, 0.0));
static FREELOOK_CURSOR_BASE: Mutex<Option<(f64, f64)>> = Mutex::new(None);

pub fn freelook_drop_in() {
     
     
    let _ = reset_vanilla_mouse();
    *FREELOOK_LOOK_DELTA
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = (0.0, 0.0);
    let position = MOUSE_STATE.lock().map(|state| (state.x, state.y)).ok();
    *FREELOOK_CURSOR_BASE
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = position;
}

pub fn freelook_take_look_delta() -> (f64, f64) {
    let mut delta = FREELOOK_LOOK_DELTA
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    std::mem::take(&mut *delta)
}

 
 
pub static FREECAM_SCROLL: Mutex<f64> = Mutex::new(0.0);
 
 
static FREECAM_CURSOR_BASE_X: AtomicU64 = AtomicU64::new(f64::NAN.to_bits());
static FREECAM_CURSOR_BASE_Y: AtomicU64 = AtomicU64::new(f64::NAN.to_bits());

fn cursor_lock() -> (f64, f64) {
    (
        f64::from_bits(CURSOR_LOCK_X.load(Ordering::Relaxed)),
        f64::from_bits(CURSOR_LOCK_Y.load(Ordering::Relaxed)),
    )
}

fn set_cursor_lock(x: f64, y: f64) {
    CURSOR_LOCK_X.store(x.to_bits(), Ordering::Relaxed);
    CURSOR_LOCK_Y.store(y.to_bits(), Ordering::Relaxed);
}

fn freecam_cursor_base() -> (f64, f64) {
    (
        f64::from_bits(FREECAM_CURSOR_BASE_X.load(Ordering::Relaxed)),
        f64::from_bits(FREECAM_CURSOR_BASE_Y.load(Ordering::Relaxed)),
    )
}

fn set_freecam_cursor_base(x: f64, y: f64) {
    FREECAM_CURSOR_BASE_X.store(x.to_bits(), Ordering::Relaxed);
    FREECAM_CURSOR_BASE_Y.store(y.to_bits(), Ordering::Relaxed);
}

 
 
pub fn freecam_drop_in() {
    if let Ok(mut delta) = FREECAM_LOOK_DELTA.lock() {
        *delta = (0.0, 0.0);
    }
    if let Ok(mut scroll) = FREECAM_SCROLL.lock() {
        *scroll = 0.0;
    }
    let (x, y) = MOUSE_STATE
        .lock()
        .map(|state| (state.x, state.y))
        .unwrap_or((0.0, 0.0));
    set_freecam_cursor_base(x, y);
}

 
 
pub fn freecam_take_look_delta() -> (f64, f64) {
    FREECAM_LOOK_DELTA
        .lock()
        .map(|mut delta| {
            let taken = *delta;
            *delta = (0.0, 0.0);
            taken
        })
        .unwrap_or((0.0, 0.0))
}

 
pub fn freecam_take_scroll() -> f64 {
    FREECAM_SCROLL
        .lock()
        .map(|mut scroll| {
            let taken = *scroll;
            *scroll = 0.0;
            taken
        })
        .unwrap_or(0.0)
}

 
 
pub fn game_window_focused() -> bool {
    let Some(hooks) = HOOKS.get() else {
        return false;
    };
    !hooks.window.is_null() && (hooks.get_window_attrib)(hooks.window, GLFW_FOCUSED) != 0
}

 

extern "C" fn on_mouse_button(window: *mut c_void, button: i32, action: i32, mods: i32) {
    let injected_event = injected_mouse_button_event(button, action);
     
    if !injected_event && matches!(action, GLFW_PRESS | GLFW_RELEASE) {
        let number = match button {
            GLFW_MOUSE_BUTTON_LEFT => Some(1),
            GLFW_MOUSE_BUTTON_RIGHT => Some(2),
            GLFW_MOUSE_BUTTON_MIDDLE => Some(3),
            _ => None,
        };
        if let Some(button) = number {
            crate::macros::push_event(crate::macros::engine::InputEvent::Mouse {
                button,
                pressed: action == GLFW_PRESS,
            });
        }
    }
    let freelook_owned_before_action = action == GLFW_PRESS
        && !injected_event
        && freelook_should_defer_action(
            crate::module::misc::freelook::owns_mouse(),
            menu_open(),
            freecam_enabled(),
        );
    let user_button = match button {
        GLFW_MOUSE_BUTTON_LEFT => Some(1),
        GLFW_MOUSE_BUTTON_RIGHT => Some(2),
        _ => None,
    };
    if let Some(mask) = user_button {
        if action == GLFW_PRESS && !injected_event {
            USER_MOUSE_BUTTONS.fetch_or(mask, Ordering::Relaxed);
        } else if action == GLFW_RELEASE && !injected_event {
            USER_MOUSE_BUTTONS.fetch_and(!mask, Ordering::Relaxed);
        }
        refresh_player_action_active();
        if !injected_event && matches!(action, GLFW_PRESS | GLFW_RELEASE) {
             
             
             
            FREELOOK_HANDOFF_HOLDING.store(
                USER_MOUSE_BUTTONS.load(Ordering::Relaxed) != 0,
                Ordering::Release,
            );
        }
    }
    if action == GLFW_PRESS && !injected_event && user_button.is_some() {
        USER_CURSOR_MOVED.fetch_add(1, Ordering::Relaxed);
    }

    if let Some(mask) = user_button {
        let deferred = DEFERRED_MOUSE_BUTTONS.load(Ordering::Relaxed);
        let should_defer = freelook_owned_before_action || deferred & mask != 0;
        if should_defer {
            if action == GLFW_PRESS {
                DEFERRED_MOUSE_BUTTONS.fetch_or(mask, Ordering::Relaxed);
                FREELOOK_HANDOFF_HOLDING.store(true, Ordering::Release);
                FREELOOK_HANDOFF_PENDING.store(true, Ordering::Release);
            } else if action == GLFW_RELEASE {
                 
                 
                DEFERRED_MOUSE_BUTTONS.fetch_and(!mask, Ordering::Relaxed);
            }
            if action != GLFW_REPEAT {
                if let Ok(mut actions) = DEFERRED_MOUSE_ACTIONS.lock() {
                    actions.push(DeferredMouseAction {
                        window: window as usize,
                        button,
                        action,
                        mods,
                    });
                }
            }
            return;
        }
    }

    if action == GLFW_PRESS && !injected_event && button == GLFW_MOUSE_BUTTON_MIDDLE {
        push_middle_click(mods);
        if crate::module::player::middleclick::swallows_vanilla_middle_click(mods) {
            return;
        }
    }

     
    if action == GLFW_PRESS && button == GLFW_MOUSE_BUTTON_LEFT {
        record_click(&CLICK_TIMES_LEFT);
    } else if action == GLFW_PRESS && button == GLFW_MOUSE_BUTTON_RIGHT {
        record_click(&CLICK_TIMES_RIGHT);
    }

    if let Ok(mut state) = MOUSE_STATE.lock() {
        match (button, action) {
            (GLFW_MOUSE_BUTTON_LEFT, GLFW_PRESS) => {
                state.left_down = true;
                state.left_clicked = true;
            }
            (GLFW_MOUSE_BUTTON_LEFT, GLFW_RELEASE) => state.left_down = false,
            (GLFW_MOUSE_BUTTON_RIGHT, GLFW_PRESS) => {
                state.right_down = true;
                state.right_clicked = true;
            }
            (GLFW_MOUSE_BUTTON_RIGHT, GLFW_RELEASE) => state.right_down = false,
            _ => {}
        }
    }

     
     
     
     
     
    if button == GLFW_MOUSE_BUTTON_RIGHT
        && action == GLFW_PRESS
        && !injected_event
        && crate::module::combat::crystal_macro::capture_right_click()
    {
         
         
        return;
    }
    if button == GLFW_MOUSE_BUTTON_RIGHT
        && (crate::module::combat::crystal_spam::owns_right_button()
            || crate::module::combat::interaction::owns_right_button())
    {
        return;
    }

     
     
     
    if GUI_OPEN.load(Ordering::Relaxed) || crate::graphic::edit_ui::is_open() || dev_takes_input() {
        return;
    }

     
     
     
     
     
    if !injected_event && button == GLFW_MOUSE_BUTTON_LEFT {
        if action == GLFW_PRESS && crate::module::combat::hitselect::take_click() {
            return;
        }
        if action == GLFW_RELEASE && crate::module::combat::hitselect::holds_click() {
            return;
        }
    }

    if let Some(hooks) = HOOKS.get() {
        if !hooks.original_mouse_button.is_null() {
            let original: MouseButtonFun =
                unsafe { std::mem::transmute(hooks.original_mouse_button) };
            original(window, button, action, mods);
        }
    }
}

extern "C" fn on_cursor_pos(window: *mut c_void, x: f64, y: f64) {
    let gui_open =
        GUI_OPEN.load(Ordering::Relaxed) || crate::graphic::edit_ui::is_open() || dev_takes_input();
    let freecam = freecam_enabled();
     
     
    let freecam_owns = freecam && !menu_open();
     
     
     
    let freelook_owns = !freecam
        && crate::module::misc::freelook::owns_mouse()
        && !player_action_active()
        && !menu_open();

     
     
     
    let injected = injected_mouse_move_echo();

    if !gui_open && !freecam && !freelook_owns && !injected {
        USER_CURSOR_MOVED.fetch_add(1, Ordering::Relaxed);
    }
    if let Ok(mut state) = MOUSE_STATE.lock() {
        state.x = x;
        state.y = y;
    }

     
     
     
     
    if freecam {
        let (base_x, base_y) = freecam_cursor_base();
        if freecam_owns {
            if let Ok(mut delta) = FREECAM_LOOK_DELTA.lock() {
                delta.0 += x - base_x;
                delta.1 += y - base_y;
            }
        }
        set_freecam_cursor_base(x, y);
    }

    {
        let mut base = FREELOOK_CURSOR_BASE
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if freelook_owns && !injected {
            let previous = base.unwrap_or_else(cursor_lock);
            if let Ok(mut delta) = FREELOOK_LOOK_DELTA.lock() {
                delta.0 += x - previous.0;
                delta.1 += y - previous.1;
            }
            *base = Some((x, y));
        } else if freelook_owns {
             
             
            *base = Some((x, y));
        } else if base.take().is_some() {
             
             
            let _ = reset_vanilla_mouse();
        }
    }

     
     
     
     
    let (send_x, send_y) = if gui_open || (freelook_owns && !injected) {
        cursor_lock()
    } else {
        set_cursor_lock(x, y);
        (x, y)
    };

    if let Some(hooks) = HOOKS.get() {
         
         
        if freecam_owns {
            return;
        }
        if !hooks.original_cursor_pos.is_null() {
            let original: CursorPosFun = unsafe { std::mem::transmute(hooks.original_cursor_pos) };
            original(window, send_x, send_y);
        }
    }
}

extern "C" fn on_key(window: *mut c_void, key: i32, scancode: i32, action: i32, mods: i32) {
     
     
    if matches!(action, GLFW_PRESS | GLFW_RELEASE) {
        crate::macros::push_event(crate::macros::engine::InputEvent::Key {
            glfw: key,
            pressed: action == GLFW_PRESS,
        });
    }
    if action == GLFW_PRESS {
        LAST_KEY_PRESSED.store(key, Ordering::Relaxed);
        if let Ok(mut keys) = KEYS_DOWN.lock() {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
    } else if action == GLFW_RELEASE {
        if let Ok(mut keys) = KEYS_DOWN.lock() {
            keys.retain(|k| *k != key);
        }
    }
    refresh_player_action_active();

     
     
    if action == GLFW_PRESS && PANIC_RECAPTURE_CURSOR.swap(false, Ordering::SeqCst) {
        apply_cursor_mode(false);
    }

    let gui_was_open = GUI_OPEN.load(Ordering::Relaxed);
    let editing = crate::graphic::edit_ui::is_open();
    let dev_open = dev_takes_input();
    if action == GLFW_PRESS && !gui_was_open && !editing && player_movement_key(key) {
        USER_CURSOR_MOVED.fetch_add(1, Ordering::Relaxed);
    }

     
     
     
     
    #[cfg(feature = "devtest")]
    if action == GLFW_PRESS && crate::graphic::dev_tester::handle_key(key, mods) {
        return;
    }
     
    #[cfg(feature = "devtest")]
    if action == GLFW_PRESS && key == GLFW_KEY_ESCAPE && crate::graphic::dev_popup::is_open() {
        crate::graphic::dev_popup::close();
        return;
    }

     
     
    if action == GLFW_PRESS && editing && key == GLFW_KEY_ESCAPE {
        crate::graphic::edit_ui::close();
        return;
    }

     
     
    if action == GLFW_PRESS
        && !editing
        && !dev_open
        && (key == menu_hud_key() || (key == GLFW_KEY_ESCAPE && gui_was_open))
    {
        toggle_gui();
    }

     
     
     
     
    if gui_was_open || editing || dev_open {
        if let Some(egui_key) = translate_glfw_key(key) {
            let pressed = action != GLFW_RELEASE;
            push_event(egui::Event::Key {
                key: egui_key,
                physical_key: None,
                pressed,
                repeat: action == GLFW_REPEAT,
                modifiers: translate_mods(mods),
            });
        }
    }

     
     
     
     
     
    if gui_was_open || GUI_OPEN.load(Ordering::Relaxed) || editing || dev_open {
        return;
    }

    if action == GLFW_PRESS || action == GLFW_RELEASE {
        handle_module_keybind(key, action);
    }

     
     
     
    if freecam_captures_key(key) {
        return;
    }

    if let Some(hooks) = HOOKS.get() {
        if !hooks.original_key.is_null() {
            let original: KeyFun = unsafe { std::mem::transmute(hooks.original_key) };
            original(window, key, scancode, action, mods);
        }
    }
}

 
 
 
extern "C" fn on_char(window: *mut c_void, codepoint: u32) {
    let gui_was_open = GUI_OPEN.load(Ordering::Relaxed);
    let editing = crate::graphic::edit_ui::is_open();
    let dev_open = dev_takes_input();

    if gui_was_open || editing || dev_open {
        if let Some(ch) = char::from_u32(codepoint) {
            push_event(egui::Event::Text(ch.to_string()));
        }
    }

    if gui_was_open || GUI_OPEN.load(Ordering::Relaxed) || editing || dev_open {
        return;
    }

    if let Some(hooks) = HOOKS.get() {
        if !hooks.original_char.is_null() {
            let original: CharFun = unsafe { std::mem::transmute(hooks.original_char) };
            original(window, codepoint);
        }
    }
}

extern "C" fn on_scroll(window: *mut c_void, x_offset: f64, y_offset: f64) {
     
     
     
    if GUI_OPEN.load(Ordering::Relaxed) || crate::graphic::edit_ui::is_open() || dev_takes_input() {
        if let Ok(mut state) = MOUSE_STATE.lock() {
            state.scroll_x += x_offset as f32;
            state.scroll_y += y_offset as f32;
        }
        return;
    }

     
     
    if freecam_owns_input() {
        if let Ok(mut scroll) = FREECAM_SCROLL.lock() {
            *scroll += y_offset;
        }
        return;
    }

    if let Some(hooks) = HOOKS.get() {
        if !hooks.original_scroll.is_null() {
            let original: ScrollFun = unsafe { std::mem::transmute(hooks.original_scroll) };
            original(window, x_offset, y_offset);
        }
    }
}

 
 
fn menu_hud_key() -> i32 {
    crate::state::client()
        .modules
        .get(crate::module::ModuleId::MenuHud)
        .and_then(|handle| {
            let module = handle.lock().ok()?;
            Some(module.get_module_data().key_bind as i32)
        })
        .filter(|key| *key >= 0)
        .unwrap_or(GLFW_KEY_RIGHT_SHIFT)
}

 
fn toggle_gui() {
     
     
    if !crate::RUNNING.load(Ordering::SeqCst) {
        return;
    }
     
    let open = !GUI_OPEN.fetch_xor(true, Ordering::Relaxed);
    if let Err(error) = crate::camera_bridge::update_scaffold_menu() {
        log::warn!("Scaffold menu gate: {error}");
    }

     
     
    if let Some(handle) = client().modules.get(crate::module::ModuleId::MenuHud) {
        if let Ok(mut module) = handle.lock() {
            module.get_module_data_mut().set_enabled(open);
        }
    }

     
    if !open {
        crate::config::save();
    } else {
         
         
        crate::graphic::edit_ui::close();
    }

     
     
    apply_cursor_mode(open || crate::graphic::edit_ui::is_open() || dev_popup_open());
}

 
 
pub fn close_gui() {
    if GUI_OPEN.load(Ordering::Relaxed) {
        toggle_gui();
    }
}

 
 
 
 
 
pub(crate) fn dev_popup_open() -> bool {
    #[cfg(feature = "devtest")]
    { crate::graphic::dev_popup::is_open() }
    #[cfg(not(feature = "devtest"))]
    { false }
}

pub(crate) fn dev_takes_input() -> bool {
    #[cfg(feature = "devtest")]
    { dev_popup_open() || crate::graphic::dev_tester::is_typing() }
    #[cfg(not(feature = "devtest"))]
    { false }
}

 
 
 
 
#[cfg(feature = "devtest")]
pub(crate) fn set_dev_cursor(open: bool) {
    if open && !crate::RUNNING.load(Ordering::SeqCst) {
        return;
    }
    apply_cursor_mode(
        open || GUI_OPEN.load(Ordering::Relaxed) || crate::graphic::edit_ui::is_open(),
    );
}

 
 
 
pub(crate) fn set_editor_cursor(open: bool) {
     
     
    if open && !crate::RUNNING.load(Ordering::SeqCst) {
        return;
    }
    apply_cursor_mode(open || GUI_OPEN.load(Ordering::Relaxed));
}

 
 
fn apply_cursor_mode(open: bool) {
    let Some(hooks) = HOOKS.get() else {
        return;
    };
    if hooks.window.is_null() {
        return;
    }

    if open {
         
        (hooks.set_input_mode)(hooks.window, GLFW_CURSOR, GLFW_CURSOR_NORMAL);
    } else if minecraft_screen_open() {
         
         
        (hooks.set_input_mode)(hooks.window, GLFW_CURSOR, GLFW_CURSOR_NORMAL);
    } else {
         
         
         
        let (lock_x, lock_y) = cursor_lock();
        (hooks.set_cursor_pos)(hooks.window, lock_x, lock_y);
        (hooks.set_input_mode)(hooks.window, GLFW_CURSOR, GLFW_CURSOR_DISABLED);
    }
}

fn freecam_captures_key(key: i32) -> bool {
    let movement_key = matches!(
        key,
        FREE_KEY_W
            | FREE_KEY_A
            | FREE_KEY_S
            | FREE_KEY_D
            | FREE_KEY_SPACE
            | FREE_KEY_SHIFT
            | FREE_KEY_CTRL
    );
    movement_key && freecam_owns_input()
}

 
 
 
fn freecam_enabled() -> bool {
    client()
        .modules
        .get(crate::module::ModuleId::Freecam)
        .and_then(|handle| {
            handle
                .lock()
                .ok()
                .map(|module| module.get_module_data().enabled)
        })
        .unwrap_or(false)
}

 
 
 
pub fn toggle_module(id: crate::module::ModuleId) {
    let Some(handle) = client().modules.get(id) else {
        return;
    };
     
     
     
    {
        let Ok(mut module) = handle.lock() else {
            return;
        };
        let next = !module.get_module_data().enabled;
        module.get_module_data_mut().set_enabled(next);
        if client().modules.is_active() {
            let result = if next {
                module.on_start()
            } else {
                module.on_stop()
            };
            if let Err(error) = result {
                log::warn!("{} toggle failed: {error}", module.get_module_data().name());
            }
        }
    }
    crate::config::save();
}

 
fn handle_module_keybind(key: i32, action: i32) {
    let minecraft = minecraft();
    if !minecraft.current_screen_is_null() || !minecraft.in_world() {
        return;
    }

    let mut changed = false;
    for handle in client().modules.handles() {
        let Ok(mut module) = handle.lock() else {
            continue;
        };
        if module.get_module_data().key_bind as i32 != key {
            continue;
        }

        let data = module.get_module_data();
        let hold = data.id == crate::module::ModuleId::Freelook
            && crate::module::misc::freelook::hold_mode(data);
        let Some(next) = keybind_next(data.enabled, hold, action) else {
            continue;
        };
        info!(
            "{} {}",
            module.get_module_data().name(),
            if next { "enabled" } else { "disabled" }
        );
        module.get_module_data_mut().set_enabled(next);
         
         
         
         
         
         
        if client().modules.is_active() {
            let result = if next {
                module.on_start()
            } else {
                module.on_stop()
            };
            if let Err(error) = result {
                log::warn!(
                    "{} keybind transition failed: {error}",
                    module.get_module_data().name()
                );
                if next && module.get_module_data().id == crate::module::ModuleId::Freelook {
                    module.get_module_data_mut().enabled = false;
                    let _ = module.on_stop();
                }
            }
        }
        changed = true;
    }

     
    if changed {
        crate::config::save();
    }
}

fn keybind_next(enabled: bool, hold: bool, action: i32) -> Option<bool> {
    let next = match (hold, action) {
        (true, GLFW_PRESS) => true,
        (true, GLFW_RELEASE) => false,
        (false, GLFW_PRESS) => !enabled,
        _ => return None,
    };
    (next != enabled).then_some(next)
}

fn reset_vanilla_mouse() -> anyhow::Result<()> {
    use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
    crate::state::mapping().in_frame(|| {
        let mouse = minecraft()
            .get_field(
                "mouseHandler",
                FieldType::Object(MinecraftClassType::MouseHandler),
            )?
            .l()?;
        crate::state::mapping().call_method(
            MinecraftClassType::MouseHandler,
            &mouse,
            "setIgnoreFirstMove",
            &[],
        )?;
        for field in ["accumulatedDX", "accumulatedDY"] {
            crate::state::mapping().set_field(
                MinecraftClassType::MouseHandler,
                &mouse,
                field,
                FieldType::Double,
                jni::objects::JValue::Double(0.0),
            )?;
        }
        Ok(())
    })
}

 
 
pub fn update_freelook_hold() {
    let Some(handle) = client().modules.get(crate::module::ModuleId::Freelook) else {
        return;
    };
    let mut changed = false;
    if let Ok(mut module) = handle.lock() {
        let data = module.get_module_data();
        if data.enabled
            && crate::module::misc::freelook::hold_mode(data)
            && (!key_down(data.key_bind as i32) || !game_window_focused() || menu_open())
        {
            module.get_module_data_mut().enabled = false;
            if let Err(error) = module.on_stop() {
                log::warn!("Freelook release: {error}");
            }
            changed = true;
        }
    }
    if changed {
        crate::config::save();
    }
}

 

 
 
 
pub fn init() {
    if HOOKS.get().is_some() {
        return;
    }
    if let Some(hooks) = install_glfw_hooks() {
        let _ = HOOKS.set(hooks);
    }
}

 
 
fn install_glfw_hooks() -> Option<GlfwHooks> {
    let library = crate::graphic::platform::open_glfw_library()?;

    unsafe {
        let get_context = *library
            .get::<GetCurrentContext>(b"glfwGetCurrentContext")
            .ok()?;
        let set_mouse_button = *library
            .get::<SetMouseButtonCallback>(b"glfwSetMouseButtonCallback")
            .ok()?;
        let set_cursor_pos_cb = *library
            .get::<SetCursorPosCallback>(b"glfwSetCursorPosCallback")
            .ok()?;
        let set_key = *library.get::<SetKeyCallback>(b"glfwSetKeyCallback").ok()?;
        let set_scroll = *library
            .get::<SetScrollCallback>(b"glfwSetScrollCallback")
            .ok()?;
        let set_char = *library
            .get::<SetCharCallback>(b"glfwSetCharCallback")
            .ok()?;
        let set_input_mode = *library.get::<SetInputMode>(b"glfwSetInputMode").ok()?;
        let set_cursor_pos = *library.get::<SetCursorPos>(b"glfwSetCursorPos").ok()?;
        let get_window_attrib = *library
            .get::<GetWindowAttrib>(b"glfwGetWindowAttrib")
            .ok()?;

        let window = get_context();
        if window.is_null() {
            return None;
        }

        info!("GLFW window acquired; installing input callbacks.");

        let original_mouse_button = set_mouse_button(window, on_mouse_button);
        let original_cursor_pos = set_cursor_pos_cb(window, on_cursor_pos);
        let original_key = set_key(window, on_key);
        let original_scroll = set_scroll(window, on_scroll);
        let original_char = set_char(window, on_char);

         
        if GUI_OPEN.load(Ordering::Relaxed) {
            set_input_mode(window, GLFW_CURSOR, GLFW_CURSOR_NORMAL);
        }

        Some(GlfwHooks {
            library,
            window,
            original_mouse_button,
            original_cursor_pos,
            original_key,
            original_scroll,
            original_char,
            set_input_mode,
            set_cursor_pos,
            get_window_attrib,
        })
    }
}

 
 
 
pub fn flush_deferred_mouse_actions() {
    if !FREELOOK_HANDOFF_PENDING.load(Ordering::Acquire) {
        return;
    }
    if menu_open() || freecam_enabled() {
         
         
        if let Ok(mut actions) = DEFERRED_MOUSE_ACTIONS.lock() {
            actions.clear();
        }
        DEFERRED_MOUSE_BUTTONS.store(0, Ordering::Relaxed);
        let still_holding = freelook_handoff_pending(
            PLAYER_ACTION_ACTIVE.load(Ordering::Acquire),
            USER_MOUSE_BUTTONS.load(Ordering::Relaxed) != 0,
            0,
        );
        FREELOOK_HANDOFF_HOLDING.store(
            USER_MOUSE_BUTTONS.load(Ordering::Relaxed) != 0,
            Ordering::Release,
        );
        FREELOOK_HANDOFF_PENDING.store(still_holding, Ordering::Release);
        return;
    }
    {
        if crate::module::misc::freelook::owns_mouse() {
            return;
        }
        let actions = DEFERRED_MOUSE_ACTIONS
            .lock()
            .map(|mut actions| std::mem::take(&mut *actions))
            .unwrap_or_default();
        if let Some(hooks) = HOOKS.get() {
            if !hooks.original_mouse_button.is_null() {
                let original: MouseButtonFun =
                    unsafe { std::mem::transmute(hooks.original_mouse_button) };
                for event in actions {
                    if event.action == GLFW_PRESS && event.button == GLFW_MOUSE_BUTTON_LEFT {
                        record_click(&CLICK_TIMES_LEFT);
                    } else if event.action == GLFW_PRESS && event.button == GLFW_MOUSE_BUTTON_RIGHT
                    {
                        record_click(&CLICK_TIMES_RIGHT);
                    }
                    if let Ok(mut mouse) = MOUSE_STATE.lock() {
                        match (event.button, event.action) {
                            (GLFW_MOUSE_BUTTON_LEFT, GLFW_PRESS) => {
                                mouse.left_down = true;
                                mouse.left_clicked = true;
                            }
                            (GLFW_MOUSE_BUTTON_LEFT, GLFW_RELEASE) => mouse.left_down = false,
                            (GLFW_MOUSE_BUTTON_RIGHT, GLFW_PRESS) => {
                                mouse.right_down = true;
                                mouse.right_clicked = true;
                            }
                            (GLFW_MOUSE_BUTTON_RIGHT, GLFW_RELEASE) => mouse.right_down = false,
                            _ => {}
                        }
                    }
                    if event.action == GLFW_PRESS {
                        let mask = if event.button == GLFW_MOUSE_BUTTON_LEFT {
                            1
                        } else {
                            2
                        };
                        DEFERRED_MOUSE_BUTTONS.fetch_and(!mask, Ordering::Relaxed);
                    }
                    original(
                        event.window as *mut c_void,
                        event.button,
                        event.action,
                        event.mods,
                    );
                }
            }
        }
        let still_holding = freelook_handoff_pending(
            PLAYER_ACTION_ACTIVE.load(Ordering::Acquire),
            FREELOOK_HANDOFF_HOLDING.load(Ordering::Acquire),
            DEFERRED_MOUSE_BUTTONS.load(Ordering::Relaxed),
        );
        FREELOOK_HANDOFF_PENDING.store(still_holding, Ordering::Release);
    }
}

 
 
pub fn cleanup() {
    PLAYER_ACTION_ACTIVE.store(false, Ordering::Release);
    FREELOOK_HANDOFF_PENDING.store(false, Ordering::Release);
    FREELOOK_HANDOFF_HOLDING.store(false, Ordering::Release);
    USER_MOUSE_BUTTONS.store(0, Ordering::Relaxed);
    DEFERRED_MOUSE_BUTTONS.store(0, Ordering::Relaxed);
    INJECTED_MOUSE_BUTTONS.store(0, Ordering::Relaxed);
    if let Ok(mut actions) = DEFERRED_MOUSE_ACTIONS.lock() {
        actions.clear();
    }
    let Some(hooks) = HOOKS.get() else {
        return;
    };
    if hooks.window.is_null() {
        return;
    }

    type RestoreCallback = extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    let restorations: [(&[u8], *mut c_void); 5] = [
        (b"glfwSetMouseButtonCallback", hooks.original_mouse_button),
        (b"glfwSetCursorPosCallback", hooks.original_cursor_pos),
        (b"glfwSetKeyCallback", hooks.original_key),
        (b"glfwSetScrollCallback", hooks.original_scroll),
        (b"glfwSetCharCallback", hooks.original_char),
    ];
    unsafe {
        for (name, original) in restorations {
            if let Ok(restore) = hooks.library.get::<RestoreCallback>(name) {
                restore(hooks.window, original);
            }
        }
    }

     
     
     
     
    let cursor_mode = if minecraft_screen_open() {
        GLFW_CURSOR_NORMAL
    } else {
        GLFW_CURSOR_DISABLED
    };
    (hooks.set_input_mode)(hooks.window, GLFW_CURSOR, cursor_mode);
    info!("GLFW input callbacks restored.");
}

 
 
 
pub fn minecraft_screen_open() -> bool {
    !minecraft().screen_is_null()
}

 
 
 
 
 
 
 
pub fn menu_open() -> bool {
    GUI_OPEN.load(Ordering::Relaxed)
        || crate::graphic::edit_ui::is_open()
        || minecraft_screen_open()
}

 
 
 
 
 
 
 
 
 
 
fn freecam_owns_input() -> bool {
    freecam_enabled() && !menu_open()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freelook_toggle_hold_and_repeat_edges() {
        assert_eq!(keybind_next(false, false, GLFW_PRESS), Some(true));
        assert_eq!(keybind_next(true, false, GLFW_PRESS), Some(false));
        assert_eq!(keybind_next(true, false, GLFW_RELEASE), None);
        assert_eq!(keybind_next(false, true, GLFW_PRESS), Some(true));
        assert_eq!(keybind_next(true, true, GLFW_PRESS), None);
        assert_eq!(keybind_next(true, true, GLFW_RELEASE), Some(false));
        assert_eq!(keybind_next(false, true, GLFW_REPEAT), None);
    }

     
     
     
    #[test]
    fn movement_keys_keep_the_freelook_orbit_attached() {
        assert!(!camera_handoff_active(0));
        assert!(camera_handoff_active(1));
        assert!(camera_handoff_active(2));
        for key in [
            FREE_KEY_W,
            FREE_KEY_A,
            FREE_KEY_S,
            FREE_KEY_D,
            FREE_KEY_SPACE,
            FREE_KEY_SHIFT,
            FREE_KEY_CTRL,
        ] {
            assert!(player_movement_key(key), "{key} is a movement key");
        }
        assert!(!player_movement_key(GLFW_KEY_ESCAPE));
    }

    #[test]
    fn handoff_lasts_until_the_physical_mouse_action_ends() {
        assert!(freelook_handoff_pending(false, true, 0));
        assert!(freelook_handoff_pending(true, false, 0));
        assert!(freelook_handoff_pending(false, false, 1));
        assert!(!freelook_handoff_pending(false, false, 0));
    }

    #[test]
    fn physical_clicks_handoff_only_from_the_freelook_game_view() {
        assert!(freelook_should_defer_action(true, false, false));
        assert!(!freelook_should_defer_action(false, false, false));
        assert!(!freelook_should_defer_action(true, true, false));
        assert!(!freelook_should_defer_action(true, false, true));
    }

    #[test]
    fn only_an_injected_mouse_move_is_swallowed_as_our_own_echo() {
        assert!(!injected_mouse_move_echo(), "nothing injected yet");
        note_injected_mouse_move();
        assert!(
            injected_mouse_move_echo(),
            "the injection's own echo is ours"
        );
        assert!(!injected_mouse_move_echo(), "the mark is consumed once");
         
         
        note_injected_mouse_move();
        note_injected_mouse_move();
        assert!(injected_mouse_move_echo());
        assert!(!injected_mouse_move_echo());
    }
}
