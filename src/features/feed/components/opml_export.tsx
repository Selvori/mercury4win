// mercury4win-linux/src/features/feed/components/opml_export.tsx
// OPML export dialog — select feeds, choose save path, export

import { useState, useMemo } from "react";
import { useQuery, useMutation } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { CheckCircle2 } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { load_feeds, export_opml } from "@/lib/tauri_bindings";

interface Props {
  open: boolean;
  on_close: () => void;
}

export function OpmlExport({ open, on_close }: Props) {
  const { t } = useTranslation();
  const [selected_ids, set_selected_ids] = useState<Set<number>>(new Set());
  const [success, set_success] = useState(false);

  const { data: feeds, isLoading } = useQuery({
    queryKey: ["feeds"],
    queryFn: load_feeds,
    enabled: open,
  });

  const all_ids = useMemo(() => new Set((feeds ?? []).map((f) => f.id)), [feeds]);

  function toggle(id: number) {
    set_selected_ids((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function toggle_all() {
    if (selected_ids.size === (feeds?.length ?? 0)) {
      set_selected_ids(new Set());
    } else {
      set_selected_ids(new Set(all_ids));
    }
  }

  const ids_to_export = selected_ids.size > 0 ? [...selected_ids] : [...all_ids];

  const export_mutation = useMutation({
    mutationFn: async () => {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const path = await save({
        defaultPath: "mercury_feeds.opml",
        filters: [{ name: "OPML", extensions: ["opml"] }],
      });
      if (!path) return null;
      await export_opml(path, ids_to_export.length === feeds?.length ? undefined : ids_to_export);
      return path;
    },
    onSuccess: (path) => {
      if (path) set_success(true);
    },
  });

  function handle_close() {
    set_success(false);
    set_selected_ids(new Set());
    export_mutation.reset();
    on_close();
  }

  const all_selected = feeds && selected_ids.size === feeds.length;

  return (
    <Dialog open={open} onOpenChange={(o) => !o && handle_close()}>
      <DialogContent className="max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t("opml.exportTitle")}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {!success ? (
            <>
              <p className="text-sm text-muted-foreground">
                {t("opml.exportDescription")}
              </p>

              {isLoading ? (
                <p className="text-xs text-muted-foreground">{t("common.loading")}</p>
              ) : !feeds?.length ? (
                <p className="text-xs text-muted-foreground">{t("feed.noFeeds")}</p>
              ) : (
                <div className="space-y-1">
                  <label className="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-accent cursor-pointer">
                    <input
                      type="checkbox"
                      className="h-4 w-4 rounded border-border"
                      checked={!!all_selected}
                      onChange={toggle_all}
                    />
                    <span className="font-medium">
                      {t("opml.exportAllFeeds", { count: feeds.length })}
                    </span>
                  </label>
                  <div className="border-t border-border pt-1">
                    {feeds.map((feed) => (
                      <label
                        key={feed.id}
                        className="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-accent cursor-pointer"
                      >
                        <input
                          type="checkbox"
                          className="h-4 w-4 rounded border-border"
                          checked={selected_ids.has(feed.id)}
                          onChange={() => toggle(feed.id)}
                        />
                        <span className={`flex-1 truncate ${!feed.title ? "text-muted-foreground" : ""}`}>
                          {feed.title || feed.feed_url}
                        </span>
                      </label>
                    ))}
                  </div>
                </div>
              )}

              {export_mutation.isError && (
                <p className="text-xs text-destructive">
                  {t("opml.exportError")}: {String(export_mutation.error)}
                </p>
              )}
            </>
          ) : (
            <div className="flex items-center gap-2 text-sm font-medium text-green-600 dark:text-green-400">
              <CheckCircle2 className="h-4 w-4" />
              {t("opml.exportSuccess", { count: ids_to_export.length })}
            </div>
          )}
        </div>

        <DialogFooter>
          {!success ? (
            <>
              <Button variant="outline" type="button" onClick={handle_close}>
                {t("feed.cancel")}
              </Button>
              <Button
                type="button"
                onClick={() => export_mutation.mutate()}
                disabled={export_mutation.isPending || (feeds?.length ?? 0) === 0}
              >
                {export_mutation.isPending ? t("opml.exporting") : t("opml.export")}
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
