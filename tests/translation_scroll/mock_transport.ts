import fixture from "production-theme-fixture";

export class Channel<T> { onmessage = (_message: T) => {}; }
const sources = Array.from({ length: 100 }, (_, index) => ({
  source_segment_id: `paragraph-${index}`, order_index: index, segment_type: "p",
  source_html: `<p id="paragraph-${index}">Paragraph ${index}: ${"Original article text for reading position. ".repeat(12)}</p>`,
  source_text: `Paragraph ${index}`,
}));
sources.push(
  { source_segment_id: "cell", order_index: 100, segment_type: "td", source_html: '<td id="cell">Table original</td>', source_text: "Table original" },
  { source_segment_id: "term", order_index: 101, segment_type: "dt", source_html: '<dt id="term">Term original</dt>', source_text: "Term original" },
  { source_segment_id: "definition", order_index: 102, segment_type: "dd", source_html: '<dd id="definition">Definition original</dd>', source_text: "Definition original" },
);
function html(args: Record<string, unknown> = {}, annotated = false) {
  const key = `${args.themeId ?? "classic-light"}:${args.fontFamily ?? "sans-serif"}:${args.fontSize ?? 22}`;
  let body = '<article><a href="https://example.com/" id="article-link">Article link</a>'
    + sources.slice(0, 100).map((s) => s.source_html).join("")
    + `<table><tbody><tr>${sources[100].source_html}</tr></tbody></table>`
    + `<dl>${sources[101].source_html}${sources[102].source_html}</dl></article>`;
  if (annotated) for (const source of sources) {
    body = body.replace(`id="${source.source_segment_id}"`, `id="${source.source_segment_id}" data-source-segment-id="${source.source_segment_id}"`);
  }
  return fixture.rendered[key].replace(/<body>[\s\S]*<\/body>/, `<body>${body}</body>`);
}
let active: { channel: Channel<string>; resolve: () => void; entry: number; id: string } | null = null;
const cache = new Map<number, object>();
export function emit(index: number, text: string) {
  if (!active) throw new Error("No active translation");
  active.channel.onmessage(JSON.stringify({ type: "segment", index, source_segment_id: sources[index].source_segment_id, text }));
}
export function finish() { active?.resolve(); active = null; }
export function seed(entry: number) {
  cache.set(entry, { source_hash: `source/${entry}`, complete: true, text: null,
    segments: sources.map((s) => `Cached ${s.source_text}`), segment_ids: sources.map((s) => s.source_segment_id),
    segment_errors: sources.map(() => null), model_name: "fixture" });
}
export async function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  const entry = args.entryId as number;
  if (command === "build_reader_content") return html(args) as T;
  if (command === "related_entries") return [] as T;
  if (command === "get_agent_fingerprint") return "fixture-config" as T;
  if (command === "get_agent_profile") return { target_language: "zh-Hans", concurrency_degree: 3 } as T;
  if (command === "load_translation_result") return (cache.get(entry) ?? null) as T;
  if (command === "load_translation_document") return { html: html({}, true), source_segments: sources,
    source_hash: `source/${entry}`, segmenter_version: "reader-html-block-v3", result: cache.get(entry) ?? null } as T;
  if (command === "get_agent_status") return { state: active ? "running" : "idle", current_entry_id: active?.entry,
    current_request_id: active?.id } as T;
  if (command === "abandon_agent_waiting") return undefined as T;
  if (command === "run_translation") return new Promise<T>((resolve) => {
    active = { entry, id: args.requestId as string, channel: args.onEvent as Channel<string>, resolve: () => resolve(undefined as T) };
    active.channel.onmessage(JSON.stringify({ type: "task_state", request_id: active.id, state: "running" }));
  });
  throw new Error(`Unexpected native command: ${command}`);
}
export function running() { return !!active; }
