import Anthropic from '@anthropic-ai/sdk';
import { IntentAnalyzer } from './intent-analyzer';
import { promptTemplates } from './prompt-templates';
import { queryIndex } from '../lib/tauri-api';
import type {
  OptimizedPrompt,
  CodeContext,
  PromptIntent,
  CodeChunk,
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
    console.log('Analyzing prompt intent...');
    const intent = await this.intentAnalyzer.analyzeIntent(rawPrompt);
    console.log('Intent analyzed:', intent);

    // Step 2: Query codebase for relevant context
    console.log('Querying codebase for context...');
    const codeContext = await this.queryCodebase(intent);
    console.log(`Found ${codeContext.length} relevant code chunks`);

    // Step 3: Select appropriate template
    const template = this.selectTemplate(intent);

    // Step 4: Build optimized prompt
    console.log('Building optimized prompt...');
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
      // Combine keywords and entities for search
      const searchTerms = [...intent.keywords, ...intent.entities].filter(
        (term) => term.length > 2
      );

      if (searchTerms.length === 0) {
        return [];
      }

      // Query Rust backend for relevant code
      const results = await queryIndex({
        keywords: searchTerms,
        max_results: 10,
      });

      return results.map((chunk: CodeChunk) => ({
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

  private selectTemplate(intent: PromptIntent): keyof typeof promptTemplates {
    switch (intent.action) {
      case 'create':
        return 'create';
      case 'modify':
        return 'modify';
      case 'fix':
        return 'fix';
      case 'explain':
        return 'explain';
      case 'refactor':
        return 'refactor';
      default:
        return 'base';
    }
  }

  private async buildOptimizedPrompt(
    rawPrompt: string,
    _intent: PromptIntent,
    codeContext: CodeContext[],
    templateType: keyof typeof promptTemplates
  ): Promise<string> {
    // Format code context
    const formattedContext = this.formatCodeContext(codeContext);

    // Apply template based on type
    switch (templateType) {
      case 'modify': {
        const targetFiles = [...new Set(codeContext.map((c) => c.filePath))];
        return promptTemplates.modify(rawPrompt, targetFiles, formattedContext);
      }

      case 'fix':
        return promptTemplates.fix(rawPrompt, formattedContext);

      case 'create': {
        const patterns = await this.extractPatterns(codeContext);
        return promptTemplates.create(rawPrompt, patterns, formattedContext);
      }

      case 'explain': {
        const mainCode = codeContext[0]?.content || '';
        const context = codeContext
          .slice(1)
          .map((c) => c.content)
          .join('\n\n');
        return promptTemplates.explain(rawPrompt, mainCode, context);
      }

      case 'refactor':
        return promptTemplates.refactor(rawPrompt, formattedContext);

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
    if (contexts.length === 0) {
      return 'No code patterns available to analyze.';
    }

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
