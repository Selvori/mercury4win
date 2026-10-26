export { Channel } from "../accessibility/mock_transport";
import * as existing from "../accessibility/mock_transport";
export const calls: { command: string; args: any }[] = [];
export const dialogs: any[] = [];
export async function open(options: any) { dialogs.push(options); return null; }
export async function save(options: any) { dialogs.push(options); return null; }
export const ask = existing.ask;
export async function invoke(command: string, args: any = {}) {
  calls.push({ command, args });
  if (command === "load_entries") return { entries: [{ id: 1, feed_id: 1, title: "Synthetic article", published_at: "2026-10-13T12:00:00Z", is_read: false, is_starred: false }], total: 1, next_cursor: null };
  if (command === "get_note" || command === "load_latest_summary" || command === "load_custom_template") return null;
  if (command === "preview_digest") return { content: `synthetic preview ${args.options.locale}`, format: "markdown", missing: [] };
  return existing.invoke(command, args);
}
