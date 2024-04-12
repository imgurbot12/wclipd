//! Configuration for WClipD
use serde::Deserialize;

use crate::backend::{Lifetime, Storage};

#[derive(Debug, Deserialize)]
pub struct DaemonConfig {
    #[serde(skip)]
    pub kill: bool,
    pub backend: Storage,
    pub lifetime: Lifetime,
    pub max_entries: Option<usize>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            kill: false,
            backend: Storage::Memory,
            lifetime: Lifetime::OnLogin,
            max_entries: None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub socket: Option<String>,
    pub daemon: DaemonConfig,
}
