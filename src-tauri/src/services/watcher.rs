use notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, RecommendedCache};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Event payload sent to frontend
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeEvent {
    pub id: String,
    pub event_type: String,
    pub path: String,
    pub file_name: String,
    pub extension: Option<String>,
    pub size: u64,
    pub content_preview: Option<String>,
}

/// Watcher state
pub struct WatcherState {
    pub watcher: Option<Debouncer<RecommendedWatcher, RecommendedCache>>,
    pub watching_path: Option<PathBuf>,
    pub enabled: bool,
}

impl Default for WatcherState {
    fn default() -> Self {
        Self {
            watcher: None,
            watching_path: None,
            enabled: false,
        }
    }
}

/// Global watcher state
pub type WatcherHandle = Arc<Mutex<WatcherState>>;

/// Create a new watcher handle
pub fn create_watcher_handle() -> WatcherHandle {
    Arc::new(Mutex::new(WatcherState::default()))
}

/// Start watching a directory
pub fn start_watcher(
    app: AppHandle,
    handle: WatcherHandle,
    path: PathBuf,
) -> Result<(), String> {
    let mut state = handle.lock().map_err(|e| e.to_string())?;

    // Stop existing watcher if any
    if state.watcher.is_some() {
        state.watcher = None;
    }

    let app_clone = app.clone();

    // Create debounced watcher (waits 500ms for file writes to complete)
    let mut debouncer = new_debouncer(
        Duration::from_millis(500),
        None,
        move |result: Result<Vec<DebouncedEvent>, Vec<notify::Error>>| {
            match result {
                Ok(events) => {
                    for event in events {
                        handle_file_event(&app_clone, &event);
                    }
                }
                Err(errors) => {
                    for error in errors {
                        eprintln!("Watcher error: {:?}", error);
                    }
                }
            }
        },
    )
    .map_err(|e| format!("Failed to create watcher: {}", e))?;

    // Start watching the path
    debouncer
        .watch(&path, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch path: {}", e))?;

    state.watcher = Some(debouncer);
    state.watching_path = Some(path);
    state.enabled = true;

    Ok(())
}

/// Stop watching
pub fn stop_watcher(handle: WatcherHandle) -> Result<(), String> {
    let mut state = handle.lock().map_err(|e| e.to_string())?;
    state.watcher = None;
    state.watching_path = None;
    state.enabled = false;
    Ok(())
}

/// Check if watcher is running
pub fn is_watcher_running(handle: &WatcherHandle) -> bool {
    handle
        .lock()
        .map(|state| state.enabled && state.watcher.is_some())
        .unwrap_or(false)
}

/// Get the path being watched
pub fn get_watching_path(handle: &WatcherHandle) -> Option<PathBuf> {
    handle
        .lock()
        .ok()
        .and_then(|state| state.watching_path.clone())
}

/// Handle a file event
fn handle_file_event(app: &AppHandle, event: &DebouncedEvent) {
    // Only handle create events for new files
    let is_create = matches!(event.kind, EventKind::Create(_));

    if !is_create {
        return;
    }

    for path in &event.paths {
        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Skip temporary files and hidden files
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if file_name.starts_with('.') || file_name.ends_with(".tmp") || file_name.ends_with(".crdownload") {
            continue;
        }

        // Get file info
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Skip if file is still being written (size is 0)
        if metadata.len() == 0 {
            continue;
        }

        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_string());

        // Read content preview (first 4KB for text files)
        let content_preview = read_content_preview(path, &extension);

        let event = FileChangeEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: "created".to_string(),
            path: path.to_string_lossy().to_string(),
            file_name,
            extension,
            size: metadata.len(),
            content_preview,
        };

        // Emit event to frontend
        if let Err(e) = app.emit("sentinel://file-created", &event) {
            eprintln!("Failed to emit file event: {}", e);
        }
    }
}

/// Read first 4KB of file content for text-based files
fn read_content_preview(path: &PathBuf, extension: &Option<String>) -> Option<String> {
    let text_extensions = [
        "txt", "md", "json", "yaml", "yml", "toml", "xml", "html", "css", "js", "ts",
        "jsx", "tsx", "py", "rb", "go", "rs", "java", "c", "cpp", "h", "hpp", "swift",
        "kt", "sh", "bash", "zsh", "csv", "log", "ini", "conf", "config", "env",
    ];

    let ext = extension.as_ref()?.to_lowercase();
    if !text_extensions.contains(&ext.as_str()) {
        return None;
    }

    // Read first 4KB
    match std::fs::read(path) {
        Ok(bytes) => {
            let preview_bytes = &bytes[..std::cmp::min(4096, bytes.len())];
            String::from_utf8(preview_bytes.to_vec()).ok()
        }
        Err(_) => None,
    }
}
