use crate::models::code_index::{CodeSymbol, SymbolKind};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexWriter, ReloadPolicy};

/// Result from a Tantivy full-text search
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Tantivy-based full-text search indexer
pub struct TantivyIndexer {
    index: Index,
    writer: IndexWriter,
    schema: Schema,
    // Field handles for fast access
    symbol_name: Field,
    file_path: Field,
    language: Field,
    symbol_kind: Field,
    signature: Field,
    doc_comment: Field,
    start_line: Field,
    end_line: Field,
    index_dir: PathBuf, // Keep track of index directory
}

impl TantivyIndexer {
    /// Create a new Tantivy indexer with schema in the specified directory
    pub fn new<P: Into<PathBuf>>(index_dir: P) -> Result<Self, String> {
        let index_dir = index_dir.into();

        // Build schema with 8 fields
        let mut schema_builder = Schema::builder();

        let symbol_name = schema_builder.add_text_field("symbol_name", TEXT | STORED);
        let file_path = schema_builder.add_text_field("file_path", TEXT | STORED);
        let language = schema_builder.add_text_field("language", STRING | STORED);
        let symbol_kind = schema_builder.add_text_field("symbol_kind", STRING | STORED);
        let signature = schema_builder.add_text_field("signature", TEXT | STORED);
        let doc_comment = schema_builder.add_text_field("doc_comment", TEXT | STORED);
        let start_line = schema_builder.add_u64_field("start_line", STORED);
        let end_line = schema_builder.add_u64_field("end_line", STORED);

        let schema = schema_builder.build();

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&index_dir)
            .map_err(|e| format!("Failed to create index directory: {}", e))?;

        // Create or open index in persistent directory
        let index = if index_dir.join("meta.json").exists() {
            // Index exists, open it
            Index::open_in_dir(&index_dir)
                .map_err(|e| format!("Failed to open index: {}", e))?
        } else {
            // Create new index
            Index::create_in_dir(&index_dir, schema.clone())
                .map_err(|e| format!("Failed to create index: {}", e))?
        };

        // Create index writer with 50MB buffer
        let writer = index
            .writer(50_000_000)
            .map_err(|e| format!("Failed to create writer: {}", e))?;

        Ok(Self {
            index,
            writer,
            schema,
            symbol_name,
            file_path,
            language,
            symbol_kind,
            signature,
            doc_comment,
            start_line,
            end_line,
            index_dir,
        })
    }

    /// Load an existing index from disk
    pub fn load<P: Into<PathBuf>>(index_dir: P) -> Result<Self, String> {
        Self::new(index_dir)
    }

    /// Clear the index directory (for re-indexing)
    pub fn clear(&mut self) -> Result<(), String> {
        // Delete and recreate the index
        let _ = std::fs::remove_dir_all(&self.index_dir);
        std::fs::create_dir_all(&self.index_dir)
            .map_err(|e| format!("Failed to recreate index directory: {}", e))?;

        // Recreate the index
        let index = Index::create_in_dir(&self.index_dir, self.schema.clone())
            .map_err(|e| format!("Failed to create index: {}", e))?;

        let writer = index
            .writer(50_000_000)
            .map_err(|e| format!("Failed to create writer: {}", e))?;

        self.index = index;
        self.writer = writer;

        Ok(())
    }

    /// Add a symbol to the full-text index
    pub fn add_symbol(&mut self, symbol: &CodeSymbol, language: &str) -> Result<(), String> {
        let kind_str = match symbol.kind {
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Class => "class",
            SymbolKind::Struct => "struct",
            SymbolKind::Interface => "interface",
            SymbolKind::Enum => "enum",
            SymbolKind::Constant => "constant",
            SymbolKind::Variable => "variable",
            SymbolKind::Import => "import",
            SymbolKind::Export => "export",
        };

        let mut doc = doc!(
            self.symbol_name => symbol.name.clone(),
            self.file_path => symbol.file_path.clone(),
            self.language => language.to_string(),
            self.symbol_kind => kind_str.to_string(),
            self.start_line => symbol.start_line as u64,
            self.end_line => symbol.end_line as u64,
        );

        // Add optional fields
        if let Some(ref sig) = symbol.signature {
            doc.add_text(self.signature, sig);
        }

        if let Some(ref comment) = symbol.doc_comment {
            doc.add_text(self.doc_comment, comment);
        }

        self.writer
            .add_document(doc)
            .map_err(|e| format!("Failed to add document: {}", e))?;

        Ok(())
    }

    /// Commit all pending writes
    pub fn commit(&mut self) -> Result<(), String> {
        self.writer
            .commit()
            .map_err(|e| format!("Failed to commit: {}", e))?;
        Ok(())
    }

    /// Search the index with a query string
    pub fn search(
        &self,
        query_str: &str,
        limit: usize,
    ) -> Result<Vec<TantivySearchResult>, String> {
        // Get a reader
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| format!("Failed to create reader: {}", e))?;

        let searcher = reader.searcher();

        // Build query parser for multiple fields
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.symbol_name,
                self.file_path,
                self.signature,
                self.doc_comment,
            ],
        );

        // Parse query
        let query = query_parser
            .parse_query(query_str)
            .map_err(|e| format!("Failed to parse query: {}", e))?;

        // Search
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| format!("Search failed: {}", e))?;

        // Convert results
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| format!("Failed to retrieve doc: {}", e))?;

            let symbol_name = retrieved_doc
                .get_first(self.symbol_name)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let file_path = retrieved_doc
                .get_first(self.file_path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let language = retrieved_doc
                .get_first(self.language)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let symbol_kind = retrieved_doc
                .get_first(self.symbol_kind)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let signature = retrieved_doc
                .get_first(self.signature)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let doc_comment = retrieved_doc
                .get_first(self.doc_comment)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let start_line = retrieved_doc
                .get_first(self.start_line)
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let end_line = retrieved_doc
                .get_first(self.end_line)
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            results.push(TantivySearchResult {
                symbol_name,
                file_path,
                language,
                symbol_kind,
                signature,
                doc_comment,
                start_line,
                end_line,
                score,
            });
        }

        Ok(results)
    }
}
