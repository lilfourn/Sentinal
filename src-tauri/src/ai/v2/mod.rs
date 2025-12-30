//! V2 AI module with semantic, rule-based file organization.
//!
//! This module replaces the shell-based V1 approach with semantic tools:
//! - `query_semantic_index`: Search files by semantic similarity
//! - `apply_organization_rules`: Define rules for bulk file operations
//! - `preview_operations`: Preview planned changes before execution
//! - `commit_plan`: Finalize and submit the organization plan
//!
//! The agent uses declarative rules instead of shell commands, enabling
//! more intelligent and bulk-oriented file organization.

#![allow(dead_code)]

mod prompts;
mod tools;
mod vfs;

pub mod agent_loop;

// Only export the main entry point
pub use agent_loop::run_v2_agentic_organize;
