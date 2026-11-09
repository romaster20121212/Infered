 
 
 
 
 

mod discovery;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod imp;
#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod imp;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
#[path = "macos.rs"]
mod imp;

pub use discovery::{find_minecraft_processes, ProcessInfo};
pub use imp::PlatformInjector;

use std::error::Error;
use std::net::SocketAddr;
use std::path::Path;

use thiserror::Error;

 
 
 
pub type BoxError = Box<dyn Error + Send + Sync>;

 
 
#[derive(Debug, Error)]
pub enum InjectError {
     
    #[error("process {0} is no longer running")]
    ProcessGone(u32),
     
     
     
     
    #[allow(dead_code)]
    #[error("could not attach to process {pid}: {source}")]
    Attach { pid: u32, source: BoxError },
     
    #[error("agent injection failed: {0}")]
    Inject(BoxError),
     
    #[error("library not found next to the injector or in the working directory: {0}")]
    LibraryMissing(String),
     
    #[error("could not reach the agent on {addr}: {source}")]
    Connect {
        addr: SocketAddr,
        source: std::io::Error,
    },
     
    #[error("failed to send the reload command: {0}")]
    Send(std::io::Error),
     
    #[error("could not resolve an absolute library path: {0}")]
    Path(std::io::Error),
     
     
     
    #[allow(dead_code)]
    #[error("agent injection is not supported on this platform yet")]
    Unsupported,
}

 
pub trait AgentInjector {
     
     
    fn is_agent_loaded(&self, pid: u32, agent_file: &str) -> bool;

     
    fn inject(&self, pid: u32, agent_path: &Path) -> Result<(), InjectError>;
}

 
pub fn is_elevated() -> bool {
    imp::is_elevated()
}
