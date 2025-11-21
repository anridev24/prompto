import { useAppStore } from '../../store/app-store';
import { Button } from '../ui/button';
import { Textarea } from '../ui/textarea';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { Sparkles, Loader2 } from 'lucide-react';

export function PromptEditor() {
  const { rawPrompt, setRawPrompt, optimizePrompt, isOptimizing, indexStatus, optimizeError } =
    useAppStore();

  const canOptimize = indexStatus === 'complete' && rawPrompt.trim().length > 0 && !isOptimizing;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Your Prompt</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <Textarea
          placeholder="Enter your development task or question here... (e.g., 'add user authentication with JWT tokens')"
          value={rawPrompt}
          onChange={(e) => setRawPrompt(e.target.value)}
          className="min-h-[200px] font-mono text-sm"
        />
        {optimizeError && (
          <div className="text-sm text-destructive bg-destructive/10 p-3 rounded-md">
            {optimizeError}
          </div>
        )}
        <Button
          onClick={optimizePrompt}
          disabled={!canOptimize}
          className="w-full gap-2"
          size="lg"
        >
          {isOptimizing ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Optimizing Prompt...
            </>
          ) : (
            <>
              <Sparkles className="h-4 w-4" />
              Optimize Prompt with AI
            </>
          )}
        </Button>
        {indexStatus !== 'complete' && (
          <p className="text-sm text-muted-foreground text-center">
            Please index a codebase first
          </p>
        )}
      </CardContent>
    </Card>
  );
}
