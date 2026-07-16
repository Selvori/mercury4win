// mercury4win-linux/src/features/usage/components/usage_stats.tsx
// LLM usage statistics display — 7d/30d/90d token counts

import { useQuery } from "@tanstack/react-query";
import { BarChart3, Loader2 } from "lucide-react";
import { get_usage_report } from "@/lib/tauri_bindings";
import { useState } from "react";
import { cn } from "@/lib/utils";

type Window = "7d" | "30d" | "90d";

export function UsageStats() {
  const [window, set_window] = useState<Window>("30d");

  const { data, isLoading, isError } = useQuery({
    queryKey: ["usage", window],
    queryFn: () => get_usage_report(window),
  });

  const [prompt_tokens, completion_tokens, request_count] = data ?? [0, 0, 0];

  const windows: { value: Window; label: string }[] = [
    { value: "7d", label: "7 days" },
    { value: "30d", label: "30 days" },
    { value: "90d", label: "90 days" },
  ];

  return (
    <div className="max-w-2xl mx-auto">
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">Usage Statistics</h3>
        <p className="text-sm text-muted-foreground mt-1">
          LLM token usage across all AI features.
        </p>
      </div>

      {/* Window selector */}
      <div className="flex gap-0.5 rounded-md border border-border bg-muted/50 p-0.5 w-fit mb-6">
        {windows.map((w) => (
          <button
            key={w.value}
            onClick={() => set_window(w.value)}
            className={cn(
              "h-7 px-3 text-xs rounded-sm transition-colors",
              window === w.value
                ? "bg-background shadow-sm text-foreground"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            {w.label}
          </button>
        ))}
      </div>

      {isLoading ? (
        <div className="flex items-center gap-2 text-sm text-muted-foreground py-8">
          <Loader2 className="h-4 w-4 animate-spin" />
          Loading...
        </div>
      ) : isError ? (
        <div className="rounded-lg border border-destructive/30 bg-destructive/5 p-4">
          <p className="text-sm text-destructive">Failed to load usage data</p>
        </div>
      ) : (
        <div className="space-y-4">
          {/* Stat cards */}
          <div className="grid grid-cols-3 gap-3">
            <StatCard
              label="Prompt Tokens"
              value={prompt_tokens.toLocaleString()}
              description="Input tokens sent to LLMs"
            />
            <StatCard
              label="Completion Tokens"
              value={completion_tokens.toLocaleString()}
              description="Output tokens from LLMs"
            />
            <StatCard
              label="API Requests"
              value={String(request_count)}
              description="Total LLM API calls"
            />
          </div>

          {/* Total */}
          <div className="rounded-lg border border-border bg-card p-4">
            <div className="flex items-center gap-2">
              <BarChart3 className="h-5 w-5 text-primary" />
              <span className="text-sm font-medium">Total Tokens</span>
              <span className="text-xl font-semibold ml-auto">
                {(prompt_tokens + completion_tokens).toLocaleString()}
              </span>
            </div>
          </div>

          {/* Usage breakdown note */}
          <div className="rounded-lg border border-border bg-muted/30 p-3">
            <p className="text-xs text-muted-foreground leading-relaxed">
              Usage is tracked per LLM API request across Summary, Translation,
              and Tagging features. Token counts are reported directly by each
              model provider and reflect actual API consumption within the
              selected time window.
            </p>
          </div>
        </div>
      )}
    </div>
  );
}

function StatCard({
  label,
  value,
  description,
}: {
  label: string;
  value: string;
  description: string;
}) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="text-2xl font-semibold text-foreground mt-1">{value}</p>
      <p className="text-[10px] text-muted-foreground mt-2 leading-tight">
        {description}
      </p>
    </div>
  );
}
