import type { Entry } from "../../src/types";
export const entries: Entry[] = [1, 2, 3].map((id) => ({ id, feed_id: 1, title: `Article ${id}`, url: `https://fixture.test/${id}`, guid: null, author: null, published_at: null, summary: null, feed_content: null, feed_content_type: null, is_read: false, is_starred: false, is_deleted: false, created_at: "2026-10-13" }));
export const commands: { command: string; args: Record<string, any> }[] = [];
export let fail_defaults = false;
export let fail_preview = false;
export let fail_export = false;
export let dialog_result: string | null = "C:/fixture/中文.md";
export let dialog_failure = false;
export const dialogs: any[] = [];
export let pending_preview: (() => void) | null = null;
export let hold_preview = false;
export function configure(values: { defaults?: boolean; preview?: boolean; export?: boolean; path?: string | null; dialog?: boolean; hold?: boolean }) {
  if (values.defaults !== undefined) fail_defaults = values.defaults;
  if (values.preview !== undefined) fail_preview = values.preview;
  if (values.export !== undefined) fail_export = values.export;
  if (values.path !== undefined) dialog_result = values.path;
  if (values.dialog !== undefined) dialog_failure = values.dialog;
  if (values.hold !== undefined) hold_preview = values.hold;
}
export async function save(options: any) { dialogs.push(options); if (dialog_failure) throw new Error("dialog failed"); return dialog_result; }
export class Channel { onmessage = () => {}; }
export async function invoke(command: string, args: Record<string, any>) {
  commands.push({ command, args });
  if (command === "get_note") { if (fail_defaults) throw new Error("saved lookup failed"); return args.entryId === 1 ? { entry_id: 1, markdown: "Saved note", updated_at: "today" } : null; }
  if (command === "load_latest_summary") { if (fail_defaults) throw new Error("saved lookup failed"); return args.entryId === 1 ? { target_language: "zh", detail_level: "brief", configuration_fingerprint: null, result: { complete: true, text: "Saved summary" } } : null; }
  if (command === "preview_digest") {
    if (hold_preview) { hold_preview = false; await new Promise<void>((resolve) => { pending_preview = resolve; }); }
    if (fail_preview) throw new Error("preview failed");
    return { content: `preview:${args.entryIds.join(",")}:${args.templateName}:${JSON.stringify(args.options)} <script>literal</script>`, format: args.templateName === "single_text" ? "text" : "markdown",
      missing: args.options.include_body ? [{ entry_id: args.entryIds[0], title: "Article", field: "body" }] : [] };
  }
  if (command === "export_digest" || command === "export_multi_digest") { if (fail_export) throw new Error("write failed"); return; }
  if (command === "load_entries") return { entries: args.feedId === 2 ? [entries[2]] : entries, total: 200, next_cursor: { id: 3, published_at: null } };
  if (command === "build_reader_content") return "<p>Article body</p>";
  throw new Error(`Unexpected IPC: ${command}`);
}
