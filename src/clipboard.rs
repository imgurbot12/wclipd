//! Clipboard Objects and Tools

use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;
use wayland_clipboard_listener::ClipBoardListenContext;
use wayland_clipboard_listener::ClipBoardListenMessage;

/// Preview of Existing Clipboard Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preview {
    pub index: usize,
    pub preview: String,
    pub last_used: SystemTime,
}

/// DataTypes for Clipboard Entry
#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClipBody {
    Text(String),
    Data(#[serde_as(as = "Base64")] Vec<u8>),
}

impl From<ClipBoardListenContext> for ClipBody {
    fn from(value: ClipBoardListenContext) -> Self {
        match value {
            ClipBoardListenContext::Text(text) => Self::Text(text),
            ClipBoardListenContext::File(data) => Self::Data(data),
        }
    }
}

impl ClipBody {
    /// Check if Clipboard Content is Empty
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Text(t) => t.is_empty(),
            Self::Data(d) => d.is_empty(),
        }
    }
    /// Convert Contents into Bytes
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Text(text) => text.as_bytes(),
            Self::Data(data) => &data,
        }
    }
    /// Generate Content Preview
    pub fn preview(&self, max_width: usize) -> String {
        let s = match self {
            Self::Text(text) => text.to_owned(),
            Self::Data(data) => {
                let mime_db = xdg_mime::SharedMimeInfo::new();
                match mime_db.get_mime_type_for_data(data) {
                    Some((mime, _)) => format!("binary data [{mime}]"),
                    None => "unknown".to_owned(),
                }
            }
        };
        let mut s = s
            .trim()
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join(" ");
        if s.len() >= max_width {
            s.truncate(max_width - 3);
            s = format!("{s}...");
        }
        s
    }
}

/// Single Record Stored in Clipboard History
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub mime: Vec<String>,
    pub body: ClipBody,
}

impl Entry {
    /// Generate new Text Clipboard Entry
    pub fn text(content: String, mime: Option<String>) -> Self {
        Self {
            mime: vec![mime.unwrap_or_else(|| "text/plain".to_owned())],
            body: ClipBody::Text(content),
        }
    }
    /// Generate new Data Clipboard Entry
    pub fn data(content: &[u8], mime: Option<String>) -> Self {
        let mime = mime.unwrap_or_else(|| {
            let mime_db = xdg_mime::SharedMimeInfo::new();
            match mime_db.get_mime_type_for_data(content) {
                Some((mime, _)) => format!("{}", mime),
                None => "unknown".to_owned(),
            }
        });
        Self {
            mime: vec![mime],
            body: ClipBody::Data(content.to_vec()),
        }
    }
    /// Check if Clipboard Body is Empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.body.is_empty()
    }
    /// Convert Contents into Bytes
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.body.as_bytes()
    }
    /// Generate Clipboard Preview
    #[inline]
    pub fn preview(&self, max_width: usize) -> String {
        self.body.preview(max_width)
    }
}

impl From<ClipBoardListenMessage> for Entry {
    fn from(value: ClipBoardListenMessage) -> Self {
        Self {
            mime: value.mime_types,
            body: ClipBody::from(value.context),
        }
    }
}
