# Comprehensive Codebase Analysis

This document explains how prompto maximizes indexing usage to provide rich, contextual prompts through comprehensive codebase analysis.

---

## Overview

The **CodebaseAnalyzer** collects comprehensive information about your project to create highly contextual, architecture-aware prompts. It goes beyond simple code search to understand:

- 🏗️ Project structure and organization
- 📦 Language and framework usage
- 🔗 File relationships and dependencies
- 🎯 Related symbols (functions, classes, interfaces)
- 🏛️ Architectural patterns and conventions

---

## Multi-Dimensional Analysis

### 1. Project Structure Analysis

**What it collects:**
- Total file count
- Language distribution (TypeScript: 50, Rust: 30, etc.)
- Root path
- File categorization by role

**File Categories:**
- **Components** - UI components and views
- **Services** - Business logic and API services
- **Utilities** - Helper functions and utilities
- **Types** - Type definitions and interfaces
- **Tests** - Test files
- **Configs** - Configuration files

**Example Output:**
```xml
<project_structure>
Total Files: 142
Languages: TypeScript (89), Rust (48), JSON (5)

File Organization:
  components: 23 files
  services: 15 files
  utilities: 12 files
  types: 8 files
  tests: 7 files
  configs: 3 files
</project_structure>
```

---

### 2. Architectural Pattern Detection

**What it detects:**
- **Frameworks**: React, Vue, Angular, Next.js, Tauri, Electron
- **Architecture patterns**: Service layer, component-based, state management
- **Integration patterns**: API layers, backend integration

**Detection Methods:**
- File name pattern matching (`*react*`, `*service*`, etc.)
- Import statement analysis
- Directory structure patterns

**Example Output:**
```xml
<architectural_patterns>
- Primary languages: TypeScript, Rust
- Uses React framework
- Uses Tauri for desktop app
- Component-based architecture
- Service layer architecture
- State management with stores
- API/backend integration layer
</architectural_patterns>
```

---

### 3. Related Files Discovery

**How it works:**
1. Takes user's intent keywords and entities
2. Searches for files matching those terms
3. Deduplicates and limits to top 15 most relevant

**Example:**
- User prompt: "Fix authentication error in login handler"
- Keywords: `authentication`, `error`, `login`, `handler`
- Related files found:
  - `src/auth/login.ts`
  - `src/auth/middleware.ts`
  - `src/services/auth-service.ts`
  - `src/types/auth-types.ts`
  - `src/utils/error-handler.ts`

**Example Output:**
```xml
<related_files>
- src/auth/login.ts
- src/auth/middleware.ts
- src/services/auth-service.ts
- src/types/auth-types.ts
- src/utils/error-handler.ts
... and 3 more
</related_files>
```

---

### 4. Symbol-Based Discovery

**What it finds:**
- Functions matching intent keywords
- Classes related to the task
- Interfaces and types
- Methods and constants

**Process:**
1. Get related files from step 3
2. Extract all symbols from top 5 files
3. Filter symbols by keyword relevance
4. Deduplicate by file + name
5. Limit to 20 most relevant

**Example Output:**
```xml
<related_symbols>

Functions:
  - authenticateUser (src/auth/login.ts:45)
  - validateCredentials (src/auth/login.ts:78)
  - handleLoginError (src/auth/login.ts:102)

Classes:
  - AuthService (src/services/auth-service.ts:12)
  - LoginHandler (src/auth/login.ts:10)

Interfaces:
  - AuthCredentials (src/types/auth-types.ts:5)
  - LoginResult (src/types/auth-types.ts:15)
</related_symbols>
```

---

### 5. Dependency Tracking

**Feature: `findDependentFiles()`**

Finds files that depend on a given file (importers).

**How it works:**
1. Extract filename without extension
2. Search for files containing that filename
3. Filter out the target file itself

**Use case:** Understanding impact of changes

**Example:**
```typescript
// Find files that import auth-service.ts
const dependents = await analyzer.findDependentFiles('src/services/auth-service.ts');
// Returns: ['src/auth/login.ts', 'src/auth/register.ts', 'src/middleware/auth.ts']
```

---

### 6. Semantic Code Search

**Feature: `findSemanticallySimilarCode()`**

Uses ML embeddings to find conceptually similar code.

**How it works:**
1. Takes natural language description
2. Generates embedding for description
3. Searches vector store for similar embeddings
4. Returns most similar code chunks

**Example:**
```typescript
// Find code similar to a concept
const similar = await analyzer.findSemanticallySimilarCode(
  'error handling for API requests',
  10
);
// Returns code chunks with try-catch, error callbacks, API error handling
```

---

## Integration with Prompt Optimization

### Enhanced Prompt Building

The optimizer now includes **7 steps** instead of 4:

1. **Intent Analysis** - Extract action, keywords, scope, entities
2. **Codebase Analysis** - Collect project structure and patterns
3. **Code Search** - Find relevant code chunks
4. **Template Selection** - Pick appropriate template
5. **Context Assembly** - Combine codebase info + code chunks
6. **Model Config** - Set temperature and parameters
7. **Prompt Generation** - Build final optimized prompt

### Context Sections in Optimized Prompts

Every optimized prompt now includes:

```xml
<codebase_info>
  <project_structure>...</project_structure>
  <architectural_patterns>...</architectural_patterns>
  <related_files>...</related_files>
  <related_symbols>...</related_symbols>
</codebase_info>

<codebase_context>
  <code>
    <!-- Actual code chunks with line numbers -->
  </code>
</codebase_context>
```

---

## Real-World Example

### User Prompt
"Add error handling to the authentication service"

### Step 1: Intent Analysis
```json
{
  "action": "modify",
  "keywords": ["error", "handling", "authentication", "service"],
  "scope": "module",
  "entities": ["authentication service"]
}
```

### Step 2: Codebase Analysis

**Project Structure:**
```
Total Files: 156
Languages: TypeScript (102), Rust (48), JSON (6)
File Organization:
  services: 18 files
  components: 34 files
  types: 12 files
```

**Architectural Patterns:**
```
- Primary languages: TypeScript, Rust
- Uses React framework
- Service layer architecture
- State management with stores
```

**Related Files (8 found):**
```
- src/services/auth-service.ts
- src/types/auth-types.ts
- src/utils/error-handler.ts
- src/middleware/auth-middleware.ts
- src/services/api-service.ts
- src/services/logger-service.ts
- src/hooks/use-auth.ts
- src/store/auth-store.ts
```

**Related Symbols (12 found):**
```
Functions:
  - authenticateUser (auth-service.ts:23)
  - refreshToken (auth-service.ts:45)
  - logout (auth-service.ts:67)

Classes:
  - AuthService (auth-service.ts:10)
  - ApiService (api-service.ts:8)

Interfaces:
  - AuthCredentials (auth-types.ts:5)
  - AuthResponse (auth-types.ts:12)
  - AuthError (auth-types.ts:20)
```

### Step 3: Code Search

Finds 5 code chunks from:
- `auth-service.ts` (main service implementation)
- `error-handler.ts` (error handling utilities)
- `api-service.ts` (API error patterns)
- `auth-types.ts` (error type definitions)
- `logger-service.ts` (logging patterns)

### Final Optimized Prompt

```
You are an expert software engineer working on a codebase.

<task>
Add error handling to the authentication service
</task>

<codebase_info>
<project_structure>
Total Files: 156
Languages: TypeScript (102), Rust (48), JSON (6)

File Organization:
  services: 18 files
  components: 34 files
  types: 12 files
</project_structure>

<architectural_patterns>
- Primary languages: TypeScript, Rust
- Uses React framework
- Service layer architecture
- State management with stores
</architectural_patterns>

<related_files>
- src/services/auth-service.ts
- src/types/auth-types.ts
- src/utils/error-handler.ts
- src/middleware/auth-middleware.ts
... and 4 more
</related_files>

<related_symbols>

Functions:
  - authenticateUser (auth-service.ts:23)
  - refreshToken (auth-service.ts:45)
  - logout (auth-service.ts:67)

Classes:
  - AuthService (auth-service.ts:10)
  - ApiService (api-service.ts:8)

Interfaces:
  - AuthCredentials (auth-types.ts:5)
  - AuthResponse (auth-types.ts:12)
  - AuthError (auth-types.ts:20)
</related_symbols>
</codebase_info>

<codebase_context>
<relevant_files>
- src/services/auth-service.ts
- src/utils/error-handler.ts
- src/types/auth-types.ts
</relevant_files>

<code>
<file path="src/services/auth-service.ts" lines="10-45" language="typescript">
export class AuthService {
  async authenticateUser(credentials: AuthCredentials): Promise<AuthResponse> {
    const response = await fetch('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify(credentials),
    });
    return response.json();
  }
}
</file>

<file path="src/utils/error-handler.ts" lines="5-30" language="typescript">
export class ErrorHandler {
  static handleApiError(error: Error): void {
    console.error('API Error:', error);
    // Could add more sophisticated error handling
  }
}
</file>
</code>
</codebase_context>

<examples>
<example>
<scenario>Adding validation to an API endpoint</scenario>
<approach>
1. Identify the validation requirements
2. Add validation logic before processing
3. Return appropriate error responses
4. Update types/interfaces if needed
5. Add tests for validation cases
</approach>
</example>
</examples>

<thinking>
Before making changes, analyze:
1. What specific changes are needed?
2. Which files need to be modified?
3. What edge cases should be considered?
4. Are there any dependencies or side effects?
5. How can I maintain backward compatibility?

Provide your detailed reasoning here.
</thinking>

<requirements>
- Make minimal, focused changes
- Preserve existing code style and patterns
- Ensure backward compatibility
- Add appropriate error handling
- Update related tests if needed
</requirements>

<output_format>
Provide your response as:
1. **Analysis**: Brief explanation of changes needed
2. **Implementation**: Code changes with inline comments
3. **Testing**: Suggested test cases
4. **Migration Notes**: Any breaking changes or migration steps
</output_format>
```

---

## Benefits

### 1. **Context-Aware Code Generation**
Claude understands where your code fits in the architecture, leading to:
- Better naming consistency
- Appropriate error handling patterns
- Correct import statements
- Architectural alignment

### 2. **Reduced Hallucinations**
With comprehensive context, Claude:
- Doesn't invent non-existent functions
- Uses actual types from your codebase
- Follows your project's patterns

### 3. **Better Integration**
Generated code integrates seamlessly because Claude knows:
- Your service layer patterns
- Error handling conventions
- State management approach
- Testing patterns

### 4. **Faster Development**
Developers spend less time:
- Finding related files manually
- Understanding dependencies
- Fixing integration issues
- Refactoring to match patterns

---

## Performance Optimization

### Parallel Requests
The analyzer runs multiple searches in parallel:
```typescript
const [components, services, utilities, types, tests, configs] = await Promise.all([
  searchFiles('component', 20),
  searchFiles('service', 20),
  searchFiles('util', 20),
  // ...
]);
```

### Caching
- Index stats cached after first load
- File categorization cached
- Symbol lookups cached per file

### Limits
- **Files per category**: 10-20
- **Related files**: 15 max
- **Related symbols**: 20 max
- **Semantic results**: 10 max

These limits ensure fast performance while providing sufficient context.

---

## Console Logging

The analyzer provides detailed logging:

```
Analyzing codebase structure and patterns...
Codebase analysis complete: {
  relatedFiles: 8,
  relatedSymbols: 12,
  patterns: 6
}
```

This helps developers understand what context was collected.

---

## Future Enhancements

Potential improvements:

1. **Import Graph Analysis** - Build actual dependency graph from imports
2. **Test Coverage Detection** - Identify which code has tests
3. **Recent Changes** - Prioritize recently modified files
4. **Usage Frequency** - Track which files are most commonly modified together
5. **Complexity Metrics** - Identify complex code that might need refactoring
6. **Documentation Coverage** - Find undocumented code
7. **Cross-Language Links** - Connect Rust backend with TypeScript frontend

---

## Summary

The **CodebaseAnalyzer** transforms prompto from a simple code search tool into a **comprehensive codebase intelligence system**:

✅ **Project-aware** - Understands your architecture
✅ **Relationship-tracking** - Knows file dependencies
✅ **Symbol-level precision** - Finds exact functions/classes
✅ **Pattern-detecting** - Identifies conventions and frameworks
✅ **Context-rich** - Provides maximum relevant information
✅ **Performance-optimized** - Fast parallel searches with limits

This results in **dramatically better prompts** that produce **higher quality, more integrated code**.
