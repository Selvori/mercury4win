// Native transport only; production TS IPC wrappers and components stay intact.
export class Channel<T> { onmessage = (_message: T) => {}; }
type Kind = "summary" | "translation" | "tagging";
export type Run = { id: string; kind: Kind; entry: number; channel: Channel<string>; started: boolean; terminal: boolean; resolve: (v: unknown) => void; reject: (e: Error) => void };
export const runs: Run[] = [];
export const commands: Array<{ command: string; args: Record<string, unknown> }> = [];
let cancel_failure = false;
export function fail_next_cancel() { cancel_failure = true; }
let pre_registration_cancel = false;
export function cancel_before_next_registration() { pre_registration_cancel = true; }
const slots = new Map<Kind, { active?: Run; waiting?: Run }>();
export function emit(run: Run, event: object) { run.channel.onmessage(JSON.stringify(event)); }
function lifecycle(run: Run, state: string) { emit(run, { type: "task_state", request_id: run.id, state }); }
function promote(kind: Kind) {
  const slot = slots.get(kind)!;
  if (!slot.active && slot.waiting) {
    slot.active = slot.waiting; slot.waiting = undefined;
    slot.active.started = true; lifecycle(slot.active, "running");
  }
}
export function finish(run: Run, error?: string) {
  if (run.terminal) return;
  run.terminal = true;
  const slot = slots.get(run.kind)!;
  if (slot.active === run) slot.active = undefined;
  if (slot.waiting === run) slot.waiting = undefined;
  if (error) run.reject(new Error(error));
  else {
    emit(run, { type: "done" });
    run.resolve(run.kind === "tagging" ? ["fixture"] : undefined);
  }
  promote(run.kind);
}
export function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (command === "load_translation_document") return Promise.resolve(translation_document(args.entryId as number) as T);
  commands.push({ command, args });
  if (command === "load_summary_result" || command === "load_translation_result") return Promise.resolve(null as T);
  if (command === "get_agent_fingerprint") return Promise.resolve("fixture-config" as T);
  if (command === "load_latest_summary" || command === "get_setting") return Promise.resolve(null as T);
  if (command === "get_agent_profile") return Promise.resolve({ target_language: "en" } as T);
  if (command === "get_agent_status") {
    const slot = slots.get(args.agentType as Kind);
    return Promise.resolve({ agent_type: args.agentType, state: slot?.active ? "running" : "idle", current_entry_id: slot?.active?.entry ?? null, current_request_id: slot?.active?.id ?? null, waiting_entry_id: slot?.waiting?.entry ?? null, waiting_request_id: slot?.waiting?.id ?? null, queue_depth: slot?.waiting ? 1 : 0 } as T);
  }
  if (command === "load_entry_detail") return Promise.resolve({ entry: { id: args.entryId }, tags: [], has_note: false } as T);
  if (command.startsWith("run_")) {
    const kind = command.slice(4) as Kind;
    if (typeof args.requestId !== "string" || !args.requestId) throw new Error("request ID missing from production wrapper");
    if (!(args.onEvent instanceof Channel)) throw new Error("Channel missing from production wrapper");
    if (pre_registration_cancel) {
      pre_registration_cancel = false;
      return Promise.reject("Agent task cancelled: abandoned before registration");
    }
    return new Promise<T>((resolve, reject) => {
      const run: Run = { id: args.requestId as string, kind, entry: args.entryId as number, channel: args.onEvent as Channel<string>, started: false, terminal: false, resolve: resolve as (v: unknown) => void, reject };
      runs.push(run);
      const slot = slots.get(kind) ?? {}; slots.set(kind, slot);
      if (!slot.active) { slot.active = run; run.started = true; lifecycle(run, "running"); }
      else {
        if (slot.waiting) { lifecycle(slot.waiting, "cancelled"); finish(slot.waiting, "Agent task cancelled: replaced_by_latest"); }
        slot.waiting = run; lifecycle(run, "waiting");
      }
    });
  }
  if (command === "cancel_agent_task" || command === "abandon_agent_waiting") {
    if (command === "cancel_agent_task" && cancel_failure) {
      cancel_failure = false;
      return Promise.reject(new Error("fixture cancel transport failure"));
    }
    const run = runs.find((r) => r.id === args.requestId && r.kind === args.taskType && !r.terminal);
    if (run && (command === "cancel_agent_task" || !run.started)) {
      lifecycle(run, "cancelled"); finish(run, "Agent task cancelled: fixture");
    }
    return Promise.resolve(undefined as T);
  }
  throw new Error(`Unexpected native command: ${command}`);
}

export function translation_document(entry: number) {
  const source_segments = [0, 1].map((i) => ({ source_segment_id: `fixture_${entry}_${i}`, order_index: i, segment_type: "p", source_html: `<p>source ${entry}/${i}</p>`, source_text: `source ${entry}/${i}` }));
  return { source_hash: "source", segmenter_version: "reader-html-block-v2", source_segments, html: `<html><head></head><body><article class="reader">${source_segments.map((s) => `<p data-source-segment-id="${s.source_segment_id}">${s.source_text}</p>`).join("")}</article></body></html>`, result: null };
}
