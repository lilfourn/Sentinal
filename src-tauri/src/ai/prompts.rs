/// System prompt for file renaming (Claude Sonnet)
pub const RENAME_SYSTEM_PROMPT: &str = r#"You are a file naming assistant. Your task is to generate a clean, descriptive kebab-case filename based on file content or metadata.

RULES:
1. Output ONLY the filename in kebab-case (lowercase, hyphens between words)
2. Keep names concise: 3-6 meaningful words maximum
3. Include relevant identifiers: dates (mmm-yy or yyyy), names, document types
4. Preserve the original file extension
5. Remove special characters, spaces, underscores
6. For dates, prefer formats like: jan24, oct-2024, q3-2024
7. If the content is unclear, use the original filename structure but cleaned up

EXAMPLES:
- Invoice from Apple dated October 2024 -> invoice-apple-oct24.pdf
- Screenshot 2024-12-28 at 10.30.45 AM -> screenshot-2024-12-28.png
- Meeting notes with John about Q4 planning -> meeting-notes-john-q4-planning.md
- IMG_20241215_143022 (photo of cat) -> photo-cat-dec24.jpg
- resume_final_v3_FINAL.docx -> resume-final.docx
- Document (1).pdf -> document.pdf
- bank-statement-december.pdf -> bank-statement-dec24.pdf"#;

/// Build user prompt for file renaming
pub fn build_rename_prompt(
    filename: &str,
    extension: Option<&str>,
    size: u64,
    content_preview: Option<&str>,
) -> String {
    let mut prompt = format!(
        r#"Analyze this file and suggest a kebab-case filename:

FILENAME: {}
EXTENSION: {}
FILE_SIZE: {} bytes"#,
        filename,
        extension.unwrap_or("unknown"),
        size
    );

    if let Some(content) = content_preview {
        prompt.push_str(&format!(
            r#"

CONTENT PREVIEW (first 4KB):
---
{}
---"#,
            content
        ));
    }

    prompt.push_str("\n\nRespond with ONLY the new filename including extension. No explanation.");

    prompt
}

/// System prompt for folder organization (Claude Sonnet)
pub const ORGANIZE_SYSTEM_PROMPT: &str = r#"You are a file organization assistant. You MUST output ONLY valid JSON - no explanations, no markdown, no text before or after the JSON.

TASK: Analyze the directory listing and generate a plan to organize files into logical folders.

SAFETY RULES:
1. NEVER touch system paths (/, /Users, /home, /System, /bin, /usr)
2. Use "move" to organize files into folders
3. Use "create_folder" to make new directories first
4. All paths must be absolute (start with /)

OUTPUT: You MUST respond with ONLY this JSON structure - nothing else:
{
  "description": "Brief summary of what this plan does",
  "operations": [
    { "type": "create_folder", "path": "/absolute/path/to/new/folder" },
    { "type": "move", "source": "/absolute/source/path", "destination": "/absolute/dest/path" }
  ]
}

OPERATION TYPES:
- create_folder: Create a new directory. Fields: path (string)
- move: Move a file/folder. Fields: source (string), destination (string)
- rename: Rename in place. Fields: path (string), newName (string)
- trash: Move to trash. Fields: path (string)

STRATEGY:
1. First create category folders (Documents, Images, Archives, Projects, etc.)
2. Then move files into appropriate folders based on extension and name patterns
3. Group related files together
4. Keep folder structure flat and simple (max 2 levels deep)

IMPORTANT: Output ONLY the JSON object. No markdown code blocks. No explanations. Just the raw JSON."#;

/// Build context prompt for folder organization (Claude Haiku for speed)
pub fn build_context_prompt(folder_path: &str, ls_output: &str) -> String {
    format!(
        r#"Analyze this folder structure and identify patterns:

FOLDER PATH: {}

DIRECTORY LISTING:
```
{}
```

Briefly describe:
1. What types of files are present
2. Any existing organizational patterns
3. Potential improvements"#,
        folder_path, ls_output
    )
}

/// System prompt for agentic folder organization with tool use (Claude Sonnet)
pub const AGENTIC_ORGANIZE_SYSTEM_PROMPT: &str = r#"You are a file organization assistant with access to shell commands.

PROCESS:
1. Use run_shell_command to explore the folder structure before planning
2. Analyze file types, naming patterns, and logical groupings
3. Return a JSON plan as your FINAL response

AVAILABLE TOOLS:
- run_shell_command: Run ls, grep, find, or cat to explore files
- edit_file: Write content to files (rarely needed for organization)

EXPLORATION COMMANDS (examples):
- ls -la /path/to/folder
- find /path -type f -name "*.pdf"
- grep -l "invoice" /path/*.txt
- cat /path/file.txt (for reading small files)

FINAL OUTPUT (after exploration):
Your LAST message must be ONLY a JSON object in this exact format:
{
  "description": "Brief summary of the organization plan",
  "operations": [
    { "type": "create_folder", "path": "/absolute/path/to/folder" },
    { "type": "move", "source": "/abs/source", "destination": "/abs/dest" },
    { "type": "rename", "path": "/abs/path", "newName": "new-name.ext" },
    { "type": "trash", "path": "/abs/path" }
  ]
}

RULES:
1. All paths must be absolute
2. Create folders before moving files into them
3. Never touch system directories (/System, /usr, /bin, /Applications)
4. Be conservative - don't over-organize
5. Your LAST message must be ONLY the JSON plan, no other text
"#;

/// Build organize prompt based on user request
pub fn build_organize_prompt(
    folder_path: &str,
    ls_output: &str,
    user_request: &str,
    context_analysis: Option<&str>,
) -> String {
    // Limit directory listing to prevent token overflow
    let truncated_ls = if ls_output.len() > 15000 {
        let lines: Vec<&str> = ls_output.lines().collect();
        let sample_size = 500.min(lines.len());
        let sampled: Vec<&str> = lines.iter().take(sample_size).copied().collect();
        format!(
            "{}\n\n... ({} more items, showing first {})",
            sampled.join("\n"),
            lines.len() - sample_size,
            sample_size
        )
    } else {
        ls_output.to_string()
    };

    let mut prompt = format!(
        r#"TARGET FOLDER: {}

FILES AND FOLDERS:
{}
"#,
        folder_path, truncated_ls
    );

    if let Some(context) = context_analysis {
        prompt.push_str(&format!("ANALYSIS: {}\n\n", context));
    }

    prompt.push_str(&format!(
        r#"REQUEST: {}

Generate the JSON plan now. Remember: output ONLY valid JSON, no other text."#,
        user_request
    ));

    prompt
}
