// Production SQLite DTOs; this mock tests UI transport and ownership, not analysis semantics.
import fixture from "topic-fixtures";
import * as existing from "../localization/mock_transport";
export class Channel<T> { onmessage: (value: T) => void = () => {}; }
export const calls: { command: string; args: any }[] = [];
let data = structuredClone(fixture.before);
let mode = "ok";
let pending: { resolve: (value: number | null) => void; reject: (e: Error) => void; channel: Channel<string> } | null = null;
export function reset() { data = structuredClone(fixture.before); mode = "ok"; calls.length = 0; }
export function set_mode(value: string) { mode = value; }
export function complete() { if (pending) { pending.channel.onmessage(JSON.stringify({ type: "phase", phase: "validating" })); pending.resolve(2); pending = null; } }
export async function invoke(command: string, args: any = {}) {
  calls.push({ command, args });
  if (command === "list_topics") return [structuredClone(data.topic)];
  if (command === "load_topic") { if (mode === "load_error") throw new Error("controlled load error"); return structuredClone(data); }
  if (command === "topic_model_info") return { routes: mode === "no_model" ? [] : [{ provider_name: "Fixture", model_name: "controlled-model" }] };
  if (command === "load_entries") return structuredClone(fixture.entries);
  if (command === "save_topic_notes") { data.topic.notes = args.notes; return; }
  if (command === "save_topic") { data.topic = { ...data.topic, ...args, rule_name: args.ruleName, id: 1 }; return 1; }
  if (command === "delete_topic") return;
  if (command === "list_smart_views") return [{ id: 1, name: "GPU scope", filter: { search: "GPU", recent_days: 7 } }];
  if (command === "update_topic_members") {
    if (args.remove) data.input.sources = data.input.sources.filter((s: any) => !args.entryIds.includes(s.entry_id));
    else if (args.groupName !== null) for (const source of data.input.sources) if (args.entryIds.includes(source.entry_id)) source.group_name = args.groupName || "auto:restored";
    data.changed = true; return;
  }
  if (command === "generate_topic_brief") {
    if (mode === "failure") throw new Error("Invalid citation article ID: 999");
    if (mode === "delayed") return new Promise<number | null>((resolve, reject) => { pending = { resolve, reject, channel: args.onEvent }; args.onEvent.onmessage(JSON.stringify({ type: "phase", phase: "comparing" })); });
    const notes = data.topic.notes; data = structuredClone(fixture.after); data.topic.notes = notes; return 2;
  }
  if (command === "cancel_agent_task") { pending?.reject(new Error("Agent task cancelled: user_cancelled")); pending = null; return; }
  if (command === "build_reader_content") { throw new Error("controlled body fetch failure"); }
  if (command === "load_entry_detail") return { entry: fixture.entries.entries.find((e: any) => e.id === args.entryId), tags: [], feed_title: "Fixture", has_note: false };
  return existing.invoke(command, args);
}
