//! Vector Search Module
//!
//! Provides semantic search capabilities over the indexed documents.
//! Uses cosine similarity to find documents matching a query.

use super::{cosine_similarity, VectorIndex};
use std::path::PathBuf;

impl VectorIndex {
    /// Search for documents matching a query string
    ///
    /// Returns documents sorted by similarity score (highest first),
    /// filtered by the configured similarity threshold.
    ///
    /// # Arguments
    /// * `query` - Natural language search query
    ///
    /// # Returns
    /// Vector of (path, similarity_score) tuples
    pub fn search(&self, query: &str) -> Result<Vec<(PathBuf, f32)>, String> {
        if query.is_empty() {
            return Err("Query cannot be empty".to_string());
        }

        if self.is_empty() {
            return Ok(vec![]);
        }

        // Generate query embedding
        let query_embedding = self.embedder().get_embedding(query)?;

        // Compute similarity with all documents
        let mut results: Vec<(PathBuf, f32)> = self
            .documents()
            .iter()
            .map(|(path, doc)| {
                let score = cosine_similarity(&query_embedding, &doc.embedding);
                (path.clone(), score)
            })
            .filter(|(_, score)| *score >= self.config().similarity_threshold)
            .collect();

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Limit results
        results.truncate(self.config().max_results);

        Ok(results)
    }

    /// Compute similarity between a specific document and a query
    ///
    /// Useful for checking if a file matches a search term
    pub fn similarity(&self, path: &PathBuf, query: &str) -> Result<f32, String> {
        let doc = self
            .get_document(path)
            .ok_or_else(|| format!("Document not found: {:?}", path))?;

        let query_embedding = self.embedder().get_embedding(query)?;

        Ok(cosine_similarity(&query_embedding, &doc.embedding))
    }

    /// Get semantic tags for a document
    ///
    /// Tags are pre-computed during indexing based on similarity
    /// to category embeddings
    pub fn get_tags(&self, path: &PathBuf) -> Option<Vec<String>> {
        self.get_document(path).map(|doc| doc.tags.clone())
    }

    /// Find documents matching a specific tag
    ///
    /// Returns all documents that have the specified tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<PathBuf> {
        self.documents()
            .iter()
            .filter(|(_, doc)| doc.tags.contains(&tag.to_string()))
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// Get all unique tags in the index
    pub fn all_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self
            .documents()
            .values()
            .flat_map(|doc| doc.tags.iter().cloned())
            .collect();

        tags.sort();
        tags.dedup();
        tags
    }

    /// Find similar documents to a given document
    ///
    /// Returns documents most similar to the one at the given path
    pub fn find_similar(&self, path: &PathBuf, limit: usize) -> Result<Vec<(PathBuf, f32)>, String> {
        let doc = self
            .get_document(path)
            .ok_or_else(|| format!("Document not found: {:?}", path))?;

        let source_embedding = &doc.embedding;

        let mut results: Vec<(PathBuf, f32)> = self
            .documents()
            .iter()
            .filter(|(p, _)| *p != path) // Exclude self
            .map(|(p, d)| {
                let score = cosine_similarity(source_embedding, &d.embedding);
                (p.clone(), score)
            })
            .filter(|(_, score)| *score >= self.config().similarity_threshold)
            .collect();

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        results.truncate(limit);
        Ok(results)
    }
}
