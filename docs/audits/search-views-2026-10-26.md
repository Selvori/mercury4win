# 全文搜索与智能视图：实现与验证

## 基线和范围

2026-10-26，Asia/Shanghai。HEAD `4b652fbeed4832c256c65f4b24daf846739207e5`，加任务开始前已有的 Feed、Reader、翻译、设置、样式、测试与验收文档修改，再加本功能未提交修改。Windows / PowerShell 7.6.7 / Rust 与 Cargo 1.97.0 GNU / Node 24.12.0 / npm 11.11.0。保留原有工作区，没有提交、推送、发布、升级依赖或修改上游。

本轮按用户明确选择实现新功能；旧 native 验收不是前置条件，既有问题状态和历史证据保留。涉及旧能力的修改仅限搜索需要的查询范围、索引更新和 Reader / 笔记缓存失效。

## 交付行为

- 侧边栏「全文搜索」与智能视图列表；支持全库及当前导航范围，并持续显示实际来源、标签、阅读、收藏、笔记、日期及字段条件。
- 标题、Feed 摘要、本地已获取正文 / Feed 明确正文、已保存笔记的本地词法搜索。空白分词后 AND，可跨字段命中；中文双字、短缩写、英文和代码标识符可用；特殊符号按字面处理。
- 来源、标签 Any / All、已读 / 未读、收藏、非空笔记、绝对 UTC 日期和相对最近 N 天可组合，空关键词可过滤。可限定字段并排除标题包含任一关键词的结果。
- 默认最新在前；相关性按每个词的标题 / 笔记 / 摘要 / 正文命中分别加 12 / 8 / 4 / 2，时间及 ID 稳定破同分。也支持最早在前；时间统一按 SQLite `julianday` 比较，避免时区字符串顺序错误，缺失日期排后。
- 结果提供来源、正文覆盖提示、字段和上下文。返回文本片段与 matched 标志，React 用文本节点和 mark 渲染，不插入来源 HTML。点击笔记命中同时打开正确文章的笔记。
- 智能视图在 SQLite 保存命名条件，支持创建、打开、仅重命名、更新当前条件、另存和删除。相对天数保持规则，完整退出后可重开；未保存修改有提示，删除视图保留文章。
- 批量已读使用完整匹配范围，与查询共享条件编译器；关键词索引刷新和批量更新置于同一写事务。文摘保留已加载文章的手动勾选快照及顺序，条件变化清空选择。普通导航仍保留搜索 / 未读条件与原快捷键行为。
- 中英文、主题变量、原生表单标签、Tab / Escape 焦点恢复、250ms 搜索延迟、无效日期 / 天数提示、查询失败重试和索引重建入口。

## 存储与索引

Schema 由 5 升至 6；新增 `entry_search` FTS5 trigram、`search_dirty` 与 `smart_view`，旧表内容保留。迁移只建表与回填 ID 队列，正文解析和建索引在搜索工作线程执行，每 100 条提交，避免在启动 UI 线程解析全库。源表增删改触发持久更新标记，覆盖正文重建、Feed 更新、笔记及文章隐藏 / 删除；刷新事务同步删除和写入索引。首次搜索加载态明确提示可能在更新索引，不进行网络抓取或 AI 调用。

三字及以上词使用绑定参数的 FTS5 短语查询，短词使用折叠字段的字面匹配；实际 bundled SQLite 的 trigram 对双字「方案」直接 MATCH 返回 0，生产短词路径仍正确返回正文命中。实现参考 [SQLite FTS5 官方说明](https://www.sqlite.org/fts5.html#the_trigram_tokenizer)，测试没有假定 trigram 已解决中文短词。索引和关键词先做 Unicode lowercase；高亮通过字节边界映射保留原文字形。

搜索刷新后固定一个读快照，确认队列未再变化，计数、当前页和片段在同一快照内读取。游标包含排序分数、原始日期、ID 和相对时间锚点；页间使用同一滚动窗口。应用写入会失效查询；活动相对日期视图每分钟刷新。动态数据仍是实时查询，非跨页冻结导出快照。

重建入口可恢复丢失的 FTS 表并重新读取来源；不删除文章、笔记或保存视图。新建库、真实历史版本升级及重复运行均有测试。旧用量升级测试原先仅造了部分 V4 表，本次改用完整历史 V4 fixture，保持旧数据验收断言并使用当前 Schema 常量。

## 实际检查

| 检查 | 结果 |
| --- | --- |
| `npm.cmd run lint` | 通过，0 warning |
| `npm.cmd run build` | 通过；保留已有 >500kB chunk 提示；末次另有 Tailwind 插件耗时提示 |
| `cargo check --locked --offline --manifest-path src-tauri/Cargo.toml` | 通过，0 warning |
| 全仓 `cargo fmt --check` | 通过；仅格式化本次修改文件，无全仓重排 |
| 搜索模块 | 11 项功能回归通过；fixture 入口显式生成通过；性能单独运行通过 |
| 全部 Rust library tests（GNU sidecar runner） | 146 通过、4 失败、1 性能项默认忽略；不是全绿 |
| 新增生产浏览器回归 | 9 组通过，含 16 种语言 / 主题 / 窗口 / 列表宽度组合 |
| 既有 DOM | 分页 6、组合筛选 6、文摘 7、可访问性 10、标签 10、Reader 归属 15，共 54 组通过 |
| 完整 Tauri debug 无 bundle 构建 | 独立 identifier override，通过 |
| Windows / WebView2 native 主要路径 | 实际通过，详见下节 |

全套 Rust 的 4 项失败均为流式请求取消后等待服务端 socket 关闭的超时：

1. `m4w_009_actual_cancellation_drops_request_without_fallback`
2. `m4w_012_cancel_socket_flush_and_waiting_cancellation_not_counted`
3. `m4w_012_translation_parallel_cancel_records_only_started_attempts`
4. `production_batch_requests_progress_no_writes_and_cancel_stops_future_requests`

在当前修改下单独运行仍失败；另将 HEAD `4b652fb` 原样 `git archive` 到忽略目录，以相同工具链构建原基线，四项分别重复同样超时，原基线 harness 为 137 项。本轮没有修改 AI 取消逻辑，也没有将这些失败记录成通过。原基线失败输出摘录见 [证据](search-views-2026-10-26-baseline-failures.txt)。其他未列出的旧 DOM 套件、binary / doc tests 和完整发布构建未重复运行。

新增回归初次发现并修正的本轮问题：外层 UPSERT 的冲突策略会覆盖触发器内的 `OR IGNORE`，导致重复 dirty ID 阻断正文保存；改为 `INSERT SELECT WHERE NOT EXISTS` 并补生产 UPSERT / REPLACE 回归后，正文 / 文摘失败恢复通过。界面回归还修正了下拉控件标签、普通 Reader 打开时过度失效列表影响 j/k 导航、展开筛选挤掉短窗口结果区和暗色文字对比；全部业务断言保留。最终差异复核补齐了智能视图内「当前导航范围」恢复其保存的来源 / 标签 / 收藏，并通过先全库、再恢复视图范围的回归，避免意外扩大批量范围。用于判断文章身份的标题在窄 Reader + 笔记布局中可能被既有工具栏省略，测试同时核对选中文章、笔记面板和正文，未将该旧布局扩展为本任务修复。

## Windows native 证据

使用独立 `com.mercury4win.search20261026`，窗口 `Mercury Search 2026-10-26`。新建合成 V5 库包含 2 源 / 140 篇 / 2 标签 / 1 笔记，启动前放入此前不存在的隔离应用目录。没有打开日常用户库或凭据；文章均为明确的本地 Feed 正文，不依赖外部站点。通过 computer-use 的公开 sky API 操作真实应用；不是浏览器 mock 或直接执行 JS 替代 IPC。

实际观察：

1. 启动生产迁移后 Schema 为 6，140 篇及原标签 / 笔记保留。查询「方案」实际 4 篇，标题、正文、笔记字段高亮与来源可见，恶意 img 标题只显示文本。
2. 点击「查看命中笔记」，打开文章 3「笔记独有测试」，正文和原保存笔记正确。
3. 创建「本地方案视图」，完整关闭、进程退出后重启，侧边栏仍在，打开仍为 4 篇。
4. 最终构建中在活动视图打开笔记，把笔记改成合成 `NativeNoteChanged` 文本并保存；没有重开视图，匹配结果实际由 4 变 3，右侧正在阅读的文章和新笔记保持。
5. 设置最近 7 天、Rust 和未读，实际只剩文章 1 / 2。更新视图名称为「近七天 Rust 未读」及当前条件；重启后名称、关键词、标签、未读及两条结果恢复。数据库保存 `recent_days: 7`，绝对日期均为 null。
6. 点击真实「重建本地搜索索引」，出现「索引已重建」，仍为 2 篇。最终作用域修正构建另做原生复验：视图 2 条→全库 3 条→当前导航恢复 Rust 范围 2 条，未保存修改提示随条件恢复消失。最终只读核对：140 篇、140 条索引、dirty 0、外键错误 0、Provider 0、AI 用量 0。

恢复任务时，旧 JS 状态和窗口均已不存在，先重新枚举并启动同一隔离构建，再继续剩余维护验证。首次窗口几何 / 屏外坐标失败未当成输入成功；刷新截图后使用实际滚动条到达控件，未绕过工具检查。结束时正常关闭隔离应用、确认无 mercury 进程并 reset JS。原始合成 UIA、数据库及构建 override 保留在忽略目录 `tmp/search-native-20261026/`；可长期引用的脱敏摘录与事实是 [UIA](search-views-2026-10-26-ui.txt)、[JSON](search-views-2026-10-26-evidence.json)。

原生覆盖上述主要用户路径；仅重命名 / 另存 / 删除的逐项交互、135 条分页与完整 16 种布局矩阵由 Rust / 浏览器覆盖，未在 native 每种组合再次重复。Linux / macOS、安装升级、真实用户库规模和长时间持续同步未运行。无 AI 的 native 主路径已实际验证，不把本轮边界扩展为旧问题逐项验收。

## 合成性能

显式 debug 测量，内存 SQLite / 10,000 篇 / 每篇约 1.2KB 正文；首次建立索引 **2.130 秒**。查询包含完整匹配计数、相关性排序、首 100 条、来源及上下文片段：

| 关键词 | 匹配数 | 耗时 |
| --- | ---: | ---: |
| 方案 | 588 | 88.15ms |
| rust::read_to_string | 588 | 37.83ms |
| reader | 10,000 | 118.71ms |
| missing | 0 | 0.46ms |

这是当前实现的实测，未与旧标题 LIKE 宣称倍数提升。短词路径需要扫描索引文本，大型磁盘库、十万篇及冷启动磁盘 I/O 未测；首次 2 秒后台索引仍显示加载反馈。

## 文件与复现

| 文件 | 目的 |
| --- | --- |
| `src-tauri/src/db/search_store.rs`、其 `tests.rs` | 队列 / 索引、文本处理、条件、片段、排序分页、重建与回归 |
| `src-tauri/src/db/smart_view_store.rs` | 命名视图持久化及 CRUD |
| `db/migrations.rs`、`models.rs`、`entry_store.rs` | Schema 6、DTO、统一查询与事务内批量范围 |
| `commands/search_cmd.rs`、`entry_cmd.rs`、`lib.rs`、模块注册 | IPC 命令及兼容旧参数 |
| `src/lib/tauri_bindings.ts`、`src/types/entry.ts`、类型导出 | 同步结构化条件、分页、片段和视图契约；不把 cursor / limit 混入筛选 |
| `search_controls.tsx`、`smart_views.tsx`、`entry_list.tsx`、`search_filter.ts` | 搜索、筛选、命中、保存 / 管理入口与验证 |
| `app_shell.tsx`、`sidebar.tsx` | 导航、视图、笔记跳转、状态与主题继承 |
| `reader_note.tsx`、`reader_pane.tsx` | 保存笔记 / 正文后刷新搜索结果，保留已有内容归属与阅读行为 |
| 双语 JSON | 64 个对应文案键 |
| `tests/search_views/`、受影响旧脚手架及历史 fixture | Rust DTO 驱动的生产界面回归与复现入口 |

命令、fixture 生成和复验步骤统一维护在 [tests/search_views](../../tests/search_views/README.md)。本次没有修改应用依赖 / 锁文件、固定 OPML 或 bootstrap；没有打发布标签。

## 搜索开关与设置界面后续调整

2026-10-26，用户要求搜索入口具备可取消的选中背景，并统一智能视图设置排版。此调整仍位于同一未提交工作区，保留上述功能与历史结果。

- `app_shell.tsx` / `sidebar.tsx`：搜索有独立状态及语义化选中 / 展开属性，使用现有accent选中色；再次点击恢复中间空状态，关闭当前文章选择。来源、标签、收藏与保存视图导航清除搜索选中，列表来源不与搜索重复高亮。
- `search_controls.tsx`：补公共DialogContent的24px留白，使用公共标题 / 页脚；名称字段、辅助操作卡片、独立删除区和主操作有清楚间距及优先级。下拉框与Input统一h-9、rounded-lg、border-border、bg-card和焦点环；辅助说明新增双语2键，当前资源425 / 425键。
- 本次实际通过：lint无warning、build（仍有原chunk提示）、11组生产浏览器回归（原9组保留，新增开关和弹窗布局，含4种弹窗语言 / 主题及原16种列表布局）；既有组合筛选6组、可访问性10组；独立完整Tauri debug构建。浏览器输出与截图在忽略目录外的临时 `mercury4win-search-v3BECh`，中英亮暗截图已实际查看。
- Windows native复用同一隔离identifier与合成库，实际查看搜索选中背景、再次点击后中间搜索控件消失 / 空状态恢复，打开保存视图及新版弹窗，Escape实际关闭。UIA文字摘录见[本次记录](search-ui-2026-10-26.txt)。未修改任何笔记 / 视图内容，结束正常退出，确认无mercury进程并reset JS。

本次未改Rust / 迁移 / IPC业务逻辑，未重跑Rust；native仅简体中文亮色实操，完整语言 / 主题矩阵由浏览器覆盖，其他平台未重复。不是以浏览器结果替代native。

## 普通浏览与全文搜索分离

2026-10-26，基线仍为 `4b652fb` 加全部保留的未提交改动，Windows / PowerShell 7.6.7。本次按用户反馈，直接选择订阅源不再展示完整搜索区。

- `app_shell.tsx` 显式传入搜索模式；`entry_list.tsx` 默认普通浏览，仅显示文章列表、未读 / 标签模式及常用文章操作。完整关键词 / 高级筛选只在主动进入全文搜索时显示；普通浏览的查询只保留当前导航范围及可见的未读 / 标签模式，不受隐藏关键词（含 debounce）、日期、笔记状态、搜索字段或排序影响。
- 智能视图默认显示名称和编辑 / 管理按钮的紧凑栏；点击「编辑筛选」主动展开完整表单，保存规则仍保留。新增中英 `search.editFilters`，资源426 / 426键一致。保持搜索选中 / 再次关闭及统一弹窗布局。
- 实际通过 lint / build（已有大chunk提示）、11组真实Edge生产组件浏览器回归，以及既有筛选6 / 分页6 / 文摘7 / 可访问性10，共29组DOM。扩展断言：普通订阅源 / 标签没有搜索输入；有关键词的搜索转订阅源不泄漏条件；包含标签、日期、阅读 / 笔记状态、字段等组合的智能视图转另一订阅源，IPC只使用可见导航条件；视图主动编辑 / CRUD仍通过。浏览器结果 / 截图在临时 `mercury4win-search-zHRuRd`，`feed-browse.png` 已实际查看。
- 旧筛选脚手架先因仍要求普通列表中的搜索框而需要适配；之后晚阶段「删除后立即重新读取标签选项」断言失败，原因是普通浏览已不挂载搜索表单，查询处于非活动状态。改为核对真实缓存存在且已失效，重新运行6组通过；没有放宽产品查询或删除行为。
- `npm.cmd run tauri -- build --debug --no-bundle --config tmp/search-native-20261026/tauri.search.json -- --locked --offline` 通过。复用同一隔离identifier与合成库，真实Windows / WebView2依次观察：直接选择A为137篇且无搜索表单；主动全文搜索展示完整控件，输入「不存在词XYZ」为0篇；选择B后无搜索表单且完整显示3篇；打开「近七天 Rust 未读」为2篇的紧凑视图，点击「编辑筛选」恢复关键词 / 标签 / 近7天及完整表单。未保存或删除内容，正常退出，确认无mercury进程并reset JS。[合成UIA摘录](search-browse-ui-2026-10-26.txt)。

无Rust / Schema / IPC协议修改，本次未重跑Rust。native本轮为简体中文亮色，浏览器仍覆盖中英亮暗矩阵；其他平台及日常用户数据库未验证或访问。未提交 / 推送 / 发布。
