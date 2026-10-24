# M4W-002 摘要与翻译持久化回归

2026-10-12；Windows、PowerShell 7.6.5、Rust 1.97.0 GNU、Node v24.12.0。基线 HEAD `c90567b` 加保留的001、016、005、008、006、011 /019、017、003和本项未提交改动。上游只读参考：README、summary-agent / translate-agent、TranslationStorage及存储测试。没有访问用户数据库 / 凭据或真实AI。

## 存储与恢复契约

- schema v3 新增 `agent_result_slot`，保留原有未使用结果表及旧数据。未知来源的旧表数据不自动恢复，避免将旧结果套在当前来源上。新建、v1升级、v2升级及重复迁移通过。
- 摘要身份为 entry / target language / detail level / source hash；翻译额外使用 prompt strategy、实际 concurrency及 `markdown-paragraph-v1` 分段版本。来源标识由标题、作者、URL、Feed summary / 正文 / 类型、Markdown / 版本及cleaned HTML / readability版本组成的长度明确JSON数组生成，避免来源未重建时误复用。来源变动仍保留旧记录，但读取不匹配，旧任务保存也被拒绝。
- 当前 model name 是结果溯源字段，未进入slot；模板版本、自定义Prompt及model route指纹还未接入，后续M4W-009须扩展parameters并同步读取身份。通用表的parameters字段允许扩展；未来004采用HTML稳定分段必须更换segmenter版本。没有声称本项已完成004的稳定p / ul / ol映射或双语显示。
- 摘要累计完整SSE delta，只在合法DONE、非空内容且数据库COMMIT成功后发done。翻译每个成功段保存checkpoint；全段成功再原子promote完整success。失败、空响应或取消不替换旧完整success；有旧完整结果时优先恢复它。没有完整success时，“继续翻译”只请求缺失段，串行模式恢复前段上下文。
- `agent_result_store::load_summary` / `load_translation` 是生产读取入口，已通过注册IPC / TS wrapper接入面板，也可供后续文摘导出使用。面板重开按参数读取；后台完成、失败、取消及Reader正文重建完成时刷新查询，旧读取和旧Channel不得覆盖当前文章。
- Request对应CommitFence，经task-local传给存储。取消 / RAII释放使fence失效；已经发出的SQLite interact可以继续执行，但事务COMMIT之前必须通过fence。fence锁只包围COMMIT且不获取runtime锁，runtime取消拿runtime→fence锁，SQLite路径不会反向拿runtime锁。先COMMIT后取消保留成功，先取消的旧事务回滚，不能覆盖新结果。

## 复现命令

```powershell
& tests/reader_pipeline/run_rust.ps1 -Filter 'agent_result_store::tests'
& tests/reader_pipeline/run_rust.ps1 -Filter 'db::migrations::tests'
& tests/reader_pipeline/run_rust.ps1 -Filter 'agent::runtime::tests'
& tests/reader_pipeline/run_rust.ps1 -Filter 'm4w_017_tests'
node tests/agent_persistence/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/agent_runtime/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm.cmd run lint
npm.cmd run build
python tests/agent_runtime/check_format.py
rustfmt --edition 2021 --check src-tauri/src/agent/provider.rs src-tauri/src/agent/translation.rs src-tauri/src/db/agent_result_store.rs src-tauri/src/db/migrations.rs
git diff --check
```

Rust复用008的Common Controls6 sidecar脚本，只写忽略的构建产物。DOM复用临时安装的Happy DOM20.14.6，无依赖升级。

## 实际结果

7项本项Rust通过：

1. 摘要语言 / detail、翻译策略 / concurrency的不同slot；checkpoint→完整success；新partial不覆盖旧success；Feed来源变化使读取失效，旧保存拒绝。
2. 独立临时文件DB，释放pool并重新创建pool后摘要与翻译checkpoint恢复。
3. 生产摘要+本地HTTP完整中文 / emoji的两次delta落库；HTTP500不覆盖原success。
4. 生产翻译串行1 / 并发3，部分失败不发done，成功段落库；下一次仅发1次HTTP补缺段；新失败不覆盖完整success。
5. 实际execute与已派发interact：事务写入后阻塞，显式取消丢弃async owner，再放行COMMIT，旧写回滚；COMMIT已完成后取消不删除新成功。
6. 模拟v2现存结构升级v3，保留旧摘要、笔记与文章；重复迁移安全。
7. 生产execute / translation取消保留已完成checkpoint，下一次仅补缺段；生产summary取消保护原成功。

7组生产组件+真实TS wrapper的隔离DOM通过：fresh QueryClient重开恢复；失败 / 取消恢复原成功；detail选择读取既有slot且不自动生成；旧读取 / 旧流隔离和后台完成返回恢复；翻译checkpoint / 完整结果恢复；来源变化拒绝旧结果；生产ReaderPane先查到空结果、正文延迟重建后重新读取。

M4W-003原8项Rust / 8组DOM、M4W-017原12项Rust、M4W-001原15项DOM复测通过。旧001mock新增load接口，并捕获真实chunk / segment、在成功Promise后保存虚拟结果；保留原归属断言，未删除或放宽测试。

本项初次测试准备失败及修正：翻译TS类型名替换未匹配；Rust channel局部变量遮蔽函数及临时结果借用；001旧mock返回null导致成功展示丢失；取消内存pool替换连接造成“no such table”；Windows临时DB清理遇到仍在释放的文件句柄。分别修正后重新执行通过；取消恢复测试使用独立临时文件DB，清理对Windows32错误最多等待2秒。DOM早期一处未放在act内的fixture通知已修正，无生产修改。

完整native Tauri / WebView2的结果读取IPC、桌面关闭重开 / 重启、真实AI / provider、Linux / macOS尚未验证。DOM是transport替换测试，pool重开是Rust持久化验证，不等同完整桌面重启。全套Rust与全仓格式未运行；既有018失败保留，M4W-002保持“待验证”。
