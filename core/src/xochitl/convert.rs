use std::collections::HashMap;
use std::path::Path;

use lopdf::Document;
use thiserror::Error;
use uuid::Uuid;

use super::metadata::{now_ms_string, Content, DocumentMetadataInfo, EntryType, Local, Metadata};

#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("unsupported file type: only .pdf is supported today (got {0:?})")]
    UnsupportedFileType(Option<String>),
    #[error("failed to read {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse PDF: {0}")]
    Pdf(#[from] lopdf::Error),
}

/// The five pieces of a xochitl document, built from a local file.
#[derive(Debug)]
pub struct DocumentBundle {
    pub metadata: Metadata,
    pub content: Content,
    pub local: Local,
    pub pagedata: String,
    pub file_bytes: Vec<u8>,
    pub file_extension: String,
}

/// Decodes a PDF "text string" (as used in the Info dictionary): either
/// UTF-16BE with a leading byte-order-mark, or PDFDocEncoding otherwise. The
/// PDFDocEncoding case is approximated with a lossy byte-as-codepoint decode
/// (a superset of Latin-1 for the common range) rather than a full encoding
/// table, which is adequate for a display-only author name.
fn decode_pdf_text_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}

/// Reads the PDF's Info dictionary `Author` entry as a single string, if
/// present. Deliberately does not attempt to split it into multiple names --
/// PDF Author metadata is free text, not a delimited list, so the original
/// TypeScript tool's `split(" ")` mangled any multi-word name.
fn read_author(doc: &Document) -> Option<String> {
    let info_ref = doc.trailer.get(b"Info").ok()?.as_reference().ok()?;
    let info_dict = doc.get_object(info_ref).ok()?.as_dict().ok()?;
    let author = info_dict.get(b"Author").ok()?;
    let text = decode_pdf_text_string(author.as_str().ok()?);
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

/// Builds the full set of xochitl sidecar data for `filepath`, to be written
/// under a freshly generated document UUID by the caller. Only PDF is
/// supported in this pass (see the project plan for why EPUB is out of
/// scope): `lopdf` cannot parse it, and the `.content` schema below assumes
/// fixed pagination, which EPUB doesn't naturally have.
pub fn convert(filepath: &Path, parent: &str) -> Result<DocumentBundle, ConvertError> {
    let extension = filepath
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    if extension.as_deref() != Some("pdf") {
        return Err(ConvertError::UnsupportedFileType(extension));
    }
    let extension = extension.unwrap();

    let file_bytes = std::fs::read(filepath).map_err(|e| ConvertError::Io {
        path: filepath.to_path_buf(),
        source: e,
    })?;

    let doc = Document::load_mem(&file_bytes)?;
    let page_count = doc.get_pages().len() as u32;
    let authors = read_author(&doc).map(|a| vec![a]).unwrap_or_default();

    let visible_name = filepath
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let now = now_ms_string();
    let metadata = Metadata {
        created_time: now.clone(),
        last_modified: now,
        last_opened: "0".to_string(),
        last_opened_page: 0,
        parent: parent.to_string(),
        pinned: false,
        entry_type: EntryType::DocumentType,
        visible_name,
    };

    let pages: Vec<String> = (0..=page_count).map(|_| Uuid::new_v4().to_string()).collect();
    let redirection_page_map: Vec<u32> = (0..=page_count).collect();

    let content = Content {
        cover_page_number: 0,
        custom_zoom_center_x: 0,
        custom_zoom_center_y: 936,
        custom_zoom_orientation: "portrait".to_string(),
        custom_zoom_page_height: 1872,
        custom_zoom_page_width: 1404,
        custom_zoom_scale: 1.0,
        extra_metadata: HashMap::new(),
        font_name: String::new(),
        format_version: 1,
        line_height: -1,
        margins: 125,
        orientation: "portrait".to_string(),
        tags: Vec::new(),
        text_alignment: "justify".to_string(),
        text_scale: 1.0,
        zoom_mode: "bestFit".to_string(),
        page_tags: Vec::new(),
        document_metadata: DocumentMetadataInfo { authors },
        file_type: extension.clone(),
        original_page_count: page_count,
        page_count,
        pages,
        redirection_page_map,
        size_in_bytes: file_bytes.len() as u64,
    };

    let local = Local {
        content_format_version: 1,
    };

    let pagedata = "blank\n".repeat(page_count as usize + 1);

    Ok(DocumentBundle {
        metadata,
        content,
        local,
        pagedata,
        file_bytes,
        file_extension: extension,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ref.pdf")
    }

    #[test]
    fn converts_the_fixture_pdf() {
        let bundle = convert(&fixture_path(), "").expect("conversion should succeed");
        assert_eq!(bundle.content.file_type, "pdf");
        assert!(bundle.content.page_count >= 1);
        assert_eq!(bundle.content.pages.len(), bundle.content.page_count as usize + 1);
        assert_eq!(bundle.content.size_in_bytes as usize, bundle.file_bytes.len());
        assert_eq!(bundle.metadata.visible_name, "ref.pdf");
    }

    #[test]
    fn rejects_non_pdf_input() {
        let err = convert(std::path::Path::new("notes.epub"), "").unwrap_err();
        assert!(matches!(err, ConvertError::UnsupportedFileType(_)));
    }
}
