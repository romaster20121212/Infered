 
 
 
 

mod command;
mod jvm;
mod library;
mod logging;
mod platform;
mod server;

use std::sync::atomic::{AtomicBool, Ordering};

use ctor::{ctor, dtor};
use log::info;

 
static RUNNING: AtomicBool = AtomicBool::new(true);

 
pub(crate) fn is_running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

 
#[ctor]
fn agent_onload() {
     
     
    info!("agent loader initialized");

    platform::install_signal_handlers();
    jvm::start_monitor();
    server::start();
}

 
#[dtor]
fn agent_onunload() {
    shutdown();
}

 
 
pub(crate) fn shutdown() {
    if !RUNNING.swap(false, Ordering::SeqCst) {
        return;  
    }
    info!("agent loader shutting down");
    library::shutdown();
}
