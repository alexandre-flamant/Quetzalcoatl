pub mod convert;
pub mod metadata;
pub mod tree;

pub use convert::{convert, ConvertError, DocumentBundle};
pub use metadata::{Content, DocumentMetadataInfo, EntryType, Local, Metadata};
pub use tree::{FolderTree, TreeNode, ROOT};

/// Path to the xochitl document store on the tablet. Confirmed by live
/// inspection on 2026-09-17 (Codex Linux 5.8.202, image 3.28.0.169).
pub const XOCHITL_PATH: &str = "/home/root/.local/share/remarkable/xochitl";
