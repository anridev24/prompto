import { invoke } from '@tauri-apps/api/core';
import type { PromptIntent } from '../types/agent';

export class IntentAnalyzer {
  private apiKey: string;

  constructor(apiKey: string) {
    this.apiKey = apiKey;
  }

  async analyzeIntent(rawPrompt: string): Promise<PromptIntent> {
    try {
      // Call Tauri backend instead of direct API call
      const response = await invoke<string>('analyze_intent', {
        apiKey: this.apiKey,
        prompt: rawPrompt,
      });

      // Clean up the response to handle markdown code blocks
      let jsonText = response.trim();
      if (jsonText.startsWith('```')) {
        // Remove markdown code block markers
        jsonText = jsonText.replace(/^```(?:json)?\n?/, '').replace(/\n?```$/, '');
      }

      const intent = JSON.parse(jsonText);
      return intent;
    } catch (error) {
      console.error('Failed to parse intent:', error);
      // Fallback to basic intent extraction
      return this.extractBasicIntent(rawPrompt);
    }
  }

  private extractBasicIntent(rawPrompt: string): PromptIntent {
    const lower = rawPrompt.toLowerCase();

    // Detect action
    let action: PromptIntent['action'] = 'other';
    if (lower.includes('create') || lower.includes('add') || lower.includes('implement')) {
      action = 'create';
    } else if (lower.includes('modify') || lower.includes('update') || lower.includes('change')) {
      action = 'modify';
    } else if (lower.includes('fix') || lower.includes('bug') || lower.includes('error')) {
      action = 'fix';
    } else if (lower.includes('explain') || lower.includes('how') || lower.includes('what')) {
      action = 'explain';
    } else if (lower.includes('refactor') || lower.includes('improve')) {
      action = 'refactor';
    }

    // Extract keywords (words longer than 3 characters, excluding common words)
    const commonWords = new Set(['this', 'that', 'with', 'from', 'have', 'will', 'make', 'when', 'what', 'where', 'which', 'should', 'could', 'would']);
    const keywords = rawPrompt
      .toLowerCase()
      .split(/\s+/)
      .filter((w) => w.length > 3 && !commonWords.has(w))
      .slice(0, 5);

    return {
      action,
      keywords,
      scope: 'project',
      entities: [],
    };
  }
}
