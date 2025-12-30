//! Vector Index Tauri Commands
//!
//! Provides Tauri commands for initializing and querying the vector index,
//! as well as generating compressed tree XML for AI context.

use crate::models::FileEntry;
use crate::tree::{to_xml, TreeCompressor, TreeConfig};
use crate::vector::{VectorConfig, VectorIndex};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tauri::State;

/// Shared state for the vector index
pub struct VectorState(pub Arc<RwLock<Option<VectorIndex>>>);

impl Default for VectorState {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(None)))
    }
}

/// Shared state for tree configuration
pub struct TreeState {
    pub config: RwLock<TreeConfig>,
}

impl Default for TreeState {
    fn default() -> Self {
        Self {
            config: RwLock::new(TreeConfig::default()),
        }
    }
}

/// Initialize the vector index for a folder
///
/// Indexes all files in the folder for semantic search.
/// Returns the number of files indexed.
///
/// Note: This downloads the embedding model on first use (~100MB)
#[tauri::command]
pub async fn init_vector_index(
    folder_path: String,
    state: State<'_, VectorState>,
) -> Result<usize, String> {
    eprintln!("[VectorCommand] Initializing vector index for: {}", folder_path);

    let path = PathBuf::from(&folder_path);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Invalid folder path: {}", folder_path));
    }

    // Create the vector index
    let config = VectorConfig::default();
    let mut index = VectorIndex::new(config)?;

    // Collect files to index
    let files = collect_files_recursive(&path, 5)?;
    eprintln!("[VectorCommand] Found {} files to index", files.len());

    if files.is_empty() {
        // Store empty index
        let mut state_guard = state.0.write().map_err(|e| e.to_string())?;
        *state_guard = Some(index);
        return Ok(0);
    }

    // Prepare batch for indexing
    let batch: Vec<(PathBuf, String, Option<String>)> = files
        .into_iter()
        .map(|entry| {
            let file_path = PathBuf::from(&entry.path);
            let content_preview = get_content_preview(&file_path);
            (file_path, entry.name, content_preview)
        })
        .collect();

    // Index in batches of 100 for memory efficiency
    let mut total_indexed = 0;
    for chunk in batch.chunks(100) {
        let chunk_vec: Vec<(PathBuf, String, Option<String>)> = chunk.to_vec();
        match index.index_batch(chunk_vec) {
            Ok(count) => {
                total_indexed += count;
                eprintln!("[VectorCommand] Indexed {} files (total: {})", count, total_indexed);
            }
            Err(e) => {
                eprintln!("[VectorCommand] Warning: Batch indexing failed: {}", e);
            }
        }
    }

    // Store the index in state
    let mut state_guard = state.0.write().map_err(|e| e.to_string())?;
    *state_guard = Some(index);

    eprintln!("[VectorCommand] Vector index initialized with {} documents", total_indexed);
    Ok(total_indexed)
}

/// Search the vector index with a natural language query
///
/// Returns a list of (path, similarity_score) tuples sorted by relevance
#[tauri::command]
pub async fn vector_search(
    query: String,
    state: State<'_, VectorState>,
) -> Result<Vec<(String, f32)>, String> {
    eprintln!("[VectorCommand] Searching for: {}", query);

    let state_guard = state.0.read().map_err(|e| e.to_string())?;
    let index = state_guard
        .as_ref()
        .ok_or_else(|| "Vector index not initialized. Call init_vector_index first.".to_string())?;

    let results = index.search(&query)?;

    let string_results: Vec<(String, f32)> = results
        .into_iter()
        .map(|(path, score)| (path.to_string_lossy().to_string(), score))
        .collect();

    eprintln!("[VectorCommand] Found {} results", string_results.len());
    Ok(string_results)
}

/// Get semantic tags for a specific file
#[tauri::command]
pub async fn vector_get_tags(
    file_path: String,
    state: State<'_, VectorState>,
) -> Result<Vec<String>, String> {
    let path = PathBuf::from(&file_path);

    let state_guard = state.0.read().map_err(|e| e.to_string())?;
    let index = state_guard
        .as_ref()
        .ok_or_else(|| "Vector index not initialized".to_string())?;

    Ok(index.get_tags(&path).unwrap_or_default())
}

/// Find files with a specific tag
#[tauri::command]
pub async fn vector_find_by_tag(
    tag: String,
    state: State<'_, VectorState>,
) -> Result<Vec<String>, String> {
    let state_guard = state.0.read().map_err(|e| e.to_string())?;
    let index = state_guard
        .as_ref()
        .ok_or_else(|| "Vector index not initialized".to_string())?;

    let paths = index.find_by_tag(&tag);
    Ok(paths.into_iter().map(|p| p.to_string_lossy().to_string()).collect())
}

/// Find files similar to a given file
#[tauri::command]
pub async fn vector_find_similar(
    file_path: String,
    limit: Option<usize>,
    state: State<'_, VectorState>,
) -> Result<Vec<(String, f32)>, String> {
    let path = PathBuf::from(&file_path);
    let max_results = limit.unwrap_or(10);

    let state_guard = state.0.read().map_err(|e| e.to_string())?;
    let index = state_guard
        .as_ref()
        .ok_or_else(|| "Vector index not initialized".to_string())?;

    let results = index.find_similar(&path, max_results)?;

    Ok(results
        .into_iter()
        .map(|(p, score)| (p.to_string_lossy().to_string(), score))
        .collect())
}

/// Get all unique tags in the index
#[tauri::command]
pub async fn vector_all_tags(
    state: State<'_, VectorState>,
) -> Result<Vec<String>, String> {
    let state_guard = state.0.read().map_err(|e| e.to_string())?;
    let index = state_guard
        .as_ref()
        .ok_or_else(|| "Vector index not initialized".to_string())?;

    Ok(index.all_tags())
}

/// Get vector index statistics
#[tauri::command]
pub async fn vector_stats(
    state: State<'_, VectorState>,
) -> Result<VectorIndexStats, String> {
    let state_guard = state.0.read().map_err(|e| e.to_string())?;

    match state_guard.as_ref() {
        Some(index) => Ok(VectorIndexStats {
            initialized: true,
            document_count: index.len(),
            tag_count: index.all_tags().len(),
        }),
        None => Ok(VectorIndexStats {
            initialized: false,
            document_count: 0,
            tag_count: 0,
        }),
    }
}

/// Vector index statistics
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorIndexStats {
    pub initialized: bool,
    pub document_count: usize,
    pub tag_count: usize,
}

/// Generate compressed tree XML for a folder
///
/// Creates a token-optimized XML representation suitable for AI context.
/// Intelligently collapses homogeneous folders to reduce token usage.
#[tauri::command]
pub async fn get_tree_xml(
    folder_path: String,
    state: State<'_, VectorState>,
    tree_state: State<'_, TreeState>,
) -> Result<String, String> {
    eprintln!("[TreeCommand] Generating tree XML for: {}", folder_path);

    let path = PathBuf::from(&folder_path);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Invalid folder path: {}", folder_path));
    }

    let config = tree_state.config.read().map_err(|e| e.to_string())?.clone();
    let compressor = TreeCompressor::new(config);

    // Get vector index reference if available
    let state_guard = state.0.read().map_err(|e| e.to_string())?;
    let vector_index = state_guard.as_ref();

    let compressed = compressor.compress(&path, vector_index)?;

    let xml = to_xml(&compressed);
    eprintln!("[TreeCommand] Generated XML ({} chars, {} nodes)", xml.len(), compressed.node_count());

    Ok(xml)
}

/// Configure tree compression settings
#[tauri::command]
pub async fn configure_tree(
    collapse_threshold: Option<usize>,
    max_depth: Option<usize>,
    include_tags: Option<bool>,
    entropy_threshold: Option<f64>,
    tree_state: State<'_, TreeState>,
) -> Result<(), String> {
    let mut config = tree_state.config.write().map_err(|e| e.to_string())?;

    if let Some(threshold) = collapse_threshold {
        config.collapse_threshold = threshold;
    }
    if let Some(depth) = max_depth {
        config.max_depth = depth;
    }
    if let Some(include) = include_tags {
        config.include_tags = include;
    }
    if let Some(entropy) = entropy_threshold {
        config.entropy_threshold = entropy;
    }

    eprintln!("[TreeCommand] Tree config updated: {:?}", *config);
    Ok(())
}

/// Get current tree configuration
#[tauri::command]
pub async fn get_tree_config(
    tree_state: State<'_, TreeState>,
) -> Result<TreeConfigResponse, String> {
    let config = tree_state.config.read().map_err(|e| e.to_string())?;

    Ok(TreeConfigResponse {
        collapse_threshold: config.collapse_threshold,
        max_depth: config.max_depth,
        include_tags: config.include_tags,
        entropy_threshold: config.entropy_threshold,
    })
}

/// Tree configuration response
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeConfigResponse {
    pub collapse_threshold: usize,
    pub max_depth: usize,
    pub include_tags: bool,
    pub entropy_threshold: f64,
}

/// Clear the vector index
#[tauri::command]
pub async fn clear_vector_index(
    state: State<'_, VectorState>,
) -> Result<(), String> {
    let mut state_guard = state.0.write().map_err(|e| e.to_string())?;
    *state_guard = None;
    eprintln!("[VectorCommand] Vector index cleared");
    Ok(())
}

// === Helper Functions ===

/// Recursively collect files from a directory
fn collect_files_recursive(path: &PathBuf, max_depth: usize) -> Result<Vec<FileEntry>, String> {
    let mut files = Vec::new();
    collect_files_recursive_inner(path, &mut files, 0, max_depth)?;
    Ok(files)
}

fn collect_files_recursive_inner(
    path: &PathBuf,
    files: &mut Vec<FileEntry>,
    depth: usize,
    max_depth: usize,
) -> Result<(), String> {
    if depth > max_depth {
        return Ok(());
    }

    let entries = std::fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory {:?}: {}", path, e))?;

    for entry in entries.filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files
        if name.starts_with('.') {
            continue;
        }

        if let Ok(file_type) = entry.file_type() {
            if file_type.is_file() {
                if let Ok(file_entry) = FileEntry::from_path(&entry_path) {
                    files.push(file_entry);
                }
            } else if file_type.is_dir() {
                collect_files_recursive_inner(&entry_path, files, depth + 1, max_depth)?;
            }
        }
    }

    Ok(())
}

/// Get a content preview for a file (for better semantic matching)
///
/// Currently supports text files; returns None for binary files
fn get_content_preview(path: &PathBuf) -> Option<String> {
    // Only read text files
    let extension = path.extension()?.to_str()?;

    let text_extensions = [
        "txt", "md", "json", "yaml", "yml", "toml", "xml", "html", "css",
        "js", "ts", "jsx", "tsx", "py", "rs", "go", "java", "c", "cpp",
        "h", "hpp", "swift", "kt", "rb", "php", "sh", "bash", "zsh",
        "sql", "csv", "log", "conf", "ini", "env", "gitignore",
    ];

    if !text_extensions.contains(&extension.to_lowercase().as_str()) {
        return None;
    }

    // Read first 500 bytes
    match std::fs::read(path) {
        Ok(bytes) => {
            let preview_len = bytes.len().min(500);
            String::from_utf8(bytes[..preview_len].to_vec()).ok()
        }
        Err(_) => None,
    }
}
