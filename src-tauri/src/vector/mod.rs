//! Vector Index Module
//!
//! Provides semantic search capabilities using local embeddings via fastembed-rs.
//! This module enables content-based file discovery without requiring external API calls.

#![allow(dead_code)]

pub mod embedder;
pub mod search;

pub use embedder::*;

use fastembed::EmbeddingModel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration for the vector index
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorConfig {
    /// Embedding model to use (default: AllMiniLmL6V2)
    pub model: VectorModelType,
    /// Minimum similarity score for search results (0.0 to 1.0)
    pub similarity_threshold: f32,
    /// Maximum number of results to return
    pub max_results: usize,
}

impl Default for VectorConfig {
    fn default() -> Self {
        Self {
            model: VectorModelType::AllMiniLmL6V2,
            similarity_threshold: 0.5,
            max_results: 20,
        }
    }
}

/// Supported embedding models (wrapper for fastembed::EmbeddingModel)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VectorModelType {
    /// All-MiniLM-L6-v2: Fast, good quality, 384 dimensions
    AllMiniLmL6V2,
    /// BGE-Small-EN-v1.5: Compact, English-focused
    BgeSmallEnV15,
}

impl VectorModelType {
    /// Convert to fastembed's EmbeddingModel enum
    pub fn to_fastembed_model(&self) -> EmbeddingModel {
        match self {
            VectorModelType::AllMiniLmL6V2 => EmbeddingModel::AllMiniLML6V2,
            VectorModelType::BgeSmallEnV15 => EmbeddingModel::BGESmallENV15,
        }
    }
}

/// A document in the vector index with its embedding
#[derive(Debug, Clone)]
pub struct VectorDocument {
    /// Absolute path to the file
    pub path: PathBuf,
    /// Combined text used for embedding (filename + content_preview)
    pub text: String,
    /// The embedding vector
    pub embedding: Vec<f32>,
    /// Semantic tags derived from similarity to category embeddings
    pub tags: Vec<String>,
}

/// The main vector index structure
/// Holds all indexed documents and provides search capabilities
pub struct VectorIndex {
    /// The embedding model instance
    embedder: VectorEmbedder,
    /// Indexed documents keyed by path
    documents: HashMap<PathBuf, VectorDocument>,
    /// Configuration
    config: VectorConfig,
    /// Pre-computed category embeddings for tag assignment
    category_embeddings: HashMap<String, Vec<f32>>,
}

impl VectorIndex {
    /// Create a new vector index with the given configuration
    ///
    /// Note: This downloads the model on first use (~100MB for AllMiniLmL6V2)
    pub fn new(config: VectorConfig) -> Result<Self, String> {
        let embedder = VectorEmbedder::new(&config)?;

        // Pre-compute category embeddings for semantic tagging
        let categories = vec![
            "document", "invoice", "photo", "screenshot", "code",
            "archive", "installer", "video", "audio", "spreadsheet",
            "presentation", "ebook", "resume", "receipt", "contract",
        ];

        let mut category_embeddings = HashMap::new();
        for category in categories {
            match embedder.get_embedding(category) {
                Ok(embedding) => {
                    category_embeddings.insert(category.to_string(), embedding);
                }
                Err(e) => {
                    eprintln!("[VectorIndex] Warning: Failed to embed category '{}': {}", category, e);
                }
            }
        }

        Ok(Self {
            embedder,
            documents: HashMap::new(),
            config,
            category_embeddings,
        })
    }

    /// Get the number of indexed documents
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Get the configuration
    pub fn config(&self) -> &VectorConfig {
        &self.config
    }

    /// Get a reference to the embedder
    pub fn embedder(&self) -> &VectorEmbedder {
        &self.embedder
    }

    /// Get a document by path
    pub fn get_document(&self, path: &PathBuf) -> Option<&VectorDocument> {
        self.documents.get(path)
    }

    /// Get all documents
    pub fn documents(&self) -> &HashMap<PathBuf, VectorDocument> {
        &self.documents
    }

    /// Get category embeddings for tag assignment
    pub fn category_embeddings(&self) -> &HashMap<String, Vec<f32>> {
        &self.category_embeddings
    }

    /// Insert a document into the index
    pub fn insert_document(&mut self, doc: VectorDocument) {
        self.documents.insert(doc.path.clone(), doc);
    }

    /// Remove a document from the index
    pub fn remove_document(&mut self, path: &PathBuf) -> Option<VectorDocument> {
        self.documents.remove(path)
    }

    /// Clear all documents from the index
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.documents.clear();
    }
}
