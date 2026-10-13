export class Channel<T> { onmessage = (_message: T) => {}; }
type Result = { source_hash: string; text: string | null; segments: (string | null)[]; complete: boolean; model_name: string };
export function deferred<T>() { let resolve!: (v: T) => void; let reject!: (e: Error) => void; const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; }); return { promise, resolve, reject }; }
export const stored = new Map<string, Result>();
export const commands: { command: string; args: Record<string, unknown> }[] = [];
export const source = new Map<number, string>();
export const delayed = new Map<number, ReturnType<typeof deferred<Result | null>>>();
export const reader_delayed = new Map<number, ReturnType<typeof deferred<string>>>();
export const runs: { kind: string; entry: number; key: string; id: string; channel: Channel<string>; pending: ReturnType<typeof deferred<void>>; terminal: boolean; text: string; segments: (string | null)[] }[] = [];
export const key = (kind: string, entry: number, detail = "medium") => `${kind}/${entry}/en/${kind === "summary" ? detail : "default"}/${source.get(entry) ?? "source"}`;
export function seed(kind: string, entry: number, text: string, detail = "medium", complete = true) {
  stored.set(key(kind, entry, detail), { source_hash: source.get(entry) ?? "source", text: kind === "summary" ? text : null, segments: kind === "summary" ? [] : [text, complete ? "second saved" : null], complete, model_name: "fixture", segment_ids: translation_document(entry).source_segments.map((s) => s.source_segment_id) });
}
export function emit(run: typeof runs[number], event: Record<string, unknown>) {
  if (event.type === "chunk") run.text += event.content;
  if (event.type === "segment") run.segments[event.index as number] = event.text as string;
  run.channel.onmessage(JSON.stringify(event));
}
export function finish(run: typeof runs[number], error?: string) {
  run.terminal = true;
  if (error) run.pending.reject(new Error(error));
  else {
    stored.set(run.key, { source_hash: "source", text: run.kind === "summary" ? run.text : null, segments: run.segments, complete: true, model_name: "fixture", segment_ids: translation_document(run.entry).source_segments.map((s) => s.source_segment_id) });
    emit(run, { type: "done" }); run.pending.resolve();
  }
}
export function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (command === "load_translation_document") return Promise.resolve(translation_document(args.entryId as number) as T);
  commands.push({ command, args }); const entry = args.entryId as number;
  if (command === "get_agent_fingerprint") return Promise.resolve("fixture-config" as T);
  if (command === "load_latest_summary" || command === "get_setting") return Promise.resolve(null as T);
  if (command === "get_agent_profile") return Promise.resolve({ target_language: "en" } as T);
  if (command === "get_agent_status") { const active = runs.find((r) => r.kind === args.agentType && !r.terminal); return Promise.resolve({ state: active ? "running" : "idle", current_entry_id: active?.entry ?? null, current_request_id: active?.id ?? null } as T); }
  if (command === "build_reader_content") return (reader_delayed.get(entry)?.promise ?? Promise.resolve("<p>reader</p>")) as Promise<T>;
  if (command === "load_summary_result" || command === "load_translation_result") return (delayed.get(entry)?.promise ?? Promise.resolve(stored.get(key(command === "load_summary_result" ? "summary" : "translation", entry, args.detailLevel as string)) ?? null)) as Promise<T>;
  if (command === "run_summary" || command === "run_translation") {
    const kind = command.slice(4); const pending = deferred<void>();
    runs.push({ kind, entry, key: key(kind, entry, args.detailLevel as string), id: args.requestId as string, channel: args.onEvent as Channel<string>, pending, terminal: false, text: "", segments: [] });
    emit(runs.at(-1)!, { type: "task_state", request_id: args.requestId, state: "running" }); return pending.promise as Promise<T>;
  }
  if (command === "cancel_agent_task") { const run = runs.find((r) => r.id === args.requestId)!; finish(run, "Agent task cancelled: fixture"); return Promise.resolve(undefined as T); }
  if (command === "abandon_agent_waiting") return Promise.resolve(undefined as T);
  throw new Error(`Unexpected native command: ${command}`);
}

export function translation_document(entry: number) {
  const source_segments = [0, 1].map((i) => ({ source_segment_id: `fixture_${entry}_${i}`, order_index: i, segment_type: "p", source_html: `<p>source ${entry}/${i}</p>`, source_text: `source ${entry}/${i}` }));
  return { source_hash: source.get(entry) ?? "source", segmenter_version: "reader-html-block-v2", source_segments, html: `<html><head></head><body><article class="reader">${source_segments.map((s) => `<p data-source-segment-id="${s.source_segment_id}">${s.source_text}</p>`).join("")}</article></body></html>`, result: null };
}
