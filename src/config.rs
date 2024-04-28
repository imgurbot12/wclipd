//! Configuration for WClipD
use serde::Deserialize;

use crate::backend::BackendConfig;
use crate::message::Cat;

#[derive(Debug, Deserialize)]
pub struct DaemonConfig {
    #[serde(skip)]
    pub kill: bool,
    pub capture_live: bool,
    pub backends: BackendConfig,
    pub term_backend: Cat,
    pub live_backend: Cat,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            kill: false,
            capture_live: true,
            backends: BackendConfig::new(),
            term_backend: None,
            live_backend: None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub socket: Option<String>,
    pub daemon: DaemonConfig,
}
