// mercury4win-linux/src/features/reader/components/reader_tagging.tsx
// Tagging panel — AI tag suggestions + manual tag input

import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQueryClient, useQuery } from "@tanstack/react-query";
import { Tag, Sparkles, Loader2, Plus, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { run_tagging, add_tag, remove_tag, load_entry_detail } from "@/lib/tauri_bindings";
import { cn } from "@/lib/utils";

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

  // Clear suggestions and error only when switching to a different entry
  useEffect(() => {
    set_suggestions([]);
    set_error(null);
  }, [entry_id]);

  const suggest_mutation = useMutation({
    mutationFn: () => run_tagging(entry_id),
    onSuccess: (tags) => {
      if (!tags || tags.length === 0) {
        set_error("AI returned no tags for this article. Try again or add tags manually.");
      } else {
        set_suggestions(tags);
        set_error(null);
      }
    },
    onError: (e) => {
      const msg = String(e);
      // Clean up common error prefixes for readability
      set_error(msg.replace(/^API error:\s*/, "").replace(/^HTTP error:\s*/, ""));
    },
  });

  const add_tag_mutation = useMutation({
    mutationFn: (name: string) => add_tag(entry_id, name),
    onSuccess: (_tag, name) => {
      // Remove the added tag from suggestions (don't clear all)
      set_suggestions((prev) => prev.filter((t) => t !== name));
      query_client.invalidateQueries({ queryKey: ["tags"] });
      query_client.invalidateQueries({ queryKey: ["entry_detail", entry_id] });
    },
  });

  const remove_tag_mutation = useMutation({
    mutationFn: (tag_id: number) => remove_tag(entry_id, tag_id),
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["entry_detail", entry_id] });
      query_client.invalidateQueries({ queryKey: ["tags"] });
    },
  });

  function handle_remove_from_applied(tag_name: string) {
    const tag_info = entry_detail?.tags.find((t) => t.name === tag_name);
    if (tag_info) {
      remove_tag_mutation.mutate(tag_info.tag_id);
    }
  }

  function handle_add_manual() {
    const name = manual_tag.trim();
    if (!name) return;
    add_tag_mutation.mutate(name);
    set_manual_tag("");
  }

  function handle_key_down(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") handle_add_manual();
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
          onClick={() => suggest_mutation.mutate()}
          disabled={suggest_mutation.isPending}
        >
          <Sparkles className="mr-1 h-3 w-3" />
          {suggest_mutation.isPending ? t("tagging.analyzing") : t("tagging.suggestTags")}
        </Button>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {/* Manual tag input */}
        <div className="mb-4 flex gap-2">
          <input
            type="text"
            value={manual_tag}
            onChange={(e) => set_manual_tag(e.target.value)}
            onKeyDown={handle_key_down}
            placeholder={t("tagging.addTag")}
            className="flex-1 h-8 rounded-md border border-border bg-transparent px-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
          />
          <Button
            size="sm"
            variant="outline"
            className="h-8"
            onClick={handle_add_manual}
            disabled={!manual_tag.trim()}
          >
            <Plus className="h-3.5 w-3.5" />
          </Button>
        </div>

        {/* AI suggestions */}
        {suggest_mutation.isPending && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground mb-4">
            <Loader2 className="h-4 w-4 animate-spin" />
            {t("tagging.analyzingArticle")}
          </div>
        )}

        {error && (
          <div className="rounded-lg border border-destructive/30 bg-destructive/5 p-3 mb-4">
            <p className="text-sm text-destructive whitespace-pre-wrap break-words">{error}</p>
          </div>
        )}

        {suggestions.length > 0 && (
          (() => {
            const applied_names = new Set((entry_detail?.tags ?? []).map((t) => t.name));
            const remaining = suggestions.filter((t) => !applied_names.has(t));
            if (!remaining.length) return null;
            return (
          <div className="mb-4">
            <p className="text-xs text-muted-foreground mb-2">{t("tagging.suggestedTags")}</p>
            <div className="flex flex-wrap gap-1.5">
              {remaining.map((tag) => (
                <button
                  key={tag}
                  onClick={() => add_tag_mutation.mutate(tag)}
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

        {!suggestions.length && !applied_tags.size && !suggest_mutation.isPending && (
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
