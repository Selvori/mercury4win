// mercury4win/src/features/feed/components/opml_import.tsx
// OPML import dialog — select file, import feeds with live progress

import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { CheckCircle2, XCircle, Loader2, Clock } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { import_opml_stream } from "@/lib/tauri_bindings";
import { Channel } from "@tauri-apps/api/core";
import type { ImportProgress } from "@/types";

interface Props {
  open: boolean;
  on_close: () => void;
}

interface FeedProgress {
  feed_url: string;
  title: string;
  status: "processing" | "added" | "skipped" | "error";
  error?: string;
}

export function OpmlImport({ open, on_close }: Props) {
  const { t } = useTranslation();
  const query_client = useQueryClient();
  const [importing, set_importing] = useState(false);
  const [progress_list, set_progress_list] = useState<FeedProgress[]>([]);
  const [current, set_current] = useState(0);
  const [total, set_total] = useState(0);
  const [done, set_done] = useState(false);
  const [error, set_error] = useState<string | null>(null);

  async function handle_import() {
    set_error(null);
    try {
      const { open: open_dialog } = await import("@tauri-apps/plugin-dialog");
      const selected = await open_dialog({
        title: t("opml.selectFile"),
        filters: [{ name: "OPML", extensions: ["opml", "xml"] }],
        multiple: false,
      });
      if (!selected) return;
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (!path) return;

      // Reset state
      set_importing(true);
      set_progress_list([]);
      set_current(0);
      set_total(0);
      set_done(false);

      // Create channel
      const channel = new Channel<ImportProgress>();
      channel.onmessage = (msg) => {
        if (msg.status === "start") {
          set_total(msg.total);
        } else if (msg.status === "processing") {
          set_current(msg.current);
          set_progress_list((prev) => [
            ...prev,
            {
              feed_url: msg.feed_url,
              title: msg.title || msg.feed_url,
              status: "processing",
            },
          ]);
        } else if (msg.status === "added") {
          set_progress_list((prev) =>
            prev.map((item) =>
              item.feed_url === msg.feed_url
                ? { ...item, status: "added" }
                : item
            )
          );
        } else if (msg.status === "skipped") {
          set_progress_list((prev) =>
            prev.map((item) =>
              item.feed_url === msg.feed_url
                ? { ...item, status: "skipped", error: msg.error || undefined }
                : item
            )
          );
        } else if (msg.status === "error") {
          set_progress_list((prev) =>
            prev.map((item) =>
              item.feed_url === msg.feed_url
                ? { ...item, status: "error", error: msg.error || undefined }
                : item
            )
          );
        } else if (msg.status === "done") {
          set_done(true);
          set_importing(false);
          query_client.invalidateQueries({ queryKey: ["feeds"] });
          query_client.invalidateQueries({ queryKey: ["entries"] });
        }
      };

      await import_opml_stream(path, channel);
    } catch (e) {
      console.error("OPML import error:", e);
      set_error(String(e));
      set_importing(false);
    }
  }

  function handle_close() {
    if (importing) return; // Don't close while importing
    set_progress_list([]);
    set_current(0);
    set_total(0);
    set_done(false);
    set_error(null);
    on_close();
  }

  const added_count = progress_list.filter((p) => p.status === "added").length;
  const skipped_count = progress_list.filter((p) => p.status === "skipped").length;
  const error_count = progress_list.filter((p) => p.status === "error").length;

  return (
    <Dialog open={open} onOpenChange={(o) => !o && handle_close()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("opml.importTitle")}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {error && <p role="alert" className="text-sm text-destructive">{t("opml.importError")}: {error}</p>}
          {!importing && !done ? (
            <>
              <p className="text-sm text-muted-foreground">
                {t("opml.importDescription")}
              </p>
            </>
          ) : (
            <div className="space-y-3">
              <div role="status" className="flex items-center justify-between text-sm">
                <span className="font-medium">
                  {done ? t("opml.importDone") : t("opml.progressTitle")}
                </span>
                <span className="text-muted-foreground">
                  {current} / {total}
                </span>
              </div>

              {/* Progress bar */}
              <div className="h-2 rounded-full bg-muted overflow-hidden">
                <div
                  className="h-full bg-primary transition-all duration-300"
                  style={{ width: `${total > 0 ? (current / total) * 100 : 0}%` }}
                />
              </div>

              {/* Stats */}
              <div className="flex gap-4 text-xs">
                <span className="flex items-center gap-1 text-green-600 dark:text-green-400">
                  <CheckCircle2 className="h-3 w-3" />
                  {t("opml.added")}: {added_count}
                </span>
                <span className="flex items-center gap-1 text-amber-600 dark:text-amber-400">
                  <Clock className="h-3 w-3" />
                  {t("opml.skipped")}: {skipped_count}
                </span>
                {error_count > 0 && (
                  <span className="flex items-center gap-1 text-red-600 dark:text-red-400">
                    <XCircle className="h-3 w-3" />
                    {t("opml.failed")}: {error_count}
                  </span>
                )}
              </div>

              {/* Feed list */}
              <div className="max-h-64 overflow-y-auto space-y-1 rounded-md border border-border p-2">
                {progress_list.map((feed, i) => (
                  <div
                    key={i}
                    className="flex items-start gap-2 text-xs p-1.5 rounded hover:bg-muted/50"
                  >
                    {feed.status === "processing" && (
                      <Loader2 className="h-3.5 w-3.5 shrink-0 animate-spin text-primary" />
                    )}
                    {feed.status === "added" && (
                      <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-green-600 dark:text-green-400" />
                    )}
                    {feed.status === "skipped" && (
                      <Clock className="h-3.5 w-3.5 shrink-0 text-amber-600 dark:text-amber-400" />
                    )}
                    {feed.status === "error" && (
                      <XCircle className="h-3.5 w-3.5 shrink-0 text-red-600 dark:text-red-400" />
                    )}
                    <div className="flex-1 min-w-0">
                      <p className="font-medium truncate">{feed.title}</p>
                      {feed.error && (
                        <p className="text-muted-foreground break-all">
                          {feed.error}
                        </p>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          {!importing && !done ? (
            <>
              <Button variant="outline" type="button" onClick={handle_close}>
                {t("feed.cancel")}
              </Button>
              <Button type="button" onClick={handle_import}>
                {t("opml.selectFile")}
              </Button>
            </>
          ) : (
            <Button
              type="button"
              onClick={handle_close}
              disabled={importing}
            >
              {importing ? t("opml.importing") : t("common.close")}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
