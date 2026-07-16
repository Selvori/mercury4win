// mercury4win-linux/src/features/entry/components/entry_list.tsx
// Entry list component with pagination, mark read/star/delete

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Loader2, CheckCheck, Star, Trash2, FileText } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { load_entries, mark_read, mark_starred, delete_entry } from "@/lib/tauri_bindings";
import type { Entry } from "@/types";

interface Props {
  feed_id: number | null;
  tag_ids?: number[];
  selected_entry_id: number | null;
  on_select_entry: (entry: Entry) => void;
}

export function EntryList({ feed_id, tag_ids, selected_entry_id, on_select_entry }: Props) {
  const { t } = useTranslation();
  const query_client = useQueryClient();

  const { data, isLoading, isFetching } = useQuery({
    queryKey: ["entries", feed_id, tag_ids],
    queryFn: () =>
      load_entries({
        feed_id: feed_id ?? undefined,
        tag_ids: tag_ids && tag_ids.length > 0 ? tag_ids : undefined,
        unread_only: false,
        limit: 100,
      }),
    enabled: feed_id !== null || (tag_ids !== undefined && tag_ids.length > 0),
    staleTime: 0,
  });

  const mark_read_mutation = useMutation({
    mutationFn: ({ ids, read }: { ids: number[]; read: boolean }) =>
      mark_read(ids, read),
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["entries"] });
      query_client.invalidateQueries({ queryKey: ["feeds"] });
    },
  });

  const star_mutation = useMutation({
    mutationFn: ({ id, starred }: { id: number; starred: boolean }) =>
      mark_starred(id, starred),
    onSuccess: () => query_client.invalidateQueries({ queryKey: ["entries"] }),
  });

  const delete_mutation = useMutation({
    mutationFn: (id: number) => delete_entry(id),
    onSuccess: () => query_client.invalidateQueries({ queryKey: ["entries"] }),
  });

  if (feed_id === null && (!tag_ids || tag_ids.length === 0)) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <p className="text-sm">{t("entry.selectFeedOrTag")}</p>
      </div>
    );
  }

  if (isLoading || isFetching) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  const show_entries = data?.entries ?? [];

  if (!show_entries.length) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <p className="text-sm">{t("entry.noEntries")}</p>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-1 border-b border-border p-2">
        <span className="flex-1 text-xs text-muted-foreground">
          {t("entry.entriesCount", { count: data?.total ?? 0 })}
        </span>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 text-xs"
          onClick={() =>
            mark_read_mutation.mutate({
              ids: show_entries.map((e) => e.id),
              read: true,
            })
          }
        >
          <CheckCheck className="mr-1 h-3.5 w-3.5" />
          {t("entry.markAllRead")}
        </Button>
      </div>

      <div className="flex-1 overflow-y-auto">
        {show_entries.map((entry) => (
          <button
            key={entry.id}
            onClick={() => on_select_entry(entry)}
            className={cn(
              "group w-full border-b border-border/50 px-3 py-2.5 text-left",
              "hover:bg-accent",
              selected_entry_id === entry.id && "bg-accent",
            )}
          >
            <div className="flex items-start gap-2">
              <div className="flex-1 min-w-0">
                <p
                  className={cn(
                    "text-sm leading-snug line-clamp-2",
                    !entry.is_read && "font-semibold text-foreground",
                    entry.is_read && "text-muted-foreground",
                  )}
                >
                  {entry.title || t("entry.untitled")}
                </p>
                <div className="mt-1 flex items-center gap-2 text-[11px] text-muted-foreground">
                  {entry.author && <span>{entry.author}</span>}
                  {entry.published_at && (
                    <span>{new Date(entry.published_at).toLocaleDateString()}</span>
                  )}
                </div>
                {entry.summary && (
                  <p className="mt-1 text-xs text-muted-foreground line-clamp-2">
                    {entry.summary}
                  </p>
                )}
              </div>

              <div className="flex shrink-0 flex-col gap-0.5 opacity-0 group-hover:opacity-100">
                <Button
                  variant="ghost"
                  size="icon"
                  className={cn("h-6 w-6", entry.is_starred && "text-yellow-500")}
                  onClick={(e) => {
                    e.stopPropagation();
                    star_mutation.mutate({ id: entry.id, starred: !entry.is_starred });
                  }}
                >
                  <Star className={cn("h-3.5 w-3.5", entry.is_starred && "fill-current")} />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6"
                  onClick={(e) => {
                    e.stopPropagation();
                    delete_mutation.mutate(entry.id);
                  }}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            </div>

            {!entry.is_read && (
              <div className="mt-1.5 flex items-center gap-1 text-[10px] text-primary">
                <FileText className="h-3 w-3" />
                {t("entry.unread")}
              </div>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
