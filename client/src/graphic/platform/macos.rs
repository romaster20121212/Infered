 
 
 
 
 
 
 

use std::ffi::c_void;
use std::sync::OnceLock;

use libloading::Library;
use log::warn;

use super::HookTarget;

 
const GL_LIBRARY: &str = "/System/Library/Frameworks/OpenGL.framework/Versions/Current/OpenGL";

 
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
        match lib.get::<unsafe extern "system" fn()>(name.as_bytes()) {
            Ok(symbol) => *symbol as *const c_void,
            Err(_) => std::ptr::null(),
        }
    }
}

 
pub fn open_glfw_library() -> Option<Library> {
     
    unsafe {
        Library::new("libglfw.3.dylib")
            .or_else(|_| Library::new("libglfw.dylib"))
            .ok()
    }
}

 
pub fn frame_hook_targets() -> Vec<HookTarget> {
    warn!("frame hooking is not implemented on macOS; the overlay will not install");
    Vec::new()
}

 

 
 
pub fn move_mouse_by(_dx: i32, _dy: i32) -> bool {
    warn_no_real_mouse();
    false
}

 
pub fn left_mouse_down() -> bool {
    warn_no_real_mouse();
    false
}

 
pub fn left_mouse_up() -> bool {
    warn_no_real_mouse();
    false
}

 
pub fn left_click() -> bool {
    warn_no_real_mouse();
    false
}

 
pub fn right_click() -> bool {
    warn_no_real_mouse();
    false
}

 
pub fn middle_click() -> bool {
    warn_no_real_mouse();
    false
}

fn warn_no_real_mouse() {
    static WARNED: std::sync::Once = std::sync::Once::new();
    WARNED.call_once(|| log::warn!("real mouse injection is only implemented on Windows"));
}
