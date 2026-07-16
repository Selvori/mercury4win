// mercury4win-linux/src/features/reader/components/reader_summary.tsx
// Summary panel — streaming display with detail level control

import { useState, useRef, useCallback } from "react";
import { Channel } from "@tauri-apps/api/core";
import { Sparkles, Loader2, FileText } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { run_summary } from "@/lib/tauri_bindings";

interface Props {
  entry_id: number;
  entry_title: string;
}

type DetailLevel = "brief" | "medium" | "detailed";

export function ReaderSummary({ entry_id, entry_title }: Props) {
  const [content, set_content] = useState("");
  const [is_loading, set_is_loading] = useState(false);
  const [detail_level, set_detail_level] = useState<DetailLevel>("medium");
  const [error, set_error] = useState<string | null>(null);
  const channel_ref = useRef<Channel<string> | null>(null);

  const start_summary = useCallback(async () => {
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
        // Treat as raw text chunk from LLM streaming
        set_content((prev) => prev + chunk);
      }
    };

    try {
      await run_summary(entry_id, "en", detail_level, channel);
    } catch (e) {
      set_error(String(e));
      set_is_loading(false);
    }
  }, [entry_id, detail_level]);

  const detail_options: { value: DetailLevel; label: string }[] = [
    { value: "brief", label: "Brief" },
    { value: "medium", label: "Medium" },
    { value: "detailed", label: "Detailed" },
  ];

  return (
    <div className="flex h-full flex-col border-l border-border bg-card">
      <div className="flex items-center gap-1 border-b border-border px-3 py-1.5">
        <Sparkles className="h-4 w-4 text-primary" />
        <span className="flex-1 text-sm font-medium">Summary</span>
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
              onClick={() => set_detail_level(opt.value)}
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
            <p className="mt-2 text-sm text-muted-foreground">Generate a summary</p>
            <Button size="sm" className="mt-3" onClick={start_summary}>
              <Sparkles className="mr-1 h-3.5 w-3.5" />
              Summarize
            </Button>
          </div>
        )}

        {is_loading && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            Generating summary...
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
