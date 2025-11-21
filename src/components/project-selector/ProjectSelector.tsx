import { useAppStore } from '../../store/app-store';
import { selectDirectory } from '../../lib/tauri-api';
import { Button } from '../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/card';
import { Folder, Loader2, CheckCircle2, XCircle } from 'lucide-react';

export function ProjectSelector() {
  const { indexedPath, indexStatus, indexError, indexResult, indexCodebase } = useAppStore();

  const handleSelectProject = async () => {
    const path = await selectDirectory();
    if (path) {
      await indexCodebase(path);
    }
  };

  const isIndexing = indexStatus === 'indexing';
  const isLoadingCache = indexStatus === 'loading_cache';
  const isComplete = indexStatus === 'complete';
  const isError = indexStatus === 'error';
  const isBusy = isIndexing || isLoadingCache;

  // Format quick summary
  const formatNumber = (num: number) => num.toLocaleString();
  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    const seconds = ms / 1000;
    return seconds < 60 ? `${seconds.toFixed(1)}s` : `${Math.floor(seconds / 60)}m ${Math.floor(seconds % 60)}s`;
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Select Project</CardTitle>
        <CardDescription>
          Choose a codebase to index for prompt optimization
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center gap-4">
          <Button
            onClick={handleSelectProject}
            disabled={isBusy}
            className="gap-2"
          >
            {isBusy ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                {isLoadingCache ? 'Loading cache...' : 'Indexing...'}
              </>
            ) : (
              <>
                <Folder className="h-4 w-4" />
                Select Folder
              </>
            )}
          </Button>
          {indexedPath && (
            <div className="flex-1 flex items-center gap-2 text-sm">
              {isComplete && <CheckCircle2 className="h-4 w-4 text-green-500" />}
              {isError && <XCircle className="h-4 w-4 text-red-500" />}
              {isLoadingCache && <Loader2 className="h-4 w-4 animate-spin text-blue-500" />}
              <span className="font-medium">
                {isComplete && 'Indexed: '}
                {isError && 'Failed: '}
                {isLoadingCache && 'Loading: '}
              </span>
              <span className="text-muted-foreground truncate">{indexedPath}</span>
            </div>
          )}
        </div>
        {isComplete && indexResult && (
          <div className="text-xs text-muted-foreground flex items-center gap-3 px-1">
            <span>{formatNumber(indexResult.total_files)} files</span>
            <span>•</span>
            <span>{formatNumber(indexResult.total_symbols)} symbols</span>
            <span>•</span>
            <span>{formatDuration(indexResult.duration_ms)}</span>
          </div>
        )}
        {indexError && (
          <div className="text-sm text-destructive bg-destructive/10 p-3 rounded-md">
            Error: {indexError}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
