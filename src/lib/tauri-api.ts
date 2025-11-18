import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { IndexResult, IndexStats, CodeChunk, IndexQuery, CodeSymbol } from '../types/agent';

export async function selectDirectory(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
  });

  return typeof selected === 'string' ? selected : null;
}

export async function indexCodebase(path: string): Promise<IndexResult> {
  return invoke<IndexResult>('index_codebase', { path });
}

export async function queryIndex(query: IndexQuery): Promise<CodeChunk[]> {
  return invoke<CodeChunk[]>('query_index', { query });
}

export async function getIndexStats(): Promise<IndexStats> {
  return invoke<IndexStats>('get_index_stats');
}

export async function getFileSymbols(filePath: string): Promise<CodeSymbol[]> {
  return invoke<CodeSymbol[]>('get_file_symbols', { filePath });
}
