//! Memory Storage Backend for Clipboard Daemon
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::clipboard::{Entry, Preview};

use super::{Backend, BackendOpts, Record};

/// Memory Storage Backend for Clipboard Daemon
pub struct MemoryStore {
    options: BackendOpts,
    store: HashMap<usize, Record>,
    last_index: usize,
    fixed_expr: Option<SystemTime>,
}

impl MemoryStore {
    /// Spawn New Memory Store Implementation
    pub fn new(options: BackendOpts) -> Self {
        let fixed_expr = options.lifetime.fixed_expr();
        Self {
            options,
            store: HashMap::new(),
            last_index: 0,
            fixed_expr,
        }
    }
    /// Remove Expired Items from Store
    fn clean(&mut self) {
        // remove expired entries
        let before = self.store.len();
        let expr_1 = self.fixed_expr.unwrap_or(UNIX_EPOCH);
        let expr_2 = self.options.lifetime.dyn_expr().unwrap_or(UNIX_EPOCH);
        let mut records: Vec<_> = self
            .store
            .clone()
            .into_iter()
            .filter(|(_, r)| r.last_used > expr_1 && r.last_used > expr_2)
            .collect();
        // sort records by earliest to eldest and remove eldest entries
        records.sort_by(|(_, r1), (_, r2)| r2.last_used.cmp(&r1.last_used));
        if let Some(max) = self.options.max_entries {
            while records.len() > max {
                let _ = records.pop();
            }
        }
        let after = records.len();
        if before > after {
            log::debug!("memory store deleted {} entries", before - after);
        }
        self.store = records.into_iter().collect();
    }
}

impl Backend for MemoryStore {
    /// Add new Clipboard Entry
    fn add(&mut self, entry: Record) -> usize {
        self.clean();
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
    fn exists(&mut self, entry: &Entry) -> Option<usize> {
        self.clean();
        self.store
            .iter()
            .find(|(_, r)| r.entry.body == entry.body)
            .map(|(i, _)| *i)
    }
    /// Update an Existing Clipboard Entry
    fn update(&mut self, index: &usize) {
        self.clean();
        if let Some(record) = self.store.get_mut(index) {
            record.update();
        };
    }
    /// Find an Existing Clipboard Entry by Index
    fn get(&mut self, index: usize) -> Option<&Record> {
        self.clean();
        self.store.get(&index)
    }
    /// Find Latest Entry from within Store
    fn latest(&mut self) -> Option<&Record> {
        self.clean();
        let mut records: Vec<_> = self.store.values().collect();
        records.sort_by_key(|r| r.last_used);
        records.last().map(|r| *r)
    }
    /// List Clipboard Entries with Page/Limit
    fn list(&mut self) -> Vec<Preview> {
        self.clean();
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
