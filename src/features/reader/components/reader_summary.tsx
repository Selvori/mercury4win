// mercury4win-linux/src/features/reader/components/reader_summary.tsx
// Summary panel — streaming display with detail level control

import { useState, useRef, useCallback, useEffect } from "react";
import { Channel } from "@tauri-apps/api/core";
import { Sparkles, Loader2, FileText } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { run_summary, get_agent_profile } from "@/lib/tauri_bindings";

interface Props {
  entry_id: number;
  entry_title: string;
}

type DetailLevel = "brief" | "medium" | "detailed";

export function ReaderSummary({ entry_id, entry_title }: Props) {
  const { t } = useTranslation();
  const [content, set_content] = useState("");
  const [is_loading, set_is_loading] = useState(false);
  const [detail_level, set_detail_level] = useState<DetailLevel>("medium");
  const [error, set_error] = useState<string | null>(null);
  const channel_ref = useRef<Channel<string> | null>(null);

  // Load user's language preference from agent settings
  const { data: profile } = useQuery({
    queryKey: ["agent_profile", "summary"],
    queryFn: () => get_agent_profile("summary"),
  });
  const target_language = profile?.target_language ?? "en";

  // Clear state when switching to a different entry
  useEffect(() => {
    set_content("");
    set_error(null);
    set_is_loading(false);
  }, [entry_id]);

  const do_summary = useCallback(async (level: DetailLevel) => {
    set_content("");
    set_error(null);
    set_is_loading(true);

    const channel = new Channel<string>();
    channel_ref.current = channel;

    channel.onmessage = (chunk) => {
      try {
        const msg = JSON.parse(chunk);
        if (msg.type === "chunk") {
          set_content((prev) => prev + msg.content);
        } else if (msg.type === "done") {
          set_is_loading(false);
        } else if (msg.type === "error") {
          set_error(msg.message);
          set_is_loading(false);
        }
      } catch {
        set_content((prev) => prev + chunk);
      }
    };

    try {
      await run_summary(entry_id, target_language, level, channel);
    } catch (e) {
      set_error(String(e));
      set_is_loading(false);
    }
  }, [entry_id, target_language]);

  function handle_detail_change(level: DetailLevel) {
    set_detail_level(level);
    if (content) {
      do_summary(level);
    }
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
              disabled={is_loading}
            >
              {opt.label}
            </Button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {!content && !is_loading && !error && (
          <div className="flex h-full flex-col items-center justify-center text-center">
            <FileText className="h-8 w-8 text-muted-foreground/50" />
            <p className="mt-2 text-sm text-muted-foreground">{t("summary.generate")}</p>
            <Button size="sm" className="mt-3" onClick={() => do_summary(detail_level)}>
              <Sparkles className="mr-1 h-3.5 w-3.5" />
              {t("summary.summarize")}
            </Button>
          </div>
        )}

        {is_loading && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            {t("summary.generating")}
          </div>
        )}

        {error && (
          <div className="rounded-lg border border-destructive/30 bg-destructive/5 p-3">
            <p className="text-sm text-destructive">{error}</p>
          </div>
        )}

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
