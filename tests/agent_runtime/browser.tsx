import { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReaderSummary } from "../../src/features/reader/components/reader_summary";
import { ReaderTranslation } from "../../src/features/reader/components/reader_translation";
import { ReaderTagging } from "../../src/features/reader/components/reader_tagging";
import * as native from "./mock_transport";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const host = document.getElementById("app")!;
const root = createRoot(host);
const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
const passed: string[] = [];
const pause = () => new Promise((resolve) => setTimeout(resolve, 10));
async function settle(fn: () => unknown = () => {}) { await act(async () => { fn(); await pause(); }); await act(async () => { await pause(); }); }
function assert(value: unknown, message: string): asserts value { if (!value) throw new Error(message); }
function button(name: string) {
  const found = [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.trim() === name);
  assert(found, `missing ${name}`); return found;
}
const text = () => host.textContent ?? "";
async function click(name: string) { await settle(() => button(name).click()); }
async function render(kind: "summary" | "translation" | "tagging", id: number | null) {
  await settle(() => root.render(<StrictMode><QueryClientProvider client={client}>
    {id == null ? null : kind === "summary" ? <ReaderSummary key={id} entry_id={id} entry_title={`article ${id}`} /> : kind === "translation" ? <ReaderTranslation key={id} entry_id={id} /> : <ReaderTagging key={id} entry_id={id} />}
  </QueryClientProvider></StrictMode>));
}
const current = () => native.runs.at(-1)!;

async function run() {
  await render("summary", 1); await click("summary.summarize"); const a = current();
  await render("summary", 2); await click("summary.summarize"); const b = current();
  assert(a.started && !a.terminal && !b.started && text().includes("agentTask.waiting"), "A was auto-cancelled or B did not visibly wait");
  assert(native.commands.some((c) => c.command === "abandon_agent_waiting" && c.args.requestId === a.id), "article leave did not use exact request ID");
  await render("summary", 3); assert(b.terminal && !b.started, "leaving waiting B did not abandon it");
  await click("summary.summarize"); const c = current();
  await settle(() => { native.emit(a, { type: "chunk", content: "STALE A" }); native.finish(a); });
  assert(c.started && !text().includes("STALE") && !text().includes("agentTask.waiting"), "promotion or display ownership failed");
  assert(native.runs.filter((r) => r.kind === "summary" && r.started).length === 2, "abandoned B started");
  passed.push("production summary wrapper IDs; visible wait; waiting leave; active continuation; promotion and stale chunk isolation");

  await click("agentTask.cancel");
  assert(c.terminal && text().includes("agentTask.cancelled"), "explicit cancel did not finish current request");
  assert(native.commands.some((x) => x.command === "cancel_agent_task" && x.args.requestId === c.id), "cancel targeted a different request");
  await click("summary.summarize"); const retry = current();
  await settle(() => { native.emit(c, { type: "task_state", request_id: c.id, state: "cancelled" }); native.emit(c, { type: "chunk", content: "STALE CANCEL" }); });
  assert(retry.started && !text().includes("STALE") && text().includes("summary.generating"), "late cancelled callback changed retry");
  await render("summary", null); assert(!retry.terminal, "closing active panel auto-cancelled");
  await settle(() => native.finish(retry));
  passed.push("cancel uses current ID, terminal cancellation is visible, retry rejects old callbacks, active close continues");

  await render("translation", 1); await click("translation.translate"); const tr_a = current();
  await settle(() => native.emit(tr_a, { type: "error", index: 0, error: "segment failed" }));
  assert(button("translation.translate").disabled && button("agentTask.cancel"), "one segment error ended active translation");
  await render("translation", 2); await click("translation.translate"); const tr_b = current();
  assert(!tr_b.started && text().includes("agentTask.waiting"), "translation did not wait");
  await click("agentTask.cancel"); assert(tr_b.terminal && !tr_b.started && !tr_a.terminal, "waiting cancel aborted other active translation");
  await render("translation", 3); await click("translation.translate"); const tr_c = current();
  await render("translation", null); assert(tr_c.terminal && !tr_c.started, "closed waiting translation survived");
  await settle(() => native.finish(tr_a));
  passed.push("translation segment error retains executor ownership; waiting cancel and close never start HTTP intent");

  await render("tagging", 1); await click("tagging.suggestTags"); const tag_a = current();
  await render("tagging", 2); await click("tagging.suggestTags"); const tag_b = current();
  assert(text().includes("agentTask.waiting") && !tag_b.started, "tagging has no waiting status/Channel");
  await click("agentTask.cancel"); assert(!tag_a.terminal && tag_b.terminal, "tagging waiting cancel affected active owner");
  await render("tagging", 3); await click("tagging.suggestTags"); const tag_c = current();
  await settle(() => native.finish(tag_a)); assert(tag_c.started && !text().includes("fixture"), "old tags leaked to new owner");
  await click("agentTask.cancel");
  passed.push("tagging production Channel and request ID, waiting cancel, promotion, stale suggestions and active cancel");

  // A forged lifecycle ID on a valid Channel must not alter the visible owner.
  await render("summary", 4); await click("summary.summarize"); const valid = current();
  await settle(() => native.emit(valid, { type: "task_state", request_id: "wrong", state: "waiting" }));
  assert(!text().includes("agentTask.waiting"), "mismatched lifecycle ID altered owner");
  await settle(() => native.finish(valid, 'route "failure"\nfixture'));
  assert(text().includes('route "failure"') && !text().includes("summary.generating"), "route failure did not release pending UI");
  passed.push("lifecycle IDs are verified; routing Promise failure releases controls with parseable visible error");

  await render("summary", 5); await click("summary.summarize"); const background = current();
  await render("summary", 6); await render("summary", 5);
  assert(text().includes("agentTask.background") && button("summary.summarize").disabled, "returning article cannot see its existing active task");
  await click("agentTask.cancel");
  assert(background.terminal && text().includes("agentTask.cancelled"), "returning article failed to cancel its background request");
  assert(native.commands.at(-2)?.command !== "run_summary", "background cancel submitted a duplicate generation");
  assert(native.commands.some((c) => c.command === "cancel_agent_task" && c.args.requestId === background.id), "background cancellation used a new ID");
  await click("summary.summarize"); await settle(() => native.finish(current()));
  passed.push("returning article projects backend active owner and cancels its original ID, then permits a new run");

  await render("summary", 7); await click("summary.summarize"); const cancel_failed = current();
  await render("summary", 8); await render("summary", 7);
  native.fail_next_cancel(); await click("agentTask.cancel");
  assert(text().includes("fixture cancel transport failure") && !cancel_failed.terminal, "background cancel failure was not recoverable/visible");
  await act(async () => { native.finish(cancel_failed); await new Promise((resolve) => setTimeout(resolve, 1150)); });
  assert(!button("summary.summarize").disabled, "background cancel failure left controls stuck after natural completion");
  await click("summary.summarize"); await settle(() => native.finish(current()));
  passed.push("failed background cancellation preserves active task and releases controls after natural completion");
  await render("summary", 9); native.cancel_before_next_registration(); await click("summary.summarize");
  assert(text().includes("agentTask.cancelled") && !button("summary.summarize").disabled, "pre-registration cancellation without Channel event left wrong UI");
  passed.push("pre-registration cancellation without lifecycle Channel still reaches visible cancelled terminal state");
  await settle(() => root.unmount()); client.clear();
}
run().then(async () => { await fetch("/result", { method: "POST", body: JSON.stringify({ passed }) }); }).catch(async (error) => {
  await fetch("/result", { method: "POST", body: JSON.stringify({ passed, error: String(error.stack) }) });
});
