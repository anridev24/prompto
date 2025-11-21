import { getIndexStats, searchFiles, getFileSymbols, searchSemantic } from '../lib/tauri-api';
import type { IndexStats, CodeSymbol, CodeChunk } from '../types/agent';

export interface CodebaseContext {
  projectStructure: ProjectStructure;
  relatedFiles: string[];
  relatedSymbols: CodeSymbol[];
  architecturalPatterns: string[];
}

export interface ProjectStructure {
  totalFiles: number;
  languages: Record<string, number>;
  rootPath: string;
  filesByType: {
    components?: string[];
    services?: string[];
    utilities?: string[];
    types?: string[];
    tests?: string[];
    configs?: string[];
  };
}

export class CodebaseAnalyzer {
  private stats: IndexStats | null = null;

  async initialize(): Promise<void> {
    try {
      this.stats = await getIndexStats();
    } catch (error) {
      console.error('Failed to initialize codebase analyzer:', error);
    }
  }

  /**
   * Analyzes the codebase and collects comprehensive context based on user intent
   */
  async analyzeForIntent(keywords: string[], entities: string[]): Promise<CodebaseContext> {
    if (!this.stats) {
      await this.initialize();
    }

    const [projectStructure, relatedFiles, relatedSymbols, architecturalPatterns] = await Promise.all([
      this.getProjectStructure(),
      this.findRelatedFiles(keywords, entities),
      this.findRelatedSymbols(keywords, entities),
      this.detectArchitecturalPatterns(),
    ]);

    return {
      projectStructure,
      relatedFiles,
      relatedSymbols,
      architecturalPatterns,
    };
  }

  /**
   * Gets high-level project structure information
   */
  private async getProjectStructure(): Promise<ProjectStructure> {
    if (!this.stats) {
      await this.initialize();
    }

    const filesByType = await this.categorizeFiles();

    return {
      totalFiles: this.stats?.total_files || 0,
      languages: this.stats?.languages || {},
      rootPath: this.stats?.root_path || '',
      filesByType,
    };
  }

  /**
   * Categorizes files by their role in the project
   */
  private async categorizeFiles(): Promise<ProjectStructure['filesByType']> {
    try {
      const [components, services, utilities, types, tests, configs] = await Promise.all([
        searchFiles('component', 20),
        searchFiles('service', 20),
        searchFiles('util', 20),
        searchFiles('type', 20),
        searchFiles('test', 20),
        searchFiles('config', 20),
      ]);

      return {
        components: components.slice(0, 10),
        services: services.slice(0, 10),
        utilities: utilities.slice(0, 10),
        types: types.slice(0, 10),
        tests: tests.slice(0, 5),
        configs: configs.slice(0, 5),
      };
    } catch (error) {
      console.error('Failed to categorize files:', error);
      return {};
    }
  }

  /**
   * Finds files related to the user's intent keywords
   */
  private async findRelatedFiles(keywords: string[], entities: string[]): Promise<string[]> {
    const allTerms = [...keywords, ...entities];
    const uniqueFiles = new Set<string>();

    try {
      // Search for files matching each keyword
      for (const term of allTerms) {
        const files = await searchFiles(term, 10);
        files.forEach((file) => uniqueFiles.add(file));
      }

      // Limit to most relevant files
      return Array.from(uniqueFiles).slice(0, 15);
    } catch (error) {
      console.error('Failed to find related files:', error);
      return [];
    }
  }

  /**
   * Finds symbols (functions, classes) related to the intent
   */
  private async findRelatedSymbols(keywords: string[], entities: string[]): Promise<CodeSymbol[]> {
    const allTerms = [...keywords, ...entities];
    const allSymbols: CodeSymbol[] = [];

    try {
      // Get related files first
      const relatedFiles = await this.findRelatedFiles(keywords, entities);

      // Get symbols from related files
      for (const filePath of relatedFiles.slice(0, 5)) {
        try {
          const symbols = await getFileSymbols(filePath);

          // Filter symbols that match keywords
          const relevantSymbols = symbols.filter((symbol) => {
            const nameLower = symbol.name.toLowerCase();
            return allTerms.some((term) => nameLower.includes(term.toLowerCase()));
          });

          allSymbols.push(...relevantSymbols);
        } catch (error) {
          // File might not be indexed, skip
          continue;
        }
      }

      // Limit and deduplicate
      const uniqueSymbols = new Map<string, CodeSymbol>();
      for (const symbol of allSymbols) {
        const key = `${symbol.file_path}:${symbol.name}`;
        if (!uniqueSymbols.has(key)) {
          uniqueSymbols.set(key, symbol);
        }
      }

      return Array.from(uniqueSymbols.values()).slice(0, 20);
    } catch (error) {
      console.error('Failed to find related symbols:', error);
      return [];
    }
  }

  /**
   * Detects architectural patterns and conventions in the codebase
   */
  private async detectArchitecturalPatterns(): Promise<string[]> {
    const patterns: string[] = [];

    if (!this.stats) {
      return patterns;
    }

    // Detect languages
    const languages = Object.keys(this.stats.languages);
    if (languages.length > 0) {
      patterns.push(`Primary languages: ${languages.join(', ')}`);
    }

    try {
      // Detect framework patterns
      const [
        reactFiles,
        vueFiles,
        angularFiles,
        nextFiles,
        tauriFiles,
        electronFiles,
      ] = await Promise.all([
        searchFiles('react', 5),
        searchFiles('vue', 5),
        searchFiles('angular', 5),
        searchFiles('next', 5),
        searchFiles('tauri', 5),
        searchFiles('electron', 5),
      ]);

      if (reactFiles.length > 0) patterns.push('Uses React framework');
      if (vueFiles.length > 0) patterns.push('Uses Vue framework');
      if (angularFiles.length > 0) patterns.push('Uses Angular framework');
      if (nextFiles.length > 0) patterns.push('Uses Next.js framework');
      if (tauriFiles.length > 0) patterns.push('Uses Tauri for desktop app');
      if (electronFiles.length > 0) patterns.push('Uses Electron for desktop app');

      // Detect architectural patterns
      const [serviceFiles, componentFiles, storeFiles, apiFiles] = await Promise.all([
        searchFiles('service', 5),
        searchFiles('component', 5),
        searchFiles('store', 5),
        searchFiles('api', 5),
      ]);

      if (serviceFiles.length > 2) patterns.push('Service layer architecture');
      if (componentFiles.length > 5) patterns.push('Component-based architecture');
      if (storeFiles.length > 2) patterns.push('State management with stores');
      if (apiFiles.length > 2) patterns.push('API/backend integration layer');

    } catch (error) {
      console.error('Failed to detect architectural patterns:', error);
    }

    return patterns;
  }

  /**
   * Finds files that import/depend on a given file
   */
  async findDependentFiles(targetFile: string): Promise<string[]> {
    try {
      // Extract filename without extension for searching
      const fileName = targetFile.split('/').pop()?.replace(/\.[^.]+$/, '') || '';

      // Search for files that might import this file
      const potentialDependents = await searchFiles(fileName, 20);

      // Filter out the target file itself
      return potentialDependents.filter((file) => file !== targetFile);
    } catch (error) {
      console.error('Failed to find dependent files:', error);
      return [];
    }
  }

  /**
   * Uses semantic search to find conceptually related code
   */
  async findSemanticallySimilarCode(description: string, maxResults = 10): Promise<CodeChunk[]> {
    try {
      return await searchSemantic(description, maxResults);
    } catch (error) {
      console.error('Failed semantic search:', error);
      return [];
    }
  }

  /**
   * Generates a summary of the codebase context for inclusion in prompts
   */
  formatCodebaseContextSummary(context: CodebaseContext): string {
    const lines: string[] = [];

    lines.push('<codebase_info>');

    // Project structure
    lines.push('<project_structure>');
    lines.push(`Total Files: ${context.projectStructure.totalFiles}`);
    lines.push(`Languages: ${Object.entries(context.projectStructure.languages)
      .map(([lang, count]) => `${lang} (${count})`)
      .join(', ')}`);

    if (Object.keys(context.projectStructure.filesByType).length > 0) {
      lines.push('\nFile Organization:');
      for (const [type, files] of Object.entries(context.projectStructure.filesByType)) {
        if (files && files.length > 0) {
          lines.push(`  ${type}: ${files.length} files`);
        }
      }
    }
    lines.push('</project_structure>');

    // Architectural patterns
    if (context.architecturalPatterns.length > 0) {
      lines.push('\n<architectural_patterns>');
      context.architecturalPatterns.forEach((pattern) => {
        lines.push(`- ${pattern}`);
      });
      lines.push('</architectural_patterns>');
    }

    // Related files
    if (context.relatedFiles.length > 0) {
      lines.push('\n<related_files>');
      context.relatedFiles.slice(0, 10).forEach((file) => {
        lines.push(`- ${file}`);
      });
      if (context.relatedFiles.length > 10) {
        lines.push(`... and ${context.relatedFiles.length - 10} more`);
      }
      lines.push('</related_files>');
    }

    // Related symbols
    if (context.relatedSymbols.length > 0) {
      lines.push('\n<related_symbols>');
      const symbolsByType = new Map<string, CodeSymbol[]>();

      for (const symbol of context.relatedSymbols.slice(0, 15)) {
        if (!symbolsByType.has(symbol.kind)) {
          symbolsByType.set(symbol.kind, []);
        }
        symbolsByType.get(symbol.kind)!.push(symbol);
      }

      for (const [kind, symbols] of symbolsByType) {
        lines.push(`\n${kind}s:`);
        symbols.slice(0, 5).forEach((symbol) => {
          const location = `${symbol.file_path}:${symbol.start_line}`;
          lines.push(`  - ${symbol.name} (${location})`);
        });
      }
      lines.push('</related_symbols>');
    }

    lines.push('</codebase_info>');

    return lines.join('\n');
  }
}
