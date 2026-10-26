import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { createServer } from "node:http";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { build } from "esbuild";

const [playwright_path, fixture_path, executable_path] = process.argv.slice(2);
if (!executable_path) throw new Error("Usage: node tests/translation_scroll/run_browser.mjs <playwright-core/index.mjs> <theme-fixture.json> <browser.exe>");
const { chromium } = await import(pathToFileURL(path.resolve(playwright_path)).href);
const dir = path.dirname(fileURLToPath(import.meta.url));
const fixture = await readFile(fixture_path, "utf8");
// Optional read-only comparison against a saved pre-fix ReaderPane source.
const baseline = process.env.M4W_READER_BASELINE ? await readFile(process.env.M4W_READER_BASELINE, "utf8") : null;
const bundle = await build({ entryPoints: [path.join(dir, "browser.tsx")], bundle: true, write: false,
  platform: "browser", format: "esm", jsx: "automatic", tsconfig: path.join(dir, "../../tsconfig.app.json"),
  define: { "process.env.NODE_ENV": '"development"' }, plugins: [{ name: "offline-transport", setup(builder) {
    if (baseline) builder.onLoad({ filter: /reader_pane\.tsx$/ }, (args) => ({ contents: baseline, loader: "tsx", resolveDir: path.dirname(args.path) }));
    builder.onResolve({ filter: /^@tauri-apps\/api\/core$/ }, () => ({ path: path.join(dir, "mock_transport.ts") }));
    builder.onResolve({ filter: /^production-theme-fixture$/ }, () => ({ path: "fixture", namespace: "fixture" }));
    builder.onLoad({ filter: /.*/, namespace: "fixture" }, () => ({ contents: `export default ${fixture};`, loader: "js" }));
  } }],
});
const assets = path.join(dir, "../../dist/assets");
const css = (await Promise.all((await readdir(assets)).filter((file) => file.endsWith(".css"))
  .map((file) => readFile(path.join(assets, file), "utf8")))).join("\n");
const server = createServer((request, response) => {
  response.setHeader("Content-Type", request.url === "/test.js" ? "text/javascript; charset=utf-8" : request.url === "/style.css" ? "text/css; charset=utf-8" : "text/html; charset=utf-8");
  response.end(request.url === "/test.js" ? bundle.outputFiles[0].text : request.url === "/style.css" ? css :
    '<!doctype html><link rel="stylesheet" href="/style.css"><div id="app" style="height:100vh"></div><script type="module" src="/test.js"></script>');
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
let browser;
const passed = [];
try {
  browser = await chromium.launch({ executablePath: executable_path, headless: true });
  const page = await browser.newPage({ viewport: { width: 1200, height: 800 } });
  await page.addInitScript(() => {
    localStorage.setItem("mercury_theme", "light"); localStorage.setItem("mercury_reader_style", "classic");
    localStorage.setItem("mercury_font_family", "sans-serif"); localStorage.setItem("mercury_font_size", "22");
  });
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  await page.route("**/*", (route) => route.request().url().startsWith(`http://127.0.0.1:${server.address().port}/`) ? route.continue() : route.abort());
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  const frame = page.locator("iframe");
  const translated = page.getByRole("button", { name: "Translation", exact: true });
  const ready = () => page.waitForFunction(() => document.querySelector("iframe")?.contentDocument?.getElementById("paragraph-99"));
  await ready();
  async function at_paragraph(index) {
    await frame.evaluate((f, index) => {
      f.contentDocument.getElementById(`paragraph-${index}`).scrollIntoView();
    }, index);
    await frame.hover(); await page.mouse.wheel(0, 50); await page.waitForTimeout(150);
  }
  const position = (index) => frame.evaluate((f, index) => ({
    top: f.contentDocument.getElementById(`paragraph-${index}`).getBoundingClientRect().top,
    scroll: f.contentDocument.scrollingElement.scrollTop,
  }), index);
  async function check_position(index, action, label) {
    const before = await position(index);
    await action(); await page.waitForTimeout(150);
    const after = await position(index);
    assert(Math.abs(after.top - before.top) <= 1, `${label}: visible paragraph moved ${JSON.stringify({ before, after })}`);
    assert(after.scroll > 1000, `${label}: reset to article start`);
    assert(await frame.evaluate((f) => f.contentDocument === window.saved_document && f.contentDocument.getElementById("paragraph-30") === window.saved_paragraph), `${label}: document or original block replaced`);
    passed.push(`${label}: visible offset ${before.top} -> ${after.top}; scrollTop ${before.scroll} -> ${after.scroll}`);
  }
  await at_paragraph(30);
  await frame.evaluate((f) => { window.saved_document = f.contentDocument; window.saved_paragraph = f.contentDocument.getElementById("paragraph-30"); });
  const start = async () => { await translated.click(); await page.waitForFunction(() => window.regression.running()); };
  if (baseline) {
    // Reposition after entering so the historical failure specifically tests
    // the next completed segment rather than the initial mode switch.
    await start(); await at_paragraph(30);
    await frame.evaluate((f) => { window.saved_document = f.contentDocument; window.saved_paragraph = f.contentDocument.getElementById("paragraph-30"); });
  } else await check_position(30, start, "enter translation midway");
  for (const [index, label] of [[70, "segment below viewport"], [0, "long segment above viewport"], [30, "current segment"], [2, "out-of-order segment"], [1, "next earlier segment"]]) {
    await check_position(30, () => page.evaluate(([index]) => window.regression.emit(index, "新增译文，保持阅读位置。".repeat(50)), [index]), label);
  }
  await check_position(30, () => page.evaluate(() => window.regression.emit(0, "Updated short translation")), "replace earlier translation");
  await check_position(30, () => page.evaluate(() => {
    window.regression.emit(100, '<img src="bad" onerror="alert(1)">');
    window.regression.emit(101, "Translated term"); window.regression.emit(102, "Translated definition");
  }), "table and definition segments");
  assert(await frame.evaluate((f) => {
    const doc = f.contentDocument;
    return doc.querySelector("#cell > [data-translation-for]")?.textContent.includes("<img")
      && doc.querySelector("#term + dd[data-translation-wrapper] [data-translation-for]")?.textContent === "Translated term"
      && !doc.querySelector("[data-translation-for] img");
  }), "Invalid table/definition insertion or unsafe provider HTML");
  await frame.evaluate((f) => {
    const doc = f.contentDocument;
    const translation = doc.querySelector('[data-translation-for="paragraph-30"]');
    translation.scrollIntoView();
    const range = doc.createRange(); range.setStart(translation.firstChild, 2); range.setEnd(translation.firstChild, 15);
    const selection = doc.getSelection(); selection.removeAllRanges(); selection.addRange(range);
    doc.getElementById("article-link").focus({ preventScroll: true });
    window.saved_selection = selection.toString();
  });
  const translated_top = () => frame.evaluate((f) => f.contentDocument.querySelector('[data-translation-for="paragraph-30"]').getBoundingClientRect().top);
  const before_translation = await translated_top();
  await page.evaluate(() => window.regression.emit(3, "Earlier translation ".repeat(100)));
  await page.waitForTimeout(150);
  assert(Math.abs(await translated_top() - before_translation) <= 1, "Visible translation moved when earlier text arrived");
  assert(await frame.evaluate((f) => f.contentDocument.getSelection().toString() === window.saved_selection
    && f.contentDocument.activeElement.id === "article-link"), "Selection or focus lost during segment update");
  passed.push("reading translated text preserves visible offset, selection and focus");
  await at_paragraph(30);
  await check_position(30, () => page.evaluate(() => {
    for (const index of [6, 4, 5]) window.regression.emit(index, "Concurrent segment ".repeat(60));
  }), "multiple segments in one render");
  await check_position(30, () => translated.click(), "return to original");
  assert.equal(await frame.evaluate((f) => f.contentDocument.querySelectorAll("[data-translation-for], [data-translation-wrapper]").length), 0);
  await page.evaluate(() => window.regression.finish());
  await page.evaluate(() => { window.regression.seed(2); window.regression.set_entry(2); });
  await ready();
  await page.waitForTimeout(150);
  assert.equal((await position(0)).scroll, 0, "Article switch inherited previous progress");
  await at_paragraph(30);
  await frame.evaluate((f) => { window.saved_document = f.contentDocument; window.saved_paragraph = f.contentDocument.getElementById("paragraph-30"); });
  await check_position(30, () => translated.click(), "restore complete cached translation");
  assert.equal(await frame.evaluate((f) => f.contentDocument.querySelectorAll("[data-translation-for]").length), 103);
  await page.setViewportSize({ width: 650, height: 480 }); await at_paragraph(30);
  await check_position(30, () => translated.click(), "narrow window returns to original");
  await check_position(30, () => frame.evaluate((f) => f.contentDocument.dispatchEvent(new KeyboardEvent("keydown", { key: "t", bubbles: true, cancelable: true }))), "iframe T shortcut restores cached translation");
  assert.equal(await frame.getAttribute("sandbox"), "allow-same-origin");
  assert.deepEqual(errors, []);
  console.log(JSON.stringify({ browser: browser.version(), passed }, null, 2));
} catch (error) {
  console.error(JSON.stringify({ passed, error: String(error.stack) }, null, 2)); process.exitCode = 1;
} finally {
  await browser?.close(); await new Promise((resolve) => server.close(resolve));
}
