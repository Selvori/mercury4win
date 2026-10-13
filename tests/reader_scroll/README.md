# M4W-020 更换字体后滚轮失效

2026-10-13，HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加全部保留的未提交修改与本项。Windows / PowerShell 7.6.5、Node 24.12.0、Edge 155.0.4283.45、临时 playwright-core。没有访问用户数据库、凭据或外部文章，没有修改应用依赖及锁文件。

## 原因与修复

字体 / 字号 / 主题改变 Reader Query 身份。此前请求期间 `isFetching` 分支卸载 iframe，完成后在设置弹窗的 `inert` 背景下新建 iframe。真实 Edge 中，即使关闭弹窗恢复祖先的 `inert=false`，该 frame 仍不响应滚轮；手动改变 scrollTop 有效。最小原生探针与生产 ReaderPane 均复现：初始 wheel 后 scrollTop=400，更换 Serif 并关闭后 wheel 的 scrollTop=0，断言超时失败。

Reader Query 仅对相同文章保留上一份 HTML；已有正文在字体请求 / 后台刷新期间保持显示，以新 srcDoc 更新同一个 iframe。初次读取仍显示加载状态；切换文章不复用其他文章正文。AI 就绪、当前正文身份、失败重试、外链与 sandbox 沿用现有逻辑。共享 Dialog 保持原有实现。

## 可复现命令

复用 reader_theme 的 Rust HTML fixture；重新生成时按其 README 运行 `m4w_014_tests` 并设置 `M4W_THEME_FIXTURE`。mock 保留真实 Rust head，以 100 段合成正文替换 body，并延迟返回 100ms。实际执行生产 ReaderPane、字体窗口、Dialog、Zustand、i18next、IPC wrapper、构建 CSS，仅替换 native transport。

```powershell
npm.cmd install --prefix "$env:TEMP/mercury4win-scroll-tests" --no-audit --no-fund --package-lock=false playwright-core
npm.cmd run lint
npm.cmd run build
node tests/reader_scroll/run_browser.mjs "$env:TEMP/mercury4win-scroll-tests/node_modules/playwright-core/index.mjs" "$env:TEMP/mercury4win-theme-fixture.json" 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'
node tests/reader_theme/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/mercury4win-theme-fixture.json"
node tests/accessibility/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/translation/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/mercury4win-translation-fixture.json"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" ../reader_pipeline/browser.tsx
git diff --check
```

Runner 使用独立 headless Edge / 临时 profile 和 loopback server，拦截所有非本地请求；结束关闭浏览器与 server。浏览器路径由参数传入，可替换为实际安装的 Chromium 系浏览器。

## 实际结果

- 8 组真实浏览器通过：初始 Reader；Serif / 关闭按钮；System / Escape；Sans-serif / 背景关闭；字号 22→23；Classic→Paper；light→dark；字体响应前关闭。每组 `page.mouse.wheel(0, 400)` 后实际 scrollTop=400；没有用手动设置 scrollTop 代替滚轮成功。
- 字体三组额外验证：弹窗打开时背景不滚动；关闭后无残留 inert；可选择文字，点击正文后 PageDown 可滚动；按钮 / Escape 恢复工具栏焦点；sandbox 仍为 `allow-same-origin`。
- 既有主题 24、可访问性 10、翻译 10、归属 15、Reader 错误 / 链接 2 组 DOM 回归，共 61 组通过。
- lint / build / diff 通过；build 保留已有 >500kB chunk 提示。
- 准备阶段：初版字号 16→17 不在严格 Rust fixture 中，改为已生成的 22→23 后通过。真实 backdrop 的默认焦点行为与程序 click 不同，焦点恢复断言限定按钮 / Escape，backdrop 仍验证滚轮恢复。共享 Dialog 同步 iframe inert 属性的尝试在生产路径无效，已撤回；最终仅修改 ReaderPane。文档补丁曾因 WORKLOG 定位未匹配而失败，核对实际 UTF-8 标题后拆分追加。

## 待桌面验收

在完整 Tauri / WebView2 中打开长文章，更换三种字体、字号与样式，分别用按钮、Escape、背景关闭；鼠标置于正文持续向上 / 向下滚动，应正常，并可拖滚动条、选择文字。再验证快速连续改字号、切换文章及翻译后更换字体。

此次真实浏览器生产组件验证不等于 native Tauri IPC / WebView2 验收。WebView2、触控板、Linux / macOS 未运行；Rust 未改，未重跑 Rust 或全仓套件。M4W-020 保持待验证。
