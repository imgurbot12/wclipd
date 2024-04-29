//! Backend Storage Manager

use std::collections::HashMap;

use crate::backend::CleanCfg;

use super::backend::{Backend, BackendGroup};
use super::config::{BackendConfig, GroupConfig};

/// Backend Storage Manager Implementation
pub struct Manager {
    config: BackendConfig,
    stores: HashMap<String, Box<dyn Backend>>,
}

impl Manager {
    pub fn new(config: BackendConfig) -> Self {
        Self {
            config,
            stores: HashMap::new(),
        }
    }
    /// Retrieve Configuration Settings for Particular Group
    fn get_config(&mut self, group: Option<&str>) -> GroupConfig {
        if let Some(name) = group {
            if let Some(config) = self.config.get(name) {
                return config.clone();
            }
        }
        if let Some(config) = self.config.get("default") {
            return config.clone();
        }
        let name = group.unwrap_or("default");
        self.config.insert(name.to_owned(), GroupConfig::default());
        return self
            .config
            .get(name)
            .expect("unable to find backend config")
            .clone();
    }
}

impl Backend for Manager {
    fn groups(&self) -> Vec<String> {
        self.stores.values().map(|b| b.groups()).flatten().collect()
    }
    fn group(&mut self, group: Option<&str>) -> Box<dyn BackendGroup> {
        let config = self.get_config(group);
        let storage = config.storage.to_string();
        log::debug!("backend for group {group:?} is {storage:?}");
        if let Some(backend) = self.stores.get_mut(&storage) {
            let mut group = backend.group(group);
            group.clean(&CleanCfg::from(&config));
            return group;
        }
        let backend = config.storage.backend();
        self.stores.insert(storage.to_owned(), backend);
        self.stores
            .get_mut(&storage)
            .expect("failed to find backend")
            .group(group)
    }
}
