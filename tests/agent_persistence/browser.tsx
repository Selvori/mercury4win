import { act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReaderSummary } from "../../src/features/reader/components/reader_summary";
import { ReaderTranslation } from "../../src/features/reader/components/reader_translation";
import { ReaderPane } from "../../src/features/reader/components/reader_pane";
import type { Entry } from "../../src/types";
import * as native from "./mock_transport";
(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const host = document.getElementById("app")!; const root = createRoot(host); let client = new QueryClient({ defaultOptions: { queries: { retry: false } } }); const passed: string[] = [];
const pause = () => new Promise((r) => setTimeout(r, 20));
async function settle(fn: () => unknown = () => {}) { await act(async () => { fn(); await pause(); }); await act(async () => { await pause(); }); }
function assert(v: unknown, message: string): asserts v { if (!v) throw new Error(message); }
const text = () => host.textContent ?? "";
async function click(label: string) { const button = [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => (b.textContent?.trim() || b.getAttribute("aria-label")) === label); assert(button, `missing ${label}`); await settle(() => button.click()); }
async function render(kind: "summary" | "translation" | "pane", id: number | null) { await settle(() => root.render(<QueryClientProvider client={client}>{id === null ? null : kind === "summary" ? <ReaderSummary key={id} entry_id={id} entry_title="fixture"/> : kind === "translation" ? <ReaderTranslation key={id} entry_id={id}/> : <ReaderPane key={id} entry={{ id, title: "fixture", url: null } as Entry}/>}</QueryClientProvider>)); }
const current = () => native.runs.at(-1)!;
async function run() {
  native.seed("summary",1,"saved success"); await render("summary",1); assert(text().includes("saved success"),"stored summary missing");
  await render("summary",null); client.clear(); client = new QueryClient(); await render("summary",1); assert(text().includes("saved success"),"reopen with empty cache lost summary"); passed.push("production wrapper and summary restore after panel reopen with a fresh QueryClient");
  await click("summary.summarize"); const failed = current(); await settle(() => { native.emit(failed,{type:"chunk",content:"failed partial"}); native.finish(failed,"fixture failure"); }); assert(text().includes("saved success") && !text().includes("failed partial"),"failed regeneration lost old success");
  await click("summary.summarize"); await settle(() => native.emit(current(),{type:"chunk",content:"cancelled partial"})); await click("agentTask.cancel"); assert(text().includes("saved success") && !text().includes("cancelled partial"),"cancel lost old success"); passed.push("failed and cancelled regeneration restores prior durable success");
  native.seed("summary",1,"brief success","brief"); const count=native.runs.length; await click("summary.brief"); assert(text().includes("brief success") && native.runs.length===count,"detail selection generated instead of restoring its slot"); await click("summary.detailed"); assert(!text().includes("brief success"),"different detail restored wrong summary"); passed.push("detail slot restoration, missing slot isolation, no automatic generation");
  const delayed=native.deferred<ReturnType<typeof native.stored.get>>(); native.delayed.set(2,delayed as never); await render("summary",2); native.seed("summary",3,"article three"); await render("summary",3); await settle(() => delayed.resolve({source_hash:"source",text:"late article two",segments:[],complete:true,model_name:"fixture"})); assert(text().includes("article three") && !text().includes("late article two"),"old restore crossed entry"); native.delayed.delete(2);
  await click("summary.summarize"); const old=current(); await render("summary",4); await settle(() => {native.emit(old,{type:"chunk",content:"background saved"}); native.finish(old);}); assert(!text().includes("background saved"),"old background output crossed entry"); await render("summary",3); assert(text().includes("background saved"),"background success not restored on return"); passed.push("late restore and old Channel isolation; background completion survives return");
  native.seed("translation",5,"checkpoint saved","medium",false); await render("translation",5); assert(text().includes("checkpoint saved"),"checkpoint not restored"); await click("translation.translate"); await settle(() => native.finish(current(),"segment failed")); assert(text().includes("checkpoint saved"),"translation error hid checkpoint");
  await click("translation.translate"); const tr=current(); await settle(() => {native.emit(tr,{type:"segment",index:0,text:"complete first"});native.emit(tr,{type:"segment",index:1,text:"complete second"});native.finish(tr);}); await render("translation",null); client.clear(); await render("translation",5); assert(text().includes("complete first") && text().includes("complete second"),"complete translation not restored"); passed.push("checkpoint recovery and complete translation restore with an empty query cache");
  native.source.set(5,"changed"); await render("translation",null); await render("translation",5); assert(!text().includes("complete first"),"source change recovered stale translation"); passed.push("changed source rejects previous result");
  native.seed("summary",6,"reader rebuilt restore"); native.source.set(6,"not-ready"); const reader=native.deferred<string>();native.reader_delayed.set(6,reader); await render("pane",6); await click("reader.summary"); assert(!text().includes("reader rebuilt restore"),"race fixture did not start before Reader"); await settle(() => {native.source.delete(6);reader.resolve("<p>built reader</p>");}); assert(text().includes("reader rebuilt restore"),"Reader completion failed to refresh an early empty restore"); passed.push("production ReaderPane invalidates early result lookup when Reader reconstruction completes");
  await render("summary",null);client.clear(); await fetch("/result",{method:"POST",body:JSON.stringify({passed})});
}
run().catch(async (e) => {await fetch("/result",{method:"POST",body:JSON.stringify({passed,error:String(e),stack:e.stack})});});
