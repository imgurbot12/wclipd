//! Data Backends for Storing Clipboard History
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use serde::{de::Error, Deserialize, Serialize};

mod disk;
mod memory;

use crate::clipboard::{Entry, Preview};
use crate::{DEFAULT_DISK_STORE, XDG_PREFIX};

// Exports
pub use disk::Disk;
pub use memory::Memory;

/// Backend Storage Options Available
#[derive(Debug, Clone)]
pub enum Storage {
    Disk(PathBuf),
    Memory,
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
pub enum Lifetime {
    Never,
    OnLogin,
    OnReboot,
    Duration(Duration),
}

impl Lifetime {
    /// Caluclate Fixed Expiration Date for Records (if Applicable)
    pub fn fixed_expr(&self) -> Option<SystemTime> {
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
    pub fn dyn_expr(&self) -> Option<SystemTime> {
        match self {
            Self::Duration(duration) => Some(SystemTime::now() - *duration),
            _ => None,
        }
    }
}

impl Display for Lifetime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Never => write!(f, "never"),
            Self::OnLogin => write!(f, "login"),
            Self::OnReboot => write!(f, "reboot"),
            Self::Duration(d) => write!(f, "{}", d.as_secs()),
        }
    }
}

impl FromStr for Lifetime {
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

impl<'de> Deserialize<'de> for Lifetime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        Lifetime::from_str(s).map_err(D::Error::custom)
    }
}

/// Clipboard Record Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub entry: Entry,
    pub last_used: SystemTime,
}

impl Record {
    /// Create new Clipboard Record from Entry
    pub fn new(entry: Entry) -> Self {
        Self {
            entry,
            last_used: SystemTime::now(),
        }
    }
    /// Update LastUsed Datetime on Record
    pub fn update(&mut self) {
        self.last_used = SystemTime::now();
    }
}

/// Backend Initialization Options
#[derive(Debug, Clone)]
pub struct BackendOpts {
    pub backend: Storage,
    pub lifetime: Lifetime,
    pub max_entries: Option<usize>,
}

/// Return Valid Backend Implementation based on Requested Settings
impl BackendOpts {
    pub fn build(self) -> Box<dyn Backend> {
        match self.backend.clone() {
            Storage::Disk(path) => Box::new(Disk::new(path, self)),
            Storage::Memory => Box::new(Memory::new(self)),
        }
    }
}

/// Storage Backend Abstraction Trait
pub trait Backend: Send + Sync {
    fn add(&mut self, entry: Record) -> usize;
    fn get(&mut self, index: usize) -> Option<Record>;
    fn latest(&mut self) -> Option<Record>;
    fn exists(&mut self, entry: &Entry) -> Option<usize>;
    fn update(&mut self, index: &usize);
    fn delete(&mut self, index: usize);
    fn clear(&mut self);
    fn list(&mut self, preview_size: usize) -> Vec<Preview>;
}

impl dyn Backend {
    /// Find Entry with Index (if Specified)
    pub fn find(&mut self, index: Option<usize>) -> Option<Record> {
        match index {
            Some(idx) => self.get(idx),
            None => self.latest(),
        }
    }
    /// Organize List of Previews before Showing
    pub fn preview(&mut self, preview_size: usize) -> Vec<Preview> {
        let mut previews = self.list(preview_size);
        previews.sort_by(|a, b| {
            let first = b.last_used.cmp(&a.last_used);
            let second = b.index.cmp(&a.index);
            first.then(second)
        });
        previews
    }
    /// Add/Update Entry in Database
    pub fn push(&mut self, entry: Entry) -> usize {
        match self.exists(&entry) {
            Some(idx) => {
                self.update(&idx);
                idx
            }
            None => {
                let record = Record::new(entry);
                self.add(record)
            }
        }
    }
}
