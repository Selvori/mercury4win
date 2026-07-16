// mercury4win-linux/src/components/layout/app_shell.tsx
// Three-column resizable layout: sidebar | entry list | reader

import { useState, useCallback, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Sidebar } from "./sidebar";
import { StatusBar } from "./status_bar";
import { ResizeHandle } from "./resize_handle";
import { EntryList } from "@/features/entry/components/entry_list";
import { FeedEditor } from "@/features/feed/components/feed_editor";
import { OpmlImport } from "@/features/feed/components/opml_import";
import { OpmlExport } from "@/features/feed/components/opml_export";
import { ReaderPane } from "@/features/reader/components/reader_pane";
import { AgentSettings } from "@/features/agent/components/agent_settings";
import { useKeyboardShortcuts } from "@/hooks/use_keyboard_shortcuts";
import type { Entry } from "@/types";

export function AppShell() {
  const { t } = useTranslation();
  const [selected_feed_id, set_selected_feed_id] = useState<number | null>(null);
  const [selected_entry, set_selected_entry] = useState<Entry | null>(null);
  const [show_feed_editor, set_show_feed_editor] = useState(false);
  const [show_opml_import, set_show_opml_import] = useState(false);
  const [show_opml_export, set_show_opml_export] = useState(false);
  const [show_settings, set_show_settings] = useState(false);
  const [sidebar_view, set_sidebar_view] = useState<"feeds" | "tags">("feeds");
  const [selected_tag_id, set_selected_tag_id] = useState<number | null>(null);

  // ── Resizable panel state ──
  const [sidebar_width, set_sidebar_width] = useState(240);
  const [entry_list_width, set_entry_list_width] = useState(320);
  const [dragging_splitter, set_dragging_splitter] = useState<"sidebar" | "entry_list" | null>(null);
  const drag_ref = useRef<{ start_x: number; start_s: number; start_e: number } | null>(null);

  // Global mouse handlers for drag-resize
  useEffect(() => {
    const on_move = (e: MouseEvent) => {
      if (!drag_ref.current) return;
      const { start_x, start_s, start_e } = drag_ref.current;
      const delta = e.clientX - start_x;

      if (dragging_splitter === "sidebar") {
        const min_s = 160;
        const max_s = Math.min(480, start_s + start_e - 200);
        const new_s = Math.min(max_s, Math.max(min_s, start_s + delta));
        const new_e = start_s + start_e - new_s;
        set_sidebar_width(new_s);
        set_entry_list_width(Math.max(200, new_e));
      } else {
        set_entry_list_width(Math.max(200, start_s + delta));
      }
    };
    const on_up = () => {
      drag_ref.current = null;
      set_dragging_splitter(null);
      // Clear any text selection that may have occurred during drag
      window.getSelection()?.removeAllRanges();
    };
    window.addEventListener("mousemove", on_move);
    window.addEventListener("mouseup", on_up);
    return () => {
      window.removeEventListener("mousemove", on_move);
      window.removeEventListener("mouseup", on_up);
    };
  }, [dragging_splitter]);

  const start_sidebar_resize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    drag_ref.current = { start_x: e.clientX, start_s: sidebar_width, start_e: entry_list_width };
    set_dragging_splitter("sidebar");
  }, [sidebar_width, entry_list_width]);

  const start_entry_resize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    drag_ref.current = { start_x: e.clientX, start_s: entry_list_width, start_e: 0 };
    set_dragging_splitter("entry_list");
  }, [entry_list_width]);

  const handle_select_tag = useCallback((tag_id: number) => {
    set_selected_tag_id(tag_id === selected_tag_id ? null : tag_id);
    set_selected_entry(null);
    if (tag_id !== selected_tag_id) {
      set_selected_feed_id(null);
    }
  }, [selected_tag_id]);

  const handle_select_entry = useCallback((entry: Entry) => {
    set_selected_entry(entry);
  }, []);

  const handle_select_feed = useCallback((id: number) => {
    if (id !== selected_feed_id) {
      set_selected_entry(null);
      set_selected_feed_id(id);
      set_selected_tag_id(null);
    }
  }, [selected_feed_id]);

  // Keyboard shortcuts
  useKeyboardShortcuts({
    on_add_feed: useCallback(() => set_show_feed_editor(true), []),
    on_toggle_read: useCallback(() => {
      if (selected_entry) {
        // mark_read would go here
      }
    }, [selected_entry]),
    on_toggle_star: useCallback(() => {
      if (selected_entry) {
        // mark_starred would go here
      }
    }, [selected_entry]),
  });

  return (
    <div className="flex h-screen flex-col bg-background">
      <div className="flex flex-1 overflow-hidden">
        {/* Left sidebar */}
        <div style={{ width: sidebar_width }} className="shrink-0 overflow-hidden">
          <Sidebar
            selected_feed_id={selected_feed_id}
            on_select_feed={handle_select_feed}
            on_add_feed={() => set_show_feed_editor(true)}
            on_import_opml={() => set_show_opml_import(true)}
            on_export_opml={() => set_show_opml_export(true)}
            on_open_settings={() => set_show_settings(true)}
            view={sidebar_view}
            on_change_view={(v) => {
              if (v === "feeds") { set_selected_tag_id(null); set_selected_entry(null); }
              if (v === "tags") { set_selected_feed_id(null); set_selected_entry(null); }
              set_sidebar_view(v);
            }}
            selected_tag_id={selected_tag_id}
            on_select_tag={handle_select_tag}
          />
        </div>

        {/* Sidebar | Entry splitter */}
        <ResizeHandle
          on_mouse_down={start_sidebar_resize}
          dragging={dragging_splitter === "sidebar"}
        />

        {/* Middle — entry list */}
        <div style={{ width: entry_list_width }} className="shrink-0 overflow-hidden border-r border-border">
          <EntryList
            feed_id={selected_feed_id}
            tag_ids={selected_tag_id ? [selected_tag_id] : undefined}
            selected_entry_id={selected_entry?.id ?? null}
            on_select_entry={handle_select_entry}
          />
        </div>

        {/* Entry list | Reader splitter */}
        <ResizeHandle
          on_mouse_down={start_entry_resize}
          dragging={dragging_splitter === "entry_list"}
        />

        {/* Right — reader area (fills remaining space) */}
        <main className="flex flex-1 flex-col overflow-hidden">
          {selected_entry ? (
            <ReaderPane entry={selected_entry} />
          ) : (
            <div className="flex flex-1 items-center justify-center text-muted-foreground">
              <div className="text-center">
                <h1 className="text-2xl font-semibold text-foreground">{t("app.welcome")}</h1>
                <p className="mt-2">{t("app.subtitle")}</p>
              </div>
            </div>
          )}
          <StatusBar />
        </main>
      </div>

      {/* Dialogs */}
      <FeedEditor open={show_feed_editor} on_close={() => set_show_feed_editor(false)} />
      <OpmlImport open={show_opml_import} on_close={() => set_show_opml_import(false)} />
      <OpmlExport open={show_opml_export} on_close={() => set_show_opml_export(false)} />

      {show_settings && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={() => set_show_settings(false)}>
          <div className="max-h-[85vh] w-full max-w-2xl overflow-y-auto rounded-xl border border-border bg-popover shadow-xl" onClick={(e) => e.stopPropagation()}>
            <AgentSettings />
          </div>
        </div>
      )}
    </div>
  );
}
