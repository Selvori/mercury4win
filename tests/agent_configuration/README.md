# M4W-009 配置执行与自动摘要回归

日期：2026-10-13。基线：HEAD `c90567b` 加保留的001、016、005、008、006、011 /019、017、003、002、004及本项未提交改动。Windows、PowerShell 7.6.5、Rust 1.97.0 GNU、Node v24.12.0；没有访问用户数据库、凭据或真实 AI。

## 实现与边界

- Configuration一次读取模板 / 路由 / 翻译策略与归一并发，在003的active许可内补凭据、执行整个agent。有效自定义文件优先于Standard / HY-MT；自定义版本为summary 1、translation 4（两种策略共用）、tagging 2。验证YAML字段、已声明变量、条件闭合，拒绝嵌套 / 重复段；上传先验证并原子替换，无效文件不覆盖旧模板。外部编辑导致模板无效时，内置模板生效并通过Channel notice / 设置错误提示显示原因；内置模板无效直接错误。
- 主 / 备用必须存在、启用、所属Provider启用并支持该任务；保存与执行均检查。未配置主模型时选择已登记的兼容模型，无兼容模型明确失败，不猜模型名称。路由配置错误不发送请求，也不触发请求备用。
- 请求失败、空结果或非stop结束会尝试配置备用一次。摘要部分流在备用前发送reset，清除当前尝试的片段；翻译按失败段备用，成功段独立保留。存储错误不触发备用，取消直接丢弃future。完整成功才覆盖旧成功；翻译checkpoint及来源检查继续沿用004 /002。
- ResultSlot统一把配置指纹加到参数身份，涵盖有效模板、实际选择的主 / 备用Provider ID / URL / 模型ID / 名称及共享请求参数；凭据不入指纹。translation_parameters的segmenter / strategy / 归一concurrency和language /source身份继续存在。生产运行、三个读取IPC、load_document均处于相同配置snapshot；前端summary_result / translation_result / translation_document查询key加入get_agent_fingerprint结果，设置变更失效该query。
- 旧slot不删除；生产当前身份不自动命中无指纹slot。`load_latest_summary(entryId)`只读当前source匹配的已有完整成功，不需要配置、凭据、网络。返回`{target_language, detail_level, configuration_fingerprint, result}`或null，供007导出；模型 / Provider停用或归档后仍可读，来源变化返回null。旧schema_v3结果可反序列化，新增溯源字段有默认值。
- 自动摘要遵循上游“任何已有摘要都不自动生成”的语义，并展示其语言 /详细度。与当前配置不同的历史摘要明确标注，可手动重新生成当前配置；没有把历史记录视为当前slot命中。保存默认详细度用于初始面板，延迟profile不覆盖用户临时选择。自动启用需确认费用 /正文发送提示；等待正文ready及持久化读取完成后1秒防抖，切文章 /关闭清timer及waiting；现有active不隐式取消。失败显示且不自动重试。
- 为012保留`RequestRoute {provider_id, provider_name, model_id, model_name}`：摘要存`AgentResult.request_route`，翻译逐段存`segment_routes`，混合模型的顶层model_name为mixed。执行参数与指纹共享一个入口。当前用量仍以任务汇总成功请求tokens为主，失败请求 /多维度 /请求粒度留012。

## 命令与实际结果

```powershell
& tests/reader_pipeline/run_rust.ps1 -Filter m4w_009
node tests/agent_configuration/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
& tests/reader_pipeline/run_rust.ps1 -Filter translation_segments::tests
& tests/reader_pipeline/run_rust.ps1 -Filter agent_result_store::tests
& tests/reader_pipeline/run_rust.ps1 -Filter agent::runtime::tests
& tests/reader_pipeline/run_rust.ps1 -Filter m4w_017_tests
node tests/translation/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:TEMP/mercury4win-translation-fixture.json"
node tests/agent_persistence/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/agent_runtime/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm.cmd run lint
npm.cmd run build
python tests/agent_runtime/check_format.py
```

- 本项7项Rust通过：实际summary /translation /tagging请求使用custom且无system时不添加额外文案；串行上下文保持；模板优先 /拒绝与旧文件保持；partial摘要reset /backup成功及主备用皆失败保留成功；逐段backup实际模型溯源；配置能力 /启用检查、无模型错误；模板 /模型 /Provider指纹失效、旧slot边界 /归档独立读取及source变化拒绝。
- 取消实测两个边界：主请求等待响应时取消，捕获1次请求；主失败且backup等待响应时取消，捕获2次请求；两种均由生产runtime取消生产summary，返回cancelled、loopback服务观察socket关闭，没有后续请求。未证明远端Provider自身停止生成 /计费，也未运行真实Provider。
- 8组DOM通过：生产Settings能力 /启用过滤与backup保存、默认值 /延迟加载临时选择、partial reset、自动确认 /1秒 /切换防抖、串行waiting /取消边界、已有历史结果投影 /读取失败不生成 /失败不自动重试、延迟正文ready、配置指纹查询key。原004 6组、002 7组、003 8组、001 15项DOM复测通过，共36组；使用生产组件和真实TS wrapper，仅替换原生transport。
- 原004 6项、002 7项、003 8项、017 12项Rust复测通过，共33项。cargo check通过（6条既有警告）；lint /build、10个本项Rust文件定向格式及修改的agent_cmd段 /原003检查、git diff --check通过。未运行全套Rust /全仓格式，018历史失败仍保留。
- 初次新增Rust fixture使用错误Runtime构造器、函数遮蔽及Debug要求导致编译失败，修正后通过；一次文本替换误伤match语法导致cargo check失败，修正并定向格式后通过。没有以失败或未运行记录为通过。

## 待验证

未运行native Tauri / WebView2：真实文件对话框上传 /替换、实际Channel与自动摘要、历史提示 /布局、完整桌面重启；真实Provider /AI /费用、Linux /macOS也未验证。DOM正文延迟测试验证ReaderSummary的ready入口，ReaderPane传递实际正文ready条件已静态核对，完整桌面抓取延迟仍待验收。因此台账保持“待验证”。本项未提交 /推送，未启动007。
