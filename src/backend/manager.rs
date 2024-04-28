//! Backend Storage Manager

use std::collections::HashMap;

use crate::backend::CleanCfg;

use super::backend::{Backend, BackendCategory};
use super::config::{BackendConfig, CategoryConfig};

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
    /// Retrieve Configuration Settings for Particular Category
    fn get_config(&mut self, category: Option<&str>) -> CategoryConfig {
        if let Some(name) = category {
            if let Some(config) = self.config.get(name) {
                return config.clone();
            }
        }
        if let Some(config) = self.config.get("default") {
            return config.clone();
        }
        let name = category.unwrap_or("default");
        self.config
            .insert(name.to_owned(), CategoryConfig::default());
        return self
            .config
            .get(name)
            .expect("unable to find backend config")
            .clone();
    }
}

impl Backend for Manager {
    fn categories(&self) -> Vec<String> {
        self.stores
            .values()
            .map(|b| b.categories())
            .flatten()
            .collect()
    }
    fn category(&mut self, category: Option<&str>) -> Box<dyn BackendCategory> {
        let config = self.get_config(category);
        let storage = config.storage.to_string();
        log::debug!("backend for category {category:?} is {storage:?}");
        if let Some(backend) = self.stores.get_mut(&storage) {
            let mut category = backend.category(category);
            category.clean(&CleanCfg::from(&config));
            return category;
        }
        let backend = config.storage.backend();
        self.stores.insert(storage.to_owned(), backend);
        self.stores
            .get_mut(&storage)
            .expect("failed to find backend")
            .category(category)
    }
}
