// mercury4win/src/features/reader/components/reader_translation.tsx
// Translation controls and source-mapped bilingual preview

import { useState, useCallback, useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Channel } from "@tauri-apps/api/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Languages, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { run_translation, get_agent_profile, load_translation_result, load_translation_document, get_agent_fingerprint } from "@/lib/tauri_bindings";
import type { SourceSegment, TranslationDocument } from "@/types/agent";
import { compose_bilingual_document } from "../lib/bilingual_document";
import { useAgentRequest } from "../hooks/use_agent_request";

interface Props {
  entry_id: number;
  automatic?: boolean;
  reader_ready?: boolean;
  reader_html?: string;
  on_bilingual_html?: (entry_id: number, html: string | null, reader_html?: string) => void;
}

interface SegmentResult {
  index: number;
  text: string;
}

const empty_sources: SourceSegment[] = [];

export function ReaderTranslation({ entry_id, reader_html, on_bilingual_html, automatic = false, reader_ready = true }: Props) {
  const { t } = useTranslation();
  const [live_segments, set_segments] = useState<SegmentResult[] | null>(null);
  const query_client = useQueryClient();
  const [live_sources, set_sources] = useState<SourceSegment[] | null>(null);
  const [segment_errors, set_segment_errors] = useState<Record<string, string>>({});
  const [outcome, set_outcome] = useState<{ status: string; succeeded: number; total: number } | null>(null);
  const [live_document, set_live_document] = useState<TranslationDocument | null>(null);
  const [progress, set_progress] = useState({ current: 0, total: 0 });
  const [error, set_error] = useState<string | null>(null);
  const agent = useAgentRequest(entry_id, "translation");
  const { begin_request, is_pending: is_loading } = agent;
  const automatic_started = useRef(false);

  // Load user's language preference from agent settings
  const { data: profile, isFetching: profile_loading, error: profile_error } = useQuery({
    queryKey: ["agent_profile", "translation"],
    queryFn: () => get_agent_profile("translation"),
  });
  const target_language = profile?.target_language ?? "zh-Hans";

  const { data: fingerprint, error: configuration_error, isFetching: fingerprint_loading } = useQuery({
    queryKey: ["agent_fingerprint", "translation"], queryFn: () => get_agent_fingerprint("translation"), retry: false,
  });
  const { data: loaded_result, error: restore_error, isFetching: restore_loading, refetch: retry_result } = useQuery({
    queryKey: ["translation_result", entry_id, target_language, profile?.prompt_strategy, profile?.concurrency_degree, fingerprint],
    queryFn: () => load_translation_result(entry_id, target_language),
    enabled: !is_loading && fingerprint !== undefined,
    retry: false,
  });
  const { data: document, error: document_error, isFetching: document_loading, refetch: retry_document } = useQuery({
    queryKey: ["translation_document", entry_id, target_language, profile?.prompt_strategy, profile?.concurrency_degree, fingerprint],
    queryFn: () => load_translation_document(entry_id, target_language),
    enabled: !is_loading && fingerprint !== undefined,
    retry: false,
  });
  const current_document = live_document ?? document;
  const sources = live_sources ?? current_document?.source_segments ?? empty_sources;
  const saved = loaded_result && current_document && loaded_result.source_hash === current_document.source_hash
    && loaded_result.segment_ids?.join("/") === sources.map((source) => source.source_segment_id).join("/")
    ? loaded_result : null;
  const cannot_start = is_loading || profile_loading || fingerprint_loading || document_loading || restore_loading || !!profile_error || !!document_error || !!restore_error || !!configuration_error || !sources.length;
  const slot_identity = JSON.stringify([target_language, profile?.prompt_strategy, profile?.concurrency_degree, fingerprint]);
  useEffect(() => { set_segments(null); set_sources(null); set_live_document(null); set_outcome(null); set_error(null); set_segment_errors({}); }, [slot_identity, document?.source_hash]);
  const saved_segments = useMemo(() => saved?.segments.flatMap((text, index) => text === null ? [] : [{ index, text }]) ?? [], [saved]);
  const segments = live_segments ?? saved_segments;

  const translations = useMemo(() => sources.map((source) =>
    segments.find((segment) => segment?.index === source.order_index)?.text ?? null), [sources, segments]);
  const bilingual_html = useMemo(() => current_document && sources.length
    ? compose_bilingual_document(current_document.html, sources, translations, reader_html) : null,
    [current_document, sources, translations, reader_html]);
  useEffect(() => {
    on_bilingual_html?.(entry_id, bilingual_html, reader_html);
    return () => on_bilingual_html?.(entry_id, null);
  }, [entry_id, bilingual_html, reader_html, on_bilingual_html]);
  const completed = translations.filter(Boolean).length;
  const status = outcome?.status ?? (saved?.complete ? "complete" : saved && completed > 0 ? "partial" : saved ? "all_failed" : null);

  const start_translation = useCallback(async (source_segment_id?: string) => {
    if (cannot_start) return;
    const request = begin_request();
    let request_sources = sources;
    set_segments(source_segment_id ? saved_segments : []);
    set_segment_errors({});
    set_outcome(null);
    set_progress({ current: 0, total: 0 });
    set_error(null);

    const channel = new Channel<string>();
    channel.onmessage = (chunk) => {
      if (!request.is_current()) return;
      try {
        const msg = JSON.parse(chunk);
        if (request.handle_event(msg)) return;
        if (msg.type === "source_segments") {
          request_sources = msg.segments;
          set_sources(msg.segments);
          if (msg.html) set_live_document({ source_hash: msg.source_hash, html: msg.html, source_segments: msg.segments, result: null, segmenter_version: "reader-html-block-v3" });
        } else if (msg.type === "notice") {
          set_error(msg.message);
        } else if (msg.type === "outcome") {
          set_outcome({ status: msg.status, succeeded: msg.succeeded, total: msg.total });
        } else if (msg.type === "progress") {
          set_progress({ current: msg.segment, total: msg.total });
        } else if (msg.type === "segment") {
          if (msg.source_segment_id && request_sources[msg.index]?.source_segment_id !== msg.source_segment_id) return;
          set_segments((prev) => {
            const next = [...(prev ?? [])];
            next[msg.index] = { index: msg.index, text: msg.text };
            return next;
          });
        } else if (msg.type === "done") {
          // The invoke settles only after the executor releases its slot.
        } else if (msg.type === "error") {
          set_segment_errors((prev) => ({ ...prev, [msg.source_segment_id ?? String(msg.index)]: msg.error }));
          // A segment error does not end other in-flight segments.
        }
      } catch {
        // raw chunk — ignore for now
      }
    };

    try {
      await run_translation(request.entry_id, target_language, channel, request.request_id, source_segment_id);
      await query_client.invalidateQueries({ queryKey: ["translation_result", request.entry_id] });
      if (!request.is_current()) return;
      set_segments(null);
      set_sources(null);
      set_live_document(null);
      request.finish();
    } catch (e) {
      if (!request.is_current()) return;
      const cancelled = /^(?:Error: )?Agent task cancelled:/.test(String(e));
      if (!cancelled) set_error(String(e));
      set_segments(null);
      set_sources(null);
      set_live_document(null);
      request.finish(cancelled);
      void query_client.invalidateQueries({ queryKey: ["translation_result", request.entry_id] });
    }
  }, [begin_request, target_language, query_client, saved_segments, sources, cannot_start]);

  useEffect(() => {
    if (!automatic || automatic_started.current || !agent.status_ready) return;
    if (agent.background_running) {
      automatic_started.current = true;
      return;
    }
    if (!reader_ready || !profile || fingerprint === undefined || cannot_start) return;
    // Let StrictMode's effect cleanup discard its first activation. Record the
    // attempt before starting; failures/refetches never cause paid retry loops.
    const timer = setTimeout(() => {
      automatic_started.current = true;
      if (!saved?.complete) void start_translation();
    }, 0);
    return () => clearTimeout(timer);
  }, [automatic, reader_ready, agent.background_running, agent.status_ready, profile, fingerprint, cannot_start, saved, start_translation]);

  if (automatic) return null;

  return (
    <div className="flex h-full flex-col border-l border-border bg-card">
      <div className="flex items-center gap-1 border-b border-border px-3 py-1.5">
        <Languages className="h-4 w-4 text-primary" />
        <span className="flex-1 text-sm font-medium">{t("reader.translation")}</span>
        {is_loading && (
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <Loader2 className="h-3 w-3 animate-spin" />
            {progress.total > 0
              ? `${progress.current}/${progress.total}`
              : "..."}
          </div>
        )}
        <Button
          size="sm"
          variant="ghost"
          className="h-6 text-xs"
          onClick={() => void start_translation()}
          disabled={cannot_start}
        >
          {t("translation.translate")}
        </Button>
      </div>

      {(is_loading || agent.phase === "cancelled") && (
        <div className="flex items-center gap-2 border-b border-border px-3 py-2 text-xs" role="status">
          <span className="flex-1">{t(agent.phase === "waiting" ? "agentTask.waiting" : agent.phase === "cancelling" ? "agentTask.cancelling" : agent.phase === "cancelled" ? "agentTask.cancelled" : agent.background_running ? "agentTask.background" : "agentTask.running")}</span>
          {is_loading && <Button size="sm" variant="ghost" onClick={agent.cancel} disabled={agent.phase === "cancelling"}>{t("agentTask.cancel")}</Button>}
        </div>
      )}
      {agent.cancel_error && <p role="alert" className="px-3 py-2 text-sm text-destructive">{agent.cancel_error}</p>}

      <div className="flex-1 overflow-y-auto p-4">
        {saved && !saved.complete && live_segments === null && (
          <p role="status" className="mb-3 text-xs text-muted-foreground">{t("translation.restoredPartial")}</p>
        )}
        {!segments.length && !is_loading && !error && (
          <div className="flex h-full flex-col items-center justify-center text-center">
            <Languages className="h-8 w-8 text-muted-foreground/50" />
            <p className="mt-2 text-sm text-muted-foreground">
              {t("translation.translateArticle")}
            </p>
            <Button size="sm" className="mt-3" disabled={cannot_start} onClick={() => void start_translation()}>
              <Languages className="mr-1 h-3.5 w-3.5" />
              {t("translation.start")}
            </Button>
          </div>
        )}

        {status && !is_loading && (
          <p role="status" className="mb-3 text-sm">{t(`translation.${status}`, { completed: outcome?.succeeded ?? completed, total: outcome?.total ?? sources.length })}</p>
        )}
        {outcome && outcome.status !== "complete" && saved?.complete && !is_loading && (
          <p role="status" className="mb-3 text-xs text-muted-foreground">{t("translation.previousSuccess")}</p>
        )}
        {(error || restore_error || document_error || configuration_error) && (
          <div className="rounded-lg border border-destructive/30 bg-destructive/5 p-3">
            <p role="alert" className="text-sm text-destructive">{error ?? String(restore_error ?? document_error ?? configuration_error)}</p>
            {(restore_error || document_error) && <Button size="sm" variant="ghost" disabled={restore_loading || document_loading}
              onClick={() => { void retry_document(); void retry_result(); }}>{t("translation.retryLoad")}</Button>}
          </div>
        )}

        {sources.length > 0 && (
          <div className="space-y-4">
            {sources.map((source) => {
              const text = translations[source.order_index];
              const segment_error = segment_errors[source.source_segment_id] ?? saved?.segment_errors?.[source.order_index] ?? segment_errors[String(source.order_index)];
              return <section key={source.source_segment_id} data-source-segment-id={source.source_segment_id} className="text-sm leading-relaxed">
                <p className="whitespace-pre-wrap text-muted-foreground">{source.source_text}</p>
                {text ? <div data-translation-for={source.source_segment_id} className="mt-2 whitespace-pre-wrap border-l-2 border-primary pl-3">{text}</div>
                  : <div className="mt-2"><p className="text-xs text-muted-foreground">{t("translation.missing")}</p>
                    {segment_error && <p role="alert" className="text-xs text-destructive">{segment_error}</p>}
                    <Button size="sm" variant="ghost" disabled={cannot_start}
                      aria-label={t("translation.retrySegment", { number: source.order_index + 1 })}
                      onClick={() => void start_translation(source.source_segment_id)}>{t("translation.retry")}</Button>
                  </div>}
              </section>;
            })}
          </div>
        )}

      </div>
    </div>
  );
}
