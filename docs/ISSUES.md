# 问题台账

最后更新：2026-10-26。所有初始问题均来自 [移植审查](audits/mercury-porting-2026-10-12.md)，当前进展见下表及对应验证记录。P1 优先处理影响数据或主要功能的问题，P2 为其他功能缺口 / 正确性问题；优先级变化不改变编号。

表中“依据”对应审查报告的章节编号；关闭时将证据链接、修复提交或工作区基线、验证日期附在本文件对应编号下。部分完成保留“处理中”或“待验证”，不要将整个问题提前关闭。任务较大可新增子问题，并在父问题中链接。

| 编号 | 优先级 | 问题 | 状态 | 依据 | 最低关闭条件 |
| --- | --- | --- | --- | --- | --- |
| M4W-001 | P1 | 笔记和异步结果跨文章归属 | 已关闭 | P1-01；[修复回归](../tests/reader_ownership/README.md)；[native 验收](audits/native-acceptance-2026-10-24.md)，2026-10-25 / 4b652fb | A 编辑→B→保存不写错笔记；旧 AI 回调不覆盖 B；桌面验证切换、关闭面板 |
| M4W-002 | P1 | 摘要 / 翻译结果未持久化 | 待验证 | P1-02；[持久化与恢复回归](../tests/agent_persistence/README.md) | 成功结果落库，关闭面板 / 重启后按参数恢复；失败和取消不破坏已有成功结果 |
| M4W-003 | P1 | AI 队列 / 取消未控制实际执行 | 待验证 | P1-03；[调度与取消回归](../tests/agent_runtime/README.md) | 实际请求遵守每类 1 active + 1 waiting；终态释放；显式取消中止任务；离开等待文章不再启动 |
| M4W-004 | P1 | 翻译失败误报成功及分段语义缺失 | 待验证 | P1-04；[翻译回归](../tests/translation/README.md) | 全失败 / 部分失败真实反馈；稳定 p / ul / ol 映射、双语显示、重试和恢复通过 |
| M4W-005 | P1 | RSS 短摘要误当全文 | 已关闭 | P1-05；[来源与回退回归](../tests/feed_content/README.md)；[native 来源 / F-001 验收](audits/native-005-2026-10-25.md)，2026-10-25 / 4b652fb + 未提交改动 | 区分全文 Feed 与 teaser，后者抓取全文；前者保留原文；固定样例与离线场景回归 |
| M4W-006 | P1 | 列表 100 条上限与游标漏项 | 待验证 | P1-06；[分页回归](../tests/entry_pagination/README.md) | UI 可访问后续页；乱序发布日期与同日文章不漏不重，总数稳定 |
| M4W-007 | P1 | 文摘导出不可用 | 待验证 | [文摘导出回归](../tests/digest_export/README.md) | 单 / 多篇模板通过，UI 可导出；标题、作者、URL、可选笔记 / 摘要符合约定 |
| M4W-008 | P2 | Reader 相对 URL、失败提示、版本与缓存 | 待验证 | P2-08；[Reader 回归](../tests/reader_pipeline/README.md)；[native 部分结果与工具中止边界](audits/native-008-2026-10-25.md) | 图片 / 链接正确解析，错误提示可见；分层版本重建与缓存命中通过且不放开不可信脚本 |
| M4W-009 | P2 | 自定义 Prompt / 备用模型及摘要默认设置未生效 | 待验证 | [配置执行回归](../tests/agent_configuration/README.md) | 实际请求使用自定义模板、正确备用路由、默认详细度；自动摘要行为实现并验证或明确另列范围 |
| M4W-010 | P2 | 标签合并、规范化、别名和批量 AI 不完整 | 待验证 | P2-10；[标签系统回归](../tests/tag_system/README.md) | 合并事务、规范化 / 别名去重、正确 IPC 与 AI 提议审核链路通过；本地建议和标签维护缺口有验收 |
| M4W-011 | P2 | 搜索 / 未读 / 组合标签 / 快捷键链路不完整 | 待验证 | P2-11；[查询与动作回归](../tests/entry_filters/README.md) | 过滤、All / Any、跨页批量已读、收藏入口和快捷键实际可用；删除后计数 / Reader 同步 |
| M4W-012 | P2 | 用量统计与 Provider 生命周期偏差 | 待验证 | P2-12；[请求事实与生命周期回归](../tests/usage_lifecycle/README.md) | 按真实请求与提供商记录成功 / 失败；多维报表正确；归档或删除失败不丢密钥 / 历史 |
| M4W-013 | P2 | Obsidian Publish 阅读分支未移植 | 待验证 | [静态Publish阅读回归](../tests/obsidian_publish/README.md) | 样例 shell 可解析 Markdown 地址，正文与缓存契约通过；或记录明确范围决策 |
| M4W-014 | P2 | Paper 主题入口和系统主题同步不完整 | 待验证 | 其他移植项；[主题与系统同步回归](../tests/reader_theme/README.md) | 可选择已提供主题，系统主题变化同步 Reader iframe，亮暗及字体桌面验证 |
| M4W-015 | P2 | 部分文案及按钮可访问性缺口 | 待验证 | 双语键 / 生产 DOM 与相关回归、[中文界面与持久化](../tests/localization/README.md) 通过；native 待验收 | 补齐双语键、按钮名称和键盘焦点；语言选择在完整退出重启后恢复，相关界面与中文文摘桌面验收 |
| M4W-016 | P2 | 笔记 Markdown HTML 净化缺失 | 已关闭 | [净化回归](../tests/markdown_safety/README.md)；[native 保存 / 重开 / 排版与完整重启](audits/native-016-2026-10-25.md)，2026-10-25 / 4b652fb + 未提交改动 | 明确允许内容并用恶意 HTML fixture 验证净化；普通 Markdown 不回归 |
| M4W-017 | P2 | SSE UTF-8 跨块及错误 JSON 编码风险 | 待验证 | [SSE 与错误事件回归](../tests/agent_stream/README.md)，已定向复现 | 字节分块中文、带引号 / 换行错误、终止与断流测试通过 |
| M4W-018 | P2 | 测试预期陈旧与格式基线未通过 | 已关闭 | [最终验证](audits/final-validation-2026-10-13.md)，2026-10-13 / c90567b + 未提交修改 | 真实语义断言及完整测试 / 格式基线通过，保留历史失败与明确格式范围；未删除或忽略测试 |
| M4W-019 | P2 | EntryDetail 前后端结构契约不一致 | 已关闭 | [契约回归](../tests/entry_filters/README.md)，2026-10-12 / c90567b + 未提交修改 | IPC 类型与实际嵌套结构一致，详情字段及调用方 / 不存在文章路径回归；可并入 M4W-011 |
| M4W-020 | P1 | 更换字体后 Reader 鼠标滚轮失效 | 待验证 | 用户反馈；[真实浏览器复现与回归](../tests/reader_scroll/README.md) | 完整 Tauri / WebView2 更换字体 / 字号 / 样式并关闭设置后，上下滚轮正常；快速连续切换、文章切换与翻译路径通过 |
| M4W-021 | P2 | 设置长内容挤压顶部导航，出现纵向滚动与裁切 | 待验证 | 用户截图；[复现与浏览器回归](../tests/localization/README.md#设置顶部导航高度修复) | 完整 Tauri / WebView2 切换六个选项，常规及矮窗口顶部语言 / 导航完整，长内容独立滚动，窄窗口末项仍可访问 |
| M4W-022 | P1 | 分段翻译完成后阅读位置重置到文章开头 | 待验证 | 用户反馈；[复现与回归](../tests/translation_scroll/README.md) | 完整 Tauri / WebView2 翻译中上下阅读，段落完成时保持可见内容位置；原文切换、缓存恢复、T 快捷键和文章切换通过 |
| M4W-023 | P2 | 文章工具栏长标题挤出操作按钮且宽窗口仍过早省略 | 待验证 | 用户截图；[布局验证](audits/reader-toolbar-2026-10-26.md) | 完整 Tauri / WebView2 调整阅读区宽度，标题按剩余宽度省略 / 完整显示，导出 / 浏览器 / 阅读设置按钮可见可用 |

## M4W-023 修复及验证记录

- 2026-10-26，`4b652fb` 加任务开始前保留的未提交改动；用户截图显示长标题在窄窗口挤出导出 / 浏览器 / 阅读设置，宽窗口又因 `max-w-md` 过早省略。标题容器缺少 `min-w-0`，无法按剩余空间收缩。
- ReaderPane 主阅读区及标题容器允许收缩，标题移除固定最大宽度并保留单行省略，悬停可查看完整标题；工具栏和左侧标签组禁止收缩，右侧沿用 Button 的 `shrink-0`。
- lint / build / diff 通过；实际 Edge 中英文 × 400 / 600 / 924 / 1181 / 1701px 十种阅读区宽度，八个按钮均完整可见且中心可命中；窄时标题省略，宽时完整。800px 打开笔记侧栏后主区 474px 无横向溢出，设置 / 文摘窗口打开及 mock 浏览器调用通过。[验证记录](audits/reader-toolbar-2026-10-26.md)。完整 Tauri / WebView2 与其他平台尚未验证，保持待验证。

## M4W-022 修复及验证记录

- 2026-10-26，`4b652fb` 加保留的未提交改动；Windows / Edge 156.0.4314.42。只读 HEAD ReaderPane 替换测试 bundle，复现视口下方一段完成后 scrollTop 10148→0，历史失败保留。
- 同文章 / 同 Reader HTML 的译文直接更新原 iframe 内对应段落，并按可见内容块偏移补偿上方新增高度；保留原文 DOM / 选区 / 焦点，文章 ID 改变时重建 iframe。表格与定义列表合法插入，输出仍为文字节点，sandbox / IPC / 后端协议不变。
- 新增真实 Edge 14 组与字体滚轮原 8 组、相关 DOM 61 组、Rust fixture 准备 3+7 项、lint / build / diff 通过；三个 DOM 脚手架同步读取实时文档，初次旧快照断言失败及修正见[证据](../tests/translation_scroll/README.md)。完整 Tauri / WebView2、真实 Provider 和其他平台未验证，保持待验证；未修改 Rust 或运行全仓测试。

## M4W-021 修复及验证记录

- 2026-10-26，`4b652fb` 加保留的未提交修改；Windows / Edge 156.0.4314.42。生产 App / CSS 的真实浏览器回归在中文 1200×800 的任务配置页复现导航 clientHeight=37、scrollHeight=44，按钮超出导航底部。
- AgentSettings 语言栏和导航栏禁止 flex 收缩，导航明确只允许横向滚动，下方内容继续独立纵向滚动。中英文六页 × 三种尺寸共 36 个布局场景及长内容独立滚动、既有键盘末项访问均通过；完整 runner 8 组、lint / build 通过。证据与命令见上述链接。
- 完整 Tauri / WebView2 与其他平台未验收，状态为待验证；未修改 Rust、共享 Dialog、依赖或用户数据，未重跑 Rust / 全仓套件。

## M4W-020 修复及验证记录

- 2026-10-13，HEAD `c90567b` 加全部保留的未提交改动；Windows / PowerShell 7.6.5、Node 24.12.0、Edge 155.0.4283.45。生产 ReaderPane + Rust HTML head fixture + 实际构建 CSS，真实滚轮复现：初始 scrollTop=400，更换 Serif / 关闭后=0。
- 字体请求的加载分支卸载旧 iframe，并在 Dialog inert 背景下新建，关闭后 Chromium 仍阻断其滚轮。同文章外观查询保留上一份 HTML，刷新期间继续显示并更新同一 iframe；文章切换不带入其他文章正文。未改共享 Dialog / IPC / sandbox / 依赖。
- 修复后真实 Edge 8 组滚轮、既有主题 / 可访问性 / 翻译 / 归属 / Reader 错误链接 61 组 DOM、lint / build / diff 通过；命令、准备失败与边界见 [证据](../tests/reader_scroll/README.md)。
- 完整 Tauri / WebView2、触控板与其他平台未验证，保持待验证。无 Rust 变更，未重跑 Rust / 全仓验证；未访问用户数据、未提交 / 推送。

## M4W-001 修复及验证记录

- 2026-10-12，基线 HEAD `c90567b` 加本项未提交改动，Windows / PowerShell 7.6.5。
- Reader 子面板按文章 ID 建立生命周期；摘要 / 翻译 / 标签使用请求身份并在切换、卸载或替换后拒收旧显示回调。此处不等同于取消实际后台任务，M4W-003 保持待处理。
- 笔记草稿按文章保留于本次应用会话，切换和关闭重开不丢弃；保存提交文章 / 文本 / 修订号快照，防重复保存，延迟完成不能清除新修订。失败保留草稿并显示中英文错误提示，取消丢弃当前草稿。应用重启不恢复未保存草稿。
- `npm.cmd run lint`、`npm.cmd run build` 通过；生产 React 组件的内存 IPC + Happy DOM 20.14.6 回归 15 项通过，命令与逐项结果见 [回归说明](../tests/reader_ownership/README.md)。未接触真实用户数据库或 AI 凭据。
- 未验证：完整 Tauri / WebView2 桌面路径、真实 AI、其他平台。当前仅隔离 DOM 交互通过，尚不满足桌面关闭条件，保留“待验证”。

### M4W-001 native 验收及关闭

- 2026-10-24 / 25，Windows / PowerShell 7.6.7、完整 Tauri debug / WebView2 155.0.4283.62；代码基线 `4b652fb`，开始工作区干净，收尾仅验收文档未提交，无产品源码改动。[实际步骤与证据](audits/native-acceptance-2026-10-24.md)保留早期 DOM 结果及本次 native 范围。
- 独立 identifier / 数据库 / WebView2，真实库仅只读取必要 AI 配置并原样复制不透明凭据；合成 A/B 文章。实际通过 A 草稿→B 编辑保存→A、笔记关闭重开、SQLite 故障导致保存失败及保留错误 / 草稿、去故障重试；隔离库笔记归属正确。
- 真实 AI 直连摘要成功；迟到场景使用只绑定 loopback 的真实服务透传响应延迟，TLS 校验保持。实际通过 A 摘要 / 翻译 / 标签执行中切 B、旧响应完成后 B 不被内容 / 状态 / 输入污染，以及摘要、静默双语与标签关闭重开。翻译 7/7 完整仅落 A 槽，B 无 translation 槽；11 次请求共 4314 token，均成功。
- 最低关闭条件满足，标为已关闭；关闭范围仅为文章归属。取消、队列、完整跨重启、其他平台及其余问题仍分别待验收。故障触发器已删除、隔离路由已恢复直连、延迟代理已停止；隔离应用保留供后续串行项使用。此次 build / 隔离 native 编译、最终 diff / UTF-8 / 证据 JSON 通过；未重跑 Rust / DOM 全套，未提交 / 推送。

## M4W-016 修复及验证记录

- 2026-10-25 用户明确恢复 Computer Use 后，仅复验此前断点：普通 Markdown 修复后实际标题 / 引用 / 列表编号 / 表格对齐 / 代码局部横滚 / 安全图片排版、关闭重开及编辑原文通过；正常退出确认进程为 0，再完整重启，两篇已保存笔记展示正确，恶意原文仍完整，隔离库逐字核对通过。无新增源码 / AI 请求，未重复已通过的构建及 6 / 15 项回归。两段 UI 调用完成后均停止调用并 reset JS 会话；公开 API 无宿主释放确认，未假称释放。最低条件满足，标为已关闭，保留首次中断和初始排版失败；其他平台仍未验证。[最终证据](audits/native-016-2026-10-25.md)。

- 2026-10-25，`4b652fb` 加保留的 001 验收文档与本项局部排版修复；Windows / 完整隔离 Tauri / WebView2。恶意副本真实保存、关闭重开及编辑原文通过；两篇笔记只读隔离库核对与输入完全一致，当日原始 jsdom 安全 6 项及既有归属 15 项通过。普通样例实际复现无标题层级 / 列表标记 / 引用区分及代码横向溢出，复用已有 reader-content 并增加笔记局部约束，lint / build / 相同 override native 重建通过。重启复验时用户物理 Escape 停止 Computer Use，未重试或绕过；修复后 native 排版及普通样例关闭重开仍未验收，保持待验证，下一步继续 016，不启动 005。[报告与脱敏证据](audits/native-016-2026-10-25.md)。

- 2026-10-12，HEAD `c90567b` 加保留的 M4W-001 与本项未提交改动；Windows / PowerShell 7.6.5。
- 统一 `render_markdown` 接入 DOMPurify 3.4.16，元素及属性使用明确白名单；保留常用 Markdown、只读任务框、表格对齐与代码语言类。链接仅 HTTP(S) / mailto / 页内锚点，图片仅 HTTP(S)，拒绝事件、脚本、样式、嵌入内容、SVG / MathML、本地 / 自定义协议与相对 URL。仅改变展示，不改存储或编辑原文。
- Node v24.19.0 + 临时 jsdom 30.1.2 的 6 项回归通过：恶意 HTML fixture、13 种危险 URL、4 个独立畸形 / 命名空间案例及重新解析、正常 Markdown、实际 ReaderNote 插入与编辑原文。命令和允许内容见 [回归说明](../tests/markdown_safety/README.md)。
- `npm.cmd run lint`、`npm.cmd run build` 通过；M4W-001 的 15 项隔离 DOM 回归复测通过。未改 Rust，未接触用户数据库 / 凭据。
- 未验证：实际浏览器与 Tauri / WebView2 桌面笔记保存 / 重开及视觉排版、其他平台。当前生产组件隔离验证通过，保持“待验证”，等待隔离桌面验收。

## M4W-005 修复及验证记录

- 2026-10-12，HEAD `c90567b` 加保留的 M4W-001、M4W-016 与本项未提交修改；Windows / PowerShell 7.6.5，Rust 1.97.0 GNU。
- Feed 解析 / Entry 模型 / SQL / TypeScript 分开 summary 与显式 feed_content、feed_content_type；正文 Feed 直接读取，导读抓文章。schema v2 保留旧 summary 和用户 / AI 数据，仅清理能确认与旧合并文本相同的派生正文和缓存，重复迁移通过。
- HTTP / 空响应 / 挑战页 / 提取失败及离线时，导读保留可见文字并输出结构化来源标记；ReaderPane 显示双语提示和重试。降级不存成功正文 / 缓存；无摘要时返回错误。更新正文或 URL 失效派生缓存，重复同步来源不变则保留缓存。
- Rust Feed / Reader 9 项、迁移 4 项通过；生产 ReaderPane 的 2 项来源提示 / 重试 / 切换回归及 M4W-001 原 15 项隔离 DOM 回归通过。cargo check（15 条已有警告）、前端 lint / build、本项 Rust 格式及 diff 检查通过；全仓格式仍因历史差异失败，全套 Rust 未重跑。命令、初始失败及 F-001 范围见 [回归记录](../tests/feed_content/README.md)。
- F-001 生产解析、内存存储首次 / 部分已有 / 重复去重、用户状态保持及临时导出往返通过。没有运行完整 Tauri 导入 Channel / 对话框、远程 11 源同步或桌面重启；真实 AI 和其他平台未验证。旧合并来源待同步重分类，离线仅作为可能不完整的旧内容展示；保持“待验证”。

## M4W-005 native 验收及关闭记录

- 2026-10-25，`4b652fb` 加保留的 001 / 016 未提交记录与笔记样式；Windows / PowerShell 7.6.7 / 完整隔离 Tauri / WebView2。实际通过显式短 Feed 正文保留且不抓文章、teaser 抓取全文、503 / 连接中断 / 停服回退显示来源与重试、不写成功缓存，以及恢复后的按钮重试全文持久化。
- 固定 F-001 原生文件对话框首次 11/11 添加并逐源在线同步成功，重复导入 0 添加 / 11 跳过，已读 / 收藏 / 合成笔记 / 标签保持。选定 11 源原生导出，另一个无 AI 配置空库原生导入 11/11，URL 集合一致；第三独立库预置 3 源后导入 F-001 为 8 添加 / 3 跳过、用户状态保持。正常退出 / 重启后订阅恢复。
- native 复现空响应同步被误标绿色成功，最小修正普通 / 流式导入的嵌套 SyncResult.error 映射。新空响应与 404 显示具体失败、后续成功源继续、最终 3/3 完成。cargo check 无 warning、错误判定 1 项和既有005九项、定向格式、三个隔离 override 重建及 diff 通过；直接测试exe首次0xc0000139与额度中断历史保留。未改本项前端或重复全套测试，未调用 AI。
- 最低关闭条件满足，标为已关闭。[完整报告 / 脱敏事实 / UIA](audits/native-005-2026-10-25.md)。最终原 override 重建恢复 `com.mercury4win.acceptance20261024`，原 A/B 笔记及用量保持；18745 无监听，最后 UI 段已 reset，无挂起调用。其他平台、008 链接 / 图片 / 分层缓存仍留后续验收；未提交 / 推送 / 发布。

## M4W-008 修复及验证记录

- 2026-10-12，HEAD `c90567b` 加保留的 M4W-001、M4W-016、M4W-005 与本项未提交改动；Windows / PowerShell 7.6.5、Rust 1.97.0 GNU。
- 最终响应 URL / 有效 HTML base 进入提取与 DOM URL 规范化；Feed 短正文的相对链接 / 图片同样处理。保留 raw HTML 禁用及现有 iframe 沙箱，危险 / 本地导航禁止；普通错误可见并有重试，005 摘要临时展示继续可重试。
- 三层版本 2 / 2 / 4 按上游有效性最小重建，Markdown 重建持久化；正文更新与全主题缓存失效为事务，主题 / 字体 / 字号缓存命中不重写。稳定无来源保存 empty，网络和提取故障不保存终态；版本 1 文章基准及新已解析 base 分别处理。
- 8 项本项 Rust 回归、原 005 的 9 项、生产 ReaderPane 2 项错误重试 / 链接保护、005 的 2 项与001的15项 DOM 回归通过；cargo check（15条已有警告）、lint / build、定向格式 / diff 通过。扩大 Reader 模块为21通过、1个已有标题精确断言失败，留待018；全套Rust未运行。
- 测试启动先报0xc0000139，核对旧exe旁已有Common Controls6 manifest后，新exe补相同sidecar、刷新时间与局部PATH后正常执行；复现脚本只写忽略的构建产物，详见[环境与命令](../tests/reader_pipeline/README.md)。未接触用户数据库 / 凭据。
- 未验证：完整Tauri / WebView2图片加载、链接打开、排版、桌面重启与其他平台。保持“待验证”，没有用隔离DOM或构建成功替代桌面验收。

### M4W-008 native 部分验收

- 2026-10-25，`4b652fb`加保留的001 / 016 / 005改动，Windows / PowerShell7.6.7 / 完整隔离Tauri / WebView2。无summary文章实际503提示与真实按钮重试恢复通过，失败不写正文 / 成功缓存；302最终URL与HTML相对base正确，WebView2实际加载相对图片。普通正文链接点击后系统Edge请求正确目标且200，但最终页面截图 / UIA未取得。危险 / 本地协议在native中变普通文字，未补点击检查。[报告、JSON与UIA](audits/native-008-2026-10-25.md)。
- 读取浏览器目标页时Computer Use因无法可靠判定当前浏览器URL执行策略检查而终止当前turn；未报告物理Escape，不等同全部Windows工具不可用。立即停用CU并reset，没有绕过。仅后续shell整理证据；本项18748 / PID10992服务已核对停止，隔离样例与scoped写审计保留。
- 旧source / Markdown / render三篇仍为预置版本，尚未native点击；分层重建、缓存命中、主题 / 字体缓存与重启复用均未满足最低条件。页内锚点被提取层展开为绝对URL的语义也待核对。维持待验证，不进入006；无本项产品源码变更，不重复历史Rust / DOM / 构建，不提交 / 推送。

## M4W-006 修复及验证记录

- 2026-10-12，HEAD `c90567b` 加保留的 M4W-001、M4W-016、M4W-005、M4W-008 与本项未提交修改；Windows / PowerShell 7.6.5、Rust 1.97.0 GNU、Node v24.12.0。
- 结构化 published_at / id 游标严格匹配 DESC 排序，NULL 单独处理；总数仅使用筛选条件，limit+1 判尾、尾页无游标。同步 Rust 模型、Tauri command、TypeScript 类型和 wrapper。EntryList 累积页，增加双语加载更多、失败提示和重试，加载期间保留已有行，订阅 / 标签范围独立缓存。
- 4 项内存 SQLite 回归及 6 组生产列表 + 实际 wrapper 的隔离 DOM 交互通过，覆盖 237 条乱序 / 同日期 / NULL 的完整顺序、稳定计数、尾页、组合筛选、边界删除 / 晚插旧文、加载防重、失败重试、末页键盘选择、已读 / 删除刷新和旧页延迟完成隔离。cargo check（15条既有警告）、lint / build、定向格式 / diff 通过；命令与范围见 [分页回归](../tests/entry_pagination/README.md)。
- 未验证：完整 Tauri / WebView2 桌面分页按钮默认键盘操作、滚动 / 排版及真实同步刷新、其他平台；未重跑全套 Rust / 全仓格式。原 011 过滤与快捷键范围未扩展，保留“待验证”。

## M4W-011 修复及验证记录

- 2026-10-12，HEAD `c90567b` 加保留的 M4W-001、016、005、008、006 与本项未提交改动；Windows / PowerShell 7.6.5、Rust 1.97.0 GNU、Node v24.12.0。
- 共享 EntryFilter / SQL 和前端参数映射覆盖 Feed、search、unread、starred、标签 Any / All；分页、计数、跨页全部已读条件一致。搜索空白归一、标签去重；UI 完整条件作为缓存键，范围切换回到第一页。最多5标签依据上游 tags-v2 §4.2，提供可见上限及禁用状态。
- 已读按钮与 m 可反转；s、j / k（含自动后续页）、Shift+A / Shift+R、r / t / n 接入实际列表 / Reader 动作。输入 / contenteditable、IME、模态设置弹窗及无关修饰键保护；旧 j 延迟结果不能覆盖新选择。删除事务重算关联可见标签计数，前端刷新 entries / feeds / tags、清理被删文章 Reader 选择和缓存。未扩展 M4W-010 或014。
- 本项4项内存 SQLite、6组生产 AppShell + 生产 IPC wrapper DOM、原006的4项SQL / 6组DOM及001的15项归属DOM通过；lint / build、cargo check（13条既有警告）、定向格式 / diff通过。首次测试准备错误及既有FeedList嵌套button警告见[回归记录](../tests/entry_filters/README.md)，没有以构建替代桌面验收。
- 未验证：完整 native Tauri / WebView2筛选、默认键盘 / iframe快捷键、布局、真实刷新及其他平台；未运行全套Rust / 全仓格式。保持“待验证”。

## M4W-019 静态证据与范围

- 2026-10-12，M4W-005 核对 IPC 数据类型时发现；HEAD `c90567b` 加本轮未提交改动。
- [Rust EntryDetail](../src-tauri/src/db/models.rs) 将文章放在 `entry: Entry`，`commands/entry_cmd.rs::load_entry_detail` 直接返回这个嵌套模型；[TypeScript EntryDetail](../src/types/entry.ts) 却 `extends Entry`，声明文章字段在顶层；[IPC 封装](../src/lib/tauri_bindings.ts) 没有转换返回结构。
- 当前唯一生产调用者 ReaderTagging 只使用两端均在顶层的 `tags`，静态核对未发现此调用受影响。不能将契约差异宣称为已复现的标签运行故障；风险是未来详情读取文章字段或测试 mock 按错误类型构造时得到 undefined。
- 本项仅登记，不随 M4W-005 改写详情协议。建议与 M4W-011 查询 / 状态路径一起修复，校验具体字段及不存在文章的错误语义。

## M4W-019 修复及关闭记录

- 2026-10-12，HEAD `c90567b` 加上述保留改动及 M4W-011 未提交实现；平台同011。TS EntryDetail改为 `entry: Entry`，同步完整嵌套测试mock；Rust仍直接返回真实模型，不添加虚构扁平字段。用 OptionalExtension 区分不存在 / 软删除文章和查询错误，命令维持 `Entry not found`。
- 内存SQLite生成的真实Rust详情JSON经生产load_entry_detail wrapper和ReaderTagging验证：entry ID / Feed正文 / 类型、null author、feed_title、tags、has_note正确，顶层没有文章ID；缺失文章的空结果和错误传播回归通过。Rust序列化与TS声明及唯一生产调用者已满足该契约问题验收，[证据与命令](../tests/entry_filters/README.md)。本项标为“已关闭”，没有将历史静态差异改写为已复现标签故障。
- 关闭范围仅为详情结构 / 消费者 / 缺失路径契约；没有运行native Tauri / WebView2或三平台桌面，相关用户交互保留在011待验证项中。

## M4W-017 修复及验证记录

- 2026-10-12，HEAD `c90567b` 加保留的001、016、005、008、006、011 /019与本项未提交修改；Windows / PowerShell7.6.5、Rust1.97.0 GNU。先本地生产HTTP探针复现中文拆字节变 `���文`、多行data / 可选空格失效、无DONE EOF误成功、引号 / 换行错误JSON非法；原历史审查静态证据保留。
- SSE字节缓冲至完整空行事件再严格UTF-8解码，LF / CRLF / CR、BOM、注释 / metadata、多行data及可选空格处理；单事件累计8MiB上限。完整`[DONE]`才成功，非法编码 / JSON / API错误、无终止EOF、截断或网络断流明确失败。摘要增量采用既有chunk结构，错误由serde_json序列化，保持summary message和translation segment / error契约。
- 12项Rust定向实际通过，包括所有字节切分位置 / 单字节块、中文 / emoji / JSON形正文、用量、完整终止、非法 / 截断与传输中断、缓冲上限、Channel失败，以及实际run_summary / run_translation+本地HTTP+内存SQLite+Channel成功 / 错误契约。cargo check（13条既有警告）、5文件定向格式和diff检查通过；[复现、命令与详细结果](../tests/agent_stream/README.md)。
- 未改前端 / 依赖 / 数据库 / 权限，未运行lint / build、全套Rust或全仓格式；未验证native Tauri / WebView2、真实AI / provider兼容性与其他平台，保持“待验证”。不发送`[DONE]`的provider现在明确失败。翻译并发错误聚合仍留M4W-004，调度 / 取消与持久化未扩展。

## M4W-003 修复及验证记录

- 2026-10-12，HEAD `c90567b` 加保留的001、016、005、008、006、011 /019、017与本项未提交改动；Windows / PowerShell7.6.5、Rust1.97.0 GNU、Node v24.12.0。先读上游README / summary-agent / translate-agent与Runtime实现和测试，再报告方案并实施。
- 实际执行许可包围模型路由 / 凭据和完整agent future，每类1active + 1latest waiting；等待替换、取消及离开撤销均按request ID。显式取消丢弃HTTP与全部分段future后才释放 / 提升，成功、错误、真实路由失败、外部drop / panic由RAII清理；旧finish / cancel不影响新active。登记前撤销短期记录覆盖IPC乱序。
- 新增abandon_agent_waiting注册及TS wrapper，三类Channel task_state协议与AgentStatus active / waiting ID契约一致。生产面板有等待 / 取消入口，切换仅撤销waiting，运行继续；返回原文章可查看并按原ID取消后台任务。翻译单段error不再提前解除运行状态，失败聚合 / 分段业务仍留004，结果保存 / 恢复仍留002。
- 8项本地HTTP + 生产agent + 内存SQLite回归通过，实际每类A+C仅2次请求，翻译取消断开1 / 3并发socket且余段未启动；8组生产组件 / 实际TS wrapper DOM通过，001原15项与017原12项复测通过。cargo check（10条既有警告）、lint / build、定向格式 / diff通过；[命令、证据与范围](../tests/agent_runtime/README.md)。扩大Agent模块30通过 / 1个既有标签Prompt版本断言失败，保留018；初次测试准备及修正如实登记。
- 未验证native Tauri / WebView2调度、Channel与取消交互 / 布局，真实provider / AI和其他平台；没有接触用户库 / 凭据、没有运行全套Rust / 全仓格式。保持“待验证”，本项未提交 / 推送，未开始下一编号。

## M4W-002 修复及验证记录

- 2026-10-12，HEAD `c90567b` 加保留的001、016、005、008、006、011 /019、017、003和本项未提交改动；Windows / PowerShell7.6.5、Rust1.97.0 GNU、Node v24.12.0。先读上游README / 摘要与翻译功能约定 / 翻译存储和测试，再报告方案并实施。
- schema v3新增通用结果slot，保留旧表 / 用户数据；摘要按文章 / 语言 / detail / 来源指纹保存完整成功，翻译按语言 / 策略 / 实际concurrency / 源指纹 / segmenter版本保存成功和独立checkpoint。来源同时检查Feed输入及正文 / 版本，未知来源旧表不自动恢复；模板 / route指纹扩展留009。
- 摘要provider累计完整delta；翻译成功段checkpoint、完整后promote，done均在COMMIT之后；失败 / 空响应 / 取消不覆盖原成功。无完整成功时续跑缺段，串行恢复前段上下文。并发最低错误聚合用于禁止不完整成功落库；稳定HTML映射 / 双语 / 逐段重试仍留004。
- Request task-local CommitFence在取消 / RAII时失效；已派发interact提交前持fence检查，不获取runtime锁。先取消的事务回滚、先COMMIT的成功保留。读取IPC注册 / TS类型 / wrapper与面板接通；Reader重建结束再次刷新，参数选择不隐式生成，重开 / 后台结束恢复并隔离旧请求。
- 7项Rust持久化 / 本地HTTP / 临时DB重开pool / 实际取消checkpoint / 已派发SQLite事务取消边界、4项迁移和7组生产面板+真实wrapper DOM通过；003原8项Rust /8组DOM、017原12项Rust、001原15项DOM复测通过。cargo check（9条既有警告）、lint / build、定向格式 / diff通过；[完整命令、初次失败与范围](../tests/agent_persistence/README.md)。
- 未验证native Tauri / WebView2的IPC、面板关闭重开与完整桌面重启、真实provider / AI及其他平台；没有访问用户数据库 / 凭据，全套Rust与全仓格式未运行。保持“待验证”，未提交 / 推送，未开始下一编号。


## M4W-004 修复及验证记录

- 2026-10-13 后续用户要求：翻译按钮 / `T` 直接生成或恢复，隐藏操作 / 进度 / 状态；每次进入最多一次启动，等待实际配置、正文 / 来源 / 后台任务读取，完整缓存不生成，失败只在显式重新选择时继续。修正原分段遗漏的 h1–h6、引用正文和表格文字等，升 `reader-html-block-v3` 隔离旧漏译缓存，表格译文放在单元格内。14 项相关 Rust（含生产 HTTP 每来源请求与旧缓存保留）和 64 组相关 DOM、check / lint / build / 定向格式通过，见 [最新证据](../tests/translation/README.md#2026-10-13-静默直接翻译与漏译范围)。此次未调用真实 API 或验证 native 桌面，保持待验证；下方控制条记录属于前一交互版本。

- 2026-10-13 用户交互调整：翻译按钮 / `T` 切换为正文上方控制条，不显示右侧面板 / 分隔条或重复来源配对列表；保留缺段编号重试、进度 / 错误 / 取消与原文切换。翻译 6 组、文章归属 15 组、主题字体 24 组隔离 DOM 和 lint / build 通过，见 [补充证据](../tests/translation/README.md#2026-10-13-翻译不显示侧边面板)；native 桌面仍未验证，问题状态保持待验证。

- 2026-10-12，HEAD `c90567b` 加保留的001、016、005、008、006、011 /019、017、003、002及本项未提交改动；Windows / PowerShell7.6.5、Rust1.97.0 GNU、Node v24.12.0。先核对环境 / 状态、只读上游README / translate-agent、Contracts / SegmentExtractor / PerSegment Execution与对应测试，先报告方案后实施。
- 按生产Reader Markdown→HTML提取稳定p / ul / ol，外层列表拥有内部p和嵌套列表，空图片段、pre代码块、引用、表格保持原文；内联code保留带反引号的变量语义。source ID基于类型 / 顺序 / 规范渲染HTML / 文本；slot分段版本改为reader-html-block-v2，不复用旧Markdown段索引。结果增加可默认反序列化的segment_ids / segment_errors，schema v3旧摘要仍可读。
- 实际并发规范为1..5（0默认3），失败 / 空响应 / 明确length等非stop终止按段失败；串行失败也继续剩余请求，所有请求收尾后才发complete / partial / all_failed；只有完整且COMMIT后发done。失败原因和成功段独立checkpoint，ID单段重试只发该段，继续只补缺段，来源变化拒绝旧ID；串行恢复前段上下文，HY-MT内置模板加入同样上下文入口。保留017的error.segment，新增index / source_segment_id，不改009的自定义Prompt / model route。
- 新load_translation_document IPC、注册 / TS契约和run_translation可选sourceSegmentId同步。iframe译文以textContent插在对应原文块之后，保留原代码 / 引用 / 图片 / 列表；复制当前Reader head保留主题 / 字体，原sandbox不扩权，不写原文HTML缓存。React有来源配对预览、段重试 / 读失败重试及三种终态提示；读失败阻止生成，取消恢复checkpoint，旧来源 / 回调按文章拒收；文章切换回原文模式。
- 本项6项Rust（生产agent + loopback HTTP / 内存与临时文件SQLite / 实际Channel）、6组生产组件 / 实际wrapper DOM通过；最大5个实际同时请求且未放行时无终态，完整来源fixture由Rust输出。002原7项Rust /7组DOM、003原8项Rust /8组DOM、017原12项Rust、001原15项DOM复测通过。cargo check（9条既有警告）、lint / build、定向格式 / diff通过，见[命令、初次失败与范围](../tests/translation/README.md)。
- 2026-10-13收尾：补齐tests/translation/README.md、核对链接与记录；最终cargo check（9条既有警告）、lint / build、定向格式 / diff通过。本日未扩大或重跑Rust / DOM回归，10月12日的6项 /6组及必要复测结果保留。
- 未验证native Tauri / WebView2实际IPC、双语布局 / iframe滚动、完整桌面重启、真实provider / AI及Linux / macOS；当前按已完成段更新完整srcDoc，未实现token级DOM增量patch。没有访问用户数据库 / 凭据或真实AI，全套Rust / 全仓格式未运行；保持“待验证”，未提交 / 推送，未开始009。



## M4W-009 修复及验证记录

- 2026-10-13，HEAD `c90567b` 加保留的001、016、005、008、006、011 /019、017、003、002、004及本项未提交修改；Windows /PowerShell7.6.5、Rust1.97.0 GNU、Node v24.12.0。先核对环境 /记录，只读上游README、agent-prompts /summary-agent /translate-agent及PromptResolver /Customization /AutoSummary，再报告方案后实施。
- 自定义模板优先载入三类执行器；上传验证YAML /变量 /条件政策并原子替换，无效外部文件以内置模板降级并提示。主 /备用能力及启用校验，无兼容模型报错；实际请求失败按段或摘要尝试backup，部分摘要reset防混杂，取消 /存储错误不触发备用，失败不覆盖旧成功。
- Configuration同时冻结模板 /路由 /翻译策略 /并发，共享请求参数进入指纹，凭据不入身份。ResultSlot统一加指纹、生产读取 /load_document与生成一致，前端查询key同步。旧slot保留但不命中新身份；只读load_latest_summary返回当前source的历史成功及language /detail /指纹，供007导出且不依赖模型配置 /凭据。新增可默认反序列化request_route /segment_routes保留实际主备用模型 /Provider溯源，012用量汇总改进另行处理。
- 摘要初始详细度使用保存默认，用户临时选择不被延迟profile覆盖。自动确认开启、正文ready及结果读取完成后1秒防抖，沿用003串行 /waiting撤销，active继续；失败不自动重试。有任意历史成功不自动生成，按其language /detail展示，并明确标注旧配置结果，可手动按当前配置重新生成。
- 本项7项loopback HTTP /生产agent /内存SQLite Rust、8组生产Settings /ReaderSummary /实际wrapper DOM通过；原004 /002 /003 /017的33项Rust及004 /002 /003 /001的36组DOM复测通过。check（6条既有警告）、lint /build、定向格式 /diff通过。取消实测主HTTP等待时1请求、备用已启动等待时2请求，两种socket均关闭且无追加。初次编译准备 /替换语法失败及修正见[命令与范围](../tests/agent_configuration/README.md)。
- 未验证native Tauri /WebView2的上传对话框 /实际Channel /自动摘要 /布局 /完整重启、真实Provider /AI与其他平台；没有访问用户库 /凭据、未运行全套Rust /全仓格式。保持“待验证”，未提交 /推送，未开始下一项。

## M4W-007 修复及验证记录

- 2026-10-13，HEAD `c90567b` 加保留的001、016、005、008、006、011 /019、017、003、002、004、009及本项未提交修改；Windows /PowerShell7.6.5、Rust1.97.0 GNU、Node v24.12.0。只读上游README、note-and-sharing、DigestComposition /projection及单多篇测试后，先报告方案再实施。
- 三套内置模板使用合法Handlebars；single_text变为真正纯文本，单 /多篇Markdown保留。标题 /原URL必需，作者entry /提取byline /feed /空回退；可选当前版本已缓存正文、已保存笔记、009当前source历史完整摘要，带模型 /语言 /detail归属；无AI /凭据 /网络。过期正文、选中内容缺失逐篇提示，草稿提示不包含，缺标题 /URL明确报错。
- 单Reader入口与多列表冻结选择 /列表序 /范围清理 /Reader inert、窗口隔离、原生save接口md /txt筛选、取消 /失败 /重试 /路径提示接通。preview IPC与export_multi封装 /类型 /注册一致，预览纯文字展示并提交expectedContent；导出前重读核对，笔记变化拒绝覆盖，异步UTF-8无BOM写入。默认单篇已有笔记 /摘要自动选择，多篇两项false，正文默认false。
- 本项5项真实Rust store /HB /临时文件、7组生产组件 /实际wrapper DOM通过，001原15、006原6组、011原6组DOM及011原4项Rust复测通过。check（6既有警告）、lint /build、3文件定向格式 /diff通过；全仓cargo fmt实际仍失败（既有文件差异），未运行全套Rust。[命令、初次fixture /断言 /旧mock准备失败及范围](../tests/digest_export/README.md)。
- 未验证native Tauri /WebView2真实IPC /系统对话框 /路径权限 /覆盖 /inert键盘与布局、实际桌面重启、Linux /macOS。没有新增macOS分享服务、配置固定目录或模板自定义，不调用AI生成 /编辑笔记。保持“待验证”，未提交 /推送，未开始010。

## 新问题模板

```text
编号 / 优先级 / 状态：
标题与影响：
发现日期 / 代码基线 / 环境：
复现步骤或静态证据：
预期 / 实际：
验收条件：
关联决策 / 报告：
修复及验证记录（完成后填写）：
```

## M4W-010 修复及验证记录

- 2026-10-13；HEAD `c90567b` 加保留的此前串行修复及本项未提交改动，Windows /PowerShell7.6.5、Rust1.97.0 GNU、Node24.12.0。先核对环境与历史审查、只读上游功能 /实现 /测试，再按方案串行实施。
- canonical /alias resolver、事务合并 /去重 /人工来源 /confidence /别名 /计数、rename冲突与旧名保留、临时提升 /清理已实现；schema v4兼容旧规范化碰撞与别名。标签库维护接通Settings /Sidebar，刷新关联Query并同步选中标签。
- batch替换错误固定赋值IPC：统一scope预览与冻结、显式模型 / >100确认 /1..5并发、完整runtime许可与冻结主备用 /Prompt、持久staging、逐提议审核后确认Apply。取消丢弃实际请求，已完成可审核 /未开始可重试；孤立running恢复ready，活动ID不会误停。Apply重解析canonical /alias并校验实际输入snapshot /来源hash，跳过已删除或变更文章，整次原子提交 /幂等，不在审核前污染tag /entry_tag。无AI本地 /手工与相关文章路径可用。
- 本项12项Rust（tag_过滤另包含011已有1项）、10组生产DOM通过；001原15组、003原8组DOM /8项Rust、011原6组DOM /4项Rust、009原7项Rust和迁移原4项复测通过。最终check（6既有警告）、lint /build（既有chunk提示）、9文件定向格式 /diff通过。loopback观察socket关闭、取消后无第3请求；fallback实际10请求 /并发峰值2 /逐篇失败继续 /成功用量真实Backup。详见[完整命令、准备失败及证据](../tests/tag_system/README.md)。
- 当前采用单次原子Apply，而非上游分块提交；生成中断回到审核，不自动重新发AI。完整请求级失败用量继续留012。native Tauri /WebView2真实IPC /键盘 /焦点 /布局、完整退出重启、真实AI与其他平台未验证；全套Rust /全仓fmt未跑，历史018失败保留。尚不满足桌面关闭条件，保持“待验证”。


## M4W-012 修复及验证记录

- 2026-10-13；HEAD `c90567b` 加保留的此前串行修复和本项未提交改动；Windows /PowerShell7.6.5、Rust1.97.0 GNU、Node24.12.0。只读上游README /token-usage /Provider lifecycle /request facts及测试，报告方案后实施。
- schema v5扩展逐HTTP事实 /真实ID与名称 /去敏endpoint /阶段 /开始结束 /NULL用量，旧聚合标legacy独立显示且不伪造精确请求数。摘要主备用、翻译每段 /重试、标签及批量每篇的全部尝试准确记录成功 /失败 /取消 /超时；SSE include_usage，失败中已有Token保留，未知与实测0区别。连接测试 /读缓存 /未发请求排队取消不计。请求成功与业务解析 /结果COMMIT分开，不把Token或已结束HTTP终态绑定结果fence。
- RAII取消只派发有限DB写、正常await日志、event_key幂等；独立轮询避免单连接池在并发日志与checkpoint下死锁，无socket或runtime锁残留。HTTP结束后取消保持其原终态。写失败日志+会话计数+报表警示可见，未承诺强杀 /即时runtime退出的未完成日志。
- Provider事务归档级联模型 /Model归档，保留Key、profile引用和历史；所有Provider归档后历史成功摘要仍独立可读且不发请求；新请求排除归档 /禁用，既有active冻结快照继续。规范endpoint+原名称才恢复Provider /原Key，UI明示新账户换名称；模型按Provider+model_name恢复。显式创建 /恢复能力选择落库；旧调用新建默认 /恢复保留。Provider恢复不自动恢复模型。配置指纹 /模型和报表缓存同步失效，不可用配置可见。
- UTC滚动[start,end) /每日UTC桶，ID稳定Provider /Model /Agent分组及当前名称 /归档标记；范围 /指标比较图 /日图 /表格 /覆盖率 /legacy /写失败提示，三维上下文入口复用报表。总计 /分组 /日桶同事务；更名不拆成多个同ID组。
- 本项13项Rust（loopback /内存SQLite /迁移 /实际取消与有限写flush）、8组生产DOM通过；003原8、009原7、010批量原7、017原12、迁移原4项Rust及009原8组DOM通过。最终check（6既有警告）、lint /build（既有chunk提示）、12文件定向fmt /diff通过。[命令、准备失败 /实际死锁 /017兼容回归失败及最终证据](../tests/usage_lifecycle/README.md)。010实际10请求现在10条用量（2成功8失败），未改批量审核业务。
- native Tauri /WebView2实际IPC /界面 /键盘 /焦点、合成凭据文件失败与完整重启、真实Provider include_usage兼容和其他平台未验证；全套Rust /全仓fmt未跑，018历史失败保留。无用户库 /真实Key /真实AI访问，未提交 /推送，保持待验证，未启动013。

## M4W-013 修复及验证记录

- 2026-10-13；HEAD `c90567b` 加保留的前述串行修复及本项未提交改动；Windows /PowerShell7.6.5、Rust1.97.0 GNU。先核对环境 /记录、只读上游README /obsidian-pub /实现和原shell fixture /测试，确认方案后实施。
- DOM精确base与静态siteInfo /preload字面量，注释 /字符串伪声明 /动态计算不触发；Publish HTTPS shard /host /uid /access Markdown路径校验，受限Client在发出下一跳前拒绝外部或不同vault，15秒deadline /UTF-8 /HTTP /空 /HTML /JSON失败明确。支持上游证据自定义域名及最终shell响应URL /编码，不猜site或page ID。
- pipeline_type=obsidian，shell /响应URL /resolved Markdown URL独立保存；失败仅pending，无canonical Markdown /成功缓存，不回退shell，005临时导读继续。成功后Readability /cleaned_html为空，共享canonical Markdown /主题字体缓存；版本过期由intermediate重抓、缺身份从shell恢复。default Readability升3防旧shell命中，007共享按管道正文判定；摘要source_snapshot /004 translation_document实际可用且render重建hash稳定。
- comrak AST规范普通链接 /图片相对URL /危险协议，raw HTML共享安全renderer禁用；![[媒体]]按cache索引exact /decoded /唯一suffix-basename匹配逐组件编码，歧义 /索引失败保留，代码不改。没有generic SPA /完整Obsidian语法 /私有登录支持，没有执行远程脚本或扩权限。
- 本项7项Rust（上游真实fixture /loopback /内存SQLite）通过，实际drop关闭socket /15秒超时无成功写；005原9 /008原8 /004原6 /007原5项通过，cargo check（6既有warning）、6文件定向fmt /diff通过。[命令、初次测试转义 /AST非Send /fixture约束 /错误filter及范围](../tests/obsidian_publish/README.md)。无前端改动，未跑lint /build /DOM或全套Rust /全仓fmt，历史018失败保留。
- native Tauri /WebView2真实IPC /媒体加载 /链接 /排版、实际在线Publish兼容、完整退出重启、Linux /macOS未验证；drop测试不代表新增Reader UI取消入口。无用户库 /凭据 /远程站点访问、未提交 /推送，保持待验证。下一014尚未启动。

## M4W-014 修复及验证记录

- 2026-10-13；HEAD `c90567b` 加保留的此前串行修复与本项未提交改动，Windows /PowerShell7.6.5、Rust1.97.0 GNU、Node24.12.0。核对实际环境 /记录、只读上游README /主题字体约定 /实现后，先报告方案再实施。
- 阅读设置沿用字体窗口新增Classic /Paper及共享auto /light /dark入口，关联label /双语 /toolbar aria-label /字体选中与字号标签；mercury_theme /font旧键保持兼容，新增mercury_reader_style默认Classic。system_theme为Zustand响应式状态，useTheme在App单次订阅 /初始重采样 /change /卸载清理，强制亮暗不被系统覆盖，auto立即恢复当前系统。
- Reader query身份按文章 /style-有效亮暗 /字体 /字号；Paper仅采用上游现有亮暗配色，正文 /次要 /链接对比度均≥4.5:1，显式字体保留。render升5失效旧Paper CSS，Readability /Markdown仍3 /2、schema5不变；复用008持久Markdown只重渲染 /不重抓。双语回调附原Reader HTML身份，迟到theme /article /translation doc不覆盖当前head，沿用004复制当前head与原sandbox，切文章原文模式，不发AI。
- 本项3项Rust /8组×3初始状态共24组生产DOM通过；008原8项Rust /2组DOM、004原6项Rust /6组DOM、001原15组DOM通过。check（6既有警告）、lint /build（既有chunk提示）、4文件定向fmt /diff通过；SQLite拒绝正文UPDATE /缓存INSERT及offline URL证明只读源与缓存命中，真实Rust HTML直接供DOM使用。[命令、初次准备失败 /零项filter及范围](../tests/reader_theme/README.md)。
- native Tauri /WebView2实际系统事件 /srcDoc排版 /字体可用性 /键盘 /完整退出重启、Linux /macOS和真实AI未验证；全套Rust /全仓fmt未跑，018历史失败保留。全局Dialog焦点仍留015。无用户库 /凭据 /远程访问、无依赖或权限变更，未提交 /推送 /发布，保持待验证。下一015尚未启动。

## M4W-015 修复及验证记录

- 2026-10-13 英文设置导航补充：真实 Edge 在 1200px 窗口复现 Tag Library 位于第二行；减少横向留白、固定单行并为窄窗口保留水平滚动后，中文 / 英文 1200 / 800 / 600px、最后标签的键盘及点击访问通过，最新 7 组浏览器与 lint / build / diff 通过。详见[中文化验证](../tests/localization/README.md)，native 桌面验收仍待完成。

- 2026-10-13 后续入口 / 术语调整：按用户要求将语言选择移至设置顶部，主界面无独立控件，用量中文保留 token。6 组真实 Edge、20 组可访问性 / 标签 DOM、lint / build / 359 对资源 / diff 通过；详情见同一[中文化入口](../tests/localization/README.md)。未改 Rust，native 桌面验收仍待完成。

- 2026-10-13 中文化补充：默认简体中文、界面语言切换 / 保存、日期 / 数字和文摘标题同步；359 对资源、6 组真实 Edge、7 项文摘 Rust、37 组相关 DOM及构建 / 格式检查通过。[专项证据](../tests/localization/README.md)保留实际命令和准备失败；native 完整退出重启及系统窗口尚待验收，状态仍为待验证。

- 2026-10-13；HEAD `c90567b` 加保留的此前串行修复及本项未提交改动；Windows /PowerShell7.6.5、Node24.12.0。核对实际环境和记录、只读上游README /l10n与相关SwiftUI名称 /键盘语义，先报告方案后实施。
- 补主题当前模式、Feed同步 /删除名称 /URL label、OPML标题 /完成 /错误重试双语，原Reader失败 /标签空状态已本地化并继续验证；机器诊断保留。Feed选择和删除改同级原生button，键盘焦点可见；Settings表单 /Reader外链 /笔记标签输入补名称，选择状态明确，局部错误 /进度角色补齐。
- Dialog body portal /顶层栈统一视觉z-index、唯一aria-modal、背景 /父层inert并恢复原值；Tab /ShiftTab循环 /隐藏祖先与closed details排除 /summary和链接 /空面板与异步控件 /程序焦点护栏 /嵌套Escape和卸载恢复。010父设置 /batch审核关闭锁和007选择inert保持；触发器消失 /不可用安全回退。标签 /文章Enter和Escape /全局及iframe桥保护IME /重复 /输入 /link Enter，sandbox不变，不改Rust /协议 /权限。
- 本项10组真实生产DOM /实际wrapper /i18next、351/351键和插值一致通过；010原10、007原7、014原24、001原15、011/019原6、006原6组DOM通过，共78组。lint /build /diff通过，build已有chunk提示。隔离准备失败及完整命令见[回归说明](../tests/accessibility/README.md)。007/010/014脚手架仅扩body查询包含portal；007预览排除测试result pre，原业务断言保留。
- native Tauri /WebView2真实焦点 /原生键盘默认激活 /屏幕阅读器 /inert /iframe事件 /窗口切换 /布局 /完整退出重启、Linux /macOS未验证；HappyDOM不能代表这些验收。未运行Rust check /tests /全仓fmt，018历史失败保留。无用户库 /凭据 /AI /远程访问、依赖和锁文件未改，未提交 /推送 /发布，保持待验证。下一018尚未启动。

## M4W-018 修复及关闭记录

- 2026-10-13；HEAD `c90567b` 加保留的全部前项及本项未提交改动，Windows / PowerShell 7.6.5、Rust 1.97.0 GNU。先确认环境和记录，报告方案后实施；原审查及历史失败保留。
- 初始完整 cargo test 实测 129 通过 / 5 失败：tagging 与 HY-MT 版本旧断言、h1 锚点整串旧断言、两个迁移测试将最新 Schema 伪装旧版本导致重复加列。模板补实际渲染 / 条件语义，标题补可见文字 / 锚点 / 正文；cfg(test) 真历史 V2 / V3 fixture 保留旧 payload / 别名 / usage_count / 幂等，并补升级到当前版本 / Feed 正文 / 收藏 / 外键断言。没有修改生产迁移来容忍错误输入，也没有删除、ignore / skip 测试。
- 初始全仓 fmt 17 文件失败；六条未用导入最小清理后，用户授权范围内 cargo fmt 纯 Rust 格式化实际改22文件。format 前后71源码文件与独立 rustfmt 快照逐Hash一致，范围明细见[验证报告](audits/final-validation-2026-10-13.md)。
- 最终普通完整 cargo test 默认并行 **134 通过、0 失败、0 忽略**，binary / doc 0项入口也执行；当前GNU测试产物已有Common Controls6 sidecar环境。cargo check无warning、全仓fmt、lint /build、diff /UTF-8 /文档链接通过，build已有chunk提示保留。全部15套生产DOM **125项 /组**通过（真实Rust临时DTO /HTML；安全用Node24.19.0 +jsdom30.1.2，其余HappyDOM20.14.6），资源351/351键 /插值一致。
- 新增[可复现入口](../tests/final_validation/README.md)，四个fixture定向生成至临时目录不计入完整Rust总数；用量bundle支持直接JSON，无仓库临时fixture改写。首次汇总器误读数字型passed计数中断，修正后最终15套完整重跑；真实失败与初始结果均已记录。
- 本项最低关闭条件已满足，标为已关闭；关闭只限测试语义 /格式基线。M4W-019原契约关闭保持，其余17产品项继续待native桌面 /真实AI /三平台验收；没有以隔离DOM /构建替代实际用户路径。未提交 /推送 /发布，未触及真实用户数据或凭据。
