use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

pub const SOCKET_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 7878);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Reload {
        library: PathBuf,
        config_dir: PathBuf,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("unknown or malformed command")]
    Invalid,
}

impl Command {
    pub fn encode(&self) -> String {
        match self {
            Command::Reload {
                library,
                config_dir,
            } => {
                format!("reload {}\t{}\n", library.display(), config_dir.display())
            }
        }
    }

    pub fn decode(line: &str) -> Result<Self, DecodeError> {
        let line = line.trim_end_matches(['\r', '\n']);
        let args = line.strip_prefix("reload ").ok_or(DecodeError::Invalid)?;
        let (library, config_dir) = args.split_once('\t').unwrap_or((args, ""));
        if library.is_empty() || config_dir.contains('\t') || args.contains(['\r', '\n']) {
            return Err(DecodeError::Invalid);
        }
        Ok(Self::Reload {
            library: PathBuf::from(library),
            config_dir: PathBuf::from(config_dir),
        })
    }
}

pub fn init_file_logger(
    path: impl AsRef<Path>,
    level: log::LevelFilter,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    simplelog::WriteLogger::init(level, simplelog::Config::default(), file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_round_trips_spaces_and_windows_paths() {
        let cmd = Command::Reload {
            library: PathBuf::from(r"C:\Program Files\Infered\client.dll"),
            config_dir: PathBuf::from(r"C:\Users\Example\Infered"),
        };
        assert_eq!(Command::decode(&cmd.encode()).unwrap(), cmd);
    }

    #[test]
    fn decodes_legacy_command_and_rejects_bad_input() {
        assert_eq!(
            Command::decode("reload client.dll\n").unwrap(),
            Command::Reload {
                library: "client.dll".into(),
                config_dir: PathBuf::new()
            }
        );
        assert!(Command::decode("reload \n").is_err());
        assert!(Command::decode("other file\n").is_err());
        assert!(Command::decode("reload a\tb\tc\n").is_err());
    }
}
