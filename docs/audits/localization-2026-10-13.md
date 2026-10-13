# 中文化专项验证（2026-10-13）

基线：HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加开始时已有的全部未提交修复，以及本次中文化。实际目录 `G:/sth_interesting/mercury4win`，Windows / PowerShell 7.6.5、Node 24.12.0、Rust 1.97.0 GNU、Edge 155.0.4283.45。未操作上游仓库、用户数据库、真实凭据或远程服务。

收尾检查发现另一流程已将此前修复提交为 `f732ee70f59a65589e0ef96ac29b289696e945af`（2026-10-13 15:39:01 +0800）。本任务修改仍未提交；18 个已有实现 / 资源文件的开始快照与新 HEAD 按换行归一比较一致，因此本轮测试覆盖新 HEAD 加中文化增量，没有因新提交更换实现。保留该提交及其文档更新。

用户最初要求重复操作交给 terra high 子 agent；当前子 agent 工具不提供该模型。询问后用户明确选择“不派发，由主 agent 完成”，本任务没有创建子 agent 或独立聊天。

## 修改范围

| 文件 / 模块 | 改动及目的 |
| --- | --- |
| `index.html`、`src/i18n/i18n.ts`、`src/stores/app_store.ts` | 默认简体中文、有效偏好恢复 / 保存、store → i18next → 根 HTML 语言同步、开发热更新释放订阅 |
| `src/components/layout/language_select.tsx`、`sidebar.tsx` | 无 AI 配置也可用的原生界面语言选择框，选项使用语言自称，支持键盘 |
| `src/i18n/en.json`、`zh-Hans.json` | 新增 8 个同步键；补齐设置中文并统一术语，保留产品名、模型标识、地址和标准格式名 |
| `src/features/entry/components/entry_list.tsx`、`usage/components/usage_stats.tsx` | 日期、数字格式和分组名称排序跟随界面语言 |
| Feed 的 `opml_import.tsx` / `opml_export.tsx`、`agent_settings.tsx` | 文件窗口标题及 OPML / YAML 筛选说明本地化，扩展名及数据协议保持 |
| `reader/components/digest_dialog.tsx`、`src/types/digest.ts` | 文摘预览 / 导出携带同一可选 locale；语言切换失效旧预览；固定失败提示双语化 |
| `src-tauri/src/digest/templates.rs`、三份 `resources/digest_templates/*.hbs` | 中文标题、来源、作者、摘要 / 笔记 / 正文标题及摘要出处；旧调用默认英文 |
| `src-tauri/src/digest/m4w_007_tests.rs`、`reader/m4w_013_tests.rs` | 新增两项语言 / UTF-8 / 兼容 / 过期预览测试；现有显式 options 初始化补 `locale: None` |
| `tests/localization/`、现有四套测试的语言 mock | 生产 App 真实浏览器验证；补齐 mocks 的 `i18n.language` 契约 |
| `tests/entry_filters/browser.tsx` | 原快捷键断言仍期待此前已经隐藏的翻译操作区；改为核对 T 切换翻译按钮选中 / 返回原文，且不出现旧操作区 |

没有数据库迁移、依赖、凭据、应用标识或 AI 配置变更。本任务没有提交 / 推送 / 标签 / 发布。已有工作区修改及另一流程的提交按原样保留；基线快照仅用于本轮增量核对。

## 实际结果

完整命令与桌面验收步骤见 [专项验证入口](../../tests/localization/README.md)。

| 检查 | 实际结果 |
| --- | --- |
| 真实 Edge 生产 App | 6 组通过；英文系统默认中文、原生键盘选择、中英偏好重载恢复、打开的设置即时切换、日期、窗口参数、文摘 locale / 预览更新、无效 / 不可用存储、800×600 亮暗布局；未捕获到页面异常 |
| 文摘 Rust（GNU sidecar runner） | 7 通过、0 失败、0 忽略；包含新增两项，三种格式实际导出中文 UTF-8 字节、无 BOM、旧调用英文兼容和跨语言旧预览拒绝写入 |
| 可访问性 / 文摘 / 分页 / 筛选 / 用量 DOM | 10 / 7 / 6 / 6 / 8 组，合计 37 通过；使用已有 Happy DOM 20.14.6 与真实 Rust 详情 / 用量 fixture |
| 资源键 / 插值 / 静态引用 / 无 BOM | 359 英文 / 359 中文一致，通过 |
| lint / build | 通过；保留已有 >500kB chunk 提示 |
| cargo check --locked | 通过，未输出 Rust warning |
| 本次三个 Rust 文件 rustfmt --check / git diff --check | 通过；没有全仓格式重排 |

浏览器运行结束关闭实例及 loopback server；结果和亮暗截图保存在临时目录。最后实际输出为 `mercury4win-localization-aYj1tC`。视觉复核发现第一版 harness 未采用生产 body 的颜色 class，随后改为读取实际 `index.html` 并替换入口，再重新通过全部 6 组；不将测试页颜色差异当成产品缺陷。

## 中间失败与修正

- 旧文摘测试 mock 只返回 `t`，新增使用语言对象后报 `Cannot read properties of undefined (reading 'language')`。文摘、分页、筛选和用量脚手架补回真实 hook 已有的 `i18n.language`，原业务断言保持，重跑通过。
- 新浏览器测试一处英文 URL 标签少了实际文案中的逗号，另一处把文章选择后的“预览”误写为“导出文摘”，导致 locator 超时。按现有生产文案和流程修正后通过，未为测试改变产品交互。
- 首次用量回归使用了不存在的临时 fixture 文件名，准备阶段报 ENOENT。检查实际 TEMP 内容后改用已有 `m4w-012-usage-fixture.json`，8 组通过。没有创建空文件冒充 fixture。
- 筛选回归运行到 T 时，既有断言还查找 `translation.translateArticle`。当前基线的静默翻译已无该操作区；按 D-006 验证选中状态及返回原文，并额外拒绝旧操作区，6 组通过。此次未修改翻译实现。
- 可访问性脚本预期的两次 `fixture file dialog failed` 日志属于失败路径测试，10 组实际通过。

## 未验证

完整 native Tauri / WebView2 的退出重启、真实文件窗口显示 / 焦点 / 屏幕阅读器、真实 AI、Linux / macOS，以及完整 Rust / 全部 DOM 套件均未运行。browser reload、mock 文件窗口参数与 Rust 文件写入不冒充 native 桌面验收；M4W-015 保持待验证。第三方诊断及用户文章原文保留来源文本，不改写已保存内容。

## 后续调整：语言入口及 token 名称

2026-10-13，当前 HEAD f732ee70f59a65589e0ef96ac29b289696e945af 加全部保留的中文化未提交修改。用户要求将语言入口移入设置，并保留 token 名称。

- LanguageSelect 移至 src/features/agent/components/language_select.tsx，由 AgentSettings 顶部显示；Sidebar 不再显示语言选择。保存 / 同步逻辑保持。
- zh-Hans 用量文案将“词元”及大写 Token 改为小写 token，键和插值不变。
- 现有 Edge runner 通过设置里的原生下拉框验证鼠标 / 键盘切换、打开表单即时更新、页面重载、中英偏好、无存储、窄窗口亮暗设置，并确认主界面没有语言控件。6 组通过，亮暗截图复核；目录 mercury4win-localization-3L9RoJ。
- lint / build、可访问性 10 组和标签 / 设置关闭锁 10 组 DOM、359 对资源 / 插值 / 静态引用及 diff 通过；OPML fixture 失败日志属于既有受控失败路径，build 保留已有 chunk 提示。命令沿用专项 README 并补标签 suite。
- 此次没有修改 Rust 或重跑 Rust / 全部 DOM，没有操作用户数据 / 凭据或提交 / 发布。native Tauri / WebView2 仍待验收。本节记录后续进展，上文的初次 Sidebar 入口及验证结果保留当时事实。

## 后续调整：英文 Tag Library 导航保持单行

2026-10-13，f732ee7 加保留的全部中文化未提交修改，Windows / PowerShell 7.6.5、Node 24.12.0、Edge 155.0.4283.45。用户反馈英文设置标签库入口换行。

- 实际复现：原生产样式运行到英文 / 1200px 布局断言，报 en 1200px: Tag Library wrapped onto another row。导航使用 flex-wrap，六个英文标签与 px-4 留白超过设置窗口可用宽度。
- 修改 src/features/agent/components/agent_settings.tsx：导航 flex-nowrap / overflow-x-auto，按钮横向留白改 px-3。常规窗口同排且无需横向滚动，较窄窗口横向滚动，保留 Button 原有不拆行、键盘和选中状态。
- 现有 tests/localization/run_browser.mjs 扩展布局检查，真实中英 1200 / 800 / 600px 六个入口 y 坐标一致，正常宽度 scrollWidth <= clientWidth，Tab 可到最后入口、点击选中有效。最新全 runner 7 组通过，英文 / 中文设置截图复核；输出 mercury4win-localization-OpgVC9。
- npm.cmd run lint / build、git diff --check 通过；build 保留已有 >500kB chunk 提示。未改 Rust、依赖或数据，未重跑完整 Rust / DOM；native Tauri / WebView2 及其他平台未验收，M4W-015 仍待验证。此前换行复现保留，不改写为一直通过。
