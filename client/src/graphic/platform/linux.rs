 

use std::ffi::{c_void, CString};
use std::sync::OnceLock;

use libloading::Library;
use log::info;

use super::HookTarget;

 
const GL_LIBRARY: &str = "libGL.so.1";

 
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
    let path = find_library_path("libglfw.so").unwrap_or_else(|| "libglfw.so".to_string());
     
    unsafe { Library::new(path).ok() }
}

 
pub fn frame_hook_targets() -> Vec<HookTarget> {
     
    let mut candidates: Vec<(String, &str)> = Vec::new();
    if let Some(path) = find_library_path("libglfw.so") {
        info!("found GLFW library: {path}");
        candidates.push((path, "glfwSwapBuffers"));
    } else if let Some(path) = find_library_path("liblwjgl.so") {
        info!("found LWJGL library (legacy): {path}");
        candidates.push((path, "glXSwapBuffers"));
    } else {
        info!("no specific GL library found; falling back to system libGL");
        candidates.push((GL_LIBRARY.to_string(), "glXSwapBuffers"));
    }

    candidates
        .into_iter()
        .filter_map(|(lib_path, func)| {
            let address = resolve_symbol(&lib_path, func)?;
            info!("found {func} in {lib_path} at 0x{address:x}");
            Some(HookTarget {
                address,
                label: lib_path,
            })
        })
        .collect()
}

 
fn resolve_symbol(lib_path: &str, func: &str) -> Option<usize> {
    let c_lib = CString::new(lib_path).ok()?;
    let c_func = CString::new(func).ok()?;
     
    unsafe {
        let handle = libc::dlopen(c_lib.as_ptr(), libc::RTLD_LAZY);
        if handle.is_null() {
            return None;
        }
        let address = libc::dlsym(handle, c_func.as_ptr()) as usize;
        (address != 0).then_some(address)
    }
}

 
 
fn find_library_path(partial_name: &str) -> Option<String> {
    use std::io::{BufRead, BufReader};

    let file = std::fs::File::open("/proc/self/maps").ok()?;
    for line in BufReader::new(file).lines().map_while(Result::ok) {
         
        if line.contains(partial_name) && line.contains(".so") {
            if let Some(path) = line.split_whitespace().last() {
                return Some(path.to_string());
            }
        }
    }
    None
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
