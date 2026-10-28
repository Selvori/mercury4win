import { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { I18nextProvider } from "react-i18next";
import i18next from "i18next";
import en from "../../src/i18n/en.json";
import zh from "../../src/i18n/zh-Hans.json";
import { EntryList } from "../../src/features/entry/components/entry_list";
import { calls, requests, setSavedResume } from "./mock_transport";
(globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
const root = createRoot(document.getElementById("app")!);
const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity }, mutations: { retry: false } } });
const results: string[] = [];
const selected:number[]=[];
let cleared=0;
const t = (key: string) => i18next.t(key);
const pause = () => new Promise((r) => setTimeout(r, 15));
async function settle(action: () => unknown = () => {}) { await act(async () => { action(); await pause(); }); await act(async () => { await pause(); }); }
function assert(v: unknown, m: string): asserts v { if (!v) throw new Error(m); }
function button(key: string) { const b = [...document.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === t(key) || b.getAttribute("aria-label") === t(key) || b.getAttribute("title") === t(key)); assert(b, `Missing button ${key}`); return b; }
const click = (key: string) => settle(() => button(key).click());
const form = () => document.querySelector<HTMLFormElement>(`form[aria-label="${t("search.natural.title")}"]`)!;
async function sentence(s: string) { await settle(() => { const input = form().querySelector("input")!;Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value")!.set!.call(input,s);input.dispatchEvent(new Event("input",{bubbles:true})); }); }
const submit = () => settle(() => form().dispatchEvent(new Event("submit",{bubbles:true,cancelable:true})));
const last = () => requests.at(-1)!;
function progress(status = "complete", pending = 0) { return {scan_id:"check-71",started_at:"2026-10-28T00:00:00Z",status,phase:pending?"full":"done",total:71,title_screened:71,title_model_reviewed:69,title_excluded:40,preview_screened:31,further_pending:pending,full_verified:12,summary_only:1,note_verified:0,excluded:57,insufficient:1,changed:0,failed:status==="failed"?1:0,pending,matches:2,full_parts_done:12,full_parts_total:18,cache_hits:0,snapshot_revision:3}; }
function result(id = "scan:scope-1", pending = 0, status = "complete") { return { session: { id, sentence: last().args.sentence, input_key: "key", plan: { intent: "AI model releases", routes: [{ language: "en", groups: [["model","AI"]] }] }, filter: {sort:"relevance"},manual:last().args.manual,checked:71-pending,total:71,pending,matches:2,stale:false,scan:progress(status,pending) }, error:null,cached:false }; }
function quick(id="scan:quick-start") {const r=result(id,71,"running");return {...r,session:{...r.session,preliminary:2,scan:{...r.session.scan,title_screened:0,title_model_reviewed:0,full_verified:0,matches:0,full_parts_done:0}}};}
async function run() {
  await i18next.init({lng:"zh-Hans",fallbackLng:"en",resources:{en:{translation:en},"zh-Hans":{translation:zh}}});
  await settle(() => root.render(<StrictMode><I18nextProvider i18n={i18next}><QueryClientProvider client={client}><EntryList feed_id={null} search_mode selected_entry_id={null} on_clear_selection={()=>cleared++} on_select_entry={entry=>selected.push(entry.id)} /></QueryClientProvider></I18nextProvider></StrictMode>));
  assert(document.body.textContent!.includes("Ordinary article"),"ordinary list missing");assert(requests.length===0,"ordinary list called AI");
  await click("search.natural.title");assert(document.body.textContent!.includes(i18next.t("search.natural.recentScope")),"newest 100 scope disclosure missing");await sentence("新的AI模型");assert(requests.length===0,"typing called AI");await submit();
  const first=last();assert(first.args.manual.sort==="relevance"&&first.args.manual.ai_search_id===null,"semantic defaults or IPC mapping");
  await settle(()=>first.args.onEvent.onmessage(JSON.stringify({type:"search_snapshot",session:quick().session})));
  assert(form().getAttribute("aria-busy")==="true"&&document.body.textContent!.includes("Keyword article 1"),"quick results must show before AI request resolves");
  assert(document.body.textContent!.includes(t("search.natural.keywordPending"))&&!document.body.textContent!.includes("Release evidence"),"keyword matches must not look AI-verified");
  await settle(()=>document.querySelector<HTMLElement>('[role="button"][aria-label="Keyword article 1"]')!.click());assert(selected.includes(1),"quick article is not readable during verification");
  const quickCount=requests.length;await click("entry.loadMore");assert(document.body.textContent!.includes("Keyword article 2"),"quick pagination missing");
  await click("entry.markAllRead");assert(calls.filter(c=>c.command==="mark_all_read_in_scope").at(-1)!.args.filter.ai_search_id==="scan:quick-start","quick bulk lost captured token");
  assert(requests.length===quickCount,"reading, paging or quick bulk called AI");
  const clearsBeforeVerification=cleared;
  await settle(()=>first.args.onEvent.onmessage(JSON.stringify({type:"scan_progress",progress:{...progress("running",12),title_fallback:16}})));
  assert(document.body.textContent!.includes("71"),"all-title coverage missing");
  assert(document.body.textContent!.includes(i18next.t("search.natural.titleFallback",{count:16})),"unusable titles must visibly continue to content review");
  await settle(()=>first.args.onEvent.onmessage(JSON.stringify({type:"search_snapshot",session:result("scan:progress",12,"running").session})));
  assert(document.body.textContent!.includes("Direct model release"),"progressive confirmed result absent");
  assert(cleared===clearsBeforeVerification,"progressive snapshots closed the article being read");
  await settle(()=>first.resolve(result()));
  assert(document.body.textContent!.includes("Release evidence"),"evidence missing");assert(button("search.saveView").disabled,"scope save enabled");
  const count=requests.length;await click("entry.loadMore");assert(document.body.textContent!.includes("AI article 2"),"page missing");
  const sort=document.querySelector<HTMLSelectElement>('select[id$="-sort"]')!;
  await settle(()=>{sort.value="oldest";sort.dispatchEvent(new Event("change",{bubbles:true}));});
  assert(requests.length===count,"local sort called AI");assert(calls.some(c=>c.command==="load_entries"&&c.args.filter.ai_search_id==="scan:scope-1"&&c.args.filter.sort==="oldest"),"sort not applied within fixed token");
  await click("entry.markAllRead");assert(requests.length===count,"bulk called AI");
  const bulk=calls.filter(c=>c.command==="mark_all_read_in_scope").at(-1)!;assert(bulk.args.filter.ai_search_id==="scan:scope-1","bulk widened");
  results.push("One submission / 71-title versus full coverage / progressive immutable snapshots / evidence / local sort, page and bulk / save disabled");
  await submit();const paused=last();await settle(()=>paused.args.onEvent.onmessage(JSON.stringify({type:"scan_progress",progress:progress("running",12)})));
  await click("search.natural.pause");assert(calls.some(c=>c.command==="stop_topic_search"&&c.args.requestId===paused.args.requestId&&c.args.mode==="paused"),"pause did not stop native task");
  await settle(()=>paused.resolve(result("scan:late-pause")));assert(!calls.some(c=>c.command==="load_entries"&&c.args.filter.ai_search_id==="scan:late-pause"),"late paused result applied");
  await click("search.natural.resume");assert(last().args.resume==="check-71","resume used scope token instead of durable scan id");await settle(()=>last().resolve(result("scan:resumed")));
  await sentence("different subject");await submit();await settle(()=>last().reject({code:"provider",detail:""}));assert(document.body.textContent!.includes("AI article 1"),"failure destroyed previous list");
  await submit();const cancelled=last();await click("note.cancel");await settle(()=>cancelled.resolve(result("scan:wrong")));assert(!calls.some(c=>c.command==="load_entries"&&c.args.filter.ai_search_id==="scan:wrong"),"late cancelled applied");
  await submit();const old=last();await submit();const latest=last();await settle(()=>old.resolve(result("scan:late")));assert(form().getAttribute("aria-busy")==="true","old finally stopped new request");await settle(()=>latest.resolve(result("scan:scope-3")));
  await submit();const failedQuick=last();await settle(()=>failedQuick.args.onEvent.onmessage(JSON.stringify({type:"search_snapshot",session:quick("scan:quick-failed").session})));
  await settle(()=>failedQuick.resolve({...quick("scan:quick-failed"),error:{code:"provider",detail:""}}));assert(document.body.textContent!.includes("Keyword article 1")&&document.querySelector('[role="alert"]'),"AI failure must retain usable preliminary matches");
  await submit();const empty=last();const removed=quick("scan:quick-empty");removed.session.matches=0;removed.session.preliminary=0;
  await settle(()=>empty.args.onEvent.onmessage(JSON.stringify({type:"search_snapshot",session:removed.session})));assert(!document.body.textContent!.includes("Keyword article 1")&&document.body.textContent!.includes(t("search.natural.quickEmpty")),"rejected quick matches must be removed, including the last match");await settle(()=>empty.resolve(result("scan:scope-3")));
  results.push("Backend pause / durable scan resume / draft and provider failure preserve old list / cancel and consecutive response fences");
  await submit();const hidden=last();const stops=calls.filter(c=>c.command==="stop_topic_search").length;await click("search.natural.title");assert(calls.filter(c=>c.command==="stop_topic_search").length===stops,"collapse must keep verification running");await settle(()=>hidden.resolve(result("scan:hidden")));assert(calls.some(c=>c.command==="load_entries"&&c.args.filter.ai_search_id==="scan:hidden"),"collapsed check did not update results");
  await click("search.natural.exit");assert(!button("search.saveView").disabled&&document.querySelector('input[type="search"]'),"ordinary exit broken");
  await settle(()=>i18next.changeLanguage("en"));await click("search.natural.title");assert(form().querySelector("label input"),"English label missing");await sentence("unsupported history");await submit();await settle(()=>last().reject({code:"unsupported",detail:"Reading history unavailable"}));assert(document.querySelector('[role="alert"]')?.textContent?.includes("Reading history"),"unsupported feedback missing");
  await submit();const unmounted=last();await settle(()=>root.unmount());await settle(()=>unmounted.resolve(result("scan:unmounted")));assert(calls.some(c=>c.command==="stop_topic_search"&&c.args.requestId===unmounted.args.requestId&&c.args.mode==="paused"),"unmount did not pause backend");
  results.push("Collapse keeps verification running / unmount pauses backend / ordinary search restored / Chinese and English labelled feedback");
  setSavedResume(null);const cancelRoot=createRoot(document.getElementById("app")!);
  await settle(()=>cancelRoot.render(<I18nextProvider i18n={i18next}><QueryClientProvider client={client}><EntryList feed_id={null} search_mode selected_entry_id={null} on_select_entry={()=>{}} /></QueryClientProvider></I18nextProvider>));
  await click("search.natural.title");await sentence("First quick query");await submit();const firstCancel=last();await settle(()=>firstCancel.args.onEvent.onmessage(JSON.stringify({type:"search_snapshot",session:quick("scan:quick-cancel").session})));
  await click("note.cancel");assert(document.body.textContent!.includes("Ordinary article 9")&&!document.body.textContent!.includes("Keyword article 1"),"first-query cancellation must restore the ordinary list");await settle(()=>firstCancel.resolve(result("scan:cancel-late")));assert(!calls.some(c=>c.command==="load_entries"&&c.args.filter.ai_search_id==="scan:cancel-late"),"late cancelled quick query applied");await settle(()=>cancelRoot.unmount());
  const beforeRestore=requests.length;setSavedResume({sentence:"Saved topic",manual:{sort:"oldest"},progress:progress("paused",5),session:null});
  const restoredRoot=createRoot(document.getElementById("app")!);
  await settle(()=>restoredRoot.render(<I18nextProvider i18n={i18next}><QueryClientProvider client={client}><EntryList feed_id={null} search_mode selected_entry_id={null} on_select_entry={()=>{}} /></QueryClientProvider></I18nextProvider>));
  await click("search.natural.title");await click("search.natural.restore");
  assert(form().querySelector("input")!.value==="Saved topic","saved sentence was not restored");assert(requests.length===beforeRestore,"loading a saved checkpoint called AI");
  await click("search.natural.resume");assert(last().args.resume==="check-71","restored check used wrong identity");await settle(()=>last().resolve(result("scan:restored")));await settle(()=>restoredRoot.unmount());
  results.push("Saved checkpoint loads sentence and hard conditions without AI; explicit resume uses persistent scan identity");
}
run().then(()=>fetch("/result",{method:"POST",body:JSON.stringify({passed:results})})).catch(error=>fetch("/result",{method:"POST",body:JSON.stringify({error:String(error),stack:error.stack,passed:results})}));
