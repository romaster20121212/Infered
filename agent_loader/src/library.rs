 

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use libloading::{Library, Symbol};
use log::{error, info, warn};

 
type Result<T> = std::result::Result<T, Box<dyn Error>>;

 
static CLIENT_LIBRARY: OnceLock<Mutex<Option<Library>>> = OnceLock::new();

 
const TEMP_CLEANUP_DELAY: Duration = Duration::from_secs(5);

 
 
fn lock() -> MutexGuard<'static, Option<Library>> {
    CLIENT_LIBRARY
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| {
            error!("client-library mutex was poisoned; recovering");
            poisoned.into_inner()
        })
}

 
 
 
 
 
pub fn reload(source: &Path) -> Result<()> {
    info!("reloading client library from {}", source.display());
    let temp = copy_to_temp(source)?;
    schedule_temp_cleanup(temp.clone());
    load(&temp)
}

 
 
pub fn shutdown() {
    if lock().take().is_some() {
        info!("client library dropped");
    }
}

 
 
 
 
 
 
fn load(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(format!("client library not found at {}", path.display()).into());
    }

    let mut slot = lock();

    if let Some(old) = slot.take() {
         
        unsafe { call_export(&old, b"cleanup_client") };
        drop(old);
        info!("previous client library unloaded");
    }

    info!("loading client library {}", path.display());
     
    let library = unsafe { Library::new(path)? };
     
    unsafe { call_export(&library, b"initialize_client") };

    *slot = Some(library);
    info!("client library loaded");
    Ok(())
}

 
 
 
 
unsafe fn call_export(library: &Library, symbol: &[u8]) {
    match library.get::<Symbol<extern "C" fn()>>(symbol) {
        Ok(func) => {
            let name = String::from_utf8_lossy(symbol);
            info!("calling {name}");
            func();
        }
        Err(_) => {
            let name = String::from_utf8_lossy(symbol);
            info!("{name} not exported; skipping");
        }
    }
}

 
fn copy_to_temp(source: &Path) -> Result<PathBuf> {
    let file_name = source
        .file_name()
        .ok_or("client library path has no file name")?
        .to_string_lossy()
        .into_owned();

     
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let mut temp = std::env::temp_dir();
    temp.push(format!("infered_{stamp}_{file_name}"));

    std::fs::copy(source, &temp)?;
    info!("client library copied to {}", temp.display());
    Ok(temp)
}

 
fn schedule_temp_cleanup(temp: PathBuf) {
    thread::spawn(move || {
        thread::sleep(TEMP_CLEANUP_DELAY);
        if let Err(e) = std::fs::remove_file(&temp) {
            warn!("could not remove temporary library {}: {e}", temp.display());
        }
    });
}
