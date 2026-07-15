// mercury4win-linux/src/features/tags/components/tag_input.tsx
// Tag input with autocomplete from existing tag library

import { useState, useRef, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { X, Plus } from "lucide-react";
import { cn } from "@/lib/utils";
import { list_tags } from "@/lib/tauri_bindings";
import type { Tag } from "@/types";

interface Props {
  entry_id: number;
  selected_tags: Tag[];
  on_add: (name: string) => void;
  on_remove: (tag_id: number) => void;
}

export function TagInput({ selected_tags, on_add, on_remove }: Props) {
  const [input, set_input] = useState("");
  const [show_suggestions, set_show_suggestions] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  const { data: all_tags } = useQuery({
    queryKey: ["tags"],
    queryFn: () => list_tags(),
  });

  // Filter suggestions
  const selected_ids = new Set(selected_tags.map((t) => t.id));
  const suggestions = (all_tags ?? [])
    .filter((t) => !selected_ids.has(t.id))
    .filter((t) =>
      input
        ? t.name.toLowerCase().includes(input.toLowerCase())
        : true,
    )
    .slice(0, 8);

  function handle_add(name: string) {
    on_add(name.trim());
    set_input("");
    set_show_suggestions(false);
  }

  function handle_key_down(e: React.KeyboardEvent) {
    if (e.key === "Enter" && input.trim()) {
      e.preventDefault();
      handle_add(input);
    }
    if (e.key === "Escape") {
      set_show_suggestions(false);
    }
  }

  // Close suggestions on outside click
  useEffect(() => {
    function handler(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        set_show_suggestions(false);
      }
    }
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  return (
    <div ref={ref} className="relative">
      {/* Selected tags */}
      <div className="flex flex-wrap gap-1.5 mb-2">
        {selected_tags.map((tag) => (
          <span
            key={tag.id}
            className={cn(
              "inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-xs font-medium",
              tag.is_provisional
                ? "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400"
                : "bg-primary/10 text-primary",
            )}
          >
            {tag.name}
            <button
              onClick={() => on_remove(tag.id)}
              className="ml-0.5 rounded-full p-0.5 hover:bg-black/10 dark:hover:bg-white/10"
            >
              <X className="h-2.5 w-2.5" />
            </button>
          </span>
        ))}
      </div>

      {/* Input */}
      <div className="flex items-center gap-1">
        <input
          type="text"
          value={input}
          onChange={(e) => {
            set_input(e.target.value);
            set_show_suggestions(true);
          }}
          onFocus={() => set_show_suggestions(true)}
          onKeyDown={handle_key_down}
          placeholder="Add tag..."
          className="flex-1 h-7 rounded-md border border-border bg-transparent px-2 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
        />
        {input.trim() && (
          <button
            onClick={() => handle_add(input)}
            className="h-7 w-7 rounded-md hover:bg-accent flex items-center justify-center"
          >
            <Plus className="h-3.5 w-3.5" />
          </button>
        )}
      </div>

      {/* Suggestions dropdown */}
      {show_suggestions && suggestions.length > 0 && (
        <div className="absolute top-full left-0 right-0 z-10 mt-1 rounded-md border border-border bg-popover shadow-lg">
          {suggestions.map((tag) => (
            <button
              key={tag.id}
              onClick={() => handle_add(tag.name)}
              className="flex w-full items-center gap-2 px-3 py-1.5 text-xs hover:bg-accent text-popover-foreground"
            >
              <span>{tag.name}</span>
              <span className="text-muted-foreground ml-auto">
                {tag.usage_count}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
