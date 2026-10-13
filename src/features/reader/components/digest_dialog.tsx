import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { get_note, load_latest_summary, preview_digest, export_digest, export_multi_digest } from "@/lib/tauri_bindings";
import { useNoteDraftStore } from "@/stores/note_draft_store";
import type { DigestOptions } from "@/types";

// Mount per confirmed selection. Options and late async responses stay in that session.
export function DigestDialog({ entry_ids, multiple = false, on_close }: { entry_ids: number[]; multiple?: boolean; on_close: () => void }) {
  const { t } = useTranslation();
  const [options, set_options] = useState<DigestOptions>({ include_body: false, include_note: false, include_summary: false });
  const [template, set_template] = useState(multiple ? "multiple_markdown" : "single_markdown");
  const [ready, set_ready] = useState(multiple);
  const [defaults_error, set_defaults_error] = useState("");
  const [defaults_attempt, set_defaults_attempt] = useState(0);
  const [busy, set_busy] = useState(false);
  const [status, set_status] = useState("");
  const [failure, set_failure] = useState("");
  const alive = useRef(true);
  const drafts = useNoteDraftStore((s) => s.drafts);
  const ids_key = JSON.stringify(entry_ids);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  useEffect(() => {
    let current = true;
    if (!multiple) {
      set_defaults_error("");
      const ids: number[] = JSON.parse(ids_key);
      Promise.all([get_note(ids[0]), load_latest_summary(ids[0])]).then(([note, summary]) => {
        if (current) { set_options({ include_body: false, include_note: !!note?.markdown.trim(), include_summary: !!summary?.result.complete && !!summary?.result.text?.trim() }); set_ready(true); }
      }).catch((error) => { if (current) set_defaults_error(String(error)); });
    }
    return () => { current = false; };
  }, [ids_key, multiple, defaults_attempt]);
  const preview = useQuery({ queryKey: ["digest_preview", ids_key, template, options],
    queryFn: () => preview_digest(entry_ids, template, options), enabled: ready, retry: false });
  async function save() {
    if (!preview.data || preview.isFetching || busy) return;
    const shown = preview.data.content;
    set_busy(true); set_status(""); set_failure("");
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const extension = preview.data.format === "text" ? "txt" : "md";
      const path = await save({ defaultPath: `mercury_digest.${extension}`, filters: [{ name: preview.data.format === "text" ? t("digest.plainText") : "Markdown", extensions: [extension] }] });
      if (!alive.current) return;
      if (!path) { set_status(t("digest.cancelled")); return; }
      if (multiple) await export_multi_digest(entry_ids, template, path, options, shown);
      else await export_digest(entry_ids[0], template, path, options, shown);
      if (alive.current) set_status(t("digest.saved", { path }));
    } catch (error) {
      if (alive.current) { set_failure(String(error)); void preview.refetch(); }
    } finally { if (alive.current) set_busy(false); }
  }
  return <Dialog open onOpenChange={(open) => { if (!open && !busy) on_close(); }} className="max-w-3xl">
    <DialogContent className="overflow-y-auto">
      <DialogHeader><DialogTitle>{t(multiple ? "digest.multiple" : "digest.title")}</DialogTitle></DialogHeader>
      <p className="mb-3 text-sm text-muted-foreground">{t("digest.savedOnly")}</p>
      {entry_ids.some((id) => drafts[id] !== undefined) && <p role="status" className="mb-3 text-sm">{t("digest.unsavedNote")}</p>}
      {!multiple && <label className="mb-3 flex items-center gap-2 text-sm">{t("digest.format")}
        <select value={template} disabled={busy} onChange={(e) => { set_template(e.target.value); set_status(""); }} className="rounded border border-border bg-background p-1">
          <option value="single_markdown">Markdown</option><option value="single_text">{t("digest.plainText")}</option>
        </select>
      </label>}
      <div className="mb-3 flex flex-wrap gap-3">{(["body", "note", "summary"] as const).map((field) => {
        const key = `include_${field}` as keyof DigestOptions;
        return <label key={field} className="flex items-center gap-2 text-sm"><input type="checkbox" disabled={!ready || busy} checked={options[key]}
          onChange={(e) => { set_options({ ...options, [key]: e.target.checked }); set_status(""); }} />{t(`digest.${field}`)}</label>;
      })}</div>
      {defaults_error && <div><p role="alert">{defaults_error}</p><Button variant="outline" onClick={() => set_defaults_attempt((n) => n + 1)}>{t("digest.retryDefaults")}</Button></div>}
      {preview.isFetching && <p role="status">{t("common.loading")}</p>}
      {preview.isError && <p role="alert" className="text-destructive">{String(preview.error)}</p>}
      {preview.data && !preview.isFetching && <>
        {!!preview.data.missing.length && <div role="status" className="mb-3 text-sm"><p>{t("digest.missing")}</p>
          <ul>{preview.data.missing.map((item) => <li key={`${item.entry_id}-${item.field}`}>{item.title}: {t(`digest.${item.field}`)}</li>)}</ul>
        </div>}
        <pre aria-label={t("digest.preview")} className="max-h-[40vh] overflow-auto whitespace-pre-wrap break-words rounded border border-border bg-muted p-3 text-xs">{preview.data.content}</pre>
      </>}
      {failure && <p role="alert" className="mt-3 text-sm text-destructive">{failure}</p>}
      {status && <p role="status" className="mt-3 break-all text-sm">{status}</p>}
      <DialogFooter><Button variant="outline" disabled={busy} onClick={on_close}>{t("digest.close")}</Button>
        <Button variant="outline" disabled={!ready || busy || preview.isFetching} onClick={() => { set_failure(""); set_status(""); void preview.refetch(); }}>{t("digest.refresh")}</Button>
        <Button disabled={!ready || busy || !preview.data || preview.isFetching || preview.isError} onClick={() => void save()}>{t(busy ? "digest.exporting" : "digest.export")}</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>;
}
