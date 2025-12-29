use serde::{Deserialize, Serialize};
use serde_json::json;

/// Tool definition for Anthropic API
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Tool use request from Claude (content block)
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

/// Tool result to send back to Claude
#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    #[serde(rename = "type")]
    pub content_type: String,
    pub tool_use_id: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolResult {
    pub fn success(tool_use_id: String, content: String) -> Self {
        Self {
            content_type: "tool_result".to_string(),
            tool_use_id,
            content,
            is_error: None,
        }
    }

    pub fn error(tool_use_id: String, error: String) -> Self {
        Self {
            content_type: "tool_result".to_string(),
            tool_use_id,
            content: error,
            is_error: Some(true),
        }
    }
}

/// Get the tools available for folder organization
pub fn get_organize_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "run_shell_command".to_string(),
            description: "Execute a read-only shell command to explore folder structure. ONLY ls, grep, find, and cat are allowed. Use this to understand the files before creating an organization plan.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to run. Only ls, grep, find, or cat are allowed. Examples: 'ls -la', 'find . -type f -name \"*.pdf\"', 'grep -l \"invoice\" *.txt'"
                    },
                    "working_directory": {
                        "type": "string",
                        "description": "Optional: Directory to run the command in. Defaults to target folder."
                    }
                },
                "required": ["command"]
            }),
        },
        ToolDefinition {
            name: "submit_plan".to_string(),
            description: "Submit the final organization plan. Call this tool ONCE when you are done exploring and ready to submit your plan. This ends the conversation.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "Brief summary of what this organization plan does"
                    },
                    "operations": {
                        "type": "array",
                        "description": "List of operations to perform",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": {
                                    "type": "string",
                                    "enum": ["create_folder", "move", "rename", "trash"],
                                    "description": "Operation type"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Absolute path (for create_folder, rename, trash)"
                                },
                                "source": {
                                    "type": "string",
                                    "description": "Source path (for move)"
                                },
                                "destination": {
                                    "type": "string",
                                    "description": "Destination path (for move)"
                                },
                                "newName": {
                                    "type": "string",
                                    "description": "New filename (for rename)"
                                }
                            },
                            "required": ["type"]
                        }
                    }
                },
                "required": ["description", "operations"]
            }),
        },
    ]
}
