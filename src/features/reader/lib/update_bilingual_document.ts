// Patch only translation nodes in the loaded reader. Keeping the document and
// original blocks alive avoids iframe navigation, lost focus and image reloads.
export function update_bilingual_document(doc: Document, html: string): boolean {
  const next = new DOMParser().parseFromString(html, "text/html");
  const translation_selector = "[data-translation-for], [data-translation-wrapper]";
  const originals = (body: HTMLElement) => Array.from(body.querySelectorAll<HTMLElement>("*"))
    .filter((element) => !element.closest(translation_selector));
  const current_blocks = originals(doc.body);
  const next_blocks = originals(next.body);
  const direct_text = (element: HTMLElement) => Array.from(element.childNodes)
    .filter((node) => node.nodeType === Node.TEXT_NODE).map((node) => node.textContent).join("");
  // The canonical translation document adds source IDs to the same reader
  // structure. A different source must take the normal document load path.
  if (current_blocks.length !== next_blocks.length || current_blocks.some(
    (element, index) => element.tagName !== next_blocks[index].tagName
      || direct_text(element) !== direct_text(next_blocks[index]),
  )) return false;

  const block_selector = "p,h1,h2,h3,h4,h5,h6,li,pre,th,td,caption,figcaption,dt,dd,[data-translation-for]";
  const anchor = Array.from(doc.querySelectorAll<HTMLElement>(block_selector)).find((element) => {
    const rect = element.getBoundingClientRect();
    return rect.bottom > 0 && rect.top < (doc.defaultView?.innerHeight ?? 0)
      && !element.querySelector(block_selector);
  });
  const anchor_top = anchor?.getBoundingClientRect().top;
  const scroll_top = doc.scrollingElement?.scrollTop ?? 0;

  const sources = new Map<string, HTMLElement>();
  next_blocks.forEach((element, index) => {
    const source_id = element.dataset.sourceSegmentId;
    if (source_id) {
      current_blocks[index].dataset.sourceSegmentId = source_id;
      sources.set(source_id, current_blocks[index]);
    }
  });
  const desired = new Map(Array.from(next.querySelectorAll<HTMLElement>("[data-translation-for]"))
    .map((element) => [element.dataset.translationFor!, element]));
  const existing = new Map(Array.from(doc.querySelectorAll<HTMLElement>("[data-translation-for]"))
    .map((element) => [element.dataset.translationFor!, element]));
  for (const [source_id, element] of existing) {
    if (!desired.has(source_id)) {
      if (element.parentElement?.hasAttribute("data-translation-wrapper")) element.parentElement.remove();
      else element.remove();
    }
  }
  for (const [source_id, element] of desired) {
    const source = sources.get(source_id);
    if (!source) continue;
    const previous = existing.get(source_id);
    if (previous) {
      if (previous.textContent !== element.textContent) previous.textContent = element.textContent;
      continue;
    }
    // Provider output remains text, never HTML imported from a translation.
    const translation = doc.createElement("div");
    translation.className = "mercury-translation";
    translation.dataset.translationFor = source_id;
    translation.textContent = element.textContent;
    if (["TH", "TD", "CAPTION"].includes(source.tagName)) source.append(translation);
    else if (source.tagName === "DT") {
      const definition = doc.createElement("dd");
      definition.dataset.translationWrapper = source_id;
      definition.append(translation);
      source.after(definition);
    } else source.after(translation);
  }
  const next_style = next.head.querySelector("style#mercury-bilingual-style");
  const current_style = doc.head.querySelector("style#mercury-bilingual-style");
  if (next_style && !current_style) doc.head.append(doc.importNode(next_style, true));
  else if (!next_style) current_style?.remove();

  // A segment completing above the viewport adds height. Preserve the visible
  // block's offset, rather than its old pixel scrollTop or percentage.
  if (doc.scrollingElement) {
    if (anchor?.isConnected && anchor_top !== undefined) {
      doc.scrollingElement.scrollTop += anchor.getBoundingClientRect().top - anchor_top;
    } else doc.scrollingElement.scrollTop = scroll_top;
  }
  return true;
}
