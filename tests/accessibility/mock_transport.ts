import * as tags from "../tag_system/mock_transport";
export { Channel } from "../tag_system/mock_transport";
export const calls: { command: string; args: any }[] = [];
export const dialogs: any[] = [];
export let reader_fails = true;
export let sync_finish: (() => void) | undefined;
export function reset_tags() { tags.reset(); }
export function configure_reader(fails: boolean) { reader_fails = fails; }
export async function ask(options: any) { dialogs.push(options); return true; }
export async function save(options: any) { dialogs.push(options); return "C:/fixture/中文.opml"; }
export async function open(options: any) { dialogs.push(options); throw new Error("fixture file dialog failed"); }
export async function invoke(command: string, args: any = {}) {
  calls.push({ command, args });
  if (command === "build_reader_content") {
    if (reader_fails) throw new Error("fixture HTTP 503");
    return '<html><body><p>Article</p><a href="https://fixture.test">Link</a><input><div contenteditable="true">Edit</div></body></html>';
  }
  if (command === "sync_all_feeds_stream") {
    args.onProgress.onmessage({ status: "start", total: 1, current: 0 });
    return new Promise<void>((resolve) => { sync_finish = resolve; });
  }
  if (command === "delete_feed" || command === "export_opml") return;
  if (command === "get_note") return null;
  if (command === "load_entries") return { entries: [{ id: 1, title: "Keyboard article", is_read: false, is_starred: false }], total: 1, next_cursor: null };
  if (command === "add_feed") throw new Error("fixture invalid Feed");
  if (command === "local_tag_suggestions" || command === "list_tags") return [];
  return tags.invoke(command, args);
}
