import assert from "node:assert/strict";
import { createServer } from "node:http";
import { readFile, readdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { build } from "esbuild";
const [playwright_path, fixture_path, executable_path] = process.argv.slice(2);
if (!executable_path) throw new Error("Usage: node tests/search_views/run_browser.mjs <playwright-core/index.mjs> <Rust pages.json> <browser.exe>");
const { chromium } = await import(pathToFileURL(path.resolve(playwright_path)).href);
const dir = path.dirname(fileURLToPath(import.meta.url));
const fixture = await readFile(fixture_path, "utf8");
const bundle = await build({ entryPoints: [path.join(dir, "browser.tsx")], bundle: true, write: false, platform: "browser", format: "esm", jsx: "automatic", tsconfig: path.join(dir, "../../tsconfig.app.json"),
  define: { "process.env.NODE_ENV": '"development"' }, plugins: [{ name: "synthetic-ipc", setup(b) {
    b.onResolve({ filter: /^@tauri-apps\/api\/core$/ }, () => ({ path: path.join(dir, "mock_transport.ts") }));
    b.onResolve({ filter: /^search-fixtures$/ }, () => ({ path: "fixture", namespace: "fixture" }));
    b.onLoad({ filter: /.*/, namespace: "fixture" }, () => ({ contents: `export default ${fixture}`, loader: "js" }));
  } }] });
const assets = path.join(dir, "../../dist/assets");
const css = (await Promise.all((await readdir(assets)).filter((f) => f.endsWith(".css")).map((f) => readFile(path.join(assets, f), "utf8")))).join("\n");
const server = createServer((req, res) => {
  res.setHeader("Content-Type", req.url === "/test.js" ? "text/javascript; charset=utf-8" : req.url === "/style.css" ? "text/css; charset=utf-8" : "text/html; charset=utf-8");
  res.end(req.url === "/test.js" ? bundle.outputFiles[0].text : req.url === "/style.css" ? css : '<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/style.css"><div id="root"></div><script type="module" src="/test.js"></script>');
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const output = await mkdtemp(path.join(tmpdir(), "mercury4win-search-"));
let browser;
const passed = [], errors = [];
try {
  browser = await chromium.launch({ executablePath: executable_path, headless: true });
  const context = await browser.newContext({ viewport: { width: 1200, height: 800 } });
  await context.route("**/*", (r) => r.request().url().startsWith(origin + "/") ? r.continue() : r.abort());
  const page = await context.newPage(); page.on("pageerror", (e) => errors.push(String(e)));
  await page.goto(origin);
  const sidebar_search = page.getByRole("button", { name: "全文搜索", exact: true });
  assert.equal(await sidebar_search.getAttribute("aria-pressed"), "false");
  await sidebar_search.click();
  assert.equal(await sidebar_search.getAttribute("aria-pressed"), "true");
  assert(await sidebar_search.evaluate((e) => e.classList.contains("bg-accent")));
  await sidebar_search.click();
  assert.equal(await sidebar_search.getAttribute("aria-pressed"), "false");
  assert.equal(await page.locator('[data-digest-selecting]').count(), 0);
  await sidebar_search.focus(); await page.keyboard.press("Space");
  assert.equal(await sidebar_search.getAttribute("aria-pressed"), "true");
  await page.getByRole("button", { name: "Source 1 1", exact: true }).click();
  assert.equal(await sidebar_search.getAttribute("aria-pressed"), "false");
  assert.equal(await page.locator('[data-entry-mode="browse"] input[type="search"]').count(),0);
  assert.equal(await page.getByRole("button",{name:"保存 / 管理智能视图",exact:true}).count(),0);
  await sidebar_search.click();
  passed.push("sidebar search has selected background; click/Space toggles the middle view, feed navigation clears selection");
  const list = page.locator('[data-digest-selecting]');
  const search = () => list.locator('input[type="search"]');
  async function edit_view() {
    if (await list.getAttribute("data-entry-mode") === "view") {
      assert.equal(await search().count(),0);
      await page.getByRole("button",{name:"编辑筛选",exact:true}).click();
      await search().waitFor();
    }
  }
  const rows = () => list.locator('[role="button"]');
  async function idle() { await page.waitForFunction(() => document.querySelector('[data-digest-selecting] [aria-busy]')?.getAttribute("aria-busy") === "false"); }
  await idle();
  await search().fill("方案"); await idle();
  await page.waitForFunction(() => document.querySelectorAll('[data-digest-selecting] mark').length >= 4);
  assert.equal(await rows().count(), 4); assert.equal(await list.locator("img").count(), 0);
  assert((await list.textContent()).includes("<img src=x onerror=evil()>"));
  passed.push("Rust DTOs: Chinese two-character matches, safe text-node highlights, body/note field badges and source");
  await page.getByRole("button",{name:"Source 1 1",exact:true}).click(); await idle();
  assert.equal(await list.getAttribute("data-entry-mode"),"browse");
  const browse_filter=await page.evaluate(()=>window.search_test.transport.calls.findLast((c)=>c.command==="load_entries").args.filter);
  assert.equal(browse_filter.feed_id,1); assert.equal(browse_filter.search ?? null,null);
  await page.screenshot({path:path.join(output,"feed-browse.png")});
  await sidebar_search.click(); await idle();
  await search().fill("方案"); await idle();
  await page.getByRole("button", { name: "查看命中笔记", exact: true }).click();
  await page.getByRole("heading", { name: "笔记独有测试", exact: true }).waitFor({state:"attached"});
  await page.getByRole("button", { name: "编辑", exact: true }).waitFor();
  passed.push("note hit opens the correct article and note panel through production AppShell");

  await page.getByRole("button", { name: "筛选", exact: true }).click();
  await page.getByLabel("订阅源", { exact: true }).selectOption("1");
  await page.getByLabel("阅读状态", { exact: true }).selectOption("unread");
  await page.getByLabel("笔记", { exact: true }).selectOption("yes");
  await page.getByLabel("仅收藏", { exact: true }).check();
  await page.getByLabel("Rust", { exact: true }).check(); await page.getByLabel("SQLite", { exact: true }).check();
  await list.getByRole("combobox", {name:"标签匹配模式",exact:true}).selectOption("all");
  await page.getByLabel("最近 N 天（相对当前时刻）", { exact: true }).fill("7");
  await page.getByLabel("搜索字段", { exact: true }).selectOption("body");
  await page.getByLabel("标题不得包含任一关键词", { exact: true }).check();
  await page.getByLabel("排序", { exact: true }).selectOption("relevance"); await idle();
  const filter = await page.evaluate(() => window.search_test.transport.calls.filter((c) => c.command === "load_entries").at(-1).args.filter);
  assert.equal(filter.feed_id, 1); assert.equal(filter.is_read, false); assert.equal(filter.has_note, true); assert.equal(filter.starred_only, true);
  assert.deepEqual(filter.tag_ids, [1, 2]); assert.equal(filter.tag_mode, "all"); assert.equal(filter.recent_days, 7); assert.equal(filter.search_field, "body"); assert.equal(filter.exclude_title, true);
  await page.getByRole("button", { name: "全部已读", exact: true }).click();
  const bulk = await page.evaluate(() => window.search_test.transport.calls.findLast((c) => c.command === "mark_all_read_in_scope").args.filter);
  assert.deepEqual(bulk, filter);
  passed.push("all combinable filters and sort use the same structured IPC filter for listing and scope-wide read");

  await page.getByRole("button", { name: "保存 / 管理智能视图", exact: true }).click();
  await page.getByLabel("视图名称", { exact: true }).fill("Rust 最近七天");
  await page.getByRole("button", { name: "创建视图", exact: true }).click();
  await page.getByRole("button", { name: "Rust 最近七天", exact: true }).waitFor();
  assert.equal(await sidebar_search.getAttribute("aria-pressed"), "false");
  await page.reload(); await page.getByRole("button", { name: "Rust 最近七天", exact: true }).click(); await idle();
  assert.equal(await list.getAttribute("data-entry-mode"),"view"); assert.equal(await search().count(),0);
  await page.getByRole("button",{name:"Source 2 1",exact:true}).click(); await idle();
  const plain=await page.evaluate(()=>window.search_test.transport.calls.findLast((c)=>c.command==="load_entries").args.filter);
  assert.equal(plain.feed_id,2);
  for(const key of ["search","tag_ids","recent_days","date_from","date_to","has_note","is_read","search_field","exclude_title"]) assert.equal(plain[key] ?? null,null,`Hidden ${key} leaked into feed navigation`);
  assert.equal(plain.starred_only,false); assert.equal(await search().count(),0);
  await page.getByRole("button",{name:"Rust 最近七天",exact:true}).click(); await idle();
  await edit_view();
  assert.equal(await search().inputValue(), "方案");
  const saved_filter = await page.evaluate(() => window.search_test.transport.calls.findLast((c) => c.command === "load_entries").args.filter);
  assert.equal(saved_filter.recent_days, 7); assert.equal(saved_filter.date_from ?? null, null);
  await page.getByRole("button", { name: "全库", exact: true }).click(); await idle();
  await page.getByRole("button", { name: "当前导航范围", exact: true }).click(); await idle();
  const restored_scope = await page.evaluate(() => window.search_test.transport.calls.findLast((c) => c.command === "load_entries").args.filter);
  assert.equal(restored_scope.feed_id, 1); assert.deepEqual(restored_scope.tag_ids, [1, 2]); assert.equal(restored_scope.starred_only, true);
  await search().fill("TCP"); await idle();
  await page.getByText("条件已修改，尚未更新保存的视图", { exact: true }).waitFor();
  await page.getByRole("button", { name: "保存 / 管理智能视图", exact: true }).click();
  await page.getByLabel("视图名称", { exact: true }).fill("Renamed view");
  await page.getByRole("button", { name: "仅重命名", exact: true }).click();
  await page.getByRole("button", { name: "Renamed view", exact: true }).waitFor(); await idle();
  await edit_view();
  assert.equal(await search().inputValue(), "方案", "rename must preserve saved conditions");
  await search().fill("TCP"); await idle();
  await page.getByRole("button", { name: "保存 / 管理智能视图", exact: true }).click();
  await page.getByRole("button", { name: "更新名称与当前条件", exact: true }).click(); await idle();
  await page.getByRole("button", { name: "Renamed view", exact: true }).click(); await idle();
  await edit_view();
  assert.equal(await search().inputValue(), "TCP");
  passed.push("view create/open/rename/update and reload persistence; relative days stored without a frozen date");

  for (const language of ["zh-Hans", "en"]) for (const theme of ["light", "dark"]) {
    await page.evaluate(({language,theme}) => { window.search_test.store.getState().set_locale(language); window.search_test.store.getState().set_theme(theme); }, {language,theme});
    await page.setViewportSize({width:800,height:600});
    await page.getByRole("button",{name:language==="en"?"Save / manage smart view":"保存 / 管理智能视图",exact:true}).click();
    const dialog=page.getByRole("dialog");
    await dialog.waitFor();
    assert(await dialog.evaluate((e) => {const content=e.querySelector("form");return content.getBoundingClientRect().left-e.getBoundingClientRect().left>=20;}), "Settings form needs the standard content inset");
    const overflow=await dialog.evaluate((e) => [...e.querySelectorAll("input,button")].filter((c)=>c.getBoundingClientRect().right>e.getBoundingClientRect().right+1).map((c)=>c.outerHTML));
    assert.deepEqual(overflow,[]);
    await page.screenshot({path:path.join(output,`view-settings-${language}-${theme}.png`)});
    await page.keyboard.press("Escape");
  }
  await page.evaluate(() => window.search_test.store.getState().set_locale("zh-Hans"));
  passed.push("smart-view settings use padded shared dialog/header/footer and grouped actions in both languages and themes");

  await page.getByRole("button", { name: "保存 / 管理智能视图", exact: true }).click();
  await page.keyboard.press("Escape");
  assert.equal(await page.getByRole("button", { name: "保存 / 管理智能视图", exact: true }).evaluate((e) => e === document.activeElement), true);
  await page.getByRole("button", { name: "保存 / 管理智能视图", exact: true }).click();
  await page.getByRole("button", { name: "删除此视图", exact: true }).click();
  await page.getByRole("button", { name: "Renamed view", exact: true }).waitFor({ state: "detached" });
  await idle(); assert(await rows().count() > 0);
  await sidebar_search.click(); await idle();
  passed.push("modal Escape restores focus; deleting a view leaves articles available");

  await search().fill("分页"); await idle(); await page.waitForFunction(() => document.querySelectorAll('[data-digest-selecting] [role="button"]').length === 100);
  await page.getByRole("button", { name: "加载更多", exact: true }).click(); await idle();
  assert.equal(await rows().count(), 135);
  assert.equal(new Set(await rows().allTextContents()).size, 135);
  passed.push("production infinite query consumes real Rust keyset cursor and loads 135 unique rows");

  for (const language of ["zh-Hans", "en"]) for (const theme of ["light", "dark"]) for (const width of [1200, 800]) {
    await page.evaluate(({language, theme}) => { window.search_test.store.getState().set_locale(language); window.search_test.store.getState().set_theme(theme); }, {language, theme});
    await page.setViewportSize({width, height: 600});
    const filters_label = language === "en" ? "Filters" : "筛选";
    const toggle = page.getByRole("button", {name:filters_label,exact:true});
    if (await toggle.getAttribute("aria-expanded") !== "true") await toggle.click();
    for (const panel_width of [320,200]) {
      await list.evaluate((e,width) => {e.parentElement.style.width=`${width}px`;},panel_width);
    const overflow = await list.evaluate((e) => [...e.querySelectorAll('input,select,button')].filter((c) => c.getBoundingClientRect().width > 0 && (c.getBoundingClientRect().right > e.getBoundingClientRect().right+1 || c.getBoundingClientRect().left < e.getBoundingClientRect().left-1)).map((c) => c.outerHTML));
    assert.deepEqual(overflow, [], `${language}/${theme}/${width}/${panel_width} controls overflow`);
      assert(await list.locator('[aria-busy]').evaluate((e) => e.getBoundingClientRect().height >= 60), "Expanded filters must leave visible results");
    }
    await list.evaluate((e) => {e.parentElement.style.width='320px';});
  }
  await page.screenshot({path:path.join(output,"english-dark-800.png")});
  passed.push("16 Chinese/English, light/dark, 1200/800 window and 320/200 panel layouts; controls fit and results remain visible");
  await page.evaluate(() => window.search_test.store.getState().set_locale("zh-Hans"));
  await page.getByLabel("最近 N 天（相对当前时刻）", {exact:true}).fill("0");
  await page.getByText("最近天数须为 1–36500 的整数。", {exact:true}).waitFor();
  await page.getByLabel("最近 N 天（相对当前时刻）", {exact:true}).fill("");
  await page.getByRole("button", {name:"重建本地搜索索引",exact:true}).click();
  await page.getByText("索引已重建",{exact:true}).waitFor();
  passed.push("invalid filter feedback and index rebuild/requery flow");
  await page.evaluate(() => {window.search_test.transport.fail();});
  await search().fill("没有匹配");
  await page.getByText("文章加载失败，请重试。",{exact:true}).waitFor();
  await page.getByRole("button",{name:"重试",exact:true}).click(); await idle();
  assert.equal(await rows().count(),0);
  passed.push("query failure distinguishes an empty result and recovers through production retry");
  assert.deepEqual(errors, []);
  await writeFile(path.join(output,"result.json"),JSON.stringify({passed,errors},null,2),"utf8");
  console.log(JSON.stringify({passed,output},null,2));
} finally { await browser?.close(); await new Promise((resolve) => server.close(resolve)); }
