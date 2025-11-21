import React from 'react';
import { useAppStore } from '../../store/app-store';
import { Button } from '../ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { Badge } from '../ui/badge';
import { Copy, Check, FileCode } from 'lucide-react';

export function OptimizedPromptViewer() {
  const { optimizedPrompt } = useAppStore();
  const [copied, setCopied] = React.useState(false);

  if (!optimizedPrompt) {
    return null;
  }

  const handleCopy = async () => {
    await navigator.clipboard.writeText(optimizedPrompt.optimized);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <Card className="border-primary/20">
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-primary" />
            Optimized Prompt
          </CardTitle>
          <Button
            variant="outline"
            size="sm"
            onClick={handleCopy}
            className="gap-2"
          >
            {copied ? (
              <>
                <Check className="h-4 w-4" />
                Copied!
              </>
            ) : (
              <>
                <Copy className="h-4 w-4" />
                Copy
              </>
            )}
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Model Configuration */}
        {optimizedPrompt.modelConfig && (
          <div className="flex flex-wrap gap-2 pb-2 border-b">
            <Badge variant="outline" className="text-xs">
              Model: {optimizedPrompt.modelConfig.model}
            </Badge>
            <Badge variant="outline" className="text-xs">
              Temperature: {optimizedPrompt.modelConfig.temperature}
            </Badge>
            <Badge variant="outline" className="text-xs">
              Max Tokens: {optimizedPrompt.modelConfig.maxTokens}
            </Badge>
          </div>
        )}

        {/* Applied Practices */}
        <div className="flex flex-wrap gap-2">
          {optimizedPrompt.appliedPractices.map((practice) => (
            <Badge key={practice} variant="secondary" className="text-xs">
              {practice}
            </Badge>
          ))}
        </div>

        {/* Optimized prompt */}
        <div className="relative">
          <pre className="bg-muted p-4 rounded-lg overflow-x-auto text-xs leading-relaxed">
            <code>{optimizedPrompt.optimized}</code>
          </pre>
        </div>

        {/* Code context summary */}
        {optimizedPrompt.codeContext.length > 0 && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <FileCode className="h-4 w-4" />
            <span>
              Included context from {optimizedPrompt.codeContext.length} code{' '}
              {optimizedPrompt.codeContext.length === 1 ? 'location' : 'locations'}
            </span>
          </div>
        )}

        {/* Context details */}
        {optimizedPrompt.codeContext.length > 0 && (
          <details className="text-sm">
            <summary className="cursor-pointer font-medium mb-2">
              View included files ({optimizedPrompt.codeContext.length})
            </summary>
            <ul className="space-y-1 text-muted-foreground pl-4">
              {optimizedPrompt.codeContext.map((ctx, idx) => (
                <li key={idx} className="text-xs">
                  {ctx.filePath} (lines {ctx.startLine}-{ctx.endLine})
                </li>
              ))}
            </ul>
          </details>
        )}
      </CardContent>
    </Card>
  );
}

// Import Sparkles from lucide-react
import { Sparkles } from 'lucide-react';
