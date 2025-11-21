# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

**prompto** is a desktop application for codebase indexing and agentic prompt generation. It combines high-performance Rust indexing with a modern React UI, all running inside a Tauri shell. The application indexes local codebases using multiple search strategies (tree-sitter, full-text, semantic) and uses Claude to optimize prompts with relevant code context.

## Architecture

### Tauri Structure
- **Backend (Rust):** `src-tauri/` - Handles codebase indexing, search, persistence, and Anthropic API calls
- **Frontend (React + TypeScript):** `src/` - UI built with React, shadcn/ui components, and Zustand for state management

### Backend Architecture (`src-tauri/src/`)

#### Core Modules
- **`main.rs`:** Application entry point. Initializes TreeSitterIndexer, manages global IndexerState, and registers Tauri commands
- **`commands/`:** Tauri command handlers exposed to the frontend
  - `index_commands.rs` - Indexing operations (index_codebase, query_index, get_index_stats, search_semantic, etc.)
  - `anthropic_commands.rs` - Claude API integration (analyze_intent, extract_patterns)
- **`indexing/`:** The heart of the indexing system
  - `tree_sitter_indexer.rs` - Main indexer using tree-sitter for code parsing
  - `tantivy_indexer.rs` - Full-text search using Tantivy
  - `embedding_generator.rs` - Generates semantic embeddings using Candle ML framework
  - `vector_store.rs` - Vector storage and semantic search using USearch
  - `hybrid_search.rs` - Combines traditional, full-text, and semantic search using Reciprocal Rank Fusion (RRF)
  - `persistence.rs` - Caches indexes to disk with timestamp-based invalidation
  - `query_analyzer.rs` - Analyzes queries to optimize search strategy
  - `relevance_scorer.rs` - Scores and ranks search results
  - `text_normalizer.rs` - Text normalization utilities
- **`anthropic/`:** Anthropic API client implementation
  - `mod.rs` - HTTP client for Claude API (analyze_intent, extract_patterns)
  - `models.rs` - Request/response types for Anthropic API
- **`models/`:** Data structures
  - `code_index.rs` - Core types: CodebaseIndex, FileIndex, Symbol, CodeChunk, etc.

#### Indexing Pipeline
1. **Tree-sitter parsing:** Extracts symbols (functions, classes, methods) from supported languages (Rust, JavaScript, TypeScript, Python)
2. **Full-text indexing:** Tantivy indexes all code content for keyword search
3. **Semantic embedding:** ML embeddings generated for semantic similarity search
4. **Hybrid search:** Combines all three strategies using Reciprocal Rank Fusion to rank results
5. **Persistence:** Indexes cached to disk with file timestamp tracking for invalidation

#### Search Strategy
The system uses three complementary search methods combined via RRF:
- **Traditional:** Basic keyword matching on symbols
- **Full-text (Tantivy):** Advanced full-text search with BM25 ranking
- **Semantic (Vector/USearch):** ML-powered semantic similarity search

### Frontend Architecture (`src/`)

- **`App.tsx`:** Main application component with API key setup and layout
- **`store/app-store.ts`:** Zustand store managing application state (indexing, prompts, agent)
- **`agents/`:** Client-side agent logic
  - `prompt-optimizer.ts` - PromptOptimizerAgent: orchestrates intent analysis and prompt optimization
  - `intent-analyzer.ts` - Analyzes user prompts to extract action, keywords, scope, entities
  - `prompt-templates.ts` - Templates for different prompt types (create, modify, fix, etc.)
- **`components/`:**
  - `project-selector/` - Directory picker for indexing
  - `prompt-editor/` - Input area for raw prompts
  - `index-viewer/` - Displays index statistics
  - `ui/` - shadcn/ui components (button, card, input, etc.)
- **`lib/tauri-api.ts`:** Frontend wrappers for Tauri backend commands

#### Prompt Optimization Flow
1. User enters raw prompt
2. IntentAnalyzer uses Claude to extract action, keywords, scope, entities
3. **CodebaseAnalyzer** collects comprehensive project context:
   - Project structure (files, languages, organization)
   - Architectural patterns (frameworks, design patterns)
   - Related files (based on keywords)
   - Related symbols (functions, classes matching intent)
4. PromptOptimizerAgent queries indexed codebase for relevant code chunks
5. Multi-stage context filtering:
   - Semantic filtering (boosts relevance by keyword/intent matches)
   - Relevance threshold (>30% score required)
   - Deduplication (removes overlaps and duplicates)
   - Token budget management (8k token limit)
6. Agent selects appropriate template based on intent
7. Builds optimized prompt with:
   - Codebase info (structure, patterns, related files/symbols)
   - Filtered code context
   - Claude 4.5 best practices (thinking, examples, output format)
8. Returns OptimizedPrompt with original, optimized version, code context, and model config

## Common Development Commands

### Setup
```bash
# Install frontend dependencies (uses bun by default)
bun install
# or use pnpm/npm
pnpm install
npm install
```

### Development
```bash
# Start development mode (frontend + backend hot-reload)
pnpm tauri:dev
# or
npm run tauri dev

# Frontend only (for UI work without Rust changes)
pnpm dev
```

### Building
```bash
# Build production release
pnpm tauri:build
# or
npm run tauri build

# Build Rust backend only
cd src-tauri
cargo build --release
```

### Testing Rust Backend
```bash
cd src-tauri
cargo test
cargo test -- --nocapture  # Show println! output
```

### Running Benchmarks
```bash
cd src-tauri
cargo bench --bench search_benchmark
```

## Key Implementation Details

### Tauri Configuration
- Uses **bun** for frontend build commands (see `tauri.conf.json`)
- Dev server runs on `localhost:5173`
- Window dimensions: 1200x800, resizable

### State Management
- Backend: Global `IndexerState` struct with Mutex-wrapped TreeSitterIndexer, CodebaseIndex, and PersistenceConfig
- Frontend: Zustand store (`app-store.ts`) manages indexing status, prompts, and agent instance

### Persistence & Caching
- Index cache location managed by PersistenceConfig (uses Tauri app data directory)
- Cache includes: main index (bincode), vector index (USearch), Tantivy directory, and metadata
- Cache validation: Compares file timestamps to detect stale caches
- On app start: Automatically attempts to load cached index for last indexed path

### API Security
- Anthropic API calls moved from frontend to Rust backend for security
- API key stored in frontend localStorage for convenience (development only - consider secure storage for production)
- Backend AnthropicClient handles all Claude API requests

### Supported Languages
Currently tree-sitter parsing supports: Rust, JavaScript, TypeScript, Python
To add more languages: Add tree-sitter-{lang} dependency in Cargo.toml and update tree_sitter_indexer.rs

## Important Notes

### Windows-Specific
- Uses MSVC toolchain with static CRT linking
- Icon generation: Use `create-icon.ps1` or `pnpm tauri icon`
- Build artifacts in `src-tauri/target/release/`

### Dependencies
- **Rust:** tokio (async runtime), reqwest (HTTP), tree-sitter (parsing), tantivy (full-text search), candle (ML embeddings), usearch (vector storage)
- **Frontend:** React 18, TypeScript, Vite, Tailwind CSS, shadcn/ui, Zustand, @anthropic-ai/sdk

### Recent Improvements
- **Comprehensive Codebase Analysis**: Collects project structure, architectural patterns, related files/symbols for rich context
- **Claude 4.5 Best Practices**: System prompts, chain-of-thought reasoning, few-shot examples, structured output formats, temperature control
- **Smart Context Filtering**: Multi-stage filtering (semantic boosting, relevance threshold, deduplication, token budget) ensures only relevant code is included
- **Hybrid Search with RRF**: Combines traditional, full-text, and semantic search
- **Symbol-Level Discovery**: Finds related functions, classes, interfaces based on intent
- **Anthropic API Backend Integration**: Secure API calls from Rust backend
- **Persistence/Caching System**: Fast startup with cached indexes

See detailed documentation:
- `CODEBASE_ANALYSIS.md` - Comprehensive codebase analysis system
- `CLAUDE_45_IMPROVEMENTS.md` - Claude 4.5 best practices implementation
- `CONTEXT_FILTERING.md` - Multi-stage context filtering
- `MODEL_CONFIGURATION.md` - Temperature settings, token limits, and cost analysis
