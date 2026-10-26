import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { ListFilter } from "lucide-react";
import { list_smart_views } from "@/lib/tauri_bindings";
import type { SmartView } from "@/types";

export function SmartViews({ selected_id, on_select }: { selected_id?: number; on_select: (view: SmartView) => void }) {
  const { t } = useTranslation();
  const views = useQuery({ queryKey: ["smart_views"], queryFn: list_smart_views });
  return <section aria-label={t("search.smartViews")} className="shrink-0 border-b border-border px-2 pb-2">
    <h2 className="px-2 py-1 text-xs font-medium text-muted-foreground">{t("search.smartViews")}</h2>
    <div className="max-h-36 overflow-y-auto">
      {views.data?.map((view) => <button key={view.id} aria-pressed={selected_id === view.id} title={view.name} onClick={() => on_select(view)}
        className={`flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm ${selected_id === view.id ? "bg-accent" : "hover:bg-accent"}`}>
        <ListFilter className="h-4 w-4 shrink-0" /><span className="min-w-0 truncate">{view.name}</span>
      </button>)}
    </div>
    {views.isLoading && <p role="status" className="px-2 text-xs text-muted-foreground">{t("search.loadingViews")}</p>}
    {!views.isLoading && !views.isError && !views.data?.length && <p className="px-2 text-xs leading-relaxed text-muted-foreground">{t("search.viewsEmpty")}</p>}
    {views.isError && <button className="px-2 text-xs text-destructive" onClick={() => void views.refetch()}>{t("search.viewsFailed")}</button>}
  </section>;
}
