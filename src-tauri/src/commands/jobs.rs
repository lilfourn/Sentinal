use crate::jobs::{JobManager, JobStatus, OrganizeJob, OrganizeOperation, OrganizePlan};

/// Start a new organize job
#[tauri::command]
pub fn start_organize_job(target_folder: String) -> Result<OrganizeJob, String> {
    let job = OrganizeJob::new(&target_folder);
    JobManager::save_job(&job)?;
    Ok(job)
}

/// Update job with the generated plan
#[tauri::command]
pub fn set_job_plan(
    job_id: String,
    plan_id: String,
    description: String,
    operations: Vec<serde_json::Value>,
    target_folder: String,
) -> Result<OrganizeJob, String> {
    let mut job = JobManager::load_job()?
        .ok_or_else(|| format!("Job not found: {}", job_id))?;

    if job.job_id != job_id {
        return Err(format!("Job ID mismatch: expected {}, got {}", job.job_id, job_id));
    }

    // Convert operations from JSON
    let ops: Vec<OrganizeOperation> = operations
        .into_iter()
        .map(|op| OrganizeOperation {
            op_id: op.get("opId").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            op_type: op.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            source: op.get("source").and_then(|v| v.as_str()).map(String::from),
            destination: op.get("destination").and_then(|v| v.as_str()).map(String::from),
            path: op.get("path").and_then(|v| v.as_str()).map(String::from),
            new_name: op.get("newName").and_then(|v| v.as_str()).map(String::from),
        })
        .collect();

    let plan = OrganizePlan {
        plan_id,
        description,
        operations: ops,
        target_folder,
    };

    job.set_plan(plan);
    JobManager::save_job(&job)?;
    Ok(job)
}

/// Mark an operation as completed
#[tauri::command]
pub fn complete_job_operation(job_id: String, op_id: String, current_index: i32) -> Result<OrganizeJob, String> {
    let mut job = JobManager::load_job()?
        .ok_or_else(|| format!("Job not found: {}", job_id))?;

    if job.job_id != job_id {
        return Err(format!("Job ID mismatch"));
    }

    job.complete_operation(&op_id);
    job.set_current_op(current_index);
    JobManager::save_job(&job)?;
    Ok(job)
}

/// Mark job as completed
#[tauri::command]
pub fn complete_organize_job(job_id: String) -> Result<(), String> {
    let mut job = JobManager::load_job()?
        .ok_or_else(|| format!("Job not found: {}", job_id))?;

    if job.job_id != job_id {
        return Err(format!("Job ID mismatch"));
    }

    job.mark_completed();
    JobManager::save_job(&job)?;

    // Clear the job file after a short delay (let frontend read final state)
    // In production, you might want to keep history
    Ok(())
}

/// Mark job as failed
#[tauri::command]
pub fn fail_organize_job(job_id: String, error: String) -> Result<(), String> {
    let mut job = JobManager::load_job()?
        .ok_or_else(|| format!("Job not found: {}", job_id))?;

    if job.job_id != job_id {
        return Err(format!("Job ID mismatch"));
    }

    job.mark_failed(&error);
    JobManager::save_job(&job)?;
    Ok(())
}

/// Check for interrupted jobs on app startup
#[tauri::command]
pub fn check_interrupted_job() -> Result<Option<OrganizeJob>, String> {
    JobManager::check_for_interrupted_job()
}

/// Get current job status
#[tauri::command]
pub fn get_current_job() -> Result<Option<OrganizeJob>, String> {
    JobManager::load_job()
}

/// Clear the current job (dismiss interrupted job or cleanup)
#[tauri::command]
pub fn clear_organize_job() -> Result<(), String> {
    JobManager::clear_job()
}

/// Resume an interrupted job (returns the job with remaining operations)
#[tauri::command]
pub fn resume_organize_job(job_id: String) -> Result<OrganizeJob, String> {
    let mut job = JobManager::load_job()?
        .ok_or_else(|| format!("Job not found: {}", job_id))?;

    if job.job_id != job_id {
        return Err(format!("Job ID mismatch"));
    }

    if job.status != JobStatus::Interrupted {
        return Err("Job is not in interrupted state".to_string());
    }

    // Mark as running again
    job.status = JobStatus::Running;
    JobManager::save_job(&job)?;

    Ok(job)
}
