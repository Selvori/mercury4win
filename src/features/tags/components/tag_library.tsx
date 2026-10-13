// mercury4win/src/features/tags/components/tag_library.tsx
// Tag library management — edit, merge, delete tags

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Pencil, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { list_tags, rename_tag, delete_tag, merge_tags, list_tag_aliases, add_tag_alias, delete_tag_alias, promote_tag, cleanup_unused_tags, load_tag_batch } from "@/lib/tauri_bindings";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Dialog, DialogTitle } from "@/components/ui/dialog";
import { BatchTagging } from "./batch_tagging";
import { invalidate_tag_queries } from "../lib/tag_queries";
import type { Tag } from "@/types";

interface Props {
  on_select_tag?: (tag_id: number) => void;
  selected_tag_ids?: number[];
  on_batch_active_change?: (active: boolean) => void;
  on_tags_changed?: (removed: number[], target?: number) => void;
}

export function TagLibrary({ on_select_tag, selected_tag_ids = [], on_tags_changed, on_batch_active_change }: Props) {
  const { t } = useTranslation();
  const query_client = useQueryClient();
  const [editing_id, set_editing_id] = useState<number | null>(null);
  const [edit_name, set_edit_name] = useState("");
  const [search, set_search] = useState("");
  const [inspector, set_inspector] = useState<Tag | null>(null);
  const [alias, set_alias] = useState("");
  const [merge_target, set_merge_target] = useState("");
  const [maintenance_error, set_maintenance_error] = useState<string | null>(null);
  const [busy, set_busy] = useState(false);
  const { data: live_batch } = useQuery({ queryKey: ["tag_batch"], queryFn: () => load_tag_batch(), refetchInterval: 2000 });
  const library_locked = live_batch?.status === "running" || live_batch?.status === "ready";
  const { data: all_tags } = useQuery({ queryKey: ["tags", ""], queryFn: () => list_tags() });
  const { data: aliases } = useQuery({ queryKey: ["tag_aliases", inspector?.id], queryFn: () => list_tag_aliases(inspector!.id), enabled: !!inspector });
  async function maintain(operation: () => Promise<unknown>, removed: number[] = [], target?: number) {
    set_busy(true); set_maintenance_error(null);
    try {
      await operation();
      if (on_tags_changed) on_tags_changed(removed, target);
      else {
        const selected_removed = removed.filter((id) => selected_tag_ids.includes(id));
        for (const id of selected_removed) on_select_tag?.(id);
        if (selected_removed.length && target && !selected_tag_ids.includes(target)) on_select_tag?.(target);
      }
      invalidate_tag_queries(query_client);
      if (removed.length) set_inspector(null);
    } catch (e) { set_maintenance_error(String(e)); }
    finally { set_busy(false); }
  }

  const { data: tags, isLoading } = useQuery({
    queryKey: ["tags", search],
    queryFn: () => list_tags(search || undefined),
  });

  const rename_mutation = useMutation({
    mutationFn: ({ id, name }: { id: number; name: string }) =>
      rename_tag(id, name),
    onSuccess: () => {
      invalidate_tag_queries(query_client);
      set_editing_id(null);
    },
  });

  const delete_mutation = useMutation({
    mutationFn: (id: number) => delete_tag(id),
    onSuccess: (_result, id) => {
      if (on_tags_changed) on_tags_changed([id]);
      else if (selected_tag_ids.includes(id)) on_select_tag?.(id);
      invalidate_tag_queries(query_client);
    },
  });

  function start_edit(tag: { id: number; name: string }) {
    set_editing_id(tag.id);
    set_edit_name(tag.name);
  }

  function save_edit() {
    if (editing_id && edit_name.trim()) {
      rename_mutation.mutate({ id: editing_id, name: edit_name.trim() });
    }
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex flex-wrap gap-1 p-2">
        <BatchTagging on_active_change={on_batch_active_change} />
        <Button size="sm" variant="ghost" disabled={busy || library_locked} onClick={() => maintain(cleanup_unused_tags, (all_tags ?? []).filter((tag) => tag.usage_count === 0).map((tag) => tag.id))}>{t("tagging.cleanup")}</Button>
      </div>
      {(maintenance_error || rename_mutation.error || delete_mutation.error) && <p role="alert" className="p-2 text-xs text-destructive break-words">{maintenance_error ?? String(rename_mutation.error ?? delete_mutation.error)}</p>}
      {library_locked && <p className="p-2 text-xs">{t("tagging.libraryLocked")}</p>}
      <div className="p-2">
        {on_select_tag && <p className="mb-2 text-xs text-muted-foreground">{t("entry.tagLimit")}</p>}
        <Input
          aria-label={t("tagging.searchTags")} placeholder={t("tagging.searchTags")}
          value={search}
          onChange={(e) => set_search(e.target.value)}
        />
      </div>

      <div className="flex-1 overflow-y-auto px-2 pb-2">
        {isLoading ? (
          <p className="text-xs text-muted-foreground p-2">{t("common.loading")}</p>
        ) : !tags?.length ? (
          <div className="p-4 text-center">
            <p className="text-xs text-muted-foreground">{t("tagging.noTagsYet")}</p>
          </div>
        ) : (
          <div className="space-y-0.5">
            {tags.map((tag) => (
              <div
                key={tag.id}
                className={cn(
                  "flex items-center gap-1.5 rounded-md px-2 py-1.5 cursor-pointer transition-colors",
                  on_select_tag && "hover:bg-accent",
                  selected_tag_ids.includes(tag.id) && "bg-accent text-accent-foreground",
                  tag.is_provisional && "border border-yellow-300 dark:border-yellow-700",
                )}
                role={on_select_tag ? "checkbox" : undefined}
                aria-label={on_select_tag ? tag.name : undefined}
                aria-checked={on_select_tag ? selected_tag_ids.includes(tag.id) : undefined}
                aria-disabled={on_select_tag && !selected_tag_ids.includes(tag.id) && selected_tag_ids.length >= 5}
                tabIndex={on_select_tag ? 0 : undefined}
                onClick={() => { if (editing_id !== tag.id && (selected_tag_ids.includes(tag.id) || selected_tag_ids.length < 5)) on_select_tag?.(tag.id); }}
                onKeyDown={(e) => {
                  if (e.nativeEvent.isComposing || e.nativeEvent.keyCode === 229 || e.repeat) return;
                  if (e.target === e.currentTarget && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    if (selected_tag_ids.includes(tag.id) || selected_tag_ids.length < 5) on_select_tag?.(tag.id);
                  }
                }}
              >
                {editing_id === tag.id ? (
                  <Input
                    onClick={(e) => e.stopPropagation()}
                    aria-label={t("tagging.rename")} value={edit_name}
                    onChange={(e) => set_edit_name(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.nativeEvent.isComposing || e.nativeEvent.keyCode === 229 || e.repeat) return;
                      if (e.key === "Enter") { e.preventDefault(); save_edit(); }
                      if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); set_editing_id(null); }
                    }}
                    className="h-6 text-xs flex-1"
                    autoFocus
                  />
                ) : (
                  <span className="flex-1 text-xs truncate">{tag.name}</span>
                )}
                <span className="text-[10px] text-muted-foreground min-w-[1.5rem] text-right">
                  {tag.usage_count}
                </span>
                {tag.is_provisional && (
                  <span className="text-[9px] text-yellow-600 dark:text-yellow-400">
                    AI
                  </span>
                )}
                {editing_id === tag.id ? (
                  <Button size="sm" className="h-5 px-1.5 text-[10px]" disabled={library_locked || rename_mutation.isPending} onClick={(e) => { e.stopPropagation(); save_edit(); }}>
                    {t("note.save")}
                  </Button>
                ) : (
                  <>
                    <Button size="sm" variant="ghost" className="h-5 px-1 text-[10px]" aria-label={t("tagging.manage") + ": " + tag.name} onClick={(e) => { e.stopPropagation(); set_inspector(tag); set_alias(""); set_merge_target(""); }}>{t("tagging.manage")}</Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-5 w-5"
                      aria-label={t("tagging.rename") + ": " + tag.name}
                      disabled={library_locked}
                      onClick={(e) => {
                        e.stopPropagation();
                        start_edit(tag);
                      }}
                    >
                      <Pencil className="h-2.5 w-2.5" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-5 w-5"
                      aria-label={t("tagging.delete") + ": " + tag.name}
                      disabled={library_locked || delete_mutation.isPending}
                      onClick={(e) => {
                        e.stopPropagation();
                        delete_mutation.mutate(tag.id);
                      }}
                    >
                      <Trash2 className="h-2.5 w-2.5 text-destructive" />
                    </Button>
                  </>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
      <Dialog open={!!inspector} onOpenChange={(open) => { if (!open && !busy) set_inspector(null); }}>
        {inspector && <div className="space-y-3 overflow-y-auto p-5">
          <DialogTitle>{inspector.name}</DialogTitle>
          {maintenance_error && <p role="alert" className="break-words text-destructive">{maintenance_error}</p>}
          <p className="text-sm">{t("tagging.aliases")}</p>
          {aliases?.map((a) => <div className="flex gap-2" key={a.id}><span className="flex-1">{a.alias}</span><Button size="sm" variant="ghost" disabled={library_locked || busy} aria-label={t("tagging.removeAlias") + ": " + a.alias} onClick={() => maintain(() => delete_tag_alias(a.id))}>{t("tagging.removeAlias")}</Button></div>)}
          <div className="flex gap-2"><Input aria-label={t("tagging.newAlias")} value={alias} onChange={(e) => set_alias(e.target.value)} /><Button disabled={library_locked || busy || !alias.trim()} onClick={() => maintain(() => add_tag_alias(inspector.id, alias)).then(() => set_alias(""))}>{t("tagging.addAlias")}</Button></div>
          {inspector.is_provisional && <Button disabled={library_locked || busy} onClick={() => maintain(() => promote_tag(inspector.id)).then(() => set_inspector(null))}>{t("tagging.promote")}</Button>}
          <label className="block text-sm">{t("tagging.mergeInto")} <select aria-label={t("tagging.mergeInto")} className="w-full border rounded bg-background p-1" value={merge_target} onChange={(e) => set_merge_target(e.target.value)}><option value="">{t("tagging.chooseTarget")}</option>{all_tags?.filter((tag) => tag.id !== inspector.id).map((tag) => <option key={tag.id} value={tag.id}>{tag.name} ({tag.usage_count})</option>)}</select></label>
          {merge_target && <p className="text-sm">{t("tagging.mergePreview", { source: inspector.name, target: all_tags?.find((tag) => tag.id === Number(merge_target))?.name })}</p>}
          <Button disabled={library_locked || busy || !merge_target} onClick={() => maintain(() => merge_tags(inspector.id, Number(merge_target)), [inspector.id], Number(merge_target))}>{t("tagging.confirmMerge")}</Button>
          <Button variant="ghost" disabled={busy} onClick={() => set_inspector(null)}>{t("common.close")}</Button>
        </div>}
      </Dialog>
    </div>
  );
}
