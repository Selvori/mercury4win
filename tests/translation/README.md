# M4W-004 翻译分段、双语、失败与恢复回归

初始实现与隔离回归：2026-10-12。2026-10-13 的说明、侧栏调整与后续静默翻译 / 漏译修正分别记录在下方日期节。基线 HEAD `c90567b` 加保留的全部未提交修复；Windows / PowerShell7.6.5、Rust1.97.0 GNU、Node v24.12.0。未访问用户数据库 / 凭据或请求真实AI。

上游只读参考：README、`docs/features/translate-agent.md`、TranslationContracts、TranslationSegmentExtractor、PerSegment Execution及对应提取 / 双语 / 存储测试。

## 当前实现契约

- 从生产Reader的安全Markdown→HTML渲染结果提取p / ul / ol、h1–h6、th / td / caption / figcaption / dt / dd；引用内部的正文 / 列表照常翻译。选中的外层列表、单元格 / 定义块拥有内部文本，不重复提取后代；空图片段不占段索引，pre代码块和非正文区域保留原文。内联代码以反引号包围保留语义。
- source ID基于段类型、有效顺序、渲染HTML及来源文本的SHA-256。相同输入得到相同映射；不同来源被来源hash拒绝。slot使用`reader-html-block-v3`，旧v2及`markdown-paragraph-v1`结果保留但不命中新身份。AgentResult的segment_ids / segment_errors使用serde默认值，旧schema v3摘要仍可读，无新增数据库迁移。
- 实际并发统一1..5；0默认3，超过5归一为5。每段非空响应且未明确报告非stop终止才成功。HTTP错误、空响应及length截断按段失败；串行失败仍处理剩余段，全部请求收尾后才发complete / partial / all_failed，完整结果COMMIT后才发done。
- 成功段和错误原因保存checkpoint。继续翻译只补缺段；source ID单段重试仅发送指定缺段；来源变化拒绝旧ID。串行恢复前一段原文 / 译文上下文，HY-MT内置模板版本2新增同样条件入口。完整旧success优先恢复，失败再生成不能覆盖它；取消沿用002的提交fence。
- `load_translation_document`返回来源映射和已标记Reader HTML；run_translation新增可选sourceSegmentId。保留017的error.segment，额外提供index / source_segment_id；TS封装、Rust注册和所有调用方同步。
- 译文以textContent插在对应原文块之后，表格译文放在对应单元格 / caption内，不解释provider输出HTML。使用当前Reader head保留主题 / 字体，iframe仍为`allow-same-origin`沙箱；不写原文HTML持久缓存。应用中点击翻译 / 按T进入模式直接启动或恢复，不显示控制条 / 进度 / 状态；完整缓存不生成，失败不自动重试，显式重新选择才继续。切文章回原文模式，旧回调按文章隔离。

## 复现命令

仓库根目录，使用已有临时Happy DOM20.14.6，不修改应用依赖。GNU Rust测试复用008的Common Controls6 sidecar / 局部PATH脚本，只写忽略的构建产物。

```powershell
$env:M4W_TRANSLATION_FIXTURE = Join-Path $env:TEMP 'mercury4win-translation-fixture.json'
try {
    & tests/reader_pipeline/run_rust.ps1 -Filter 'translation_segments::tests'
} finally {
    Remove-Item Env:/M4W_TRANSLATION_FIXTURE
}
node tests/translation/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/mercury4win-translation-fixture.json"
& tests/reader_pipeline/run_rust.ps1 -Filter 'agent_result_store::tests'
& tests/reader_pipeline/run_rust.ps1 -Filter 'agent::runtime::tests'
& tests/reader_pipeline/run_rust.ps1 -Filter 'm4w_017_tests'
node tests/agent_persistence/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/agent_runtime/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm.cmd run lint
npm.cmd run build
python tests/agent_runtime/check_format.py
rustfmt --edition 2021 --check src-tauri/src/agent/translation.rs src-tauri/src/agent/translation_segments.rs src-tauri/src/agent/translation_tests.rs src-tauri/src/db/agent_result_store.rs src-tauri/src/db/agent_result_store_tests.rs src-tauri/src/agent/provider.rs
git diff --check
```

Rust fixture由生产load_document输出到指定临时JSON，包含四个有效p / ul / ol来源段及原始代码 / 引用 / 图片 / 嵌套列表。DOM使用该真实序列化结构，仅替换native transport；临时SQLite重开是持久化验证，不等同完整桌面重启。

## 2026-10-12 实际结果

本项6项Rust通过：

1. 相同HTML稳定ID / 顺序；列表内部p / 嵌套列表不重复，空图片段过滤，代码块 / 引用 / 表格排除，内联code保留，内容变化改变ID。
2. concurrency0 / 99规范化为3 / 5，新segmenter参数身份与旧版不同。
3. 串行1 / 并发3：HTTP500、空响应、显式length截断全部失败且不发done；错误checkpoint恢复；仅请求第二段得到partial1/3；再仅发两个缺段请求得到完整成功。
4. 临时文件数据库释放pool后重开恢复段ID / checkpoint；单段重试使用前段已存上下文；来源改变后不恢复旧结果且拒绝旧ID，不发送请求。
5. 七段、concurrency99：实际HTTP同时最多5请求；未放行前没有第六请求或终态；全部七请求收尾后partial6/7，六个成功段、七次progress、无假done。
6. HY-MT实际串行HTTP请求包含前段原文 / 译文；旧摘要JSON默认字段兼容；生产document生成完整DOM fixture。

本项6组生产组件 / 实际TS wrapper DOM通过：

1. 原文p / ul / ol后紧接对应译文；嵌套列表不重复，原代码 / 引用 / 图片保留；provider HTML按字面显示，当前主题 / 字体head保留。
2. checkpoint恢复为来源配对；仅缺段有重试按钮；真实wrapper发送正确camelCase sourceSegmentId / requestId；错误来源ID拒收，单段失败不会提前解除pending。
3. all_failed / partial / complete明确区分；清空QueryClient后重开恢复完整结果。
4. 切文章拒收旧sources / segment / failure；取消恢复checkpoint并拒绝取消后的旧输出。
5. 生产ReaderPane收到双语HTML，保留原sandbox；关闭翻译回原文，切文章同样回原文。
6. 来源读取失败显示错误，主按钮和空状态按钮均禁止启动；显式重读恢复控件，未隐式发送生成请求。

必要复测通过：002原7项Rust / 7组DOM、003原8项Rust / 8组DOM、017原12项Rust、001原15项DOM。001测试新增文章切换后原文模式断言，再显式打开翻译，保留原有旧回调 / pending断言。已有transport增加新document读取契约，未删除或放宽原断言。

初次失败与修正：Rust新IPC注册漏完整模块路径、文本借用类型不符；并发HTTP fixture只读一个TCP块导致偶发RST，修正为按Content-Length收齐完整请求；017复测发现遗漏旧error.segment，恢复兼容字段后12项全部通过。早期lint的依赖数组警告通过稳定空数组及memo修正。

## 2026-10-13 收尾检查

本日补齐说明、核对ISSUES / STATUS / WORKLOG及证据链接，保持M4W-004“待验证”。实际通过：cargo check（9条既有警告）、npm lint / build、003定向格式脚本及004的六个Rust文件rustfmt检查、git diff --check。build有Tailwind插件耗时提示但构建成功。本日没有重跑Rust / DOM测试或扩大范围，10月12日结果保留在前节。

## 未验证范围与009交接

native Tauri / WebView2实际IPC、双语布局 / iframe滚动、完整桌面重启、真实AI / provider、Linux / macOS尚未验证。当前按完成段更新完整srcDoc，未实现token级DOM增量patch。全套Rust / 全仓格式未运行，历史018失败仍保留；未提交 / 推送，未开始009。

009扩展身份时需同步生产生成与读取：`agent_result_store::translation_parameters`当前含segmenter / strategy / 归一concurrency，ResultSlot另包含entry / language / source hash；`translation_segments::load_document`用于生成与document恢复；`agent_cmd::load_translation_result` / `load_translation_document`及run_translation读取profile。内置模板入口仍为`prompt_templates::load_builtin_template("translation", prompt_strategy)`，模型路由仍为agent_cmd的resolve_agent_model / resolve_agent_route。009应统一模板 / route指纹，并同步读取身份和前端查询key，避免只更改请求而继续恢复旧slot。

## 2026-10-13 翻译不显示侧边面板

基线：HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加全部既有未提交修复与本次前端交互调整；Windows / PowerShell 7.6.5、Node 24.12.0、已有临时 Happy DOM 20.14.6。没有修改 Rust、依赖、IPC、用户数据库或凭据，没有调用真实 AI。

ReaderPane 仅为笔记 / 摘要 / 标签显示侧栏；翻译控制位于正文上方，保留启动、状态、进度、取消、错误 / 读取重试与带段编号的缺段重试，对照原文 / 译文只在 iframe 内显示。控制区高度受限且可滚动。再次点击翻译回原文，切文章回原文、主题字体及旧回调隔离沿用原契约。

实际通过下列命令：

```powershell
npm.cmd run lint
npm.cmd run build
node tests/translation/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/mercury4win-translation-fixture.json"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/reader_theme/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/mercury4win-theme-fixture.json"
git diff --check
```

翻译 6 组：扩展既有 ReaderPane 组，实际检查按钮 / `T` 均不显示侧栏或分隔条、无 iframe 外重复译文、笔记仍有侧栏、缺段重试发送正确 sourceSegmentId、进度 / 段错误 / pending、取消恢复已存对照结果、返回原文和切文章。文章归属 15 组：翻译断言改为同时检查外部控件与 iframe srcdoc，保留原有旧回调拒收断言。主题字体 3 种初始状态 × 8 组通过，共计 45 组。复用已存在的生产 Rust 临时 fixture，本次没有重跑 fixture 生产器或 Rust 套件。build 保留既有 >500kB chunk 提示。

未验证 native Tauri / WebView2 排版、控制区滚动 / 焦点和实际 IPC / Provider，以及其他平台。桌面验收应检查无缓存启动翻译、已有完整 / 部分结果、段失败重试与取消、从笔记切换翻译、`T` 开关、窄窗口、主题字体和切文章。

## 2026-10-13 静默直接翻译与漏译范围

基线 HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加全部既有未提交改动与本项；Windows / PowerShell 7.6.5、Rust 1.97.0 GNU、Node 24.12.0、Happy DOM 20.14.6。此前“正文上方控制条”版本被本次用户明确要求替代，历史验收结果保留。

根因：原遍历只选择 p / ul / ol，并显式排除 blockquote / table，标题未进入请求；该问题发生在 Provider 调用之前。现在加入 h1–h6、引用段落 / 列表、表格单元格及 caption / figcaption / dt / dd，选中外层拥有内部文本以免重复。代码块、非正文区域和无文字图片仍不翻译。表格译文在 th / td / caption 内追加，防止 iframe HTML 重解析把 div 移出表格；dt 译文包装为合法 dd，provider 输出仍使用 textContent。

缓存身份升 `reader-html-block-v3`，旧 v2 完整成功不能继续冒充覆盖所有文字的新结果，记录保持原样。生产 Rust 验证先保存旧 v2 success，再确认新版不读取它、实际对全部 10 个来源发送 HTTP 请求、收齐 10 个 segment、持久化完整结果并仍能读取旧 slot；fixture包含标题、作者、段落、嵌套列表、引用和四个表格单元格。

ReaderPane 挂载无 UI 的 automatic 翻译控制器，选择 / T 后先等待实际 profile、fingerprint、来源 / 结果读取、Reader 重建和后台状态刷新；完整结果直接展示，部分结果由既有后端只补缺段。延迟 effect 清理避免 StrictMode 重放重复请求，每次激活至多一次启动；失败 / refetch不自动循环，原文→翻译的显式再次选择可重试。后台仍运行时重开不提交重复意图，文章切换 / 退出等待沿用既有归属与调度。翻译控件 / 进度 / 状态 / 错误均不在应用阅读区显示。

实际通过：

```powershell
# 保留并恢复 M4W_TRANSLATION_FIXTURE，指定为下方临时 JSON
& tests/reader_pipeline/run_rust.ps1 -Filter 'translation_segments::tests'
& tests/reader_pipeline/run_rust.ps1 -Filter 'agent_result_store::tests'
cargo check --locked --manifest-path src-tauri/Cargo.toml
rustfmt --edition 2021 --check src-tauri/src/agent/translation_segments.rs src-tauri/src/agent/translation_tests.rs src-tauri/src/db/agent_result_store.rs
npm.cmd run lint
npm.cmd run build
node tests/translation/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/mercury4win-translation-fixture.json"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/reader_theme/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/mercury4win-theme-fixture.json"
node tests/agent_runtime/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/agent_persistence/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
git diff --check
```

Rust 7 + 7 = 14 项；DOM 10 + 15 + 24 + 8 + 7 = 64 组。新的15来源提取边界覆盖所有标题等级、引用、单元格嵌套、图注 / 定义及正文外区域；生产请求 / 缓存测试如前。DOM检查实际 Rust 10来源 fixture、表格重解析安全与无重复、button / T 直接启动、缓存免调用、StrictMode一次、流式 / 完成展示、失败无循环、部分恢复、文章隔离、后台重开、失败来源不调用、延迟语言 / 来源 / Reader、主题字体及已有 runtime / persistence契约。没有移除或放宽旧回调拒收断言。check零warning；build保留已有 >500kB chunk提示；所有上述最终检查通过。

真实API调用已由用户授权，但本项根因可通过生产分段、loopback Provider HTTP、真实临时 SQLite 和 DOM重解析完整验证，因此没有读取实际凭据 / 数据库或调用真实付费API。未验证 native Tauri / WebView2 真实IPC / 排版 / 滚动、具体真实Provider的长段输出与目标文章、其他平台或完整全仓套件；桌面验收应覆盖点击 / T、标题 / 引用 / 表格、长文、窄窗口、切文章 / 重开和无AI配置。
