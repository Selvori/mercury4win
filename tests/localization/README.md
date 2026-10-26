# 中文界面与语言持久化验证

2026-10-13；开始基线为 `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加保留的全部未提交修复。本次进行中另一流程将此前修复提交为 `f732ee70f59a65589e0ef96ac29b289696e945af`；中文化仍是其上的未提交增量，涉及文件的开始快照与新提交一致。Windows / PowerShell 7.6.5、Node 24.12.0、Rust 1.97.0 GNU、Edge 155.0.4283.45。详细改动、结果及准备失败见 [中文化验证记录](../../docs/audits/localization-2026-10-13.md)。

## 实现

- 无有效偏好时默认简体中文；设置窗口顶部提供原生语言选择框，主界面没有独立语言控件，选择以 `mercury_locale` 保存，重启恢复。i18next、Zustand 和根 HTML 的语言同步；存储不可用仍可切换。开发热更新清理语言订阅。
- 中文用量文案保留小写 `token` 名称，不翻译为“词元”。
- 中文设置使用“接口地址 / API 密钥 / 模型标识 / 智能体”等文案，订阅源 / 文章 / 服务商术语统一；OPML、YAML 和文摘文件窗口标题及筛选说明双语化。
- 日期、统计数字和报表名称排序使用当前界面语言。UTC 统计边界及已配置的 AI 输出语言保持原契约。
- 文摘预览和导出传递可选 `DigestOptions.locale`，中文标题、来源、作者、笔记、正文和摘要生成说明同步；省略此字段的旧调用仍导出原英文。语言改变使预览缓存重新查询，导出仍校验展示内容。

## 命令

复用已安装的隔离测试依赖，不修改应用依赖或锁文件。浏览器 runner 使用生产 App / index.html / CSS / i18next / Zustand / IPC wrapper，只有 native transport 替换成内存 fixture；所有外部网络均拦截，浏览器使用独立临时 profile。

```powershell
npm.cmd run lint
npm.cmd run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
pwsh -NoProfile -File tests/reader_pipeline/run_rust.ps1 -Filter digest::
rustfmt --edition 2021 --check src-tauri/src/digest/templates.rs src-tauri/src/digest/m4w_007_tests.rs src-tauri/src/reader/m4w_013_tests.rs
node tests/accessibility/check_resources.mjs
node tests/localization/run_browser.mjs "$env:TEMP/mercury4win-scroll-tests/node_modules/playwright-core/index.mjs" 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'
node tests/accessibility/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/digest_export/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/entry_pagination/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/entry_filters/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/m4w-019-detail-wire.json"
node tests/usage_lifecycle/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/m4w-012-usage-fixture.json"
git diff --check
```

详情 / 用量 fixture 复用已存在的真实 Rust 输出，没有手工伪造。缺少 fixture 时按 [最终验证入口](../final_validation/README.md) 重新生成，或按对应专项 README 运行生产者；核对实际路径后再运行。

初次中文化实际通过：6 组真实 Edge、7 项文摘 Rust、37 组既有 DOM、359 / 359 资源键及插值 / 静态引用检查、lint / build / cargo check / 定向格式 / diff。浏览器结果和亮暗截图保存在 runner 返回的独立临时目录；当时目录为 `C:/Users/Selvori/AppData/Local/Temp/mercury4win-localization-aYj1tC`。

2026-10-13 用户后续要求将语言移入设置并保留 token 名称：本次 lint / build、6 组真实 Edge、可访问性及标签 / 设置关闭锁各 10 组 DOM、359 / 359 资源检查及 diff 通过。浏览器实际从设置下拉框执行键盘 / 鼠标切换，确认主界面无语言控件，重载恢复和窄窗口亮暗布局通过；结果目录为 `C:/Users/Selvori/AppData/Local/Temp/mercury4win-localization-3L9RoJ`。另执行 `node tests/tag_system/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"`；没有新增测试依赖或改变断言语义。本次仅修改前端 / 验证入口 / 文档，未重跑 Rust 或其他 DOM 套件。

## 验收边界

浏览器验证包含英文系统首次启动中文、中英键盘切换及页面重载恢复、打开的设置即时切换、中文日期和 API 标签、原生文件窗口参数、文摘语言 IPC / 缓存更新、800×600 亮暗布局，以及无效偏好 / localStorage 拒绝访问。Rust 使用内存数据库及随机临时文件，三种格式的中文输出与 UTF-8 文件字节一致，旧英文调用及语言切换后的过期预览拒绝写入通过。

页面重载验证不等于完整 native 应用退出重启。真实 Tauri / WebView2、系统文件选择窗口显示、屏幕阅读器、Linux / macOS、实际 Provider 未验收；第三方错误诊断及文章原文保留来源文本。没有访问用户数据库 / 凭据；本任务没有提交、推送或发布，另一流程的已有修复提交保留；未重跑完整 Rust / 全部 DOM 套件。

完整桌面应以隔离数据首次启动确认中文，切换中英后完整退出重开，检查侧栏 / 设置 / 阅读工具栏 / OPML / YAML / 文摘窗口及导出文件；确认已有 AI 语言配置未被覆盖，亮暗主题和窄窗口可操作。M4W-015 保持待验证。

## 英文设置导航单行修复

2026-10-13，HEAD f732ee7 加保留的中文化修改；Windows / Edge 155。原导航在英文 1200px 窗口将 Tag Library 放到第二行，新增布局检查实际失败后再修复。AgentSettings 减少按钮横向留白并使用不换行 / 水平滚动导航；常规窗口六个入口完全显示，窄窗口保持单行。

沿用上面的 lint / build、浏览器和 diff 命令，最新 runner 7 组通过，覆盖中英 1200 / 800 / 600px 的同排坐标、正常宽度无滚动以及最后入口的键盘和点击访问。目录 C:/Users/Selvori/AppData/Local/Temp/mercury4win-localization-OpgVC9，含 settings-en.png / settings-zh-Hans.png；截图已复核。此次未改 Rust、未重跑 Rust / 全套 DOM，native Tauri / WebView2 未验收。完整历史证据见中文化审查最后章节。

## 设置顶部导航高度修复

2026-10-26；基线 `4b652fb` 加保留的 native 验收记录、feed_cmd 与笔记样式未提交修改，以及本次设置布局增量。Windows / PowerShell 7.6.7、Node 24.12.0、Edge 156.0.4314.42。

新增浏览器布局断言先在未修复的生产组件上失败：中文 1200×800，任务配置页导航 clientHeight=37 / scrollHeight=44，按钮底部超出导航，复现用户截图的纵向滚动和裁切。原因是 flex 列中顶部导航允许收缩，而 overflow-x:auto 会使另一个轴的 visible 计算为 auto。修复语言栏 / 导航栏 shrink-0，并明确导航 overflow-y-hidden；保留窄窗口水平滚动，下方长内容独立纵向滚动。

```powershell
npm.cmd run lint
npm.cmd run build
node tests/localization/run_browser.mjs "$env:TEMP/mercury4win-settings-layout-runtime/node_modules/playwright-core/index.mjs" 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'
git diff --check
```

旧临时 Playwright 目录已不完整，先检查实际文件，再在新临时目录通过 `npm.cmd install --prefix "$env:TEMP/mercury4win-settings-layout-runtime" --no-save --package-lock=false playwright-core` 安装 1.64.1；未修改项目依赖或锁文件。首次布局断言实际失败，修复后 runner 8 组通过；补齐 mock 的默认 Prompt 返回 null 后再次 8 组通过。覆盖中英文六个设置页 × 1200×800 / 800×600 / 600×400 共 36 个布局场景：导航无纵向溢出、按钮和语言栏完整、内容底部不超出窗口；长 Prompt 内容可独立滚动，原有窄窗口最后入口点击 / Tab 检查保留。

lint / build 通过，build 保留既有 >500kB chunk 提示。最终结果与截图在 `C:/Users/Selvori/AppData/Local/Temp/mercury4win-localization-HJY2Cb`（result.json、settings-long-zh-Hans.png / settings-long-en.png）；真实浏览器使用生产 App / CSS，native transport 为合成内存 fixture，外部网络拦截。完整 Tauri / WebView2、其他平台与非空真实配置尚未验收，M4W-021 保持待验证。未访问用户数据库或凭据，未重跑 Rust / 全仓套件，未提交 / 推送。