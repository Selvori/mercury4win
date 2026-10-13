import type { Entry, EntryCursor, EntryDetail, EntryPage } from "../../src/types";
export const entries: Entry[] = Array.from({ length: 237 }, (_, index) => ({
  id: index + 1, feed_id: index % 3 === 0 ? 2 : 1, guid: `guid-${index}`, url: null,
  title: `Article ${index + 1}`, author: null, published_at: "2026-10-12",
  summary: "needle", feed_content: null, feed_content_type: null,
  is_read: false, is_starred: index % 7 !== 0, is_deleted: false, created_at: "2026-10-01",
}));
export const calls: { command: string; args: Record<string, any> }[] = [];
export let pending: (() => void) | undefined;
let hold_next = false;
export function hold_once() { hold_next = true; }
export class Channel { onmessage = () => {}; }
function tagged(entry: Entry, id: number) { return entry.id % (id === 1 ? 4 : id === 2 ? 6 : 1) === 0; }
export function filtered(args: Record<string, any>) {
  return entries.filter((e) => !e.is_deleted && (args.feedId == null || e.feed_id === args.feedId)
    && (!args.unreadOnly || !e.is_read) && (!args.starredOnly || e.is_starred)
    && (!args.search || (e.title + " " + e.summary).toLowerCase().includes(args.search.toLowerCase()))
    && (!args.tagIds?.length || (args.tagMode === "all" ? args.tagIds.every((id: number) => tagged(e, id)) : args.tagIds.some((id: number) => tagged(e,id)))))
    .sort((a,b) => b.id - a.id);
}
export async function invoke(command: string, args: Record<string, any> = {}) {
  calls.push({ command, args });
  if (command === "load_entries") {
    if (hold_next) { hold_next = false; await new Promise<void>((resolve) => { pending = resolve; }); pending = undefined; }
    const all = filtered(args), cursor = args.cursor as EntryCursor | null;
    const after = cursor ? all.filter((e) => e.id < cursor.id) : all;
    const page = after.slice(0, args.limit), last = page.at(-1);
    return { entries: page.map((e) => ({...e})), total: all.length,
      next_cursor: after.length > page.length && last ? { published_at: last.published_at, id: last.id } : null,
    } satisfies EntryPage;
  }
  if (command === "load_feeds") return [1,2].map((id) => ({ id, title: `Feed ${id}`, feed_url: `https://${id}.test`, unread_count: entries.filter((e) => e.feed_id === id && !e.is_deleted && !e.is_read).length, total_count: entries.filter((e) => e.feed_id === id && !e.is_deleted).length }));
  if (command === "list_tags") return Array.from({length:6}, (_,i) => ({ id:i+1,name:`Tag ${i+1}`,usage_count:entries.filter((e) => !e.is_deleted && tagged(e,i+1)).length,is_provisional:false }));
  if (command === "build_reader_content") return "<p>Article content</p>";
  if (command === "load_entry_detail") {
    if (args.entryId === 9999) throw new Error("Entry not found");
    return (window as Window & { detail_wire: EntryDetail }).detail_wire;
  }
  if (command === "get_note") return null;
  if (command === "get_agent_status") return { state: "idle" };
  if (command === "list_providers") return [];
  if (command === "mark_read") { entries.filter((e) => args.entryIds.includes(e.id)).forEach((e) => { e.is_read = args.isRead; }); return; }
  if (command === "mark_all_read_in_scope") { filtered(args).forEach((e) => { e.is_read = true; }); return; }
  if (command === "mark_starred") { entries.find((e) => e.id === args.entryId)!.is_starred = args.isStarred; return; }
  if (command === "delete_entry") { entries.find((e) => e.id === args.entryId)!.is_deleted = true; return; }
  if (command === "sync_feed") return {feed_id:args.id,new_entries:0,updated_entries:0,error:null};
  if (command === "sync_all_feeds") return [];
  throw new Error(`Unexpected IPC: ${command}`);
}
