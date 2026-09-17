use std::collections::HashMap;

use super::metadata::{EntryType, Metadata};

/// UUID used to mean "the tablet's root" throughout this crate. Not a real
/// on-device UUID — xochitl documents/collections use "" as `parent` to mean
/// the root, and we mirror that instead of inventing a sentinel.
pub const ROOT: &str = "";

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub uuid: String,
    pub name: String,
    pub folders: Vec<TreeNode>,
    pub documents: Vec<(String, String)>,
}

/// In-memory model of the on-device folder/document tree, built from the full
/// set of `.metadata` sidecars. Mirrors (and extends, with folder creation and
/// move support) the read-only `RM2FileSystem` tree from the original
/// TypeScript implementation.
#[derive(Debug, Clone)]
pub struct FolderTree {
    pub metadata: HashMap<String, Metadata>,
}

impl FolderTree {
    pub fn new(metadata: HashMap<String, Metadata>) -> Self {
        Self { metadata }
    }

    /// Splits a `//`-separated folder path into non-empty segments. An empty
    /// path (or one made only of separators) yields no segments, meaning the
    /// root — this is resolved directly, with no ambiguous lookup, unlike the
    /// original code's unclear handling of the empty-path case.
    pub fn split_path(path: &str) -> Vec<&str> {
        path.split("//").filter(|s| !s.is_empty()).collect()
    }

    /// Finds a direct child collection of `parent` with the given name.
    pub fn find_child_collection(&self, parent: &str, name: &str) -> Option<String> {
        self.metadata
            .iter()
            .find(|(_, m)| {
                m.entry_type == EntryType::CollectionType
                    && m.parent == parent
                    && m.visible_name == name
            })
            .map(|(uuid, _)| uuid.clone())
    }

    /// Finds any direct child (document or collection) of `parent` with the
    /// given name, along with its type.
    pub fn find_child(&self, parent: &str, name: &str) -> Option<(String, EntryType)> {
        self.metadata
            .iter()
            .find(|(_, m)| m.parent == parent && m.visible_name == name)
            .map(|(uuid, m)| (uuid.clone(), m.entry_type))
    }

    /// Resolves a `//`-separated path to a UUID, returning `None` if any
    /// segment along the way doesn't exist. Does not create anything --
    /// see `crate::ops::resolve_or_create_path` for the creating variant.
    pub fn resolve_existing(&self, path: &str) -> Option<String> {
        let mut parent = ROOT.to_string();
        for segment in Self::split_path(path) {
            parent = self.find_child_collection(&parent, segment)?;
        }
        Some(parent)
    }

    pub fn list_tree(&self) -> TreeNode {
        self.compute_node(ROOT)
    }

    fn compute_node(&self, uuid: &str) -> TreeNode {
        let name = if uuid == ROOT {
            String::new()
        } else {
            self.metadata
                .get(uuid)
                .map(|m| m.visible_name.clone())
                .unwrap_or_default()
        };

        let mut documents = Vec::new();
        let mut folders = Vec::new();
        for (child_uuid, m) in self.metadata.iter().filter(|(_, m)| m.parent == uuid) {
            match m.entry_type {
                EntryType::DocumentType => {
                    documents.push((child_uuid.clone(), m.visible_name.clone()))
                }
                EntryType::CollectionType => folders.push(self.compute_node(child_uuid)),
            }
        }
        documents.sort_by(|a, b| a.1.cmp(&b.1));
        folders.sort_by(|a, b| a.name.cmp(&b.name));

        TreeNode {
            uuid: uuid.to_string(),
            name,
            folders,
            documents,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collection(parent: &str, name: &str) -> Metadata {
        Metadata {
            created_time: "0".into(),
            last_modified: "0".into(),
            last_opened: "0".into(),
            last_opened_page: 0,
            parent: parent.into(),
            pinned: false,
            entry_type: EntryType::CollectionType,
            visible_name: name.into(),
        }
    }

    fn document(parent: &str, name: &str) -> Metadata {
        Metadata {
            entry_type: EntryType::DocumentType,
            ..collection(parent, name)
        }
    }

    #[test]
    fn empty_path_resolves_to_root_without_lookup() {
        let tree = FolderTree::new(HashMap::new());
        assert_eq!(tree.resolve_existing(""), Some(ROOT.to_string()));
        assert_eq!(tree.resolve_existing("//"), Some(ROOT.to_string()));
    }

    #[test]
    fn resolves_nested_existing_path() {
        let mut metadata = HashMap::new();
        metadata.insert("livres".into(), collection(ROOT, "Livres"));
        metadata.insert("mangas".into(), collection("livres", "mangas"));
        let tree = FolderTree::new(metadata);

        assert_eq!(
            tree.resolve_existing("Livres//mangas"),
            Some("mangas".to_string())
        );
        assert_eq!(tree.resolve_existing("Livres//comics"), None);
        assert_eq!(tree.resolve_existing("Livres"), Some("livres".to_string()));
    }

    #[test]
    fn list_tree_partitions_documents_and_folders() {
        let mut metadata = HashMap::new();
        metadata.insert("livres".into(), collection(ROOT, "Livres"));
        metadata.insert("doc1".into(), document(ROOT, "root.pdf"));
        metadata.insert("doc2".into(), document("livres", "book.pdf"));
        let tree = FolderTree::new(metadata);

        let root = tree.list_tree();
        assert_eq!(root.documents, vec![("doc1".to_string(), "root.pdf".to_string())]);
        assert_eq!(root.folders.len(), 1);
        assert_eq!(root.folders[0].name, "Livres");
        assert_eq!(
            root.folders[0].documents,
            vec![("doc2".to_string(), "book.pdf".to_string())]
        );
    }
}
