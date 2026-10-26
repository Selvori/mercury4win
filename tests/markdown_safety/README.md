# M4W-016 笔记 Markdown 净化回归

## 展示策略

生产 `render_markdown` 先使用 marked，再通过 DOMPurify 3.4.16 净化。仅允许标题、段落、强调 / 删除线、引用、列表、表格、代码、分隔线、换行、链接、图片和只读 checkbox。保留 GFM 任务列表、代码语言类、列表起始编号与表格对齐。

- 链接：仅绝对 HTTP(S)、mailto、`#` 锚点。
- 图片：仅绝对 HTTP(S)。不允许 data、file、blob、Tauri / asset / 其他自定义协议或相对路径。笔记没有独立文档基地址，不让相对路径解析到应用 WebView。
- 属性按元素限制；不允许事件、style、id / name、任意 CSS class、srcset、srcdoc、data / aria 属性。
- 不允许脚本、SVG / MathML、iframe / object / embed、样式及表单控件；任务 checkbox 强制 disabled。
- 仅净化展示结果，数据库 / 编辑器中的原始 Markdown 不变。Markdown 内的代码文本仍作为文本显示。

## 执行

从仓库根目录运行。使用项目现有 esbuild；安全 DOM 测试使用单独安装的 jsdom，不加入项目依赖 / 锁文件。DOMPurify 官方明确不建议以 Happy DOM 判断净化安全性，见 [官方说明](https://github.com/cure53/DOMPurify#running-dompurify-on-the-server)。

```powershell
$task_dom_dir = Join-Path $env:TEMP 'mercury4win-markdown-tests'
npm.cmd install --prefix "$task_dom_dir" --no-save --package-lock=false --ignore-scripts jsdom@30.1.2
node tests/markdown_safety/run_dom.mjs "$task_dom_dir/node_modules/jsdom/lib/api.js"
```

jsdom 30.1.2 要求 Node `^22.22.2 || ^24.15.0 || >=26.0.0`；使用满足该要求的 Node。本次系统 Node v24.12.0 不满足，安装提示 EBADENGINE；实际测试改用 Codex 已有运行时 Node v24.19.0：

```powershell
& 'C:\Users\Selvori\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe' tests/markdown_safety/run_dom.mjs "$task_dom_dir/node_modules/jsdom/lib/api.js"
```

浏览器版无需额外依赖，运行后打开输出的 loopback URL，完成后 Ctrl+C 停止：

```powershell
node tests/markdown_safety/serve.mjs
```

成功输出 6 项 `passed`，失败输出 `error` 并返回 1。测试使用实际渲染函数、生产 ReaderNote、StrictMode 和 React Query；复用 M4W-001 内存 IPC 替身，不访问用户数据库、凭据或 AI。jsdom 默认不加载外部资源，`runScripts: outside-only` 只执行测试 bundle；断言检查被插入 DOM 的结构 / 属性，不将“脚本未执行”本身作为净化通过依据。

## 2026-10-12 实测

Windows / PowerShell 7.6.5；HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加 M4W-001、M4W-016 未提交修改。Node v24.19.0 / jsdom 30.1.2 实际通过：

1. 恶意 fixture 的脚本、事件、样式、DOM clobbering、危险表单内容被移除，安全文本 / 图片与只读勾选框保留。
2. 13 种混淆 / 危险 / 本地 / 相对 URL 在链接及图片上均被拒绝。
3. 4 个独立 SVG / MathML / 畸形 HTML 案例在净化输出再次解析后仍符合策略。
4. 正常 fixture 的中文、标题、强调、列表、编号、表格对齐、代码、任务框、四类安全链接、图片及安全 HTML 保留。
5. 生产 ReaderNote 在将存储笔记交给 React 插入前执行净化。
6. 编辑器保持原始 Markdown，净化仅影响展示。

`npm.cmd run lint` / `npm.cmd run build` 通过；M4W-001 原 15 项 Happy DOM 归属回归复测通过。初次安全回归发现严格 URL 正则也删除 checkbox type、ol start 和表格 align，现已为受限非 URL 属性增加专门配置并复测通过。

未验证：真实浏览器、Tauri / WebView2 桌面、macOS / Linux。仓库已有浏览器连接为空，因此不以 jsdom 通过代替桌面验收；M4W-016 保留待验证。桌面验收应使用隔离应用数据，将两份 fixture 粘贴为笔记并保存 / 重开，检查净化展示、编辑原文与普通 Markdown 排版；不要使用用户真实数据库。

## 2026-10-25 native 验收与排版修复

完整隔离 Tauri / WebView2 实际通过恶意副本保存、关闭重开及编辑原文，SQLite 只读确认普通 / 恶意两篇输入完整持久化。native 仅将 example.com 资源替换为 loopback，原始 fixture 未改；原始 jsdom 6 项与归属 15 项当日通过。普通样例实际发现 prose 无样式导致标题 / 列表 / 引用 / 代码排版失败，最小复用已有 reader-content 和笔记局部约束后 lint / build / 同 override 重建通过。用户物理 Escape 停止 Computer Use，因此修复后 native 视觉排版尚未复验，M4W-016 保持待验证。详见[报告、失败与脱敏证据](../../docs/audits/native-016-2026-10-25.md)；不要把结构断言或构建通过记为已完成 native 排版验收。

同日用户明确恢复授权后，仅继续上述断点，实际通过修复后的普通 Markdown 排版、关闭重开 / 编辑原文及完整退出重启后的两篇笔记持久展示，恶意原文仍完整；隔离库精确比对通过。未新增源码或重复构建 / 6 / 15 项回归，原中断与初始失败保留。M4W-016 最低条件已满足并关闭；native UIA 证据与原始 jsdom 结构断言的边界、其他平台未验证范围见同一报告。
