// mercury4win-linux/src/features/feed/components/feed_list.tsx
// Feed list sidebar component with add/import/export functionality

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Rss, Plus, RefreshCw, FileDown, FileUp, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { load_feeds, sync_all_feeds, delete_feed } from "@/lib/tauri_bindings";

interface Props {
  selected_id: number | null;
  on_select: (id: number) => void;
  on_add: () => void;
  on_import: () => void;
  on_export: () => void;
}

export function FeedList({ selected_id, on_select, on_add, on_import, on_export }: Props) {
  const query_client = useQueryClient();

  const { data: feeds, isLoading } = useQuery({
    queryKey: ["feeds"],
    queryFn: load_feeds,
  });

  const sync_mutation = useMutation({
    mutationFn: sync_all_feeds,
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["feeds"] });
      query_client.invalidateQueries({ queryKey: ["entries"] });
    },
  });

  const delete_mutation = useMutation({
    mutationFn: delete_feed,
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["feeds"] });
      query_client.invalidateQueries({ queryKey: ["entries"] });
    },
  });

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-1 p-2 border-b border-border">
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={on_add}
          title="Add feed"
        >
          <Plus className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={() => sync_mutation.mutate()}
          disabled={sync_mutation.isPending}
          title="Sync all"
        >
          <RefreshCw className={cn("h-4 w-4", sync_mutation.isPending && "animate-spin")} />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={on_import}
          title="Import OPML"
        >
          <FileDown className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={on_export}
          title="Export OPML"
        >
          <FileUp className="h-4 w-4" />
        </Button>
      </div>

      <div className="flex-1 overflow-y-auto p-1">
        {isLoading ? (
          <p className="px-3 py-2 text-xs text-muted-foreground">Loading...</p>
        ) : !feeds?.length ? (
          <div className="px-3 py-8 text-center">
            <Rss className="mx-auto h-8 w-8 text-muted-foreground/50" />
            <p className="mt-2 text-sm text-muted-foreground">No feeds yet</p>
            <Button variant="outline" size="sm" className="mt-3" onClick={on_add}>
              <Plus className="mr-1 h-3.5 w-3.5" />
              Add your first feed
            </Button>
          </div>
        ) : (
          feeds.map((feed) => (
            <button
              key={feed.id}
              onClick={() => on_select(feed.id)}
              className={cn(
                "group flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-left text-sm",
                "hover:bg-accent",
                selected_id === feed.id
                  ? "bg-accent text-accent-foreground"
                  : "text-sidebar-foreground",
              )}
            >
              <Rss className="h-3.5 w-3.5 shrink-0 text-primary" />
              <span className="flex-1 truncate">{feed.title || feed.feed_url}</span>
              {feed.unread_count > 0 && (
                <span className="rounded-full bg-primary px-1.5 py-0.5 text-[10px] font-medium text-primary-foreground">
                  {feed.unread_count > 99 ? "99+" : feed.unread_count}
                </span>
              )}
              <Button
                variant="ghost"
                size="icon"
                className="h-5 w-5 opacity-0 group-hover:opacity-100"
                onClick={(e) => {
                  e.stopPropagation();
                  if (confirm("Delete this feed and all its entries?")) {
                    delete_mutation.mutate(feed.id);
                  }
                }}
              >
                <Trash2 className="h-3 w-3 text-destructive" />
              </Button>
            </button>
          ))
        )}
      </div>
    </div>
  );
}
