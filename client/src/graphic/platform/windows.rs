 

use std::ffi::{c_void, CString};
use std::sync::OnceLock;

use libloading::Library;
use log::info;

use super::HookTarget;

 
const GL_LIBRARY: &str = "opengl32.dll";

 
fn gl_library() -> Option<&'static Library> {
    static LIB: OnceLock<Option<Library>> = OnceLock::new();
     
    LIB.get_or_init(|| unsafe { Library::new(GL_LIBRARY).ok() })
        .as_ref()
}

 
pub fn gl_proc_address(name: &str) -> *const c_void {
    let Some(lib) = gl_library() else {
        return std::ptr::null();
    };
     
    unsafe {
         
         
        if let Ok(c_name) = CString::new(name) {
            type WglGetProcAddress = unsafe extern "system" fn(*const i8) -> *const c_void;
            if let Ok(wgl) = lib.get::<WglGetProcAddress>(b"wglGetProcAddress") {
                let ptr = wgl(c_name.as_ptr());
                if !ptr.is_null() {
                    return ptr;
                }
            }
        }
        match lib.get::<unsafe extern "system" fn()>(name.as_bytes()) {
            Ok(symbol) => *symbol as *const c_void,
            Err(_) => std::ptr::null(),
        }
    }
}

 
pub fn open_glfw_library() -> Option<Library> {
     
    unsafe {
        Library::new("glfw.dll")
            .or_else(|_| Library::new("glfw3.dll"))
            .or_else(|_| Library::new("glfw64.dll"))
            .ok()
    }
}

 
 
 
 
 
 
pub fn frame_hook_targets() -> Vec<HookTarget> {
     
     
    unsafe {
        let Some(library) = open_glfw_library() else {
            return Vec::new();
        };
        let address = match library.get::<unsafe extern "system" fn()>(b"glfwSwapBuffers") {
            Ok(symbol) => *symbol as *const () as usize,
            Err(_) => return Vec::new(),
        };
        info!("found glfwSwapBuffers at 0x{address:x}");
         
        std::mem::forget(library);
        vec![HookTarget {
            address,
            label: "glfwSwapBuffers".to_string(),
        }]
    }
}

 

 
#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

 
#[repr(C)]
#[derive(Clone, Copy)]
struct MouseInput {
    dx: i32,
    dy: i32,
    mouse_data: u32,
    flags: u32,
    time: u32,
    extra_info: usize,
}

 
 
#[repr(C)]
#[derive(Clone, Copy)]
struct MouseEvent {
    kind: u32,
    _padding: u32,
    mouse: MouseInput,
}

const INPUT_MOUSE: u32 = 0;
const MOUSEEVENTF_MOVE: u32 = 0x0001;
const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;
const MOUSEEVENTF_MIDDLEDOWN: u32 = 0x0020;
const MOUSEEVENTF_MIDDLEUP: u32 = 0x0040;
const GA_ROOT: u32 = 2;

type GetForegroundWindow = unsafe extern "system" fn() -> isize;
type WindowFromPoint = unsafe extern "system" fn(Point) -> isize;
type GetAncestor = unsafe extern "system" fn(isize, u32) -> isize;
type GetCursorPos = unsafe extern "system" fn(*mut Point) -> i32;
type SendInputFn = unsafe extern "system" fn(u32, *const MouseEvent, i32) -> u32;

 
 
fn user32() -> Option<&'static Library> {
    static LIB: OnceLock<Option<Library>> = OnceLock::new();
     
    LIB.get_or_init(|| unsafe { Library::new("user32.dll").ok() })
        .as_ref()
}

 
 
 
 
fn mouse_may_act() -> bool {
    if !crate::graphic::input::game_window_focused() || crate::graphic::input::menu_open() {
        return false;
    }
    let Some(lib) = user32() else {
        return false;
    };
     
    unsafe {
        let (Ok(get_foreground), Ok(window_from_point), Ok(get_ancestor), Ok(get_cursor_pos)) = (
            lib.get::<GetForegroundWindow>(b"GetForegroundWindow"),
            lib.get::<WindowFromPoint>(b"WindowFromPoint"),
            lib.get::<GetAncestor>(b"GetAncestor"),
            lib.get::<GetCursorPos>(b"GetCursorPos"),
        ) else {
            return false;
        };
        let foreground = get_foreground();
        if foreground == 0 {
            return false;
        }
        let mut point = Point { x: 0, y: 0 };
        if get_cursor_pos(&mut point) == 0 {
            return false;
        }
        let hit = window_from_point(point);
        hit != 0 && get_ancestor(hit, GA_ROOT) == foreground
    }
}

 
 
fn send_events(events: &[MouseEvent]) -> bool {
    if events.is_empty() {
        return false;
    }
    let Some(lib) = user32() else {
        return false;
    };
     
    unsafe {
        let Ok(send_input) = lib.get::<SendInputFn>(b"SendInput") else {
            return false;
        };
        let sent = send_input(
            events.len() as u32,
            events.as_ptr(),
            std::mem::size_of::<MouseEvent>() as i32,
        );
        sent == events.len() as u32
    }
}

fn mouse_event(flags: u32, dx: i32, dy: i32) -> MouseEvent {
    MouseEvent {
        kind: INPUT_MOUSE,
        _padding: 0,
        mouse: MouseInput {
            dx,
            dy,
            mouse_data: 0,
            flags,
            time: 0,
            extra_info: 0,
        },
    }
}

 
 
pub fn move_mouse_by(dx: i32, dy: i32) -> bool {
    if dx == 0 && dy == 0 {
        return true;
    }
    if !mouse_may_act() {
        return false;
    }
    if !send_events(&[mouse_event(MOUSEEVENTF_MOVE, dx, dy)]) {
        return false;
    }
     
     
     
     
     
    crate::graphic::input::note_injected_mouse_move();
    true
}

 
pub fn left_mouse_down() -> bool {
    if !mouse_may_act() {
        return false;
    }
    crate::graphic::input::note_injected_mouse_press();
    if send_events(&[mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0)]) {
        true
    } else {
        crate::graphic::input::note_injected_mouse_release();
        false
    }
}

 
 
pub fn left_mouse_up() -> bool {
    crate::graphic::input::note_injected_mouse_release();
    send_events(&[mouse_event(MOUSEEVENTF_LEFTUP, 0, 0)])
}

 
pub fn left_click() -> bool {
    if !mouse_may_act() {
        return false;
    }
    crate::graphic::input::note_injected_mouse_press();
    send_events(&[
        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0),
        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0),
    ])
}

 
pub fn right_click() -> bool {
    if !mouse_may_act() {
        return false;
    }
    crate::graphic::input::note_injected_right_mouse_press();
    send_events(&[
        mouse_event(MOUSEEVENTF_RIGHTDOWN, 0, 0),
        mouse_event(MOUSEEVENTF_RIGHTUP, 0, 0),
    ])
}

 
pub fn middle_click() -> bool {
    if !mouse_may_act() {
        return false;
    }
    crate::graphic::input::note_injected_middle_mouse_press();
    send_events(&[
        mouse_event(MOUSEEVENTF_MIDDLEDOWN, 0, 0),
        mouse_event(MOUSEEVENTF_MIDDLEUP, 0, 0),
    ])
}
