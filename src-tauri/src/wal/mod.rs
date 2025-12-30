//! Write-Ahead Log (WAL) for crash recovery
//!
//! The WAL module provides durable, crash-recoverable file operation logging.
//! Operations are logged before execution and marked complete after,
//! enabling recovery of interrupted operations on restart.

#![allow(dead_code)]
#![allow(unused_imports)]

pub mod entry;
pub mod journal;
pub mod recovery;

pub use entry::*;
pub use journal::*;
pub use recovery::*;
