// mercury4win-linux/src/features/reader/components/reader_pane.tsx
// Reader main panel — renders article HTML with resizable side panels

import { useQuery } from "@tanstack/react-query";
import {
  Loader2,
  FileText,
  Sparkles,
  Languages,
  Tag,
  BookOpen,
  Type,
  ExternalLink,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { build_reader_content } from "@/lib/tauri_bindings";
import { useAppStore } from "@/stores/app_store";
import type { Entry } from "@/types";
import { useState, useRef, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ReaderNote } from "./reader_note";
import { ReaderSummary } from "./reader_summary";
import { ReaderTranslation } from "./reader_translation";
import { ReaderTagging } from "./reader_tagging";
import { FontSettingsDialog } from "./font_settings_dialog";

interface Props {
  entry: Entry;
}

type Panel = "reader" | "notes" | "summary" | "translation" | "tags";

export function ReaderPane({ entry }: Props) {
  const { t } = useTranslation();
  const [panel, set_panel] = useState<Panel>("reader");
  const [font_dialog_open, set_font_dialog_open] = useState(false);
  const [side_panel_width, set_side_panel_width] = useState(320);
  const [dragging_side, set_dragging_side] = useState(false);

  const font_family = useAppStore((s) => s.font_family);
  const font_size = useAppStore((s) => s.font_size);
  const effective_theme = useAppStore((s) => s.effective_theme());

  // Compute reader theme ID (classic/paper + light/dark)
  const reader_theme_id = `classic-${effective_theme}`;

  const drag_ref = useRef<{ start_x: number; start_w: number } | null>(null);

  // Global mouse handlers for side panel resize
  const on_mouse_move = useCallback((e: MouseEvent) => {
    if (!drag_ref.current) return;
    const { start_x, start_w } = drag_ref.current;
    const delta = start_x - e.clientX; // reversed: drag left = wider panel
    set_side_panel_width(Math.max(200, Math.min(560, start_w + delta)));
  }, []);
  const on_mouse_up = useCallback(() => {
    // Ignore mouseups from regular controls. This handler is registered on
    // window for resizing, but input clicks also reach it in WebView2.
    if (!drag_ref.current) return;
    drag_ref.current = null;
    set_dragging_side(false);
    window.getSelection()?.removeAllRanges();
  }, []);

  useEffect(() => {
    window.addEventListener("mousemove", on_mouse_move);
    window.addEventListener("mouseup", on_mouse_up);
    window.addEventListener("blur", on_mouse_up);
    document.addEventListener("mouseleave", on_mouse_up);
    return () => {
      window.removeEventListener("mousemove", on_mouse_move);
      window.removeEventListener("mouseup", on_mouse_up);
      window.removeEventListener("blur", on_mouse_up);
      document.removeEventListener("mouseleave", on_mouse_up);
    };
  }, [on_mouse_move, on_mouse_up]);

  const start_side_resize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    drag_ref.current = { start_x: e.clientX, start_w: side_panel_width };
    set_dragging_side(true);
  }, [side_panel_width]);

  const { data: reader_html, isLoading, isFetching, isError, error } = useQuery({
    queryKey: ["reader", entry.id, reader_theme_id, font_family, font_size],
    queryFn: () => build_reader_content(entry.id, reader_theme_id, font_family, font_size),
    staleTime: 0,
  });

  const panel_options: { value: Panel; icon: typeof BookOpen; label: string }[] = [
    { value: "reader", icon: BookOpen, label: t("reader.reader") },
    { value: "notes", icon: FileText, label: t("reader.notes") },
    { value: "summary", icon: Sparkles, label: t("reader.summary") },
    { value: "translation", icon: Languages, label: t("reader.translation") },
    { value: "tags", icon: Tag, label: t("reader.tags") },
  ];

  const show_side_panel = panel !== "reader";

  return (
    <div className="flex h-full">
      {/* Main reader area */}
      <div className="flex h-full flex-col flex-1 overflow-hidden">
        {/* Toolbar */}
        <div className="flex items-center gap-1.5 border-b border-border px-3 py-1.5">
          {/* Panel tabs */}
          <div className="flex items-center gap-0.5 rounded-md border border-border bg-muted/50 p-0.5 mr-2">
            {panel_options.map((opt) => (
              <Button
                key={opt.value}
                variant="ghost"
                size="sm"
                className={cn(
                  "h-7 px-2 text-xs rounded-sm",
                  panel === opt.value && "bg-background shadow-sm",
                )}
                onClick={() => set_panel(panel === opt.value ? "reader" : opt.value)}
                title={opt.label}
              >
                <opt.icon className="h-3.5 w-3.5" />
              </Button>
            ))}
          </div>
          <div className="flex-1 flex items-center gap-2">
            <h2 className="text-sm font-medium truncate max-w-md">
              {entry.title || t("entry.untitled")}
            </h2>
          </div>
          {/* Open in browser button */}
          {entry.url && (
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={async () => {
                const { openUrl } = await import("@tauri-apps/plugin-opener");
                await openUrl(entry.url!);
              }}
              title={t("reader.openInBrowser")}
            >
              <ExternalLink className="h-4 w-4" />
            </Button>
          )}
          {/* Font settings button */}
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => set_font_dialog_open(true)}
            title={t("fontSettings.title")}
          >
            <Type className="h-4 w-4" />
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto">
          {isLoading || isFetching ? (
            <div className="flex h-full items-center justify-center">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : isError ? (
            <div className="flex h-full items-center justify-center p-8 text-center">
              <div>
                <p className="text-sm font-medium text-destructive">{t("readerPane.failedToLoad")}</p>
                <p className="mt-1 text-xs text-muted-foreground">{String(error)}</p>
              </div>
            </div>
          ) : !reader_html ? (
            <div className="flex h-full items-center justify-center p-8 text-center">
              <p className="text-sm text-muted-foreground">
                {t("readerPane.extractionFailed")}
              </p>
            </div>
          ) : (
            <iframe
              srcDoc={reader_html}
              className="h-full w-full border-0"
              sandbox="allow-same-origin"
              title={t("readerPane.readerContent")}
              onLoad={(e) => {
                const doc = e.currentTarget.contentDocument;
                if (!doc) return;
                // Route in-article link clicks to the system browser
                doc.addEventListener("click", async (ev) => {
                  const anchor = (ev.target as HTMLElement)?.closest("a");
                  const href = anchor?.getAttribute("href");
                  if (!href) return;
                  if (/^https?:\/\//i.test(href)) {
                    ev.preventDefault();
                    const { openUrl } = await import("@tauri-apps/plugin-opener");
                    await openUrl(href);
                  }
                });
              }}
            />
          )}
        </div>
      </div>

      {/* Resize handle + Side panel */}
      {show_side_panel && (
        <>
          {/* Draggable splitter */}
          <div
            className={cn(
              "relative w-1.5 shrink-0 cursor-col-resize border-l border-border select-none",
              dragging_side && "bg-primary/50",
            )}
            onMouseDown={start_side_resize}
          >
            <div className="absolute inset-y-0 -left-1 -right-1" />
          </div>

          <div style={{ width: side_panel_width }} className="shrink-0 overflow-hidden">
            {panel === "notes" && <ReaderNote entry_id={entry.id} />}
            {panel === "summary" && (
              <ReaderSummary entry_id={entry.id} entry_title={entry.title || t("entry.untitled")} />
            )}
            {panel === "translation" && <ReaderTranslation entry_id={entry.id} />}
            {panel === "tags" && <ReaderTagging entry_id={entry.id} />}
          </div>
        </>
      )}

      {/* Font settings dialog */}
      <FontSettingsDialog
        open={font_dialog_open}
        on_close={() => set_font_dialog_open(false)}
      />
    </div>
  );
}
