import { act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReaderSummary } from "../../src/features/reader/components/reader_summary";
import { AgentSettings } from "../../src/features/agent/components/agent_settings";
import * as native from "./mock_transport";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const host = document.getElementById("app")!;
const root = createRoot(host);
const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
const passed: string[] = [];
const wait = (ms = 20) => new Promise((resolve) => setTimeout(resolve, ms));
async function settle(fn: () => unknown = () => {}, ms = 20) { await act(async () => { fn(); await wait(ms); }); await act(async () => { await wait(20); }); }
function assert(value: unknown, message: string): asserts value { if (!value) throw new Error(message); }
function button(name: string) { const value = [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.trim() === name); assert(value, `Missing ${name}`); return value; }
async function click(name: string) { await settle(() => button(name).click()); }
const text = () => host.textContent ?? "";
const current = () => native.runs.at(-1)!;
async function render(id: number | "settings" | null, ready = true) {
  await settle(() => root.render(<QueryClientProvider client={client}>{id === null ? null : id === "settings" ? <AgentSettings /> : <ReaderSummary key={id} entry_id={id} entry_title={`Article ${id}`} reader_ready={ready} />}</QueryClientProvider>));
}
async function reset() { await render(null); await settle(() => client.clear()); }
async function toggle() { await settle(() => host.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click()); }

async function run() {
  await render(1); await click("summary.summarize");
  assert(native.commands.findLast((c) => c.command === "run_summary")?.args.detailLevel === "detailed", "Saved detailed default ignored");
  await settle(() => { native.emit(current(), { type: "chunk", content: "PRIMARY PARTIAL" }); native.emit(current(), { type: "reset", reason: "fallback" }); native.emit(current(), { type: "chunk", content: "BACKUP ONLY" }); });
  assert(text().includes("BACKUP ONLY") && !text().includes("PRIMARY PARTIAL"), "Fallback streams mixed");
  await settle(() => native.finish(current()));
  passed.push("Saved detail default reaches production IPC; partial primary stream reset keeps fallback output separate");

  await reset(); let resolve_profile!: (v: unknown) => void;
  native.delay_profile(new Promise((resolve) => { resolve_profile = resolve; }));
  await render(2); await click("summary.brief");
  await settle(() => { native.delay_profile(null); resolve_profile({ target_language: "en", detail_level: "detailed" }); });
  await click("summary.summarize");
  assert(native.commands.findLast((c) => c.command === "run_summary")?.args.detailLevel === "brief", "Late default replaced user choice");
  await settle(() => native.finish(current()));
  passed.push("Delayed saved profile does not overwrite manual detail selection");

  await reset(); await render("settings"); await click("settings.profiles");
  const summaries = [...host.querySelectorAll("h4")].find((h) => h.textContent === "settings.agentTypeSummary")!.parentElement!;
  const selects = summaries.querySelectorAll<HTMLSelectElement>("select");
  assert(!selects[0].textContent?.includes("Translation") && !selects[0].textContent?.includes("Disabled"), "Incompatible/disabled model offered");
  await settle(() => { selects[0].value = "summary-model"; selects[0].dispatchEvent(new Event("change", { bubbles: true })); });
  await settle(() => { selects[1].value = "backup"; selects[1].dispatchEvent(new Event("change", { bubbles: true })); });
  assert(native.commands.findLast((c) => c.command === "update_agent_profile")?.args.fallbackModelId === "backup", "Fallback selection not saved");
  passed.push("Production settings filters capabilities and disabled models; fallback choice reaches production IPC");

  await reset(); native.enable_auto(false); await render(3);
  const before = native.runs.length;
  await toggle(); assert(host.querySelector('[role="alertdialog"]') && !native.auto_enabled, "Auto enabled before confirmation");
  await settle(() => {}, 1050); assert(native.runs.length === before, "Unconfirmed auto generated content");
  await click("summary.enableAuto"); await settle(() => {}, 500); assert(native.runs.length === before, "Auto skipped 1-second delay");
  await render(4); await settle(() => {}, 1050);
  assert(current().entry === 4 && native.runs.length === before + 1, "Navigation debounce ran old article or duplicated run");
  const active = current();
  await render(5); await settle(() => {}, 1050); const waiting = current();
  assert(!waiting.started && !active.terminal, "Auto cancelled active or started parallel run");
  await render(6); assert(waiting.terminal && !waiting.started && !active.terminal, "Leaving article retained waiting auto or cancelled active");
  await toggle(); await settle(() => {}, 1050); assert(current() === waiting, "Disabling auto failed to clear timer");
  await settle(() => native.finish(active));
  passed.push("Confirmation, full 1-second navigation debounce, serial scheduling, waiting leave and disabling timer; active work continues");

  await reset(); native.enable_auto(true); await render(7); await settle(() => {}, 1050); const run_a = current();
  await render(8); await settle(() => {}, 1050); const run_b = current();
  await toggle(); assert(run_b.terminal && !run_b.started && !run_a.terminal, "Disabling auto cancelled active or retained waiting work");
  await settle(() => native.finish(run_a));
  passed.push("Disabling auto abandons its exact waiting request while the active request continues");

  await reset(); native.enable_auto(true);
  native.latest.set(9, { target_language: "ja", detail_level: "brief", result: { text: "Historical summary", complete: true } });
  const count = native.runs.length; await render(9); await settle(() => {}, 1050);
  assert(text().includes("Historical summary") && text().includes("summary.historicalConfiguration") && native.runs.length === count, "Existing summary regenerated or not projected");
  await render(10); native.fail_reads(true); await settle(() => client.invalidateQueries({ queryKey: ["latest_summary", 10] }));
  await settle(() => {}, 1050); assert(native.runs.length === count && text().includes("summary lookup failed"), "Failed read treated as missing summary");
  native.fail_reads(false); await render(11); await settle(() => {}, 1050); const failed = current();
  await settle(() => native.finish(failed, "fixture auto failure")); await settle(() => {}, 1050);
  assert(current() === failed && text().includes("fixture auto failure"), "Auto failure retried without user intent");
  passed.push("Any existing slot projects saved language/detail; failed restore prevents generation; auto failure is surfaced without retry");

  await reset(); native.enable_auto(true); const delayed_count = native.runs.length;
  await render(13, false); await settle(() => {}, 1050);
  assert(native.runs.length === delayed_count, "Auto ran before Reader source readiness");
  await render(13, true); await settle(() => {}, 1050);
  assert(current().entry === 13 && native.runs.length === delayed_count + 1, "Delayed Reader never generated after readiness");
  await settle(() => native.finish(current()));
  passed.push("Delayed Reader readiness prevents empty-source auto run and starts once after readiness");

  await reset(); native.enable_auto(false); native.results.set("12/configuration-A", { text: "Old configuration result" });
  await render(12); assert(text().includes("Old configuration result"), "Cache setup failed");
  await settle(() => { native.change_fingerprint("configuration-B"); void client.invalidateQueries({ queryKey: ["agent_fingerprint"] }); });
  assert(!text().includes("Old configuration result"), "New fingerprint restored old query identity");
  assert(client.getQueryCache().findAll({ queryKey: ["summary_result", 12] }).some((query) => query.queryKey.includes("configuration-B")), "Fingerprint omitted from result key");
  passed.push("Production query keys include configuration fingerprint and reject old-config cached display");
  await reset(); await fetch("/result", { method: "POST", body: JSON.stringify({ passed }) });
}
run().catch(async (error) => { await fetch("/result", { method: "POST", body: JSON.stringify({ passed, error: String(error), stack: error.stack }) }); });
