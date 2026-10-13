# M4W-012 请求用量与 Provider / Model 生命周期回归

日期：2026-10-13。基线：HEAD `c90567b` 加此前串行修复（001、016、005、008、006、011 /019、017、003、002、004、009、007、010）及本项未提交改动。平台：Windows、PowerShell 7.6.5、Rust 1.97.0 GNU、Node 24.12.0、Python 3.14.3。未提交 /推送；上游只读，无真实数据库、凭据或 AI 请求。

## 实现与计数契约

- schema v5 给现有 `llm_usage_event` 增加幂等 event_key、文章 /实际 Provider /model ID、名称快照、去凭据 /query /fragment 的真实 endpoint、阶段、开始 /结束及用量可用性。每个实际发起的 HTTP 尝试一条事实：摘要主备用、翻译逐段 /重试、标签重试及批量每篇均经过相同 Provider 边界。正常 /fallback /retry 分开记录；未发网络的排队取消、非法 request preflight、结果读取和连接测试不计入本报表。
- HTTP 完整响应 /终止及 Provider completion 校验决定请求状态 `succeeded /failed /cancelled /timedOut`，与任务结果的成功、标签解析、checkpoint /最终 COMMIT 是不同生命周期。例如合法 HTTP completion 携带 Token，但标签业务解析失败，仍保留该次请求的 succeeded 与 Token；请求成功率不能当作任务成功率。失败 HTTP 如有 usage 也保留；SSE 显式 `stream_options.include_usage=true`，断流前已有 usage 不丢。
- 未报告 /部分 usage 的字段为 NULL；真实 0 保留为 0。缺任一 Token 字段为 missing，完整为 actual，不估算缺失值。报表 Token 是已报告值的和；覆盖率与 missing 数单独展示。总 Token 不用输入 /输出之和伪造。
- 升级前聚合事件保留原字段，并标为 legacy。其准确 HTTP 次数与旧 0 的可用性不可恢复，因此独立显示旧事件数 /旧记录 Token；不混入新的准确请求数、状态比例和已知 Token。没有伪造历史身份或删除原历史。
- RAII guard 在取消时丢弃 HTTP future 后只派发有限 SQLite 写任务。正常完成 await 有限写任务；日志不使用结果 CommitFence、不保留 socket /凭据、不获取 runtime 锁。取消发生在 HTTP 完成而 SQLite 写尚未结束时，保留已确定的 HTTP 终态；重复派发用 event_key 幂等。独立轮询 DB 写避免 `buffer_unordered` 暂停请求 future 持池租约、checkpoint 又等待单连接的死锁。测试 flush 等待已派发写；生产写失败记录日志及会话计数，报表明确提示可能不完整。强杀进程 /runtime 立即退出时尚未完成的取消日志没有持久化保证，未宣称已实现退出 flush。
- Provider 删除 IPC 改为事务归档并级联模型；Model 删除改归档，保留 profile 外键、历史、凭据。活跃 Settings 查询及实际路由排除归档对象，禁用也排除执行；配置指纹 /models /usage 缓存随维护失效。已冻结的 active 请求按其快照完成，新请求重新解析。Provider 创建匹配规范 endpoint +原名称才复活原 ID，未提供新 Key 沿用原 Key，UI 明示；不同名称作为新账户身份，不继承旧 Key。Provider 复活不自动复活模型；Model 按同 Provider +model_name 复活。新建 /复活能力选择实际传入，显式值生效，旧 IPC 不传时新建全支持 /复活保留原值。
- 共享报告使用 UTC 滚动 `[from,to)`，按请求开始时间筛选，日桶也是 UTC；兼容旧 SQLite 日期与有 offset 的 RFC3339，先用索引粗界再精确时间比较。Provider /Model 以实际 ID 分组，更名前后不拆组，当前对象名优先、缺对象才用快照；归档历史默认保留并标注。7 /14 /30 /90 天、Provider /Model /Agent、三种对象的预过滤入口、请求 /输入 /输出 /总 Token 指标比较、UTC 每日图、表格与质量指标使用同一次 SQLite 读事务。无成本估算或新增保留策略。

## 实际命令与结果

```powershell
pwsh -NoProfile -File tests/reader_pipeline/run_rust.ps1 -Filter m4w_012
pwsh -NoProfile -File tests/reader_pipeline/run_rust.ps1 -Filter 'agent::runtime::tests'
pwsh -NoProfile -File tests/reader_pipeline/run_rust.ps1 -Filter m4w_009
pwsh -NoProfile -File tests/reader_pipeline/run_rust.ps1 -Filter 'agent::tag_batch::tests'
pwsh -NoProfile -File tests/reader_pipeline/run_rust.ps1 -Filter m4w_017_tests
pwsh -NoProfile -File tests/reader_pipeline/run_rust.ps1 -Filter 'db::migrations::tests'
```

最终结果：本项13项通过；必要回归003的8、009的7、010批量的7、017的12、迁移4项通过。helper 只为 GNU 测试 executable 写入 ignored target 下 Common Controls 6 sidecar manifest；未修改产品权限。

本项实际边界覆盖：摘要 SSE 完整 usage /缺失 /usage 后错误，全部Provider归档后历史摘要仍可独立读取且不追加请求；翻译2段主备用共4请求且单连接无死锁；标签前2次 empty +第3次成功 /业务解析失败保留 Token；1 active +1 waiting 取消只有 active 1条，socket 关闭；翻译3并发取消只有3条且余2段未启动；HTTP read_timeout；部分 Token 与真实0；HTTP完成后、已派发 DB 写前取消的幂等记录；v4升级及重复运行保留旧行；归档 cascade /触发器失败回滚 /原 ID复活 /不同名称新身份 /禁用路由；创建 /复活显式能力及旧调用兼容；带时区边界、归档历史、改名前后同ID聚合；写失败计数及非法 preflight 0条。010批量真实10请求的统计断言同步为10条（2成功、8失败），含5条备用尝试。

生产 DOM 使用另装临时 Happy DOM（不修改依赖 /锁文件）：

```powershell
$env:M4W_USAGE_FIXTURE = Join-Path $env:TEMP 'm4w-012-usage-fixture.json'
pwsh -NoProfile -File tests/reader_pipeline/run_rust.ps1 -Filter m4w_012_report_history
python tests/usage_lifecycle/build_fixture.py $env:M4W_USAGE_FIXTURE
node tests/usage_lifecycle/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/agent_configuration/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
```

本项8组生产 UsageStats /AgentSettings /真实 wrapper DOM通过；原009的8组通过。实际 Rust DTO fixture用于历史 /归档 /NULL及legacy语义；只 mock 传输，覆盖范围 /维度 /指标排序 /图、读失败刷新 /写失败提示、归档级联清单 /取消 /失败保留 /重试 /成功后活跃列表刷新、Provider /Model /Agent 上下文过滤、禁用与不可用模型可见性、新增表单能力确实传入并进入兼容模型 picker、旧报告迟到不覆盖新范围。归档错误反馈原在新增表单内不可见，本回归发现后已移到全局反馈。

```powershell
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm.cmd run lint
npm.cmd run build
rustfmt --edition 2021 --config skip_children=true --check src-tauri/src/agent/usage.rs src-tauri/src/agent/usage_tests.rs src-tauri/src/db/usage_store.rs src-tauri/src/db/agent_store.rs src-tauri/src/agent/provider.rs src-tauri/src/agent/summary.rs src-tauri/src/agent/translation.rs src-tauri/src/agent/tagging.rs src-tauri/src/commands/agent_cmd.rs src-tauri/src/commands/settings_cmd.rs src-tauri/src/db/migrations.rs src-tauri/src/agent/tag_batch_tests.rs
git diff --check
```

上述最终检查通过，check保留6条既有 warning，build保留既有 chunk >500kB提示。最后能力路径补全后重跑本项13 Rust /8 DOM及check /lint /build；再补摘要全部Provider归档后历史读取断言，单项定向通过；之前必要回归结果按当时记录保留，没有无必要扩大重跑。

## 中间失败及修正

1. 首次代码编辑脚本把 translation reconcile 的单闭括号误改成双括号；修正后 check 通过。测试初稿错误引用不存在的 `AgentRuntime::new`；改用已有 `create_runtime` 后编译通过。
2. 第一轮6项中5项通过，翻译请求 /日志在 max_size=1 的池租约与 buffer_unordered 暂停轮询下死锁。终止挂起测试，按上面的独立有限 DB 任务修正；加入10秒上限的真实回归后通过，没有以扩大池或延长超时掩盖。
3. 初次 DOM mock 在 Happy DOM 使用不存在的 structuredClone，随后断言误用“No providers” /“Agents”而生产英文为“No AI providers configured” /“Profiles”；修正 mock /预期。归档反馈不可见是实际生产问题，另行修复，不属于测试准备错误。
4. 必要017回归初次10通过 /2失败：本项错误诊断改写丢了原 Stream error 类别及含中文 /引号的 HTTP 错误正文。恢复兼容错误类别和正确 JSON消息，去掉原 Key片段 debug日志、正文中实际 Key字符串去敏；保留017原断言，随后12通过。首次 `runtime_tests` /`tag_batch_tests` 过滤器各运行0项，不能算通过证据；改用实际模块路径后8 /7项通过。

## 未验证与桌面验收

2026-10-13 M4W-018 补充可复现入口：`run_dom.mjs` 的第三个参数现在可直接传生产 Rust report JSON，bundle 内存加载该 fixture，无需调用 `build_fixture.py` 改写仓库 `fixture.ts`。不传参数仍沿用已登记的合成静态 fixture。完整临时生成和执行见 [最终回归入口](../final_validation/README.md)；上面的历史命令与结果保留。

未运行完整 native Tauri /WebView2 的真实 IPC、状态刷新 /布局 /焦点 /键盘、实际凭据文件读写失败、完整退出重启、真实 Provider 的 include_usage 兼容性与 Linux /macOS。未读写真实用户库 /Key；凭据仍为历史 XOR 混淆格式，不是安全加密 /Stronghold。未运行全套 Rust /全仓 fmt，历史018失败保留。

后续在独立测试环境用合成 Key 与本地 Provider 验收：三类实际操作与用量表逐条核对主备用 /逐段 /重试 /取消；归档前后历史保留和模型不可用、同名同 endpoint 恢复 /换名新账户、归档失败保留 Key；实际重启验证统计与恢复；无usage /实际0区分及各上下文报表 /筛选。满足这些路径后再按台账关闭，当前保持待验证。
