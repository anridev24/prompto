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

  // Template for refactoring
  refactor: (
    refactorGoal: string,
    currentCode: string,
    constraints?: string
  ) => `
You are refactoring code to improve its quality.

<refactoring_goal>
${refactorGoal}
</refactoring_goal>

<current_implementation>
${currentCode}
</current_implementation>

${constraints ? `<constraints>\n${constraints}\n</constraints>` : ''}

<refactoring_principles>
- Maintain existing functionality
- Improve code readability and maintainability
- Follow SOLID principles where applicable
- Reduce code duplication
- Enhance testability
</refactoring_principles>

Provide:
1. Analysis of current issues
2. Proposed refactoring approach
3. Refactored code
4. Benefits of the changes
  `.trim(),
};
