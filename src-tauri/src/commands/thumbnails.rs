use crate::services::thumbnails;

#[tauri::command]
pub async fn get_thumbnail(path: String, size: Option<u32>) -> Result<String, String> {
    // Run thumbnail generation in a blocking task to not block the async runtime
    tokio::task::spawn_blocking(move || thumbnails::get_thumbnail(&path, size))
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
pub async fn clear_thumbnail_cache() -> Result<u64, String> {
    tokio::task::spawn_blocking(thumbnails::clear_cache)
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
pub async fn get_thumbnail_cache_stats() -> Result<thumbnails::CacheStats, String> {
    tokio::task::spawn_blocking(thumbnails::get_cache_stats)
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}
