// mercury4win-linux/src/features/feed/components/opml_import.tsx
// OPML import dialog — select file, import feeds, show results

import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { CheckCircle2, AlertTriangle } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { import_opml } from "@/lib/tauri_bindings";

interface Props {
  open: boolean;
  on_close: () => void;
}

interface ImportResult {
  feeds_added: number;
  feeds_skipped: number;
  errors: string[];
}

export function OpmlImport({ open, on_close }: Props) {
  const { t } = useTranslation();
  const query_client = useQueryClient();
  const [result, set_result] = useState<ImportResult | null>(null);
  const [file_path, set_file_path] = useState<string | null>(null);

  const mutation = useMutation({
    mutationFn: async () => {
      const { open: open_dialog } = await import("@tauri-apps/plugin-dialog");
      const selected = await open_dialog({
        title: "Select OPML File",
        filters: [{ name: "OPML", extensions: ["opml", "xml"] }],
        multiple: false,
      });
      if (!selected) return null;
      const path = selected as string;
      set_file_path(path);
      return await import_opml(path);
    },
    onSuccess: (data) => {
      if (data) {
        set_result(data);
        query_client.invalidateQueries({ queryKey: ["feeds"] });
        query_client.invalidateQueries({ queryKey: ["entries"] });
      }
    },
  });

  function handle_close() {
    set_result(null);
    set_file_path(null);
    mutation.reset();
    on_close();
  }

  return (
    <Dialog open={open} onOpenChange={(o) => !o && handle_close()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("opml.importTitle")}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {!result ? (
            <>
              <p className="text-sm text-muted-foreground">
                {t("opml.importDescription")}
              </p>
              {file_path && (
                <p className="rounded-md border border-border bg-muted/50 px-3 py-2 text-xs text-muted-foreground">
                  {t("opml.fileSelected")}: <span className="font-mono text-foreground">{file_path}</span>
                </p>
              )}
              {mutation.isError && (
                <p className="text-xs text-destructive">
                  {t("opml.importError")}: {String(mutation.error)}
                </p>
              )}
            </>
          ) : (
            <div className="space-y-3">
              <div className="flex items-center gap-2 text-sm font-medium text-green-600 dark:text-green-400">
                <CheckCircle2 className="h-4 w-4" />
                {t("opml.importSuccess", { added: result.feeds_added, skipped: result.feeds_skipped })}
              </div>
              {result.errors.length > 0 && (
                <div className="space-y-1">
                  <div className="flex items-center gap-1.5 text-xs font-medium text-amber-600 dark:text-amber-400">
                    <AlertTriangle className="h-3.5 w-3.5" />
                    Sync warnings
                  </div>
                  <ul className="max-h-32 overflow-y-auto space-y-0.5">
                    {result.errors.map((err, i) => (
                      <li key={i} className="text-xs text-muted-foreground break-all">{err}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}
        </div>

        <DialogFooter>
          {!result ? (
            <>
              <Button variant="outline" type="button" onClick={handle_close}>
                {t("feed.cancel")}
              </Button>
              <Button
                type="button"
                onClick={() => mutation.mutate()}
                disabled={mutation.isPending}
              >
                {mutation.isPending ? t("opml.importing") : t("opml.selectFile")}
              </Button>
            </>
          ) : (
            <Button type="button" onClick={handle_close}>
              Done
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
