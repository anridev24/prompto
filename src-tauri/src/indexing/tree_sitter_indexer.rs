use crate::models::code_index::*;
use ignore::WalkBuilder;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor};

// Import language parsers
extern "C" {
    fn tree_sitter_rust() -> Language;
    fn tree_sitter_javascript() -> Language;
    fn tree_sitter_typescript() -> Language;
    fn tree_sitter_python() -> Language;
}

pub struct TreeSitterIndexer {
    parsers: HashMap<String, Parser>,
    queries: HashMap<String, String>,
}

impl TreeSitterIndexer {
    pub fn new() -> Result<Self, String> {
        let mut indexer = TreeSitterIndexer {
            parsers: HashMap::new(),
            queries: HashMap::new(),
        };

        // Initialize parsers for each language
        indexer.init_parser("rust", unsafe { tree_sitter_rust() })?;
        indexer.init_parser("javascript", unsafe { tree_sitter_javascript() })?;
        indexer.init_parser("typescript", unsafe { tree_sitter_typescript() })?;
        indexer.init_parser("python", unsafe { tree_sitter_python() })?;

        // Initialize queries for symbol extraction
        indexer.init_queries();

        Ok(indexer)
    }

    fn init_parser(&mut self, lang: &str, language: Language) -> Result<(), String> {
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to set language {}: {}", lang, e))?;
        self.parsers.insert(lang.to_string(), parser);
        Ok(())
    }

    fn init_queries(&mut self) {
        // For now, we'll use a simpler approach - identify symbols by node type
        // In a production app, you'd use more sophisticated tree-sitter queries

        // Rust query patterns
        self.queries.insert("rust".to_string(), "function_item,struct_item,impl_item,enum_item,use_declaration".to_string());

        // TypeScript/JavaScript query patterns
        self.queries.insert("typescript".to_string(), "function_declaration,class_declaration,method_definition,import_statement,export_statement".to_string());
        self.queries.insert("javascript".to_string(), "function_declaration,class_declaration,method_definition,import_statement,export_statement".to_string());

        // Python query patterns
        self.queries.insert("python".to_string(), "function_definition,class_definition,import_statement,import_from_statement".to_string());
    }

    /// Main indexing function
    pub fn index_codebase(&mut self, root_path: &str) -> Result<CodebaseIndex, String> {
        let start_time = std::time::Instant::now();
        let mut index = CodebaseIndex::new(root_path.to_string());

        // Walk directory respecting .gitignore
        let walker = WalkBuilder::new(root_path)
            .hidden(false)
            .git_ignore(true)
            .git_exclude(true)
            .build();

        for entry in walker.filter_map(Result::ok) {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            // Determine language from extension
            if let Some(language) = self.detect_language(path) {
                match self.index_file(path, &language) {
                    Ok(indexed_file) => {
                        index.add_file(indexed_file);
                    }
                    Err(e) => {
                        eprintln!("Failed to index {}: {}", path.display(), e);
                    }
                }
            }
        }

        println!(
            "Indexed {} files in {:?}",
            index.total_files,
            start_time.elapsed()
        );

        Ok(index)
    }

    /// Index a single file
    fn index_file(&mut self, path: &Path, language: &str) -> Result<IndexedFile, String> {
        let source_code = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        let parser = self
            .parsers
            .get_mut(language)
            .ok_or_else(|| format!("No parser for language: {}", language))?;

        let tree = parser
            .parse(&source_code, None)
            .ok_or_else(|| format!("Failed to parse {}", path.display()))?;

        let symbols = self.extract_symbols(&tree, &source_code, language, path);
        let imports = self.extract_imports(tree.root_node(), &source_code, language);

        Ok(IndexedFile {
            path: path.to_string_lossy().to_string(),
            language: language.to_string(),
            symbols,
            imports,
            exports: Vec::new(),
            last_modified: fs::metadata(path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
        })
    }

    /// Extract symbols using tree-sitter queries
    fn extract_symbols(
        &self,
        tree: &tree_sitter::Tree,
        source_code: &str,
        language: &str,
        file_path: &Path,
    ) -> Vec<CodeSymbol> {
        let mut symbols = Vec::new();
        let root = tree.root_node();

        // Get relevant node types for this language
        let node_types = self.queries.get(language);
        if node_types.is_none() {
            return symbols;
        }

        // Walk the tree and find matching nodes
        let mut cursor = root.walk();
        self.visit_node(root, &mut symbols, source_code, file_path, language);

        symbols
    }

    fn visit_node(
        &self,
        node: Node,
        symbols: &mut Vec<CodeSymbol>,
        source_code: &str,
        file_path: &Path,
        language: &str,
    ) {
        // Check if this node type is a symbol we care about
        let symbol = match node.kind() {
            "function_item" | "function_declaration" | "function_definition" => {
                self.create_symbol(node, source_code, file_path, SymbolKind::Function)
            }
            "struct_item" => {
                self.create_symbol(node, source_code, file_path, SymbolKind::Struct)
            }
            "class_declaration" | "class_definition" => {
                self.create_symbol(node, source_code, file_path, SymbolKind::Class)
            }
            "method_definition" => {
                self.create_symbol(node, source_code, file_path, SymbolKind::Method)
            }
            "enum_item" => {
                self.create_symbol(node, source_code, file_path, SymbolKind::Enum)
            }
            "impl_item" => {
                self.create_symbol(node, source_code, file_path, SymbolKind::Interface)
            }
            _ => None,
        };

        if let Some(s) = symbol {
            symbols.push(s);
        }

        // Visit children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(child, symbols, source_code, file_path, language);
        }
    }

    fn create_symbol(
        &self,
        node: Node,
        source_code: &str,
        file_path: &Path,
        kind: SymbolKind,
    ) -> Option<CodeSymbol> {
        let name = self.extract_name_from_node(node, source_code)?;
        let start = node.start_position();
        let end = node.end_position();

        // Get the full text of the node (limited to reasonable size)
        let text = &source_code[node.byte_range()];
        let signature = if text.len() > 500 {
            Some(text.chars().take(500).collect::<String>() + "...")
        } else {
            Some(text.to_string())
        };

        Some(CodeSymbol {
            name,
            kind,
            file_path: file_path.to_string_lossy().to_string(),
            start_line: start.row + 1,
            end_line: end.row + 1,
            signature,
            doc_comment: None,
            parent: None,
        })
    }

    fn extract_name_from_node(&self, node: Node, source_code: &str) -> Option<String> {
        // Find identifier child node
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            if kind == "identifier" || kind == "type_identifier" || kind.contains("name") {
                return Some(source_code[child.byte_range()].to_string());
            }
        }
        None
    }

    fn extract_imports(
        &self,
        node: Node,
        source_code: &str,
        _language: &str,
    ) -> Vec<String> {
        let mut imports = Vec::new();
        let mut cursor = node.walk();

        fn visit_for_imports(node: Node, imports: &mut Vec<String>, source_code: &str) {
            let kind = node.kind();
            if kind == "use_declaration"
                || kind == "import_statement"
                || kind == "import_from_statement"
            {
                let text = &source_code[node.byte_range()];
                imports.push(text.to_string());
            }

            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                visit_for_imports(child, imports, source_code);
            }
        }

        visit_for_imports(node, &mut imports, source_code);
        imports
    }

    fn detect_language(&self, path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext {
                "rs" => Some("rust"),
                "js" | "jsx" => Some("javascript"),
                "ts" | "tsx" => Some("typescript"),
                "py" => Some("python"),
                _ => None,
            })
            .map(String::from)
    }

    /// Query the index for relevant code chunks
    pub fn query_index(
        &self,
        index: &CodebaseIndex,
        query: &IndexQuery,
    ) -> Vec<CodeChunk> {
        let mut results = Vec::new();
        let max_results = query.max_results.unwrap_or(50);

        // Search for symbols matching keywords
        for keyword in &query.keywords {
            // Exact match
            if let Some(symbols) = index.symbol_map.get(keyword) {
                for symbol in symbols {
                    results.push(self.symbol_to_chunk(symbol, &index.files));
                }
            }

            // Partial match (contains keyword)
            for (name, symbols) in &index.symbol_map {
                if name.to_lowercase().contains(&keyword.to_lowercase()) && name != keyword {
                    for symbol in symbols {
                        results.push(self.symbol_to_chunk(symbol, &index.files));
                    }
                }
            }
        }

        // Remove duplicates and sort by relevance
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(max_results);
        results
    }

    fn symbol_to_chunk(
        &self,
        symbol: &CodeSymbol,
        files: &HashMap<String, IndexedFile>,
    ) -> CodeChunk {
        CodeChunk {
            file_path: symbol.file_path.clone(),
            start_line: symbol.start_line,
            end_line: symbol.end_line,
            content: symbol.signature.clone().unwrap_or_default(),
            language: files
                .get(&symbol.file_path)
                .map(|f| f.language.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            symbols: vec![symbol.name.clone()],
            relevance_score: 1.0,
        }
    }
}
