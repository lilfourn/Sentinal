//! Execution Engine Module
//!
//! Provides parallel execution of file operations using a DAG-based
//! dependency graph. Operations at the same level (no dependencies between
//! them) are executed in parallel for optimal performance.

#![allow(dead_code)]
#![allow(unused_imports)]

pub mod dag;
pub mod executor;

pub use dag::*;
pub use executor::*;
