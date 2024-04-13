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

/// All Possible Request Messages Supported by Daemon
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "request", rename_all = "lowercase")]
pub enum Request {
    /// Ping Message to Check if Server is Alive
    Ping,
    /// Stop Daemon Instance
    Stop,
    /// Add New Clipboard Entry
    Copy { entry: Entry },
    /// View Clipboard History
    List { length: usize },
    /// Find Specific History Entry
    Find { index: Option<usize> },
    /// Delete Clipboard Entries
    Wipe(Wipe),
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
