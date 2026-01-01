//! Shared types for Grok multi-agent system

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Result of analyzing a single document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentAnalysis {
    /// Original file path
    pub file_path: String,

    /// Original filename
    pub file_name: String,

    /// Concise summary of document content (1-2 sentences)
    pub content_summary: String,

    /// Document type classification
    pub document_type: DocumentType,

    /// Key entities extracted (people, companies, dates, amounts)
    pub key_entities: Vec<String>,

    /// AI-suggested descriptive filename (without extension)
    pub suggested_name: Option<String>,

    /// Confidence score (0.0-1.0)
    pub confidence: f32,

    /// Analysis method used
    pub method: AnalysisMethod,
}

/// Document type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    Invoice,
    Contract,
    Report,
    Letter,
    Form,
    Receipt,
    Statement,
    Proposal,
    Presentation,
    Spreadsheet,
    Manual,
    Certificate,
    License,
    Permit,
    Application,
    Resume,
    Photo,
    Diagram,
    Drawing,
    Unknown,
}

impl DocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Invoice => "invoice",
            Self::Contract => "contract",
            Self::Report => "report",
            Self::Letter => "letter",
            Self::Form => "form",
            Self::Receipt => "receipt",
            Self::Statement => "statement",
            Self::Proposal => "proposal",
            Self::Presentation => "presentation",
            Self::Spreadsheet => "spreadsheet",
            Self::Manual => "manual",
            Self::Certificate => "certificate",
            Self::License => "license",
            Self::Permit => "permit",
            Self::Application => "application",
            Self::Resume => "resume",
            Self::Photo => "photo",
            Self::Diagram => "diagram",
            Self::Drawing => "drawing",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "invoice" => Self::Invoice,
            "contract" => Self::Contract,
            "report" => Self::Report,
            "letter" => Self::Letter,
            "form" => Self::Form,
            "receipt" => Self::Receipt,
            "statement" => Self::Statement,
            "proposal" => Self::Proposal,
            "presentation" => Self::Presentation,
            "spreadsheet" => Self::Spreadsheet,
            "manual" => Self::Manual,
            "certificate" => Self::Certificate,
            "license" => Self::License,
            "permit" => Self::Permit,
            "application" => Self::Application,
            "resume" | "cv" => Self::Resume,
            "photo" | "image" => Self::Photo,
            "diagram" => Self::Diagram,
            "drawing" => Self::Drawing,
            _ => Self::Unknown,
        }
    }
}

/// How the document was analyzed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisMethod {
    /// Retrieved from persistent cache
    Cached,
    /// Text extracted directly from file
    TextExtraction,
    /// Analyzed via Grok Vision API
    GrokVision,
    /// OCR fallback
    Ocr,
    /// Metadata only (filename, extension, size)
    MetadataOnly,
}

/// Summary format for passing between explore agents and orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FileSummary {
    pub file_name: String,
    pub content_summary: String,
    pub suggested_name: String,
}

impl From<&DocumentAnalysis> for FileSummary {
    fn from(analysis: &DocumentAnalysis) -> Self {
        Self {
            file_name: analysis.file_name.clone(),
            content_summary: analysis.content_summary.clone(),
            suggested_name: analysis.suggested_name.clone().unwrap_or_else(|| analysis.file_name.clone()),
        }
    }
}

/// Batch of files for an explore agent
#[derive(Debug, Clone)]
pub struct ExploreBatch {
    pub batch_id: usize,
    pub files: Vec<PathBuf>,
}

/// Result from an explore agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExploreResult {
    pub batch_id: usize,
    pub analyses: Vec<DocumentAnalysis>,
    pub failed_files: Vec<(String, String)>, // (path, error)
    pub total_tokens_used: u32,
    pub duration_ms: u64,
}

/// Folder assignment from orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderAssignment {
    pub file_path: String,
    pub original_name: String,
    pub destination_folder: String,
    pub new_name: Option<String>,
    pub confidence: f32,
}

/// Complete organization plan from orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPlan {
    /// Detected file domain (e.g., "Software developer workspace", "Business records")
    #[serde(default)]
    pub detected_domain: Option<String>,
    /// Key entities extracted from content (company names, project names, etc.)
    #[serde(default)]
    pub key_entities_found: Vec<String>,
    pub strategy_name: String,
    pub description: String,
    pub folder_structure: Vec<PlannedFolder>,
    pub assignments: Vec<FolderAssignment>,
    pub unassigned_files: Vec<String>,
}

/// A folder in the planned structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedFolder {
    pub path: String,
    pub description: String,
    pub expected_file_count: usize,
}

/// Configuration for the multi-agent system
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GrokConfig {
    /// API key for xAI
    pub api_key: String,

    /// Base URL for API (default: https://api.x.ai)
    pub base_url: String,

    /// Model to use (default: grok-4-1-fast)
    pub model: String,

    /// Maximum concurrent explore agents
    pub max_parallel_agents: usize,

    /// Files per explore agent batch
    pub batch_size: usize,

    /// Maximum cost in cents per job
    pub budget_cents: u32,

    /// Rate limit: requests per second
    pub requests_per_second: f32,

    /// Rate limit: max concurrent requests
    pub max_concurrent_requests: usize,
}

impl Default for GrokConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.x.ai".to_string(),
            model: "grok-4-1-fast".to_string(),
            max_parallel_agents: 4,
            batch_size: 50,
            budget_cents: 100, // $1 default budget
            requests_per_second: 5.0,
            max_concurrent_requests: 10,
        }
    }
}

/// Progress event for UI updates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisProgress {
    pub phase: AnalysisPhase,
    pub current: usize,
    pub total: usize,
    pub current_file: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisPhase {
    Scanning,
    CheckingCache,
    RenderingPdf,
    AnalyzingContent,
    Aggregating,
    Planning,
    Complete,
    Failed,
}
