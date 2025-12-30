//! Tree Compressor Module
//!
//! Intelligently compresses directory trees for optimal token usage.
//! Uses Shannon entropy to detect homogeneous folders that can be summarized.

use super::{format_date_range, CollapsedSummary, CompressedNode, TreeConfig};
use crate::models::FileEntry;
use crate::vector::VectorIndex;
use std::collections::HashMap;
use std::path::PathBuf;

/// Tree compressor that creates token-optimized representations
pub struct TreeCompressor {
    config: TreeConfig,
}

impl TreeCompressor {
    /// Create a new tree compressor with the given configuration
    pub fn new(config: TreeConfig) -> Self {
        Self { config }
    }

    /// Compress a directory tree starting from the given root
    ///
    /// # Arguments
    /// * `root` - The root directory to compress
    /// * `vector_index` - Optional vector index for semantic tags
    ///
    /// # Returns
    /// A compressed tree representation suitable for AI context
    pub fn compress(
        &self,
        root: &PathBuf,
        vector_index: Option<&VectorIndex>,
    ) -> Result<CompressedNode, String> {
        if !root.exists() {
            return Err(format!("Path does not exist: {:?}", root));
        }

        if !root.is_dir() {
            return Err(format!("Path is not a directory: {:?}", root));
        }

        self.compress_node(root, 0, vector_index)
    }

    /// Recursively compress a node in the tree
    fn compress_node(
        &self,
        path: &PathBuf,
        depth: usize,
        vector_index: Option<&VectorIndex>,
    ) -> Result<CompressedNode, String> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        // Check depth limit
        if depth >= self.config.max_depth {
            let summary = self.create_depth_limit_summary(path)?;
            let tags = self.get_tags_for_path(path, vector_index);
            return Ok(CompressedNode::collapsed(path.clone(), name, summary, tags));
        }

        // Read directory contents
        let entries = std::fs::read_dir(path)
            .map_err(|e| format!("Failed to read directory {:?}: {}", path, e))?;

        let mut files: Vec<FileEntry> = Vec::new();
        let mut dirs: Vec<PathBuf> = Vec::new();

        for entry in entries.filter_map(|e| e.ok()) {
            let entry_path = entry.path();
            let file_type = entry.file_type().ok();

            // Skip hidden files
            let entry_name = entry.file_name().to_string_lossy().to_string();
            if entry_name.starts_with('.') {
                continue;
            }

            if let Some(ft) = file_type {
                if ft.is_dir() {
                    dirs.push(entry_path);
                } else if ft.is_file() {
                    if let Ok(file_entry) = FileEntry::from_path(&entry_path) {
                        files.push(file_entry);
                    }
                }
            }
        }

        // Decide whether to collapse this folder
        if files.len() >= self.config.collapse_threshold {
            let entropy = self.calculate_entropy(&files);

            // Low entropy means homogeneous content - good candidate for collapse
            if entropy < self.config.entropy_threshold {
                let summary = self.create_summary(&files, &dirs);
                let tags = self.aggregate_tags(&files, vector_index);
                return Ok(CompressedNode::collapsed(path.clone(), name, summary, tags));
            }
        }

        // Build children nodes
        let mut children: Vec<CompressedNode> = Vec::new();

        // Process files
        for file in files {
            let file_path = PathBuf::from(&file.path);
            let tags = self.get_tags_for_path(&file_path, vector_index);

            children.push(CompressedNode::file(
                file_path,
                file.name,
                file.size,
                file.extension,
                tags,
            ));
        }

        // Recursively process subdirectories
        for dir_path in dirs {
            match self.compress_node(&dir_path, depth + 1, vector_index) {
                Ok(node) => children.push(node),
                Err(e) => {
                    eprintln!("[TreeCompressor] Warning: Failed to compress {:?}: {}", dir_path, e);
                    // Create a placeholder for inaccessible directories
                    let dir_name = dir_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    children.push(CompressedNode::folder(
                        dir_path,
                        dir_name,
                        Vec::new(),
                        vec!["inaccessible".to_string()],
                    ));
                }
            }
        }

        // Sort children: directories first, then files, both alphabetically
        children.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        let folder_tags = self.get_tags_for_path(path, vector_index);
        Ok(CompressedNode::folder(path.clone(), name, children, folder_tags))
    }

    /// Calculate Shannon entropy of file types in a folder
    ///
    /// Low entropy = homogeneous (mostly same type)
    /// High entropy = diverse (many different types)
    fn calculate_entropy(&self, files: &[FileEntry]) -> f64 {
        if files.is_empty() {
            return 0.0;
        }

        // Count files by extension
        let mut type_counts: HashMap<String, usize> = HashMap::new();
        for file in files {
            let ext = file.extension.clone().unwrap_or_else(|| "none".to_string());
            *type_counts.entry(ext).or_insert(0) += 1;
        }

        // Calculate Shannon entropy: -sum(p * log2(p))
        let total = files.len() as f64;
        let mut entropy = 0.0;

        for count in type_counts.values() {
            let p = (*count as f64) / total;
            if p > 0.0 {
                entropy -= p * p.log2();
            }
        }

        // Normalize to 0-1 range (max entropy is log2(n) where n is number of types)
        let max_entropy = (type_counts.len() as f64).log2();
        if max_entropy > 0.0 {
            entropy / max_entropy
        } else {
            0.0
        }
    }

    /// Create a summary for a collapsed folder
    fn create_summary(&self, files: &[FileEntry], dirs: &[PathBuf]) -> CollapsedSummary {
        // Count files by type
        let mut type_counts: HashMap<String, usize> = HashMap::new();
        let mut total_size: u64 = 0;
        let mut min_timestamp: Option<i64> = None;
        let mut max_timestamp: Option<i64> = None;

        for file in files {
            let ext = file.extension.clone().unwrap_or_else(|| "other".to_string());
            *type_counts.entry(ext).or_insert(0) += 1;
            total_size += file.size;

            // Track date range
            if let Some(modified) = file.modified_at {
                min_timestamp = Some(min_timestamp.map_or(modified, |m| m.min(modified)));
                max_timestamp = Some(max_timestamp.map_or(modified, |m| m.max(modified)));
            }
        }

        // Sort by count descending
        let mut type_breakdown: Vec<(String, usize)> = type_counts.into_iter().collect();
        type_breakdown.sort_by(|a, b| b.1.cmp(&a.1));

        // Determine primary type
        let primary_type = type_breakdown.first().map(|(ext, _)| {
            Self::extension_to_category(ext)
        });

        // Build description
        let description = self.summarize_children(files, &type_breakdown);

        // Date range
        let date_range = match (min_timestamp, max_timestamp) {
            (Some(min), Some(max)) => {
                let range = format_date_range(min, max);
                if range.is_empty() { None } else { Some(range) }
            }
            _ => None,
        };

        CollapsedSummary {
            file_count: files.len(),
            dir_count: dirs.len(),
            total_size,
            primary_type,
            description,
            common_tags: Vec::new(), // Will be populated by aggregate_tags
            date_range,
            type_breakdown,
        }
    }

    /// Create summary when depth limit is reached
    fn create_depth_limit_summary(&self, path: &PathBuf) -> Result<CollapsedSummary, String> {
        // Quick count without full traversal
        let entries = std::fs::read_dir(path)
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        let mut file_count = 0;
        let mut dir_count = 0;
        let mut total_size: u64 = 0;

        for entry in entries.filter_map(|e| e.ok()) {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    dir_count += 1;
                } else if ft.is_file() {
                    file_count += 1;
                    if let Ok(meta) = entry.metadata() {
                        total_size += meta.len();
                    }
                }
            }
        }

        Ok(CollapsedSummary {
            file_count,
            dir_count,
            total_size,
            primary_type: None,
            description: format!("{} files, {} folders (depth limit reached)", file_count, dir_count),
            common_tags: Vec::new(),
            date_range: None,
            type_breakdown: Vec::new(),
        })
    }

    /// Generate a human-readable summary of folder contents
    fn summarize_children(&self, files: &[FileEntry], type_breakdown: &[(String, usize)]) -> String {
        if type_breakdown.is_empty() {
            return "Empty folder".to_string();
        }

        let mut parts: Vec<String> = Vec::new();

        // Take top 3 types
        for (ext, count) in type_breakdown.iter().take(3) {
            let category = Self::extension_to_category(ext);
            parts.push(format!("{} {} files", count, category));
        }

        // Add "and X more" if there are additional types
        let shown_count: usize = type_breakdown.iter().take(3).map(|(_, c)| c).sum();
        let remaining = files.len() - shown_count;
        if remaining > 0 {
            parts.push(format!("{} others", remaining));
        }

        parts.join(", ")
    }

    /// Map file extension to a category name
    fn extension_to_category(ext: &str) -> String {
        match ext.to_lowercase().as_str() {
            // Images
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "heic" | "heif" | "bmp" | "tiff" | "svg" => "image".to_string(),
            // Documents
            "pdf" | "doc" | "docx" | "odt" | "rtf" | "txt" | "md" => "document".to_string(),
            // Spreadsheets
            "xls" | "xlsx" | "csv" | "ods" | "numbers" => "spreadsheet".to_string(),
            // Presentations
            "ppt" | "pptx" | "key" | "odp" => "presentation".to_string(),
            // Code
            "js" | "ts" | "jsx" | "tsx" | "py" | "rs" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "swift" | "kt" | "rb" | "php" => "code".to_string(),
            // Archives
            "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" => "archive".to_string(),
            // Video
            "mp4" | "mov" | "avi" | "mkv" | "wmv" | "flv" | "webm" | "m4v" => "video".to_string(),
            // Audio
            "mp3" | "wav" | "aac" | "flac" | "ogg" | "m4a" | "wma" => "audio".to_string(),
            // Installers
            "dmg" | "pkg" | "app" | "exe" | "msi" | "deb" | "rpm" => "installer".to_string(),
            // Ebooks
            "epub" | "mobi" | "azw" | "azw3" => "ebook".to_string(),
            // Config
            "json" | "yaml" | "yml" | "toml" | "xml" | "ini" | "conf" => "config".to_string(),
            // Database
            "db" | "sqlite" | "sql" => "database".to_string(),
            // Fonts
            "ttf" | "otf" | "woff" | "woff2" => "font".to_string(),
            // Other
            _ => ext.to_string(),
        }
    }

    /// Get tags for a path from the vector index
    fn get_tags_for_path(&self, path: &PathBuf, vector_index: Option<&VectorIndex>) -> Vec<String> {
        if !self.config.include_tags {
            return Vec::new();
        }

        vector_index
            .and_then(|idx| idx.get_tags(path))
            .unwrap_or_default()
    }

    /// Aggregate tags from multiple files
    fn aggregate_tags(&self, files: &[FileEntry], vector_index: Option<&VectorIndex>) -> Vec<String> {
        if !self.config.include_tags || vector_index.is_none() {
            return Vec::new();
        }

        let index = vector_index.unwrap();

        // Count tag occurrences
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        for file in files {
            let path = PathBuf::from(&file.path);
            if let Some(tags) = index.get_tags(&path) {
                for tag in tags {
                    *tag_counts.entry(tag).or_insert(0) += 1;
                }
            }
        }

        // Return tags that appear in at least 25% of files
        let threshold = (files.len() as f64 * 0.25).max(1.0) as usize;
        let mut common_tags: Vec<String> = tag_counts
            .into_iter()
            .filter(|(_, count)| *count >= threshold)
            .map(|(tag, _)| tag)
            .collect();

        common_tags.sort();
        common_tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_to_category() {
        assert_eq!(TreeCompressor::extension_to_category("pdf"), "document");
        assert_eq!(TreeCompressor::extension_to_category("jpg"), "image");
        assert_eq!(TreeCompressor::extension_to_category("mp4"), "video");
        assert_eq!(TreeCompressor::extension_to_category("rs"), "code");
        assert_eq!(TreeCompressor::extension_to_category("unknown"), "unknown");
    }

    #[test]
    fn test_calculate_entropy_homogeneous() {
        let compressor = TreeCompressor::new(TreeConfig::default());

        // All same type = 0 entropy
        let files: Vec<FileEntry> = (0..10)
            .map(|i| FileEntry {
                name: format!("file{}.pdf", i),
                path: format!("/test/file{}.pdf", i),
                is_directory: false,
                is_file: true,
                is_symlink: false,
                size: 1000,
                modified_at: None,
                created_at: None,
                extension: Some("pdf".to_string()),
                mime_type: None,
                is_hidden: false,
            })
            .collect();

        let entropy = compressor.calculate_entropy(&files);
        assert!(entropy < 0.01, "Homogeneous files should have near-zero entropy");
    }

    #[test]
    fn test_calculate_entropy_diverse() {
        let compressor = TreeCompressor::new(TreeConfig::default());

        // All different types = high entropy
        let extensions = vec!["pdf", "jpg", "mp4", "zip", "txt"];
        let files: Vec<FileEntry> = extensions
            .into_iter()
            .enumerate()
            .map(|(i, ext)| FileEntry {
                name: format!("file{}.{}", i, ext),
                path: format!("/test/file{}.{}", i, ext),
                is_directory: false,
                is_file: true,
                is_symlink: false,
                size: 1000,
                modified_at: None,
                created_at: None,
                extension: Some(ext.to_string()),
                mime_type: None,
                is_hidden: false,
            })
            .collect();

        let entropy = compressor.calculate_entropy(&files);
        assert!(entropy > 0.9, "Diverse files should have high entropy");
    }
}
