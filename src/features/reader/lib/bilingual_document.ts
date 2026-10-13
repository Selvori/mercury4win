import type { SourceSegment } from "@/types/agent";

/** Source is the backend's canonical, annotated reader HTML. Translations are
 * always text nodes: provider output cannot introduce HTML or permissions. */
export function compose_bilingual_document(
  canonical_html: string,
  source_segments: SourceSegment[],
  translations: (string | null)[],
  original_html?: string,
): string {
  const parser = new DOMParser();
  const doc = parser.parseFromString(canonical_html, "text/html");
  if (original_html) {
    const original = parser.parseFromString(original_html, "text/html");
    doc.head.replaceWith(doc.importNode(original.head, true));
  }
  const blocks = new Map(Array.from(doc.querySelectorAll<HTMLElement>("[data-source-segment-id]"))
    .map((node) => [node.dataset.sourceSegmentId, node]));
  for (const source of source_segments) {
    const text = translations[source.order_index];
    const block = blocks.get(source.source_segment_id);
    if (!block || !text) continue;
    const translation = doc.createElement("div");
    translation.className = "mercury-translation";
    translation.dataset.translationFor = source.source_segment_id;
    translation.textContent = text;
    // A div between table rows/cells is invalid and can be moved outside the
    // table by the HTML parser. Keep cell/caption translations inside them.
    if (["TH", "TD", "CAPTION"].includes(block.tagName)) block.append(translation);
    else if (block.tagName === "DT") {
      const definition = doc.createElement("dd");
      definition.append(translation);
      block.after(definition);
    } else block.after(translation);
  }
  const style = doc.createElement("style");
  style.textContent = ".mercury-translation{white-space:pre-wrap;border-left:3px solid currentColor;padding:0.4em 0.8em;margin:0.5em 0 1.5em;opacity:.85}";
  doc.head.append(style);
  return `<!doctype html>${doc.documentElement.outerHTML}`;
}
