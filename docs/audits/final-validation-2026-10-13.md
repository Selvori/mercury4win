# 移植修复最终隔离验证 / M4W-018

日期：2026-10-13。结论：**当前工作区完整 Rust、前端构建、全部已登记隔离 DOM 回归与 Rust 格式基线通过。** M4W-018 的陈旧预期与格式问题已满足验收；native 桌面、真实 Provider、在线站点兼容及三平台验收仍未完成，其他功能问题按台账保持待验证。

## 基线与环境

- 仓库：`G:\sth_interesting\mercury4win`，HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de`，加所有此前串行修复与本项未提交修改。开始先确认实际 cwd、PowerShell、Git 根和 status；保留既有改动，无提交、推送、发布或版本标签。
- Windows / PowerShell 7.6.5、Rust / Cargo 1.97.0 `x86_64-pc-windows-gnu`。lint / build 使用现有 Node 24.12.0；最终各 DOM runner 使用已安装 Node 24.19.0。临时 Happy DOM 20.14.6 与 jsdom 30.1.2 已存在，没有增加依赖、升级包或修改锁文件。
- schema 5；Reader Readability / Markdown / render 为 3 / 2 / 5，本项不改变生产迁移、缓存版本、IPC 或权限。仅内存 / 临时 SQLite、本地 HTTP、合成数据与 Key、内存 native transport；未访问真实用户数据库或凭据，未向外部 AI 发送请求。
- 原审查 [2026-10-12](mercury-porting-2026-10-12.md) 的 `395f997` 工作区及历史失败保持原样；本报告是新的工作区验证，不能冒充已提交版本或三平台结果。

## 先运行的实际失败及修复分类

原样执行 `cargo test --locked --manifest-path src-tauri/Cargo.toml`：**129 通过、5 失败、0 忽略**。运行正常进入 harness；不是 executable 加载失败。

| 失败 | 原因与处理 |
| --- | --- |
| `test_load_builtin_tagging` | 实际 YAML version 2，旧断言为 1；改为 2，并验证中文输出约定、现有标签库与 title / byline / content 插值、缺 author 条件移除 |
| `test_load_hy_mt` | 004 已将模板升为 2 以支持串行上下文，旧断言仍为 1；验证 version 2、当前原文与前段原文 / 译文插值，缺上下文不输出前段提示 |
| `test_render_with_theme` | comrak 已生成标题锚点，旧 `<h1>Title</h1>` 整串匹配失效；明确验证 h1 内可见 Title、匹配的 href / id、隐藏锚点及正文，保留 Reader 外层断言 |
| `v3_upgrade_preserves_v2_and_legacy_payloads_and_repeats` | 用最新 schema 伪装 version 2，留下 v4 / v5 结构，重复加 alias；改用 cfg(test) 真正 V1 + V2 fixture，确认结果表未存在，升级到当前版本并重复运行，保留 summary / note，补 Feed 正文 / 收藏 / 外键断言 |
| `v4_upgrade_preserves_canonical_collisions_and_legacy_aliases` | 用最新 schema 伪装 version 3，只去除 v4 结构却留下 v5 archived_at；改用真实 V1 + V2 + V3 fixture，确认 alias / archived_at 未存在，保留冲突合并、别名与 usage_count、外键、重复迁移并核对当前版本 |

后两项属于测试输入过期，不代表合法历史用户库迁移已复现失败；生产迁移未因此增加“重复列容错”或修改版本判定。三个真实旧版本入口加既有 v4 fixture、全套新建 / 重复迁移检查在本轮均实际运行。没有删测试、ignore / skip、放宽失败状态或用单线程隐藏并发问题。007 已修复的两套文摘模板原测试在完整套件中通过。

## 格式与警告范围

初始 `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 失败，报告 **17 个文件**：build.rs，agent/mod.rs，commands/{feed_cmd,mod,note_cmd,reader_cmd}.rs，db/{connection,feed_store,mod,note_store}.rs，feed/{opml_import,sync_service,use_cases}.rs，lib.rs、main.rs，reader/{extraction,md_to_html}.rs。

最小移除六条未使用导入：feed_cmd 的 Arc / Mutex / entry_store，reader_cmd 的 Client，feed/use_cases 的 FeedWithCounts，reader/theme 的 Content。没有额外业务重构。随后用户授权范围内运行全仓 **Rust** cargo fmt；它实际改动 **22 个文件**，包含上面 17 个和本项新编辑的 prompt_templates.rs、migrations.rs、agent_result_store_tests.rs、tag_store_tests.rs、theme.rs。format 前复制全部 **71 个 .rs** 到临时快照；对副本逐个以 edition 2021 / skip_children 独立运行 rustfmt，逐 SHA-256 比较真实格式化结果，71 个全部一致。结合差异检查，format 阶段只有 formatter 生成的缩进、换行、尾逗号和声明排序；没有改其他语言格式。

最终全仓 cargo fmt check 通过，cargo check 及完整 cargo test 无 Rust warning。前项 Rust 业务改动的 diff 与本项纯格式 diff 共存在未提交工作区，不能把 HEAD 全部 diff 当作本项新改动。

## 最终实际验证

| 检查 | 实际结果 |
| --- | --- |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | **134 通过、0 失败、0 忽略**，默认并行 harness；binary 0 项与 doc 0 项入口也完成 |
| `cargo check --locked --manifest-path src-tauri/Cargo.toml` | 通过，0 Rust warning |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 全仓通过 |
| `npm.cmd run lint` | 通过 |
| `npm.cmd run build` | TypeScript / Vite 通过，保留已有 >500kB chunk 提示（主 chunk 543.62kB） |
| 15 套生产 DOM 回归 | **125 项 / 组通过、0 失败套件**，见下表 |
| `tests/accessibility/check_resources.mjs` | **351 英文 / 351 中文键**、插值、静态 UI 引用与 UTF-8 BOM 检查通过 |
| `git diff --check`、工作区 UTF-8 / 相对文档链接核对 | 通过；205 个新增 / 修改文本严格 UTF-8、0 BOM，92 个本地 Markdown 链接有效；未将 target / dist / node_modules、日志、临时 DB / JSON / HTML 纳入修改 |

普通 cargo test 的 lib / binary / doc 命令确实成功；当前 GNU executable 旁已有前项生成的 Common Controls 6 sidecar，不能推断全新机器无需该环境准备。四个 fixture 生产步骤另用 [run_rust.ps1](../../tests/reader_pipeline/run_rust.ps1) 运行精确 filter，脚本只写 ignored target 下 sidecar并临时添加 WebView2 loader PATH。它们各执行 1 项；这些准备步骤不重复计入 134。

| DOM 套件 | 实际通过项 / 组 |
| --- | ---: |
| reader_ownership / M4W-001 | 15 |
| feed_content / M4W-005 | 2 |
| reader_pipeline / M4W-008 | 2 |
| entry_pagination / M4W-006 | 6 |
| entry_filters / M4W-011 /019 | 6 |
| agent_runtime / M4W-003 | 8 |
| agent_persistence / M4W-002 | 7 |
| translation / M4W-004 | 6 |
| agent_configuration / M4W-009 | 8 |
| digest_export / M4W-007 | 7 |
| tag_system / M4W-010 | 10 |
| usage_lifecycle / M4W-012 | 8 |
| reader_theme / M4W-014（8 × 3 初始状态） | 24 |
| accessibility / M4W-015 | 10 |
| markdown_safety / M4W-016 | 6 |
| 合计 | **125** |

安全净化只使用 jsdom 30.1.2，其他交互使用 Happy DOM；不是用 Happy DOM 判断 DOMPurify 的安全性。DTO、translation document 与主题 HTML 从实际 Rust 测试生成到临时目录，用量 bundle 可直接加载 JSON，不改写仓库 fixture.ts。未运行不存在的 npm test。125 沿用各脚手架的项 / 组统计，不额外累加其中 13 种 URL 或 4 个 payload 等内部断言。

## 复现与证据保留

可复现脚本和依赖要求见 [最终验证入口](../../tests/final_validation/README.md)：

```powershell
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo check --locked --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm.cmd run lint
npm.cmd run build
pwsh -NoProfile -File tests/final_validation/run_dom.ps1
git diff --check
```

本轮日志 / 源码快照根为 `$env:TEMP/mercury4win-final-validation-20261013`；完整最终 DOM 在其 `dom-final/summary.json` 及逐套 log，四个生产 fixture 为 detail / translation / theme / usage.json。日志在本机临时目录，不作为永久源文件；本报告和 runner 保留可复现命令、逐项结果及失败分类。

初次 DOM 汇总脚本运行到 tag_system 时误把数字型 `passed: 10` 当成数组型 passed，断言计数为 0 而停止；该套实际产品回归已返回 10 通过。修正汇总器同时支持两种已存在的结果结构后，最终完整 15 套重跑得到 125，通过结果不包含那次中断未执行的套件。没有因该准备错误弱化产品断言。本次全部 Rust 默认并行运行没有复现 PENDING drain / WRITE_FAILURES 的跨测试失败，没有基于猜测修改生产计数、扩大 SQLite pool 或串行化套件。

## 尚未验证

未运行 native Tauri / WebView2 的真实 IPC / Channel、文件对话框 / 权限、默认键盘 / 焦点 / inert / iframe 操作、屏幕阅读器、字体与图片排版、真实系统外观事件、完整退出重启；真实 AI / include_usage Provider 兼容、在线 Feed / Obsidian Publish 适配和 macOS / Linux 构建及运行也未执行。固定 F-001 已在本轮完整 Rust 套件中解析 / 存储 / 去重 / 导出往返通过，但不是联网 11 源同步或 Tauri 对话框验收。

下一步是在隔离应用数据与合成凭据环境完成实际桌面及三平台验收，沿各 tests README 和 [台账](../ISSUES.md) 条件逐项关闭。M4W-018 仅关闭测试语义 / 格式基线；M4W-019 原契约关闭保持，其余 17 项仍待验证。不能凭本报告承诺全部上游产品行为已完成验收。
