//! Disk Storage Backend for Clipboard Daemon

use std::{
    collections::HashMap,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use kv::{Config, Integer, Json, Store};

use super::{Backend, BackendOpts, Record};
use crate::clipboard::Preview;

type Bucket<'a> = kv::Bucket<'a, Integer, Json<Record>>;

/// Disk Clipboard Storage Implementation
pub struct Disk {
    store: kv::Store,
    options: BackendOpts,
    last_index: usize,
    fixed_expr: Option<SystemTime>,
}

impl Disk {
    /// Spawn new KV Disk Storage Backend
    pub fn new(path: PathBuf, options: BackendOpts) -> Self {
        let fixed_expr = options.lifetime.fixed_expr();
        let config = Config::new(path);
        let store = Store::new(config).expect("failed to open kv store");
        Self {
            store,
            options,
            fixed_expr,
            last_index: 0,
        }
    }
    /// Retrieve Default Bucket for Storing KV Records
    fn bucket<'a>(&self) -> Bucket<'a> {
        self.store.bucket(None).expect("failed to open kv bucket")
    }
    /// Iterate Records Contained within Bucket
    fn records<'a>(&self, bucket: Option<Bucket<'a>>) -> impl Iterator<Item = (usize, Record)> {
        let bucket = bucket.unwrap_or_else(|| self.bucket());
        bucket.iter().filter_map(|r| r.ok()).map(|i| {
            let key: Integer = i.key().expect("failed kv key lookup");
            let value: Json<Record> = i.value().expect("failed kv value lookup");
            (usize::from(key), value.0)
        })
    }
    /// Remove Expired Items from Store
    fn clean(&mut self) {
        // remove expired entries
        let bucket = self.bucket();
        let before = bucket.len();
        let expr_1 = self.fixed_expr.unwrap_or(UNIX_EPOCH);
        let expr_2 = self.options.lifetime.dyn_expr().unwrap_or(UNIX_EPOCH);
        let mut records: Vec<_> = self
            .records(Some(bucket))
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
        // delete expired records
        let records: HashMap<_, _> = records.into_iter().collect();
        for index in self
            .records(None)
            .map(|(i, _)| i)
            .filter(|i| !records.contains_key(i))
        {
            self.delete(index);
        }
    }
}

impl Backend for Disk {
    fn add(&mut self, entry: Record) -> usize {
        self.clean();
        self.last_index += 1;
        let bucket = self.bucket();
        bucket
            .set(&Integer::from(self.last_index), &Json(entry))
            .expect("failed to store kv record");
        bucket.flush().expect("failed kv flush to disk");
        self.last_index
    }
    fn get(&mut self, index: usize) -> Option<Record> {
        self.clean();
        self.bucket()
            .get(&Integer::from(index))
            .expect("failed to lookup kv record")
            .map(|r| r.0)
    }
    fn exists(&mut self, entry: &crate::clipboard::Entry) -> Option<usize> {
        self.clean();
        self.records(None)
            .find(|(_, r)| r.entry.body == entry.body)
            .map(|(i, _)| i)
    }
    fn latest(&mut self) -> Option<Record> {
        self.clean();
        let mut records: Vec<_> = self.records(None).map(|(_, r)| r).collect();
        records.sort_by_key(|r| r.last_used);
        records.last().cloned()
    }
    fn update(&mut self, index: &usize) {
        self.clean();
        if let Some(mut record) = self.get(*index) {
            record.update();
            let bucket = self.bucket();
            bucket
                .set(&Integer::from(*index), &Json(record.clone()))
                .expect("failed to update kv record");
            bucket.flush().expect("failed kv flush to disk");
        }
    }
    fn delete(&mut self, index: usize) {
        let bucket = self.bucket();
        bucket
            .remove(&Integer::from(index))
            .expect("failed to delete kv record");
        bucket.flush().expect("failed kv flush to disk");
    }
    fn clear(&mut self) {
        let bucket = self.bucket();
        bucket.clear().expect("failed to empty kv store");
        bucket.flush().expect("failed kv flush to disk");
    }
    fn list(&mut self, preview_size: usize) -> Vec<Preview> {
        self.clean();
        self.records(None)
            .map(|(i, r)| Preview {
                index: i,
                preview: r.entry.preview(preview_size),
                last_used: r.last_used,
            })
            .collect()
    }
}
