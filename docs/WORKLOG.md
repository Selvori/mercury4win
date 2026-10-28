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

## 2026-10-24 / 25 — M4W-001 完整 Tauri 文章归属验收

- 代码基线 `4b652fb`，首次工作区干净；Windows / PowerShell 7.6.7，完整 Tauri debug / WebView2 155.0.4283.62。核对实际 cwd / Shell / Git 根 / 状态、AGENTS / STATUS / ISSUES、原串行顺序与 reader_ownership 证据；读取 Computer Use 技能，实际使用 `@oai/sky` 观察及操作原生应用。没有用此前 DOM / mock 代替 native 结果。
- 使用临时 Tauri override 建立独立 identifier `com.mercury4win.acceptance20261024`、空数据库与独立 WebView2 profile，先观察空库证明隔离。真实用户库仅通过 SQLite 只读连接提取必要 AI 配置，原样复制不透明凭据；只种入合成 A/B 文章。未改真实配置、订阅、笔记或历史结果，凭据及端点不进入文档 / 日志。
- native 实际通过 A 草稿→B 保存→A、笔记关闭重开、隔离 SQLite 故障触发器导致保存失败 / 草稿与错误保留、移除触发器后重试正确落库。真实直连摘要成功；真实服务响应经 loopback 透传延迟后，实际通过摘要 / 静默翻译 / 标签 A→B 旧结果隔离及展示关闭重开。翻译 7/7 完整仅归属 A，标签 A 手工输入 / 推荐不污染 B。合计 11 次真实请求 / 4314 token；[逐项步骤、时间及脱敏持久证据](audits/native-acceptance-2026-10-24.md)。
- 本次未修改产品源码；`npm.cmd run build` 及 `npm.cmd run tauri -- build --debug --no-bundle --config tmp/native-acceptance-2026-10-24/tauri.acceptance.json -- --locked --offline` 通过（保留既有 chunk 提示），最终 diff / UTF-8 / 证据 JSON 检查通过。未重跑 Rust / DOM 全套、安装包或其他平台。准备阶段的错误路径、缺少点击几何信息和命令引号 / 控制台编码问题均已核对并恢复，未计为产品失败。
- M4W-001 最低关闭条件满足，更新 ISSUES 为已关闭，STATUS 同步当前验收进度。其余问题按原顺序后续分别验收，本项未扩展到取消 / 队列 / 跨重启完整恢复。隔离故障触发器已删除、路由已恢复直连、延迟代理已停止且 18741 无监听；隔离应用保留在 B 标签面板供下个串行项接手。未提交 / 推送 / 发布。

## 2026-10-25 — M4W-016 native 净化验收及笔记排版修复

- 基线 `4b652fb` 加保留的 001 验收文档；确认 cwd / PowerShell 7.6.7 / Git 根 / 状态，读取 STATUS / ISSUES / 净化 fixtures 和 Computer Use 规范。完整隔离 Tauri / WebView2 实际保存恶意笔记，关闭重开及编辑原文通过；仅 loopback 图片资源副本，原始 fixtures 未变。隔离 SQLite 只读核对两篇输入逐字持久化，用量仍为此前 11 次，无新 AI 请求。
- 普通 Markdown 实际复现标题层级 / 列表标记 / 引用区分缺失及代码溢出：prose 没有对应插件样式。ReaderNote 复用已有 reader-content，仅增加 note-content 断行 / 局部横滚 / 表格 align 约束；未改摘要、净化策略、存储、IPC、Rust 或依赖。
- lint / build、原始安全 jsdom 6 项、既有 reader_ownership 15 项（实际运行进程返回 0）及相同 override 完整 native 重建通过；build 保留已有 chunk 提示。临时 DOM 依赖旧目录不完整，补装固定版本；实际执行安全回归使用 Node 24.19.0，未改项目锁文件。
- 重启复验时 Computer Use 返回用户物理 Escape 停止，立即停止全部 UI 操作，没有重试或绕过。修复后 native 排版、普通样例关闭重开 / 编辑与持久化展示未验收，016 保持待验证，下一步仍继续016，不启动005。[完整报告、失败和脱敏证据](audits/native-016-2026-10-25.md)。
- 本项 loopback 18742 服务已停止且无监听，隔离 mercury 进程未运行，两篇合成笔记及忽略目录临时副本保留。未重跑 Rust / 全套回归，未提交 / 推送 / 发布。

## 2026-10-25 — M4W-016 用户授权恢复与关闭

- 用户明确恢复上次因物理 Escape 停止的 Computer Use。核对实际 cwd / Shell / Git 根 / 状态、最新技能及工具文档，保留 `4b652fb` 上全部未提交修改，只继续已构建隔离 exe 的复验断点；未新增产品源码或重跑已通过的构建、安全 6 项、归属 15 项。
- 实际通过普通笔记修复后的标题 / 列表 / 引用 / 表格左右对齐 / 代码块独立横滚 / 图片 / 安全 HTML 排版、面板关闭重开与完整编辑原文。正常退出确认 mercury 进程数为 0，再完整重启，两篇已保存笔记展示正确，恶意原文仍完整；只读隔离库逐字比对两篇输入相等，用量保持 11 次，无新增 AI 请求。
- 两段 UI 均全部等待调用返回后停止调用并 reset JavaScript 会话，无挂起控制调用；公开 sky API 无 end / release 接口和宿主释放确认，未声称 reset 能保证宿主释放。写记录期间没有持续 UI 调用。用户停止历史及首次排版失败保留，并追加恢复证据到[016 报告 / JSON / UIA](audits/native-016-2026-10-25.md)。
- 016 最低条件满足，ISSUES 标为已关闭、STATUS 下一项005；本阶段未启动005。18742 loopback 图片服务已停止且无监听，隔离应用保留运行在 B 笔记，后续重新列窗，不复用句柄。diff / UTF-8 / 证据 JSON 通过；未提交 / 推送 / 发布，其他平台未验证。

## 2026-10-25 — M4W-005 native Feed 来源与 F-001 验收关闭

- 基线 `4b652fb` 加保留的001 / 016验收文档与笔记样式；Windows / PowerShell7.6.7 / 完整隔离Tauri debug / WebView2。核对cwd / Shell / Git / 状态、项目证据和Computer Use规范；额度中断后用户继续，重读更新版技能及三份规范，只补剩余partial与恢复，不重做已过路径。
- native实际通过短显式Feed原文且不抓文章、teaser抓全文、503 / 连接中断 / 真停服临时导读与来源提示、不写成功缓存；恢复后通过按钮重试读全文并持久化。F-001原生首次11/11添加且11源在线同步成功，重复0添加 / 11跳过，已读 / 收藏 / 合成笔记 / 标签保持。
- 选定11源原生保存导出，独立新identifier空库原生往返11/11、URL集合相同；第三独立库预置3源后F-001原生导入8添加 / 3跳过，11唯一URL、用户状态和外键检查通过。原隔离库没有搬动 / 清空；固定fixture与bootstrap未变，没有AI请求。
- native复现空200响应被标成功，最小修改feed_cmd普通 / 流式导入映射SyncResult.error；新空响应 / 404明确失败、后续正常源继续、最终3/3完成。cargo check无warning、错误判定1项与既有005九项、定向格式 / diff、原 / roundtrip / partial三个override重建通过。直接测试exe首次0xc0000139未进harness，仓库run_rust脚本sidecar后实际通过；失败历史保留。无本项前端改动，未重复lint / build、全套Rust / DOM。
- 完成后正常关partial、原override重建恢复identifier `com.mercury4win.acceptance20261024`与原窗口；A/B笔记长度463 / 2294、F-001十一源及此前11条用量保持。18745无监听 / 无本项服务；最后所有UI调用返回后reset，无挂起调用，未声称宿主释放保证。只写报告期间不继续UI。
- [native报告 / 脱敏事实与UIA](audits/native-005-2026-10-25.md)及tests/feed_content / TESTING同步，最低条件满足，ISSUES关闭005、STATUS下一项008。本任务没有启动008，没有提交 / 推送 / 发布；其他平台和后续专项未验证。

## 2026-10-25 — M4W-008 native 部分验收及控制中止记录

- `4b652fb`加保留的001 / 016 / 005未提交记录、笔记样式与feed_cmd修复；Windows / PowerShell7.6.7 / 完整隔离Tauri / WebView2。核对cwd / Shell / Git / status，读取STATUS / ISSUES / Reader证据及Computer Use完整规范。只在隔离库增加feed10800 / entry10801–10805与scoped写入审计；原A/B笔记长度 / 时间和11条用量保持，无AI请求或日常库 / 凭据访问。
- 无summary文章实际503后native原因 / 重试按钮可见、失败不写成功正文 / 缓存；恢复服务并点击按钮全文恢复。302最终URL + HTML相对base持久正确，WebView2实际请求并显示相对图片。正文普通链接点击后系统Edge请求正确目标并获200；Reader仍about:srcdoc。危险JS / file / data链接变普通文字且渲染HTML无这些href或script，未补点击；页内锚点被展开为绝对URL的语义待核对。
- 读取Edge目标窗口状态时，Computer Use返回“could not determine the current browser URL on Windows with enough confidence to enforce policy”，终止本turn；精确全文保留于[报告](audits/native-008-2026-10-25.md)及[脱敏JSON / UIA](audits/native-008-2026-10-25-evidence.json)。没有用户物理Escape报告，不推断全部Windows或非浏览器控制不可用。立即停止全部CU，公开js_reset完成，无挂起调用或绕过；不宣称宿主释放保证。后续明确安排仅shell整理已有证据。
- source1/1、Markdown3/1、render3/2/4三篇仍未native点击，分层重建 / 缓存命中 / 外观缓存 / 重启复用不足；目标页截图 / UIA未取得，008保持待验证，不启动006。无本项产品源码变化、不机械重跑历史Rust8项 / DOM / 构建；证据JSON、UTF-8、文档链接与diff检查收尾。
- 核对PID10992命令行为008_fixture.py serve后停止，18748无监听。隔离应用保留、5篇样例 / scoped审计和忽略目录原始日志保留供续验；正式报告明确成功 / 未执行 / 工具中止范围，ISSUES / STATUS / tests/reader_pipeline同步。不提交 / 推送 / 发布。

## 2026-10-26 — M4W-021 设置导航纵向滚动与裁切修复

- 核对实际 cwd / PowerShell / Git 根与状态，读取项目状态、问题台账与已有导航回归；保留 `4b652fb` 上全部原有未提交改动。
- 真实 Edge 156.0.4314.42 在生产组件中文任务配置页复现：导航高度 37px，内容 44px，按钮被裁切。AgentSettings 仅增加语言栏 / 导航栏禁止收缩及导航禁止纵向滚动，长内容继续独立滚动；共享 Dialog / Rust / 依赖未改。
- 扩展既有 localization runner，实际通过中英文六个设置页 × 三种尺寸共 36 个布局场景、长内容滚动及既有键盘 / 窄窗口验证，完整 runner 8 组通过。测试 mock 补默认 Prompt 的 null 返回；初次复现失败和修复后结果保留，命令与临时证据见 [回归记录](../tests/localization/README.md#设置顶部导航高度修复)。lint / build 通过，保留已有 chunk 提示。
- 新增 M4W-021 待验证并同步当前状态；完整 Tauri / WebView2、其他平台和真实非空配置未验收，未重跑 Rust / 全仓套件。无用户数据库 / 凭据访问，未提交 / 推送 / 发布。

## 2026-10-26 — M4W-022 分段翻译保持阅读位置

- 确认实际 cwd / PowerShell 7.6.7 / Git 根 / 状态并读取 STATUS、ISSUES 与相关证据；基线 `4b652fb`，保留任务开始前所有未提交改动。只读 HEAD ReaderPane 注入测试 bundle，进入翻译后重新滚到正文中段，视口下方一段完成即复现 scrollTop 10148→0。
- ReaderPane 的同文章 / 同 Reader HTML 译文改为原文档内更新，新增 update_bilingual_document 验证结构 / 直接文字并按 source ID 增删或更新译文，使用可见块偏移补偿滚动。保留原文 DOM、图片、选区和焦点；表格 / 定义列表合法插入，仍用 textContent，文章切换按 ID 重建 iframe，sandbox / IPC / Rust 不变。
- 新增 tests/translation_scroll 的生产组件真实 Edge 14 组通过：上下方 / 当前段、乱序 / 同次多段、前段更新、阅读译文与选择 / 焦点、缓存、原文、窄窗口和 T；文档及原文节点身份保持，视口偏移误差不超过 1 CSS px。原字体滚轮 Edge 8 组及翻译10 / 主题24 / 归属15 / Reader2 / 可访问性10，共61组 DOM 通过。Rust fixture 准备实际运行主题3项和翻译分段7项，均通过；lint无警告、build与diff通过，已有chunk提示保留。
- 三个旧 DOM 脚手架读取静态 srcdoc 导致翻译 / 主题 / 归属初次断言失败，改读实时 contentDocument 后保留全部业务断言并通过；新增测试组件的 lint Fast Refresh 警告已消除。命令、历史失败、数值证据和边界见[专项记录](../tests/translation_scroll/README.md)。
- ISSUES新增022待验证、STATUS同步，完整Tauri / WebView2实际Channel、真实AI和其他平台未验收，不改变原008串行验收断点；未重跑全仓Rust / 完整套件，未访问用户库或凭据，未提交 / 推送 / 发布。

## 2026-10-26 — M4W-023 文章工具栏标题自适应

- 基线 `4b652fb` 加开始任务前全部未提交改动；核对 cwd / PowerShell 7.6.7 / Git 根 / 状态，保留 ReaderPane 现有翻译增量及其他修改。
- ReaderPane 标题容器使用 min-w-0 / flex-1，移除 max-w-md，继续单行省略并补完整标题 tooltip；主阅读区允许收缩，工具栏 / 标签组固定自身尺寸。只改变布局，无 Rust / IPC / 依赖修改。
- npm.cmd run lint、npm.cmd run build、git diff --check 通过，构建保留已有 chunk 提示。实际 Edge 156.0.4314.42 中英文 × 五种阅读区宽度十个场景按钮完整且可命中，窄时省略 / 宽时完整；800px 打开笔记主区 474px 无溢出。设置 / 文摘窗口与 mock 浏览器入口通过，截图已查看。临时文摘 mock 首次遗漏 missing 数组导致打开断言超时，补齐后通过，无产品代码变更。
- 证据与命令见 [布局验证](audits/reader-toolbar-2026-10-26.md)，新增023待验证并同步STATUS。完整 Tauri / WebView2、系统浏览器 / 文件导出与其他平台未验收，未重跑 Rust 或全套回归；未访问用户数据 / 凭据，未提交 / 推送 / 发布。

## 2026-10-26 — 原版对照与产品创新机会评估

- 核对 cwd / PowerShell 7.6.7 / Git 根 / 远程与状态，保留全部既有未提交改动；当前基线 `4b652fb` 加工作区，上游只读 `5b2e58e` / v1.4.32。
- 阅读 STATUS、ISSUES、既有审查和 native 008 证据，对照两项目搜索、同步、凭据、相关文章、文摘及上游功能文档；在线辅助核对官方 Mercury README、SQLite FTS5 / Backup 文档。
- 新增 [产品机会评估](audits/product-opportunities-2026-10-26.md)，区分已有能力、待验收实现与候选功能；建议以全文检索、主题对照 / 增量简报、段落证据卡构成演进主线，并列出实现边界与验收方法。所有方案为建议，未写入已接受决策，也未更改原问题状态或 native 验收顺序。
- 本次只新增评估文档并追加日志；UTF-8、文档相对链接与 diff 检查通过。未运行构建、产品测试、性能基准或桌面操作；未修改产品源码、依赖或用户数据，未提交 / 推送。
- 初次附加的替换字符扫描命中日志第98行；核对 HEAD 后确认是既有 SSE 乱码复现记录的原文，并非本次编码损坏，保留该证据。新报告与本次新增日志无替换字符；报告11个相对链接实际存在。

## 2026-10-26 — 全文搜索与智能视图

- 基线 `4b652fb` 加任务开始前全部未提交改动；核对 cwd / PowerShell 7.6.7 / Git 根 / 状态，UTF-8 读写，保留原 Feed、Reader、翻译、设置、样式和验收工作。用户明确选择新功能，旧 native 顺序不作为前置，未继续旧问题逐项验收。
- 实现 Schema 6：FTS5 / 短词字面查询、可恢复索引 / 持久更新队列、正文和笔记回填 / 更新 / 隐藏删除、参数化组合条件、Unicode 安全片段、相关性 / 时区时间游标分页、SQLite 命名视图 CRUD / 动态相对日期。同步命令注册、TS DTO / IPC、侧边栏、搜索 / 筛选 / 保存界面与中英 64 个文案键。保持普通搜索 / 未读导航、文章 / 标签 / 收藏 / 批量 / 文摘行为；笔记和正文写入失效搜索。
- 修正本轮回归发现的 dirty 触发器与外层 UPSERT 冲突，补真实 UPSERT / REPLACE 测试；旧用量迁移测试改为完整历史 V4 fixture，未修改 AI 逻辑。UI 补明确标签、未保存 / 校验 / 错误反馈、暗色文字和展开筛选后的结果空间，避免缓存命中 Reader 无条件重查列表干扰 j/k；最终复核补齐已保存视图内的当前范围恢复，并补先全库再恢复的作用域回归。
- lint / build、cargo check --locked --offline、全仓 fmt 检查通过；build 保留已有 chunk 提示。11 项新增功能回归、显式 fixture / 性能通过；最新全库 library 为146通过 / 4失败 / 1忽略。四个 AI socket 取消超时单独运行仍失败，并在原样 git archive 的 `4b652fb` 相同环境重现，未扩展修复，不声称全套通过。
- 新增生产浏览器9组、16种中英 / 主题 / 窗口 / 列表宽度布局通过，实际查看截图；既有分页6 / 筛选6 / 文摘7 / 可访问性10 / 标签10 / 归属15，共54组DOM通过。浏览器使用真实 Rust DTO，仅替换 native transport，未代替桌面证据。
- 独立 identifier / 全新合成 V5 库，完整 Tauri debug 构建与 Windows WebView2 实操通过升级、四字段中文命中 / 安全标题 / 笔记跳转、创建 / 完整退出重启、笔记保存后活动结果4→3、七天Rust未读2条、视图名称 / 条件更新和再次重启、实际重建按钮。恢复任务时重新确认窗口与持久状态后补完重建，未把此前失败点击当通过。最终作用域修正构建补验2→全库3→恢复Rust2通过；最终140篇 / 140索引 / dirty0 / FK0 / Provider0 / 用量0，应用已退出、JS reset，无本项服务或挂起输入。
- 合成内存库1万篇、每篇约1.2KB，初建2.130s；「方案」88.15ms、代码词37.83ms、全命中reader118.71ms、未命中0.46ms，均包含计数 / 排序 / 首100及片段。没有宣称磁盘大库或相对旧搜索的倍数加速。
- [专项报告与证据](audits/search-views-2026-10-26.md)、[使用 / 复现](../tests/search_views/README.md)；STATUS及D-008同步，旧问题台账状态保持。Linux / macOS、安装升级及更大磁盘库未测；native未逐种重复浏览器全部矩阵。未访问日常用户库 / 凭据，未改依赖 / 固定OPML / bootstrap，未提交、推送或发布。

## 2026-10-26 — 全文搜索与智能视图交付复核

- 用户要求检查完成情况；核对 cwd / PowerShell 7.6.7 / Git 根与工作区，保留全部改动。聊天状态接口首次请求失败，改用读取聊天确认「实现全文搜索与智能视图」已 idle、最近执行 completed；核对当前源码的搜索、视图持久化、IPC 注册及专项报告 / 脱敏证据。
- 本轮独立重跑 npm.cmd run lint / build、cargo check --locked --offline、cargo fmt --check，均通过，build 保留已有大 chunk 提示。运行 tests/reader_pipeline/run_rust.ps1 db::search_store::tests：12 passed / 0 failed / 1 ignored，其中11项业务回归实际通过，fixture入口无环境变量直接返回，性能项默认忽略；本轮未重新生成 fixture 或测性能。
- 已有 Windows native 主要路径与9组浏览器 / 54组DOM结果属于执行对话的验证，本轮只检查记录、未重做UI。完整library的4项既有AI取消超时及原基线复现摘录保留，不声称全套通过；Linux / macOS、安装升级与大型磁盘库仍未验证。结论为新功能第一版已实现并有Windows主要路径证据，改动尚未提交 / 发布；未扩展修复旧问题。

## 2026-10-26 — 搜索开关状态与智能视图设置统一

- 基线为 `4b652fb` 加上述全部未提交改动；再次核对 cwd / PowerShell 7.6.7 / Git 根 / 状态并读 STATUS，保留既有功能和验证成果。
- AppShell 独立维护搜索选中状态；Sidebar 采用现有导航的 accent 背景、字重及 aria-pressed / expanded，取消时清空当前选择并恢复中间空状态。普通来源 / 标签 / 收藏 / 智能视图导航同步取消搜索选中，避免「全部订阅源」与搜索双重高亮；可用 Space / Enter 操作。
- 智能视图弹窗复用 DialogContent / Header / Title / Footer，统一24px内容留白、字号、表单和底部按钮区；重命名 / 另存为辅助分组，删除独立呈现，主操作与取消保留在页脚。筛选下拉框同步 Input 的高度、圆角、边框与焦点样式；新增中英2个说明键。
- lint无警告、build通过（已有大chunk提示）；浏览器11组通过，覆盖搜索开关 / 导航清除、视图原有CRUD、四种中英亮暗弹窗截图、16种列表布局和原有业务断言；截图实际查看。既有组合筛选6组、可访问性10组通过。完整隔离Tauri debug构建通过，真实Windows / WebView2点击搜索选中、再次点击关闭中间视图、切换已保存视图、新弹窗排版和Escape关闭通过；未保存或删除任何隔离内容，结束正常关闭并reset JS。
- [本次证据与边界](audits/search-views-2026-10-26.md#搜索开关与设置界面后续调整)。本项无Rust / Schema / IPC逻辑变更，未重跑Rust；native中英亮暗全矩阵及其他平台未重复。未访问日常用户库 / 凭据，未提交或推送。

## 2026-10-26 — 订阅浏览不再自动展示全文搜索

- 基线 `4b652fb` 加全部保留的未提交改动；核对cwd / PowerShell7.6.7 / Git根 / 状态并读STATUS，UTF-8读写。
- AppShell明确搜索模式，EntryList默认普通浏览：直接点击订阅源、标签或收藏不挂载完整搜索表单，保留未读 / 标签模式和文章操作；查询也去除隐藏的关键词、日期、笔记等高级条件，避免只隐藏UI却继续过滤。智能视图默认紧凑名称栏，主动点击「编辑筛选」才展开完整条件，管理入口保持；新增中英1键。
- lint / build、11组Edge浏览器和既有筛选6 / 分页6 / 文摘7 / 可访问性10，共29组DOM通过，保留既有chunk提示。筛选脚手架按显式搜索入口调整；非活动标签选项不再立即读取，改核对真实缓存失效后通过。新增浏览器核对有关键词 / 组合智能视图切到普通源的IPC不含隐藏条件，截图已查看。
- 完整隔离Tauri debug构建通过，Windows / WebView2实际验证A直接137篇无搜索区、搜索无匹配词0篇后转B正常3篇、智能视图紧凑2篇及主动编辑恢复表单。未更改隔离内容，正常关闭 / 确认进程退出 / reset JS。[证据与命令](audits/search-views-2026-10-26.md#普通浏览与全文搜索分离)。STATUS和测试说明同步；无Rust / Schema / IPC协议变化，未重跑Rust，其他平台未验证，未访问用户库 / 凭据，未提交或推送。

## 2026-10-26 — 主题追踪与增量简报及收尾

- 基线 `4b652fb` 加全部保留的既有未提交改动，含全文搜索／智能视图。核对 cwd、PowerShell 7.6.7、Git 根和状态，默认 UTF-8；未改上游、依赖、固定 OPML、日常数据库或凭据，未提交／推送／发布。
- 增加 Schema 7 专题／成员／运行／简报版本、独立 topic 任务和用量、冻结规则与资料快照、首次跨来源对照及成功基线增量、引用结构／原文定位校验、历史回看、独立用户判断和 Markdown 导出。接通双语专题入口、候选确认、列表多选加入、归组、状态／重试／取消及主题／窄窗口布局。
- lint／build、Rust check／格式通过；专题 12、迁移 4、运行时 8 项通过；搜索模块 12 通过／1 忽略，专题浏览器 10 组含 12 种布局、搜索浏览器 11 组、既有文摘／分页／筛选 19 组 DOM 通过。完整 library 为 158 通过、4 项已在原基线复现的 socket 取消超时失败、1 忽略，没有将失败算作通过。
- 完整隔离 Tauri debug 构建及 Windows／WebView2 主要路径通过：无 AI 管理、规则副本、首次／增量、非法引用拒绝、取消／重试、旧正文与当前正文各自映射、原生 Markdown 导出、重启及个人判断保留；V6→V7 保留文章／笔记／视图／收藏。4 个请求均为 loopback 受控响应，另预置中断运行验证启动恢复；未评估真实模型质量。
- 验收中修正状态漏翻译、证据组辨识、时间显示及测试 fixture／取消后内存连接失真。两次中断历史保留；用户恢复授权后补完主要路径，最终按用户要求仅完成报告、状态／决策／日志、必要的 UTF-8／链接／diff 与进程检查，不重复构建、测试或额外 native 验收。
- 收尾未发现隔离应用、Provider 进程或 18756 监听，Computer Use JS 已重置；合成库、导出和原始日志保留在忽略的 tmp 目录。旧问题状态不变。[专项报告和未验证边界](audits/topics-2026-10-26.md)、[数据库／请求证据](audits/topics-2026-10-26-evidence.json)、[使用与复验](../tests/topics/README.md)。

## 2026-10-26 — 本地提交已实现与验收成果

- 用户明确授权提交，保存当前整合的主题追踪／增量简报、全文搜索／智能视图、此前保留的 Feed／Reader／翻译／设置修复和对应测试、验收及项目记录。保留历史验证时的未提交基线说明，STATUS 同步当前提交状态。
- 本轮仅核对 Git 差异、暂存文件范围和提交结果，不重复运行构建、测试或桌面验收；排除忽略的依赖、构建产物与隔离数据库。既有 4 项 socket 取消超时和真实模型质量等未验证边界继续以专项报告为准。
- 仅本地 Git 提交，不推送、不创建标签或发布。

## 2026-10-26 — 主题追踪下一版候选设计

- 用户咨询下一版方向，未要求启动实施。核对 cwd / PowerShell / Git 根与状态：本地基线 b93753a，开始时工作区干净；执行对话已结束。读取 STATUS、专题报告、测试说明及当前 topic 存储 / 生成实现，确认现有资料确认、成功基线、引用快照、历史和导出能力。
- 建议下一版围绕“持续研究几个明确问题”：增加结构化关注问题、带入选理由及近似重复分组的专题收件箱、跨版本判断演变与新旧证据对照、先展示重要变化的简报，以及可纠正的人工反馈。优先做收件箱、关注问题与变化卡片；定时生成、跨专题日报和通用问答后置，不创建任何自动化。
- 配套建议为正文版本级提取缓存、按模型容量选择相关原文上下文、专题独立模型设置及预算；不能只把新文章交给模型而丢失旧证据，也不能把所有新增文章当成新信息。下一版质量验收应加入真实模型在固定合成 / 授权资料上的证据支持、新颖性、误合并和遗漏检查；本轮不调用模型或凭据。
- 只作为候选设计，未写入已接受决策。参考并实际打开 Feedly 官方去重说明（https://docs.feedly.com/article/218-how-does-deduplication-work）和 Readwise 官方 Global Ghostreader 说明（https://docs.readwise.io/reader/guides/ghostreader/global），用于区分行业已有去重 / 带引用问答与本项目拟加强的持续变化追踪，不声称行业首创。
- 本轮仅追加本日志，未改产品代码、未启动新对话或修改已有任务。UTF-8 与日志 diff 检查通过；未运行构建、测试或桌面验证，现有验证边界保持。

## 2026-10-26 — 自动选文与信息简报

- 基线 b93753a，开始时仅有上方候选设计日志修改，完整保留。用户随后明确以“一句话主题、AI 自动选文、信息简报、一键更新”为主线并要求实施；研究问题 / 证据裁决 / 逐篇审批不再作为必经流程，记录为 D-010。未修改上游、依赖、固定 OPML、bootstrap、日常数据库或凭据，未提交 / 推送 / 发布或创建自动化。
- 在现有专题系统增加 Schema 8 自动 / 手动模式、资料检查版本、反馈、筛选缓存、检查时间和已读位置。新主题默认自动，旧主题保持手动。接通有界发现、真实 AI 筛选再总结、信息栏目、卡片生成 / 阅读、来源调整、缺全文依据、预算延后、无信息检查、缓存重试与取消围栏；历史、原文快照、独立备注、手动管理和导出保留。
- 最终 lint / build、590 双语键、Rust check / 格式及独立 Tauri debug 构建通过。全 library 170 通过、4 项既有 socket 取消超时失败、1 忽略；专题 24 个入口通过，含 3 个可选 fixture 入口，本轮显式导出自动模式生产 DTO。自动浏览器 11 组（12 个简报 / 6 个设置布局）、旧专题 10 组、搜索 / 智能视图 11 组通过；受控 HTTP 验证生产筛选和总结及实际请求数，不代表真实模型质量。
- 回归中修正高级设置保存按钮超出弹窗、已删除资料被恢复为元数据候选、旧历史哈希兼容、超预算资料错误消耗、跨期修订缺少最近原文快照，以及 Channel 无信息阶段缺失时的展示。固定选文质量评估样例已准备，未虚报准确率、语义支持率或费用。
- 曾准备独立应用和 7 篇合成库并启动返回窗口；用户随后要求继续但暂不使用 Computer Use，其后未再使用或绕过桌面控制约束，已按确切路径终止该隔离进程。新版桌面创建 / 生成 / 更新 / 导出 / 重启及真实 V7→V8 启动升级尚未完成，其他平台、大型磁盘库、极长历史及真实 Provider 质量 / 成本未验证。旧问题台账状态不变。
- [专项报告与失败证据](audits/topics-auto-2026-10-26.md)、[使用和复验](../tests/topics/README.md)。原始日志、DTO、构建与合成库保留在忽略的 tmp/topics-auto-native-20261026/，不纳入源码。

## 2026-10-27 — AI 选文 JSON 容器兼容

- 用户报告生成简报时 `Invalid AI selection JSON: invalid type: map, expected a sequence at line 2 column 2`。核对 cwd / PowerShell 7.6.7 / Git 根 / 状态，保留 b93753a 后整轮自动简报等未提交修改；UTF-8 读写，未读取实际模型回复、日常数据库或凭据。M4W-024 登记为待验证。
- 新合成文章数组测试先在旧解析器实际失败，再在修复后通过。仅规范化明确的顶层数组、单篇对象、selected 单对象 / ID 键文章映射及完整 JSON 围栏；保留所有候选身份、字段与限额校验，拒绝不明确结构或相互矛盾的 ID / 无信息标志。加强请求格式提示，规范化缓存保持标准结构，不增加自动纠错 AI 请求。
- `./tests/reader_pipeline/run_rust.ps1 db::topic_`：27 个入口全部通过，含 3 个本轮未启用的可选 fixture 导出入口；新增 3 项覆盖兼容 / 拒绝结构及真实 loopback HTTP 选文后总结、简报保存、缓存和无变化零请求。`cargo check --locked --offline --manifest-path src-tauri/Cargo.toml` / `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 通过。未改前端或数据库迁移，未重跑前端、全 library 或 native 点击。
- 开发应用在 Rust 修改后已重编译并以新进程启动，未擅自终止或操作其界面；继续遵守暂停 Computer Use 的指令。真实 Provider 重试待用户路径确认，没有消费真实额度。未提交、推送或发布。[问题与边界](ISSUES.md#m4w-024-修复及验证记录)、[复验入口](../tests/topics/README.md#2026-10-27-选文-json-容器兼容)。

## 2026-10-27 — 冻结段落引用与桌面恢复验证

- 用户报告 `Quote not found in article 585`，随后明确继续并允许 Computer Use。核对 cwd / Shell / Git 根和状态，保留所有既有未提交改动；未读取用户原文、日常数据库或凭据，未调用真实 Provider。M4W-025 待验证。
- 自动简报改为完整冻结文本片段目录和 passage_id，程序恢复准确原文 / 字节位置，避免模型必须逐字抄写。旧 quote 仅兼容唯一空白归一，仍拒绝编造、改写、重复 / 重叠歧义、非法 ID 与超限；旧存储、历史及导出契约不变。总结 Prompt 记为 topic-information-v2，选文缓存继续复用。
- 专题 32 项入口通过，含 3 个未启用 fixture 入口，新增 5 项覆盖 UTF-8 / 空白映射、拒绝边界、完整段落 / 重复文本编号定位及生产 HTTP 失败后缓存重试。cargo check / 格式和隔离 / 正常配置完整 Tauri debug 构建通过。未改前端或迁移，未重跑前端、完整 library 或把旧 4 项 socket 超时算通过。
- 通过 Computer Use 的公开 sky API 在独立 citations20261027 合成应用实际完成：一句话创建并生成 → 编造引文失败（0 简报 / 0 checked）→ 编号引用重试成功 → 中文高亮与 Escape 焦点 → 已读 → 正常退出重启 / 卡片阅读 / 原引用恢复。共 selection / summary / summary 三次 loopback HTTP，无远程模型；3 个保存引用都与冻结 UTF-8 原文一致。首次无截图点击遇到 geometry unavailable，重新观察后继续，未将首次失败记录为通过。
- 验收应用正常退出，Provider 仅在核对确切 PID / 命令后停止；默认应用标识的 debug 构建已恢复。用户原文章和真实模型重试、其他平台、完整增量 / 导出等边界保持未验证；未提交、推送或发布。[报告](audits/citations-2026-10-27.md)、[合成证据](audits/citations-2026-10-27-evidence.json)。

## 2026-10-27 — 连续文章简报与文末来源

- 用户明确要求简报从逐条小卡片改为连贯文章，证据仅放文末，不显示生成模型名称。核对 cwd / Shell / Git 根与状态，保留此前所有未提交改动，记录 D-011；未改迁移、依赖或凭据，未提交 / 推送 / 发布。
- 新增 TopicBriefArticle，以连续正文段落展示旧 / 新简报，按来源与冻结片段去重收集文末引用；移除阅读界面模型路由标识，保留内部归属和用量。切换页签 / 历史版本恢复正文开头；来源管理、原文弹窗、个人备注及反馈保留。
- 自动 topic-article-v3 与手动 topic-comparison-v2 提示要求连接段落而非独立卡片；Markdown 同步为先正文后来源与原文片段，移除生成模型 / Provider 信息，保留版本 / 基线和独立备注。既有历史不自动改写或调用 AI。
- lint / build、594 双语资源、cargo check / 格式及专题 33 个入口通过（含 3 个未启用 fixture 入口）。新增文章式导出回归，自动浏览器 13 组 / 原专题 10 组通过，含正文无卡片 / 内联证据、文末去重、无模型标识及 12 个阅读 / 6 个高级设置布局。早期导出基线元数据断言失败后将版本 / 基线移到文末保留；重复标题造成的定位失败通过移除冗余文章标题修正，没有把早期失败算通过。
- 在当前 Windows Tauri dev / WebView2 仅阅读已保存简报，实际核对连续长文、文末来源、旧原文快照高亮及 Escape 焦点返回，未点击生成 / 更新或触发真实 AI，未保存用户正文至仓库。阅读窗口保留在文章开头；没有停止用户应用或覆盖其数据库。未重跑全 library / 其他平台，也未评估真实模型在新写作提示下的质量；既有 4 项 socket 取消超时基线保持。

## 2026-10-27 — 自动选文漏选与无信息恢复

- 基线 b93753a 加所有此前未提交修改，Windows / PowerShell 7.6.7；核对 cwd / Git 根 / 状态，UTF-8 操作并保留既有工作。用户确认症状为“暂无值得补充的新信息”，登记 M4W-026 待验证。
- 旧代码先实际复现两项失败：主题说明对应的旧文被最近 1,000 篇窗口排除；首批空选后不再检查后续相关候选。发现现在结合名称 / 说明 / 中文短片段 / 常用安全领域中英别名，保留最近窗口并从原规则允许的全范围补充有界词法匹配；预览含开头与后部相关片段，近期候选交错入队。原始正文和成功快照不重写。
- 一次点击空选最多继续 3 批，共用一个运行和取消围栏；有剩余候选明确提示继续，而非整体无信息。增加显式“重新检查选文”及兼容旧调用的可选 recheck IPC 参数，仅重新打开未用于成功简报的版本并清选文缓存，保留反馈 / 历史 / 已读。前批已确认的空选可保留，失败批待处理；后批失败复用其有效筛选只重发总结。
- 最终 ./tests/reader_pipeline/run_rust.ps1 db::topic_：41 个入口通过，含 3 个本轮未启用的可选 fixture 入口；新增 8 项覆盖旧文 / 说明 / 中英安全主题、中文短片段与 Feed 限制、UTF-8 后部预览、跨批生成、三批上限、显式重查隔离及后批失败缓存重试。旧窗口测试预期按新覆盖范围更新；早期新测试在旧实现实际失败，第一次修复后旧窗口断言及错误的“成功后无成员”断言失败，修正语义后最终通过。
- lint / build（保留既有 chunk 提示）、596 对双语资源、cargo check --locked --offline / 全仓格式通过；自动浏览器 14 组、旧专题 10 组通过。浏览器新验显式 recheck 参数、无信息后可重查、候选未耗尽提示与按钮；保留语言 / 主题 / 窄窗口和取消等回归。
- 按已恢复的 Computer Use 授权，使用技能公开 sky API 只读打开当前 Windows Tauri / WebView2 主题，实际核对尚有候选时的新提示及可用生成按钮，没有点击生成 / 重查。首次无截图的索引点击报告 geometry unavailable，重新观察截图后用坐标导航成功，未将失败算通过。没有调用真实模型、直接读取 / 覆盖日常库或凭据、保存用户正文、停止用户应用、提交 / 推送 / 发布。真实 Provider 选文 / 写作、完整原生重查 IPC、其他平台及大型库性能未验证；未重跑完整 library，既有 4 项 socket 超时基线不变。
## 2026-10-27 — 移除 token 预算与重复段落字段兼容

- 基线 b93753a 加所有此前未提交修改，Windows / PowerShell 7.6.7，核对工作目录 / Shell / Git 根 / 状态并保留既有工作；用户明确移除 token 预算，并报告 duplicate field passage_id。D-012 记录预算决策，M4W-027 待验证；未取得用户实际模型响应。
- 删除预算表单 / 保存参数发送与范围校验、自动选文输入预算缩批、完整正文延后，以及自动 / 手动固定输出 max_tokens；手动正文 / 上下文字符总预算同步移除。旧数据库列 / DTO / 旧 IPC 参数与缓存身份保留兼容，不再约束请求。资料数量 / 引用定位校验保留，未改 Schema 或依赖。
- 旧解析器的新合成测试实际复现 duplicate field passage_id，行列随输入变化。改为引用级 visitor 保留字段序列；同编号去重，多有效编号展开成同一主张的独立冻结引用。非法编号、混合 null / quote、错误类型、其他重复字段、未知字段和超引用数量仍拒绝，不使用末值覆盖。写作 Prompt 升 topic-article-v4，输出结构和历史保存不变。
- 最终 ./tests/reader_pipeline/run_rust.ps1 db::topic_：45 个入口通过（3 个本轮未启用可选 fixture），新增 4 项覆盖重复编号 / 多编号准确 UTF-8 切片、拒绝边界、生产 HTTP 非法重复编号失败后复用选文只重发总结，以及 >160,000 字符手动上下文完整发送。原预算测试更新为 legacy 小值无影响、长文完整发送 / 不延后 / 不带 max_tokens；首次重复字段回归在旧代码实际失败，修复后通过。
- npm.cmd run lint / build（既有 chunk 提示）、593 对双语资源、cargo check --locked --offline / 全仓格式通过；自动浏览器 14 组 / 旧专题 10 组通过。新界面断言覆盖中英文 / 6 个高级设置尺寸中无预算控件、名称必填校验和保存不发送 inputBudget。
- 使用 Computer Use 技能公开 sky API 在当前 Windows Tauri / WebView2 打开未保存的新主题弹窗、展开高级设置，实际确认预算项已移除，Escape 取消并返回主题列表。未输入 / 保存 / 生成，未调用真实 Provider、直接读取 / 覆盖日常库或凭据、保存用户正文、停止用户应用、提交 / 推送 / 发布。真实 Provider 的原回复重试、其他平台和大型上下文服务端兼容未验证；全 library 未重跑，旧 socket 超时失败保持。
## 2026-10-27 — 持续生成进度与长等待反馈

- 基线 b93753a 加全部既有未提交修改，Windows / PowerShell 7.6.7；核对 cwd / Shell / Git 根 / 状态，UTF-8 操作并保留工作。用户要求生成时展示进度以区别于卡住。本轮不改 Schema、依赖、预算或此前问题状态。
- 新增 topic_generation_progress.tsx，自动四阶段 / 手动三阶段，显示当前批次 / 候选数 / 采用数 / 仅标题摘要资料数 / 缓存复用，以及每秒更新的总用时与阶段用时。等待 20 秒增加服务响应提示，不伪造完成百分比、逐篇处理量或预计时间；动画支持减少动态效果，计时不每秒播报给读屏软件。
- topic_auto.rs 通过原 Channel 增加 batch、selection 事实及 saving 阶段；topic.rs 为手动模式补来源数和保存阶段。没有为了进度增发 AI 请求或改变生成策略。topic_workspace.tsx 归属校验后消费事件，隐藏生成期间重复的静态范围说明，保持窄窗取消可达；嵌套 finally 保证刷新失败仍释放 active / busy。组件卸载清计时器，迟到事件不重新显示进度。
- 最终 npm.cmd run lint / build（既有 chunk 提示）、608 对双语资源、cargo check --locked --offline / 全仓格式通过；./tests/reader_pipeline/run_rust.ps1 db::topic_：46 个入口通过（含 3 个未启用 fixture）。新增实际 HTTP / Tauri Channel 回归核对真实数量、非法引用失败不发保存阶段、缓存重试按顺序发阶段且仍共 3 次请求；既有三批测试补核对批次 1 / 2 / 3。
- 自动浏览器 16 组 / 旧专题 10 组通过。新验实际等待 20 秒后的反馈、每秒计时、阶段 / 批次 / 缓存变化、4 个中英宽窄进度布局、成功 / 取消 / 卸载 / 迟到消息清理、刷新失败不会卡 busy，以及旧手动三阶段。最终截图 mercury4win-auto-topics-F1lrYA / mercury4win-topics-GS4fI9（系统临时目录）；实际查看最终中文窄窗截图，卡片和长等待提示完整，取消可见。首次截图发现矮窗口提示裁切后隐藏重复静态统计并允许进度区收缩 / 内滚动修正。
- 首次新增 Rust 测试误把 Channel::new 回调的 InvokeResponseBody 当字符串，编译失败；按实际双层序列化解包后最终通过，没有把首次失败算通过。未重跑全 library / 完整原生生成 / 真实 Provider / 其他平台，没有使用日常凭据、直接读取或覆盖日常库、保存用户正文、提交 / 推送 / 发布；既有 4 项 socket 超时基线和问题台账保持。

## 2026-10-27 — 提交当前自动简报改动

- 用户明确要求把目前改动提交 Git；核对实际目录、PowerShell 7.6.7、Git 根、main 分支和 b93753a 基线，保留全部既有工作。
- 本地提交自动选文 / Schema 8、文章式简报与文末证据、引用和 JSON 兼容修复、预算移除、持续生成进度，以及对应测试、决策、问题和审计记录；STATUS 同步提交状态，不改写历史验证结论。
- 本轮仅检查差异、UTF-8、暂存范围和提交结果，沿用最近通过的 lint / build、Rust check / 格式、专题 46 个入口及自动 16 / 手动 10 组浏览器结果，不重复构建或测试。全 library 既有 4 项 socket 取消超时和真实 Provider / 完整原生验收边界保持。
- 仅本地 Git 提交，不推送、不创建标签或发布；忽略的依赖、构建产物和临时数据库不纳入。

## 2026-10-27 — 全文搜索与智能视图第二阶段建议

- 用户咨询下一步方案，未授权启动实现。核对 cwd / PowerShell 7.6.7 / Git 根 / 状态，基线81f7b2e，开始时工作区干净；读取当前STATUS、搜索说明、筛选界面和视图存储，确认仍以词法检索、手动组合条件与保存规则为主，保留普通浏览和主动搜索分离的交互。
- 建议按“少操作、快获取信息”推进：一句话转可编辑搜索条件、快捷模板 / 历史；智能视图展示新增匹配并可一键查看；点击命中直接定位正文或笔记；与现有自动简报用显式入口衔接。普通关键词继续本地执行，自然语言解析不等同于语义召回，不宣称能检索未记录的阅读时间或未获取的正文。
- 优先做自然语言条件与易用入口、视图新增匹配和命中定位；跨语言 / 语义召回作为后续增量，需评估召回与误匹配并保留原文命中依据。新匹配、未读和已浏览视图分别定义，避免打开视图即把全部文章标已读；滚动日期、迟到文章和条件修改需有明确计数语义。已有加入专题和规则副本能力复用，不另建简报系统、不自动付费生成。
- 在线核对并打开Inoreader官方搜索说明（https://www.inoreader.com/features/search），其搜索转持续监测流程作为交互参考，当前方案仅覆盖本地订阅库，不声称具备外部公共索引。建议未写入已接受决策；本轮仅追加日志，UTF-8 / diff检查通过，未改代码或创建任务，未执行构建 / 测试。

## 2026-10-27 — 实现全文搜索自然语言筛选

- 基线 `81f7b2e`，Windows / PowerShell 7.6.7 / Node 24.12.0；核对实际 cwd、Shell、Git 根与状态，读取 AGENTS / STATUS 和搜索 / AI 实现，保留任务前已有 WORKLOG 建议记录。只实现本项，未修改上游、简报策略、Schema 或依赖。
- 「全文搜索」中增加可收起的自然语言表单，只有明确提交调用 AI；成功全量替换旧条件并展开原表单，继续复用关键词、字段、来源、标签、时间 / 状态 / 排序与智能视图保存。提示全库替换、信息发送范围、发布时间 / UTC 语义；中英文资源同步，普通浏览仍紧凑，普通搜索 / 同步 / 保存视图无 AI 请求。
- 新 search_filter 模块复用摘要主备配置、凭据、Provider、AgentRuntime 与用量事实。只发句子、来源名 / URL、标签名和当前本地时间 / 时区及 UTC 日期，不发 ID / 正文 / 笔记。严格 JSON / 字段 / 类型 / 枚举 / 日期 / 现有边界校验，名称按真实元数据解析，未知和重名反馈可操作 URL / 标签；额外拒绝模型代选同名来源 URL，返回前复核 ID，沿用参数化本地查询。保留动态 recent_days；不可表达条件必须解释，不执行 SQL 或语义召回。
- 失败 / 无配置 / 不支持条件保留有效查询，空结果正常显示。请求 ID、过滤条件身份与取消共同阻止旧结果覆盖；取消按钮、连续提交、编辑句子 / 手动条件、关闭面板及卸载清理后端任务，注册前取消沿用原运行时保护。自然语言用量增加对应双语显示。
- 实际通过：新增 Rust 7 项（真实 loopback HTTP、合成 SQLite、保存视图同条件、跨日时区 / 滚动天数、严格结构 / 元数据、失败与取消 / 后续请求、无配置 / 普通搜索、用量），原搜索 12 项 / 原性能测试 1 忽略（可选 fixture 入口未导出），原任务运行时 8 项，生产组件 DOM 5 组。lint / build（已有 chunk 提示）、cargo check --locked --offline、全仓 cargo fmt --check、双语资源 / UTF-8 / diff 检查通过。初次 Rust 测试因新 TaskType 未补原测试穷尽 match 编译失败，补齐后最终通过；新增 DOM 的 Fast Refresh warning 通过导出测试组件消除。
- 本轮没有使用 Computer Use、启动 / 操作或截图浏览器 / 桌面，没有读取真实用户库 / 凭据或调用付费模型。原生 Tauri IPC、保存 / 重启、实际亮暗 / 窄窗 / 焦点，以及真实模型语义质量未验证，mock 不代表真实理解能力；其他平台、大型元数据库及全 library 未重跑。旧问题状态保持；未 commit / push / 发布或创建其他对话。完整入口、范围与复验命令见 tests/natural_search/README.md；STATUS 同步当前增量。

## 2026-10-27 — 用户批准后的自然语言筛选真实 API 测试

- 用户明确批准 API 真实测试，继续不授权 Computer Use。重新核对实际目录 / Shell / Git 根与状态，保留全部功能及记录改动；只读当前应用 AI 路由三个配置表及选中主模型凭据，复用现有私有解码器，仅在内存使用。不读取真实 Feed / 文章 / 笔记 / 正文，不修改日常库、用量、凭据或应用配置。
- 新增默认忽略的 paid test（挂在 credential_store 的 cfg(test) 模块下以复用私有解码）及 run_live.ps1；内存复制配置、使用合成元数据及文章，调用真实生产 completion → parse / 元数据校验 → SQLite 查询 / 智能视图保存。关闭测试中的自动 fallback / 重试，最多每次 6 请求，支持单例 / 三个元数据异常场景，拒绝覆盖报告、恢复环境；生产主备及凭据格式未改。
- 首轮真实 6 请求：ecnu-plus 对中文最近 7 天收藏 Rust、英文组合条件（源 / 标签 / 已读 / 收藏 / 排序）、查询合成命中和视图保存一致性通过；上周读过正确 unsupported。未知源 / 标签安全解释因测试只接受专用错误码被记失败；同名源一次 invalid，首轮原始内容未捕获。单例诊断再 1 请求得到合法的歧义说明，仍因旧错误码断言失败；保留两份失败证据，不把失败改写成通过。
- 细化 Prompt：明确缺失 / 同名是应用名称解析问题，不是能力不支持；给出保留 source / tags 字符串的输出样例。提取共享 completion 供显式合成诊断留原始 JSON，生产 request 严格校验不变。live 断言允许信息明确的安全 unsupported，拒绝宽泛 invalid 和错误成功；最后真实复测 3 请求均为专用 unknownSource / unknownTag / ambiguousSource，后者列出实际合成 URL，3 项通过。
- 总计 **10 请求 / 输入 7,541 + 输出 403 = 7,944 token**，服务端均提供用量；不推算货币成本。基础三例在提示修订前通过，修订后只跑异常三例，未完整重复或做稳定率 / 跨模型评估。完整经过与三份脱敏 JSON 见 docs/audits/natural-search-live-2026-10-27.md；STATUS / 测试 README 同步本次增量。
- 修订后离线 Rust 自然语言 7 项 / 凭据兼容 2 项、cargo check --locked --offline、全仓 fmt、UTF-8 / diff / 文档链接通过；前端未改，沿用此前 lint / build / DOM 和原搜索 / 运行时通过结果，没有重跑后声称新通过。未使用 Computer Use 或启动 / 操作 / 截图浏览器与桌面；Tauri IPC、原生保存 / 重启 / 布局、真实网络取消和更广泛模型质量仍未验证，未重跑全 library。未提交 / 推送 / 发布。

## 2026-10-27 — 自然语言筛选 Computer Use 原生验收与修正

- 用户明确批准 Computer Use 检测、验收及修正，替代此前暂停限制。核对 cwd / PowerShell 7.6.7 / Git 根 / 状态，保留全部原有未提交功能与日志。读取 computer-use SKILL / guidance / confirmations / API，通过公开 sky API 选择唯一独立窗口；没有操作日常库 / 凭据 / 偏好或终止用户应用。
- 创建独立 com.mercury4win.naturalsearch20261027 应用、新目录与合成 4 文章 / 4 来源 / 2 标签 / 1 笔记库，从既有合成 V5 fixture 由生产启动迁移至 V8。新增 seed_native.py 和仅 loopback 的 native_provider.py，按控制文件产生正常 / 非法结构 / 503 / 30 秒延迟；没有真实 AI 转发或密钥。
- 首次原生复现 M4W-028：请求实际成功 2 篇但成功说明不见。根因是 EntryList 合并导航默认值后对象字段次序改变，JSON.stringify 误判条件不同。先改 DOM 模拟合并，旧实现失败 success explanation missing；新增 search_filter_identity 固定次序、补默认值、统一已读语义与标签集合，仅用于归属 / 状态判断，SQL / 保存 / 模型校验不变。补等价重排不取消和真实修改仍失效的回归；修正后 DOM 5 组与原生中英文成功提示通过，M4W-028 登记并满足关闭条件。
- 实际原生通过：普通浏览紧凑 / Python 关键词 1 篇零请求；Enter 请求 Rust＋收藏＋7 天 2 篇；未读修改 2→1、组合字段和移除标签；书签创建 / 更新 / 正常退出重启后条件及同一篇 A 保留；歧义来源列具体 URL、阅读历史不支持、非法结构 / HTTP 错误不破坏当前 1 篇；取消、连续提交和卸载旧任务均越过 30 秒旧响应后不覆盖新查询。无模型配置指引下普通 Rust 仍可查询；恢复隔离路由。中英、亮暗、约 800×600、320 / 200 px 面板，Tab→Space / Enter / 设置 Escape、滚动访问均实际检查，普通来源切换不携带隐藏条件。
- 本轮 loopback 12 请求与 SQLite HTTP 用量一致：8 succeeded / 1 failed / 3 cancelled；HTTP 200 的结构无效只代表传输 succeeded，不计业务成功。实际核对出站只含句子 / 名称 / URL / 当前时间，无 ID / 正文 / 笔记。最终 4 文章 / 3 收藏 / 1 笔记 / 1 视图及相对 recent_days=7 保留，外键错误 0，没有真实凭据文件；未增加真实模型请求。首次只读 CLI 输出中文乱码时先查 stdout=GBK / PowerShell=UTF-8，随后用 python -X utf8 核对实际内容正确，未改写数据。
- lint / build（原 chunk 提示）、修正后 DOM 5 组、626 对资源与静态引用、初始及修正后隔离完整 Tauri debug、恢复默认 identifier 完整构建、UTF-8 / diff / 文档链接通过；本轮未改 Rust 产品代码，未重跑 Rust library。关闭隔离窗口并重枚举确认退出，准确识别停止自建 Provider，18761 无监听；CU JS reset，无挂起输入，默认 debug 构建身份恢复。完整证据见 docs/audits/natural-search-native-2026-10-27.md，STATUS / ISSUES / 测试 README 同步。
- 未提交 / 推送 / 发布。Linux / macOS、安装升级、大型元数据库、完整辅助技术矩阵及真实网络取消 / fallback 未验证；真实 API 小样本仍沿用上一轮单独报告和提示版本边界，不把本机 mock 等同真实模型质量。其他旧问题状态保持。

## 2026-10-27 — 自然语言筛选完成情况复核

- 基线：81f7b2e 加已有自然语言筛选实现、真实 API / Windows 原生验收及文档未提交改动；Windows / PowerShell 7.6.7。保留全部既有改动。本轮仅追加本记录，未修改产品代码。
- 已核对执行对话完成状态和用户后续明确批准真实 API 测试、Computer Use 验收的原始消息；检查前后端入口、结构校验、条件身份与取消保护，以及原生报告 / 合成数据库证据。Windows 原生路径与真实模型结果引用对应既有报告，本轮没有重复执行，也不视为本轮独立实测。
- 本轮独立执行并通过：npm.cmd run lint；npm.cmd run build；node tests/natural_search/run_dom.mjs（已有临时 Happy DOM 模块，5 组）；./tests/reader_pipeline/run_rust.ps1 agent::search_filter（7 通过）；cargo fmt --manifest-path src-tauri/Cargo.toml -- --check；git diff --check。构建保留大 chunk 及 Tailwind 插件耗时提示，无失败。
- 结论：当前授权的自然语言筛选核心功能已实现，主要 Windows 用户路径已有验收证据；M4W-028 的条件身份修正已包含在本次组件复验中。真实模型仅小样本，最终提示未完整重复全部真实场景；Linux / macOS、大型元数据库、真实网络取消 / fallback 仍未验证。
- 本轮未使用 Computer Use、未调用真实 API、未访问用户数据库或凭据，未运行全套 Rust library 或重新执行 cargo check；没有提交、推送或发布。

## 2026-10-28 — AI 主题搜索实现、真实评估及隔离原生验收

- 基线 81f7b2e 加全部保留的自然语言筛选 / 真实 API / 原生验收未提交修改。确认实际 cwd、PowerShell 7.6.7、Git 根及状态；UTF-8 操作。用户要求真实跨语言主题召回与逐篇语义判断，撤销只生成 ALL 关键词的限制，并批准 API / Computer Use；本轮未用子代理，未创建其他聊天。
- 新增 agent/topic_search.rs、db/topic_search_store.rs 与 Schema 9：摘要主备路由理解多语主题和硬条件，参数化 OR / AND 多路本地召回，交错新旧文章 / 语言 / 内容字段；模型对真实候选逐篇评分并提供可逐字校验的依据。未知型号保留原词，不为“新”默认七天，不下载缺失正文，无向量服务。一次 3×12，显式继续，已发布范围不可变、继续分叉；令牌保护分页 / 批量，篡改 / 过期拒绝而非回到全库，旧结果在失败 / 取消后保留。
- UI 改为 AI 主题搜索，阶段 / 覆盖 / 理解 / 依据 / 片段缺失可见，草稿与有效列表分开；手动条件显式应用，冲突不静默覆盖。普通关键词 / 原视图 / 专题保持；AI 快照当前禁保存有解释，后端也拒绝保存临时范围。隐私说明改为发送候选标题 / 摘要 / 本地正文片段，仅明确请求 / 手选笔记字段才发送笔记，has_note 不等同授权。
- 固定人工标注 20 篇合成文章和 7 例，覆盖中英、陌生型号、180 天旧文、后部正文、融资 / 教程 / 汽车等干扰、数据库性能 / 开源构建两个非 AI 主题及明确硬条件 / 无相关。实际模型暴露宽泛词不足、无关元数据被复制、短词抢占片段、概念组 / 围栏 / 一致重复字段等问题并修正；后部片段回归先实际失败再通过。额外 / 冲突条件、SQL 字段、非法 ID、改写依据继续拒绝。字段专用召回所有通道遵守字段 / 排除标题，另有生产 DB 回归。
- 首次评估末尾计量 SQL 字段错误导致原始明细未保存，真实请求 / token 无法完整恢复，明确登记，未写成零；后改为付费前预检和逐例落盘。已保存的逐轮合计 59 请求 / 86,226 token，另有该丢失首轮。最终 topic-search-v4 同一固定七例全量 7/7，15 请求 / 22,100 token，无漏选 / 误选，不跨轮挑成功。只读 AI 配置白名单与私有解码，合成业务资料，主路由无 fallback / 自动重试；没有发送日常正文 / 笔记或写日常库。完整失败与交换见 docs/audits/topic-search-live-2026-10-28.md。
- Computer Use 按技能公开 sky API 操作独立 identifier / 80 篇合成库 / 本机 18762；实际迁移 V5→V9，36→72→76 显式继续、六篇双语含旧文和后部正文、缓存、503 / 30 秒取消保留、范围全部已读只改 1–6、AI 禁保存 / 退出普通保存 / 重启80篇、中英亮暗1200×800与800×600通过。原生修正旧“不用 AI”提示、排序显示与借旧检查数误报部分完成。共享构建因已有用户进程锁住 DLL 失败，采用独立 target，未结束用户应用；最终完整构建及窗口复核通过。原始 UIA、隔离库 / 请求证据见原生报告。
- 通过：topic_search 本地 11 项（付费入口默认忽略）、db::87通过 / 原性能1忽略、运行时8、旧解析器7；生产 EntryList DOM3组及旧兼容入口、lint / build（原大 chunk 提示）、cargo check / fmt、649对双语键 / 插值、UTF-8 / diff。未执行全 library；可选导出 fixture 未设置变量，未宣称导出。STATUS / ISSUES / TESTING / DECISIONS（D-013）及测试入口同步，M4W-029 最低条件满足关闭，其他旧问题保持。
- 未提交 / 推送 / 发布，未改根 Cargo / Tauri 配置和锁文件；日常数据未写入。其他平台、大库、模型稳定性、安装升级、200px列表及真实网络 fallback / 取消完整矩阵未验证。收尾关闭隔离窗口 / Provider 并重置 CU，会保留合成证据供复验。

- 最终收尾：22 次 loopback = 20 succeeded / 1 failed / 1 cancelled，与 SQLite 一致；Schema9、80 篇、外键0、1笔记 / 1视图，read_ids=1–6，笔记未出站，无凭据文件。隔离窗口确已退出，18762 无监听，CU已reset。额外审阅补充创建范围时检查干净索引并固定读快照，避免并发正文更新使新 revision 搭配旧索引；check / topic_search11 / 完整构建通过，此围栏未额外做原生或多连接压力测试。最终复制程序副本保留供复验，未再发起真实模型请求。

## 2026-10-28 — AI 主题搜索完成情况独立复核

- 基线：81f7b2e 加全部既有自然语言 / AI 主题搜索未提交改动；Windows / PowerShell 7.6.7。执行对话已结束，产品代码、Schema 9 和既有验收文件均已落在当前工作区。本轮保留全部改动，只追加复核记录。
- 核对生产主题检索 IPC、双语查询计划与逐篇判定、证据校验、后端结果范围接入及原始真实 API / 隔离原生报告。最终 topic-search-v4 的 20 篇合成库 / 七例全部通过；这是执行对话既有证据，本轮未重复调用真实模型或操作桌面。不能据此推断任意用户库的召回率。
- 本轮独立执行通过：npm.cmd run lint；npm.cmd run build（已有 >500 kB chunk 提示）；node tests/topic_search/run_dom.mjs（已有 Happy DOM，3 组）；./tests/reader_pipeline/run_rust.ps1 topic_search（11 通过，1 付费测试按设计忽略）；cargo fmt --manifest-path src-tauri/Cargo.toml -- --check；git diff --check。未重跑 cargo check、完整 Rust library、其他平台或原生构建。
- 当前限制：每次检查最多 36 篇召回候选，需显式继续；AI 结果暂不能保存为智能视图；模型读取候选必要片段，未做全库向量索引，候选之外的语义覆盖不能保证。最后增加的索引快照并发保护已有回归但未再原生 / 压力测试。真实 API 历史记录可恢复 59 请求 / 86,226 token，另首轮计量丢失，总消耗不完整；最终整轮为 15 请求 / 22,100 token，不能作为全部消耗。
- 结论：所派发的跨语言主题理解、召回、内容判定和结果列表核心链路已实现，当前本地核心复验通过。仍需用真实使用反馈观察模型稳定性和大库效果。本轮没有 Computer Use、真实 API 或日常数据库 / 凭据访问，未提交、推送、发布或另行派发任务。


## 2026-10-28：全标题分阶段检索、持久恢复及真实/原生验收

- 基线 `81f7b2e` 加全部保留的既有未提交改动，实际目录 `G:\sth_interesting\mercury4win`，Windows / PowerShell7.6.7、UTF-8。先核对cwd/Shell/Git根/status与STATUS/ISSUES；没有覆盖用户改动、创建新聊天/子代理、提交/推送/发布或改上游。
- 按已确认方案新增 `agent/staged_search.rs` 与 `db/staged_search_store.rs`：硬范围全标题入池，词法只作优先/后部救援；标题/预览/连续全文块/整篇判定独立，5000字符块及200重叠不截尾，语境/反证及来源版本绑定。一次提交处理有限范围，取消36篇边界，按长度组批，无产品总token预算/后台监控/无限重试。
- Schema10后续迁移保存资料版本、阶段缓存、块检查点及不可变快照；默认材料不读取出站笔记，显式笔记字段才发送；缺正文仅标题/摘要标记。read/star变更不改变快照，新文章留下一轮；变更/删除版本过期，严格批量拒绝。旧Schema9视图、AI判定、笔记与来源保留，升级/重复迁移测试通过。
- 搜索IPC切换到新运行器并新增stop/recent，沿用runtime实际取消/提交围栏；类型/绑定/入口同步。NaturalFilter展示独立计数、逐步快照、暂停/取消/恢复/重启载入、缓存/错误/部分结果；EntryList批量捕获点击令牌、本地排序/分页不付费，AI快照禁保存而普通视图继续可用。原关键词、旧AI范围/解析IPC和专题保留。
- 71篇人工标注混合集含基础20+新增11+自然负例40，七主题覆盖模糊/误导/空标题、周报混合、正文后部/长文、历史引文/融资/否定、缺全文、新实体及非AI主题。受控响应借标签只测协议，真实理解单独评价；不让固定词法候选假装全标题。
- 初始离线块数断言在fixture缩短后陈旧，暂停断言把两个本地保留空标题错写为0；修正预期后回归通过。初次测试可见性/类型推导编译错误也已修复。最终9项分阶段Rust（付费入口默认忽略）、生产DOM4组、lint/build、cargo check/fmt及完整独立Tauri debug通过；682对双语键/插值与UTF-8检查通过。全library实际219通过/4失败/4忽略，四项既有socket取消超时名称及证据见专项报告，未算通过/删除/跳过。
- 真实API首轮v1为41请求/91,216已报token，新流程1/7完成（5结构失败/1超时），含旧对照；v2整套98请求/200,868token为5/7直接完成。兼容根数组、段引文字符串、仅同篇已给材料中逐字引文的已知键错配；不可核验预览证据保留不确定继续完整资料，不能排除。最后空上下文null与旧块号回显修正后，对原始system/user严格匹配回放98响应恢复合成检查点，只补4次真实请求/3,961token，最终七例处理完且符合标签。回放不计新增HTTP/用量，也不把其耗时当首次扫描。
- 本轮实际143真实请求、296,045已报token，另1次超时用量未知，未超过累计160次实验上限。只读现有AI配置白名单及选定主模型凭据到内存，合成业务库，未读取日常正文/笔记、写日常库或记录密钥；逐请求开始/结束落盘及完整SQL预检先于HTTP，所有失败/中断报告保留。旧同71硬范围模型发布10/12、新12/12，数据库均5/5；新样本更慢/用量更大，未承诺省钱。
- 使用computer-use技能，在独立identifier/合成V5→10库及18763 loopback实际验证71标题/26块、缺全文依据、HTTP暂停/取消、关闭重启载入/恢复、故意中途失败/3篇部分快照/续跑、本地排序/精确12篇批量、普通关键词/保存视图/重启。发现800×600英文长说明挤走操作按钮，按钮提前并把详情折叠后实际复验可见；中英亮暗均核对，最终旧块号/数组/字符串/null受控响应完成。99 loopback=96成功/1故意失败/2取消，无笔记外发/密钥/外键错误。
- 隔离应用正常退出，按PID/完整命令核对停止Provider，18763无监听、窗口0、CU JS reset；日常开发进程未终止。D-014替代D-013词法准入/36篇约定；M4W-030按限定验收关闭，其他旧问题保持。更新STATUS/ISSUES/DECISIONS/TESTING、测试说明及[真实报告](audits/staged-search-live-2026-10-28.md)、[原生报告](audits/staged-search-native-2026-10-28.md)，不改写旧报告。
- 未验证其他平台、安装升级、大库/极长单篇/任意模型稳定性、所有字段组合及本轮无配置完整原生路径；全标题覆盖仍可能有模型误排除，滚动上下文是模型归纳，缺正文不自动联网补取。产品可暂停恢复，不存在常驻扫描或未授权发送消息。


## 2026-10-28 “新的AI模型”超时诊断与修复

- 基线81f7b2e及所有保留未提交改动，Windows / PowerShell7.6.7；核对实际cwd、Git根/status并读STATUS/ISSUES。只读原库扫描状态/请求事实，确认北京时间15:30:43～15:32:43首批64标题超时，原范围609篇；旧文本日志无此次记录。未读取原文章正文/笔记或修改日常数据库业务数据。
- `agent/staged_search.rs`标题改16项/6000序列化字符；`db/staged_search_store.rs`在claim前读取上次超时数量，显式恢复减半至1～16项。无自动付费重试、Schema变更或全文截断。新增安全阶段日志，只记录阶段、条数、耗时、错误码。609篇Rust回归检验16项超时、旧64项兼容、8项续跑、全部609覆盖、12篇准确结果和不重复规划。
- 真实ecnu-plus同查询首轮15HTTP/34,400token通过标题，但整篇结果为单元素数组包裹对象导致invalidJudgment。只兼容单元素/单已知数组键容器，额外字段、多容器、缺项、未知ID、伪造引文继续拒绝；真实合成交换追加protocol_cases。再次从零23HTTP/43,126token完整通过71标题/26块/12预期命中，258.374秒，0回放；本轮总38HTTP/77,526token。用户明确批准真实API测试，配置表及一份密钥只读/内存解码，全部业务资料为合成库，不发送日常文章/笔记。没有本轮重测全部七主题。
- 初始化测试有括号与SQLite usize类型编译错误，修正后初次9过/1失败/1忽略；失败为暂停断言写死空标题计数2，在小批次下实际只处理1，改为断言已保存检查点保持不变。最终10项定向Rust全过（付费入口另行通过），生产DOM4组、lint、前端build、cargo check/fmt通过；未重跑全library，旧4项socket失败基线保留。构建已有大chunk提示。
- 按computer-use技能使用独立identifier/609合成库/18763loopback，实际等待120秒超时，UI点击恢复后16→8，完成609/609标题、607模型初筛、26/26块、12命中，0剩余/错误、1资料不足。94事实=93成功/1超时（该次用量未知）；无笔记外发、密钥文件或外键错误。该native构建先于后来单元素容器修复，后者单独由真实交换Rust与最终真实复测覆盖。隔离应用正常退出、Provider停止、18763无监听。
- dev监听源码修改自动重编译并重启，日常实例已运行修复版本；用UI只载入原检查点并显示恢复按钮，原库search_filter事实仍只有本次原先2条，无新增HTTP。未自动续跑用户原609篇，M4W-031待验证。更新STATUS/ISSUES/本日志、测试说明及[专项报告](audits/staged-search-timeout-2026-10-28.md)；未提交、推送、发布、改依赖或调用子代理。其他平台及任意模型稳定性未验证。


## 2026-10-28 Computer Use 关闭窗口偏好

- 用户要求关闭按钮点不到时改用其他方法，不反复点击右上角。已在AGENTS.md协作约定记录：一次未生效后确认目标，改用Alt+F4或正常退出方式。仅文档修改，核对UTF-8及diff；搜索修复任务继续。


## 2026-10-28 第13批标题判断中断修复

- 基线81f7b2e及保留的全部未提交修改；按实际cwd/PowerShell7.6.7/Git根/status核对环境并读STATUS/ISSUES。用户复测原库新scan：609篇，前12批192篇成功，第13批16篇invalidJudgment；13次HTTP全成功，非此前超时。原回复未保存，不能断言具体字段/ID原因；仅读阶段/用量事实，未导出日常文章/笔记。
- 标题是初筛优化：严格解析失败后丢弃整批模型结论，原冻结ID转内容检查；不信任排除/相关/陌生ID，不自动重试或缓存失败判断。defer_titles保留owned/提交围栏，title_model=2持久区分有效模型初筛，Progress.title_fallback缺省兼容旧快照，NaturalFilter双语说明数量；无Schema迁移。固定日志原因码代替正文，后补安全Provider failure_class，仅固定类别/有效HTTP状态数字，不泄露返回体或凭据。
- 新609篇回归在第13批正常loopback HTTP中替换陌生ID，验证16篇进入预览、未知ID写入0、不额外请求/缓存失败、最后12预期命中及全部609处理。11项staged Rust、原topic6项、DOM4组、lint/build/check/fmt、683对双语资源通过。最初将三态title_model仍读为bool引发编译错误，改i64后通过；未重跑完整library，既有4项socket失败基线保持。
- 使用computer-use技能在独立identifier/609合成库/18763服务实际一键完成第13批异常转内容，界面明确16fallback；609标题、591有效模型初筛、26块、12匹配、1资料不足、0错误/剩余。57个loopback HTTP均成功，无笔记外发/密钥文件/外键错误/陌生ID。应用正常退出、服务停止。截图/UIA不同目标时重新激活后再输入；用户随后要求关闭按钮一次不生效改Alt+F4，已单独写入AGENTS.md。
- 真实合成初次1HTTP/1,213token在主题规划9词边界失败；仅容许OR组1～16个合法短词，总字符/硬条件/实体/未知字段保留，17词拒绝，原始交换加入协议回归。随后609篇真实首扫35HTTP/49,862已报token，34成功/1Provider失败用量未知；保存529标题进度、剩余97，原库第13批校验错误未再现。原真实失败类别未记录，不能追溯声称具体HTTP状态；已补安全诊断。此前独立71篇成功不替代本次609首扫失败。
- 根target完整构建因文件占用os error32失败；换独立target完整Tauri debug成功（大chunk提示保留），不终止未知持有者。测试回放增加明确区分已完成响应与已记录失败；只有完整输入匹配才重建，失败不伪造HTTP/Token，且只在显式M4W_STAGED_RESUME_ONCE下恢复一次。恢复真实结果另补，详见[报告](audits/staged-search-title-fallback-2026-10-28.md)。

- 最终按完全匹配原system/user的记录重建34成功响应及1个已知Provider失败，单独记录replayed_failures，不假定原HTTP状态。一次显式store::claim恢复后新增20实际HTTP/42,266token全成功，完成609标题/607有效模型初筛/26块/12预期匹配、0误选漏选/错误剩余、1资料不足。708.822秒含回放，不是冷扫描性能；真实样本fallback=0，降级路径仅故障注入验证。本任务56实际HTTP/93,341已报token，另1Provider失败用量未知，原失败报告保留。
- 后补Provider诊断的最终11项staged Rust/check/fmt及完整独立默认identifier构建通过。根开发进程本轮中已退出；最新默认构建现从独立target运行，CU只载入原192/609标题检查点并把恢复按钮滚动显示，无新增原库AI请求，实例供用户继续。隔离实例/18763已退出，CU最终reset不关闭日常实例。原库同次剩余592篇尚未最终核验，M4W-032待验证；M4W-031及其他旧项状态保持。
- 更新STATUS/ISSUES/D-014容错说明/本日志、分阶段README、原始合成格式fixture及[真实/原生报告](audits/staged-search-title-fallback-2026-10-28.md)。无依赖/锁文件变更、提交/推送/发布、新聊天或子代理。其他平台及任意模型稳定性未验证。


## 2026-10-28 默认只搜索最新100篇

- 基线81f7b2e及全部保留修改；核对cwd/PowerShell7.6.7/Git根/status并读STATUS/ISSUES。按用户明确要求，建池先应用当前硬条件，再按现有最新顺序选100篇（发布时间降序/ID降序，无日期排后）；不足100取全部。范围内仍执行阶段/全文资料核验，本地排序不扩大范围，普通关键词搜索保持全库。
- staged_search.rs增加SCOPE_LIMIT=100、算法topic-search-latest100-v3；staged_search_store建池SQL加LIMIT。新身份隔离旧全量检查点/缓存，旧recent/resume不进入新流程，不删除或截断旧数据，旧结果令牌仍可读；Schema10不变。前端NaturalFilter及双语范围说明同步。
- 新硬范围/排序/旧检查点隔离回归，原609库容错/超时回归调整到100篇池/第3批异常；12项staged Rust、DOM4组、lint/build/check/fmt、684双语键及UTF-8通过。未跑完整library，旧4项socket失败基线保持。新target依赖检查启动后主动停止，改复用现有target；中止未算通过。
- computer-use独立identifier/609合成库实际验证普通列表609，而AI仅100。7批/98非空标题、16异常标题转内容、26资料块/12预期结果/0错误剩余；全部出站ID属于SQL最新100集合，26个loopback HTTP成功，无笔记外发/密钥/外键错误。隔离应用Alt+F4正常退出，服务停止，18763无监听。历史609/第13批及真实失败报告保留，没有改写成当前算法范围。
- 默认identifier完整桌面构建通过并已启动原应用目录，最新100说明可见，旧全量恢复不再出现；已填好“新的AI模型”，未提交，不新增真实API或以日常正文/笔记测试。旧实例和隔离窗口均用Alt+F4，不反复点击右上角。D-015更新范围约定，STATUS/ISSUES/README/本日志和[专项报告](audits/staged-search-latest100-2026-10-28.md)同步；原库新100篇路径未验证，旧问题不提前关闭。
- 未升级依赖/锁文件、提交、推送、发布、开新聊天或子代理。其他平台/大正文/模型质量未在本轮重测。日常新构建保留运行，CU最终reset。


## 2026-10-28 多语言关键词先展示与渐进AI核验

- 基线81f7b2e及全部保留未提交改动，Windows/PowerShell7.6.7；核对cwd/Shell/Git根/status及STATUS/ISSUES。用户要求部分多语言词命中即可先展示，AI慢慢检查，最新100篇约定继续。采用topic-search-quick100-v4，Schema10保持；复用主题理解、OR本地匹配、字段权重与短词边界，初步/确认单独标记和计数，确认优先，成员变化发布不可变快照，空快照可清空列表。笔记仅显式字段参与。
- 错误/暂停保留当前快照，取消恢复此前范围，收起继续且保留暂停/取消入口。发现渐进on_ai_scope清空Reader，改为同input_key更新保留当前文章；补生产DOM。修改staged_search及store/keywords、topic_search_store/search_store、NaturalFilter/EntryList、类型和688对双语资源，计时脚本分开初步与确认首结果。D-016/STATUS/TESTING及测试说明同步。
- 14项staged Rust通过、1付费入口默认忽略；原topic6、普通搜索12（另1性能忽略）、生产DOM4组、lint/build/check/fmt、688对键/插值通过。阻塞标题HTTP仍可分页/严格批量，AI移除弱匹配、加入无关键词相关文，旧令牌稳定；失败/显式笔记/过期范围覆盖。最初DOM旧收起断言失败后按新行为调整；未重跑全library，历史4项socket失败不改写。独立target首次DLL/EXE占用os32失败；根target隔离及默认identifier完整debug构建成功，既有chunk提示保留。
- 真实models_zh合成案例首扫15HTTP/34,502token，17.547秒初步结果、550.375秒首AI确认，664.269秒因105资金说明书final缺少实际依据被严格拒绝；保留失败。逐项输入匹配回放15已付费响应重建错误，测试显式恢复新增8HTTP/10,774token，71篇/26块/12预期结果完成。合计23真实HTTP/45,276已报token；回放44/126ms不算首次性能，非七例重测或冷启动无中断通过；未放宽依据规则。只读配置/一份内存凭据，未发日常正文/笔记或写日常库。
- computer-use技能实际首轮609合成库冻结100篇，24初步可读/打开Sable，故意预览503后17初步保留和Reader保持，恢复完成100标题/26块/12篇；该实例后续资料变化不还原。全新71篇补验首个标题响应前0/71+24初步，收起时3确认/10待核验并继续到12最终/26块，Reader始终保留，UIA/结构证据落盘。累计47loopback（46成功/1故意失败）、无笔记标记出站/凭据文件/外键错误；Alt+F4正常退出，两测试窗口和18763监听已停止。
- 会话中sky变量重置后重选唯一窗口；无截图点击报geometry不可用后改为含截图观察。旧日常窗口曾被用户输入保护阻止关闭，停止尝试，继续任务时旧进程已退出。没有重复点关闭按钮或强杀未知进程。[完整报告、实测失败及边界](audits/staged-search-quick-2026-10-28.md)。未提交/推送/发布；日常新搜索、其他平台/安装升级及模型稳定性未验证，M4W-031/032保持待验证。

- 交付补记：默认identifier完整构建后已启动根target新版，CU确认唯一默认Mercury窗口/全文搜索入口，未发起日常AI查询。30文件UTF-8/无BOM、JSON/JSONL和文档链接通过；最初替换字符断言命中历史SSE乱码示例，保留旧记录，新增段落检查通过。diff --check通过；两个隔离窗口和18763均退出。

## 2026-10-29 外部 RSS 清单整理

- 基线 81f7b2e，保留已有未提交改动；Windows / PowerShell 7.6.7。读取 weekend-project-space/top-rss-list 当前 main 的 README，按原顺序取前20条，保留同名不同地址，生成 UTF-8 无 BOM 的 OPML 2.0：C:\Users\Selvori\Desktop\opml\top-rss-list-top20.opml 。
- 实际通过：XML 回读、20条计数、名称/地址及顺序逐项比对、绝对 URL 与 UTF-8 解码检查。未联网验证各订阅源可用性，未导入应用或改动固定 OPML 样例；本任务未修改应用代码。


## 2026-10-29 提交多语言渐进AI搜索

- 按用户要求提交当前仓库工作；确认G:\sth_interesting\mercury4win、PowerShell7.6.7、Git根、main分支及81f7b2e基线，保留已有改动与外部RSS整理记录。提交范围包含自然语言筛选、分阶段搜索、最新100篇和关键词先展示、相应测试及真实/隔离验证记录，以及窗口关闭操作约定。
- 提交前核对142个变更/新增文本文件（约5.2MB），无构建产物、临时目录、数据库或凭据文件；凭据模式扫描仅命中已存在的兼容性假密钥测试。核对diff、UTF-8及JSON/JSONL结构；未修改产品代码或重复运行上轮已完成的编译/测试，实际通过与历史失败继续以原验证记录为准。
- 此次执行本地Git提交；不推送远程、不创建发布标签或版本。
