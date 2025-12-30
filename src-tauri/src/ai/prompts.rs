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

/// System prompt for naming convention analysis (Claude Haiku for speed)
pub const NAMING_CONVENTION_SYSTEM_PROMPT: &str = r#"You are a file naming pattern analyst. Analyze files in a folder and suggest appropriate naming conventions.

TASK: Examine file names and suggest 3 naming conventions that would work well for organizing this folder.

ANALYSIS APPROACH:
1. Identify existing patterns (dates, prefixes, case styles)
2. Note inconsistencies in current naming
3. Consider file types and their typical naming needs
4. Look for semantic patterns (invoices, receipts, screenshots, etc.)

OUTPUT: Respond with ONLY valid JSON in this exact format:
{
  "totalFilesAnalyzed": <number>,
  "suggestions": [
    {
      "id": "conv-1",
      "name": "Human Readable Name",
      "description": "Brief description of how files would be named",
      "example": "example-filename.pdf",
      "pattern": "Pattern description for AI to follow when renaming",
      "confidence": 0.85,
      "matchingFiles": 12
    }
  ]
}

CONVENTION STYLES TO CONSIDER:
- kebab-case: lowercase-words-with-hyphens (invoice-apple-oct24.pdf)
- snake_case: lowercase_words_with_underscores (invoice_apple_oct24.pdf)
- Date prefixed: YYYY-MM-DD at start (2024-10-15-invoice-apple.pdf)
- Category prefixed: type-name at start (invoice-apple-oct24.pdf, receipt-amazon-dec24.pdf)
- Descriptive: clear descriptive names (apple-invoice-october-2024.pdf)

RULES:
1. Always suggest exactly 3 conventions
2. Order by confidence (highest first) - how well the convention matches existing files
3. At least one should match existing file patterns if any exist
4. Consider the file types present (documents, images, code, etc.)
5. Include realistic examples based on actual files in the folder
6. matchingFiles = count of existing files that already follow this pattern
7. confidence = 0.0-1.0 based on how well this convention fits the folder contents
"#;

/// Build user prompt for naming convention analysis
pub fn build_naming_convention_prompt(folder_path: &str, file_listing: &str) -> String {
    // Limit file listing to prevent token overflow
    let truncated_listing = if file_listing.len() > 8000 {
        let lines: Vec<&str> = file_listing.lines().collect();
        let sample_size = 200.min(lines.len());
        let sampled: Vec<&str> = lines.iter().take(sample_size).copied().collect();
        format!(
            "{}\n\n... ({} more files, showing first {})",
            sampled.join("\n"),
            lines.len() - sample_size,
            sample_size
        )
    } else {
        file_listing.to_string()
    };

    format!(
        r#"FOLDER: {}

FILE LISTING:
{}

Analyze these files and suggest 3 naming conventions. Output ONLY valid JSON."#,
        folder_path, truncated_listing
    )
}
