# M4W-015 双语文案与可访问性回归

2026-10-13；HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加保留的此前串行修复及本项未提交改动。Windows / PowerShell 7.6.5、Node v24.12.0、临时 Happy DOM 20.14.6。上游只读参考 README、docs/l10n.md、SwiftUI 的 Reader toolbar / Font chooser 与 OPML / batch 的可访问名称及键盘入口。没有访问真实用户数据库、凭据、AI 或远程订阅，没有提交 / 推送 / 发布。

## 实现范围

- 根据当前生产界面审查，Reader 错误和标签空状态已使用双语键，保持其既有诊断原文。补齐主题切换的当前模式名称、Feed 同步固定状态 / 失败回退、Feed 删除名称、Feed URL 表单标签、OPML 文件选择窗口标题 / 完成按钮、自动摘要确认取消键和共享错误重试文案；中英资源为 351 / 351 键，插值参数一致。
- Feed 行的选择与删除改为同级原生 button，消除 button 嵌套；删除控件在键盘焦点进入行时可见。Feed toolbar、Reader 外链、Settings 图标编辑、Provider / Model / Profile 表单、笔记 / 标签输入均有可理解名称；Sidebar / Reader / Settings / 摘要详细度及文章 / 标签选择明确选中状态。
- Dialog 使用 body portal，所有打开层共享按深度 / 打开顺序排序的栈，统一视觉 z-index 和唯一 aria-modal 所有者；背景和父 modal 使用 inert，关闭后恢复此前 inert 值。只有顶层处理 Escape / backdrop；原 batch / Settings / 导出的关闭回调锁保持。Tab / Shift+Tab 在可用控件间循环，排除 disabled、负 tabIndex、隐藏祖先和未打开 details 正文，包含链接和 summary；无控件时焦点落在 panel，异步控件出现后自动进入。
- 关闭及组件卸载都恢复连接且可见的触发器；触发器已卸载 / 隐藏 / 禁用时回到父窗口首个控件，或背景首个可用控件。嵌套同时挂载、StrictMode 清理、局部控件先消费 Escape、IME / 重复 Escape 都有保护。程序焦点离开顶层会被拉回。
- 标签 Enter / 重命名、文章及标签选择与全局快捷键增加 IME / 重复保护。已有 iframe 桥补输入 / contenteditable / IME / link Enter 保护，普通正文仍转交父快捷键；未扩大 `sandbox="allow-same-origin"`，未添加远程脚本或权限。
- Reader / AI 错误、OPML / Feed 操作错误和 OPML 进度仅在对应作用域添加 alert / status；OPML 文件窗口失败显示固定本地化提示并保留诊断。没有更改后端错误协议。

## 实际命令与结果

从仓库根目录运行，使用已有临时 Happy DOM，不新增依赖或修改锁文件。本项测试真实 React / Query / i18next / 生产组件与实际 `tauri_bindings`；仅 native transport 替换为隔离数据。旧回归保持其原有 mock 范围。

```powershell
node tests/accessibility/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/accessibility/check_resources.mjs
node tests/tag_system/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/digest_export/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/reader_theme/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/mercury4win-theme-fixture.json"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/entry_filters/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/m4w-019-detail-wire.json"
node tests/entry_pagination/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
npm.cmd run lint
npm.cmd run build
git diff --check
```

主题与详情 fixture 沿用本轮 014 / 011 已生成的真实 Rust JSON，文件存在已确认；本项未重新执行 Rust。缺文件时按 [主题回归](../reader_theme/README.md) 与 [筛选回归](../entry_filters/README.md) 的命令生成，不能静默造 fixture 替代。

| 检查 | 实际结果 |
| --- | --- |
| 本项生产 DOM | 10 组通过；英文 / 中文固定提示、名称与错误分开验收 |
| 中英资源键、插值参数、无 BOM、静态界面键引用 | 351 / 351 一致，通过 |
| M4W-010 标签 / Settings 批量关闭锁 | 原 10 组通过 |
| M4W-007 文摘选择 / 导出 / Reader inert | 原 7 组通过 |
| M4W-014 主题 / 字体 / 系统订阅 | 原 8 组 × 3 初始状态，24 组通过 |
| M4W-001 文章归属 / 迟到异步结果 | 原 15 组通过 |
| M4W-011 /019 筛选 / 快捷键 / 实际详情 JSON | 原 6 组通过 |
| M4W-006 分页 | 原 6 组通过 |
| lint / build / diff | 通过；build 仍有已有 >500kB chunk 提示 |

10 组新 DOM 验证包括顶层 Tab / Shift+Tab、隐藏祖先 / details / disabled 排除、背景 inert / 程序焦点 / 快捷键抑制、嵌套视觉 z-index / 唯一 modal / 焦点恢复、IME Escape、同 render 父子开启、StrictMode、无可聚焦控件与 React 异步出现内容、关闭锁、触发器卸载后的安全回退、已有 inert 恢复、双语 Feed / OPML / Reader / 标签提示及控件名称、实际 wrapper 参数、Settings 表单名称、iframe 事件保护和文章 / 标签 Enter / Space handler。Happy DOM 不执行浏览器原生 button 的键盘默认激活，相关实测仍需桌面。

007 /010 /014 的回归脚手架仅将查询范围扩到 body 以包含生产 portal，React 仍挂载 app；文摘预览查询排除测试自身的 result pre，未放松业务断言。

## 中间准备失败

- 一次批量补表单名脚本按 LF 查找 CRLF 文本，定位失败且尚未写文件；检查实际换行后按真实标签定位，完成后 lint / build 通过。
- 第一轮新 DOM 中文空状态失败：英文手工标签测试修改了内存 fixture，中文继续读取了已应用标签。每种语言重置 native fixture 后通过，未改生产空状态行为。
- 加入 summary 焦点测试后，一次 child trigger 恢复断言失败：Happy DOM 的程序 `.click()` 不像真实鼠标点击那样自动给按钮焦点。明确先 focus 正在打开子窗口的 trigger 后复测通过。
- 007 首次 portal 复测错误抓取测试结果 pre 的 `Running...`，导致 expectedContent 断言失败。限定生产预览 `pre:not(#result)` 后原 7 组全部通过，没有修改导出协议。
- 最终静态界面键扫描发现既有自动摘要确认引用缺少 common.cancel；补齐两种语言后资源为351 /351，静态引用扫描、新10组DOM和lint /build重新通过。
- 两次预期 OPML 文件窗口失败场景输出 `fixture file dialog failed` 日志；测试同时确认作用域 alert 与本地化窗口标题，属于受控失败路径，整组通过。

## 未验收与桌面步骤

native Tauri / WebView2 的真实键盘默认行为、焦点顺序 / 可见轮廓、inert 与窗口切换、屏幕阅读器名称和朗读、窄窗口 / 亮暗布局、iframe 事件及页面内链接、完整退出重启、Linux / macOS 未验证。DOM 中的 z-index / srcDoc / 事件断言不能代替真实显示和系统焦点验收；没有运行全套 Rust 或全仓 fmt，历史 M4W-018 失败保留。

桌面验收应分别切换英文与简体中文：Tab 到 Feed toolbar、选择行和删除按钮，Enter / Space 操作；打开 Settings 再进入标签维护 / batch，验证仅最上层响应 Tab / Shift+Tab / Escape，生成和未审核状态不能关闭父窗口，Stop / Discard 后可正常退出并恢复 trigger 焦点。打开阅读设置、导出及 OPML，检查异步控件焦点、关闭和组件卸载恢复；使用 IME 输入标签及笔记不触发后台快捷键；正文 iframe 中 r / t / n 生效，输入或链接 Enter 保持自身行为。用屏幕阅读器检查控件名称、选中状态和作用域错误 / 进度提示。需使用隔离数据库，不以清空真实用户数据验收。

M4W-015 保持待验证；下一项为 M4W-018，本项没有启动下一编号。
