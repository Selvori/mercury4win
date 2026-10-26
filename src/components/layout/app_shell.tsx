// mercury4win/src/components/layout/app_shell.tsx
// Three-column resizable layout: sidebar | entry list | reader

import { useState, useCallback, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Sidebar } from "./sidebar";
import { TopicWorkspace } from "@/features/topics/topic_workspace";
import { StatusBar } from "./status_bar";
import { ResizeHandle } from "./resize_handle";
import { EntryList } from "@/features/entry/components/entry_list";
import { FeedEditor } from "@/features/feed/components/feed_editor";
import { OpmlImport } from "@/features/feed/components/opml_import";
import { OpmlExport } from "@/features/feed/components/opml_export";
import { ReaderPane } from "@/features/reader/components/reader_pane";
import { AgentSettings } from "@/features/agent/components/agent_settings";
import { Dialog } from "@/components/ui/dialog";
import { useKeyboardShortcuts } from "@/hooks/use_keyboard_shortcuts";
import type { Entry, SmartView } from "@/types";

export function AppShell() {
  const { t } = useTranslation();
  const [topics_active, set_topics_active] = useState(false);
  const [selected_feed_id, set_selected_feed_id] = useState<number | null | undefined>(undefined);
  const [selected_entry, set_selected_entry] = useState<Entry | null>(null);
  const [show_feed_editor, set_show_feed_editor] = useState(false);
  const [show_opml_import, set_show_opml_import] = useState(false);
  const [show_opml_export, set_show_opml_export] = useState(false);
  const [show_settings, set_show_settings] = useState(false);
  const [tag_batch_active, set_tag_batch_active] = useState(false);
  const [sidebar_view, set_sidebar_view] = useState<"feeds" | "tags">("feeds");
  const [selected_tag_ids, set_selected_tag_ids] = useState<number[]>([]);

  const [saved_view, set_saved_view] = useState<SmartView | null>(null);
  const [search_active, set_search_active] = useState(false);
  const [scope_revision, set_scope_revision] = useState(0);
  const [note_request, set_note_request] = useState(0);
  const open_view = (view: SmartView) => {
    set_topics_active(false);
    set_search_active(false);
    set_saved_view(view); set_scope_revision((v) => v + 1); set_selected_feed_id(undefined);
    set_selected_tag_ids([]); set_starred_only(false); set_selected_entry(null);
  };

  const [starred_only, set_starred_only] = useState(false);

  // ── Resizable panel state ──
  const [sidebar_width, set_sidebar_width] = useState(240);
  const [entry_list_width, set_entry_list_width] = useState(320);
  const [dragging_splitter, set_dragging_splitter] = useState<"sidebar" | "entry_list" | null>(null);
  const drag_ref = useRef<{ kind: "sidebar" | "entry_list"; start_x: number; start_s: number; start_e: number } | null>(null);

  // Global mouse handlers for drag-resize
  useEffect(() => {
    const on_move = (e: MouseEvent) => {
      if (!drag_ref.current) return;
      const { start_x, start_s, start_e } = drag_ref.current;
      const delta = e.clientX - start_x;

      if (drag_ref.current.kind === "sidebar") {
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
      // A window-level mouseup also fires for ordinary clicks. Only run the
      // drag cleanup when a splitter drag was actually started; otherwise the
      // selection cleanup can interfere with input focus in WebView2.
      if (!drag_ref.current) return;
      drag_ref.current = null;
      set_dragging_splitter(null);
      // Clear any text selection that may have occurred during drag
      window.getSelection()?.removeAllRanges();
    };
    window.addEventListener("mousemove", on_move);
    window.addEventListener("mouseup", on_up);
    window.addEventListener("blur", on_up);
    document.addEventListener("mouseleave", on_up);
    return () => {
      window.removeEventListener("mousemove", on_move);
      window.removeEventListener("mouseup", on_up);
      window.removeEventListener("blur", on_up);
      document.removeEventListener("mouseleave", on_up);
    };
  }, []);

  const start_sidebar_resize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    drag_ref.current = { kind: "sidebar", start_x: e.clientX, start_s: sidebar_width, start_e: entry_list_width };
    set_dragging_splitter("sidebar");
  }, [sidebar_width, entry_list_width]);

  const start_entry_resize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    drag_ref.current = { kind: "entry_list", start_x: e.clientX, start_s: entry_list_width, start_e: 0 };
    set_dragging_splitter("entry_list");
  }, [entry_list_width]);

  const handle_select_tag = useCallback((tag_id: number) => {
    set_topics_active(false);
    set_search_active(false);
    set_saved_view(null);
    set_selected_tag_ids((ids) => ids.includes(tag_id) ? ids.filter((id) => id !== tag_id) : ids.length < 5 ? [...ids, tag_id] : ids);
    set_selected_entry(null);
    set_selected_feed_id(undefined);
    set_starred_only(false);
  }, []);

  const handle_tags_changed = useCallback((removed: number[], target?: number) => {
    set_selected_tag_ids((ids) => {
      const affected = ids.some((id) => removed.includes(id));
      const remaining = ids.filter((id) => !removed.includes(id));
      return affected && target ? [...new Set([...remaining, target])].slice(0, 5) : remaining;
    });
  }, []);

  const handle_select_entry = useCallback((entry: Entry) => {
    set_note_request(0);
    set_selected_entry(entry);
  }, []);

  const handle_select_feed = useCallback((id: number | null) => {
    set_topics_active(false);
    set_search_active(false);
    set_saved_view(null);
    if (id !== selected_feed_id || starred_only || selected_tag_ids.length || saved_view) {
      set_selected_entry(null);
      set_selected_feed_id(id);
      set_selected_tag_ids([]);
      set_starred_only(false);
    }
  }, [selected_feed_id, starred_only, selected_tag_ids, saved_view]);

  function toggle_search() {
    set_topics_active(false);
    set_search_active(!search_active);
    set_saved_view(null);
    set_selected_feed_id(search_active ? undefined : null);
    set_selected_tag_ids([]);
    set_starred_only(false);
    set_selected_entry(null);
    set_note_request(0);
  }

  // Keyboard shortcuts
  useKeyboardShortcuts({
    on_add_feed: useCallback(() => {
      set_show_settings(false);
      set_show_feed_editor(true);
    }, []),
  });

  return (
    <div className="flex h-screen flex-col bg-background">
      <div className="flex flex-1 overflow-hidden">
        {/* Left sidebar */}
        <div style={{ width: sidebar_width }} className="shrink-0 overflow-hidden">
          <Sidebar
            topics_active={topics_active}
            on_topics={() => { set_topics_active(true); set_search_active(false); set_saved_view(null); set_selected_feed_id(undefined); set_starred_only(false); }}
            selected_view_id={search_active ? undefined : saved_view?.id}
            on_select_view={open_view}
            search_active={search_active}
            on_search={toggle_search}
            selected_feed_id={selected_feed_id}
            on_select_feed={handle_select_feed}
            on_add_feed={() => { set_show_settings(false); set_show_feed_editor(true); }}
            on_import_opml={() => { set_show_settings(false); set_show_opml_import(true); }}
            on_export_opml={() => { set_show_settings(false); set_show_opml_export(true); }}
            on_open_settings={() => {
              // Close any modal <dialog> first — a modal dialog locks focus to
              // itself, so an open feed/OPML dialog would steal focus from the
              // Settings overlay's inputs (click-to-focus loses to the dialog).
              set_show_feed_editor(false);
              set_show_opml_import(false);
              set_show_opml_export(false);
              set_show_settings(true);
            }}
            view={sidebar_view}
            on_change_view={(v) => {
              set_topics_active(false);
              set_search_active(false);
              set_saved_view(null);
              if (v === "feeds") { set_selected_tag_ids([]); set_selected_entry(null); }
              if (v === "tags") { set_selected_feed_id(undefined); set_selected_entry(null); }
              set_starred_only(false);
              set_sidebar_view(v);
            }}
            selected_tag_ids={selected_tag_ids}
            starred_only={starred_only}
            on_select_starred={() => {
              set_topics_active(false);
              set_search_active(false);
              set_saved_view(null);
              set_selected_feed_id(null); set_selected_tag_ids([]); set_selected_entry(null); set_starred_only(true); set_sidebar_view("feeds");
            }}
            on_select_tag={handle_select_tag}
            on_tags_changed={handle_tags_changed}
          />
        </div>

        {topics_active ? <TopicWorkspace /> : <>
        {/* Sidebar | Entry splitter */}
        <ResizeHandle
          on_mouse_down={start_sidebar_resize}
          dragging={dragging_splitter === "sidebar"}
        />

        {/* Middle — entry list */}
        <div style={{ width: entry_list_width }} className="shrink-0 overflow-hidden border-r border-border">
          <EntryList
            key={saved_view ? `view-${saved_view.id}-${scope_revision}` : "navigation"}
            saved_view={saved_view}
            search_mode={search_active}
            on_edit_search={() => set_search_active(true)}
            on_view_saved={open_view}
            on_view_deleted={() => handle_select_feed(null)}
            on_select_note={(entry) => { set_selected_entry(entry); set_note_request((n) => n + 1); }}
            feed_id={selected_feed_id}
            tag_ids={selected_tag_ids.length ? selected_tag_ids : undefined}
            starred_only={starred_only}
            on_clear_selection={() => set_selected_entry(null)}
            on_delete_entry={(id) => set_selected_entry((entry) => entry?.id === id ? null : entry)}
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
            <ReaderPane note_request={note_request} entry={selected_entry} on_select_entry={handle_select_entry} />
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
        </>}
      </div>

      {/* Dialogs */}
      <FeedEditor open={show_feed_editor} on_close={() => set_show_feed_editor(false)} />
      <OpmlImport open={show_opml_import} on_close={() => set_show_opml_import(false)} />
      <OpmlExport open={show_opml_export} on_close={() => set_show_opml_export(false)} />

      {show_settings && (
        <Dialog
          open={show_settings}
          aria_label={t("sidebar.settings")}
          onOpenChange={(o) => { if (!o && !tag_batch_active) set_show_settings(false); }}
          className="max-w-2xl"
        >
          <AgentSettings on_tags_changed={handle_tags_changed} on_batch_active_change={set_tag_batch_active} />
        </Dialog>
      )}
    </div>
  );
}
