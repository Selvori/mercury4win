// mercury4win/src/features/reader/components/reader_summary.tsx
// Summary panel — streaming display with detail level control

import { useState, useCallback, useEffect, useRef } from "react";
import { Channel } from "@tauri-apps/api/core";
import { Sparkles, Loader2, FileText } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { run_summary, get_agent_profile, load_summary_result, get_agent_fingerprint, load_latest_summary, get_setting, set_setting, abandon_agent_waiting } from "@/lib/tauri_bindings";
import { useAgentRequest } from "../hooks/use_agent_request";

interface Props {
  entry_id: number;
  entry_title: string;
  reader_ready?: boolean;
}

type DetailLevel = "brief" | "medium" | "detailed";

export function ReaderSummary({ entry_id, entry_title, reader_ready = true }: Props) {
  const { t } = useTranslation();
  const [stream_content, set_content] = useState<string | null>(null);
  const query_client = useQueryClient();
  const [detail_override, set_detail_level] = useState<DetailLevel | null>(null);
  const [error, set_error] = useState<string | null>(null);
  const agent = useAgentRequest(entry_id, "summary");
  const { begin_request, is_pending: is_loading } = agent;

  // Load user's language preference from agent settings
  const { data: profile, isPending: profile_loading } = useQuery({
    queryKey: ["agent_profile", "summary"],
    queryFn: () => get_agent_profile("summary"),
  });
  const { data: auto_enabled = false, error: auto_setting_error } = useQuery({
    queryKey: ["summary_auto_enabled"], queryFn: async () => (await get_setting("summary_auto_enabled")) === "true", retry: false,
  });
  const [confirm_auto, set_confirm_auto] = useState(false);
  const attempted = useRef<string | null>(null);
  const auto_request = useRef<string | null>(null);
  const { data: latest, error: latest_error, isFetching: latest_loading } = useQuery({
    queryKey: ["latest_summary", entry_id], queryFn: () => load_latest_summary(entry_id), retry: false,
  });
  const detail_level = detail_override ?? (auto_enabled && latest ? latest.detail_level as DetailLevel : profile?.detail_level as DetailLevel) ?? "medium";
  const target_language = auto_enabled && latest && !detail_override ? latest.target_language : profile?.target_language ?? "en";
  const { data: fingerprint, error: configuration_error } = useQuery({
    queryKey: ["agent_fingerprint", "summary"], queryFn: () => get_agent_fingerprint("summary"), retry: false,
  });

  const result_key = ["summary_result", entry_id, target_language, detail_level, fingerprint];
  const { data: saved, error: restore_error, isFetching: restore_loading } = useQuery({
    queryKey: result_key,
    queryFn: () => load_summary_result(entry_id, target_language, detail_level),
    enabled: !is_loading && fingerprint !== undefined,
    retry: false,
  });
  const content = stream_content ?? saved?.text ?? (auto_enabled && !detail_override ? latest?.result.text : null) ?? "";

  const do_summary = useCallback(async (level: DetailLevel, automatic = false) => {
    const request = begin_request();
    if (automatic) auto_request.current = request.request_id;
    set_content("");
    set_error(null);

    const channel = new Channel<string>();
    channel.onmessage = (chunk) => {
      if (!request.is_current()) return;
      try {
        const msg = JSON.parse(chunk);
        if (request.handle_event(msg)) return;
        if (msg.type === "reset") {
          set_content("");
          set_error(null);
        } else if (msg.type === "notice") {
          set_error(msg.message);
        } else if (msg.type === "chunk") {
          set_content((prev) => (prev ?? "") + msg.content);
        } else if (msg.type === "done") {
          // The invoke settles only after the executor releases its slot.
        } else if (msg.type === "error") {
          set_error(msg.message);
          // The invoke settles only after the executor releases its slot.
        }
      } catch {
        set_content((prev) => (prev ?? "") + chunk);
      }
    };

    try {
      await run_summary(request.entry_id, target_language, level, channel, request.request_id);
      await query_client.invalidateQueries({ queryKey: ["summary_result", request.entry_id] });
      await query_client.invalidateQueries({ queryKey: ["latest_summary", request.entry_id] });
      if (!request.is_current()) return;
      set_content(null);
      request.finish();
    } catch (e) {
      if (!request.is_current()) return;
      const cancelled = /^(?:Error: )?Agent task cancelled:/.test(String(e));
      if (!cancelled) set_error(String(e));
      set_content(null);
      request.finish(cancelled);
      void query_client.invalidateQueries({ queryKey: ["summary_result", request.entry_id] });
    }
  }, [begin_request, target_language, query_client]);

  useEffect(() => {
    if (!reader_ready || !auto_enabled || profile_loading || !fingerprint || latest_loading || latest || latest_error || configuration_error || is_loading || restore_loading || error || restore_error || auto_setting_error) return;
    const identity = JSON.stringify([entry_id, fingerprint]);
    if (attempted.current === identity) return;
    const timer = setTimeout(() => { attempted.current = identity; void do_summary(detail_level, true); }, 1000);
    return () => clearTimeout(timer);
  }, [reader_ready, auto_enabled, profile_loading, fingerprint, latest_loading, latest, latest_error, configuration_error, is_loading, restore_loading, error, restore_error, auto_setting_error, entry_id, detail_level, do_summary]);

  async function save_auto(enabled: boolean) {
    try {
      await set_setting("summary_auto_enabled", String(enabled));
      if (!enabled && auto_request.current) await abandon_agent_waiting("summary", auto_request.current);
      query_client.setQueryData(["summary_auto_enabled"], enabled);
      set_confirm_auto(false);
    } catch (e) { set_error(String(e)); }
  }

  function handle_detail_change(level: DetailLevel) {
    set_detail_level(level);
    set_content(null);
    set_error(null);
  }

  const detail_options: { value: DetailLevel; label: string }[] = [
    { value: "brief", label: t("summary.brief") },
    { value: "medium", label: t("summary.medium") },
    { value: "detailed", label: t("summary.detailed") },
  ];

  return (
    <div className="flex h-full flex-col border-l border-border bg-card">
      <div className="flex items-center gap-1 border-b border-border px-3 py-1.5">
        <Sparkles className="h-4 w-4 text-primary" />
        <span className="flex-1 text-sm font-medium">{t("reader.summary")}</span>
        <Button size="sm" variant="ghost" className="h-6 text-xs" onClick={() => do_summary(detail_level)} disabled={is_loading || !reader_ready}>
          {t("summary.summarize")}
        </Button>
        <div className="flex gap-0.5 rounded-md border border-border bg-muted/50 p-0.5">
          {detail_options.map((opt) => (
            <Button
              key={opt.value}
              variant="ghost"
              size="sm"
              className={cn(
                "h-6 px-2 text-[11px] rounded-sm",
                detail_level === opt.value && "bg-background shadow-sm",
              )}
              onClick={() => handle_detail_change(opt.value)}
              aria-pressed={detail_level === opt.value}
              disabled={is_loading || !reader_ready}
            >
              {opt.label}
            </Button>
          ))}
        </div>
      </div>

      <div className="border-b border-border px-3 py-2 text-xs">
        <label className="flex items-center gap-2"><input type="checkbox" checked={auto_enabled} onChange={(event) => event.target.checked ? set_confirm_auto(true) : void save_auto(false)} />{t("summary.autoSummary")}</label>
        {confirm_auto && <div role="alertdialog" aria-label={t("summary.enableAuto")} className="mt-2">
          <p>{t("summary.autoWarning")}</p>
          <Button size="sm" onClick={() => void save_auto(true)}>{t("summary.enableAuto")}</Button>
          <Button size="sm" variant="ghost" onClick={() => set_confirm_auto(false)}>{t("common.cancel")}</Button>
        </div>}
      </div>
      {(is_loading || agent.phase === "cancelled") && (
        <div className="flex items-center gap-2 border-b border-border px-3 py-2 text-xs" role="status">
          <span className="flex-1">{t(agent.phase === "waiting" ? "agentTask.waiting" : agent.phase === "cancelling" ? "agentTask.cancelling" : agent.phase === "cancelled" ? "agentTask.cancelled" : agent.background_running ? "agentTask.background" : "agentTask.running")}</span>
          {is_loading && <Button size="sm" variant="ghost" onClick={agent.cancel} disabled={agent.phase === "cancelling"}>{t("agentTask.cancel")}</Button>}
        </div>
      )}
      {agent.cancel_error && <p role="alert" className="px-3 py-2 text-sm text-destructive">{agent.cancel_error}</p>}

      <div className="flex-1 overflow-y-auto p-4">
        {!content && !is_loading && !error && (
          <div className="flex h-full flex-col items-center justify-center text-center">
            <FileText className="h-8 w-8 text-muted-foreground/50" />
            <p className="mt-2 text-sm text-muted-foreground">{t("summary.generate")}</p>
            <Button size="sm" className="mt-3" disabled={!reader_ready} onClick={() => do_summary(detail_level)}>
              <Sparkles className="mr-1 h-3.5 w-3.5" />
              {t("summary.summarize")}
            </Button>
          </div>
        )}

        {is_loading && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            {t(agent.phase === "waiting" ? "agentTask.waiting" : "summary.generating")}
          </div>
        )}

        {(error || restore_error || configuration_error || latest_error || auto_setting_error) && (
          <div className="rounded-lg border border-destructive/30 bg-destructive/5 p-3">
            <p role="alert" className="text-sm text-destructive">{error ?? String(restore_error ?? configuration_error ?? latest_error ?? auto_setting_error)}</p>
          </div>
        )}

        {content && stream_content === null && !saved && auto_enabled && latest && latest.configuration_fingerprint !== fingerprint && <p role="status" className="mb-2 text-xs text-muted-foreground">{t("summary.historicalConfiguration")}</p>}
        {content && (
          <div className="prose prose-sm dark:prose-invert max-w-none">
            <h3 className="text-sm font-semibold mb-2">{entry_title}</h3>
            {content.split("\n").map((para, i) => (
              <p key={i} className="text-sm leading-relaxed text-card-foreground mb-2">
                {para || " "}
              </p>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
