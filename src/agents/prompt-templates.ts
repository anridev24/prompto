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

<example>
<scenario>Refactoring a function to use async/await</scenario>
<approach>
1. Identify all Promise-based code
2. Convert function to async
3. Replace .then() chains with await
4. Add proper error handling with try/catch
5. Update calling code to handle async nature
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

<examples>
<example>
<bug>Null pointer exception when user object is undefined</bug>
<diagnosis>Missing null check before accessing user properties</diagnosis>
<fix>Add optional chaining or early return with validation</fix>
</example>

<example>
<bug>Race condition causing data inconsistency</bug>
<diagnosis>Multiple async operations modifying shared state</diagnosis>
<fix>Use proper synchronization (locks, atomic operations) or restructure to eliminate shared state</fix>
</example>
</examples>

<thinking>
Debug systematically:
1. What is the root cause of this issue?
2. Under what conditions does the bug occur?
3. What is the minimal fix that addresses the root cause?
4. Could this fix introduce new issues?
5. What preventive measures would catch similar bugs?

Work through the debugging process step-by-step.
</thinking>

<output_format>
Provide:
1. **Root Cause**: Clear explanation of what's causing the bug
2. **Fix**: The corrected code with comments explaining changes
3. **Why This Works**: Explanation of how the fix resolves the issue
4. **Prevention**: Suggestions to prevent similar bugs (tests, type safety, validation)
</output_format>
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

<examples>
<example>
<feature>Add user authentication</feature>
<approach>
1. Create authentication service following existing service patterns
2. Add middleware for route protection
3. Implement token management (JWT)
4. Add login/logout endpoints
5. Update types and interfaces
6. Write unit and integration tests
</approach>
</example>

<example>
<feature>Implement caching layer</feature>
<approach>
1. Choose caching strategy (memory, Redis, etc.)
2. Create cache service with consistent interface
3. Add cache decorators/middleware for easy adoption
4. Implement invalidation strategy
5. Add configuration for cache TTL
6. Monitor cache hit rates
</approach>
</example>
</examples>

<thinking>
Plan the implementation:
1. How should this feature integrate with existing code?
2. What files need to be created or modified?
3. What design patterns should be used?
4. What edge cases need handling?
5. How will this scale?
6. What tests are needed?

Think through the architecture before coding.
</thinking>

<implementation_guidelines>
- Follow existing code patterns and conventions
- Integrate seamlessly with current architecture
- Maintain consistency with similar features
- Consider scalability and maintainability
- Add appropriate documentation and comments
- Ensure backward compatibility
</implementation_guidelines>

<output_format>
Provide:
1. **Architecture**: High-level design and integration points
2. **Implementation**: Complete code with explanatory comments
3. **Usage Examples**: How developers will use this feature
4. **Testing Strategy**: Unit, integration, and edge case tests
5. **Documentation**: API docs or usage guide
</output_format>
  `.trim(),

  // Template for code explanation
  explain: (question: string, codeSnippet: string, context: string) => `
You are explaining code to a developer who wants to understand how something works.

<question>
${question}
</question>

<code_to_explain>
${codeSnippet}
</code_to_explain>

${context ? `<surrounding_context>\n${context}\n</surrounding_context>` : ''}

<examples>
<example>
<code>async function fetchUser(id: string) { return db.users.findOne({ id }); }</code>
<explanation>
This async function retrieves a user from the database. It takes a user ID as input and uses the findOne method to search for a matching record in the users collection. The async/await pattern ensures the database query completes before returning the result.
</explanation>
</example>
</examples>

<thinking>
To explain this code effectively:
1. What is the main purpose of this code?
2. What are the key operations or logic flows?
3. What concepts or patterns are being used?
4. What might be confusing or non-obvious?
5. How does this relate to the broader system?

Think through each aspect before explaining.
</thinking>

<explanation_format>
Structure your explanation as:
1. **Purpose**: What this code does and why it exists
2. **How It Works**: Step-by-step breakdown of the logic
3. **Key Concepts**: Important patterns, algorithms, or techniques used
4. **Important Details**: Edge cases, gotchas, or subtle behaviors
5. **Context**: How this fits into the larger system
</explanation_format>

Provide a clear, comprehensive explanation that helps the developer truly understand the code.
  `.trim(),

  // Template for refactoring
  refactor: (
    refactorGoal: string,
    currentCode: string,
    constraints?: string
  ) => `
You are refactoring code to improve its quality while maintaining functionality.

<refactoring_goal>
${refactorGoal}
</refactoring_goal>

<current_implementation>
${currentCode}
</current_implementation>

${constraints ? `<constraints>\n${constraints}\n</constraints>` : ''}

<examples>
<example>
<before>
function processUsers(users) {
  for (let i = 0; i < users.length; i++) {
    if (users[i].active) {
      sendEmail(users[i].email, 'Welcome');
    }
  }
}
</before>
<after>
function processUsers(users: User[]): void {
  const activeUsers = users.filter(user => user.active);
  activeUsers.forEach(user => sendWelcomeEmail(user));
}

function sendWelcomeEmail(user: User): void {
  sendEmail(user.email, 'Welcome');
}
</after>
<improvements>Type safety, functional style, single responsibility, better naming</improvements>
</example>
</examples>

<thinking>
Analyze the refactoring opportunity:
1. What are the current code smells or issues?
2. What specific improvements would most benefit this code?
3. How can we maintain functionality while improving structure?
4. What refactoring patterns apply here (extract method, introduce parameter object, etc.)?
5. Are there any risks or trade-offs?

Plan the refactoring step-by-step.
</thinking>

<refactoring_principles>
- Maintain existing functionality (no behavior changes)
- Improve code readability and maintainability
- Follow SOLID principles where applicable
- Reduce code duplication (DRY)
- Enhance testability
- Improve type safety
- Better naming and structure
</refactoring_principles>

<output_format>
Provide:
1. **Code Smells**: What issues exist in the current code
2. **Refactoring Strategy**: Which refactoring patterns you'll apply
3. **Refactored Code**: The improved implementation with comments
4. **Benefits**: Specific improvements gained (readability, performance, maintainability)
5. **Testing Notes**: How to verify functionality is preserved
</output_format>
  `.trim(),
};
