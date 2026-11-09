 

use std::thread;
use std::time::Duration;

use jni::sys::{jsize, JNI_GetCreatedJavaVMs, JNI_OK};
use jni::JavaVM;
use log::info;

use crate::is_running;

 
const POLL_INTERVAL: Duration = Duration::from_millis(500);
 
const MAX_FAILURES: u32 = 3;

 
pub fn start_monitor() {
    thread::spawn(monitor);
}

 
 
fn monitor() {
    info!("jvm monitor started");

    let Some(jvm) = wait_for_jvm() else {
        return;
    };
    info!("jvm detected; monitoring health");

    let mut failures = 0u32;
    while is_running() {
        thread::sleep(POLL_INTERVAL);

        if jvm_healthy(&jvm) {
            failures = 0;
            continue;
        }

        failures += 1;
        info!("jvm health check failed ({failures}/{MAX_FAILURES})");
        if failures >= MAX_FAILURES {
            info!("jvm appears to be gone; shutting the agent down");
            crate::shutdown();
            break;
        }
    }

    info!("jvm monitor stopped");
}

 
fn wait_for_jvm() -> Option<JavaVM> {
    while is_running() {
        if let Some(jvm) = get_jvm() {
            return Some(jvm);
        }
        thread::sleep(POLL_INTERVAL);
    }
    None
}

 
 
fn jvm_healthy(jvm: &JavaVM) -> bool {
    if jvm.get_java_vm_pointer().is_null() {
        return false;
    }
    match jvm.attach_current_thread_as_daemon() {
        Ok(mut env) => env.find_class("java/lang/System").is_ok(),
        Err(_) => false,
    }
}

 
fn get_jvm() -> Option<JavaVM> {
    let mut raw: *mut jni::sys::JavaVM = std::ptr::null_mut();
    let mut count: jsize = 0;

     
     
    unsafe {
        if JNI_GetCreatedJavaVMs(&mut raw, 1, &mut count) != JNI_OK || count == 0 {
            return None;
        }
        JavaVM::from_raw(raw).ok()
    }
}
