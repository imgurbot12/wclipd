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

/// Message Category Type Alias
pub type Cat = Option<String>;

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
    /// Add New Clipboard Entry
    Copy {
        entry: Entry,
        primary: bool,
        category: Cat,
    },
    /// Recopy an Existing Entry
    Select {
        index: usize,
        primary: bool,
        category: Cat,
    },
    /// View Clipboard History
    List { length: usize, category: Cat },
    /// Delete an Existing Clipboard Entry
    Delete { index: usize, category: Cat },
    /// Find Specific History Entry
    Find { index: Option<usize>, category: Cat },
    /// Delete Clipboard Entries
    Wipe { wipe: Wipe, category: Cat },
}

/// All Possible Response Messages Supported by Daemon
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "response", rename_all = "lowercase")]
pub enum Response {
    /// Simple Success Message
    Ok,
    /// Error Message
    Error { error: String },
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
