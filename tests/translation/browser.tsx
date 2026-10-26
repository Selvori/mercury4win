import { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReaderTranslation } from "../../src/features/reader/components/reader_translation";
import { ReaderPane } from "../../src/features/reader/components/reader_pane";
import { compose_bilingual_document } from "../../src/features/reader/lib/bilingual_document";
import type { Entry } from "../../src/types";
import * as native from "./mock_transport";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const host = document.getElementById("app")!;
const root = createRoot(host);
const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
const passed: string[] = [];
const pause = () => new Promise((r) => setTimeout(r, 12));
async function settle(fn: () => unknown = () => {}) { await act(async () => { fn(); await pause(); }); await act(async () => { await pause(); }); }
function assert(value: unknown, message: string): asserts value { if (!value) throw new Error(message); }
function button(name: string) { const b = [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.trim() === name || b.getAttribute("aria-label") === name); assert(b, `missing ${name}`); return b; }
const text = () => host.textContent ?? "";
const current = () => native.runs.at(-1)!;
function frame_html() { const frame = host.querySelector<HTMLIFrameElement>("iframe")!; return frame.contentDocument?.documentElement.outerHTML ?? frame.srcdoc; }
async function click(name: string) { await settle(() => button(name).click()); }
async function render(entry: number | null, pane = false) { await settle(() => root.render(<StrictMode><QueryClientProvider client={client}>{entry === null ? null : pane ? <ReaderPane entry={{ id: entry, title: "fixture", url: null } as Entry}/> : <ReaderTranslation key={entry} entry_id={entry}/>}</QueryClientProvider></StrictMode>)); }

async function run() {
  const doc = native.document_for(1);
  const values = doc.source_segments.map((source) => `${source.segment_type} 中文译文 ${source.order_index}`);
  values[1] = "<img src=x onerror=alert(1)> 中文";
  const html = compose_bilingual_document(doc.html, doc.source_segments, values, "<html><head><style id='current-reader-theme'>body{font-family:Serif;font-size:22px;color:white;background:black}</style></head><body></body></html>");
  const parsed = new DOMParser().parseFromString(html, "text/html");
  assert(parsed.querySelector("#current-reader-theme"), "current theme and font were lost");
  assert(parsed.querySelectorAll("[data-translation-for]").length === doc.source_segments.length, "missing source translations or nested list duplicated");
  for (const source of doc.source_segments) {
    const original = parsed.querySelector(`[data-source-segment-id='${source.source_segment_id}']`)!;
    const projected = ["th", "td", "caption"].includes(source.segment_type) ? original.querySelector("[data-translation-for]") : original.nextElementSibling;
    assert(projected?.getAttribute("data-translation-for") === source.source_segment_id, "translation inserted at a different source block");
  }
  assert(parsed.querySelectorAll("[data-translation-for] img").length === 0 && parsed.querySelectorAll("script").length === 0, "provider HTML executed as markup");
  assert(parsed.querySelector("pre")?.textContent?.includes("original") && parsed.querySelector("blockquote")?.textContent?.includes("Quoted original"), "unsupported original blocks were lost");
  assert(parsed.querySelector("ul")?.querySelector("ul") && parsed.querySelectorAll("img").length === 1, "original nested list or image lost");
  assert(parsed.querySelector("h1 + [data-translation-for]") && parsed.querySelector("blockquote [data-translation-for]") && parsed.querySelectorAll("table [data-translation-for]").length === 4, "heading, quote or table translations missing");
  const reparsed = new DOMParser().parseFromString(parsed.documentElement.outerHTML, "text/html");
  assert(reparsed.querySelectorAll("table [data-translation-for]").length === 4 && reparsed.querySelectorAll("tr").length === 2, "translated cells invalid after iframe HTML reparse");
  passed.push("production Rust fixture: heading/quote/table/p/list translations, valid cell insertion, nested-list ownership, original code/image, inert provider HTML and reader theme/font");

  const partial_values = values.map((value, index) => index === 1 || index === values.length - 1 ? null : value);
  native.seed(1, partial_values); await render(1);
  assert(text().includes("translation.partial") && text().includes("Lead") && text().includes(values[2]), "partial checkpoint did not restore as bilingual source pairs");
  const missing = [...host.querySelectorAll<HTMLButtonElement>("button")].filter((b) => b.textContent === "translation.retry"); assert(missing.length === 2, "retry shown for successful/non-source blocks");
  await settle(() => missing[0].click()); const retry = current();
  assert(native.commands.at(-1)?.command !== "unexpected", "bad IPC");
  const request = native.commands.findLast((c) => c.command === "run_translation")!;
  assert(request.args.sourceSegmentId === doc.source_segments[1].source_segment_id && request.args.requestId === retry.id, "production wrapper lost retry source identity");
  await settle(() => {
    native.emit(retry, { type: "segment", index: 1, source_segment_id: "wrong-source", text: "WRONG MAP" });
    native.emit(retry, { type: "error", index: 1, source_segment_id: doc.source_segments[1].source_segment_id, error: "specific failure" });
  });
  assert(!text().includes("WRONG MAP") && text().includes("specific failure") && button("translation.translate").disabled, "source gate or per-segment failure prematurely unlocked executor");
  await settle(() => { native.emit(retry, { type: "outcome", status: "partial", succeeded: 2, total: 4 }); native.finish(retry, "Translation partial"); });
  assert(text().includes("translation.partial") && !button("translation.translate").disabled, "partial terminal did not release controls");
  passed.push("checkpoint source pairs, missing-only retry, real camelCase sourceSegmentId/requestId, invalid mapping rejected and segment failure waits for invoke terminal");

  native.seed(2, values.map(() => null)); await render(2);
  assert(text().includes("translation.all_failed") && !text().includes("translation.complete"), "all failure shown as complete");
  await click("translation.translate"); const all=current();
  await settle(() => {native.emit(all,{type:"outcome",status:"all_failed",succeeded:0,total:4});native.finish(all,"Translation all_failed");});
  assert(text().includes("translation.all_failed"), "run all-failure state missing");
  native.seed(3, values); await render(3); assert(text().includes("translation.complete"), "complete not restored");
  await render(null); client.clear(); await render(3); assert(text().includes(values[2]), "reopen did not restore result");
  passed.push("all-failed/partial/complete are distinct and persisted complete translation restores with an empty query cache");

  await click("translation.translate"); const old=current(); await render(4);
  await settle(() => { native.emit(old,{type:"source_segments",source_hash:"old",segments:doc.source_segments,html:doc.html});native.emit(old,{type:"segment",index:1,text:"STALE OUTPUT"}); native.finish(old,"STALE ERROR"); });
  assert(!text().includes("STALE"), "old source or translation crossed article");
  native.seed(4,values.map((_, index) => index === 0 ? "saved" : null)); await settle(() => client.invalidateQueries({queryKey:["translation_result",4]}));
  await click("translation.translate"); const cancelled=current(); await click("agentTask.cancel");
  assert(text().includes("saved") && text().includes("agentTask.cancelled"), "cancellation hid checkpoint");
  await settle(() => native.emit(cancelled,{type:"segment",index:0,text:"STALE CANCEL"})); assert(!text().includes("STALE"), "late cancelled output accepted");
  passed.push("article switch rejects old sources/segments/failure; cancellation restores checkpoint and rejects late output");

  await render(null); client.clear(); native.seed(5,values); await render(5,true);
  const cached_count = native.runs.length;
  await click("reader.translation");
  const iframe=host.querySelector<HTMLIFrameElement>("iframe")!;
  assert(iframe.getAttribute("sandbox")==="allow-same-origin", "iframe permissions expanded");
  const main=new DOMParser().parseFromString(frame_html(),"text/html");
  assert(main.querySelectorAll("[data-translation-for]").length===values.length && main.querySelector("#current-reader-theme"), "production ReaderPane did not receive bilingual HTML with current theme");
  const silent = () => !host.querySelector(".cursor-col-resize") && !host.querySelector("[data-translation-for]") && !text().includes("translation.") && !text().includes("agentTask.");
  assert(silent() && native.runs.length === cached_count, "complete cache caused generation or displayed translation controls/status");
  await click("reader.notes"); assert(host.querySelector(".cursor-col-resize"), "notes lost their side panel");
  await settle(() => document.dispatchEvent(new KeyboardEvent("keydown", { key: "t", bubbles: true, cancelable: true })));
  assert(silent() && frame_html().includes("data-translation-for") && native.runs.length === cached_count, "translation shortcut did not restore cache silently");
  await click("reader.translation"); assert(!frame_html().includes("data-translation-for"), "return to original retained translation");
  passed.push("complete cache restores silently by click/T, without sidebar/controls/progress or API request; notes/sidebar, sandbox and original mode preserved");

  await render(7,true); const initial_runs = native.runs.length;
  await click("reader.translation"); const auto = current();
  assert(native.runs.length === initial_runs + 1 && auto.entry === 7 && silent(), "one selection did not start exactly one silent request under StrictMode");
  await settle(() => {native.emit(auto,{type:"segment",index:0,source_segment_id:native.document_for(7).source_segments[0].source_segment_id,text:"live title translation"});native.emit(auto,{type:"progress",segment:1,total:values.length});});
  assert(frame_html().includes("live title translation") && silent() && !text().includes("1/"), "live translation or silent progress projection failed");
  await settle(() => client.invalidateQueries({queryKey:["translation_document",7]}));
  assert(native.runs.length === initial_runs + 1, "refetch started a duplicate request");
  await settle(() => {native.emit(auto,{type:"error",index:1,error:"silent segment failure"});native.finish(auto,"silent terminal failure");});
  await settle(() => {});
  assert(silent() && !text().includes("silent") && native.runs.length === initial_runs + 1, "failure displayed controls/errors or retried without user intent");
  native.seed(7,partial_values);
  await click("reader.translation");
  await settle(() => document.dispatchEvent(new KeyboardEvent("keydown", {key:"t",bubbles:true,cancelable:true})));
  const resume = current();
  assert(native.runs.length === initial_runs + 2 && resume.entry === 7 && silent(), "T did not resume partial result with one request");
  await settle(() => { native.seed(7,values);native.finish(resume); });
  assert(frame_html().includes(values.at(-1)!) && silent(), "automatic result was not restored after completion");
  passed.push("one-click/T starts/resumes automatically under StrictMode; streamed content and completion display silently; errors/refetches do not create retry loops");

  await render(8,true); const switched_runs = native.runs.length; await click("reader.translation"); const abandoned=current();
  await render(9,true);
  assert(native.runs.length === switched_runs + 1 && !frame_html().includes("data-translation-for"), "article switch started generation or retained translation");
  await settle(() => {native.emit(abandoned,{type:"segment",index:0,text:"STALE AUTO"});native.finish(abandoned,"STALE AUTO FAILURE");});
  assert(!frame_html().includes("STALE"), "old automatic translation crossed article");
  await click("reader.translation"); const background=current(); await click("reader.translation"); await click("reader.translation");
  assert(current() === background && silent(), "reopening an active background run duplicated generation");
  await settle(() => {native.seed(9,values);native.finish(background);});
  await settle(() => client.invalidateQueries({queryKey:["agent_status","translation"]}));
  assert(current() === background && frame_html().includes(values[0]), "background completion did not restore cache without a new request");
  passed.push("article switch returns to original and rejects old stream; reopening a background run does not enqueue another generation");
  await render(null); client.clear(); native.fail_next_document(true); await render(6);
  assert(button("translation.translate").disabled && button("translation.start").disabled && text().includes("fixture fetch failed"), "fetch failure did not fail closed"); native.fail_next_document(false);
  const run_count = native.runs.length; await click("translation.retryLoad");
  assert(!button("translation.translate").disabled && native.runs.length === run_count, "source retry did not recover reads without generating");
  passed.push("failed source read visibly blocks article and empty-state start controls");
  await render(null); client.clear(); native.fail_next_document(true); await render(10,true);
  const blocked_count = native.runs.length; await click("reader.translation");
  assert(native.runs.length === blocked_count && silent() && !text().includes("fixture fetch failed"), "failed source read generated or exposed controls/error");
  native.fail_next_document(false); await click("reader.translation"); await click("reader.translation");
  const recovered=current(); assert(native.runs.length === blocked_count + 1 && recovered.entry === 10, "explicit reselection failed to recover source read and start");
  await settle(() => {native.seed(10,values);native.finish(recovered);});
  passed.push("source read failure stays silent and sends no API call; explicit reselection recovers reads and starts once");

  await render(null); client.clear();
  let resolve_profile!: (profile: {target_language:string;concurrency_degree:number}) => void;
  native.delay_profile(new Promise((resolve) => {resolve_profile=resolve;}));
  await render(11,true); const delayed_count=native.runs.length; await click("reader.translation");
  assert(native.runs.length === delayed_count, "generation started before saved profile was read");
  await settle(() => resolve_profile({target_language:"fr",concurrency_degree:3}));
  const configured=current(); assert(native.runs.length === delayed_count+1 && native.commands.findLast((c)=>c.command==="run_translation")!.args.targetLanguage === "fr", "late target language used default or started multiple calls");
  await settle(() => {native.seed(11,values);native.finish(configured);}); native.delay_profile(null);
  await render(null); client.clear();
  let resolve_document!: (doc: ReturnType<typeof native.document_for>) => void;
  native.delayed_documents.set(12,new Promise((resolve) => {resolve_document=resolve;}));
  await render(12,true); const abandoned_count=native.runs.length; await click("reader.translation");
  assert(native.runs.length === abandoned_count, "generation started before source read finished");
  await render(13,true); await settle(() => resolve_document(native.document_for(12))); native.delayed_documents.delete(12);
  assert(native.runs.length === abandoned_count, "late source lookup started abandoned article generation");
  let resolve_reader!: (html:string) => void;
  native.delayed_readers.set(14,new Promise((resolve) => {resolve_reader=resolve;}));
  await render(14,true); await click("reader.translation");
  assert(native.runs.length === abandoned_count, "generation started before Reader reconstruction finished");
  await settle(() => resolve_reader(native.document_for(14).html)); native.delayed_readers.delete(14);
  assert(native.runs.length === abandoned_count+1 && current().entry===14, "Reader ready did not refresh sources and start once");
  await settle(() => {native.seed(14,values);native.finish(current());});
  await render(null);
  native.delay_profile(new Promise((resolve) => {resolve_profile=resolve;}));
  const cached_profile_count = native.runs.length;
  await render(15,true); await click("reader.translation");
  assert(native.runs.length === cached_profile_count, "stale cached profile started generation while refresh was pending");
  await settle(() => resolve_profile({target_language:"de",concurrency_degree:3}));
  assert(native.runs.length === cached_profile_count+1 && native.commands.findLast((c)=>c.command==="run_translation")!.args.targetLanguage === "de", "cached profile refresh used previous target language");
  await settle(() => {native.seed(15,values);native.finish(current());}); native.delay_profile(null);
  passed.push("automatic generation waits for saved/cached profile refresh, sources and Reader readiness; late abandoned source read does not generate");
  await render(null); client.clear(); await fetch("/result",{method:"POST",body:JSON.stringify({passed})});
}
run().catch(async (e) => { await fetch("/result", {method:"POST",body:JSON.stringify({passed,error:String(e),stack:e.stack})}); });
