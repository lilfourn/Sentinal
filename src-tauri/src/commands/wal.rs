//! Tauri Commands for WAL Operations
//!
//! These commands expose WAL functionality to the frontend, including
//! recovery checking, resuming interrupted jobs, and rollback operations.

use crate::wal::recovery::{
    check_for_recovery, discard_journal, get_journal_details, rollback_journal, resume_journal,
    RecoveryInfo, RecoveryResult,
};
use crate::wal::{WALJournal, WALManager, WALOperationType};
use crate::execution::{ExecutionBuilder, ExecutionEngine, ExecutionResult};
use std::path::PathBuf;

/// Check if there are any interrupted jobs that need recovery
///
/// This should be called on application startup to detect jobs that
/// were interrupted due to crash or unexpected shutdown.
#[tauri::command]
pub async fn wal_check_recovery() -> Result<Option<RecoveryInfo>, String> {
    check_for_recovery()
}

/// Resume an interrupted job by executing remaining pending operations
///
/// This will execute all pending operations in the journal sequentially,
/// updating entry statuses as operations complete or fail.
#[tauri::command]
pub async fn wal_resume_job(job_id: String) -> Result<RecoveryResult, String> {
    resume_journal(&job_id)
}

/// Rollback an interrupted job by undoing completed operations
///
/// This will execute undo operations in reverse order for all
/// completed operations in the journal.
#[tauri::command]
pub async fn wal_rollback_job(job_id: String) -> Result<RecoveryResult, String> {
    rollback_journal(&job_id)
}

/// Discard an interrupted job without recovery or rollback
///
/// Use this when the user wants to abandon the interrupted job.
/// The journal file will be deleted.
#[tauri::command]
pub async fn wal_discard_job(job_id: String) -> Result<(), String> {
    discard_journal(&job_id)
}

/// Get details about a specific journal
///
/// Returns the full journal including all entries and their statuses.
#[tauri::command]
pub async fn wal_get_journal(job_id: String) -> Result<Option<WALJournal>, String> {
    get_journal_details(&job_id)
}

/// List all journal IDs in the WAL directory
#[tauri::command]
pub async fn wal_list_journals() -> Result<Vec<String>, String> {
    let manager = WALManager::new();
    manager.list_journals().map_err(|e| e.message)
}

/// Create a new WAL journal for an organize operation
///
/// Returns the job_id of the created journal.
#[tauri::command]
pub async fn wal_create_journal(target_folder: String) -> Result<String, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let job_id = format!("wal-{}", now);

    let journal = WALJournal::new(job_id.clone(), PathBuf::from(target_folder));
    let manager = WALManager::new();
    manager.save_journal(&journal).map_err(|e| e.message)?;

    Ok(job_id)
}

/// Add an operation to a journal
///
/// Returns the UUID of the created entry.
#[tauri::command]
pub async fn wal_add_operation(
    job_id: String,
    operation_type: String,
    source: Option<String>,
    destination: Option<String>,
    path: Option<String>,
    new_name: Option<String>,
    quarantine_path: Option<String>,
    depends_on: Option<Vec<String>>,
) -> Result<String, String> {
    let manager = WALManager::new();

    let mut journal = manager
        .load_journal(&job_id)?
        .ok_or_else(|| format!("Journal not found: {}", job_id))?;

    // Parse the operation type
    let operation = match operation_type.as_str() {
        "create_folder" => {
            let path = path.ok_or("path is required for create_folder")?;
            WALOperationType::CreateFolder {
                path: PathBuf::from(path),
            }
        }
        "move" => {
            let source = source.ok_or("source is required for move")?;
            let destination = destination.ok_or("destination is required for move")?;
            WALOperationType::Move {
                source: PathBuf::from(source),
                destination: PathBuf::from(destination),
            }
        }
        "rename" => {
            let path = path.ok_or("path is required for rename")?;
            let new_name = new_name.ok_or("new_name is required for rename")?;
            WALOperationType::Rename {
                path: PathBuf::from(path),
                new_name,
            }
        }
        "quarantine" => {
            let path = path.ok_or("path is required for quarantine")?;
            let qpath = quarantine_path.ok_or("quarantine_path is required for quarantine")?;
            WALOperationType::Quarantine {
                path: PathBuf::from(path),
                quarantine_path: PathBuf::from(qpath),
            }
        }
        "copy" => {
            let source = source.ok_or("source is required for copy")?;
            let destination = destination.ok_or("destination is required for copy")?;
            WALOperationType::Copy {
                source: PathBuf::from(source),
                destination: PathBuf::from(destination),
            }
        }
        "delete_folder" => {
            let path = path.ok_or("path is required for delete_folder")?;
            WALOperationType::DeleteFolder {
                path: PathBuf::from(path),
            }
        }
        _ => return Err(format!("Unknown operation type: {}", operation_type)),
    };

    // Parse dependencies
    let deps: Vec<uuid::Uuid> = depends_on
        .unwrap_or_default()
        .iter()
        .filter_map(|s| uuid::Uuid::parse_str(s).ok())
        .collect();

    // Add operation
    let entry_id = if deps.is_empty() {
        journal.add_operation(operation)
    } else {
        journal.add_operation_with_deps(operation, deps)
    };

    manager.save_journal(&journal).map_err(|e| e.message)?;

    Ok(entry_id.to_string())
}

/// Execute all pending operations in a journal
///
/// Uses the DAG-based execution engine for parallel execution.
#[tauri::command]
pub async fn wal_execute_journal(job_id: String) -> Result<ExecutionResult, String> {
    let engine = ExecutionEngine::new();
    engine.execute_journal(&job_id).await
}

/// Execute operations with a new builder pattern
///
/// Creates a new journal, adds operations, and executes them.
/// This is a convenience command for simple use cases.
#[tauri::command]
pub async fn wal_execute_operations(
    target_folder: String,
    operations: Vec<serde_json::Value>,
) -> Result<ExecutionResult, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let job_id = format!("wal-{}", now);

    let mut builder = ExecutionBuilder::new(job_id, PathBuf::from(&target_folder));

    // Track operation IDs for dependency resolution
    let mut op_id_map: std::collections::HashMap<String, uuid::Uuid> = std::collections::HashMap::new();

    for op in operations {
        let op_id = op
            .get("opId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let op_type = op
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Parse dependencies
        let deps: Vec<uuid::Uuid> = op
            .get("dependsOn")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| op_id_map.get(s).copied())
                    .collect()
            })
            .unwrap_or_default();

        let operation = match op_type {
            "create_folder" | "createFolder" => {
                let path = op.get("path").and_then(|v| v.as_str()).ok_or("path required")?;
                WALOperationType::CreateFolder {
                    path: PathBuf::from(path),
                }
            }
            "move" => {
                let source = op.get("source").and_then(|v| v.as_str()).ok_or("source required")?;
                let dest = op.get("destination").and_then(|v| v.as_str()).ok_or("destination required")?;
                WALOperationType::Move {
                    source: PathBuf::from(source),
                    destination: PathBuf::from(dest),
                }
            }
            "rename" => {
                let path = op.get("path").and_then(|v| v.as_str()).ok_or("path required")?;
                let new_name = op.get("newName").and_then(|v| v.as_str()).ok_or("newName required")?;
                WALOperationType::Rename {
                    path: PathBuf::from(path),
                    new_name: new_name.to_string(),
                }
            }
            "copy" => {
                let source = op.get("source").and_then(|v| v.as_str()).ok_or("source required")?;
                let dest = op.get("destination").and_then(|v| v.as_str()).ok_or("destination required")?;
                WALOperationType::Copy {
                    source: PathBuf::from(source),
                    destination: PathBuf::from(dest),
                }
            }
            "quarantine" => {
                let path = op.get("path").and_then(|v| v.as_str()).ok_or("path required")?;
                let qpath = op.get("quarantinePath").and_then(|v| v.as_str()).ok_or("quarantinePath required")?;
                WALOperationType::Quarantine {
                    path: PathBuf::from(path),
                    quarantine_path: PathBuf::from(qpath),
                }
            }
            _ => return Err(format!("Unknown operation type: {}", op_type)),
        };

        let uuid = if deps.is_empty() {
            builder.add_operation(operation)
        } else {
            builder.add_operation_with_deps(operation, deps)
        };

        if !op_id.is_empty() {
            op_id_map.insert(op_id, uuid);
        }
    }

    builder.execute().await
}

/// Get WAL directory path
#[tauri::command]
pub fn wal_get_directory() -> Result<String, String> {
    let manager = WALManager::new();
    Ok(manager.get_wal_dir().to_string_lossy().to_string())
}
