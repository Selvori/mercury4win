// mercury4win/src/features/entry/components/entry_list.tsx
// Entry list component with pagination, mark read/star/delete

import { useState, useRef, useLayoutEffect, useEffect } from "react";
import { DigestDialog } from "@/features/reader/components/digest_dialog";
import { AddToTopic } from "@/features/topics/topic_editor";
import { useDigestSelectionStore } from "@/stores/digest_selection_store";
import { useKeyboardShortcuts } from "@/hooks/use_keyboard_shortcuts";
import { SearchControls } from "./search_controls";
import { search_filter_error } from "../lib/search_filter";
import { useInfiniteQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Loader2, CheckCheck, Star, Trash2, FileText } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { load_entries, mark_read, mark_all_read_in_scope, mark_starred, delete_entry, sync_feed, sync_all_feeds } from "@/lib/tauri_bindings";
import type { Entry, EntryFilter, EntryCursor, SmartView, SearchPart } from "@/types";
import type { TopicSearchSession } from "@/types/topic_search";

interface Props {
  feed_id: number | null | undefined;
  search_mode?: boolean;
  on_edit_search?: () => void;
  saved_view?: SmartView | null;
  on_view_saved?: (view: SmartView) => void;
  on_view_deleted?: () => void;
  on_select_note?: (entry: Entry) => void;
  tag_ids?: number[];
  starred_only?: boolean;
  on_clear_selection?: () => void;
  on_delete_entry?: (id: number) => void;
  selected_entry_id: number | null;
  on_select_entry: (entry: Entry) => void;
}

export function EntryList({ search_mode = false, on_edit_search, saved_view, on_view_saved, on_view_deleted, on_select_note, feed_id, tag_ids, starred_only = false, selected_entry_id, on_select_entry, on_clear_selection, on_delete_entry }: Props) {
  const { t, i18n } = useTranslation();
  const query_client = useQueryClient();
  const [ai_scope, set_ai_scope] = useState<TopicSearchSession | null>(null);
  const current_scope: EntryFilter = { feed_id: feed_id ?? undefined, tag_ids, starred_only };
  const [options, set_options] = useState<EntryFilter>(() => saved_view?.filter ?? {});
  const uses_search_filter = search_mode || !!saved_view;
  const draft_filter: EntryFilter = saved_view ? options : search_mode ? { ...current_scope, ...options }
    : { ...current_scope, unread_only: options.unread_only ?? false, tag_mode: options.tag_mode ?? "any" };
  const base_key = JSON.stringify(current_scope);
  useLayoutEffect(() => { set_ai_scope(null); }, [base_key, saved_view?.id]);
  useLayoutEffect(() => { if (!uses_search_filter) set_ai_scope(null); }, [uses_search_filter]);
  useLayoutEffect(() => {
    if (!saved_view) {
      const base = JSON.parse(base_key) as EntryFilter;
      set_options((previous) => ({ ...previous, ...base, feed_id: base.feed_id, tag_ids: base.tag_ids }));
    }
  }, [base_key, saved_view]);
  const [applied_search, set_applied_search] = useState(draft_filter.search);
  useEffect(() => {
    const timer = setTimeout(() => set_applied_search(draft_filter.search), 250);
    return () => clearTimeout(timer);
  }, [draft_filter.search]);
  const filter: EntryFilter = ai_scope && uses_search_filter ? { ...ai_scope.filter, sort: draft_filter.sort ?? ai_scope.filter.sort, ai_search_id: ai_scope.id } : { ...draft_filter, search: uses_search_filter ? applied_search : undefined };
  const filtering = !ai_scope && uses_search_filter && (draft_filter.search ?? "") !== (applied_search ?? "");
  const scope_key = JSON.stringify(filter);
  const [digest_selection, set_digest_selection] = useState<{ scope: string; entries: Entry[]; ids: Set<number> } | null>(null);
  const [digest_ids, set_digest_ids] = useState<number[] | null>(null);
  const [topic_ids, set_topic_ids] = useState<number[] | null>(null);
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
  const enabled = !!saved_view || feed_id !== undefined || !!tag_ids?.length || starred_only;
  function change_filter(action: () => void) { navigation_generation.current++; action(); on_clear_selection?.(); }
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
    enabled: enabled && !search_filter_error(filter),
    staleTime: 0,
    refetchInterval: filter.recent_days ? 60_000 : false,
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
    mutationFn: (visible_scope: EntryFilter) => mark_all_read_in_scope(visible_scope),
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
  const search_info = new Map(data?.pages.flatMap((page) => page.search_info ?? []).map((info) => [info.entry_id, info]));
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
  useKeyboardShortcuts(enabled && !selecting && !filtering ? {
    on_down: () => navigate(1), on_up: () => navigate(-1),
    on_open: () => { if (selected) on_select_entry(selected); },
    on_toggle_read: () => { if (selected && !mark_read_mutation.isPending) mark_read_mutation.mutate({ ids: [selected.id], read: !selected.is_read }); },
    on_toggle_star: () => { if (selected && !star_mutation.isPending) star_mutation.mutate({ id: selected.id, starred: !selected.is_starred }); },
    on_mark_all_read: () => { if (!mark_all_read_mutation.isPending && !isLoading && !isError && show_entries.length) mark_all_read_mutation.mutate({ ...filter }); },
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
    <div className="flex h-full flex-col text-foreground" data-digest-selecting={selecting} data-entry-mode={search_mode ? "search" : saved_view ? "view" : "browse"}>
      {digest_ids && <DigestDialog key={JSON.stringify(digest_ids)} entry_ids={digest_ids} multiple on_close={() => set_digest_ids(null)} />}
      {topic_ids && <AddToTopic ids={topic_ids} on_close={() => set_topic_ids(null)} />}
      {(search_mode || saved_view) && <SearchControls compact={!search_mode} on_edit_filters={on_edit_search}
        ai_scope={ai_scope} on_ai_scope={(scope) => {
          navigation_generation.current++;
          if (!scope || !ai_scope || scope.input_key!==ai_scope.input_key) on_clear_selection?.();
          set_ai_scope(scope); set_digest_selection(null);
        }}
        filter={draft_filter} current_scope={saved_view?.filter ?? current_scope} saved_view={saved_view}
        on_change={(value) => change_filter(() => { set_options(value); set_digest_selection(null); })}
        on_saved={on_view_saved} on_deleted={on_view_deleted} />}
      {ai_scope && data?.pages.some((page) => page.ai_stale) && <p role="alert" className="shrink-0 border-b border-border p-2 text-xs text-destructive">{t("search.natural.staleResults")}</p>}
      <div className="flex shrink-0 flex-wrap items-center gap-1 border-b border-border px-2 py-1">
        {!search_mode && !saved_view && <>
          <Button size="sm" variant={filter.unread_only ? "secondary" : "ghost"} aria-pressed={filter.unread_only ?? false}
            onClick={() => change_filter(() => set_options((previous) => ({ ...previous, unread_only: !filter.unread_only })))}>{t("entry.unreadOnly")}</Button>
          {!!tag_ids?.length && <Button size="sm" variant="outline" aria-label={t("entry.tagMode")}
            onClick={() => change_filter(() => set_options((previous) => ({ ...previous, tag_mode: filter.tag_mode === "all" ? "any" : "all" })))}>{t(filter.tag_mode === "all" ? "entry.tagAll" : "entry.tagAny")}</Button>}
        </>}
        <Button size="sm" variant="ghost" disabled={selecting || refresh_mutation.isPending} onClick={() => refresh_mutation.mutate()}>{t("entry.refresh")}</Button>
        <span role="status" className="text-xs text-muted-foreground">{filtering || isFetching && !isFetchingNextPage ? t("search.searching") : ""}</span>
      </div>
      <div className="flex shrink-0 flex-wrap items-center gap-1 border-b border-border p-2">
        <span className="flex-1 text-xs text-muted-foreground">
          {t("entry.entriesCount", { count: data?.pages[0]?.total ?? 0 })}
        </span>
        {selecting ? <>
          <span className="text-xs">{t("digest.selected", { count: digest_selection.ids.size })}</span>
          <Button size="sm" variant="outline" onClick={() => set_digest_selection(null)}>{t("digest.cancelSelection")}</Button>
          <Button size="sm" variant="outline" disabled={!digest_selection.ids.size} onClick={() => {
            set_topic_ids(digest_selection.entries.filter((entry) => digest_selection.ids.has(entry.id)).map((entry) => entry.id));
            set_digest_selection(null);
          }}>{t("topics.addTo")}</Button>
          <Button size="sm" disabled={!digest_selection.ids.size} onClick={() => {
            set_digest_ids(digest_selection.entries.filter((entry) => digest_selection.ids.has(entry.id)).map((entry) => entry.id));
            set_digest_selection(null);
          }}>{t("digest.preview")}</Button>
        </> : <Button size="sm" variant="outline" disabled={filtering || isFetching || !show_entries.length} onClick={() => {
          navigation_generation.current++;
          set_digest_selection({ scope: scope_key, entries: [...show_entries], ids: new Set() });
        }}>{t("topics.selectMultiple")}</Button>}
        <Button
          variant="ghost"
          size="sm"
          className="h-7 text-xs"
          onClick={() => mark_all_read_mutation.mutate({ ...filter })}
          title={t("search.bulkHint")}
          disabled={filtering || !!search_filter_error(filter) || selecting || mark_all_read_mutation.isPending || isLoading || isError || !show_entries.length || data?.pages.some((page) => page.ai_stale)}
        >
          <CheckCheck className="mr-1 h-3.5 w-3.5" />
          {t("entry.markAllRead")}
        </Button>
      </div>
      {uses_search_filter && <p className="shrink-0 border-b border-border px-3 py-1 text-[11px] text-muted-foreground">{t("search.bulkHint")}</p>}
      {selecting && <p className="px-3 py-2 text-xs text-muted-foreground">{t("digest.selectionHint")}</p>}

      {(mark_read_mutation.isError || mark_all_read_mutation.isError || star_mutation.isError || delete_mutation.isError || refresh_mutation.isError) &&
        <p role="alert" className="px-3 py-2 text-sm text-destructive">{t("entry.actionFailed")}</p>}
      <div className="min-h-0 flex-1 overflow-y-auto" aria-busy={filtering || isFetching}>
        {isLoading && <Loader2 className="mx-auto my-4 h-5 w-5 animate-spin" />}
        {!isLoading && !isError && !show_entries.length && <p className="p-4 text-sm text-muted-foreground">{t("entry.noEntries")}</p>}
        {show_entries.map((entry) => (
          <div
            key={entry.id}
            role="button"
            aria-label={entry.title || t("entry.untitled")}
            aria-pressed={selected_entry_id === entry.id}
            tabIndex={0}
            onClick={() => { if (!filtering) choose_entry(entry); }}
            onKeyDown={(e) => {
              if (e.nativeEvent.isComposing || e.nativeEvent.keyCode === 229 || e.repeat) return;
              if (e.target === e.currentTarget && (e.key === "Enter" || e.key === " ")) {
                e.preventDefault();
                if (!filtering) choose_entry(entry);
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
                  {search_info.get(entry.id)?.hits.find((hit) => hit.field === "title")
                    ? <Highlight parts={search_info.get(entry.id)!.hits.find((hit) => hit.field === "title")!.parts} />
                    : entry.title || t("entry.untitled")}
                </p>
                <div className="mt-1 flex items-center gap-2 text-[11px] text-muted-foreground">
                  {entry.author && <span>{entry.author}</span>}
                  {entry.published_at && (
                    <span>{new Date(entry.published_at).toLocaleDateString(i18n.language)}</span>
                  )}
                </div>
                {uses_search_filter && search_info.has(entry.id) && <div className="mt-1 space-y-1 text-xs">
                  <p className="text-muted-foreground break-all">{search_info.get(entry.id)!.feed_title} · {t(search_info.get(entry.id)!.body_available ? "search.bodyAvailable" : "search.bodyMissing")}</p>
                  {search_info.get(entry.id)?.ai && <div className="space-y-1 text-muted-foreground">
                    {search_info.get(entry.id)!.ai!.basis?.startsWith("keyword_") ? <>
                      <p>{t("search.natural.keywordPending")}</p>
                      <p className="break-words">{t("search.natural.keywordReason", { terms: search_info.get(entry.id)!.ai!.reason })}</p>
                      <p className="line-clamp-3 break-words">{search_info.get(entry.id)!.ai!.quote}</p>
                    </> : <>
                      <p>{t("search.natural.reason", { reason: search_info.get(entry.id)!.ai!.reason })}</p>
                      {search_info.get(entry.id)!.ai!.basis && <p>{t(`search.natural.basis_${search_info.get(entry.id)!.ai!.basis}`, { count: search_info.get(entry.id)!.ai!.parts ?? 0 })}</p>}
                      {search_info.get(entry.id)!.ai!.limited && <p>{t("search.natural.limited")}</p>}
                      <p className="line-clamp-3 break-words">“{search_info.get(entry.id)!.ai!.quote}”</p>
                    </>}
                  </div>}
                  {search_info.get(entry.id)!.hits.filter((hit) => hit.field !== "title").map((hit) => <div key={hit.field}>
                    <span className="mr-1 rounded bg-secondary px-1 text-[11px] text-secondary-foreground">{t(`search.field_${hit.field}`)}</span>
                    <span className="break-words text-muted-foreground"><Highlight parts={hit.parts} /></span>
                    {hit.field === "note" && on_select_note && <Button size="sm" variant="outline" className="ml-1 mt-1 h-6 text-xs" disabled={selecting || filtering}
                      onClick={(e) => { e.stopPropagation(); on_select_note(entry); }}>{t("search.openNote")}</Button>}
                  </div>)}
                </div>}
                {(!filter.search?.trim() || !search_info.has(entry.id)) && entry.summary && (
                  <p className="mt-1 text-xs text-muted-foreground line-clamp-2">
                    {search_info.get(entry.id)?.preview ?? entry.summary}
                  </p>
                )}
              </div>

              <div className="flex shrink-0 flex-col gap-0.5 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100">
                <Button
                  variant="ghost"
                  size="icon"
                  className={cn("h-6 w-6", entry.is_starred && "text-yellow-500")}
                  aria-label={t(entry.is_starred ? "entry.unstar" : "entry.star")}
                  disabled={filtering || selecting || star_mutation.isPending}
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
                  disabled={filtering || selecting || delete_mutation.isPending}
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
                disabled={filtering || selecting || mark_read_mutation.isPending}
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

function Highlight({ parts }: { parts: SearchPart[] }) {
  return <>{parts.map((part, index) => part.matched
    ? <mark key={index} className="rounded-sm bg-primary/20 text-foreground">{part.text}</mark>
    : <span key={index}>{part.text}</span>)}</>;
}
