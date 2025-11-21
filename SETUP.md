# prompto - Setup & Implementation Guide

## Project Overview

**prompto** is a desktop application for codebase indexing and agentic prompt generation, powered by high-performance Rust libraries and an intuitive React UI, all running inside a Tauri shell.

### Core Product

**prompto** is a **prompt optimization tool** that:
1. ✅ Indexes user's local codebase using tree-sitter (syntax-aware)
2. ✅ Takes a user's raw prompt as input
3. ✅ Enhances the prompt using Claude's best practices
4. ✅ Enriches it with relevant codebase context via Claude Agent SDK
5. ✅ Outputs optimized, context-aware prompt ready for development tasks

**This is NOT just a code browser** - it's an intelligent prompt enhancer that bridges the gap between developer intent and AI-ready prompts.

---

## Tech Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| **Desktop Framework** | Tauri v2 | Latest version, better performance, improved APIs |
| **Package Manager** | Bun | Fastest runtime, native TypeScript support, drop-in Node replacement |
| **Backend** | Rust | Fast indexing, file system operations, Tauri commands |
| **Frontend** | React + TypeScript | Modern UI framework |
| **UI Components** | shadcn/ui + Tailwind | Beautiful, accessible components |
| **State Management** | Zustand | Lightweight, simple API, perfect for desktop apps |
| **Code Indexing** | tree-sitter | **Syntax-aware parsing, semantic understanding, AST-based** - industry standard for AI code tools |
| **AI Integration** | Claude Agent SDK (Node/TS) | Official SDK, context management, tool ecosystem |
| **Secrets Management** | .env files | Simple local development |
| **Target Platform** | macOS (primary) | Development and initial release focus |
| **Git Workflow** | Feature branches + conventional commits | Industry best practices |

---

## Indexing Library: tree-sitter

**Why tree-sitter over Tantivy:**
- ✅ **Semantic code understanding** - parses actual syntax structure
- ✅ **AST-based chunking** - preserves code context and relationships
- ✅ **Multi-language support** - works with all major programming languages
- ✅ **Used by GitHub, VSCode, Cursor** - proven for AI code tools
- ✅ **Exact syntax preservation** - crucial for showing relevant code snippets
- ✅ **Function/class/import extraction** - gives agents real understanding

**Tantivy** is a full-text search engine (like Lucene) - great for text search but doesn't understand code structure.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    TAURI v2 DESKTOP APP                  │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌────────────────────────────────────────────────┐    │
│  │         FRONTEND (React + TypeScript)          │    │
│  ├────────────────────────────────────────────────┤    │
│  │  • UI Components (shadcn/ui + Tailwind)        │    │
│  │  • State Management (Zustand)                   │    │
│  │  • Prompt Editor                                │    │
│  │  • Codebase Browser                             │    │
│  │  • Claude Agent SDK Integration ←───────────┐  │    │
│  └────────────────────────────────────────────────┘  │    │
│                         ↕                           │    │
│  ┌────────────────────────────────────────────────┐  │    │
│  │         BACKEND (Rust via Tauri)               │  │    │
│  ├────────────────────────────────────────────────┤  │    │
│  │  • File System Operations                      │  │    │
│  │  • tree-sitter Integration                     │  │    │
│  │  • Code Indexing Engine                        │  │    │
│  │  • Index Query API (Tauri Commands)            │  │    │
│  └────────────────────────────────────────────────┘  │    │
│                                                       │    │
└───────────────────────────────────────────────────────┘    │
                                                             │
  ┌──────────────────────────────────────────────────┐      │
  │         CLAUDE AGENT (Node/TS)                   │◄─────┘
  ├──────────────────────────────────────────────────┤
  │  • Prompt Optimization Logic                     │
  │  • Context Selection from Index                  │
  │  • Best Practices Application                    │
  │  • API Communication with Claude                 │
  └──────────────────────────────────────────────────┘
                    ↕
         [ ANTHROPIC API - Claude ]
```

---

## Phase 1: Environment Setup (macOS)

### 1.1 Install Core Tools
```bash
# Install Rust (latest stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Install Bun
curl -fsSL https://bun.sh/install | bash

# Verify installations
rustc --version
cargo --version
bun --version
```

### 1.2 Install macOS Dependencies
```bash
# Install Xcode Command Line Tools (if not already installed)
xcode-select --install

# Tauri v2 dependencies for macOS
# (Most are already included with Xcode)
```

### 1.3 Setup Environment Variables
```bash
# Create .env file for development
touch .env
echo "ANTHROPIC_API_KEY=your_key_here" >> .env
echo ".env" >> .gitignore
```

---

## Phase 2: Project Initialization

### 2.1 Create Tauri v2 + React Project
```bash
# Install create-tauri-app
bun add -g create-tauri-app

# Create project (from parent directory)
cd /path/to/parent
bunx create-tauri-app prompto

# During interactive setup:
# - Package manager: bun
# - UI template: React
# - TypeScript: Yes
# - Tauri version: v2

cd prompto
```

### 2.2 Verify Initial Structure
```
prompto/
├── src-tauri/              # Rust backend
│   ├── src/
│   │   └── main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                    # React frontend
│   ├── App.tsx
│   ├── main.tsx
│   └── styles.css
├── package.json
├── tsconfig.json
└── README.md
```

---

## Phase 3: Frontend Setup

### 3.1 Install shadcn/ui
```bash
# Initialize shadcn/ui with Tailwind
bunx shadcn@latest init

# Configuration:
# - TypeScript: Yes
# - Style: Default
# - Base color: Slate (or your preference)
# - CSS variables: Yes
# - Import alias: @/components
```

### 3.2 Install Frontend Dependencies
```bash
# Install Zustand for state management
bun add zustand

# Install Claude Agent SDK
bun add @anthropic-ai/claude-agent-sdk

# Install additional utilities
bun add clsx tailwind-merge
bun add lucide-react  # Icons

# Dev dependencies
bun add -d @types/node
```

### 3.3 Setup Project Structure
```bash
mkdir -p src/{components,lib,hooks,types,store,agents}
mkdir -p src/components/{ui,prompt-editor,codebase-browser,index-viewer}
```

---

## Phase 4: Backend (Rust) Setup

### 4.1 Add Rust Dependencies
Edit `src-tauri/Cargo.toml`:
```toml
[dependencies]
tauri = { version = "2.0", features = ["shell-open"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }

# tree-sitter for code parsing
tree-sitter = "0.20"
tree-sitter-rust = "0.20"
tree-sitter-javascript = "0.20"
tree-sitter-typescript = "0.20"
tree-sitter-python = "0.20"
# Add more language parsers as needed

# File system utilities
walkdir = "2"
ignore = "0.4"  # Respects .gitignore

# Async runtime
```

### 4.2 Create Backend Module Structure
```bash
cd src-tauri/src
mkdir -p {indexing,commands,models,utils}

# Create module files
touch indexing/mod.rs
touch indexing/tree_sitter_indexer.rs
touch commands/mod.rs
touch commands/index_commands.rs
touch models/mod.rs
touch models/code_index.rs
```

---

## Phase 5: Core Implementation

**See PHASE5_IMPLEMENTATION.md for detailed breakdown**

### 5.1 Implement tree-sitter Indexing (Rust)

**Purpose:** Parse codebase and extract:
- Functions, classes, methods
- Imports and dependencies
- File structure and relationships
- Code snippets with context

**Files to create:**
- `src-tauri/src/indexing/tree_sitter_indexer.rs`
- `src-tauri/src/models/code_index.rs` (data structures)
- `src-tauri/src/commands/index_commands.rs` (Tauri commands)

**Key Tauri Commands to expose:**
```rust
#[tauri::command]
async fn index_codebase(path: String) -> Result<IndexResult, String>

#[tauri::command]
async fn query_index(query: String) -> Result<Vec<CodeChunk>, String>

#[tauri::command]
async fn get_file_structure(path: String) -> Result<FileStructure, String>
```

### 5.2 Create Claude Agent (TypeScript)

**Purpose:**
- Accept user's raw prompt
- Query indexed codebase for relevant context
- Apply Claude prompt engineering best practices
- Generate optimized prompt

**File:** `src/agents/prompt-optimizer.ts`

**Key responsibilities:**
```typescript
class PromptOptimizerAgent {
  // Load indexed codebase context
  async loadCodebaseContext(projectPath: string)

  // Optimize prompt using Claude best practices
  async optimizePrompt(rawPrompt: string): Promise<OptimizedPrompt>

  // Apply best practices:
  // - Clear task description
  // - Relevant code context from index
  // - XML tags for structure
  // - Examples when beneficial
  // - Output format specification
}
```

### 5.3 Setup Zustand Store

**File:** `src/store/app-store.ts`

```typescript
interface AppState {
  // Indexing state
  indexedPath: string | null
  indexStatus: 'idle' | 'indexing' | 'complete' | 'error'
  indexData: CodeIndex | null

  // Prompt state
  rawPrompt: string
  optimizedPrompt: string | null

  // Actions
  setIndexedPath: (path: string) => void
  indexCodebase: (path: string) => Promise<void>
  optimizePrompt: (prompt: string) => Promise<void>
}
```

### 5.4 Build UI Components

**Components to create:**
1. **ProjectSelector** - Select codebase folder to index
2. **IndexingProgress** - Show indexing status
3. **PromptEditor** - Input area for raw prompt
4. **OptimizedPromptViewer** - Display enhanced prompt
5. **CodebaseExplorer** - Browse indexed codebase structure
6. **ContextPreview** - Show which code will be included in context

---

## Phase 6: Claude Prompt Engineering Integration

### 6.1 Prompt Best Practices to Implement

Based on Claude documentation, the optimizer should:

1. **Clear Task Description**
   - Transform vague prompts into specific, actionable tasks
   - Add "You are a..." role definition if beneficial

2. **Provide Relevant Context**
   - Include code from index that's relevant to task
   - Add file structure overview
   - Include related functions/classes

3. **Use XML Tags for Structure**
   ```xml
   <task>User's goal here</task>
   <codebase_context>
     <file path="...">Code here</file>
   </codebase_context>
   <requirements>Specific requirements</requirements>
   ```

4. **Add Examples (when helpful)**
   - Include similar code patterns from codebase
   - Show expected output format

5. **Specify Output Format**
   - Clear instructions on how to structure response

6. **Chain of Thought**
   - Ask Claude to think step-by-step for complex tasks

### 6.2 Agent Implementation Pattern

```typescript
async optimizePrompt(rawPrompt: string): Promise<string> {
  // 1. Analyze raw prompt to understand intent
  const intent = await this.analyzeIntent(rawPrompt)

  // 2. Query index for relevant code
  const relevantCode = await this.queryCodebase(intent)

  // 3. Build structured prompt using best practices
  const optimized = this.buildOptimizedPrompt({
    originalPrompt: rawPrompt,
    intent,
    codebaseContext: relevantCode,
    bestPractices: this.promptTemplates
  })

  return optimized
}
```

---

## Phase 7: Testing & Verification

### 7.1 Manual Testing Workflow
```bash
# Run in development mode
bun run tauri dev

# Test workflow:
# 1. Select a codebase folder
# 2. Wait for indexing to complete
# 3. Enter a raw prompt (e.g., "add user authentication")
# 4. Click "Optimize Prompt"
# 5. Review optimized output with codebase context
```

### 7.2 Verify Core Features
- [ ] Codebase indexing completes successfully
- [ ] tree-sitter parses common languages (JS, TS, Rust, Python)
- [ ] Index can be queried from frontend
- [ ] Claude Agent SDK initializes correctly
- [ ] Prompt optimization produces enhanced output
- [ ] Relevant code context is included

---

## Phase 8: Build & Distribution

### 8.1 Production Build
```bash
# Build for macOS
bun run tauri build

# Output locations:
# - DMG: src-tauri/target/release/bundle/dmg/
# - App: src-tauri/target/release/bundle/macos/
```

### 8.2 Code Signing (macOS)
```bash
# Configure in src-tauri/tauri.conf.json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "Developer ID Application: Your Name"
    }
  }
}
```

---

## Implementation Timeline

| Phase | Task | Estimated Time |
|-------|------|----------------|
| 1 | Environment setup (macOS) | 30 min |
| 2 | Project initialization | 20 min |
| 3 | Frontend setup (shadcn, deps) | 45 min |
| 4 | Backend setup (Rust deps) | 30 min |
| 5.1 | tree-sitter indexing implementation | 4-6 hours |
| 5.2 | Claude Agent creation | 3-4 hours |
| 5.3 | Zustand store setup | 1 hour |
| 5.4 | UI components | 4-6 hours |
| 6 | Prompt engineering integration | 3-4 hours |
| 7 | Testing & iteration | 2-3 hours |
| 8 | Build & polish | 1-2 hours |
| **TOTAL** | **~20-30 hours** |

---

## Key Files Overview

```
prompto/
├── .env                                    # API keys
├── src-tauri/
│   ├── Cargo.toml                          # Rust dependencies
│   ├── tauri.conf.json                     # Tauri v2 config
│   └── src/
│       ├── main.rs                         # Entry point
│       ├── indexing/
│       │   ├── mod.rs
│       │   └── tree_sitter_indexer.rs     # Core indexing logic
│       ├── commands/
│       │   ├── mod.rs
│       │   └── index_commands.rs          # Tauri commands
│       └── models/
│           └── code_index.rs              # Data structures
├── src/
│   ├── App.tsx                            # Main app component
│   ├── agents/
│   │   └── prompt-optimizer.ts            # Claude Agent
│   ├── store/
│   │   └── app-store.ts                   # Zustand state
│   ├── components/
│   │   ├── ui/                            # shadcn components
│   │   ├── prompt-editor/                 # Prompt input UI
│   │   ├── codebase-browser/              # Code explorer
│   │   └── index-viewer/                  # Index visualization
│   └── lib/
│       ├── utils.ts                       # Utilities
│       └── tauri-api.ts                   # Tauri command wrappers
└── package.json                           # Frontend deps (bun)
```

---

## Git Best Practices

### Branch Strategy
```bash
# Feature branches
git checkout -b feature/tree-sitter-indexing
git checkout -b feature/claude-agent
git checkout -b feature/prompt-editor-ui

# Work on feature, commit often
git add .
git commit -m "feat(indexing): implement tree-sitter parser for TypeScript"

# Push feature branch
git push -u origin feature/tree-sitter-indexing
```

### Conventional Commits
```bash
# Format: <type>(<scope>): <description>

feat(indexing): add tree-sitter integration for Rust files
fix(agent): correct context selection logic
docs(readme): update setup instructions
refactor(ui): extract PromptEditor into separate component
chore(deps): upgrade Tauri to v2.1
```

### Commit Types
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `refactor`: Code restructuring
- `chore`: Maintenance tasks
- `perf`: Performance improvements
- `test`: Adding tests

---

## Critical Success Factors

### ✅ Must-Have Features
1. **Accurate Indexing** - tree-sitter correctly parses all target languages
2. **Fast Performance** - Indexing completes in reasonable time
3. **Relevant Context Selection** - Agent picks the right code for prompts
4. **Clear UI/UX** - Users understand the workflow immediately
5. **Prompt Quality** - Output is genuinely better than raw input

### 🎯 Success Metrics
- Index a 10,000-file codebase in < 5 minutes
- Optimized prompts include relevant context 80%+ of the time
- Users can go from raw prompt to optimized output in < 30 seconds

---

## Resources

- **Tauri v2 Docs:** https://v2.tauri.app/
- **shadcn/ui:** https://ui.shadcn.com/
- **tree-sitter:** https://tree-sitter.github.io/
- **Claude Agent SDK:** https://docs.claude.com/en/docs/agent-sdk/
- **Bun:** https://bun.sh/docs
- **Zustand:** https://docs.pmnd.rs/zustand/

---

**Last Updated:** 2025-11-18
