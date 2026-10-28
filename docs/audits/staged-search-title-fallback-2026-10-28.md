# 第13批标题判断中断修复（2026-10-28）

基线81f7b2e，加全部保留的未提交修改；Windows / PowerShell7.6.7 / WebView2。M4W-032，原库恢复尚待验证；M4W-031的超时修复及此前失败报告保持。

## 此次实际故障

只读原库扫描状态、阶段计数和请求事实，并检查最新Mercury.log。新扫描固定609篇，16:12:18～16:19:34（北京时间）发出13次search_titles，HTTP全部成功，各18.558～49.049秒；前12批192篇保存成功（175篇进入preview，17篇rejected_title），第13批16篇标记invalidJudgment，标题尚余417篇。没有新的120秒超时。

原始模型回复没有落盘，日志只证明“正常HTTP后标题判断校验失败”，不能断言具体字段/缺项/陌生ID原因。未将日常正文/笔记用于测试或写入报告。原库同次扫描仍保留进度，未通过SQL清空或改写。

## 修复

标题初筛只是排除明显无关内容的优化。正常HTTP后的判断无法校验时，丢弃整批模型判断，将原始冻结ID保留到内容检查；不接受其相关/排除结论、未知ID或引用，不自动重试，也不缓存失败判断。来源版本与内容/整篇证据校验继续执行，失败标题不会被计为有效模型初筛。

staged_search.rs保留严格解析器，生产调用在标题校验失败后调用staged_search_store::defer_titles。写入遵守当前拥有者/取消提交围栏，title_model=2记录转内容；旧0/1含义兼容，不变Schema10。Progress.title_fallback可缺省，旧快照可读。前端单独显示转内容的文章数量，i18n双语同步。日志只加json_container/fields_or_types/id_membership_or_count/decision_or_reason等固定原因码及数量，不打印文章、用户查询、模型正文或凭据。

真实合成复测首个规划请求另返回9个合法英文同义词，被原8项边界拒绝；仅将每个OR组容许值扩大到16，保留术语长度64、总字符3500、语言/实体/未知字段/硬条件校验，17词仍拒绝。原始合成交换追加protocol_cases.json；没有因此删改硬条件或用词法截断标题池。

修改：agent/staged_search.rs、db/staged_search_store.rs、agent/topic_search.rs及其tests、staged_search/tests.rs/live_tests.rs、NaturalFilter、topic_search.ts、双语资源、DOM/loopback脚手架与文档。

## 离线与原生证据

- staged_search定向11项通过（付费入口默认忽略），原topic_search 6项通过；生产DOM4组通过，含转内容警告、暂停/恢复/旧响应围栏及不可变范围。
- 新609篇回归在第13个loopback响应中用999999替换一个ID，HTTP正常。旧流程会invalidJudgment；新流程保留原16篇进入预览，不新增陌生文章、不重发本批、不缓存失败判断。最终609标题全部处理、12预期命中、无剩余/错误，title_fallback=16。
- lint、frontend build、cargo check --locked、cargo fmt -- --check通过；独立Tauri debug完整构建通过，已有大chunk提示。未重跑全library，旧4项socket失败基线保持。实现中曾把title_model按bool读取，与新增三态不符导致编译失败，已改为i64并重跑回归。
- Computer Use独立identifier/609合成库/18763loopback实际一次提交到完成，第13批注入异常，界面显示16篇转内容；609/609标题、591模型初筛、26/26块、12匹配、0错误/剩余、1资料不足。57次HTTP全成功（understanding1/titles38/previews5/full7/final6），无额外重试、未知ID写入、密钥文件、笔记外发或外键错误。
- 隔离应用正常退出、Provider停止，18763无监听。native构建验证标题转内容及界面，先于后来的9同义词兼容；后者由原始交换Rust和真实规模测试单独验证。截图/UIA曾不一致，重新激活目标并读取一致状态后才进行输入，没有保存截图。

证据：[原生结构化结果](staged-search-title-fallback-2026-10-28-native.json)、[UI文字](staged-search-title-fallback-2026-10-28-ui.txt)。

## 真实API

现有主模型ecnu-plus，仅读AI路由表和选中一份凭据到内存，业务库完全合成。原用户文章/笔记未用于测试。

首轮1请求/1,213token，HTTP成功但规划9词触发边界校验，保留[失败报告](staged-search-title-fallback-2026-10-28-live.json)及[逐请求记录](staged-search-title-fallback-2026-10-28-live.requests.jsonl)。

609篇首扫35次实际HTTP，34成功/1Provider失败，49,862已报token，另失败用量未知；529标题已保存，97篇尚未最终完成，耗时1,147.27秒。原失败没有细分日志，不能追溯断言具体HTTP状态/传输原因；现已补只含固定类别/有效状态数字的failure_class日志。

严格匹配原system/user输入回放34个成功合成响应，并单独重建1次已记录Provider失败，随后用生产store::claim执行一次显式恢复。回放不发HTTP、不计新增Token；已记录失败只恢复provider错误状态，没有伪造未知原始状态码。恢复仅新增20次真实HTTP/42,266token，全部成功；最终609标题、607有效模型初筛、26/26资料块、12预期命中，漏选/误选0，错误/剩余0，资料不足1。此真实规模样本title_fallback=0；标题失败转内容由前述故障注入原生/609回归验证，不能声称本次真实响应实际触发了它。

本任务共56次实际HTTP/93,341已报token，另1次失败用量未知；包括首个规划业务失败。恢复运行708.822秒包含回放，不能当成从零扫描性能，也不是最后一次从零无中断通过609篇。未重新扫描用户原库。

证据：[609首扫失败](staged-search-title-fallback-2026-10-28-large.json)、[逐请求](staged-search-title-fallback-2026-10-28-large.requests.jsonl)、[显式恢复检查点](staged-search-title-fallback-2026-10-28-resume.resume.json)、[恢复后完成](staged-search-title-fallback-2026-10-28-resume.json)、[新增20次HTTP](staged-search-title-fallback-2026-10-28-resume.requests.jsonl)。

原开发进程在本轮中已退出。根target构建期间遇到文件占用os error32，独立target完整默认identifier构建通过；最新构建已启动至原应用数据目录，原192/609标题检查点通过UI载入并显示恢复按钮。仅读取/载入未触发原库AI请求，日常实例保留运行；测试服务与隔离实例已退出。

这只是本查询的规模复测，不是本轮全七主题重测；一次通过不保证任意模型/文章均不会出现网络、用量或内容校验错误。

## 复验入口与边界

```powershell
$env:CARGO_TARGET_DIR="$PWD/tmp/topic-search-native-20261028/target"
./tests/reader_pipeline/run_rust.ps1 staged_search
./tests/reader_pipeline/run_rust.ps1 agent::topic_search::tests
node tests/topic_search/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
```

明确授权的真实规模复验使用M4W_STAGED_CASE=models_zh、M4W_STAGED_SKIP_BASELINE=1、M4W_STAGED_LARGE_SCOPE=609，调用tests/staged_search/run_live.ps1并指定新的ReportPath；默认范围仍71篇。追加538篇合成体育负例，预检JSONL记录实际范围；不与71篇旧回放混用。

原用户库同次扫描待恢复验证，M4W-032保持待验证。其他平台、所有硬条件组合、超长资料及任意Provider稳定性未验证；正文/最终结果不因标题降级而降低证据标准。未提交、推送、发布、改依赖或创建新聊天/子代理。
