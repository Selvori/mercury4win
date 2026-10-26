// UI contract tests replay production Rust DTOs. Search semantics are tested in
// SQLite, not reimplemented here. This is deliberately not native IPC evidence.
import pages from "search-fixtures";
import * as existing from "../localization/mock_transport";
export { Channel } from "../localization/mock_transport";
import type { EntryPage, SmartView } from "../../src/types";
export const calls: { command: string; args: any }[] = [];
export let fail_next = false;
export function fail() { fail_next = true; }
export async function invoke(command: string, args: any = {}) {
  calls.push({ command, args: structuredClone(args) });
  if (command === "load_entries") {
    if (fail_next) { fail_next = false; throw new Error("synthetic query failure"); }
    const key = (args.filter.search?.trim() ?? "") + (args.cursor ? "::next" : "");
    return structuredClone((pages as Record<string, EntryPage>)[key] ?? { entries: [], total: 0, next_cursor: null, search_info: [] });
  }
  if (command === "load_feeds") return [1, 2].map((id) => ({ id, title: `Source ${id}`, feed_url: `https://${id}.test`, unread_count: 1, total_count: 1 }));
  if (command === "list_tags") return [{ id: 1, name: "Rust", usage_count: 2 }, { id: 2, name: "SQLite", usage_count: 2 }];
  const views: SmartView[] = JSON.parse(localStorage.getItem("synthetic_search_views") ?? "[]");
  if (command === "list_smart_views") return views;
  if (command === "save_smart_view") {
    if (views.some((v) => v.name.toLowerCase() === args.name.trim().toLowerCase() && v.id !== args.id)) throw new Error("Duplicate");
    const id = args.id ?? Math.max(0, ...views.map((v) => v.id)) + 1;
    const view = { id, name: args.name.trim(), filter: args.filter, updated_at: "2026-10-26" };
    localStorage.setItem("synthetic_search_views", JSON.stringify([...views.filter((v) => v.id !== id), view]));
    return id;
  }
  if (command === "delete_smart_view") { localStorage.setItem("synthetic_search_views", JSON.stringify(views.filter((v) => v.id !== args.id))); return; }
  if (command === "rebuild_search_index" || command === "mark_all_read_in_scope") return;
  if (command === "build_reader_content") return "<html><body><p>Production UI synthetic reader</p></body></html>";
  if (command === "get_note") return { entry_id: args.entryId, markdown: "笔记独有回忆 async_fn 与双字方案。", updated_at: "2026-10-26" };
  if (command === "related_entries") return [];
  if (command === "load_entry_detail") return { entry: (pages as any)["方案"].entries.find((e: any) => e.id === args.entryId), tags: [], has_note: true, feed_title: "Source" };
  if (command === "save_note") return { entry_id: args.entryId, markdown: args.markdown, updated_at: "2026-10-26" };
  return existing.invoke(command, args);
}
