# 主题追踪与信息简报

## 默认自动模式（Schema 8）

入口为侧边栏“主题追踪”。输入一句话，点“创建并生成简报”；后续可从主题卡片直接“生成更新简报”，无需逐篇审批。也可“仅创建”，没有 AI 时继续管理主题、保存备注和阅读旧简报。仅显式生成调用 AI，不在同步后自动调用。

默认发现最近入库 1,000 篇本地文章，补充原允许范围内最多 1,000 篇主题词法匹配及保留资料；可选高级设置复制智能视图中的关键词、来源和日期规则。名称 / 说明、中文片段 / 常用中英别名排序与近期发现通道仅提供候选，**AI 实际判断相关性、重复报道及新增信息**，随后自动总结。每批最多 60 篇、每篇预览最多 1,000 字符（开头及后部主题相关片段）、最多选 12 篇；空选一次最多继续检查 3 批。界面显示本批与范围边界，不能据候选数宣称已确认重要新闻。

简报以连续文章呈现，来源与引文集中于文末，不显示生成模型名称；“本次采用的文章”可展开来源、反馈无关或排除，资料页可恢复候选资格。旧引用仍对应冻结原文。缺正文的项目明确仅依据标题和 Feed 摘要；主动抓取沿用原正文管道，不静默全库抓取。

检查位置、成功简报与已读位置互不代替。无变化不请求；AI 判断无新信息时记录检查，避免同批反复付费，但不创建成功简报。失败 / 取消保留待处理资料，生成重试可复用仍有效的筛选。已读须显式标记。所选文章使用完整可用正文，不按本地 token 估算缩批、延后或拒绝；服务端请求失败不确认失败批资料。

仍有候选时提示继续生成，不把本批空选说成全库无信息。检查完后可点“重新检查选文”，仅重新检查未用于成功简报的版本，不需手动加文；它会再次请求 AI，普通打开 / 刷新不重置检查。

默认复用已启用的摘要模型及主 / 备用路由，不需要另配两个模型。应用不设置简报输入 token 预算或固定输出 max_tokens；实际模型容量由服务端决定。预览之外的细节、窗口外未匹配 / 未保留旧文、极长历史与真实质量仍有边界。[原自动模式报告与当时基线](../../docs/audits/topics-auto-2026-10-26.md)保留，后续变更及验证见下文。

### 自动模式复验

```powershell
./tests/reader_pipeline/run_rust.ps1 db::topic_
cargo check --locked --offline --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm.cmd run lint
npm.cmd run build
node tests/accessibility/check_resources.mjs
```

24 个专题入口包含 3 个可选 fixture 入口；正常测试不读取安装应用数据或凭据。真实 loopback HTTP 测试运行生产 AI 选文与总结代码，覆盖请求数、非法 ID / 引用、空选择、无新信息、失败 / 取消 / 缓存重试、正文版本、跨期修订、范围变化、预算延后、反馈、已读 / 重开及旧 Schema 兼容；固定合成资料包含中英、同名歧义、转载、无关关键词、缺全文和恶意指令。受控响应不能代替真实模型质量评估。

导出新生产 DTO 后运行自动界面回归，目标文件必须不存在：

```powershell
$env:M4W_AUTO_WIRE_JSON = Join-Path (Get-Location) 'tmp/auto-ui-new.json'
try { ./tests/reader_pipeline/run_rust.ps1 export_automatic_fixture }
finally { Remove-Item Env:M4W_AUTO_WIRE_JSON -ErrorAction SilentlyContinue }
node tests/topics/run_auto_browser.mjs <playwright-core/index.mjs> tmp/auto-ui-new.json <browser.exe>
```

runner 使用项目已有 esbuild 和外置 Playwright / 实际 headless Edge，不增加依赖。11 组含一句话创建、卡片直接生成、引用 / 焦点、反馈恢复、已读、无 AI、无信息且 Channel 阶段缺失、取消及卸载，另含 12 个双语亮暗简报布局和 6 个展开设置布局。传输 mock 不冒充 native IPC。

隔离 native 准备：`M4W_AUTO_NATIVE_DB` 可导出空主题的合成 V8 库，`M4W_AUTO_SCHEMA7_NATIVE_DB` 可导出真实 V7 库，均拒绝覆盖。`auto_provider.py <新输出目录> [端口]` 只监听 loopback，默认 18757，不调用远程模型；control.json 支持 `ok / no_information / invalid / summary_error / summary_empty`、delay 与 delay_stage。只使用独立应用标识，不能把合成库或此路由写进日常应用。

本轮按用户要求暂停 Computer Use，尚未完成新版 native 点击闭环或 V7→V8 应用启动验收。恢复后应验证无 AI 创建 → 自动选文及简报 → 本次来源 / 反馈 → 迟到 / 修订资料更新 → 无新信息 / 失败 / 取消 / 重试 → 已读与退出重启 → 原生 Markdown 导出。真实模型评估使用 [selection_eval.json](selection_eval.json) 记录误选、遗漏、重复、引用语义及实际用量，不虚报分数。

## 2026-10-27 连续文章与文末来源

简报正文改用连续段落，不渲染逐条卡片 / kind 栏目或内联引用按钮；来源和冻结片段统一放在文末去重展示。阅读界面 / Markdown 不显示生成模型名，历史模型身份仍内部保留。自动与手动新提示都要求文章式行文，旧简报无需付费重写即可用新排版阅读。

当前专题 33 个入口通过，含 3 个未启用 fixture 入口；新增 Markdown 正文先于引用、去重、无模型名和独立备注回归。run_auto_browser.mjs 13 组 / 原 run_browser.mjs 10 组通过，检查正文直接段落、没有卡片或内联证据、来源位于末尾、重复片段只有一个入口、无模型标识，以及双语亮暗 / 窄窗口 / 焦点恢复等既有交互。lint / build、594 双语键与 Rust check / 格式通过。

当前完整 Tauri dev / WebView2 已只读核对长文、文末来源与原文弹窗 / Escape 返回；没有自动请求真实模型或改写已存内容。新提示下真实模型的行文质量未另行评估，不能把布局检查当作质量评估；完整 library 的历史失败保持。

## 2026-10-27 冻结段落引用

默认自动简报使用 topic-information-v2。请求将完整可用正文分为稳定的 `{ passage_id, text }` 片段，模型返回 `{ entry_id, passage_id }`，后端从冻结正文恢复原文和 UTF-8 位置；无需模型重抄 quote。旧 quote 仍接受逐字或唯一空白归一匹配，拒绝改写、编造、歧义 / 重叠及过长跨度。保存 DTO 和 Schema 不变，不清空历史或筛选缓存。

`./tests/reader_pipeline/run_rust.ps1 db::topic_` 当前 32 个入口通过（含 3 个未启用的 fixture 入口）。新增 5 项覆盖中英 / emoji / NBSP / 换行映射、非法引用、完整分段和边界、重复文本由编号消歧，以及非法编号后仅重新请求总结。新版 `auto_provider.py` 返回请求中的 passage_id，新增 `invalid_quote` 控制模式可复现严格失败恢复；不调用真实模型。

用户已允许恢复 Computer Use。本次独立 Windows 应用实际完成创建生成、编造引文失败、编号重试、中文高亮 / Escape、已读及正常退出重启引用恢复，共 3 次 loopback 请求；正常应用 debug 构建配置已恢复。用户文章 585 / 真实 Provider 未核验，M4W-025 待验证。[报告](../../docs/audits/citations-2026-10-27.md)、[合成证据](../../docs/audits/citations-2026-10-27-evidence.json)。

## 2026-10-27 选文 JSON 容器兼容

M4W-024：旧解析器将顶层文章数组按 Selection 的位置字段读取，可复现用户报告的 map / sequence 类型错误。没有取得用户实际模型回复，复现依据为固定合成响应。

支持标准选择对象，以及含明确 entry_id / group 的顶层数组、单篇对象、selected 单对象、以整数文章 ID 为键的对象映射和完整 JSON 围栏。ID 键与内嵌 ID 必须一致；严格校验候选范围、字段、空结果、数量与大小，不容错为“任意 JSON 都成功”。缓存继续使用标准 selected 数组对象。

```powershell
./tests/reader_pipeline/run_rust.ps1 db::topic_
cargo check --locked --offline --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

本次 27 个专题入口通过，包含 3 个未启用的可选 fixture 导出入口。新增 3 项实际覆盖格式归一化及拒绝结构，loopback HTTP 的裸数组 / selected 单对象响应通过生产筛选与总结、保存成功简报及标准缓存，之后无变化不再请求。前端、完整 library、真实 Provider 及桌面点击未在本次重跑；M4W-024 待验证，继续暂停 Computer Use。

## 手动模式（旧专题兼容路径）

入口：侧边栏「主题追踪」。新建专题时可保存一份智能视图规则副本；相对日期仍滚动，原视图修改、改名或删除不改变副本。手动模式的候选需要主动纳入。也可在普通列表、搜索或智能视图中点「选择文章」，勾选后「加入专题」。删除专题只删除专题内容，不删除原文章、文章笔记、收藏或视图。

资料页显示已纳入、新增纳入、上次成功使用、正文缺失和内容／归组变化；相同正文默认归入同一证据组，可以人工更正。自动组只识别相同正文，近似转载需要用户和模型审查；不同网站并不自动构成独立证据。正文仅使用本地抽取文本，不用文章摘要补充缺失正文，不自动全库抓取。主动「抓取／重建正文」使用既有正文管道，失败可重试。

显式生成使用「设置 → 任务」中摘要任务的已启用主／备用模型路由，专题任务、结果、取消槽和用量类型独立。界面展示实际路由，开始时冻结配置。每次最多 40 篇，当前正文总计不超过 100,000 字符，当前＋基线完整请求不超过 160,000 字符；超限拒绝，不静默截断。这些是输入上限，不保证任意模型都有足够上下文；模型拒绝或输出不完整时保留旧成功版本。

首次输出为跨来源对照，增量请求同时提供上个成功版本的资料正文／结论和当前确认范围。新增、修订、移除、分组改变及正文就绪均进入比较；仅查看、刷新或保存个人判断不触发 AI。无变化时界面禁用生成，后端也在读取配置／凭据前返回。失败／取消不推进基线，写入前重新核对来源、专题修订和成功基线，使用现有提交围栏原子保存。

简报历史保留每版来源、原文文本、版本标识、分组、模型归属及时间；个人判断独立保存，生成不覆盖。点击证据显示保存正文并精确高亮唯一原文片段；位置为快照中的 UTF-8 字节偏移。历史引用不会重新绑定到当前正文；无法对应当前资料时明确提示，仍可打开正确文章。结构校验拒绝未知 ID、非原文／歧义片段、缺少关键引用、错误类别或超长输出。引用位置有效不代表判断正确。Markdown 导出保留引用和版本，以及已保存的个人判断。

## 自动回归

从仓库根目录，Windows GNU 沿用既有 runner：

```powershell
./tests/reader_pipeline/run_rust.ps1 db::topic_store::tests
./tests/reader_pipeline/run_rust.ps1 db::search_store::tests
cargo check --locked --offline --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm.cmd run lint
npm.cmd run build
```

专题模块 12 项入口（包含两个可选 fixture 导出），使用内存历史 Schema 或隔离临时磁盘库。覆盖 V1–V6 升级／重复迁移、CRUD／规则副本、成员／人工归组、正文就绪／增删改、笔记隔离、快照引用、无效／歧义引用、输入上限、陈旧范围／并发基线／取消围栏、真实 loopback HTTP 首次／增量／非法输出／503／重试及用量、实际 HTTP 取消、连接池重开和等价正文格式不产生假增量。测试文件不读取安装应用数据库或凭据。

生产 DTO 的界面回归：

```powershell
$env:M4W_TOPIC_WIRE_JSON = Join-Path (Get-Location) 'tmp/topic-ui-new.json'
try { ./tests/reader_pipeline/run_rust.ps1 export_topic_ui_fixture }
finally { Remove-Item Env:M4W_TOPIC_WIRE_JSON -ErrorAction SilentlyContinue }
node tests/topics/run_browser.mjs <playwright-core/index.mjs> tmp/topic-ui-new.json <browser.exe>
```

目标文件必须不存在。runner 使用单独安装的 Playwright 和真实 Edge，不改项目依赖。10 组覆盖生产 SQLite DTO、缺失正文、重复组、变化标签、引用／Escape 焦点恢复、个人判断、抓取失败、非法引用保留版本、重试／历史／无变化、本地刷新无 AI、12 种语言／主题／尺寸布局、规则副本 IPC、取消与卸载归属。mock 仅替换 IPC，不把它作为 native 或真实模型质量证据。既有文摘、筛选和分页 DOM 及搜索浏览器回归仍保留。

## 隔离桌面

使用独立 Tauri identifier。可选 `M4W_TOPIC_NATIVE_DB` 导出全新 V7 合成库；`M4W_TOPIC_SCHEMA6_NATIVE_DB` 导出真实 V6 合成库供生产升级。均拒绝覆盖已有文件。`provider.py` 只监听 loopback，不转发任何远程模型请求，可用 `control.json` 的 `mode=ok/invalid/error` 和 `delay` 控制响应；`requests.jsonl` 仅记录合成资料。

`native_fixture.py` 只接受明确命名的 `com.mercury4win.topics20261026/mercury.db`，并核对合成标题后才允许准备路由／修订或读取证据。不得把这个限制改掉用来操作用户数据库。

复验要点：无 AI 建专题／资料／备注 → 绑定规则副本 → 首次简报与引用 → 列表多选加入新证据＋正文修订 → 非法引用／取消保留旧基线 → 重试增量 → 历史旧引文与当前正文不同 → 正常退出重启 → 原生 Markdown 导出。另验证旧库升级、归档／恢复、人工归组、移除、正文抓取失败及删除保留原数据。只有实际完成的步骤可记为 native 通过。

实际检查和平台边界见 [专项报告](../../docs/audits/topics-2026-10-26.md)。受控 Provider 验证请求／保存／失败协议，不能代替真实模型的证据支持率、转载识别和简报质量评估。

## 2026-10-27 自动选文漏选与无信息重试

基线 b93753a + 保留的自动简报 / 文章式阅读修改及本轮修复，Windows / PowerShell 7.6.7。M4W-026 待验证。旧实现的两个新回归先实际失败：描述中的旧文不进入发现窗口，及第一批空选阻止检查后续可选文章。

当前发现仍是有界词法检索，不能视作任意主题的全库语义搜索：最近 1,000 篇 + 允许范围内最多 1,000 篇匹配 + 保留资料；名称 / 描述、中文双字和有限中英别名用于排序，短词 / FTS 均参数化，且继续使用原来源 / 日期 / 标签条件。后部相关片段补充开头预览，不增大每篇 1,000 字符上限；总结仍使用完整可用正文。

一次点击空选最多继续 3 批，每批最多 60 篇且按预算缩小，一旦成功生成即结束。每批检查与成功基线独立，后批失败不撤销前批已确认的无信息结果，但失败批不确认；只重试失败总结时复用有效选文。持续无信息但仍有候选时显示剩余数量；已耗尽时可显式重新检查未用于成功简报的版本。此操作再次请求 AI，保留成功快照、已读、排除反馈；普通刷新及无变化生成仍零请求。recheck 是现有 generate_topic_brief 的可选布尔参数，旧调用省略仍兼容。

实际通过：

- ./tests/reader_pipeline/run_rust.ps1 db::topic_：41 个入口（3 个本轮未启用的可选 fixture），新增 8 项覆盖旧文 / 说明 / 中英安全主题、中文片段与来源约束、后部 UTF-8 预览、跨批成功、三批上限、显式重查保留成功 / 已读 / 反馈、后批失败缓存重试。
- cargo check --locked --offline --manifest-path src-tauri/Cargo.toml、cargo fmt --manifest-path src-tauri/Cargo.toml -- --check、npm.cmd run lint、npm.cmd run build、node tests/accessibility/check_resources.mjs（596 / 596）。
- run_auto_browser.mjs 14 组 / run_browser.mjs 10 组。沿用各 runner 的命令格式和此前 Rust fixture：自动 tmp/topics-auto-native-20261026/final-ui-fixture.json，手动 tmp/topics-native-20261026/ui-fixture.json；外置运行时为 $env:TEMP/mercury4win-settings-layout-runtime/node_modules/playwright-core/index.mjs，浏览器为 C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe。输出分别在临时目录 mercury4win-auto-topics-7U46xL / mercury4win-topics-iJr5xj。新断言验证无信息后显式 recheck=true、剩余候选不误报耗尽及按钮可用。

当前 Windows Tauri / WebView2 仅只读核对剩余候选提示和可用生成入口；没有执行原生真实重查、消费真实模型额度或读取日常凭据。完整原生重查 IPC、用户原主题的选文质量、大型库性能和其他平台未验证，不把受控回复视为模型质量通过。全 library 未重跑，旧 socket 超时失败保留。

## 2026-10-27 移除 token 预算与重复段落字段兼容

基线 b93753a + 之前全部未提交修改及本轮修复，Windows / PowerShell 7.6.7，D-012 / M4W-027。默认自动写作 Prompt 为 topic-article-v4。旧解析器由新增合成测试实际复现 duplicate field passage_id；没有读取用户原始响应，行列不视为精确复现条件。

预算设置、输入估算缩批、完整正文预算延后、手动模式正文 / 上下文总字符预算及自动 / 手动固定 max_tokens 已移除。旧 input_budget 列 / DTO / IPC 参数为兼容保留，保存忽略旧参数，旧缓存身份也保留；不会清空用户历史或重置检查。继续保留发现窗口、每批候选 / 选文数量及原文引用结构校验。服务端实际容量限制仍可能返回错误，不假装发送了被截断的全文。

重复 passage_id 仅在同一 entry_id 引用对象中规范化：同号合并，多有效号展开，冻结原文和 UTF-8 位置逐个验证；quote / null 与多个编号混用、其他重复字段、未知字段、非法编号、错误类型和超数量仍失败。不会让 JSON 末值覆盖掩盖前一个非法编号。失败不保存 / 不确认，仍有效选文可复用。

实际通过：

- ./tests/reader_pipeline/run_rust.ps1 db::topic_：45 个入口（3 个本轮未启用的可选 fixture），新增重复字段 / 拒绝边界 / HTTP 缓存重试 / 超长手动上下文 4 项。自动长文测试实际比较完整 passages 拼接与冻结全文；手动 HTTP 请求 >160,000 字符也逐字一致；两类请求都不带 max_tokens，旧小预算值不缩减候选。
- cargo check --locked --offline --manifest-path src-tauri/Cargo.toml、cargo fmt --manifest-path src-tauri/Cargo.toml -- --check、npm.cmd run lint、npm.cmd run build、node tests/accessibility/check_resources.mjs（593 / 593）。
- 自动浏览器 14 组、旧专题浏览器 10 组。命令及 fixture / 外置 Playwright / Edge 路径与上一节相同；本次临时截图目录为 mercury4win-auto-topics-GwcwDj / mercury4win-topics-jF2Wc4。编辑器不再有 spinbutton，原有名称校验正常，保存不带 inputBudget；原文章式简报、来源、反馈、取消与布局回归保留。
- 当前 Windows Tauri / WebView2 实际打开新主题高级设置，确认预算字段移除，Escape 取消，无保存 / 生成操作。

真实 Provider 原回复重试、服务端长上下文兼容及其他平台未验证，M4W-027 保持待验证。没有使用真实额度或日常凭据，未重跑完整 library，历史失败保留。
## 2026-10-27 持续生成进度

基线 b93753a + 此前全部保留修改及本轮进度增强，Windows / PowerShell 7.6.7。进度是阶段与已知事实，不是服务端内部写作百分比：自动准备 / 选文 / 写作 / 校验保存四阶段，手动省略选文；相同 Channel 增加 batch、selection 和 saving，保留旧 phase / coverage / task_state 协议。

当前批次、候选 / 采用数、仅摘要资料数与缓存复用来自后端实际值。每秒计时只说明总等待和当前阶段时长，20 秒后提示等待服务返回；不增加 AI 请求、不估算完成时间、不显示生成模型名称。计时器卸载清理，事件仍按请求 / 文章主题归属隔离；刷新失败也清除活动进度与 busy。进度区允许收缩与内部滚动，生成时隐藏重复静态候选说明，保留取消按钮。

实际通过：

- npm.cmd run lint、npm.cmd run build、node tests/accessibility/check_resources.mjs（608 / 608）、cargo check --locked --offline --manifest-path src-tauri/Cargo.toml、cargo fmt --manifest-path src-tauri/Cargo.toml -- --check。
- ./tests/reader_pipeline/run_rust.ps1 db::topic_：46 个入口，含 3 个未启用的可选 fixture。新增 production HTTP + 实际 Tauri Channel 双层 JSON 回归：失败无 saving、候选 7 / 7、选中 3 / 元数据 1、缓存重试顺序 selecting → summarizing → validating → saving，仍共 3 个 HTTP 请求；既有多批测试核对 1 / 2 / 3 批事件。
- run_auto_browser.mjs 16 组 / run_browser.mjs 10 组（命令、fixture、外置 Playwright / Edge 路径同前节）。新增阶段 / 批次 / 缓存计数、真实每秒计时和 20 秒等待提示、4 个中英宽窄进度布局、完成 / 取消 / 卸载 / 迟到事件、刷新失败清理、手动三阶段。临时截图 mercury4win-auto-topics-F1lrYA / mercury4win-topics-GS4fI9；最终中文 560×500 截图已实际查看，进度卡完整且取消可见。

首次 Rust 测试回调类型编译失败后修正 InvokeResponseBody 解包；首次窄窗截图提示裁切后修正布局，两者都未作为最终通过证据。完整 Tauri 原生生成、真实 Provider 与其他平台本轮未执行；浏览器只替换 IPC，不能证明 native 传输端到端。未新增真实 AI 用量或改动日常数据库 / 凭据，旧问题和完整 library 失败基线保持。