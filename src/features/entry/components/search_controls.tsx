import { useId, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Search, SlidersHorizontal, BookmarkPlus, Pencil, Copy, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { load_feeds, list_tags, save_smart_view, delete_smart_view, rebuild_search_index } from "@/lib/tauri_bindings";
import type { EntryFilter, SmartView } from "@/types";
import { search_filter_error } from "../lib/search_filter";
import { NaturalFilter } from "./natural_filter";
import type { TopicSearchSession } from "@/types/topic_search";

interface Props {
  ai_scope?: TopicSearchSession | null;
  on_ai_scope?: (scope: TopicSearchSession | null) => void;
  compact?: boolean;
  on_edit_filters?: () => void;
  filter: EntryFilter;
  current_scope: EntryFilter;
  saved_view?: SmartView | null;
  on_change: (filter: EntryFilter) => void;
  on_saved?: (view: SmartView) => void;
  on_deleted?: () => void;
}

const select_style = "h-9 w-full min-w-0 rounded-lg border border-border bg-card px-3 text-sm text-foreground shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:border-ring";

export function SearchControls({ compact = false, on_edit_filters, filter, current_scope, saved_view, on_change, on_saved, on_deleted, ai_scope, on_ai_scope }: Props) {
  const { t } = useTranslation();
  const id = useId();
  const client = useQueryClient();
  const [advanced, set_advanced] = useState(false);
  const [natural, set_natural] = useState(false);
  const ai_active = natural || !!ai_scope;
  const [editing, set_editing] = useState(false);
  const [name, set_name] = useState("");
  const [saved_message, set_saved_message] = useState(false);
  const [rebuild_done, set_rebuild_done] = useState(false);
  const feeds = useQuery({ queryKey: ["feeds"], queryFn: load_feeds });
  const tags = useQuery({ queryKey: ["tags"], queryFn: () => list_tags() });
  const patch = (value: Partial<EntryFilter>) => { set_saved_message(false); on_change({ ...filter, ...value }); };
  const save = useMutation({
    mutationFn: ({ mode }: { mode: "new" | "rename" | "update" }) => {
      const conditions = mode === "rename" && saved_view ? saved_view.filter : filter;
      return save_smart_view(mode === "new" ? null : saved_view!.id, name, conditions)
        .then((id) => ({ id, name: name.trim(), filter: conditions, updated_at: new Date().toISOString() }));
    },
    onSuccess: (view) => {
      void client.invalidateQueries({ queryKey: ["smart_views"] });
      set_editing(false); set_saved_message(true); on_saved?.(view);
    },
  });
  const remove = useMutation({
    mutationFn: () => delete_smart_view(saved_view!.id),
    onSuccess: () => { void client.invalidateQueries({ queryKey: ["smart_views"] }); set_editing(false); on_deleted?.(); },
  });
  const rebuild = useMutation({
    mutationFn: rebuild_search_index,
    onSuccess: () => { set_rebuild_done(true); void client.invalidateQueries({ queryKey: ["entries"] }); },
  });
  const busy = save.isPending || remove.isPending;
  const active_tags = filter.tag_ids ?? [];
  const filter_error = search_filter_error(ai_active ? { ...filter, search: null } : filter);
  const extras = Boolean(filter.is_read != null || filter.unread_only || filter.starred_only || filter.has_note != null || active_tags.length || filter.feed_id != null || filter.recent_days || filter.date_from || filter.date_to || filter.exclude_title || (filter.search_field && filter.search_field !== "all"));
  return <div className="max-h-[50%] shrink-0 space-y-2 overflow-y-auto border-b border-border p-2 text-foreground">
    <div className="flex items-center gap-2 text-xs font-medium">
      <Search className="h-3.5 w-3.5 shrink-0" />
      {compact ? <span className="min-w-0 flex-1 truncate" title={saved_view?.name}>{saved_view?.name}</span>
        : <label htmlFor={`${id}-search`} className="min-w-0 flex-1 truncate" title={saved_view?.name}>{saved_view?.name ?? t("search.title")}</label>}
      {compact && on_edit_filters && <Button size="icon" variant="ghost" className="h-7 w-7 shrink-0" aria-label={t("search.editFilters")} title={t("search.editFilters")} onClick={on_edit_filters}><SlidersHorizontal className="h-4 w-4" /></Button>}
      <Button size="icon" variant="ghost" disabled={ai_active} className="h-7 w-7 shrink-0" aria-label={t("search.saveView")} title={t(ai_active ? "search.natural.cannotSave" : "search.saveView")}
        onClick={() => { set_name(saved_view?.name ?? ""); save.reset(); remove.reset(); set_editing(true); }}><BookmarkPlus className="h-4 w-4" /></Button>
    </div>
    {!compact && <>
    <Button size="sm" variant="outline" className={ai_active ? "border-primary bg-accent text-accent-foreground" : undefined} aria-expanded={natural} aria-controls={`${id}-natural`}
      onClick={() => { set_natural(!natural); if (natural) on_ai_scope?.(null); }}>{t("search.natural.title")}</Button>
    {ai_active && <div id={`${id}-natural`}><NaturalFilter filter={filter} scope={ai_scope} compact={!natural}
      on_apply={(scope) => { set_saved_message(false); on_ai_scope?.(scope); }} /></div>}
    {!ai_active && <>
    {!ai_active && <Input id={`${id}-search`} type="search" value={filter.search ?? ""} maxLength={1000}
      placeholder={t("search.placeholder")} onChange={(e) => patch({ search: e.target.value })} />}
    {filter_error && <p role="alert" className="text-xs text-destructive">{t(filter_error)}</p>}
    {saved_view && JSON.stringify(filter) !== JSON.stringify(saved_view.filter) && <p className="text-xs text-muted-foreground">{t("search.unsaved")}</p>}
    <div className="flex flex-wrap items-center gap-1">
      <Button size="sm" variant="outline" onClick={() => patch({ feed_id: undefined, tag_ids: [], starred_only: false })}>{t("search.everywhere")}</Button>
      <Button size="sm" variant="ghost" onClick={() => patch({ feed_id: current_scope.feed_id, tag_ids: current_scope.tag_ids ?? [], starred_only: current_scope.starred_only ?? false })}>{t("search.currentScope")}</Button>
      <Button size="sm" variant={extras ? "secondary" : "ghost"} aria-expanded={advanced} aria-controls={`${id}-filters`} onClick={() => set_advanced(!advanced)}>
        <SlidersHorizontal className="mr-1 h-3.5 w-3.5" />{t("search.filters")}{extras && " •"}
      </Button>
    </div>
    <div className="flex flex-wrap items-center gap-1">
      <Button size="sm" variant={filter.unread_only || filter.is_read === false ? "secondary" : "ghost"} aria-pressed={filter.unread_only || filter.is_read === false}
        onClick={() => patch({ unread_only: !(filter.unread_only || filter.is_read === false), is_read: null })}>{t("entry.unreadOnly")}</Button>
      {!!active_tags.length && <Button size="sm" variant="outline" aria-label={t("entry.tagMode")} onClick={() => patch({ tag_mode: filter.tag_mode === "all" ? "any" : "all" })}>{t(filter.tag_mode === "all" ? "entry.tagAll" : "entry.tagAny")}</Button>}
    </div>
    <div className="flex items-center gap-2 text-xs">
      <label htmlFor={`${id}-sort`} className="shrink-0">{t("search.sort")}</label>
      <select id={`${id}-sort`} className={select_style} value={filter.sort ?? "newest"} onChange={(e) => patch({ sort: e.target.value as EntryFilter["sort"] })}>
        <option value="relevance">{t("search.relevance")}</option><option value="newest">{t("search.newest")}</option><option value="oldest">{t("search.oldest")}</option>
      </select>
    </div>
    {/* The scope is visible even with the advanced form closed. */}
    <p className="text-xs leading-relaxed text-muted-foreground" data-search-scope>
      {filter.feed_id != null ? feeds.data?.find((f) => f.id === filter.feed_id)?.title ?? t("search.missingFeed", { id: filter.feed_id }) : t("search.allFeeds")}
      {active_tags.length > 0 && ` · ${t(filter.tag_mode === "all" ? "entry.tagAll" : "entry.tagAny")}: ${active_tags.map((tag_id) => tags.data?.find((tag) => tag.id === tag_id)?.name ?? `#${tag_id}`).join(", ")}`}
      {(filter.is_read != null || filter.unread_only) && ` · ${t(filter.is_read === true ? "search.read" : "search.unread")}`}
      {filter.starred_only && ` · ${t("sidebar.starred")}`}
      {filter.has_note != null && ` · ${t(filter.has_note ? "search.hasNote" : "search.noNote")}`}
      {filter.recent_days && ` · ${t("search.lastDays", { count: filter.recent_days })}`}
      {filter.date_from && ` · ≥ ${filter.date_from}`}{filter.date_to && ` · ≤ ${filter.date_to}`}
      {filter.search_field && filter.search_field !== "all" && ` · ${t(`search.field_${filter.search_field}`)}`}
      {filter.exclude_title && ` · ${t("search.excludeTitle")}`}
    </p>
    {advanced && <div id={`${id}-filters`} className="space-y-4 rounded-lg border border-border p-3 text-sm">
      {ai_active && <p className="text-xs text-muted-foreground">{t("search.natural.manualHint")}</p>}
      <div className="block space-y-1"><label htmlFor={`${id}-feed`} className="block">{t("search.source")}</label>
        <select id={`${id}-feed`} className={select_style} value={filter.feed_id ?? ""} onChange={(e) => patch({ feed_id: e.target.value ? Number(e.target.value) : undefined })}>
          <option value="">{t("search.allFeeds")}</option>
          {filter.feed_id != null && feeds.data && !feeds.data.some((f) => f.id === filter.feed_id) && <option value={filter.feed_id}>{t("search.missingFeed", { id: filter.feed_id })}</option>}
          {feeds.data?.map((feed) => <option key={feed.id} value={feed.id}>{feed.title ?? feed.feed_url}</option>)}
        </select>
      </div>
      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1"><label htmlFor={`${id}-read`} className="block">{t("search.readState")}</label><select id={`${id}-read`} className={select_style} value={filter.is_read === true ? "read" : filter.is_read === false || filter.unread_only ? "unread" : "any"}
          onChange={(e) => patch({ is_read: e.target.value === "any" ? null : e.target.value === "read", unread_only: false })}>
          <option value="any">{t("search.any")}</option><option value="unread">{t("search.unread")}</option><option value="read">{t("search.read")}</option>
        </select></div>
        <div className="space-y-1"><label htmlFor={`${id}-note`} className="block">{t("reader.notes")}</label><select id={`${id}-note`} className={select_style} value={filter.has_note == null ? "any" : filter.has_note ? "yes" : "no"} onChange={(e) => patch({ has_note: e.target.value === "any" ? null : e.target.value === "yes" })}>
          <option value="any">{t("search.any")}</option><option value="yes">{t("search.hasNote")}</option><option value="no">{t("search.noNote")}</option>
        </select></div>
      </div>
      <label className="flex items-center gap-2"><input type="checkbox" checked={filter.starred_only ?? false} onChange={(e) => patch({ starred_only: e.target.checked })} />{t("search.starredOnly")}</label>
      <fieldset className="space-y-2"><legend>{t("search.tags")}</legend>
        <select aria-label={t("entry.tagMode")} className={select_style} value={filter.tag_mode ?? "any"} onChange={(e) => patch({ tag_mode: e.target.value as "any" | "all" })}>
          <option value="any">{t("entry.tagAny")}</option><option value="all">{t("entry.tagAll")}</option>
        </select>
        <div className="flex max-h-28 flex-wrap gap-2 overflow-y-auto">
          {tags.data?.map((tag) => <label key={tag.id} className="flex min-w-0 items-center gap-1"><input type="checkbox" checked={active_tags.includes(tag.id)}
            onChange={(e) => patch({ tag_ids: e.target.checked ? [...active_tags, tag.id] : active_tags.filter((id) => id !== tag.id) })} /><span className="break-all">{tag.name}</span></label>)}
          {active_tags.filter((tag_id) => tags.data && !tags.data.some((tag) => tag.id === tag_id)).map((tag_id) => <label key={tag_id}><input type="checkbox" checked onChange={() => patch({ tag_ids: active_tags.filter((id) => id !== tag_id) })} /> #{tag_id}</label>)}
        </div>
      </fieldset>
      <div className="block space-y-1"><label htmlFor={`${id}-days`} className="block">{t("search.recentDays")}</label>
        <Input id={`${id}-days`} type="number" min={1} max={36500} value={filter.recent_days ?? ""} placeholder={t("search.any")}
          onChange={(e) => patch({ recent_days: e.target.value ? Number(e.target.value) : null })} />
      </div>
      <div className="grid grid-cols-1 gap-2">
        <div className="min-w-0 space-y-1"><label htmlFor={`${id}-from`} className="block">{t("search.dateFrom")}</label><Input id={`${id}-from`} className="min-w-0 w-full" type="date" value={filter.date_from ?? ""} onChange={(e) => patch({ date_from: e.target.value || null })} /></div>
        <div className="min-w-0 space-y-1"><label htmlFor={`${id}-to`} className="block">{t("search.dateTo")}</label><Input id={`${id}-to`} className="min-w-0 w-full" type="date" value={filter.date_to ?? ""} onChange={(e) => patch({ date_to: e.target.value || null })} /></div>
      </div>
      <p className="text-muted-foreground">{t("search.dateHint")}</p>
      <div className="block space-y-1"><label htmlFor={`${id}-field`} className="block">{t("search.fields")}</label><select id={`${id}-field`} className={select_style} value={filter.search_field ?? "all"} onChange={(e) => patch({ search_field: e.target.value as EntryFilter["search_field"] })}>
        {["all", "title", "summary", "body", "note"].map((field) => <option key={field} value={field}>{t(`search.field_${field}`)}</option>)}
      </select></div>
      <label className="flex items-center gap-2"><input type="checkbox" checked={filter.exclude_title ?? false} onChange={(e) => patch({ exclude_title: e.target.checked })} />{t("search.excludeTitle")}</label>
      <Button size="sm" variant="outline" onClick={() => on_change({ search: filter.search, sort: filter.sort })}>{t("search.clearFilters")}</Button>
      {(feeds.isError || tags.isError) && <p role="alert">{t("search.optionsFailed")}<Button size="sm" variant="ghost" onClick={() => { void feeds.refetch(); void tags.refetch(); }}>{t("entry.retry")}</Button></p>}
      <div className="space-y-1 border-t border-border pt-2">
        {!ai_active && <p className="text-muted-foreground">{t("search.coverage")}</p>}
        <Button size="sm" variant="ghost" disabled={rebuild.isPending} onClick={() => { set_rebuild_done(false); rebuild.mutate(); }}>{t(rebuild.isPending ? "search.rebuilding" : "search.rebuild")}</Button>
        {rebuild_done && <p role="status">{t("search.rebuilt")}</p>}
        {rebuild.isError && <p role="alert" className="text-destructive">{t("search.rebuildFailed")}</p>}
      </div>
    </div>}
    {!advanced && !ai_active && <p className="text-[11px] leading-relaxed text-muted-foreground">{t("search.coverageShort")}</p>}
    {saved_message && <p role="status" className="text-xs">{t("search.saved")}</p>}
    </>}
    </>}
    <Dialog open={editing} onOpenChange={(open) => { if (!busy) set_editing(open); }} aria_label={t("search.saveView")}>
      <DialogContent className="min-h-0 overflow-y-auto">
        <form aria-busy={busy} onSubmit={(e) => { e.preventDefault(); if (name.trim() && !busy && !filter_error) save.mutate({ mode: saved_view ? "update" : "new" }); }}>
          <DialogHeader>
            <DialogTitle>{t("search.saveView")}</DialogTitle>
            <p className="mt-2 text-sm leading-relaxed text-muted-foreground">{t("search.saveHint")}</p>
          </DialogHeader>
          <div className="space-y-5">
            <div className="space-y-2">
              <label htmlFor={`${id}-name`} className="text-sm font-medium">{t("search.viewName")}</label>
              <Input id={`${id}-name`} value={name} maxLength={80} required onChange={(e) => set_name(e.target.value)} />
            </div>
            {saved_view && <>
              <section aria-labelledby={`${id}-actions`} className="space-y-3 rounded-lg border border-border bg-muted/20 p-4">
                <div className="space-y-1">
                  <h3 id={`${id}-actions`} className="text-sm font-medium">{t("search.otherActions")}</h3>
                  <p className="text-xs leading-relaxed text-muted-foreground">{t("search.otherActionsHint")}</p>
                </div>
                <div className="flex flex-wrap gap-2">
                  <Button type="button" variant="outline" disabled={busy || !name.trim()} onClick={() => save.mutate({ mode: "rename" })}><Pencil className="h-4 w-4" />{t("search.renameView")}</Button>
                  <Button type="button" variant="outline" disabled={busy || !name.trim() || !!filter_error} onClick={() => save.mutate({ mode: "new" })}><Copy className="h-4 w-4" />{t("search.saveNew")}</Button>
                </div>
              </section>
              <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4">
                <p className="min-w-0 flex-1 text-xs leading-relaxed text-muted-foreground">{t("search.deleteHint")}</p>
                <Button type="button" variant="outline" className="text-destructive hover:bg-destructive/10 hover:text-destructive" disabled={busy} onClick={() => remove.mutate()}><Trash2 className="h-4 w-4" />{t("search.deleteView")}</Button>
              </div>
            </>}
            {(save.isError || remove.isError) && <p role="alert" className="text-sm text-destructive">{t("search.saveFailed")}</p>}
          </div>
          <DialogFooter className="flex-wrap border-border">
            <Button type="button" variant="outline" disabled={busy} onClick={() => set_editing(false)}>{t("note.cancel")}</Button>
            <Button type="submit" disabled={busy || !name.trim() || !!filter_error}>{t(saved_view ? "search.updateView" : "search.createView")}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  </div>;
}
