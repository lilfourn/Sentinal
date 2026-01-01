//! Shell command permissions management
//!
//! Stores user-approved command patterns in a settings file,
//! similar to Claude Code's ~/.claude/settings.json
//!
//! File location: ~/.sentinel/shell_permissions.json

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Shell permissions configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShellPermissions {
    /// Commands that are always allowed (user approved)
    #[serde(default)]
    pub allowed_commands: Vec<String>,

    /// Command patterns that are always allowed (e.g., "find *", "grep *")
    #[serde(default)]
    pub allowed_patterns: Vec<String>,

    /// Commands that are explicitly denied
    #[serde(default)]
    pub denied_commands: Vec<String>,
}

impl ShellPermissions {
    /// Get the path to the permissions file
    pub fn file_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".sentinel").join("shell_permissions.json"))
    }

    /// Load permissions from file, or return default if not exists
    pub fn load() -> Self {
        let Some(path) = Self::file_path() else {
            return Self::default();
        };

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save permissions to file
    pub fn save(&self) -> Result<(), String> {
        let path = Self::file_path().ok_or("Could not determine home directory")?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create settings directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize permissions: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Failed to write permissions file: {}", e))
    }

    /// Check if a command is allowed
    pub fn is_allowed(&self, command: &str) -> bool {
        // Check exact match in allowed commands
        if self.allowed_commands.contains(&command.to_string()) {
            return true;
        }

        // Check patterns (simple glob-style matching)
        for pattern in &self.allowed_patterns {
            if Self::matches_pattern(command, pattern) {
                return true;
            }
        }

        false
    }

    /// Add a command to the allowed list
    pub fn allow_command(&mut self, command: &str) {
        let cmd = command.to_string();
        if !self.allowed_commands.contains(&cmd) {
            self.allowed_commands.push(cmd);
        }
    }

    /// Add a pattern to the allowed list (e.g., "find ~*", "grep *")
    pub fn allow_pattern(&mut self, pattern: &str) {
        let pat = pattern.to_string();
        if !self.allowed_patterns.contains(&pat) {
            self.allowed_patterns.push(pat);
        }
    }

    /// Remove a command from the allowed list
    pub fn revoke_command(&mut self, command: &str) {
        self.allowed_commands.retain(|c| c != command);
    }

    /// Simple glob-style pattern matching
    /// Supports * for any characters
    fn matches_pattern(command: &str, pattern: &str) -> bool {
        // Handle simple patterns:
        // "find *" matches any find command
        // "*cover*" matches any command containing "cover"

        if pattern == "*" {
            return true;
        }

        if pattern.contains('*') {
            // Split pattern by * and check if all parts appear in order
            let parts: Vec<&str> = pattern.split('*').filter(|s| !s.is_empty()).collect();

            if parts.is_empty() {
                return true; // Pattern is just "*"
            }

            let mut remaining = command;
            for (i, part) in parts.iter().enumerate() {
                if let Some(pos) = remaining.find(part) {
                    // First part must be at start if pattern doesn't start with *
                    if i == 0 && !pattern.starts_with('*') && pos != 0 {
                        return false;
                    }
                    remaining = &remaining[pos + part.len()..];
                } else {
                    return false;
                }
            }

            // Last part must be at end if pattern doesn't end with *
            if !pattern.ends_with('*') && !remaining.is_empty() {
                return false;
            }

            true
        } else {
            // Exact match
            command == pattern
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        assert!(ShellPermissions::matches_pattern("find ~ -name '*.pdf'", "find *"));
        assert!(ShellPermissions::matches_pattern("grep pattern file.txt", "grep *"));
        assert!(ShellPermissions::matches_pattern("ls -la /home", "*"));
        assert!(!ShellPermissions::matches_pattern("rm -rf /", "find *"));

        // Test patterns with prefix/suffix
        assert!(ShellPermissions::matches_pattern("find ~ -name foo", "find ~*"));
        assert!(!ShellPermissions::matches_pattern("grep foo", "find ~*"));
    }

    #[test]
    fn test_allow_command() {
        let mut perms = ShellPermissions::default();
        assert!(!perms.is_allowed("find ~ -name '*.pdf'"));

        perms.allow_command("find ~ -name '*.pdf'");
        assert!(perms.is_allowed("find ~ -name '*.pdf'"));
    }

    #[test]
    fn test_allow_pattern() {
        let mut perms = ShellPermissions::default();
        perms.allow_pattern("find *");

        assert!(perms.is_allowed("find ~ -name '*.pdf'"));
        assert!(perms.is_allowed("find /home -type f"));
        assert!(!perms.is_allowed("grep pattern file"));
    }
}
