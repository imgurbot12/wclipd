//! Memory Storage for Backend Implementation

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::backend::*;

pub struct Memory {
    store: HashMap<String, MemoryCategory>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }
}

impl<'a> Backend for Memory {
    fn categories(&self) -> Vec<String> {
        self.store.keys().map(|c| c.to_owned()).collect()
    }
    fn category(&mut self, category: Option<&str>) -> Box<dyn BackendCategory> {
        let name = category.unwrap_or("default");
        if !self.store.contains_key(name) {
            let category = MemoryCategory::new();
            self.store.insert(name.to_owned(), category);
        }
        let category = self.store.get(name).unwrap();
        Box::new((*category).clone())
    }
}

struct MemoryCategory {
    store: Rc<RefCell<HashMap<usize, Record>>>,
    last_index: usize,
}

impl MemoryCategory {
    fn new() -> Self {
        Self {
            store: Rc::new(RefCell::new(HashMap::new())),
            last_index: 0,
        }
    }
}

impl Clone for MemoryCategory {
    fn clone(&self) -> Self {
        Self {
            store: Rc::clone(&self.store),
            last_index: self.last_index,
        }
    }
}

impl BackendCategory for MemoryCategory {
    fn get(&self, index: &usize) -> Option<Record> {
        self.store.borrow().get(index).map(|r| r.clone())
    }
    fn insert(&mut self, index: usize, record: Record) {
        self.store.borrow_mut().insert(index, record);
    }
    fn delete(&mut self, index: &usize) {
        self.store.borrow_mut().remove(index);
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Record>> {
        Box::new(self.store.borrow().clone().into_values())
    }
    fn index(&mut self) -> usize {
        let index = self.last_index;
        self.last_index += 1;
        index
    }
}
