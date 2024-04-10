//! Daemon Message Implementations

use serde::{Deserialize, Serialize};
use wayland_clipboard_listener::ClipBoardListenContext;
use wayland_clipboard_listener::ClipBoardListenMessage;

/// DataTypes for Clipboard Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipboardBody {
    Text(String),
    Data(Vec<u8>),
}

impl From<ClipBoardListenContext> for ClipboardBody {
    fn from(value: ClipBoardListenContext) -> Self {
        match value {
            ClipBoardListenContext::Text(text) => Self::Text(text),
            ClipBoardListenContext::File(data) => Self::Data(data),
        }
    }
}

impl ClipboardBody {
    /// Check if Clipboard Content is Empty
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Text(t) => t.is_empty(),
            Self::Data(d) => d.is_empty(),
        }
    }
}

/// Single Record Stored in Clipboard History
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub mime: Vec<String>,
    pub body: ClipboardBody,
}

impl ClipboardEntry {
    /// Check if Clipboard Body is Empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.body.is_empty()
    }
}

impl From<ClipBoardListenMessage> for ClipboardEntry {
    fn from(value: ClipBoardListenMessage) -> Self {
        Self {
            mime: value.mime_types,
            body: ClipboardBody::from(value.context),
        }
    }
}

/// Preview of Existing Clipboard Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardPreview {
    pub index: usize,
    pub preview: String,
}

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
    /// View Clipboard History
    List,
    /// Find Specific History Entry
    Find { index: usize },
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
    Entry { entry: ClipboardEntry },
    /// Clipboard Previews
    Previews { previews: Vec<ClipboardPreview> },
}
