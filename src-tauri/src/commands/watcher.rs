use std::path::PathBuf;
use tauri::{AppHandle, State};

use crate::services::watcher::{
    self, is_watcher_running, get_watching_path, WatcherHandle,
};

/// Watcher status response
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatcherStatus {
    pub enabled: bool,
    pub watching_path: Option<String>,
}

/// Start the downloads watcher
#[tauri::command]
pub async fn start_downloads_watcher(
    app: AppHandle,
    handle: State<'_, WatcherHandle>,
    path: Option<String>,
) -> Result<(), String> {
    let watch_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        dirs::download_dir().ok_or("Could not determine downloads directory")?
    };

    if !watch_path.exists() {
        return Err(format!("Path does not exist: {:?}", watch_path));
    }

    if !watch_path.is_dir() {
        return Err(format!("Path is not a directory: {:?}", watch_path));
    }

    watcher::start_watcher(app, handle.inner().clone(), watch_path)?;

    Ok(())
}

/// Stop the downloads watcher
#[tauri::command]
pub async fn stop_downloads_watcher(
    handle: State<'_, WatcherHandle>,
) -> Result<(), String> {
    watcher::stop_watcher(handle.inner().clone())
}

/// Get watcher status
#[tauri::command]
pub fn get_watcher_status(handle: State<'_, WatcherHandle>) -> WatcherStatus {
    WatcherStatus {
        enabled: is_watcher_running(handle.inner()),
        watching_path: get_watching_path(handle.inner())
            .map(|p| p.to_string_lossy().to_string()),
    }
}
