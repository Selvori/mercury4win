// mercury4win-linux/src/features/reader/components/reader_pane.tsx
// Reader main panel — renders article HTML with theme control and side panels

import { useQuery } from "@tanstack/react-query";
import {
  Loader2,
  Monitor,
  Moon,
  Sun,
  FileText,
  Sparkles,
  Languages,
  Tag,
  BookOpen,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { build_reader_content } from "@/lib/tauri_bindings";
import type { Entry } from "@/types";
import { useState } from "react";
import { ReaderNote } from "./reader_note";
import { ReaderSummary } from "./reader_summary";
import { ReaderTranslation } from "./reader_translation";
import { ReaderTagging } from "./reader_tagging";

interface Props {
  entry: Entry;
}

type Theme = "classic-light" | "classic-dark" | "paper-light" | "paper-dark";
type Panel = "reader" | "notes" | "summary" | "translation" | "tags";

export function ReaderPane({ entry }: Props) {
  const [theme, set_theme] = useState<Theme>("classic-light");
  const [panel, set_panel] = useState<Panel>("reader");

  const { data: reader_html, isLoading, isFetching, isError, error } = useQuery({
    queryKey: ["reader", entry.id, theme],
    queryFn: () => build_reader_content(entry.id),
    staleTime: 0,
  });

  const theme_options: { value: Theme; icon: typeof Sun; label: string }[] = [
    { value: "classic-light", icon: Sun, label: "Light" },
    { value: "classic-dark", icon: Moon, label: "Dark" },
    { value: "paper-light", icon: Monitor, label: "Paper Light" },
    { value: "paper-dark", icon: Monitor, label: "Paper Dark" },
  ];

  const panel_options: { value: Panel; icon: typeof BookOpen; label: string }[] = [
    { value: "reader", icon: BookOpen, label: "Reader" },
    { value: "notes", icon: FileText, label: "Notes" },
    { value: "summary", icon: Sparkles, label: "Summary" },
    { value: "translation", icon: Languages, label: "Translation" },
    { value: "tags", icon: Tag, label: "Tags" },
  ];

  return (
    <div className="flex h-full">
      {/* Main reader area */}
      <div className={cn("flex h-full flex-col", panel !== "reader" ? "flex-1 border-r border-border" : "flex-1")}>
        {/* Toolbar */}
        <div className="flex items-center gap-1.5 border-b border-border px-3 py-1.5">
          {/* Panel tabs */}
          <div className="flex items-center gap-0.5 rounded-md border border-border bg-muted/50 p-0.5 mr-2">
            {panel_options.map((opt) => (
              <Button
                key={opt.value}
                variant="ghost"
                size="sm"
                className={cn(
                  "h-7 px-2 text-xs rounded-sm",
                  panel === opt.value && "bg-background shadow-sm",
                )}
                onClick={() => set_panel(panel === opt.value ? "reader" : opt.value)}
                title={opt.label}
              >
                <opt.icon className="h-3.5 w-3.5" />
              </Button>
            ))}
          </div>
          <div className="flex-1 flex items-center gap-2">
            <h2 className="text-sm font-medium truncate max-w-md">
              {entry.title || "(untitled)"}
            </h2>
          </div>
          <div className="flex items-center gap-0.5 rounded-md border border-border bg-muted/50 p-0.5">
            {theme_options.map((opt) => (
              <Button
                key={opt.value}
                variant="ghost"
                size="icon"
                className={cn(
                  "h-7 w-7 rounded-sm",
                  theme === opt.value && "bg-background shadow-sm",
                )}
                onClick={() => set_theme(opt.value)}
                title={opt.label}
              >
                <opt.icon className="h-3.5 w-3.5" />
              </Button>
            ))}
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto">
          {isLoading || isFetching ? (
            <div className="flex h-full items-center justify-center">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : isError ? (
            <div className="flex h-full items-center justify-center p-8 text-center">
              <div>
                <p className="text-sm font-medium text-destructive">Failed to load content</p>
                <p className="mt-1 text-xs text-muted-foreground">{String(error)}</p>
              </div>
            </div>
          ) : !reader_html ? (
            <div className="flex h-full items-center justify-center p-8 text-center">
              <p className="text-sm text-muted-foreground">
                Content extraction failed. The article may not be available in reader mode.
              </p>
            </div>
          ) : (
            <iframe
              srcDoc={reader_html}
              className="h-full w-full border-0"
              sandbox="allow-same-origin"
              title="Reader content"
            />
          )}
        </div>
      </div>

      {/* Side panel */}
      {panel === "notes" && <ReaderNote entry_id={entry.id} />}
      {panel === "summary" && (
        <ReaderSummary entry_id={entry.id} entry_title={entry.title || "(untitled)"} />
      )}
      {panel === "translation" && <ReaderTranslation entry_id={entry.id} />}
      {panel === "tags" && <ReaderTagging entry_id={entry.id} />}
    </div>
  );
}
