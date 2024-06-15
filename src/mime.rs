//! MimeType Evaluation for Clipboard Entries

use std::path::PathBuf;

/// Check if given MIME type is valid plain-text
pub fn is_text(mime_type: &str) -> bool {
    match mime_type {
        "TEXT" | "STRING" | "UTF8_STRING" => true,
        x if x.starts_with("text/") => true,
        _ => false,
    }
}

/// Check if given MIME type is valid image
pub fn is_image(mime_type: &str) -> bool {
    mime_type.starts_with("image/")
}

/// Guess MimeType from FilePath
pub fn guess_mime_path(path: &PathBuf) -> String {
    let mime_db = xdg_mime::SharedMimeInfo::new();
    let guess = mime_db.guess_mime_type().path(path).guess();
    guess.mime_type().to_string()
}

/// Guess MimeType from Raw Bytes Slice
pub fn guess_mime_data(data: &[u8]) -> String {
    let mime_db = xdg_mime::SharedMimeInfo::new();
    match mime_db.get_mime_type_for_data(data) {
        Some((mime, _)) => format!("{}", mime),
        None => match data.is_ascii() {
            true => "text/plain".to_owned(),
            false => "unknown".to_owned(),
        },
    }
}

/// Preview Raw Bytes Slice using MimeDB and Available Mime Hints
pub fn preview_data(data: &[u8], hints: &Vec<String>) -> String {
    let mime_db = xdg_mime::SharedMimeInfo::new();
    match mime_db.get_mime_type_for_data(data) {
        Some((mime, _)) => format!("binary data [{mime}]"),
        None => match hints.iter().any(|h| is_text(h)) {
            true => String::from_utf8(data.to_owned()).expect("invalid text"),
            false => format!("unknown data {data:?}"),
        },
    }
}
