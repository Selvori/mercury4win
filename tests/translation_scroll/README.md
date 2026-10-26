# M4W-022 分段翻译保持阅读位置

2026-10-26；基线 `4b652fb` 加任务开始前已有的未提交改动及本项修改。Windows / PowerShell 7.6.7 / Node 24.12.0 / Edge 156.0.4314.42，临时 playwright-core 与 Happy DOM。未访问用户数据库、凭据或真实 AI；应用依赖及锁文件未改。

## 原因与实现

原 ReaderPane 把每次完成段合成的整篇双语 HTML 传给 iframe.srcDoc。浏览器因此重新导航，滚动、选择和焦点都可能丢失。用 HEAD 的 ReaderPane 只读覆盖测试 bundle，进入翻译后重新滚到文章中部，再完成视口下方一段，实际复现 scrollTop 从 10148 变为 0，可见段落 top 从 -50.40625 变为 10097.59375。

现在 srcDoc 只随原始 Reader HTML 改变；同文章、同 Reader HTML 的译文在加载完成的文档内按来源段落更新。原文结构及直接文本一致时，只增删或更新译文文字节点，表格单元格与定义列表沿用合法插入方式。段落增长或缩短后，以当前可见内容块的视口偏移补偿滚动，保留原文节点、图片、选择与焦点。文章 ID 改变时重建 iframe；外观改变或实际来源结构 / 文字改变仍使用完整文档加载。sandbox、IPC 和后端分段协议保持。

## 复现命令

在仓库根目录运行。主题 fixture 由真实 Rust renderer 生成；mock 只将 body 换成 100 段合成正文及表格 / 定义列表，并提供可控制的翻译事件。生产 ReaderPane、ReaderTranslation、React Query、Zustand、i18next、IPC wrapper 与构建 CSS 均实际执行。浏览器和 server 退出时关闭，非 loopback 请求全部拦截。

```powershell
$env:M4W_THEME_FIXTURE = Join-Path $env:TEMP 'mercury4win-theme-fixture.json'
try { & tests/reader_pipeline/run_rust.ps1 -Filter 'm4w_014_tests' }
finally { Remove-Item Env:/M4W_THEME_FIXTURE }
$env:M4W_TRANSLATION_FIXTURE = Join-Path $env:TEMP 'mercury4win-translation-fixture.json'
try { & tests/reader_pipeline/run_rust.ps1 -Filter 'translation_segments::tests' }
finally { Remove-Item Env:/M4W_TRANSLATION_FIXTURE }
npm.cmd run lint
npm.cmd run build
node tests/translation_scroll/run_browser.mjs "$env:TEMP/mercury4win-settings-layout-runtime/node_modules/playwright-core/index.mjs" "$env:TEMP/mercury4win-theme-fixture.json" 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'
node tests/reader_scroll/run_browser.mjs "$env:TEMP/mercury4win-settings-layout-runtime/node_modules/playwright-core/index.mjs" "$env:TEMP/mercury4win-theme-fixture.json" 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'
node tests/translation/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/mercury4win-translation-fixture.json"
node tests/reader_theme/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/mercury4win-theme-fixture.json"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" ../reader_pipeline/browser.tsx
node tests/accessibility/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
git diff --check
```

浏览器与测试模块路径依照实际安装位置传入。runner 可通过 `M4W_READER_BASELINE` 读取另存的旧 ReaderPane.tsx，只替换测试 bundle 的模块内容，保留其原 resolveDir；此次使用 `git show HEAD:src/features/reader/components/reader_pane.tsx` 的 UTF-8 临时副本，不回写生产源码。旧代码运行预期断言失败。

## 实际结果

- 新增 14 组真实 Edge 回归通过：文章中段进入翻译、视口下方 / 上方 / 当前段、乱序完成、早期译文缩短更新、表格 / 定义列表及恶意输出纯文字、阅读译文时保持偏移 / 选区 / 焦点、同次渲染多段完成、回原文、完整缓存恢复、650×480 窄窗口和 iframe 的 T 快捷键。每次更新均断言同一 Document / 原文节点和可见段落偏移，误差不超过 1 CSS px；同时断言文章切换从顶部开始、103 段缓存完整和 sandbox 未扩大。
- 上方长段完成时 scrollTop 10148→11109，可见原文 top -50.40625→-50.15625；多段完成 13557→16084，top -49.9375→-49.53125。scrollTop 随新增内容高度调整，实际阅读位置保持。
- 字体滚轮既有真实 Edge 8 组通过；翻译 10、主题 24、归属 15、Reader 错误 / 链接 2、可访问性 10，共 61 组 DOM 通过。三个既有脚手架改为读取 iframe 的实时文档，继续保留全部业务断言。
- Rust fixture 准备实际运行主题 3 项、翻译分段 7 项，均通过；没有修改 Rust。本次 lint 无警告、build 通过，保留已有 >500 kB chunk 提示；diff 检查通过。
- 初次旧 DOM 回归仍读取静态 srcdoc，翻译 / 主题 / 归属断言失败；改为读取实际 contentDocument 后全部通过。测试 App 的初次 lint Fast Refresh 警告通过导出组件消除；失败与修正未当作通过。可访问性脚手架预期的两次模拟文件对话框失败仍有诊断输出，最终 10 组通过。

## 未验证

完整 Tauri / WebView2 的实际 Channel、真实 Provider、触控板、Linux / macOS 尚未运行。M4W-022 保持待验证；不以 headless Edge / DOM 代替完整桌面验收。未重跑完整 Rust / 全仓测试，未提交、推送或发布。

桌面最低验收：长文章翻译中持续向上 / 向下阅读，观察上方、下方及并发段完成；当前段落不回顶部、不跳位。补翻译 / 原文切换、缓存恢复、键盘 T、文章切换及译文中选字，确认已完成段和旧文章回调不会覆盖当前阅读。
