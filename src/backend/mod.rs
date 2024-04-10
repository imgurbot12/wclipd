//! Data Backends for Storing Clipboard History
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

mod memory;

use crate::message::{ClipboardEntry, ClipboardPreview};

// Exports
pub use memory::MemoryStore;

/// Clipboard Record Object
#[derive(Debug, Serialize, Deserialize)]
pub struct ClipboardRecord {
    pub entry: ClipboardEntry,
    pub entry_date: SystemTime,
}

impl ClipboardRecord {
    /// Create new Clipboard Record from Entry
    pub fn new(entry: ClipboardEntry) -> Self {
        Self {
            entry,
            entry_date: SystemTime::now(),
        }
    }
}

/// Storage Backend Abstraction Trait
pub trait Backend: Send + Sync {
    fn add(&mut self, entry: ClipboardRecord) -> usize;
    fn delete(&mut self, index: usize);
    fn clear(&mut self);
    fn update(&mut self, index: usize, entry: ClipboardRecord);
    fn find(&self, index: usize) -> Option<&ClipboardRecord>;
    fn list(&self) -> Vec<ClipboardPreview>;
}
