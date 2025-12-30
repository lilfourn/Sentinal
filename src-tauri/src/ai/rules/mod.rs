//! Rule-based file matching and organization module.
//!
//! This module provides a DSL for expressing file matching rules that the AI agent
//! can use to organize files in bulk. Instead of shell-based tools (ls, grep, find, cat),
//! the V2 agent uses semantic, declarative rules.
//!
//! Example rule expressions:
//! - `file.ext == 'pdf'`
//! - `file.ext IN ['pdf', 'docx']`
//! - `file.vector_similarity('tax invoice') > 0.8`
//! - `file.name.contains('invoice') AND file.size > 10KB`
//! - `NOT file.isHidden AND file.modifiedAt > '2024-01-01'`
//! - `(file.ext == 'jpg' OR file.ext == 'png') AND file.size < 5MB`

#![allow(dead_code)]
#![allow(unused_imports)]

pub mod ast;
pub mod evaluator;
pub mod parser;

pub use ast::*;
pub use evaluator::*;
pub use parser::*;
