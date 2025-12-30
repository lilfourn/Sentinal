//! Shadow VFS Graph
//!
//! The ShadowVFS is an in-memory representation of a filesystem tree.
//! It supports staging operations (move, create, delete) that can be
//! validated before being applied to the real filesystem.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use thiserror::Error;

use super::node::{FileNode, VFSNodeType};

/// Errors that can occur during VFS operations
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "details")]
pub enum VFSError {
    /// The requested path does not exist in the VFS
    #[error("Path not found: {0}")]
    PathNotFound(String),

    /// A path collision would occur (e.g., move to existing path)
    #[error("Path collision: {target} already exists (source: {source_path})")]
    PathCollision { source_path: String, target: String },

    /// A cycle would be created (e.g., moving parent into child)
    #[error("Cycle detected: cannot move {source_path} into {target_path}")]
    CycleDetected { source_path: String, target_path: String },

    /// The operation is invalid in the current state
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// Parent directory does not exist
    #[error("Parent directory not found: {0}")]
    ParentNotFound(String),

    /// Cannot perform operation on root
    #[error("Cannot modify root: {0}")]
    CannotModifyRoot(String),
}

/// Shadow Virtual File System
///
/// Maintains an in-memory graph of FileNodes that mirrors the real filesystem.
/// Supports staging operations that can be validated and applied atomically.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowVFS {
    /// All nodes indexed by their path
    nodes: HashMap<PathBuf, FileNode>,

    /// Root path of this VFS
    root: PathBuf,

    /// Paths of nodes staged for creation
    staged_creates: HashSet<PathBuf>,

    /// Paths of nodes staged for deletion
    staged_deletes: HashSet<PathBuf>,

    /// Staged moves: source -> destination
    staged_moves: HashMap<PathBuf, PathBuf>,

    /// When this VFS was last scanned from the real filesystem
    last_scan: Option<DateTime<Utc>>,

    /// Total size of all files in bytes
    total_size_bytes: u64,
}

impl ShadowVFS {
    /// Create a new empty VFS rooted at the given path
    pub fn new(root: PathBuf) -> Self {
        let mut nodes = HashMap::new();

        // Create root node
        let root_node = FileNode::directory(root.clone());
        nodes.insert(root.clone(), root_node);

        Self {
            nodes,
            root,
            staged_creates: HashSet::new(),
            staged_deletes: HashSet::new(),
            staged_moves: HashMap::new(),
            last_scan: None,
            total_size_bytes: 0,
        }
    }

    /// Get the root path
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Get when this VFS was last scanned
    pub fn last_scan(&self) -> Option<DateTime<Utc>> {
        self.last_scan
    }

    /// Set the last scan time
    pub fn set_last_scan(&mut self, time: DateTime<Utc>) {
        self.last_scan = Some(time);
    }

    /// Get total number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get total size in bytes
    pub fn total_size(&self) -> u64 {
        self.total_size_bytes
    }

    /// Get a node by path
    pub fn get(&self, path: &PathBuf) -> Option<&FileNode> {
        // If staged for delete, don't return it
        if self.staged_deletes.contains(path) {
            return None;
        }
        self.nodes.get(path)
    }

    /// Get a mutable reference to a node by path
    pub fn get_mut(&mut self, path: &PathBuf) -> Option<&mut FileNode> {
        if self.staged_deletes.contains(path) {
            return None;
        }
        self.nodes.get_mut(path)
    }

    /// Insert a node into the VFS
    pub fn insert(&mut self, node: FileNode) {
        if node.is_file() {
            self.total_size_bytes += node.size;
        }
        self.nodes.insert(node.path.clone(), node);
    }

    /// List all children of a directory
    pub fn list_dir(&self, path: &PathBuf) -> Result<Vec<&FileNode>, VFSError> {
        let node = self
            .get(path)
            .ok_or_else(|| VFSError::PathNotFound(path.display().to_string()))?;

        if !node.is_directory() {
            return Err(VFSError::InvalidOperation(format!(
                "{} is not a directory",
                path.display()
            )));
        }

        let children: Vec<&FileNode> = node
            .children
            .iter()
            .filter_map(|child_path| {
                // Skip deleted nodes
                if self.staged_deletes.contains(child_path) {
                    None
                } else {
                    self.nodes.get(child_path)
                }
            })
            .collect();

        Ok(children)
    }

    /// Stage a move operation
    ///
    /// Validates that:
    /// - Source exists
    /// - Destination doesn't exist (unless staged for delete)
    /// - No cycle would be created
    pub fn stage_move(&mut self, src: PathBuf, dest: PathBuf) -> Result<(), VFSError> {
        // Validate source exists
        if !self.nodes.contains_key(&src) || self.staged_deletes.contains(&src) {
            return Err(VFSError::PathNotFound(src.display().to_string()));
        }

        // Check for path collision (unless destination is staged for delete)
        if self.nodes.contains_key(&dest) && !self.staged_deletes.contains(&dest) {
            return Err(VFSError::PathCollision {
                source_path: src.display().to_string(),
                target: dest.display().to_string(),
            });
        }

        // Detect cycles: cannot move a parent into its child
        if dest.starts_with(&src) && dest != src {
            return Err(VFSError::CycleDetected {
                source_path: src.display().to_string(),
                target_path: dest.display().to_string(),
            });
        }

        // Validate destination parent exists
        if let Some(dest_parent) = dest.parent() {
            let parent_path = dest_parent.to_path_buf();
            if parent_path != self.root
                && (!self.nodes.contains_key(&parent_path)
                    || self.staged_deletes.contains(&parent_path))
            {
                // Check if parent is staged for creation
                if !self.staged_creates.contains(&parent_path) {
                    return Err(VFSError::ParentNotFound(parent_path.display().to_string()));
                }
            }
        }

        // Cannot move root
        if src == self.root {
            return Err(VFSError::CannotModifyRoot(
                "Cannot move root directory".to_string(),
            ));
        }

        self.staged_moves.insert(src, dest);
        Ok(())
    }

    /// Stage a folder creation
    ///
    /// Validates that:
    /// - Path doesn't already exist
    /// - Parent directory exists
    pub fn stage_create_folder(&mut self, path: PathBuf) -> Result<(), VFSError> {
        // Check for collision
        if self.nodes.contains_key(&path) && !self.staged_deletes.contains(&path) {
            return Err(VFSError::PathCollision {
                source_path: path.display().to_string(),
                target: path.display().to_string(),
            });
        }

        // Validate parent exists
        if let Some(parent) = path.parent() {
            let parent_path = parent.to_path_buf();
            if !self.nodes.contains_key(&parent_path) || self.staged_deletes.contains(&parent_path)
            {
                // Check if parent is also staged for creation
                if !self.staged_creates.contains(&parent_path) {
                    return Err(VFSError::ParentNotFound(parent_path.display().to_string()));
                }
            }
        }

        self.staged_creates.insert(path);
        Ok(())
    }

    /// Stage a deletion
    ///
    /// Validates that:
    /// - Path exists
    /// - Path is not root
    pub fn stage_delete(&mut self, path: PathBuf) -> Result<(), VFSError> {
        // Validate path exists
        if !self.nodes.contains_key(&path) {
            return Err(VFSError::PathNotFound(path.display().to_string()));
        }

        // Cannot delete root
        if path == self.root {
            return Err(VFSError::CannotModifyRoot(
                "Cannot delete root directory".to_string(),
            ));
        }

        self.staged_deletes.insert(path);
        Ok(())
    }

    /// Validate all staged operations
    ///
    /// Returns Ok if all operations are valid, or a list of errors
    pub fn validate_staged(&self) -> Result<(), Vec<VFSError>> {
        let mut errors = Vec::new();

        // Check moves don't conflict with each other
        let mut destinations: HashSet<PathBuf> = HashSet::new();
        for (src, dest) in &self.staged_moves {
            // Check source still exists
            if !self.nodes.contains_key(src) {
                errors.push(VFSError::PathNotFound(src.display().to_string()));
                continue;
            }

            // Check for duplicate destinations
            if destinations.contains(dest) {
                errors.push(VFSError::PathCollision {
                    source_path: src.display().to_string(),
                    target: dest.display().to_string(),
                });
            }
            destinations.insert(dest.clone());
        }

        // Check creates don't conflict
        for path in &self.staged_creates {
            if destinations.contains(path) {
                errors.push(VFSError::PathCollision {
                    source_path: path.display().to_string(),
                    target: path.display().to_string(),
                });
            }
        }

        // Check deletes are valid
        for path in &self.staged_deletes {
            if !self.nodes.contains_key(path) {
                errors.push(VFSError::PathNotFound(path.display().to_string()));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Clear all staged operations without applying them
    pub fn clear_staged(&mut self) {
        self.staged_creates.clear();
        self.staged_deletes.clear();
        self.staged_moves.clear();
    }

    /// Get all staged creates
    pub fn staged_creates(&self) -> &HashSet<PathBuf> {
        &self.staged_creates
    }

    /// Get all staged deletes
    pub fn staged_deletes(&self) -> &HashSet<PathBuf> {
        &self.staged_deletes
    }

    /// Get all staged moves
    pub fn staged_moves(&self) -> &HashMap<PathBuf, PathBuf> {
        &self.staged_moves
    }

    /// Check if there are any staged operations
    pub fn has_staged_operations(&self) -> bool {
        !self.staged_creates.is_empty()
            || !self.staged_deletes.is_empty()
            || !self.staged_moves.is_empty()
    }

    /// Search for nodes by content preview
    pub fn search_content(&self, query: &str) -> Vec<&FileNode> {
        self.nodes
            .values()
            .filter(|node| {
                // Skip deleted nodes
                if self.staged_deletes.contains(&node.path) {
                    return false;
                }
                // Check content and name
                node.content_contains(query) || node.name_contains(query)
            })
            .collect()
    }

    /// Search for nodes by name only
    pub fn search_name(&self, query: &str) -> Vec<&FileNode> {
        self.nodes
            .values()
            .filter(|node| {
                !self.staged_deletes.contains(&node.path) && node.name_contains(query)
            })
            .collect()
    }

    /// Get all nodes of a specific type
    pub fn get_by_type(&self, node_type: VFSNodeType) -> Vec<&FileNode> {
        self.nodes
            .values()
            .filter(|node| {
                !self.staged_deletes.contains(&node.path) && node.node_type == node_type
            })
            .collect()
    }

    /// Get all file nodes
    pub fn files(&self) -> Vec<&FileNode> {
        self.get_by_type(VFSNodeType::File)
    }

    /// Get all directory nodes
    pub fn directories(&self) -> Vec<&FileNode> {
        self.get_by_type(VFSNodeType::Directory)
    }

    /// Get statistics about the VFS
    pub fn stats(&self) -> VFSStats {
        let files = self.files();
        let directories = self.directories();

        VFSStats {
            total_nodes: self.nodes.len(),
            total_files: files.len(),
            total_directories: directories.len(),
            total_size_bytes: self.total_size_bytes,
            staged_creates: self.staged_creates.len(),
            staged_deletes: self.staged_deletes.len(),
            staged_moves: self.staged_moves.len(),
            last_scan: self.last_scan,
        }
    }

    /// Remove a node and update parent references
    pub fn remove(&mut self, path: &PathBuf) -> Option<FileNode> {
        if let Some(node) = self.nodes.remove(path) {
            // Update size tracking
            if node.is_file() {
                self.total_size_bytes = self.total_size_bytes.saturating_sub(node.size);
            }

            // Remove from parent's children
            if let Some(parent_path) = &node.parent {
                if let Some(parent) = self.nodes.get_mut(parent_path) {
                    parent.remove_child(path);
                }
            }

            Some(node)
        } else {
            None
        }
    }

    /// Check if a path exists in the VFS (excluding staged deletes)
    pub fn exists(&self, path: &PathBuf) -> bool {
        self.nodes.contains_key(path) && !self.staged_deletes.contains(path)
    }

    /// Get all nodes as an iterator
    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &FileNode)> {
        self.nodes
            .iter()
            .filter(|(path, _)| !self.staged_deletes.contains(*path))
    }
}

/// Statistics about the VFS state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VFSStats {
    pub total_nodes: usize,
    pub total_files: usize,
    pub total_directories: usize,
    pub total_size_bytes: u64,
    pub staged_creates: usize,
    pub staged_deletes: usize,
    pub staged_moves: usize,
    pub last_scan: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_vfs() -> ShadowVFS {
        let mut vfs = ShadowVFS::new(PathBuf::from("/root"));

        // Add some files
        let mut dir = FileNode::directory(PathBuf::from("/root/docs"));
        dir.parent = Some(PathBuf::from("/root"));

        let mut file = FileNode::file(PathBuf::from("/root/docs/readme.txt"));
        file.parent = Some(PathBuf::from("/root/docs"));
        file.size = 1024;

        dir.add_child(PathBuf::from("/root/docs/readme.txt"));
        vfs.insert(dir);
        vfs.insert(file);

        // Update root's children
        if let Some(root) = vfs.get_mut(&PathBuf::from("/root")) {
            root.add_child(PathBuf::from("/root/docs"));
        }

        vfs
    }

    #[test]
    fn test_vfs_creation() {
        let vfs = ShadowVFS::new(PathBuf::from("/test"));
        assert_eq!(vfs.node_count(), 1); // Just root
        assert!(vfs.exists(&PathBuf::from("/test")));
    }

    #[test]
    fn test_list_dir() {
        let vfs = create_test_vfs();
        let children = vfs.list_dir(&PathBuf::from("/root")).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "docs");
    }

    #[test]
    fn test_stage_move_valid() {
        let mut vfs = create_test_vfs();

        // Create destination directory first
        let mut archive = FileNode::directory(PathBuf::from("/root/archive"));
        archive.parent = Some(PathBuf::from("/root"));
        vfs.insert(archive);

        let result = vfs.stage_move(
            PathBuf::from("/root/docs/readme.txt"),
            PathBuf::from("/root/archive/readme.txt"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_stage_move_cycle_detected() {
        let mut vfs = create_test_vfs();
        let result = vfs.stage_move(
            PathBuf::from("/root/docs"),
            PathBuf::from("/root/docs/readme.txt/subdoc"),
        );
        assert!(matches!(result, Err(VFSError::CycleDetected { .. })));
    }

    #[test]
    fn test_stage_move_collision() {
        let mut vfs = create_test_vfs();
        let result = vfs.stage_move(
            PathBuf::from("/root/docs/readme.txt"),
            PathBuf::from("/root/docs"), // Already exists
        );
        assert!(matches!(result, Err(VFSError::PathCollision { .. })));
    }

    #[test]
    fn test_stage_create_folder() {
        let mut vfs = create_test_vfs();
        let result = vfs.stage_create_folder(PathBuf::from("/root/new_folder"));
        assert!(result.is_ok());
        assert!(vfs.staged_creates.contains(&PathBuf::from("/root/new_folder")));
    }

    #[test]
    fn test_stage_delete() {
        let mut vfs = create_test_vfs();
        let result = vfs.stage_delete(PathBuf::from("/root/docs/readme.txt"));
        assert!(result.is_ok());

        // Node should not be returned anymore
        assert!(vfs.get(&PathBuf::from("/root/docs/readme.txt")).is_none());
    }

    #[test]
    fn test_cannot_delete_root() {
        let mut vfs = create_test_vfs();
        let result = vfs.stage_delete(PathBuf::from("/root"));
        assert!(matches!(result, Err(VFSError::CannotModifyRoot(_))));
    }

    #[test]
    fn test_content_search() {
        let mut vfs = create_test_vfs();
        if let Some(node) = vfs.get_mut(&PathBuf::from("/root/docs/readme.txt")) {
            node.content_preview = Some("Hello World".to_string());
        }

        let results = vfs.search_content("hello");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_clear_staged() {
        let mut vfs = create_test_vfs();
        vfs.stage_create_folder(PathBuf::from("/root/new")).unwrap();
        vfs.stage_delete(PathBuf::from("/root/docs/readme.txt"))
            .unwrap();

        assert!(vfs.has_staged_operations());
        vfs.clear_staged();
        assert!(!vfs.has_staged_operations());
    }
}
