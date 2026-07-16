// mercury4win-linux/src/features/tags/components/tag_library.tsx
// Tag library management — edit, merge, delete tags

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Pencil, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { list_tags, rename_tag, delete_tag } from "@/lib/tauri_bindings";
import { useState } from "react";
import { useTranslation } from "react-i18next";

interface Props {
  on_select_tag?: (tag_id: number) => void;
  selected_tag_id?: number | null;
}

export function TagLibrary({ on_select_tag, selected_tag_id }: Props) {
  const { t } = useTranslation();
  const query_client = useQueryClient();
  const [editing_id, set_editing_id] = useState<number | null>(null);
  const [edit_name, set_edit_name] = useState("");
  const [search, set_search] = useState("");

  const { data: tags, isLoading } = useQuery({
    queryKey: ["tags", search],
    queryFn: () => list_tags(search || undefined),
  });

  const rename_mutation = useMutation({
    mutationFn: ({ id, name }: { id: number; name: string }) =>
      rename_tag(id, name),
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["tags"] });
      set_editing_id(null);
    },
  });

  const delete_mutation = useMutation({
    mutationFn: (id: number) => delete_tag(id),
    onSuccess: () => query_client.invalidateQueries({ queryKey: ["tags"] }),
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
      <div className="p-2">
        <Input
          placeholder={t("tagging.searchTags")}
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
                  selected_tag_id === tag.id && "bg-accent text-accent-foreground",
                  tag.is_provisional && "border border-yellow-300 dark:border-yellow-700",
                )}
                onClick={() => on_select_tag?.(tag.id)}
              >
                {editing_id === tag.id ? (
                  <Input
                    value={edit_name}
                    onChange={(e) => set_edit_name(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") save_edit();
                      if (e.key === "Escape") set_editing_id(null);
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
                  <Button size="sm" className="h-5 px-1.5 text-[10px]" onClick={save_edit}>
                    {t("note.save")}
                  </Button>
                ) : (
                  <>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-5 w-5"
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
    </div>
  );
}
