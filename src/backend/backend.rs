//! Backend Interface and Implementation Abstractions
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::clipboard::{Entry, Preview};

use super::GroupConfig;

/// Backend Storage Record Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub index: usize,
    pub entry: Entry,
    pub last_used: SystemTime,
    pub entry_date: SystemTime,
}

impl Record {
    pub fn new(index: usize, entry: Entry) -> Self {
        let now = SystemTime::now();
        Record {
            index,
            entry,
            last_used: now,
            entry_date: now,
        }
    }
    fn preview(&self, size: usize) -> Preview {
        Preview {
            index: self.index,
            preview: self.entry.preview(size),
            last_used: self.last_used,
        }
    }
}

/// Backend Record Cleanup Configuration
pub struct CleanCfg {
    pub fixed: Option<SystemTime>,
    pub dynamic: Option<SystemTime>,
    pub max_entries: Option<usize>,
}

impl CleanCfg {
    #[inline]
    fn is_expired(&self, last_used: SystemTime) -> bool {
        last_used <= self.fixed.unwrap_or(UNIX_EPOCH)
            || last_used <= self.dynamic.unwrap_or(UNIX_EPOCH)
    }
}

impl From<&GroupConfig> for CleanCfg {
    fn from(value: &GroupConfig) -> Self {
        Self {
            fixed: value.expiration.fixed_expiration(),
            dynamic: value.expiration.dynanmic_expriration(),
            max_entries: value.max_entries,
        }
    }
}

/// Backend Group Implementation
pub trait BackendGroup: Send + Sync {
    fn iter(&self) -> Box<dyn Iterator<Item = Record>>;
    fn get(&self, index: &usize) -> Option<Record>;
    fn insert(&mut self, index: usize, record: Record);
    fn delete(&mut self, index: &usize);
    fn index(&mut self) -> usize;
}

impl dyn BackendGroup {
    /// Retrieve Latest Stored Record
    pub fn latest(&self) -> Option<Record> {
        self.iter().max_by_key(|r| r.last_used)
    }
    /// Return Index of Record if Entry Exists
    pub fn exists(&self, entry: &Entry) -> Option<usize> {
        self.iter()
            .find(|r| r.entry.body == entry.body)
            .map(|r| r.index)
    }
    /// List Unsorted Previews
    pub fn preview(&self, size: usize) -> Vec<Preview> {
        let mut previews: Vec<Preview> = self.iter().map(|r| r.preview(size)).collect();
        previews.sort_by_key(|p| p.index);
        previews
    }
    /// Find Latest or Index (if Specfied)
    pub fn find(&self, index: Option<usize>) -> Option<Record> {
        match index {
            Some(idx) => self.get(&idx),
            None => self.latest(),
        }
    }
    /// Update LastUpdated Date for Record
    pub fn touch(&mut self, index: usize) {
        if let Some(mut record) = self.get(&index) {
            record.last_used = SystemTime::now();
            self.insert(index, record);
        }
    }
    /// Add/Touch Entry Record in Database
    pub fn push(&mut self, entry: Entry) -> usize {
        match self.exists(&entry) {
            Some(index) => {
                self.touch(index);
                index
            }
            None => {
                let index = self.index();
                let record = Record::new(index, entry);
                self.insert(index, record);
                index
            }
        }
    }
    /// Find & Touch Record (if Found)
    pub fn select(&mut self, index: Option<usize>) -> Option<Record> {
        match self.find(index) {
            Some(record) => {
                self.touch(record.index);
                Some(record)
            }
            None => None,
        }
    }
    /// Delete All Records within the Group
    pub fn clear(&mut self) {
        let indexes: Vec<_> = self.iter().map(|r| r.index).collect();
        for index in indexes {
            self.delete(&index);
        }
    }
    /// Delete Expired Records within Backend
    pub fn clean(&mut self, cfg: &CleanCfg) {
        // delete expired records and collect non-expired
        let mut valid: Vec<(usize, SystemTime)> = vec![];
        for record in self.iter() {
            match cfg.is_expired(record.last_used) {
                true => self.delete(&record.index),
                false => valid.push((record.index, record.last_used)),
            }
        }
        // delete oldest records until within size
        if let Some(max_size) = cfg.max_entries {
            valid.sort_by_key(|(_, last_used)| last_used.to_owned());
            valid.reverse();
            while valid.len() > max_size {
                let (index, _) = valid.pop().expect("empty record set");
                self.delete(&index);
            }
        }
    }
}

/// Type Alias for Group Specification
pub type Group<'a> = Option<&'a str>;

/// Backend Implementation
pub trait Backend: Send + Sync {
    fn groups(&self) -> Vec<String>;
    fn group(&mut self, group: Group) -> Box<dyn BackendGroup>;
}
