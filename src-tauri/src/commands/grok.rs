//! Grok AI Commands
//!
//! Tauri commands for the multi-agent Grok analysis system.

use crate::ai::grok::{
    DocumentAnalysis, GrokOrganizer, OrganizationPlan,
    ScanResult,
};
#[allow(unused_imports)]
use crate::ai::grok::{AnalysisPhase, AnalysisProgress};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

/// State for the Grok organizer
pub struct GrokState {
    organizer: Mutex<Option<Arc<GrokOrganizer>>>,
}

impl GrokState {
    pub fn new() -> Self {
        Self {
            organizer: Mutex::new(None),
        }
    }
}

impl Default for GrokState {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the Grok organizer with an API key
/// If api_key is empty, will try to get from environment or credential manager
#[tauri::command]
pub async fn grok_init(
    api_key: Option<String>,
    state: State<'_, GrokState>,
    app: AppHandle,
) -> Result<(), String> {
    // Get API key from parameter or fallback sources
    let key = match api_key {
        Some(k) if !k.is_empty() => k,
        _ => get_grok_api_key()?,
    };

    // Get cache directory
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("Failed to get cache dir: {}", e))?
        .join("grok_cache");

    let organizer = GrokOrganizer::new(key, &cache_dir)?;

    let mut guard = state.organizer.lock().await;
    *guard = Some(Arc::new(organizer));

    tracing::info!("[Grok] Organizer initialized");
    Ok(())
}

/// Scan a folder to identify analyzable files
#[tauri::command]
pub async fn grok_scan_folder(
    path: String,
    state: State<'_, GrokState>,
) -> Result<ScanResult, String> {
    let guard = state.organizer.lock().await;
    let organizer = guard
        .as_ref()
        .ok_or("Grok not initialized. Call grok_init first.")?;

    let path = PathBuf::from(path);
    organizer.scan_folder(&path).await
}

/// Run the full organization pipeline
#[tauri::command]
pub async fn grok_organize(
    path: String,
    user_instruction: String,
    state: State<'_, GrokState>,
    app: AppHandle,
) -> Result<OrganizationPlan, String> {
    let guard = state.organizer.lock().await;
    let organizer = guard
        .as_ref()
        .ok_or("Grok not initialized. Call grok_init first.")?
        .clone();
    drop(guard); // Release lock before long-running operation

    let path = PathBuf::from(path);
    let app_clone = app.clone();

    let plan = organizer
        .organize(&path, &user_instruction, move |progress| {
            // Emit progress events to frontend
            let _ = app_clone.emit("grok:progress", &progress);
        })
        .await?;

    Ok(plan)
}

/// Analyze a single file
#[tauri::command]
pub async fn grok_analyze_file(
    path: String,
    state: State<'_, GrokState>,
) -> Result<DocumentAnalysis, String> {
    let guard = state.organizer.lock().await;
    let organizer = guard
        .as_ref()
        .ok_or("Grok not initialized. Call grok_init first.")?;

    let path = PathBuf::from(path);
    organizer.analyze_single(&path).await
}

/// Get cache statistics
#[tauri::command]
pub async fn grok_cache_stats(
    state: State<'_, GrokState>,
) -> Result<GrokCacheStats, String> {
    let guard = state.organizer.lock().await;
    let organizer = guard
        .as_ref()
        .ok_or("Grok not initialized. Call grok_init first.")?;

    let stats = organizer.cache_stats()?;

    Ok(GrokCacheStats {
        files_analyzed: stats.files_analyzed as usize,
        tokens_used: stats.tokens_used as usize,
        cost_cents: stats.cost_cents as usize,
        cache_hits: stats.cache_hits as usize,
    })
}

/// Clear the content cache
#[tauri::command]
pub async fn grok_clear_cache(state: State<'_, GrokState>) -> Result<(), String> {
    let guard = state.organizer.lock().await;
    let organizer = guard
        .as_ref()
        .ok_or("Grok not initialized. Call grok_init first.")?;

    organizer.clear_cache()?;
    tracing::info!("[Grok] Cache cleared");
    Ok(())
}

/// Cache statistics for frontend
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrokCacheStats {
    pub files_analyzed: usize,
    pub tokens_used: usize,
    pub cost_cents: usize,
    pub cache_hits: usize,
}

/// Check if Grok API key is configured
#[tauri::command]
pub async fn grok_check_api_key() -> Result<bool, String> {
    // Check environment variable or credential manager
    let has_env_key = std::env::var("XAI_API_KEY").is_ok()
        || std::env::var("GROK_API_KEY").is_ok()
        || std::env::var("VITE_XAI_API_KEY").is_ok();

    if has_env_key {
        return Ok(true);
    }

    // Check credential manager
    use crate::ai::credentials::CredentialManager;
    match CredentialManager::get_api_key("xai") {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Get the Grok API key from any available source
fn get_grok_api_key() -> Result<String, String> {
    // Priority: env vars > credential manager
    if let Ok(key) = std::env::var("XAI_API_KEY") {
        return Ok(key);
    }
    if let Ok(key) = std::env::var("GROK_API_KEY") {
        return Ok(key);
    }
    if let Ok(key) = std::env::var("VITE_XAI_API_KEY") {
        return Ok(key);
    }

    // Try credential manager
    use crate::ai::credentials::CredentialManager;
    CredentialManager::get_api_key("xai")
        .map_err(|_| "No Grok API key found. Set XAI_API_KEY in .env or configure in settings.".to_string())
}

/// Store Grok API key (uses the existing credential manager)
#[tauri::command]
pub async fn grok_set_api_key(api_key: String) -> Result<(), String> {
    use crate::ai::credentials::CredentialManager;

    CredentialManager::store_api_key("xai", &api_key)?;
    tracing::info!("[Grok] API key stored");
    Ok(())
}

/// Get Grok API key from credential manager
#[tauri::command]
pub async fn grok_get_api_key() -> Result<Option<String>, String> {
    use crate::ai::credentials::CredentialManager;

    match CredentialManager::get_api_key("xai") {
        Ok(key) => Ok(Some(key)),
        Err(_) => Ok(None),
    }
}
