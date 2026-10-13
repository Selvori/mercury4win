import fixture from "production-theme-fixture";
export class Channel<T> { onmessage = (_message: T) => {}; }
export const commands: { command: string; args: Record<string, unknown> }[] = [];
export const pending: { args: Record<string, unknown>; resolve: () => void }[] = [];
export const pending_documents: { resolve: () => void }[] = [];
let defer = false;
let defer_document = false;
export function defer_next_reader() { defer = true; }
export function defer_next_document() { defer_document = true; }
export function reader_html(args: Record<string, unknown>) {
  const key = `${args.themeId}:${args.fontFamily}:${args.fontSize}`;
  const html = fixture.rendered[key];
  if (!html) throw new Error(`Missing Rust fixture ${key}`);
  return html.replaceAll("Reader theme body.", `Reader article ${args.entryId}.`);
}
function document_for(entry: unknown) {
  const doc = JSON.parse(JSON.stringify(fixture.translation));
  doc.html = doc.html.replaceAll("Reader theme body.", `Reader article ${entry}.`);
  doc.source_hash = `source/${entry}`;
  return doc;
}
export function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  commands.push({ command, args });
  if (command === "build_reader_content") {
    const html = reader_html(args);
    if (defer) { defer = false; return new Promise<T>((resolve) => pending.push({ args, resolve: () => resolve(html as T) })); }
    return Promise.resolve(html as T);
  }
  if (command === "related_entries") return Promise.resolve([] as T);
  if (command === "get_agent_fingerprint") return Promise.resolve("theme-config" as T);
  if (command === "get_agent_profile") return Promise.resolve({ target_language: "en", concurrency_degree: 3 } as T);
  if (command === "get_agent_status") return Promise.resolve({ state: "idle" } as T);
  if (command === "load_translation_document") {
    const doc = document_for(args.entryId);
    if (defer_document) { defer_document = false; return new Promise<T>((resolve) => pending_documents.push({ resolve: () => resolve(doc as T) })); }
    return Promise.resolve(doc as T);
  }
  if (command === "load_translation_result") {
    const doc = document_for(args.entryId);
    return Promise.resolve({ source_hash: doc.source_hash, text: null, segments: doc.source_segments.map(() => "Saved 中文 translation"),
      segment_ids: doc.source_segments.map((s: { source_segment_id: string }) => s.source_segment_id), segment_errors: [], complete: true, model_name: "fixture" } as T);
  }
  throw new Error(`Unexpected native command (AI/network forbidden): ${command}`);
}
