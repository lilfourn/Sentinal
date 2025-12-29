use regex::Regex;
use std::path::{Path, PathBuf};

/// Security validator for path operations
pub struct PathValidator;

/// Command validator for shell operations
#[allow(dead_code)]
pub struct CommandValidator;

impl PathValidator {
    /// Check if a path is protected and should not be modified
    pub fn is_protected_path(path: &Path) -> bool {
        let protected_paths: Vec<PathBuf> = vec![
            PathBuf::from("/"),
            PathBuf::from("/System"),
            PathBuf::from("/usr"),
            PathBuf::from("/bin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/Library"),
            PathBuf::from("/Applications"),
            PathBuf::from("/private"),
            PathBuf::from("/var"),
            // Windows system paths
            PathBuf::from("C:\\Windows"),
            PathBuf::from("C:\\Program Files"),
            PathBuf::from("C:\\Program Files (x86)"),
        ];

        // Get canonical path if possible
        let check_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        for protected in &protected_paths {
            if check_path == *protected {
                return true;
            }
            // Only protect the root of these paths, not subdirectories we own
            if check_path.starts_with(protected) {
                // Allow user directories within home
                if let Some(home) = dirs::home_dir() {
                    if check_path.starts_with(&home) {
                        return false;
                    }
                }
                // Block if it's a direct child of a protected path
                if check_path.parent() == Some(protected) {
                    return true;
                }
            }
        }

        // Block home directory itself (but not subdirectories)
        if let Some(home) = dirs::home_dir() {
            if check_path == home {
                return true;
            }
        }

        false
    }

    /// Check if a path is within allowed user directories
    #[allow(dead_code)]
    pub fn is_allowed_path(path: &Path) -> bool {
        if let Some(home) = dirs::home_dir() {
            let allowed_dirs = [
                home.join("Downloads"),
                home.join("Documents"),
                home.join("Desktop"),
                home.join("Pictures"),
                home.join("Music"),
                home.join("Videos"),
                home.join("Movies"),
            ];

            let check_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

            for allowed in &allowed_dirs {
                if check_path.starts_with(allowed) {
                    return true;
                }
            }
        }

        // Also allow any path that's not protected
        !Self::is_protected_path(path)
    }

    /// Validate a path for delete operations (more strict)
    pub fn validate_for_delete(path: &Path) -> Result<(), String> {
        if Self::is_protected_path(path) {
            return Err(format!("Cannot delete protected path: {:?}", path));
        }

        // Don't allow deleting home directory
        if let Some(home) = dirs::home_dir() {
            if path == home {
                return Err("Cannot delete home directory".to_string());
            }
        }

        Ok(())
    }
}

#[allow(dead_code)]
impl CommandValidator {
    /// Dangerous command patterns that should be blocked
    const BLOCKED_PATTERNS: &'static [&'static str] = &[
        r"rm\s+-rf\s+/",          // rm -rf /
        r"rm\s+-rf\s+~",          // rm -rf ~
        r"rm\s+-rf\s+\$HOME",     // rm -rf $HOME
        r"rm\s+-rf\s+/home",      // rm -rf /home
        r"rm\s+-rf\s+/Users",     // rm -rf /Users
        r">\s*/dev/",             // redirect to /dev/
        r"dd\s+.*of=/dev/",       // dd to device
        r"mkfs\.",                // format filesystem
        r"chmod\s+-R\s+777\s+/",  // chmod 777 /
        r"chown\s+-R\s+.*\s+/",   // chown root stuff
        r":()\{:|:&\};:",         // fork bomb
        r"\|\s*bash",             // pipe to bash (potential injection)
        r"\|\s*sh\s",             // pipe to sh
        r"curl\s+.*\|\s*bash",    // curl | bash
        r"wget\s+.*\|\s*bash",    // wget | bash
        r"sudo\s+",               // sudo commands
        r"doas\s+",               // doas commands
    ];

    /// Validate a command before execution
    pub fn validate_command(command: &str) -> Result<(), String> {
        let command_lower = command.to_lowercase();

        for pattern in Self::BLOCKED_PATTERNS {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(&command_lower) {
                    return Err(format!(
                        "Command blocked: matches dangerous pattern '{}'",
                        pattern
                    ));
                }
            }
        }

        // Check for attempts to modify system paths
        let system_paths = ["/bin", "/sbin", "/usr", "/System", "/Library", "/etc"];
        for sys_path in system_paths {
            if command.contains(sys_path) {
                // Allow read operations
                if command.starts_with("ls ")
                    || command.starts_with("cat ")
                    || command.starts_with("head ")
                    || command.starts_with("tail ")
                    || command.starts_with("grep ")
                    || command.starts_with("find ")
                {
                    continue;
                }
                // Block write operations to system paths
                if command.contains("rm ")
                    || command.contains("mv ")
                    || command.contains("cp ")
                    || command.contains(">")
                {
                    return Err(format!(
                        "Cannot modify system path: {}",
                        sys_path
                    ));
                }
            }
        }

        Ok(())
    }

    /// Sanitize a command for safe execution
    pub fn sanitize_command(command: &str) -> String {
        // Remove any null bytes
        let sanitized = command.replace('\0', "");
        // Remove any ANSI escape sequences
        let ansi_regex = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
        ansi_regex.replace_all(&sanitized, "").to_string()
    }
}
