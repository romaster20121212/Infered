 

use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use log::{error, info};
use protocol::Command;

use crate::{library, logging};

 
const READ_TIMEOUT: Duration = Duration::from_secs(5);

 
static CONFIG_DIR: Once = Once::new();

 
 
pub fn handle_connection(stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if let Err(e) = reader.read_line(&mut line) {
        error!("failed to read command: {e}");
        return;
    }

    match Command::decode(&line) {
        Ok(Command::Reload {
            library,
            config_dir,
        }) => {
             
             
             
             
            if !config_dir.as_os_str().is_empty() {
                 
                 
                 
                 
                 
                 
                 
                CONFIG_DIR.call_once(|| {
                    logging::init(&config_dir);
                    std::env::set_var("INFERED_CONFIG_DIR", &config_dir);
                });
            }
            info!("reload command received: {}", library.display());
            if let Err(e) = library::reload(&library) {
                error!("reload failed: {e}");
            }
        }
        Err(e) => error!("ignoring invalid command {:?}: {e}", line.trim()),
    }
}
