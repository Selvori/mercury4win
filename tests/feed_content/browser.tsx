import { act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReaderPane } from "../../src/features/reader/components/reader_pane";
import type { Entry } from "../../src/types";
import { reader_requests, reader_responses } from "../reader_ownership/mock_bindings";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const host = document.getElementById("app")!;
const root = createRoot(host);
const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
const results: string[] = [];
const fallback = '<html><head><meta name="mercury-content-source" content="summary"></head><body>TEASER</body></html>';
const full = '<html><head></head><body>FULL_ARTICLE</body></html>';
const pause = () => new Promise((resolve) => setTimeout(resolve, 20));
async function settle(action = () => {}) {
  await act(async () => { action(); await pause(); });
  await act(async () => { await pause(); });
}
function assert(value: unknown, message: string): asserts value {
  if (!value) throw new Error(message);
}
async function render(id: number) {
  const entry = { id, title: `Article ${id}`, url: null } as Entry;
  await settle(() => root.render(<QueryClientProvider client={client}><ReaderPane entry={entry} /></QueryClientProvider>));
}
async function run() {
  reader_responses.set(1, [fallback, full]);
  await render(1);
  assert(host.querySelector('[role="status"]')?.textContent?.includes("readerPane.summaryFallback"), "fallback warning missing");
  assert(host.querySelector("iframe")?.srcdoc === fallback, "fallback body not displayed");
  const retry = [...host.querySelectorAll("button")].find((button) => button.textContent?.includes("readerPane.retryFullArticle"));
  assert(retry, "retry button missing");
  await settle(() => retry.click());
  assert(reader_requests.filter((id) => id === 1).length === 2, "retry did not request same article");
  assert(!host.querySelector('[role="status"]'), "successful full body retained fallback warning");
  assert(host.querySelector("iframe")?.srcdoc === full, "retry did not display full article");
  results.push("production ReaderPane labels fallback and retries same article; full response clears warning");
  reader_responses.set(2, [fallback]);
  reader_responses.set(3, [full]);
  await render(2);
  assert(host.querySelector('[role="status"]'), "second article fallback warning missing");
  await render(3);
  assert(!host.querySelector('[role="status"]'), "previous article warning leaked into full article");
  assert(host.querySelector("iframe")?.srcdoc === full, "article switch retained old fallback");
  results.push("article switch uses current response and does not inherit fallback warning");
  await settle(() => root.unmount());
  return results;
}
run().then(async (passed) => {
  await fetch("/result", { method: "POST", body: JSON.stringify({ passed }) });
}).catch(async (error) => {
  await fetch("/result", { method: "POST", body: JSON.stringify({ passed: results, error: String(error.stack) }) });
});
