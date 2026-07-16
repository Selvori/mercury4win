// mercury4win-linux/src/features/reader/components/reader_note.tsx
// Note editor for entries — Markdown editing with save

import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { FileText, Save } from "lucide-react";
import { Button } from "@/components/ui/button";
import { get_note, save_note } from "@/lib/tauri_bindings";
import { render_markdown } from "@/lib/markdown";

interface Props {
  entry_id: number;
}

export function ReaderNote({ entry_id }: Props) {
  const query_client = useQueryClient();
  const [draft, set_draft] = useState("");
  const [is_editing, set_is_editing] = useState(false);

  const { data: existing_note, isLoading: is_loading_note } = useQuery({
    queryKey: ["notes", entry_id],
    queryFn: () => get_note(entry_id),
  });

  const save_mutation = useMutation({
    mutationFn: async (markdown: string) => {
      return save_note(entry_id, markdown);
    },
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["notes", entry_id] });
      set_is_editing(false);
    },
  });

  function handle_save() {
    save_mutation.mutate(draft);
  }

  function handle_edit() {
    // Populate draft with existing note content when entering edit mode
    if (existing_note) {
      set_draft(existing_note.markdown);
    }
    set_is_editing(true);
  }

  return (
    <div className="flex h-full flex-col border-l border-border bg-card">
      <div className="flex items-center gap-1 border-b border-border px-3 py-1.5">
        <FileText className="h-4 w-4 text-primary" />
        <span className="flex-1 text-sm font-medium">Notes</span>
        {is_editing && (
          <Button
            size="sm"
            className="h-6 text-xs"
            onClick={handle_save}
            disabled={save_mutation.isPending}
          >
            <Save className="mr-1 h-3 w-3" />
            {save_mutation.isPending ? "Saving..." : "Save"}
          </Button>
        )}
        <Button
          size="sm"
          variant="ghost"
          className="h-6 text-xs"
          onClick={() => {
            if (is_editing) {
              set_is_editing(false);
            } else {
              handle_edit();
            }
          }}
        >
          {is_editing ? "Cancel" : existing_note ? "Edit" : "Add Note"}
        </Button>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {is_loading_note ? (
          <div className="flex h-full items-center justify-center">
            <p className="text-sm text-muted-foreground">Loading...</p>
          </div>
        ) : is_editing ? (
          <textarea
            value={draft}
            onChange={(e) => set_draft(e.target.value)}
            placeholder="Write your notes in Markdown..."
            className="h-full w-full resize-none rounded-lg border border-border bg-transparent p-3 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
          />
        ) : existing_note ? (
          <div
            className="prose prose-sm dark:prose-invert max-w-none"
            dangerouslySetInnerHTML={{
              __html: render_markdown(existing_note.markdown),
            }}
          />
        ) : (
          <div className="flex h-full flex-col items-center justify-center text-center">
            <FileText className="h-8 w-8 text-muted-foreground/50" />
            <p className="mt-2 text-sm text-muted-foreground">
              No notes for this entry
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              Click "Add Note" to write your thoughts
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
