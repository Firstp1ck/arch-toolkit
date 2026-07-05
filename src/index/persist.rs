//! Index persistence functions for loading and saving the official package index.

use std::path::Path;

use crate::error::{ArchToolkitError, Result};
use crate::types::index::OfficialIndex;

/// What: Load an official package index from a JSON file on disk.
///
/// Inputs:
/// - `path`: File path to read the JSON index from.
///
/// Output:
/// - Returns `Ok(OfficialIndex)` with the deserialized index on success.
/// - Returns `Err(ArchToolkitError::Io)` if the file cannot be read.
/// - Returns `Err(ArchToolkitError::Json)` if the content is not valid index JSON.
///
/// Details:
/// - Rebuilds the `name_to_idx` `HashMap` after deserialization so O(1) name
///   lookups via `find_package_by_name()` work immediately.
/// - Unlike Pacsea's original implementation, errors are propagated instead of
///   silently ignored; callers decide how to handle a missing or corrupt index.
///
/// # Errors
///
/// Returns an error if the file cannot be read or the JSON cannot be parsed.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::index::load_from_disk;
/// use std::path::Path;
///
/// let index = load_from_disk(Path::new("official_index.json"))?;
/// println!("Loaded {} packages", index.pkgs.len());
/// # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
/// ```
pub fn load_from_disk(path: &Path) -> Result<OfficialIndex> {
    tracing::debug!(path = %path.display(), "Loading official index from disk");
    let content = std::fs::read_to_string(path)
        .map_err(|e| ArchToolkitError::io(path.display().to_string(), e))?;
    let mut index: OfficialIndex = serde_json::from_str(&content)?;
    index.rebuild_name_index();
    tracing::debug!(
        path = %path.display(),
        package_count = index.pkgs.len(),
        "Successfully loaded official index"
    );
    Ok(index)
}

/// What: Persist an official package index to a JSON file on disk.
///
/// Inputs:
/// - `index`: The index to serialize and write.
/// - `path`: File path to write the JSON index to.
///
/// Output:
/// - Returns `Ok(())` on success.
/// - Returns `Err(ArchToolkitError::Io)` if the directory or file cannot be written.
/// - Returns `Err(ArchToolkitError::Json)` if serialization fails.
///
/// Details:
/// - Creates the parent directory if it does not exist.
/// - The derived `name_to_idx` field is skipped during serialization; it is
///   rebuilt on load via `load_from_disk()`.
/// - Logs a warning when saving an empty index, since that usually indicates
///   the index was never populated.
///
/// # Errors
///
/// Returns an error if the parent directory cannot be created, serialization
/// fails, or the file cannot be written.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::index::{save_to_disk, OfficialIndex};
/// use std::path::Path;
///
/// let index = OfficialIndex::default();
/// save_to_disk(&index, Path::new("official_index.json"))?;
/// # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
/// ```
pub fn save_to_disk(index: &OfficialIndex, path: &Path) -> Result<()> {
    if index.pkgs.is_empty() {
        tracing::warn!(
            path = %path.display(),
            "Saving empty index to disk"
        );
    }
    let json = serde_json::to_string(index)?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| ArchToolkitError::io(parent.display().to_string(), e))?;
    }
    std::fs::write(path, json).map_err(|e| ArchToolkitError::io(path.display().to_string(), e))?;
    tracing::debug!(
        path = %path.display(),
        package_count = index.pkgs.len(),
        "Successfully saved official index to disk"
    );
    Ok(())
}

/// What: Load an official package index from disk asynchronously.
///
/// Inputs:
/// - `path`: File path to read the JSON index from.
///
/// Output:
/// - Returns a future resolving to `Result<OfficialIndex>` (same semantics as `load_from_disk`).
///
/// Details:
/// - Uses `tokio::task::spawn_blocking` to avoid blocking the async runtime on file I/O.
///
/// # Errors
///
/// Returns an error if the blocking task fails, the file cannot be read, or
/// the JSON cannot be parsed.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::index::load_from_disk_async;
/// use std::path::PathBuf;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let index = load_from_disk_async(PathBuf::from("official_index.json")).await?;
/// println!("Loaded {} packages", index.pkgs.len());
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "index")]
pub async fn load_from_disk_async(path: std::path::PathBuf) -> Result<OfficialIndex> {
    tokio::task::spawn_blocking(move || load_from_disk(&path))
        .await
        .map_err(|e| ArchToolkitError::Parse(format!("Blocking task failed: {e}")))?
}

/// What: Persist an official package index to disk asynchronously.
///
/// Inputs:
/// - `index`: The index to serialize and write (moved into the blocking task).
/// - `path`: File path to write the JSON index to.
///
/// Output:
/// - Returns a future resolving to `Result<()>` (same semantics as `save_to_disk`).
///
/// Details:
/// - Uses `tokio::task::spawn_blocking` to avoid blocking the async runtime on file I/O.
/// - Takes the index by value because the blocking task requires `'static` data;
///   clone before calling if the index is still needed.
///
/// # Errors
///
/// Returns an error if the blocking task fails, the parent directory cannot be
/// created, serialization fails, or the file cannot be written.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::index::{save_to_disk_async, OfficialIndex};
/// use std::path::PathBuf;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let index = OfficialIndex::default();
/// save_to_disk_async(index, PathBuf::from("official_index.json")).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "index")]
pub async fn save_to_disk_async(index: OfficialIndex, path: std::path::PathBuf) -> Result<()> {
    tokio::task::spawn_blocking(move || save_to_disk(&index, &path))
        .await
        .map_err(|e| ArchToolkitError::Parse(format!("Blocking task failed: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::index::OfficialPackage;

    fn sample_index() -> OfficialIndex {
        let mut index = OfficialIndex {
            pkgs: vec![
                OfficialPackage {
                    name: "ripgrep".to_string(),
                    repo: "extra".to_string(),
                    arch: "x86_64".to_string(),
                    version: "14.0.0".to_string(),
                    description: "Fast grep".to_string(),
                },
                OfficialPackage {
                    name: "vim".to_string(),
                    repo: "extra".to_string(),
                    arch: "x86_64".to_string(),
                    version: "9.0".to_string(),
                    description: "Text editor".to_string(),
                },
            ],
            name_to_idx: std::collections::HashMap::new(),
        };
        index.rebuild_name_index();
        index
    }

    fn temp_path(tag: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "arch_toolkit_persist_{tag}_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("System time is before UNIX epoch")
                .as_nanos()
        ));
        path
    }

    #[test]
    /// What: Verify a saved index round-trips through disk unchanged.
    ///
    /// Inputs:
    /// - Sample index with two packages saved to a temp file.
    ///
    /// Output:
    /// - Loaded index has the same packages and a rebuilt name index.
    ///
    /// Details:
    /// - Confirms `name_to_idx` is rebuilt on load so O(1) lookups work.
    fn save_and_load_roundtrip() {
        let index = sample_index();
        let path = temp_path("roundtrip");

        save_to_disk(&index, &path).expect("save should succeed");
        let loaded = load_from_disk(&path).expect("load should succeed");

        assert_eq!(loaded.pkgs, index.pkgs);
        assert_eq!(loaded.name_to_idx.len(), 2);
        let found = loaded.find_package_by_name("RIPGREP");
        assert_eq!(found.map(|p| p.name.as_str()), Some("ripgrep"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    /// What: Verify loading a missing file returns an `Io` error.
    ///
    /// Inputs:
    /// - Path that does not exist on disk.
    ///
    /// Output:
    /// - `Err(ArchToolkitError::Io)` with the path in the message.
    ///
    /// Details:
    /// - Errors are propagated (not silently ignored) so callers can fall back to fetching.
    fn load_missing_file_returns_io_error() {
        let path = temp_path("missing");
        let result = load_from_disk(&path);
        match result {
            Err(ArchToolkitError::Io { path: p, .. }) => {
                assert!(p.contains("arch_toolkit_persist_missing"));
            }
            other => panic!("Expected Io error, got {other:?}"),
        }
    }

    #[test]
    /// What: Verify loading invalid JSON returns a `Json` error.
    ///
    /// Inputs:
    /// - Temp file containing invalid JSON content.
    ///
    /// Output:
    /// - `Err(ArchToolkitError::Json)`.
    ///
    /// Details:
    /// - Corrupt cache files must surface as errors instead of empty indices.
    fn load_invalid_json_returns_json_error() {
        let path = temp_path("invalid");
        std::fs::write(&path, "not valid json {").expect("write should succeed");

        let result = load_from_disk(&path);
        assert!(matches!(result, Err(ArchToolkitError::Json(_))));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    /// What: Verify saving creates missing parent directories.
    ///
    /// Inputs:
    /// - Path with a non-existent parent directory.
    ///
    /// Output:
    /// - Save succeeds and the file exists.
    ///
    /// Details:
    /// - Matches Pacsea behavior of creating parent directories before writing.
    fn save_creates_parent_directories() {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "arch_toolkit_persist_dir_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("System time is before UNIX epoch")
                .as_nanos()
        ));
        let path = dir.join("nested").join("index.json");

        let index = sample_index();
        save_to_disk(&index, &path).expect("save should create parent dirs");
        assert!(path.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    /// What: Verify saving an empty index succeeds (with a warning logged).
    ///
    /// Inputs:
    /// - Default (empty) `OfficialIndex`.
    ///
    /// Output:
    /// - Save succeeds and the loaded index is empty.
    ///
    /// Details:
    /// - Empty indices are legal but unusual; they warn rather than fail.
    fn save_empty_index_succeeds() {
        let index = OfficialIndex::default();
        let path = temp_path("empty");

        save_to_disk(&index, &path).expect("save should succeed");
        let loaded = load_from_disk(&path).expect("load should succeed");
        assert!(loaded.pkgs.is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    /// What: Verify the serialized file does not contain the derived name index.
    ///
    /// Inputs:
    /// - Sample index with populated `name_to_idx` saved to disk.
    ///
    /// Output:
    /// - File content lacks the `name_to_idx` key.
    ///
    /// Details:
    /// - The lookup map is derived data and must be rebuilt on load, not persisted.
    fn saved_file_skips_name_index() {
        let index = sample_index();
        let path = temp_path("skipidx");

        save_to_disk(&index, &path).expect("save should succeed");
        let content = std::fs::read_to_string(&path).expect("read should succeed");
        assert!(!content.contains("name_to_idx"));

        let _ = std::fs::remove_file(&path);
    }

    #[cfg(feature = "index")]
    #[tokio::test]
    /// What: Verify the async save/load variants round-trip correctly.
    ///
    /// Inputs:
    /// - Sample index saved and loaded via the async functions.
    ///
    /// Output:
    /// - Loaded index matches the original.
    ///
    /// Details:
    /// - Exercises the `spawn_blocking` wrappers end to end.
    async fn async_save_and_load_roundtrip() {
        let index = sample_index();
        let path = temp_path("async");

        save_to_disk_async(index.clone(), path.clone())
            .await
            .expect("async save should succeed");
        let loaded = load_from_disk_async(path.clone())
            .await
            .expect("async load should succeed");

        assert_eq!(loaded.pkgs, index.pkgs);
        assert_eq!(loaded.name_to_idx.len(), 2);

        let _ = std::fs::remove_file(&path);
    }
}
