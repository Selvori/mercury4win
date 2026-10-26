import { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReaderPane } from "../../src/features/reader/components/reader_pane";
import { useNoteDraftStore } from "../../src/stores/note_draft_store";
import type { Entry } from "../../src/types";
import * as ipc from "./mock_bindings";

// Render the production ReaderPane and panels, including their real hooks,
// React Query observers and Zustand draft store. Only external IPC is mocked.
(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const host = document.getElementById("app")!;
const root = createRoot(host);
const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
const invalidated: unknown[][] = [];
const invalidate = client.invalidateQueries.bind(client);
client.invalidateQueries = (filters, options) => {
  if (filters?.queryKey) invalidated.push([...filters.queryKey]);
  return invalidate(filters, options);
};
const results: string[] = [];
const pause = () => new Promise((resolve) => setTimeout(resolve, 15));
async function settle(action: () => unknown = () => {}) {
  await act(async () => { action(); await pause(); });
  await act(async () => { await pause(); });
}
function assert(value: unknown, message: string): asserts value {
  if (!value) throw new Error(message);
}
const text = () => host.textContent ?? "";
const reader_text = () => {
  const frame = host.querySelector<HTMLIFrameElement>("iframe");
  return text() + (frame?.contentDocument?.documentElement.outerHTML ?? frame?.srcdoc ?? "");
};
function button(label: string) {
  const found = [...host.querySelectorAll<HTMLButtonElement>("button")].find((item) => item.textContent?.trim() === label);
  assert(found, `button missing: ${label}`);
  return found;
}
async function click(label: string) { await settle(() => button(label).click()); }
async function panel(name: string) {
  const found = host.querySelector<HTMLButtonElement>(`button[title="reader.${name}"]`);
  assert(found, `panel missing: ${name}`);
  await settle(() => found.click());
}
async function render(entry_id: number | null) {
  const entry = { id: entry_id, title: `Article ${entry_id}`, url: null } as Entry;
  await settle(() => root.render(<StrictMode><QueryClientProvider client={client}>
    {entry_id === null ? null : <ReaderPane entry={entry} />}
  </QueryClientProvider></StrictMode>));
}
async function input(selector: string, value: string) {
  const element = host.querySelector<HTMLInputElement | HTMLTextAreaElement>(selector);
  assert(element, `input missing: ${selector}`);
  const prototype = element instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  await settle(() => {
    Object.getOwnPropertyDescriptor(prototype, "value")!.set!.call(element, value);
    element.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
function sent(stream: { channel: { onmessage: (message: string) => void } }, message: object) {
  stream.channel.onmessage(JSON.stringify(message));
}
function pass(message: string) { results.push(message); }

async function run() {
  await render(1);
  await panel("notes");
  await click("note.edit");
  await input("textarea", "draft A");
  await render(2);
  assert(text().includes("saved B") && !host.querySelector("textarea"), "A draft leaked into B");
  await click("note.edit");
  assert(host.querySelector("textarea")!.value === "saved B", "B editor did not load B note");
  await input("textarea", "draft B");
  await click("note.save");
  const save_b = ipc.saves.at(-1)!;
  assert(save_b.entry_id === 2 && save_b.markdown === "draft B", "B save wrote the wrong owner/text");
  await render(1);
  assert(host.querySelector("textarea")!.value === "draft A", "A unsaved draft was discarded");
  await settle(() => save_b.task.resolve({ entry_id: 2, markdown: "draft B" }));
  assert(host.querySelector("textarea")!.value === "draft A", "B save completion closed A editor");
  assert(ipc.notes.get(1)!.markdown === "saved A", "saving B changed A note");
  pass("A→B save uses B snapshot; delayed completion preserves A draft");

  await panel("notes"); // close
  await panel("notes"); // reopen
  assert(host.querySelector("textarea")!.value === "draft A", "close/reopen lost draft");
  await render(null);
  await render(1);
  await panel("notes");
  assert(host.querySelector("textarea")!.value === "draft A", "reader unmount lost session draft");
  pass("closing panel/reader and reopening restores article draft");

  await click("note.save");
  const save_a = ipc.saves.at(-1)!;
  const saves_before_reopen = ipc.saves.length;
  await panel("notes");
  await panel("notes");
  assert(button("note.saving").disabled, "reopened panel allowed a duplicate pending save");
  await settle(() => button("note.saving").click());
  assert(ipc.saves.length === saves_before_reopen, "duplicate save was submitted");
  await input("textarea", "newer A edits");
  await settle(() => save_a.task.resolve({ entry_id: 1, markdown: "draft A" }));
  assert(host.querySelector("textarea")!.value === "newer A edits", "save discarded newer edits");
  await click("note.cancel");
  assert(useNoteDraftStore.getState().drafts[1] === undefined && !host.querySelector("textarea"), "cancel did not discard current draft");
  pass("saving preserves newer edits; cancel discards only current article draft");

  await click("note.edit");
  await input("textarea", "same text A");
  await click("note.save");
  const same_text_save = ipc.saves.at(-1)!;
  await input("textarea", "temporary change");
  await input("textarea", "same text A");
  await settle(() => same_text_save.task.resolve({ entry_id: 1, markdown: "same text A" }));
  assert(host.querySelector("textarea")!.value === "same text A", "old save discarded a newer revision with same text");
  await click("note.cancel");
  pass("close/reopen blocks duplicate save; revision protects later edits even with equal text");

  await click("note.edit");
  await input("textarea", "unsaved after failure");
  await click("note.save");
  await settle(() => ipc.saves.at(-1)!.task.reject(new Error("mock save failure")));
  assert(host.querySelector("textarea")!.value === "unsaved after failure", "failed save discarded draft");
  assert(host.querySelector('[role="alert"]')?.textContent?.includes("note.saveFailed"), "save failure was invisible");
  await panel("notes");
  await panel("notes");
  assert(host.querySelector('[role="alert"]')?.textContent?.includes("mock save failure"), "reopened panel lost save error");
  await click("note.save");
  await settle(() => ipc.saves.at(-1)!.task.resolve({ entry_id: 1, markdown: "unsaved after failure" }));
  assert(!host.querySelector('[role="alert"]') && !host.querySelector("textarea"), "successful retry did not clear saved draft/error");
  pass("failed save keeps draft and visible error through reopening; retry succeeds");

  await panel("summary");
  await click("summary.summarize");
  const summary_a = ipc.summaries.at(-1)!;
  await settle(() => sent(summary_a, { type: "chunk", content: "A summary" }));
  assert(text().includes("A summary"), "A summary was not projected");
  await render(2);
  assert(!text().includes("A summary") && !text().includes("summary.generating"), "summary state crossed articles");
  await click("summary.summarize");
  const summary_b = ipc.summaries.at(-1)!;
  await settle(() => {
    sent(summary_a, { type: "chunk", content: "STALE SUMMARY" });
    sent(summary_a, { type: "done" });
    summary_a.task.reject(new Error("STALE SUMMARY FAILURE"));
  });
  assert(!text().includes("STALE") && text().includes("summary.generating"), "old summary changed B loading/error/content");
  await settle(() => { sent(summary_b, { type: "chunk", content: "B summary" }); sent(summary_b, { type: "done" }); summary_b.task.resolve(); });
  assert(text().includes("B summary") && !text().includes("summary.generating"), "B summary did not finish");
  pass("summary switch isolates old chunks/done/Promise failure from B run");

  await panel("summary");
  await panel("summary");
  await click("summary.summarize");
  const summary_closed = ipc.summaries.at(-1)!;
  await panel("summary");
  await panel("summary");
  await click("summary.summarize");
  await settle(() => { sent(summary_closed, { type: "error", message: "STALE CLOSED SUMMARY" }); summary_closed.task.resolve(); });
  assert(!text().includes("STALE") && text().includes("summary.generating"), "closed summary affected reopened panel");
  await settle(() => { const current = ipc.summaries.at(-1)!; sent(current, { type: "done" }); current.task.resolve(); });
  pass("summary close/reopen rejects earlier error callback");

  await render(1);
  const initial_translation_count = ipc.translations.length;
  await panel("translation");
  const translation_a = ipc.translations.at(-1)!;
  assert(ipc.translations.length === initial_translation_count + 1 && !text().includes("translation.translate"), "selection did not start one hidden translation");
  await settle(() => { sent(translation_a, { type: "segment", index: 0, text: "A translation" }); sent(translation_a, { type: "progress", segment: 3, total: 9 }); });
  await render(2);
  assert(!reader_text().includes("A translation") && !reader_text().includes("3/9"), "translation state crossed articles");
  assert(host.querySelector('button[title="reader.translation"]')?.getAttribute("aria-pressed") === "false", "new article did not reset to original mode");
  await panel("translation");
  const translation_b = ipc.translations.at(-1)!;
  await settle(() => {
    sent(translation_a, { type: "segment", index: 0, text: "STALE TRANSLATION" });
    sent(translation_a, { type: "progress", segment: 8, total: 9 });
    sent(translation_a, { type: "error", error: "STALE TRANSLATION ERROR" });
    sent(translation_a, { type: "done" });
    translation_a.task.reject(new Error("STALE TRANSLATION FAILURE"));
  });
  assert(!reader_text().includes("STALE") && !reader_text().includes("8/9") && ipc.translations.at(-1) === translation_b, "old translation affected B");
  await settle(() => { sent(translation_b, { type: "segment", index: 0, text: "B translation" }); sent(translation_b, { type: "segment", index: 1, text: "B second" }); sent(translation_b, { type: "done" }); translation_b.task.resolve(); });
  assert(reader_text().includes("B translation") && !text().includes("translation.translate"), "B translation did not finish silently");
  pass("translation switch resets segments/progress and rejects old events/failure");

  await render(1); await panel("translation");
  const translation_replaced = ipc.translations.at(-1)!;
  await settle(() => sent(translation_replaced, { type: "error", error: "retryable segment error" }));
  const retry_count = ipc.translations.length;
  await settle(() => client.invalidateQueries({queryKey:["translation_document",1]}));
  assert(ipc.translations.length === retry_count, "segment error started another executor while still running");
  await settle(() => translation_replaced.task.reject(new Error("terminal translation failure")));
  assert(ipc.translations.length === retry_count, "terminal error retried without user intent");
  await panel("translation"); await panel("translation");
  const translation_retry = ipc.translations.at(-1)!;
  assert(ipc.translations.length === retry_count + 1, "explicit reselection did not retry once");
  await settle(() => {
    sent(translation_replaced, { type: "segment", index: 0, text: "STALE SAME ARTICLE" });
    sent(translation_replaced, { type: "done" });
    translation_replaced.task.reject(new Error("STALE SAME ARTICLE FAILURE"));
  });
  assert(!reader_text().includes("STALE") && ipc.translations.at(-1) === translation_retry, "replaced request affected same-article retry");
  await settle(() => { sent(translation_retry, { type: "done" }); translation_retry.task.resolve(); });
  pass("same-article translation retry ignores replaced request events/failure");

  await panel("translation"); await panel("translation");
  const translation_closed = ipc.translations.at(-1)!;
  await panel("translation"); await panel("translation");
  const reopened_translation = ipc.translations.at(-1)!;
  await settle(() => { sent(translation_closed, { type: "segment", index: 0, text: "STALE CLOSED TRANSLATION" }); sent(translation_closed, { type: "done" }); translation_closed.task.resolve(); });
  assert(!reader_text().includes("STALE") && ipc.translations.at(-1) === reopened_translation, "closed translation affected reopened run");
  await settle(() => { sent(reopened_translation, { type: "done" }); reopened_translation.task.resolve(); });
  pass("translation close/reopen rejects earlier stream");
  await render(1);
  await panel("tags");
  await input("input", "manual A");
  await click("tagging.suggestTags");
  const suggestion_a = ipc.suggestions.at(-1)!;
  await render(2);
  assert(host.querySelector("input")!.value === "", "manual tag input crossed articles");
  await click("tagging.suggestTags");
  const suggestion_b = ipc.suggestions.at(-1)!;
  await settle(() => suggestion_a.task.resolve(["STALE TAG"]));
  assert(!text().includes("STALE") && text().includes("tagging.analyzingArticle"), "old tags changed B");
  await settle(() => suggestion_b.task.resolve(["B suggested"]));
  assert(text().includes("B suggested"), "B suggestions missing");
  pass("tag suggestions/manual input isolated across articles");

  await click("tagging.suggestTags");
  const suggestion_closed = ipc.suggestions.at(-1)!;
  await panel("tags");
  await panel("tags");
  await click("tagging.suggestTags");
  await settle(() => suggestion_closed.task.reject(new Error("STALE TAG FAILURE")));
  assert(!text().includes("STALE") && text().includes("tagging.analyzingArticle"), "old tagging error affected reopened panel");
  await settle(() => ipc.suggestions.at(-1)!.task.resolve(["B fresh"]));
  pass("tag close/reopen rejects earlier Promise failure");

  await click("B fresh");
  const add_b = ipc.tag_writes.at(-1)!;
  await render(1);
  await click("tagging.suggestTags");
  await settle(() => ipc.suggestions.at(-1)!.task.resolve(["B fresh"]));
  invalidated.length = 0;
  await settle(() => add_b.task.resolve({}));
  assert(add_b.entry_id === 2 && text().includes("B fresh"), "old tag write affected A suggestions/owner");
  assert(invalidated.some((key) => key[0] === "entry_detail" && key[1] === 2), "tag write invalidated wrong entry");
  assert(!invalidated.some((key) => key[0] === "entry_detail" && key[1] === 1), "old write invalidated A detail");
  pass("delayed tag write uses original article and invalidates original cache");

  const applied_tag = [...host.querySelectorAll("span")].find((item) => item.textContent?.trim() === "existing 1");
  assert(applied_tag?.querySelector("button"), "applied tag removal button missing");
  await settle(() => applied_tag.querySelector("button")!.click());
  const remove_a = ipc.tag_deletes.at(-1)!;
  await render(2);
  invalidated.length = 0;
  await settle(() => remove_a.task.resolve());
  assert(remove_a.entry_id === 1 && remove_a.tag_id === 1, "tag removal lost original owner");
  assert(invalidated.some((key) => key[0] === "entry_detail" && key[1] === 1), "removal failed to invalidate original cache");
  assert(!invalidated.some((key) => key[0] === "entry_detail" && key[1] === 2), "removal invalidated new article cache");
  pass("delayed tag removal uses original article/tag and original cache");

  // Switching display/closing panels must not resolve or reject the old IPC:
  // cancellation belongs to the separate runtime lifecycle work item.
  await render(1);
  await panel("summary");
  await click("summary.summarize");
  const continuing = ipc.summaries.at(-1)!;
  let terminal = false;
  continuing.task.promise.finally(() => { terminal = true; });
  await render(2);
  await panel("summary");
  assert(!terminal, "display isolation implicitly cancelled backend work");
  await settle(() => continuing.task.resolve());
  pass("display switch/close leaves backend request pending until its own completion");

  await settle(() => root.unmount());
  return results;
}

run().then(async (passed) => {
  document.getElementById("result")!.textContent = `PASS ${passed.length}\n${passed.join("\n")}`;
  await fetch("/result", { method: "POST", body: JSON.stringify({ passed }) });
}).catch(async (error) => {
  document.getElementById("result")!.textContent = `FAIL after ${results.length}: ${error.stack}`;
  await fetch("/result", { method: "POST", body: JSON.stringify({ passed: results, error: String(error.stack) }) });
});
