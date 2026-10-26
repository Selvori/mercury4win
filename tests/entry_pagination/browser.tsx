import { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { EntryList } from "../../src/features/entry/components/entry_list";
import type { Entry } from "../../src/types";
import * as transport from "./mock_transport";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const host = document.getElementById("app")!;
const root = createRoot(host);
const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
const results: string[] = [];
let selected: Entry | null = null;
const pause = () => new Promise((resolve) => setTimeout(resolve, 20));
async function settle(action: () => unknown = () => {}) {
  await act(async () => { action(); await pause(); });
  await act(async () => { await pause(); });
}
function assert(value: unknown, message: string): asserts value { if (!value) throw new Error(message); }
const rows = () => [...host.querySelectorAll<HTMLDivElement>('[role="button"]')];
const titles = () => rows().map((r) => r.querySelector("p")?.textContent);
function button(label: string) {
  const b = [...host.querySelectorAll<HTMLButtonElement>("button")].find((e) => e.textContent?.trim() === label);
  assert(b, `missing button: ${label}`); return b;
}
async function click(label: string) { await settle(() => button(label).click()); }
async function render(feed_id: number | null | undefined, tag_ids?: number[]) {
  await settle(() => root.render(<StrictMode><QueryClientProvider client={client}>
    <EntryList feed_id={feed_id} tag_ids={tag_ids} selected_entry_id={selected?.id ?? null} on_select_entry={(e) => { selected = e; }} />
  </QueryClientProvider></StrictMode>));
}
async function run() {
  await render(1);
  assert(rows().length === 100 && host.textContent?.includes("entry.entriesCount:237"), "first page/count wrong");
  assert(transport.calls.find((call) => call.command === "load_entries")!.args.cursor === null && transport.calls.find((call) => call.command === "load_entries")!.args.feedId === 1, "first IPC args wrong");
  transport.hold_once();
  await click("entry.loadMore");
  assert(rows().length === 100 && button("entry.loadingMore").disabled, "pending page hid rows or allows duplicate clicks");
  const call = transport.calls.findLast((call) => call.command === "load_entries")!;
  const boundary = transport.ordered()[99];
  assert(call.args.cursor.id === boundary.id && call.args.cursor.published_at === boundary.published_at, "wrapper lost composite cursor");
  await settle(() => transport.pending!());
  assert(rows().length === 200 && host.textContent?.includes("entry.entriesCount:237"), "next page/count wrong");
  await click("entry.loadMore");
  assert(JSON.stringify(titles()) === JSON.stringify(transport.ordered().map((e) => e.title)), "237 rows order/identity wrong");
  assert(!host.textContent?.includes("entry.loadMore"), "terminal page still offers more");
  await settle(() => rows().at(-1)!.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
  assert(selected?.id === transport.ordered().at(-1)!.id, "last page row keyboard inaccessible");
  results.push("237 entries accumulate in exact order; count stays 237; composite IPC cursor, loading guard, terminal page and last-row keyboard selection pass");

  await settle(() => rows().at(-1)!.querySelector<HTMLButtonElement>('button[title="entry.markRead"]')!.click());
  assert(rows().length === 237 && transport.entries.find((e) => e.id === selected!.id)!.is_read, "mutation refetch lost pages/read state");
  assert(!rows().at(-1)!.querySelector('button[title="entry.markRead"]'), "mark read display stale");
  await settle(() => rows().at(-1)!.querySelectorAll<HTMLButtonElement>("button")[1].click());
  assert(rows().length === 236 && host.textContent?.includes("entry.entriesCount:236"), "delete refetch/count wrong");
  assert(new Set(titles()).size === rows().length, "delete/refetch duplicated boundary");
  results.push("mark read/delete invalidate and sequentially refresh every loaded page, without duplicate boundaries; total updates");

  await render(2);
  assert(rows().length === 1 && titles()[0] === "Other scope", "scope cache leaked");
  await settle(() => client.removeQueries({ predicate: (query) => query.queryKey[0] === "entries" && (query.queryKey[1] as { feed_id?: number })?.feed_id === 1 }));
  await render(1);
  transport.fail_once();
  await click("entry.loadMore");
  assert(rows().length === 100 && host.querySelector('[role="alert"]'), "page error hid rows/no alert");
  await click("entry.retry");
  assert(rows().length === 200 && !host.querySelector('[role="alert"]'), "page retry failed");
  results.push("scope switching excludes old rows; next-page failure preserves 100 rows, exposes alert and retries same cursor");

  transport.hold_once();
  await click("entry.loadMore");
  await render(2);
  await settle(() => transport.pending!());
  assert(rows().length === 1 && titles()[0] === "Other scope", "old in-flight page mixed into new scope");
  results.push("delayed old page completion cannot enter the newly selected feed");

  transport.fail_once();
  await render(3);
  assert(host.querySelector('[role="alert"]') && !host.textContent?.includes("entry.noEntries"), "first failure reported empty");
  await click("entry.retry");
  assert(host.textContent?.includes("entry.noEntries") && !host.querySelector('[role="alert"]'), "empty result retry wrong");
  await render(undefined);
  assert(host.textContent?.includes("entry.selectFeedOrTag"), "unset scope wrong");
  results.push("initial failure/retry differs from successful empty list; unset scope does not fetch");

  await render(undefined, [7]);
  assert(transport.calls.findLast((call) => call.command === "load_entries")!.args.feedId === null && JSON.stringify(transport.calls.findLast((call) => call.command === "load_entries")!.args.tagIds) === "[7]", "tag scope IPC mapping wrong");
  results.push("tag scope query preserves existing filter mapping through production IPC wrapper");
}
run().then(() => finish({ passed: results })).catch((error) => finish({ passed: results, error: String(error.stack || error) }));
async function finish(result: object) {
  await settle(() => root.unmount()); client.clear();
  await fetch("/result", { method: "POST", body: JSON.stringify(result) });
}
