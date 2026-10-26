// mercury4win/src/components/layout/sidebar.tsx
// Left sidebar with feed list and navigation

import { useTranslation } from "react-i18next";
import { Rss, Library, Tags, Settings, Star, Search, FolderSearch } from "lucide-react";
import { FeedList } from "@/features/feed/components/feed_list";
import { TagLibrary } from "@/features/tags/components/tag_library";
import { ThemeToggle } from "./theme_toggle";

import { SmartViews } from "@/features/entry/components/smart_views";
import type { SmartView } from "@/types";

interface Props {
  topics_active?: boolean;
  on_topics?: () => void;
  selected_view_id?: number;
  on_select_view?: (view: SmartView) => void;
  on_search?: () => void;
  search_active?: boolean;
  selected_feed_id: number | null | undefined;
  on_select_feed: (id: number | null) => void;
  on_add_feed: () => void;
  on_import_opml: () => void;
  on_export_opml: () => void;
  on_open_settings: () => void;
  view: "feeds" | "tags";
  on_change_view: (view: "feeds" | "tags") => void;
  selected_tag_ids: number[];
  starred_only: boolean;
  on_select_starred: () => void;
  on_select_tag: (tag_id: number) => void;
  on_tags_changed?: (removed: number[], target?: number) => void;
}

export function Sidebar({
  topics_active, on_topics,
  selected_view_id, on_select_view, on_search, search_active = false,
  selected_feed_id,
  on_select_feed,
  on_add_feed,
  on_import_opml,
  on_export_opml,
  on_open_settings,
  view,
  on_change_view,
  selected_tag_ids,
  starred_only,
  on_select_starred,
  on_select_tag,
  on_tags_changed,
}: Props) {
  const { t } = useTranslation();

  return (
    <aside className="flex h-full flex-col border-r border-border bg-sidebar text-sidebar-foreground">
      <div className="flex h-12 items-center gap-2 border-b border-border px-4">
        <Rss className="h-5 w-5 text-primary" />
        <span className="font-semibold text-sidebar-foreground">Mercury</span>
      </div>

      <nav className="flex gap-1 p-2">
        <button
          onClick={() => on_change_view("feeds")}
          aria-pressed={view === "feeds"}
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
          aria-pressed={view === "tags"}
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
      {on_topics && <button onClick={on_topics} aria-pressed={topics_active}
        className={`mx-2 mb-1 flex items-center gap-2 rounded-md px-3 py-2 text-sm ${topics_active ? "bg-accent text-accent-foreground font-medium" : "hover:bg-accent/50"}`}>
        <FolderSearch className="h-4 w-4" />{t("topics.title")}
      </button>}

      {on_search && <button onClick={on_search} aria-pressed={search_active} aria-expanded={search_active}
        className={`mx-2 mb-1 flex items-center gap-2 rounded-md px-3 py-2 text-sm ${search_active ? "bg-accent text-accent-foreground font-medium" : "hover:bg-accent/50"}`}>
        <Search className="h-4 w-4" />{t("search.title")}
      </button>}
      <button onClick={on_select_starred} aria-pressed={starred_only}
        className={`mx-2 mb-2 flex items-center gap-2 rounded-md px-3 py-1.5 text-sm ${starred_only ? "bg-accent" : "hover:bg-accent"}`}>
        <Star className="h-4 w-4" />{t("sidebar.starred")}
      </button>
      {on_select_view && <SmartViews selected_id={selected_view_id} on_select={on_select_view} />}
      <div className="min-h-0 flex-1 overflow-hidden">
        {view === "feeds" ? (
          <FeedList
            selected_id={search_active || starred_only || selected_view_id != null ? undefined : selected_feed_id}
            on_select={on_select_feed}
            on_add={on_add_feed}
            on_import={on_import_opml}
            on_export={on_export_opml}
          />
        ) : (
          <TagLibrary
            on_select_tag={on_select_tag}
            selected_tag_ids={selected_tag_ids}
            on_tags_changed={on_tags_changed}
          />
        )}
      </div>

      <div className="border-t border-border p-2">
        <div className="flex items-center gap-1">
          <button
            onClick={on_open_settings}
            className="flex flex-1 items-center gap-2 rounded-md px-3 py-1.5 text-sm text-sidebar-foreground hover:bg-accent"
          >
            <Settings className="h-4 w-4" />
            {t("sidebar.settings")}
          </button>
          <ThemeToggle />
        </div>
      </div>
    </aside>
  );
}
