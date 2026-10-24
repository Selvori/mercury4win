# M4W-005 Feed 正文来源与回退回归

2026-10-12，HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加保留的 M4W-001、M4W-016 与本项未提交实现。Windows / PowerShell 7.6.5，Rust 1.97.0（x86_64-pc-windows-gnu），Node v24.12.0，Happy DOM 20.14.6。

## 方案与兼容边界

`entry.summary` 只保留 Feed 导读；新增 `feed_content` / `feed_content_type` 保存显式 RSS `content:encoded`、Atom `content`、JSON `content_html` / `content_text`。只有显式正文可直接阅读，正文长短不参与判断。Feed 仅有 description / summary 时，即使文字很长，也请求文章 URL。

数据库 v2 保留旧合并 summary；旧内容来源无法可靠还原，不能猜成全文。只清理 `content.html = entry.summary` 且非空的派生正文与对应 HTML 缓存，保留已下载的网页正文及订阅、已读、收藏、笔记、标签、摘要、翻译历史。旧合并文本在离线时仍可见，并标注可能不完整；下一次 Feed 同步重新确定来源。新库、v1 升级和重复升级有回归。

成功正文会写入原始 HTML、Markdown 和阅读缓存。HTTP / 空响应 / 挑战页 / 提取失败、无 URL 或离线时，摘要仅作临时展示：去标签、解码、转义为代码块，再经过现有转换与安全渲染。输出 HTML 含固定 `mercury-content-source=summary` meta 标记，生产 ReaderPane 显示双语来源提示和重试按钮。降级页不写 content / HTML 缓存，恢复后可重试。没有可用摘要时返回错误。IPC 仍返回 HTML 字符串，签名不变。

显式正文或文章 URL 改变时，只失效对应派生正文与阅读缓存；重复同步且来源不变保留缓存。`document_base_url` 已保留文章 URL 供 M4W-008 后续处理；本项没有完成最终重定向 URL / HTML base、相对 URL 重写、分层版本重建或一般错误展示的验收。`READER_RENDER_VERSION` 从 2 升到 3 防止返回旧降级缓存。

## 执行与实际结果

从仓库根目录执行：

```powershell
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib m4w_005_tests
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib db::migrations::tests
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm.cmd run lint
npm.cmd run build
rustfmt --edition 2021 --check src-tauri/src/db/entry_store.rs src-tauri/src/db/migrations.rs src-tauri/src/db/models.rs src-tauri/src/feed/parser.rs src-tauri/src/reader/pipeline.rs src-tauri/src/feed/m4w_005_tests.rs
rustfmt --edition 2021 --config skip_children=true --check src-tauri/src/feed/mod.rs
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" ../feed_content/browser.tsx
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
git diff --check
```

Happy DOM 路径是前两项修复建立的临时依赖；首次运行安装方法见 [Reader 归属回归](../reader_ownership/README.md)。本项没有新增项目依赖或变更锁文件。

| 检查 | 实际结果 |
| --- | --- |
| Feed / Reader Rust 回归 | 9 通过、0 失败 |
| 数据库迁移回归 | 4 通过、0 失败 |
| cargo check | 通过，15 条既有警告 |
| npm lint / build | 通过 |
| 本项 Rust 文件格式 / diff | 通过 |
| 生产 ReaderPane 的来源提示、重试和文章切换 | 2 项隔离 DOM 回归通过 |
| 原 M4W-001 归属回归 | 原 15 项复测通过 |
| 仓库 `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 失败，仍有未修改文件的历史格式差异，未进行全仓重排 |

Rust 测试是仓库持久 `#[cfg(test)]` 模块 `src-tauri/src/feed/m4w_005_tests.rs`：单连接内存 SQLite、127.0.0.1 随机端口 HTTP，不使用真实数据库、凭据或远程服务。逐项覆盖：

1. RSS / Atom / JSON 明确正文与导读分开解析。
2. teaser 访问本地长文，生成的 HTML / Markdown 含完整文章标记；后续无网络请求也能读持久正文。
3. 短 Feed 正文离线可读；重复同步保留正文，显式正文修改失效实际已有缓存并保留已读 / 收藏。
4. HTTP 503 后显示可见中文导读、结构化标记，危险属性不进入输出，不存成功内容 / 缓存；恢复 HTTP 200 后读取全文。
5. 离线连接失败及缺少 URL 均保留旧文本可见，并不声称全文。
6. JSON content_text 的 `<b>` 等字符保持字面量，空摘要与抓取失败返回错误。
7. 空响应、挑战页及不可提取 HTML 不污染成功正文缓存。
8. RSS、Atom HTML / XHTML、JSON HTML 的短正文直接读取，文章 URL 不可达仍成功。
9. 固定 F-001 的实际 OPML 解析器、隔离存储去重与导出往返（范围见下节）。

迁移用 v1 内存库检查两种旧合并内容（短导读及完整旧 Feed body）原始 summary 保留，错误派生内容清理，已下载网页 Markdown 保留，已读 / 收藏 / 笔记 / 标签及 AI 摘要 / 翻译历史保留，重复执行和外键检查通过。

首次独立 integration-test exe 与历史审查一样报 `STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)`，未能进入测试；改为 lib 单元测试模块后正常执行。初次回归还发现 Markdown 中的下划线转义需要语义断言，以及 htmd 对只有 `<pre>` 无 `<code>` 的文本会降为普通 Markdown，导致 HTML 字面量丢失；已修复为 `<pre><code>` 后通过。以上初始失败未计作通过。

## F-001 实测边界

使用仓库 [hn-popular10+1.opml](../fixtures/opml/hn-popular10+1.opml)。生产解析器得到 11 唯一 HTTPS URL、10 个 site URL，Paradigm X 缺省值正常。

生产 `insert_feed` 在空内存库预置 3 个源、导入全部再新增 8 个、重复导入新增 0 个；预置已读 / 收藏 / 笔记 / 标签保持。生产导出器写临时 UTF-8 OPML，再由生产解析器读取，11 个 URL 集合相等并能写入另一空内存库。导出临时文件已删除，内存库随测试结束释放，未改固定样例或 bootstrap。

这验证实际解析、存储和导出层；**没有调用完整 Tauri import_opml_stream、没有验证逐源进度 Channel、没有执行远程 11 源同步或文件对话框**。在线同步失败恢复使用本地文章响应，不将远程源可用性假定为通过。完整桌面 / 重启、Windows WebView2 排版、真实 AI 输入与其他平台尚未验证；M4W-005 保持“待验证”。
