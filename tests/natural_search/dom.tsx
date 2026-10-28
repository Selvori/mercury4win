import { StrictMode, act, useState } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { I18nextProvider } from "react-i18next";
import i18next from "i18next";
import en from "../../src/i18n/en.json";
import zh from "../../src/i18n/zh-Hans.json";
import { SearchControls } from "../../src/features/entry/components/search_controls";
import { search_filter_identity } from "../../src/features/entry/lib/search_filter";
import type { EntryFilter } from "../../src/types";
import { calls, requests } from "./mock_transport";

(globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
const results: string[] = [];
const host = document.getElementById("app")!;
const root = createRoot(host);
const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity }, mutations: { retry: false } } });
let current: EntryFilter;
let replace: (f: EntryFilter) => void;
export function Harness() {
  const [filter, set_filter] = useState<EntryFilter>({ search: "old", feed_id: 2, tag_ids: [3], has_note: true, date_from: "2026-01-01" });
  current = filter; replace = set_filter;
  // EntryList merges navigation defaults before options; property order is not identity.
  return <SearchControls filter={{ feed_id: undefined, tag_ids: [], starred_only: false, ...filter }} current_scope={{ feed_id: 2 }} on_change={set_filter} />;
}
const pause = () => new Promise((r) => setTimeout(r, 10));
async function settle(action: () => unknown = () => {}) { await act(async () => { action(); await pause(); }); await act(async () => { await pause(); }); }
function assert(v: unknown, message: string): asserts v { if (!v) throw new Error(message); }
function button(text: string) { const b = [...document.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === text || b.getAttribute("aria-label") === text); assert(b, `Missing button ${text}`); return b; }
const t = (key: string) => i18next.t(key);
const click = (key: string) => settle(() => button(t(key)).click());
function input(el: HTMLInputElement, text: string) {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(el, text);
  el.dispatchEvent(new Event("input", { bubbles: true }));
}
const form = () => host.querySelector<HTMLFormElement>(`form[aria-label="${t("search.natural.title")}"]`)!;
const sentence = (text: string) => settle(() => input(form().querySelector("input")!, text));
const submit = () => settle(() => form().dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })));
const last = () => requests.at(-1)!;
async function run() {
  await i18next.init({ lng: "zh-Hans", fallbackLng: "en", resources: { en: { translation: en }, "zh-Hans": { translation: zh } } });
  await settle(() => root.render(<StrictMode><I18nextProvider i18n={i18next}><QueryClientProvider client={client}><Harness /></QueryClientProvider></I18nextProvider></StrictMode>));
  await settle(() => input(host.querySelector<HTMLInputElement>('input[type="search"]')!, "ordinary Rust"));
  await click("search.saveView");
  await settle(() => document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  assert(requests.length === 0, "ordinary typing and opening save must never call AI");
  await click("search.natural.title");
  await sentence("找最近一周我收藏的 Rust 文章"); assert(requests.length === 0, "natural typing must not call AI");
  await submit(); const first = last();
  assert(first.args.sentence === "找最近一周我收藏的 Rust 文章" && first.args.requestId, "IPC request casing");
  assert(current!.search === "ordinary Rust", "pending parse changed active conditions");
  const normalized: EntryFilter = { search: "Rust", recent_days: 7, starred_only: true, feed_id: null, tag_ids: [], is_read: null, has_note: null, date_from: null, date_to: null, unread_only: false, exclude_title: false, search_field: "all", sort: "newest", tag_mode: "any" };
  await settle(() => first.resolve(normalized));
  assert(JSON.stringify(current!) === JSON.stringify(normalized), "replace must not inherit old source, tags, notes or dates");
  assert(host.querySelector('input[type="number"]')?.getAttribute("max") === "36500", "successful parse must expose existing editable conditions");
  assert(host.textContent!.includes(t("search.natural.applied")), "success explanation missing");
  await click("search.saveView");
  const name = document.querySelector<HTMLInputElement>('input[maxlength="80"]')!;
  await settle(() => input(name, "Rolling Rust"));
  await settle(() => name.closest("form")!.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })));
  assert(search_filter_identity(calls.find((c) => c.command === "save_smart_view")!.args.filter) === search_filter_identity(normalized), "smart view must save normalized conditions unchanged");
  results.push("explicit submit / whole-filter replacement / editable conditions / normalized smart-view IPC");

  await sentence("unknown source"); await submit();
  await settle(() => last().reject({ code: "unknownSource", detail: "Ghost" }));
  assert(current!.search === "Rust" && host.textContent!.includes("Ghost"), "parse failure damaged valid conditions");
  await submit(); await settle(() => last().reject({ code: "configuration", detail: "" }));
  assert(host.textContent!.includes(t("search.natural.errors.configuration")), "missing AI must explain configuration");
  await settle(() => input(host.querySelector<HTMLInputElement>('input[type="search"]')!, "still local"));
  assert(current!.search === "still local", "ordinary search blocked without AI");
  results.push("failure / missing configuration keep effective conditions and ordinary keyword search");

  await submit(); const cancelled = last(); await click("note.cancel");
  await settle(() => cancelled.resolve({ search: "cancelled stale" }));
  assert(current!.search === "still local", "late cancelled response applied");
  assert(calls.some((c) => c.command === "cancel_agent_task" && c.args.taskType === "search_filter" && c.args.requestId === cancelled.args.requestId), "backend cancellation missing");
  await submit(); const old = last(); await submit(); const latest = last();
  await settle(() => replace({ search: current.search, ...current! }));
  assert(form().getAttribute("aria-busy") === "true", "equivalent object key reordering cancelled active work");
  await settle(() => old.resolve({ search: "old stale" }));
  assert(form().getAttribute("aria-busy") === "true", "old finally cleared newer request loading");
  await settle(() => latest.resolve({ search: "newest", recent_days: 7 }));
  assert(current!.search === "newest", "latest response missing");
  results.push("cancel and consecutive submit fence late results and preserve new loading state");

  await submit(); const manual = last(); await settle(() => replace({ search: "manually changed" }));
  await settle(() => manual.resolve({ search: "wrong" })); assert(current!.search === "manually changed", "manual edit overwritten");
  await submit(); const editing = last(); await sentence("new draft");
  await settle(() => editing.resolve({ search: "wrong" })); assert(current!.search === "manually changed", "draft edit overwritten");
  await submit(); const hidden = last(); await click("search.natural.title");
  await settle(() => hidden.resolve({ search: "hidden wrong" })); assert(current!.search === "manually changed", "unmounted response applied");
  results.push("manual filter edits / draft edits / panel unmount invalidate pending responses");

  await settle(() => i18next.changeLanguage("en")); await click("search.natural.title");
  assert(form().querySelector("label input") && form().querySelector('button[type="submit"]'), "English labelled keyboard form missing");
  await sentence("unsupported reading history"); await submit();
  await settle(() => last().reject({ code: "unsupported", detail: "Reading dates are unavailable" }));
  assert(host.querySelector('[role="alert"]')?.textContent?.includes("Reading dates"), "actionable unsupported condition feedback missing");
  assert(!host.innerHTML.includes("<script>"), "unsafe markup in form");
  results.push("Chinese / English accessible labels and actionable unsupported-condition feedback");
  await settle(() => root.unmount()); client.clear();
}
run().then(() => fetch("/result", { method: "POST", body: JSON.stringify({ passed: results.length, results }) })).catch((e) => fetch("/result", { method: "POST", body: JSON.stringify({ error: String(e), stack: e.stack, results }) }));
