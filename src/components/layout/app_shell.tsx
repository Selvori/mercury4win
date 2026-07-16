// mercury4win-linux/src/components/layout/app_shell.tsx
// Three-column layout: sidebar | entry list | reader

import { useState, useCallback } from "react";
import { Sidebar } from "./sidebar";
import { StatusBar } from "./status_bar";
import { EntryList } from "@/features/entry/components/entry_list";
import { FeedEditor } from "@/features/feed/components/feed_editor";
import { ReaderPane } from "@/features/reader/components/reader_pane";
import { AgentSettings } from "@/features/agent/components/agent_settings";
import { useKeyboardShortcuts } from "@/hooks/use_keyboard_shortcuts";
import type { Entry } from "@/types";

export function AppShell() {
  const [selected_feed_id, set_selected_feed_id] = useState<number | null>(null);
  const [selected_entry, set_selected_entry] = useState<Entry | null>(null);
  const [show_feed_editor, set_show_feed_editor] = useState(false);
  const [show_settings, set_show_settings] = useState(false);
  const [sidebar_view, set_sidebar_view] = useState<"feeds" | "tags">("feeds");
  const [selected_tag_id, set_selected_tag_id] = useState<number | null>(null);

  const handle_select_tag = useCallback((tag_id: number) => {
    set_selected_tag_id(tag_id === selected_tag_id ? null : tag_id);
    set_selected_entry(null);
    if (tag_id !== selected_tag_id) {
      set_sidebar_view("feeds");
    }
  }, [selected_tag_id]);

  const handle_select_entry = useCallback((entry: Entry) => {
    set_selected_entry(entry);
  }, []);

  const handle_select_feed = useCallback((id: number) => {
    if (id !== selected_feed_id) {
      set_selected_entry(null);
      set_selected_feed_id(id);
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
        {/* Left sidebar — feeds or tags */}
        <Sidebar
          selected_feed_id={selected_feed_id}
          on_select_feed={handle_select_feed}
          on_add_feed={() => set_show_feed_editor(true)}
          on_open_settings={() => set_show_settings(true)}
          view={sidebar_view}
          on_change_view={set_sidebar_view}
          selected_tag_id={selected_tag_id}
          on_select_tag={handle_select_tag}
        />

        {/* Middle — entry list */}
        <div className="w-80 shrink-0 border-r border-border">
          <EntryList
            feed_id={selected_feed_id}
            tag_ids={selected_tag_id ? [selected_tag_id] : undefined}
            selected_entry_id={selected_entry?.id ?? null}
            on_select_entry={handle_select_entry}
          />
        </div>

        {/* Right — reader area */}
        <main className="flex flex-1 flex-col overflow-hidden">
          {selected_entry ? (
            <ReaderPane entry={selected_entry} />
          ) : (
            <div className="flex flex-1 items-center justify-center text-muted-foreground">
              <div className="text-center">
                <h1 className="text-2xl font-semibold text-foreground">Mercury</h1>
                <p className="mt-2">Select a feed and an entry to start reading</p>
              </div>
            </div>
          )}
          <StatusBar />
        </main>
      </div>

      {/* Feed add dialog */}
      <FeedEditor
        open={show_feed_editor}
        on_close={() => set_show_feed_editor(false)}
      />

      {/* Settings modal */}
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
