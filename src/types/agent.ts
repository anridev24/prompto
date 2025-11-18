// Agent types
export interface OptimizedPrompt {
  original: string;
  optimized: string;
  codeContext: CodeContext[];
  appliedPractices: string[];
  timestamp: number;
}

export interface CodeContext {
  filePath: string;
  startLine: number;
  endLine: number;
  content: string;
  language: string;
  relevance: number;
}

export interface PromptIntent {
  action: 'create' | 'modify' | 'fix' | 'explain' | 'refactor' | 'other';
  keywords: string[];
  scope: 'file' | 'function' | 'class' | 'project';
  entities: string[]; // Functions, classes, files mentioned
}

// Index types (matching Rust models)
export interface CodeSymbol {
  name: string;
  kind: SymbolKind;
  file_path: string;
  start_line: number;
  end_line: number;
  signature?: string;
  doc_comment?: string;
  parent?: string;
}

export type SymbolKind =
  | 'Function'
  | 'Method'
  | 'Class'
  | 'Struct'
  | 'Interface'
  | 'Enum'
  | 'Constant'
  | 'Variable'
  | 'Import'
  | 'Export';

export interface IndexResult {
  success: boolean;
  total_files: number;
  total_symbols: number;
  languages: string[];
  duration_ms: number;
  errors: string[];
}

export interface IndexStats {
  total_files: number;
  languages: Record<string, number>;
  root_path: string;
  indexed_at: number;
}

export interface CodeChunk {
  file_path: string;
  start_line: number;
  end_line: number;
  content: string;
  language: string;
  symbols: string[];
  relevance_score: number;
}

export interface IndexQuery {
  keywords: string[];
  symbol_kinds?: SymbolKind[];
  file_patterns?: string[];
  max_results?: number;
}
