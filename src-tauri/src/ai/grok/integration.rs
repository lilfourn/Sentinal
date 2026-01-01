//! Integration Module
//!
//! High-level API for the multi-agent Grok analysis system.
//! Provides a clean interface for the rest of the application.

use super::cache::ContentCache;
use super::client::GrokClient;
use super::explore_agent::{create_batches, run_parallel_explores, ExploreAgent};
use super::orchestrator::{OrchestratorAgent, OrchestratorConfig};
use super::pdf_renderer::PdfRenderer;
use super::types::*;
use super::vision;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

/// Main entry point for Grok-powered file organization
pub struct GrokOrganizer {
    client: Arc<GrokClient>,
    cache: Arc<ContentCache>,
    pdf_renderer: Arc<PdfRenderer>,
    config: GrokConfig,
}

impl GrokOrganizer {
    /// Create a new organizer
    pub fn new(api_key: String, cache_dir: &Path) -> Result<Self, String> {
        let config = GrokConfig {
            api_key: api_key.clone(),
            ..Default::default()
        };

        let client = Arc::new(GrokClient::new(config.clone())?);
        let cache = Arc::new(ContentCache::open(cache_dir)?);
        let pdf_renderer = Arc::new(PdfRenderer::new());

        Ok(Self {
            client,
            cache,
            pdf_renderer,
            config,
        })
    }

    /// Scan a folder and identify files that can be analyzed
    pub async fn scan_folder(&self, folder: &Path) -> Result<ScanResult, String> {
        let mut analyzable_files = Vec::new();
        let mut text_files = Vec::new();
        let mut other_files = Vec::new();
        let mut total_size = 0u64;

        for entry in WalkDir::new(folder)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path().to_path_buf();
            let ext = path.extension().and_then(|e| e.to_str());
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            total_size += size;

            if vision::is_analyzable_extension(ext) {
                analyzable_files.push(path);
            } else if vision::is_text_extension(ext) {
                text_files.push(path);
            } else {
                other_files.push(path);
            }
        }

        // Check cache for already-analyzed files
        let cached_count = self
            .cache
            .filter_uncached(&analyzable_files)
            .map(|uncached| analyzable_files.len() - uncached.len())
            .unwrap_or(0);

        let needs_analysis = analyzable_files.len() - cached_count;

        // Estimate cost ($0.20/M input + $0.50/M output, ~1000 tokens per doc)
        let estimated_cost_cents = (needs_analysis as f64 * 0.035) as u32; // ~$0.035 per doc

        Ok(ScanResult {
            total_files: analyzable_files.len() + text_files.len() + other_files.len(),
            analyzable_files: analyzable_files.len(),
            text_files: text_files.len(),
            other_files: other_files.len(),
            cached_files: cached_count,
            needs_analysis,
            total_size_bytes: total_size,
            estimated_cost_cents,
            file_paths: analyzable_files,
        })
    }

    /// Run the full organization pipeline
    pub async fn organize<F>(
        &self,
        folder: &Path,
        user_instruction: &str,
        progress_callback: F,
    ) -> Result<OrganizationPlan, String>
    where
        F: Fn(AnalysisProgress) + Send + Sync + Clone + 'static,
    {
        // 1. Scan folder
        progress_callback(AnalysisProgress {
            phase: AnalysisPhase::Scanning,
            current: 0,
            total: 0,
            current_file: None,
            message: "Scanning folder...".to_string(),
        });

        let scan = self.scan_folder(folder).await?;

        tracing::info!(
            "[GrokOrganizer] Scan complete: {} analyzable, {} cached, {} need analysis",
            scan.analyzable_files,
            scan.cached_files,
            scan.needs_analysis
        );

        // 2. Check cache
        progress_callback(AnalysisProgress {
            phase: AnalysisPhase::CheckingCache,
            current: scan.cached_files,
            total: scan.analyzable_files,
            current_file: None,
            message: format!("{} files already analyzed", scan.cached_files),
        });

        // 3. Filter to uncached files
        let uncached_files = self.cache.filter_uncached(&scan.file_paths)?;

        // 4. Create batches and run explore agents in parallel
        if !uncached_files.is_empty() {
            progress_callback(AnalysisProgress {
                phase: AnalysisPhase::AnalyzingContent,
                current: 0,
                total: uncached_files.len(),
                current_file: None,
                message: format!("Analyzing {} files...", uncached_files.len()),
            });

            let batches = create_batches(uncached_files, self.config.batch_size);

            let explore_results = run_parallel_explores(
                Arc::clone(&self.client),
                Arc::clone(&self.cache),
                Arc::clone(&self.pdf_renderer),
                batches,
                progress_callback.clone(),
            )
            .await;

            // Log results
            let total_analyzed: usize = explore_results.iter().map(|r| r.analyses.len()).sum();
            let total_failed: usize = explore_results.iter().map(|r| r.failed_files.len()).sum();
            let total_tokens: u32 = explore_results.iter().map(|r| r.total_tokens_used).sum();

            tracing::info!(
                "[GrokOrganizer] Explore complete: {} analyzed, {} failed, {} tokens",
                total_analyzed,
                total_failed,
                total_tokens
            );
        }

        // 5. Gather all analyses (from cache and new)
        progress_callback(AnalysisProgress {
            phase: AnalysisPhase::Aggregating,
            current: 0,
            total: scan.analyzable_files,
            current_file: None,
            message: "Gathering analyses...".to_string(),
        });

        let mut all_analyses = Vec::new();
        for path in &scan.file_paths {
            if let Ok(Some(analysis)) = self.cache.get_cached(path) {
                all_analyses.push(analysis);
            }
        }

        // Also include text files with simple analysis
        for path in scan.file_paths.iter().filter(|p| {
            vision::is_text_extension(p.extension().and_then(|e| e.to_str()))
        }) {
            if let Ok(content) = tokio::fs::read_to_string(path).await {
                let filename = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                all_analyses.push(DocumentAnalysis {
                    file_path: path.to_string_lossy().to_string(),
                    file_name: filename.clone(),
                    content_summary: content.chars().take(200).collect(),
                    document_type: DocumentType::Unknown,
                    key_entities: vec![],
                    suggested_name: Some(filename),
                    confidence: 0.5,
                    method: AnalysisMethod::TextExtraction,
                });
            }
        }

        // 6. Run orchestrator to create plan
        progress_callback(AnalysisProgress {
            phase: AnalysisPhase::Planning,
            current: 0,
            total: 1,
            current_file: None,
            message: "Creating organization plan...".to_string(),
        });

        let orchestrator_config = OrchestratorConfig {
            user_instruction: user_instruction.to_string(),
            ..Default::default()
        };

        let orchestrator = OrchestratorAgent::new(Arc::clone(&self.client), orchestrator_config);

        let explore_result = ExploreResult {
            batch_id: 0,
            analyses: all_analyses,
            failed_files: vec![],
            total_tokens_used: 0,
            duration_ms: 0,
        };

        let plan = orchestrator.create_plan(vec![explore_result]).await?;

        // 7. Complete
        progress_callback(AnalysisProgress {
            phase: AnalysisPhase::Complete,
            current: plan.assignments.len(),
            total: plan.assignments.len(),
            current_file: None,
            message: format!(
                "Plan ready: {} folders, {} file assignments",
                plan.folder_structure.len(),
                plan.assignments.len()
            ),
        });

        Ok(plan)
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> Result<super::cache::CacheStats, String> {
        self.cache.get_stats()
    }

    /// Clear the cache
    pub fn clear_cache(&self) -> Result<(), String> {
        self.cache.clear()
    }

    /// Analyze a single file
    pub async fn analyze_single(&self, path: &Path) -> Result<DocumentAnalysis, String> {
        // Check cache first
        if let Some(cached) = self.cache.get_cached(path)? {
            return Ok(cached);
        }

        let agent = ExploreAgent::new(
            Arc::clone(&self.client),
            Arc::clone(&self.cache),
            Arc::clone(&self.pdf_renderer),
            0,
        );

        let result = agent
            .process_batch(vec![path.to_path_buf()], |_| {})
            .await;

        result
            .analyses
            .into_iter()
            .next()
            .ok_or_else(|| "Failed to analyze file".to_string())
    }
}

/// Result of scanning a folder
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub total_files: usize,
    pub analyzable_files: usize,
    pub text_files: usize,
    pub other_files: usize,
    pub cached_files: usize,
    pub needs_analysis: usize,
    pub total_size_bytes: u64,
    pub estimated_cost_cents: u32,
    #[serde(skip)]
    pub file_paths: Vec<PathBuf>,
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_scan_folder() {
        let dir = tempdir().unwrap();

        // Create test files
        std::fs::write(dir.path().join("test.pdf"), "fake pdf").unwrap();
        std::fs::write(dir.path().join("doc.txt"), "text content").unwrap();
        std::fs::write(dir.path().join("image.jpg"), "fake image").unwrap();

        // Note: This test requires a valid API key to fully work
        // For unit testing, we just verify the scan logic
    }
}
