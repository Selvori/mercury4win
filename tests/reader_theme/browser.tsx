import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import en from "../../src/i18n/en.json";
import zh from "../../src/i18n/zh-Hans.json";
import { ReaderPane } from "../../src/features/reader/components/reader_pane";
import { useTheme } from "../../src/hooks/use_theme";
import { useAppStore } from "../../src/stores/app_store";
import type { Entry } from "../../src/types";
import * as native from "./mock_transport";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const env = window as typeof window & { initial_case: string; theme_media: { count: () => number; silently: (value: boolean) => void; change: (value: boolean) => void } };
// Dialogs render into body; keep queries inclusive of their portals.
const host = document.body;
const root = createRoot(document.getElementById("app")!);
const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
const passed: string[] = [];
const pause = () => new Promise((r) => setTimeout(r, 15));
async function settle(action: () => unknown = () => {}) { await act(async () => { action(); await pause(); }); await act(async () => { await pause(); }); }
function assert(value: unknown, message: string): asserts value { if (!value) throw new Error(message); }
function button(name: string) { const b = [...host.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.getAttribute("aria-label") === name || b.textContent?.trim() === name); assert(b, `Missing ${name}`); return b; }
function select(label: string) { const l = [...host.querySelectorAll("label")].find((l) => l.textContent === label); assert(l?.htmlFor, `Missing label ${label}`); const s = document.getElementById(l.htmlFor); assert(s instanceof HTMLSelectElement, `Missing select ${label}`); return s; }
async function choose(label: string, value: string) { await settle(() => { const s = select(label); s.value = value; s.dispatchEvent(new Event("change", { bubbles: true })); }); }
function frame_html() { const f = host.querySelector("iframe"); assert(f, "Missing iframe"); return f.srcdoc; }
function current_head() { return new DOMParser().parseFromString(frame_html(), "text/html").head.innerHTML; }
function latest_request() { return native.commands.findLast((c) => c.command === "build_reader_content")!.args; }
export function Shell({ entry }: { entry: number }) { useTheme(); return <ReaderPane entry={{ id: entry, title: `Article ${entry}`, url: null } as Entry} />; }
async function render(entry: number) { await settle(() => root.render(<StrictMode><QueryClientProvider client={client}><Shell entry={entry}/></QueryClientProvider></StrictMode>)); }

async function run() {
  await i18next.use(initReactI18next).init({ lng: "en", fallbackLng: "en", resources: { en: { translation: en }, "zh-Hans": { translation: zh } }, interpolation: { escapeValue: false } });
  const initial = useAppStore.getState();
  assert(initial.theme === (env.initial_case === "saved-dark" ? "dark" : "auto"), "Stored mode or invalid fallback changed");
  assert(initial.reader_style === (env.initial_case === "saved-auto" ? "paper" : "classic"), "Stored style or default changed");
  assert(initial.font_family === (env.initial_case === "saved-auto" ? "serif" : env.initial_case === "saved-dark" ? "system" : "sans-serif"), "Old font preference ignored");
  assert(initial.font_size === (env.initial_case === "saved-auto" ? 22 : 16), "Old size preference ignored");
  env.theme_media.silently(false); await render(1);
  assert(env.theme_media.count() === 1, "StrictMode duplicate listener or missing subscription");
  assert(useAppStore.getState().system_theme === "light", "Mount did not re-sample system appearance");
  assert(document.documentElement.classList.contains("dark") === (env.initial_case === "saved-dark"), "Root initial appearance wrong");
  passed.push("real localStorage old keys, style persistence/invalid fallback, initial system re-sample and StrictMode subscription");

  await settle(() => { useAppStore.getState().set_theme("auto"); useAppStore.getState().set_font_family("serif"); useAppStore.getState().set_font_size(22); });
  await settle(() => button("Reader Settings").click());
  assert(host.querySelector('[role="dialog"]')?.getAttribute("aria-labelledby"), "Settings dialog unlabeled");
  await choose("Reader style", "paper");
  assert(latest_request().themeId === "paper-light" && frame_html().includes("#f5efe6"), "Paper selection did not use actual Rust render");
  assert(latest_request().entryId === 1 && latest_request().fontFamily === "serif" && latest_request().fontSize === 22, "CamelCase wrapper or font retention wrong");
  assert(localStorage.getItem("mercury_reader_style") === "paper", "Style did not persist");
  passed.push("accessible production settings selects Paper via actual wrapper; Rust-rendered CSS, font and size are retained");

  await settle(() => env.theme_media.change(true));
  assert(document.documentElement.classList.contains("dark") && frame_html().includes("#1f1a16"), "System event did not update root and iframe");
  assert(latest_request().themeId === "paper-dark", "System event used wrong Reader key");
  await choose("Appearance", "light"); const before = native.commands.filter((c) => c.command === "build_reader_content").length;
  await settle(() => env.theme_media.change(false)); await settle(() => env.theme_media.change(true));
  assert(!document.documentElement.classList.contains("dark") && frame_html().includes("#f5efe6"), "Explicit light overridden");
  assert(native.commands.filter((c) => c.command === "build_reader_content").length === before, "Explicit mode unnecessarily rebuilt on system change");
  await choose("Appearance", "dark"); await settle(() => env.theme_media.change(false));
  assert(document.documentElement.classList.contains("dark") && frame_html().includes("#1f1a16"), "Explicit dark overridden");
  await choose("Appearance", "auto");
  assert(!document.documentElement.classList.contains("dark") && latest_request().themeId === "paper-light", "Return to auto ignored current system");
  assert(localStorage.getItem("mercury_theme") === "auto" && env.theme_media.count() === 1, "Old auto persistence/subscription incompatible");
  passed.push("system changes update root/iframe; explicit light/dark ignore events; auto resumes immediately with one listener");

  await settle(() => button("System").click()); await settle(() => button("Increase font size").click());
  assert(latest_request().fontFamily === "system" && latest_request().fontSize === 23 && frame_html().includes("Noto Sans") && frame_html().includes("font-size: 23px"), "Font/size settings lost");
  assert(button("System").getAttribute("aria-pressed") === "true" && host.querySelector('input[type="range"]')?.id, "Font controls unlabeled");
  await settle(() => useAppStore.getState().set_font_size(24)); assert(button("Increase font size").disabled, "Upper size limit lost");
  await settle(() => useAppStore.getState().set_font_size(12)); assert(button("Decrease font size").disabled, "Lower size limit lost");
  await settle(() => useAppStore.getState().set_font_size(22));
  assert(localStorage.getItem("mercury_font_family") === "system" && localStorage.getItem("mercury_font_size") === "22", "Old font storage keys changed");
  await settle(() => button("Close").click());
  passed.push("existing three fonts, labeled size slider, 12/24 bounds and persisted font keys work with Paper");

  await settle(() => button("Translation").click());
  assert(frame_html().includes("Saved 中文 translation"), "Saved bilingual result unavailable");
  assert(current_head().includes("#f5efe6") && current_head().includes("Noto Sans"), "Saved translation canonical head overwrote Reader head");
  await settle(() => env.theme_media.change(true));
  assert(frame_html().includes("Saved 中文 translation") && current_head().includes("#1f1a16"), "Bilingual mode lost current dark head or saved translation");
  await settle(() => { useAppStore.getState().set_font_family("sans-serif"); useAppStore.getState().set_font_size(24); });
  assert(current_head().includes("Inter") && current_head().includes("font-size: 24px") && frame_html().includes("Saved 中文 translation"), "Bilingual font change stale");
  assert(host.querySelector("iframe")?.getAttribute("sandbox") === "allow-same-origin", "Sandbox expanded");
  passed.push("production bilingual document follows theme and fonts using current Reader head and stored translation; sandbox preserved");

  native.defer_next_reader(); await settle(() => env.theme_media.change(false));
  const old = native.pending.at(-1)!; assert(old.args.themeId === "paper-light", "Theme race not deferred");
  await settle(() => useAppStore.getState().set_reader_style("classic"));
  assert(current_head().includes("#ffffff") && frame_html().includes("Saved 中文 translation"), "New Classic render used old bilingual head");
  const current_html = frame_html(); await settle(() => old.resolve());
  assert(frame_html() === current_html, "Late Paper render overwrote Classic bilingual");
  native.defer_next_reader(); await render(2); const other = native.pending.at(-1)!;
  await render(3); assert(frame_html().includes("Reader article 3.") && !frame_html().includes("Saved 中文 translation"), "Article switch retained translation");
  await settle(() => other.resolve()); assert(frame_html().includes("Reader article 3.") && !frame_html().includes("Reader article 2."), "Late article response changed current iframe");
  assert(!native.commands.some((c) => c.command.startsWith("run_")), "Theme settings unexpectedly generated AI");
  passed.push("late theme and article requests cannot overwrite current iframe/bilingual; article resets original mode; no AI generation");

  native.defer_next_document(); await settle(() => button("Translation").click());
  const late_document = native.pending_documents.at(-1)!;
  assert(late_document, "Translation document race not deferred");
  await settle(() => { useAppStore.getState().set_reader_style("paper"); useAppStore.getState().set_theme("light"); });
  assert(current_head().includes("#f5efe6"), "Reader theme did not change while document pending");
  await settle(() => late_document.resolve());
  assert(current_head().includes("#f5efe6") && current_head().includes("Inter") && current_head().includes("font-size: 24px") && frame_html().includes("Saved 中文 translation"), "Late canonical translation head replaced current Reader appearance");
  assert(!native.commands.some((c) => c.command.startsWith("run_")), "Document retry generated AI");
  passed.push("late translation document with stale dark/serif head uses current light/font/size after resolution, without new AI");

  await settle(() => i18next.changeLanguage("zh-Hans")); await settle(() => button("阅读设置").click());
  assert(select("阅读样式").querySelector('option[value="paper"]')?.textContent === "纸张" && select("亮暗模式").querySelector('option[value="auto"]')?.textContent === "跟随系统", "Chinese Reader settings missing");
  await settle(() => root.unmount()); const system_before = useAppStore.getState().system_theme;
  assert(env.theme_media.count() === 0, "Appearance listener leaked on unmount");
  env.theme_media.change(true); assert(useAppStore.getState().system_theme === system_before, "Unmounted subscriber still updates store");
  client.clear();
  passed.push("actual English/Chinese resources label controls; unmount removes listener without subsequent store updates");
  return passed;
}
run().then(async (passed) => { await fetch("/result", { method: "POST", body: JSON.stringify({ passed }) }); }).catch(async (error) => { await fetch("/result", { method: "POST", body: JSON.stringify({ passed, error: String(error.stack) }) }); });
