use crate::security::PathValidator;
use duct::cmd;
use std::path::{Path, PathBuf};

/// Whitelist of allowed shell commands
const ALLOWED_COMMANDS: &[&str] = &["ls", "grep", "find", "cat"];

/// Maximum output size in bytes (10KB)
const MAX_OUTPUT_SIZE: usize = 10 * 1024;

/// Execute a tool and return the result
pub fn execute_tool(
    tool_name: &str,
    input: &serde_json::Value,
    allowed_base_path: &Path,
) -> Result<String, String> {
    match tool_name {
        "run_shell_command" => execute_shell_command(input, allowed_base_path),
        "edit_file" => execute_edit_file(input, allowed_base_path),
        _ => Err(format!("Unknown tool: {}", tool_name)),
    }
}

/// Execute a whitelisted shell command
fn execute_shell_command(
    input: &serde_json::Value,
    allowed_base: &Path,
) -> Result<String, String> {
    let command = input
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'command' parameter")?;

    let working_dir = input
        .get("working_directory")
        .and_then(|v| v.as_str())
        .map(Path::new)
        .unwrap_or(allowed_base);

    // 1. Validate working directory is within allowed base
    validate_path_within(working_dir, allowed_base)?;

    // 2. Parse command and validate against whitelist
    let parts: Vec<&str> = command.split_whitespace().collect();
    let cmd_name = parts.first().ok_or("Empty command")?;

    if !ALLOWED_COMMANDS.contains(cmd_name) {
        return Err(format!(
            "Command '{}' not allowed. Only {:?} are permitted.",
            cmd_name, ALLOWED_COMMANDS
        ));
    }

    // 3. Execute using duct (safer than std::process::Command)
    eprintln!("[ToolExecutor] Running: {} in {:?}", command, working_dir);

    let output = cmd("sh", &["-c", command])
        .dir(working_dir)
        .stderr_to_stdout()
        .read()
        .map_err(|e| format!("Command failed: {}", e))?;

    // 4. Truncate output if too large
    Ok(truncate_output(&output, MAX_OUTPUT_SIZE))
}

/// Write content to a file within the allowed base path
fn execute_edit_file(
    input: &serde_json::Value,
    allowed_base: &Path,
) -> Result<String, String> {
    let path_str = input
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' parameter")?;

    let content = input
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'content' parameter")?;

    let path = Path::new(path_str);

    // 1. Validate path is within allowed base
    validate_path_within(path, allowed_base)?;

    // 2. Check not a protected system path
    if PathValidator::is_protected_path(path) {
        return Err(format!("Cannot write to protected path: {}", path_str));
    }

    // 3. Write file
    eprintln!("[ToolExecutor] Writing {} bytes to {}", content.len(), path_str);

    std::fs::write(path, content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(format!(
        "Successfully wrote {} bytes to {}",
        content.len(),
        path_str
    ))
}

/// Validate that a path is within the allowed base directory
///
/// NOTE: Uses lexical comparison instead of canonicalize() because macOS has
/// issues with canonicalize() on paths containing apostrophes/special chars.
/// See: https://doc.rust-lang.org/std/fs/fn.canonicalize.html
fn validate_path_within(path: &Path, base: &Path) -> Result<(), String> {
    // Normalize both paths lexically (resolve . and .., make absolute)
    let normalized_path = normalize_path(path)?;
    let normalized_base = normalize_path(base)?;

    // Check that path starts with base
    if !normalized_path.starts_with(&normalized_base) {
        return Err(format!(
            "Path {} is outside allowed directory {}",
            path.display(),
            base.display()
        ));
    }

    // Additional check: ensure no path traversal after normalization
    let relative = normalized_path
        .strip_prefix(&normalized_base)
        .map_err(|_| "Path is outside allowed directory")?;

    // Reject if the relative path tries to escape (shouldn't happen after normalization, but safety check)
    for component in relative.components() {
        if let std::path::Component::ParentDir = component {
            return Err("Path traversal detected".to_string());
        }
    }

    Ok(())
}

/// Normalize a path lexically without filesystem access
/// This avoids macOS canonicalize() issues with special characters like apostrophes
fn normalize_path(path: &Path) -> Result<PathBuf, String> {
    use std::path::Component;

    let mut normalized = PathBuf::new();

    // Make path absolute if it isn't already
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Cannot get current directory: {}", e))?
            .join(path)
    };

    for component in absolute_path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push("/"),
            Component::CurDir => {} // Skip .
            Component::ParentDir => {
                // Go up one directory
                normalized.pop();
            }
            Component::Normal(name) => normalized.push(name),
        }
    }

    Ok(normalized)
}

/// Truncate output to max size with indicator
fn truncate_output(output: &str, max_len: usize) -> String {
    if output.len() > max_len {
        format!(
            "{}...\n[truncated, {} more bytes]",
            &output[..max_len],
            output.len() - max_len
        )
    } else {
        output.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_whitelist_enforcement() {
        let input = serde_json::json!({
            "command": "rm -rf /"
        });
        let base = env::temp_dir();
        let result = execute_shell_command(&input, &base);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not allowed"));
    }

    #[test]
    fn test_allowed_command() {
        let input = serde_json::json!({
            "command": "ls -la"
        });
        let base = env::temp_dir();
        let result = execute_shell_command(&input, &base);
        assert!(result.is_ok());
    }
}
