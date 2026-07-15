// mercury4win-linux/src/components/layout/sidebar.tsx
// Left sidebar with feed list and navigation

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Rss, Library, Tags, Settings } from "lucide-react";
import { FeedList } from "@/features/feed/components/feed_list";
import { import_opml, export_opml } from "@/lib/tauri_bindings";

interface Props {
  selected_feed_id: number | null;
  on_select_feed: (id: number) => void;
  on_add_feed: () => void;
  on_open_settings: () => void;
  view: "feeds" | "tags";
  on_change_view: (view: "feeds" | "tags") => void;
}

export function Sidebar({
  selected_feed_id,
  on_select_feed,
  on_add_feed,
  on_open_settings,
  view,
  on_change_view,
}: Props) {
  const query_client = useQueryClient();

  const import_mutation = useMutation({
    mutationFn: async () => {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        filters: [{ name: "OPML", extensions: ["opml", "xml"] }],
        multiple: false,
      });
      if (selected) {
        await import_opml(selected as string);
      }
    },
    onSuccess: () => {
      query_client.invalidateQueries({ queryKey: ["feeds"] });
      query_client.invalidateQueries({ queryKey: ["entries"] });
    },
  });

  const export_mutation = useMutation({
    mutationFn: async () => {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const path = await save({
        filters: [{ name: "OPML", extensions: ["opml"] }],
      });
      if (path) {
        await export_opml(path, undefined);
      }
    },
  });

  const handle_import = () => import_mutation.mutate();
  const handle_export = () => export_mutation.mutate();

  return (
    <aside className="flex w-64 flex-col border-r border-border bg-sidebar">
      <div className="flex h-12 items-center gap-2 border-b border-border px-4">
        <Rss className="h-5 w-5 text-primary" />
        <span className="font-semibold text-sidebar-foreground">Mercury</span>
      </div>

      <nav className="flex gap-1 p-2">
        <button
          onClick={() => on_change_view("feeds")}
          className={`flex flex-1 items-center justify-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium ${
            view === "feeds"
              ? "bg-accent text-accent-foreground"
              : "text-sidebar-foreground hover:bg-accent"
          }`}
        >
          <Library className="h-3.5 w-3.5" />
          Feeds
        </button>
        <button
          onClick={() => on_change_view("tags")}
          className={`flex flex-1 items-center justify-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium ${
            view === "tags"
              ? "bg-accent text-accent-foreground"
              : "text-sidebar-foreground hover:bg-accent"
          }`}
        >
          <Tags className="h-3.5 w-3.5" />
          Tags
        </button>
      </nav>

      <div className="flex-1 overflow-hidden">
        {view === "feeds" ? (
          <FeedList
            selected_id={selected_feed_id}
            on_select={on_select_feed}
            on_add={on_add_feed}
            on_import={handle_import}
            on_export={handle_export}
          />
        ) : (
          <div className="flex h-full items-center justify-center p-4 text-center">
            <p className="text-xs text-muted-foreground">
              Tags will be available in Phase 5
            </p>
          </div>
        )}
      </div>

      <div className="border-t border-border p-2">
        <button
          onClick={on_open_settings}
          className="flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-sm text-sidebar-foreground hover:bg-accent"
        >
          <Settings className="h-4 w-4" />
          Settings
        </button>
      </div>
    </aside>
  );
}
