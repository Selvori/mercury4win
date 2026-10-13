import type { EntryDetail } from "../../src/types";
// Memory-only IPC doubles: never open a user database or contact an AI service.
export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

type Stream = { onmessage: (message: string) => void };
type Note = { entry_id: number; markdown: string };
export const notes = new Map<number, Note>([[1, { entry_id: 1, markdown: "saved A" }], [2, { entry_id: 2, markdown: "saved B" }]]);
export const saves: Array<{ entry_id: number; markdown: string; task: ReturnType<typeof deferred<Note>> }> = [];
export const summaries: Array<{ entry_id: number; channel: Stream; task: ReturnType<typeof deferred<void>> }> = [];
export const translations: typeof summaries = [];
export const suggestions: Array<{ entry_id: number; task: ReturnType<typeof deferred<string[]>> }> = [];
export const tag_writes: Array<{ entry_id: number; name: string; task: ReturnType<typeof deferred<object>> }> = [];
export const tag_deletes: Array<{ entry_id: number; tag_id: number; task: ReturnType<typeof deferred<void>> }> = [];

export const get_note = async (entry_id: number) => notes.get(entry_id) ?? null;
export function save_note(entry_id: number, markdown: string) {
  const task = deferred<Note>();
  saves.push({ entry_id, markdown, task });
  return task.promise.then((note) => { notes.set(entry_id, note); return note; });
}
export const reader_responses = new Map<number, Array<string | Error>>();
export const reader_requests: number[] = [];
export const build_reader_content = async (entry_id: number) => {
  reader_requests.push(entry_id);
  const responses = reader_responses.get(entry_id);
  const response = responses?.length ? (responses.length > 1 ? responses.shift()! : responses[0]) : "<p>mock article</p>";
  if (response instanceof Error) throw response;
  return response;
};
export const get_agent_profile = async () => ({ target_language: "en" });
export const get_agent_status = async () => ({ state: "idle", current_entry_id: null, current_request_id: null, waiting_entry_id: null, waiting_request_id: null, queue_depth: 0 });
export const abandoned: Array<{ task_type: string; request_id: string }> = [];
export const cancelled: typeof abandoned = [];
export async function abandon_agent_waiting(task_type: string, request_id: string) {
  abandoned.push({ task_type, request_id });
}
export async function cancel_agent_task(task_type: string, request_id: string) {
  cancelled.push({ task_type, request_id });
}
const summary_results = new Map<string, object>();
const translation_results = new Map<number, object>();
export function run_summary(entry_id: number, _language: string, level: string, channel: Stream) {
  const task = deferred<void>(); let text = ""; let failed = false;
  const callback = channel.onmessage;
  channel.onmessage = (chunk) => { const message = JSON.parse(chunk); if (message.type === "chunk") text += message.content; if (message.type === "error") failed = true; callback(chunk); };
  summaries.push({ entry_id, channel, task });
  return task.promise.then(() => { if (text && !failed) summary_results.set(`${entry_id}/${level}`, { text, segments: [], complete: true }); });
}
export function run_translation(entry_id: number, _language: string, channel: Stream) {
  const task = deferred<void>(); const segments: (string | null)[] = []; let failed = false;
  const callback = channel.onmessage;
  channel.onmessage = (chunk) => { const message = JSON.parse(chunk); if (message.type === "segment") segments[message.index] = message.text; if (message.type === "error") failed = true; callback(chunk); };
  translations.push({ entry_id, channel, task });
  return task.promise.then(() => { if (segments.length && !failed) translation_results.set(entry_id, { source_hash: "source", text: null, segments, complete: true, segment_ids: translation_document(entry_id).source_segments.map((s) => s.source_segment_id) }); });
}
export function run_tagging(entry_id: number) {
  const task = deferred<string[]>();
  suggestions.push({ entry_id, task });
  return task.promise;
}
export const load_entry_detail = async (entry_id: number) => ({
  entry: { id: entry_id, feed_id: 1, guid: null, url: null, title: null, author: null,
    published_at: null, summary: null, feed_content: null, feed_content_type: null,
    is_read: false, is_starred: false, is_deleted: false, created_at: "2026-10-12" },
  feed_title: null, has_note: notes.has(entry_id),
  tags: [{ tag_id: entry_id, name: `existing ${entry_id}`, source: "manual" }],
} satisfies EntryDetail);
export function add_tag(entry_id: number, name: string) {
  const task = deferred<object>();
  tag_writes.push({ entry_id, name, task });
  return task.promise;
}
export function remove_tag(entry_id: number, tag_id: number) {
  const task = deferred<void>();
  tag_deletes.push({ entry_id, tag_id, task });
  return task.promise;
}

export const load_summary_result = async (entry_id: number, _language: string, level: string) => summary_results.get(`${entry_id}/${level}`) ?? null;
export const load_translation_result = async (entry_id: number) => translation_results.get(entry_id) ?? null;

export function translation_document(entry: number) {
  const source_segments = [0, 1].map((i) => ({ source_segment_id: `fixture_${entry}_${i}`, order_index: i, segment_type: "p", source_html: `<p>source ${entry}/${i}</p>`, source_text: `source ${entry}/${i}` }));
  return { source_hash: "source", segmenter_version: "reader-html-block-v2", source_segments, html: `<html><head></head><body><article class="reader">${source_segments.map((s) => `<p data-source-segment-id="${s.source_segment_id}">${s.source_text}</p>`).join("")}</article></body></html>`, result: null };
}

export const load_translation_document = async (entry_id: number) => translation_document(entry_id);

export const get_agent_fingerprint = async () => "fixture-config";
export const load_latest_summary = async () => null;
export const get_setting = async () => null;
export const set_setting = async () => {};

// Ownership regressions do not open export UI; fail if they accidentally do.
export const preview_digest = async () => { throw new Error("Digest IPC belongs to tests/digest_export"); };
export const export_digest = async () => { throw new Error("Digest IPC belongs to tests/digest_export"); };
export const export_multi_digest = async () => { throw new Error("Digest IPC belongs to tests/digest_export"); };

// M4W-010 read-only queries are inert in ownership fixtures; original request
// and mutation probes above remain unchanged.
export const local_tag_suggestions = async (_entry_id: number): Promise<string[]> => [];
export const list_tags = async (_search?: string): Promise<object[]> => [];
export const related_entries = async (_entry_id: number): Promise<object[]> => [];
