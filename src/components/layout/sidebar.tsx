// mercury4win-linux/src/components/layout/sidebar.tsx
// Left sidebar with feed list and navigation

import { useTranslation } from "react-i18next";
import { Rss, Library, Tags, Settings } from "lucide-react";
import { FeedList } from "@/features/feed/components/feed_list";
import { TagLibrary } from "@/features/tags/components/tag_library";

interface Props {
  selected_feed_id: number | null;
  on_select_feed: (id: number) => void;
  on_add_feed: () => void;
  on_import_opml: () => void;
  on_export_opml: () => void;
  on_open_settings: () => void;
  view: "feeds" | "tags";
  on_change_view: (view: "feeds" | "tags") => void;
  selected_tag_id: number | null;
  on_select_tag: (tag_id: number) => void;
}

export function Sidebar({
  selected_feed_id,
  on_select_feed,
  on_add_feed,
  on_import_opml,
  on_export_opml,
  on_open_settings,
  view,
  on_change_view,
  selected_tag_id,
  on_select_tag,
}: Props) {
  const { t } = useTranslation();

  return (
    <aside className="flex h-full flex-col border-r border-border bg-sidebar">
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
          {t("sidebar.feeds")}
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
          {t("sidebar.tags")}
        </button>
      </nav>

      <div className="flex-1 overflow-hidden">
        {view === "feeds" ? (
          <FeedList
            selected_id={selected_feed_id}
            on_select={on_select_feed}
            on_add={on_add_feed}
            on_import={on_import_opml}
            on_export={on_export_opml}
          />
        ) : (
          <TagLibrary
            on_select_tag={on_select_tag}
            selected_tag_id={selected_tag_id}
          />
        )}
      </div>

      <div className="border-t border-border p-2">
        <button
          onClick={on_open_settings}
          className="flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-sm text-sidebar-foreground hover:bg-accent"
        >
          <Settings className="h-4 w-4" />
          {t("sidebar.settings")}
        </button>
      </div>
    </aside>
  );
}
