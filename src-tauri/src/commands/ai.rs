use crate::ai::{run_v2_agentic_organize, ExpandableDetail, ProgressEvent, AnthropicClient, CredentialManager};
use crate::jobs::OrganizePlan;
use std::path::Path;

/// Rename suggestion response
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameSuggestion {
    pub original_name: String,
    pub suggested_name: String,
    pub path: String,
}

/// API provider status
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub provider: String,
    pub configured: bool,
}

/// Set API key for a provider
#[tauri::command]
pub async fn set_api_key(provider: String, api_key: String) -> Result<bool, String> {
    eprintln!("[DEBUG] set_api_key called for provider: {}", provider);

    // Validate the key first
    if provider == "anthropic" {
        eprintln!("[DEBUG] Validating API key with Anthropic...");
        let is_valid = AnthropicClient::validate_api_key(&api_key).await?;
        if !is_valid {
            eprintln!("[DEBUG] API key validation failed");
            return Ok(false);
        }
        eprintln!("[DEBUG] API key validated successfully");
    }

    // Store the key
    eprintln!("[DEBUG] Attempting to store API key in keychain...");
    match CredentialManager::store_api_key(&provider, &api_key) {
        Ok(_) => {
            eprintln!("[DEBUG] API key stored successfully in keychain");
            // Verify it was stored
            let verify = CredentialManager::has_api_key(&provider);
            eprintln!("[DEBUG] Verification - key exists in keychain: {}", verify);
        }
        Err(e) => {
            eprintln!("[DEBUG] Failed to store API key: {}", e);
            return Err(e);
        }
    }

    Ok(true)
}

/// Delete API key for a provider
#[tauri::command]
pub fn delete_api_key(provider: String) -> Result<(), String> {
    CredentialManager::delete_api_key(&provider)
}

/// Check which providers are configured
/// Checks both credential manager and environment variables
#[tauri::command]
pub fn get_configured_providers() -> Vec<ProviderStatus> {
    // Anthropic: credential manager only (user must configure in settings)
    let has_anthropic = CredentialManager::has_api_key("anthropic");

    // xAI/Grok: check env vars first, then credential manager
    let has_xai = std::env::var("XAI_API_KEY").is_ok()
        || std::env::var("GROK_API_KEY").is_ok()
        || std::env::var("VITE_XAI_API_KEY").is_ok()
        || CredentialManager::has_api_key("xai");

    // OpenAI: check env vars first, then credential manager
    let has_openai = std::env::var("OPENAI_API_KEY").is_ok()
        || std::env::var("VITE_OPENAI_API_KEY").is_ok()
        || CredentialManager::has_api_key("openai");

    eprintln!("[DEBUG] Provider status - anthropic: {}, xai: {}, openai: {}",
        has_anthropic, has_xai, has_openai);

    vec![
        ProviderStatus {
            provider: "anthropic".to_string(),
            configured: has_anthropic,
        },
        ProviderStatus {
            provider: "xai".to_string(),
            configured: has_xai,
        },
        ProviderStatus {
            provider: "openai".to_string(),
            configured: has_openai,
        },
    ]
}

/// Get rename suggestion for a file
#[tauri::command]
pub async fn get_rename_suggestion(
    path: String,
    filename: String,
    extension: Option<String>,
    size: u64,
    content_preview: Option<String>,
) -> Result<RenameSuggestion, String> {
    let client = AnthropicClient::new();

    let suggested = client
        .suggest_rename(
            &filename,
            extension.as_deref(),
            size,
            content_preview.as_deref(),
        )
        .await?;

    Ok(RenameSuggestion {
        original_name: filename,
        suggested_name: suggested,
        path,
    })
}

/// Apply a rename (with undo info)
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameResult {
    pub success: bool,
    pub old_path: String,
    pub new_path: String,
}

/// Validate that a filename is safe (no path traversal)
fn validate_filename(name: &str) -> Result<(), String> {
    // Reject path separators
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Invalid filename: path separators not allowed".to_string());
    }

    // Reject control characters and null bytes
    if name.chars().any(|c| c.is_control() || c == '\0') {
        return Err("Invalid filename: control characters not allowed".to_string());
    }

    // Reject empty or whitespace-only names
    if name.trim().is_empty() {
        return Err("Invalid filename: cannot be empty".to_string());
    }

    // Reject names that are too long (filesystem limit)
    if name.len() > 255 {
        return Err("Invalid filename: name too long".to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn apply_rename(
    old_path: String,
    new_name: String,
) -> Result<RenameResult, String> {
    // SECURITY: Validate filename before any operations
    validate_filename(&new_name)?;

    let old = std::path::Path::new(&old_path);

    if !old.exists() {
        return Err(format!("File does not exist: {}", old_path));
    }

    // SECURITY: Reject symlinks to prevent symlink attacks
    if old.is_symlink() {
        return Err("Cannot rename symbolic links".to_string());
    }

    let parent = old.parent().ok_or("Could not get parent directory")?;
    let new_path = parent.join(&new_name);

    // SECURITY: Verify the new path stays within the same directory
    let canonical_parent = parent.canonicalize()
        .map_err(|e| format!("Parent path validation failed: {}", e))?;

    // For the new file (which doesn't exist yet), verify the parent matches
    let new_parent = new_path.parent().ok_or("Invalid new path")?;
    if new_parent.canonicalize().ok() != Some(canonical_parent.clone()) {
        return Err("Path traversal detected: new file must be in same directory".to_string());
    }

    // Use atomic rename - handles EEXIST race condition
    match std::fs::rename(&old, &new_path) {
        Ok(()) => Ok(RenameResult {
            success: true,
            old_path,
            new_path: new_path.to_string_lossy().to_string(),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(format!("File already exists: {}", new_path.display()))
        }
        Err(e) => Err(format!("Failed to rename: {}", e))
    }
}

/// Undo a rename
#[tauri::command]
pub async fn undo_rename(
    current_path: String,
    original_path: String,
) -> Result<(), String> {
    let current = std::path::Path::new(&current_path);
    let original = std::path::Path::new(&original_path);

    if !current.exists() {
        return Err(format!("File does not exist: {}", current_path));
    }

    // SECURITY: Reject symlinks
    if current.is_symlink() {
        return Err("Cannot undo rename of symbolic links".to_string());
    }

    // SECURITY: Verify both paths are in the same directory
    let current_parent = current.parent().ok_or("Invalid current path")?;
    let original_parent = original.parent().ok_or("Invalid original path")?;

    let canonical_current_parent = current_parent.canonicalize()
        .map_err(|e| format!("Current path validation failed: {}", e))?;
    let canonical_original_parent = original_parent.canonicalize()
        .map_err(|e| format!("Original path validation failed: {}", e))?;

    if canonical_current_parent != canonical_original_parent {
        return Err("Security: undo can only restore to same directory".to_string());
    }

    // SECURITY: Validate the original filename
    let original_name = original.file_name()
        .ok_or("Invalid original filename")?
        .to_string_lossy();
    validate_filename(&original_name)?;

    // Use atomic rename
    match std::fs::rename(&current, &original) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(format!("Original path already exists: {}", original_path))
        }
        Err(e) => Err(format!("Failed to undo rename: {}", e))
    }
}

/// Agentic organize command - explores folder and generates typed plan
/// Uses Claude tool-use with V2 semantic tools and Shadow VFS
#[tauri::command]
pub async fn generate_organize_plan_agentic(
    folder_path: String,
    user_request: String,
    app_handle: tauri::AppHandle,
) -> Result<OrganizePlan, String> {
    use tauri::Emitter;

    let emit = |thought_type: &str, content: &str, expandable_details: Option<Vec<ExpandableDetail>>| {
        let _ = app_handle.emit(
            "ai-thought",
            serde_json::json!({
                "type": thought_type,
                "content": content,
                "expandableDetails": expandable_details,
            }),
        );
    };

    // Create progress emitter for analysis-progress events (updates progress bar)
    let app_handle_clone = app_handle.clone();
    let progress_emit = move |progress: ProgressEvent| {
        let _ = app_handle_clone.emit("analysis-progress", &progress);
    };

    run_v2_agentic_organize(Path::new(&folder_path), &user_request, emit, Some(progress_emit)).await
}

/// Suggest naming conventions for a folder
#[tauri::command]
pub async fn suggest_naming_conventions(
    folder_path: String,
    app_handle: tauri::AppHandle,
) -> Result<crate::ai::NamingConventionSuggestions, String> {
    use tauri::Emitter;

    let path = std::path::Path::new(&folder_path);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Invalid folder path: {}", folder_path));
    }

    // Emit progress event
    let _ = app_handle.emit(
        "ai-thought",
        serde_json::json!({
            "type": "naming_conventions",
            "content": "Analyzing file naming patterns...",
        }),
    );

    // Build file listing (just top-level files for naming analysis)
    let mut file_listing = String::new();
    let entries = std::fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = entry.file_type().map_err(|e| e.to_string())?;

        if file_type.is_file() {
            file_listing.push_str(&format!("{}\n", name));
        }
    }

    if file_listing.is_empty() {
        return Err("No files found in folder".to_string());
    }

    // Get AI suggestions
    let client = AnthropicClient::new();
    let suggestions = client
        .suggest_naming_conventions(&folder_path, &file_listing)
        .await?;

    let _ = app_handle.emit(
        "ai-thought",
        serde_json::json!({
            "type": "naming_conventions",
            "content": format!("Found {} naming conventions", suggestions.suggestions.len()),
        }),
    );

    Ok(suggestions)
}

/// Generate organize plan with selected naming convention
#[tauri::command]
pub async fn generate_organize_plan_with_convention(
    folder_path: String,
    user_request: String,
    convention: Option<crate::ai::NamingConvention>,
    app_handle: tauri::AppHandle,
) -> Result<crate::jobs::OrganizePlan, String> {
    use tauri::Emitter;

    let emit = |thought_type: &str, content: &str, expandable_details: Option<Vec<ExpandableDetail>>| {
        let _ = app_handle.emit(
            "ai-thought",
            serde_json::json!({
                "type": thought_type,
                "content": content,
                "expandableDetails": expandable_details,
            }),
        );
    };

    // Create progress emitter for analysis-progress events (updates progress bar)
    let app_handle_clone = app_handle.clone();
    let progress_emit = move |progress: ProgressEvent| {
        let _ = app_handle_clone.emit("analysis-progress", &progress);
    };

    // Build modified request with convention if provided
    let full_request = if let Some(ref conv) = convention {
        format!(
            "{}\n\nIMPORTANT - NAMING CONVENTION TO APPLY:\nWhen renaming files, use the '{}' convention.\nPattern: {}\nExample: {}\n\nApply this naming style consistently to all file rename operations.",
            user_request, conv.name, conv.pattern, conv.example
        )
    } else {
        user_request
    };

    run_v2_agentic_organize(Path::new(&folder_path), &full_request, emit, Some(progress_emit)).await
}
