import fixture from "production-translation-fixture";
import type { AgentResult, TranslationDocument } from "../../src/types/agent";
export class Channel<T> { onmessage = (_message: T) => {}; }
export const commands: { command: string; args: Record<string, unknown> }[] = [];
export const stored = new Map<number, AgentResult>();
export const runs: { entry: number; id: string; channel: Channel<string>; resolve: () => void; reject: (e: Error) => void; terminal: boolean }[] = [];
export let fail_document = false;
export const delayed_documents = new Map<number, Promise<TranslationDocument>>();
export const delayed_readers = new Map<number, Promise<string>>();
export let delayed_profile: Promise<{ target_language: string; concurrency_degree: number }> | null = null;
export function delay_profile(value: typeof delayed_profile) { delayed_profile = value; }
export function fail_next_document(value: boolean) { fail_document = value; }
export function document_for(entry: number): TranslationDocument {
  const doc = JSON.parse(JSON.stringify(fixture)) as TranslationDocument;
  for (const source of doc.source_segments) {
    const old = source.source_segment_id;
    source.source_segment_id = `${entry}_${old}`;
    doc.html = doc.html.replaceAll(old, source.source_segment_id);
  }
  doc.source_hash = `source/${entry}`;
  doc.result = stored.get(entry) ?? null;
  return doc;
}
export function seed(entry: number, values: (string | null)[]) {
  stored.set(entry, { source_hash: `source/${entry}`, text: null, segments: values,
    segment_ids: document_for(entry).source_segments.map((s) => s.source_segment_id),
    segment_errors: values.map((s) => s ? null : "fixture missing"), complete: values.every(Boolean), model_name: "fixture" });
}
export function emit(run: typeof runs[number], event: object) { run.channel.onmessage(JSON.stringify(event)); }
export function finish(run: typeof runs[number], error?: string) {
  run.terminal = true;
  if (error) run.reject(new Error(error)); else run.resolve();
}
export function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  commands.push({ command, args }); const entry = args.entryId as number;
  if (command === "get_agent_fingerprint") return Promise.resolve("fixture-config" as T);
  if (command === "load_latest_summary" || command === "get_setting") return Promise.resolve(null as T);
  if (command === "get_agent_profile") return (delayed_profile ?? Promise.resolve({ target_language: "en", concurrency_degree: 3 })) as Promise<T>;
  if (command === "load_translation_result") return Promise.resolve((stored.get(entry) ?? null) as T);
  if (command === "load_translation_document") return fail_document ? Promise.reject(new Error("fixture fetch failed")) : delayed_documents.get(entry) as Promise<T> ?? Promise.resolve(document_for(entry) as T);
  if (command === "build_reader_content") return delayed_readers.get(entry) as Promise<T> ?? Promise.resolve(document_for(entry).html.replace("</head>", "<style id='current-reader-theme'>body{color:white;background:black;font-family:Serif;font-size:22px}</style></head>") as T);
  if (command === "get_agent_status") {
    const active = runs.find((r) => !r.terminal);
    return Promise.resolve({ state: active ? "running" : "idle", current_entry_id: active?.entry ?? null, current_request_id: active?.id ?? null } as T);
  }
  if (command === "run_translation") return new Promise<T>((resolve, reject) => {
    const run = { entry, id: args.requestId as string, channel: args.onEvent as Channel<string>, resolve: () => resolve(undefined as T), reject, terminal: false };
    runs.push(run); emit(run, { type: "task_state", request_id: run.id, state: "running" });
  });
  if (command === "cancel_agent_task") { const run = runs.find((r) => r.id === args.requestId)!; finish(run, "Agent task cancelled: fixture"); return Promise.resolve(undefined as T); }
  if (command === "abandon_agent_waiting") return Promise.resolve(undefined as T);
  throw new Error(`Unexpected native command: ${command}`);
}
