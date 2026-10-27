import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { list_smart_views, save_topic, list_topics, update_topic_members } from "@/lib/tauri_bindings";
import type { Topic } from "@/types/topic";

export const topic_field = "w-full rounded-md border border-border bg-background p-2 text-sm text-foreground focus-visible:outline-ring";
export function TopicEditor({ topic, on_close, on_saved, generate_on_create = true }: { topic?: Topic; on_close: () => void; on_saved: (id: number, generate?: boolean) => void; generate_on_create?: boolean }) {
  const { t } = useTranslation(); const qc = useQueryClient();
  const [name, set_name] = useState(topic?.name ?? ""); const [description, set_description] = useState(topic?.description ?? "");
  const [binding, set_binding] = useState("keep"); const [error, set_error] = useState(""); const [busy, set_busy] = useState(false);
  const views = useQuery({ queryKey: ["smart_views"], queryFn: list_smart_views });
  const [mode, set_mode] = useState<"automatic" | "manual">(topic ? topic.mode ?? "manual" : "automatic");
  const valid = !!name.trim() && name.length <= 80;
  async function submit(generate = false) {
    if (!valid || busy) return;
    set_busy(true); set_error("");
    try {
      const view = views.data?.find((v) => String(v.id) === binding);
      const rule = binding === "keep" ? topic?.rule ?? null : view?.filter ?? null;
      const label = binding === "keep" ? topic?.rule_name ?? null : view?.name ?? null;
      const id = await save_topic(topic?.id ?? null, name, description, topic?.archived ?? false, rule, label, mode);
      await qc.invalidateQueries({ queryKey: ["topics"] }); await qc.invalidateQueries({ queryKey: ["topic", id] }); await qc.invalidateQueries({ queryKey: ["topic-status", id] }); on_saved(id, generate && mode === "automatic");
    } catch (e) { set_error(String(e)); } finally { set_busy(false); }
  }
  return <Dialog open onOpenChange={(o) => { if (!o && !busy) on_close(); }} aria_label={t(topic ? "topics.edit" : "topics.create")}>
    <DialogContent className="min-h-0 overflow-y-auto"><DialogHeader><DialogTitle>{t(topic ? "topics.edit" : "topics.create")}</DialogTitle></DialogHeader>
      <form onSubmit={(e) => { e.preventDefault(); void submit(!topic && generate_on_create); }} className="space-y-4">
        <label className="block text-sm">{t("topics.name")}<input autoFocus required maxLength={80} value={name} onChange={(e) => set_name(e.target.value)} className={topic_field} /></label>
        <p className="text-sm leading-6 text-muted-foreground">{t("topics.simpleHint")}</p>
        <details open={!!topic}><summary className="cursor-pointer text-sm">{t("topics.advanced")}</summary><div className="space-y-4 pt-4">
        <label className="block text-sm">{t("topics.mode")}<select className={topic_field} value={mode} onChange={(e) => set_mode(e.target.value as typeof mode)}><option value="automatic">{t("topics.automatic")}</option><option value="manual">{t("topics.manual")}</option></select></label>
        {topic?.mode === "manual" && mode === "automatic" && <p className="text-xs leading-6 text-muted-foreground">{t("topics.switchHint")}</p>}
        <label className="block text-sm">{t("topics.description")}<textarea rows={3} maxLength={4000} value={description} onChange={(e) => set_description(e.target.value)} className={topic_field} /></label>
        <label className="block text-sm">{t("topics.rule")}<select value={binding} onChange={(e) => set_binding(e.target.value)} className={topic_field}>
          <option value="keep">{topic?.rule ? t("topics.keepRule", { name: topic.rule_name }) : t("topics.noRule")}</option>
          <option value="none">{t("topics.noRule")}</option>{views.data?.map((v) => <option key={v.id} value={v.id}>{v.name}</option>)}
        </select></label><p className="text-xs text-muted-foreground">{t("topics.ruleHint")}</p>
        </div></details>
        {views.isError && <p role="alert">{t("topics.loadFailed")}</p>}{error && <p role="alert" className="text-sm text-destructive">{error}</p>}
        <DialogFooter className="flex-wrap"><Button type="button" variant="outline" disabled={busy} onClick={on_close}>{t("topics.cancel")}</Button>{!topic && mode === "automatic" && generate_on_create && <Button type="button" variant="outline" disabled={busy || !valid} onClick={() => void submit()}>{t("topics.createOnly")}</Button>}<Button type="submit" disabled={busy || !valid}>{t(topic ? "topics.save" : mode === "automatic" && generate_on_create ? "topics.createGenerate" : "topics.create")}</Button></DialogFooter>
      </form>
    </DialogContent></Dialog>;
}

export function AddToTopic({ ids, on_close }: { ids: number[]; on_close: () => void }) {
  const { t } = useTranslation(); const qc = useQueryClient(); const topics = useQuery({ queryKey: ["topics"], queryFn: list_topics });
  const [selected, set_selected] = useState(0); const [creating, set_creating] = useState(false); const [error, set_error] = useState(""); const [busy, set_busy] = useState(false);
  async function add(id: number) { set_busy(true); try { await update_topic_members(id, ids); await qc.invalidateQueries({ queryKey: ["topic", id] }); await qc.invalidateQueries({ queryKey: ["topic-status", id] }); on_close(); } catch (e) { set_error(String(e)); } finally { set_busy(false); } }
  return <Dialog open onOpenChange={(o) => { if (!o && !busy) on_close(); }} aria_label={t("topics.addTo")}>
    <DialogContent><DialogHeader><DialogTitle>{t("topics.addTo")}</DialogTitle></DialogHeader><p className="mb-3 text-sm">{t("topics.selected", { count: ids.length })}</p>
      <label>{t("topics.choose")}<select className={topic_field} value={selected} onChange={(e) => set_selected(Number(e.target.value))}><option value={0}>{t("topics.choose")}</option>{topics.data?.filter((v) => !v.archived).map((v) => <option key={v.id} value={v.id}>{v.name}</option>)}</select></label>
      {topics.isError && <p role="alert">{t("topics.loadFailed")}</p>}{error && <p role="alert" className="text-destructive">{error}</p>}
      <DialogFooter><Button variant="ghost" disabled={busy} onClick={on_close}>{t("topics.cancel")}</Button><Button variant="outline" disabled={busy} onClick={() => set_creating(true)}>{t("topics.create")}</Button><Button disabled={!selected || busy} onClick={() => void add(selected)}>{t("topics.add")}</Button></DialogFooter>
      {creating && <TopicEditor generate_on_create={false} on_close={() => set_creating(false)} on_saved={(id) => { set_creating(false); set_selected(id); }} />}
    </DialogContent></Dialog>;
}
