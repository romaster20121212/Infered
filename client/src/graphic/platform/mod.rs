 
 
 
 
 
 

use std::ffi::c_void;

use libloading::Library;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod imp;
#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod imp;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
#[path = "macos.rs"]
mod imp;

 
 
pub struct HookTarget {
     
    pub address: usize,
     
    pub label: String,
}

 
 
pub fn gl_proc_address(name: &str) -> *const c_void {
    imp::gl_proc_address(name)
}

 
pub fn open_glfw_library() -> Option<Library> {
    imp::open_glfw_library()
}

 
pub fn frame_hook_targets() -> Vec<HookTarget> {
    imp::frame_hook_targets()
}

 

 
 
 
 
 
 
pub fn move_mouse_by(dx: i32, dy: i32) -> bool {
    imp::move_mouse_by(dx, dy)
}

 
 
pub fn left_mouse_down() -> bool {
    imp::left_mouse_down()
}

 
 
pub fn left_mouse_up() -> bool {
    imp::left_mouse_up()
}

 
pub fn left_click() -> bool {
    imp::left_click()
}

 
pub fn right_click() -> bool {
    imp::right_click()
}

 
pub fn middle_click() -> bool {
    imp::middle_click()
}
