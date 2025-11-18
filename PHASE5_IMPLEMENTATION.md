# Phase 5: Core Implementation - Detailed Breakdown

This document provides step-by-step implementation details for Phase 5, the core functionality of prompto.

---

## Table of Contents

- [5.1 tree-sitter Indexing (Rust Backend)](#51-tree-sitter-indexing-rust-backend)
- [5.2 Claude Agent (TypeScript)](#52-claude-agent-typescript)
- [5.3 Zustand Store Setup](#53-zustand-store-setup)
- [5.4 UI Components](#54-ui-components)

---

## 5.1 tree-sitter Indexing (Rust Backend)

**Goal:** Create a high-performance code indexer that parses codebases using tree-sitter and extracts semantic information for AI agents.

### Step 1: Define Data Models

**File:** `src-tauri/src/models/code_index.rs`

```rust
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub symbol_kinds: Option<Vec<SymbolKind>>,
    pub file_patterns: Option<Vec<String>>,
    pub max_results: Option<usize>,
}
```

### Step 2: Implement tree-sitter Parser

**File:** `src-tauri/src/indexing/tree_sitter_indexer.rs`

```rust
use tree_sitter::{Parser, Language, Node, Tree, Query, QueryCursor};
use walkdir::WalkDir;
use ignore::WalkBuilder;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use crate::models::code_index::*;

// Import language parsers
extern "C" {
    fn tree_sitter_rust() -> Language;
    fn tree_sitter_javascript() -> Language;
    fn tree_sitter_typescript() -> Language;
    fn tree_sitter_python() -> Language;
}

pub struct TreeSitterIndexer {
    parsers: HashMap<String, Parser>,
    queries: HashMap<String, Query>,
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
        indexer.init_queries()?;

        Ok(indexer)
    }

    fn init_parser(&mut self, lang: &str, language: Language) -> Result<(), String> {
        let mut parser = Parser::new();
        parser
            .set_language(language)
            .map_err(|e| format!("Failed to set language {}: {}", lang, e))?;
        self.parsers.insert(lang.to_string(), parser);
        Ok(())
    }

    fn init_queries(&mut self) -> Result<(), String> {
        // Rust query for functions, structs, impls
        let rust_query = r#"
            (function_item
              name: (identifier) @function.name) @function.def

            (struct_item
              name: (type_identifier) @struct.name) @struct.def

            (impl_item
              type: (type_identifier) @impl.type) @impl.def

            (use_declaration) @import
        "#;

        // TypeScript/JavaScript query
        let ts_query = r#"
            (function_declaration
              name: (identifier) @function.name) @function.def

            (class_declaration
              name: (type_identifier) @class.name) @class.def

            (method_definition
              name: (property_identifier) @method.name) @method.def

            (import_statement) @import

            (export_statement) @export
        "#;

        // Python query
        let python_query = r#"
            (function_definition
              name: (identifier) @function.name) @function.def

            (class_definition
              name: (identifier) @class.name) @class.def

            (import_statement) @import
            (import_from_statement) @import
        "#;

        // Store queries (error handling simplified for brevity)
        self.queries.insert("rust".to_string(),
            Query::new(unsafe { tree_sitter_rust() }, rust_query)
                .map_err(|e| format!("Rust query error: {}", e))?);

        self.queries.insert("typescript".to_string(),
            Query::new(unsafe { tree_sitter_typescript() }, ts_query)
                .map_err(|e| format!("TS query error: {}", e))?);

        self.queries.insert("python".to_string(),
            Query::new(unsafe { tree_sitter_python() }, python_query)
                .map_err(|e| format!("Python query error: {}", e))?);

        Ok(())
    }

    /// Main indexing function
    pub fn index_codebase(&mut self, root_path: &str) -> Result<CodebaseIndex, String> {
        let start_time = std::time::Instant::now();
        let mut index = CodebaseIndex {
            root_path: root_path.to_string(),
            files: HashMap::new(),
            symbol_map: HashMap::new(),
            language_stats: HashMap::new(),
            total_files: 0,
            indexed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Walk directory respecting .gitignore
        let walker = WalkBuilder::new(root_path)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker {
            let entry = entry.map_err(|e| format!("Walk error: {}", e))?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            // Determine language from extension
            if let Some(language) = self.detect_language(path) {
                if let Ok(indexed_file) = self.index_file(path, &language) {
                    // Update stats
                    *index.language_stats.entry(language.clone()).or_insert(0) += 1;
                    index.total_files += 1;

                    // Add symbols to symbol map
                    for symbol in &indexed_file.symbols {
                        index.symbol_map
                            .entry(symbol.name.clone())
                            .or_insert_with(Vec::new)
                            .push(symbol.clone());
                    }

                    // Store indexed file
                    index.files.insert(
                        path.to_string_lossy().to_string(),
                        indexed_file,
                    );
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

        let parser = self.parsers.get_mut(language)
            .ok_or_else(|| format!("No parser for language: {}", language))?;

        let tree = parser.parse(&source_code, None)
            .ok_or_else(|| format!("Failed to parse {}", path.display()))?;

        let symbols = self.extract_symbols(&tree, &source_code, language, path)?;
        let imports = self.extract_imports(&tree, &source_code, language)?;

        Ok(IndexedFile {
            path: path.to_string_lossy().to_string(),
            language: language.to_string(),
            symbols,
            imports,
            exports: Vec::new(), // TODO: Extract exports
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
        tree: &Tree,
        source_code: &str,
        language: &str,
        file_path: &Path,
    ) -> Result<Vec<CodeSymbol>, String> {
        let mut symbols = Vec::new();

        if let Some(query) = self.queries.get(language) {
            let mut cursor = QueryCursor::new();
            let matches = cursor.matches(query, tree.root_node(), source_code.as_bytes());

            for match_ in matches {
                for capture in match_.captures {
                    let node = capture.node;
                    let symbol = self.node_to_symbol(node, source_code, file_path)?;
                    symbols.push(symbol);
                }
            }
        }

        Ok(symbols)
    }

    /// Convert tree-sitter node to CodeSymbol
    fn node_to_symbol(
        &self,
        node: Node,
        source_code: &str,
        file_path: &Path,
    ) -> Result<CodeSymbol, String> {
        let start = node.start_position();
        let end = node.end_position();
        let text = &source_code[node.byte_range()];

        // Determine symbol kind based on node type
        let kind = match node.kind() {
            "function_item" | "function_declaration" | "function_definition" => SymbolKind::Function,
            "struct_item" => SymbolKind::Struct,
            "class_declaration" | "class_definition" => SymbolKind::Class,
            "method_definition" => SymbolKind::Method,
            "impl_item" => SymbolKind::Interface,
            _ => SymbolKind::Variable,
        };

        // Extract name (simplified - would need proper query capture)
        let name = self.extract_name_from_node(node, source_code)
            .unwrap_or_else(|| "unknown".to_string());

        Ok(CodeSymbol {
            name,
            kind,
            file_path: file_path.to_string_lossy().to_string(),
            start_line: start.row + 1,
            end_line: end.row + 1,
            signature: Some(text.to_string()),
            doc_comment: None, // TODO: Extract doc comments
            parent: None,
        })
    }

    fn extract_name_from_node(&self, node: Node, source_code: &str) -> Option<String> {
        // Find identifier child node
        for child in node.children(&mut node.walk()) {
            if child.kind().contains("identifier") || child.kind().contains("name") {
                return Some(source_code[child.byte_range()].to_string());
            }
        }
        None
    }

    fn extract_imports(
        &self,
        tree: &Tree,
        source_code: &str,
        language: &str,
    ) -> Result<Vec<String>, String> {
        let mut imports = Vec::new();
        // TODO: Extract import statements using queries
        Ok(imports)
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

    /// Query the index
    pub fn query_index(
        &self,
        index: &CodebaseIndex,
        query: &IndexQuery,
    ) -> Result<Vec<CodeChunk>, String> {
        let mut results = Vec::new();
        let max_results = query.max_results.unwrap_or(50);

        // Search for symbols matching keywords
        for keyword in &query.keywords {
            if let Some(symbols) = index.symbol_map.get(keyword) {
                for symbol in symbols.iter().take(max_results) {
                    if let Some(file) = index.files.get(&symbol.file_path) {
                        let chunk = CodeChunk {
                            file_path: symbol.file_path.clone(),
                            start_line: symbol.start_line,
                            end_line: symbol.end_line,
                            content: symbol.signature.clone().unwrap_or_default(),
                            language: file.language.clone(),
                            symbols: vec![symbol.name.clone()],
                            relevance_score: 1.0, // TODO: Implement ranking
                        };
                        results.push(chunk);
                    }
                }
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results.into_iter().take(max_results).collect())
    }
}
```

### Step 3: Create Tauri Commands

**File:** `src-tauri/src/commands/index_commands.rs`

```rust
use crate::indexing::tree_sitter_indexer::TreeSitterIndexer;
use crate::models::code_index::*;
use std::sync::Mutex;
use tauri::State;

// Global state for the indexer
pub struct IndexerState {
    pub indexer: Mutex<TreeSitterIndexer>,
    pub current_index: Mutex<Option<CodebaseIndex>>,
}

#[tauri::command]
pub async fn index_codebase(
    path: String,
    state: State<'_, IndexerState>,
) -> Result<IndexResult, String> {
    let start_time = std::time::Instant::now();

    // Get indexer from state
    let mut indexer = state.indexer.lock()
        .map_err(|e| format!("Failed to lock indexer: {}", e))?;

    // Perform indexing
    let index = indexer.index_codebase(&path)?;

    // Calculate result
    let total_symbols: usize = index.files.values()
        .map(|f| f.symbols.len())
        .sum();

    let result = IndexResult {
        success: true,
        total_files: index.total_files,
        total_symbols,
        languages: index.language_stats.keys().cloned().collect(),
        duration_ms: start_time.elapsed().as_millis() as u64,
        errors: Vec::new(),
    };

    // Store index in state
    *state.current_index.lock()
        .map_err(|e| format!("Failed to lock index: {}", e))? = Some(index);

    Ok(result)
}

#[tauri::command]
pub async fn query_index(
    query: IndexQuery,
    state: State<'_, IndexerState>,
) -> Result<Vec<CodeChunk>, String> {
    let indexer = state.indexer.lock()
        .map_err(|e| format!("Failed to lock indexer: {}", e))?;

    let index_lock = state.current_index.lock()
        .map_err(|e| format!("Failed to lock index: {}", e))?;

    let index = index_lock.as_ref()
        .ok_or_else(|| "No codebase indexed".to_string())?;

    indexer.query_index(index, &query)
}

#[tauri::command]
pub async fn get_index_stats(
    state: State<'_, IndexerState>,
) -> Result<serde_json::Value, String> {
    let index_lock = state.current_index.lock()
        .map_err(|e| format!("Failed to lock index: {}", e))?;

    let index = index_lock.as_ref()
        .ok_or_else(|| "No codebase indexed".to_string())?;

    Ok(serde_json::json!({
        "total_files": index.total_files,
        "languages": index.language_stats,
        "root_path": index.root_path,
        "indexed_at": index.indexed_at,
    }))
}

#[tauri::command]
pub async fn get_file_symbols(
    file_path: String,
    state: State<'_, IndexerState>,
) -> Result<Vec<CodeSymbol>, String> {
    let index_lock = state.current_index.lock()
        .map_err(|e| format!("Failed to lock index: {}", e))?;

    let index = index_lock.as_ref()
        .ok_or_else(|| "No codebase indexed".to_string())?;

    index.files
        .get(&file_path)
        .map(|f| f.symbols.clone())
        .ok_or_else(|| format!("File not found: {}", file_path))
}
```

### Step 4: Update main.rs

**File:** `src-tauri/src/main.rs`

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod indexing;
mod commands;
mod models;

use commands::index_commands::*;
use indexing::tree_sitter_indexer::TreeSitterIndexer;
use std::sync::Mutex;

fn main() {
    // Initialize indexer state
    let indexer = TreeSitterIndexer::new()
        .expect("Failed to initialize tree-sitter indexer");

    let indexer_state = IndexerState {
        indexer: Mutex::new(indexer),
        current_index: Mutex::new(None),
    };

    tauri::Builder::default()
        .manage(indexer_state)
        .invoke_handler(tauri::generate_handler![
            index_codebase,
            query_index,
            get_index_stats,
            get_file_symbols,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Step 5: Update mod.rs files

**File:** `src-tauri/src/indexing/mod.rs`
```rust
pub mod tree_sitter_indexer;
```

**File:** `src-tauri/src/commands/mod.rs`
```rust
pub mod index_commands;
```

**File:** `src-tauri/src/models/mod.rs`
```rust
pub mod code_index;
```

---

## 5.2 Claude Agent (TypeScript)

**Goal:** Create an intelligent agent that optimizes user prompts using Claude best practices and codebase context.

### Step 1: Define Types

**File:** `src/types/agent.ts`

```typescript
export interface OptimizedPrompt {
  original: string;
  optimized: string;
  codeContext: CodeContext[];
  appliedPractices: string[];
  timestamp: number;
}

export interface CodeContext {
  filePath: string;
  startLine: number;
  endLine: number;
  content: string;
  language: string;
  relevance: number;
}

export interface PromptIntent {
  action: 'create' | 'modify' | 'fix' | 'explain' | 'refactor' | 'other';
  keywords: string[];
  scope: 'file' | 'function' | 'class' | 'project';
  entities: string[]; // Functions, classes, files mentioned
}

export interface BestPractice {
  name: string;
  description: string;
  apply: (prompt: string, context?: any) => string;
}
```

### Step 2: Create Prompt Templates

**File:** `src/agents/prompt-templates.ts`

```typescript
export const promptTemplates = {
  // Base template for all prompts
  base: (task: string, context: string, requirements?: string) => `
<task>
${task}
</task>

${context ? `<codebase_context>\n${context}\n</codebase_context>` : ''}

${requirements ? `<requirements>\n${requirements}\n</requirements>` : ''}

Please analyze the task and codebase context carefully before responding.
  `.trim(),

  // Template for code modification tasks
  modify: (
    originalPrompt: string,
    targetFiles: string[],
    codeContext: string
  ) => `
You are an expert software engineer working on a codebase.

<task>
${originalPrompt}
</task>

<codebase_context>
<relevant_files>
${targetFiles.map((f) => `- ${f}`).join('\n')}
</relevant_files>

<code>
${codeContext}
</code>
</codebase_context>

<requirements>
- Make minimal, focused changes
- Preserve existing code style and patterns
- Ensure backward compatibility
- Add appropriate error handling
- Update related tests if needed
</requirements>

Think step-by-step:
1. What specific changes are needed?
2. Which files need to be modified?
3. What edge cases should be considered?
4. Are there any dependencies or side effects?

Then provide your implementation.
  `.trim(),

  // Template for bug fixing
  fix: (
    bugDescription: string,
    relevantCode: string,
    errorContext?: string
  ) => `
You are debugging a codebase issue.

<problem>
${bugDescription}
</problem>

${errorContext ? `<error_details>\n${errorContext}\n</error_details>` : ''}

<relevant_code>
${relevantCode}
</relevant_code>

<debugging_steps>
1. Identify the root cause of the issue
2. Determine the minimal fix required
3. Consider potential side effects
4. Suggest preventive measures
</debugging_steps>

Provide your analysis and solution.
  `.trim(),

  // Template for feature creation
  create: (
    featureDescription: string,
    existingPatterns: string,
    relatedCode: string
  ) => `
You are implementing a new feature in an existing codebase.

<feature_requirements>
${featureDescription}
</feature_requirements>

<existing_patterns>
The codebase follows these patterns:
${existingPatterns}
</existing_patterns>

<related_code>
${relatedCode}
</related_code>

<implementation_guidelines>
- Follow existing code patterns and conventions
- Integrate seamlessly with current architecture
- Maintain consistency with similar features
- Consider scalability and maintainability
- Add appropriate documentation
</implementation_guidelines>

Think through:
1. How should this feature integrate with existing code?
2. What files need to be created or modified?
3. What edge cases need handling?
4. What tests are needed?

Provide your implementation plan and code.
  `.trim(),

  // Template for code explanation
  explain: (question: string, codeSnippet: string, context: string) => `
You are explaining code to a developer.

<question>
${question}
</question>

<code_to_explain>
${codeSnippet}
</code_to_explain>

${context ? `<surrounding_context>\n${context}\n</surrounding_context>` : ''}

<explanation_format>
1. High-level overview
2. Step-by-step breakdown
3. Key concepts and patterns used
4. Potential gotchas or important details
5. How it fits into the larger system
</explanation_format>

Provide a clear, comprehensive explanation.
  `.trim(),
};
```

### Step 3: Implement Intent Analyzer

**File:** `src/agents/intent-analyzer.ts`

```typescript
import Anthropic from '@anthropic-ai/sdk';
import { PromptIntent } from '../types/agent';

export class IntentAnalyzer {
  private client: Anthropic;

  constructor(apiKey: string) {
    this.client = new Anthropic({ apiKey });
  }

  async analyzeIntent(rawPrompt: string): Promise<PromptIntent> {
    const response = await this.client.messages.create({
      model: 'claude-3-5-sonnet-20241022',
      max_tokens: 500,
      messages: [
        {
          role: 'user',
          content: `Analyze this development task and extract structured information:

"${rawPrompt}"

Respond with JSON only:
{
  "action": "create" | "modify" | "fix" | "explain" | "refactor" | "other",
  "keywords": ["keyword1", "keyword2"],
  "scope": "file" | "function" | "class" | "project",
  "entities": ["function/class/file names mentioned"]
}`,
        },
      ],
    });

    const content = response.content[0];
    if (content.type !== 'text') {
      throw new Error('Unexpected response type from Claude');
    }

    try {
      return JSON.parse(content.text);
    } catch (error) {
      console.error('Failed to parse intent:', error);
      // Fallback to basic intent
      return {
        action: 'other',
        keywords: rawPrompt.toLowerCase().split(' ').filter((w) => w.length > 3),
        scope: 'project',
        entities: [],
      };
    }
  }
}
```

### Step 4: Implement Main Agent

**File:** `src/agents/prompt-optimizer.ts`

```typescript
import Anthropic from '@anthropic-ai/sdk';
import { invoke } from '@tauri-apps/api/core';
import { IntentAnalyzer } from './intent-analyzer';
import { promptTemplates } from './prompt-templates';
import {
  OptimizedPrompt,
  CodeContext,
  PromptIntent,
} from '../types/agent';

export class PromptOptimizerAgent {
  private client: Anthropic;
  private intentAnalyzer: IntentAnalyzer;
  private indexedPath: string | null = null;

  constructor(apiKey: string) {
    this.client = new Anthropic({ apiKey });
    this.intentAnalyzer = new IntentAnalyzer(apiKey);
  }

  setIndexedPath(path: string) {
    this.indexedPath = path;
  }

  async optimizePrompt(rawPrompt: string): Promise<OptimizedPrompt> {
    if (!this.indexedPath) {
      throw new Error('No codebase indexed. Please index a project first.');
    }

    // Step 1: Analyze intent
    const intent = await this.intentAnalyzer.analyzeIntent(rawPrompt);
    console.log('Analyzed intent:', intent);

    // Step 2: Query codebase for relevant context
    const codeContext = await this.queryCodebase(intent);
    console.log(`Found ${codeContext.length} relevant code chunks`);

    // Step 3: Select appropriate template
    const template = this.selectTemplate(intent);

    // Step 4: Build optimized prompt
    const optimized = await this.buildOptimizedPrompt(
      rawPrompt,
      intent,
      codeContext,
      template
    );

    return {
      original: rawPrompt,
      optimized,
      codeContext,
      appliedPractices: this.getAppliedPractices(intent),
      timestamp: Date.now(),
    };
  }

  private async queryCodebase(intent: PromptIntent): Promise<CodeContext[]> {
    try {
      // Query Rust backend for relevant code
      const results = await invoke<any[]>('query_index', {
        query: {
          keywords: [...intent.keywords, ...intent.entities],
          max_results: 10,
        },
      });

      return results.map((chunk) => ({
        filePath: chunk.file_path,
        startLine: chunk.start_line,
        endLine: chunk.end_line,
        content: chunk.content,
        language: chunk.language,
        relevance: chunk.relevance_score,
      }));
    } catch (error) {
      console.error('Failed to query codebase:', error);
      return [];
    }
  }

  private selectTemplate(intent: PromptIntent): string {
    switch (intent.action) {
      case 'create':
        return 'create';
      case 'modify':
        return 'modify';
      case 'fix':
        return 'fix';
      case 'explain':
        return 'explain';
      default:
        return 'base';
    }
  }

  private async buildOptimizedPrompt(
    rawPrompt: string,
    intent: PromptIntent,
    codeContext: CodeContext[],
    templateType: string
  ): Promise<string> {
    // Format code context
    const formattedContext = this.formatCodeContext(codeContext);

    // Get template function
    const template = promptTemplates[templateType as keyof typeof promptTemplates];

    if (typeof template !== 'function') {
      // Fallback to base template
      return promptTemplates.base(rawPrompt, formattedContext);
    }

    // Apply template based on type
    switch (templateType) {
      case 'modify':
        const targetFiles = [...new Set(codeContext.map((c) => c.filePath))];
        return promptTemplates.modify(rawPrompt, targetFiles, formattedContext);

      case 'fix':
        return promptTemplates.fix(rawPrompt, formattedContext);

      case 'create':
        const patterns = await this.extractPatterns(codeContext);
        return promptTemplates.create(rawPrompt, patterns, formattedContext);

      case 'explain':
        const mainCode = codeContext[0]?.content || '';
        const context = codeContext.slice(1).map((c) => c.content).join('\n\n');
        return promptTemplates.explain(rawPrompt, mainCode, context);

      default:
        return promptTemplates.base(rawPrompt, formattedContext);
    }
  }

  private formatCodeContext(contexts: CodeContext[]): string {
    if (contexts.length === 0) {
      return 'No specific code context found in the indexed codebase.';
    }

    return contexts
      .map(
        (ctx) => `
<file path="${ctx.filePath}" lines="${ctx.startLine}-${ctx.endLine}" language="${ctx.language}">
${ctx.content}
</file>
      `.trim()
      )
      .join('\n\n');
  }

  private async extractPatterns(contexts: CodeContext[]): Promise<string> {
    // Use Claude to extract common patterns from code context
    const codeSnippets = contexts.map((c) => c.content).join('\n\n---\n\n');

    try {
      const response = await this.client.messages.create({
        model: 'claude-3-5-sonnet-20241022',
        max_tokens: 1000,
        messages: [
          {
            role: 'user',
            content: `Analyze these code snippets and identify common patterns, conventions, and architectural approaches:

${codeSnippets}

Provide a concise summary of:
1. Code style and formatting patterns
2. Architectural patterns (e.g., class structure, dependency injection)
3. Naming conventions
4. Error handling approaches
5. Common libraries or frameworks used`,
          },
        ],
      });

      const content = response.content[0];
      return content.type === 'text' ? content.text : '';
    } catch (error) {
      console.error('Failed to extract patterns:', error);
      return 'Unable to extract patterns from codebase.';
    }
  }

  private getAppliedPractices(intent: PromptIntent): string[] {
    const practices = [
      'Structured prompt with XML tags',
      'Relevant codebase context included',
      'Clear task description',
    ];

    if (intent.action === 'modify' || intent.action === 'fix') {
      practices.push('Explicit requirements for changes');
    }

    if (intent.action === 'create') {
      practices.push('Existing patterns and conventions provided');
    }

    practices.push('Step-by-step thinking encouraged');

    return practices;
  }
}
```

---

## 5.3 Zustand Store Setup

**Goal:** Create centralized state management for the application.

### Step 1: Create Store

**File:** `src/store/app-store.ts`

```typescript
import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { PromptOptimizerAgent } from '../agents/prompt-optimizer';
import { OptimizedPrompt } from '../types/agent';

interface IndexStats {
  total_files: number;
  languages: Record<string, number>;
  root_path: string;
  indexed_at: number;
}

interface AppState {
  // Indexing state
  indexedPath: string | null;
  indexStatus: 'idle' | 'indexing' | 'complete' | 'error';
  indexStats: IndexStats | null;
  indexError: string | null;

  // Prompt state
  rawPrompt: string;
  optimizedPrompt: OptimizedPrompt | null;
  isOptimizing: boolean;
  optimizeError: string | null;

  // Agent instance
  agent: PromptOptimizerAgent | null;

  // Actions
  initializeAgent: (apiKey: string) => void;
  setIndexedPath: (path: string) => void;
  indexCodebase: (path: string) => Promise<void>;
  setRawPrompt: (prompt: string) => void;
  optimizePrompt: () => Promise<void>;
  clearOptimizedPrompt: () => void;
  getIndexStats: () => Promise<void>;
}

export const useAppStore = create<AppState>((set, get) => ({
  // Initial state
  indexedPath: null,
  indexStatus: 'idle',
  indexStats: null,
  indexError: null,
  rawPrompt: '',
  optimizedPrompt: null,
  isOptimizing: false,
  optimizeError: null,
  agent: null,

  // Initialize agent with API key
  initializeAgent: (apiKey: string) => {
    const agent = new PromptOptimizerAgent(apiKey);
    set({ agent });
  },

  // Set indexed path
  setIndexedPath: (path: string) => {
    set({ indexedPath: path });
    const { agent } = get();
    if (agent) {
      agent.setIndexedPath(path);
    }
  },

  // Index codebase
  indexCodebase: async (path: string) => {
    set({ indexStatus: 'indexing', indexError: null });

    try {
      const result = await invoke('index_codebase', { path });
      console.log('Indexing result:', result);

      set({
        indexStatus: 'complete',
        indexedPath: path,
      });

      // Update agent
      const { agent } = get();
      if (agent) {
        agent.setIndexedPath(path);
      }

      // Fetch stats
      await get().getIndexStats();
    } catch (error) {
      console.error('Indexing failed:', error);
      set({
        indexStatus: 'error',
        indexError: error instanceof Error ? error.message : String(error),
      });
    }
  },

  // Get index statistics
  getIndexStats: async () => {
    try {
      const stats = await invoke<IndexStats>('get_index_stats');
      set({ indexStats: stats });
    } catch (error) {
      console.error('Failed to get index stats:', error);
    }
  },

  // Set raw prompt
  setRawPrompt: (prompt: string) => {
    set({ rawPrompt: prompt });
  },

  // Optimize prompt
  optimizePrompt: async () => {
    const { agent, rawPrompt } = get();

    if (!agent) {
      set({ optimizeError: 'Agent not initialized. Please set API key.' });
      return;
    }

    if (!rawPrompt.trim()) {
      set({ optimizeError: 'Please enter a prompt to optimize.' });
      return;
    }

    set({ isOptimizing: true, optimizeError: null });

    try {
      const optimized = await agent.optimizePrompt(rawPrompt);
      set({
        optimizedPrompt: optimized,
        isOptimizing: false,
      });
    } catch (error) {
      console.error('Optimization failed:', error);
      set({
        isOptimizing: false,
        optimizeError: error instanceof Error ? error.message : String(error),
      });
    }
  },

  // Clear optimized prompt
  clearOptimizedPrompt: () => {
    set({ optimizedPrompt: null, optimizeError: null });
  },
}));
```

---

## 5.4 UI Components

**Goal:** Build user interface components for the application.

### Step 1: Create Tauri API Wrapper

**File:** `src/lib/tauri-api.ts`

```typescript
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export async function selectDirectory(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
  });

  return typeof selected === 'string' ? selected : null;
}

export async function indexCodebase(path: string) {
  return invoke('index_codebase', { path });
}

export async function queryIndex(keywords: string[]) {
  return invoke('query_index', {
    query: {
      keywords,
      max_results: 10,
    },
  });
}

export async function getIndexStats() {
  return invoke('get_index_stats');
}
```

### Step 2: Project Selector Component

**File:** `src/components/project-selector/ProjectSelector.tsx`

```typescript
import React from 'react';
import { useAppStore } from '../../store/app-store';
import { selectDirectory } from '../../lib/tauri-api';
import { Button } from '../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/card';
import { Folder, Loader2 } from 'lucide-react';

export function ProjectSelector() {
  const { indexedPath, indexStatus, indexCodebase } = useAppStore();

  const handleSelectProject = async () => {
    const path = await selectDirectory();
    if (path) {
      await indexCodebase(path);
    }
  };

  const isIndexing = indexStatus === 'indexing';

  return (
    <Card>
      <CardHeader>
        <CardTitle>Select Project</CardTitle>
        <CardDescription>
          Choose a codebase to index for prompt optimization
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex items-center gap-4">
          <Button
            onClick={handleSelectProject}
            disabled={isIndexing}
            className="gap-2"
          >
            {isIndexing ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                Indexing...
              </>
            ) : (
              <>
                <Folder className="h-4 w-4" />
                Select Folder
              </>
            )}
          </Button>
          {indexedPath && (
            <div className="flex-1 text-sm text-muted-foreground">
              <span className="font-medium">Current:</span> {indexedPath}
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
```

### Step 3: Prompt Editor Component

**File:** `src/components/prompt-editor/PromptEditor.tsx`

```typescript
import React from 'react';
import { useAppStore } from '../../store/app-store';
import { Button } from '../ui/button';
import { Textarea } from '../ui/textarea';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { Sparkles, Loader2 } from 'lucide-react';

export function PromptEditor() {
  const { rawPrompt, setRawPrompt, optimizePrompt, isOptimizing, indexStatus } =
    useAppStore();

  const canOptimize = indexStatus === 'complete' && rawPrompt.trim().length > 0;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Your Prompt</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <Textarea
          placeholder="Enter your development task or question here..."
          value={rawPrompt}
          onChange={(e) => setRawPrompt(e.target.value)}
          className="min-h-[200px] font-mono"
        />
        <Button
          onClick={optimizePrompt}
          disabled={!canOptimize || isOptimizing}
          className="w-full gap-2"
        >
          {isOptimizing ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Optimizing...
            </>
          ) : (
            <>
              <Sparkles className="h-4 w-4" />
              Optimize Prompt
            </>
          )}
        </Button>
      </CardContent>
    </Card>
  );
}
```

### Step 4: Optimized Prompt Viewer

**File:** `src/components/prompt-editor/OptimizedPromptViewer.tsx`

```typescript
import React from 'react';
import { useAppStore } from '../../store/app-store';
import { Button } from '../ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { Badge } from '../ui/badge';
import { Copy, Check } from 'lucide-react';

export function OptimizedPromptViewer() {
  const { optimizedPrompt } = useAppStore();
  const [copied, setCopied] = React.useState(false);

  if (!optimizedPrompt) {
    return null;
  }

  const handleCopy = async () => {
    await navigator.clipboard.writeText(optimizedPrompt.optimized);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>Optimized Prompt</CardTitle>
          <Button
            variant="outline"
            size="sm"
            onClick={handleCopy}
            className="gap-2"
          >
            {copied ? (
              <>
                <Check className="h-4 w-4" />
                Copied!
              </>
            ) : (
              <>
                <Copy className="h-4 w-4" />
                Copy
              </>
            )}
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Applied Practices */}
        <div className="flex flex-wrap gap-2">
          {optimizedPrompt.appliedPractices.map((practice) => (
            <Badge key={practice} variant="secondary">
              {practice}
            </Badge>
          ))}
        </div>

        {/* Optimized prompt */}
        <pre className="bg-muted p-4 rounded-lg overflow-x-auto">
          <code className="text-sm">{optimizedPrompt.optimized}</code>
        </pre>

        {/* Code context summary */}
        {optimizedPrompt.codeContext.length > 0 && (
          <div className="text-sm text-muted-foreground">
            Included context from {optimizedPrompt.codeContext.length} code{' '}
            {optimizedPrompt.codeContext.length === 1 ? 'location' : 'locations'}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
```

### Step 5: Index Stats Component

**File:** `src/components/index-viewer/IndexStats.tsx`

```typescript
import React from 'react';
import { useAppStore } from '../../store/app-store';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { Badge } from '../ui/badge';

export function IndexStats() {
  const { indexStats, indexStatus } = useAppStore();

  if (indexStatus !== 'complete' || !indexStats) {
    return null;
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Index Statistics</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <div className="text-2xl font-bold">{indexStats.total_files}</div>
            <div className="text-sm text-muted-foreground">Files Indexed</div>
          </div>
          <div>
            <div className="text-2xl font-bold">
              {Object.keys(indexStats.languages).length}
            </div>
            <div className="text-sm text-muted-foreground">Languages</div>
          </div>
        </div>

        <div>
          <div className="text-sm font-medium mb-2">Languages:</div>
          <div className="flex flex-wrap gap-2">
            {Object.entries(indexStats.languages).map(([lang, count]) => (
              <Badge key={lang} variant="outline">
                {lang}: {count}
              </Badge>
            ))}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
```

### Step 6: Main App Layout

**File:** `src/App.tsx`

```typescript
import React from 'react';
import { useAppStore } from './store/app-store';
import { ProjectSelector } from './components/project-selector/ProjectSelector';
import { PromptEditor } from './components/prompt-editor/PromptEditor';
import { OptimizedPromptViewer } from './components/prompt-editor/OptimizedPromptViewer';
import { IndexStats } from './components/index-viewer/IndexStats';
import { Button } from './components/ui/button';
import { Input } from './components/ui/input';
import { Label } from './components/ui/label';

function App() {
  const { agent, initializeAgent } = useAppStore();
  const [apiKey, setApiKey] = React.useState('');

  const handleSetApiKey = () => {
    if (apiKey.trim()) {
      initializeAgent(apiKey);
      // Store in localStorage for development
      localStorage.setItem('anthropic_api_key', apiKey);
    }
  };

  // Load API key from localStorage on mount
  React.useEffect(() => {
    const stored = localStorage.getItem('anthropic_api_key');
    if (stored) {
      setApiKey(stored);
      initializeAgent(stored);
    }
  }, []);

  if (!agent) {
    return (
      <div className="container max-w-md mx-auto mt-20">
        <div className="space-y-4">
          <div>
            <h1 className="text-2xl font-bold">Welcome to prompto</h1>
            <p className="text-muted-foreground">
              Enter your Anthropic API key to get started
            </p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="api-key">API Key</Label>
            <Input
              id="api-key"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="sk-ant-..."
            />
          </div>
          <Button onClick={handleSetApiKey} className="w-full">
            Continue
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-6 space-y-6">
      <header>
        <h1 className="text-3xl font-bold">prompto</h1>
        <p className="text-muted-foreground">
          AI-powered prompt optimization for your codebase
        </p>
      </header>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-6">
          <ProjectSelector />
          <PromptEditor />
          <OptimizedPromptViewer />
        </div>

        <div className="space-y-6">
          <IndexStats />
        </div>
      </div>
    </div>
  );
}

export default App;
```

---

## Implementation Checklist

### 5.1 Backend (Rust)
- [ ] Create data models in `models/code_index.rs`
- [ ] Implement tree-sitter indexer in `indexing/tree_sitter_indexer.rs`
- [ ] Add language parsers (Rust, TS, Python, etc.)
- [ ] Create tree-sitter queries for symbol extraction
- [ ] Implement Tauri commands in `commands/index_commands.rs`
- [ ] Update `main.rs` with command handlers
- [ ] Test indexing on sample codebases

### 5.2 Frontend Agent
- [ ] Define TypeScript types in `types/agent.ts`
- [ ] Create prompt templates in `agents/prompt-templates.ts`
- [ ] Implement intent analyzer in `agents/intent-analyzer.ts`
- [ ] Build main agent in `agents/prompt-optimizer.ts`
- [ ] Test agent with various prompts

### 5.3 State Management
- [ ] Create Zustand store in `store/app-store.ts`
- [ ] Implement all state actions
- [ ] Test state updates and persistence

### 5.4 UI Components
- [ ] Install shadcn/ui components (button, card, input, etc.)
- [ ] Create ProjectSelector component
- [ ] Create PromptEditor component
- [ ] Create OptimizedPromptViewer component
- [ ] Create IndexStats component
- [ ] Build main App layout
- [ ] Test full user workflow

---

## Testing Strategy

### Manual Testing Workflow

1. **Start application**
   ```bash
   bun run tauri dev
   ```

2. **Enter API key**
   - Use valid Anthropic API key

3. **Select project**
   - Choose a codebase folder
   - Wait for indexing to complete
   - Verify stats appear

4. **Test prompt optimization**
   - Enter: "add user authentication"
   - Click "Optimize Prompt"
   - Verify optimized output includes:
     - Structured XML format
     - Relevant code context
     - Clear requirements
     - Applied best practices

5. **Test different prompt types**
   - Bug fix: "fix the login error"
   - Feature: "create a search feature"
   - Explanation: "explain how the router works"

### Performance Benchmarks

- Index 1,000 files: < 30 seconds
- Index 10,000 files: < 5 minutes
- Prompt optimization: < 10 seconds
- UI responsiveness: No blocking operations

---

## Next Steps After Phase 5

1. **Phase 6:** Integrate Claude best practices patterns
2. **Phase 7:** End-to-end testing and bug fixes
3. **Phase 8:** Production build and distribution

---

**Document Version:** 1.0
**Last Updated:** 2025-11-18
