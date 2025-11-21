import { useAppStore } from '../../store/app-store';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { Badge } from '../ui/badge';
import { FileCode, Languages, Clock } from 'lucide-react';

export function IndexStats() {
  const { indexStats, indexResult, indexStatus } = useAppStore();

  if (indexStatus !== 'complete' || !indexStats) {
    return null;
  }

  const indexedDate = new Date(indexStats.indexed_at * 1000).toLocaleString();

  // Format number with commas
  const formatNumber = (num: number) => {
    return num.toLocaleString();
  };

  // Format duration in human-readable format
  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    const seconds = ms / 1000;
    if (seconds < 60) return `${seconds.toFixed(1)}s`;
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.floor(seconds % 60);
    return `${minutes}m ${remainingSeconds}s`;
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <FileCode className="h-5 w-5" />
          Index Statistics
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <div className="text-3xl font-bold">{formatNumber(indexStats.total_files)}</div>
            <div className="text-sm text-muted-foreground">Files Indexed</div>
          </div>
          <div>
            <div className="text-3xl font-bold">
              {Object.keys(indexStats.languages).length}
            </div>
            <div className="text-sm text-muted-foreground">Languages</div>
          </div>
          {indexResult && (
            <>
              <div>
                <div className="text-3xl font-bold">{formatNumber(indexResult.total_symbols)}</div>
                <div className="text-sm text-muted-foreground">Symbols Found</div>
              </div>
              <div>
                <div className="text-3xl font-bold">{formatDuration(indexResult.duration_ms)}</div>
                <div className="text-sm text-muted-foreground">Duration</div>
              </div>
            </>
          )}
        </div>

        <div className="space-y-2">
          <div className="flex items-center gap-2 text-sm font-medium">
            <Languages className="h-4 w-4" />
            <span>Languages:</span>
          </div>
          <div className="flex flex-wrap gap-2">
            {Object.entries(indexStats.languages)
              .sort(([, a], [, b]) => b - a)
              .map(([lang, count]) => (
                <Badge key={lang} variant="outline" className="text-xs">
                  {lang}: {count}
                </Badge>
              ))}
          </div>
        </div>

        <div className="pt-2 border-t">
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Clock className="h-3 w-3" />
            <span>Indexed: {indexedDate}</span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
