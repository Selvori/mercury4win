import { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AppShell } from "../../src/components/layout/app_shell";
import { load_entry_detail } from "../../src/lib/tauri_bindings";
import * as transport from "./mock_transport";

(globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
const host = document.getElementById("app")!, root = createRoot(host);
const client = new QueryClient({ defaultOptions: { queries: { retry:false, gcTime:Infinity }, mutations: { retry:false } } });
const results:string[]=[];
const pause=()=>new Promise((resolve)=>setTimeout(resolve,30));
async function settle(action:()=>unknown=()=>{}) { await act(async()=>{ action();await pause(); }); await act(async()=>{await pause();}); }
function assert(value:unknown,message:string):asserts value { if(!value) throw new Error(message); }
const rows=()=>[...host.querySelectorAll<HTMLElement>('[role="button"]')];
function button(label:string) { const b=[...host.querySelectorAll<HTMLButtonElement>('button')].find((b)=>b.textContent?.includes(label) || b.getAttribute('aria-label')===label); assert(b,`missing ${label}`); return b; }
async function click(label:string) { await settle(()=>button(label).click()); }
async function key(key:string, init:KeyboardEventInit={},target:EventTarget=document) { await settle(()=>target.dispatchEvent(new KeyboardEvent('keydown',{key,bubbles:true,cancelable:true,...init}))); }
function input(value:string) { const e=host.querySelector<HTMLInputElement>('input[type="search"]')!; const setter=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')!.set!; setter.call(e,value); e.dispatchEvent(new Event('input',{bubbles:true})); }
const last_load=()=>transport.calls.filter((c)=>c.command==='load_entries').at(-1)!.args;
const selected=()=>rows().find((r)=>r.classList.contains('bg-accent'));
async function run() {
  await settle(()=>root.render(<StrictMode><QueryClientProvider client={client}><AppShell /></QueryClientProvider></StrictMode>));
  await click('feed.allFeeds');
  assert(rows().length===100,'all entries first page missing');
  await click('entry.loadMore'); assert(rows().length===200,'second page missing');
  await settle(()=>input('Article 23')); assert(last_load().cursor===null && last_load().search==='Article 23' && rows().length===9,'search/filter cursor reset failed');
  await settle(()=>input('')); assert(rows().length===100,'returning filter retained old pages');
  await click('entry.unreadOnly'); assert(last_load().unreadOnly===true,'unread not wired');
  await click('sidebar.starred'); assert(last_load().starredOnly===true && last_load().unreadOnly===true,'starred navigation did not combine unread');
  results.push('production AppShell search/unread/starred navigation and scope changes reset pagination');

  await click('sidebar.tags');
  const tags=()=>[...host.querySelectorAll<HTMLElement>('[role="checkbox"]')];
  await settle(()=>tags()[0].click()); await settle(()=>tags()[1].click());
  assert(JSON.stringify(last_load().tagIds)==='[1,2]' && last_load().tagMode==='any','multi-select any missing');
  await click('entry.tagAny'); assert(last_load().tagMode==='all' && rows().length===19,'All not wired');
  for(let i=2;i<5;i++) await settle(()=>tags()[i].click());
  await settle(()=>tags()[5].click());
  assert(last_load().tagIds.length===5 && tags()[5].getAttribute('aria-disabled')==='true' && host.textContent?.includes('entry.tagLimit'),'5 tag limit not communicated');
  for(let i=4;i>=2;i--) await settle(()=>tags()[i].click());
  await settle(()=>input('Article 12'));
  const scope={...last_load()}; delete scope.cursor; delete scope.limit;
  const matches=transport.filtered(scope).map((e)=>e.id);
  await key('A',{shiftKey:true});
  const bulk=transport.calls.findLast((c)=>c.command==='mark_all_read_in_scope')!;
  assert(JSON.stringify(bulk.args)===JSON.stringify(scope),'bulk scope diverged from listing');
  assert(matches.every((id)=>transport.entries.find((e)=>e.id===id)!.is_read),'bulk did not update matched scope');
  assert(transport.entries.filter((e)=>e.is_read).every((e)=>matches.includes(e.id)),'bulk touched other scopes');
  results.push('production multi-select Any/All, visible 5-tag cap and exact combined bulk-read IPC scope');

  await settle(()=>input('')); await click('entry.unreadOnly');
  await click('sidebar.feeds'); await click('feed.allFeeds');
  await settle(()=>rows()[0].click()); const first=transport.entries.find((e)=>e.id===237)!;
  await key('m'); assert(first.is_read,'m not wired'); await key('m'); assert(!first.is_read,'m not reversible');
  const starred=first.is_starred; await key('s'); assert(first.is_starred!==starred,'s not wired');
  await key('j'); assert(selected()?.textContent?.includes('Article 236'),'j not wired');
  await key('k'); assert(selected()?.textContent?.includes('Article 237'),'k not wired');
  await key('R',{shiftKey:true}); assert(transport.calls.some((c)=>c.command==='sync_all_feeds'),'refresh not wired');
  await key('n'); assert(host.textContent?.includes('note.noNotes'),'note shortcut missing');
  await key('n'); assert(!host.textContent?.includes('note.noNotes'),'note toggle missing');
  await key('r'); assert(host.textContent?.includes('summary.generate'),'summary shortcut missing');
  await key('t'); assert(host.textContent?.includes('translation.translateArticle'),'translation shortcut missing');
  await key('t');
  const count=transport.calls.filter((c)=>c.command==='mark_read').length;
  await key('m',{},host.querySelector('input[type="search"]')!);
  const edit=document.createElement('div');edit.setAttribute('contenteditable','true');const child=document.createElement('span');edit.append(child);host.append(edit);
  await key('m',{},child);edit.remove();
  await key('m',{ctrlKey:true});await key('m',{altKey:true});await key('m',{isComposing:true});
  await click('sidebar.settings');await key('m');
  assert(transport.calls.filter((c)=>c.command==='mark_read').length===count,'editing/modal/modifier shortcut guard failed');
  await key('Escape');
  await click('reader.tags');
  assert(host.textContent?.includes('alpha') && host.textContent?.includes('beta'), 'production ReaderTagging did not consume actual Rust detail tags');
  await click('reader.tags');
  results.push('production m/s/j/k/refresh/r/t/n actions, reversible read, editing/IME/modifier/settings guards');

  await settle(()=>rows().at(-1)!.click());transport.hold_once();await key('j');
  assert(transport.pending,'j did not fetch across page');
  await settle(()=>rows()[0].click());await settle(()=>transport.pending!());
  assert(selected()?.textContent?.includes('Article 237'),'delayed j stole newer selection');
  await settle(()=>rows().at(-1)!.click());await key('j');
  assert(selected()?.textContent?.includes('Article 37'),'j did not select next-page first article');
  await key('k'); assert(selected()?.textContent?.includes('Article 38'),'k page boundary missing');
  results.push('j/k cross-page navigation with delayed j unable to overwrite a newer selection');

  const before={feeds:transport.calls.filter((c)=>c.command==='load_feeds').length,tags:transport.calls.filter((c)=>c.command==='list_tags').length};
  await settle(()=>selected()!.querySelector<HTMLButtonElement>('[aria-label="entry.delete"]')!.click());
  assert(!host.querySelector('iframe') && host.textContent?.includes('app.welcome'),'delete did not clear Reader');
  assert(transport.calls.filter((c)=>c.command==='load_feeds').length>before.feeds,'delete did not refresh feeds');
  assert(client.getQueryCache().findAll({queryKey:['tags']}).every((q)=>q.state.isInvalidated),'delete did not invalidate tags');
  results.push('delete clears selected Reader, refreshes feeds and invalidates tag counts');
  const detail=await load_entry_detail(12);
  assert(detail.entry.id===12 && detail.feed_title==='Two' && detail.entry.feed_content==='<p>Feed text</p>' && detail.tags.length===2 && detail.has_note,'actual Rust detail JSON/production wrapper contract failed');
  assert(detail.entry.author===null && !(detail as any).id,'optional/nested contract wrong');
  let missing=false;try{await load_entry_detail(9999);}catch(e){missing=String(e).includes('Entry not found');}assert(missing,'missing detail error lost');
  results.push('real Rust-serialized EntryDetail consumed by production TS wrapper, nested/optional/tags/note and missing error');
}
run().then(()=>finish({passed:results})).catch((e)=>finish({passed:results,error:String(e.stack||e)}));
async function finish(result:object) { await settle(()=>root.unmount());client.clear();await fetch('/result',{method:'POST',body:JSON.stringify(result)}); }
