// mercury4win-linux/src/features/reader/components/reader_translation.tsx
// Translation panel — bilingual side-by-side display

import { useState, useCallback } from "react";
import { Channel } from "@tauri-apps/api/core";
import { Languages, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { run_translation } from "@/lib/tauri_bindings";

interface Props {
  entry_id: number;
}

interface SegmentResult {
  index: number;
  text: string;
}

export function ReaderTranslation({ entry_id }: Props) {
  const [segments, set_segments] = useState<SegmentResult[]>([]);
  const [is_loading, set_is_loading] = useState(false);
  const [progress, set_progress] = useState({ current: 0, total: 0 });
  const [error, set_error] = useState<string | null>(null);

  const start_translation = useCallback(async () => {
    set_segments([]);
    set_error(null);
    set_is_loading(true);

    const channel = new Channel<string>();
    channel.onmessage = (chunk) => {
      try {
        const msg = JSON.parse(chunk);
        if (msg.type === "progress") {
          set_progress({ current: msg.segment, total: msg.total });
        } else if (msg.type === "segment") {
          set_segments((prev) => {
            const next = [...prev];
            next[msg.index] = { index: msg.index, text: msg.text };
            return next;
          });
        } else if (msg.type === "done") {
          set_is_loading(false);
        } else if (msg.type === "error") {
          set_error(msg.error);
          set_is_loading(false);
        }
      } catch {
        // raw chunk — ignore for now
      }
    };

    try {
      await run_translation(entry_id, "zh-Hans", channel);
    } catch (e) {
      set_error(String(e));
      set_is_loading(false);
    }
  }, [entry_id]);

  return (
    <div className="flex h-full flex-col border-l border-border bg-card">
      <div className="flex items-center gap-1 border-b border-border px-3 py-1.5">
        <Languages className="h-4 w-4 text-primary" />
        <span className="flex-1 text-sm font-medium">Translation</span>
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
          onClick={start_translation}
          disabled={is_loading}
        >
          Translate
        </Button>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {!segments.length && !is_loading && !error && (
          <div className="flex h-full flex-col items-center justify-center text-center">
            <Languages className="h-8 w-8 text-muted-foreground/50" />
            <p className="mt-2 text-sm text-muted-foreground">
              Translate this article
            </p>
            <Button size="sm" className="mt-3" onClick={start_translation}>
              <Languages className="mr-1 h-3.5 w-3.5" />
              Start Translation
            </Button>
          </div>
        )}

        {error && (
          <div className="rounded-lg border border-destructive/30 bg-destructive/5 p-3">
            <p className="text-sm text-destructive">{error}</p>
          </div>
        )}

        {segments.length > 0 && (
          <div className="space-y-4">
            {segments
              .filter(Boolean)
              .sort((a, b) => a.index - b.index)
              .map((seg) => (
                <div key={seg.index} className="text-sm leading-relaxed">
                  {seg.text}
                </div>
              ))}
          </div>
        )}
      </div>
    </div>
  );
}
