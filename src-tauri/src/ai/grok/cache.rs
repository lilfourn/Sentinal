//! Content Cache System
//!
//! Persistent SQLite cache for document analyses.
//! Uses content hash (SHA-256) as key so analyses survive file moves.

use super::types::{AnalysisMethod, DocumentAnalysis, DocumentType};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// SQLite-backed content cache
pub struct ContentCache {
    db_path: std::path::PathBuf,
}

impl ContentCache {
    /// Open or create the cache database
    pub fn open(cache_dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {}", e))?;

        let db_path = cache_dir.join("content_cache.db");

        // Initialize database
        let conn = Self::connect(&db_path)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS document_analysis (
                content_hash TEXT PRIMARY KEY,
                file_path TEXT,
                file_name TEXT,
                content_summary TEXT,
                document_type TEXT,
                key_entities TEXT,
                suggested_name TEXT,
                confidence REAL,
                method TEXT,
                analyzed_at TEXT DEFAULT CURRENT_TIMESTAMP,
                token_cost INTEGER DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_file_path ON document_analysis(file_path);
            CREATE INDEX IF NOT EXISTS idx_analyzed_at ON document_analysis(analyzed_at);

            CREATE TABLE IF NOT EXISTS cache_stats (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                total_files_analyzed INTEGER DEFAULT 0,
                total_tokens_used INTEGER DEFAULT 0,
                total_cost_cents INTEGER DEFAULT 0,
                cache_hits INTEGER DEFAULT 0,
                last_updated TEXT DEFAULT CURRENT_TIMESTAMP
            );

            INSERT OR IGNORE INTO cache_stats (id) VALUES (1);
            "#,
        )
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

        Ok(Self { db_path })
    }

    /// Connect to the database
    fn connect(path: &Path) -> Result<rusqlite::Connection, String> {
        rusqlite::Connection::open(path)
            .map_err(|e| format!("Failed to open database: {}", e))
    }

    /// Get connection for operations
    fn conn(&self) -> Result<rusqlite::Connection, String> {
        Self::connect(&self.db_path)
    }

    /// Compute SHA-256 hash of file content
    pub fn hash_file(path: &Path) -> Result<String, String> {
        let mut file = File::open(path)
            .map_err(|e| format!("Failed to open file for hashing: {}", e))?;

        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file
                .read(&mut buffer)
                .map_err(|e| format!("Failed to read file: {}", e))?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Check if a file is already analyzed (by content hash)
    pub fn get_cached(&self, path: &Path) -> Result<Option<DocumentAnalysis>, String> {
        let hash = Self::hash_file(path)?;
        self.get_by_hash(&hash)
    }

    /// Get analysis by content hash
    pub fn get_by_hash(&self, hash: &str) -> Result<Option<DocumentAnalysis>, String> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT file_path, file_name, content_summary, document_type,
                       key_entities, suggested_name, confidence, method
                FROM document_analysis
                WHERE content_hash = ?
                "#,
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let result = stmt
            .query_row([hash], |row| {
                let entities_json: String = row.get(4)?;
                let entities: Vec<String> =
                    serde_json::from_str(&entities_json).unwrap_or_default();

                Ok(DocumentAnalysis {
                    file_path: row.get(0)?,
                    file_name: row.get(1)?,
                    content_summary: row.get(2)?,
                    document_type: DocumentType::from_str(&row.get::<_, String>(3)?),
                    key_entities: entities,
                    suggested_name: row.get(5)?,
                    confidence: row.get(6)?,
                    method: AnalysisMethod::Cached,
                })
            })
            .optional()
            .map_err(|e| format!("Query failed: {}", e))?;

        // Update cache hit stats
        if result.is_some() {
            let _ = conn.execute(
                "UPDATE cache_stats SET cache_hits = cache_hits + 1, last_updated = CURRENT_TIMESTAMP WHERE id = 1",
                [],
            );
        }

        Ok(result)
    }

    /// Store analysis result
    pub fn store(
        &self,
        path: &Path,
        analysis: &DocumentAnalysis,
        tokens: u32,
    ) -> Result<(), String> {
        let hash = Self::hash_file(path)?;
        let conn = self.conn()?;

        let entities_json = serde_json::to_string(&analysis.key_entities)
            .map_err(|e| format!("Failed to serialize entities: {}", e))?;

        conn.execute(
            r#"
            INSERT OR REPLACE INTO document_analysis
            (content_hash, file_path, file_name, content_summary, document_type,
             key_entities, suggested_name, confidence, method, token_cost, analyzed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            "#,
            rusqlite::params![
                hash,
                analysis.file_path,
                analysis.file_name,
                analysis.content_summary,
                analysis.document_type.as_str(),
                entities_json,
                analysis.suggested_name,
                analysis.confidence,
                format!("{:?}", analysis.method),
                tokens,
            ],
        )
        .map_err(|e| format!("Failed to store analysis: {}", e))?;

        // Update stats
        let cost_cents = (tokens as f64 * 0.00035 * 100.0) as i64; // Rough estimate
        conn.execute(
            r#"
            UPDATE cache_stats SET
                total_files_analyzed = total_files_analyzed + 1,
                total_tokens_used = total_tokens_used + ?,
                total_cost_cents = total_cost_cents + ?,
                last_updated = CURRENT_TIMESTAMP
            WHERE id = 1
            "#,
            rusqlite::params![tokens, cost_cents],
        )
        .map_err(|e| format!("Failed to update stats: {}", e))?;

        Ok(())
    }

    /// Filter paths to only those not in cache
    pub fn filter_uncached(&self, paths: &[std::path::PathBuf]) -> Result<Vec<std::path::PathBuf>, String> {
        let conn = self.conn()?;
        let mut uncached = Vec::new();

        for path in paths {
            if let Ok(hash) = Self::hash_file(path) {
                let exists: bool = conn
                    .query_row(
                        "SELECT 1 FROM document_analysis WHERE content_hash = ?",
                        [&hash],
                        |_| Ok(true),
                    )
                    .unwrap_or(false);

                if !exists {
                    uncached.push(path.clone());
                }
            } else {
                // If we can't hash it, include it as uncached
                uncached.push(path.clone());
            }
        }

        Ok(uncached)
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> Result<CacheStats, String> {
        let conn = self.conn()?;

        conn.query_row(
            "SELECT total_files_analyzed, total_tokens_used, total_cost_cents, cache_hits FROM cache_stats WHERE id = 1",
            [],
            |row| {
                Ok(CacheStats {
                    files_analyzed: row.get(0)?,
                    tokens_used: row.get(1)?,
                    cost_cents: row.get(2)?,
                    cache_hits: row.get(3)?,
                })
            },
        )
        .map_err(|e| format!("Failed to get stats: {}", e))
    }

    /// Clear all cached analyses
    pub fn clear(&self) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM document_analysis", [])
            .map_err(|e| format!("Failed to clear cache: {}", e))?;
        conn.execute(
            "UPDATE cache_stats SET total_files_analyzed = 0, total_tokens_used = 0, total_cost_cents = 0, cache_hits = 0 WHERE id = 1",
            [],
        )
        .map_err(|e| format!("Failed to reset stats: {}", e))?;
        Ok(())
    }

    /// Get count of cached analyses
    #[allow(dead_code)]
    pub fn count(&self) -> Result<usize, String> {
        let conn = self.conn()?;
        conn.query_row("SELECT COUNT(*) FROM document_analysis", [], |row| {
            row.get(0)
        })
        .map_err(|e| format!("Failed to count: {}", e))
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub files_analyzed: i64,
    pub tokens_used: i64,
    pub cost_cents: i64,
    pub cache_hits: i64,
}

// Add rusqlite feature for optional
trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_creation() {
        let dir = tempdir().unwrap();
        let cache = ContentCache::open(dir.path()).unwrap();
        assert_eq!(cache.count().unwrap(), 0);
    }

    #[test]
    fn test_hash_consistency() {
        let dir = tempdir().unwrap();
        let test_file = dir.path().join("test.txt");
        std::fs::write(&test_file, "Hello, world!").unwrap();

        let hash1 = ContentCache::hash_file(&test_file).unwrap();
        let hash2 = ContentCache::hash_file(&test_file).unwrap();
        assert_eq!(hash1, hash2);

        // Modify file
        std::fs::write(&test_file, "Different content").unwrap();
        let hash3 = ContentCache::hash_file(&test_file).unwrap();
        assert_ne!(hash1, hash3);
    }
}
