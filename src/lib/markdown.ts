// mercury4win/src/lib/markdown.ts
// Frontend Markdown rendering (optional - reader uses Rust-rendered HTML)

import { marked } from "marked";
import createDOMPurify from "dompurify";

// Keep note HTML independent of the app DOM: no scripts, styles, embeds,
// SVG/MathML, IDs/names, or arbitrary interactive controls.
const note_purifier = createDOMPurify(window);
const allowed_attributes: Record<string, readonly string[]> = {
  a: ["href", "title"],
  img: ["src", "alt", "title"],
  code: ["class"],
  ol: ["start"],
  th: ["align"],
  td: ["align"],
  input: ["type", "checked", "disabled"],
};

function allowed_url(value: string, image: boolean): boolean {
  if (!image && value.startsWith("#")) return true;
  try {
    // No base URL: relative paths must not navigate the app WebView or load
    // local resources. DOMPurify supplies already decoded attribute values.
    const protocol = new URL(value).protocol;
    return protocol === "http:" || protocol === "https:" ||
      (!image && protocol === "mailto:");
  } catch {
    return false;
  }
}

note_purifier.addHook("uponSanitizeElement", (node) => {
  if (node.nodeName !== "INPUT") return;
  const input = node as HTMLInputElement;
  if (input.getAttribute("type")?.toLowerCase() !== "checkbox") {
    input.remove();
  } else {
    // GFM task lists remain visible but cannot collect user input.
    input.setAttribute("disabled", "");
  }
});

note_purifier.addHook("uponSanitizeAttribute", (node, attribute) => {
  const tag = node.nodeName.toLowerCase();
  if (!allowed_attributes[tag]?.includes(attribute.attrName)) {
    attribute.keepAttr = false;
  } else if (attribute.attrName === "href" || attribute.attrName === "src") {
    attribute.keepAttr = allowed_url(attribute.attrValue, tag === "img");
  } else if (attribute.attrName === "class") {
    attribute.keepAttr = /^language-[\w-]+$/.test(attribute.attrValue);
  } else if (attribute.attrName === "align") {
    attribute.keepAttr = /^(left|center|right)$/i.test(attribute.attrValue);
  } else if (attribute.attrName === "start") {
    attribute.keepAttr = /^\d+$/.test(attribute.attrValue);
  }
});

export function render_markdown(md: string): string {
  const html = marked.parse(md, { async: false }) as string;
  return note_purifier.sanitize(html, {
    ALLOWED_TAGS: [
      "a", "b", "blockquote", "br", "code", "del", "em", "h1", "h2", "h3",
      "h4", "h5", "h6", "hr", "i", "img", "input", "li", "ol", "p", "pre",
      "s", "strong", "table", "tbody", "td", "th", "thead", "tr", "ul",
    ],
    ALLOWED_ATTR: [...new Set(Object.values(allowed_attributes).flat())],
    ALLOW_ARIA_ATTR: false,
    ALLOW_DATA_ATTR: false,
    ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto):|#)/i,
    // These non-URL values are restricted to checkbox / digits / alignment
    // above; they must not be checked against the stricter URL protocol rule.
    ADD_URI_SAFE_ATTR: ["type", "start", "align"],
  });
}
