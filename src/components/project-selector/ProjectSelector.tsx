import { useAppStore } from '../../store/app-store';
import { selectDirectory } from '../../lib/tauri-api';
import { Button } from '../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/card';
import { Folder, Loader2, CheckCircle2, XCircle } from 'lucide-react';

export function ProjectSelector() {
  const { indexedPath, indexStatus, indexError, indexCodebase } = useAppStore();

  const handleSelectProject = async () => {
    const path = await selectDirectory();
    if (path) {
      await indexCodebase(path);
    }
  };

  const isIndexing = indexStatus === 'indexing';
  const isComplete = indexStatus === 'complete';
  const isError = indexStatus === 'error';

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
            disabled={isIndexing}
            className="gap-2"
          >
            {isIndexing ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                Indexing...
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
              <span className="font-medium">
                {isComplete && 'Indexed: '}
                {isError && 'Failed: '}
              </span>
              <span className="text-muted-foreground truncate">{indexedPath}</span>
            </div>
          )}
        </div>
        {indexError && (
          <div className="text-sm text-destructive bg-destructive/10 p-3 rounded-md">
            Error: {indexError}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
