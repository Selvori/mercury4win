// mercury4win/src/features/reader/components/reader_pane.tsx
// Reader main panel — renders article HTML with resizable side panels

import { useKeyboardShortcuts } from "@/hooks/use_keyboard_shortcuts";
import { useQuery, useQueryClient } from "@tanstack/react-query";
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
import type { Entry, EntryFilter } from "@/types";
import { useState, useRef, useCallback, useEffect, useLayoutEffect } from "react";
import { useTranslation } from "react-i18next";
import { ReaderNote } from "./reader_note";
import { ReaderSummary } from "./reader_summary";
import { ReaderTranslation } from "./reader_translation";
import { RelatedEntries } from "./related_entries";
import { ReaderTagging } from "./reader_tagging";
import { FontSettingsDialog } from "./font_settings_dialog";
import { DigestDialog } from "./digest_dialog";
import { useDigestSelectionStore } from "@/stores/digest_selection_store";
import { update_bilingual_document } from "../lib/update_bilingual_document";

interface Props {
  entry: Entry;
  note_request?: number;
  on_select_entry?: (entry: Entry) => void;
}

type Panel = "reader" | "notes" | "summary" | "translation" | "tags";

export function ReaderPane({ entry, on_select_entry, note_request }: Props) {
  const digest_selecting = useDigestSelectionStore((s) => s.selecting);
  const query_client = useQueryClient();
  const { t } = useTranslation();
  const [bilingual, set_bilingual] = useState<{ entry_id: number; html: string; reader_html?: string } | null>(null);
  const receive_bilingual = useCallback((entry_id: number, html: string | null, reader_html?: string) => {
    set_bilingual((previous) => html ? { entry_id, html, reader_html } : previous?.entry_id === entry_id ? null : previous);
  }, []);
  const [panel, set_panel] = useState<Panel>("reader");
  useEffect(() => { if (note_request) set_panel("notes"); }, [note_request]);
  useLayoutEffect(() => {
    set_panel((current) => current === "translation" ? "reader" : current);
    set_bilingual(null);
  }, [entry.id]);
  const [font_dialog_open, set_font_dialog_open] = useState(false);
  const [digest_entry, set_digest_entry] = useState<number | null>(null);
  useLayoutEffect(() => { set_digest_entry(null); }, [entry.id]);
  const [side_panel_width, set_side_panel_width] = useState(320);
  const [dragging_side, set_dragging_side] = useState(false);

  const font_family = useAppStore((s) => s.font_family);
  const font_size = useAppStore((s) => s.font_size);
  const reader_style = useAppStore((s) => s.reader_style);
  const effective_theme = useAppStore((s) => s.effective_theme());

  // Compute reader theme ID (classic/paper + light/dark)
  const reader_theme_id = `${reader_style}-${effective_theme}`;

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

  const { data: reader_html, dataUpdatedAt: reader_updated_at, isLoading, isFetching, isError, error, refetch } = useQuery({
    queryKey: ["reader", entry.id, reader_theme_id, font_family, font_size],
    queryFn: () => build_reader_content(entry.id, reader_theme_id, font_family, font_size),
    staleTime: 0,
    // Keep the same frame during appearance changes. Chromium can leave frames
    // created inside an inert modal background unable to receive wheel input.
    // Never carry an article's HTML into a different article's query.
    placeholderData: (previous, previous_query) => previous_query?.queryKey[1] === entry.id ? previous : undefined,
  });
  useEffect(() => {
    if (!reader_updated_at || isFetching) return;
    void query_client.invalidateQueries({ queryKey: ["entries"], predicate: (query) => Boolean((query.queryKey[1] as EntryFilter | undefined)?.search?.trim()) });
    // Reader reconstruction can finish after a result's initial source lookup.
    void query_client.invalidateQueries({ queryKey: ["summary_result", entry.id] });
    void query_client.invalidateQueries({ queryKey: ["latest_summary", entry.id] });
    void query_client.invalidateQueries({ queryKey: ["translation_result", entry.id] });
    void query_client.invalidateQueries({ queryKey: ["translation_document", entry.id] });
  }, [entry.id, reader_updated_at, isFetching, query_client]);
  const summary_fallback = reader_html?.includes(
    '<meta name="mercury-content-source" content="summary">',
  ) ?? false;

  const panel_options: { value: Panel; icon: typeof BookOpen; label: string }[] = [
    { value: "reader", icon: BookOpen, label: t("reader.reader") },
    { value: "notes", icon: FileText, label: t("reader.notes") },
    { value: "summary", icon: Sparkles, label: t("reader.summary") },
    { value: "translation", icon: Languages, label: t("reader.translation") },
    { value: "tags", icon: Tag, label: t("reader.tags") },
  ];

  useKeyboardShortcuts({
    on_toggle_reader: () => set_panel((current) => current === "summary" ? "reader" : "summary"),
    on_toggle_translation: () => set_panel((current) => current === "translation" ? "reader" : "translation"),
    on_toggle_note: () => set_panel((current) => current === "notes" ? "reader" : "notes"),
  });

  const show_side_panel = panel === "notes" || panel === "summary" || panel === "tags";
  const display_html = panel === "translation" && bilingual?.entry_id === entry.id
    && bilingual.reader_html === reader_html ? bilingual.html : reader_html;
  const frame_ref = useRef<HTMLIFrameElement>(null);
  const loaded_frame = useRef<{ doc: Document; reader_html: string; entry_id: number } | null>(null);
  const update_translation = useCallback(() => {
    const frame = frame_ref.current;
    if (!frame || !reader_html || !display_html) return;
    const doc = frame.contentDocument;
    if (!doc) return;
    if (loaded_frame.current?.doc !== doc || loaded_frame.current.reader_html !== reader_html
      || loaded_frame.current.entry_id !== entry.id) return;
    if (!update_bilingual_document(doc, display_html) && frame.srcdoc !== display_html) {
      frame.srcdoc = display_html;
    }
  }, [entry.id, reader_html, display_html]);
  useLayoutEffect(update_translation, [update_translation]);

  return (
    <div className="flex h-full" inert={digest_selecting}>
      {/* Main reader area */}
      <div className="flex h-full min-w-0 flex-col flex-1 overflow-hidden">
        {/* Toolbar */}
        <div className="flex shrink-0 items-center gap-1.5 border-b border-border px-3 py-1.5">
          {/* Panel tabs */}
          <div className="flex shrink-0 items-center gap-0.5 rounded-md border border-border bg-muted/50 p-0.5 mr-2">
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
                aria-label={opt.label}
                aria-pressed={panel === opt.value}
              >
                <opt.icon className="h-3.5 w-3.5" />
              </Button>
            ))}
          </div>
          <div className="min-w-0 flex-1">
            <h2 className="text-sm font-medium truncate" title={entry.title || t("entry.untitled")}>
              {entry.title || t("entry.untitled")}
            </h2>
          </div>
          {/* Open in browser button */}
          <Button variant="ghost" size="sm" onClick={() => set_digest_entry(entry.id)}>{t("digest.title")}</Button>
          {entry.url && (
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={async () => {
                const { openUrl } = await import("@tauri-apps/plugin-opener");
                await openUrl(entry.url!);
              }}
              title={t("reader.openInBrowser")}
              aria-label={t("reader.openInBrowser")}
            >
              <ExternalLink className="h-4 w-4" />
            </Button>
          )}
          {/* Font settings button */}
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => set_font_dialog_open(true)}
            title={t("readerSettings.title")}
            aria-label={t("readerSettings.title")}
          >
            <Type className="h-4 w-4" />
          </Button>
        </div>

        {panel === "translation" && (
          <ReaderTranslation key={entry.id} entry_id={entry.id} reader_html={reader_html} reader_ready={!!reader_html && !isFetching && !summary_fallback} on_bilingual_html={receive_bilingual} automatic />
        )}

        {summary_fallback && !isFetching && (
          <div role="status" className="flex items-center gap-2 border-b border-border bg-muted px-3 py-2">
            <p className="flex-1 text-xs text-muted-foreground">{t("readerPane.summaryFallback")}</p>
            <Button variant="ghost" size="sm" onClick={() => void refetch()}>
              {t("readerPane.retryFullArticle")}
            </Button>
          </div>
        )}

        {/* Content */}
        <div className="flex-1 overflow-y-auto">
          {(isLoading || isFetching) && !reader_html ? (
            <div role="status" aria-label={t("common.loading")} className="flex h-full items-center justify-center">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : isError ? (
            <div className="flex h-full items-center justify-center p-8 text-center">
              <div>
                <p role="alert" className="text-sm font-medium text-destructive">{t("readerPane.failedToLoad")}</p>
                <p className="mt-1 text-xs text-muted-foreground">{String(error)}</p>
                <Button className="mt-3" variant="outline" size="sm" onClick={() => void refetch()}>
                  {t("readerPane.retryFullArticle")}
                </Button>
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
              key={entry.id}
              ref={frame_ref}
              srcDoc={reader_html}
              className="h-full w-full border-0"
              sandbox="allow-same-origin"
              title={t("readerPane.readerContent")}
              onLoad={(e) => {
                const doc = e.currentTarget.contentDocument;
                if (!doc) return;
                loaded_frame.current = { doc, reader_html, entry_id: entry.id };
                update_translation();
                // Route in-article link clicks to the system browser
                doc.addEventListener("keydown", (event) => {
                  if (event.defaultPrevented || event.isComposing || event.keyCode === 229 || event.repeat || event.altKey) return;
                  const target = event.target as Element | null;
                  if (target?.nodeType === 1 && target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"]')) return;
                  if (event.key === "Enter" && target?.nodeType === 1 && target.closest('button, a, [role="button"]')) return;
                  const forwarded = new KeyboardEvent("keydown", { key: event.key, ctrlKey: event.ctrlKey,
                    metaKey: event.metaKey, shiftKey: event.shiftKey, bubbles: true, cancelable: true });
                  if (!document.dispatchEvent(forwarded)) event.preventDefault();
                });
                doc.addEventListener("click", async (ev) => {
                  const anchor = (ev.target as HTMLElement)?.closest("a");
                  const href = anchor?.getAttribute("href");
                  if (!href) return;
                  if (href.startsWith("#")) return;
                  // Unknown/local protocols must never navigate the srcDoc.
                  ev.preventDefault();
                  if (/^https?:\/\//i.test(href)) {
                    const { openUrl } = await import("@tauri-apps/plugin-opener");
                    await openUrl(href);
                  }
                });
              }}
            />
          )}
        </div>
        <RelatedEntries entry_id={entry.id} on_select={on_select_entry} />
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
            {panel === "notes" && <ReaderNote key={entry.id} entry_id={entry.id} />}
            {panel === "summary" && (
              <ReaderSummary key={entry.id} entry_id={entry.id} entry_title={entry.title || t("entry.untitled")} reader_ready={!!reader_html && !isFetching && !summary_fallback} />
            )}
            {panel === "tags" && <ReaderTagging key={entry.id} entry_id={entry.id} />}
          </div>
        </>
      )}

      {/* Font settings dialog */}
      {digest_entry === entry.id && <DigestDialog key={entry.id} entry_ids={[entry.id]} on_close={() => set_digest_entry(null)} />}
      <FontSettingsDialog
        open={font_dialog_open}
        on_close={() => set_font_dialog_open(false)}
      />
    </div>
  );
}
