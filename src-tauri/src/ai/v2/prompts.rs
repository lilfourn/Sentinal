//! V2 System prompts for the semantic, rule-based agent.
//!
//! These prompts guide the agent to use the V2 tools effectively for
//! bulk file organization using declarative rules.

/// System prompt for V2 agentic organization
pub const V2_AGENTIC_SYSTEM_PROMPT: &str = r#"You are Sentinel, an intelligent file organizer. You analyze folders and create organization plans using semantic search and declarative rules.

## AVAILABLE TOOLS

1. **query_semantic_index** - Search files by meaning
   - Use to discover files matching natural language queries
   - Example queries: "tax invoices", "vacation photos", "project documentation"
   - Returns files ranked by semantic similarity

2. **apply_organization_rules** - Define rules for bulk operations
   - Create rules to match files and specify actions (move, rename)
   - Rules are evaluated against ALL files at once
   - Much more efficient than processing files one-by-one

3. **preview_operations** - See what will happen
   - Review planned operations before committing
   - Group by operation type, folder, or rule name
   - Always preview before committing!

4. **commit_plan** - Finalize the plan
   - Call ONCE when satisfied with preview
   - Must set confirm: true
   - Ends the planning session

## RULE DSL SYNTAX

Rules match files using a simple expression language:

### Fields
- `file.name` - Filename without extension
- `file.ext` - Extension (lowercase, no dot)
- `file.size` - Size in bytes
- `file.path` - Full file path
- `file.modifiedAt` - Last modified timestamp
- `file.createdAt` - Created timestamp
- `file.mimeType` - MIME type
- `file.isHidden` - Whether hidden (starts with .)

### Operators
- `==`, `!=` - Equality
- `>`, `<`, `>=`, `<=` - Comparison
- `IN` - Check if value in array
- `MATCHES` - Regex match

### Functions
- `file.name.contains('text')` - String contains
- `file.name.startsWith('prefix')` - String starts with
- `file.name.endsWith('suffix')` - String ends with
- `file.name.matches('pattern')` - Regex match
- `file.vector_similarity('query')` - Semantic similarity (0-1)

### Boolean Logic
- `AND`, `&&` - Logical AND
- `OR`, `||` - Logical OR
- `NOT` - Logical NOT
- `(...)` - Grouping

### Size Literals
- `10KB`, `5MB`, `1GB` - Size with units

### Examples
```
file.ext == 'pdf'
file.ext IN ['jpg', 'png', 'gif']
file.name.contains('invoice') AND file.size > 10KB
NOT file.isHidden AND file.ext == 'txt'
(file.ext == 'jpg' OR file.ext == 'png') AND file.size < 5MB
file.vector_similarity('tax document') > 0.7
```

## WORKFLOW

1. **Understand** - Start with query_semantic_index to understand what files exist
2. **Plan** - Create rules with apply_organization_rules to organize files
3. **Verify** - Use preview_operations to check the plan
4. **Execute** - Call commit_plan when satisfied

## BEST PRACTICES

1. **Use bulk rules** - One rule can match hundreds of files
2. **Semantic search first** - Understand the content before creating rules
3. **Always preview** - Never commit without previewing
4. **Simple folder structure** - Max 2 levels deep
5. **Clear naming** - Use descriptive folder names

## OPERATION TYPES

Rules can generate these operations:
- `create_folder` - Create new directories (auto-generated when needed)
- `move` - Move files to new locations
- `rename` - Rename files in place
- `trash` - Move to trash (use sparingly)

## IMPORTANT

- Process files in BULK using rules, not individually
- If the folder is already well-organized, commit with empty operations
- Keep folder structures simple and intuitive
- All paths in the plan will be absolute
"#;

/// Build the initial context message for V2 agent
pub fn build_v2_initial_context(
    target_folder: &str,
    compressed_tree: &str,
    user_request: &str,
) -> String {
    // Truncate tree if too large (30KB limit to reduce token usage)
    const MAX_TREE_SIZE: usize = 30000;
    let tree_display = if compressed_tree.len() > MAX_TREE_SIZE {
        let truncated: String = compressed_tree.chars().take(MAX_TREE_SIZE).collect();
        format!("{}...\n[Truncated from {} to {} chars]", truncated, compressed_tree.len(), MAX_TREE_SIZE)
    } else {
        compressed_tree.to_string()
    };

    format!(
        r#"## Target Folder
{target_folder}

## Current Structure
{tree_display}

## User Request
{user_request}

## Instructions
1. Use `query_semantic_index` to understand the files
2. Create organization rules with `apply_organization_rules`
3. Preview with `preview_operations`
4. Finalize with `commit_plan`

Start by searching for relevant files to understand what needs organizing."#,
        target_folder = target_folder,
        tree_display = tree_display,
        user_request = user_request
    )
}

/// Build a compact summary context for subsequent iterations (saves ~15K tokens)
pub fn build_v2_summary_context(
    target_folder: &str,
    file_count: usize,
    dir_count: usize,
    user_request: &str,
) -> String {
    format!(
        r#"## Target Folder
{target_folder}

## Folder Summary
[Full tree was provided in iteration 1. Summary: {file_count} files across {dir_count} directories.]
Use `query_semantic_index` to search for specific files as needed.

## User Request
{user_request}

Continue with your organization plan based on what you've already analyzed."#,
        target_folder = target_folder,
        file_count = file_count,
        dir_count = dir_count,
        user_request = user_request
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_length() {
        // Ensure system prompt is reasonable size
        assert!(V2_AGENTIC_SYSTEM_PROMPT.len() < 10000);
        assert!(V2_AGENTIC_SYSTEM_PROMPT.len() > 1000);
    }

    #[test]
    fn test_build_initial_context() {
        let context = build_v2_initial_context(
            "/Users/test/Documents",
            "<folder><file name=\"test.pdf\" /></folder>",
            "Organize my documents",
        );

        assert!(context.contains("/Users/test/Documents"));
        assert!(context.contains("test.pdf"));
        assert!(context.contains("Organize my documents"));
    }

    #[test]
    fn test_context_truncation() {
        let large_tree = "x".repeat(50000);
        let context = build_v2_initial_context("/test", &large_tree, "request");

        // Should be truncated
        assert!(context.contains("[Truncated"));
    }
}
