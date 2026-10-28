# 自然语言筛选 Windows 原生验收与修正

日期：2026-10-27（Asia/Shanghai）。基线：`81f7b2e` 加保留的全部自然语言筛选 / 真实 API 验证未提交修改。平台：Windows / PowerShell 7.6.7 / 完整 Tauri debug / WebView2。用户本轮明确批准 Computer Use 检测、验收及修正，替代此前暂停限制；使用 computer-use 技能公开 sky API 操作独立窗口。

## 隔离环境与证据

- 独立 identifier：`com.mercury4win.naturalsearch20261027`，窗口标题 `Mercury Natural Search Acceptance 2026-10-27`。程序和完整 UIA 文本位于忽略的 `tmp/natural-search-native-20261027/`；启动前没有 Mercury 窗口，没有终止用户进程。
- 从此前合成 V5 搜索 fixture 创建新库，重置为 4 篇合成文章、4 来源（含重名）、2 标签、1 笔记，应用实际启动迁移至 Schema 8。创建前确认新测试目录不存在，没有复制日常库 / 真实 Provider / 凭据。
- [受控 Provider](../../tests/natural_search/native_provider.py) 仅监听 `127.0.0.1:18761`，不转发外部网络；控制文件提供确定性错误 / 30 秒延迟。[seed_native.py](../../tests/natural_search/seed_native.py) 使用已有合成 fixture 与不存在的目标文件。
- 版本化证据：[SQLite 与请求核对](natural-search-native-2026-10-27-evidence.json)、[实际 UIA 片段](natural-search-native-2026-10-27-ui.txt)。片段来自原始日志，按观察顺序去掉完全重复行；Computer Use 返回的截图直接检查，没有外部截图或 UI 注入。

## M4W-028：成功提示及任务条件身份修正

首次原生操作由 Python 搜索转为 Rust＋收藏＋最近 7 天，查询成功得到 A / B 两篇，但缺少「已替换条件」说明。EntryList 合并导航默认值后，筛选对象字段顺序不同；NaturalFilter 用原始 JSON.stringify 比较响应与当前条件，误判为不同条件。等价对象重排也可能错误取消进行中的请求。

先让 DOM 脚手架模拟真实父组件合并，旧代码实际失败 `Error: success explanation missing`。新增 search_filter_identity，固定字段次序、补齐默认值、统一当前已读语义、标签排序去重，用于请求归属和成功说明比较。没有改变 SQL、保存格式或模型输出校验。回归继续确认真实条件修改取消旧请求，等价重排保留请求。

修正后重新构建隔离应用，Windows 实际显示中英文成功说明；保存、手动修改和完整退出重启通过，M4W-028 最低关闭条件满足。首次失败保留，没有计作通过。

## 实际原生路径

| 路径 | 实际结果 |
| --- | --- |
| 普通浏览 / 普通关键词 | 全库 4 篇，普通来源无完整搜索表单；Python 关键词得到 1 篇，Provider 请求数仍为 0。 |
| 明确提交 / 条件替换 | Enter 后才请求，Rust＋收藏＋最近 7 天得到 A / B 两篇，Python 旧条件被替换；修正后成功提示可见，高级表单展开。 |
| 手动编辑 / 移除 | 加未读后 2→1；组合解析映射来源 / 标签 / 未读 / 笔记 / 收藏 / 相对日期 / 排序，控件可滚动访问，实际取消 Rust 标签勾选。 |
| 保存 / 更新 / 重启 | 书签弹窗 Enter 创建；增加未读后更新同一视图。完整退出、确认进程退出后重启，仍保存 Rust＋未读＋收藏＋recent_days=7，固定日期为空；打开得到同一篇 A，没有 AI 请求。 |
| 歧义 / 不支持 | 同名来源列出两个具体合成 Feed URL；上周读过明确说明阅读历史未记录，均保留已有 1 篇结果。 |
| 非法结构 / HTTP 错误 | 不允许的 ID / SQL 字段返回无效条件；HTTP 503 显示请求失败；已有关键词及 1 篇结果保持。 |
| 取消后新请求 | 30 秒慢请求显示加载与取消按钮；显式取消后立即提交不同组合查询成功。超过旧响应延迟后仍为新结果，无旧空查询覆盖。 |
| 连续提交 | 慢非法响应期间直接点 Start a new request；新请求成功为空结果。旧响应延迟超过 30 秒后仍有成功提示 / 0 entries，没有旧错误。 |
| 离开搜索 / 卸载 | 请求中切到新闻来源，紧凑列表显示 Python / Rust 旧收藏两篇。超过旧请求 30 秒延迟后仍为 2 篇，没有表单或旧查询覆盖。 |
| 无 AI 配置 | 原生设置暂禁隔离 Provider，出现英文配置指引；普通 Rust 关键词仍得到 2 篇且不增加请求。之后在设置恢复 Provider。 |
| 空结果与失败区别 | no-such-article 解析成功显示成功条件 / 0 篇 / 无文章；解析失败只在表单报错，保留有效条件和结果。 |
| 布局 / 键盘 | 中文亮色 1200×800、中文及英文暗色约 800×600、列表约 320 / 200 px 实际检查。长说明 / 高级条件可在面板内滚动，保留结果区；输入有标签，Enter、Tab→Space 提交及设置 Escape 关闭可用，重启保留英文 / 暗色偏好。 |

共 **12 个 loopback 请求**：用量记录 8 succeeded / 1 failed / 3 cancelled，与服务端数量一致。这是 HTTP 事实，例如非法结构但 HTTP 200 仍计 HTTP succeeded，不代表业务解析成功。合成 token 数不代表真实消耗。出站实际核对仅句子、来源名 / URL、标签名、日期 / 时区，无 ID / 正文 / 笔记。最终保留 4 文章 / 3 收藏 / 1 笔记 / 1 视图，外键错误 0，没有真实凭据文件。

## 检查、收尾与边界

- 修正后生产组件 DOM 5 组、npm lint / build、626 对双语资源 / 插值 / 静态引用、UTF-8 / diff / 文档链接检查通过。旧代码失败后同一回归通过，包含等价条件重排不取消和真实编辑失效。build 保留已有大 chunk 提示。
- 初始及修正后独立 Tauri debug 构建通过；结束时使用仅跳过重复 beforeBuildCommand 的 default override 重新构建默认 identifier，恢复普通 src-tauri/target/debug/mercury.exe；隔离副本保留已验收构建。
- 窗口以 Alt+F4 正常退出，重新枚举确认无窗口；准确识别并停止本任务的 Python Provider，18761 无监听。Computer Use JS 会话已 reset，无挂起输入。未修改根 Tauri 配置，未提交、推送或发布。

本轮未改 Rust 产品代码，不重跑 Rust library；此前 Rust 模块通过及旧 socket 超时失败基线保持。没有额外真实 API 请求，受控结果不代表模型质量；上一轮真实 API 小样本及提示版本边界见[独立报告](natural-search-live-2026-10-27.md)。Linux / macOS、安装升级、完整辅助技术矩阵、大型元数据库、真实网络取消 / fallback、长时间实际滚动边界未验证。
