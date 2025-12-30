//! Tree Compression Module
//!
//! Creates token-optimized XML representations of large directories for AI context.
//! Intelligently collapses homogeneous folders to reduce token usage while
//! preserving semantic understanding.

#![allow(dead_code)]

pub mod compressor;
pub mod xml_writer;

pub use compressor::*;
pub use xml_writer::*;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for tree compression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeConfig {
    /// Number of files above which a folder may be collapsed
    pub collapse_threshold: usize,
    /// Maximum depth to traverse (prevents runaway recursion)
    pub max_depth: usize,
    /// Whether to include semantic tags from vector index
    pub include_tags: bool,
    /// Entropy threshold below which folders are collapsed (0.0 to 1.0)
    /// Low entropy = homogeneous content = good candidate for collapse
    pub entropy_threshold: f64,
}

impl Default for TreeConfig {
    fn default() -> Self {
        Self {
            collapse_threshold: 50,
            max_depth: 10,
            include_tags: true,
            entropy_threshold: 0.5,
        }
    }
}

/// A node in the compressed tree structure
///
/// Can represent either a fully expanded folder, a collapsed summary,
/// or a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressedNode {
    /// Absolute path to the file or folder
    pub path: PathBuf,
    /// Display name
    pub name: String,
    /// Whether this node represents a collapsed folder
    pub is_collapsed: bool,
    /// Summary description for collapsed folders
    pub summary: Option<CollapsedSummary>,
    /// Child nodes (empty for files or collapsed folders)
    pub children: Vec<CompressedNode>,
    /// Semantic tags from vector index
    pub tags: Vec<String>,
    /// Whether this is a directory
    pub is_directory: bool,
    /// File size in bytes (for files only)
    pub size: Option<u64>,
    /// File extension (for files only)
    pub extension: Option<String>,
}

/// Summary information for a collapsed folder
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollapsedSummary {
    /// Total number of files in the collapsed folder
    pub file_count: usize,
    /// Total number of subdirectories
    pub dir_count: usize,
    /// Total size of all files
    pub total_size: u64,
    /// Predominant file type (e.g., "image", "document")
    pub primary_type: Option<String>,
    /// Human-readable description (e.g., "47 PDF files, 23 images")
    pub description: String,
    /// Common semantic tags across files
    pub common_tags: Vec<String>,
    /// Date range of files (e.g., "2023-01 to 2024-12")
    pub date_range: Option<String>,
    /// File type breakdown (extension -> count)
    pub type_breakdown: Vec<(String, usize)>,
}

impl CompressedNode {
    /// Create a new file node
    pub fn file(path: PathBuf, name: String, size: u64, extension: Option<String>, tags: Vec<String>) -> Self {
        Self {
            path,
            name,
            is_collapsed: false,
            summary: None,
            children: Vec::new(),
            tags,
            is_directory: false,
            size: Some(size),
            extension,
        }
    }

    /// Create a new folder node
    pub fn folder(path: PathBuf, name: String, children: Vec<CompressedNode>, tags: Vec<String>) -> Self {
        Self {
            path,
            name,
            is_collapsed: false,
            summary: None,
            children,
            tags,
            is_directory: true,
            size: None,
            extension: None,
        }
    }

    /// Create a collapsed folder node with a summary
    pub fn collapsed(path: PathBuf, name: String, summary: CollapsedSummary, tags: Vec<String>) -> Self {
        Self {
            path,
            name,
            is_collapsed: true,
            summary: Some(summary),
            children: Vec::new(),
            tags,
            is_directory: true,
            size: None,
            extension: None,
        }
    }

    /// Get total node count (recursive)
    pub fn node_count(&self) -> usize {
        1 + self.children.iter().map(|c| c.node_count()).sum::<usize>()
    }

    /// Get total file count (recursive)
    pub fn file_count(&self) -> usize {
        if self.is_collapsed {
            self.summary.as_ref().map(|s| s.file_count).unwrap_or(0)
        } else if !self.is_directory {
            1
        } else {
            self.children.iter().map(|c| c.file_count()).sum()
        }
    }
}

/// Format a byte size as human-readable string
///
/// Examples: "1.2MB", "450KB", "23B"
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1}TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

/// Format a date range from timestamps
///
/// Returns something like "2023-01 to 2024-12" or "2024-06" if same month
pub fn format_date_range(min_timestamp: i64, max_timestamp: i64) -> String {
    use chrono::{DateTime, Utc};

    let min_date = DateTime::from_timestamp_millis(min_timestamp)
        .map(|d: DateTime<Utc>| d.format("%Y-%m").to_string());

    let max_date = DateTime::from_timestamp_millis(max_timestamp)
        .map(|d: DateTime<Utc>| d.format("%Y-%m").to_string());

    match (min_date, max_date) {
        (Some(min), Some(max)) if min == max => min,
        (Some(min), Some(max)) => format!("{} to {}", min, max),
        (Some(min), None) => min,
        (None, Some(max)) => max,
        (None, None) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500B");
        assert_eq!(format_size(1024), "1KB");
        assert_eq!(format_size(1536), "2KB");
        assert_eq!(format_size(1024 * 1024), "1.0MB");
        assert_eq!(format_size(1_500_000), "1.4MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0GB");
    }

    #[test]
    fn test_compressed_node_file() {
        let node = CompressedNode::file(
            PathBuf::from("/test/file.pdf"),
            "file.pdf".to_string(),
            1024,
            Some("pdf".to_string()),
            vec!["document".to_string()],
        );

        assert!(!node.is_directory);
        assert!(!node.is_collapsed);
        assert_eq!(node.file_count(), 1);
    }

    #[test]
    fn test_compressed_node_folder() {
        let child = CompressedNode::file(
            PathBuf::from("/test/folder/file.pdf"),
            "file.pdf".to_string(),
            1024,
            Some("pdf".to_string()),
            vec![],
        );

        let node = CompressedNode::folder(
            PathBuf::from("/test/folder"),
            "folder".to_string(),
            vec![child],
            vec![],
        );

        assert!(node.is_directory);
        assert!(!node.is_collapsed);
        assert_eq!(node.file_count(), 1);
        assert_eq!(node.node_count(), 2);
    }
}
