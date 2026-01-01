//! Write-Ahead Log (WAL) for crash recovery
//!
//! The WAL module provides durable, crash-recoverable file operation logging.
//! Operations are logged before execution and marked complete after,
//! enabling recovery of interrupted operations on restart.
//!
//! ## Modules
//! - `entry` - WAL entry types and journal structure
//! - `io` - Safe I/O utilities (atomic writes, fsync, symlink detection)
//! - `journal` - Journal persistence with file locking
//! - `recovery` - Recovery operations for interrupted jobs

#![allow(dead_code)]
#![allow(unused_imports)]

pub mod entry;
pub mod io;
pub mod journal;
pub mod recovery;

pub use entry::*;
pub use io::{atomic_write, copy_dir_safe, file_type_no_follow, is_symlink, FileTypeInfo, SafeIoError};
pub use journal::*;
pub use recovery::*;
