import React from 'react';
import { useAppStore } from './store/app-store';
import { ProjectSelector } from './components/project-selector/ProjectSelector';
import { PromptEditor } from './components/prompt-editor/PromptEditor';
import { OptimizedPromptViewer } from './components/prompt-editor/OptimizedPromptViewer';
import { IndexStats } from './components/index-viewer/IndexStats';
import { Button } from './components/ui/button';
import { Input } from './components/ui/input';
import { Label } from './components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './components/ui/card';

function App() {
  const { agent, initializeAgent } = useAppStore();
  const [apiKey, setApiKey] = React.useState('');

  const handleSetApiKey = () => {
    if (apiKey.trim()) {
      initializeAgent(apiKey);
      // Store in localStorage for development convenience
      localStorage.setItem('anthropic_api_key', apiKey);
    }
  };

  // Load API key from localStorage on mount
  React.useEffect(() => {
    const stored = localStorage.getItem('anthropic_api_key');
    if (stored) {
      setApiKey(stored);
      initializeAgent(stored);
    }
  }, [initializeAgent]);

  // API Key Setup Screen
  if (!agent) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center p-6">
        <Card className="w-full max-w-md">
          <CardHeader className="space-y-1">
            <CardTitle className="text-2xl font-bold">Welcome to prompto</CardTitle>
            <CardDescription>
              AI-powered prompt optimization for your codebase
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="api-key">Anthropic API Key</Label>
              <Input
                id="api-key"
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="sk-ant-..."
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    handleSetApiKey();
                  }
                }}
              />
              <p className="text-xs text-muted-foreground">
                Get your API key from{' '}
                <a
                  href="https://console.anthropic.com"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="underline hover:text-foreground"
                >
                  console.anthropic.com
                </a>
              </p>
            </div>
            <Button onClick={handleSetApiKey} className="w-full" disabled={!apiKey.trim()}>
              Continue
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  // Main Application
  return (
    <div className="min-h-screen bg-background">
      <div className="container mx-auto p-6 space-y-6">
        {/* Header */}
        <header className="border-b pb-4">
          <h1 className="text-4xl font-bold tracking-tight">prompto</h1>
          <p className="text-muted-foreground mt-1">
            AI-powered prompt optimization for your codebase
          </p>
        </header>

        {/* Main Content */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Left Column - Main Workflow */}
          <div className="lg:col-span-2 space-y-6">
            <ProjectSelector />
            <PromptEditor />
            <OptimizedPromptViewer />
          </div>

          {/* Right Column - Stats */}
          <div className="space-y-6">
            <IndexStats />
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
