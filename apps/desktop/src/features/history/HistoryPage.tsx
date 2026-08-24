import { listen } from "@tauri-apps/api/event";
import { Clipboard, History as HistoryIcon, LoaderCircle, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { errorMessage, run } from "@/lib/tauri";
import type { HistoryItem, HistoryPageResult } from "@/lib/types";

function timestamp(value: string) {
  const date = new Date(value);
  return {
    date: new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(date),
    time: new Intl.DateTimeFormat(undefined, { timeStyle: "short" }).format(date),
  };
}

export function HistoryPage({ onCopy }: { onCopy: (text: string) => Promise<void> }) {
  const [items, setItems] = useState<HistoryItem[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (nextCursor?: string | null) => {
    nextCursor ? setLoadingMore(true) : setLoading(true);
    setError(null);
    try {
      const page = await run<HistoryPageResult>("history_list", { query: { limit: 30, cursor: nextCursor ?? null } });
      setItems((current) => nextCursor ? [...current, ...page.items] : page.items);
      setCursor(page.nextCursor ?? null);
    } catch (loadError) {
      setError(errorMessage(loadError));
    } finally {
      setLoading(false);
      setLoadingMore(false);
    }
  }, []);

  useEffect(() => {
    void load();
    let dispose = () => {};
    listen("history_changed", () => void load()).then((unlisten) => { dispose = unlisten; }).catch(() => {});
    return () => dispose();
  }, [load]);

  if (loading) {
    return <div className="mx-auto grid max-w-5xl gap-3 p-6 lg:p-8">{Array.from({ length: 5 }).map((_, index) => <Skeleton key={index} className="h-28 w-full" />)}</div>;
  }

  if (error && items.length === 0) {
    return (
      <div className="grid min-h-full place-items-center p-8 text-center">
        <div><p className="font-medium">History could not be loaded.</p><p className="mt-1 text-sm text-muted-foreground">{error}</p><Button className="mt-4" variant="outline" onClick={() => void load()}><RefreshCw /> Try again</Button></div>
      </div>
    );
  }

  if (items.length === 0) {
    return (
      <div className="grid min-h-full place-items-center p-8 text-center">
        <div className="max-w-sm"><div className="mx-auto mb-4 grid size-12 place-items-center rounded-xl bg-muted"><HistoryIcon className="size-5 text-muted-foreground" /></div><h2 className="font-semibold">No transcriptions yet</h2><p className="mt-1 text-sm text-muted-foreground">Completed recordings will appear here as text only.</p></div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-5xl space-y-3 p-6 lg:p-8">
      {items.map((item) => {
        const when = timestamp(item.createdAt);
        return (
          <Card key={item.id}>
            <CardContent className="grid gap-4 p-5 sm:grid-cols-[150px_minmax(0,1fr)_auto] sm:items-start">
              <div className="text-xs"><div className="font-medium text-foreground">{when.date}</div><div className="mt-1 text-muted-foreground">{when.time}</div></div>
              <p className="whitespace-pre-wrap text-sm leading-6 text-card-foreground">{item.finalText}</p>
              <Button size="icon" variant="ghost" aria-label="Copy transcription" title="Copy transcription" onClick={() => void onCopy(item.finalText)}><Clipboard /></Button>
            </CardContent>
          </Card>
        );
      })}
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {cursor ? <div className="flex justify-center pt-3"><Button variant="outline" disabled={loadingMore} onClick={() => void load(cursor)}>{loadingMore ? <LoaderCircle className="animate-spin" /> : null}Load more</Button></div> : null}
    </div>
  );
}
