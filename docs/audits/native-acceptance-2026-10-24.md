# Native 验收：2026-10-24

本次验收跨 2026-10-24 / 2026-10-25（Asia/Shanghai）继续；下列代理 / UI 事件使用 UTC。

## 基线与环境

- 基线 `4b652fb`，开始时 `git status --short` 为空；Windows、PowerShell 7.6.7，完整 Tauri debug 应用及系统 WebView2。所有 UI 操作通过 Computer Use `@oai/sky`，截图和可访问树均真实取得。
- 使用临时 Tauri override：identifier `com.mercury4win.acceptance20261024`，窗口 `Mercury Acceptance 2026-10-24`。启动后先观察空库无订阅，实际数据库新建于 `%APPDATA%/com.mercury4win.acceptance20261024/mercury.db`，WebView2 数据位于 `%LOCALAPPDATA%/com.mercury4win.acceptance20261024/EBWebView`，均独立于现有应用目录。
- 真实用户库仅通过 SQLite `mode=ro` 读取 Provider / Model / Agent Profile 必要字段，复制 2 / 2 / 3 条配置及 2 个不透明凭据文件；不复制订阅、文章、笔记或历史结果，不输出端点和凭据。隔离库只加入 1 个合成 Feed、文章 `10001`（A / COBALT_BALLOON）和 `10002`（B / ORANGE_SUBMARINE）。
- `npm.cmd run build` 通过，保留已有 chunk 大小提示；随后 `npm.cmd run tauri -- build --debug --no-bundle --config tmp/native-acceptance-2026-10-24/tauri.acceptance.json -- --locked --offline` 通过。没有安装包、发布或提交操作。
- 2026-10-25 恢复时核对：旧隔离应用和 18741 代理均已退出，合成库 / 笔记 / 摘要证据完整；仅重新启动已有验收 exe 和透传代理，未重做已通过项目。系统 WebView2 文件版本 `155.0.4283.62`。

## M4W-001：真实桌面归属验收

本节在实际操作后逐项记录。早期 Happy DOM 记录保持原样，不作为本次 native 结果。

| 路径 | 实际操作及结果 |
| --- | --- |
| A 草稿→B 保存→A | A 输入 `A_DRAFT_COBALT` 合成草稿，切 B 显示空笔记，输入 `B_SAVED_ORANGE` 并保存；回 A 草稿完整保留。隔离库只出现 B 的笔记，A 没有被错误写入。通过。 |
| 笔记关闭重开 | A 草稿关闭到阅读模式，再开笔记，原草稿完整保留。通过。 |
| 保存失败及重试 | 仅隔离库创建 `BEFORE INSERT` 触发器，A 保存由实际 IPC / SQLite 返回 `M4W001 synthetic save failure`。UI 显示保存失败并保留草稿；关闭重开保留错误和文本。删除该测试触发器再点击保存成功，错误消失，A/B 各自正确落库。通过。 |
| 真实 AI 直连 | 现有配置直接生成 A 摘要成功，使用量 689 token；该次响应在切 B 前已完成，因此不计入迟到回调证据。 |
| 摘要迟到隔离 | 仅隔离 Provider 路由接本机 `127.0.0.1:18741` 透传代理，上游仍为原真实 Provider、默认 TLS 校验；不伪造内容，真实响应取得后延迟 20 秒再返回。A 运行时切 B，B 启动摘要并等待；A 完成后 B 仍正确处于执行中，未出现 A 文本。B 最终只显示 ORANGE_SUBMARINE 摘要。两次请求均 HTTP 200，使用量 698 / 659 token。通过。 |
| 摘要关闭重开 | B 请求执行中关闭摘要、重开显示后台执行；请求完成后显示 B 的内容。通过。 |
| 静默翻译迟到隔离 | 2026-10-25，A 点工具栏翻译，立即切 B；同一次 A 任务的 7 段真实服务响应经延迟代理陆续完成，B 始终显示自己的英文 ORANGE_SUBMARINE 正文，没有 A 译文、进度或错误。隔离库仅 A 的 translation 槽 `complete=true`、7 段且无 checkpoint，B 没有 translation 槽。7 次请求均成功，共 1642 token。通过。 |
| 静默翻译关闭重开 | 回 A 点翻译显示正确的钴蓝气球中英双语；点阅读恢复原文，再点翻译恢复双语。用量仍为原 7 次，没有新请求。现有静默模式没有翻译侧栏，本次按生产工具栏路径验收。通过。 |
| 标签迟到及输入隔离 | A 标签输入 `A_MANUAL_ONLY_COBALT` 而不提交，点击智能推荐；真实上游 HTTP 200 后延迟 20 秒回传。执行中切 B，输入框为空，B 显示本地 Native / ORANGE / SUBMARINE。A 真实请求成功后，B 仍保持这些项、输入为空，无 A 建议 / 错误 / loading；没有申请 B 的 AI 请求，也没有向任一文章添加标签。该次使用 626 token。通过。 |
| 标签关闭重开 | 在 B 上点阅读关闭标签再重开；旧 A 请求完成后再次观察 B，仍为自己的本地建议与空输入。通过。 |

代理阶段事件（UTC，时间来自本地透传器）：A 请求 `2026-10-24T12:09:28.907Z`，真实上游完成 `12:09:31.830Z`，A 下游完成 `12:09:52.056Z`；B 请求 `12:09:52.049Z`，真实上游完成 `12:09:55.929Z`，B 下游完成 `12:10:16.133Z`。B 在 A 完成前已选中并提交等待。本时序是受控传输延迟，未声称原网络自然迟到。

翻译第一批请求 `2026-10-24T16:09:57.517Z`，最后一段下游完成 `16:10:59.432Z`；B 完成后观察 `16:11:38.379Z` 仍为自己的正文。标签请求 `16:12:42.918Z`，上游完成 `16:12:43.584Z`，下游完成 `16:13:03.605Z`；B 最终观察 `16:13:37.721Z` 保持空输入及自己的本地建议。持久证据为[脱敏事实与逐请求时间](native-001-2026-10-24-25-evidence.json)和[原生可访问树摘录](native-001-ui-2026-10-24-25.txt)，过渡截图均已在 Computer Use 返回时实际检查。树在过渡时可能落后截图，摘录采用稳定观察，不把过渡时的旧树误认为新 UI 状态。

M4W-001 的最低关闭条件已满足，2026-10-25 标为已关闭。本次没有改产品代码；累计 11 次真实请求 / 4314 token（3 摘要、7 翻译分段、1 标签）。只关闭文章归属问题；取消、队列、跨重启完整恢复、错误协议、标签写事务以及其他平台仍由各自问题继续验收，不从本次通过推导关闭。

准备阶段曾遇到两类操作问题：按错误大小写 / 目录猜测源码路径，检查实际 `rg --files` 后改用真实路径；不带截图的可访问元素点击返回 `coordinate input geometry is unavailable`，重新取得带截图状态后正常操作。一次 PowerShell 内联 Python 的引号解析失败，在执行 SQL 前即退出，改用 UTF-8 临时脚本后完成故障注入；Python 控制台中文输出曾乱码，改为显式 UTF-8 后核对，应用 UI 与合成数据库文本正常。这些没有计作产品失败。

## 可复用环境与范围

- 临时文件在仓库忽略目录 `tmp/native-acceptance-2026-10-24/`：Tauri override、`seed.py`、隔离库检查及故障开关 `state.py`、真实 Provider 延迟透传器 `delayed_proxy.py`、每次原生 UI 观察的 UTC / 可访问树 `native-001-ui.txt` 和不含端点 / 头 / 正文的 `proxy-events.jsonl`。
- 原生可执行文件为 `src-tauri/target/debug/mercury.exe`，本次由 override 构建；其内嵌当前生产 dist，不需要 Vite 端口。再次构建普通配置会替换此产物，因此后续启动前需核对窗口标题与数据目录。
- `state.py fail-note` 仅在隔离数据库创建故障触发器；`state.py allow-note` 删除该触发器。故障注入已恢复。
- 收尾运行 `python tmp/native-acceptance-2026-10-24/delayed_proxy.py restore`，只恢复隔离库的原真实路由；已停止唯一延迟代理并核对 18741 无监听，所有代理请求都已结束。隔离数据库中测试触发器不存在，路由与只读来源比对相等；[事实文件](native-001-2026-10-24-25-evidence.json)仅保存布尔结果，不含配置内容。
- 隔离应用保留运行在 B 标签面板，可供下一串行项接手；无 Vite 服务或端口。后续通过 `sky.list_windows()` 重新获取唯一标题窗口，不能依赖本代理 JavaScript 变量或句柄。Node REPL 工具为可发现的 `mcp__node_repl__js`，直接 `await import('@oai/sky')`；当前验证会话中的 `sky` 可用，但新代理应重新初始化、列窗、观察再操作。
- 最终 `git diff --check`、新增 JSON 解析及文档严格 UTF-8 检查通过。本次只新增验收记录及更新状态，不重跑 Rust / DOM 全套或重新声明历史检查为当日通过。
- 后续项目按原顺序继续，本阶段只处理 M4W-001。尚未扩大为 Linux / macOS、安装升级或其他问题的关闭证据。
