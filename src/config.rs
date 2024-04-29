//! Configuration for WClipD
use std::str::FromStr;

use serde::{de::Error, Deserialize};

use crate::backend::{BackendConfig, Expiration, Storage};
use crate::message::Grp;
use crate::table::{Align, Style};

#[derive(Debug, Deserialize)]
pub struct DaemonConfig {
    #[serde(skip)]
    pub kill: bool,
    pub capture_live: bool,
    pub backends: BackendConfig,
    pub term_backend: Grp,
    pub live_backend: Grp,
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

#[inline]
fn _align() -> Align {
    Align::Right
}

#[derive(Debug, Deserialize)]
pub struct TableConfig {
    #[serde(default)]
    pub style: Style,
    #[serde(default = "_align")]
    pub index_align: Align,
    #[serde(default)]
    pub preview_align: Align,
    #[serde(default)]
    pub time_align: Align,
}

impl Default for TableConfig {
    fn default() -> Self {
        Self {
            style: Style::default(),
            index_align: Align::Right,
            preview_align: Align::default(),
            time_align: Align::default(),
        }
    }
}

fn _preview() -> usize {
    60
}

#[derive(Debug, Deserialize)]
pub struct ListConfig {
    #[serde(default)]
    pub default_group: Grp,
    #[serde(default = "_preview")]
    pub preview_length: usize,
    #[serde(default)]
    pub table: TableConfig,
}

impl Default for ListConfig {
    fn default() -> Self {
        Self {
            default_group: None,
            preview_length: 80,
            table: TableConfig::default(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub socket: Option<String>,
    #[serde(default)]
    pub list: ListConfig,
    #[serde(default)]
    pub daemon: DaemonConfig,
}

macro_rules! de_fromstr {
    ($s:ident) => {
        impl<'de> Deserialize<'de> for $s {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s: &str = Deserialize::deserialize(deserializer)?;
                $s::from_str(s).map_err(D::Error::custom)
            }
        }
    };
}

// implement `Deserialize` using `FromStr`
de_fromstr!(Style);
de_fromstr!(Align);
de_fromstr!(Storage);
de_fromstr!(Expiration);
