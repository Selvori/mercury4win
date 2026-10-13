# M4W-014 阅读主题入口与系统主题同步

2026-10-13；HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加保留的此前串行修复和本项未提交改动。Windows / PowerShell 7.6.5、Node v24.12.0、Cargo / Rust 1.97.0 GNU、临时 Happy DOM 20.14.6。使用内存 SQLite、合成 localStorage、生产 Rust HTML fixture 和内存 native transport；不访问用户库、凭据、远程站点或真实 AI。

## 实现与边界

Reader 工具栏的“阅读设置 / Reader Settings”沿用现有字体窗口，提供 Classic / Paper 和跟随系统 / 浅色 / 深色选项。阅读样式只影响 Reader；亮暗模式与界面共享，字体始终使用已保存的显式选择。新样式键为 `mercury_reader_style`，默认 Classic；旧 `mercury_theme` 的 `auto` / `light` / `dark`、`mercury_font_family` 与 `mercury_font_size` 保持兼容，非法值回退，系统状态本身不持久化。新增双语文案、工具栏名称、select 关联 label、字体选中状态和字号 label；全局 Dialog 焦点行为仍留 M4W-015。

`useTheme` 在 App 中订阅一次 matchMedia change，订阅后重采样初始系统状态，并在卸载清理。系统状态写入 Zustand，使 App 根 class 与 Reader Query 同时更新；显式浅色 / 深色不会被系统事件覆盖。返回 auto 时立即读取当前系统值。Reader 请求身份仍为文章、样式与有效亮暗、字体、字号，原有 008 分层缓存继续使用。

Paper 采用上游 `ReaderTheme.swift` 已有两版配色（亮背景 `#f5efe6`、暗背景 `#1f1a16`，对应正文 / 次要文字 / 链接 / 代码 / 边界色），不改 Classic 或布局。两版正文、次要文字与链接在背景上均达到 4.5:1；显式 sans-serif / serif / system 不被样式覆盖。Reader render version 从 4 升为 5，以免命中原先与 Classic 相同的旧 Paper HTML；Readability / Markdown 仍为 3 / 2，schema 仍为 5。版本失效只重渲染当前 canonical Markdown，不重抓正文，不写 source 层。

双语回调携带用于生成 head 的 Reader HTML 身份，只有文章与当前 Reader HTML 同时一致才展示。沿用 004 的 canonical 来源块和当前 Reader head 复制；切换主题 / 字体或迟到的 translation document 都使用当前 head。文章切换回原文，迟到 Reader 请求不覆盖新文章 / 新样式；iframe sandbox 仍为 `allow-same-origin`。主题和字体设置不会发送 AI 生成请求。

## 复现命令

从仓库根目录运行；Happy DOM 使用此前已有临时安装，不修改依赖或锁文件。Windows GNU 测试启动沿用 008 的 Common Controls 6 sidecar 脚本，仅写忽略的构建产物。

```powershell
$env:M4W_THEME_FIXTURE = Join-Path $env:TEMP 'mercury4win-theme-fixture.json'
try {
    & tests/reader_pipeline/run_rust.ps1 -Filter 'm4w_014_tests'
} finally { Remove-Item Env:/M4W_THEME_FIXTURE }
node tests/reader_theme/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/mercury4win-theme-fixture.json"
& tests/reader_pipeline/run_rust.ps1 -Filter 'm4w_008_tests'
$env:M4W_TRANSLATION_FIXTURE = Join-Path $env:TEMP 'mercury4win-translation-fixture.json'
try {
    & tests/reader_pipeline/run_rust.ps1 -Filter 'translation_segments::tests'
} finally { Remove-Item Env:/M4W_TRANSLATION_FIXTURE }
node tests/translation/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/mercury4win-translation-fixture.json"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" ../reader_pipeline/browser.tsx
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm.cmd run lint
npm.cmd run build
rustfmt --edition 2021 --check src-tauri/src/reader/theme.rs src-tauri/src/reader/m4w_014_tests.rs src-tauri/src/reader/pipeline.rs
rustfmt --edition 2021 --config skip_children=true --check src-tauri/src/reader/mod.rs
git diff --check
```

## 实际结果

| 检查 | 实际结果 |
| --- | --- |
| M4W-014 Rust | 3 通过、0 失败 |
| 生产 Reader / 设置 / 翻译 / useTheme / 实际 TS wrapper DOM | 8 组 × 3 种初始状态，24 组通过 |
| M4W-008 Rust / DOM | 原 8 项 / 2 组通过 |
| M4W-004 Rust / DOM | 原 6 项 / 6 组通过 |
| M4W-001 DOM | 原 15 组通过 |
| cargo check | 通过，6 条既有警告 |
| lint / build | 通过；build 保留既有 >500kB chunk 提示，最终 lint 无警告 |
| 本项 4 文件定向格式 / diff 检查 | 通过 |

Rust 验证三种字体、12 / 16 / 22 / 24 字号与四种样式变体输出，以及 Paper 配色对比度；缓存测试生成 60 个 `style-variant:font:size` 身份（含 UI 的 23px），逐项读取匹配。首次正文完成后清空 entry 的 Feed 输入，URL 为不可访问的 loopback 端口，同时 SQLite trigger 禁止 content UPDATE，全部新样式仍成功：证明使用持久 Markdown，主题改变不重抓 / 不重建正文。另一 trigger 禁止缓存 INSERT 后相同身份命中仍成功。旧 version=4 缓存拒读并只生成 version=5 CSS，原 canonical 内容未被更新。

DOM 直接消费这些 Rust-rendered HTML，只有 Tauri transport 被替换，不 mock `tauri_bindings`、ReaderPane、字体窗口、ReaderTranslation、useTheme 或 i18next。三种 Window 初始状态为已保存 auto / Paper / serif / 22px，已保存 dark / Classic / system / 16px，以及非法值回退。覆盖 StrictMode setup / cleanup（始终一个监听）、初始化重采样、系统事件同时改变 root / srcDoc、强制浅色深色及 auto 恢复、真实 select label 与中英文资源、字体与大小持久化及上下限、双语当前 head / sandbox、旧主题请求与旧文章请求迟到、带旧暗色 / serif head 的 translation document 迟到后仍采用当前浅色 / sans-serif / 24px。mock 对未定义命令严格抛错，并断言没有任何 `run_` AI 命令。

## 中间准备失败与修正

- 早期读取历史命名的 store / hook / test 文件时路径不存在，随即通过实际文件清单确认 `app_store.ts` / `browser.tsx`，没有创建冒充项目目录；正式新增 `use_theme.ts` 是实现的一部分。文本读写明确 UTF-8。
- 首次本项 Rust 测试错误引用不存在的 Content.updated_at，编译失败；改为真实 Markdown / Readability 版本字段并保留禁止 source UPDATE 的 trigger 后 3 项通过。
- 首次 DOM 在已通过 3 组后因测试使用假设文案 `System Default` 失败；读取实际英文资源为 `System` 后纠正测试，最终 8 组 × 3 状态通过。初次 lint 提示测试 Shell 无 export，给测试组件正常 export 后最终 lint 无警告。
- 复测翻译时误用 `agent::translation_tests` filter 实际运行 0 项，不计为通过；读取真实模块 / 原回归说明后用 `translation_segments::tests`，实际 6 项通过并重新生成 fixture，随后原 6 组 DOM 通过。

未运行完整 native Tauri / WebView2、真实系统外观切换 / 退出重启、字体实际可用性与 iframe 视觉布局、Linux / macOS、真实 Provider / AI；Happy DOM 的 srcDoc 断言不代表真实排版验收。未执行全套 Rust 或全仓 fmt，018 历史断言 / 格式失败保留。M4W-014 标为待验证，尚未启动 M4W-015，未提交 / 推送 / 发布。
