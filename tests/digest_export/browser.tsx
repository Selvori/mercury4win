import { act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { DigestDialog } from "../../src/features/reader/components/digest_dialog";
import { EntryList } from "../../src/features/entry/components/entry_list";
import { ReaderPane } from "../../src/features/reader/components/reader_pane";
import { useNoteDraftStore } from "../../src/stores/note_draft_store";
import { useDigestSelectionStore } from "../../src/stores/digest_selection_store";
import * as native from "./mock_transport";
// Dialogs render into body; keep queries inclusive of their portals.
const host = document.body;
const root = createRoot(document.getElementById("app")!);
const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
(globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
const passed: string[] = [];
const wait = () => new Promise((resolve) => setTimeout(resolve, 25));
async function settle(action: () => unknown = () => {}) { await act(async () => { action(); await wait(); }); await act(async () => { await wait(); }); }
function assert(value: unknown, message: string): asserts value { if (!value) throw new Error(message); }
function button(name: string) { const b = [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.trim() === name); assert(b, `Missing ${name}`); return b; }
async function click(name: string) { await settle(() => button(name).click()); }
const text = () => host.textContent ?? "";
const latest = (name: string) => native.commands.findLast((c) => c.command === name)!;
const checkboxes = () => [...host.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')];
async function render(kind: "single" | "multi" | "list" | "reader" | null, id = 1) {
  await settle(() => root.render(<QueryClientProvider client={client}>{kind === "single" ? <DigestDialog key={id} entry_ids={[id]} on_close={() => void render(null)} /> :
    kind === "multi" ? <DigestDialog entry_ids={[2,1]} multiple on_close={() => void render(null)} /> :
    kind === "list" ? <EntryList feed_id={id} selected_entry_id={1} on_select_entry={() => { navigation++; }} /> :
    kind === "reader" ? <ReaderPane entry={native.entries[id-1]} /> : null}</QueryClientProvider>));
}
let navigation = 0;
async function reset() { await render(null); await settle(() => client.clear()); }
async function run() {
  useNoteDraftStore.getState().set_draft(1,"Unsaved secret note");
  await render("single");
  assert(checkboxes()[1].checked && checkboxes()[2].checked && !checkboxes()[0].checked, "Saved defaults wrong");
  assert(text().includes("digest.unsavedNote") && !text().includes("Unsaved secret note"), "Unsaved note leak/warning missing");
  assert(!host.querySelector("script") && text().includes("<script>literal</script>"), "Preview executed HTML");
  const shown = host.querySelector("pre:not(#result)")!.textContent;
  await click("digest.export");
  assert(latest("export_digest").args.expectedContent === shown && latest("export_digest").args.options.include_note, "Export differs from preview/IPC options");
  assert(text().includes("digest.saved:C:/fixture/中文.md"), "Success path not visible");
  passed.push("Production single defaults read saved note/summary, exclude session draft with notice; preview text stays inert and exact content/options reach export IPC");

  await settle(() => { const select = host.querySelector("select")!; select.value="single_text"; select.dispatchEvent(new Event("change",{bubbles:true})); });
  await click("digest.export");
  assert(native.dialogs.at(-1).filters[0].extensions[0] === "txt" && latest("export_digest").args.templateName === "single_text", "Text path/filter mismatch");
  await settle(() => checkboxes()[0].click()); assert(text().includes("digest.missing") && text().includes("digest.body"), "Missing selected body silent");
  passed.push("Plain-text format reaches template and .txt save filter; selected unavailable content is listed before export");

  native.configure({path:null}); const count = native.commands.filter((c)=>c.command.startsWith("export_")).length;
  await click("digest.export"); assert(text().includes("digest.cancelled") && native.commands.filter((c)=>c.command.startsWith("export_")).length===count,"Cancel wrote file");
  native.configure({path:"C:/fixture/中文.md",dialog:true}); await click("digest.export"); assert(text().includes("dialog failed"),"Dialog failure hidden");
  native.configure({dialog:false,export:true}); await click("digest.export"); assert(text().includes("write failed"),"Write failure hidden");
  native.configure({export:false}); await click("digest.export"); assert(text().includes("digest.saved:"),"Retry didn't recover");
  passed.push("Save-dialog cancellation calls no export; dialog/write failures are visible and retry succeeds");

  await reset(); native.configure({defaults:true}); await render("single");
  assert(text().includes("saved lookup failed") && button("digest.export").disabled,"Defaults failure allowed export");
  native.configure({defaults:false}); await click("digest.retryDefaults"); assert(!button("digest.export").disabled && checkboxes()[1].checked,"Defaults retry failed");
  native.configure({preview:true}); await click("digest.refresh"); assert(text().includes("preview failed") && button("digest.export").disabled,"Preview failure allowed export");
  native.configure({preview:false}); await click("digest.refresh"); assert(!button("digest.export").disabled,"Preview retry failed");
  passed.push("Saved-content and preview failures block export with recoverable retries");

  await reset(); native.configure({hold:true}); await render("single",2); const finish = native.pending_preview!;
  await render("single",3); await settle(finish); assert(host.querySelector("pre:not(#result)")!.textContent!.startsWith("preview:3:"),"Late article 2 preview replaced 3");
  await reset(); await render("multi"); assert(checkboxes().every((c)=>!c.checked),"Multi defaults enabled options");
  await click("digest.export"); assert(JSON.stringify(latest("export_multi_digest").args.entryIds)==="[2,1]", "Multi IPC order wrong");
  passed.push("Late preview is isolated by selection; multi defaults are opt-in and export_multi_digest preserves supplied order");

  await reset(); await render("list"); await click("digest.multiple");
  assert(useDigestSelectionStore.getState().selecting && !text().includes("entry.loadMore"),"Paging/mode not frozen");
  await settle(() => { const rows=host.querySelectorAll<HTMLElement>('[role="button"]'); rows[1].click(); rows[0].click(); document.dispatchEvent(new KeyboardEvent("keydown",{key:"j",bubbles:true})); });
  assert(navigation===0 && button("entry.markAllRead").disabled && button("entry.refresh").disabled,"Selection mutated Reader/actions");
  await click("digest.preview"); assert(JSON.stringify(latest("preview_digest").args.entryIds)==="[1,2]", "Checkbox click order used instead of list order");
  assert(!useDigestSelectionStore.getState().selecting,"Selection didn't exit on confirmation");
  await click("digest.close"); await click("digest.multiple"); await render("list",2);
  assert(!useDigestSelectionStore.getState().selecting && !host.querySelector('[data-digest-selecting="true"]'),"Scope change retained selection");
  passed.push("List mode freezes paging/keyboard/mutations, preserves Reader choice, exports list order and clears on scope navigation");

  await reset(); await render("reader"); await click("digest.title"); assert(host.querySelector('[role="dialog"]'),"Reader entry missing");
  await render("reader",2); assert(!host.querySelector('[role="dialog"]'),"Reader navigation retained dialog");
  await settle(()=>useDigestSelectionStore.getState().set_selecting(true)); assert(host.querySelector('[inert]'),"Reader interaction not frozen");
  await settle(()=>useDigestSelectionStore.getState().set_selecting(false));
  assert(!native.commands.some((c)=>/run_summary|run_translation|get_agent|resolve_model/.test(c.command)),"Export requested AI configuration/generation");
  passed.push("Actual Reader toolbar opens single export, navigation closes it, list selection makes Reader inert; no AI/configuration IPC");
  await reset(); return { passed };
}
run().then((result)=>fetch("/result",{method:"POST",body:JSON.stringify(result)})).catch((error)=>fetch("/result",{method:"POST",body:JSON.stringify({passed,error:String(error),stack:error.stack})}));
