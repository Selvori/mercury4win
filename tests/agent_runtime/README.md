# M4W-003 实际 AI 调度与取消回归

2026-10-12；Windows、PowerShell 7.6.5、Rust 1.97.0 GNU、Node v24.12.0。基线为 HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加保留的 M4W-001、016、005、008、006、011 /019、017 和本项未提交改动。上游只读：README、summary-agent 的无隐式取消规则、translate-agent §2.5、AgentRuntimeEngine 与对应测试。

## 实现契约

- 每个类型（summary / translation / tagging）最多 1 个 active + 1 个 waiting；新的 waiting 取代旧 waiting。执行许可包围模型路由、凭据读取和完整 agent future，等待不执行这些操作。
- 每次前端意图生成独立 request ID。终态、旧取消和等待替换均按 ID 校验；同文章不同参数也是独立请求。Channel 新增 `task_state`（request_id / state / reason），保留原有 chunk / segment / progress / error / done。
- `cancel_agent_task(task_type, request_id)` 通知实际 future 退出；scope 先丢弃全部 HTTP / buffered 分段 future，再释放 active、提升 waiting。成功、错误、路由失败由同一 RAII guard 清理；外部 drop / panic 也清理。
- `abandon_agent_waiting(task_type, request_id)` 仅撤销等待；已获 active 许可的任务继续。未登记请求 ID 的撤销保存 5 分钟，覆盖本地 IPC 乱序。切文章 / 关闭面板不会显式取消 active。
- 前端展示等待 / 运行 / 取消状态和取消按钮。返回运行中的原文章时，通过 `get_agent_status` 的 current_entry_id / current_request_id 查看后台任务并取消原 ID；无需再发一次生成请求。后台取消失败保留实际运行状态，自然完成后恢复控件。后台输出恢复 / 持久化留 M4W-002。
- 翻译单段 error 不再提前解锁运行控件；以 invoke 终态作为调度结束。并发翻译聚合错误与分段 / 双语语义仍属 M4W-004，未在本项改写。

## 复现命令

```powershell
& tests/reader_pipeline/run_rust.ps1 -Filter 'agent::runtime::tests'
& tests/reader_pipeline/run_rust.ps1 -Filter 'm4w_017_tests'
node tests/agent_runtime/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm.cmd run lint
npm.cmd run build
python tests/agent_runtime/check_format.py
git diff --check
```

Rust 复用 008 的 Common Controls 6 sidecar 与局部 PATH 脚本，仅写忽略的构建产物。DOM 使用此前安装在临时目录的 Happy DOM 20.14.6，不修改项目依赖。用内存 SQLite 和 `127.0.0.1` HTTP fixture，未读取用户数据库或凭据，未请求真实 AI。

## 实际结果

8 项 Rust 定向测试全部通过（部分测试循环三种任务）：

1. 三类生产 agent 分别验证 A active、B waiting、C 替换 B：B 返回 replaced_by_latest，放行 A 前实际请求数为 1，最终只发 A+C 共 2 次；Channel waiting / running / cancelled、终态 idle 正确。
2. 三类 active 取消后实际 HTTP socket 关闭，再执行等待 B；旧 A finish / cancel 不释放 B。摘要已收到真实 SSE chunk 后取消，取消任务没有 done。
3. 六段翻译，串行度 1 / 并发度 3：取消关闭全部已发出的 1 / 3 个 socket，剩余段无请求，状态 idle。
4. waiting 显式取消、离开撤销、撤销先于 submit：不发 HTTP；active 离开仍可成功。
5. 真实空配置模型路由 `resolve_agent_route` 返回错误后释放，waiting 开始唯一 HTTP 请求；生产摘要 HTTP 500 错误也释放 / 推进。
6. 三种任务互不阻塞，实际同时发出三次请求。
7. 外部 abort / executor panic 后 RAII 清理并提升 waiting；旧等待终态不影响最新项。
8. 空 ID / 重复 ID 拒绝，waiting 和 active ID 状态投影正确。

8 组生产 React 组件 + **实际 TypeScript IPC wrapper** 的隔离 DOM 交互通过，仅替换 native transport：等待 / 离开 / 提升、显式取消和当前 ID、旧回调隔离、运行面板关闭、翻译段失败仍 pending、标签 Channel / 取消、错误与伪造生命周期 ID、返回原文章取消后台任务、后台取消失败后自然完成可重试、登记前取消无 Channel 仍显示终态。不是 native Tauri IPC 或 WebView2 验收。

原 M4W-001 的 15 项生产 ReaderPane DOM 回归通过；其中“单段 error 后同文章重试”改为先等待原 invoke 的真实终态，新增断言未结束时按钮仍禁用，保留旧回调隔离测试。M4W-017 原 12 项 SSE / agent / Channel 回归复测通过；仅将其 fixture runtime 初始化改为新 `create_runtime()`。

cargo check 通过，10 条既有警告；lint / build、定向格式及 diff 检查通过。格式检查覆盖 runtime / tests / summary / models / 017 fixture 全文件，以及 agent_cmd 本项路由、调度 / 取消和 get_status；保留无关 Provider / 模型管理段的历史格式。没有重排全仓。

扩大执行 `& tests/reader_pipeline/run_rust.ps1 -Filter 'agent::'`：31 项中 30 通过、1 失败，既有 `prompt_templates::tests::test_load_builtin_tagging` 期望版本 1、实际 2，留 M4W-018，未修改断言或模板。全套 Rust 与全仓格式未运行。

初次测试准备曾失败：标签状态栏误用 is_loading 导致 TS 错误；Rust 测试局部变量遮蔽 channel 函数；DOM 把带 `Error:` 前缀的取消当错误；001 旧测试假设段失败立即解锁。修正后按上述命令重新通过。父代理复核发现后台取消失败后 phase 可能停留 running，已改为由后台 status 投影控制并补自然完成回归，实际通过。

未验证：完整 native Tauri / WebView2 的 Channel 序列化、等待 / 取消按钮和布局、真实 provider / AI、Linux / macOS。服务端已收到的请求可能继续服务端计算；本项证明客户端连接 / 本地任务取消，不能控制远程 provider 内部调度。M4W-003 保持“待验证”。
