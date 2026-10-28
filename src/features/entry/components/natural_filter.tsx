import { useEffect, useId, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
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
  on_apply: (scope: TopicSearchSession | null) => void;
}

export function NaturalFilter({ filter, scope, compact = false, on_apply }: Props) {
  const { t } = useTranslation();
  const id = useId();
  const [sentence, set_sentence] = useState(scope?.sentence ?? "");
  const [busy, set_busy] = useState(false);
  const [message, set_message] = useState<SearchFilterError | null>(null);
  const [work, set_work] = useState<TopicSearchResume | null>(scope?.scan ? { sentence: scope.sentence, manual: scope.manual, progress: scope.scan, session: scope } : null);
  const [recent, set_recent] = useState<TopicSearchResume | null>(null);
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
    set_busy(true); set_message(null);
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
  const dirty = work ? signature(work.sentence, work.manual) !== signature(sentence, filter) : !!scope && signature(scope.sentence, scope.manual) !== signature(sentence, filter);
  const resume_progress = progress && !dirty ? progress
    : recent && signature(recent.sentence, recent.manual) === current_signature.current ? recent.progress : null;
  const resumable = resume_progress && resume_progress.status !== "complete" && resume_progress.pending > 0 ? resume_progress.scan_id : null;
  const error_text = message ? t(`search.natural.errors.${message.code}`) : undefined;
  if (compact) return null;
  return <form className="space-y-2" aria-label={t("search.natural.title")} aria-busy={busy}
    onSubmit={(e) => { e.preventDefault(); if (busy) stop("paused"); else void submit(resumable); }}>
    <Input aria-label={t("search.natural.title")} aria-invalid={!!message} aria-describedby={message ? `${id}-error` : undefined}
      title={error_text} className={message ? "border-destructive" : undefined} value={sentence} maxLength={1000}
      placeholder={t("search.natural.placeholder")} onChange={(e) => { stop("paused"); set_message(null); set_sentence(e.target.value); }} />
    <Button type="submit" size="sm" variant="outline" title={error_text} disabled={!sentence.trim()}>
      {busy && <Loader2 className="size-3.5 animate-spin" aria-hidden="true" />}
      {t(busy ? "search.natural.stop" : message ? "search.natural.retrySearch" : "search.natural.submit")}
    </Button>
    {message && <span id={`${id}-error`} role="alert" className="sr-only">{error_text}</span>}
  </form>;
}
