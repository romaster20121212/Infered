 
 
 
 
 

use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use log::info;
use protocol::{Command, SOCKET_ADDR};

use crate::platform::{AgentInjector, InjectError, PlatformInjector};

 
 
 
 
const AGENT_BASE: &str = "agent_loader";
const CLIENT_BASE: &str = "client";

 
const CONNECT_DEADLINE: Duration = Duration::from_secs(5);
 
const CONNECT_ATTEMPT_TIMEOUT: Duration = Duration::from_millis(500);
 
const CONNECT_RETRY_PAUSE: Duration = Duration::from_millis(100);
 
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);

 
 
pub fn inject(pid: u32) -> Result<(), InjectError> {
    let injector = PlatformInjector;

    let agent_file = library_file_name(AGENT_BASE);
    let agent_path = locate_library(&agent_file)?;
    let client_path = locate_library(&library_file_name(CLIENT_BASE))?;

    if injector.is_agent_loaded(pid, &agent_file) {
        info!("agent already present in pid {pid}; reloading client only");
    } else {
        injector.inject(pid, &agent_path)?;
    }

    send_reload(&client_path)
}

 
fn send_reload(client_lib: &Path) -> Result<(), InjectError> {
    let absolute = std::path::absolute(client_lib).map_err(InjectError::Path)?;
     
    let config_dir = std::env::current_dir().map_err(InjectError::Path)?;

    let mut stream = connect_with_retry()?;
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));

    let command = Command::Reload {
        library: absolute,
        config_dir,
    }
    .encode();
     
     
    info!("sending command: {}", command.trim_end());
    stream
        .write_all(command.as_bytes())
        .map_err(InjectError::Send)?;
    Ok(())
}

 
 
 
fn connect_with_retry() -> Result<TcpStream, InjectError> {
    let deadline = Instant::now() + CONNECT_DEADLINE;
    loop {
        match TcpStream::connect_timeout(&SOCKET_ADDR, CONNECT_ATTEMPT_TIMEOUT) {
            Ok(stream) => return Ok(stream),
            Err(source) => {
                if Instant::now() >= deadline {
                    return Err(InjectError::Connect {
                        addr: SOCKET_ADDR,
                        source,
                    });
                }
                thread::sleep(CONNECT_RETRY_PAUSE);
            }
        }
    }
}

 
fn library_file_name(base: &str) -> String {
    format!(
        "{}{}.{}",
        std::env::consts::DLL_PREFIX,
        base,
        std::env::consts::DLL_EXTENSION
    )
}

 
 
fn locate_library(file_name: &str) -> Result<PathBuf, InjectError> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(file_name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
     
     
     
     
    Err(InjectError::LibraryMissing(format!(
        "{file_name} beside injector executable"
    )))
}
