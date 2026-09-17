use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Mirrors the on-device schema confirmed by live inspection of a reMarkable 2
/// running Codex Linux 5.8.202 (image 3.28.0.169) on 2026-09-17: no
/// `synced`/`deleted`/`version` fields exist on this OS build.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntryType {
    DocumentType,
    CollectionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub created_time: String,
    pub last_modified: String,
    pub last_opened: String,
    pub last_opened_page: u32,
    /// UUID of the parent collection, or "" for the root.
    pub parent: String,
    pub pinned: bool,
    #[serde(rename = "type")]
    pub entry_type: EntryType,
    pub visible_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentMetadataInfo {
    pub authors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    pub cover_page_number: i32,
    pub custom_zoom_center_x: i32,
    pub custom_zoom_center_y: i32,
    pub custom_zoom_orientation: String,
    pub custom_zoom_page_height: i32,
    pub custom_zoom_page_width: i32,
    pub custom_zoom_scale: f64,
    pub extra_metadata: HashMap<String, String>,
    pub font_name: String,
    pub format_version: i32,
    pub line_height: i32,
    pub margins: i32,
    pub orientation: String,
    pub tags: Vec<String>,
    pub text_alignment: String,
    pub text_scale: f64,
    pub zoom_mode: String,
    pub page_tags: Vec<String>,
    pub document_metadata: DocumentMetadataInfo,
    pub file_type: String,
    pub original_page_count: u32,
    pub page_count: u32,
    pub pages: Vec<String>,
    pub redirection_page_map: Vec<u32>,
    pub size_in_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Local {
    pub content_format_version: u32,
}

/// Returns milliseconds since the Unix epoch as a string, matching the
/// on-device `.metadata` timestamp format (e.g. `createdTime`/`lastModified`).
pub fn now_ms_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    ms.to_string()
}
