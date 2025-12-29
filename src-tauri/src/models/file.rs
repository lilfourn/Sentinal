use serde::{Deserialize, Serialize};

/// Represents a single file or directory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    /// File or directory name
    pub name: String,
    /// Absolute path
    pub path: String,
    /// Whether this is a directory
    pub is_directory: bool,
    /// Whether this is a file
    pub is_file: bool,
    /// Whether this is a symbolic link
    pub is_symlink: bool,
    /// File size in bytes (0 for directories)
    pub size: u64,
    /// Last modified timestamp (milliseconds since epoch)
    pub modified_at: Option<i64>,
    /// Created timestamp (milliseconds since epoch)
    pub created_at: Option<i64>,
    /// File extension (without dot), None for directories
    pub extension: Option<String>,
    /// MIME type guess based on extension
    pub mime_type: Option<String>,
    /// Whether file is hidden (starts with . on Unix, hidden attribute on Windows)
    pub is_hidden: bool,
}

/// Represents a directory with its contents
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryContents {
    /// Path of this directory
    pub path: String,
    /// Directory name
    pub name: String,
    /// Parent directory path (None for root)
    pub parent_path: Option<String>,
    /// List of entries in this directory
    pub entries: Vec<FileEntry>,
    /// Total count of entries
    pub total_count: usize,
}

/// File metadata for detailed info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_directory: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub is_readonly: bool,
    pub modified_at: Option<i64>,
    pub created_at: Option<i64>,
    pub accessed_at: Option<i64>,
    pub extension: Option<String>,
    pub mime_type: Option<String>,
}

impl FileEntry {
    /// Create a FileEntry from a path
    pub fn from_path(path: &std::path::Path) -> std::io::Result<Self> {
        let metadata = std::fs::symlink_metadata(path)?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let is_symlink = metadata.is_symlink();
        let is_directory = if is_symlink {
            path.is_dir()
        } else {
            metadata.is_dir()
        };
        let is_file = if is_symlink {
            path.is_file()
        } else {
            metadata.is_file()
        };

        let extension = if is_file {
            path.extension().map(|e| e.to_string_lossy().to_string())
        } else {
            None
        };

        let mime_type = extension.as_ref().and_then(|ext| {
            mime_guess::from_ext(ext)
                .first()
                .map(|m| m.to_string())
        });

        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64);

        let created_at = metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64);

        let is_hidden = name.starts_with('.');

        Ok(Self {
            name,
            path: path.to_string_lossy().to_string(),
            is_directory,
            is_file,
            is_symlink,
            size: if is_file { metadata.len() } else { 0 },
            modified_at,
            created_at,
            extension,
            mime_type,
            is_hidden,
        })
    }
}
