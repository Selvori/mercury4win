import { act, StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import en from "../../src/i18n/en.json";
import zh from "../../src/i18n/zh-Hans.json";
import { Dialog, DialogTitle } from "../../src/components/ui/dialog";
import { ThemeToggle } from "../../src/components/layout/theme_toggle";
import { FeedList } from "../../src/features/feed/components/feed_list";
import { FeedEditor } from "../../src/features/feed/components/feed_editor";
import { OpmlImport } from "../../src/features/feed/components/opml_import";
import { OpmlExport } from "../../src/features/feed/components/opml_export";
import { ReaderPane } from "../../src/features/reader/components/reader_pane";
import { ReaderTagging } from "../../src/features/reader/components/reader_tagging";
import { TagInput } from "../../src/features/tags/components/tag_input";
import { AgentSettings } from "../../src/features/agent/components/agent_settings";
import { ErrorState } from "../../src/components/ui/error_state";
import { EntryList } from "../../src/features/entry/components/entry_list";
import { TagLibrary } from "../../src/features/tags/components/tag_library";
import { useKeyboardShortcuts } from "../../src/hooks/use_keyboard_shortcuts";
import * as native from "./mock_transport";
import type { Entry } from "../../src/types";

(globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
const host = document.body;
const root = createRoot(document.getElementById("app")!);
const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
const results: string[] = [];
const pause = () => new Promise(r => setTimeout(r, 20));
async function settle(action: () => unknown = () => {}) { await act(async () => { action(); await pause(); }); await act(async () => { await pause(); }); }
function assert(value: unknown, message: string): asserts value { if (!value) throw new Error(message); }
function button(name: string) { const b = [...host.querySelectorAll<HTMLButtonElement>("button")].find(b => b.getAttribute("aria-label") === name || b.textContent?.trim() === name); assert(b, `Missing ${name}`); return b; }
function key(target: EventTarget, name: string, extra: KeyboardEventInit = {}) { const e = new KeyboardEvent("keydown", { key: name, bubbles: true, cancelable: true, ...extra }); target.dispatchEvent(e); return e; }
function input_value(input: HTMLInputElement, value: string) { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value); input.dispatchEvent(new Event("input", { bubbles: true })); }
async function render(element: React.ReactNode) { await settle(() => root.render(<StrictMode><QueryClientProvider client={client}>{element}</QueryClientProvider></StrictMode>)); }
let shortcuts = 0, parent_closes = 0, child_closes = 0;
export function ModalCase({ both = false, empty = false, locked = false, trigger = true, controls_ready }: { both?: boolean; empty?: boolean; locked?: boolean; trigger?: boolean; controls_ready?: boolean }) {
  const [parent, set_parent] = useState(both);
  const [child, set_child] = useState(both);
  const [ready, set_ready] = useState(!empty);
  useKeyboardShortcuts({ on_down: () => shortcuts++ });
  return <><button onClick={() => set_ready(true)}>Background</button>{trigger && <button onClick={() => set_parent(true)}>Launch</button>}
    <Dialog open={parent} onOpenChange={() => { parent_closes++; set_parent(false); }}>
      <DialogTitle>Parent</DialogTitle>
      {(controls_ready ?? ready) && <><span hidden><button>Hidden attr</button></span><div style={{ display: "none" }}><button>Hidden CSS</button></div><details><summary tabIndex={0}>Details</summary><button>Hidden details</button></details><button disabled>Disabled</button>
      <button onClick={() => set_child(true)}>Child trigger</button><a href="#fixture">Last</a></>}
      <Dialog open={child} onOpenChange={() => { child_closes++; if (!locked) set_child(false); }}>
        <DialogTitle>Child</DialogTitle><input aria-label="First child"/><button>Last child</button>
      </Dialog>
    </Dialog></>;
}
async function launch() { await settle(() => { button("Launch").focus(); button("Launch").click(); }); }

async function run() {
  await i18next.use(initReactI18next).init({ lng: "en", fallbackLng: "en", resources: { en: { translation: en }, "zh-Hans": { translation: zh } }, interpolation: { escapeValue: false } });
  await render(<ModalCase/>); await launch();
  const parent = host.querySelector<HTMLDivElement>('[role="dialog"]')!;
  assert(parent && document.activeElement?.textContent === "Details", "Initial visible focus incorrect");
  assert(document.getElementById("app")!.inert, "Background not inert");
  await settle(() => { button("Background").focus(); }); assert(parent.contains(document.activeElement), "Background programmatic focus escaped");
  await settle(() => { host.querySelector<HTMLElement>("summary")!.focus(); key(document.activeElement!, "Tab", { shiftKey: true }); });
  assert(document.activeElement?.textContent === "Last", "Shift Tab did not wrap or included hidden ancestor");
  await settle(() => key(document.activeElement!, "Tab")); assert(document.activeElement?.textContent === "Details", "Tab did not wrap");
  await settle(() => key(parent, "j")); assert(shortcuts === 0, "Background shortcut ran in modal");
  results.push("production Dialog visible focus, Tab/Shift+Tab wrap, hidden ancestor/details/disabled exclusions and inert/background shortcut suppression");
  await settle(() => { button("Child trigger").focus(); button("Child trigger").click(); });
  const child = [...host.querySelectorAll<HTMLDivElement>('[role="dialog"]')].find(d => d.textContent?.includes("Last child"))!;
  assert(Number(child.parentElement!.style.zIndex) > Number(parent.parentElement!.style.zIndex), "Child visually below parent");
  assert(parent.getAttribute("aria-modal") === "false" && child.getAttribute("aria-modal") === "true", "More than one modal owner");
  assert(parent.parentElement!.inert, "Parent keyboard remains available");
  await settle(() => { button("Last child").focus(); key(button("Last child"), "Tab"); }); assert(document.activeElement?.getAttribute("aria-label") === "First child", "Child Tab escaped");
  await settle(() => key(document.activeElement!, "Escape", { isComposing: true })); assert(child.isConnected, "IME Escape closed modal");
  const before = parent_closes;
  await settle(() => key(document.activeElement!, "Escape")); assert(!child.isConnected && parent.isConnected && parent_closes === before, "Escape closed two layers");
  assert(document.activeElement === button("Child trigger") && !parent.parentElement!.inert, "Child trigger restoration wrong");
  await settle(() => key(document.activeElement!, "Escape")); assert(!parent.isConnected && document.activeElement === button("Launch") && !document.getElementById("app")!.inert, "Parent focus/inert restoration wrong");
  results.push("nested modal visual order, one aria-modal owner, parent inert, child Tab/IME/Escape and parent/child trigger restoration");
  await render(null); await render(<ModalCase both/>);
  const simultaneous = [...host.querySelectorAll<HTMLElement>('[role="dialog"]')]; const top = simultaneous.find(d => d.getAttribute("aria-modal") === "true")!;
  assert(top.textContent?.includes("Last child") && top.contains(document.activeElement) && Number(top.parentElement!.style.zIndex) > Number(simultaneous.find(d => d !== top)!.parentElement!.style.zIndex), "Same-commit parent/child ownership wrong");
  await render(null); assert(!document.getElementById("app")!.inert, "Unmount leaked inert");
  await render(<ModalCase empty/>); await launch(); const empty_panel = host.querySelector<HTMLElement>('[role="dialog"]')!;
  assert(document.activeElement === empty_panel, "Empty modal lacks fallback focus");
  await settle(() => key(empty_panel,"Tab")); assert(document.activeElement === empty_panel, "Empty Tab escaped");
  await render(<ModalCase empty controls_ready/>);
  assert(document.activeElement?.textContent === "Details", "Async first control not focused");
  await render(null);
  results.push("StrictMode and simultaneous nested mounts, empty-panel Tab fallback, async control focus and complete-unmount inert cleanup");
  await render(<ModalCase locked/>); await launch(); await settle(() => button("Child trigger").click());
  const locked_child = host.querySelector('[aria-label="First child"]')!.closest('[role="dialog"]')!;
  await settle(() => key(document.activeElement!,"Escape")); assert(locked_child.isConnected, "Close lock bypassed");
  await render(<ModalCase locked trigger={false}/>); await settle(() => key(document.activeElement!,"Escape"));
  await render(<ModalCase locked={false} trigger={false}/>); await settle(() => key(document.activeElement!,"Escape"));
  await settle(() => key(document.activeElement!,"Escape")); assert(document.activeElement===button("Background"), "Detached trigger fallback wrong");
  await render(null); assert(!document.getElementById("app")!.inert, "Detached trigger cleanup leaked inert");
  const preexisting = document.createElement("section"); preexisting.inert = true; host.append(preexisting);
  await render(<ModalCase/>); await launch(); await render(null); assert(preexisting.inert, "Existing inert overwritten"); preexisting.remove();
  results.push("locked close requests preserve top modal, detached triggers are safe, and pre-existing inert state survives cleanup");

  for (const language of ["en", "zh-Hans"]) {
    native.reset_tags();
    await settle(() => i18next.changeLanguage(language));
    const t = i18next.t.bind(i18next); let selected: number | null = null;
    await render(<><ThemeToggle/><FeedList selected_id={1} on_select={id => selected=id} on_add={() => {}} on_import={() => {}} on_export={() => {}}/></>);
    assert(!host.querySelector("button button"), "FeedList nested button remains");
    for (const label of ["feed.addFeed","feed.syncAll","feed.importOpml","feed.exportOpml"]) assert(button(t(label)).getAttribute("aria-label") === t(label), "Feed icon missing localized name");
    const theme = host.querySelector<HTMLButtonElement>(`button[aria-label^="${language === 'en' ? 'Change appearance' : '切换外观'}"]`)!; assert(theme, "Theme name not localized"); await settle(() => theme.click());
    await settle(() => button("Fixture3").click()); assert(selected === 1, "Feed select wrong ID");
    await settle(() => button(t("feed.deleteNamed", { name: "Fixture" })).click()); assert(native.calls.findLast(c=>c.command==="delete_feed")?.args.id===1, "Delete wrong IPC ID");
    await settle(() => button(t("feed.syncAll")).click()); assert(host.textContent?.includes(t("feed.refreshing")), "Sync fixed status not localized"); await settle(() => native.sync_finish?.());
    await render(<ErrorState on_retry={() => {}}/>); assert(button(t("common.retry")) && host.querySelector('[role="alert"]')?.textContent === t("common.error"), "Generic error fixed strings wrong");
    await render(<FeedEditor open on_close={() => {}}/>); const field = host.querySelector<HTMLInputElement>("input")!;
    assert(host.querySelector<HTMLLabelElement>(`label[for="${field.id}"]`)?.textContent === t("feed.feedUrl"), "Feed label unassociated");
    await settle(() => { input_value(field, "fixture"); host.querySelector("form")!.dispatchEvent(new Event("submit",{bubbles:true,cancelable:true})); }); assert(host.querySelector('[role="alert"]')?.textContent?.includes("fixture invalid Feed"), "Raw diagnostic lost");
    await render(<OpmlImport open on_close={() => {}}/>); await settle(() => button(t("opml.selectFile")).click()); assert(native.dialogs.at(-1).title===t("opml.selectFile") && host.querySelector('[role="alert"]')?.textContent?.includes("fixture file dialog failed"), "Native dialog title/error wrong");
    await render(<OpmlExport open on_close={() => {}}/>); await settle(() => button(t("opml.export")).click()); assert(button(t("common.done")) && host.querySelector('[role="status"]'), "OPML Done/status wrong");
    await render(<ReaderTagging entry_id={1}/>); assert(host.textContent?.includes(t("tagging.noTags")) && host.textContent?.includes(t("tagging.clickToSuggest")), "Tag empty state missing localization");
    const manual = host.querySelector<HTMLInputElement>(`input[aria-label="${t("tagging.addTag")}"]`)!; const count = native.calls.filter(c=>c.command==="add_tag").length;
    await settle(() => { input_value(manual,"输入"); key(manual,"Enter",{isComposing:true}); }); assert(native.calls.filter(c=>c.command==="add_tag").length===count, "IME Enter submitted tag");
    await settle(() => key(manual,"Enter")); assert(native.calls.filter(c=>c.command==="add_tag").length===count+1, "Normal Enter tag not submitted");
    await render(null); client.clear();
    native.configure_reader(true); await render(<ReaderPane entry={{ id: 2, title:"Article",url:null } as Entry}/>);
    assert(host.querySelector('[role="alert"]')?.textContent === t("readerPane.failedToLoad") && host.textContent?.includes("fixture HTTP 503"), "Reader fixed error/diagnostic wrong");
    results.push(`${language}: production theme/Feed/OPML/error/Reader/tagging names, fixed status/empty/error text, associated form label and IME/Enter via actual wrappers`);
    await render(null); client.clear();
  }
  await settle(() => i18next.changeLanguage("en")); await render(<AgentSettings/>); await settle(() => button("Add Provider").click());
  for (const label of ["settings.providerName","settings.baseUrl","settings.apiKey","settings.testModel"]) assert(host.querySelector(`[aria-label="${i18next.t(label)}"]`), `Unlabeled ${label}`);
  await settle(() => button("Profiles").click()); for(const select of host.querySelectorAll("select")) assert(select.getAttribute("aria-label"), "Unlabeled profile select");
  results.push("production provider form and per-agent primary/fallback/language/detail/strategy selectors have accessible names");
  await render(null); client.clear(); native.configure_reader(false); await render(<ReaderPane entry={{id:3,title:"Iframe",url:null} as Entry}/>);
  const frame = host.querySelector("iframe")!; frame.dispatchEvent(new Event("load")); const doc = frame.contentDocument!;
  doc.body.innerHTML='<p id="article">Article</p><a href="https://fixture.test">Link</a><input><div contenteditable="true">Edit</div>';
  await settle(() => key(doc.querySelector("input")!,"n")); assert(!host.querySelector("textarea"), "Iframe input forwarded shortcut");
  await settle(() => key(doc.querySelector('[contenteditable]')!,"n")); assert(!host.querySelector("textarea"), "Iframe editing forwarded shortcut");
  await settle(() => key(doc.querySelector("p")!,"n",{isComposing:true})); assert(!host.querySelector("textarea"), "Iframe IME forwarded shortcut");
  const forwarded: string[]=[]; const watch=(event:KeyboardEvent)=>forwarded.push(event.key);document.addEventListener("keydown",watch);
  await settle(() => key(doc.querySelector("a")!,"Enter")); assert(!forwarded.includes("Enter"), "Iframe anchor Enter forwarded");
  await settle(() => key(doc.querySelector("p")!,"n")); assert(host.textContent?.includes("Add Note"), "Iframe article shortcut not forwarded");
  document.removeEventListener("keydown",watch);assert(frame.getAttribute("sandbox")==="allow-same-origin","Sandbox permissions changed");
  results.push("production iframe keyboard forwarding preserves input/contenteditable/IME/link Enter and article shortcuts with unchanged sandbox (DOM only)");
  await render(null); client.clear(); let added=0;
  await render(<TagInput entry_id={1} selected_tags={[]} on_add={()=>added++} on_remove={()=>{}}/>); const tag_field=host.querySelector<HTMLInputElement>("input")!;
  await settle(()=>{input_value(tag_field,"tag");key(tag_field,"Enter",{isComposing:true});});assert(added===0,"TagInput IME submitted");await settle(()=>key(tag_field,"Enter"));assert(added===1,"TagInput Enter failed");
  results.push("standalone production TagInput accessible input/add and IME-safe Enter");
  await render(null); client.clear(); let selections = 0;
  await render(<EntryList feed_id={1} selected_entry_id={1} on_select_entry={() => selections++}/>);
  const row = host.querySelector<HTMLElement>('[role="button"]')!;
  assert(row.getAttribute("aria-label") === "Keyboard article" && row.getAttribute("aria-pressed") === "true", "Entry selection name/state wrong");
  await settle(() => key(row,"Enter",{isComposing:true})); assert(selections===0,"Entry IME selected");
  await settle(() => key(row,"Enter")); await settle(() => key(row," "));assert(selections===2,"Entry Enter/Space activation failed");
  // Tag data remains empty in this transport, so test actual checkbox behavior
  // with data populated in the real Query cache rather than mocking a component.
  client.setQueryData(["tags",""],[{id:1,name:"Keyboard tag",usage_count:1,is_provisional:false}]);
  await render(<TagLibrary selected_tag_ids={[1]} on_select_tag={() => selections++}/>);
  client.setQueryData(["tags",""],[{id:1,name:"Keyboard tag",usage_count:1,is_provisional:false}]); await settle();
  const checkbox = host.querySelector<HTMLElement>('[role="checkbox"]')!; assert(checkbox?.getAttribute("aria-label")==="Keyboard tag" && checkbox.getAttribute("aria-checked")==="true","Tag checkbox name/state wrong");
  await settle(()=>key(checkbox,"Enter"));await settle(()=>key(checkbox," "));assert(selections===4,"Tag Enter/Space activation failed");
  results.push("production entry and tag selection names/state, Enter/Space handlers and entry IME protection");
}
run().then(async()=>{await act(async()=>root.unmount());client.clear();}).then(()=>fetch("/result",{method:"POST",body:JSON.stringify({passed:results.length,results})})).catch(e=>fetch("/result",{method:"POST",body:JSON.stringify({results,error:String(e),stack:e.stack})}));
