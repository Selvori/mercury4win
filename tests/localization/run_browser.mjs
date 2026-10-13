import assert from "node:assert/strict";
import { createServer } from "node:http";
import { readFile, readdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { build } from "esbuild";

const [playwright_path, executable_path] = process.argv.slice(2);
if (!playwright_path || !executable_path) throw new Error("Usage: node tests/localization/run_browser.mjs <playwright-core/index.mjs> <browser.exe>");
const { chromium } = await import(pathToFileURL(path.resolve(playwright_path)).href);
const dir = path.dirname(fileURLToPath(import.meta.url));
const bundle = await build({ entryPoints: [path.join(dir, "browser.tsx")], bundle: true, write: false,
  platform: "browser", format: "esm", jsx: "automatic", tsconfig: path.join(dir, "../../tsconfig.app.json"),
  define: { "process.env.NODE_ENV": '"development"' }, plugins: [{ name: "offline-native", setup(builder) {
    builder.onResolve({ filter: /^@tauri-apps\/api\/core$|^@tauri-apps\/plugin-dialog$/ }, () => ({ path: path.join(dir, "mock_transport.ts") }));
  } }],
});
const css_files = (await readdir(path.join(dir, "../../dist/assets"))).filter(file => file.endsWith(".css"));
assert(css_files.length, "Run npm run build first");
const css = (await Promise.all(css_files.map(file => readFile(path.join(dir, "../../dist/assets", file), "utf8")))).join("\n");
const html = (await readFile(path.join(dir, "../../index.html"), "utf8"))
  .replace('<script type="module" src="/src/main.tsx"></script>', '<link rel="stylesheet" href="/style.css"><script type="module" src="/test.js"></script>');
const server = createServer((request, response) => {
  if (request.url === "/favicon.svg") { response.writeHead(204).end(); return; }
  response.setHeader("Content-Type", request.url === "/test.js" ? "text/javascript; charset=utf-8" : request.url === "/style.css" ? "text/css; charset=utf-8" : "text/html; charset=utf-8");
  response.end(request.url === "/test.js" ? bundle.outputFiles[0].text : request.url === "/style.css" ? css : html);
});
await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const output = await mkdtemp(path.join(tmpdir(), "mercury4win-localization-"));
let browser;
const passed = [], errors = [];
try {
  browser = await chromium.launch({ executablePath: executable_path, headless: true });
  const context = await browser.newContext({ viewport: { width: 1200, height: 800 }, locale: "en-US" });
  await context.route("**/*", route => route.request().url().startsWith(origin + "/") ? route.continue() : route.abort());
  const page = await context.newPage();
  page.on("pageerror", error => errors.push(String(error)));
  async function ready(language) {
    await page.waitForFunction(language => document.documentElement.lang === language && window.localization_test?.store.getState().locale === language, language);
    assert.equal(await page.locator("html").getAttribute("lang"), language);
    assert.equal(await page.evaluate(() => window.localization_test.store.getState().locale), language);
  }
  async function open_settings() {
    const language = await page.locator("html").getAttribute("lang");
    await page.getByRole("button", { name: language === "zh-Hans" ? "设置" : "Settings", exact: true }).click();
    await page.getByRole("combobox", { name: language === "zh-Hans" ? "界面语言" : "Language", exact: true }).waitFor();
  }
  async function choose_language(language) {
    await open_settings();
    await page.getByRole("dialog").getByRole("combobox", { name: /^(界面语言|Language)$/ }).selectOption(language);
    await ready(language);
    await page.keyboard.press("Escape");
  }
  await page.goto(origin); await ready("zh-Hans");
  await page.getByRole("button", { name: "设置", exact: true }).waitFor();
  assert((await page.locator("body").innerText()).includes("就绪"));
  assert.equal(await page.getByRole("combobox", { name: "界面语言", exact: true }).count(), 0);
  passed.push("Fresh browser starts in Chinese; main UI has no language selector; language is available only inside Settings");
  await open_settings();
  const select = page.getByRole("combobox", { name: "界面语言", exact: true });
  await select.focus(); await page.keyboard.press("ArrowDown"); await page.keyboard.press("Enter"); await ready("en");
  assert.equal(await page.evaluate(() => localStorage.getItem("mercury_locale")), "en");
  await page.reload(); await ready("en");
  await page.getByRole("button", { name: "Settings", exact: true }).waitFor();
  passed.push("Native keyboard language selection inside Settings updates the UI, persists, and survives page restart");
  await open_settings();
  for (const [language, names] of [
    ["en", ["Providers", "Models", "Profiles", "Usage", "Prompts", "Tag Library"]],
    ["zh-Hans", ["服务商", "模型", "任务配置", "用量", "提示词", "标签库"]],
  ]) {
    await page.getByRole("dialog").getByRole("combobox", { name: /^(界面语言|Language)$/ }).selectOption(language);
    await ready(language);
    for (const width of [1200, 800, 600]) {
      await page.setViewportSize({ width, height: 800 });
      const first = page.getByRole("button", { name: names[0], exact: true });
      const first_box = await first.boundingBox();
      for (const name of names) {
        const tab = page.getByRole("button", { name, exact: true });
        const box = await tab.boundingBox();
        assert(Math.abs(box.y - first_box.y) < 1, `${language} ${width}px: ${name} wrapped onto another row`);
        assert.equal(await tab.evaluate(element => getComputedStyle(element).whiteSpace), "nowrap");
      }
      if (width >= 800) {
        const fits = await first.evaluate(element => {
          const nav = element.parentElement;
          return nav.scrollWidth <= nav.clientWidth + 1;
        });
        assert(fits, `${language} normal settings width requires horizontal scrolling`);
        if (width === 1200) await page.screenshot({ path: path.join(output, `settings-${language}.png`) });
      }
      await first.focus();
      for (let index = 1; index < names.length; index++) await page.keyboard.press("Tab");
      const last = page.getByRole("button", { name: names.at(-1), exact: true });
      assert(await last.evaluate(element => document.activeElement === element), "Last settings tab lost keyboard access");
      await last.click();
      assert.equal(await last.getAttribute("aria-pressed"), "true");
    }
  }
  await page.keyboard.press("Escape");
  await page.setViewportSize({ width: 1200, height: 800 });
  passed.push("English/Chinese Settings tabs stay on one row at 1200/800/600px; normal dialog fits all tabs and last tab remains keyboard accessible");
  await choose_language("zh-Hans");
  await page.getByRole("button", { name: "全部订阅源" }).click();
  await page.getByText("Synthetic article", { exact: true }).waitFor();
  assert((await page.locator("body").innerText()).includes(await page.evaluate(() => new Date("2026-10-13T12:00:00Z").toLocaleDateString("zh-Hans"))));
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await page.getByRole("button", { name: "添加服务商", exact: true }).click();
  await page.getByRole("textbox", { name: "接口地址（如 https://api.openai.com/v1）", exact: true }).waitFor();
  await page.getByLabel("API 密钥", { exact: true }).waitFor();
  await page.getByRole("dialog").getByRole("combobox", { name: "界面语言", exact: true }).selectOption("en");
  await page.getByRole("textbox", { name: "Base URL (e.g., https://api.openai.com/v1)", exact: true }).waitFor();
  assert.equal(await page.locator("html").getAttribute("lang"), "en");
  await page.keyboard.press("Escape");
  passed.push("Article dates use UI language; open Settings form updates reactively with localized API labels");
  await choose_language("zh-Hans");
  await page.getByRole("button", { name: "导入 OPML", exact: true }).click();
  await page.getByRole("button", { name: "选择文件", exact: true }).click();
  const dialog = await page.evaluate(() => window.localization_test.native.dialogs.at(-1));
  assert.equal(dialog.title, "选择文件"); assert.equal(dialog.filters[0].name, "OPML 订阅文件");
  assert.deepEqual(dialog.filters[0].extensions, ["opml", "xml"]);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "导出多篇文摘", exact: true }).click();
  await page.getByRole("checkbox", { name: "选择 Synthetic article", exact: true }).check();
  await page.getByRole("button", { name: "预览", exact: true }).click();
  await page.getByRole("button", { name: "选择文件并导出", exact: true }).waitFor();
  await page.waitForFunction(() => window.localization_test.native.calls.some(c => c.command === "preview_digest"));
  assert.equal(await page.evaluate(() => window.localization_test.native.calls.findLast(c => c.command === "preview_digest").args.options.locale), "zh-Hans");
  await page.evaluate(() => window.localization_test.store.getState().set_locale("en"));
  await page.waitForFunction(() => window.localization_test.native.calls.findLast(c => c.command === "preview_digest").args.options.locale === "en");
  passed.push("OPML native dialog receives Chinese title/filter; digest IPC includes locale and reloads preview on switch");
  await page.reload(); await ready("en");
  await choose_language("zh-Hans");
  await page.reload(); await ready("zh-Hans");
  await page.setViewportSize({ width: 800, height: 600 });
  await open_settings();
  await page.screenshot({ path: path.join(output, "chinese-ui.png") });
  const bounds = await page.getByRole("combobox", { name: "界面语言", exact: true }).boundingBox();
  assert(bounds.width > 40 && bounds.x >= 0 && bounds.y + bounds.height <= 600);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "切换外观（当前：跟随系统）", exact: true }).click();
  await page.getByRole("button", { name: "切换外观（当前：浅色）", exact: true }).click();
  assert(await page.locator("html").evaluate(element => element.classList.contains("dark")));
  await open_settings();
  await page.screenshot({ path: path.join(output, "chinese-ui-dark.png") });
  await page.evaluate(() => localStorage.setItem("mercury_locale", "unsupported"));
  await page.reload(); await ready("zh-Hans");
  passed.push("Chinese choice survives restart; Settings language control fits 800×600 light/dark UI; invalid saved language falls back to Chinese");
  const denied = await context.newPage();
  denied.on("pageerror", error => errors.push(String(error)));
  await denied.addInitScript(() => Object.defineProperty(window, "localStorage", { get() { throw new DOMException("Storage denied", "SecurityError"); } }));
  await denied.goto(origin);
  await denied.getByRole("button", { name: "设置", exact: true }).click();
  await denied.getByRole("combobox", { name: "界面语言", exact: true }).selectOption("en");
  await denied.getByRole("combobox", { name: "Language", exact: true }).waitFor();
  assert.equal(await denied.locator("html").getAttribute("lang"), "en");
  assert.equal(errors.length, 0, errors.join("\n"));
  passed.push("Unavailable localStorage still permits in-memory language switching; no uncaught browser errors");
  await writeFile(path.join(output, "result.json"), JSON.stringify({ passed, errors }, null, 2), "utf8");
  console.log(JSON.stringify({ passed, output }, null, 2));
} finally {
  await browser?.close();
  await new Promise(resolve => server.close(resolve));
}
