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
  const render = (target: ReturnType<typeof createRoot>) => target.render(<StrictMode><I18nextProvider i18n={i18next}><QueryClientProvider client={client}><EntryList feed_id={null} search_mode selected_entry_id={null} on_clear_selection={()=>cleared++} on_select_entry={entry=>selected.push(entry.id)} /></QueryClientProvider></I18nextProvider></StrictMode>);
  function simpleForm() {
    const node=form();
    assert(node.querySelectorAll("input").length===1&&node.querySelectorAll("button").length===1,"Topic search must contain one input and one button");
    assert(!node.querySelector("details, p"),"Diagnostic paragraphs or expandable details remain");
    assert(node.querySelector("input")!.getAttribute("placeholder")===t("search.natural.placeholder"),"Topic placeholder mismatch");
    assert(!document.querySelector('select[id$="-sort"]'),"Ordinary filters remain below AI topic input");
    for(const key of ["search.natural.recentScope","search.natural.scopeDetails","search.natural.dataDisclosure","search.natural.routeHint"])
      assert(!document.body.textContent!.includes(t(key)),`Diagnostic disclosure still visible: ${key}`);
  }
  await settle(()=>render(root));
  assert(document.body.textContent!.includes("Ordinary article"),"Ordinary list missing");assert(requests.length===0,"Ordinary list called AI");
  await click("search.natural.title");simpleForm();
  assert(form().querySelector("input")!.placeholder==="输入需要搜索的主题","Chinese placeholder not updated");
  assert(form().querySelector("button")!.disabled,"Blank topic must not submit");
  await sentence("需要关注的模型发布");assert(requests.length===0,"Typing called AI");await submit();
  const first=last();assert(first.args.manual.sort==="relevance"&&first.args.manual.ai_search_id===null,"IPC scope mapping changed");
  assert(button("search.natural.stop"),"Search button must allow stopping the running request");
  await settle(()=>first.args.onEvent.onmessage(JSON.stringify({type:"search_snapshot",session:quick().session})));
  simpleForm();assert(document.body.textContent!.includes("Keyword article 1"),"Quick results not readable while checking");
  assert(document.body.textContent!.includes(t("search.natural.keywordPending")),"Preliminary result must remain distinguishable");
  assert(!document.body.textContent!.includes("release / 语言模型"),"Internal keyword route exposed");
  await settle(()=>document.querySelector<HTMLElement>('[role="button"][aria-label="Keyword article 1"]')!.click());
  assert(selected.includes(1),"Quick article cannot be read");
  await click("entry.loadMore");await click("entry.markAllRead");
  assert(calls.filter(c=>c.command==="mark_all_read_in_scope").at(-1)!.args.filter.ai_search_id==="scan:quick-start","Bulk lost immutable result scope");
  assert(requests.length===1,"Reading, paging or bulk called AI");
  const clearedBefore=cleared;
  await settle(()=>first.args.onEvent.onmessage(JSON.stringify({type:"scan_progress",progress:{...progress("running",12),title_fallback:16}})));
  simpleForm();assert(!document.body.textContent!.includes("71"),"Internal scan counts are displayed");
  await settle(()=>first.args.onEvent.onmessage(JSON.stringify({type:"search_snapshot",session:result("scan:progress",12,"running").session})));
  assert(cleared===clearedBefore,"Progress snapshot closed the reader");
  await click("search.natural.stop");
  assert(calls.some(c=>c.command==="stop_topic_search"&&c.args.requestId===first.args.requestId&&c.args.mode==="paused"),"Single button did not pause backend");
  await settle(()=>first.resolve(result("scan:late-pause")));
  assert(!calls.some(c=>c.command==="load_entries"&&c.args.filter.ai_search_id==="scan:late-pause"),"Late paused response applied");
  await submit();assert(last().args.resume==="check-71","Same topic did not resume durable progress");
  await settle(()=>last().resolve(result()));simpleForm();
  assert(document.body.textContent!.includes("Release evidence"),"Useful article excerpt removed");
  assert(!document.body.textContent!.includes(t("search.natural.basis_full_body",{count:3})),"Internal part counts remain");
  assert(button("search.saveView").disabled,"AI scope can be saved as ordinary view");
  const count=requests.length;await click("entry.loadMore");await click("entry.markAllRead");assert(requests.length===count,"Paging or bulk called AI");
  assert(calls.filter(c=>c.command==="mark_all_read_in_scope").at(-1)!.args.filter.ai_search_id==="scan:scope-1","Confirmed bulk widened scope");
  results.push("Minimal bilingual form / updated placeholder / readable quick and confirmed results / scoped page and bulk / stop and implicit resume");

  await sentence("another topic");await submit();
  await settle(()=>last().reject({code:"provider",detail:"PRIVATE_PROVIDER_DIAGNOSTIC"}));
  assert(document.body.textContent!.includes("AI article 1"),"Failure destroyed previous list");
  assert(form().querySelector("input")!.getAttribute("aria-invalid")==="true"&&button("search.natural.retrySearch"),"Failure is not actionable");
  assert(form().querySelector('[role="alert"]')?.classList.contains("sr-only"),"Error introduced a visible diagnostic panel");
  assert(!document.body.textContent!.includes("PRIVATE_PROVIDER_DIAGNOSTIC")&&!form().querySelector("input")!.title.includes("PRIVATE_PROVIDER_DIAGNOSTIC"),"Raw provider diagnostics exposed");
  await submit();const old=last();await sentence("new topic");await submit();const latest=last();
  await settle(()=>old.resolve(result("scan:late")));assert(form().getAttribute("aria-busy")==="true","Old finally stopped new request");
  await settle(()=>latest.resolve(result("scan:latest")));
  await submit();const empty=last();const emptyResult=quick("scan:quick-empty");emptyResult.session.matches=0;emptyResult.session.preliminary=0;
  await settle(()=>empty.args.onEvent.onmessage(JSON.stringify({type:"search_snapshot",session:emptyResult.session})));
  assert(!document.body.textContent!.includes("AI article 1")&&document.body.textContent!.includes(t("entry.noEntries")),"Rejected final result remained visible");
  await settle(()=>empty.resolve(result("scan:latest")));
  await submit();const closing=last();await click("search.natural.title");
  assert(!form()&&document.querySelector('input[type="search"]')&&!button("search.saveView").disabled,"Exit did not restore ordinary search");
  assert(calls.some(c=>c.command==="stop_topic_search"&&c.args.requestId===closing.args.requestId&&c.args.mode==="paused"),"Leaving AI mode did not pause task");
  await settle(()=>closing.resolve(result("scan:closed")));assert(!calls.some(c=>c.command==="load_entries"&&c.args.filter.ai_search_id==="scan:closed"),"Closed search applied late result");
  results.push("Friendly retry without raw diagnostics / previous results retained on failure / input change and exit fence late responses / ordinary filters restored");

  await settle(()=>i18next.changeLanguage("en"));await click("search.natural.title");simpleForm();
  assert(form().querySelector("input")!.placeholder==="Enter a topic to search","English placeholder mismatch");
  await sentence("English topic");await submit();const unmounted=last();await settle(()=>root.unmount());
  await settle(()=>unmounted.resolve(result("scan:unmounted")));
  assert(calls.some(c=>c.command==="stop_topic_search"&&c.args.requestId===unmounted.args.requestId&&c.args.mode==="paused"),"Unmount did not pause task");
  results.push("English form / accessible input and error / unmount pauses native work");

  setSavedResume({sentence:"Saved topic",manual:{sort:"relevance"},progress:progress("paused",5),session:null});
  const restored=createRoot(document.getElementById("app")!);const beforeRestore=requests.length;
  await settle(()=>render(restored));await click("search.natural.title");simpleForm();
  assert(form().querySelector("input")!.value===""&&requests.length===beforeRestore,"Saved checkpoint prefilled or called AI");
  await sentence("Saved topic");await submit();assert(last().args.resume==="check-71","Submitting matching topic did not resume saved checkpoint");
  await settle(()=>last().resolve(result("scan:restored")));
  await submit();assert(last().args.resume===null,"Completed work resumed an obsolete saved checkpoint");
  await settle(()=>last().resolve(result("scan:refreshed")));await settle(()=>restored.unmount());
  results.push("Saved checkpoint stays invisible and resumes only on matching explicit submission");
}
run().then(()=>fetch("/result",{method:"POST",body:JSON.stringify({passed:results})})).catch(error=>fetch("/result",{method:"POST",body:JSON.stringify({error:String(error),stack:error.stack,passed:results})}));
