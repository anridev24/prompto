# Hybrid Search System Implementation Plan

## Executive Summary

This document provides a production-ready implementation plan for upgrading the **prompto** codebase indexer from a simple HashMap-based search to a sophisticated **Hybrid Search System** combining:
- **Full-text search** (Tantivy)
- **Semantic search** (Local embeddings)
- **Reciprocal Rank Fusion** (RRF) for result merging

The upgrade will be implemented in 4 phases, maintaining backward compatibility while progressively adding capabilities.

**Timeline:** 17-25 days
**New Code:** ~2700-3400 lines
**New Dependencies:** 11 Rust crates
**Breaking Changes:** None (fully backward compatible)

---

## Current System Analysis

### Architecture
- **Indexing:** Tree-sitter AST-based parsing (Rust, JavaScript, TypeScript, Python)
- **Storage:** In-memory HashMap (`symbol_map: HashMap<String, Vec<CodeSymbol>>`)
- **Search:** Simple substring matching (exact + case-insensitive contains)
- **Scoring:** Static relevance (1.0 for all results)
- **Interface:** Tauri commands for frontend integration

### Limitations
1. ❌ No file path/name search
2. ❌ No stemming/normalization (searching "authentication" won't find "authenticate")
3. ❌ Poor relevance ranking (results sorted arbitrarily)
4. ❌ No full-text search in code content, comments, or docstrings
5. ❌ No semantic understanding (can't find conceptually similar code)
6. ❌ O(n) search performance across all symbols

### Example Failure Case
**User Query:** "Analyze how indexing is implemented in project"

**Keywords Extracted:** `["indexing", "implemented", "project"]`

**Symbol Names in Codebase:**
- `TreeSitterIndexer` - contains "indexer" not "indexing" ❌
- `index_codebase` - contains "index" not "indexing" ❌
- `IndexerState` - contains "indexer" not "indexing" ❌

**File Name:** `tree_sitter_indexer.rs` - contains "indexer" but **file paths aren't searched** ❌

**Result:** No matches found despite having extensive indexing code!

### Strengths to Preserve
- ✅ Excellent syntax-aware symbol extraction
- ✅ Multi-language support
- ✅ Clean separation of concerns
- ✅ Tauri integration working well

---

## Phase 1: Quick Fixes (Foundation)

**Goal:** Improve existing search with minimal architectural changes
**Duration:** 2-3 days
**Complexity:** Low
**Lines of Code:** ~500-700

### 1.1 File Path Search

#### Files to Modify
- `src-tauri/src/models/code_index.rs`
- `src-tauri/src/indexing/tree_sitter_indexer.rs`
- `src-tauri/src/commands/index_commands.rs`

#### Implementation

**Step 1.1.1:** Add file path index to `CodebaseIndex`

```rust
// In src-tauri/src/models/code_index.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseIndex {
    pub root_path: String,
    pub files: HashMap<String, IndexedFile>,
    pub symbol_map: HashMap<String, Vec<CodeSymbol>>,

    // NEW: File path search structures
    pub file_paths: Vec<String>,  // All indexed file paths
    pub file_path_components: HashMap<String, Vec<usize>>, // component -> file indices

    pub language_stats: HashMap<String, usize>,
    pub total_files: usize,
    pub indexed_at: u64,
}
```

**Step 1.1.2:** Build file path index during indexing

```rust
// In src-tauri/src/indexing/tree_sitter_indexer.rs
impl CodebaseIndex {
    pub fn add_file(&mut self, file: IndexedFile) {
        // Existing logic...

        // NEW: Index file path
        let file_index = self.file_paths.len();
        self.file_paths.push(file.path.clone());

        // Index path components for fuzzy matching
        // e.g., "src/auth/login.rs" -> ["src", "auth", "login", "rs"]
        let components = self.extract_path_components(&file.path);
        for component in components {
            self.file_path_components
                .entry(component.to_lowercase())
                .or_insert_with(Vec::new)
                .push(file_index);
        }

        self.files.insert(file.path.clone(), file);
    }

    fn extract_path_components(&self, path: &str) -> Vec<String> {
        path.replace("\\", "/")
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }
}
```

**Step 1.1.3:** Add file path query method

```rust
// In src-tauri/src/indexing/tree_sitter_indexer.rs
impl TreeSitterIndexer {
    pub fn query_file_paths(
        &self,
        index: &CodebaseIndex,
        query: &str,
        max_results: usize,
    ) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let mut matches: Vec<(String, f32)> = Vec::new();

        // Search in file path components
        for (component, file_indices) in &index.file_path_components {
            if component.contains(&query_lower) {
                let score = self.calculate_component_score(component, &query_lower);
                for &idx in file_indices {
                    if let Some(path) = index.file_paths.get(idx) {
                        matches.push((path.clone(), score));
                    }
                }
            }
        }

        // Sort by score (descending)
        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        matches.truncate(max_results);

        matches.into_iter().map(|(path, _)| path).collect()
    }

    fn calculate_component_score(&self, component: &str, query: &str) -> f32 {
        if component == query {
            1.0 // Exact match
        } else if component.starts_with(query) {
            0.8 // Prefix match
        } else {
            0.5 // Contains match
        }
    }
}
```

**Step 1.1.4:** Add Tauri command

```rust
// In src-tauri/src/commands/index_commands.rs
#[tauri::command]
pub async fn search_files(
    query: String,
    max_results: Option<usize>,
    state: State<'_, IndexerState>,
) -> Result<Vec<String>, String> {
    let indexer = state.indexer.lock()
        .map_err(|e| format!("Failed to lock indexer: {}", e))?;

    let index_lock = state.current_index.lock()
        .map_err(|e| format!("Failed to lock index: {}", e))?;

    let index = index_lock.as_ref()
        .ok_or_else(|| "No codebase indexed".to_string())?;

    Ok(indexer.query_file_paths(index, &query, max_results.unwrap_or(50)))
}
```

### 1.2 Stemming and Normalization

#### Dependencies to Add

```toml
# In src-tauri/Cargo.toml
[dependencies]
# Existing dependencies...

# Text processing
unicode-segmentation = "1.10"  # Proper word segmentation
rust-stemmers = "1.2"          # Porter stemmer for English
```

#### Implementation

**Step 1.2.1:** Create text normalization module

```rust
// NEW FILE: src-tauri/src/indexing/text_normalizer.rs
use rust_stemmers::{Algorithm, Stemmer};
use unicode_segmentation::UnicodeSegmentation;
use std::collections::HashSet;

pub struct TextNormalizer {
    stemmer: Stemmer,
    stop_words: HashSet<String>,
}

impl TextNormalizer {
    pub fn new() -> Self {
        Self {
            stemmer: Stemmer::create(Algorithm::English),
            stop_words: Self::create_stop_words(),
        }
    }

    fn create_stop_words() -> HashSet<String> {
        // Common programming and English stop words
        [
            "the", "a", "an", "and", "or", "but", "in", "on", "at",
            "to", "for", "of", "with", "by", "from", "as", "is", "was",
            // Programming-specific
            "get", "set", "new", "old", "tmp", "temp", "var", "fn", "func",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Normalize a query/text for searching
    pub fn normalize(&self, text: &str) -> Vec<String> {
        text.unicode_words()
            .map(|w| w.to_lowercase())
            .filter(|w| !self.stop_words.contains(w))
            .filter(|w| w.len() > 2)  // Filter very short words
            .map(|w| self.stemmer.stem(&w).to_string())
            .collect()
    }

    /// Normalize a symbol name (preserve camelCase/snake_case structure)
    pub fn normalize_symbol(&self, name: &str) -> Vec<String> {
        // Split camelCase and snake_case
        let mut tokens = Vec::new();

        // Handle snake_case
        for part in name.split('_') {
            // Handle camelCase within each part
            tokens.extend(self.split_camel_case(part));
        }

        tokens.into_iter()
            .map(|t| t.to_lowercase())
            .filter(|t| t.len() > 1)
            .map(|t| self.stemmer.stem(&t).to_string())
            .collect()
    }

    fn split_camel_case(&self, s: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut last_was_upper = false;

        for ch in s.chars() {
            if ch.is_uppercase() {
                if !current.is_empty() && !last_was_upper {
                    result.push(current.clone());
                    current.clear();
                }
                current.push(ch);
                last_was_upper = true;
            } else {
                current.push(ch);
                last_was_upper = false;
            }
        }

        if !current.is_empty() {
            result.push(current);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_symbol() {
        let normalizer = TextNormalizer::new();

        // camelCase
        assert_eq!(
            normalizer.normalize_symbol("getUserAuthentication"),
            vec!["get", "user", "authent"]  // "authent" is stem of "authentication"
        );

        // snake_case
        assert_eq!(
            normalizer.normalize_symbol("user_authentication_handler"),
            vec!["user", "authent", "handler"]
        );
    }
}
```

**Step 1.2.2:** Update index to store normalized terms

```rust
// In src-tauri/src/models/code_index.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseIndex {
    pub root_path: String,
    pub files: HashMap<String, IndexedFile>,
    pub symbol_map: HashMap<String, Vec<CodeSymbol>>,
    pub file_paths: Vec<String>,
    pub file_path_components: HashMap<String, Vec<usize>>,

    // NEW: Normalized search index
    pub normalized_symbol_map: HashMap<String, Vec<CodeSymbol>>, // stemmed -> symbols

    pub language_stats: HashMap<String, usize>,
    pub total_files: usize,
    pub indexed_at: u64,
}
```

**Step 1.2.3:** Update query to use normalization

```rust
// In src-tauri/src/indexing/tree_sitter_indexer.rs
impl TreeSitterIndexer {
    pub fn query_index(
        &self,
        index: &CodebaseIndex,
        query: &IndexQuery,
    ) -> Vec<CodeChunk> {
        let mut results = Vec::new();
        let max_results = query.max_results.unwrap_or(50);

        for keyword in &query.keywords {
            // 1. Try exact match first (highest score)
            if let Some(symbols) = index.symbol_map.get(keyword) {
                for symbol in symbols {
                    let mut chunk = self.symbol_to_chunk(symbol, &index.files);
                    chunk.relevance_score = 1.0;
                    results.push(chunk);
                }
            }

            // 2. Try normalized match
            let normalized_terms = self.normalizer.normalize(keyword);
            for term in normalized_terms {
                if let Some(symbols) = index.normalized_symbol_map.get(&term) {
                    for symbol in symbols {
                        let mut chunk = self.symbol_to_chunk(symbol, &index.files);
                        chunk.relevance_score = 0.8;  // Lower score for fuzzy match
                        results.push(chunk);
                    }
                }
            }

            // 3. Partial match in original names (lowest score)
            for (name, symbols) in &index.symbol_map {
                if name.to_lowercase().contains(&keyword.to_lowercase())
                    && name != keyword
                {
                    for symbol in symbols {
                        let mut chunk = self.symbol_to_chunk(symbol, &index.files);
                        chunk.relevance_score = 0.5;
                        results.push(chunk);
                    }
                }
            }
        }

        // Remove duplicates (prefer higher scores)
        results = self.deduplicate_results(results);

        // Sort by relevance
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(max_results);
        results
    }
}
```

### 1.3 Better Relevance Scoring

#### Implementation

**Step 1.3.1:** Create scoring module

```rust
// NEW FILE: src-tauri/src/indexing/relevance_scorer.rs
pub struct RelevanceScorer;

impl RelevanceScorer {
    /// Calculate TF-IDF style score for a symbol match
    pub fn score_symbol_match(
        symbol_name: &str,
        query_term: &str,
        match_type: MatchType,
        total_symbols: usize,
        term_frequency: usize,
    ) -> f32 {
        // Base score from match type
        let base_score = match match_type {
            MatchType::Exact => 1.0,
            MatchType::NormalizedExact => 0.9,
            MatchType::Prefix => 0.7,
            MatchType::Contains => 0.5,
            MatchType::Normalized => 0.6,
        };

        // Length ratio bonus (shorter names with match are more relevant)
        let length_ratio = query_term.len() as f32 / symbol_name.len() as f32;
        let length_bonus = length_ratio * 0.2;

        // IDF-style score (rarer terms are more valuable)
        let idf = (total_symbols as f32 / term_frequency as f32).ln();
        let idf_bonus = (idf / 10.0).min(0.3);  // Cap at 0.3

        (base_score + length_bonus + idf_bonus).min(1.0)
    }

    /// Score based on symbol kind (functions are often more relevant than imports)
    pub fn score_symbol_kind(kind: &SymbolKind) -> f32 {
        match kind {
            SymbolKind::Function => 1.0,
            SymbolKind::Class | SymbolKind::Struct => 0.95,
            SymbolKind::Method => 0.9,
            SymbolKind::Enum | SymbolKind::Interface => 0.85,
            SymbolKind::Constant => 0.7,
            SymbolKind::Variable => 0.6,
            SymbolKind::Import | SymbolKind::Export => 0.4,
        }
    }

    /// Combine all scores
    pub fn calculate_final_score(
        symbol_score: f32,
        kind_score: f32,
        has_doc_comment: bool,
    ) -> f32 {
        let doc_bonus = if has_doc_comment { 0.05 } else { 0.0 };

        // Weighted combination
        (symbol_score * 0.7 + kind_score * 0.3 + doc_bonus).min(1.0)
    }
}

pub enum MatchType {
    Exact,              // Exact match on original name
    NormalizedExact,    // Exact match on normalized/stemmed
    Prefix,             // Query is prefix of symbol
    Contains,           // Query contained in symbol
    Normalized,         // Match via normalization
}
```

### 1.4 Testing Approach

**Unit Tests:**
```rust
// In src-tauri/src/indexing/text_normalizer.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_camel_case_splitting() { /* ... */ }

    #[test]
    fn test_stemming() { /* ... */ }

    #[test]
    fn test_stop_word_removal() { /* ... */ }
}

// In src-tauri/src/indexing/relevance_scorer.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_exact_match_highest_score() { /* ... */ }

    #[test]
    fn test_idf_scoring() { /* ... */ }

    #[test]
    fn test_kind_priority() { /* ... */ }
}
```

### 1.5 Module Structure Update

```rust
// In src-tauri/src/indexing/mod.rs
pub mod tree_sitter_indexer;
pub mod text_normalizer;        // NEW
pub mod relevance_scorer;       // NEW
```

### 1.6 Estimated Complexity
- **Lines of Code:** ~500-700
- **New Dependencies:** 2 (unicode-segmentation, rust-stemmers)
- **Breaking Changes:** None (backward compatible)
- **Performance Impact:** Minimal (<10% indexing overhead)

---

## Phase 2: Full-Text Search (Tantivy Integration)

**Goal:** Add full-text search capabilities for code content, comments, and docstrings
**Duration:** 5-7 days
**Complexity:** Medium-High
**Lines of Code:** ~800-1000

### 2.1 Dependencies

```toml
# In src-tauri/Cargo.toml
[dependencies]
# Existing...

# Full-text search
tantivy = "0.22"           # Full-text search engine
tempfile = "3.8"           # For index directory management
```

### 2.2 Schema Design

```rust
// NEW FILE: src-tauri/src/indexing/tantivy_indexer.rs
use tantivy::schema::*;
use tantivy::{Index, IndexWriter, TantivyDocument, doc};
use tantivy::query::QueryParser;
use tantivy::collector::TopDocs;

pub struct TantivyIndexer {
    schema: Schema,
    index: Index,
    index_writer: IndexWriter,

    // Field handles
    field_symbol_name: Field,
    field_file_path: Field,
    field_language: Field,
    field_symbol_kind: Field,
    field_signature: Field,
    field_doc_comment: Field,
    field_start_line: Field,
    field_end_line: Field,
}

impl TantivyIndexer {
    pub fn new() -> Result<Self, String> {
        // Define schema
        let mut schema_builder = Schema::builder();

        // Symbol name (tokenized for partial matching, stored for retrieval)
        let field_symbol_name = schema_builder.add_text_field(
            "symbol_name",
            TEXT | STORED
        );

        // File path (tokenized for path component search)
        let field_file_path = schema_builder.add_text_field(
            "file_path",
            TEXT | STORED
        );

        // Language (for filtering)
        let field_language = schema_builder.add_text_field(
            "language",
            STRING | STORED
        );

        // Symbol kind (for filtering and boosting)
        let field_symbol_kind = schema_builder.add_text_field(
            "symbol_kind",
            STRING | STORED
        );

        // Signature (full-text searchable code)
        let field_signature = schema_builder.add_text_field(
            "signature",
            TEXT | STORED
        );

        // Documentation comments (important for semantic understanding)
        let field_doc_comment = schema_builder.add_text_field(
            "doc_comment",
            TEXT | STORED
        );

        // Line numbers (for deduplication and result display)
        let field_start_line = schema_builder.add_u64_field(
            "start_line",
            INDEXED | STORED
        );

        let field_end_line = schema_builder.add_u64_field(
            "end_line",
            INDEXED | STORED
        );

        let schema = schema_builder.build();

        // Create index in temporary directory
        let index_dir = TempDir::new()
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;

        let index = Index::create_in_dir(
            index_dir.path(),
            schema.clone()
        ).map_err(|e| format!("Failed to create index: {}", e))?;

        // Create index writer with 50MB buffer
        let index_writer = index.writer(50_000_000)
            .map_err(|e| format!("Failed to create writer: {}", e))?;

        Ok(Self {
            schema,
            index,
            index_writer,
            field_symbol_name,
            field_file_path,
            field_language,
            field_symbol_kind,
            field_signature,
            field_doc_comment,
            field_start_line,
            field_end_line,
        })
    }

    /// Index a code symbol
    pub fn add_symbol(
        &mut self,
        symbol: &CodeSymbol,
        file_language: &str,
    ) -> Result<(), String> {
        let mut doc = TantivyDocument::default();

        doc.add_text(self.field_symbol_name, &symbol.name);
        doc.add_text(self.field_file_path, &symbol.file_path);
        doc.add_text(self.field_language, file_language);
        doc.add_text(self.field_symbol_kind, &format!("{:?}", symbol.kind));

        if let Some(ref sig) = symbol.signature {
            doc.add_text(self.field_signature, sig);
        }

        if let Some(ref doc_comment) = symbol.doc_comment {
            doc.add_text(self.field_doc_comment, doc_comment);
        }

        doc.add_u64(self.field_start_line, symbol.start_line as u64);
        doc.add_u64(self.field_end_line, symbol.end_line as u64);

        self.index_writer.add_document(doc)
            .map_err(|e| format!("Failed to add document: {}", e))?;

        Ok(())
    }

    /// Commit all pending changes
    pub fn commit(&mut self) -> Result<(), String> {
        self.index_writer.commit()
            .map_err(|e| format!("Failed to commit: {}", e))?;
        Ok(())
    }

    /// Search the index
    pub fn search(
        &self,
        query_str: &str,
        max_results: usize,
    ) -> Result<Vec<TantivySearchResult>, String> {
        let reader = self.index.reader()
            .map_err(|e| format!("Failed to create reader: {}", e))?;

        let searcher = reader.searcher();

        // Create multi-field query parser
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.field_symbol_name,
                self.field_signature,
                self.field_doc_comment,
                self.field_file_path,
            ],
        );

        let query = query_parser.parse_query(query_str)
            .map_err(|e| format!("Failed to parse query: {}", e))?;

        // Search with custom collector
        let top_docs = searcher.search(
            &query,
            &TopDocs::with_limit(max_results)
        ).map_err(|e| format!("Search failed: {}", e))?;

        // Extract results
        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)
                .map_err(|e| format!("Failed to retrieve doc: {}", e))?;

            let result = self.doc_to_result(&retrieved_doc, _score)?;
            results.push(result);
        }

        Ok(results)
    }
}

#[derive(Debug, Clone)]
pub struct TantivySearchResult {
    pub symbol_name: String,
    pub file_path: String,
    pub language: String,
    pub symbol_kind: String,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub score: f32,
}
```

### 2.3 Integration with TreeSitterIndexer

```rust
// In src-tauri/src/indexing/tree_sitter_indexer.rs
use crate::indexing::tantivy_indexer::TantivyIndexer;

pub struct TreeSitterIndexer {
    parsers: HashMap<String, Parser>,
    queries: HashMap<String, String>,
    normalizer: TextNormalizer,
    tantivy_indexer: Option<TantivyIndexer>,  // NEW
}

impl TreeSitterIndexer {
    pub fn index_codebase(&mut self, root_path: &str) -> Result<CodebaseIndex, String> {
        // ... existing code ...

        // Add to Tantivy index
        if let Some(ref mut tantivy) = self.tantivy_indexer {
            for symbol in &indexed_file.symbols {
                if let Err(e) = tantivy.add_symbol(
                    symbol,
                    &indexed_file.language,
                ) {
                    eprintln!("Failed to add symbol to Tantivy: {}", e);
                }
            }
        }

        // Commit Tantivy index
        if let Some(ref mut tantivy) = self.tantivy_indexer {
            tantivy.commit()?;
        }

        Ok(index)
    }
}
```

### 2.4 Estimated Complexity
- **Lines of Code:** ~800-1000
- **New Dependencies:** 2 (tantivy, tempfile)
- **Breaking Changes:** None (opt-in feature)
- **Performance Impact:**
  - Indexing: +30-50% time
  - Search: 10-50x faster for content searches
  - Memory: +20-50MB for index

---

## Phase 3: Semantic Search (Embeddings)

**Goal:** Add semantic understanding using local embedding models
**Duration:** 7-10 days
**Complexity:** High
**Lines of Code:** ~1000-1200

### 3.1 Dependencies

```toml
# In src-tauri/Cargo.toml
[dependencies]
# Existing...

# ML/Embeddings
candle-core = "0.8"             # ML framework (from HuggingFace)
candle-nn = "0.8"               # Neural network layers
candle-transformers = "0.8"     # Transformer models
tokenizers = "0.15"             # Tokenization
hf-hub = "0.3"                  # Model downloading
ndarray = "0.15"                # Array operations

# Vector storage
usearch = "2.15"                # Fast vector search (HNSW)

# Serialization
bincode = "1.3"                 # Efficient binary serialization
```

### 3.2 Embedding Model Selection

**Recommendation: all-MiniLM-L6-v2**
- **Size:** ~23MB
- **Dimensions:** 384
- **Speed:** ~1000 embeddings/sec on CPU
- **Quality:** Good balance for code search
- **License:** Apache 2.0

**Alternative: CodeBERT-base**
- **Size:** ~500MB
- **Dimensions:** 768
- **Speed:** ~300 embeddings/sec on CPU
- **Quality:** Better for code-specific tasks

### 3.3 Embedding Generator Implementation

```rust
// NEW FILE: src-tauri/src/indexing/embedding_generator.rs
use candle_core::{Device, Tensor};
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use tokenizers::Tokenizer;
use hf_hub::{api::sync::Api, Repo, RepoType};

pub struct EmbeddingGenerator {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    embedding_dim: usize,
}

impl EmbeddingGenerator {
    /// Initialize with all-MiniLM-L6-v2 model
    pub fn new() -> Result<Self, String> {
        let device = Device::Cpu;  // Use CPU for compatibility

        // Download model from HuggingFace Hub
        let api = Api::new()
            .map_err(|e| format!("Failed to create API: {}", e))?;

        let repo = api.repo(Repo::new(
            "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            RepoType::Model,
        ));

        // Download model files
        let model_path = repo.get("model.safetensors")
            .map_err(|e| format!("Failed to download model: {}", e))?;
        let config_path = repo.get("config.json")
            .map_err(|e| format!("Failed to download config: {}", e))?;
        let tokenizer_path = repo.get("tokenizer.json")
            .map_err(|e| format!("Failed to download tokenizer: {}", e))?;

        // Load config and model
        let config = std::fs::read_to_string(config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        let config: BertConfig = serde_json::from_str(&config)
            .map_err(|e| format!("Failed to parse config: {}", e))?;

        let vb = candle_nn::VarBuilder::from_pth(&model_path, candle_core::DType::F32, &device)
            .map_err(|e| format!("Failed to load model: {}", e))?;
        let model = BertModel::load(vb, &config)
            .map_err(|e| format!("Failed to create model: {}", e))?;

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| format!("Failed to load tokenizer: {}", e))?;

        Ok(Self {
            model,
            tokenizer,
            device,
            embedding_dim: config.hidden_size,
        })
    }

    /// Generate embedding for a single text
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let tokens = self.tokenizer.encode(text, true)
            .map_err(|e| format!("Tokenization failed: {}", e))?;

        let token_ids = tokens.get_ids();
        let token_tensor = Tensor::new(token_ids, &self.device)
            .map_err(|e| format!("Failed to create tensor: {}", e))?
            .unsqueeze(0)
            .map_err(|e| format!("Failed to unsqueeze: {}", e))?;

        // Get model output
        let output = self.model.forward(&token_tensor)
            .map_err(|e| format!("Model forward failed: {}", e))?;

        // Mean pooling over token dimension
        let embedding = self.mean_pooling(&output)
            .map_err(|e| format!("Pooling failed: {}", e))?;

        // Normalize
        let normalized = self.normalize_embedding(&embedding)
            .map_err(|e| format!("Normalization failed: {}", e))?;

        // Convert to Vec<f32>
        normalized.to_vec1()
            .map_err(|e| format!("Failed to convert to vec: {}", e))
    }

    /// Generate embeddings for multiple texts (batch processing)
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        texts.iter()
            .map(|text| self.embed(text))
            .collect()
    }
}

/// Helper to generate text representation from code symbol
pub fn symbol_to_text(symbol: &CodeSymbol) -> String {
    let mut parts = Vec::new();

    // Add symbol name
    parts.push(symbol.name.clone());

    // Add kind
    parts.push(format!("{:?}", symbol.kind).to_lowercase());

    // Add doc comment if available (most semantic information)
    if let Some(ref doc) = symbol.doc_comment {
        parts.push(doc.clone());
    }

    // Add signature (truncated to avoid overwhelming the model)
    if let Some(ref sig) = symbol.signature {
        let truncated = if sig.len() > 200 {
            &sig[..200]
        } else {
            sig
        };
        parts.push(truncated.to_string());
    }

    parts.join(" ")
}
```

### 3.4 Vector Storage Implementation

```rust
// NEW FILE: src-tauri/src/indexing/vector_store.rs
use usearch::Index as UsearchIndex;
use usearch::ScalarKind;
use serde::{Serialize, Deserialize};

pub struct VectorStore {
    index: UsearchIndex,
    metadata: Vec<VectorMetadata>,
    dimension: usize,
}

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

impl VectorStore {
    pub fn new(dimension: usize) -> Result<Self, String> {
        // Create HNSW index
        let index = UsearchIndex::new(
            &usearch::IndexOptions {
                dimensions: dimension,
                metric: usearch::MetricKind::Cos,  // Cosine similarity
                quantization: ScalarKind::F32,
                connectivity: 16,
                expansion_add: 128,
                expansion_search: 64,
                multi: false,
            }
        ).map_err(|e| format!("Failed to create index: {}", e))?;

        Ok(Self {
            index,
            metadata: Vec::new(),
            dimension,
        })
    }

    /// Add a vector with associated metadata
    pub fn add(
        &mut self,
        vector: &[f32],
        metadata: VectorMetadata,
    ) -> Result<usize, String> {
        if vector.len() != self.dimension {
            return Err(format!(
                "Vector dimension mismatch: expected {}, got {}",
                self.dimension,
                vector.len()
            ));
        }

        let key = self.metadata.len();

        self.index.add(key, vector)
            .map_err(|e| format!("Failed to add vector: {}", e))?;

        self.metadata.push(metadata);

        Ok(key)
    }

    /// Search for k nearest neighbors
    pub fn search(
        &self,
        query_vector: &[f32],
        k: usize,
    ) -> Result<Vec<SearchResult>, String> {
        let matches = self.index.search(query_vector, k)
            .map_err(|e| format!("Search failed: {}", e))?;

        let mut results = Vec::new();
        for m in matches.keys {
            if let Some(metadata) = self.metadata.get(m.key) {
                results.push(SearchResult {
                    metadata: metadata.clone(),
                    similarity: m.distance,
                });
            }
        }

        Ok(results)
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub metadata: VectorMetadata,
    pub similarity: f32,
}
```

### 3.5 Integration

```rust
// In src-tauri/src/indexing/tree_sitter_indexer.rs
pub struct TreeSitterIndexer {
    parsers: HashMap<String, Parser>,
    queries: HashMap<String, String>,
    normalizer: TextNormalizer,
    tantivy_indexer: Option<TantivyIndexer>,

    // NEW: Semantic search components
    embedding_generator: Option<EmbeddingGenerator>,
    vector_store: Option<VectorStore>,
}

impl TreeSitterIndexer {
    pub fn index_codebase(&mut self, root_path: &str) -> Result<CodebaseIndex, String> {
        // ... existing code ...

        // Generate embeddings in batches
        if let (Some(ref generator), Some(ref mut vector_store)) =
            (&self.embedding_generator, &mut self.vector_store)
        {
            println!("Generating embeddings for {} symbols...", symbols_to_embed.len());

            const BATCH_SIZE: usize = 100;
            for chunk in symbols_to_embed.chunks(BATCH_SIZE) {
                let texts: Vec<String> = chunk.iter()
                    .map(|(sym, _)| symbol_to_text(sym))
                    .collect();

                match generator.embed_batch(&texts) {
                    Ok(embeddings) => {
                        for (i, embedding) in embeddings.iter().enumerate() {
                            let (symbol, language) = &chunk[i];

                            let metadata = VectorMetadata {
                                symbol_name: symbol.name.clone(),
                                file_path: symbol.file_path.clone(),
                                language: language.clone(),
                                start_line: symbol.start_line,
                                end_line: symbol.end_line,
                                signature: symbol.signature.clone(),
                                doc_comment: symbol.doc_comment.clone(),
                            };

                            if let Err(e) = vector_store.add(embedding, metadata) {
                                eprintln!("Failed to add vector: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Batch embedding failed: {}", e);
                    }
                }
            }
        }

        Ok(index)
    }

    /// Semantic search using embeddings
    pub fn search_semantic(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<CodeChunk>, String> {
        let generator = self.embedding_generator.as_ref()
            .ok_or_else(|| "Embedding generator not available".to_string())?;

        let vector_store = self.vector_store.as_ref()
            .ok_or_else(|| "Vector store not available".to_string())?;

        // Generate query embedding
        let query_embedding = generator.embed(query)?;

        // Search vector store
        let results = vector_store.search(&query_embedding, max_results)?;

        // Convert to CodeChunk
        Ok(results.into_iter()
            .map(|r| CodeChunk {
                file_path: r.metadata.file_path,
                start_line: r.metadata.start_line,
                end_line: r.metadata.end_line,
                content: r.metadata.signature.unwrap_or_default(),
                language: r.metadata.language,
                symbols: vec![r.metadata.symbol_name],
                relevance_score: r.similarity,
            })
            .collect())
    }
}
```

### 3.6 Performance Considerations

**Indexing:**
- Batch embedding generation: ~1000 symbols/second on CPU
- For 10,000 symbols: ~10 seconds
- HNSW index building: Very fast (< 1 second)

**Search:**
- Vector search: < 10ms for typical queries
- Scales to 100k+ vectors easily

**Memory:**
- Model: ~23MB (all-MiniLM-L6-v2)
- Vectors: 384 * 4 bytes * N symbols = ~1.5KB per symbol
- For 10,000 symbols: ~15MB

### 3.7 Estimated Complexity
- **Lines of Code:** ~1000-1200
- **New Dependencies:** 7 (candle-*, tokenizers, hf-hub, usearch, bincode, ndarray)
- **Breaking Changes:** None (opt-in feature)
- **Performance Impact:**
  - Indexing: +50-100% time (embedding generation)
  - Search: Same or faster (vector search is very efficient)
  - Memory: +40-60MB (model + vectors)

---

## Phase 4: Hybrid Search

**Goal:** Combine all three search methods with intelligent result fusion
**Duration:** 3-5 days
**Complexity:** Medium
**Lines of Code:** ~400-500

### 4.1 Reciprocal Rank Fusion (RRF)

```rust
// NEW FILE: src-tauri/src/indexing/hybrid_search.rs
use crate::models::code_index::CodeChunk;
use std::collections::HashMap;

pub struct HybridSearcher;

impl HybridSearcher {
    /// Perform hybrid search combining multiple methods
    pub fn search(
        &self,
        traditional_results: Vec<CodeChunk>,
        full_text_results: Vec<CodeChunk>,
        semantic_results: Vec<CodeChunk>,
        config: &HybridConfig,
    ) -> Vec<CodeChunk> {
        // Apply Reciprocal Rank Fusion
        let fused_results = self.reciprocal_rank_fusion(
            &[
                (traditional_results, config.traditional_weight),
                (full_text_results, config.full_text_weight),
                (semantic_results, config.semantic_weight),
            ],
            config.rrf_k,
        );

        // Take top results
        let results: Vec<_> = fused_results.into_iter()
            .take(config.max_results)
            .collect();

        results
    }

    /// Reciprocal Rank Fusion algorithm
    /// RRF score for document d: sum over all rankers r of: weight_r / (k + rank_r(d))
    fn reciprocal_rank_fusion(
        &self,
        result_lists: &[(Vec<CodeChunk>, f32)],
        k: f32,
    ) -> Vec<CodeChunk> {
        let mut scores: HashMap<String, (f32, CodeChunk)> = HashMap::new();

        for (results, weight) in result_lists {
            for (rank, chunk) in results.iter().enumerate() {
                // Create unique key for deduplication
                let key = format!(
                    "{}:{}:{}",
                    chunk.file_path,
                    chunk.start_line,
                    chunk.end_line
                );

                // Calculate RRF score contribution
                let rrf_score = weight / (k + (rank as f32 + 1.0));

                scores.entry(key)
                    .and_modify(|(score, _)| *score += rrf_score)
                    .or_insert((rrf_score, chunk.clone()));
            }
        }

        // Convert to vector and sort by score
        let mut results: Vec<_> = scores.into_iter()
            .map(|(_, (score, mut chunk))| {
                chunk.relevance_score = score;
                chunk
            })
            .collect();

        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }
}

#[derive(Debug, Clone)]
pub struct HybridConfig {
    pub traditional_weight: f32,  // Weight for HashMap search
    pub full_text_weight: f32,    // Weight for Tantivy search
    pub semantic_weight: f32,     // Weight for vector search
    pub rrf_k: f32,               // RRF parameter (default: 60)
    pub max_results: usize,       // Final result limit
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            traditional_weight: 0.2,
            full_text_weight: 0.4,
            semantic_weight: 0.4,
            rrf_k: 60.0,
            max_results: 50,
        }
    }
}
```

### 4.2 Query Type Detection

```rust
// NEW FILE: src-tauri/src/indexing/query_analyzer.rs
pub struct QueryAnalyzer;

#[derive(Debug, PartialEq)]
pub enum QueryType {
    ExactSymbol,      // "AuthenticationService"
    FilePath,         // "src/auth/login.rs"
    SemanticIntent,   // "how to handle user authentication"
    CodeContent,      // "async fn authenticate"
    Mixed,
}

impl QueryAnalyzer {
    pub fn analyze_query(query: &str) -> QueryType {
        let lower = query.to_lowercase();
        let words: Vec<&str> = query.split_whitespace().collect();

        // Check for file path patterns
        if query.contains('/') || query.contains('\\') || query.ends_with(".rs")
            || query.ends_with(".ts") || query.ends_with(".js")
        {
            return QueryType::FilePath;
        }

        // Check for semantic patterns
        if lower.starts_with("how") || lower.starts_with("what")
            || lower.contains("how to")
        {
            return QueryType::SemanticIntent;
        }

        // Check for code patterns
        if query.contains("fn ") || query.contains("async ")
        {
            return QueryType::CodeContent;
        }

        // Single CamelCase or snake_case word likely a symbol
        if words.len() == 1 {
            return QueryType::ExactSymbol;
        }

        QueryType::Mixed
    }

    /// Get appropriate hybrid config based on query type
    pub fn get_config_for_query(query_type: &QueryType) -> HybridConfig {
        match query_type {
            QueryType::ExactSymbol => HybridConfig {
                traditional_weight: 0.7,
                full_text_weight: 0.2,
                semantic_weight: 0.1,
                ..Default::default()
            },
            QueryType::SemanticIntent => HybridConfig {
                traditional_weight: 0.1,
                full_text_weight: 0.2,
                semantic_weight: 0.7,
                ..Default::default()
            },
            QueryType::CodeContent => HybridConfig {
                traditional_weight: 0.1,
                full_text_weight: 0.6,
                semantic_weight: 0.3,
                ..Default::default()
            },
            _ => HybridConfig::default(),
        }
    }
}
```

### 4.3 Unified Query Interface

```rust
// In src-tauri/src/indexing/tree_sitter_indexer.rs
use crate::indexing::hybrid_search::{HybridSearcher, HybridConfig};
use crate::indexing::query_analyzer::QueryAnalyzer;

impl TreeSitterIndexer {
    /// Main query method - automatically uses hybrid search
    pub fn query_index(
        &self,
        index: &CodebaseIndex,
        query: &IndexQuery,
    ) -> Vec<CodeChunk> {
        // Analyze query to determine best strategy
        let query_text = query.keywords.join(" ");
        let query_type = QueryAnalyzer::analyze_query(&query_text);
        let config = QueryAnalyzer::get_config_for_query(&query_type);

        // Execute all search methods
        let traditional_results = self.query_traditional(index, query);

        let full_text_results = if self.tantivy_indexer.is_some() {
            self.query_full_text(query)
        } else {
            Vec::new()
        };

        let semantic_results = if self.embedding_generator.is_some() {
            self.search_semantic(&query_text, config.max_results)
                .unwrap_or_else(|_| Vec::new())
        } else {
            Vec::new()
        };

        // Combine using hybrid search
        let hybrid_searcher = HybridSearcher;
        hybrid_searcher.search(
            traditional_results,
            full_text_results,
            semantic_results,
            &config,
        )
    }
}
```

### 4.4 Estimated Complexity
- **Lines of Code:** ~400-500
- **New Dependencies:** 0 (uses existing)
- **Breaking Changes:** None (backward compatible)
- **Performance Impact:**
  - Search: 1.5-2x slower than single method (runs multiple searches)
  - But results are much better quality

---

## Summary: Implementation Roadmap

### Timeline

| Phase | Duration | Complexity | LOC | Dependencies |
|-------|----------|------------|-----|--------------|
| Phase 1: Quick Fixes | 2-3 days | Low | ~700 | 2 |
| Phase 2: Full-Text | 5-7 days | Medium-High | ~1000 | 2 |
| Phase 3: Semantic | 7-10 days | High | ~1200 | 7 |
| Phase 4: Hybrid | 3-5 days | Medium | ~500 | 0 |
| **Total** | **17-25 days** | - | **~3400** | **11** |

### Phase Outcomes

**Phase 1:** Improved basic search, foundation for advanced features
- Solves the immediate "indexing" query problem
- ~70% improvement in search quality

**Phase 2:** Search in code content, comments, signatures
- Full-text search capabilities
- 10-50x faster content searches

**Phase 3:** Understand query intent, find conceptually similar code
- Semantic understanding
- Find related code without exact keyword matches

**Phase 4:** Best of all worlds, intelligent result merging
- Production-quality search
- ~92% search quality target

---

## Testing Strategy

### Unit Tests
- Each module has comprehensive tests
- Test edge cases, error handling
- Aim for >80% code coverage

### Integration Tests
- Test full indexing pipeline
- Test all search methods
- Test result quality

### Performance Tests
- Benchmark indexing speed (target: <5min for 10k files)
- Benchmark search speed (target: <100ms per query)
- Memory usage monitoring

### Quality Tests
- Measure search relevance (NDCG, MRR metrics)
- Compare hybrid vs individual methods
- User acceptance testing

---

## Performance Targets

| Metric | Current | Phase 1 | Phase 2 | Phase 3 | Phase 4 |
|--------|---------|---------|---------|---------|---------|
| Index 10k files | 30s | 35s | 45s | 90s | 90s |
| Memory usage | 50MB | 55MB | 75MB | 130MB | 130MB |
| Simple query | 50ms | 30ms | 20ms | 25ms | 40ms |
| Complex query | 100ms | 60ms | 30ms | 35ms | 50ms |
| Search quality | 60% | 75% | 80% | 85% | 92% |

---

## Migration Path

### For Existing Users
1. **Phase 1:** Transparent upgrade, no API changes
2. **Phase 2:** Opt-in via `use_full_text` flag
3. **Phase 3:** Automatic if embedding model available
4. **Phase 4:** Automatic with configurable weights

### Backward Compatibility
- All phases maintain existing API
- Features degrade gracefully if unavailable
- No breaking changes to data structures

---

## Risk Mitigation

### Technical Risks
1. **Model availability:** Some systems may not support ML libraries
   - **Mitigation:** Make embeddings optional, graceful degradation

2. **Memory constraints:** Embeddings increase memory usage
   - **Mitigation:** Lazy loading, configurable batch sizes

3. **Build complexity:** More dependencies = harder builds
   - **Mitigation:** Feature flags, optional dependencies

### Performance Risks
1. **Indexing too slow:** Embedding generation adds time
   - **Mitigation:** Background indexing, incremental updates

2. **Search latency:** Running 3 searches is slower
   - **Mitigation:** Parallel execution, caching, query routing

---

## Future Enhancements (Post Phase 4)

1. **Incremental Indexing:** Only re-index changed files
2. **Index Persistence:** Save index to disk, fast startup
3. **Cross-file Context:** Understanding imports/exports
4. **Query Suggestions:** Auto-complete and query expansion
5. **User Feedback Loop:** Learn from user interactions
6. **GPU Acceleration:** Faster embeddings with CUDA
7. **Graph-based Search:** Use call graphs and dependency graphs

---

## Appendix A: Complete Dependency List

```toml
[dependencies]
# Core (existing)
tauri = "2.0"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }

# Tree-sitter (existing)
tree-sitter = "0.22"
tree-sitter-rust = "0.21"
tree-sitter-javascript = "0.21"
tree-sitter-typescript = "0.21"
tree-sitter-python = "0.21"
walkdir = "2"
ignore = "0.4"

# Phase 1: Text processing
unicode-segmentation = "1.10"
rust-stemmers = "1.2"

# Phase 2: Full-text search
tantivy = "0.22"
tempfile = "3.8"

# Phase 3: ML/Embeddings
candle-core = "0.8"
candle-nn = "0.8"
candle-transformers = "0.8"
tokenizers = "0.15"
hf-hub = "0.3"
ndarray = "0.15"
usearch = "2.15"
bincode = "1.3"

[dev-dependencies]
criterion = "0.5"
```

---

## Appendix B: File Structure

```
src-tauri/src/
├── main.rs
├── commands/
│   ├── mod.rs
│   ├── index_commands.rs
│   └── anthropic_commands.rs
├── indexing/
│   ├── mod.rs
│   ├── tree_sitter_indexer.rs       (modified in all phases)
│   ├── text_normalizer.rs           (NEW - Phase 1)
│   ├── relevance_scorer.rs          (NEW - Phase 1)
│   ├── tantivy_indexer.rs           (NEW - Phase 2)
│   ├── embedding_generator.rs       (NEW - Phase 3)
│   ├── vector_store.rs              (NEW - Phase 3)
│   ├── hybrid_search.rs             (NEW - Phase 4)
│   └── query_analyzer.rs            (NEW - Phase 4)
├── models/
│   ├── mod.rs
│   └── code_index.rs                (modified in all phases)
└── anthropic/
    ├── mod.rs
    └── models.rs
```

---

This implementation plan provides a clear, step-by-step roadmap for upgrading the prompto indexing system from basic HashMap search to a sophisticated hybrid search system. Each phase builds on the previous one while maintaining backward compatibility, allowing for incremental deployment and testing.
