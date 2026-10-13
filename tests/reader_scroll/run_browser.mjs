// No app dependency changes: supply a separately installed playwright-core and
// the Rust-generated fixture used by tests/reader_theme. Run npm run build first.
import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { createServer } from "node:http";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { build } from "esbuild";

const [playwright_path, fixture_path, executable_path] = process.argv.slice(2);
if (!playwright_path || !fixture_path || !executable_path) {
  throw new Error("Usage: node tests/reader_scroll/run_browser.mjs <playwright-core/index.mjs> <theme-fixture.json> <browser.exe>");
}
const { chromium } = await import(pathToFileURL(path.resolve(playwright_path)).href);
const dir = path.dirname(fileURLToPath(import.meta.url));
const fixture = await readFile(fixture_path, "utf8");
const bundle = await build({
  entryPoints: [path.join(dir, "browser.tsx")], bundle: true, write: false,
  platform: "browser", format: "esm", jsx: "automatic", tsconfig: path.join(dir, "../../tsconfig.app.json"),
  define: { "process.env.NODE_ENV": '"development"' },
  plugins: [{ name: "offline-transport", setup(builder) {
    builder.onResolve({ filter: /^@tauri-apps\/api\/core$/ }, () => ({ path: path.join(dir, "mock_transport.ts") }));
    builder.onResolve({ filter: /^production-theme-fixture$/ }, () => ({ path: "fixture", namespace: "fixture" }));
    builder.onLoad({ filter: /.*/, namespace: "fixture" }, () => ({ contents: `export default ${fixture};`, loader: "js" }));
  } }],
});
const assets = path.join(dir, "../../dist/assets");
const css_files = (await readdir(assets)).filter((file) => file.endsWith(".css"));
assert(css_files.length, "Run npm run build to generate production CSS");
const css = (await Promise.all(css_files.map((file) => readFile(path.join(assets, file), "utf8")))).join("\n");
const server = createServer((request, response) => {
  const route = request.url;
  response.setHeader("Content-Type", route === "/test.js" ? "text/javascript; charset=utf-8" : route === "/style.css" ? "text/css; charset=utf-8" : "text/html; charset=utf-8");
  response.end(route === "/test.js" ? bundle.outputFiles[0].text : route === "/style.css" ? css :
    '<!doctype html><link rel="stylesheet" href="/style.css"><div id="app" style="height:100vh"></div><script type="module" src="/test.js"></script>');
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
let browser;
const passed = [];
try {
  browser = await chromium.launch({ executablePath: executable_path, headless: true });
  const page = await browser.newPage({ viewport: { width: 1200, height: 800 } });
  await page.addInitScript(() => {
    localStorage.setItem("mercury_theme", "light");
    localStorage.setItem("mercury_reader_style", "classic");
    localStorage.setItem("mercury_font_family", "sans-serif");
    localStorage.setItem("mercury_font_size", "22");
  });
  const errors = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  await page.route("**/*", (route) => route.request().url().startsWith(`http://127.0.0.1:${server.address().port}/`) ? route.continue() : route.abort());
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  const frame = page.locator("iframe");
  const settings = page.getByRole("button", { name: "Reader Settings", exact: true });
  const close = page.getByRole("button", { name: "Close", exact: true });
  async function ready() {
    await frame.waitFor();
    await page.waitForFunction(() => document.querySelector("iframe")?.contentDocument?.querySelectorAll("p").length === 100);
  }
  const scroll_top = () => frame.evaluate((f) => f.contentDocument.scrollingElement.scrollTop);
  async function wheel(label) {
    await ready();
    await frame.evaluate((f) => { f.contentDocument.scrollingElement.scrollTop = 0; });
    await frame.hover();
    await page.mouse.wheel(0, 400);
    await page.waitForFunction(() => document.querySelector("iframe")?.contentDocument?.scrollingElement.scrollTop > 100, undefined, { timeout: 1500 });
    passed.push(`${label}: native wheel scrollTop=${await scroll_top()}`);
  }
  async function changed(action) {
    const old_html = await frame.getAttribute("srcdoc");
    await action();
    await page.waitForFunction((old_html) => document.querySelector("iframe")?.getAttribute("srcdoc") !== old_html, old_html);
    await ready();
    assert(await page.locator("#app").evaluate((e) => e.inert), "Background must remain inert during settings");
  }
  await wheel("initial reader");
  for (const [font, dismiss] of [["Serif", "button"], ["System", "escape"], ["Sans-serif", "backdrop"]]) {
    await settings.click();
    await changed(() => page.getByRole("button", { name: font, exact: true }).click());
    // The updated document is ready while settings is open. Wheel stays blocked.
    const box = await frame.boundingBox();
    await page.mouse.move(box.x + 30, box.y + 30);
    await page.mouse.wheel(0, 400);
    await page.waitForTimeout(150);
    assert.equal(await scroll_top(), 0, "Modal background scrolled");
    if (dismiss === "button") await close.click();
    else if (dismiss === "escape") await page.keyboard.press("Escape");
    else await page.mouse.click(5, 5);
    assert.equal(await page.locator("#app").evaluate((e) => e.inert), false, "Background inert leaked");
    if (dismiss !== "backdrop") assert.equal(await settings.evaluate((e) => document.activeElement === e), true, "Focus must return to settings trigger");
    await wheel(`${font} / ${dismiss} close`);
    assert.equal(await frame.getAttribute("sandbox"), "allow-same-origin");
    // Verify ordinary in-frame text selection and keyboard scrolling also resume.
    assert(await frame.evaluate((f) => {
      const doc = f.contentDocument;
      const range = doc.createRange(); range.selectNodeContents(doc.querySelector("p"));
      const selection = doc.getSelection(); selection.removeAllRanges(); selection.addRange(range);
      return selection.toString().includes("Paragraph 0");
    }), "Text selection unavailable after close");
    await frame.click({ position: { x: 100, y: 50 } });
    const before = await scroll_top();
    await page.keyboard.press("PageDown");
    await page.waitForFunction((before) => document.querySelector("iframe").contentDocument.scrollingElement.scrollTop > before, before, { timeout: 1500 });
  }
  for (const [label, action] of [
    ["font size", () => page.getByRole("button", { name: "Increase font size" }).click()],
    ["Paper style", () => page.getByLabel("Reader style", { exact: true }).selectOption("paper")],
    ["dark appearance", () => page.getByLabel("Appearance", { exact: true }).selectOption("dark")],
  ]) {
    await settings.click(); await changed(action); await close.click(); await wheel(label);
  }
  // Closing before a slow build completes also must leave a working new iframe.
  await settings.click();
  const old_html = await frame.getAttribute("srcdoc");
  await page.getByRole("button", { name: "Serif", exact: true }).click();
  await close.click();
  await page.waitForFunction((old_html) => document.querySelector("iframe")?.getAttribute("srcdoc") !== old_html, old_html);
  await wheel("close before reader response");
  assert.deepEqual(errors, []);
  console.log(JSON.stringify({ browser: browser.version(), passed }, null, 2));
} catch (error) {
  console.error(JSON.stringify({ passed, error: String(error.stack) }, null, 2));
  process.exitCode = 1;
} finally {
  await browser?.close();
  await new Promise((resolve) => server.close(resolve));
}
