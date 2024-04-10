//! Memory Storage Backend for Clipboard Daemon
use std::collections::HashMap;

use crate::message::ClipboardPreview;

use super::{Backend, ClipboardRecord};

/// Memory Storage Backend for Clipboard Daemon
pub struct MemoryStore {
    store: HashMap<usize, ClipboardRecord>,
    last_index: usize,
}

impl MemoryStore {
    /// Spawn New Memory Store Implementation
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
            last_index: 0,
        }
    }
}

impl Backend for MemoryStore {
    /// Add new Clipboard Entry
    fn add(&mut self, entry: ClipboardRecord) -> usize {
        self.last_index += 1;
        self.store.insert(self.last_index, entry);
        self.last_index
    }
    /// Delete Existing Clipboard Entry from Storage
    fn delete(&mut self, index: usize) {
        self.store.remove(&index);
    }
    /// Delete All Clipboard Records from MemoryStore
    fn clear(&mut self) {
        self.store.clear();
    }
    /// Update an Existing Clipboard Entry
    fn update(&mut self, index: usize, entry: ClipboardRecord) {
        self.store.insert(index, entry);
    }
    /// Find an Existing Clipboard Entry
    fn find(&self, index: usize) -> Option<&ClipboardRecord> {
        self.store.get(&index)
    }
    /// List Clipboard Entries with Page/Limit
    fn list(&self) -> Vec<ClipboardPreview> {
        self.store
            .iter()
            .map(|(i, e)| ClipboardPreview {
                index: *i,
                preview: format!("{:?}", e.entry),
            })
            .collect()
    }
}
