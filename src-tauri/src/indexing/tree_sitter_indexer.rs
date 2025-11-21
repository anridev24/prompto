use crate::models::code_index::*;
use crate::indexing::text_normalizer::TextNormalizer;
use crate::indexing::tantivy_indexer::TantivyIndexer;
use crate::indexing::embedding_generator::{EmbeddingGenerator, symbol_to_text};
use crate::indexing::vector_store::{VectorStore, VectorMetadata};
use crate::indexing::hybrid_search::HybridSearcher;
use crate::indexing::query_analyzer::QueryAnalyzer;
use ignore::WalkBuilder;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

pub struct TreeSitterIndexer {
    parsers: HashMap<String, Parser>,
    queries: HashMap<String, String>,
    normalizer: TextNormalizer,
    tantivy_indexer: Option<TantivyIndexer>,
    embedding_generator: Option<EmbeddingGenerator>,
    vector_store: Option<VectorStore>,
    tantivy_path: Option<std::path::PathBuf>,
}

impl TreeSitterIndexer {
    pub fn new() -> Result<Self, String> {
        // Initialize embedding generator and vector store
        let embedding_generator = EmbeddingGenerator::new().ok();
        let vector_store = if let Some(ref gen) = embedding_generator {
            VectorStore::new(gen.embedding_dim()).ok()
        } else {
            None
        };

        let mut indexer = TreeSitterIndexer {
            parsers: HashMap::new(),
            queries: HashMap::new(),
            normalizer: TextNormalizer::new(),
            tantivy_indexer: None, // Will be initialized when needed
            embedding_generator,
            vector_store,
            tantivy_path: None,
        };

        // Initialize parsers for each language
        indexer.init_parser("rust", tree_sitter_rust::language())?;
        indexer.init_parser("javascript", tree_sitter_javascript::language())?;
        indexer.init_parser("typescript", tree_sitter_typescript::language_tsx())?;
        indexer.init_parser("python", tree_sitter_python::language())?;

        // Initialize queries for symbol extraction
        indexer.init_queries();

        Ok(indexer)
    }

    /// Set the Tantivy index directory and initialize/load the indexer
    pub fn set_tantivy_path<P: Into<std::path::PathBuf>>(&mut self, path: P) -> Result<(), String> {
        let path = path.into();
        self.tantivy_path = Some(path.clone());
        self.tantivy_indexer = Some(TantivyIndexer::new(path)?);
        Ok(())
    }

    /// Save vector store to disk
    pub fn save_vector_store<P: AsRef<Path>>(
        &self,
        index_path: P,
        metadata_path: P,
    ) -> Result<(), String> {
        if let Some(ref store) = self.vector_store {
            store.save(index_path, metadata_path)?;
        }
        Ok(())
    }

    /// Load vector store from disk
    pub fn load_vector_store<P: AsRef<Path>>(
        &mut self,
        index_path: P,
        metadata_path: P,
    ) -> Result<(), String> {
        if let Some(ref gen) = self.embedding_generator {
            let dimensions = gen.embedding_dim();
            self.vector_store = Some(VectorStore::load(index_path, metadata_path, dimensions)?);
        }
        Ok(())
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
                        // Add to Tantivy
                        if let Some(ref mut tantivy) = self.tantivy_indexer {
                            for symbol in &indexed_file.symbols {
                                if let Err(e) = tantivy.add_symbol(
                                    symbol,
                                    &indexed_file.language,
                                ) {
                                    eprintln!("Tantivy add failed: {}", e);
                                }
                            }
                        }

                        // Generate embeddings and add to vector store
                        if let (Some(ref mut gen), Some(ref mut store)) =
                            (&mut self.embedding_generator, &mut self.vector_store)
                        {
                            for symbol in &indexed_file.symbols {
                                let text = symbol_to_text(symbol);
                                match gen.embed(&text) {
                                    Ok(embedding) => {
                                        let metadata = VectorMetadata {
                                            symbol_name: symbol.name.clone(),
                                            file_path: symbol.file_path.clone(),
                                            language: indexed_file.language.clone(),
                                            start_line: symbol.start_line,
                                            end_line: symbol.end_line,
                                            signature: symbol.signature.clone(),
                                            doc_comment: symbol.doc_comment.clone(),
                                        };
                                        if let Err(e) = store.add(&embedding, metadata) {
                                            eprintln!("Vector store add failed: {}", e);
                                        }
                                    }
                                    Err(e) => eprintln!("Embedding generation failed: {}", e),
                                }
                            }
                        }

                        index.add_file(indexed_file);
                    }
                    Err(e) => {
                        eprintln!("Failed to index {}: {}", path.display(), e);
                    }
                }
            }
        }

        // Commit Tantivy index
        if let Some(ref mut tantivy) = self.tantivy_indexer {
            tantivy.commit()?;
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
    /// Traditional keyword search with normalization
    fn query_traditional(
        &self,
        index: &CodebaseIndex,
        query: &IndexQuery,
    ) -> Vec<CodeChunk> {
        let mut results = Vec::new();
        let max_results = query.max_results.unwrap_or(50);

        // Three-tier search with normalization
        for keyword in &query.keywords {
            // 1. Exact match (score 1.0)
            if let Some(symbols) = index.symbol_map.get(keyword) {
                for symbol in symbols {
                    let mut chunk = self.symbol_to_chunk(symbol, &index.files);
                    chunk.relevance_score = 1.0;
                    results.push(chunk);
                }
            }

            // 2. Normalized match (score 0.8)
            let normalized_terms = self.normalizer.normalize(keyword);
            for term in normalized_terms {
                if let Some(symbols) = index.normalized_symbol_map.get(&term) {
                    for symbol in symbols {
                        let mut chunk = self.symbol_to_chunk(symbol, &index.files);
                        chunk.relevance_score = 0.8;
                        results.push(chunk);
                    }
                }
            }

            // 3. Partial match (score 0.5)
            for (name, symbols) in &index.symbol_map {
                if name.to_lowercase().contains(&keyword.to_lowercase()) && name != keyword {
                    for symbol in symbols {
                        let mut chunk = self.symbol_to_chunk(symbol, &index.files);
                        chunk.relevance_score = 0.5;
                        results.push(chunk);
                    }
                }
            }
        }

        // Deduplicate
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

    /// Main query method using hybrid search with RRF
    pub fn query_index(
        &self,
        index: &CodebaseIndex,
        query: &IndexQuery,
    ) -> Vec<CodeChunk> {
        let query_text = query.keywords.join(" ");
        let query_type = QueryAnalyzer::analyze_query(&query_text);
        let config = query.hybrid_config
            .clone()
            .unwrap_or_else(|| QueryAnalyzer::get_config_for_query(&query_type));

        // Execute all searches
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

        // Combine with hybrid search using RRF
        let hybrid_searcher = HybridSearcher;
        hybrid_searcher.search(
            traditional_results,
            full_text_results,
            semantic_results,
            &config,
        )
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

    fn query_full_text(&self, query: &IndexQuery) -> Vec<CodeChunk> {
        let tantivy = match self.tantivy_indexer.as_ref() {
            Some(t) => t,
            None => return Vec::new(),
        };

        let query_str = query.keywords.join(" OR ");
        let max_results = query.max_results.unwrap_or(50);

        let results = match tantivy.search(&query_str, max_results) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Tantivy search failed: {}", e);
                return Vec::new();
            }
        };

        results.into_iter()
            .map(|r| CodeChunk {
                file_path: r.file_path,
                start_line: r.start_line,
                end_line: r.end_line,
                content: r.signature.unwrap_or_default(),
                language: r.language,
                symbols: vec![r.symbol_name],
                relevance_score: r.score,
            })
            .collect()
    }

    fn deduplicate_results(&self, results: Vec<CodeChunk>) -> Vec<CodeChunk> {
        use std::collections::HashMap;
        let mut seen = HashMap::new();
        let mut deduped = Vec::new();

        for chunk in results {
            let key = format!("{}:{}:{}", chunk.file_path, chunk.start_line, chunk.end_line);
            let entry = seen.entry(key.clone()).or_insert(0.0f32);

            if chunk.relevance_score > *entry {
                *entry = chunk.relevance_score;
                deduped.retain(|c: &CodeChunk| {
                    format!("{}:{}:{}", c.file_path, c.start_line, c.end_line) != key
                });
                deduped.push(chunk);
            }
        }

        deduped
    }

    pub fn query_file_paths(
        &self,
        index: &CodebaseIndex,
        query: &str,
        max_results: usize,
    ) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let mut matches: Vec<(String, f32)> = Vec::new();

        for (component, file_indices) in &index.file_path_components {
            if component.contains(&query_lower) {
                let score = if component == &query_lower {
                    1.0
                } else if component.starts_with(&query_lower) {
                    0.8
                } else {
                    0.5
                };

                for &idx in file_indices {
                    if let Some(path) = index.file_paths.get(idx) {
                        matches.push((path.clone(), score));
                    }
                }
            }
        }

        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        matches.truncate(max_results);
        matches.into_iter().map(|(path, _)| path).collect()
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

        // Generate embedding for query
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

    /// Collect file timestamps for cache validation
    pub fn collect_file_timestamps(
        root_path: &str,
    ) -> Result<HashMap<String, u64>, String> {
        let mut timestamps = HashMap::new();

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

            // Only track source files
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "rs" | "js" | "jsx" | "ts" | "tsx" | "py") {
                    if let Ok(metadata) = fs::metadata(path) {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                                let path_str = path.to_string_lossy().to_string();
                                timestamps.insert(path_str, duration.as_secs());
                            }
                        }
                    }
                }
            }
        }

        Ok(timestamps)
    }
}
