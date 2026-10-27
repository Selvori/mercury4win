import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import type { TopicBrief, TopicCitation, TopicSource } from "@/types/topic";

export function TopicBriefArticle({ brief, on_citation, on_open }: {
  brief: TopicBrief;
  on_citation: (citation: TopicCitation) => void;
  on_open: (id: number) => void;
}) {
  const { t } = useTranslation();
  const references = new Map<number, { source: TopicSource | undefined; citations: TopicCitation[] }>();
  for (const claim of brief.result.claims) for (const citation of claim.citations) {
    let reference = references.get(citation.entry_id);
    if (!reference) {
      reference = { source: brief.input.sources.find((s) => s.entry_id === citation.entry_id), citations: [] };
      references.set(citation.entry_id, reference);
    }
    if (!reference.citations.some((c) => c.content_version === citation.content_version && c.start === citation.start && c.end === citation.end)) reference.citations.push(citation);
  }
  return <article data-topic-brief-article aria-label={t("topics.article")} className="py-3">
    <div data-topic-brief-body className="space-y-6 text-base leading-8 text-foreground">
      {brief.result.claims.flatMap((claim, index) => claim.text.split(/\n\s*\n/).filter((p) => p.trim()).map((paragraph, pi) => <p key={`${index}-${pi}`} className="whitespace-pre-line break-words">{paragraph}</p>))}
    </div>
    {!!references.size && <footer data-topic-brief-references className="mt-12 border-t border-border pt-6">
      <h3 className="mb-5 text-sm font-semibold">{t("topics.references")}</h3>
      <ol className="list-decimal space-y-5 pl-5 text-sm">
        {[...references].map(([id, reference]) => <li key={id} className="pl-1">
          <button className="break-words text-left font-medium text-primary hover:underline focus-visible:outline-ring" onClick={() => on_open(id)}>{reference.source?.title || t("entry.untitled")}</button>
          {reference.source?.feed && <p className="mt-1 text-xs text-muted-foreground">{reference.source.feed}</p>}
          {reference.source?.basis === "metadata" && <p className="mt-1 text-xs text-muted-foreground">{t("topics.metadataOnly")}</p>}
          <div className="mt-2 flex flex-wrap gap-2">{reference.citations.map((citation, index) => <Button key={`${citation.start}-${citation.end}`} variant="ghost" size="sm" onClick={() => on_citation(citation)} aria-label={reference.citations.length === 1 ? t("topics.citation", { id }) : t("topics.citationPart", { id, index: index + 1 })}>{t("topics.citation", { id })}{reference.citations.length > 1 && ` · ${index + 1}`}</Button>)}</div>
        </li>)}
      </ol>
      <p className="mt-6 text-xs leading-6 text-muted-foreground">{t("topics.citationLimitShort")}</p>
    </footer>}
  </article>;
}
