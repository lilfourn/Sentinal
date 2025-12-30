//! Shadow Virtual File System for planning file operations.
//!
//! The ShadowVFS maintains a virtual representation of the file system that
//! the agent can modify without touching the real filesystem. This allows:
//! - Safe preview of planned operations
//! - Conflict detection before execution
//! - Rule-based bulk operations

use crate::ai::rules::{RuleEvaluator, SimpleVectorIndex, VirtualFile, VectorIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Maximum number of operations allowed to prevent memory exhaustion with large folders
const MAX_OPERATIONS: usize = 5000;

/// A planned file operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedOperation {
    /// Unique operation identifier
    pub op_id: String,
    /// Operation type
    #[serde(rename = "type")]
    pub op_type: OperationType,
    /// Source path (for move/rename)
    pub source: Option<String>,
    /// Destination path (for move/create_folder)
    pub destination: Option<String>,
    /// Path for single-path operations (create_folder, trash)
    pub path: Option<String>,
    /// New name (for rename)
    pub new_name: Option<String>,
    /// The rule that generated this operation (if any)
    pub rule_name: Option<String>,
}

/// Types of file operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    CreateFolder,
    Move,
    Rename,
    Trash,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::CreateFolder => write!(f, "create_folder"),
            OperationType::Move => write!(f, "move"),
            OperationType::Rename => write!(f, "rename"),
            OperationType::Trash => write!(f, "trash"),
        }
    }
}

/// An organization rule that matches files and specifies actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationRule {
    /// Human-readable rule name
    pub name: String,
    /// Rule expression in DSL syntax (e.g., "file.ext == 'pdf'")
    #[serde(rename = "if")]
    pub condition: String,
    /// Destination folder to move matching files to
    #[serde(rename = "thenMoveTo")]
    pub then_move_to: Option<String>,
    /// New name pattern for matching files
    #[serde(rename = "thenRenameTo")]
    pub then_rename_to: Option<String>,
    /// Rule priority (higher = earlier execution)
    pub priority: Option<i32>,
}

/// Shadow Virtual File System for planning operations
pub struct ShadowVFS {
    /// Root path of the target folder
    root: PathBuf,
    /// Virtual files indexed by path
    files: HashMap<String, VirtualFile>,
    /// Planned operations
    operations: Vec<PlannedOperation>,
    /// Operation ID counter
    op_counter: usize,
    /// Vector index for semantic search
    vector_index: SimpleVectorIndex,
}

impl ShadowVFS {
    /// Create a new ShadowVFS from a target folder
    pub fn new(root: &Path) -> std::io::Result<Self> {
        let mut files = HashMap::new();
        let mut file_list = Vec::new();

        // Recursively scan the folder
        Self::scan_directory(root, &mut files, &mut file_list)?;

        // Build the vector index
        let vector_index = SimpleVectorIndex::build_from_files(&file_list);

        Ok(Self {
            root: root.to_path_buf(),
            files,
            operations: Vec::new(),
            op_counter: 0,
            vector_index,
        })
    }

    fn scan_directory(
        dir: &Path,
        files: &mut HashMap<String, VirtualFile>,
        file_list: &mut Vec<VirtualFile>,
    ) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Ok(vf) = VirtualFile::from_path(&path) {
                let path_str = path.to_string_lossy().to_string();
                file_list.push(vf.clone());
                files.insert(path_str, vf);

                if path.is_dir() {
                    Self::scan_directory(&path, files, file_list)?;
                }
            }
        }
        Ok(())
    }

    /// Get the root path
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get all files (not directories)
    pub fn files(&self) -> Vec<&VirtualFile> {
        self.files
            .values()
            .filter(|f| !f.is_directory)
            .collect()
    }

    /// Get count of directories
    pub fn directory_count(&self) -> usize {
        self.files
            .values()
            .filter(|f| f.is_directory)
            .count()
    }

    /// Get all directories
    pub fn directories(&self) -> Vec<&VirtualFile> {
        self.files.values().filter(|f| f.is_directory).collect()
    }

    /// Get all entries (files and directories)
    pub fn all_entries(&self) -> Vec<&VirtualFile> {
        self.files.values().collect()
    }

    /// Get the vector index
    pub fn vector_index(&self) -> &SimpleVectorIndex {
        &self.vector_index
    }

    /// Get planned operations
    pub fn operations(&self) -> &[PlannedOperation] {
        &self.operations
    }

    /// Clear all planned operations
    pub fn clear_operations(&mut self) {
        self.operations.clear();
    }

    /// Generate a new operation ID
    fn next_op_id(&mut self) -> String {
        self.op_counter += 1;
        format!("op-{}", self.op_counter)
    }

    /// Query files using semantic search
    pub fn query_semantic(
        &self,
        query: &str,
        filter_ext: Option<&[String]>,
        min_size_bytes: Option<u64>,
        max_results: usize,
        min_similarity: f32,
    ) -> Vec<(VirtualFile, f32)> {
        let mut results: Vec<(VirtualFile, f32)> = self
            .files()
            .iter()
            .filter_map(|file| {
                // Apply extension filter
                if let Some(exts) = filter_ext {
                    if let Some(ref ext) = file.ext {
                        if !exts.iter().any(|e| e.to_lowercase() == ext.to_lowercase()) {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }

                // Apply size filter
                if let Some(min_size) = min_size_bytes {
                    if file.size < min_size {
                        return None;
                    }
                }

                // Get similarity score
                match self.vector_index.similarity(&file.path, query) {
                    Ok(score) if score >= min_similarity => Some(((*file).clone(), score)),
                    _ => None,
                }
            })
            .collect();

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Limit results
        results.truncate(max_results);

        results
    }

    /// Apply organization rules to generate operations
    pub fn apply_rules(
        &mut self,
        rules: &[OrganizationRule],
        mode: &str,
    ) -> Result<usize, String> {
        if mode == "replace" {
            self.operations.clear();
        }

        // Sort rules by priority (descending)
        let mut sorted_rules: Vec<_> = rules.iter().collect();
        sorted_rules.sort_by_key(|r| std::cmp::Reverse(r.priority.unwrap_or(0)));

        // Track which files have been processed to avoid duplicates
        let mut processed_files: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut operations_created = 0;

        // Collect folders that need to be created
        let mut folders_to_create: std::collections::HashSet<String> = std::collections::HashSet::new();

        for rule in &sorted_rules {
            // Parse the rule condition
            let expr = crate::ai::rules::RuleParser::parse(&rule.condition)
                .map_err(|e| format!("Failed to parse rule '{}': {}", rule.name, e))?;

            let evaluator = RuleEvaluator::new(&self.vector_index);

            // Find matching files
            let matching_files: Vec<VirtualFile> = self
                .files()
                .iter()
                .filter(|f| {
                    !processed_files.contains(&f.path)
                        && evaluator.evaluate(&expr, f).unwrap_or(false)
                })
                .cloned()
                .cloned()
                .collect();

            for file in matching_files {
                processed_files.insert(file.path.clone());

                // Handle move operation
                if let Some(ref dest_folder) = rule.then_move_to {
                    let dest_path = if dest_folder.starts_with('/') {
                        PathBuf::from(dest_folder)
                    } else {
                        self.root.join(dest_folder)
                    };

                    // Track folder creation
                    let dest_str = dest_path.to_string_lossy().to_string();
                    if !folders_to_create.contains(&dest_str)
                        && !self.files.contains_key(&dest_str)
                    {
                        folders_to_create.insert(dest_str.clone());
                    }

                    // Create move operation
                    let file_name = Path::new(&file.path)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let op_id = self.next_op_id();
                    self.operations.push(PlannedOperation {
                        op_id,
                        op_type: OperationType::Move,
                        source: Some(file.path.clone()),
                        destination: Some(dest_path.join(&file_name).to_string_lossy().to_string()),
                        path: None,
                        new_name: None,
                        rule_name: Some(rule.name.clone()),
                    });
                    operations_created += 1;
                }

                // Handle rename operation
                if let Some(ref new_name_pattern) = rule.then_rename_to {
                    let new_name = self.apply_rename_pattern(new_name_pattern, &file);

                    let op_id = self.next_op_id();
                    self.operations.push(PlannedOperation {
                        op_id,
                        op_type: OperationType::Rename,
                        source: None,
                        destination: None,
                        path: Some(file.path.clone()),
                        new_name: Some(new_name),
                        rule_name: Some(rule.name.clone()),
                    });
                    operations_created += 1;
                }

                // Check operation limit to prevent memory exhaustion
                if self.operations.len() > MAX_OPERATIONS {
                    return Err(format!(
                        "Operation limit exceeded ({} > {}). Try organizing smaller subfolders separately.",
                        self.operations.len(),
                        MAX_OPERATIONS
                    ));
                }
            }
        }

        // Add folder creation operations at the beginning
        let folder_ops: Vec<PlannedOperation> = folders_to_create
            .into_iter()
            .map(|path| {
                self.op_counter += 1;
                PlannedOperation {
                    op_id: format!("op-{}", self.op_counter),
                    op_type: OperationType::CreateFolder,
                    source: None,
                    destination: None,
                    path: Some(path),
                    new_name: None,
                    rule_name: None,
                }
            })
            .collect();

        // Prepend folder operations
        let mut combined_ops = folder_ops;
        combined_ops.append(&mut self.operations);
        self.operations = combined_ops;

        Ok(operations_created)
    }

    /// Apply a rename pattern to a file
    fn apply_rename_pattern(&self, pattern: &str, file: &VirtualFile) -> String {
        let mut result = pattern.to_string();

        // Replace placeholders
        result = result.replace("{name}", &file.name);
        if let Some(ref ext) = file.ext {
            result = result.replace("{ext}", ext);
        }
        if let Some(modified) = file.modified_at {
            // Convert to date string
            let date = chrono::DateTime::from_timestamp_millis(modified)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            result = result.replace("{date}", &date);
        }

        result
    }

    /// Preview operations grouped by a field
    pub fn preview_operations(
        &self,
        group_by: &str,
        include_unchanged: bool,
    ) -> OperationPreview {
        let mut groups: HashMap<String, Vec<&PlannedOperation>> = HashMap::new();

        for op in &self.operations {
            let key = match group_by {
                "operation_type" => op.op_type.to_string(),
                "destination_folder" => op
                    .destination
                    .as_ref()
                    .and_then(|d| Path::new(d).parent())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "root".to_string()),
                "source_folder" => op
                    .source
                    .as_ref()
                    .or(op.path.as_ref())
                    .and_then(|s| Path::new(s).parent())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "root".to_string()),
                "rule_name" => op
                    .rule_name
                    .clone()
                    .unwrap_or_else(|| "manual".to_string()),
                _ => "unknown".to_string(),
            };

            groups.entry(key).or_default().push(op);
        }

        let unchanged_count = if include_unchanged {
            self.files().len() - self.operations.len()
        } else {
            0
        };

        OperationPreview {
            groups: groups
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().cloned().collect()))
                .collect(),
            total_operations: self.operations.len(),
            unchanged_files: unchanged_count,
        }
    }

    /// Add a single operation manually
    pub fn add_operation(&mut self, op_type: OperationType, params: OperationParams) {
        let op_id = self.next_op_id();
        self.operations.push(PlannedOperation {
            op_id,
            op_type,
            source: params.source,
            destination: params.destination,
            path: params.path,
            new_name: params.new_name,
            rule_name: params.rule_name,
        });
    }

    /// Generate a compressed tree representation for context
    ///
    /// Uses the TreeCompressor for intelligent folder collapsing
    /// based on Shannon entropy analysis.
    pub fn generate_compressed_tree(&self) -> String {
        use crate::tree::{TreeCompressor, TreeConfig, to_xml, to_compact_xml};

        // Use aggressive compression for large folders to fit context limits
        let file_count = self.files.len();
        let config = if file_count > 500 {
            eprintln!("[ShadowVFS] Large folder detected ({} files), using aggressive compression", file_count);
            TreeConfig {
                collapse_threshold: 15,   // Collapse folders with 15+ files
                max_depth: 4,             // Limit depth to reduce output
                include_tags: false,      // Skip tags to reduce size
                entropy_threshold: 0.7,   // More aggressive collapsing
            }
        } else if file_count > 200 {
            TreeConfig {
                collapse_threshold: 30,
                max_depth: 6,
                include_tags: true,
                entropy_threshold: 0.6,
            }
        } else {
            TreeConfig::default()
        };

        let compressor = TreeCompressor::new(config);

        match compressor.compress(&self.root, None) {
            Ok(compressed) => {
                // Use compact XML for large trees, full XML for smaller ones
                if compressed.node_count() > 50 {
                    to_compact_xml(&compressed)
                } else {
                    to_xml(&compressed)
                }
            }
            Err(e) => {
                eprintln!("[ShadowVFS] TreeCompressor failed: {}, using fallback", e);
                self.generate_fallback_tree()
            }
        }
    }

    /// Fallback tree generation if TreeCompressor fails
    fn generate_fallback_tree(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("<folder path=\"{}\">", self.root.display()));

        // Group files by directory
        let mut dirs: HashMap<String, Vec<&VirtualFile>> = HashMap::new();
        for file in self.files() {
            let parent = Path::new(&file.path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            dirs.entry(parent).or_default().push(file);
        }

        // Sort directories for consistent output
        let mut sorted_dirs: Vec<_> = dirs.keys().collect();
        sorted_dirs.sort();

        for dir_path in sorted_dirs {
            let files = &dirs[dir_path];
            let rel_path = Path::new(dir_path)
                .strip_prefix(&self.root)
                .unwrap_or(Path::new("."));

            if !rel_path.as_os_str().is_empty() && rel_path != Path::new(".") {
                lines.push(format!("  <dir path=\"{}\">", rel_path.display()));
            }

            for file in files {
                let size_str = format_size(file.size);
                lines.push(format!(
                    "    <file name=\"{}\" ext=\"{}\" size=\"{}\" />",
                    file.name,
                    file.ext.as_deref().unwrap_or(""),
                    size_str
                ));
            }

            if !rel_path.as_os_str().is_empty() && rel_path != Path::new(".") {
                lines.push("  </dir>".to_string());
            }
        }

        lines.push("</folder>".to_string());
        lines.join("\n")
    }
}

/// Parameters for manual operation creation
pub struct OperationParams {
    pub source: Option<String>,
    pub destination: Option<String>,
    pub path: Option<String>,
    pub new_name: Option<String>,
    pub rule_name: Option<String>,
}

/// Preview of planned operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationPreview {
    /// Operations grouped by the specified field
    pub groups: HashMap<String, Vec<PlannedOperation>>,
    /// Total number of operations
    pub total_operations: usize,
    /// Number of files that won't be changed
    pub unchanged_files: usize,
}

/// Format file size for display
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_vfs() -> (ShadowVFS, tempfile::TempDir) {
        let temp = tempdir().unwrap();

        // Create test files
        fs::write(temp.path().join("doc1.pdf"), "test content").unwrap();
        fs::write(temp.path().join("doc2.pdf"), "test content").unwrap();
        fs::write(temp.path().join("image1.jpg"), "fake image").unwrap();
        fs::write(temp.path().join("image2.png"), "fake image").unwrap();
        fs::write(temp.path().join("archive.zip"), "fake archive").unwrap();

        let vfs = ShadowVFS::new(temp.path()).unwrap();
        (vfs, temp)
    }

    #[test]
    fn test_vfs_creation() {
        let (vfs, _temp) = create_test_vfs();
        assert_eq!(vfs.files().len(), 5);
    }

    #[test]
    fn test_semantic_query() {
        let (vfs, _temp) = create_test_vfs();
        let results = vfs.query_semantic("doc", None, None, 10, 0.0);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_apply_rules() {
        let (mut vfs, _temp) = create_test_vfs();

        let rules = vec![OrganizationRule {
            name: "Move PDFs".to_string(),
            condition: "file.ext == 'pdf'".to_string(),
            then_move_to: Some("Documents".to_string()),
            then_rename_to: None,
            priority: Some(1),
        }];

        let count = vfs.apply_rules(&rules, "replace").unwrap();
        assert!(count >= 2); // At least 2 PDFs
    }

    #[test]
    fn test_preview_operations() {
        let (mut vfs, _temp) = create_test_vfs();

        vfs.add_operation(
            OperationType::Move,
            OperationParams {
                source: Some("/test/file.pdf".to_string()),
                destination: Some("/test/Documents/file.pdf".to_string()),
                path: None,
                new_name: None,
                rule_name: Some("test rule".to_string()),
            },
        );

        let preview = vfs.preview_operations("operation_type", false);
        assert_eq!(preview.total_operations, 1);
    }
}
