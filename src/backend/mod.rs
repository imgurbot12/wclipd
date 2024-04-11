//! Data Backends for Storing Clipboard History
use std::fmt::Display;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

mod memory;

use crate::clipboard::{Entry, Preview};

// Exports
pub use memory::MemoryStore;

/// Backend Storage Options Available
#[derive(Debug, Clone)]
pub enum Storage {
    Memory,
}

impl FromStr for Storage {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "memory" => Ok(Self::Memory),
            _ => Err(format!("invalid storage option: {s:?}")),
        }
    }
}

impl Display for Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Memory => write!(f, "memory"),
        }
    }
}

/// Clipboard Record Object
#[derive(Debug, Serialize, Deserialize)]
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
    pub max_entries: Option<usize>,
    pub lifetime: Option<Duration>,
}

/// Return Valid Backend Implementation based on Requested Settings
impl BackendOpts {
    pub fn build(self) -> Box<dyn Backend> {
        Box::new(match self.backend {
            Storage::Memory => MemoryStore::new(self),
        })
    }
}

/// Storage Backend Abstraction Trait
pub trait Backend: Send + Sync {
    fn add(&mut self, entry: Record) -> usize;
    fn get(&self, index: usize) -> Option<&Record>;
    fn latest(&self) -> Option<&Record>;
    fn exists(&self, entry: &Entry) -> Option<usize>;
    fn update(&mut self, index: &usize);
    fn delete(&mut self, index: usize);
    fn clear(&mut self);
    fn list(&self) -> Vec<Preview>;
}

impl dyn Backend {
    /// Find Entry with Index (if Specified)
    pub fn find(&self, index: Option<usize>) -> Option<&Record> {
        match index {
            Some(idx) => self.get(idx),
            None => self.latest(),
        }
    }
    /// Organize List of Previews before Showing
    pub fn preview(&self) -> Vec<Preview> {
        let mut previews = self.list();
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
