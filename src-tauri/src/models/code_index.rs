use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a code symbol (function, class, method, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
    pub parent: Option<String>, // For nested symbols
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Interface,
    Enum,
    Constant,
    Variable,
    Import,
    Export,
}

/// Represents a file in the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFile {
    pub path: String,
    pub language: String,
    pub symbols: Vec<CodeSymbol>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub last_modified: u64,
}

/// The main index structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseIndex {
    pub root_path: String,
    pub files: HashMap<String, IndexedFile>,
    pub symbol_map: HashMap<String, Vec<CodeSymbol>>, // Quick lookup by symbol name
    pub language_stats: HashMap<String, usize>, // File count per language
    pub total_files: usize,
    pub indexed_at: u64,
}

impl CodebaseIndex {
    pub fn new(root_path: String) -> Self {
        Self {
            root_path,
            files: HashMap::new(),
            symbol_map: HashMap::new(),
            language_stats: HashMap::new(),
            total_files: 0,
            indexed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn add_file(&mut self, file: IndexedFile) {
        // Update language stats
        *self.language_stats.entry(file.language.clone()).or_insert(0) += 1;
        self.total_files += 1;

        // Add symbols to symbol map
        for symbol in &file.symbols {
            self.symbol_map
                .entry(symbol.name.clone())
                .or_insert_with(Vec::new)
                .push(symbol.clone());
        }

        // Store indexed file
        self.files.insert(file.path.clone(), file);
    }
}

/// Result of indexing operation
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexResult {
    pub success: bool,
    pub total_files: usize,
    pub total_symbols: usize,
    pub languages: Vec<String>,
    pub duration_ms: u64,
    pub errors: Vec<String>,
}

/// Code chunk for context injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub language: String,
    pub symbols: Vec<String>, // Symbol names in this chunk
    pub relevance_score: f32, // For ranking
}

/// Query request from frontend
#[derive(Debug, Deserialize)]
pub struct IndexQuery {
    pub keywords: Vec<String>,
    #[serde(default)]
    pub symbol_kinds: Option<Vec<SymbolKind>>,
    #[serde(default)]
    pub file_patterns: Option<Vec<String>>,
    #[serde(default)]
    pub max_results: Option<usize>,
}
