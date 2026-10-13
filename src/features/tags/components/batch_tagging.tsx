import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Channel } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Dialog, DialogTitle } from "@/components/ui/dialog";
import { apply_tag_batch, batch_tag, cancel_agent_task, count_tag_batch, discard_tag_batch, load_tag_batch, get_agent_profile } from "@/lib/tauri_bindings";
import type { BatchScope, BatchSnapshot } from "@/types";
import { invalidate_tag_queries } from "../lib/tag_queries";

export function BatchTagging({ on_active_change }: { on_active_change?: (active: boolean) => void } = {}) {
  const { t } = useTranslation();
  const query_client = useQueryClient();
  const [open, set_open] = useState(false);
  const [days, set_days] = useState("30");
  const [unread, set_unread] = useState(false);
  const [skip_tagged, set_skip_tagged] = useState(true);
  const [skip_applied, set_skip_applied] = useState(true);
  const [concurrency, set_concurrency] = useState(3);
  const [confirmed, set_confirmed] = useState(false);
  const [run, set_run] = useState<BatchSnapshot | null>(null);
  const [request_id, set_request_id] = useState<string | null>(null);
  const request_ref = useRef<string | null>(null);
  const [phase, set_phase] = useState("");
  const [decisions, set_decisions] = useState<Record<string, boolean>>({});
  const [error, set_error] = useState<string | null>(null);
  const [busy, set_busy] = useState(false);
  const [progress, set_progress] = useState({ processed: 0, total: 0, failed: 0 });
  // Scope dates stay fixed throughout preview and start; never recalculate on render.
  const [scope, set_scope] = useState<BatchScope>({ from: new Date(Date.now() - 30 * 86400000).toISOString(), to: new Date().toISOString(), unread_only: false, skip_tagged: true, skip_applied: true });
  useEffect(() => {
    set_scope({ from: days === "all" ? null : new Date(Date.now() - Number(days) * 86400000).toISOString(), to: new Date().toISOString(), unread_only: unread, skip_tagged, skip_applied });
    set_confirmed(false);
  }, [days, unread, skip_tagged, skip_applied]);
  const { data: count, error: count_error } = useQuery({ queryKey: ["tag_batch_count", scope], queryFn: () => count_tag_batch(scope), enabled: open && !run && !request_id, refetchOnWindowFocus: "always" });
  const { data: profile } = useQuery({ queryKey: ["agent_profile", "tagging"], queryFn: () => get_agent_profile("tagging"), enabled: open });
  const { data: saved, error: load_error, isFetching: recovering, isSuccess: recovered, refetch: recover } = useQuery({ queryKey: ["tag_batch"], queryFn: () => load_tag_batch(), enabled: open && !request_id, refetchInterval: run?.status === "running" ? 1000 : false });
  useEffect(() => { if (saved && !request_ref.current) set_run(saved); }, [saved]);
  useEffect(() => () => { if (request_ref.current) void cancel_agent_task("tagging", request_ref.current).catch(() => {}); }, []);
  async function refresh(id: string) {
    const snapshot = await load_tag_batch(id);
    set_run(snapshot);
    set_decisions({});
    await query_client.invalidateQueries({ queryKey: ["tag_batch"] });
  }
  async function start() {
    const id = crypto.randomUUID();
    request_ref.current = id; set_request_id(id); set_phase("starting"); set_error(null); set_progress({ processed: 0, total: count ?? 0, failed: 0 });
    const channel = new Channel<string>();
    channel.onmessage = (event) => {
      if (request_ref.current !== id) return;
      try {
        const message = JSON.parse(event);
        if (message.request_id === id && message.type === "task_state") set_phase(message.state);
        if (message.request_id === id && message.type === "batch_progress") set_progress(message);
        if (message.type === "notice") set_error(message.message);
      } catch { /* Ignore malformed lifecycle messages. */ }
    };
    try { set_run(await batch_tag(scope, id, confirmed, concurrency, channel)); }
    catch (e) { if (!String(e).includes("Agent task cancelled:")) set_error(String(e)); }
    finally {
      if (request_ref.current === id) { request_ref.current = null; set_request_id(null); set_phase(""); await refresh(id).catch((e) => set_error(String(e))); }
    }
  }
  async function stop() {
    const id = request_id ?? run?.id;
    if (!id) return;
    set_phase("cancelling");
    try { await cancel_agent_task("tagging", id); }
    catch (e) { set_error(String(e)); set_phase("running"); }
  }
  async function apply() {
    if (!run) return;
    set_busy(true); set_error(null);
    try {
      const kept = run.proposals.filter((p) => decisions[p.normalized_name] === true).map((p) => p.normalized_name);
      const discarded = run.proposals.filter((p) => decisions[p.normalized_name] === false).map((p) => p.normalized_name);
      const result = await apply_tag_batch(run.id, kept, discarded);
      set_run({ ...run, status: "done", result });
      invalidate_tag_queries(query_client);
      await query_client.invalidateQueries({ queryKey: ["tag_batch"] });
    } catch (e) { set_error(String(e)); await refresh(run.id).catch(() => {}); }
    finally { set_busy(false); }
  }
  async function discard() {
    if (!run) return;
    set_busy(true);
    try { await discard_tag_batch(run.id); set_run(null); set_decisions({}); await query_client.invalidateQueries({ queryKey: ["tag_batch"] }); }
    catch (e) { set_error(String(e)); }
    finally { set_busy(false); }
  }
  const active = !!request_id || run?.status === "running" || run?.status === "ready" || busy;
  useEffect(() => { on_active_change?.(active); return () => on_active_change?.(false); }, [active, on_active_change]);
  const reviewed = run?.proposals.every((p) => decisions[p.normalized_name] !== undefined);
  return <>
    <Button variant="outline" size="sm" onClick={() => set_open(true)}>{t("tagging.batch")}</Button>
    <Dialog open={open} onOpenChange={(value) => { if (!active) { set_open(value); if (!value) { set_run(null); set_error(null); } } }}>
      <div className="space-y-3 overflow-y-auto p-5">
        <DialogTitle>{t("tagging.batch")}</DialogTitle>
        {(error || count_error || load_error) && <p role="alert" className="text-sm text-destructive">{error ?? String(count_error ?? load_error)}</p>}
        {load_error && <Button variant="outline" onClick={() => void recover()}>{t("readerPane.retryFullArticle")}</Button>}
        {!run && !request_id && <>
          <label className="block text-sm">{t("tagging.scope")} <select value={days} onChange={(e) => set_days(e.target.value)} className="border rounded bg-background p-1">
            {[7,30,90,180,365].map((n) => <option key={n} value={n}>{t("tagging.pastDays", { count: n })}</option>)}<option value="all">{t("tagging.allEntries")}</option>
          </select></label>
          {[[unread, set_unread, "unreadScope"], [skip_tagged, set_skip_tagged, "skipTagged"], [skip_applied, set_skip_applied, "skipApplied"]].map(([checked, setter, key]) => <label key={String(key)} className="block text-sm"><input type="checkbox" checked={Boolean(checked)} onChange={(e) => (setter as (value: boolean) => void)(e.target.checked)} /> {t(`tagging.${key}`)}</label>)}
          <label className="block text-sm">{t("tagging.concurrency")} <input type="number" min="1" max="5" value={concurrency} onChange={(e) => set_concurrency(Math.max(1, Math.min(5, Number(e.target.value) || 1)))} className="w-16 rounded border bg-background p-1" /></label>
          <p>{t("tagging.candidateCount", { count: count ?? 0 })}</p>
          <p className="text-xs text-muted-foreground">{scope.from ?? t("tagging.allEntries")} → {scope.to}</p>
          {!profile?.primary_model_id && <p>{t("tagging.configureBatch")}</p>}
          {(count ?? 0) > 100 && <label className="block text-sm"><input type="checkbox" checked={confirmed} onChange={(e) => set_confirmed(e.target.checked)} /> {t("tagging.largeConfirm")}</label>}
          <Button disabled={!recovered || recovering || !!load_error || !!saved || !profile?.primary_model_id || !count || !!count_error || ((count ?? 0) > 100 && !confirmed)} onClick={start}>{t("tagging.startBatch")}</Button>
        </>}
        {(request_id || run?.status === "running") && <>
          <p role="status">{t(phase === "waiting" ? "agentTask.waiting" : phase === "cancelling" ? "agentTask.cancelling" : "agentTask.running")} {progress.processed} / {progress.total || run?.total} · {t("tagging.failedCount", { count: progress.failed })}</p>
          <Button onClick={stop} disabled={phase === "cancelling"}>{t("tagging.stopReview")}</Button>
        </>}
        {run && <>
          <p>{t("tagging.batchStats", { processed: run.processed, total: run.total, succeeded: run.succeeded, failed: run.failures.length })}</p>
          <p className="text-xs text-muted-foreground">{run.scope.from ?? t("tagging.allEntries")} → {run.scope.to ?? ""}</p>
          {run.failures.length > 0 && <details><summary>{t("tagging.failures")}</summary>{run.failures.map((f) => <p key={f.entry_id} className="break-words text-xs">#{f.entry_id}: {f.error}</p>)}</details>}
        </>}
        {run?.status === "ready" && <>
          <p className="text-sm">{t("tagging.reviewRequired")}</p>
          {run.proposals.map((p) => <div key={p.normalized_name} className="flex flex-wrap items-center gap-2 border-b py-2 text-sm">
            <span className="flex-1">{p.name} ({p.hits})</span>
            <Button size="sm" variant={decisions[p.normalized_name] === true ? "default" : "outline"} onClick={() => set_decisions((d) => ({ ...d, [p.normalized_name]: true }))}>{t("tagging.keep")}</Button>
            <Button size="sm" variant={decisions[p.normalized_name] === false ? "default" : "outline"} onClick={() => set_decisions((d) => ({ ...d, [p.normalized_name]: false }))}>{t("tagging.discard")}</Button>
          </div>)}
          <div className="flex flex-wrap gap-2">
            <Button variant="ghost" disabled={busy} onClick={() => set_decisions(Object.fromEntries(run.proposals.map((p) => [p.normalized_name, true])))}>{t("tagging.keepAll")}</Button>
            <Button variant="ghost" disabled={busy} onClick={() => set_decisions(Object.fromEntries(run.proposals.map((p) => [p.normalized_name, false])))}>{t("tagging.discardAll")}</Button>
            <Button variant="outline" disabled={busy} onClick={discard}>{t("tagging.abortBatch")}</Button>
            <Button disabled={!reviewed || busy} onClick={apply}>{t("tagging.confirmApply")}</Button>
          </div>
        </>}
        {run?.result && <p role="status">{t("tagging.applyResult", { count: run.result.tags_assigned, deleted: run.result.skipped_deleted, changed: run.result.skipped_changed })}</p>}
        <Button variant="ghost" disabled={active} onClick={() => { set_open(false); set_run(null); }}>{t("common.close")}</Button>
      </div>
    </Dialog>
  </>;
}
