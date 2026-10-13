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
const pause = () => new Promise((resolve) => setTimeout(resolve, 20));
async function settle(action = () => {}) {
  await act(async () => { action(); await pause(); });
  await act(async () => { await pause(); });
}
function assert(value: unknown, message: string): asserts value {
  if (!value) throw new Error(message);
}
async function run() {
  const full = '<html><head></head><body>RECOVERED_BODY</body></html>';
  reader_responses.set(8, [new Error("Article HTTP error: 503"), full]);
  const entry = { id: 8, title: "Retry article", url: null } as Entry;
  await settle(() => root.render(<QueryClientProvider client={client}><ReaderPane entry={entry} /></QueryClientProvider>));
  assert(host.textContent?.includes("readerPane.failedToLoad"), "translated failure missing");
  assert(host.textContent?.includes("Article HTTP error: 503"), "actual failure detail missing");
  const retry = [...host.querySelectorAll("button")].find((button) => button.textContent?.includes("readerPane.retryFullArticle"));
  assert(retry, "error retry control missing");
  await settle(() => retry.click());
  assert(reader_requests.filter((id) => id === 8).length === 2, "retry used wrong article");
  assert(!host.textContent?.includes("readerPane.failedToLoad"), "recovered article retains error");
  assert(host.querySelector("iframe")?.srcdoc === full, "recovered body missing");
  results.push("production ReaderPane displays error and retry; retry restores same article without stale error");
  const frame = host.querySelector("iframe")!;
  assert(frame.getAttribute("sandbox") === "allow-same-origin", "iframe script permission changed");
  const doc = frame.contentDocument!;
  assert(doc, "isolated iframe document unavailable");
  doc.body.innerHTML = '<a href="file:///C:/secret">local</a><a href="javascript:alert(1)">script</a><a href="#section">section</a>';
  await settle(() => frame.dispatchEvent(new Event("load")));
  for (const anchor of doc.querySelectorAll("a")) {
    const ev = new MouseEvent("click", { bubbles: true, cancelable: true });
    anchor.dispatchEvent(ev);
    assert(ev.defaultPrevented === !anchor.getAttribute("href")!.startsWith("#"), "unexpected link navigation policy");
  }
  results.push("iframe keeps existing sandbox, blocks local/script navigation, and preserves fragment links");
  await settle(() => root.unmount());
  return results;
}
run().then(async (passed) => {
  await fetch("/result", { method: "POST", body: JSON.stringify({ passed }) });
}).catch(async (error) => {
  await fetch("/result", { method: "POST", body: JSON.stringify({ passed: results, error: String(error.stack) }) });
});
