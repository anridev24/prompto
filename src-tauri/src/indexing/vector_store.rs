use serde::{Deserialize, Serialize};
use std::path::Path;
use usearch::ffi::{IndexOptions, MetricKind, ScalarKind};
use usearch::Index as UsearchIndex;

/// Metadata associated with each vector in the store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMetadata {
    pub symbol_name: String,
    pub file_path: String,
    pub language: String,
    pub start_line: usize,
    pub end_line: usize,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
}

/// Result from a vector search
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub metadata: VectorMetadata,
    pub similarity: f32,
}

/// HNSW-based vector store for semantic code search
pub struct VectorStore {
    index: UsearchIndex,
    metadata: Vec<VectorMetadata>,
    dimensions: usize,
    next_id: u64,
}

impl VectorStore {
    /// Create a new vector store with specified dimensions
    pub fn new(dimensions: usize) -> Result<Self, String> {
        println!("Creating vector store with {} dimensions", dimensions);

        let options = IndexOptions {
            dimensions,
            metric: MetricKind::Cos, // Cosine similarity
            quantization: ScalarKind::F32,
            connectivity: 16, // HNSW M parameter
            expansion_add: 128, // HNSW efConstruction
            expansion_search: 64, // HNSW ef
            multi: false,
        };

        let index = UsearchIndex::new(&options)
            .map_err(|e| format!("Failed to create index: {}", e))?;

        Ok(Self {
            index,
            metadata: Vec::new(),
            dimensions,
            next_id: 0,
        })
    }

    /// Add a vector with associated metadata to the store
    pub fn add(&mut self, vector: &[f32], metadata: VectorMetadata) -> Result<(), String> {
        if vector.len() != self.dimensions {
            return Err(format!(
                "Vector dimension mismatch: expected {}, got {}",
                self.dimensions,
                vector.len()
            ));
        }

        let id = self.next_id;
        self.index
            .add(id, vector)
            .map_err(|e| format!("Failed to add vector: {}", e))?;

        self.metadata.push(metadata);
        self.next_id += 1;

        Ok(())
    }

    /// Search for k nearest neighbors
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>, String> {
        if query.len() != self.dimensions {
            return Err(format!(
                "Query dimension mismatch: expected {}, got {}",
                self.dimensions,
                query.len()
            ));
        }

        let results = self
            .index
            .search(query, k)
            .map_err(|e| format!("Search failed: {}", e))?;

        let mut search_results = Vec::new();
        for i in 0..results.keys.len() {
            let id = results.keys[i] as usize;
            let distance = results.distances[i];

            // Convert distance to similarity (cosine distance -> similarity)
            // For cosine: similarity = 1 - distance
            let similarity = 1.0 - distance;

            if id < self.metadata.len() {
                search_results.push(SearchResult {
                    metadata: self.metadata[id].clone(),
                    similarity,
                });
            }
        }

        // Sort by similarity (highest first)
        search_results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

        Ok(search_results)
    }

    /// Get the number of vectors in the store
    pub fn len(&self) -> usize {
        self.metadata.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.metadata.is_empty()
    }

    /// Save the index and metadata to disk
    pub fn save<P: AsRef<Path>>(&self, index_path: P, metadata_path: P) -> Result<(), String> {
        // Save HNSW index
        self.index
            .save(index_path.as_ref().to_str().unwrap())
            .map_err(|e| format!("Failed to save index: {}", e))?;

        // Save metadata using bincode
        let metadata_bytes = bincode::serialize(&self.metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        std::fs::write(metadata_path, metadata_bytes)
            .map_err(|e| format!("Failed to write metadata: {}", e))?;

        println!("Vector store saved ({} vectors)", self.len());
        Ok(())
    }

    /// Load the index and metadata from disk
    pub fn load<P: AsRef<Path>>(
        index_path: P,
        metadata_path: P,
        dimensions: usize,
    ) -> Result<Self, String> {
        println!("Loading vector store from disk...");

        // Load HNSW index
        let options = IndexOptions {
            dimensions,
            metric: MetricKind::Cos,
            quantization: ScalarKind::F32,
            connectivity: 16,
            expansion_add: 128,
            expansion_search: 64,
            multi: false,
        };

        let index = UsearchIndex::new(&options)
            .map_err(|e| format!("Failed to create index: {}", e))?;

        index
            .load(index_path.as_ref().to_str().unwrap())
            .map_err(|e| format!("Failed to load index: {}", e))?;

        // Load metadata
        let metadata_bytes = std::fs::read(metadata_path)
            .map_err(|e| format!("Failed to read metadata: {}", e))?;

        let metadata: Vec<VectorMetadata> = bincode::deserialize(&metadata_bytes)
            .map_err(|e| format!("Failed to deserialize metadata: {}", e))?;

        let next_id = metadata.len() as u64;

        println!("Vector store loaded ({} vectors)", metadata.len());

        Ok(Self {
            index,
            metadata,
            dimensions,
            next_id,
        })
    }

    /// Clear all vectors and metadata
    pub fn clear(&mut self) {
        // Recreate the index
        let options = IndexOptions {
            dimensions: self.dimensions,
            metric: MetricKind::Cos,
            quantization: ScalarKind::F32,
            connectivity: 16,
            expansion_add: 128,
            expansion_search: 64,
            multi: false,
        };

        if let Ok(new_index) = UsearchIndex::new(&options) {
            self.index = new_index;
        }

        self.metadata.clear();
        self.next_id = 0;
    }

    /// Get metadata by index
    pub fn get_metadata(&self, index: usize) -> Option<&VectorMetadata> {
        self.metadata.get(index)
    }

    /// Get all metadata
    pub fn all_metadata(&self) -> &[VectorMetadata] {
        &self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_store_creation() {
        let store = VectorStore::new(384);
        assert!(store.is_ok());
        let store = store.unwrap();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn test_add_and_search() {
        let mut store = VectorStore::new(3).unwrap();

        let metadata1 = VectorMetadata {
            symbol_name: "test_func".to_string(),
            file_path: "test.rs".to_string(),
            language: "rust".to_string(),
            start_line: 1,
            end_line: 10,
            signature: None,
            doc_comment: None,
        };

        let vector1 = vec![1.0, 0.0, 0.0];
        store.add(&vector1, metadata1).unwrap();

        assert_eq!(store.len(), 1);

        let results = store.search(&vector1, 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.symbol_name, "test_func");
        assert!(results[0].similarity > 0.99); // Should be very close to 1.0
    }

    #[test]
    fn test_dimension_mismatch() {
        let mut store = VectorStore::new(3).unwrap();

        let metadata = VectorMetadata {
            symbol_name: "test".to_string(),
            file_path: "test.rs".to_string(),
            language: "rust".to_string(),
            start_line: 1,
            end_line: 10,
            signature: None,
            doc_comment: None,
        };

        let wrong_vector = vec![1.0, 0.0]; // Wrong dimension
        let result = store.add(&wrong_vector, metadata);
        assert!(result.is_err());
    }

    #[test]
    fn test_semantic_similarity() {
        let mut store = VectorStore::new(3).unwrap();

        // Add similar vectors
        let vector1 = vec![1.0, 0.0, 0.0];
        let vector2 = vec![0.9, 0.1, 0.0]; // Similar to vector1
        let vector3 = vec![0.0, 0.0, 1.0]; // Different from vector1

        let meta1 = VectorMetadata {
            symbol_name: "login".to_string(),
            file_path: "auth.rs".to_string(),
            language: "rust".to_string(),
            start_line: 1,
            end_line: 10,
            signature: None,
            doc_comment: None,
        };

        let meta2 = VectorMetadata {
            symbol_name: "authenticate".to_string(),
            file_path: "auth.rs".to_string(),
            language: "rust".to_string(),
            start_line: 20,
            end_line: 30,
            signature: None,
            doc_comment: None,
        };

        let meta3 = VectorMetadata {
            symbol_name: "parse_json".to_string(),
            file_path: "utils.rs".to_string(),
            language: "rust".to_string(),
            start_line: 1,
            end_line: 10,
            signature: None,
            doc_comment: None,
        };

        store.add(&vector1, meta1).unwrap();
        store.add(&vector2, meta2).unwrap();
        store.add(&vector3, meta3).unwrap();

        // Search with vector similar to vector1
        let query = vec![0.95, 0.05, 0.0];
        let results = store.search(&query, 2).unwrap();

        // Should get vector2 and vector1 as most similar
        assert_eq!(results.len(), 2);
        // First result should have higher similarity
        assert!(results[0].similarity > results[1].similarity);
    }
}
