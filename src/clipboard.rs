//! Clipboard Objects and Tools

use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use wayland_clipboard_listener::ClipBoardListenContext;
use wayland_clipboard_listener::ClipBoardListenMessage;

use crate::mime::*;

/// Preview of Existing Clipboard Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preview {
    pub index: usize,
    pub preview: String,
    pub last_used: SystemTime,
}

/// DataTypes for Clipboard Entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClipBody {
    Text(String),
    Data(#[serde(with = "base64_serial")] Vec<u8>),
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
        let mut mimes = vec![
            "text/plain".to_owned(),
            "TEXT".to_owned(),
            "UTF8_STRING".to_owned(),
            "text/plain;charset=utf-8".to_owned(),
        ];
        if let Some(mime) = mime {
            if !mimes.contains(&mime) {
                mimes.insert(0, mime);
            }
        }
        Self {
            mime: mimes,
            body: ClipBody::Text(content),
        }
    }
    /// Generate new Data Clipboard Entry
    pub fn data(content: &[u8], mime: Option<String>) -> Self {
        let mime = mime.unwrap_or_else(|| guess_mime_data(content));
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
    /// Check if Clipboard Body is Text
    pub fn is_text(&self) -> bool {
        match self.body {
            ClipBody::Text(_) => true,
            _ => self.mime.iter().any(|m| is_text(m)),
        }
    }
    /// Get First MimeType in Available MimeTypes
    #[inline]
    pub fn mime(&self) -> String {
        self.mime
            .get(0)
            .cloned()
            .unwrap_or_else(|| "N/A".to_owned())
    }
    /// Generate Content Preview
    pub fn preview(&self, max_width: usize) -> String {
        let mut s = match &self.body {
            ClipBody::Text(text) => text.to_owned(),
            ClipBody::Data(data) => preview_data(data, &self.mime),
        };
        if s.chars().all(char::is_whitespace) {
            s = format!("{s:?}");
        }
        let mut s = s
            .trim()
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join(" ");
        if s.len() > max_width {
            let max = std::cmp::max(max_width, 3);
            s.truncate(max - 3);
            s = format!("{s}...");
        }
        s
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

mod base64_serial {
    use base64::prelude::{Engine as _, BASE64_STANDARD};
    use serde::{Deserialize, Serialize};
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let b64 = BASE64_STANDARD.encode(v);
        String::serialize(&b64, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let b64 = String::deserialize(d)?;
        BASE64_STANDARD
            .decode(b64.as_bytes())
            .map_err(|e| serde::de::Error::custom(e))
    }
}
