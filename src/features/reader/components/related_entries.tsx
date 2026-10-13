import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { related_entries } from "@/lib/tauri_bindings";
import type { Entry } from "@/types";
export function RelatedEntries({ entry_id, on_select }: { entry_id: number; on_select?: (entry: Entry) => void }) {
  const { t } = useTranslation();
  const { data: related } = useQuery({ queryKey: ["related_entries", entry_id], queryFn: () => related_entries(entry_id), retry: false });
  if (!related?.length || !on_select) return null;
  return <div className="shrink-0 border-t border-border px-3 py-2">
    <p className="text-xs text-muted-foreground">{t("tagging.related")}</p>
    <div className="flex gap-2 overflow-x-auto">{related.map((entry) => <button key={entry.id} className="shrink-0 max-w-52 truncate rounded border border-border p-2 text-xs hover:bg-accent" onClick={() => on_select(entry)}>{entry.title || t("entry.untitled")}</button>)}</div>
  </div>;
}
