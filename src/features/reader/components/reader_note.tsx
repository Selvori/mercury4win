// mercury4win/src/features/reader/components/reader_note.tsx
// Note editor for entries — Markdown editing with save

import { useTranslation } from "react-i18next";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { FileText, Save } from "lucide-react";
import { Button } from "@/components/ui/button";
import { get_note, save_note } from "@/lib/tauri_bindings";
import { render_markdown } from "@/lib/markdown";
import { useNoteDraftStore } from "@/stores/note_draft_store";

interface Props {
  entry_id: number;
}

export function ReaderNote({ entry_id }: Props) {
  const { t } = useTranslation();
  const query_client = useQueryClient();
  const draft = useNoteDraftStore((state) => state.drafts[entry_id]);
  const set_draft = useNoteDraftStore((state) => state.set_draft);
  const discard_draft = useNoteDraftStore((state) => state.discard_draft);
  const begin_save = useNoteDraftStore((state) => state.begin_save);
  const finish_save = useNoteDraftStore((state) => state.finish_save);
  const fail_save = useNoteDraftStore((state) => state.fail_save);
  const is_saving = useNoteDraftStore((state) => state.pending_saves[entry_id] !== undefined);
  const save_error = useNoteDraftStore((state) => state.save_errors[entry_id]);
  const is_editing = draft !== undefined;

  const { data: existing_note, isLoading: is_loading_note } = useQuery({
    queryKey: ["notes", entry_id],
    queryFn: () => get_note(entry_id),
  });

  const save_mutation = useMutation({
    mutationFn: async (snapshot: NonNullable<ReturnType<typeof begin_save>>) => {
      return save_note(snapshot.entry_id, snapshot.markdown);
    },
    onSuccess: (_note, snapshot) => {
      query_client.invalidateQueries({ queryKey: ["notes", snapshot.entry_id] });
      query_client.invalidateQueries({ queryKey: ["entries"] });
      finish_save(snapshot);
    },
    onError: (error, snapshot) => fail_save(snapshot, String(error)),
  });

  function handle_save() {
    const snapshot = begin_save(entry_id);
    if (snapshot) save_mutation.mutate(snapshot);
  }

  function handle_edit() {
    // Populate draft with existing note content when entering edit mode
    set_draft(entry_id, existing_note?.markdown ?? "");
  }

  return (
    <div className="flex h-full flex-col border-l border-border bg-card">
      <div className="flex items-center gap-1 border-b border-border px-3 py-1.5">
        <FileText className="h-4 w-4 text-primary" />
        <span className="flex-1 text-sm font-medium">{t("reader.notes")}</span>
        {is_editing && (
          <Button
            size="sm"
            className="h-6 text-xs"
            onClick={handle_save}
            disabled={is_saving}
          >
            <Save className="mr-1 h-3 w-3" />
            {is_saving ? t("note.saving") : t("note.save")}
          </Button>
        )}
        <Button
          size="sm"
          variant="ghost"
          className="h-6 text-xs"
          onClick={() => {
            if (is_editing) {
              discard_draft(entry_id);
            } else {
              handle_edit();
            }
          }}
          disabled={is_loading_note}
        >
          {is_editing ? t("note.cancel") : existing_note ? t("note.edit") : t("note.addNote")}
        </Button>
      </div>

      {save_error && (
        <div role="alert" className="mx-4 mt-3 rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
          <p>{t("note.saveFailed")}</p>
          <p className="mt-1 whitespace-pre-wrap break-words text-xs">{save_error}</p>
        </div>
      )}

      <div className="flex-1 overflow-y-auto p-4">
        {is_loading_note ? (
          <div className="flex h-full items-center justify-center">
            <p className="text-sm text-muted-foreground">{t("note.loading")}</p>
          </div>
        ) : is_editing ? (
          <textarea
            value={draft}
            onChange={(e) => set_draft(entry_id, e.target.value)}
            placeholder={t("note.placeholder")}
            aria-label={t("reader.notes")}
            className="h-full w-full resize-none rounded-lg border border-border bg-transparent p-3 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
          />
        ) : existing_note ? (
          <div
            className="reader-content note-content prose prose-sm dark:prose-invert max-w-none text-sm"
            dangerouslySetInnerHTML={{
              __html: render_markdown(existing_note.markdown),
            }}
          />
        ) : (
          <div className="flex h-full flex-col items-center justify-center text-center">
            <FileText className="h-8 w-8 text-muted-foreground/50" />
            <p className="mt-2 text-sm text-muted-foreground">
              {t("note.noNotes")}
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              {t("note.clickToAdd")}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
