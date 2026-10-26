# 全文搜索与智能视图

入口：侧边栏「全文搜索」。点击后显示选中背景，再次点击取消选中并关闭中间搜索视图；切换其他导航也会取消搜索选中。关键词以空白分隔，所有词都需命中，可跨字段匹配；符号按字面处理。默认保持时间排序，可切换相关性或最早在前。「全库」清除导航的来源 / 标签 / 收藏范围；「当前导航范围」恢复左侧导航范围；在智能视图内恢复该视图已保存的来源 / 标签 / 收藏范围，其他附加筛选仍保留。当前实际条件始终显示在搜索框下方。

「筛选」支持来源、标签 Any / All、阅读状态、收藏、非空笔记、最近 N 天和 UTC 日期区间；空关键词可独立过滤。搜索字段可限定标题、Feed 摘要、正文、笔记，并可排除标题中包含任一关键词的文章。正文范围包含本地已下载正文及 Feed 明确提供的正文；没有正文时仅查其他已存字段，不自动抓取、配置或调用 AI。

书签按钮用于创建、重命名、更新、另存和删除视图。仅重命名保留已保存条件；更新使用当前条件；未保存修改有提示。视图存入 SQLite；最近 N 天保存相对规则，分页中使用统一时间锚点，活动视图每分钟重查，应用内写入后失效重查。删除视图不删除文章。

批量已读作用于全部匹配文章，包括未加载页；文摘只处理进入选择模式时已加载并手动勾选的文章，保持列表顺序。搜索条件改变会结束选择模式。结果点击打开文章，笔记命中按钮同时打开对应笔记。

## Rust 与真实 DTO

从仓库根运行，Windows GNU 环境沿用现有 Common Controls 6 runner：

```powershell
./tests/reader_pipeline/run_rust.ps1 db::search_store::tests
cargo check --locked --offline --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

模块含 11 项功能回归、一个显式 fixture 生成入口和一个默认忽略的性能测量。覆盖中文双字、英文 / 缩写 / 代码标识符、字面符号、正文 / 笔记独有命中、Unicode 片段、源更新 / UPSERT / 删除 / 隐藏 / 重建、组合筛选、相关性及时间游标、时区与 NULL 日期、相对边界、真实历史库迁移与重复运行、视图 CRUD / 新连接池重开、无 Provider / 用量依赖。FTS5 trigram 的真实双字 MATCH 返回 0，生产短词路径返回正确结果。

按以下方式生成全新合成 V5 库和生产查询 DTO，不接触安装应用的数据。目标文件存在时会拒绝覆盖。

```powershell
$searchFixtureDir = Join-Path (Get-Location) ('tmp/search-fixtures-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $searchFixtureDir | Out-Null
$env:M4W_SEARCH_NATIVE_DB = Join-Path $searchFixtureDir 'seed-v5.db'
$env:M4W_SEARCH_FIXTURE = Join-Path $searchFixtureDir 'pages.json'
try {
    ./tests/reader_pipeline/run_rust.ps1 export_search_ui_fixtures
} finally {
    Remove-Item Env:M4W_SEARCH_NATIVE_DB, Env:M4W_SEARCH_FIXTURE -ErrorAction SilentlyContinue
}
```

生成库含 2 个来源、140 篇合成文章、标签、笔记及一个应按文本显示的恶意标题。生产 SQL 生成的第一页 / 第二页和命中片段用于界面契约测试，浏览器 mock 不实现另一套搜索算法。

## 浏览器与已有行为

使用独立安装的 `playwright-core`、实际 Chromium 浏览器路径和上一步生成的 `pages.json`，无需修改应用依赖：

```powershell
npm.cmd run lint
npm.cmd run build
node tests/search_views/run_browser.mjs <playwright-core/index.mjs> <pages.json> <browser.exe>
```

9 组生产界面回归覆盖安全高亮 / 命中来源、正确笔记跳转、全部条件及批量 IPC 一致性、视图 CRUD / 页面重新加载、相对规则、Escape 焦点恢复、135 条真实 DTO 分页、筛选校验 / 重建 / 查询错误重试。布局包含中英 × 亮暗 × 1200 / 800 窗口 × 320 / 200 列表面板，共 16 种组合；展开筛选后仍保留可见结果区。浏览器只替换 native transport，所有非 loopback 网络请求被拦截。

既有回归入口包括 `entry_pagination`、`entry_filters`、`digest_export`、`accessibility`、`tag_system` 和 `reader_ownership` 的 `run_dom.mjs`。`entry_filters` 仍使用真实 Rust `EntryDetail` fixture，设置 `M4W_DETAIL_WIRE_PATH` 后运行 `m4w_011_tests` 生成。由于搜索现在有 250ms 延迟，搜索断言等待该延迟；IPC 断言按命令选择记录，避免把筛选选项读取误当分页请求。

这些是浏览器 / DOM 契约验证，不代替 Tauri IPC 和 WebView2；实际结果、native 证据及全套 Rust 的既有失败见 [专项报告](../../docs/audits/search-views-2026-10-26.md)。

## 桌面复验

使用独立 Tauri identifier、新目录及上述 V5 合成库；应用未运行时才放入隔离目录。以实际 Tauri Path API 对应目录为准，不覆盖日常用户库。启动后应自动升级至 V6。

1. 查询「方案」应命中标题、正文、笔记，共 4 篇；恶意标题只显示文字。
2. 点击「查看命中笔记」打开文章 3；修改笔记，去掉「方案」，保存后活动查询应从 4 条自动变 3 条。
3. 加上最近 7 天、Rust、未读，得到文章 1 / 2；更新视图名称和条件，完整退出、确认进程退出、重启，视图和相对规则仍在。
4. 点击「重建本地搜索索引」，成功后结果及已保存视图保持。文章、笔记及用量数据不因重建删除。
5. 空关键词组合收藏 / 有笔记；Any / All 与日期边界；相关性 / 时间排序跨页；仅重命名、另存、删除后文章保留；窄窗口、暗色、Tab / Escape 和既有文摘 / 标记操作按对应范围工作。

最后核对隔离库 Provider / AI 用量为 0，索引行数与可见文章一致，待更新队列排空，外键检查为 0。不要把尚未执行的复验步骤写成通过。

### 后续界面验证

2026-10-26 搜索开关与设置样式调整后，runner为11组：增加搜索按钮背景 / Space / 点击关闭 / 来源导航清除、以及中英亮暗四种智能视图弹窗的标准留白 / 分组 / 页脚 / 无溢出检查。原9组断言仍保留；弹窗截图由runner写入其返回的临时目录。实际结果与native边界见专项报告末节。

### 普通浏览与搜索模式复验

2026-10-26，普通订阅源 / 标签 / 收藏导航默认只显示紧凑文章列表；从侧边栏「全文搜索」才进入完整表单。智能视图默认名称栏，可通过「编辑筛选」主动展开，书签管理入口保留。

同一11组runner新增断言：普通浏览不含关键词 / 智能视图管理控件；关键词搜索切到源不残留查询；组合智能视图转另一源不携带隐藏标签、日期、笔记 / 阅读状态和搜索字段；再次打开视图后条件完整，编辑 / CRUD正常。既有entry_filters脚手架先主动进入搜索再测试关键词，并核对普通浏览期间非活动标签选项缓存失效，而非立即读取。

桌面最短复验：点源A，检查顶部仅常用操作；进入全文搜索并输入无匹配词，再点源B，检查源B文章完整且无搜索表单；点保存视图确认紧凑展示，再点「编辑筛选」确认完整条件恢复。实际Windows结果及边界见[专项记录](../../docs/audits/search-views-2026-10-26.md#普通浏览与全文搜索分离)。
