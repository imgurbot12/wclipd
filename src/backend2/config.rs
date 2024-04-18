//! Configuration Settings for Backend Implementations

use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use serde::{de::Error, Deserialize};

use super::backend::Backend;
use super::store_kv::Kv;
use super::store_memory::Memory;

use crate::{DEFAULT_DISK_STORE, XDG_PREFIX};

fn disk_default() -> PathBuf {
    xdg::BaseDirectories::with_prefix(XDG_PREFIX)
        .expect("Failed to read xdg base dirs")
        .get_cache_file(DEFAULT_DISK_STORE)
}

/// Backend Configuration Settings
pub type BackendConfig = HashMap<String, CategoryConfig>;

/// Backend Category Configuration Settings
#[derive(Debug, Clone)]
pub struct CategoryConfig {
    pub storage: Storage,
    pub expiration: Expiration,
    pub max_entries: Option<usize>,
}

impl Default for CategoryConfig {
    fn default() -> Self {
        Self {
            storage: Storage::Disk(disk_default()),
            expiration: Expiration::OnReboot,
            max_entries: None,
        }
    }
}

/// Backend Storage Options Available
#[derive(Debug, Clone)]
pub enum Storage {
    Disk(PathBuf),
    Memory,
}

impl Storage {
    pub fn backend(&self) -> Box<dyn Backend> {
        match self {
            Storage::Disk(path) => Box::new(Kv::new(path.to_owned())),
            Storage::Memory => Box::new(Memory::new()),
        }
    }
}

impl FromStr for Storage {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "memory" => Ok(Self::Memory),
            "disk" => {
                let path = xdg::BaseDirectories::with_prefix(XDG_PREFIX)
                    .expect("Failed to read xdg base dirs")
                    .get_cache_file(DEFAULT_DISK_STORE);
                Ok(Self::Disk(path))
            }
            path => {
                let path = PathBuf::from_str(&path)
                    .map_err(|_| format!("invalid storate option: {s:?}"))?;
                Ok(Self::Disk(path))
            }
        }
    }
}

impl Display for Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disk(path) => write!(f, "{path:?}"),
            Self::Memory => write!(f, "memory"),
        }
    }
}

impl<'de> Deserialize<'de> for Storage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        Storage::from_str(s).map_err(D::Error::custom)
    }
}

/// Cache Lifetime for Storage Backend
#[derive(Debug, Clone)]
pub enum Expiration {
    Never,
    OnLogin,
    OnReboot,
    Duration(Duration),
}

impl Expiration {
    fn fixed_expiration(&self) -> Option<SystemTime> {
        match self {
            Self::Never => None,
            Self::Duration(_) => None,
            Self::OnLogin => match lastlog::search_self() {
                Ok(record) => record.last_login.into(),
                Err(err) => {
                    log::error!("failed last-login check: {err:?}");
                    None
                }
            },
            Self::OnReboot => match lastlog::system_boot() {
                Ok(uptime) => uptime.last_login.into(),
                Err(err) => {
                    log::error!("failed last-reboot check: {err:?}");
                    None
                }
            },
        }
    }
    /// Runtime Check if Timestamp is Past Expiration
    fn dynanmic_expriration(&self) -> Option<SystemTime> {
        match self {
            Self::Duration(duration) => Some(SystemTime::now() - *duration),
            _ => None,
        }
    }
}

impl Display for Expiration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Never => write!(f, "never"),
            Self::OnLogin => write!(f, "login"),
            Self::OnReboot => write!(f, "reboot"),
            Self::Duration(d) => write!(f, "{}", d.as_secs()),
        }
    }
}

impl FromStr for Expiration {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "never" => Ok(Self::Never),
            "login" | "onlogin" => Ok(Self::OnLogin),
            "reboot" | "onreboot" => Ok(Self::OnReboot),
            _ => {
                let seconds: u64 = s.parse().map_err(|_| format!("invalid lifetime: {s:?}"))?;
                Ok(Self::Duration(Duration::from_secs(seconds)))
            }
        }
    }
}

impl<'de> Deserialize<'de> for Expiration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        Expiration::from_str(s).map_err(D::Error::custom)
    }
}
