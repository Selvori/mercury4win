# M4W-011 查询、动作与 M4W-019 详情契约回归

2026-10-12，HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加保留的 M4W-001、016、005、008、006 和本项未提交修改；Windows / PowerShell 7.6.5、Rust 1.97.0 GNU、Node v24.12.0、临时 Happy DOM 20.14.6。只使用内存 SQLite 和内存 native IPC transport，没有接触用户数据库、凭据或远程服务。

## 实现范围

`EntryFilter` 统一 Feed、搜索标题 / 摘要、仅未读、仅收藏及标签 Any / All 条件；SQL 列表、总数和批量已读共用同一构造器。批量更新不接收游标 / limit，适用于整个匹配范围。空白搜索等同无搜索，标签去重；非法 tag mode 由 serde 拒绝。保留 M4W-006 的日期 / ID 复合游标。前端共享同一参数映射，完整条件作为 Query key；离开范围即清理分页缓存，切换或返回筛选从第一页开始。

界面有搜索、仅未读开关、收藏入口、多标签选择和 Any / All 切换。最多 5 个标签的约定来自上游 `docs/features/tags-v2.md` §4.2，界面显示上限，第 6 个未选项标为 aria-disabled 并拒绝选择。仅调整查询选择，没有实现 M4W-010 标签合并 / 别名等维护功能。

列表快捷键为 m 已读 / 未读反转、s 收藏反转、j / k 下一篇 / 上一篇（j 自动加载后续页）、Shift+A 当前完整范围已读、Shift+R 刷新当前订阅或全局订阅。Reader 的 r 切换摘要 / Reader、t 翻译 / Reader、n 笔记 / Reader；原 Ctrl / Cmd+N 新增订阅保留。输入、textarea、select、contenteditable 后代、textbox、模态 dialog、IME 组合输入、重复事件和无关修饰键不触发单键快捷操作。iframe 只读文章键盘事件转发到父文档；此分支未做真实 WebView2 验收。延迟 j 同时检查筛选、导航代次和文章选择，不能抢回更新后的选择。

删除文章的软删除与关联标签 usage_count 重算放在事务内；前端失效 entries / feeds / tags，移除详情 / Reader 缓存，只在被删文章仍是当前选择时清除 Reader。已读按钮可反转，文章动作均有标签和失败提示。

详情 TS 模型改为 `entry: Entry`，保持 Rust 的真实嵌套结构；不增加扁平兼容数据。使用 OptionalExtension 判断不存在或软删除文章，命令继续返回 `Entry not found`。生产唯一 TS 消费者 ReaderTagging 使用顶层 tags；归属测试 mock 同步完整嵌套契约。

## 执行与实测

从仓库根目录运行，Happy DOM 临时安装见 [Reader 归属脚手架](../reader_ownership/README.md)。先由生产 Rust 序列化临时详情 fixture，再交给生产 TS wrapper / ReaderTagging；不是手写详情响应。

```powershell
$env:M4W_DETAIL_WIRE_PATH = Join-Path $env:TEMP 'm4w-019-detail-wire.json'
pwsh -NoProfile -File tests/reader_pipeline/run_rust.ps1 -Filter m4w_011_tests
node tests/entry_filters/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:M4W_DETAIL_WIRE_PATH"
Remove-Item Env:M4W_DETAIL_WIRE_PATH
pwsh -NoProfile -File tests/reader_pipeline/run_rust.ps1 -Filter m4w_006_tests
node tests/entry_pagination/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
npm.cmd run lint
npm.cmd run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
rustfmt --edition 2021 --config skip_children=true --check src-tauri/src/db/entry_store.rs src-tauri/src/commands/entry_cmd.rs src-tauri/src/db/entry_store/m4w_011_tests.rs
git diff --check
```

- 本项 Rust 4 项通过：237 条文章以 7 条分页，Any / All + Feed / unread / starred / search 对照独立 SQL；空白 / 无匹配 / 重复标签；批量已读超过首个分页范围且不触碰其他文章，已读反转；删除后的可见标签 / Feed 计数；真实嵌套详情 JSON、null 字段、Feed 原文、标签、笔记及缺失文章错误。
- 生产 AppShell 隔离 DOM 6 组通过：搜索 / 未读 / 收藏及分页重置；标签模式、上限、批量范围 IPC 完全一致；实际 m / s / j / k / 刷新 / r / t / n 和输入 / IME / 修饰键 / 设置弹窗保护；j / k 跨页与旧导航不抢选择；删除清 Reader / 刷 feeds / 失效 tags；实际 Rust JSON 经生产 wrapper、ReaderTagging 展示与缺失错误。
- 原 M4W-006 Rust 4 项、生产 EntryList DOM 6 组及 M4W-001 DOM 15 项复测通过。旧 006 测试的缓存清理 predicate 随完整过滤 Query key 同步，没有删除断言。
- lint / build、cargo check（13 条既有警告）、上述定向 Rust 格式及 diff 检查通过。没有执行全套 Rust 或全仓格式。

首次 build 因刷新单个 / 全部 Feed 返回值的推断差异失败，改为 async void 动作后通过。首次新增 Rust 测试编译因 fixture 使用错误 feed store 函数名失败，修正后首次运行仍因 note fixture 缺必填 updated_at 失败，补齐真实 schema 要求后 4 项通过；这些失败均未算作通过。旧 006 DOM 首次使用旧 Query key 清理缓存后找不到 loadMore，更新测试 predicate 后原 6 组通过。

AppShell DOM 仍输出已有 FeedList 的 button 嵌套 button React 警告；本项没有改写该订阅行，此警告和真实浏览器焦点行为留待界面验收。未验证 native Tauri IPC / WebView2 的默认键盘操作、iframe 内快捷键、布局 / 滚动、真实订阅同步、重启和三平台。M4W-011 保持待验证；M4W-019 的模型 / 序列化 / 唯一消费者 / 缺失路径契约验收已通过，关闭仅限该契约缺陷。
