import { act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AppShell } from "../../src/components/layout/app_shell";
import { BatchTagging } from "../../src/features/tags/components/batch_tagging";
import { TagLibrary } from "../../src/features/tags/components/tag_library";
import { ReaderTagging } from "../../src/features/reader/components/reader_tagging";
import { RelatedEntries } from "../../src/features/reader/components/related_entries";
import * as transport from "./mock_transport";
(globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
// Dialogs render into body; mount React separately from query scope.
const mount_host=document.getElementById("app")!;
const host=document.body;
let root=createRoot(mount_host);
let client:QueryClient;
const results:string[]=[];
const pause=()=>new Promise((r)=>setTimeout(r,25));
async function settle(action:()=>unknown=()=>{}) {await act(async()=>{action();await pause();});await act(async()=>{await pause();});}
function assert(value:unknown,message:string):asserts value{if(!value)throw new Error(message);}
function button(key:string){const b=[...host.querySelectorAll<HTMLButtonElement>("button")].find((b)=>b.getAttribute("aria-label")===key||b.textContent===key);assert(b,`missing ${key}`);return b;}
async function click(key:string){await settle(()=>button(key).click());}
function set_input(input:HTMLInputElement,value:string){Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value")!.set!.call(input,value);input.dispatchEvent(new Event("input",{bubbles:true}));}
async function mount(element:React.ReactNode){await act(async()=>root.unmount());mount_host.innerHTML="";root=createRoot(mount_host);client=new QueryClient({defaultOptions:{queries:{retry:false,gcTime:Infinity},mutations:{retry:false}}});await settle(()=>root.render(<QueryClientProvider client={client}>{element}</QueryClientProvider>));}
async function run(){
 transport.reset();transport.no_model();
 await mount(<ReaderTagging entry_id={1}/>);
 assert(host.textContent?.includes("SQLite"),"local suggestions missing without AI");
 await click("SQLite");assert(transport.calls.findLast((c)=>c.command==="add_tag")?.args.entryId===1,"local suggestion wrong entry");assert(!transport.calls.some((c)=>c.command==="run_tagging"),"local suggestions triggered AI");
 assert(host.textContent?.includes("tagging.applied"),"applied list did not refresh");
 const input=host.querySelector<HTMLInputElement>("input")!;await settle(()=>set_input(input,"Rust"));await click("tagging.addTag");assert(transport.detail.tags.some((t:any)=>t.name==="Rust"),"manual add failed");
 results.push("production local/manual tagging works without model and never starts AI; detail and suggestions refresh");
 let selected=0;await mount(<RelatedEntries entry_id={1} on_select={(entry)=>selected=entry.id}/>);await click("Related Rust");assert(selected===2,"related navigation not wired");results.push("production related strip calls actual related_entries wrapper and navigates to related article");
 transport.reset();let mapped:any=null;
 await mount(<TagLibrary selected_tag_ids={[1]} on_tags_changed={(removed,target)=>mapped={removed,target}}/>);
 await click("tagging.manage: Rust");const alias=host.querySelector<HTMLInputElement>('[aria-label="tagging.newAlias"]')!;await settle(()=>set_input(alias,"Intel"));await click("tagging.addAlias");assert(host.textContent?.includes("Alias conflicts"),"alias conflict not visible");await settle(()=>set_input(alias,"RustLang"));await click("tagging.addAlias");assert(host.textContent?.includes("RustLang"),"alias list not refreshed");
 const select=host.querySelector<HTMLSelectElement>("select")!;await settle(()=>{select.value="2";select.dispatchEvent(new Event("change",{bubbles:true}));});assert(host.textContent?.includes("tagging.mergePreview"),"merge preview missing");await click("tagging.confirmMerge");assert(mapped?.removed[0]===1&&mapped?.target===2,"merge selected tag mapping lost");assert(!transport.tags.some((t)=>t.id===1),"merge wrong wrapper arguments");
 results.push("production library alias conflict/success and explicit merge preview/confirmation; selected source remaps to target");
 transport.reset();transport.delay_load();await mount(<BatchTagging/>);await click("tagging.batch");assert(button("tagging.startBatch").disabled,"start allowed during slow recovery");await settle(()=>transport.load_hold!());assert(!button("tagging.startBatch").disabled,"start blocked after successful empty recovery");await click("tagging.startBatch");assert(transport.started,"batch request missing");assert(transport.started.args.scope.skip_tagged===true&&transport.started.args.scope.from&&transport.started.args.scope.to,"batch scope IPC malformed");assert(!transport.calls.some((c)=>c.command==="apply_tag_batch"),"generation wrote tags without review");await settle(()=>transport.complete());assert(button("tagging.confirmApply").disabled,"pending proposals allow apply");await click("tagging.keepAll");await click("tagging.confirmApply");const applied=transport.calls.find((c)=>c.command==="apply_tag_batch")!;assert(JSON.stringify(applied.args.kept)==='["edge ai","sqlite"]'&&applied.args.discarded.length===0,"review decisions IPC malformed");assert(host.textContent?.includes("tagging.applyResult"),"applied summary missing");
 results.push("production batch recovery gate, frozen scope/count/progress, staging without apply and explicit complete review/confirm IPC");
 transport.reset();await mount(<BatchTagging/>);await click("tagging.batch");await click("tagging.startBatch");await click("tagging.stopReview");assert(host.textContent?.includes("tagging.reviewRequired"),"cancel lost completed review");assert(transport.calls.filter((c)=>c.command==="batch_tag").length===1,"cancel restarted batch");assert(!transport.calls.some((c)=>c.command==="apply_tag_batch"),"cancel applied suggestions");await click("tagging.abortBatch");assert(!host.textContent?.includes("tagging.reviewRequired"),"discard did not release lifecycle");
 results.push("production cancellation preserves completed staging for explicit review and discard without extra requests/apply");
 transport.reset();transport.restore_stale();await mount(<BatchTagging/>);await click("tagging.batch");await click("tagging.keepAll");await click("tagging.confirmApply");assert(host.textContent?.includes("Review every current new proposal"),"stale review error missing");assert(host.textContent?.includes("Changed")&&button("tagging.confirmApply").disabled,"stale proposals/decisions not reset");
 results.push("restored persisted review, failed stale apply reloads current proposals and requires decisions again");
 transport.reset();transport.recovery_failure(true);await mount(<BatchTagging/>);await click("tagging.batch");assert(button("tagging.startBatch").disabled&&host.textContent?.includes("fixture recovery failed"),"failed recovery permits generation");transport.recovery_failure(false);await click("readerPane.retryFullArticle");assert(!button("tagging.startBatch").disabled,"recovery retry unusable");
 results.push("failed recovery blocks Start and explicit retry restores usable configuration");
 transport.reset();transport.no_model();await mount(<BatchTagging/>);await click("tagging.batch");assert(button("tagging.startBatch").disabled&&host.textContent?.includes("tagging.configureBatch"),"missing explicit model did not disable batch");
 results.push("batch requires explicit configured tagging model and communicates local/manual alternative");
 transport.reset();await mount(<AppShell/>);await click("sidebar.tags");await settle(()=>host.querySelector<HTMLElement>('[role="checkbox"]')!.click());
 assert(transport.calls.findLast((c)=>c.command==="load_entries")?.args.tagIds[0]===1,"AppShell source not selected");
 await click("sidebar.settings");await settle(()=>[...host.querySelectorAll<HTMLButtonElement>('button')].findLast((b)=>b.textContent?.includes("tagging.library"))!.click());
 await settle(()=>[...host.querySelectorAll<HTMLButtonElement>('button')].findLast((b)=>b.getAttribute('aria-label')==="tagging.manage: Rust")!.click());
 const targets=host.querySelector<HTMLSelectElement>('[aria-label="tagging.mergeInto"]')!;await settle(()=>{targets.value="2";targets.dispatchEvent(new Event('change',{bubbles:true}));});await click("tagging.confirmMerge");
 assert(JSON.stringify(transport.calls.findLast((c)=>c.command==="load_entries")?.args.tagIds)==='[2]',"Settings merge did not remap production AppShell filter");
 results.push("production Settings library merge remaps active AppShell tag filter and reloads entries with target ID");
 transport.reset();await mount(<AppShell/>);await click("sidebar.settings");await settle(()=>[...host.querySelectorAll<HTMLButtonElement>('button')].findLast((b)=>b.textContent?.includes("tagging.library"))!.click());await click("tagging.batch");await click("tagging.startBatch");
 await settle(()=>document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true})));
 assert(host.textContent?.includes("tagging.stopReview"),"Escape closed parent settings during generation");
 const providers=[...host.querySelectorAll<HTMLButtonElement>('button')].find((b)=>b.textContent?.includes("settings.providers"));assert(providers?.disabled,"settings routes can change during batch lifecycle");
 await settle(()=>transport.complete());await settle(()=>document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true})));assert(host.textContent?.includes("tagging.confirmApply"),"Escape discarded unreviewed durable results");await click("tagging.abortBatch");
 results.push("production Settings Escape/route-tab lifecycle lock preserves generation and unreviewed staging until explicit stop/discard");


}
run().then(async()=>{await act(async()=>root.unmount());client?.clear();}).then(()=>fetch("/result",{method:"POST",body:JSON.stringify({passed:results.length,results})})).catch((e)=>fetch("/result",{method:"POST",body:JSON.stringify({results,error:String(e),stack:e.stack})}));
