import { useEffect, useRef, useState } from "react";
import { Channel } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { recent_topic_search, search_local_topic, stop_topic_search } from "@/lib/tauri_bindings";
import type { EntryFilter } from "@/types";
import type { SearchFilterError } from "@/types/search_filter";
import type { TopicSearchSession, TopicSearchResume } from "@/types/topic_search";
import { search_filter_identity } from "../lib/search_filter";

const errors = ["input", "configuration", "metadata", "invalid", "unsupported", "unknownSource", "ambiguousSource", "unknownTag", "ambiguousTag", "provider", "cancelled", "conflict", "invalidJudgment", "storage", "expired", "changed", "stale", "rateLimited", "timeout"];
const signature = (sentence: string, filter: EntryFilter) => JSON.stringify([sentence.trim(), search_filter_identity({ ...filter, search: null, sort: "relevance" })]);
interface Props {
  filter: EntryFilter;
  scope?: TopicSearchSession | null;
  compact?: boolean;
  describe_scope?: (filter: EntryFilter) => string;
  on_apply: (scope: TopicSearchSession | null) => void;
  on_restore?: (filter: EntryFilter) => void;
  on_exit: () => void;
}

export function NaturalFilter({ filter, scope, compact = false, describe_scope, on_apply, on_restore, on_exit }: Props) {
  const { t } = useTranslation();
  const [sentence, set_sentence] = useState(scope?.sentence ?? "");
  const [busy, set_busy] = useState(false);
  const [message, set_message] = useState<SearchFilterError | null>(null);
  const [work, set_work] = useState<TopicSearchResume | null>(scope?.scan ? { sentence: scope.sentence, manual: scope.manual, progress: scope.scan, session: scope } : null);
  const [recent, set_recent] = useState<TopicSearchResume | null>(null);
  const [cached, set_cached] = useState(false);
  const active = useRef<string | null>(null);
  const last_request = useRef<string | null>(null);
  const mounted = useRef(false);
  const previous_scope = useRef(scope);
  const current_snapshot = useRef<TopicSearchSession | null>(null);
  const latest_apply = useRef(on_apply); latest_apply.current = on_apply;
  const identity = search_filter_identity({ ...filter, search: null, sort: "relevance" });
  const current_signature = useRef(signature(sentence, filter)); current_signature.current = signature(sentence, filter);

  const stop = (mode: "paused" | "cancelled" = "paused", visible = true) => {
    const id = active.current; active.current = null;
    if (!id) return;
    if (visible) {
      set_busy(false);
      set_work((value) => value ? { ...value, progress: { ...value.progress, status: mode } } : value);
      if (mode === "cancelled") {
        set_message({ code: "cancelled", detail: "" });
        latest_apply.current(previous_scope.current ?? null);
      }
    }
    void stop_topic_search(id, mode).then((progress) => {
      if (mounted.current && last_request.current === id && !active.current && progress) set_work((value) => value ? { ...value, progress } : value);
    }).catch(() => { /* The runtime cancellation can race a completed task. */ });
  };
  useEffect(() => {
    mounted.current = true;
    void recent_topic_search().then((value) => { if (mounted.current) set_recent(value); }).catch(() => {});
    return () => { mounted.current = false; stop("paused", false); };
  }, []);
  useEffect(() => { stop("paused", false); set_busy(false); return () => stop("paused", false); }, [identity]);

  const submit = async (resume: string | null = null, force = false) => {
    if (!sentence.trim()) return;
    stop("paused", false);
    const id = crypto.randomUUID(); active.current = id; last_request.current = id;
    const expected = current_signature.current;
    const topic = sentence.trim(); const manual = { ...filter, search: null, sort: filter.sort ?? "relevance" } as EntryFilter;
    previous_scope.current = scope;
    current_snapshot.current = null;
    set_busy(true); set_message(null); set_cached(false);
    const channel = new Channel<string>();
    channel.onmessage = (raw) => {
      if (active.current !== id || current_signature.current !== expected) return;
      try {
        const event = JSON.parse(raw);
        if (event.type === "scan_progress" && event.progress) set_work((prior) => ({ sentence: topic, manual, progress: event.progress, session: prior && signature(prior.sentence, prior.manual) === expected ? prior.session : null }));
        if (event.type === "search_snapshot" && event.session?.scan) {
          const session = event.session as TopicSearchSession;
          current_snapshot.current = session;
          set_work({ sentence: topic, manual, progress: session.scan!, session });
          // Quick matches are readable while AI verifies them. Snapshots may
          // grow OR shrink; a bulk click still captures one immutable token.
          latest_apply.current(session);
        }
      } catch { /* Malformed progress cannot change the active query. */ }
    };
    try {
      const result = await search_local_topic(topic, manual, id, channel, resume, force);
      if (active.current !== id || current_signature.current !== expected) return;
      if (result.session.scan) set_work({ sentence: topic, manual, progress: result.session.scan, session: result.session });
      set_cached(result.cached);
      if (result.error) {
        set_message(result.error);
        latest_apply.current(result.session);
      } else latest_apply.current(result.session);
    } catch (error) {
      if (active.current !== id || current_signature.current !== expected) return;
      const candidate = error as Partial<SearchFilterError> | null;
      set_message({ code: candidate && errors.includes(candidate.code ?? "") ? candidate.code! : "provider", detail: typeof candidate?.detail === "string" ? candidate.detail : "" });
      if (current_snapshot.current) latest_apply.current(current_snapshot.current);
      else if (previous_scope.current) latest_apply.current(previous_scope.current);
    } finally {
      if (active.current === id) { active.current = null; set_busy(false); }
    }
  };
  const progress = work?.progress ?? scope?.scan;
  const inspected = work?.session ?? scope;
  const dirty = work ? signature(work.sentence, work.manual) !== signature(sentence, filter) : !!scope && signature(scope.sentence, scope.manual) !== signature(sentence, filter);
  const status = !busy && progress?.status === "running" ? "paused" : progress?.status;
  const restore = (saved: TopicSearchResume) => {
    stop("paused"); set_message(null); set_sentence(saved.sentence); on_restore?.(saved.manual);
    set_work(saved); set_recent(null);
    if (saved.session) latest_apply.current(saved.session);
  };
  return <section className="space-y-2 rounded-lg border border-border bg-muted/20 p-2" aria-label={t("search.natural.title")}>
    {!compact && <form className="space-y-2" aria-label={t("search.natural.title")} aria-busy={busy} onSubmit={(e) => { e.preventDefault(); void submit(); }}>
      <label className="block space-y-1 text-xs"><span>{t("search.natural.title")}</span>
        <Input value={sentence} maxLength={1000} placeholder={t("search.natural.placeholder")} onChange={(e) => { stop("paused"); set_message(null); set_sentence(e.target.value); }} />
      </label>
      <div className="flex flex-wrap gap-1">
        <Button type="submit" size="sm" variant="outline" disabled={!sentence.trim()}>{t(busy ? "search.natural.restart" : "search.natural.submit")}</Button>
        {busy && <>
          <Button type="button" size="sm" variant="ghost" onClick={() => stop("paused")}>{t("search.natural.pause")}</Button>
          <Button type="button" size="sm" variant="ghost" onClick={() => stop("cancelled")}>{t("note.cancel")}</Button>
        </>}
      </div>
      <p className="text-xs text-muted-foreground">{t("search.natural.recentScope")}</p>
      <details className="text-xs leading-relaxed text-muted-foreground">
        <summary className="cursor-pointer">{t("search.natural.dataDisclosure")}</summary>
        <p className="mt-1">{t("search.natural.hint")}</p>
      </details>
    </form>}
    {!busy && recent && !work && <div className="space-y-1 text-xs">
      <p>{t("search.natural.savedCheck", { topic: recent.sentence })}</p>
      <Button type="button" size="sm" variant="outline" onClick={() => restore(recent)}>{t("search.natural.restore")}</Button>
    </div>}
    {compact && busy && <div className="flex flex-wrap gap-1">
      <Button type="button" size="sm" variant="ghost" onClick={() => stop("paused")}>{t("search.natural.pause")}</Button>
      <Button type="button" size="sm" variant="ghost" onClick={() => stop("cancelled")}>{t("note.cancel")}</Button>
    </div>}
    {scope && <p className="text-xs font-medium">{(scope.preliminary ?? 0)>0
      ? t("search.natural.quickResults", { count: scope.matches, pending: scope.preliminary, verified: scope.scan?.matches ?? 0 })
      : t("search.natural.resultScope", { count: scope.matches })}</p>}
    {busy && scope?.matches===0 && <p className="text-xs text-muted-foreground">{t("search.natural.quickEmpty")}</p>}
    {dirty && <p role="status" className="text-xs text-muted-foreground">{t("search.natural.draft")}</p>}
    {progress && <div className="space-y-1 text-xs leading-relaxed" aria-live="polite">
      {progress.started_at && <p>{t("search.natural.frozenAt", { time: new Date(progress.started_at).toLocaleString() })}</p>}
      <p role="status">{t(`search.natural.status_${status}`)} · {t(`search.natural.phase_${progress.phase}`)}</p>
      <p>{t("search.natural.titleCoverage", { checked: progress.title_screened, total: progress.total, model: progress.title_model_reviewed })}</p>
      {!!progress.title_fallback && <p>{t("search.natural.titleFallback", { count: progress.title_fallback })}</p>}
      <p>{t("search.natural.deepCoverage", { pending: progress.further_pending, full: progress.full_verified, summary: progress.summary_only, notes: progress.note_verified })}</p>
      <p>{t("search.natural.partCoverage", { done: progress.full_parts_done, total: progress.full_parts_total })}</p>
      <p>{t("search.natural.otherCoverage", { excluded: progress.excluded, insufficient: progress.insufficient, changed: progress.changed, failed: progress.failed, pending: progress.pending })}</p>
      {progress.cache_hits > 0 && <p>{t("search.natural.cacheItems", { count: progress.cache_hits })}</p>}
      {status === "complete" && <p>{t(progress.matches > 0 ? "search.natural.finished" : progress.insufficient + progress.changed > 0 ? "search.natural.noConfirmed" : "search.natural.noRelevant")}</p>}
      {!busy && progress.pending > 0 && progress.scan_id && <Button type="button" size="sm" variant="outline" disabled={dirty} onClick={() => void submit(progress.scan_id)}>{t("search.natural.resume")}</Button>}
    </div>}
    <details className="text-[11px] leading-relaxed text-muted-foreground"><summary className="cursor-pointer">{t("search.natural.scopeDetails")}</summary><p className="mt-1">{t("search.natural.boundary")}</p></details>
    {cached && <p className="text-xs">{t("search.natural.cached")}</p>}
    {work?.session && work.session.id !== scope?.id && work.session.matches > 0 && <Button type="button" size="sm" variant="outline" onClick={() => latest_apply.current(work.session!)}>{t("search.natural.viewPartial", { count: work.session.matches })}</Button>}
    {inspected && <>
      {!busy && <Button type="button" size="sm" variant="ghost" disabled={!sentence.trim()} onClick={() => void submit(null, true)}>{t("search.natural.refresh")}</Button>}
      <details className="text-xs leading-relaxed"><summary className="cursor-pointer">{t("search.natural.understood")}</summary>
        <p className="mt-1 break-words">{inspected.plan.intent}</p>
        {describe_scope && <p className="mt-1 break-words">{describe_scope(inspected.filter)}</p>}
        <p className="mt-1">{t("search.natural.routeHint")}</p>
        <ul className="mt-1 space-y-1">{inspected.plan.routes.map((route, index) => <li key={index} className="break-words">{route.language}: {[...new Set(route.groups.flat())].join(" / ")}</li>)}</ul>
      </details>
    </>}
    {message && <p role="alert" className="break-words text-xs text-destructive">{t(`search.natural.errors.${message.code}`)}{message.detail && ` ${message.detail}`}</p>}
    <Button type="button" size="sm" variant="ghost" onClick={() => { stop("paused"); on_exit(); }}>{t("search.natural.exit")}</Button>
  </section>;
}
