# AGENTS.md

本文件适用于仓库根目录及其所有子目录。更深层的 AGENTS.md 可补充对应目录的约定；用户当前明确指令优先。

## 协作与环境

- 默认用中文沟通，代码名称沿用所在模块的风格。
- **遇到路径或乱码问题先检查实际环境再操作，默认使用 UTF-8 编码。**
- 开始工作先确认实际工作目录、Shell、Git 根目录和 `git status --short`，保留用户已有改动。
- 仓库目录名可能与远程仓库名不同。发现配置路径不存在时，检查实际目录和 Git 远程；不要创建空目录冒充项目，也不要猜测后执行移动或删除。
- Windows PowerShell 读写文本显式指定 UTF-8。Windows PowerShell 5.1 的默认输出编码不可靠；新文件优先用 UTF-8 无 BOM。
- 含空格或中文的路径使用正确引用；PowerShell 文件操作优先使用 `-LiteralPath`。删除或移动目录前检查解析后的绝对路径及作用范围。
- 搜索优先使用 `rg` / `rg --files`，排除 `node_modules/`、`dist/`、`src-tauri/target/` 和 `.git/`。
- 修改聚焦当前任务；避免顺带升级依赖、重排整仓代码或改写无关模块。提交前检查 diff，交付时说明修改、验证结果和未验证部分。

## 项目信息记录

- 项目记录入口为 `docs/README.md`。每次开始任务先读 `docs/STATUS.md`，涉及已有问题时再读 `docs/ISSUES.md` 及其证据链接；不要默认聊天历史或 README 功能概览代表已验证状态。
- `docs/STATUS.md` 只记录当前状态、当前验证基线和下一步；`docs/ISSUES.md` 是问题状态的唯一台账；`docs/DECISIONS.md` 记录已接受的长期决策及理由；`docs/WORKLOG.md` 按任务追加实际工作和验证结果。
- 每个有实质产出的任务结束前更新工作日志；改变状态、问题或长期约定时同步对应记录。没有实际变化的文件无需机械更新，避免重复抄写细节。
- 问题使用稳定编号 `M4W-001` 起连续分配，编号不复用。修复完成但实际用户路径未验证时标为“待验证”；只有满足该问题验收条件并附证据后才能标为“已关闭”。
- 历史审查及验证报告放在 `docs/audits/`，保留当时基线与结论；后续进展记录在台账，不把历史失败改写成从未发生。
- 验证记录包含日期、代码基线及未提交改动说明、平台、命令或操作、实际结果、未验证部分。未运行、静态确认、实际通过和失败必须区分，不记录真实凭据或用户数据库。
- 用户指定的固定 RSS 导入样例为 `tests/fixtures/opml/hn-popular10+1.opml`；预期与验收步骤见 `docs/TESTING.md`。后续 Feed / OPML 任务优先使用仓库副本，不依赖桌面原路径，不静默修改样例或替换应用的 bootstrap.opml。

## 项目定位

- 项目与仓库名称统一为 `mercury4win`。已有数据库路径、应用标识、localStorage 键及凭据格式属于兼容协议，改名时须保持兼容；凭据混淆使用固定历史字节，不得随项目名称修改。

mercury4win 是基于原始 Mercury 开发的本地优先、多平台 RSS 阅读器，支持 Windows、Linux 和 macOS。原始 Mercury 只支持 macOS，采用 Swift / SwiftUI；本项目采用 Tauri v2、Rust 与 React 实现多平台支持。订阅、文章、笔记、标签、设置及 AI 结果存储在本地 SQLite；AI 摘要、翻译和标签通过 OpenAI 兼容接口调用。没有 AI 配置时，非 AI 功能应可独立使用。

- 前端：React 19、TypeScript 6、Vite 8、Tailwind CSS v4、项目内的 UI 组件、TanStack Query、Zustand、i18next。
- 后端：Tauri v2、Rust 2021、Tokio、rusqlite / deadpool-sqlite、reqwest。
- 内容处理：decruft 提取正文，htmd 转 Markdown，comrak 渲染，阅读器主题包装。
- 依赖与版本以 `package.json`、锁文件、`src-tauri/Cargo.toml` 和 Tauri 配置为准；README 中的版本和环境说明可能滞后。

## 上游项目与功能对照

- 原始 Mercury 的本地参考仓库：`G:\sth_interesting\mercury`，远程仓库：`https://github.com/neolee/mercury.git`。当前工作仓库是 `mercury4win`，实际本地目录为 `G:\sth_interesting\mercury4win`；操作前仍须检查实际路径。
- `Mercury` / `mercury` 在上游引用、来源说明及历史兼容协议中具有明确含义，不应作为旧项目名批量替换。当前项目和仓库名称使用 `mercury4win`；应用中保留的 Mercury 产品名与安装身份另行处理兼容性。
- 对照功能或修复行为差异时，先阅读上游 README、`docs/` 中的相关功能文档，再查看 `Mercury/` 下对应的实现与测试；关注订阅、阅读管道、标签、AI 任务、翻译缓存、笔记和导出的业务约定。
- 上游实现和测试可作为语义参考，但 macOS 专用 API 与 SwiftUI 交互须结合 Tauri / React 和目标平台重新实现。以当前仓库代码及验证结果判断移植状态，不默认所有上游功能已完整移植。
- 上游 `PORTING.md` 是方案参考，不等同于本项目的实际架构；当前实现没有因此自动采用共享 Swift 模块。数据库结构相似也不代表两个应用的数据可直接互换，跨应用复用数据前需核对 Schema、迁移版本、缓存格式和路径。
- 默认只读上游参考仓库；除非当前任务明确涉及上游修改，否则将实现变更留在本项目。

## 代码导航

| 路径 | 职责 |
| --- | --- |
| `src/main.tsx`、`src/App.tsx` | 前端启动与应用入口 |
| `src/components/layout/` | 应用布局、侧边栏、状态栏、面板拖拽 |
| `src/components/ui/` | 可复用基础 UI 组件 |
| `src/features/` | feed、entry、reader、agent、tags、usage 功能界面 |
| `src/lib/tauri_bindings.ts` | 手工维护的 Tauri IPC 封装 |
| `src/types/` | 前端数据契约 |
| `src/stores/`、`src/hooks/` | UI 状态与复用交互 |
| `src/i18n/en.json`、`src/i18n/zh-Hans.json` | 英文与简体中文文案 |
| `src/styles/globals.css` | 全局样式与主题变量 |
| `src-tauri/src/lib.rs` | 插件初始化、共享状态、数据库初始化、IPC 命令注册 |
| `src-tauri/src/commands/` | Tauri 命令入口 |
| `src-tauri/src/db/` | 连接池、迁移、模型与各类 store |
| `src-tauri/src/feed/` | Feed 解析、同步和 OPML |
| `src-tauri/src/reader/` | 内容管道、HTML/Markdown 转换与阅读主题 |
| `src-tauri/src/agent/` | Provider、任务运行时、摘要、翻译、标签、凭据与 Prompt |
| `src-tauri/src/digest/` | 文摘生成和导出模板 |
| `src-tauri/src/utils/paths.rs` | 基于 Tauri Path API 的应用数据路径 |
| `src-tauri/resources/` | YAML Prompt、Handlebars 模板、初始 OPML |
| `src-tauri/tauri.conf.json`、`src-tauri/capabilities/` | 桌面配置、打包和权限 |
| `.github/workflows/release.yml` | Windows / Linux / macOS 构建与发布 |

## 开发命令

以下命令从仓库根目录运行，使用 npm 及现有锁文件。Windows 下如 PowerShell 的脚本策略阻止 npm/npx，可用 `npm.cmd` / `npx.cmd`。

| 命令 | 用途 |
| --- | --- |
| `npm ci --legacy-peer-deps` | 按锁文件安装依赖，与发布工作流一致；仓库 `.npmrc` 也启用了 legacy-peer-deps |
| `npm run dev` | 仅启动 Vite 前端，端口 5173 |
| `npm run tauri -- dev` | 完整桌面开发，启动前端和 Rust 后端 |
| `npx tsc -b` | TypeScript 类型检查 |
| `npm run lint` | oxlint 检查 |
| `npm run build` | TypeScript 检查并构建前端到 dist |
| `cargo check --locked --manifest-path src-tauri/Cargo.toml` | Rust 编译检查 |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | Rust 单元测试 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Rust 格式检查 |
| `npm run tauri -- build` | 完整桌面发布构建 |

- 仅浏览器运行不能验证 Tauri IPC、文件对话框和后端行为；相关功能需要完整桌面环境。
- Vite 8 的 Node 要求应查看已安装包的 `engines`；发布 CI 使用 Node 22，不要沿用 README 的 Node ≥18 作为充分条件。Rust 使用稳定版并结合 Cargo 依赖的实际要求。
- Windows 桌面构建需要可用的 MSVC 工具链及 WebView2；Linux 依赖参考发布工作流中的 WebKitGTK 4.1、GTK3、AppIndicator 和 librsvg 安装步骤。
- 初次 Rust 构建可能耗时较长；工具缺失或平台依赖不足时，报告具体阻塞和已完成的检查。

## 前端约定

- 沿用已有文件命名：文件及业务函数/变量多为 snake_case，React 组件为 PascalCase。采用 `@/` 导入别名和 `import type`。
- IPC 调用集中在 `src/lib/tauri_bindings.ts`，数据类型集中在 `src/types/`；组件调用封装，不散布重复的原始 invoke。
- TanStack Query 管理后端数据；写操作后更新或失效受影响的缓存，例如 entries 和 feeds。Zustand 用于主题、语言、字体等 UI 状态。
- 复用 `src/components/ui/` 和主题变量，保持亮/暗主题及窄窗口布局可用。
- 新增可见文案使用 i18next，并同步英文和简体中文资源键；图标按钮提供可理解的标签，维持键盘操作和焦点行为。
- 流式摘要、翻译和同步沿用 Tauri Channel 的现有消息协议；处理结束、错误、取消、组件卸载和文章切换，避免旧任务覆盖当前文章。
- 修改文章 HTML/Markdown 展示或链接处理时，检查不可信内容与外部链接行为，不扩大脚本执行能力或桌面权限。

## 后端与 IPC 约定

- Tauri 命令放在 `commands/`，数据库操作放在 `db/*_store.rs`，领域逻辑放在 feed、reader、agent 或 digest。
- 新增/修改命令时一起核对：Rust 命令签名与返回模型、`lib.rs` 的 invoke_handler 注册、TypeScript 封装及类型、所有调用方。
- 注意 Rust 参数 snake_case 与 invoke 参数对象 camelCase 的映射，以及 `Option`、null、错误和 Channel 消息的实际序列化形式。
- 阻塞 SQLite 操作使用现有连接池的 `interact` 模式；HTTP 和文件任务沿用异步模式，避免在长时间网络操作中持有共享锁或数据库事务。
- 任务运行时、流式协议、翻译分段/缓存和用量记录的变更应验证成功、失败及取消路径。
- 修改 Prompt 时核对 YAML 结构、模板变量和条件语法；修改导出时核对 Handlebars 模板、格式和编码。
- 平台相关行为使用 Tauri API 或明确的条件编译，保持 Windows / Linux / macOS 兼容。

## 数据与迁移

- 数据库位于应用数据目录的 `mercury.db`，路径通过 `utils/paths.rs` 和 Tauri Path API 获取；不要硬编码用户目录、盘符或平台路径。
- Schema 位于 `db/migrations.rs`，通过 settings 表中的 schema_version 跟踪版本。为已有用户新增结构时添加后续版本迁移并递增 SCHEMA_VERSION，不只修改 MIGRATION_V1。
- 迁移应覆盖新建数据库、旧版本升级和重复运行；保留用户订阅、笔记、标签及 AI 历史。
- 数据层变更同步 Rust 模型、SQL 和前端类型，检查外键、事务、分页、去重和缓存失效。
- 验证优先使用内存或临时数据库；不要清空实际用户数据作为修复或测试手段。
- API Key 的实际实现见 `agent/credential_store.rs`：当前是 XOR 混淆后的文件存储，不是已接入的 Stronghold 或安全加密。相关变更不能仅依据注释或前端依赖判断实现。
- 不在代码、日志、示例、测试或提交中写入真实密钥、凭据文件或用户数据库。

## 验证与交付

- Release、README 和项目文档不得包含 SHA-256 校验说明、对应附件或相关内容，也不得提及这一限制；该约定仅在本 AGENTS.md 中维护。
- 文档修改：核对命令、路径、配置事实和 UTF-8 编码，检查 Git diff；无需为纯文档修改新增测试或执行全套桌面构建。
- 前端修改：运行 `npm run lint` 和 `npm run build`；涉及交互时补充相应桌面验证。
- Rust 修改：运行 cargo check 及相关单元测试；可用模块过滤器缩小范围，例如 `cargo test --locked --manifest-path src-tauri/Cargo.toml reader::`。修改 Rust 文件时检查格式；已有格式问题不应导致无关的全仓重排。
- 现有 Rust 测试覆盖迁移、Prompt、任务运行时、正文提取、Markdown 转换和文摘模板；当前 package.json 没有前端 test 脚本，不要声称已运行不存在的测试。
- 涉及数据库升级、Feed/OPML、IPC、流式 AI、主题/字体或导出时，验证对应的实际用户路径，避免仅凭类型检查认定运行行为正常。
- 依赖修改维护对应锁文件，不手改锁文件；不要提交 node_modules、dist、target 或临时产物。
- 发布工作流会被 `v*` 标签或手动触发。发布相关任务需核对各配置版本与三平台打包；普通实现任务不顺带创建发布标签或发布版本。
- 最后报告改动文件、目的、通过/失败的检查和未验证部分。只把实际执行并通过的检查列为通过。
