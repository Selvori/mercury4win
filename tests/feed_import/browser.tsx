import React, { act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import { OpmlImport } from "@/features/feed/components/opml_import";
import en from "@/i18n/en.json";
import zh from "@/i18n/zh-Hans.json";
import * as transport from "./mock_transport";

(globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;
const host = document.getElementById("app")!;
const root = createRoot(host);
const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
const results: string[] = [];
function assert(condition: unknown, message: string): asserts condition { if (!condition) throw new Error(message); }
async function settle(action: () => void = () => {}) { await act(async () => { action(); await new Promise(r => setTimeout(r, 20)); }); }
function text() { return document.body.textContent!.replace(/\s+/g, " "); }
async function send(status: string, current: number, feed_url: string, error: string | null = null) {
  await settle(() => transport.progress!.onmessage({ status, current, total: 3, feed_url, title: `Feed ${current}`, error }));
}
async function run() {
  await i18next.use(initReactI18next).init({ lng: "en", fallbackLng: "en", resources: { en: { translation: en }, "zh-Hans": { translation: zh } }, interpolation: { escapeValue: false } });
  for (const language of ["en", "zh-Hans"]) {
    await act(async () => { await i18next.changeLanguage(language); });
    await act(async () => root.render(<QueryClientProvider client={client}><OpmlImport key={language} open on_close={() => {}} /></QueryClientProvider>));
    await settle(() => {
      const button = [...document.querySelectorAll("button")].find(b => b.textContent === i18next.t("opml.selectFile"));
      assert(button, "Missing import file button"); button.click();
    });
    await send("start", 0, "");
    await send("processing", 1, "https://feed.test/rss"); await send("added", 1, "https://feed.test/rss");
    await send("processing", 2, "https://feed.test/rss"); await send("skipped", 2, "https://feed.test/rss", "duplicate_feed");
    assert(text().includes(`${i18next.t("opml.added")}: 1`), "Repeated URL overwrote the original added row");
    assert(text().includes(`${i18next.t("opml.skipped")}: 1`), "Duplicate row not counted separately");
    assert(text().includes(i18next.t("opml.duplicateFeed")), "Duplicate explanation not localized");
    results.push(`${language}: repeated identical URL retains separate added/skipped rows and counts`);
    await send("processing", 3, "https://bad.test/rss"); await send("retrying", 3, "https://bad.test/rss");
    assert(text().includes(i18next.t("opml.retrying")), "Retry status missing");
    const close = [...document.querySelectorAll("button")].find(b => b.textContent === i18next.t("opml.importing"));
    assert(close?.disabled, "Dialog can close while retry is active");
    await send("error", 3, "https://bad.test/rss", "Feed HTTP error: 503 Service Unavailable");
    await send("done", 3, ""); await settle(() => transport.finish!());
    assert(text().includes(`${i18next.t("opml.added")}: 1`) && text().includes(`${i18next.t("opml.failed")}: 1`), "Final counts incorrect");
    assert(text().includes("503 Service Unavailable") && !text().includes(i18next.t("opml.retrying")), "Final error or retry terminal state incorrect");
    results.push(`${language}: retry once indication, close lock, final error and complete counters`);
  }
}
run().then(async () => { await act(async () => root.unmount()); client.clear(); }).then(() => fetch("/result", { method: "POST", body: JSON.stringify({ passed: results.length, results }) })).catch(e => fetch("/result", { method: "POST", body: JSON.stringify({ results, error: String(e), stack: e.stack }) }));
