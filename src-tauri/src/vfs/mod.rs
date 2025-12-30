//! Virtual File System (VFS) Module
//!
//! Provides an in-memory shadow filesystem that mirrors the real filesystem.
//! This enables simulation of file operations before committing changes,
//! allowing for validation, conflict detection, and undo/redo capabilities.

pub mod graph;
pub mod node;
pub mod scanner;
pub mod simulator;

pub use graph::*;
pub use node::*;
pub use scanner::*;
pub use simulator::*;
