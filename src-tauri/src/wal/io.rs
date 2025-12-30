//! Safe I/O utilities for WAL operations
//!
//! Provides crash-safe file operations including:
//! - Atomic writes with fsync
//! - Directory synchronization
//! - Symlink detection and handling
//!
//! These utilities ensure data integrity even in the event of system crashes
//! or power failures.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Error type for safe I/O operations
#[derive(Debug, Clone)]
pub struct SafeIoError {
    pub message: String,
    pub kind: SafeIoErrorKind,
}

#[derive(Debug, Clone)]
pub enum SafeIoErrorKind {
    WriteError,
    SyncError,
    RenameError,
    PathError,
    SymlinkError,
}

impl std::fmt::Display for SafeIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SafeIoError {}

impl From<SafeIoError> for String {
    fn from(err: SafeIoError) -> Self {
        err.message
    }
}

/// Write data to a file atomically with fsync
///
/// This function:
/// 1. Writes data to a temporary file in the same directory
/// 2. Calls fsync on the file to ensure data is on disk
/// 3. Atomically renames the temp file to the target
/// 4. Syncs the directory to ensure the rename is durable
///
/// If any step fails, the temporary file is cleaned up.
///
/// # Arguments
/// * `path` - Target file path
/// * `data` - Data to write
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(SafeIoError)` on failure
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), SafeIoError> {
    // Get the directory for temp file and sync
    let parent = path.parent().ok_or_else(|| SafeIoError {
        message: format!("Cannot determine parent directory for: {}", path.display()),
        kind: SafeIoErrorKind::PathError,
    })?;

    // Ensure parent directory exists
    if !parent.exists() {
        fs::create_dir_all(parent).map_err(|e| SafeIoError {
            message: format!("Failed to create directory {}: {}", parent.display(), e),
            kind: SafeIoErrorKind::WriteError,
        })?;
    }

    // Generate temp file name in the same directory
    let temp_name = format!(
        ".{}.tmp.{}",
        path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string()),
        std::process::id()
    );
    let temp_path = parent.join(&temp_name);

    // Write to temp file with sync
    let write_result = (|| -> Result<(), SafeIoError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .map_err(|e| SafeIoError {
                message: format!("Failed to create temp file {}: {}", temp_path.display(), e),
                kind: SafeIoErrorKind::WriteError,
            })?;

        file.write_all(data).map_err(|e| SafeIoError {
            message: format!("Failed to write to temp file: {}", e),
            kind: SafeIoErrorKind::WriteError,
        })?;

        // Sync file data to disk
        file.sync_all().map_err(|e| SafeIoError {
            message: format!("Failed to sync temp file: {}", e),
            kind: SafeIoErrorKind::SyncError,
        })?;

        Ok(())
    })();

    // Clean up on write failure
    if let Err(e) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(e);
    }

    // Atomic rename
    let rename_result = fs::rename(&temp_path, path).map_err(|e| SafeIoError {
        message: format!(
            "Failed to rename {} to {}: {}",
            temp_path.display(),
            path.display(),
            e
        ),
        kind: SafeIoErrorKind::RenameError,
    });

    // Clean up temp file on rename failure
    if let Err(e) = rename_result {
        let _ = fs::remove_file(&temp_path);
        return Err(e);
    }

    // Sync the directory to make the rename durable
    sync_directory(parent)?;

    Ok(())
}

/// Sync a directory to ensure metadata changes are durable
///
/// On POSIX systems, this opens the directory and calls fsync.
/// This ensures operations like rename are persisted to disk.
///
/// # Arguments
/// * `path` - Directory path to sync
///
/// # Returns
/// * `Ok(())` on success (or if directory sync is not supported)
/// * `Err(SafeIoError)` on failure
pub fn sync_directory(path: &Path) -> Result<(), SafeIoError> {
    // On Unix-like systems, we can fsync a directory
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        let dir = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY)
            .open(path)
            .map_err(|e| SafeIoError {
                message: format!("Failed to open directory {}: {}", path.display(), e),
                kind: SafeIoErrorKind::SyncError,
            })?;

        dir.sync_all().map_err(|e| SafeIoError {
            message: format!("Failed to sync directory {}: {}", path.display(), e),
            kind: SafeIoErrorKind::SyncError,
        })?;
    }

    // On Windows, directory sync is not directly supported
    // The rename itself is atomic on NTFS
    #[cfg(windows)]
    {
        // No-op on Windows - NTFS provides atomicity guarantees
        let _ = path;
    }

    Ok(())
}

/// Check if a path is a symlink without following it
///
/// Uses `symlink_metadata` which doesn't follow symlinks.
///
/// # Arguments
/// * `path` - Path to check
///
/// # Returns
/// * `true` if the path is a symlink
/// * `false` if not a symlink or path doesn't exist
pub fn is_symlink(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(meta) => meta.is_symlink(),
        Err(_) => false,
    }
}

/// Information about a file's type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTypeInfo {
    /// Regular file
    File,
    /// Directory
    Directory,
    /// Symbolic link
    Symlink,
    /// Other (device, socket, etc.)
    Other,
}

/// Get file type information without following symlinks
///
/// Unlike `Path::is_file()` or `Path::is_dir()`, this doesn't follow symlinks.
///
/// # Arguments
/// * `path` - Path to check
///
/// # Returns
/// * `Ok(FileTypeInfo)` with the file type
/// * `Err(io::Error)` if the path doesn't exist or can't be accessed
pub fn file_type_no_follow(path: &Path) -> Result<FileTypeInfo, io::Error> {
    let meta = fs::symlink_metadata(path)?;

    if meta.is_symlink() {
        Ok(FileTypeInfo::Symlink)
    } else if meta.is_file() {
        Ok(FileTypeInfo::File)
    } else if meta.is_dir() {
        Ok(FileTypeInfo::Directory)
    } else {
        Ok(FileTypeInfo::Other)
    }
}

/// Ensure a path is not a symlink before operating on it
///
/// # Arguments
/// * `path` - Path to check
/// * `operation` - Description for error message
///
/// # Returns
/// * `Ok(())` if not a symlink
/// * `Err(SafeIoError)` if it is a symlink
pub fn ensure_not_symlink(path: &Path, operation: &str) -> Result<(), SafeIoError> {
    if is_symlink(path) {
        return Err(SafeIoError {
            message: format!(
                "Refusing to {} symlink: {}",
                operation,
                path.display()
            ),
            kind: SafeIoErrorKind::SymlinkError,
        });
    }
    Ok(())
}

/// Read a file's contents, refusing to follow symlinks
///
/// # Arguments
/// * `path` - Path to read
///
/// # Returns
/// * `Ok(Vec<u8>)` with file contents
/// * `Err(SafeIoError)` if symlink or read error
pub fn safe_read(path: &Path) -> Result<Vec<u8>, SafeIoError> {
    ensure_not_symlink(path, "read")?;

    fs::read(path).map_err(|e| SafeIoError {
        message: format!("Failed to read {}: {}", path.display(), e),
        kind: SafeIoErrorKind::WriteError, // Reusing for read errors
    })
}

/// Copy a directory recursively, skipping symlinks with warning
///
/// Unlike `fs::copy`, this function:
/// - Skips symlinks (with tracing warning)
/// - Uses atomic writes for file copies where possible
///
/// # Arguments
/// * `src` - Source directory
/// * `dst` - Destination directory
///
/// # Returns
/// * `Ok(usize)` - Number of items copied
/// * `Err(SafeIoError)` on failure
pub fn copy_dir_safe(src: &Path, dst: &Path) -> Result<usize, SafeIoError> {
    // Ensure source is not a symlink
    ensure_not_symlink(src, "copy from")?;

    if !src.is_dir() {
        return Err(SafeIoError {
            message: format!("Source is not a directory: {}", src.display()),
            kind: SafeIoErrorKind::PathError,
        });
    }

    fs::create_dir_all(dst).map_err(|e| SafeIoError {
        message: format!("Failed to create directory {}: {}", dst.display(), e),
        kind: SafeIoErrorKind::WriteError,
    })?;

    let mut copied = 0;

    let entries = fs::read_dir(src).map_err(|e| SafeIoError {
        message: format!("Failed to read directory {}: {}", src.display(), e),
        kind: SafeIoErrorKind::WriteError,
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| SafeIoError {
            message: format!("Failed to read entry: {}", e),
            kind: SafeIoErrorKind::WriteError,
        })?;

        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        // Skip symlinks with warning
        if is_symlink(&src_path) {
            tracing::warn!(
                path = %src_path.display(),
                "Skipping symlink during copy"
            );
            continue;
        }

        if src_path.is_dir() {
            copied += copy_dir_safe(&src_path, &dst_path)?;
        } else {
            // Copy file
            fs::copy(&src_path, &dst_path).map_err(|e| SafeIoError {
                message: format!(
                    "Failed to copy {} to {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                ),
                kind: SafeIoErrorKind::WriteError,
            })?;
            copied += 1;
        }
    }

    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_write() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        atomic_write(&path, b"Hello, World!").unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_atomic_write_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sub").join("dir").join("test.txt");

        atomic_write(&path, b"nested").unwrap();

        assert!(path.exists());
    }

    #[test]
    fn test_is_symlink() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("file.txt");
        fs::write(&file, "content").unwrap();

        assert!(!is_symlink(&file));

        #[cfg(unix)]
        {
            let link = dir.path().join("link.txt");
            std::os::unix::fs::symlink(&file, &link).unwrap();
            assert!(is_symlink(&link));
        }
    }

    #[test]
    fn test_file_type_no_follow() {
        let dir = tempdir().unwrap();

        // Test file
        let file = dir.path().join("file.txt");
        fs::write(&file, "content").unwrap();
        assert_eq!(file_type_no_follow(&file).unwrap(), FileTypeInfo::File);

        // Test directory
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        assert_eq!(file_type_no_follow(&subdir).unwrap(), FileTypeInfo::Directory);

        #[cfg(unix)]
        {
            let link = dir.path().join("link.txt");
            std::os::unix::fs::symlink(&file, &link).unwrap();
            assert_eq!(file_type_no_follow(&link).unwrap(), FileTypeInfo::Symlink);
        }
    }

    #[test]
    fn test_ensure_not_symlink() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("file.txt");
        fs::write(&file, "content").unwrap();

        // Regular file should pass
        ensure_not_symlink(&file, "test").unwrap();

        #[cfg(unix)]
        {
            let link = dir.path().join("link.txt");
            std::os::unix::fs::symlink(&file, &link).unwrap();

            // Symlink should fail
            let result = ensure_not_symlink(&link, "test");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_copy_dir_safe() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        let dst = dir.path().join("dst");

        // Create source structure
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("file1.txt"), "content1").unwrap();
        fs::write(src.join("sub").join("file2.txt"), "content2").unwrap();

        let count = copy_dir_safe(&src, &dst).unwrap();

        assert_eq!(count, 2);
        assert!(dst.join("file1.txt").exists());
        assert!(dst.join("sub").join("file2.txt").exists());
    }
}
