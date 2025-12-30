//! V2 Agentic loop for semantic, rule-based file organization.
//!
//! This module implements the main agent loop that:
//! 1. Builds a ShadowVFS from the target folder
//! 2. Generates a compressed tree context
//! 3. Runs the conversation loop with Claude using V2 tools
//! 4. Returns the finalized OrganizePlan

use crate::ai::client::ClaudeModel;
use crate::ai::credentials::CredentialManager;
use crate::jobs::OrganizePlan;

use super::prompts::{build_v2_initial_context, build_v2_summary_context, V2_AGENTIC_SYSTEM_PROMPT};
use super::tools::{execute_v2_tool, get_v2_organize_tools, V2ToolResult};
use super::vfs::ShadowVFS;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Maximum iterations before giving up
const MAX_ITERATIONS: usize = 10;

/// Maximum tokens for response
const MAX_TOKENS: u32 = 8192;

/// API request with tools support
#[derive(Serialize)]
struct ToolApiRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ToolMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<crate::ai::tools::ToolDefinition>>,
}

/// Message with tool support
#[derive(Serialize, Clone)]
struct ToolMessage {
    role: String,
    content: Vec<ToolMessageContent>,
}

/// Content block for tool messages
#[derive(Serialize, Clone)]
#[serde(untagged)]
enum ToolMessageContent {
    Text {
        #[serde(rename = "type")]
        content_type: String,
        text: String,
    },
    ToolUse {
        #[serde(rename = "type")]
        content_type: String,
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        #[serde(rename = "type")]
        content_type: String,
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

impl ToolMessageContent {
    fn text(text: &str) -> Self {
        Self::Text {
            content_type: "text".to_string(),
            text: text.to_string(),
        }
    }

    fn tool_use(id: &str, name: &str, input: &serde_json::Value) -> Self {
        Self::ToolUse {
            content_type: "tool_use".to_string(),
            id: id.to_string(),
            name: name.to_string(),
            input: input.clone(),
        }
    }

    fn tool_result(tool_use_id: &str, content: &str, is_error: bool) -> Self {
        Self::ToolResult {
            content_type: "tool_result".to_string(),
            tool_use_id: tool_use_id.to_string(),
            content: content.to_string(),
            is_error: if is_error { Some(true) } else { None },
        }
    }
}

/// Content block in response
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ContentBlockResponse {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

/// API response
#[derive(Deserialize, Debug)]
struct ToolApiResponse {
    content: Vec<ContentBlockResponse>,
    stop_reason: String,
}

/// API error response
#[derive(Deserialize)]
struct ApiError {
    error: ApiErrorDetail,
}

#[derive(Deserialize)]
struct ApiErrorDetail {
    message: String,
}

/// Event types emitted during the agent loop
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent is indexing files
    Indexing(String),
    /// Agent is searching files
    Searching(String),
    /// Agent is applying rules
    ApplyingRules(String),
    /// Agent is previewing operations
    Previewing(String),
    /// Agent is committing the plan
    Committing(String),
    /// Agent is thinking (text output)
    Thinking(String),
    /// Agent encountered an error
    Error(String),
}

/// Run the V2 agentic organize workflow
///
/// This function:
/// 1. Builds a ShadowVFS from the target folder
/// 2. Generates a compressed tree for context
/// 3. Runs the conversation loop with V2 tools
/// 4. Returns the final OrganizePlan
pub async fn run_v2_agentic_organize<F>(
    target_folder: &Path,
    user_request: &str,
    event_emitter: F,
) -> Result<OrganizePlan, String>
where
    F: Fn(&str, &str),
{
    // 1. Build ShadowVFS from target folder
    event_emitter("indexing", "Scanning folder structure...");
    eprintln!("[V2AgentLoop] Building VFS for: {}", target_folder.display());

    let mut vfs = ShadowVFS::new(target_folder).map_err(|e| {
        format!("Failed to scan folder: {}", e)
    })?;

    let file_count = vfs.files().len();
    event_emitter("indexing", &format!("Found {} files", file_count));

    // 2. Generate compressed tree for context
    let compressed_tree = vfs.generate_compressed_tree();
    eprintln!(
        "[V2AgentLoop] Generated tree context: {} chars",
        compressed_tree.len()
    );

    // 3. Build initial context message
    let initial_context = build_v2_initial_context(
        &target_folder.to_string_lossy(),
        &compressed_tree,
        user_request,
    );

    // 4. Initialize conversation
    let tools = get_v2_organize_tools();
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_key = CredentialManager::get_api_key("anthropic")?;

    let mut messages = vec![ToolMessage {
        role: "user".to_string(),
        content: vec![ToolMessageContent::text(&initial_context)],
    }];

    // 5. Agentic loop
    for iteration in 0..MAX_ITERATIONS {
        eprintln!("[V2AgentLoop] Iteration {}", iteration + 1);

        // After first iteration, replace full tree context with compact summary
        // This saves ~15,000 tokens per request (from 60KB tree to 500 byte summary)
        if iteration == 1 {
            let summary_context = build_v2_summary_context(
                &target_folder.to_string_lossy(),
                vfs.files().len(),
                vfs.directory_count(),
                user_request,
            );
            messages[0] = ToolMessage {
                role: "user".to_string(),
                content: vec![ToolMessageContent::text(&summary_context)],
            };
            eprintln!("[V2AgentLoop] Replaced tree context with summary ({} chars)", summary_context.len());
        }

        // Prune old messages to prevent context overflow (keep initial + last N)
        const MAX_MESSAGES: usize = 7; // Initial message + 3 roundtrips (6 messages)
        if messages.len() > MAX_MESSAGES {
            let initial_message = messages.remove(0);
            let to_remove = messages.len() - (MAX_MESSAGES - 1);
            messages.drain(0..to_remove);
            messages.insert(0, initial_message);
            eprintln!("[V2AgentLoop] Pruned messages: kept {} of {} total", messages.len(), messages.len() + to_remove);
        }

        // Send request to Claude
        let request = ToolApiRequest {
            model: ClaudeModel::Sonnet.as_str().to_string(),
            max_tokens: MAX_TOKENS,
            system: V2_AGENTIC_SYSTEM_PROMPT.to_string(),
            messages: messages.clone(),
            tools: Some(tools.clone()),
        };

        let response = client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            if let Ok(api_error) = serde_json::from_str::<ApiError>(&error_text) {
                return Err(format!("API error: {}", api_error.error.message));
            }
            return Err(format!("API error ({}): {}", status, error_text));
        }

        let api_response: ToolApiResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        eprintln!("[V2AgentLoop] stop_reason: {}", api_response.stop_reason);

        // Process response content
        let mut assistant_content: Vec<ToolMessageContent> = Vec::new();
        let mut tool_results: Vec<ToolMessageContent> = Vec::new();

        for block in &api_response.content {
            match block {
                ContentBlockResponse::Text { text } => {
                    if !text.trim().is_empty() {
                        let preview: String = text.chars().take(200).collect();
                        eprintln!("[V2AgentLoop] Thinking: {}...", &preview);

                        if preview.len() > 20 {
                            event_emitter("thinking", &preview);
                        }
                    }
                    assistant_content.push(ToolMessageContent::text(text));
                }

                ContentBlockResponse::ToolUse { id, name, input } => {
                    eprintln!("[V2AgentLoop] Tool use: {}", name);
                    assistant_content.push(ToolMessageContent::tool_use(id, name, input));

                    // Emit appropriate event based on tool name
                    let _event_type = match name.as_str() {
                        "query_semantic_index" => {
                            let query = input.get("query").and_then(|v| v.as_str()).unwrap_or("files");
                            event_emitter("searching", &format!("Searching for '{}'", query));
                            "searching"
                        }
                        "apply_organization_rules" => {
                            let count = input.get("rules").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
                            event_emitter("applying_rules", &format!("Applying {} rules", count));
                            "applying_rules"
                        }
                        "preview_operations" => {
                            event_emitter("previewing", "Generating preview...");
                            "previewing"
                        }
                        "commit_plan" => {
                            event_emitter("committing", "Finalizing plan...");
                            "committing"
                        }
                        _ => "executing"
                    };

                    // Execute the tool
                    let result = execute_v2_tool(name, input, &mut vfs);

                    match result {
                        V2ToolResult::Continue(output) => {
                            eprintln!("[V2AgentLoop] Tool success: {} bytes", output.len());
                            tool_results.push(ToolMessageContent::tool_result(
                                id,
                                &output,
                                false,
                            ));
                        }
                        V2ToolResult::Commit(plan) => {
                            eprintln!(
                                "[V2AgentLoop] Plan committed: {} operations",
                                plan.operations.len()
                            );
                            event_emitter(
                                "committing",
                                &format!("Plan created with {} operations", plan.operations.len()),
                            );
                            return Ok(plan);
                        }
                        V2ToolResult::Error(err) => {
                            let context = format!(
                                "Tool error (files: {}, ops: {}): {}",
                                vfs.files().len(),
                                vfs.operations().len(),
                                err
                            );
                            eprintln!("[V2AgentLoop] {}", context);
                            event_emitter("error", &context);
                            tool_results.push(ToolMessageContent::tool_result(
                                id,
                                &context,
                                true,
                            ));
                        }
                    }
                }
            }
        }

        // Check if we should end
        if api_response.stop_reason == "end_turn" && tool_results.is_empty() {
            // Agent finished without committing - try to commit what we have
            if !vfs.operations().is_empty() {
                eprintln!("[V2AgentLoop] Auto-committing {} operations", vfs.operations().len());
                let plan = OrganizePlan {
                    plan_id: format!("plan-{}", chrono::Utc::now().timestamp_millis()),
                    description: "Auto-generated organization plan".to_string(),
                    operations: vfs
                        .operations()
                        .iter()
                        .map(|op| crate::jobs::OrganizeOperation {
                            op_id: op.op_id.clone(),
                            op_type: op.op_type.to_string(),
                            source: op.source.clone(),
                            destination: op.destination.clone(),
                            path: op.path.clone(),
                            new_name: op.new_name.clone(),
                        })
                        .collect(),
                    target_folder: target_folder.to_string_lossy().to_string(),
                };
                return Ok(plan);
            }

            return Err(format!(
                "Agent finished after searching {} files but created no operations. {}",
                vfs.files().len(),
                if vfs.operations().is_empty() {
                    "The folder may already be well-organized, or the files didn't match any organization rules."
                } else {
                    "Try organizing with different rules or a smaller subfolder."
                }
            ));
        }

        // Add assistant message
        messages.push(ToolMessage {
            role: "assistant".to_string(),
            content: assistant_content,
        });

        // Add tool results if any
        if !tool_results.is_empty() {
            messages.push(ToolMessage {
                role: "user".to_string(),
                content: tool_results,
            });
        }
    }

    Err("Organization took too long. Please try with a smaller folder or simpler request.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // Note: Full integration tests require API key and network access
    // These tests verify the module structure and basic functionality

    #[test]
    fn test_tool_message_content() {
        let text = ToolMessageContent::text("Hello");
        assert!(matches!(text, ToolMessageContent::Text { .. }));

        let tool_use = ToolMessageContent::tool_use(
            "123",
            "test_tool",
            &serde_json::json!({"key": "value"}),
        );
        assert!(matches!(tool_use, ToolMessageContent::ToolUse { .. }));

        let result = ToolMessageContent::tool_result("123", "success", false);
        assert!(matches!(result, ToolMessageContent::ToolResult { .. }));
    }
}
