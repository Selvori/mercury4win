# 工作日志

按实质任务追加，使用日期和主题区分同一天的任务。只记录实际动作与证据；当前问题状态统一查 ISSUES，当前概况查 STATUS。

## 2026-10-12 · 更新参考仓库与重要功能审查

- 目的：更新 Mercury，检查重要功能移植是否正确。
- 基线：当前项目 HEAD 395f997，包含开始时已有的未提交改动；参考项目从 3cbcbc1 快进到 5b2e58e / v1.4.32。
- 产出：`docs/audits/mercury-porting-2026-10-12.md` 和隔离探针 `porting_audit_probe.rs`。业务实现未修复，临时测试模块接入已移除。
- 验证：前端 lint / build 通过，cargo check 通过并有 17 条警告；原有测试 32 通过 / 4 失败，隔离探针 3 通过 / 10 失败；全仓格式检查失败，探针格式通过。
- 限制：未做完整桌面、真实 AI、三平台验收；探针使用内存 SQLite / 本地 HTTP，无真实用户数据。
- 跟进：问题现登记为 M4W-001…M4W-018，详情与复现证据保留在审查报告。

## 2026-10-12 · 登记 RSS 导入样例与建立项目记录机制

- 目的：保存用户指定 hn-popular10+1.opml 为后续导入样例，建立持续的信息记录机制。
- 变更：原样复制到 tests/fixtures/opml，以 .gitattributes 保持样例字节；建立 docs 导航、STATUS / ISSUES / DECISIONS / TESTING / WORKLOG；将维护规则接入 AGENTS.md 和根 README。
- 决策：D-001 仓库记录分工、D-002 固定 OPML 基线；D-003 登记既有兼容约定。登记审查问题，不将其标记为已修复。
- 验证：原文件与副本内容相同；严格 UTF-8、XML 结构、11 个唯一 HTTPS xmlUrl、10 个 htmlUrl / 1 个缺省值通过；新增文档链接与 Git diff 检查通过。
- 未运行：实际 RSS 导入 / 网络同步 / Tauri 对话框，未重复执行前后端构建（本次仅文档和固定样例变更）。
- 遗留：下次涉及 Feed / OPML 任务按 TESTING.md 执行隔离验收并记录逐源结果；业务缺陷仍按 ISSUES 处理。

## 2026-10-12 · 整理并提交修复前基线

- 用户要求先处理未提交改动，因此暂停 M4W-001 子任务；业务修复尚未写入代码。
- 基线：395f997 加既有项目名称统一、固定历史凭据混淆字节与兼容测试、审查及记录文件。本轮新增串行修复顺序文档。
- 将源代码/包名称与文档/审查/固定样例分开提交，保留全部已有修改；仅本地提交，不推送、不发布。
- Windows PowerShell 7.6.5 实测：npm.cmd run lint、npm.cmd run build、cargo check --locked --manifest-path src-tauri/Cargo.toml 通过；Rust 保留 17 条已有警告。
- cargo test --locked --manifest-path src-tauri/Cargo.toml credential_store：2 通过、0 失败；rustfmt --edition 2021 --check src-tauri/src/agent/credential_store.rs 与 git diff --check 通过。
- 未重跑全套 Rust 测试及桌面/跨平台验收；此前报告中的失败仍有效，不将本次提交作为业务缺陷已修复的证据。

## 2026-10-12 · M4W-001 文章归属隔离

- 基线：HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de`，恢复时工作区干净；本项业务实现、回归及记录为未提交改动。Windows、PowerShell 7.6.5、Node v24.12.0。
- 方案：Reader 四类子面板按文章 ID 建立生命周期；共享 Hook 以请求身份保护 Channel / Promise 回调，并在文章切换、面板卸载、请求替换或终态后失去显示归属。仅隔离显示，不隐式取消后台任务。
- 笔记：新增会话草稿存储，切换 / 关闭重开按文章恢复；保存绑定文章、文本、修订号，pending 跨面板共享以防重复提交，旧完成不会清除更新草稿。失败保留草稿并显示双语错误；取消明确丢弃当前草稿。未保存草稿不跨应用重启恢复。
- 标签：建议按请求身份隔离，手工输入随文章重置；添加 / 删除使用原文章快照，完成后失效对应文章详情缓存。
- 验证：`npm.cmd run lint`、`npm.cmd run build`、`git diff --check` 通过；全部改动 / 新增文件严格 UTF-8 无 BOM 校验、双语 JSON 解析和错误提示键检查通过。初次 lint 指出自定义 Hook 命名不符合 React 规则，改为 `useReaderRequest` 后通过。
- 隔离交互：`node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"`，15 项全部通过；覆盖跨文章草稿 / 保存、关闭重开、防重复、修订保护、失败重试、三类 AI 旧回调及同文章请求替换、标签写入 / 删除快照与缓存。脚手架初次缺少路径别名 / mock invoke / DOM MessageChannel 已修正，最终成功结果不是桌面验证。完整命令及逐项证据见 [回归说明](../tests/reader_ownership/README.md)。
- 依赖来源：npm 官方 registry 单独安装 Happy DOM `20.14.6` 到系统临时目录；项目 package.json / 锁文件未改。测试使用内存替身，不访问用户数据库、凭据或真实 AI。临时 loopback 服务已停止。
- 未验证：CUA 实测浏览器 / 应用列表为空、iab 不可用；完整 Tauri / WebView2 路径、视觉布局、真实 AI、macOS / Linux 均未验收。未改 Rust，未重跑 Rust。台账 M4W-001 保留“待验证”，M4W-002 / M4W-003 等问题未推进。

## 2026-10-12 · M4W-016 笔记 Markdown HTML 净化

- 基线：HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de`，保留 M4W-001 全部未提交修改；本项实现 / 回归 / 记录同样未提交。Windows / PowerShell 7.6.5。
- 方案：确认 `render_markdown` 的唯一调用者是 ReaderNote，先 marked 转换，再用 DOMPurify 3.4.16 明确限制标签和各元素属性；HTTP(S) / mailto / 锚点链接、HTTP(S) 图片，拒绝脚本、事件、样式、SVG / MathML、嵌入、危险协议、相对路径及表单交互。保留常用 Markdown、只读 GFM checkbox、列表编号、表格对齐和代码语言类。原始 Markdown 仍可编辑，存储不变。
- 依赖：通过 `npm.cmd install dompurify@^3.4.16 --legacy-peer-deps --ignore-scripts` 维护 package.json / package-lock.json，只新增 DOMPurify 及可选 trusted-types 类型包，没有无关升级。`npm.cmd audit --json` 仍返回已有 nanoid、postcss、source-map-js 共 3 个告警（1 moderate / 2 high），未涉及新增 DOMPurify，未执行无关 audit fix。
- 安全验证：DOMPurify 官方不建议用 Happy DOM 判断净化安全，单独从 npm 官方 registry 安装 jsdom 30.1.2 到系统临时目录。系统 Node24.12 安装显示引擎要求告警，实际运行改用现有 Codex Node24.19。6 项实际通过：恶意 HTML fixture、13 种危险 / 混淆 URL、4 个独立畸形 / 命名空间案例并重新解析、正常 Markdown、生产 ReaderNote 净化插入、编辑器保留原文。详见 [命令与结果](../tests/markdown_safety/README.md)。
- 首次回归发现严格 URL 正则也删除 checkbox 的 type（会退化为文本输入）、ol start、表格 align，已为这些受限非 URL 属性添加专门配置，最终回归通过。测试断言净化后的 DOM，不以 jsdom 禁止自动脚本执行作为净化证据。
- 最终检查：`npm.cmd run lint`、`npm.cmd run build`、`git diff --check` 通过；本项 13 个代码 / 依赖 / 记录 / fixture 文件严格 UTF-8 无 BOM 检查通过；M4W-001 的 15 项 Happy DOM 回归复测通过。未接触真实数据库、凭据或 AI；没有改 Rust / Tauri 权限，未重跑 Rust 检查。
- 未验证：真实浏览器 / Tauri / WebView2 的笔记保存、重开与视觉排版，以及 macOS / Linux；按已知浏览器连接缺失保留 M4W-016“待验证”。本子任务仅处理本编号，后续顺序仍按计划继续 M4W-005。

## 2026-10-12 · M4W-005 Feed 正文来源、升级与离线回退

- 基线 HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de`，保留 M4W-001、M4W-016 全部未提交改动；本项也未提交。实际目录 G:/sth_interesting/mercury4win，Windows / PowerShell 7.6.5、Rust 1.97.0 x86_64-pc-windows-gnu、Node24.12.0。
- 先阅读状态 / 问题 / F-001 约定、上游 README 与 reader-mode / feed-parser-fix 文档，再核对上游 FeedEntryMapper、ReaderDefaultPipeline。先向父任务说明来源拆分、schema v2、失败不污染成功缓存和旧数据保留方案后实现。
- 分开 summary 与 feed_content / feed_content_type，同步 parser、Rust Entry / SQL、前端类型；显式正文含短贴直接读，summary-only 抓文章。v2 事务升级保留原始旧 summary、订阅与用户 / AI 历史，只清理 html 与旧合并字段相等的派生正文及缓存；旧来源不能推断，离线带标记保留可见，刷新后重分类。
- 抓取失败、空 / 挑战 / 提取失败、无 URL 或离线时，导读安全转为字面量代码块，HTML 输出固定来源标记；前端增加双语来源提示与重试，降级不写成功正文 / 缓存。显式正文或 URL 改变失效对应派生内容，重复同步来源不变保留已下载正文。阅读渲染版本升至 3，保留文章 URL 到 document_base_url 供 M4W-008 后续处理。
- 实测通过：Rust Feed / Reader 9 项、迁移 4 项；Node + Happy DOM20.14.6 生产 ReaderPane 的2项来源提示 / 重试 / 切换回归，M4W-001 原15项复测；cargo check（15条既有警告）、npm.cmd run lint / build、本项7个Rust文件的定向格式检查及 git diff --check。详细命令及覆盖见 [回归说明](../tests/feed_content/README.md)。没有新增项目依赖。
- 初始失败保留：独立integration-test exe报历史同款0xc0000139，改为仓库持久lib单元测试模块后正常执行；初版无code的pre包装导致字面量尖括号被comrak当HTML省略，改为pre/code转换后实际文字 / 中文 / 尖括号保留；Markdown下划线转义断言改为语义核对。全仓cargo fmt仍失败，未改文件已有格式差异不作全仓重排；全套Rust测试未重跑，历史失败记录保留。
- F-001内容保持原样，实际解析11唯一HTTPS源、10个site_url通过；内存库预置3源→全量再增8→重复增0，已读 / 收藏 / 笔记 / 标签保留；生产临时导出往返到另一空库11源通过，临时文件删除。未调用完整Tauri导入Channel或对话框，未请求远程11源、用户DB或真实AI。Reader成功 / 失败恢复仅用127.0.0.1响应。
- 未验证：Tauri / WebView2全文读取、导入 / 导出进度与重启，真实AI输入、跨平台。M4W-005 保持待验证；M4W-008相对URL / 最终重定向 / 分层缓存未在本项完成。静态发现EntryDetail嵌套Rust/平铺TS契约差异，当前唯一ReaderTagging调用仅访问共同顶层tags，没有已复现运行故障；登记M4W-019并建议随M4W-011处理，不在本项改写协议。

## 2026-10-12 M4W-008 Reader URL、版本与缓存

- 基线 HEAD c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de，保留001、016、005全部未提交修改；本项未提交。实际 cwd / Git根G:/sth_interesting/mercury4win，PowerShell7.6.5、Rust1.97.0 GNU、Node24.12.0。
- 先核对环境/台账/审查与上游README、docs/reader-mode.md，再按实际Mercury/Mercury/Reader路径读取DefaultPipeline、BuildPipeline及VersioningTests，报告方案后实现。
- 最终响应URL/有效HTML base供提取与DOM链接图片规范化，Feed片段同样支持相对URL；危险/本地协议移除或禁止导航。raw HTML保持禁用、iframe权限未扩大；错误提示加原双语重试入口，005摘要降级与恢复不回归。
- 三层版本2/2/4按有效上游逐层重建，Markdown回写；content事务同时清理全主题/字体缓存，有效cache直接读取且不重写。旧版本1文章基准重新解析base，新持久base不重复拼接；稳定无来源empty与可重试HTTP/提取故障区分。
- 用cargo add --offline添加已有markup5ever_rcdom0.38.0为直接DOM依赖，锁文件只增加关系，没有升级。持久生产回归在reader/m4w_008_tests.rs、tests/reader_pipeline；未操作真实用户库/凭据。
- 实测本项8项Rust、005原9项、ReaderPane本项2项/005原2项、001原15项DOM通过；cargo check（15条已有警告）、lint/build、定向Rust格式及diff检查通过。扩大reader::模块21通过1失败，原md_to_html标题精确断言仍未适配header ID，留待018；全套Rust/全仓格式未重跑。
- 初始lib测试exe 0xc0000139尚未进入harness，GCC/Rust局部PATH尝试无效。objdump与DLL入口检查定位Common Controls；原19e54测试exe旁实际已有2026-10-11 CommonControls6 manifest，新54fad没有。只复用sidecar到新ignored exe、刷新mtime、进程PATH包含target/debug后正常；run_rust.ps1可重复且返回真实测试退出码，无全局环境改变。详见tests/reader_pipeline/README.md，没有改写旧审查/005历史记录。
- ISSUES将008记为待验证，STATUS记实际结果与下一项006；完整Tauri/WebView2图片、链接打开/排版、桌面重启与其他平台未验证，不据隔离测试关闭。

## 2026-10-12 · M4W-006 文章列表分页

- 基线 HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de`，保留001、016、005、008全部未提交修改；本项未提交。实际 cwd / Git 根 G:/sth_interesting/mercury4win，PowerShell7.6.5、Rust1.97.0 GNU、Node24.12.0。
- 先核对环境、状态 / 台账 / 审查，上游 README 与 docs/archives/stage-2.md，再对照 EntryStore、EntryQueryBuilder 和 QueryRegressionTests，报告日期/ID复合游标、独立总数和列表累积加载方案后实施。
- 游标改为 published_at（可空）/id对象，对齐现有 DESC,DESC；NULL分支、limit+1判尾和1～500limit保护。计数不带游标，保留原筛选；Rust模型/command、TS类型/导出/IPC同步。没有修改用户数据格式或迁移。
- EntryList useInfiniteQuery累积、明确加载更多按钮、双语错误和重试；加载保留已有行、按钮禁用防重，沿用缓存失效前缀。未扩展011搜索/未读/All-Any/快捷键语义，019详情契约仍留后续项。
- 实测4项SQLite与6组生产EntryList/实际wrapper隔离DOM交互通过；覆盖237条乱序日期/同日/NULL完整精确顺序、多页稳定总数、筛选/空结果/尾页、删除边界/晚插旧文、加载/重试、末页Enter选择、已读/删除全页重刷和切范围旧请求隔离。cargo check（15条既有警告）、lint/build、定向Rust格式/diff通过。新增测试首次COUNT读usize编译失败，改为i64后成功；没有把失败记为通过。
- 复用CommonControls6 sidecar/局部PATH脚本和临时HappyDOM20.14.6，未改项目依赖、全局环境、真实用户库或凭据。entry_cmd仅检查本项imports/load_entries格式，保留其他命令历史格式。详见[分页回归](../tests/entry_pagination/README.md)。
- ISSUES记006待验证，STATUS记下一项011；完整Tauri/WebView2分页/默认键盘/滚动排版、真实同步刷新和其他平台未验证，全套Rust/全仓格式未重跑。本项停止后续实施，不提交/推送。


## 2026-10-12 — M4W-011 查询、快捷键 / 删除同步与 M4W-019 详情契约

- 基线：HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de`，保留001、016、005、008、006已有未提交修改；Windows / PowerShell7.6.5、Rust1.97.0 GNU、Node v24.12.0。开始确认实际目录 / Shell / Git根 / status，读STATUS、ISSUES及审查，上游先README / tags-v2 / star / entry-delete文档再查询实现。
- 先报告方案后实现共享EntryFilter：Feed、搜索、未读、收藏和Any / All标签用于SQL列表、总数和跨页已读；Rust command / TS wrapper同一参数契约。前端筛选重置分页、收藏入口、最多5标签及双语上限提示；保持006复合游标。只在查询选择层修改标签UI，未扩展010标签维护或014主题。
- 接入m / s / j / k、Shift+A / Shift+R及Reader r / t / n；屏蔽输入 / contenteditable / IME / modal / 修饰键，j加载下一页并用导航代次和选择快照防延迟抢选。已读可反转、动作可访问标签 / 失败提示；删除后事务更新可见标签计数，缓存失效feeds / tags / entries并按ID清当前Reader。
- 019修复TS嵌套entry，标签归属mock同步真实结构；OptionalExtension恢复不存在文章语义并隐藏软删除详情。Rust实际查询序列化输出到临时JSON，生产TS wrapper和ReaderTagging消费此JSON回归通过，不虚构返回字段。
- 实际通过：011 Rust4项，生产AppShell DOM6组；006 Rust4项 / DOM6组；001 DOM15项；npm lint / build；cargo check（13条既有警告）；本项Rust定向格式 / git diff。命令、逐项范围及环境见[回归README](../tests/entry_filters/README.md)。首次build刷新返回类型推断、测试编译函数名、fixture note.updated_at、旧006缓存键清理均曾失败，修正后重新执行通过。
- 未运行：完整native Tauri / WebView2、iframe内默认键盘、布局 / 滚动、真实订阅刷新 / 重启 / 三平台、全套Rust / 全仓格式。DOM保留原FeedList嵌套button的React警告。未接触用户数据库或凭据、未提交或推送、未开始下一编号；011待验证，019按已满足的结构契约验收关闭。

## 2026-10-12 — M4W-017 SSE UTF-8 与错误事件

- 基线 HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de`，保留001、016、005、008、006、011 /019全部未提交改动；Windows / PowerShell7.6.5、Rust1.97.0 GNU。开始核对cwd / Shell / Git根 / status、AGENTS / STATUS / ISSUES / 审查，先读上游README与stage-3流式约定，再读AgentLLMProvider（SwiftOpenAI负责底层流），参考WHATWG SSE解析标准；上游全程只读。
- 先报告方案后写本地127.0.0.1 HTTP / 实际Channel探针。原实现4项均失败：from_utf8_lossy逐块解码导致拆中文得到���文，多行data / 可选空格丢内容和usage，无DONE的EOF被当成功，quote / newline错误JSON非法。最初中文断言被raw文本非JSON协议阻住，补raw观察再次执行确认字符破坏，失败如实保留于tests/agent_stream/README.md。
- provider改为字节缓冲完整SSE事件再严格UTF8：LF / CRLF / CR、BOM、注释 / metadata、多行data、可选空格；单事件行+内容累计8MiB上限。完整DONE才成功，非法UTF8 / JSON / API错误、EOF缺终止 / 截断、网络断流或Channel失败返回错误，不把部分结果误报成功。摘要增量JSON chunk与既有前端契约一致；summary message和translation segment / error统一serde_json编码。
- 12项定向Rust通过：所有字节分割位置 / 单字节块、中文emoji / 引号换行 / JSON形正文、usage / DONE、三种换行、截断和HTTP实际断流、两种缓冲上限、Channel失败，以及真实summary / translation agent+内存SQLite+本地HTTP+实际Channel错误JSON。cargo check通过13条既有警告；5文件rustfmt / diff通过。仅对本项Rust文件格式化，无全仓重排；未改前端，未重跑lint / build / 全套Rust / 全仓格式。
- 复用reader_pipeline/run_rust.ps1的CommonControls6 sidecar和局部PATH，仅写忽略的构建产物。没有依赖 / 锁文件 / 用户数据库 / 凭据 / 权限变更，没有远程AI请求。翻译并发失败后done / success的既有缺陷仍留004，未实现调度003、持久化002或翻译分段004。
- ISSUES记017待验证，STATUS记录本项结果及下一项003；native Tauri / WebView2展示、真实provider兼容性 / AI、其他平台未验证，尤其无DONE provider现在明确失败。未提交 / 推送，未启动下一编号。详细证据见[回归说明](../tests/agent_stream/README.md)。


## 2026-10-12 — M4W-003 实际执行许可、取消与终态释放

- HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de`，保留001、016、005、008、006、011 /019、017已有未提交修改；Windows / PowerShell7.6.5、Rust1.97.0 GNU、Node v24.12.0。核对实际cwd / Shell / Git根 / status，读STATUS、ISSUES与审查；上游先README、摘要无隐式取消与翻译§2.5串行约定，再Runtime和测试。上游只读；先报告总体方案再实施。
- runtime以短std::Mutex临界区维护每类1active + 1latest waiting和request ID；watch许可控制路由 / 凭据及完整executor。Scoped future先drop所有HTTP / buffer_unordered分段，再由RAII guard按ID释放 / 提升；成功、错误、路由失败、外部drop / panic均覆盖。显式cancel只发信号，abandon仅撤waiting；5分钟登记前撤销记录应对IPC乱序。
- 三类command统一execute入口；模型路由提取不涉及凭据的resolve_agent_route以实际空配置验证失败终态，选择规则没有扩展009。新增abandon_agent_waiting IPC注册和TS封装，AgentStatus带active / waiting请求ID，标签也有Channel。摘要done移到用量await后，终态事件后不再await；017安全SSE / JSON事件保持。
- 前端useAgentRequest沿用001展示owner，新增等待 / 运行 / 取消、重试入口；卸载撤waiting、运行继续。返回原文章通过后台status投影原任务与取消ID；取消失败由实际status投影，自然完成不锁控件；无Channel的登记前取消也展示终态。翻译段error不提前解锁，004错误聚合 / 双语 / 分段及002持久化未扩展。
- 实际通过：003 Rust8项（生产三类agent / loopback HTTP / 内存SQLite），A+C每类仅2请求、取消关闭socket / 无done、翻译1 / 3并发全部断开且后续段未启动；生产组件 + 实际wrapper DOM8组；001原15项DOM、017原12项Rust；cargo check（10既有警告）、lint / build、定向格式 / diff。复用008 GNU sidecar和既有临时HappyDOM20.14.6，没有依赖 / 权限 / 用户数据变化。
- 扩大Agent模块31项，30通过、1个既有tagging Prompt版本2与断言1失败，留018；全套Rust / 全仓格式未运行。初次标签变量TS、Rust channel变量遮蔽、带Error前缀取消、001段失败提前重试断言均失败过并修正；父代理发现后台取消失败卡住，补自然完成回归通过，详见[证据](../tests/agent_runtime/README.md)。agent_cmd仅格式化本项路由 / runtime / status段，保留无关Provider和模型管理格式。
- ISSUES标003待验证，STATUS记下一项002；native Tauri / WebView2实际IPC / Channel / 按钮与布局、真实AI / provider、Linux / macOS尚未验证。未接触真实库 / 凭据，未提交 / 推送，未开始下一项。

## 2026-10-12 — M4W-002 摘要 / 翻译持久化与恢复

- 基线HEAD `c90567b`，保留此前001、016、005、008、006、011 /019、017、003未提交修改；Windows /PowerShell7.6.5、Rust1.97.0 GNU、Node v24.12.0。核对cwd /Shell /Git及状态，只读上游README、summary-agent /translate-agent、翻译存储与测试；先报告方案后实现。
- 新增schema v3与agent_result_store，按来源 /参数存储摘要完整success及翻译checkpoint /完整success。完整SSE delta收集；成功段恢复和缺段续跑，失败 /取消保护旧成功；最低错误聚合防止partial被误记success。Request提交fence处理已发interact晚提交，COMMIT与取消有明确边界；来源变化旧保存拒绝。
- load_*生产IPC /注册 /TS类型 /wrapper /React查询接通，参数切换只读取、后台完成 /取消刷新、Reader重建刷新首次空结果；部分译文恢复有双语提示。未来004换稳定HTMLsegmenter版本，007使用load_summary接口，009扩展Prompt /route参数指纹，范围未混入本项。
- 7项本项Rust（本地HTTP、内存 /临时文件DB、重开pool、实际取消checkpoint与已派发interact边界）、4项迁移及7组生产DOM通过；003原8项Rust /8组DOM、017原12项Rust、001原15项DOM复测通过。cargo check（9条既有警告）、lint /build、定向格式 /diff通过；详见[命令及范围](../tests/agent_persistence/README.md)。旧001mock扩展捕获实际事件后保存结果，不弱化断言。
- 初次准备失败和修正如实记录在回归README：类型替换 /函数遮蔽 /临时借用、旧mock缺持久结果、取消内存连接被替换及Windows临时DB句柄释放。实际取消恢复改用临时文件DB；清理等待只处理生成的临时文件。
- 未访问用户库 /凭据或真实AI，未提交 /推送；native Tauri /WebView2关闭重开 /完整重启、真实provider和其他平台未验证，全套Rust /全仓格式未运行。M4W-002标“待验证”，未开始004。


## 2026-10-12 — M4W-004 翻译稳定分段、失败反馈、双语与恢复

- HEAD `c90567b`，保留001、016、005、008、006、011 /019、017、003、002全部未提交修改；Windows /PowerShell7.6.5、Rust1.97.0 GNU、Node v24.12.0。核对实际cwd /Shell /Git状态，读STATUS /ISSUES /原审查，只读上游README、translate-agent、Contracts /SegmentExtractor /Execution及测试；先报告方案后实施。
- 新translation_segments用生产Reader渲染HTML提取p /ul /ol；外层列表不再重复内p /嵌套列表，空图片段 /代码块 /引用 /表格原样保留，内联代码用反引号保留语义。源段ID基于类型 /顺序 /HTML /文本，升级reader-html-block-v2使旧索引slot失效。AgentResult增加默认字段segment_ids /segment_errors，保留schema3摘要反序列化兼容，不增加迁移。
- 翻译共享串行 /并发结果归并；实际concurrency统一1..5，所有请求收尾再判完整 /部分 /全失败，空响应 /显式截断失败，完整COMMIT后才done。逐段成功和错误checkpoint，取消沿用002fence；续跑缺段、source ID单段重试、来源变化拒绝旧ID、临时pool重开恢复及串行上下文通过。HY-MT内置模板版本2补同样上下文条件，不混入009自定义模板 /路由功能。
- load_translation_document及可选sourceSegmentId IPC同步Rust注册 /TS类型 /wrapper /消费者。来源配对面板、三种终态、段重试、读失败可见与阻止 /重试接通；iframe使用纯textContent译文插入原文后，沿用当前head主题 /字体和sandbox，不改持久原文HTML cache。切文章回原文，取消 /旧回调隔离保持。
- 实际通过004Rust6项 /生产DOM6组、002原Rust7 /DOM7、003原Rust8 /DOM8、017原Rust12、001原DOM15；cargo check（9既有警告）、lint /build、定向rustfmt /diff通过。生产DOM读取Rust生成HTMLfixture，本地HTTP实测5个并发上限 /不提前终态，临时文件DB重开恢复。测试第一次并发HTTPfixture只读一块导致偶发RST，改按完整Content-Length收齐；017复测检出旧segment字段遗漏，保留兼容字段后12项重新通过；均未删除或弱化原验收断言。
- 更新ISSUES为待验证 /STATUS /本日志，详细命令和逐项结果见[翻译回归](../tests/translation/README.md)。native Tauri /WebView2实际IPC /双语视觉与滚动、完整桌面重启、真实AI /provider及其他平台未验证；按完成段更新srcDoc，token级DOMpatch未实现。全套Rust /全仓格式未运行，无依赖 /锁文件 /权限 /用户数据变化，未提交 /推送 /开始009。


## 2026-10-13 — M4W-004 收尾与009交接

- 延续HEAD `c90567b`及全部既有未提交修改，重新核对实际工作目录 / PowerShell7.6.5 / Git根 / 状态。补齐此前中断未写入的tests/translation/README.md，同步ISSUES / STATUS，保留10月12日实现、定向Rust / DOM结果和曾出现的失败记录。
- 本日实际通过：cargo check（9条既有警告）、npm.cmd run lint / build、003定向格式脚本、004相关六文件rustfmt检查、git diff --check；build有Tailwind插件耗时提示但成功。没有扩大或重跑Rust / DOM回归，没有业务代码变更、提交 / 推送或启动009。
- M4W-004仍待验证：native Tauri / WebView2实际IPC、双语排版 / 滚动、完整桌面重启、真实provider / AI、Linux / macOS；全套Rust / 全仓格式未运行。完成段目前替换完整srcDoc，token级增量patch尚未实现。
- 009交接：translation_parameters统一segmenter / strategy / 归一concurrency，ResultSlot另包含entry / language / source hash；translation_segments::load_document及agent_cmd三个读取 / 运行入口须同步新模板 / route指纹。内置模板仍由load_builtin_template选择，model由resolve_agent_model / resolve_agent_route选择；相关前端查询key也应同步，防止恢复旧参数结果。完整命令与范围见[翻译回归](../tests/translation/README.md)。


## 2026-10-13 — M4W-009 配置进入实际执行与自动摘要

- 基线HEAD `c90567b`，完整保留此前001 /016 /005 /008 /006 /011 /019 /017 /003 /002 /004未提交修改。Windows /PowerShell7.6.5、Rust1.97.0 GNU、Node v24.12.0；先检查环境 /台账并只读上游对应文档和实现，先说明方案再修改。
- Prompt统一resolve和校验，三类custom优先；原子上传 /拒绝策略、无效磁盘template内置降级notice、主备用能力与启用校验及真实请求backup接入。摘要reset避免partial混合，翻译失败段backup；失败不破坏既有成功，取消不备用。
- Configuration快照同步模板 /route /strategy /concurrency、共享请求参数指纹；002统一slot参数、三读取IPC /load_document与前端query同步。历史load_latest_summary只读当前source成功、不依赖凭据 /Provider，旧slot不误命中新配置；request_route /segment_routes记录实际模型与Provider，为007 /012交接。
- 保存默认与临时选择、确认开启auto、正文ready /读取完成后1秒防抖、003串行 /等待清除 /active保留、失败不自动重试实现。有历史成功不auto重新发送，旧配置结果明确标注且可手动生成。
- 7项本项Rust /8组本项DOM通过；原004 /002 /003 /017共33项Rust、004 /002 /003 /001共36组DOM复测通过。取消主请求与backup已启动边界均实测socket关闭、1 /2请求无追加。check（6条既有警告）、lint /build、定向格式 /diff通过。初次测试准备构造 /遮蔽 /Debug编译失败与一次替换误伤语法失败均修正后重检，详情见[配置回归](../tests/agent_configuration/README.md)。
- 未验证native Tauri /WebView2对话框 /Channel /auto /布局 /完整重启、真实AI /Provider及其他平台，全套Rust /全仓格式未运行；没有访问用户数据库 /凭据。本项保持待验证，未提交 /推送，未启动007。


## 2026-10-13 — M4W-007 文摘模板、单篇与多篇导出

- HEAD `c90567b`，保留001、016、005、008、006、011 /019、017、003、002、004、009全部未提交修改；Windows /PowerShell7.6.5、Rust1.97.0 GNU、Node v24.12.0。核对实际目录 /Shell /Git状态，读取STATUS /ISSUES /原审查，只读上游README、note-and-sharing、DigestComposition /projection和单多篇测试，先报告方案再实施。上游路径首次少一层Mercury，实际rg列目录后读取正确路径，没有创建假目录或移动文件。
- 三模板改合法Handlebars each /if；single_text改真正纯文本，单 /多篇Markdown保留。元数据文字转义 /原URL /作者fallback，摘要带模型 /语言 /detail；正文按008当前版本缓存、笔记只读已存note、摘要使用009当前source完整success。不抓文 /生成AI /要求凭据，source变化不复用旧成功。选中缺项逐篇展示，未保存笔记提示；单默认已有笔记 /摘要，多默认false，正文false。
- Reader单篇窗口、EntryList临时冻结选择 /列表顺序 /筛选清理、共享选择store使Reader inert及快捷键暂停、preview IPC /export_multi wrapper /TS模型注册接通。预览pre纯文字显示实际输出，expectedContent重读核对后异步UTF-8无BOM写入；真实保存接口md /txt过滤、取消无export、失败重试和成功路径提示。固定导出目录 /macOS分享 /模板自定义没有扩展；共享Dialog焦点留015。
- 5项本项Rust（内存真实store /HB /临时中文文件 /来源失效 /无配置读取）、7组生产组件 /实际wrapper DOM通过；001原15、006原6组、011原6组DOM及011原4项Rust复测通过。cargo check（6既有警告）、lint /build、3文件定向格式 /git diff通过；全仓cargo fmt实际失败，仍为既有未修改文件格式差异，未运行全套Rust。
- 初次fixture漏笔记updated_at和Markdown归属括号转义断言失败，修正后通过；旧001mock静态导入缺export符号、011DOM缺必需详情JSON参数导致准备失败，补严格抛错stub /按原流程生成fixture后原回归通过。没有弱化验收断言。详细命令及未验证范围见[文摘回归](../tests/digest_export/README.md)。
- 更新ISSUES为待验证 /STATUS下一项010与本日志。native Tauri /WebView2真实IPC /系统保存对话框 /路径权限 /覆盖 /inert键盘与排版、完整重启、Linux /macOS未验证；没有访问用户库 /凭据 /真实AI，无依赖 /锁文件 /权限变更，未提交 /推送，未开始010。

## 2026-10-13 — M4W-010 标签事务、规范化 /别名和批量审核

- 基线：HEAD `c90567b` 加保留001、016、005、008、006、011 /019、017、003、002、004、009、007和本项未提交改动；Windows /PowerShell7.6.5、Rust1.97.0 GNU、Node24.12.0。未提交 /推送，未修改只读上游或真实数据库 /凭据。
- 读环境 /AGENTS /STATUS /台账 /审计P2-10 /串行顺序；对照上游标签规范化、alias bypass、批量staging /审核、TagLibrary维护合同与实现测试，报告方案后实施。
- 数据：统一canonical和alias gate，合并事务迁移关联 /confidence、保留人工来源与旧名 /别名、去重计数及回滚；rename /alias冲突、提升 /清理。schema v4新增alias显示及批量独立表，兼容旧库规范化碰撞 /人工状态 /别名关系，迁移幂等。
- 执行：批量scope预览 /冻结数量、显式模型 / >100确认 /1..5并发；复用Tagging外层runtime许可和冻结configuration /真实主备用执行。创建 /staging受取消fence；AI输出只staging，审核每个新提议后另次确认Apply。输入与来源hash单次SQLite快照，Apply校验来源 /删除及重解析canonical；整次原子提交、完成幂等。取消保留已完成审核，未开始条目仍可重试；孤立running恢复ready，不自动重发AI。
- 前端：同步所有Rust注册 /TS wrapper /类型 /生产调用；Settings /Sidebar维护、选择映射与关联Query失效，Reader无AI本地 /手工 /常用标签、共标签相关文章导航。batch UI提供scope /进度 /逐篇失败 /取消 /Keep-Discard /确认Apply /恢复重试，生命周期锁覆盖Settings Escape /route tabs。单篇保留原文章详情失效key及001归属护栏。
- 实际验证：`run_rust.ps1 -Filter tag_`13通过（本项12 +011旧删除1）；迁移4、runtime8、m4w_0097、m4w_0114分别通过。本项生产DOM10组；001原15、003原8、011原6组DOM通过。最终cargo check通过（6既有警告）、npm.cmd lint /build通过（既有chunk提示）、9文件定向rustfmt检查和git diff --check通过。loopback取消观察socket关闭与无第3请求，主备用10请求 /峰值2 /2成功1失败，成功用量标实际Backup；临时SQLite重开保留审核 /标签。
- 中间准备 /回归失败及修正：SQLite usize类型、测试临时Client借用、fake响应缺role、DOM cleanup竞争、011 fixture参数、001旧mock新增query导出和原文章cachekey、临时SQLite后台drop文件锁。保留失败原因及最终通过证据于[标签回归](../tests/tag_system/README.md)，没有改写历史审计。FeedList既有嵌套button提示仍存在。
- 边界 /未验证：Apply采用整次原子事务，未实现上游分块提交 /分块progress；生成中断回到审核，无自动付费续跑。请求级失败用量留012，旧Prompt /Reader断言 /全仓fmt失败留018。未运行native Tauri /WebView2、真实AI、完整退出重启及其他平台或全套Rust /全仓fmt。010标待验证，下一012尚未启动。


## 2026-10-13 — M4W-012 请求用量与 Provider /Model 生命周期

- 基线：HEAD `c90567b` 加此前串行修复和本项未提交工作区；Windows /PowerShell7.6.5、Rust1.97.0 GNU、Node24.12.0、Python3.14.3。保留所有前项改动，确认实际cwd /shell /Git状态；上游只读，没有用户库 /真实凭据或AI访问，未提交 /推送。
- 对照上游token-usage、Provider归档 /恢复实现及LLMUsageEventContext /query测试后先报告方案。schema v5仅追加迁移，旧usage保留legacy；每次实际Provider HTTP尝试记录实际ID /名称 /去敏endpoint、normal /fallback /retry、成功 /失败 /取消 /超时、NULL /真实0 /部分Token，SSE要求include_usage。去除原debug Key片段；HTTP错误信息保持017格式兼容并去Key字符串。任务解析 /缓存COMMIT与请求事实独立，读缓存和未发送排队取消不虚增请求。
- RAII取消仅有限DB任务、正常await写、event_key幂等、失败日志及会话报表警告；取消已结束HTTP不改其终态。真实单连接并发回归发现buffer_unordered暂停持池租约与checkpoint等待死锁，改独立轮询有限DB写解决；不保留HTTP/socket、不绕过结果fence、不扩大pool。强杀退出未完成日志不保证。
- Provider /Model删除IPC转事务归档，cascade /错误回滚、保留Key /外键引用 /历史；启用入口、活跃列表 /route合法性与配置fingerprint /models /usage失效一致。恢复Provider要求规范endpoint+同名称，UI明示原Key复用 /新账户换名；模型独立复活，显式创建 /恢复能力三项选择贯通wrapper /command /store，旧调用新建默认全支持 /恢复保留。
- 报表同事务的UTC[start,end) /日桶、ID稳定三维分组 /当前名 /归档历史；共享对象范围、7 /14 /30 /90日、已知Token与coverage /legacy独立 /写失败可见，指标排序的分组比较图 /UTC日图 /表格。生产Settings各对象统计入口、归档模型清单 /取消 /错误重试、禁用 /不可用选项、全局错误反馈接通。
- 实际结果：本项13项Rust /8组生产DOM通过（Rust真实DTO fixture）；0038 /0097 /010批量7 /01712 /迁移4项Rust与009原8组DOM通过。010批量同10请求现在准确10事件（2成功8失败，备用5次），审核业务未改。最终cargo check通过保留6既有warning，lint /build通过保留chunk提示，12文件定向fmt /diff通过；能力补全后只重跑本项及check /lint /build；追加全部Provider归档后历史摘要读取 /无追加请求断言单项通过。
- 中间失败：脚本括号替换、runtime测试构造API、HappyDOM mock函数 /英文断言准备；单连接真死锁；017原2断言因本项错误诊断丢消息而失败，恢复契约后原12通过；初始两个错误filter跑0项，纠正实际模块路径后8 /7通过。细节和所有最终命令见[用量回归](../tests/usage_lifecycle/README.md)，保留历史audit /018失败。
- 更新012为待验证 /STATUS下一013、本日志和长期事实 /归档约定。native Tauri /WebView2实际IPC /界面 /焦点键盘、实际合成凭据文件失败 /完整退出重启、真实Provider include_usage和其他平台未验证；全套Rust /全仓fmt未运行。尚未启动013。

## 2026-10-13 — M4W-013 Obsidian Publish 静态阅读管道

- 基线：HEAD `c90567b` 加保留001、016、005、008、006、011 /019、017、003、002、004、009、007、010、012及本项未提交工作区；Windows /PowerShell7.6.5、Rust1.97.0 GNU。确认cwd /Shell /Git根及status，读AGENTS /STATUS /ISSUES /原审计 /串行顺序。最初序列文档路径不匹配，检查实际docs/audits后按真实文件读取，没有创建虚假目录；UTF-8读写。
- 对照只读上游README /obsidian-pub /Resolver /ObsidianPipeline与原fixture /测试，先报告静态管道方案后实施。新增reader/obsidian和7项生产回归，保留上游真实shell fixture；DOM /静态字面量 /Publish host和uid校验、不执行脚本，有限重定向 /15秒deadline /响应状态和UTF-8验证。
- 独立pipeline_type /resolved URL /原shell /最终响应URL，失败pending而非成功正文，005临时导读和重试保留；成功canonical Markdown /共享renderer，Readability元数据空、正文 /所有主题字体缓存由既有事务更新。Readability3重新识别错误旧shell，Obsidian当前Markdown离线命中 /重渲染、过期重抓 /缺身份重解析。007正文判定共享has_current_markdown及当前fixture版本同步，004 /摘要无需改业务且生产消费者通过。
- AST处理普通链接 /图片相对URL及危险协议，资源cache对象键仅exact /decoded /唯一suffix-basename匹配媒体，逐路径编码；未匹配 /歧义 /索引失败保留文字，inline code /code block保留。raw HTML仍共享禁用，未改iframe /IPC /schema /依赖 /权限，不处理generic SPA和完整Obsidian语法。
- 实际最终结果：本项7通过；0059 /0088 /004 translation_segments6 /0075通过。cancel-drop真实socket关闭，无成功缓存；生产15秒HTTP超时失败保留pending；拒绝外部重定向后被禁止listener没有请求；真实SQLitetrigger证明命中不写，render版本失效离线重渲染并保持hash，translation_document和digest正文通过。cargo check通过保留6既有warning，6文件定向fmt /diff通过。
- 中间准备失败：新测试字符串转义语法；comrak AST跨await使Tauri future非Send；旧cache fixture漏updated_at；007错误filter匹配0项。逐项修正后真实通过，不计0项为验收，不改历史audit /018失败。详见[013证据](../tests/obsidian_publish/README.md)。
- 更新ISSUES待验证、STATUS下一014、D-005静态分支范围与本日志。无前端改动，未跑lint /build /DOM、全套Rust /全仓fmt；native Tauri /WebView2、在线Publish兼容 /媒体实际加载 /链接和排版、完整桌面重启、其他平台未验证。无用户库 /凭据 /远程访问，未提交 /推送，未启动014。
## 2026-10-13 — M4W-014 阅读主题入口与系统主题同步

- 基线：HEAD c90567b加此前串行修复与本项未提交工作区；Windows /PowerShell7.6.5、Rust1.97.0 GNU、Node24.12.0。确认实际cwd /shell /Git状态，保留全部前项改动；上游只读，未访问用户库 /凭据 /真实AI，未提交 /推送 /发布。
- 对照上游README、docs/features/theme.md、custom-font与ReaderTheme.swift /ContentView的主题身份约定，报告方案后实施。沿用已有字体窗口为阅读设置，增加Classic /Paper与共享亮暗模式、双语说明 /关联label /toolbar名称 /选中状态 /字号label；旧localStorage键和值保持，新样式键默认Classic。
- App改调用useTheme，系统外观进Zustand响应式system_theme；一次change订阅、订阅后初始重采样、StrictMode清理、强制light /dark保持、auto回归即时取当前值。Reader以style+有效亮暗及原字体 /字号发实际wrapper请求；Paper只采纳上游现有亮暗配色，显式字体和Classic保留，render version升5只重渲染原Markdown，不重抓 /不迁移DB /不加权限。
- 双语回调携带原Reader HTML身份，当前文章和当前HTML一致才显示；004继续复制当前head，迟到主题 /文章 /translation document不覆盖新head，切文章回原文，sandbox不扩权。主题字体修改不新发AI。
- 实际验证：本项3项Rust、8组×3localStorage状态共24组生产DOM通过（真实Rust HTML fixture /生产组件、useTheme、i18next与TS wrapper）；008原8Rust /2DOM、004原6Rust /6DOM、001原15DOM通过。Paper正文 /次要文字 /链接两版均≥4.5:1；60缓存身份 /拒绝source UPDATE /拒绝cache INSERT /offline URL证明只渲染和命中。check通过6既有警告、lint /build通过（build已有chunk提示）、4文件定向fmt /diff通过。
- 中间准备失败：早期历史命名路径不存在后rg核对；Rust测试引用不存在Content.updated_at改真实字段；DOM假设System Default改实际System；测试组件无export lint提示已修正；错误translation filter跑0项不计通过，纠正实际模块后6项通过。详情见[主题回归](../tests/reader_theme/README.md)，历史audit保持原样。
- 更新014为待验证 /STATUS下一015与本日志。native Tauri /WebView2真实系统外观事件 /字体可用性 /iframe视觉排版 /完整退出重启、Linux /macOS未验证；全套Rust /全仓fmt未跑，018旧失败保留。全局Dialog焦点和其他文案入口留015；无依赖 /锁文件变更。未启动015。

## 2026-10-13 — M4W-015 双语文案与可访问性

- 基线：HEAD c90567b加保留的此前串行修复和本项未提交工作区；Windows /PowerShell7.6.5、Node24.12.0。开始确认实际cwd /shell /Git根 /status，读当前STATUS /ISSUES /历史audit /串行计划；上游README /l10n /SwiftUI名称和键盘入口只读。先报告范围后实施，不提交 /推送 /发布，不访问用户库 /凭据 /真实AI。
- 当前Reader错误 /标签空状态已有双语键，补主题切换当前模式、Feed同步 /失败 /删除名称 /URL label、OPML窗口标题 /完成按钮与共享错误重试；保留机器诊断。Feed行拆同级原生选择 /删除buttons并维持焦点可见；Settings表单 /Reader外链 /笔记标签输入补名称，Sidebar /Reader /Settings /摘要详细度 /文章标签选中状态及作用域alert /status完善。
- 共享Dialog改body portal并维护嵌套stack，顶层视觉z-index /唯一aria-modal /背景与父层inert原值恢复；Tab /ShiftTab包括链接 /summary并排除不可用和隐藏祖先 /closed details正文，空面板fallback与异步React控件focus、程序focus护栏、单层Escape /backdrop、IME /repeat和卸载 /失效trigger恢复。保留010父设置 /batch审核关闭锁、007选择inert和014主题样式；不改Rust /IPC /sandbox /权限 /依赖。
- 本项10组DOM实际生产组件 /wrapper /i18next通过，351/351键与插值 /无BOM检查通过；010原10、007原7、014原24、001原15、011/019原6、006原6组DOM复测通过，共78组。lint /build /diff通过，build保留已有chunk提示。没有运行Rust或全仓fmt /全套测试，018历史失败不改。
- 准备失败如实登记：CRLF文本定位脚本失败未写；中文测试沿用英文写过的内存fixture导致空状态断言失败，重置后通过；summary加入后程序click无默认focus导致trigger断言失败，明确focus后通过；007查询body误取测试result pre导致expectedContent失败，排除测试pre后原7组通过。旧007/010/014脚手架仅扩query范围包含portal，断言保留。预期OPML窗口失败日志对应受控错误路径。详见[015回归](../tests/accessibility/README.md)。
- 最终静态界面键扫描发现既有自动摘要确认common.cancel缺键，补齐双语；最终351 /351资源 /插值 /静态引用、新10组DOM、lint /build重新通过。
- 更新ISSUES待验证 /STATUS下一018 /本日志及桌面验收步骤。真实WebView2 /Tauri焦点 /原生button EnterSpace默认行为 /屏幕阅读器 /inert /iframe /窗口切换 /亮暗窄窗口布局 /完整退出重启、Linux /macOS未验证。未启动018。

## 2026-10-13 — M4W-018 测试语义、格式与最终隔离验证

- 基线HEAD `c90567b` +所有此前串行修复和本项未提交改动；Windows /PowerShell7.6.5、Rust1.97.0 GNU。先确认cwd /shell /Git根 /status，读AGENTS /STATUS /ISSUES /原audit /串行顺序，先报告大体方案再实施，全部前项保留，无提交 /推送 /发布 /外部AI或用户数据访问。
- 初始全套Rust129通过5失败；核对后修正tagging /HY-MT模板version2并补渲染与条件、Reader可见h1 /匹配锚点 /正文语义，002 /010测试改真实旧Schema而非最新库降版本marker，保留数据 /别名 /计数 /重复迁移及外键断言。没有删测试 /ignore /skip /单线程，没有为坏fixture改变生产迁移。全套未复现usage全局drain或failure隔离失败，不猜测重构。
- 初始fmt17文件失败，最小清理6unused import；用户授权cargo fmt纯Rust实际22文件，format前后71源码文件对独立rustfmt副本逐Hash一致。本项更改测试和导入，生产业务版本 /迁移 /IPC /权限不变；历史audit没有改写。
- 最终普通cargo test --locked134通过0失败0忽略（默认并行，binary /doc0项执行）；当前GNU产物已有sidecar准备。cargo check0warning、全仓cargo fmt --check、lint /build、diff /UTF-8 /文档链接通过，既有buildchunk提示保留。15套生产DOM125项 /组通过（014为24），351双语keys /插值一致；安全净化Node24.19.0 +jsdom30.1.2，其余HappyDOM20.14.6，实际RustDTO /HTML临时生成，无native替代宣称。
- 新增可复现DOM runner与[最终报告](audits/final-validation-2026-10-13.md)，直接JSON用量fixture避免改写repo测试源。初次runner只识别array型passed，tag_system数字10导致汇总中断；修正统计后完整15套重跑125通过，中间失败独立保留，未删除产品断言。
- 关闭018，019原关闭保持，其余17问题待验证；STATUS收敛为当前结论 /最终基线 /下一步并链接原证据，序列追加完成记录。下一步为隔离native Tauri /WebView2真实IPC /Channel /对话框 /键盘焦点 /排版 /重启 /无AI配置验收，再真实Provider /在线站点与三平台；没有下一实现编号。

## 2026-10-13 — Windows 快捷开发启动脚本

- 基线 HEAD `c90567b` 加现有未提交改动；新增根目录 `start-dev.bat`，通过脚本自身路径进入项目目录，检查 Node / npm / Cargo / Rust 与本地 Tauri CLI，调用 `npm.cmd run tauri -- dev`。依赖缺失只提示安装命令，启动失败保留错误窗口；支持传递 CLI 参数。未修改业务代码或现有开发配置。
- Windows 下从 `C:\` 使用 CMD 执行 `start-dev.bat --help`，实际通过并返回 0，确认跨盘目录定位、工具检查和 Tauri 参数转发。脚本采用 UTF-8 无 BOM / CRLF。
- 未通过本脚本启动桌面应用、执行构建或访问用户数据；此项验证不替代实际桌面启动验收。既有修复与验证记录保留。

## 2026-10-13 — 翻译模式隐藏侧边面板

- 基线：HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加保留的全部既有未提交修复与本次改动；Windows / PowerShell 7.6.5、Node 24.12.0。开始确认实际 cwd / Shell / Git 根 / status，读取 STATUS、AGENTS、M4W-004 记录与证据。默认 UTF-8，无提交 / 推送 / 发布。
- 修改 ReaderPane 的侧栏条件：仅笔记 / 摘要 / 标签显示右侧面板和拖拽分隔条；翻译按钮或 `T` 将控制区放在正文上方。ReaderTranslation 增加 inline 展示，隐藏重复来源 / 译文预览，保留启动、进度、取消、状态 / 错误 / 读取重试、缺段编号重试；控制区限高可滚动，iframe 对照内容沿用既有安全与归属逻辑。
- 扩展既有翻译 ReaderPane 回归并将文章归属的翻译断言移到包含 iframe 的实际展示范围。实际通过 lint / build、翻译 6 组、文章归属 15 组、主题字体 24 组隔离 DOM，共 45 组；diff 检查通过，build 保留既有 >500kB chunk 提示。命令与范围见 [翻译补充回归](../tests/translation/README.md#2026-10-13-翻译不显示侧边面板)。
- 文档补丁曾因既有行空格不同而未匹配，核对实际 UTF-8 内容后以标题定位补充，保留并行写入的启动脚本日志。复用此前生产 Rust 临时 fixture，未重跑 Rust / 完整前端套件；未验证 native Tauri / WebView2 布局 / 滚动 / 焦点 / IPC、真实 AI 和其他平台。未访问用户数据库 / 凭据。同步 STATUS 和 M4W-004 补充证据，问题保持待验证。

## 2026-10-13 — 静默直接翻译与漏译范围修正

- 基线 HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加保留的全部未提交修复与本项；Windows / PowerShell7.6.5、Rust1.97.0 GNU、Node24.12.0。检查cwd / shell / Git状态、STATUS、M4W-004台账 / 证据；只读对照上游README / translate-agent与SegmentExtractor，当前用户指令扩展上游旧p / ul / ol范围，不改上游。
- ReaderPane使用无UI的automatic翻译控制器，点击 / T直接生成或恢复。等待实际profile / fingerprint、来源结果 / Reader就绪与后台状态刷新；完整缓存不请求，每次激活最多一次，失败无付费自动循环、部分结果显式重新选择继续；StrictMode延迟effect清理、旧回调归属与背景任务去重保持。应用不显示操作、进度、状态或错误控制区。
- 漏译根因是原代码只选p / ul / ol且排除引用 / 表格。加入h1–h6、引用正文 / 列表、th / td / caption / figcaption / dt / dd，外层拥有后代避免重复；代码 / 非正文原样。表格译文在合法单元格内追加、dt包装dd，保持textContent安全。分段身份v3隔离漏译v2缓存，保存记录不删除，无schema / 依赖 / 权限变化。
- 扩展原Rust提取及真实生产HTTP / SQLite / fixture回归；14项相关Rust、5套64组DOM、cargo check零warning、定向rustfmt、lint / build、diff / UTF-8检查通过，build既有chunk提示保留。新版10来源逐个真实HTTP请求 / 完整持久化、旧v2拒命中且保留，所有标题等级与引用 / 表格映射、表格HTML重解析、点击 / T / StrictMode / 失败 / 部分 / 迟到配置正文 / 背景 / 主题及文章归属覆盖。命令与边界见[最新翻译证据](../tests/translation/README.md#2026-10-13-静默直接翻译与漏译范围)。
- 用户已授权必要的真实API调用；本项根因在Provider前，采用生产loopback HTTP验证，没有访问真实凭据 / 用户库或付费API。未验证native Tauri / WebView2实际IPC / 排版滚动、用户具体漏译文章 / 真实Provider长段输出、其他平台或全仓套件。同步STATUS、ISSUES、DECISIONS和翻译说明，M4W-004保持待验证；无提交 / 推送 / 发布。
- 收尾补查缓存配置刷新：profile / fingerprint处于刷新或读取失败时禁止启动，后台状态刷新失败同样不使用旧状态启动；新增既有缓存目标语言刷新等待断言，确保实际发送新目标语言。最终上述5套64组DOM与lint / build再次通过，Rust代码未再变更。

## 2026-10-13 — M4W-020 更换字体后滚轮失效

- 基线 HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加全部保留的未提交改动；Windows / PowerShell 7.6.5、Node 24.12.0、Edge 155.0.4283.45。确认 cwd / Shell / Git 根 / status，读取 STATUS / ISSUES 和主题 / 可访问性证据；无用户库 / 凭据访问，无提交 / 推送。
- 真实 Edge 最小探针与生产 ReaderPane 均复现：字体刷新卸载旧 iframe，新 iframe 在 Dialog 的 inert 背景内创建，关闭后滚轮仍失效。初始 wheel scrollTop=400，更换 Serif 后=0；手动改变位置仍有效。共享 Dialog 属性同步尝试在生产路径无效，已撤回。
- ReaderPane 同文章字体 / 字号 / 主题查询保留旧 HTML，刷新不卸载已有 iframe；新 srcDoc 更新同一 frame。初次加载和文章切换保持加载处理，不将 A 正文带入 B。未改 Dialog / IPC / sandbox / Rust / 依赖 / 锁文件。
- 新增 tests/reader_scroll：生产组件、IPC wrapper、实际构建 CSS、Rust fixture head + 合成 100 段正文 / 延迟响应，在独立 headless Edge 和 loopback server 下运行，外部网络拦截。修复后 8 组真实滚轮全部 scrollTop=400，设置打开背景仍不可滚动，选择文字 / PageDown、按钮 / Escape 焦点恢复及 sandbox 通过。
- 既有主题24 / 可访问性10 / 翻译10 / 归属15 / Reader错误链接2，共61组 DOM、lint / build / diff通过，build既有chunk提示保留。准备失败：初版17px不在严格fixture中，改22→23；真实backdrop默认焦点与程序click不同，焦点断言限定按钮 / Escape；文档补丁定位失败核对UTF-8后拆分。详情见 [滚轮回归](../tests/reader_scroll/README.md)。
- 登记 M4W-020 待验证，同步 STATUS / TESTING。本项未运行完整 Tauri / WebView2、触控板、其他平台或 Rust / 全仓验证；真实浏览器回归不等于桌面验收。
## 2026-10-13 — 提交当前修复工作区

- 按用户要求将当前工作区改动提交到本地 main；父基线为 c90567b，包含18阶段移植修复、后续静默翻译 / 漏译范围、M4W-020字体滚轮修复、开发启动脚本、测试与文档。保留所有既有修改，不推送、不发布、不创建标签。
- 提交前核对实际目录 / PowerShell / Git根 / 状态、已登记验证与新增文件，diff空白检查通过；未纳入用户数据库、凭据、临时日志或target / dist / node_modules。此次仅提交整理与文档状态更新，未改业务代码、未重跑测试；既有全套和后续定向验证的范围与未验证项维持原记录。
- STATUS更新为本次提交状态；历史审查和验证报告仍保留各自当时的未提交基线，不改写历史。实际桌面、真实Provider和三平台验收继续按ISSUES推进。

## 2026-10-13 — 软件中文化及语言选择持久化

- HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加开始时已有的全部未提交改动和本任务；实际目录 / Shell / Git 根与状态、STATUS、ISSUES、已有双语资源及语言初始化已检查。Windows / PowerShell 7.6.5、Node 24.12.0、Rust 1.97.0 GNU、Edge 155.0.4283.45。用户在模型可用性询问后明确选择主 agent 完成，没有派发子 agent / 新聊天。
- 原界面有 351 对资源却固定英文启动且无语言入口。改为默认简体中文，侧栏原生选择框保存 mercury_locale，store / i18next / HTML 语言同步；无存储也可切换。补设置 / 文件窗口中文、日期 / 数字格式；文摘携带可选 locale，三种格式标题 / 作者 / 来源 / 摘要出处中文化，旧调用英文兼容，原文和 AI 输出语言配置保留。
- 新增生产 App 的真实 Edge 专项 6 组和两项文摘 Rust；7 项文摘 Rust、37 组相关 DOM、359 对资源键 / 插值 / 引用、lint / build / cargo check / 定向 rustfmt / diff 实际通过，build 保留已有 chunk 提示。修正语言 mock 缺项及此前静默翻译后的过时快捷键断言；浏览器标签定位和临时 fixture 路径准备失败已查实修正。完整命令、修改范围和结果见 [中文化专项记录](audits/localization-2026-10-13.md) / [验证入口](../tests/localization/README.md)。
- 未验证 native Tauri / WebView2 完整退出重启、实际系统文件窗口 / 屏幕阅读器、真实 AI、Linux / macOS 或全仓套件。未读写真实用户数据库 / 凭据、未修改依赖 / 锁文件、无提交 / 推送 / 发布。同步 STATUS、M4W-015 与 D-007；M4W-015 仍待验证。
- 收尾 Git 核对：另一流程已于 15:39:01 +0800 将此前修复提交为 f732ee70f59a65589e0ef96ac29b289696e945af；中文化仍未提交。本任务涉及的 18 个既有实现 / 资源文件快照与该提交按换行归一一致，保留其提交与记录更新。

## 2026-10-13 — 语言选择移入设置，保留 token 术语

- 当前 HEAD f732ee70f59a65589e0ef96ac29b289696e945af 加保留的全部中文化未提交修改；实际目录 / PowerShell 7.6.5 / Git 根与状态、STATUS / M4W-015 已检查。用户明确要求仅在设置提供语言入口，并保留 token 名称。
- 将 LanguageSelect 从 src/components/layout 移至 src/features/agent/components，在 AgentSettings 顶部显示；Sidebar 移除独立入口，保存偏好和语言同步逻辑不变。zh-Hans 用量相关描述、输入 / 输出 / 总数、覆盖率、缺失及历史说明统一使用小写 token。
- 更新现有生产 App 浏览器回归，从设置执行键盘 / 鼠标切换并断言主界面无语言控件。lint / build、6 组真实 Edge、可访问性 10 / 标签及设置关闭锁 10 组 DOM、359 对资源 / 插值 / 引用、diff 通过；亮暗设置截图已复核，build 保留既有 chunk 提示。详见 tests/localization/README.md 及中文化审查后续记录。
- 无 Rust / 数据 / 依赖变更；未重跑 Rust / 完整前端套件。native Tauri / WebView2、系统窗口、真实 AI 和其他平台仍未验证；保留 M4W-015 待验证状态。未访问用户库 / 凭据，未提交 / 推送 / 发布。

## 2026-10-13 — 修正英文设置 Tag Library 入口换行

- HEAD f732ee70f59a65589e0ef96ac29b289696e945af 加保留的全部中文化未提交修改；核对实际 cwd / PowerShell 7.6.5 / Git 根、status、STATUS、M4W-015。用户反馈英文标签库设置入口换行。
- 真实 Edge 在 1200px 窗口复现 Tag Library 与 Providers 不同 y 坐标。AgentSettings 导航容器改为不换行并允许横向滚动，按钮横向留白从 px-4 收至 px-3；正常宽度全显示，窄窗口仍可访问最后入口。
- 现有生产 App 浏览器回归补中英 1200 / 800 / 600px 的六个标签同排、正常宽度无横向溢出，以及 Tab 到最后标签 / 点击选中验证；7 组通过，英文设置截图已复核。lint / build / diff 通过，build 保留已有 >500kB chunk 提示；结果目录 mercury4win-localization-OpgVC9。详细证据见 tests/localization/README.md 和 docs/audits/localization-2026-10-13.md 后续章节。
- 仅修改 agent_settings.tsx 两处样式、现有浏览器验证及记录；不改文案 / Rust / IPC / 数据 / 依赖。未重跑 Rust / 全套 DOM，native Tauri / WebView2 和其他平台未验收。保留 M4W-015 待验证；未访问用户数据 / 凭据，未提交 / 推送 / 发布。

## 2026-10-13 — 提交中文化与设置布局调整

- 用户明确授权将当前改动提交 Git。父基线 f732ee70f59a65589e0ef96ac29b289696e945af，main 分支；提交范围为这轮中文化、设置内语言选择 / 持久化、token 术语、文摘双语导出、英文导航单行布局及其回归 / 文档，共 33 个文件，不包含构建产物或临时截图。
- 提交前复核当前 diff、新增文件、359 对资源 / 插值 / 引用及 git diff --check；此前实际通过的 lint / build、7 组最新 Edge、7 项文摘 Rust和各专项 DOM / 编译 / 格式结果保留，源码没有新变更，不重复运行整套测试。完整命令和阶段结果见中文化专项记录。
- 同步 STATUS 的提交状态；历史工作日志和审查保留当时未提交事实。仅本地提交，不推送、不建标签或发布。native Tauri / WebView2、真实 AI、其他平台仍未验收，M4W-015 状态不变。

## 2026-10-13 — 合入远端 README 提交并统一版本为 0.4.0

- 基线 main HEAD `12be8ea77a86483d99daa2a9df69a3bcb66eb17e`，工作区干净；Windows / PowerShell 7.6.5、Node 24.12.0、Cargo / Rust 1.97.0 GNU。确认 cwd / Shell / Git 根 / status，读取 AGENTS / STATUS，核对最高本地标签 v0.3.1、各配置和三平台发布工作流。
- 用户要求各处统一为更高版本，采用 0.4.0；合入本地已保存的 origin/main 提交 `5563cad6636015bdca81ae036ecca63108bed2be`（仅将 README v0.2.0 改为 v0.3.1）。README 内容冲突保留本地项目说明，以 v0.4.0 解决，并更新 Windows 安装包文件名示例；不改写历史标签或审查记录。此前 fetch 因本机代理连接失败，未宣称已获取最新远端状态。
- package.json、package-lock.json 根项目、src-tauri/Cargo.toml、Cargo.lock 本项目包、tauri.conf.json 和 src/lib/constants.ts 的 APP_VERSION 全部统一为 0.4.0。锁文件通过 `npm.cmd version 0.4.0 --no-git-tag-version --ignore-scripts --offline` 与 `cargo metadata --offline --manifest-path src-tauri/Cargo.toml --format-version 1` 更新，diff 确认无依赖版本变化。
- 实际通过 `npm.cmd run lint`、`npm.cmd run build`、`cargo check --locked --offline --manifest-path src-tauri/Cargo.toml`、七个版本相关文件一致性 / 严格 UTF-8 无 BOM / `git diff HEAD --check` 检查；build 保留既有 >500kB chunk 提示。初次 PowerShell JSON 检查因锁文件空字符串根键解析失败，改用 ConvertFrom-Json -AsHashtable 后通过，没有修改锁文件结构。
- 本次仅版本元数据和文档变更，没有 Rust 业务源码或依赖变更，未重跑 Rust 单元 / DOM / native 桌面验收；三平台安装包构建与升级安装未验证。完成本地合并提交，不推送、不创建发布标签或发布，不访问用户数据库 / 凭据；既有问题状态保持。

## 2026-10-24 — 检查 GitHub Actions 并推送 main

- 基线 HEAD `0c8258d8fd2e27112bfd4536cfea0c232c3d3c34`，开始时工作区干净、领先 origin/main 5 个提交；Windows / PowerShell 7.6.7、Node 24.12.0、Rust 1.97.0 GNU。按用户要求先检查编译流程，再推送；读取 AGENTS / STATUS，核对实际目录 / Shell / Git 根 / 状态与远端身份。
- 通过 gh 查询 Release 工作流与近期运行。工作流 blob `9a821e053ef187c557bfafac61c627205469a030` 与远端完全一致且已启用；最近 v0.3.1 的 [运行 30750008150](https://github.com/Selvori/mercury4win/actions/runs/30750008150) 三平台构建、上传和发布成功，API 中三份 artifact 均存在。npm ci 使用现有 legacy-peer-deps 配置，Node 22 满足锁定 Vite 的 >=22.12.0 要求，Linux 系统依赖与三平台 bundle 输出路径已静态核对；版本均为 0.4.0。没有修改工作流、依赖或产品源码。
- 本次实际通过 `npm.cmd run lint`、`npm.cmd run build`、`cargo check --locked --offline --manifest-path src-tauri/Cargo.toml` 与初始 diff 检查；build 保留已有 >500kB chunk 提示。默认 git fetch 因失效的本机代理失败，使用单次 `git -c http.proxy= fetch origin` 成功，没有改全局代理配置；远端仍为 5563cad 且已被本地合入。
- 预检通过后，`git -c http.proxy= push origin main` 成功，将远端 5563cad 更新至 0c8258d。随后 `gh workflow run release.yml --repo Selvori/mercury4win --ref main` 启动 [运行 40003947795](https://github.com/Selvori/mercury4win/actions/runs/40003947795)，head SHA 已核对。该工作流仅由 v* 标签或手动触发，main 的手动构建会跳过 release 作业，不创建标签或发布。
- 首次运行 Windows / macOS 构建及上传成功，Linux 在下载系统依赖时耗时较长；为获取日志取消剩余运行，保留该次 cancelled 记录。日志显示 azure.archive.ubuntu.com 下载持续缓慢，最后正在获取 25.5MB WebKit 包，尚未进入源码编译，没有源码编译失败证据。`gh run rerun 40003947795 --repo Selvori/mercury4win --failed` 重跑未完成作业，新 runner 顺利安装系统库，最终 Linux 构建 / 上传成功；第二次尝试的整体 conclusion 为 success，release 为 skipped。无需修改工作流或产品源码。
- 实际日志确认生成 `Mercury_0.4.0_x64_en-US.msi`、`Mercury_0.4.0_x64-setup.exe`、`Mercury_0.4.0_amd64.deb`、`Mercury_0.4.0_aarch64.dmg`。API 核对 mercury-windows / mercury-linux / mercury-macos-arm64 三个 artifact 均存在、非空且未过期，压缩大小分别为 17,638,010 / 12,842,756 / 10,823,884 bytes；没有下载或运行安装包。本条记录所在提交只更新 STATUS / WORKLOG，随 main 同步，产品构建基线仍为 0c8258d。
- 本次未重跑 Rust 单元 / DOM，也未验证安装包升级、native 桌面或真实 AI；产品问题状态保持。源码与验证记录均通过普通 push 同步，不创建标签、不发布、不改远端历史，也未访问用户数据库 / 凭据。

## 2026-10-24 — 正式发布 v0.4.0

- 用户在确认 Releases 尚无新版本后明确授权“是的，发布”。开始基线 main `0d8551f66e7b7fcb21ac277280e526a184d83e9d`、工作区干净；Windows / PowerShell 7.6.7。核对实际目录 / Shell / Git 根 / 状态、STATUS、远端 main、既有版本与构建配置；远端尚无 v0.4.0。`git diff --exit-code 0c8258d HEAD -- .github package.json package-lock.json src src-tauri index.html` 通过，main 相比实际构建基线仅有验证文档差异。
- 重新核对 [运行 40003947795](https://github.com/Selvori/mercury4win/actions/runs/40003947795) 的成功结论、完整 head SHA、三平台作业与三个未过期 artifact，使用 `gh run download` 下载到独立系统临时目录；不将安装包或临时发布说明纳入仓库。复用已验证的产物，没有重新编译或升级依赖。
- 四个安装包为 `Mercury_0.4.0_x64-setup.exe`（7,347,323 bytes）、`Mercury_0.4.0_x64_en-US.msi`（10,559,488）、`Mercury_0.4.0_amd64.deb`（12,935,194）、`Mercury_0.4.0_aarch64.dmg`（10,877,459）。先通过 `gh release create --draft --target 0c8258d8fd2e27112bfd4536cfea0c232c3d3c34 --notes-file ...` 上传，API 对照安装包的 uploaded 状态与大小，全部与本地文件一致。
- `gh release edit v0.4.0 --draft=false --latest` 正式发布 [Release v0.4.0](https://github.com/Selvori/mercury4win/releases/tag/v0.4.0)；API 核对 latest 标签正确、非草稿、非预发布、安装包附件齐全，远端标签 v0.4.0 指向实际构建提交 0c8258d，已 fetch 到本地。发布说明列出改动、四种安装包、构建来源及未验收范围。
- 发布标签额外触发同一提交的 [运行 40022933876](https://github.com/Selvori/mercury4win/actions/runs/40022933876)。核对 head SHA / v0.4.0 / push 事件后请求取消，避免重复构建和重复创建已有 Release；该取消不替代也不否定此前已通过的三平台构建。本次只更新 STATUS / WORKLOG 并同步 main，没有修改工作流或产品源码。
- 未安装或执行附件，未重新运行单元 / DOM / 完整桌面验收；安装、升级与真实 AI 兼容仍未验证，问题台账状态保持；未访问用户数据库 / 凭据。

## 2026-10-24 — 发布页面与项目文档整理

- 基线 main `18218be`，工作区干净；Windows / PowerShell 7.6.7。核对实际目录 / Shell / Git 根 / 状态、STATUS、全部 Release 与项目说明，更新 v0.4.0 发布文案和附件列表；页面保留四个已上传的三平台安装包。
- 统一 STATUS、测试说明、样例登记、专项记录与工作日志中的相关表述，保留原有验证结果、代码基线和未验收范围。项目配置、业务源码、测试实现和固定 OPML 样例没有变动。
- 文本扫描、Release 内容与四个安装包的 API 检查、UTF-8 与 diff 检查通过。此次仅整理文档和发布页面，未重新构建或执行桌面测试；安装 / 升级与实际桌面验收状态保持。
