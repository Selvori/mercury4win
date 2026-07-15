// mercury4win-linux/src/features/tags/components/tag_library.tsx
// Tag library management — edit, merge, delete tags

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Pencil, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { list_tags, rename_tag, delete_tag } from "@/lib/tauri_bindings";
import { useState } from "react";

export function TagLibrary() {
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
    <div className="p-6 max-w-2xl mx-auto">
      <div className="mb-6">
        <h2 className="text-xl font-semibold text-foreground">Tag Library</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Manage tags, merge duplicates, and clean up your tag library.
        </p>
      </div>

      <Input
        placeholder="Search tags..."
        value={search}
        onChange={(e) => set_search(e.target.value)}
        className="mb-4"
      />

      {isLoading ? (
        <p className="text-sm text-muted-foreground">Loading...</p>
      ) : !tags?.length ? (
        <div className="rounded-lg border border-border bg-card p-8 text-center">
          <p className="text-sm text-muted-foreground">No tags yet</p>
        </div>
      ) : (
        <div className="space-y-1">
          {tags.map((tag) => (
            <div
              key={tag.id}
              className={cn(
                "flex items-center gap-2 rounded-lg border border-border bg-card px-3 py-2",
                tag.is_provisional && "border-yellow-300 dark:border-yellow-700",
              )}
            >
              {editing_id === tag.id ? (
                <Input
                  value={edit_name}
                  onChange={(e) => set_edit_name(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") save_edit();
                    if (e.key === "Escape") set_editing_id(null);
                  }}
                  className="h-7 text-sm flex-1"
                  autoFocus
                />
              ) : (
                <span className="flex-1 text-sm">{tag.name}</span>
              )}
              <span className="text-xs text-muted-foreground min-w-[2rem] text-right">
                {tag.usage_count}
              </span>
              {tag.is_provisional && (
                <span className="text-[10px] text-yellow-600 dark:text-yellow-400">
                  provisional
                </span>
              )}
              {editing_id === tag.id ? (
                <Button size="sm" className="h-6 text-xs" onClick={save_edit}>
                  Save
                </Button>
              ) : (
                <>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6"
                    onClick={() => start_edit(tag)}
                  >
                    <Pencil className="h-3 w-3" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6"
                    onClick={() => delete_mutation.mutate(tag.id)}
                  >
                    <Trash2 className="h-3 w-3 text-destructive" />
                  </Button>
                </>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
