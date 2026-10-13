// mercury4win/src/features/entry/components/entry_list.tsx
// Entry list component with pagination, mark read/star/delete

import { useState, useRef, useLayoutEffect } from "react";
import { DigestDialog } from "@/features/reader/components/digest_dialog";
import { useDigestSelectionStore } from "@/stores/digest_selection_store";
import { useKeyboardShortcuts } from "@/hooks/use_keyboard_shortcuts";
import { Input } from "@/components/ui/input";
import { useInfiniteQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Loader2, CheckCheck, Star, Trash2, FileText } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { load_entries, mark_read, mark_all_read_in_scope, mark_starred, delete_entry, sync_feed, sync_all_feeds } from "@/lib/tauri_bindings";
import type { Entry, EntryFilter, EntryCursor, TagMatchMode } from "@/types";

interface Props {
  feed_id: number | null | undefined;
  tag_ids?: number[];
  starred_only?: boolean;
  on_clear_selection?: () => void;
  on_delete_entry?: (id: number) => void;
  selected_entry_id: number | null;
  on_select_entry: (entry: Entry) => void;
}

export function EntryList({ feed_id, tag_ids, starred_only = false, selected_entry_id, on_select_entry, on_clear_selection, on_delete_entry }: Props) {
  const { t, i18n } = useTranslation();
  const query_client = useQueryClient();
  const [search, set_search] = useState("");
  const [unread_only, set_unread_only] = useState(false);
  const [tag_mode, set_tag_mode] = useState<TagMatchMode>("any");
  const filter: EntryFilter = {
    feed_id: feed_id ?? undefined, tag_ids, starred_only, unread_only,
    search: search.trim() || undefined, tag_mode,
  };
  const scope_key = JSON.stringify(filter);
  const [digest_selection, set_digest_selection] = useState<{ scope: string; entries: Entry[]; ids: Set<number> } | null>(null);
  const [digest_ids, set_digest_ids] = useState<number[] | null>(null);
  const selecting = digest_selection?.scope === scope_key;
  const set_selecting = useDigestSelectionStore((s) => s.set_selecting);
  useLayoutEffect(() => {
    set_selecting(selecting);
    return () => set_selecting(false);
  }, [selecting, set_selecting]);
  useLayoutEffect(() => { set_digest_selection(null); set_digest_ids(null); }, [scope_key]);
  const active_scope = useRef(scope_key);
  active_scope.current = scope_key;
  const current_selection = useRef(selected_entry_id);
  current_selection.current = selected_entry_id;
  const navigation_generation = useRef(0);
  function choose_entry(entry: Entry) {
    if (selecting) {
      set_digest_selection((previous) => {
        if (!previous) return previous;
        const ids = new Set(previous.ids);
        if (ids.has(entry.id)) ids.delete(entry.id); else ids.add(entry.id);
        return { ...previous, ids };
      });
      return;
    }
    navigation_generation.current++; on_select_entry(entry);
  }
  const enabled = feed_id !== undefined || !!tag_ids?.length || starred_only;
  function change_filter(action: () => void) { action(); on_clear_selection?.(); }
  function invalidate_counts() {
    void query_client.invalidateQueries({ queryKey: ["entries"] });
    void query_client.invalidateQueries({ queryKey: ["feeds"] });
    void query_client.invalidateQueries({ queryKey: ["tags"] });
  }


  const {
    data, isLoading, isError, isFetchNextPageError, hasNextPage,
    isFetching, isFetchingNextPage, fetchNextPage, refetch,
  } = useInfiniteQuery({
    queryKey: ["entries", filter],
    initialPageParam: undefined as EntryCursor | undefined,
    queryFn: ({ pageParam }) =>
      load_entries({
        ...filter,
        limit: 100,
        cursor: pageParam,
      }),
    getNextPageParam: (last_page) => last_page.next_cursor ?? undefined,
    enabled,
    staleTime: 0,
    gcTime: 0,
  });

  const mark_read_mutation = useMutation({
    mutationFn: ({ ids, read }: { ids: number[]; read: boolean }) =>
      mark_read(ids, read),
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["entries"] });
      query_client.invalidateQueries({ queryKey: ["feeds"] });
    },
  });

  const mark_all_read_mutation = useMutation({
    mutationFn: () => mark_all_read_in_scope(filter),
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
    onSuccess: (_, id) => {
      invalidate_counts();
      on_delete_entry?.(id);
      query_client.removeQueries({ queryKey: ["entry_detail", id] });
      query_client.removeQueries({ queryKey: ["reader", id] });
    },
  });

  const refresh_mutation = useMutation({
    mutationFn: async () => {
      if (typeof feed_id === "number") await sync_feed(feed_id);
      else await sync_all_feeds();
    },
    onSuccess: invalidate_counts,
  });
  const show_entries = selecting ? digest_selection.entries : data?.pages.flatMap((page) => page.entries) ?? [];
  const selected = show_entries.find((entry) => entry.id === selected_entry_id);
  function navigate(direction: number) {
    const generation = ++navigation_generation.current;
    const selection = selected_entry_id;
    const index = show_entries.findIndex((entry) => entry.id === selected_entry_id);
    const next = index < 0 ? (direction > 0 ? 0 : show_entries.length - 1) : index + direction;
    if (next >= 0 && next < show_entries.length) on_select_entry(show_entries[next]);
    else if (direction > 0 && hasNextPage && !isFetching) {
      const scope = scope_key;
      const count = show_entries.length;
      void fetchNextPage().then((result) => {
        if (active_scope.current !== scope || result.isError || generation !== navigation_generation.current || current_selection.current !== selection) return;
        const entry = result.data?.pages.flatMap((page) => page.entries)[count];
        if (entry) on_select_entry(entry);
      });
    }
  }
  useKeyboardShortcuts(enabled && !selecting ? {
    on_down: () => navigate(1), on_up: () => navigate(-1),
    on_open: () => { if (selected) on_select_entry(selected); },
    on_toggle_read: () => { if (selected && !mark_read_mutation.isPending) mark_read_mutation.mutate({ ids: [selected.id], read: !selected.is_read }); },
    on_toggle_star: () => { if (selected && !star_mutation.isPending) star_mutation.mutate({ id: selected.id, starred: !selected.is_starred }); },
    on_mark_all_read: () => { if (!mark_all_read_mutation.isPending) mark_all_read_mutation.mutate(); },
    on_refresh: () => { if (!refresh_mutation.isPending) refresh_mutation.mutate(); },
  } : {});

  if (!enabled) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <p className="text-sm">{t("entry.selectFeedOrTag")}</p>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col" data-digest-selecting={selecting}>
      {digest_ids && <DigestDialog key={JSON.stringify(digest_ids)} entry_ids={digest_ids} multiple on_close={() => set_digest_ids(null)} />}
      <div className="space-y-2 border-b border-border p-2">
        <Input type="search" aria-label={t("entry.search")} placeholder={t("entry.search")} value={search}
          onChange={(e) => change_filter(() => set_search(e.target.value))} />
        <div className="flex flex-wrap items-center gap-1">
          <Button size="sm" variant={unread_only ? "secondary" : "ghost"} aria-pressed={unread_only}
            onClick={() => change_filter(() => set_unread_only(!unread_only))}>{t("entry.unreadOnly")}</Button>
          {!!tag_ids?.length && <Button size="sm" variant="outline" aria-label={t("entry.tagMode")}
            onClick={() => change_filter(() => set_tag_mode(tag_mode === "any" ? "all" : "any"))}>
            {t(tag_mode === "any" ? "entry.tagAny" : "entry.tagAll")}
          </Button>}
          <Button size="sm" variant="ghost" disabled={selecting || refresh_mutation.isPending} onClick={() => refresh_mutation.mutate()}>{t("entry.refresh")}</Button>
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-1 border-b border-border p-2">
        <span className="flex-1 text-xs text-muted-foreground">
          {t("entry.entriesCount", { count: data?.pages[0]?.total ?? 0 })}
        </span>
        {selecting ? <>
          <span className="text-xs">{t("digest.selected", { count: digest_selection.ids.size })}</span>
          <Button size="sm" variant="outline" onClick={() => set_digest_selection(null)}>{t("digest.cancelSelection")}</Button>
          <Button size="sm" disabled={!digest_selection.ids.size} onClick={() => {
            set_digest_ids(digest_selection.entries.filter((entry) => digest_selection.ids.has(entry.id)).map((entry) => entry.id));
            set_digest_selection(null);
          }}>{t("digest.preview")}</Button>
        </> : <Button size="sm" variant="outline" disabled={isFetching || !show_entries.length} onClick={() => {
          navigation_generation.current++;
          set_digest_selection({ scope: scope_key, entries: [...show_entries], ids: new Set() });
        }}>{t("digest.multiple")}</Button>}
        <Button
          variant="ghost"
          size="sm"
          className="h-7 text-xs"
          onClick={() => mark_all_read_mutation.mutate()}
          disabled={selecting || mark_all_read_mutation.isPending || isLoading || !show_entries.length}
        >
          <CheckCheck className="mr-1 h-3.5 w-3.5" />
          {t("entry.markAllRead")}
        </Button>
      </div>
      {selecting && <p className="px-3 py-2 text-xs text-muted-foreground">{t("digest.selectionHint")}</p>}

      {(mark_read_mutation.isError || mark_all_read_mutation.isError || star_mutation.isError || delete_mutation.isError || refresh_mutation.isError) &&
        <p role="alert" className="px-3 py-2 text-sm text-destructive">{t("entry.actionFailed")}</p>}
      <div className="flex-1 overflow-y-auto">
        {isLoading && <Loader2 className="mx-auto my-4 h-5 w-5 animate-spin" />}
        {!isLoading && !isError && !show_entries.length && <p className="p-4 text-sm text-muted-foreground">{t("entry.noEntries")}</p>}
        {show_entries.map((entry) => (
          <div
            key={entry.id}
            role="button"
            aria-label={entry.title || t("entry.untitled")}
            aria-pressed={selected_entry_id === entry.id}
            tabIndex={0}
            onClick={() => choose_entry(entry)}
            onKeyDown={(e) => {
              if (e.nativeEvent.isComposing || e.nativeEvent.keyCode === 229 || e.repeat) return;
              if (e.target === e.currentTarget && (e.key === "Enter" || e.key === " ")) {
                e.preventDefault();
                choose_entry(entry);
              }
            }}
            className={cn(
              "group w-full border-b border-border/50 px-3 py-2.5 text-left",
              "hover:bg-accent",
              selected_entry_id === entry.id && "bg-accent",
            )}
          >
            <div className="flex items-start gap-2">
              {selecting && <input type="checkbox" aria-label={t("digest.selectEntry", { title: entry.title || t("entry.untitled") })}
                checked={digest_selection.ids.has(entry.id)} onClick={(e) => e.stopPropagation()} onChange={() => choose_entry(entry)} />}
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
                    <span>{new Date(entry.published_at).toLocaleDateString(i18n.language)}</span>
                  )}
                </div>
                {entry.summary && (
                  <p className="mt-1 text-xs text-muted-foreground line-clamp-2">
                    {entry.summary}
                  </p>
                )}
              </div>

              <div className="flex shrink-0 flex-col gap-0.5 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100">
                <Button
                  variant="ghost"
                  size="icon"
                  className={cn("h-6 w-6", entry.is_starred && "text-yellow-500")}
                  aria-label={t(entry.is_starred ? "entry.unstar" : "entry.star")}
                  disabled={selecting || star_mutation.isPending}
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
                  aria-label={t("entry.delete")}
                  disabled={selecting || delete_mutation.isPending}
                  onClick={(e) => {
                    e.stopPropagation();
                    delete_mutation.mutate(entry.id);
                  }}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            </div>

            {(
              <Button
                variant="ghost"
                size="sm"
                className="mt-1.5 h-5 gap-1 px-1 text-[10px] text-primary hover:text-primary"
                onClick={(e) => {
                  e.stopPropagation();
                  mark_read_mutation.mutate({ ids: [entry.id], read: !entry.is_read });
                }}
                disabled={selecting || mark_read_mutation.isPending}
                title={t(entry.is_read ? "entry.markUnread" : "entry.markRead")}
                aria-label={t(entry.is_read ? "entry.markUnread" : "entry.markRead")}
              >
                <FileText className="h-3 w-3" />
                {t(entry.is_read ? "entry.markUnread" : "entry.unread")}
              </Button>
            )}
          </div>
        ))}
        {isError && (
          <p role="alert" className="px-3 py-2 text-sm text-destructive">{t("entry.loadFailed")}</p>
        )}
        {!selecting && (hasNextPage || isError) && (
          <div className="flex justify-center p-3">
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                if (isError && !isFetchNextPageError) void refetch();
                else void fetchNextPage();
              }}
              disabled={isFetching}
            >
              {isFetchingNextPage && <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" />}
              {t(isFetchingNextPage ? "entry.loadingMore" : isError ? "entry.retry" : "entry.loadMore")}
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
