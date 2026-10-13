import type { Entry, EntryCursor, EntryPage } from "../../src/types";

export const entries: Entry[] = Array.from({ length: 237 }, (_, index) => ({
  id: index + 1, feed_id: 1, guid: `guid-${index}`, url: null,
  title: `Article ${index + 1}`, author: null,
  published_at: index % 5 === 0 ? null : `2026-10-${String((index * 7) % 11 + 1).padStart(2, "0")}T00:00:00Z`,
  summary: null, feed_content: null, feed_content_type: null,
  is_read: false, is_starred: false, is_deleted: false, created_at: "2026-10-01",
}));
entries.push({ ...entries[0], id: 300, feed_id: 2, title: "Other scope" });
export const calls: { command: string; args: Record<string, any> }[] = [];
export let fail_next = false;
export function fail_once() { fail_next = true; }
export let pending: (() => void) | undefined;
let hold_next = false;
export function hold_once() { hold_next = true; }
export function ordered(scope = 1) {
  return entries.filter((e) => e.feed_id === scope && !e.is_deleted).sort((a, b) => {
    if (a.published_at === b.published_at) return b.id - a.id;
    if (a.published_at === null) return 1;
    if (b.published_at === null) return -1;
    return a.published_at > b.published_at ? -1 : 1;
  });
}
export class Channel { onmessage = () => {}; }
export async function invoke(command: string, args: Record<string, any>) {
  calls.push({ command, args });
  if (command === "load_entries") {
    const scope = args.feedId ?? 1;
    if (hold_next) {
      hold_next = false;
      await new Promise<void>((resolve) => { pending = resolve; });
      pending = undefined;
    }
    if (fail_next) { fail_next = false; throw new Error("temporary failure"); }
    const all = ordered(scope);
    const cursor = args.cursor as EntryCursor | null;
    const after = cursor ? all.filter((e) => cursor.published_at === null
      ? e.published_at === null && e.id < cursor.id
      : e.published_at === null || e.published_at < cursor.published_at || (e.published_at === cursor.published_at && e.id < cursor.id)) : all;
    const page = after.slice(0, args.limit);
    const last = page.at(-1);
    return { entries: page, total: all.length,
      next_cursor: after.length > page.length && last ? { published_at: last.published_at, id: last.id } : null,
    } satisfies EntryPage;
  }
  if (command === "mark_read") {
    entries.filter((e) => args.entryIds.includes(e.id)).forEach((e) => { e.is_read = args.isRead; });
    return;
  }
  if (command === "delete_entry") {
    entries.find((e) => e.id === args.entryId)!.is_deleted = true;
    return;
  }
  throw new Error(`Unexpected IPC: ${command}`);
}
