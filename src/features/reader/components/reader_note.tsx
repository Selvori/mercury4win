// mercury4win-linux/src/features/reader/components/reader_note.tsx
// Note editor for entries — Markdown editing with save

import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { FileText, Save } from "lucide-react";
import { Button } from "@/components/ui/button";

interface Props {
  entry_id: number;
}

export function ReaderNote({ entry_id }: Props) {
  const query_client = useQueryClient();
  const [draft, set_draft] = useState("");
  const [is_editing, set_is_editing] = useState(false);

  // In Tauri mode, would use get_note/save_note
  // For now, placeholder behavior
  const save_mutation = useMutation({
    mutationFn: async (markdown: string) => {
      // import { save_note } from "@/lib/tauri_bindings";
      // return save_note(entry_id, markdown);
      await new Promise((r) => setTimeout(r, 100));
      return markdown;
    },
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["notes", entry_id] });
      set_is_editing(false);
    },
  });

  function handle_save() {
    save_mutation.mutate(draft);
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
            Save
          </Button>
        )}
        <Button
          size="sm"
          variant="ghost"
          className="h-6 text-xs"
          onClick={() => {
            set_is_editing(!is_editing);
            if (!is_editing) set_draft("");
          }}
        >
          {is_editing ? "Cancel" : "Edit"}
        </Button>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {!is_editing ? (
          <div className="flex h-full flex-col items-center justify-center text-center">
            <FileText className="h-8 w-8 text-muted-foreground/50" />
            <p className="mt-2 text-sm text-muted-foreground">
              No notes for this entry
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              Click Edit to add your thoughts
            </p>
          </div>
        ) : (
          <textarea
            value={draft}
            onChange={(e) => set_draft(e.target.value)}
            placeholder="Write your notes in Markdown..."
            className="h-full w-full resize-none rounded-lg border border-border bg-transparent p-3 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
          />
        )}
      </div>
    </div>
  );
}
