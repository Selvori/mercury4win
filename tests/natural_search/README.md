# 自然语言筛选

本文保留 2026-10-27 的历史语境。2026-10-28 当前 UI 已由 [AI 主题搜索](../topic_search/README.md) 替代，新的内容传输、语义判定、范围令牌及复验入口以该文档为准。旧解析 IPC 保留兼容，历史失败不改写。

入口：侧边栏「全文搜索」→「自然语言筛选」。例如输入「找最近一周我收藏的 Rust 文章」，明确提交后调用现有摘要模型，成功时替换**全部**当前条件并查询本地库，自动展开既有筛选表单。关键词、来源、标签、日期等可继续编辑或移除，书签按钮保存的是同一个规范化 `EntryFilter`。普通浏览保持紧凑，普通关键词、同步及保存视图不触发解析请求。

最近 N 天是滚动的 N × 24 小时，保存后仍动态计算；时间字段指发布时间，缺失时沿用创建时间。固定日期沿用现有 UTC 日期控件，不能精确表示的本地日历时段应报告不支持。「上周读过」属于未记录的阅读历史，不能等同于当前已读＋最近发布时间。检索仍为本地词法匹配，不增加语义召回；多来源 OR、关键词 OR、排除标签、未收藏、精确短语等当前契约无法表达的条件应返回解释且不应用。未提供旧筛选或导航范围给模型，因此「当前来源」应显式写出来源名或 URL。

复用设置中的服务商、摘要模型主备路由、凭据和 Provider HTTP，任务类型为独立的 `search_filter`，用量沿用请求事实记录。只发送这句话、来源名称 / Feed URL、标签名称和本机当前时间 / 时区上下文；不发送数据库 ID、文章、笔记或旧筛选。后端拒绝未知 / 重复字段、错误类型 / 枚举、无效日期 / 范围和 ID 字段，按本地元数据解析名称，结果交给原有参数化查询，不执行模型 SQL。同名来源需用户句子明确指定 Feed URL，不能由模型猜选 URL；未知 / 歧义标签提示手动调整。返回前重新检查所选元数据，防止解析期间删除后静默扩大范围。

无 AI 配置、模型拒绝、非法返回、网络错误和不支持条件都保留当前有效查询；没有命中文章是正常空结果。取消、连续提交、编辑句子 / 手动条件、关闭面板及卸载都会使旧响应失效，并向后端取消对应请求。没有二次审批或后台逐字调用。

## 受控测试

Windows / PowerShell，从仓库根运行；使用已有依赖，不安装新包，不读用户数据或真实凭据，不请求真实模型：

```powershell
./tests/reader_pipeline/run_rust.ps1 agent::search_filter
./tests/reader_pipeline/run_rust.ps1 db::search_store::tests
./tests/reader_pipeline/run_rust.ps1 agent::runtime::tests
node tests/natural_search/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
npm.cmd run lint
npm.cmd run build
cargo check --locked --offline --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
git diff --check
```

`run_dom.mjs` 接受已单独安装的 Happy DOM 模块绝对路径；用 Node 中的 DOM 模拟运行生产组件与实际 IPC wrapper，仅替换 native transport，不启动或操作浏览器 / 桌面。

2026-10-27，基线 `81f7b2e` 加本功能未提交修改；开始前仅有 WORKLOG 建议记录的未提交修改，已保留。Windows / PowerShell 7.6.7 / Node 24.12.0：

- 新增 Rust 7 项通过：中英句子经真实 loopback HTTP → 严格解析 → 生产 SQLite 查询 → 保存 / 读取相同条件；UTC 与 +08:00 跨日上下文、相对规则滚动、空结果、来源 / 标签未知及歧义、禁止模型猜选同名来源、非法结构 / ID / 字段 / 枚举 / 类型 / 日期、拒绝 / HTTP 失败 / 连接失败、无配置仍可普通搜索、请求用量记录、取消实际等待中的 HTTP 与后续请求，以及注册前取消。
- 原搜索模块 12 通过 / 1 忽略（其中可选 fixture 入口未设置输出变量，没有导出文件）；原任务运行时 8 通过。忽略项是原有显式性能测量，本轮未测性能。
- 新增生产组件 DOM 5 组通过：不自动请求、成功全量替换及手动控件、保存规范化条件、失败 / 无配置、取消 / 连续请求 / 晚到结果、新请求加载状态、手动或句子编辑 / 卸载保护、中英文标签与错误反馈。
- lint / build、Rust check / 全仓格式、双语键和 UTF-8 / diff 检查通过；build 保留已有 >500 kB chunk 提示。最初新增 TaskType 漏补原测试的穷尽 match，编译失败；补齐分支后通过，没有计作通过。

本轮未使用 Computer Use，也没有启动、操作或截图任何浏览器 / 桌面。Tauri IPC、原生保存 / 重启、实际亮暗 / 窄窗口排版及键盘焦点路径未验证；DOM 只验证组件行为及标签。受控模型返回用于验证契约和失败保护，不证明真实模型能准确理解所有句子；真实模型质量、其他平台和大型来源 / 标签库未验证。未重跑完整 Rust library 或此前所有组件套件，旧问题状态保持。

## 原生验收计划（首次实现时未执行）

需另行取得用户对 Computer Use 的明确授权。使用隔离库及受控 Provider，检查：普通搜索无需模型；提交自然语言后替换所有旧筛选；修改 / 移除和保存 / 重启恢复；动态最近天数；重名来源提示具体 URL；不支持的阅读历史；失败 / 空结果区别；取消后晚到结果、连续提交和离开搜索；中英亮暗及窄面板的 Tab / Enter / Escape 与滚动可达性。真实模型质量另需明确额度授权。

## 真实 API 验证（默认不运行）

用户于 2026-10-27 明确批准真实 API 后，实际使用现有主模型 ecnu-plus、合成元数据及内存文章库共 10 次请求 / 7,944 token。中文 / 英文组合、本地查询 / 保存条件一致和阅读历史拒绝通过；首轮存在未知 / 重名反馈错误码预期过严及一次 invalid，后续明确 Prompt 中应用负责名称解析的输出示例，异常三项复测通过。完整失败、复测、请求计数和范围见[真实验证报告](../../docs/audits/natural-search-live-2026-10-27.md)。前文首次实现的未验证结论保留其当时语境，不代表本次新增结果。

`approved_live_natural_search` 使用 `#[ignore]`，普通测试不读取真实配置、不请求 API。仅在用户明确批准的额度范围内手工运行：

```powershell
# 显式传入已核实的应用数据目录；只读 AI 配置表与选中主模型的 key，不读取用户文章。
./tests/natural_search/run_live.ps1 -AppDataDirectory '<已核实的应用数据目录>' -ReportPath '<新报告绝对路径.json>'
# 可选：-Case basic_zh / combined_en / reading_history / unknown_source / unknown_tag / ambiguous_source
# 或 -Case metadata_errors，仅运行三个名称异常场景。
```

每次最多 6 个主模型请求，本测试不使用 fallback / 自动重试；报告拒绝覆盖，进程环境在结束后恢复。报告仅包含合成句子 / 元数据、模型返回、条件、合成命中 ID、模型名及实际用量，不保存密钥 / 真实 URL / 用户正文。样本不能替代稳定性或原生验收，Computer Use 限制继续保持。

## 2026-10-27 原生验收与条件身份修正

用户随后明确授权 Computer Use，已实际完成 Windows / WebView2 隔离验收；上文未执行与暂停限制均保留当时语境。本次完整结果、初始失败与边界见[原生报告](../../docs/audits/natural-search-native-2026-10-27.md)，对应 M4W-028 已关闭。

原生发现父组件合并默认值后对象字段顺序不同，成功说明缺失。DOM 脚手架先改成真实合并方式，旧实现明确失败；search_filter_identity 固定字段次序、统一默认值 / 已读语义及标签集合后，修正后的 DOM 5 组通过，补充等价条件重排不取消进行中任务。原生重新构建后，中英文成功提示、编辑 / 移除、保存 / 更新 / 重启和晚到保护实际通过。lint / build、626 对资源、隔离及恢复默认 identifier 的完整 Tauri debug 构建通过。

实际覆盖普通关键词无请求、Enter 提交 / Tab→Space、解析成功替换全条件、组合字段、视图保存 / 更新及重启 7 天相对规则、重名 URL / 不支持阅读历史、非法模型 / HTTP 错误保留条件、30 秒延迟取消 / 连续提交 / 卸载、空结果、无配置普通搜索、中英亮暗及约 800×600 窗口 / 200px 列表可滚动访问。未进行跨平台 / 安装升级 / 大库 / 真实网络取消等完整矩阵。

复验资源：`seed_native.py` 从既有合成 V5 fixture 创建不存在的隔离库，应用实际迁移；`native_provider.py <new evidence directory> <port>` 提供仅 loopback 的成功 / 错误 / 延迟响应，默认 fixture 配置端口 18761。二者不读取或转发真实库 / 凭据，不能作为模型质量证明。本轮 12 请求全部为本机受控调用，未新增真实模型消耗。隔离窗口与 Provider 已退出，18761 无监听，默认应用构建身份已恢复；没有提交 / 推送 / 发布。
