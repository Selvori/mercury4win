import fixture from "production-theme-fixture";

export class Channel<T> { onmessage = (_message: T) => {}; }
export async function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (command === "related_entries") return [] as T;
  if (command === "build_reader_content") {
    await new Promise((resolve) => setTimeout(resolve, 100));
    const key = `${args.themeId}:${args.fontFamily}:${args.fontSize}`;
    const html = fixture.rendered[key];
    if (!html) throw new Error(`Missing Rust Reader fixture: ${key}`);
    // Keep the production head; supply a long offline article for native scrolling.
    const body = Array.from({ length: 100 }, (_, i) => `<p>Paragraph ${i}: 中文正文 / wheel scrolling regression.</p>`).join("");
    return html.replace(/<body>[\s\S]*<\/body>/, `<body>${body}</body>`) as T;
  }
  throw new Error(`Unexpected native command: ${command}`);
}
