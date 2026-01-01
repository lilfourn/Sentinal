//! Grok AI Integration Module
//!
//! Multi-agent architecture using Grok's 2M context window:
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     ORCHESTRATOR AGENT                          │
//! │  - Receives file summaries from explore agents                  │
//! │  - Uses 2M context to hold all summaries at once                │
//! │  - Creates optimal folder architecture                          │
//! │  - Generates execution plan                                     │
//! └──────────────────────────┬──────────────────────────────────────┘
//!                            │ aggregates
//!        ┌───────────────────┼───────────────────┐
//!        │                   │                   │
//!        ▼                   ▼                   ▼
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │ EXPLORE     │     │ EXPLORE     │     │ EXPLORE     │
//! │ AGENT 1     │     │ AGENT 2     │     │ AGENT N     │
//! │             │     │             │     │             │
//! │ Batch 1-50  │     │ Batch 51-100│     │ Batch N     │
//! │ PDFs/Docs   │     │ PDFs/Docs   │     │ PDFs/Docs   │
//! └─────────────┘     └─────────────┘     └─────────────┘
//!        │                   │                   │
//!        ▼                   ▼                   ▼
//! ┌─────────────────────────────────────────────────────┐
//! │              VISION API (grok-4-1-fast)             │
//! │  - Analyzes PDF page images                         │
//! │  - Extracts content summaries                       │
//! │  - Suggests filenames                               │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## Output Format (Explore Agents → Orchestrator)
//!
//! Each explore agent returns summaries in this format:
//! ```text
//! filename | content_summary | document_type | suggested_name
//! ```

pub mod cache;
pub mod client;
pub mod document_parser;
pub mod explore_agent;
pub mod integration;
pub mod orchestrator;
pub mod pdf_renderer;
pub mod types;
pub mod vision;

#[allow(unused_imports)]
pub use cache::ContentCache;
#[allow(unused_imports)]
pub use client::GrokClient;
#[allow(unused_imports)]
pub use explore_agent::ExploreAgent;
pub use integration::{GrokOrganizer, ScanResult};
#[allow(unused_imports)]
pub use orchestrator::OrchestratorAgent;
pub use types::*;
