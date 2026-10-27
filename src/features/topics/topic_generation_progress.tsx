import { useEffect, useState } from "react";
import { Check, LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

export interface TopicProgressFacts {
  batch?: number;
  max_batches?: number;
  candidates?: number;
  pending?: number;
  selected?: number;
  metadata_only?: number;
  cached?: boolean;
}

function duration(seconds: number) {
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

function ProgressClock({ started_at, waiting }: { started_at: number; waiting: boolean }) {
  const { t } = useTranslation();
  const [stage_started_at] = useState(() => Date.now());
  const [now, set_now] = useState(stage_started_at);
  useEffect(() => {
    const timer = window.setInterval(() => set_now(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);
  const elapsed = Math.max(0, Math.floor((now - started_at) / 1000));
  const stage_elapsed = Math.max(0, Math.floor((now - stage_started_at) / 1000));
  return <>
    <p role="timer" aria-live="off" className="text-xs tabular-nums text-muted-foreground">{t("topics.progress.elapsed", { total: duration(elapsed), stage: duration(stage_elapsed) })}</p>
    {waiting && stage_elapsed >= 20 && <p className="text-xs leading-5 text-muted-foreground">{t("topics.progress.waitingHint")}</p>}
  </>;
}

export function TopicGenerationProgress({ phase, facts, automatic }: { phase: string; facts: TopicProgressFacts; automatic: boolean }) {
  const { t } = useTranslation();
  const [started_at] = useState(() => Date.now());
  const steps = automatic ? ["prepare", "select", "write", "save"] : ["prepare", "write", "save"];
  const current = ["validating", "saving", "succeeded"].includes(phase) ? steps.length - 1
    : ["summarizing", "comparing"].includes(phase) ? steps.indexOf("write")
    : phase === "selecting" ? steps.indexOf("select") : 0;
  const waiting = ["selecting", "summarizing", "comparing"].includes(phase);
  return <section data-topic-generation-progress aria-label={t("topics.progress.title")} className="mx-4 my-3 min-h-24 max-h-[40vh] shrink space-y-3 overflow-y-auto rounded-lg border border-border bg-muted/40 p-3 text-sm">
    <div role="status" className="flex items-start gap-2 font-medium">
      <LoaderCircle aria-hidden="true" className="mt-0.5 size-4 shrink-0 animate-spin text-primary motion-reduce:animate-none" />
      <span>{t(`topics.phase.${phase}`, { defaultValue: t("topics.phase.preparing") })}</span>
      <span className="ml-auto shrink-0 text-xs font-normal text-muted-foreground">{t("topics.progress.step", { current: current + 1, total: steps.length })}</span>
    </div>
    <ol aria-label={t("topics.progress.steps")} className={`grid gap-2 ${automatic ? "grid-cols-4" : "grid-cols-3"}`}>
      {steps.map((step, index) => <li key={step} aria-current={index === current ? "step" : undefined} className={`min-w-0 space-y-1.5 text-xs ${index <= current ? "text-primary" : "text-muted-foreground"}`}>
        <div aria-hidden="true" className={`h-1 rounded-full ${index < current ? "bg-primary" : index === current ? "animate-pulse bg-primary/60 motion-reduce:animate-none" : "bg-border"}`} />
        <span className="flex items-center gap-1">{index < current && <Check aria-hidden="true" className="size-3 shrink-0" />}{t(`topics.progress.${step}`)}</span>
      </li>)}
    </ol>
    <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
      {facts.batch !== undefined && <span>{t("topics.progress.batch", { current: facts.batch, total: facts.max_batches })}</span>}
      {facts.candidates !== undefined && <span>{t("topics.progress.candidates", { count: facts.candidates, pending: facts.pending })}</span>}
      {facts.selected !== undefined && <span>{t("topics.progress.selected", { count: facts.selected })}</span>}
      {!!facts.metadata_only && <span>{t("topics.progress.metadata", { count: facts.metadata_only })}</span>}
      {facts.cached && <span>{t("topics.progress.cached")}</span>}
    </div>
    <ProgressClock key={`${phase}-${facts.batch ?? 0}`} started_at={started_at} waiting={waiting} />
  </section>;
}
