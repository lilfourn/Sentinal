use serde::{Deserialize, Serialize};

/// A suggested naming convention
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamingConvention {
    pub id: String,
    pub name: String,
    pub description: String,
    pub example: String,
    pub pattern: String,
    pub confidence: f64,
    pub matching_files: u32,
}

/// Response from naming convention analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamingConventionSuggestions {
    pub folder_path: String,
    pub total_files_analyzed: u32,
    pub suggestions: Vec<NamingConvention>,
}
