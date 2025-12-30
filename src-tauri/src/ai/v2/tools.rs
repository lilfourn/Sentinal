//! V2 tool definitions for the semantic, rule-based agent.
//!
//! Four new tools replace the shell-based approach:
//! 1. query_semantic_index - Search files by semantic similarity
//! 2. apply_organization_rules - Define rules for bulk file operations
//! 3. preview_operations - Preview planned changes
//! 4. commit_plan - Finalize and submit the plan

use crate::ai::tools::ToolDefinition;
use crate::jobs::OrganizePlan;

use super::vfs::{OperationType, OrganizationRule, ShadowVFS};
use serde_json::json;

/// Get V2 tool definitions for the agent
pub fn get_v2_organize_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "query_semantic_index".to_string(),
            description: r#"Search files by semantic similarity to find relevant files for organization.
Use this to understand what files exist before creating rules. Returns files ranked by relevance."#
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language query to search for (e.g., 'tax invoices', 'vacation photos', 'project documentation')"
                    },
                    "filter_ext": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional: Only include files with these extensions (e.g., ['pdf', 'docx'])"
                    },
                    "min_size_bytes": {
                        "type": "integer",
                        "description": "Optional: Minimum file size in bytes"
                    },
                    "max_results": {
                        "type": "integer",
                        "default": 20,
                        "description": "Maximum number of results to return (default: 20, max: 30)"
                    },
                    "min_similarity": {
                        "type": "number",
                        "default": 0.6,
                        "description": "Minimum similarity score 0.0-1.0 (default: 0.6)"
                    }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "apply_organization_rules".to_string(),
            description: r#"Apply organization rules to match files and generate operations in bulk.
Rules use a simple DSL to match files and specify actions.

Rule DSL examples:
- file.ext == 'pdf'
- file.ext IN ['jpg', 'png', 'gif']
- file.name.contains('invoice')
- file.size > 10MB
- file.vector_similarity('tax document') > 0.8
- NOT file.isHidden AND file.modifiedAt > '2024-01-01'
- (file.ext == 'jpg' OR file.ext == 'png') AND file.size < 5MB

Available fields: name, ext, size, path, modifiedAt, createdAt, mimeType, isHidden
Available operators: ==, !=, >, <, >=, <=, IN, MATCHES
Available functions: contains(), startsWith(), endsWith(), matches(), vector_similarity()"#
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "rules": {
                        "type": "array",
                        "description": "Organization rules to apply",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {
                                    "type": "string",
                                    "description": "Human-readable rule name"
                                },
                                "if": {
                                    "type": "string",
                                    "description": "Rule expression in DSL syntax"
                                },
                                "thenMoveTo": {
                                    "type": "string",
                                    "description": "Destination folder to move matching files to (relative to target folder or absolute)"
                                },
                                "thenRenameTo": {
                                    "type": "string",
                                    "description": "New name pattern. Supports: {name}, {ext}, {date}"
                                },
                                "priority": {
                                    "type": "integer",
                                    "description": "Rule priority (higher = earlier execution)"
                                }
                            },
                            "required": ["name", "if"]
                        }
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["append", "replace"],
                        "default": "append",
                        "description": "Whether to append to or replace existing operations (default: append)"
                    }
                },
                "required": ["rules"]
            }),
        },
        ToolDefinition {
            name: "preview_operations".to_string(),
            description: r#"Preview the planned operations before committing.
Use this to verify the plan looks correct before finalizing."#
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "group_by": {
                        "type": "string",
                        "enum": ["operation_type", "destination_folder", "source_folder", "rule_name"],
                        "default": "operation_type",
                        "description": "How to group the preview (default: operation_type)"
                    },
                    "include_unchanged": {
                        "type": "boolean",
                        "default": false,
                        "description": "Include count of files that won't be changed"
                    }
                }
            }),
        },
        ToolDefinition {
            name: "commit_plan".to_string(),
            description: r#"Finalize and submit the organization plan.
Call this ONCE when you're satisfied with the preview. This ends the planning session."#
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "Brief summary of what this organization plan does"
                    },
                    "confirm": {
                        "type": "boolean",
                        "description": "Must be true to commit the plan"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "default": false,
                        "description": "If true, return the plan without marking as final"
                    }
                },
                "required": ["description", "confirm"]
            }),
        },
    ]
}

/// Result of executing a V2 tool
pub enum V2ToolResult {
    /// Tool executed successfully, continue the loop
    Continue(String),
    /// Plan is ready to commit
    Commit(OrganizePlan),
    /// Tool execution failed
    Error(String),
}

/// Execute a V2 tool
pub fn execute_v2_tool(
    name: &str,
    input: &serde_json::Value,
    vfs: &mut ShadowVFS,
) -> V2ToolResult {
    match name {
        "query_semantic_index" => execute_query_semantic(input, vfs),
        "apply_organization_rules" => execute_apply_rules(input, vfs),
        "preview_operations" => execute_preview(input, vfs),
        "commit_plan" => execute_commit(input, vfs),
        _ => V2ToolResult::Error(format!("Unknown tool: {}", name)),
    }
}

fn execute_query_semantic(input: &serde_json::Value, vfs: &ShadowVFS) -> V2ToolResult {
    let query = match input.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return V2ToolResult::Error("Missing 'query' parameter".to_string()),
    };

    let filter_ext: Option<Vec<String>> = input
        .get("filter_ext")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

    let min_size_bytes = input
        .get("min_size_bytes")
        .and_then(|v| v.as_u64());

    let max_results = input
        .get("max_results")
        .and_then(|v| v.as_u64())
        .unwrap_or(20)
        .min(30) as usize;  // Cap at 30 to reduce token usage

    let min_similarity = input
        .get("min_similarity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.6) as f32;

    eprintln!(
        "[V2Tool] query_semantic_index: query='{}', max_results={}",
        query, max_results
    );

    let results = vfs.query_semantic(
        query,
        filter_ext.as_deref(),
        min_size_bytes,
        max_results,
        min_similarity,
    );

    if results.is_empty() {
        return V2ToolResult::Continue("No files found matching the query.".to_string());
    }

    // Format results
    let mut output = format!("Found {} matching files:\n\n", results.len());
    for (file, score) in &results {
        output.push_str(&format!(
            "- {} (ext: {}, size: {}, similarity: {:.2})\n",
            file.name,
            file.ext.as_deref().unwrap_or("none"),
            format_size(file.size),
            score
        ));
    }

    V2ToolResult::Continue(output)
}

fn execute_apply_rules(input: &serde_json::Value, vfs: &mut ShadowVFS) -> V2ToolResult {
    let rules_json = match input.get("rules").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return V2ToolResult::Error("Missing 'rules' array".to_string()),
    };

    let mode = input
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("append");

    // Parse rules from JSON
    let rules: Result<Vec<OrganizationRule>, _> = rules_json
        .iter()
        .map(|r| serde_json::from_value(r.clone()))
        .collect();

    let rules = match rules {
        Ok(r) => r,
        Err(e) => return V2ToolResult::Error(format!("Failed to parse rules: {}", e)),
    };

    eprintln!("[V2Tool] apply_organization_rules: {} rules, mode={}", rules.len(), mode);

    match vfs.apply_rules(&rules, mode) {
        Ok(count) => {
            let output = format!(
                "Applied {} rules, generated {} operations.\nTotal operations in plan: {}",
                rules.len(),
                count,
                vfs.operations().len()
            );
            V2ToolResult::Continue(output)
        }
        Err(e) => V2ToolResult::Error(format!("Failed to apply rules: {}", e)),
    }
}

fn execute_preview(input: &serde_json::Value, vfs: &ShadowVFS) -> V2ToolResult {
    let group_by = input
        .get("group_by")
        .and_then(|v| v.as_str())
        .unwrap_or("operation_type");

    let include_unchanged = input
        .get("include_unchanged")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    eprintln!("[V2Tool] preview_operations: group_by={}", group_by);

    let preview = vfs.preview_operations(group_by, include_unchanged);

    if preview.total_operations == 0 {
        return V2ToolResult::Continue("No operations planned. Use apply_organization_rules first.".to_string());
    }

    let mut output = format!(
        "Operation Preview (grouped by {})\n",
        group_by
    );
    output.push_str(&format!("Total operations: {}\n", preview.total_operations));

    if include_unchanged {
        output.push_str(&format!("Unchanged files: {}\n", preview.unchanged_files));
    }

    output.push('\n');

    // Sort groups for consistent output
    let mut sorted_groups: Vec<_> = preview.groups.iter().collect();
    sorted_groups.sort_by_key(|(k, _)| k.as_str());

    for (group_name, ops) in sorted_groups {
        output.push_str(&format!("## {} ({} operations)\n", group_name, ops.len()));

        for op in ops.iter().take(10) {
            // Limit preview per group
            match op.op_type {
                OperationType::CreateFolder => {
                    output.push_str(&format!(
                        "  - CREATE FOLDER: {}\n",
                        op.path.as_deref().unwrap_or("?")
                    ));
                }
                OperationType::Move => {
                    output.push_str(&format!(
                        "  - MOVE: {} -> {}\n",
                        op.source.as_deref().unwrap_or("?"),
                        op.destination.as_deref().unwrap_or("?")
                    ));
                }
                OperationType::Rename => {
                    output.push_str(&format!(
                        "  - RENAME: {} -> {}\n",
                        op.path.as_deref().unwrap_or("?"),
                        op.new_name.as_deref().unwrap_or("?")
                    ));
                }
                OperationType::Trash => {
                    output.push_str(&format!(
                        "  - TRASH: {}\n",
                        op.path.as_deref().unwrap_or("?")
                    ));
                }
            }
        }

        if ops.len() > 10 {
            output.push_str(&format!("  ... and {} more\n", ops.len() - 10));
        }

        output.push('\n');
    }

    // Truncate output if too large to prevent context overflow (4KB max to save tokens)
    const MAX_PREVIEW_SIZE: usize = 4000;
    if output.len() > MAX_PREVIEW_SIZE {
        // Count operation types for summary
        let mut creates = 0;
        let mut moves = 0;
        let mut renames = 0;
        let mut trashes = 0;
        for ops in preview.groups.values() {
            for op in ops {
                match op.op_type {
                    OperationType::CreateFolder => creates += 1,
                    OperationType::Move => moves += 1,
                    OperationType::Rename => renames += 1,
                    OperationType::Trash => trashes += 1,
                }
            }
        }
        let truncated = format!(
            "{}...\n\n[Preview truncated]\nSummary: {} total operations ({} creates, {} moves, {} renames, {} deletes) across {} folders\n",
            &output[..MAX_PREVIEW_SIZE.min(output.len())],
            preview.total_operations,
            creates, moves, renames, trashes,
            preview.groups.len()
        );
        return V2ToolResult::Continue(truncated);
    }

    V2ToolResult::Continue(output)
}

fn execute_commit(input: &serde_json::Value, vfs: &ShadowVFS) -> V2ToolResult {
    let description = match input.get("description").and_then(|v| v.as_str()) {
        Some(d) => d,
        None => return V2ToolResult::Error("Missing 'description' parameter".to_string()),
    };

    let confirm = input
        .get("confirm")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let dry_run = input
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !confirm {
        return V2ToolResult::Error(
            "Must set 'confirm: true' to commit the plan".to_string(),
        );
    }

    eprintln!(
        "[V2Tool] commit_plan: description='{}', dry_run={}",
        description, dry_run
    );

    let operations = vfs.operations();

    if operations.is_empty() {
        // Return an empty plan - folder is already organized
        return V2ToolResult::Commit(OrganizePlan {
            plan_id: format!("plan-{}", chrono::Utc::now().timestamp_millis()),
            description: description.to_string(),
            operations: Vec::new(),
            target_folder: vfs.root().to_string_lossy().to_string(),
        });
    }

    // Convert to OrganizeOperation format
    let organize_ops: Vec<crate::jobs::OrganizeOperation> = operations
        .iter()
        .map(|op| crate::jobs::OrganizeOperation {
            op_id: op.op_id.clone(),
            op_type: op.op_type.to_string(),
            source: op.source.clone(),
            destination: op.destination.clone(),
            path: op.path.clone(),
            new_name: op.new_name.clone(),
        })
        .collect();

    let plan = OrganizePlan {
        plan_id: format!("plan-{}", chrono::Utc::now().timestamp_millis()),
        description: description.to_string(),
        operations: organize_ops,
        target_folder: vfs.root().to_string_lossy().to_string(),
    };

    if dry_run {
        // Return as a preview
        let output = format!(
            "Dry run - plan would contain {} operations:\n{}",
            plan.operations.len(),
            serde_json::to_string_pretty(&plan).unwrap_or_default()
        );
        V2ToolResult::Continue(output)
    } else {
        V2ToolResult::Commit(plan)
    }
}

/// Format file size for display
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definitions() {
        let tools = get_v2_organize_tools();
        assert_eq!(tools.len(), 4);

        let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"query_semantic_index"));
        assert!(names.contains(&"apply_organization_rules"));
        assert!(names.contains(&"preview_operations"));
        assert!(names.contains(&"commit_plan"));
    }
}
