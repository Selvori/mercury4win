import { act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { UsageStats } from "../../src/features/usage/components/usage_stats";
import { AgentSettings } from "../../src/features/agent/components/agent_settings";
import * as native from "./mock_transport";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const host = document.getElementById("app")!;
const root = createRoot(host);
const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
const passed: string[] = [];
const wait = (ms = 20) => new Promise(resolve => setTimeout(resolve, ms));
async function settle(action: () => unknown = () => {}) { await act(async () => { action(); await wait(); }); await act(async () => { await wait(); }); }
function assert(value: unknown, message: string): asserts value { if (!value) throw new Error(message); }
const text = () => host.textContent ?? "";
function buttons(name: string) { return [...host.querySelectorAll<HTMLButtonElement>("button")].filter(b => b.textContent?.trim() === name || b.getAttribute("aria-label") === name); }
async function click(name: string, index = 0) { const b = buttons(name)[index]; assert(b, `Missing ${name}`); await settle(() => b.click()); }
async function select(label: string, value: string) { const element = [...host.querySelectorAll<HTMLSelectElement>("select")].find(s => s.getAttribute("aria-label") === label); assert(element, `Missing select ${label}`); await settle(() => { element.value = value; element.dispatchEvent(new Event("change", { bubbles: true })); }); }
const last_report = () => native.calls.filter(c => c.command === "get_usage_report").at(-1)!;
async function render(element: React.ReactNode) { await settle(() => root.render(<QueryClientProvider client={client}>{element}</QueryClientProvider>)); }
async function reset() { await settle(() => root.render(null)); client.clear(); native.restore(); }
async function run() {
  await render(<UsageStats />);
  assert(text().includes("renamed current") && text().includes("Archived"), "Rust history group or archive badge missing");
  assert(text().includes("historical aggregate events") && text().includes("not measured zeroes"), "Legacy/missing semantics missing");
  assert(host.querySelector("table")?.textContent?.includes("Failed"), "Status breakdown absent");
  assert(text().includes("UTC rolling interval"), "Timezone semantics missing");
  passed.push("Actual Rust report fixture: archived identity, unknown tokens, legacy facts, statuses, UTC range");
  await select("Group by", "agent"); assert(last_report().args.dimension === "agent", "Dimension mapping broken"); assert(text().includes("Summary"), "Agent labels not localized");
  await select("Time range", "14d"); assert(last_report().args.window === "14d", "Window not transmitted");
  await select("Comparison metric", "total_tokens"); assert(host.querySelector('[role="img"]')?.textContent?.includes("20"), "Metric chart not updated");assert(host.querySelector('th[aria-sort="descending"]')?.textContent?.includes("Known total tokens"), "Comparison did not sort by chosen metric");
  passed.push("Production wrapper parameters and cache identities for grouping/window/metric");
  await reset(); native.failure(true); await render(<UsageStats />); assert(host.querySelector('[role="alert"]'), "Query error missing");native.failure(false);await click("Refresh");assert(text().includes("renamed current"), "Report refresh did not recover");
  native.set_persistence_failures(2);await click("Refresh");assert(text().includes("2 usage writes failed"), "Persistence failure report not observable");
  passed.push("Report read failure, explicit refresh recovery and usage write failure warning");

  await reset(); await render(<AgentSettings />); assert(text().includes("restores the archived provider"), "Archived key restoration is ambiguous");
  await click("Statistics"); assert((last_report().args.filter as { provider_id: string }).provider_id === "primary", "Provider report scope lost");
  assert(text().includes("Scope: Primary"), "Context label missing");await click("All history");assert(Object.keys(last_report().args.filter as object).length === 0, "Clear scope did not remove provider filter");
  passed.push("Provider contextual report and all-history reset use same report shell");
  await click("Providers");await click("Archive provider");assert(host.querySelector('[role="alertdialog"]')?.textContent?.includes("Model"), "Cascade model list missing");await click("Cancel");assert(!native.calls.some(c => c.command === "delete_provider"), "Cancelled archive mutated configuration");
  await click("Archive provider");native.failure(false,true);await click("Archive");assert(text().includes("fixture archive failed") && text().includes("Primary"), "Archive error or config retention missing");native.failure();await click("Archive");assert(!text().includes("Primary") && text().includes("Add Provider"), "Successful archive did not refresh active list");
  await click("Usage");assert(text().includes("renamed current") && text().includes("Archived"), "History lost after archive");
  passed.push("Archive cascade confirmation/cancel/error/retry keeps history visible and refreshes active list");
  await reset();await render(<AgentSettings />);await click("Disable");assert(native.calls.some(c => c.command === "set_configuration_enabled" && c.args.entity === "provider" && c.args.enabled === false), "Provider disable mapping incorrect");await click("Enable");
  await click("Models");const picker=host.querySelector("select")!;await settle(() => {picker.value="primary";picker.dispatchEvent(new Event("change",{bubbles:true}));});await click("Statistics");assert((last_report().args.filter as { model_id:string }).model_id === "model", "Model scope lost");
  await click("Models");const again=host.querySelector("select")!;await settle(() => {again.value="primary";again.dispatchEvent(new Event("change",{bubbles:true}));});await click("Disable");assert(native.calls.some(c => c.command === "set_configuration_enabled" && c.args.entity === "model" && c.args.enabled === false), "Model disable mapping incorrect");
  await click("Profiles");assert(text().includes("Configured model unavailable"), "Disabled configured model hidden without explanation");await click("Statistics");assert((last_report().args.filter as {agent_type:string}).agent_type==="summary", "Agent report scope lost");
  passed.push("Provider/model enable lifecycle, disabled route visibility and model/agent contextual scope");
  await reset();await render(<AgentSettings />);await click("Models");const model_provider=host.querySelector("select")!;await settle(() => {model_provider.value="primary";model_provider.dispatchEvent(new Event("change",{bubbles:true}));});await click("Add Model");
  await settle(() => {const inputs=[...host.querySelectorAll<HTMLInputElement>('input:not([type="checkbox"])')];for(const [index,value] of ["Fresh capability","fresh-api"].entries()){Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value")!.set!.call(inputs[index],value);inputs[index].dispatchEvent(new Event("input",{bubbles:true}));}const checks=host.querySelectorAll<HTMLInputElement>('input[type="checkbox"]');checks[0].click();checks[2].click();});
  await click("Save");const add=native.calls.find(c => c.command==="add_model")!;assert(add.args.supportsSummary===false && add.args.supportsTranslation===true && add.args.supportsTagging===false,"Create form capability choices did not reach production IPC");
  await click("Profiles");const selects=[...host.querySelectorAll<HTMLSelectElement>("select")];assert(!selects[0].textContent?.includes("Fresh capability") && selects.some(s=>s.textContent?.includes("Fresh capability")),"Create capabilities not reflected by actual picker filtering");
  passed.push("New/reactivated model form explicit capabilities reach real wrapper and compatible pickers");
  await reset(); native.delay_report(); await render(<UsageStats />);await select("Group by","agent");assert(text().includes("Summary"), "Current report not loaded");await settle(() => native.release_report());assert(!text().includes("renamed current"), "Old delayed report overwrote current dimension");
  passed.push("Delayed old report cannot overwrite a new scope/dimension");
  await reset();await fetch("/result",{method:"POST",body:JSON.stringify({passed})});
}
run().catch(async error => {await fetch("/result",{method:"POST",body:JSON.stringify({passed,error:String(error),stack:error.stack})});});
