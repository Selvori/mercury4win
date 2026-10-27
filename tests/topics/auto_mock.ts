import fixture from "auto-fixtures";
import * as existing from "../localization/mock_transport";
export class Channel<T> { onmessage: (value: T) => void = () => {}; }
export const calls: { command: string; args: any }[] = [];
let data = structuredClone(fixture.before);
let mode = "ok";
let pending: { resolve: (value: number | null) => void; reject: (e: Error) => void } | null = null;
let progress_channel: Channel<string> | null = null;
export function reset() { data = structuredClone(fixture.before); mode = "ok"; calls.length = 0; }
export function set_mode(value: string) { mode = value; }
export function progress(event: object) { progress_channel?.onmessage(JSON.stringify(event)); }
export function repeat_reference() { const claim = structuredClone(data.history[0].result.claims[0]); claim.text = "Additional connected paragraph for repeated-source coverage."; data.history[0].result.claims.push(claim); return claim.citations[0].entry_id; }
export function complete() { const topic = data.topic; data = structuredClone(fixture.after); data.topic = { ...data.topic, name: topic.name, notes: topic.notes }; pending?.resolve(data.history[0].id); pending = null; }
export async function invoke(command: string, args: any = {}) {
  calls.push({ command, args });
  if (command === "list_topics") return [structuredClone(data.topic)];
  if (command === "load_topic") return structuredClone(data);
  if (command === "topic_status") return structuredClone(data);
  if (command === "topic_model_info") return { routes: mode === "no_model" ? [] : [{ provider_name: "Controlled", model_name: "controlled-auto" }] };
  if (command === "mark_topic_read") { data.topic.read_brief_id = args.briefId; data.topic.unread = false; return; }
  if (command === "topic_feedback") { data.automatic.excluded = args.reason ? [...data.automatic.excluded, args.entryId] : data.automatic.excluded.filter((v: number) => v !== args.entryId); data.input.sources = data.input.sources.filter((s: any) => s.entry_id !== args.entryId); return; }
  if (command === "save_topic_notes") { data.topic.notes = args.notes; return; }
  if (command === "save_topic") { data.topic = { ...data.topic, ...args, id: 1, input_budget: args.inputBudget ?? 24000, rule_name: args.ruleName }; return 1; }
  if (command === "list_smart_views") return [];
  if (command === "load_entries") return { entries: [], total: 0, next_cursor: null };
  if (command === "generate_topic_brief") {
    progress_channel = args.onEvent;
    progress({ type: "batch", batch: 1, max_batches: 3 });
    progress({ type: "coverage", count: 7, pending: 137 });
    args.onEvent.onmessage(JSON.stringify({ type: "phase", phase: "selecting" }));
    if (mode === "no_model") throw new Error("Configure a model in Settings");
    if (mode === "failure") throw new Error("Invalid citation article ID: 999");
    if (mode === "delayed") return new Promise<number | null>((resolve, reject) => { pending = { resolve, reject }; });
    if (mode.startsWith("no_information")) { data.changed = mode === "no_information_remaining"; data.automatic.pending = data.changed ? 13 : 0; data.topic.last_checked_at = "2026-10-26 10:00:00"; data.runs.unshift({ request_id: args.requestId, status: "no_information", error: null, created_at: "2026-10-26 10:00:00" }); if (mode !== "no_information_late") args.onEvent.onmessage(JSON.stringify({ type: "phase", phase: "no_information" })); return null; }
    const topic = data.topic; data = structuredClone(fixture.after); data.topic = { ...data.topic, name: topic.name, notes: topic.notes }; args.onEvent.onmessage(JSON.stringify({ type: "phase", phase: "summarizing" })); return data.history[0].id;
  }
  if (command === "cancel_agent_task") { pending?.reject(new Error("Agent task cancelled: user_cancelled")); pending = null; return; }
  return existing.invoke(command, args);
}
