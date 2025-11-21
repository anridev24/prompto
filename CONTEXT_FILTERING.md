# Context Filtering & Relevance System

This document explains how prompto ensures only **relevant code** is included in optimized prompts.

---

## Problem Statement

When optimizing prompts, including irrelevant code can:
- ❌ Confuse Claude with unrelated context
- ❌ Waste tokens and increase costs
- ❌ Reduce response quality
- ❌ Dilute focus from truly relevant code

---

## Multi-Stage Filtering Pipeline

prompto uses a **4-stage filtering pipeline** to ensure only relevant code is included:

```
Raw Search Results (20 items)
    ↓
[1] Semantic Filtering (intent-aware boosting)
    ↓
[2] Relevance Threshold (>30% score required)
    ↓
[3] Deduplication (remove overlaps)
    ↓
[4] Token Budget (8k token limit)
    ↓
Final Context (3-8 highly relevant items)
```

---

## Stage 1: Semantic Filtering

**Purpose:** Boost relevance scores based on intent and keywords

**How it works:**
```typescript
private applySemanticFiltering(contexts: CodeContext[], intent: PromptIntent) {
  // Count keyword matches
  for (const term of intent.keywords + intent.entities) {
    if (content.includes(term) || filePath.includes(term)) {
      matchCount++;
    }
  }

  // Boost: up to +30% for keyword matches
  const termMatchBoost = (matchCount / totalTerms) * 0.3;

  // Action-specific boosts:
  // - fix: boost code with error handling (+20%)
  // - create: boost code with patterns/exports (+15%)
  // - refactor: boost complex code blocks (+10%)
}
```

**Example:**
- User prompt: "Fix authentication error in login function"
- Keywords: `authentication`, `error`, `login`, `function`
- Code with all 4 keywords gets +30% relevance boost
- Code with error handling gets additional +20% boost
- Total: potentially +50% relevance boost for highly relevant code

---

## Stage 2: Relevance Threshold

**Purpose:** Filter out low-quality matches

**Threshold:** `0.3` (30%)

**Logic:**
```typescript
const RELEVANCE_THRESHOLD = 0.3;
const relevantContexts = contexts.filter(ctx => ctx.relevance >= RELEVANCE_THRESHOLD);

if (relevantContexts.length === 0) {
  console.log('No contexts meet relevance threshold, returning empty');
  return [];
}
```

**Why 30%?**
- Too low (e.g., 10%) → Still includes irrelevant code
- Too high (e.g., 70%) → Misses potentially useful context
- 30% → Sweet spot for quality vs. coverage

**Result:** Only code with meaningful relevance passes through

---

## Stage 3: Deduplication

**Purpose:** Remove duplicate and overlapping code

### 3A: Exact Duplicate Removal

```typescript
const key = `${filePath}:${startLine}-${endLine}`;
if (!seen.has(key)) {
  seen.add(key);
  deduplicated.push(context);
}
```

**Removes:** Identical file path + line range matches

### 3B: Overlapping Range Removal

```typescript
private rangesOverlap(a: CodeContext, b: CodeContext): boolean {
  return a.startLine <= b.endLine && b.startLine <= a.endLine;
}
```

**Logic:**
1. Group contexts by file path
2. Check for overlapping line ranges within each file
3. Keep the higher-relevance version
4. Discard the lower-relevance version

**Example:**
```
Context A: utils.ts:10-30 (relevance: 0.8)
Context B: utils.ts:25-45 (relevance: 0.6)
Lines 25-30 overlap → Keep A, discard B
```

**Result:** No duplicate or overlapping code in final context

---

## Stage 4: Token Budget Management

**Purpose:** Stay within token limits for cost and context window

**Budget:** `8,000 tokens` (~32,000 characters of code)

**Logic:**
```typescript
const selected: CodeContext[] = [];
let totalTokens = 0;

for (const context of sorted) {
  const contextTokens = this.estimateTokens(context.content);

  if (totalTokens + contextTokens <= maxTokens) {
    selected.push(context);
    totalTokens += contextTokens;
  } else if (selected.length === 0) {
    // Always include at least top result
    selected.push(context);
    break;
  } else {
    break; // Budget exceeded
  }
}
```

**Token Estimation:**
```typescript
private estimateTokens(text: string): number {
  return Math.ceil(text.length / 4); // ~4 characters per token
}
```

**Result:** Maximum context within budget, prioritized by relevance

---

## Console Logging

prompto logs the filtering process for transparency:

```
Filtered: 20 → 18 (threshold) → 15 (dedup) → 8 (budget)
Total estimated tokens: 7,234
```

**Interpretation:**
- Started with 20 search results
- 2 filtered out by relevance threshold
- 3 removed as duplicates/overlaps
- 7 excluded due to token budget
- Final: 8 highly relevant contexts using 7,234 tokens

---

## Action-Specific Relevance Boosting

Different task types prioritize different code patterns:

### Fix (Bug Fixes)
**Boost code with:**
- `error`, `catch`, `throw`, `validate`
- Error handling patterns
- **Why:** Bug fixes often involve error paths

### Create (New Features)
**Boost code with:**
- `class`, `function`, `interface`, `export`
- Structural patterns
- **Why:** Need to understand existing patterns to create consistent new code

### Refactor
**Boost code with:**
- Large code blocks (>500 characters)
- Complex logic
- **Why:** Refactoring targets complex, verbose code

---

## Real-World Example

### Scenario
**User prompt:** "Fix the authentication error in the login handler"

### Filtering Process

**Initial Results (20):**
1. `auth/login.ts:45-80` - login handler (score: 0.5)
2. `auth/middleware.ts:10-30` - auth middleware (score: 0.4)
3. `utils/logger.ts:50-60` - error logging (score: 0.3)
4. `config/database.ts:100-120` - DB config (score: 0.2)
5. `auth/login.ts:45-85` - duplicate login handler (score: 0.48)
6. ... 15 more results

**Stage 1 - Semantic Filtering:**
- Keywords: `authentication`, `error`, `login`, `handler`
- `auth/login.ts:45-80` has all keywords → +30% boost → 0.8
- `auth/middleware.ts:10-30` has 2 keywords + error handling → +15% + 20% → 0.75
- `utils/logger.ts:50-60` has 1 keyword → +7.5% → 0.375

**Stage 2 - Relevance Threshold (>0.3):**
- ❌ `config/database.ts:100-120` (0.2) - FILTERED OUT
- ✅ All others pass (>0.3)

**Stage 3 - Deduplication:**
- ❌ `auth/login.ts:45-85` (0.48) - overlaps with 45-80 (0.8) → REMOVED

**Stage 4 - Token Budget:**
- Selected top 3 contexts totaling ~6,000 tokens
- Remaining contexts excluded due to budget

### Final Context (3 items)
1. `auth/login.ts:45-80` - login handler (0.8) - 2,500 tokens
2. `auth/middleware.ts:10-30` - auth middleware (0.75) - 2,000 tokens
3. `utils/logger.ts:50-60` - error logging (0.375) - 1,500 tokens

**Result:** Highly relevant, deduplicated context focused on authentication and error handling.

---

## Benefits

### Quality
✅ **No irrelevant code** - Strict 30% relevance threshold
✅ **No duplicates** - Exact and overlap deduplication
✅ **Intent-aligned** - Action-specific boosting
✅ **Keyword-focused** - Boosts code matching user's terms

### Efficiency
✅ **Token optimized** - 8k budget ensures staying within limits
✅ **Cost effective** - Only send necessary context
✅ **Performance** - Less context = faster responses

### User Experience
✅ **Better prompts** - Focused, relevant context
✅ **Higher quality responses** - Claude isn't confused by noise
✅ **Transparency** - Console logs show filtering process

---

## Configuration

### Adjusting Thresholds

**Relevance Threshold** (`src/agents/prompt-optimizer.ts:113`)
```typescript
const RELEVANCE_THRESHOLD = 0.3; // Adjust between 0.1 (permissive) to 0.5 (strict)
```

**Token Budget** (`src/agents/prompt-optimizer.ts:98`)
```typescript
return this.rankAndFilterContexts(contexts, 8000); // Adjust based on needs
```

**Semantic Boost Weights** (`src/agents/prompt-optimizer.ts:128-157`)
```typescript
const termMatchBoost = (matchCount / allTerms.size) * 0.3; // Term match boost
actionBoost = 0.2; // Action-specific boost
```

---

## Future Enhancements

Potential improvements:
1. **User-adjustable relevance threshold** - Let users control strictness
2. **ML-based relevance scoring** - Use embeddings for better semantic matching
3. **Cross-file dependency detection** - Include related files automatically
4. **Historical learning** - Learn which contexts were most useful
5. **Real-time token counting** - Use official tokenizer API

---

## Summary

prompto's context filtering system ensures **only relevant code** reaches Claude:

1. ✅ Semantic filtering boosts relevant code
2. ✅ 30% relevance threshold filters noise
3. ✅ Deduplication removes redundancy
4. ✅ Token budget optimizes efficiency

**Result:** High-quality, focused prompts that generate better code.
