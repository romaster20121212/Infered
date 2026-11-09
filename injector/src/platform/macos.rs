 
 
 
 
 
 

use std::path::Path;

use super::{AgentInjector, InjectError};

 
#[derive(Debug, Default)]
pub struct PlatformInjector;

impl AgentInjector for PlatformInjector {
    fn is_agent_loaded(&self, _pid: u32, _agent_file: &str) -> bool {
        false
    }

    fn inject(&self, _pid: u32, _agent_path: &Path) -> Result<(), InjectError> {
        Err(InjectError::Unsupported)
    }
}

 
pub fn is_elevated() -> bool {
     
    unsafe { libc::geteuid() == 0 }
}
