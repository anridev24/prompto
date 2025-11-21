import { invoke } from '@tauri-apps/api/core';
import { IntentAnalyzer } from './intent-analyzer';
import { promptTemplates } from './prompt-templates';
import { CodebaseAnalyzer } from './codebase-analyzer';
import { queryIndex } from '../lib/tauri-api';
import type {
  OptimizedPrompt,
  CodeContext,
  PromptIntent,
  CodeChunk,
  ModelConfig,
} from '../types/agent';

export class PromptOptimizerAgent {
  private apiKey: string;
  private intentAnalyzer: IntentAnalyzer;
  private codebaseAnalyzer: CodebaseAnalyzer;
  private indexedPath: string | null = null;

  constructor(apiKey: string) {
    this.apiKey = apiKey;
    this.intentAnalyzer = new IntentAnalyzer(apiKey);
    this.codebaseAnalyzer = new CodebaseAnalyzer();
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

    // Step 2: Analyze codebase for comprehensive context
    console.log('Analyzing codebase structure and patterns...');
    const codebaseContext = await this.codebaseAnalyzer.analyzeForIntent(
      intent.keywords,
      intent.entities
    );
    console.log('Codebase analysis complete:', {
      relatedFiles: codebaseContext.relatedFiles.length,
      relatedSymbols: codebaseContext.relatedSymbols.length,
      patterns: codebaseContext.architecturalPatterns.length,
    });

    // Step 3: Query codebase for relevant code chunks
    console.log('Querying codebase for relevant code...');
    const codeContext = await this.queryCodebase(intent);
    console.log(`Found ${codeContext.length} relevant code chunks`);

    // Step 4: Select appropriate template
    const template = this.selectTemplate(intent);

    // Step 5: Build optimized prompt with codebase context
    console.log('Building optimized prompt...');
    const optimized = await this.buildOptimizedPrompt(
      rawPrompt,
      intent,
      codeContext,
      codebaseContext,
      template
    );

    // Step 6: Get model configuration for this intent
    const modelConfig = this.getModelConfigForIntent(intent);

    return {
      original: rawPrompt,
      optimized,
      codeContext,
      appliedPractices: this.getAppliedPractices(intent),
      modelConfig,
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

      // Query Rust backend for relevant code (request more results for ranking)
      const results = await queryIndex({
        keywords: searchTerms,
        max_results: 20,
      });

      // Map and rank contexts
      const contexts = results.map((chunk: CodeChunk) => ({
        filePath: chunk.file_path,
        startLine: chunk.start_line,
        endLine: chunk.end_line,
        content: chunk.content,
        language: chunk.language,
        relevance: chunk.relevance_score,
      }));

      // Apply semantic filtering to boost relevance based on intent
      const semanticallyFiltered = this.applySemanticFiltering(contexts, intent);

      // Rank and filter contexts based on relevance and token budget
      return this.rankAndFilterContexts(semanticallyFiltered, 8000); // ~8k token budget for context
    } catch (error) {
      console.error('Failed to query codebase:', error);
      return [];
    }
  }

  private applySemanticFiltering(
    contexts: CodeContext[],
    intent: PromptIntent
  ): CodeContext[] {
    // Create a set of all relevant terms (lowercase for matching)
    const allTerms = new Set([
      ...intent.keywords.map((k) => k.toLowerCase()),
      ...intent.entities.map((e) => e.toLowerCase()),
    ]);

    return contexts.map((context) => {
      const contentLower = context.content.toLowerCase();
      const filePathLower = context.filePath.toLowerCase();

      // Count how many intent terms appear in this context
      let matchCount = 0;
      for (const term of allTerms) {
        if (contentLower.includes(term) || filePathLower.includes(term)) {
          matchCount++;
        }
      }

      // Boost relevance score based on term matches
      const termMatchBoost = (matchCount / allTerms.size) * 0.3; // Up to 30% boost

      // Apply action-specific filters
      let actionBoost = 0;

      if (intent.action === 'fix') {
        // For bug fixes, boost contexts with error handling, try/catch, validation
        if (
          contentLower.includes('error') ||
          contentLower.includes('catch') ||
          contentLower.includes('throw') ||
          contentLower.includes('validate')
        ) {
          actionBoost = 0.2;
        }
      } else if (intent.action === 'create') {
        // For creation, boost contexts with similar patterns (class, function, interface)
        if (
          contentLower.includes('class ') ||
          contentLower.includes('function ') ||
          contentLower.includes('interface ') ||
          contentLower.includes('export ')
        ) {
          actionBoost = 0.15;
        }
      } else if (intent.action === 'refactor') {
        // For refactoring, boost longer code blocks with complex logic
        if (context.content.length > 500) {
          actionBoost = 0.1;
        }
      }

      // Calculate adjusted relevance (capped at 1.0)
      const adjustedRelevance = Math.min(
        1.0,
        context.relevance + termMatchBoost + actionBoost
      );

      return {
        ...context,
        relevance: adjustedRelevance,
      };
    });
  }

  private estimateTokens(text: string): number {
    // Rough estimation: ~4 characters per token for code
    // This is conservative; Claude's actual tokenizer may differ
    return Math.ceil(text.length / 4);
  }

  private rankAndFilterContexts(
    contexts: CodeContext[],
    maxTokens: number
  ): CodeContext[] {
    // Step 1: Filter out low-relevance results (relevance threshold)
    const RELEVANCE_THRESHOLD = 0.3; // Only include results with >30% relevance
    const relevantContexts = contexts.filter(
      (ctx) => ctx.relevance >= RELEVANCE_THRESHOLD
    );

    if (relevantContexts.length === 0) {
      console.log('No contexts meet relevance threshold, returning empty');
      return [];
    }

    // Step 2: Remove duplicate or highly overlapping contexts
    const deduplicated = this.deduplicateContexts(relevantContexts);

    // Step 3: Sort by relevance score (descending)
    const sorted = [...deduplicated].sort((a, b) => b.relevance - a.relevance);

    // Step 4: Select contexts within token budget
    const selected: CodeContext[] = [];
    let totalTokens = 0;

    for (const context of sorted) {
      const contextTokens = this.estimateTokens(context.content);

      // Include context if it fits within budget
      if (totalTokens + contextTokens <= maxTokens) {
        selected.push(context);
        totalTokens += contextTokens;
      } else if (selected.length === 0) {
        // Always include at least the top result, even if it exceeds budget
        selected.push(context);
        break;
      } else {
        // Budget exceeded, stop adding contexts
        break;
      }
    }

    console.log(
      `Filtered: ${contexts.length} → ${relevantContexts.length} (threshold) → ${deduplicated.length} (dedup) → ${selected.length} (budget)`
    );
    console.log(`Total estimated tokens: ${totalTokens}`);

    return selected;
  }

  private deduplicateContexts(contexts: CodeContext[]): CodeContext[] {
    const seen = new Set<string>();
    const deduplicated: CodeContext[] = [];

    for (const context of contexts) {
      // Create a key based on file path and line range
      const key = `${context.filePath}:${context.startLine}-${context.endLine}`;

      if (!seen.has(key)) {
        seen.add(key);
        deduplicated.push(context);
      }
    }

    // Also check for overlapping ranges in the same file
    const grouped = new Map<string, CodeContext[]>();
    for (const context of deduplicated) {
      const file = context.filePath;
      if (!grouped.has(file)) {
        grouped.set(file, []);
      }
      grouped.get(file)!.push(context);
    }

    const final: CodeContext[] = [];
    for (const [, fileContexts] of grouped) {
      // Sort by start line
      fileContexts.sort((a, b) => a.startLine - b.startLine);

      // Remove overlapping contexts (prefer higher relevance)
      for (let i = 0; i < fileContexts.length; i++) {
        const current = fileContexts[i];
        let shouldInclude = true;

        // Check for overlap with already included contexts from this file
        for (const included of final.filter((c) => c.filePath === current.filePath)) {
          if (this.rangesOverlap(current, included)) {
            // Keep the one with higher relevance
            if (current.relevance <= included.relevance) {
              shouldInclude = false;
              break;
            } else {
              // Remove the lower relevance one
              const idx = final.indexOf(included);
              if (idx > -1) {
                final.splice(idx, 1);
              }
            }
          }
        }

        if (shouldInclude) {
          final.push(current);
        }
      }
    }

    return final;
  }

  private rangesOverlap(a: CodeContext, b: CodeContext): boolean {
    // Check if two line ranges overlap
    return a.startLine <= b.endLine && b.startLine <= a.endLine;
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
    codebaseContext: any,
    templateType: keyof typeof promptTemplates
  ): Promise<string> {
    // Format code context
    const formattedContext = this.formatCodeContext(codeContext);

    // Generate codebase context summary
    const codebaseSummary = this.codebaseAnalyzer.formatCodebaseContextSummary(codebaseContext);

    // Build comprehensive context
    const fullContext = `${codebaseSummary}\n\n${formattedContext}`;

    // Apply template based on type
    switch (templateType) {
      case 'modify': {
        const targetFiles = [...new Set(codeContext.map((c) => c.filePath))];
        return promptTemplates.modify(rawPrompt, targetFiles, fullContext);
      }

      case 'fix':
        return promptTemplates.fix(rawPrompt, fullContext);

      case 'create': {
        const patterns = await this.extractPatterns(codeContext);
        const enhancedPatterns = `${patterns}\n\nArchitectural Context:\n${codebaseContext.architecturalPatterns.join('\n')}`;
        return promptTemplates.create(rawPrompt, enhancedPatterns, fullContext);
      }

      case 'explain': {
        const mainCode = codeContext[0]?.content || '';
        const context = codeContext
          .slice(1)
          .map((c) => c.content)
          .join('\n\n');
        return promptTemplates.explain(rawPrompt, mainCode, `${codebaseSummary}\n\n${context}`);
      }

      case 'refactor':
        return promptTemplates.refactor(rawPrompt, fullContext);

      default:
        return promptTemplates.base(rawPrompt, fullContext);
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
      // Call Tauri backend instead of direct API call
      const patterns = await invoke<string>('extract_patterns', {
        apiKey: this.apiKey,
        codeSnippets,
      });
      return patterns;
    } catch (error) {
      console.error('Failed to extract patterns:', error);
      return 'Unable to extract patterns from codebase.';
    }
  }

  private getAppliedPractices(intent: PromptIntent): string[] {
    const practices = [
      'Claude 4.5 system prompts',
      'Structured prompt with XML tags',
      'Comprehensive codebase analysis',
      'Project structure and patterns included',
      'Related files and symbols identified',
      'Architectural context provided',
      'Chain-of-thought reasoning with <thinking> tags',
      'Few-shot examples provided',
      'Structured output format specified',
      'Multi-stage context filtering (relevance, deduplication, token budget)',
    ];

    if (intent.action === 'modify' || intent.action === 'fix') {
      practices.push('Explicit requirements for changes');
      practices.push('Low temperature (0.3) for deterministic code generation');
      practices.push('Related file dependencies identified');
    }

    if (intent.action === 'create') {
      practices.push('Existing patterns and conventions provided');
      practices.push('Architecture-first approach');
      practices.push('Moderate temperature (0.5) for creative but consistent code');
      practices.push('Framework and library patterns detected');
    }

    if (intent.action === 'refactor') {
      practices.push('Code smell analysis');
      practices.push('Refactoring patterns identified');
      practices.push('Impact analysis of changes');
    }

    if (intent.action === 'explain') {
      practices.push('Structured explanation format');
      practices.push('Higher temperature (0.7) for natural explanations');
      practices.push('System architecture context');
    }

    return practices;
  }

  private getModelConfigForIntent(intent: PromptIntent): ModelConfig {
    // Temperature settings optimized for different task types
    const temperatureMap: Record<PromptIntent['action'], number> = {
      fix: 0.3,       // Low - deterministic bug fixes
      modify: 0.3,    // Low - precise code modifications
      create: 0.5,    // Medium - balanced creativity and consistency
      refactor: 0.4,  // Low-medium - structured improvements
      explain: 0.7,   // Higher - natural, flowing explanations
      other: 0.5,     // Medium - general purpose
    };

    return {
      temperature: temperatureMap[intent.action],
      model: 'claude-sonnet-4-5-20250929',
      maxTokens: 16384, // High limit for comprehensive, unrestricted responses
    };
  }
}
