// mercury4win/src/features/reader/components/reader_tagging.tsx
// Tagging panel — AI tag suggestions + manual tag input

import { useState, useEffect } from "react";
import { Channel } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { useMutation, useQueryClient, useQuery } from "@tanstack/react-query";
import { Tag, Sparkles, Loader2, Plus, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { run_tagging, add_tag, remove_tag, load_entry_detail, local_tag_suggestions, list_tags } from "@/lib/tauri_bindings";
import { invalidate_tag_queries } from "@/features/tags/lib/tag_queries";
import { cn } from "@/lib/utils";
import { useReaderRequest } from "../hooks/use_reader_request";
import { useAgentRequest } from "../hooks/use_agent_request";

interface Props {
  entry_id: number;
}

export function ReaderTagging({ entry_id }: Props) {
  const { t } = useTranslation();
  const query_client = useQueryClient();
  const [suggestions, set_suggestions] = useState<string[]>([]);
  const [applied_tags, set_applied_tags] = useState<Set<string>>(new Set());
  const [manual_tag, set_manual_tag] = useState("");
  const [error, set_error] = useState<string | null>(null);
  const agent = useAgentRequest(entry_id, "tagging");
  const { begin_request: begin_suggestion } = agent;
  const begin_tag_change = useReaderRequest(entry_id);
  const { data: local } = useQuery({ queryKey: ["local_tag_suggestions", entry_id], queryFn: () => local_tag_suggestions(entry_id), retry: false });
  const { data: existing } = useQuery({ queryKey: ["tags", manual_tag], queryFn: () => list_tags(manual_tag || undefined), retry: false });
  const visible_suggestions = suggestions.length ? suggestions : local ?? [];

  // Load existing tags for this entry
  const { data: entry_detail } = useQuery({
    queryKey: ["entry_detail", entry_id],
    queryFn: () => load_entry_detail(entry_id),
  });

  // Sync applied_tags from backend when entry detail changes
  // (e.g., after adding/removing a tag, or switching entries)
  useEffect(() => {
    if (entry_detail?.tags) {
      set_applied_tags(new Set(entry_detail.tags.map((t) => t.name)));
    } else {
      set_applied_tags(new Set());
    }
  }, [entry_detail]);

  const suggest_mutation = useMutation({
    mutationFn: (request: ReturnType<typeof begin_suggestion>) => {
      const channel = new Channel<string>();
      channel.onmessage = (event) => {
        if (!request.is_current()) return;
        try { const message = JSON.parse(event); if (!request.handle_event(message) && message.type === "notice") set_error(message.message); } catch { /* Ignore invalid lifecycle messages. */ }
      };
      return run_tagging(request.entry_id, channel, request.request_id);
    },
    onSuccess: (tags, request) => {
      if (!request.is_current()) return;
      if (!tags || tags.length === 0) {
        set_error(t("tagging.noAiSuggestions"));
      } else {
        set_suggestions(tags);
        set_error(null);
      }
      request.finish();
    },
    onError: (e, request) => {
      if (!request.is_current()) return;
      const msg = String(e);
      // Clean up common error prefixes for readability
      const cancelled = /^(?:Error: )?Agent task cancelled:/.test(msg);
      if (!cancelled) set_error(msg.replace(/^API error:\s*/, "").replace(/^HTTP error:\s*/, ""));
      request.finish(cancelled);
    },
  });

  const add_tag_mutation = useMutation({
    mutationFn: (snapshot: { name: string; request: ReturnType<typeof begin_tag_change> }) =>
      add_tag(snapshot.request.entry_id, snapshot.name),
    onSuccess: (_tag, { name, request }) => {
      // Remove the added tag from suggestions (don't clear all)
      if (request.is_current()) set_suggestions((prev) => prev.filter((t) => t !== name));
      invalidate_tag_queries(query_client, request.entry_id);
    },
    onError: (e, { request }) => { if (request.is_current()) set_error(String(e)); },
  });

  const remove_tag_mutation = useMutation({
    mutationFn: (snapshot: { entry_id: number; tag_id: number }) =>
      remove_tag(snapshot.entry_id, snapshot.tag_id),
    onSuccess: (_result, snapshot) => {
      invalidate_tag_queries(query_client, snapshot.entry_id);
    },
    onError: (e) => set_error(String(e)),
  });

  function handle_remove_from_applied(tag_name: string) {
    const tag_info = entry_detail?.tags.find((t) => t.name === tag_name);
    if (tag_info) {
      remove_tag_mutation.mutate({ entry_id, tag_id: tag_info.tag_id });
    }
  }

  function handle_add_manual() {
    const name = manual_tag.trim();
    if (!name) return;
    add_tag_mutation.mutate({ name, request: begin_tag_change() });
    set_manual_tag("");
  }

  function handle_key_down(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.nativeEvent.isComposing || e.nativeEvent.keyCode === 229 || e.repeat) return;
    if (e.key === "Enter") { e.preventDefault(); handle_add_manual(); }
  }

  return (
    <div className="flex h-full flex-col border-l border-border bg-card">
      <div className="flex items-center gap-1 border-b border-border px-3 py-1.5">
        <Tag className="h-4 w-4 text-primary" />
        <span className="flex-1 text-sm font-medium">{t("reader.tags")}</span>
        <Button
          size="sm"
          variant="ghost"
          className="h-6 text-xs"
          onClick={() => suggest_mutation.mutate(begin_suggestion())}
          disabled={agent.is_pending}
        >
          <Sparkles className="mr-1 h-3 w-3" />
          {agent.is_pending ? t("tagging.analyzing") : t("tagging.suggestTags")}
        </Button>
      </div>

      {(agent.is_pending || agent.phase === "cancelled") && (
        <div className="flex items-center gap-2 border-b border-border px-3 py-2 text-xs" role="status">
          <span className="flex-1">{t(agent.phase === "waiting" ? "agentTask.waiting" : agent.phase === "cancelling" ? "agentTask.cancelling" : agent.phase === "cancelled" ? "agentTask.cancelled" : agent.background_running ? "agentTask.background" : "agentTask.running")}</span>
          {agent.is_pending && <Button size="sm" variant="ghost" onClick={agent.cancel} disabled={agent.phase === "cancelling"}>{t("agentTask.cancel")}</Button>}
        </div>
      )}
      {agent.cancel_error && <p role="alert" className="px-3 py-2 text-sm text-destructive">{agent.cancel_error}</p>}

      <div className="flex-1 overflow-y-auto p-4">
        {/* Manual tag input */}
        <div className="mb-4 flex gap-2">
          <input
            type="text"
            value={manual_tag}
            onChange={(e) => set_manual_tag(e.target.value)}
            onKeyDown={handle_key_down}
            placeholder={t("tagging.addTag")}
            aria-label={t("tagging.addTag")}
            className="flex-1 h-8 rounded-md border border-border bg-transparent px-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
          />
          <Button
            size="sm"
            variant="outline"
            className="h-8"
            onClick={handle_add_manual}
            aria-label={t("tagging.addTag")}
            disabled={!manual_tag.trim() || add_tag_mutation.isPending}
          >
            <Plus className="h-3.5 w-3.5" />
          </Button>
        </div>

        {/* AI suggestions */}
        {agent.is_pending && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground mb-4">
            <Loader2 className="h-4 w-4 animate-spin" />
            {t(agent.phase === "waiting" ? "agentTask.waiting" : "tagging.analyzingArticle")}
          </div>
        )}

        {error && (
          <div className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 mb-4">
            <p role="alert" className="text-sm text-destructive whitespace-pre-wrap break-words">{error}</p>
          </div>
        )}

        {visible_suggestions.length > 0 && (
          (() => {
            const applied_names = new Set((entry_detail?.tags ?? []).map((t) => t.name));
            const remaining = visible_suggestions.filter((t) => !applied_names.has(t));
            if (!remaining.length) return null;
            return (
          <div className="mb-4">
            <p className="text-xs text-muted-foreground mb-2">{t("tagging.suggestedTags")}</p>
            <div className="flex flex-wrap gap-1.5">
              {remaining.map((tag) => (
                <button
                  key={tag}
                  onClick={() => add_tag_mutation.mutate({ name: tag, request: begin_tag_change() })}
                  disabled={add_tag_mutation.isPending}
                  className={cn(
                    "inline-flex items-center gap-1 rounded-full border border-border px-2.5 py-1 text-xs",
                    "hover:bg-primary/10 hover:border-primary/30 transition-colors",
                    "disabled:opacity-50",
                  )}
                >
                  <Plus className="h-3 w-3" />
                  {tag}
                </button>
              ))}
            </div>
          </div>
            );
          })()
        )}

        {(existing ?? []).some((tag) => !tag.is_provisional && !applied_tags.has(tag.name)) && <div className="mb-4">
          <p className="mb-2 text-xs text-muted-foreground">{t("tagging.existing")}</p>
          <div className="flex flex-wrap gap-1.5">{(existing ?? []).filter((tag) => !tag.is_provisional && !applied_tags.has(tag.name) && !visible_suggestions.includes(tag.name)).slice(0, 10).map((tag) => <Button key={tag.id} size="sm" variant="outline" disabled={add_tag_mutation.isPending} onClick={() => add_tag_mutation.mutate({ name: tag.name, request: begin_tag_change() })}>{tag.name}</Button>)}</div>
        </div>}
        {/* Applied tags */}
        {applied_tags.size > 0 && (
          <div>
            <p className="text-xs text-muted-foreground mb-2">{t("tagging.applied")}</p>
            <div className="flex flex-wrap gap-1.5">
              {[...applied_tags].map((tag) => (
                <span
                  key={tag}
                  className="inline-flex items-center gap-1 rounded-full bg-primary/10 border border-primary/20 px-2.5 py-1 text-xs text-primary"
                >
                  <Tag className="h-3 w-3" />
                  {tag}
                  <button
                    aria-label={t("tagging.remove") + ": " + tag}
                    disabled={remove_tag_mutation.isPending}
                    onClick={() => handle_remove_from_applied(tag)}
                    className="ml-0.5 rounded-full p-0.5 hover:bg-destructive/20"
                  >
                    <X className="h-2.5 w-2.5" />
                  </button>
                </span>
              ))}
            </div>
          </div>
        )}

        {!visible_suggestions.length && !applied_tags.size && !agent.is_pending && (
          <div className="flex h-full flex-col items-center justify-center text-center">
            <Tag className="h-8 w-8 text-muted-foreground/50" />
            <p className="mt-2 text-sm text-muted-foreground">
              {t("tagging.noTags")}
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              {t("tagging.clickToSuggest")}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
