// mercury4win/src/features/feed/components/feed_list.tsx
// Feed list sidebar component with add/import/export functionality

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Rss, Plus, RefreshCw, FileDown, FileUp, Trash2 } from "lucide-react";
import { Channel } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { load_feeds, sync_all_feeds_stream, delete_feed } from "@/lib/tauri_bindings";
import type { ImportProgress } from "@/types";
import { useState } from "react";

interface Props {
  selected_id: number | null | undefined;
  on_select: (id: number | null) => void;
  on_add: () => void;
  on_import: () => void;
  on_export: () => void;
}

export function FeedList({ selected_id, on_select, on_add, on_import, on_export }: Props) {
  const { t } = useTranslation();
  const query_client = useQueryClient();
  const [sync_progress, set_sync_progress] = useState<ImportProgress | null>(null);

  const { data: feeds, isLoading } = useQuery({
    queryKey: ["feeds"],
    queryFn: load_feeds,
  });

  const sync_mutation = useMutation({
    mutationFn: async () => {
      const channel = new Channel<ImportProgress>();
      channel.onmessage = (progress) => set_sync_progress(progress);
      await sync_all_feeds_stream(channel);
    },
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["feeds"] });
      query_client.invalidateQueries({ queryKey: ["entries"] });
    },
    onSettled: () => set_sync_progress(null),
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
          title={t("feed.addFeed")}
          aria-label={t("feed.addFeed")}
        >
          <Plus className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={() => sync_mutation.mutate()}
          disabled={sync_mutation.isPending}
          title={t("feed.syncAll")}
          aria-label={t("feed.syncAll")}
        >
          <RefreshCw className={cn("h-4 w-4", sync_mutation.isPending && "animate-spin")} />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={on_import}
          title={t("feed.importOpml")}
          aria-label={t("feed.importOpml")}
        >
          <FileDown className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={on_export}
          title={t("feed.exportOpml")}
          aria-label={t("feed.exportOpml")}
        >
          <FileUp className="h-4 w-4" />
        </Button>
      </div>
      {sync_mutation.isPending && sync_progress && sync_progress.total > 0 && (
        <div className="border-b border-border px-3 py-2" role="status" aria-live="polite">
          <div className="mb-1 flex items-center justify-between gap-2 text-[11px] text-muted-foreground">
            <span className="truncate">
              {sync_progress.status === "processing"
                ? (sync_progress.title || sync_progress.feed_url)
                : sync_progress.status === "error"
                  ? `${sync_progress.title || sync_progress.feed_url}: ${sync_progress.error || t("feed.syncFailed")}`
                  : t("feed.refreshing")}
            </span>
            <span className="shrink-0 tabular-nums">{sync_progress.current}/{sync_progress.total}</span>
          </div>
          <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
            <div
              className={cn("h-full rounded-full transition-all", sync_progress.status === "error" ? "bg-destructive" : "bg-primary")}
              style={{ width: `${Math.min(100, (sync_progress.current / sync_progress.total) * 100)}%` }}
            />
          </div>
        </div>
      )}
      {(sync_mutation.isError || delete_mutation.isError) && <p role="alert" className="px-3 py-2 text-xs text-destructive">{t("feed.actionFailed")} {String(sync_mutation.error ?? delete_mutation.error)}</p>}

      <div className="flex-1 overflow-y-auto p-1">
        {isLoading ? (
          <p className="px-3 py-2 text-xs text-muted-foreground">{t("common.loading")}</p>
        ) : !feeds?.length ? (
          <div className="px-3 py-8 text-center">
            <Rss className="mx-auto h-8 w-8 text-muted-foreground/50" />
            <p className="mt-2 text-sm text-muted-foreground">{t("feed.noFeeds")}</p>
            <Button variant="outline" size="sm" className="mt-3" onClick={on_add}>
              <Plus className="mr-1 h-3.5 w-3.5" />
              {t("feed.addFirstFeed")}
            </Button>
          </div>
        ) : (
          <>
            {/* All Feeds virtual entry */}
            <button
              aria-pressed={selected_id === null}
              onClick={() => on_select(null)}
              className={cn(
                "group flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-left text-sm mb-1",
                "hover:bg-accent",
                selected_id === null
                  ? "bg-accent text-accent-foreground font-medium"
                  : "text-sidebar-foreground",
              )}
            >
              <Rss className="h-3.5 w-3.5 shrink-0 text-primary" />
              <span className="flex-1 truncate">{t("feed.allFeeds")}</span>
              {feeds.reduce((sum, f) => sum + f.unread_count, 0) > 0 && (
                <span className="rounded-full bg-primary px-1.5 py-0.5 text-[10px] font-medium text-primary-foreground">
                  {feeds.reduce((sum, f) => sum + f.unread_count, 0) > 99
                    ? "99+"
                    : feeds.reduce((sum, f) => sum + f.unread_count, 0)}
                </span>
              )}
            </button>

            {/* Individual feeds */}
            {feeds.map((feed) => (
              <div
                key={feed.id}
                className={cn(
                  "group flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-left text-sm",
                  "hover:bg-accent",
                  selected_id === feed.id
                    ? "bg-accent text-accent-foreground"
                    : "text-sidebar-foreground",
                )}
              >
                <button onClick={() => on_select(feed.id)} aria-pressed={selected_id === feed.id}
                  className="flex min-w-0 flex-1 items-center gap-2 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring rounded">
                <Rss className="h-3.5 w-3.5 shrink-0 text-primary" />
                <span className="flex-1 truncate">{feed.title || feed.feed_url}</span>
                {feed.unread_count > 0 && (
                  <span className="rounded-full bg-primary px-1.5 py-0.5 text-[10px] font-medium text-primary-foreground">
                    {feed.unread_count > 99 ? "99+" : feed.unread_count}
                  </span>
                )}
                </button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-5 w-5 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 focus-visible:opacity-100"
                  aria-label={t("feed.deleteNamed", { name: feed.title || feed.feed_url })}
                  onClick={async (e) => {
                    e.stopPropagation();
                    const { ask } = await import("@tauri-apps/plugin-dialog");
                    const confirmed = await ask(t("feed.deleteConfirm"), {
                      kind: "warning",
                    });
                    if (confirmed) {
                      delete_mutation.mutate(feed.id);
                    }
                  }}
                >
                  <Trash2 className="h-3 w-3 text-destructive" />
                </Button>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}
