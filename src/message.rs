//! Daemon Message Implementations

use serde::{Deserialize, Serialize};

use crate::clipboard::{Entry, Preview};

/// Delete Specified Items from History
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "request", rename_all = "lowercase")]
pub enum Wipe {
    All,
    Single { index: usize },
}

/// Message Backend Group Type Alias
pub type Grp = Option<String>;

/// Message Index Type Alias;
pub type Idx = Option<usize>;

/// All Possible Request Messages Supported by Daemon
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "request", rename_all = "lowercase")]
pub enum Request {
    /// Ping Message to Check if Server is Alive
    Ping,
    /// Stop Daemon Instance
    Stop,
    /// Clear Active Clipboard
    Clear,
    /// List Existing Groups
    Groups,
    /// Add New Clipboard Entry
    Copy {
        entry: Entry,
        primary: bool,
        group: Grp,
        index: Idx,
    },
    /// Recopy an Existing Entry
    Select {
        index: usize,
        primary: bool,
        group: Grp,
    },
    /// View Clipboard History
    List { length: usize, group: Grp },
    /// Delete an Existing Clipboard Entry
    Delete { index: usize, group: Grp },
    /// Find Specific History Entry
    Find { index: Option<usize>, group: Grp },
    /// Delete Clipboard Entries
    Wipe { wipe: Wipe, group: Grp },
}

/// All Possible Response Messages Supported by Daemon
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "response", rename_all = "lowercase")]
pub enum Response {
    /// Simple Success Message
    Ok,
    /// Error Message
    Error { error: String },
    /// List of Avaialble Groups
    Groups { groups: Vec<String> },
    /// Returned Clipboard Entry
    Entry { entry: Entry },
    /// Clipboard Previews
    Previews { previews: Vec<Preview> },
}

impl Response {
    /// Spawn Error Response Message
    #[inline]
    pub fn error(error: String) -> Self {
        Self::Error { error }
    }
}
