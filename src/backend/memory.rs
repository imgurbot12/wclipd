//! Memory Storage Backend for Clipboard Daemon
use std::collections::HashMap;

use crate::clipboard::{Entry, Preview};

use super::{Backend, BackendOpts, Record};

/// Memory Storage Backend for Clipboard Daemon
pub struct MemoryStore {
    options: BackendOpts,
    store: HashMap<usize, Record>,
    last_index: usize,
}

impl MemoryStore {
    /// Spawn New Memory Store Implementation
    pub fn new(options: BackendOpts) -> Self {
        Self {
            options,
            store: HashMap::new(),
            last_index: 0,
        }
    }
}

impl Backend for MemoryStore {
    /// Add new Clipboard Entry
    fn add(&mut self, entry: Record) -> usize {
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
    /// Check if Specified Clipboard Entry Already Exists
    fn exists(&self, entry: &Entry) -> Option<usize> {
        self.store
            .iter()
            .find(|(_, r)| r.entry.body == entry.body)
            .map(|(i, _)| *i)
    }
    /// Update an Existing Clipboard Entry
    fn update(&mut self, index: &usize) {
        if let Some(record) = self.store.get_mut(index) {
            record.update();
        };
    }
    /// Find an Existing Clipboard Entry by Index
    fn get(&self, index: usize) -> Option<&Record> {
        self.store.get(&index)
    }
    // Find Latest Entry from within Store
    fn latest(&self) -> Option<&Record> {
        let mut records: Vec<_> = self.store.values().collect();
        records.sort_by_key(|r| r.last_used);
        records.last().map(|r| *r)
    }
    /// List Clipboard Entries with Page/Limit
    fn list(&self) -> Vec<Preview> {
        self.store
            .iter()
            .map(|(i, r)| Preview {
                index: *i,
                preview: r.entry.preview(100),
                last_used: r.last_used,
            })
            .collect()
    }
}
