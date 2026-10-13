import { StrictMode, act } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReaderNote } from "../../src/features/reader/components/reader_note";
import { render_markdown } from "../../src/lib/markdown";
import { notes } from "../reader_ownership/mock_bindings";
import malicious from "./fixtures/malicious.md";
import normal from "./fixtures/normal.md";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const results: string[] = [];
const host = document.getElementById("app")!;
function assert(value: unknown, message: string): asserts value {
  if (!value) throw new Error(message);
}
function pass(message: string) { results.push(message); }
function parse(md: string) {
  const container = document.createElement("div");
  container.innerHTML = render_markdown(md);
  return container;
}
function assert_safe(container: Element) {
  assert(!container.querySelector("script,style,iframe,object,embed,link,meta,base,svg,math,form,button,textarea,select"), "active HTML survived");
  for (const element of container.querySelectorAll("*")) {
    for (const attribute of element.attributes) {
      assert(!/^on/i.test(attribute.name), "event handler survived");
      assert(!["style", "id", "name", "srcdoc", "srcset", "data-danger", "aria-hidden"].includes(attribute.name), `unsafe attribute: ${attribute.name}`);
      if (["href", "src"].includes(attribute.name)) {
        const protocol = attribute.value.startsWith("#") ? "#" : new URL(attribute.value).protocol;
        assert(["http:", "https:"].includes(protocol) || (element.tagName === "A" && ["mailto:", "#"].includes(protocol)), `unsafe URL: ${attribute.value}`);
      }
    }
  }
  for (const input of container.querySelectorAll("input")) {
    assert(input.type === "checkbox" && input.disabled, `interactive input survived: ${input.outerHTML}`);
  }
}
async function run() {
  const attack = parse(malicious);
  assert_safe(attack);
  assert(attack.querySelector("h1")?.textContent === "Safe heading", "safe content was removed");
  assert(attack.textContent?.includes("safe paragraph"), "safe HTML text lost");
  assert(attack.querySelectorAll("input").length === 1 && attack.querySelector("input")!.checked, "checkbox lost or controls survived");
  assert(attack.querySelectorAll("[href]").length === 0, "dangerous/relative href survived");
  assert(attack.querySelectorAll("[src]").length === 1, "unsafe images survived or safe image lost");
  assert(!attack.querySelector("code")!.hasAttribute("class"), "arbitrary CSS class survived");
  pass("malicious fixture: active HTML, handlers, style, clobbering and mutation payloads removed");

  const schemes = ["javascript:alert(1)", "JaVaScRiPt:alert(1)", "java\tscript:alert(1)", "java\nscript:alert(1)", "vbscript:alert(1)", "data:text/html,evil", "file:///C:/secret", "tauri://localhost", "asset://localhost/secret", "blob:https://example.com/evil", "/relative", "//example.com", "ftp://example.com"];
  for (const url of schemes) {
    const escaped = url.replaceAll("&", "&amp;").replaceAll('"', "&quot;");
    const result = parse(`<a href="${escaped}">link</a><img src="${escaped}">`);
    assert_safe(result);
    assert(!result.querySelector("[href],[src]"), `scheme survived: ${url}`);
  }
  pass("13 dangerous, obfuscated, local and relative URL variants rejected on links and images");

  for (const payload of [
    '<math><mtext><table><mglyph><style><!--</style><img title="--><img src=x onerror=alert(1)>">',
    '<svg><style><a id="</style><img src=x onerror=alert(1)>"></a></style></svg>',
    '<svg><g/onload=alert(1)//<p>',
    '<p><noscript><img src=x onerror=alert(1)></noscript></p>',
  ]) {
    const result = parse(payload);
    assert_safe(result);
    const reparsed = document.createElement("div");
    reparsed.innerHTML = result.innerHTML;
    assert_safe(reparsed);
  }
  pass("4 isolated namespace/malformed HTML payloads stay safe after output reparse");

  const content = parse(normal);
  assert_safe(content);
  for (const tag of ["h1", "h2", "p", "strong", "em", "del", "blockquote", "ul", "ol", "li", "table", "thead", "tbody", "th", "td", "pre", "code", "hr", "br"]) {
    assert(content.querySelector(tag), `Markdown element lost: ${tag}`);
  }
  assert(content.querySelector("ol")!.start === 3, "ordered list start lost");
  assert(content.querySelector('th[align="right"]'), "table alignment lost");
  assert(content.querySelector("code.language-typescript")?.textContent?.includes("<img src=x onerror=alert(1)>"), "fenced code lost or rendered as HTML");
  assert(content.querySelectorAll("input:disabled").length === 2 && content.querySelectorAll("input:checked").length === 1, "GFM task list lost");
  assert(content.querySelectorAll("a[href]").length === 4, "safe links lost");
  assert(content.querySelector("img")?.alt === "图片", "safe Markdown image lost");
  assert(content.querySelector("img")?.title === "图片标题", "image title lost");
  pass("normal fixture: Chinese, headings, emphasis, lists, tables, code, tasks, safe links/images and HTML preserved");

  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
  const root = createRoot(host);
  notes.set(16, { entry_id: 16, markdown: malicious });
  const render = async () => act(async () => {
    root.render(<StrictMode><QueryClientProvider client={client}><ReaderNote entry_id={16} /></QueryClientProvider></StrictMode>);
    await new Promise((resolve) => setTimeout(resolve, 30));
  });
  try {
    await render();
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 30)); });
    const preview = host.querySelector(".prose");
    assert(preview, "note preview missing");
    assert_safe(preview);
    pass("production ReaderNote sanitizes stored malicious Markdown before React insertion");
    const edit = [...host.querySelectorAll("button")].find((button) => button.textContent === "note.edit");
    assert(edit, "edit button missing");
    await act(async () => edit.click());
    assert(host.querySelector("textarea")?.value === malicious, "sanitization changed stored editable Markdown");
    pass("editor retains original Markdown; sanitation affects display only");
  } finally {
    await act(async () => root.unmount());
    client.clear();
  }
  document.getElementById("result")!.textContent = JSON.stringify({ passed: results }, null, 2);
  await fetch("/result", { method: "POST", body: JSON.stringify({ passed: results }) });
}
run().catch(async (error) => {
  const result = { passed: results, error: String(error) };
  document.getElementById("result")!.textContent = JSON.stringify(result, null, 2);
  await fetch("/result", { method: "POST", body: JSON.stringify(result) });
});
