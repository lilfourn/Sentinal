use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::credentials::CredentialManager;
use super::tools::{ToolDefinition, ToolResult};
use crate::jobs::OrganizePlan;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Claude model identifiers
pub enum ClaudeModel {
    /// Claude 4.5 Haiku - fast, for context gathering
    Haiku,
    /// Claude 4.5 Sonnet - balanced, for rename and organize decisions
    Sonnet,
}

impl ClaudeModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClaudeModel::Haiku => "claude-haiku-4-5",
            ClaudeModel::Sonnet => "claude-sonnet-4-5",
        }
    }
}

/// Message content for API request
#[derive(Serialize)]
struct MessageContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

/// Message in conversation
#[derive(Serialize)]
struct Message {
    role: String,
    content: Vec<MessageContent>,
}

/// API request body
#[derive(Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

/// Content block in API response
#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

/// API response body
#[derive(Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
    #[allow(dead_code)]
    stop_reason: Option<String>,
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

// ============ Tool Use API Structures ============

/// API request with tools support
#[derive(Serialize)]
struct ToolApiRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ToolMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDefinition>>,
}

/// Message with tool support (can contain multiple content types)
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

    fn tool_result(result: ToolResult) -> Self {
        Self::ToolResult {
            content_type: "tool_result".to_string(),
            tool_use_id: result.tool_use_id,
            content: result.content,
            is_error: result.is_error,
        }
    }
}

/// Extended content block in response (text or tool_use)
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

/// Extended API response with stop_reason
#[derive(Deserialize, Debug)]
struct ToolApiResponse {
    content: Vec<ContentBlockResponse>,
    stop_reason: String,
}

/// Anthropic API client
pub struct AnthropicClient {
    client: Client,
}

impl AnthropicClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Send a message to Claude
    pub async fn send_message(
        &self,
        model: ClaudeModel,
        system_prompt: &str,
        user_message: &str,
        max_tokens: u32,
    ) -> Result<String, String> {
        let api_key = CredentialManager::get_api_key("anthropic")?;

        let request = ApiRequest {
            model: model.as_str().to_string(),
            max_tokens,
            system: system_prompt.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![MessageContent {
                    content_type: "text".to_string(),
                    text: user_message.to_string(),
                }],
            }],
        };

        let response = self
            .client
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

        let api_response: ApiResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Extract text from response
        let text = api_response
            .content
            .iter()
            .filter_map(|block| {
                if block.content_type == "text" {
                    block.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(text.trim().to_string())
    }

    /// Generate a rename suggestion using Claude Sonnet
    pub async fn suggest_rename(
        &self,
        filename: &str,
        extension: Option<&str>,
        size: u64,
        content_preview: Option<&str>,
    ) -> Result<String, String> {
        let user_prompt = super::prompts::build_rename_prompt(
            filename,
            extension,
            size,
            content_preview,
        );

        self.send_message(
            ClaudeModel::Sonnet,
            super::prompts::RENAME_SYSTEM_PROMPT,
            &user_prompt,
            100, // Short response expected
        )
        .await
    }

    /// Analyze folder context using Claude Haiku (fast)
    pub async fn analyze_folder_context(
        &self,
        folder_path: &str,
        ls_output: &str,
    ) -> Result<String, String> {
        let prompt = super::prompts::build_context_prompt(folder_path, ls_output);

        self.send_message(
            ClaudeModel::Haiku,
            "You are a file organization analyst. Be concise.",
            &prompt,
            500,
        )
        .await
    }

    /// Generate organization plan using Claude Sonnet
    pub async fn generate_organize_plan(
        &self,
        folder_path: &str,
        ls_output: &str,
        user_request: &str,
        context_analysis: Option<&str>,
    ) -> Result<String, String> {
        let prompt = super::prompts::build_organize_prompt(
            folder_path,
            ls_output,
            user_request,
            context_analysis,
        );

        eprintln!("[AI] Generating organize plan for: {}", folder_path);
        eprintln!("[AI] Prompt length: {} chars", prompt.len());

        let response = self.send_message(
            ClaudeModel::Sonnet,
            super::prompts::ORGANIZE_SYSTEM_PROMPT,
            &prompt,
            4096, // Increased for large folder operations
        )
        .await?;

        eprintln!("[AI] Response length: {} chars", response.len());
        eprintln!("[AI] Response preview: {}...", &response.chars().take(200).collect::<String>());

        Ok(response)
    }

    /// Validate API key by making a minimal request
    pub async fn validate_api_key(api_key: &str) -> Result<bool, String> {
        let client = Client::new();

        let request = ApiRequest {
            model: ClaudeModel::Haiku.as_str().to_string(),
            max_tokens: 10,
            system: "Say 'ok'".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![MessageContent {
                    content_type: "text".to_string(),
                    text: "test".to_string(),
                }],
            }],
        };

        let response = client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        Ok(response.status().is_success())
    }
}

impl Default for AnthropicClient {
    fn default() -> Self {
        Self::new()
    }
}
