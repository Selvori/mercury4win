import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { get_usage_report } from "@/lib/tauri_bindings";
import { Button } from "@/components/ui/button";
import type { UsageContext, UsageMetrics } from "@/types/agent";

export function UsageStats({ context, on_clear }: { context?: UsageContext; on_clear?: () => void }) {
  const { t, i18n } = useTranslation();
  const [window, set_window] = useState("30d");
  const [dimension, set_dimension] = useState(context?.dimension ?? "provider");
  const [metric, set_metric] = useState<"request_count" | "prompt_tokens" | "completion_tokens" | "total_tokens">("request_count");
  const filter = context?.filter ?? {};
  const query = useQuery({ queryKey: ["usage", window, dimension, filter], queryFn: () => get_usage_report(window, dimension, filter), refetchInterval: 15000 });
  const data = query.data;
  const totals = data?.totals;
  const columns: (keyof UsageMetrics)[] = ["request_count", "prompt_tokens", "completion_tokens", "total_tokens", "succeeded", "failed", "cancelled", "timed_out", "missing_usage", "legacy_events", "legacy_tokens"];
  const groups = [...(data?.groups ?? [])].sort((a, b) => b.metrics[metric] - a.metrics[metric] || a.label.localeCompare(b.label, i18n.language));
  const group_max = Math.max(1, ...groups.map(g => g.metrics[metric]));
  const max = Math.max(1, ...(data?.daily.map(d => d.metrics[metric]) ?? []));
  const group_label = (label: string) => dimension === "agent" ? label === "topic" ? t("topics.title") : t(`settings.agentType${label.charAt(0).toUpperCase()}${label.slice(1)}`, { defaultValue: label }) : label;
  const percent = (value: number) => totals?.request_count ? `${(value / totals.request_count * 100).toFixed(1)}%` : "—";
  return <div className="mx-auto max-w-4xl space-y-4">
    <h3 className="text-lg font-semibold">{t("usage.title")}</h3>
    <p className="text-sm text-muted-foreground">{t("usage.requestScope")}</p>
    <div className="flex flex-wrap items-center gap-2">
      <label>{t("usage.range")} <select aria-label={t("usage.range")} value={window} onChange={e => set_window(e.target.value)} className="rounded border border-border bg-background p-1">
        {["7d", "14d", "30d", "90d"].map(w => <option key={w} value={w}>{t(`usage.window${w}`)}</option>)}
      </select></label>
      <label>{t("usage.dimension")} <select aria-label={t("usage.dimension")} value={dimension} onChange={e => set_dimension(e.target.value as typeof dimension)} className="rounded border border-border bg-background p-1">
        {["provider", "model", "agent"].map(d => <option key={d} value={d}>{t(`usage.${d}`)}</option>)}
      </select></label>
      <Button size="sm" variant="outline" onClick={() => query.refetch()} disabled={query.isFetching}>{t("usage.refresh")}</Button>
    </div>
    {context?.label && <div className="flex items-center gap-2 text-sm"><span>{t("usage.filtered", { name: context.label })}</span><Button size="sm" variant="outline" onClick={on_clear}>{t("usage.allHistory")}</Button></div>}
    {query.isLoading ? <p role="status">{t("common.loading")}</p> : query.isError ? <p role="alert" className="text-destructive">{t("usage.loadError")}</p> : data && totals && <>
      <p className="text-xs text-muted-foreground">{t("usage.utcRange", { from: data.from_utc, to: data.to_utc })}</p>
      {!!data.persistence_failures && <p role="alert" className="text-destructive">{t("usage.persistenceError", { count: data.persistence_failures })}</p>}
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        {["request_count", "prompt_tokens", "completion_tokens", "total_tokens"].map(k => <div key={k} className="rounded border border-border p-3"><p className="text-xs text-muted-foreground">{t(`usage.${k}`)}</p><p className="text-xl">{totals[k as keyof UsageMetrics].toLocaleString(i18n.language)}</p></div>)}
      </div>
      <p className="text-sm">{t("usage.quality", { success: percent(totals.succeeded), coverage: percent(totals.request_count - totals.missing_usage) })}</p>
      {!!totals.missing_usage && <p className="text-sm text-muted-foreground">{t("usage.missingNote", { count: totals.missing_usage })}</p>}
      {!!totals.legacy_events && <p className="rounded border border-border p-3 text-sm">{t("usage.legacyNote", { count: totals.legacy_events, tokens: totals.legacy_tokens.toLocaleString(i18n.language) })}</p>}
      <label className="block text-sm">{t("usage.metric")} <select aria-label={t("usage.metric")} value={metric} onChange={e => set_metric(e.target.value as typeof metric)} className="rounded border border-border bg-background p-1">
        {["request_count", "prompt_tokens", "completion_tokens", "total_tokens"].map(m => <option key={m} value={m}>{t(`usage.${m}`)}</option>)}
      </select></label>
      <div role="img" aria-label={t("usage.groupChart")} className="space-y-2 rounded border border-border p-3">
        {groups.map(g => <div key={g.id} className="text-xs"><div className="mb-1 flex justify-between gap-2"><span>{group_label(g.label)}{g.archived && ` (${t("usage.archived")})`}</span><span>{g.metrics[metric].toLocaleString(i18n.language)}</span></div><div className="h-3 rounded-sm bg-primary" style={{width:`${g.metrics[metric] / group_max * 100}%`}} /></div>)}
      </div>
      <div role="img" aria-label={t("usage.dailyChart")} className="space-y-1 rounded border border-border p-3">
        {!data.daily.length && <p>{t("usage.empty")}</p>}
        {data.daily.map(day => <div key={day.id} className="flex items-center gap-2 text-xs"><span className="w-20 shrink-0">{day.label}</span><div className="min-w-0 flex-1"><div className="h-3 rounded-sm bg-primary" style={{ width: `${day.metrics[metric] / max * 100}%` }} /></div><span>{day.metrics[metric].toLocaleString(i18n.language)}</span></div>)}
      </div>
      <div className="overflow-x-auto rounded border border-border">
        <table className="w-full text-xs"><caption className="p-2 text-left">{t("usage.comparison")}</caption><thead><tr><th className="p-2 text-left">{t(`usage.${dimension}`)}</th>{columns.map(k => <th key={k} aria-sort={k === metric ? "descending" : undefined} className={`p-2 text-right ${k === metric ? "text-primary" : ""}`}>{t(`usage.${k}`)}</th>)}</tr></thead>
          <tbody>{groups.map(g => <tr key={g.id} className="border-t border-border"><th className="p-2 text-left font-normal">{group_label(g.label)}{g.archived && <span className="ml-1 text-muted-foreground">({t("usage.archived")})</span>}</th>{columns.map(k => <td key={k} className="p-2 text-right">{g.metrics[k].toLocaleString(i18n.language)}</td>)}</tr>)}</tbody>
        </table>
      </div>
    </>}
  </div>;
}
