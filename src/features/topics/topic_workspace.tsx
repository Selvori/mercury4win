import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient, useInfiniteQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Channel } from "@tauri-apps/api/core";
import { FolderSearch, Plus, RefreshCw, X, ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { ReaderPane } from "@/features/reader/components/reader_pane";
import { TopicEditor, topic_field } from "./topic_editor";
import * as api from "@/lib/tauri_bindings";
import type { Entry, EntryCursor, EntryFilter } from "@/types";
import type { Topic, TopicBrief, TopicCitation, TopicSource } from "@/types/topic";

function topic_time(value: string, locale: string) {
  const date = new Date(value.includes("T") ? value : value.replace(" ", "T") + "Z");
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString(locale);
}

export function TopicWorkspace() {
  const { t } = useTranslation();
  const topics = useQuery({ queryKey: ["topics"], queryFn: api.list_topics });
  const [selected, set_selected] = useState<number | null>(null); const [creating, set_creating] = useState(false);
  return <main className="flex min-w-0 flex-1 flex-col overflow-hidden bg-background text-foreground" data-topic-workspace>
    <header className="flex shrink-0 flex-wrap items-center gap-2 border-b border-border p-3">
      <FolderSearch className="size-5 text-primary" /><h1 className="mr-auto text-base font-semibold">{t("topics.title")}</h1>
      <label className="min-w-0 max-w-64 text-sm"><span className="sr-only">{t("topics.choose")}</span><select className={topic_field} value={selected ?? ""} onChange={(e) => set_selected(Number(e.target.value) || null)}><option value="">{t("topics.choose")}</option>{topics.data?.map((v) => <option key={v.id} value={v.id}>{v.archived ? `${t("topics.archived")} · ` : ""}{v.name}</option>)}</select></label>
      <Button size="sm" onClick={() => set_creating(true)}><Plus />{t("topics.create")}</Button>
    </header>
    {topics.isError && <p role="alert" className="p-4 text-destructive">{t("topics.loadFailed")} <Button onClick={() => void topics.refetch()}>{t("topics.retry")}</Button></p>}
    {selected ? <TopicPage key={selected} id={selected} on_deleted={() => set_selected(null)} /> : <div className="m-auto max-w-xl p-8 text-center"><FolderSearch className="mx-auto mb-4 size-10 text-primary" /><h2 className="text-xl font-semibold">{t("topics.emptyTitle")}</h2><p className="mt-3 text-sm leading-7 text-muted-foreground">{t("topics.emptyHint")}</p><Button className="mt-5" onClick={() => set_creating(true)}>{t("topics.create")}</Button></div>}
    {creating && <TopicEditor on_close={() => set_creating(false)} on_saved={(id) => { set_selected(id); set_creating(false); }} />}
  </main>;
}

function TopicPage({ id, on_deleted }: { id: number; on_deleted: () => void }) {
  const { t, i18n } = useTranslation(); const qc = useQueryClient();
  const detail = useQuery({ queryKey: ["topic", id], queryFn: () => api.load_topic(id), refetchOnWindowFocus: true });
  const model_info = useQuery({ queryKey: ["topic-model"], queryFn: api.topic_model_info, staleTime: 0 });
  const [tab, set_tab] = useState<"sources" | "brief" | "notes">("sources"); const [editing, set_editing] = useState(false); const [deleting, set_deleting] = useState(false);
  const [busy, set_busy] = useState(false); const [phase, set_phase] = useState(""); const [error, set_error] = useState(""); const [message, set_message] = useState("");
  const [version, set_version] = useState<number | null>(null); const [evidence, set_evidence] = useState<{ brief: TopicBrief; citation: TopicCitation } | null>(null);
  const [reader, set_reader] = useState<Entry | null>(null); const active = useRef<string | null>(null); const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; if (active.current) void api.cancel_agent_task("topic", active.current).catch(() => {}); }; }, []);
  async function refresh() { await qc.invalidateQueries({ queryKey: ["topic", id] }); await qc.invalidateQueries({ queryKey: ["topics"] }); }
  async function act(work: () => Promise<unknown>) { set_error(""); set_message(""); set_busy(true); try { await work(); await refresh(); } catch (e) { if (alive.current) set_error(String(e)); } finally { if (alive.current) set_busy(false); } }
  async function generate() {
    const request_id = crypto.randomUUID(); active.current = request_id;
    set_busy(true); set_phase("preparing"); set_error(""); set_message("");
    const channel = new Channel<string>(); channel.onmessage = (raw) => { if (!alive.current || active.current !== request_id) return; try { const event = JSON.parse(raw); if (event.type === "phase") set_phase(event.phase); else if (event.type === "task_state") set_phase(event.state); } catch { /* malformed progress never becomes content */ } };
    try { const result = await api.generate_topic_brief(id, request_id, i18n.resolvedLanguage ?? i18n.language, channel); if (alive.current && active.current === request_id) { if (result) { set_version(result); set_tab("brief"); } else set_message(t("topics.noChanges")); } }
    catch (e) { if (alive.current && active.current === request_id) set_error(String(e)); }
    finally { await refresh(); if (alive.current && active.current === request_id) { active.current = null; set_busy(false); set_phase(""); } }
  }
  async function open_article(id: number) { await act(async () => { const result = await api.load_entry_detail(id); if (alive.current) set_reader(result.entry); }); }
  const data = detail.data; const brief = data?.history.find((b) => b.id === version) ?? data?.history[0];
  if (!data) return <div className="p-5" role="status">{detail.isError ? <><p role="alert">{String(detail.error)}</p><Button onClick={() => void detail.refetch()}>{t("topics.retry")}</Button></> : t("topics.loading")}</div>;
  const missing = data.input.sources.filter((s) => !s.body.trim()).length;
  const baseline = data.history[0];
  const removed = baseline?.input.sources.filter((s) => !data.input.sources.some((c) => c.entry_id === s.entry_id)) ?? [];
  return <>
    <div className="shrink-0 border-b border-border px-4 py-3">
      <div className="flex flex-wrap items-center gap-2"><h2 className="mr-auto break-words text-xl font-semibold">{data.topic.name}</h2>
        <Button variant="ghost" size="sm" disabled={busy} onClick={() => void act(refresh)}><RefreshCw />{t("topics.refresh")}</Button>
        <Button variant="outline" size="sm" disabled={busy} onClick={() => set_editing(true)}>{t("topics.edit")}</Button>
        <Button variant="outline" size="sm" disabled={busy} onClick={() => void act(() => api.save_topic(id, data.topic.name, data.topic.description, !data.topic.archived, data.topic.rule, data.topic.rule_name))}>{t(data.topic.archived ? "topics.restore" : "topics.archive")}</Button>
        <Button variant="ghost" size="sm" disabled={busy} onClick={() => set_deleting(true)}>{t("topics.delete")}</Button>
      </div><p className="mt-2 whitespace-pre-wrap break-words text-sm text-muted-foreground">{data.topic.description || t("topics.noDescription")}</p>
      <div className="mt-3 flex flex-wrap items-center gap-3 text-xs"><span>{t("topics.scope", { count: data.input.sources.length, missing })}</span><span className={data.changed ? "text-primary" : "text-muted-foreground"}>{t(data.changed ? "topics.scopeChanged" : "topics.upToDate")}</span>{baseline && <span>{t("topics.baseline", { id: baseline.id, time: topic_time(baseline.created_at, i18n.language) })}</span>}</div>
    </div>
    <nav className="flex shrink-0 flex-wrap items-center gap-2 border-b border-border p-2" aria-label={t("topics.sections")}>
      {(["sources", "brief", "notes"] as const).map((v) => <Button key={v} size="sm" variant={tab === v ? "secondary" : "ghost"} aria-pressed={tab === v} onClick={() => set_tab(v)}>{t(`topics.${v}`)}</Button>)}
      <div className="ml-auto flex flex-wrap gap-2">{active.current ? <Button variant="destructive" size="sm" onClick={() => void api.cancel_agent_task("topic", active.current!).catch((e) => set_error(String(e)))}>{t("topics.cancel")}</Button> : <Button size="sm" disabled={busy || !data.input.sources.length || !data.changed || data.topic.archived} onClick={() => void generate()}>{t(baseline ? "topics.generateIncremental" : "topics.generate")}</Button>}</div>
    </nav>
    <div aria-live="polite" className="shrink-0 px-4 text-sm">{phase && <p className="py-2">{t(`topics.phase.${phase}`, { defaultValue: phase })}</p>}{message && <p className="py-2">{message}</p>}{error && <p role="alert" className="break-words py-2 text-destructive">{error}</p>}</div>
    <div className="min-h-0 flex-1 overflow-y-auto p-4">
      {tab === "sources" && <div className="mx-auto max-w-5xl space-y-5">
        <p className="rounded-lg border border-border p-3 text-sm">{model_info.data?.routes.length ? t("topics.model", { name: model_info.data.routes.map((r) => `${r.provider_name} / ${r.model_name}`).join(" → ") }) : t("topics.aiHint")}{model_info.isError && <span role="alert"> {String(model_info.error)}</span>}</p>
        <p className="text-sm leading-6 text-muted-foreground">{t("topics.generateHint")}</p>
        {!!missing && <p role="status" className="rounded-lg border border-border bg-muted p-3 text-sm">{t("topics.missingHint", { count: missing })}</p>}
        <section><h3 className="mb-2 font-semibold">{t("topics.included")}</h3>{!data.input.sources.length && <p className="text-sm text-muted-foreground">{t("topics.noSources")}</p>}
          <div className="space-y-2">{data.input.sources.map((source) => { const old = baseline?.input.sources.find((s) => s.entry_id === source.entry_id); const state = !old ? "new" : old.content_version !== source.content_version || old.group_name !== source.group_name ? "changed" : "used";
            return <SourceRow key={`${source.entry_id}-${source.content_version}-${source.group_name}`} family={[...new Set(data.input.sources.map((s) => s.group_name))].indexOf(source.group_name) + 1} source={source} state={state} disabled={busy} on_open={() => void open_article(source.entry_id)} on_remove={() => void act(() => api.update_topic_members(id, [source.entry_id], true))}
              on_group={(group) => void act(() => api.update_topic_members(id, [source.entry_id], false, group))}
              on_fetch={() => void act(async () => { await api.build_reader_content(source.entry_id); await qc.invalidateQueries({ queryKey: ["entries"] }); })} />;
          })}</div>
        </section>
        {!!removed.length && <section><h3 className="font-semibold">{t("topics.removed")}</h3>{removed.map((s) => <p key={s.entry_id} className="text-sm text-muted-foreground">#{s.entry_id} · {s.title}</p>)}</section>}
        <Candidates key={JSON.stringify(data.topic.rule)} topic={data.topic} included={data.input.sources.map((s) => s.entry_id)} disabled={busy} on_add={(ids) => void act(() => api.update_topic_members(id, ids))} on_open={(id) => void open_article(id)} />
        <details className="border-t border-border pt-3"><summary className="cursor-pointer text-sm">{t("topics.runHistory")}</summary><div className="space-y-2 pt-3">{data.runs.map((r) => <p key={r.request_id} className="break-words text-xs">{topic_time(r.created_at, i18n.language)} · {t(`topics.run.${r.status}`, { defaultValue: r.status })}{r.error && ` · ${r.error}`}</p>)}</div></details>
      </div>}
      {tab === "brief" && <div className="mx-auto max-w-3xl">{brief ? <>
        <div className="mb-4 flex flex-wrap items-center gap-3"><label className="text-sm">{t("topics.history")}<select className={`${topic_field} mt-1`} value={brief.id} onChange={(e) => set_version(Number(e.target.value))}>{data.history.map((b) => <option key={b.id} value={b.id}>#{b.id} · {topic_time(b.created_at, i18n.language)}</option>)}</select></label><Button variant="outline" disabled={busy} onClick={() => void act(async () => { const { save } = await import("@tauri-apps/plugin-dialog"); const path = await save({ defaultPath: `topic-${id}-v${brief.id}.md`, filters: [{ name: "Markdown", extensions: ["md"] }] }); if (path) { await api.export_topic_brief(id, brief.id, path); if (alive.current) set_message(t("topics.exported")); } })}>{t("topics.export")}</Button></div>
        <p className="mb-2 text-xs text-muted-foreground">{brief.model.route?.provider_name} · {brief.model.route?.model_name ?? brief.model.model} · {t("topics.versionScope", { count: brief.input.sources.length, baseline: brief.baseline_id ?? "—" })}</p>
        {brief.input.hash !== data.input.hash && <p role="status" className="mb-4 rounded border border-border bg-muted p-3 text-sm">{t("topics.stale")}</p>}
        <p className="mb-6 text-xs leading-6 text-muted-foreground">{t("topics.citationLimit")}</p>
        {brief.result.claims.map((claim, index) => <section key={index} className="mb-5 rounded-xl border border-border p-4"><h3 className="mb-2 text-xs font-semibold text-primary">{t(`topics.kind.${claim.kind}`)}</h3><p className="whitespace-pre-wrap break-words text-sm leading-7">{claim.text}</p><div className="mt-3 flex flex-wrap gap-2">{claim.citations.map((citation, ci) => <Button variant="outline" size="sm" key={ci} onClick={() => set_evidence({ brief, citation })}>{t("topics.citation", { id: citation.entry_id })}</Button>)}</div>{!claim.citations.length && <p className="mt-3 text-xs text-muted-foreground">{t("topics.insufficientEvidence")}</p>}</section>)}
        <details><summary className="cursor-pointer text-sm">{t("topics.frozenScope")}</summary>{brief.input.sources.map((s) => <p key={s.entry_id} className="break-all py-1 text-xs">#{s.entry_id} · {s.title} · {s.group_name} · {s.content_version}</p>)}</details>
      </> : <div className="py-12 text-center"><p>{t("topics.noBrief")}</p><p className="mt-3 text-sm text-muted-foreground">{t("topics.aiHint")}</p></div>}</div>}
      <div hidden={tab !== "notes"}><TopicNotes key={id} id={id} initial={data.topic.notes} /></div>
    </div>
    {editing && <TopicEditor topic={data.topic} on_close={() => set_editing(false)} on_saved={() => set_editing(false)} />}
    {deleting && <Dialog open aria_label={t("topics.delete")} onOpenChange={(o) => { if (!o && !busy) set_deleting(false); }}><DialogContent><DialogHeader><DialogTitle>{t("topics.delete")}</DialogTitle></DialogHeader><p className="my-4 text-sm">{t("topics.deleteHint")}</p><DialogFooter><Button variant="outline" disabled={busy} onClick={() => set_deleting(false)}>{t("topics.cancel")}</Button><Button variant="destructive" disabled={busy} onClick={() => void act(async () => { await api.delete_topic(id); on_deleted(); })}>{t("topics.delete")}</Button></DialogFooter></DialogContent></Dialog>}
    {evidence && <Evidence brief={evidence.brief} citation={evidence.citation} current={data.input.sources} on_close={() => set_evidence(null)} on_open={() => void open_article(evidence.citation.entry_id)} />}
    {reader && <Dialog open aria_label={reader.title ?? t("topics.source")} className="h-[85vh] max-w-5xl" onOpenChange={(o) => { if (!o) { set_reader(null); void refresh(); } }}><div className="flex h-full min-h-0 flex-col"><div className="flex shrink-0 items-center justify-between border-b border-border p-2"><Button variant="ghost" onClick={() => { set_reader(null); void refresh(); }}><ArrowLeft />{t("topics.back")}</Button><span className="text-xs text-muted-foreground">{t("topics.currentArticle")}</span></div><div className="min-h-0 flex-1"><ReaderPane entry={reader} on_select_entry={set_reader} /></div></div></Dialog>}
  </>;
}

function SourceRow({ source, family, state, disabled, on_open, on_remove, on_group, on_fetch }: { source: TopicSource; family: number; state: string; disabled: boolean; on_open: () => void; on_remove: () => void; on_group: (s: string) => void; on_fetch: () => void }) {
  const { t } = useTranslation(); const [group, set_group] = useState(source.group_name.startsWith("auto:") ? "" : source.group_name);
  return <article className="rounded-lg border border-border p-3"><div className="flex flex-wrap items-start gap-2"><button className="min-w-0 flex-1 break-words text-left text-sm font-medium text-primary hover:underline" onClick={on_open}>#{source.entry_id} · {source.title || t("entry.untitled")}</button><span className="rounded bg-muted px-2 py-1 text-xs">{t(`topics.state.${state}`)}</span><Button variant="ghost" size="icon-sm" disabled={disabled} onClick={on_remove} aria-label={t("topics.remove", { title: source.title })}><X /></Button></div>
    <p className="mt-1 break-all text-xs text-muted-foreground">{source.feed} · {source.url}</p><div className="mt-2 flex flex-wrap items-center gap-2"><span className="rounded bg-muted px-2 py-1 text-xs">{t("topics.family", { index: family })}</span><span className="text-xs">{source.body.trim() ? t("topics.bodyReady", { count: source.body.length }) : t("topics.bodyMissing")}</span><Button size="sm" variant="outline" disabled={disabled} onClick={on_fetch}>{t("topics.fetch")}</Button></div>
    <div className="mt-2 flex flex-wrap items-center gap-2"><label className="flex min-w-0 flex-1 items-center gap-2 text-xs">{t("topics.group")}<input className={`${topic_field} max-w-64`} maxLength={100} value={group} placeholder={t("topics.autoGroup")} onChange={(e) => set_group(e.target.value)} /></label><Button variant="ghost" size="sm" disabled={disabled} onClick={() => on_group(group.trim())}>{t("topics.saveGroup")}</Button></div><p className="mt-1 text-xs text-muted-foreground">{t("topics.groupHint")}</p>
  </article>;
}

function Candidates({ topic, included, disabled, on_add, on_open }: { topic: Topic; included: number[]; disabled: boolean; on_add: (ids: number[]) => void; on_open: (id: number) => void }) {
  const { t } = useTranslation(); const [search, set_search] = useState(""); const [applied, set_applied] = useState(""); const [use_rule, set_use_rule] = useState(!!topic.rule); const [selected, set_selected] = useState<Set<number>>(new Set());
  const filter: EntryFilter = use_rule && topic.rule ? topic.rule : { search: applied };
  const key = JSON.stringify(filter);
  const candidates = useInfiniteQuery({ queryKey: ["topic-candidates", topic.id, key], initialPageParam: undefined as EntryCursor | undefined, queryFn: ({ pageParam }) => api.load_entries({ ...filter, cursor: pageParam, limit: 30 }), getNextPageParam: (last) => last.next_cursor ?? undefined });
  const rows = candidates.data?.pages.flatMap((p) => p.entries).filter((e) => !included.includes(e.id)) ?? [];
  return <section className="rounded-xl border border-border p-4"><h3 className="font-semibold">{t("topics.candidates")}</h3><p className="my-2 text-xs leading-6 text-muted-foreground">{t("topics.candidateHint")}</p>
    {topic.rule && <><label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={use_rule} onChange={(e) => { set_use_rule(e.target.checked); set_selected(new Set()); }} />{t("topics.boundRule", { name: topic.rule_name })}</label><details className="my-2 text-xs"><summary className="cursor-pointer">{t("topics.ruleDetails")}</summary><pre className="max-h-48 overflow-auto whitespace-pre-wrap break-all rounded bg-muted p-2">{JSON.stringify(topic.rule, null, 2)}</pre></details></>}
    {!use_rule && <form className="my-3 flex gap-2" onSubmit={(e) => { e.preventDefault(); set_applied(search); set_selected(new Set()); }}><label className="min-w-0 flex-1"><span className="sr-only">{t("topics.search")}</span><input value={search} onChange={(e) => set_search(e.target.value)} placeholder={t("topics.search")} className={topic_field} /></label><Button type="submit" variant="outline">{t("topics.searchAction")}</Button></form>}
    <div className="my-3 flex flex-wrap gap-2"><Button size="sm" disabled={disabled || ![...selected].some((id) => !included.includes(id))} onClick={() => { on_add([...selected].filter((id) => !included.includes(id))); set_selected(new Set()); }}>{t("topics.addSelected", { count: [...selected].filter((id) => !included.includes(id)).length })}</Button><Button variant="ghost" size="sm" onClick={() => void candidates.refetch()}>{t("topics.refresh")}</Button><span className="text-xs text-muted-foreground">{t("topics.matches", { count: candidates.data?.pages[0]?.total ?? 0 })}</span></div>
    {candidates.isFetching && <p role="status" className="text-sm">{t("topics.loading")}</p>}{candidates.isError && <p role="alert" className="text-sm text-destructive">{String(candidates.error)} <Button onClick={() => void candidates.refetch()}>{t("topics.retry")}</Button></p>}
    <div className="max-h-80 overflow-y-auto">{rows.map((entry) => <div key={entry.id} className="flex items-start gap-2 border-t border-border py-2"><input type="checkbox" className="mt-1" aria-label={t("topics.selectArticle", { title: entry.title })} checked={selected.has(entry.id)} onChange={(e) => set_selected((old) => { const next = new Set(old); if (e.target.checked) next.add(entry.id); else next.delete(entry.id); return next; })} /><button className="text-left text-sm text-primary hover:underline" onClick={() => on_open(entry.id)}>{entry.title || t("entry.untitled")}</button></div>)}</div>
    {!rows.length && !candidates.isFetching && <p className="text-sm text-muted-foreground">{t("topics.noCandidates")}</p>}{candidates.hasNextPage && <Button variant="outline" className="mt-2" disabled={candidates.isFetching} onClick={() => void candidates.fetchNextPage()}>{t("topics.loadMore")}</Button>}
  </section>;
}

function TopicNotes({ id, initial }: { id: number; initial: string }) {
  const { t } = useTranslation(); const qc = useQueryClient(); const [draft, set_draft] = useState(initial); const [saved, set_saved] = useState(initial); const [status, set_status] = useState(""); const [busy, set_busy] = useState(false);
  return <section className="mx-auto max-w-3xl"><p className="mb-4 text-sm text-muted-foreground">{t("topics.notesHint")}</p><label><span className="sr-only">{t("topics.notes")}</span><textarea className={`${topic_field} min-h-64`} value={draft} onChange={(e) => set_draft(e.target.value)} /></label><div className="mt-3 flex items-center gap-3"><Button disabled={busy || draft === saved} onClick={async () => { set_busy(true); const value = draft; try { await api.save_topic_notes(id, value); set_saved(value); set_status(t("topics.saved")); await qc.invalidateQueries({ queryKey: ["topic", id] }); } catch (e) { set_status(String(e)); } finally { set_busy(false); } }}>{t("topics.saveNotes")}</Button><span role="status" className="text-sm">{draft !== saved ? t("topics.unsaved") : status}</span></div></section>;
}

function Evidence({ brief, citation, current, on_close, on_open }: { brief: TopicBrief; citation: TopicCitation; current: TopicSource[]; on_close: () => void; on_open: () => void }) {
  const { t } = useTranslation(); const source = brief.input.sources.find((s) => s.entry_id === citation.entry_id); const now = current.find((s) => s.entry_id === citation.entry_id);
  // Offsets are UTF-8 bytes from Rust. Decode bytes instead of slicing JS UTF-16.
  const bytes = new TextEncoder().encode(source?.body ?? ""); const decode = (start: number, end?: number) => new TextDecoder().decode(bytes.slice(start, end));
  const quote_ref = useRef<HTMLElement>(null); useEffect(() => { quote_ref.current?.scrollIntoView({ block: "center" }); }, []);
  return <Dialog open onOpenChange={(o) => { if (!o) on_close(); }} className="max-w-3xl" aria_label={t("topics.evidenceTitle")}><DialogContent><DialogHeader><DialogTitle>{t("topics.evidenceTitle")}</DialogTitle></DialogHeader><p className="my-2 break-words text-sm font-medium">#{source?.entry_id} · {source?.title}</p><p className="break-all text-xs text-muted-foreground">{source?.url}</p><p className="my-3 break-all text-xs">{t("topics.snapshotVersion", { id: brief.id, version: citation.content_version, start: citation.start, end: citation.end })}</p><p className="mb-3 text-sm">{t(now?.content_version === citation.content_version ? "topics.currentMatch" : "topics.oldSnapshot")}</p>
    <div className="max-h-[45vh] overflow-y-auto rounded-lg border border-border bg-muted/40 p-4 text-sm leading-7 whitespace-pre-wrap break-words" tabIndex={0}>{decode(0, citation.start)}<mark ref={quote_ref} className="bg-primary/20 text-foreground">{decode(citation.start, citation.end)}</mark>{decode(citation.end)}</div><p className="my-3 text-xs text-muted-foreground">{t("topics.citationLimit")}</p><DialogFooter><Button variant="outline" onClick={on_close}>{t("topics.close")}</Button><Button onClick={on_open}>{t("topics.openArticle")}</Button></DialogFooter></DialogContent></Dialog>;
}
