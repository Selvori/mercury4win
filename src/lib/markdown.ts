// mercury4win-linux/src/lib/markdown.ts
// Frontend Markdown rendering (optional - reader uses Rust-rendered HTML)

import { marked } from "marked";

export function render_markdown(md: string): string {
  return marked.parse(md, { async: false }) as string;
}
