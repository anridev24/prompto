# Model Configuration Guide

This document explains how prompto configures Claude for optimal code generation and prompt optimization.

---

## Response Token Limits

### Configuration

**Max Tokens: 16,384**

This is set high to allow comprehensive, unrestricted responses from Claude.

**Why 16,384 tokens?**
- ~65,000 characters of output
- Enough for complete feature implementations
- Can include code + tests + documentation
- No artificial truncation mid-response

**What this allows:**
```typescript
✅ Complete class implementations with all methods
✅ Multiple related files in one response
✅ Full test suites with edge cases
✅ Detailed explanations with examples
✅ Comprehensive refactoring with before/after
✅ Documentation and inline comments
```

**Location:** `src/agents/prompt-optimizer.ts:469`

```typescript
return {
  temperature: temperatureMap[intent.action],
  model: 'claude-sonnet-4-5-20250929',
  maxTokens: 16384, // High limit for comprehensive responses
};
```

---

## Temperature Settings by Task Type

prompto uses **task-specific temperature** to optimize output quality:

### Fix (Bug Fixes) - Temperature: 0.3
**Low temperature for deterministic fixes**

- Consistent, reliable bug fixes
- Minimal randomness in solutions
- Focus on proven patterns
- Repeatable results

**Best for:**
- Fixing null pointer exceptions
- Correcting type errors
- Resolving race conditions
- Patching security vulnerabilities

### Modify (Code Changes) - Temperature: 0.3
**Low temperature for precise modifications**

- Focused, minimal changes
- Consistent with existing code
- Predictable refactoring
- Safe transformations

**Best for:**
- Adding validation logic
- Updating function signatures
- Modifying API endpoints
- Changing behavior precisely

### Create (New Features) - Temperature: 0.5
**Medium temperature for balanced creativity**

- Creative but consistent
- Follows patterns but not rigid
- Appropriate naming choices
- Reasonable architecture decisions

**Best for:**
- New feature implementation
- Creating new classes/modules
- Designing APIs
- Building components

### Refactor - Temperature: 0.4
**Low-medium for structured improvements**

- Consistent refactoring patterns
- Established design patterns
- Predictable transformations
- Maintainable code

**Best for:**
- Code cleanup
- Pattern application
- Structure improvements
- Performance optimization

### Explain - Temperature: 0.7
**Higher temperature for natural explanations**

- Flowing, natural prose
- Varied phrasing
- Engaging explanations
- Human-like communication

**Best for:**
- Code explanations
- Documentation generation
- Tutorial content
- Conceptual overviews

### Other - Temperature: 0.5
**Medium for general purpose**

- Balanced approach
- Flexible to task needs
- Reasonable default

---

## Model Selection

**Model: `claude-sonnet-4-5-20250929`**

Claude Sonnet 4.5 is used for all operations because it:

✅ **Excellent code generation** - Produces high-quality, idiomatic code
✅ **Strong reasoning** - Handles complex architectural decisions
✅ **Context understanding** - Works well with large codebases
✅ **Speed** - Fast responses for good UX
✅ **Cost-effective** - Balanced performance and pricing
✅ **200K context window** - Can handle large amounts of codebase context

---

## Internal API Calls

prompto makes two internal Claude API calls during optimization:

### 1. Intent Analysis
- **Model:** Claude Sonnet 4.5
- **Max Tokens:** 1,024 (sufficient for JSON response)
- **Temperature:** 0.3 (low for structured output)
- **Purpose:** Extract action, keywords, scope, entities from user prompt

### 2. Pattern Extraction
- **Model:** Claude Sonnet 4.5
- **Max Tokens:** 2,048 (for pattern descriptions)
- **Temperature:** 0.5 (medium for analytical output)
- **Purpose:** Analyze code to identify patterns and conventions

**Total internal cost:** ~3,072 tokens output max per optimization

---

## Context Token Budget

**Context Input Budget: 8,000 tokens**

This is the maximum amount of codebase context included in prompts.

**Why 8,000 tokens?**
- Leaves room for ~192,000 tokens of remaining context window
- Sufficient for 5-10 relevant code files
- Includes project structure, patterns, and symbols
- Balanced between context richness and remaining capacity

**What's included:**
```
<codebase_info>          ~500-1,000 tokens
  - Project structure
  - Architectural patterns
  - Related files list
  - Related symbols
</codebase_info>

<codebase_context>       ~7,000-7,500 tokens
  - Actual code chunks
  - 5-10 most relevant files
  - Filtered and deduplicated
</codebase_context>
```

**Filtering ensures quality:**
- Only >30% relevance code included
- Deduplication removes overlaps
- Semantic boosting prioritizes best matches
- Token-aware selection

---

## Complete Flow with Token Usage

### Example: "Fix authentication error in login handler"

**Step 1: Intent Analysis**
- Input: ~50 tokens (user prompt)
- Output: ~200 tokens (JSON intent)
- **Cost: ~250 tokens**

**Step 2: Codebase Analysis**
- No API calls, uses local index
- **Cost: 0 tokens**

**Step 3: Code Search**
- No API calls, uses local search
- **Cost: 0 tokens**

**Step 4: Pattern Extraction**
- Input: ~2,000 tokens (code snippets)
- Output: ~500 tokens (patterns)
- **Cost: ~2,500 tokens**

**Step 5: Build Optimized Prompt**
- **Output: ~10,000-15,000 token prompt**

**Final Optimized Prompt Structure:**
```
System prompt                    ~200 tokens
Task description                 ~100 tokens
Codebase info                    ~800 tokens
Code context                     ~7,000 tokens
Examples                         ~500 tokens
Thinking prompt                  ~200 tokens
Requirements                     ~300 tokens
Output format                    ~200 tokens
                                 ─────────────
Total input for user's Claude    ~9,300 tokens
```

**User's Claude Response:**
- Max output: **16,384 tokens** (~65,000 chars)
- Actual output: Varies by task (typically 2,000-8,000 tokens)

**Total cost per optimization:**
- Internal API calls: ~2,750 tokens
- User's prompt to Claude: ~9,300 input + up to 16,384 output
- **Total: ~28,434 tokens worst case**

---

## Cost Estimation

**Using Claude Sonnet 4.5 pricing** (as of docs):
- Input: $3 per million tokens
- Output: $15 per million tokens

**Per optimization:**
- Internal calls: ~2,750 tokens output = $0.041
- User's full response (worst case): 16,384 tokens = $0.246
- **Total: ~$0.29 per optimization (worst case)**

**Typical optimization (8,000 token response):**
- Internal: $0.041
- User's response: $0.120
- **Total: ~$0.16 per optimization**

---

## Configuration Locations

### Frontend (TypeScript)
- **Model Config:** `src/agents/prompt-optimizer.ts:455-471`
- **Temperature Map:** `src/agents/prompt-optimizer.ts:457-464`
- **Max Tokens:** `src/agents/prompt-optimizer.ts:469`

### Backend (Rust)
- **API Client:** `src-tauri/src/anthropic/mod.rs`
- **Message Request:** `src-tauri/src/anthropic/models.rs:10-20`

### Types
- **ModelConfig Interface:** `src/types/agent.ts:11-15`

---

## Customization Guide

### To Change Max Tokens

**Frontend:**
```typescript
// src/agents/prompt-optimizer.ts:469
maxTokens: 16384, // Change this value
```

**Options:**
- `4096` - Short responses (1-2 files)
- `8192` - Medium responses (small features)
- `16384` - Long responses (complete implementations)
- `32768` - Maximum (very long responses, higher cost)

### To Adjust Temperature

**Frontend:**
```typescript
// src/agents/prompt-optimizer.ts:457-464
const temperatureMap: Record<PromptIntent['action'], number> = {
  fix: 0.3,       // Adjust for more/less creativity
  modify: 0.3,
  create: 0.5,
  refactor: 0.4,
  explain: 0.7,
  other: 0.5,
};
```

**Guidelines:**
- `0.0-0.3` - Very deterministic (math, debugging)
- `0.3-0.5` - Mostly consistent (code generation)
- `0.5-0.7` - Balanced (features, docs)
- `0.7-1.0` - Creative (brainstorming, examples)

### To Change Context Budget

**Frontend:**
```typescript
// src/agents/prompt-optimizer.ts:98
return this.rankAndFilterContexts(contexts, 8000); // Change this value
```

**Trade-offs:**
- Lower (4,000) - Faster, less context, cheaper
- Higher (12,000) - More context, slower, more expensive

---

## Best Practices

✅ **Do:**
- Keep temperature low (0.3-0.5) for code generation
- Use high max tokens (16K+) to avoid truncation
- Adjust context budget based on codebase size
- Monitor token usage in console logs

❌ **Don't:**
- Set temperature above 0.8 for code (too random)
- Limit max tokens below 4K (will truncate code)
- Exceed 200K total context (API limit)
- Ignore cost implications of high limits

---

## Summary

prompto is configured for **unrestricted, high-quality responses**:

- ✅ **16,384 max tokens** - No artificial limits
- ✅ **Task-specific temperatures** - Optimal for each action
- ✅ **8,000 token context budget** - Rich codebase information
- ✅ **Claude Sonnet 4.5** - Best balance of quality and cost
- ✅ **Smart filtering** - Only relevant code included

This configuration ensures Claude can provide complete, comprehensive responses without truncation while maintaining cost efficiency through intelligent context filtering.
