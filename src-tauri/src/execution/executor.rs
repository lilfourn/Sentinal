//! Execution Engine
//!
//! Executes WAL operations using the DAG-based dependency graph.
//! Operations at the same level are executed in parallel using tokio tasks.

use crate::security::PathValidator;
use crate::wal::entry::{WALEntry, WALJournal, WALOperationType, WALStatus};
use crate::wal::journal::WALManager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::dag::ExecutionDAG;

/// Result of executing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResult {
    /// Number of operations completed successfully
    pub completed_count: usize,
    /// Number of operations that failed
    pub failed_count: usize,
    /// Error messages from failed operations
    pub errors: Vec<String>,
    /// Whether all operations completed successfully
    pub success: bool,
}

impl ExecutionResult {
    /// Create a successful result
    pub fn success(completed: usize) -> Self {
        Self {
            completed_count: completed,
            failed_count: 0,
            errors: Vec::new(),
            success: true,
        }
    }

    /// Create a partial success result
    pub fn partial(completed: usize, failed: usize, errors: Vec<String>) -> Self {
        Self {
            completed_count: completed,
            failed_count: failed,
            errors,
            success: failed == 0,
        }
    }
}

/// Execution engine for WAL operations
pub struct ExecutionEngine {
    /// WAL manager for persistence
    wal_manager: WALManager,
}

impl ExecutionEngine {
    /// Create a new execution engine
    pub fn new() -> Self {
        Self {
            wal_manager: WALManager::new(),
        }
    }

    /// Create an execution engine with a custom WAL manager (for testing)
    #[allow(dead_code)]
    pub fn with_manager(wal_manager: WALManager) -> Self {
        Self { wal_manager }
    }

    /// Execute all pending operations in a journal using the DAG
    ///
    /// This method:
    /// 1. Builds a DAG from pending entries
    /// 2. Executes each level in parallel
    /// 3. Updates WAL entries as operations complete
    /// 4. Stops on first failure within a level
    pub async fn execute_journal(&self, job_id: &str) -> Result<ExecutionResult, String> {
        let journal = self
            .wal_manager
            .load_journal(job_id)?
            .ok_or_else(|| format!("Journal not found: {}", job_id))?;

        // Get pending entries
        let pending_entries: Vec<WALEntry> = journal
            .entries
            .iter()
            .filter(|e| matches!(e.status, WALStatus::Pending | WALStatus::InProgress))
            .cloned()
            .collect();

        if pending_entries.is_empty() {
            return Ok(ExecutionResult::success(0));
        }

        // Build DAG from pending entries
        let dag = ExecutionDAG::from_entries(pending_entries)?;

        eprintln!(
            "[Executor] Built DAG with {} entries across {} levels",
            dag.len(),
            dag.level_count()
        );

        self.execute_dag(&dag, job_id).await
    }

    /// Execute operations organized by the DAG
    ///
    /// Each level is executed in parallel, but levels are executed sequentially.
    pub async fn execute_dag(&self, dag: &ExecutionDAG, job_id: &str) -> Result<ExecutionResult, String> {
        let levels = dag.get_levels_owned();
        let mut total_completed = 0;
        let mut total_failed = 0;
        let mut all_errors: Vec<String> = Vec::new();

        for (level_idx, level) in levels.into_iter().enumerate() {
            eprintln!(
                "[Executor] Executing level {} with {} operations",
                level_idx,
                level.len()
            );

            let (completed, failed, errors) = self.execute_level(level, job_id).await?;

            total_completed += completed;
            total_failed += failed;
            all_errors.extend(errors);

            // If any operation in this level failed, stop execution
            // (dependents in later levels may not be safe to execute)
            if failed > 0 {
                eprintln!(
                    "[Executor] Level {} had {} failures, stopping execution",
                    level_idx, failed
                );
                break;
            }
        }

        Ok(ExecutionResult::partial(total_completed, total_failed, all_errors))
    }

    /// Execute a single level of operations in parallel
    async fn execute_level(
        &self,
        entries: Vec<WALEntry>,
        job_id: &str,
    ) -> Result<(usize, usize, Vec<String>), String> {
        if entries.is_empty() {
            return Ok((0, 0, Vec::new()));
        }

        // Shared state for collecting results
        let completed = Arc::new(Mutex::new(0usize));
        let failed = Arc::new(Mutex::new(0usize));
        let errors = Arc::new(Mutex::new(Vec::<String>::new()));

        // Load journal for updating (we'll save after each operation)
        let job_id_owned = job_id.to_string();

        // Spawn tasks for each operation
        let mut handles = Vec::new();

        for entry in entries {
            let entry_id = entry.id;
            let operation = entry.operation.clone();
            let completed = Arc::clone(&completed);
            let failed = Arc::clone(&failed);
            let errors = Arc::clone(&errors);
            let job_id = job_id_owned.clone();

            let handle = tokio::spawn(async move {
                let manager = WALManager::new();

                // Mark as in progress
                if let Err(e) = manager.mark_entry_in_progress(&job_id, entry_id) {
                    eprintln!("[Executor] Failed to mark in progress: {}", e);
                }

                eprintln!(
                    "[Executor] Executing operation: {}",
                    operation.description()
                );

                // Execute the operation
                match execute_operation(&operation).await {
                    Ok(()) => {
                        if let Err(e) = manager.mark_entry_complete(&job_id, entry_id) {
                            eprintln!("[Executor] Failed to mark complete: {}", e);
                        }
                        let mut c = completed.lock().await;
                        *c += 1;
                        eprintln!("[Executor] Operation completed successfully");
                    }
                    Err(err) => {
                        if let Err(e) = manager.mark_entry_failed(&job_id, entry_id, err.clone()) {
                            eprintln!("[Executor] Failed to mark failed: {}", e);
                        }
                        let mut f = failed.lock().await;
                        *f += 1;
                        let mut e = errors.lock().await;
                        e.push(err.clone());
                        eprintln!("[Executor] Operation failed: {}", err);
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all operations in this level to complete
        for handle in handles {
            if let Err(join_err) = handle.await {
                eprintln!("[Executor] Task panicked: {}", join_err);
                let mut f = failed.lock().await;
                *f += 1;
                let mut errs = errors.lock().await;
                errs.push(format!("Task panicked: {}", join_err));
            }
        }

        let completed = *completed.lock().await;
        let failed = *failed.lock().await;
        let errors = errors.lock().await.clone();

        Ok((completed, failed, errors))
    }

    /// Execute a single entry (for recovery or single-operation execution)
    pub async fn execute_entry(
        &self,
        entry: &WALEntry,
        job_id: &str,
    ) -> Result<(), String> {
        self.wal_manager
            .mark_entry_in_progress(job_id, entry.id)
            .map_err(|e| e.message)?;

        match execute_operation(&entry.operation).await {
            Ok(()) => {
                self.wal_manager
                    .mark_entry_complete(job_id, entry.id)
                    .map_err(|e| e.message)?;
                Ok(())
            }
            Err(err) => {
                self.wal_manager
                    .mark_entry_failed(job_id, entry.id, err.clone())
                    .map_err(|e| e.message)?;
                Err(err)
            }
        }
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute a single WAL operation
///
/// This function performs the actual filesystem operation.
/// It's async to work with tokio's spawn but currently does blocking I/O.
/// In production, you might want to use tokio::fs for true async I/O.
async fn execute_operation(operation: &WALOperationType) -> Result<(), String> {
    // Use blocking task for filesystem operations
    let operation = operation.clone();
    tokio::task::spawn_blocking(move || execute_operation_sync(&operation))
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}

/// Synchronous operation execution
fn execute_operation_sync(operation: &WALOperationType) -> Result<(), String> {
    match operation {
        WALOperationType::CreateFolder { path } => {
            if path.exists() {
                return Ok(());
            }
            fs::create_dir_all(path)
                .map_err(|e| format!("Failed to create folder {}: {}", path.display(), e))
        }

        WALOperationType::Move {
            source,
            destination,
        } => {
            if !source.exists() {
                if destination.exists() {
                    return Ok(());
                }
                return Err(format!("Source not found: {}", source.display()));
            }

            if destination.exists() {
                return Err(format!("Destination already exists: {}", destination.display()));
            }

            if PathValidator::is_protected_path(source) {
                return Err(format!("Cannot move protected path: {}", source.display()));
            }

            // Ensure destination parent exists
            if let Some(parent) = destination.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create destination directory: {}", e))?;
                }
            }

            // Try rename first (same filesystem), fall back to copy+delete
            if fs::rename(source, destination).is_err() {
                if source.is_dir() {
                    copy_dir_all(source, destination)?;
                    fs::remove_dir_all(source)
                        .map_err(|e| format!("Failed to remove source: {}", e))?;
                } else {
                    fs::copy(source, destination)
                        .map_err(|e| format!("Failed to copy: {}", e))?;
                    fs::remove_file(source)
                        .map_err(|e| format!("Failed to remove source: {}", e))?;
                }
            }

            Ok(())
        }

        WALOperationType::Rename { path, new_name } => {
            if !path.exists() {
                return Err(format!("Path not found: {}", path.display()));
            }

            let parent = path
                .parent()
                .ok_or_else(|| format!("Cannot determine parent of {}", path.display()))?;
            let new_path = parent.join(new_name);

            if new_path.exists() {
                return Err(format!("Target already exists: {}", new_path.display()));
            }

            if PathValidator::is_protected_path(path) {
                return Err(format!("Cannot rename protected path: {}", path.display()));
            }

            fs::rename(path, &new_path)
                .map_err(|e| format!("Failed to rename {} to {}: {}", path.display(), new_name, e))
        }

        WALOperationType::Quarantine {
            path,
            quarantine_path,
        } => {
            execute_operation_sync(&WALOperationType::Move {
                source: path.clone(),
                destination: quarantine_path.clone(),
            })
        }

        WALOperationType::Copy {
            source,
            destination,
        } => {
            if !source.exists() {
                return Err(format!("Source not found: {}", source.display()));
            }

            if destination.exists() {
                return Err(format!("Destination already exists: {}", destination.display()));
            }

            // Ensure destination parent exists
            if let Some(parent) = destination.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create destination directory: {}", e))?;
                }
            }

            if source.is_dir() {
                copy_dir_all(source, destination)
            } else {
                fs::copy(source, destination)
                    .map_err(|e| format!("Failed to copy: {}", e))
                    .map(|_| ())
            }
        }

        WALOperationType::DeleteFolder { path } => {
            if !path.exists() {
                return Ok(());
            }

            if PathValidator::is_protected_path(path) {
                return Err(format!("Cannot delete protected path: {}", path.display()));
            }

            if !path.is_dir() {
                return fs::remove_file(path)
                    .map_err(|e| format!("Failed to delete file {}: {}", path.display(), e));
            }

            let is_empty = fs::read_dir(path)
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(false);

            if is_empty {
                fs::remove_dir(path)
                    .map_err(|e| format!("Failed to delete folder {}: {}", path.display(), e))
            } else {
                fs::remove_dir_all(path)
                    .map_err(|e| format!("Failed to delete folder {}: {}", path.display(), e))
            }
        }
    }
}

/// Helper function to copy a directory recursively
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Failed to create directory: {}", e))?;

    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let ty = entry
            .file_type()
            .map_err(|e| format!("Failed to get file type: {}", e))?;

        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| format!("Failed to copy file: {}", e))?;
        }
    }

    Ok(())
}

/// Builder for creating and executing operations
pub struct ExecutionBuilder {
    journal: WALJournal,
    wal_manager: WALManager,
}

impl ExecutionBuilder {
    /// Create a new execution builder for a target folder
    pub fn new(job_id: String, target_folder: std::path::PathBuf) -> Self {
        Self {
            journal: WALJournal::new(job_id, target_folder),
            wal_manager: WALManager::new(),
        }
    }

    /// Add an operation to the execution plan
    pub fn add_operation(&mut self, operation: WALOperationType) -> uuid::Uuid {
        self.journal.add_operation(operation)
    }

    /// Add an operation with dependencies
    pub fn add_operation_with_deps(
        &mut self,
        operation: WALOperationType,
        depends_on: Vec<uuid::Uuid>,
    ) -> uuid::Uuid {
        self.journal.add_operation_with_deps(operation, depends_on)
    }

    /// Save the journal and prepare for execution
    pub fn save(&self) -> Result<(), String> {
        self.wal_manager
            .save_journal(&self.journal)
            .map_err(|e| e.message)
    }

    /// Save and execute all operations
    pub async fn execute(self) -> Result<ExecutionResult, String> {
        self.save()?;
        let engine = ExecutionEngine::new();
        engine.execute_journal(&self.journal.job_id).await
    }

    /// Get the job ID
    pub fn job_id(&self) -> &str {
        &self.journal.job_id
    }

    /// Get the current journal
    pub fn journal(&self) -> &WALJournal {
        &self.journal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wal::entry::WALEntry;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_execute_create_folder() {
        let dir = tempdir().unwrap();
        let new_folder = dir.path().join("test_folder");

        let op = WALOperationType::CreateFolder {
            path: new_folder.clone(),
        };

        execute_operation(&op).await.unwrap();
        assert!(new_folder.exists());
    }

    #[tokio::test]
    async fn test_execute_move() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        fs::write(&source, "test content").unwrap();

        let op = WALOperationType::Move {
            source: source.clone(),
            destination: dest.clone(),
        };

        execute_operation(&op).await.unwrap();
        assert!(!source.exists());
        assert!(dest.exists());
    }

    #[tokio::test]
    async fn test_execute_level_parallel() {
        let dir = tempdir().unwrap();

        // Create multiple folders in parallel
        let entries: Vec<WALEntry> = (0..5)
            .map(|i| {
                WALEntry::new(
                    WALOperationType::CreateFolder {
                        path: dir.path().join(format!("folder_{}", i)),
                    },
                    i as u32,
                )
            })
            .collect();

        // Create a temporary journal
        let job_id = "test-parallel";
        let mut journal = WALJournal::new(job_id.to_string(), dir.path().to_path_buf());
        for entry in entries {
            journal.add_entry(entry);
        }

        let manager = WALManager::new();
        manager.save_journal(&journal).unwrap();

        let engine = ExecutionEngine::new();
        let result = engine.execute_journal(job_id).await.unwrap();

        assert_eq!(result.completed_count, 5);
        assert_eq!(result.failed_count, 0);
        assert!(result.success);

        // Verify all folders exist
        for i in 0..5 {
            assert!(dir.path().join(format!("folder_{}", i)).exists());
        }

        // Cleanup
        manager.discard_journal(job_id).unwrap();
    }
}
