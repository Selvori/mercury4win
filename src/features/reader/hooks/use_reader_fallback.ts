// mercury4win-linux/src/features/reader/hooks/use_reader_fallback.ts
// Frontend Readability.js fallback for content extraction

import { Readability } from "@mozilla/readability";

export interface ReadabilityResult {
  content: string;
  title: string | null;
  byline: string | null;
}

/**
 * Extract article content using @mozilla/readability in a hidden iframe.
 * Used as a fallback when Rust decruft fails to extract adequate content.
 */
export async function extractViaReadability(
  raw_html: string,
  _base_url: string,
): Promise<ReadabilityResult | null> {
  const iframe = document.createElement("iframe");
  iframe.style.display = "none";
  document.body.appendChild(iframe);

  try {
    const doc = iframe.contentDocument!;
    doc.open();
    doc.write(raw_html);
    doc.close();

    const reader = new Readability(doc);
    const article = reader.parse();

    if (!article) return null;

    return {
      content: article.content ?? "",
      title: article.title ?? null,
      byline: article.byline ?? null,
    };
  } finally {
    document.body.removeChild(iframe);
  }
}
