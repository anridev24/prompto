import { create } from 'zustand';
import { PromptOptimizerAgent } from '../agents/prompt-optimizer';
import { indexCodebase, getIndexStats } from '../lib/tauri-api';
import type { OptimizedPrompt, IndexStats } from '../types/agent';

interface AppState {
  // Indexing state
  indexedPath: string | null;
  indexStatus: 'idle' | 'indexing' | 'complete' | 'error';
  indexStats: IndexStats | null;
  indexError: string | null;

  // Prompt state
  rawPrompt: string;
  optimizedPrompt: OptimizedPrompt | null;
  isOptimizing: boolean;
  optimizeError: string | null;

  // Agent instance
  agent: PromptOptimizerAgent | null;
  apiKey: string | null;

  // Actions
  initializeAgent: (apiKey: string) => void;
  setIndexedPath: (path: string) => void;
  indexCodebase: (path: string) => Promise<void>;
  setRawPrompt: (prompt: string) => void;
  optimizePrompt: () => Promise<void>;
  clearOptimizedPrompt: () => void;
  getIndexStats: () => Promise<void>;
  resetIndexing: () => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  // Initial state
  indexedPath: null,
  indexStatus: 'idle',
  indexStats: null,
  indexError: null,
  rawPrompt: '',
  optimizedPrompt: null,
  isOptimizing: false,
  optimizeError: null,
  agent: null,
  apiKey: null,

  // Initialize agent with API key
  initializeAgent: (apiKey: string) => {
    const agent = new PromptOptimizerAgent(apiKey);
    set({ agent, apiKey });

    // Restore indexed path if it exists
    const { indexedPath } = get();
    if (indexedPath) {
      agent.setIndexedPath(indexedPath);
    }
  },

  // Set indexed path
  setIndexedPath: (path: string) => {
    set({ indexedPath: path });
    const { agent } = get();
    if (agent) {
      agent.setIndexedPath(path);
    }
  },

  // Index codebase
  indexCodebase: async (path: string) => {
    set({ indexStatus: 'indexing', indexError: null });

    try {
      console.log('Starting indexing for:', path);
      const result = await indexCodebase(path);
      console.log('Indexing result:', result);

      if (result.success) {
        set({
          indexStatus: 'complete',
          indexedPath: path,
        });

        // Update agent
        const { agent } = get();
        if (agent) {
          agent.setIndexedPath(path);
        }

        // Fetch stats
        await get().getIndexStats();
      } else {
        set({
          indexStatus: 'error',
          indexError: result.errors.join(', ') || 'Indexing failed',
        });
      }
    } catch (error) {
      console.error('Indexing failed:', error);
      set({
        indexStatus: 'error',
        indexError: error instanceof Error ? error.message : String(error),
      });
    }
  },

  // Get index statistics
  getIndexStats: async () => {
    try {
      const stats = await getIndexStats();
      set({ indexStats: stats });
    } catch (error) {
      console.error('Failed to get index stats:', error);
    }
  },

  // Set raw prompt
  setRawPrompt: (prompt: string) => {
    set({ rawPrompt: prompt, optimizeError: null });
  },

  // Optimize prompt
  optimizePrompt: async () => {
    const { agent, rawPrompt, indexStatus } = get();

    if (!agent) {
      set({ optimizeError: 'Agent not initialized. Please set API key.' });
      return;
    }

    if (indexStatus !== 'complete') {
      set({ optimizeError: 'Please index a codebase first.' });
      return;
    }

    if (!rawPrompt.trim()) {
      set({ optimizeError: 'Please enter a prompt to optimize.' });
      return;
    }

    set({ isOptimizing: true, optimizeError: null });

    try {
      console.log('Optimizing prompt:', rawPrompt);
      const optimized = await agent.optimizePrompt(rawPrompt);
      console.log('Optimization complete');
      set({
        optimizedPrompt: optimized,
        isOptimizing: false,
      });
    } catch (error) {
      console.error('Optimization failed:', error);
      set({
        isOptimizing: false,
        optimizeError: error instanceof Error ? error.message : String(error),
      });
    }
  },

  // Clear optimized prompt
  clearOptimizedPrompt: () => {
    set({ optimizedPrompt: null, optimizeError: null });
  },

  // Reset indexing state
  resetIndexing: () => {
    set({
      indexedPath: null,
      indexStatus: 'idle',
      indexStats: null,
      indexError: null,
    });
  },
}));
