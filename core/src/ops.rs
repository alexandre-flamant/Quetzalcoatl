use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::config::Config;
use crate::transport::{RemoteTransport, SftpTransport, TransportError};
use crate::xochitl::metadata::now_ms_string;
use crate::xochitl::{convert, ConvertError, EntryType, FolderTree, Metadata, TreeNode, XOCHITL_PATH, ROOT};

#[derive(Debug, Error)]
pub enum OpsError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Convert(#[from] ConvertError),
    #[error("failed to parse metadata for {uuid}: {source}")]
    MetadataParse {
        uuid: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("no such entry: {0}")]
    NotFound(String),
    #[error(
        "uploaded file size ({uploaded}) does not match local size ({local}) for {path} -- \
         the transfer may have been interrupted"
    )]
    SizeMismatch {
        path: String,
        local: u64,
        uploaded: u64,
    },
    #[error("integrity check failed for {path}: local sha256 {local_hash} != remote sha256 {remote_hash}")]
    HashMismatch {
        path: String,
        local_hash: String,
        remote_hash: String,
    },
}

fn staged(final_path: &str) -> String {
    format!("{final_path}.partial")
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Writes `files` and creates `dirs` under staged names first, only renaming
/// everything to its final name once every staged write/mkdir has succeeded.
/// On any failure, best-effort deletes whatever staged artifacts exist and
/// returns the error -- no partially-named *final* path is ever left for
/// `rm-sync` to see.
async fn write_staged<T: SftpTransport + ?Sized>(
    transport: &T,
    files: &[(String, Vec<u8>)],
    dirs: &[String],
) -> Result<(), OpsError> {
    let mut written_files = Vec::new();
    let mut created_dirs = Vec::new();

    let result: Result<(), TransportError> = async {
        for (final_path, data) in files {
            let staging_path = staged(final_path);
            transport.write(&staging_path, data).await?;
            written_files.push(staging_path);
        }
        for final_path in dirs {
            let staging_path = staged(final_path);
            transport.mkdir(&staging_path).await?;
            created_dirs.push(staging_path);
        }
        Ok(())
    }
    .await;

    if let Err(err) = result {
        for path in &written_files {
            let _ = transport.remove_file(path).await;
        }
        for path in &created_dirs {
            let _ = transport.remove_dir(path).await;
        }
        return Err(err.into());
    }

    for (final_path, _) in files {
        transport.rename(&staged(final_path), final_path).await?;
    }
    for final_path in dirs {
        transport.rename(&staged(final_path), final_path).await?;
    }

    Ok(())
}

async fn fetch_all_metadata<T: SftpTransport + ?Sized>(
    transport: &T,
) -> Result<HashMap<String, Metadata>, OpsError> {
    let names = transport.list_names(XOCHITL_PATH).await?;
    let mut metadata = HashMap::new();

    for name in names {
        let Some(uuid) = name.strip_suffix(".metadata") else {
            continue;
        };
        let path = format!("{XOCHITL_PATH}/{name}");
        let bytes = transport.read(&path).await?;
        let parsed: Metadata = serde_json::from_slice(&bytes).map_err(|e| OpsError::MetadataParse {
            uuid: uuid.to_string(),
            source: e,
        })?;
        metadata.insert(uuid.to_string(), parsed);
    }

    Ok(metadata)
}

/// Walks a `//`-separated folder path, creating any missing segment as a new
/// collection along the way. Always behaves this way -- there is no flag to
/// opt in or out of creation. An empty path resolves to the root with no
/// writes at all.
pub async fn resolve_or_create_path<T: SftpTransport + ?Sized>(
    transport: &T,
    tree: &mut FolderTree,
    path: &str,
) -> Result<String, OpsError> {
    let mut parent = ROOT.to_string();
    for segment in FolderTree::split_path(path) {
        match tree.find_child_collection(&parent, segment) {
            Some(uuid) => parent = uuid,
            None => {
                let uuid = Uuid::new_v4().to_string();
                let now = now_ms_string();
                let metadata = Metadata {
                    created_time: now.clone(),
                    last_modified: now,
                    last_opened: "0".to_string(),
                    last_opened_page: 0,
                    parent: parent.clone(),
                    pinned: false,
                    entry_type: EntryType::CollectionType,
                    visible_name: segment.to_string(),
                };
                let final_path = format!("{XOCHITL_PATH}/{uuid}.metadata");
                let bytes = serde_json::to_vec_pretty(&metadata).expect("Metadata always serializes");
                write_staged(transport, &[(final_path, bytes)], &[]).await?;
                tree.metadata.insert(uuid.clone(), metadata);
                parent = uuid;
            }
        }
    }
    Ok(parent)
}

pub struct ImportOptions {
    pub target_folder: String,
    pub verify_hash: bool,
    pub restart: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            target_folder: ROOT.to_string(),
            verify_hash: false,
            restart: true,
        }
    }
}

/// Imports a local PDF onto the tablet: resolves (creating as needed) the
/// target folder, generates a fresh document UUID, builds the xochitl
/// sidecar files, uploads everything atomically, verifies the transfer, and
/// (unless disabled) restarts xochitl so it picks up the new document.
pub async fn import_file<T: SftpTransport + ?Sized>(
    transport: &T,
    local_path: &Path,
    options: &ImportOptions,
) -> Result<String, OpsError> {
    let mut tree = FolderTree::new(fetch_all_metadata(transport).await?);
    let parent = resolve_or_create_path(transport, &mut tree, &options.target_folder).await?;

    let bundle = convert::convert(local_path, &parent)?;
    let uuid = Uuid::new_v4().to_string();
    let expected_size = bundle.content.size_in_bytes;
    let local_hash = to_hex(&Sha256::digest(&bundle.file_bytes));

    let payload_path = format!("{XOCHITL_PATH}/{uuid}.{}", bundle.file_extension);
    let files = vec![
        (payload_path.clone(), bundle.file_bytes),
        (
            format!("{XOCHITL_PATH}/{uuid}.metadata"),
            serde_json::to_vec_pretty(&bundle.metadata).expect("Metadata always serializes"),
        ),
        (
            format!("{XOCHITL_PATH}/{uuid}.content"),
            serde_json::to_vec_pretty(&bundle.content).expect("Content always serializes"),
        ),
        (
            format!("{XOCHITL_PATH}/{uuid}.local"),
            serde_json::to_vec_pretty(&bundle.local).expect("Local always serializes"),
        ),
        (
            format!("{XOCHITL_PATH}/{uuid}.pagedata"),
            bundle.pagedata.into_bytes(),
        ),
    ];
    let dirs = vec![
        format!("{XOCHITL_PATH}/{uuid}"),
        format!("{XOCHITL_PATH}/{uuid}.thumbnails"),
    ];

    write_staged(transport, &files, &dirs).await?;

    let uploaded_size = transport.file_size(&payload_path).await?;
    if uploaded_size != expected_size {
        return Err(OpsError::SizeMismatch {
            path: payload_path,
            local: expected_size,
            uploaded: uploaded_size,
        });
    }

    if options.verify_hash {
        let remote_bytes = transport.read(&payload_path).await?;
        let remote_hash = to_hex(&Sha256::digest(&remote_bytes));
        if remote_hash != local_hash {
            return Err(OpsError::HashMismatch {
                path: payload_path,
                local_hash,
                remote_hash,
            });
        }
    }

    if options.restart {
        restart_xochitl(transport).await?;
    }

    Ok(uuid)
}

/// Rewrites an existing document or folder's `parent` to `new_parent_path`
/// (creating any missing segment of it along the way), reorganizing the
/// on-device tree. This is what a future GUI's drag-and-drop between folders
/// calls, and is exercised via the CLI's `move` command too so it's tested
/// now rather than left as an unverified GUI-only path.
pub async fn move_entry<T: SftpTransport + ?Sized>(
    transport: &T,
    entry_uuid: &str,
    new_parent_path: &str,
) -> Result<(), OpsError> {
    let mut tree = FolderTree::new(fetch_all_metadata(transport).await?);
    let mut entry = tree
        .metadata
        .get(entry_uuid)
        .cloned()
        .ok_or_else(|| OpsError::NotFound(entry_uuid.to_string()))?;

    let new_parent = resolve_or_create_path(transport, &mut tree, new_parent_path).await?;
    entry.parent = new_parent;

    let final_path = format!("{XOCHITL_PATH}/{entry_uuid}.metadata");
    let bytes = serde_json::to_vec_pretty(&entry).expect("Metadata always serializes");
    write_staged(transport, &[(final_path, bytes)], &[]).await
}

pub async fn list_tree<T: SftpTransport + ?Sized>(transport: &T) -> Result<TreeNode, OpsError> {
    let tree = FolderTree::new(fetch_all_metadata(transport).await?);
    Ok(tree.list_tree())
}

/// Restarts xochitl (the only known way to make it pick up manually-dropped
/// files) and polls `systemctl is-active xochitl` until it reports `active`,
/// erroring out if it never does within the retry budget.
pub async fn restart_xochitl<T: SftpTransport + ?Sized>(transport: &T) -> Result<(), OpsError> {
    restart_xochitl_with_retry(transport, 10, Duration::from_millis(500)).await
}

async fn restart_xochitl_with_retry<T: SftpTransport + ?Sized>(
    transport: &T,
    max_attempts: u32,
    retry_delay: Duration,
) -> Result<(), OpsError> {
    let restart = transport.exec("systemctl restart xochitl").await?;
    if restart.exit_status != 0 {
        return Err(TransportError::CommandFailed {
            command: "systemctl restart xochitl".to_string(),
            exit_status: restart.exit_status,
            stderr: restart.stderr,
        }
        .into());
    }

    for _ in 0..max_attempts {
        let status = transport.exec("systemctl is-active xochitl").await?;
        if status.stdout.trim() == "active" {
            return Ok(());
        }
        tokio::time::sleep(retry_delay).await;
    }

    Err(TransportError::RestartTimedOut.into())
}

pub async fn test_connection(config: &Config) -> Result<(), OpsError> {
    let mut transport = RemoteTransport::connect(config).await?;
    let _ = transport.list_names(XOCHITL_PATH).await?;
    transport.disconnect().await?;
    Ok(())
}

pub async fn connect(config: &Config) -> Result<RemoteTransport, OpsError> {
    Ok(RemoteTransport::connect(config).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::ExecOutput;
    use std::collections::{HashSet, VecDeque};
    use std::sync::Mutex;

    /// In-memory stand-in for the real tablet, used so these tests never
    /// touch the network. Keys in `files`/`dirs` are the full remote paths
    /// exactly as production code would pass them.
    #[derive(Default)]
    struct FakeTransport {
        files: Mutex<HashMap<String, Vec<u8>>>,
        dirs: Mutex<HashSet<String>>,
        exec_responses: Mutex<VecDeque<ExecOutput>>,
        rename_calls: Mutex<Vec<(String, String)>>,
        fail_writes_containing: Mutex<Option<String>>,
    }

    impl FakeTransport {
        fn failing_on(needle: &str) -> Self {
            let transport = Self::default();
            *transport.fail_writes_containing.lock().unwrap() = Some(needle.to_string());
            transport
        }

        fn queue_exec(&self, output: ExecOutput) {
            self.exec_responses.lock().unwrap().push_back(output);
        }

        fn seed_metadata(&self, uuid: &str, metadata: &Metadata) {
            self.files.lock().unwrap().insert(
                format!("{XOCHITL_PATH}/{uuid}.metadata"),
                serde_json::to_vec(metadata).unwrap(),
            );
        }
    }

    #[async_trait::async_trait]
    impl SftpTransport for FakeTransport {
        async fn write(&self, path: &str, data: &[u8]) -> Result<(), TransportError> {
            if let Some(needle) = self.fail_writes_containing.lock().unwrap().clone() {
                if path.contains(&needle) {
                    return Err(TransportError::Simulated(format!("forced failure writing {path}")));
                }
            }
            self.files.lock().unwrap().insert(path.to_string(), data.to_vec());
            Ok(())
        }

        async fn mkdir(&self, path: &str) -> Result<(), TransportError> {
            self.dirs.lock().unwrap().insert(path.to_string());
            Ok(())
        }

        async fn rename(&self, from: &str, to: &str) -> Result<(), TransportError> {
            self.rename_calls
                .lock()
                .unwrap()
                .push((from.to_string(), to.to_string()));
            let moved_file = self.files.lock().unwrap().remove(from);
            if let Some(data) = moved_file {
                self.files.lock().unwrap().insert(to.to_string(), data);
            } else if self.dirs.lock().unwrap().remove(from) {
                self.dirs.lock().unwrap().insert(to.to_string());
            }
            Ok(())
        }

        async fn remove_file(&self, path: &str) -> Result<(), TransportError> {
            self.files.lock().unwrap().remove(path);
            Ok(())
        }

        async fn remove_dir(&self, path: &str) -> Result<(), TransportError> {
            self.dirs.lock().unwrap().remove(path);
            Ok(())
        }

        async fn read(&self, path: &str) -> Result<Vec<u8>, TransportError> {
            self.files
                .lock()
                .unwrap()
                .get(path)
                .cloned()
                .ok_or_else(|| TransportError::Simulated(format!("no such file: {path}")))
        }

        async fn list_names(&self, path: &str) -> Result<Vec<String>, TransportError> {
            let prefix = format!("{path}/");
            Ok(self
                .files
                .lock()
                .unwrap()
                .keys()
                .filter_map(|k| k.strip_prefix(&prefix))
                .map(|s| s.to_string())
                .collect())
        }

        async fn file_size(&self, path: &str) -> Result<u64, TransportError> {
            Ok(self
                .files
                .lock()
                .unwrap()
                .get(path)
                .map(|v| v.len() as u64)
                .unwrap_or(0))
        }

        async fn exec(&self, _command: &str) -> Result<ExecOutput, TransportError> {
            self.exec_responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| TransportError::Simulated("no more queued exec responses".to_string()))
        }
    }

    fn sample_document(parent: &str, name: &str) -> Metadata {
        Metadata {
            created_time: "0".into(),
            last_modified: "0".into(),
            last_opened: "0".into(),
            last_opened_page: 0,
            parent: parent.into(),
            pinned: false,
            entry_type: EntryType::DocumentType,
            visible_name: name.into(),
        }
    }

    #[tokio::test]
    async fn resolve_or_create_path_creates_missing_segments_once() {
        let transport = FakeTransport::default();
        let mut tree = FolderTree::new(HashMap::new());

        let uuid = resolve_or_create_path(&transport, &mut tree, "Livres//mangas")
            .await
            .unwrap();
        assert_eq!(tree.metadata.get(&uuid).unwrap().visible_name, "mangas");
        assert_eq!(
            tree.metadata.values().filter(|m| m.visible_name == "Livres").count(),
            1
        );

        // Resolving the same path again must reuse the existing folders, not
        // create duplicates.
        let uuid_again = resolve_or_create_path(&transport, &mut tree, "Livres//mangas")
            .await
            .unwrap();
        assert_eq!(uuid, uuid_again);
        assert_eq!(
            tree.metadata.values().filter(|m| m.visible_name == "mangas").count(),
            1
        );
    }

    #[tokio::test]
    async fn resolve_or_create_path_empty_is_root_with_no_writes() {
        let transport = FakeTransport::default();
        let mut tree = FolderTree::new(HashMap::new());

        let uuid = resolve_or_create_path(&transport, &mut tree, "").await.unwrap();
        assert_eq!(uuid, ROOT);
        assert!(transport.files.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn write_staged_never_renames_on_partial_failure() {
        let transport = FakeTransport::failing_on(".content");
        let files = vec![
            (format!("{XOCHITL_PATH}/x.metadata"), b"a".to_vec()),
            (format!("{XOCHITL_PATH}/x.content"), b"b".to_vec()),
        ];

        let result = write_staged(&transport, &files, &[]).await;

        assert!(result.is_err());
        assert!(transport.rename_calls.lock().unwrap().is_empty());
        // The staged metadata write that *did* succeed must be cleaned up
        // rather than left behind as an orphaned `.partial` file.
        assert!(transport.files.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn move_entry_reparents_existing_document_and_creates_target_folder() {
        let transport = FakeTransport::default();
        transport.seed_metadata("doc-1", &sample_document(ROOT, "book.pdf"));

        move_entry(&transport, "doc-1", "Livres//mangas").await.unwrap();

        let updated: Metadata = serde_json::from_slice(
            &transport.files.lock().unwrap()[&format!("{XOCHITL_PATH}/doc-1.metadata")],
        )
        .unwrap();
        assert_ne!(updated.parent, ROOT);
        assert_eq!(updated.visible_name, "book.pdf");

        let tree = FolderTree::new(fetch_all_metadata(&transport).await.unwrap());
        assert_eq!(tree.resolve_existing("Livres//mangas"), Some(updated.parent));
    }

    #[tokio::test]
    async fn move_entry_fails_for_unknown_uuid() {
        let transport = FakeTransport::default();
        let err = move_entry(&transport, "does-not-exist", "").await.unwrap_err();
        assert!(matches!(err, OpsError::NotFound(_)));
    }

    #[tokio::test]
    async fn restart_xochitl_succeeds_immediately_when_already_active() {
        let transport = FakeTransport::default();
        transport.queue_exec(ExecOutput {
            exit_status: 0,
            stdout: String::new(),
            stderr: String::new(),
        });
        transport.queue_exec(ExecOutput {
            exit_status: 0,
            stdout: "active".to_string(),
            stderr: String::new(),
        });

        restart_xochitl_with_retry(&transport, 3, Duration::from_millis(1))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn restart_xochitl_retries_until_active() {
        let transport = FakeTransport::default();
        transport.queue_exec(ExecOutput {
            exit_status: 0,
            stdout: String::new(),
            stderr: String::new(),
        });
        transport.queue_exec(ExecOutput {
            exit_status: 0,
            stdout: "activating".to_string(),
            stderr: String::new(),
        });
        transport.queue_exec(ExecOutput {
            exit_status: 0,
            stdout: "active".to_string(),
            stderr: String::new(),
        });

        restart_xochitl_with_retry(&transport, 3, Duration::from_millis(1))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn restart_xochitl_fails_if_restart_command_errors() {
        let transport = FakeTransport::default();
        transport.queue_exec(ExecOutput {
            exit_status: 1,
            stdout: String::new(),
            stderr: "unit xochitl.service not found".to_string(),
        });

        let err = restart_xochitl_with_retry(&transport, 3, Duration::from_millis(1))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            OpsError::Transport(TransportError::CommandFailed { .. })
        ));
    }

    #[tokio::test]
    async fn restart_xochitl_times_out_if_never_active() {
        let transport = FakeTransport::default();
        transport.queue_exec(ExecOutput {
            exit_status: 0,
            stdout: String::new(),
            stderr: String::new(),
        });
        for _ in 0..5 {
            transport.queue_exec(ExecOutput {
                exit_status: 0,
                stdout: "activating".to_string(),
                stderr: String::new(),
            });
        }

        let err = restart_xochitl_with_retry(&transport, 3, Duration::from_millis(1))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            OpsError::Transport(TransportError::RestartTimedOut)
        ));
    }
}
