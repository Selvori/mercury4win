# M4W-010 标签系统回归

2026-10-13；HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加保留的 M4W-001、016、005、008、006、011 /019、017、003、002、004、009、007 及本项未提交修改。Windows / PowerShell 7.6.5、Rust 1.97.0 GNU、Node v24.12.0、临时 Happy DOM 20.14.6。上游只读参考 README、tags-v2、batch / management 合同、TagNormalization、TagLibraryStore、TagBatchExecution、TagAliasBypass / TagLibraryStore 测试。没有访问用户数据库、凭据、真实 AI 或远程订阅。

## 实现与范围

- 每个写入 / AI 输出 / 本地建议 / 搜索使用同一 canonical 规范化（trim、lowercase、连续 `-_.` / 空白折叠），先直接匹配，再解析别名。匹配沿用 canonical 显示名，重复赋值不增加计数；人工接受提升 provisional，并保留人工关联来源。
- 合并验证 source / target、禁止自合并，在一笔事务内迁移关联与 confidence、去重、保留人工来源及 source 名称 / 别名，重算未软删除文章的计数。别名冲突不会偷取别人的 canonical；任何失败回滚。
- schema v4 添加别名显示名、反向关联索引和独立 batch staging 表；原 v1/v2/v3 不被当作当前版本改写。旧规范化碰撞合并到确定的 canonical，保留关联、别名及人工永久状态，重复迁移幂等。当前兼容规范化不会建立层级标签。
- 标签库新增合并确认预览、别名添加 / 删除、临时标签提升、未使用清理及错误反馈，Settings 的标签库与 Sidebar 使用同一组件；写后刷新 entries / feeds / tags / entry_detail / 本地建议 / 相关文。Settings 合并 / 删除也同步 AppShell 的选中标签；合并保留目标范围，最多5标签策略不变。
- 无 AI 时标题 / 摘要提供粗略本地建议，另可选正式常用标签；所有 chip 都必须由人工点击才写入。共标签相关文章最多5篇，可点击切换 Reader；无 OS 专用 NLP 依赖。本地抽取是便携式词语 / 已有标签匹配，不宣称等同 macOS NLTagger 质量。
- `batch_tag` 已替换错误的 timeRange / tag_ids 固定赋值接口；Rust 注册、TypeScript wrapper / 类型、生产调用者同步。开始前预览相同筛选契约的数量与冻结日期，支持最近7/30/90/180/365天、全部、仅未读、独立跳过已有标签 / 已应用条目；超过100篇必须显式勾选确认，不静默截断。
- 批量必须明确配置 tagging primary model，默认内层并发3、可选1..5。外层使用已有 Tagging runtime 许可（每类1active +1latest waiting），许可内加载配置 / 凭据；单篇 / 批量共用实际 tagging 执行器及冻结 Prompt / 主备用 provider 请求。未引入脱离 runtime 的后台任务。每篇失败积累并继续，成功用量保留现有 tagging 记录与实际 provider/model；完整请求级失败用量仍属于 M4W-012。
- 生成只写 staging，已有匹配与新提议都不直接写 tag / entry_tag。逐提议 Keep / Discard 完整决策后，用户再点击确认应用；陈旧决策集合变化拒绝并刷新审核。取消关闭当前 HTTP、停止后续请求，已完成结果保留可审核，未开始条目仍可重试。
- 开始 / staging 事务使用 runtime CommitFence。Prompt 的标题、作者和正文与来源 hash 从同一 SQLite snapshot 读取；hash 另覆盖 URL / Feed 内容和 Markdown 版本。Apply 重新解析当前 canonical / aliases，跳过已删除或来源改变的文章，返回计数；重新运行此类文章可再次分析。
- 活动 batch 禁止重命名 / 合并 / 删除 / 别名维护等破坏性操作（UI 与 DB 双重保护）。批量 Dialog 的关闭 / Escape 和 Settings 的 Escape / 路由 tabs 在生命周期内锁定；显式 Stop / Discard 保留退出途径。进程或组件中断留下 running 时，恢复查询只在对应 ID 不在 runtime active / waiting 时转 ready；保留审核，避免自动重发付费请求。

当前 Apply 使用**整次原子事务**及幂等完成结果，失败 / 中断不会留下部分 tag 关联；尚未采用上游分块提交 / 分块进度或自动续跑生成。应用期间按钮禁用、完成后显示新增关联和删除 / 变更跳过数量。成功清理建议正文，仅保留轻量条目标记供 skip_applied；取消但未分析的行不会误标为已应用。生成恢复到审核，而非自动重发尚未完成的请求。

## 复现命令

在仓库根目录运行，Rust sidecar 仅写 ignored target 构建产物；DOM 不增加永久项目依赖。

```powershell
& tests/reader_pipeline/run_rust.ps1 -Filter 'tag_'
& tests/reader_pipeline/run_rust.ps1 -Filter 'db::migrations::tests'
& tests/reader_pipeline/run_rust.ps1 -Filter 'agent::runtime::tests'
& tests/reader_pipeline/run_rust.ps1 -Filter 'm4w_009'
node tests/tag_system/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/agent_runtime/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
$env:M4W_DETAIL_WIRE_PATH = Join-Path $env:TEMP 'm4w-019-detail-wire.json'
& tests/reader_pipeline/run_rust.ps1 -Filter 'm4w_011_tests'
node tests/entry_filters/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:M4W_DETAIL_WIRE_PATH"
Remove-Item Env:M4W_DETAIL_WIRE_PATH
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm.cmd run lint
npm.cmd run build
rustfmt --edition 2021 --config skip_children=true --check src-tauri/src/db/tag_store.rs src-tauri/src/db/tag_store_tests.rs src-tauri/src/db/tag_batch_store.rs src-tauri/src/agent/tag_batch.rs src-tauri/src/agent/tag_batch_tests.rs src-tauri/src/commands/tag_cmd.rs src-tauri/src/agent/tagging.rs src-tauri/src/db/migrations.rs src-tauri/src/db/models.rs
git diff --check
```

## 实际结果

最终 `tag_` 13项通过（本项12项，另包含011已有标签删除1项）：

1. separator / Unicode 空白规范化、alias canonical 复用、去重 / 人工提升和无效空名称。
2. 合并 source / target 关联去重、人工来源、confidence、别名和软删除排除计数。
3. namespace 冲突触发事务回滚，自合并 / 目标不存在拒绝。
4. v3→v4 规范化碰撞、旧别名显示与关系、计数、永久状态、重复升级和外键检查。
5. 生产异步维护：alias 搜索、rename 冲突 / 旧名保留、清理、无AI本地建议、相关文章。
6. staging 不污染标签表、未完成提议不允许 Apply、一次完成 / 重复 Apply、skip_applied。
7. discard 提议、来源变更 / 删除时拒用陈旧 AI 建议。
8. 审核中人工 canonical / alias 变化，陈旧审核被拒绝；当前 resolver 应用到 canonical。
9. scope 共享筛选、skip/read/date、冻结条目集、batch 期间维护阻止、显式 discard 解锁。
10. 实际 loopback tagging + runtime：第1篇完成，第2请求等待时取消，观察 socket 关闭，没有第3请求；保留1篇审核，Apply 后剩余2篇仍可重试。恢复查询不误终止活动ID。
11. 实际主备用 + custom Prompt：并发峰值2，3篇中2成功 /1失败、10次真实HTTP（5次backup），失败逐篇继续，别名与 separator 提议去重；2条成功用量记录实际 Backup/model。生成期间无 tag / entry_tag 新写入。
12. 临时 SQLite 文件重开，生产恢复 helper 将孤立 running 转 ready；缺失的预登记取消 ID 返回空；Apply 后再次重开保留标签和计数。

最终生产 DOM **10组**、退出0：无模型本地 / 手工且无AI意图，相关文章真实 wrapper / 点击，alias冲突及合并预览与确认，慢恢复门禁与冻结scope / count / progress，显式审核Apply，取消部分审核 / discard，恢复后陈旧审核重置，失败恢复重试，无显式模型禁用，Settings合并映射实际AppShell筛选，Settings Escape / route-tab 生命周期锁（部分并入同组）。测试运行真实 React / Query / Zustand / 实际 IPC wrapper；只替换 native transport，没有声称真实Tauri IPC运行。

前项复测：001原15组DOM、003原8组DOM /8项Rust、011原6组DOM /4项Rust、009原7项Rust、迁移原4项均通过。最终 cargo check通过（6条既有警告）、lint / build通过（既有>500KiB chunk提示）、9文件定向Rust格式通过。FeedList既有嵌套button提示仍出现，未在010扩展修复。未跑全套Rust /全仓fmt；既有Prompt版本和Reader标题断言及全仓格式失败保留M4W-018。

开发准备及中间失败保留：初次check发现usize不能直接作为SQLite参数 /读取类型；test编译发现临时Client借用；fake响应缺role导致预期完成数量失败；首次DOM结果提交后cleanup与HappyDOM.close竞争；011 DOM缺少实际JSON fixture参数；001旧mock缺新增只读查询导出、随后发现单篇成功应失效原文章详情而非只发全局key；临时DB关闭的异步清理曾触发Windows共享文件删除失败。均修正后上述最终命令通过：同步mock仅补空只读查询，保留原15项断言；单篇保持原文章key，关联查询广泛刷新；临时DB测试显式关闭自己的SQLite连接后删除。

## native 桌面关闭条件（未执行）

在专用测试用户 /隔离应用数据下验证，使用临时Feed数据与loopback fake provider，不对真实用户库测试：

- 无AI时手动输入 /本地chip /正式标签、相关文章切换；跨文章延迟写入和错误归属。
- alias冲突、rename、merge确认、临时提升、cleanup、Sidebar与Settings的选中筛选 /计数，窄窗口与键盘。
- batch scope预览 /冻结数量、>100确认、1..5实际并发、waiting /进度 /逐项失败、Stop关闭HTTP、不发剩余请求，Keep /Discard后确认应用。
- tag维护生命周期锁；Settings /batch的Escape、关闭、焦点行为；旧审核来源改变 /文章删除时跳过。
- 完整退出 /重启后恢复staging与计数，保留已完成审核且不自动发AI；Apply失败回滚 /幂等重试。

本轮没有启动完整Tauri / WebView2、真实Provider /AI或其他平台，也没有用临时pool重开代替完整桌面重启验收。M4W-010保持“待验证”。
