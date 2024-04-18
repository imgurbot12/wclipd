//! KV Store Disk Backend Database

use std::path::PathBuf;

use super::backend::*;

pub struct Kv {
    store: kv::Store,
}

impl Kv {
    pub fn new(path: PathBuf) -> Self {
        let config = kv::Config::new(path);
        let store = kv::Store::new(config).expect("unable to spawn kv");
        Self { store }
    }
}

impl Backend for Kv {
    fn categories(&self) -> Vec<String> {
        self.store.buckets()
    }
    fn category(&mut self, category: Option<&str>) -> Box<dyn BackendCategory> {
        let bucket = self
            .store
            .bucket(category)
            .expect("kv failed to access bucket");
        Box::new(KvCategory { bucket })
    }
}

struct KvCategory<'a> {
    bucket: kv::Bucket<'a, kv::Integer, kv::Json<Record>>,
}

impl<'a> BackendCategory for KvCategory<'a> {
    fn get(&self, index: &usize) -> Option<Record> {
        self.bucket
            .get(&kv::Integer::from(*index))
            .expect("kv bucket read failed")
            .map(|j| j.0)
    }
    fn insert(&mut self, index: usize, record: Record) {
        self.bucket
            .set(&kv::Integer::from(index), &kv::Json(record))
            .expect("kv bucket write failed");
    }
    fn delete(&mut self, index: &usize) {
        self.bucket
            .remove(&kv::Integer::from(*index))
            .expect("kv bucket delete failed");
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Record>> {
        Box::new(
            self.bucket
                .iter()
                .filter_map(|r| r.ok())
                .map(|i| i.value().expect("kv bucket iter failed"))
                .map(|r: kv::Json<Record>| r.0),
        )
    }
    fn index(&mut self) -> usize {
        self.bucket
            .iter()
            .filter_map(|r| r.ok())
            .map(|i| i.key().expect("kv bucket index failed"))
            .map(|i: kv::Integer| usize::from(i))
            .max()
            .map(|max| max + 1)
            .unwrap_or(0)
    }
}
