# Mercury

**本地优先、开源、跨平台 RSS 阅读器** — 基于 Tauri v2（Rust + React 19），所有数据 100% 本地 SQLite 存储，集成 OpenAI 兼容 API 实现 AI 摘要、翻译与智能标签。

> 版本 0.1.0 · MIT License · Windows / macOS / Linux

---

## 目录

1. [技术架构](#技术架构)
2. [① 基础功能：Feed / OPML 解析 + Sync + 内容呈现](#-基础功能feed--opml-解析--sync--内容呈现)
3. [② 内容清洗：Cleaned HTML + Cleaned Markdown + 定制样式](#-内容清洗cleaned-html--cleaned-markdown--定制样式)
4. [③ AI 功能之一：Summary Agent + LLM Providers](#-ai-功能之一summary-agent--llm-providers)
5. [④ AI 功能之二：Translation Agent](#-ai-功能之二translation-agent)
6. [⑤ 辅助功能：多语言支持、日志上报与调试工具](#-辅助功能多语言支持日志上报与调试工具)
7. [⑥ 辅助功能：大语言模型用量统计](#-辅助功能大语言模型用量统计)
8. [⑦ 笔记与文摘导出：笔记 + 单篇导出 + 多篇导出](#-笔记与文摘导出笔记--单篇导出--多篇导出)
9. [⑧ 标签系统：文章标签 + 按标签筛选 + Tag Agent + 标签管理](#-标签系统文章标签--按标签筛选--tag-agent--标签管理)
10. [大模型中立的 LLM Provider 架构](#大模型中立的-llm-provider-架构)
11. [OPML 导入与导出](#opml-导入与导出)
12. [键盘快捷键](#键盘快捷键)
13. [快速开始](#快速开始)
14. [项目结构](#项目结构)

---

## 技术架构

```
┌──────────────────────────────────────────────┐
│  React 19 + TypeScript                       │
│  (Tailwind CSS v4 · shadcn/ui · Zustand)     │
├──────────────────────────────────────────────┤
│  Tauri v2 IPC Bridge (33 Commands)           │
├──────────────────────────────────────────────┤
│  Rust Backend                                │
│  ┌──────────┬──────────┬──────────┬───────┐ │
│  │ Feed Sync│ Reader   │ AI Agent │ Digest│ │
│  │ Parser   │ Pipeline │ Runtime  │Export │ │
│  └──────────┴──────────┴──────────┴───────┘ │
├──────────────────────────────────────────────┤
│  SQLite (WAL · deadpool-sqlite · 17 tables)  │
└──────────────────────────────────────────────┘
```

---

## ① 基础功能：Feed / OPML 解析 + Sync + 内容呈现

### 已完成 ✅

| 功能 | 实现位置 | 说明 |
|------|---------|------|
| RSS/Atom/JSON Feed 解析 | `src-tauri/src/feed/parser.rs` | 基于 `feed-rs` crate，支持全部主流格式 |
| Feed CRUD | `src-tauri/src/commands/feed_cmd.rs` | 添加、编辑、删除 Feed 订阅 |
| 批量同步 | `src-tauri/src/feed/sync_service.rs` | HTTP 获取 → 解析 → 去重 → 事务批量 upsert，每次 3 个 |
| Cloudflare 绕过 | `src-tauri/src/feed/sync_service.rs` | Chrome 131 UA + 挑战页面检测 |
| 条目去重 | `src-tauri/src/db/entry_store.rs` | 按 `(feed_id, guid)` 或 `(feed_id, url)` 去重 |
| 条目列表 | `src/features/entry/components/entry_list.tsx` | 游标分页、搜索、未读筛选、星标筛选 |
| 条目操作 | `src-tauri/src/commands/entry_cmd.rs` | 已读/未读切换、星标、删除 |
| 三栏布局 | `src/components/layout/app_shell.tsx` | 侧边栏 | 条目列表 | 阅读器 |
| Feed 侧边栏 | `src/components/layout/sidebar.tsx` | 显示未读计数，Feeds/Tags 视图切换 |
| 12 个键盘快捷键 | `src/hooks/use_keyboard_shortcuts.ts` | j/k 导航、m 已读、s 星标、r 阅读器等 |
| Bootstrap OPML | `src-tauri/resources/bootstrap.opml` | 首次启动预装示例 Feed |

### 待实现 / 占位符 ⚠️

| 项目 | 现状 | 位置 |
|------|------|------|
| OPML 导入/导出 UI 组件 | 后端逻辑已完成，前端为 `return null` 占位符 | `src/features/feed/components/opml_import.tsx`、`opml_export.tsx` |
| 状态栏动态反馈 | 当前固定显示 "Ready" | `src/components/layout/status_bar.tsx` |
| 应用内通知/Toast | 未实现 | — |

---

## ② 内容清洗：Cleaned HTML + Cleaned Markdown + 定制样式

### 已完成 ✅

| 功能 | 实现位置 | 说明 |
|------|---------|------|
| 4 层内容管道 | `src-tauri/src/reader/pipeline.rs` | 原始 HTML → decruft 提取 → htmd 转 Markdown → comrak 渲染 GFM HTML |
| decruft 内容提取 | `src-tauri/src/reader/extraction.rs` | Rust 原生文章正文提取 |
| HTML→Markdown 转换 | `src-tauri/src/reader/html_to_md.rs` | 基于 `htmd` crate |
| Markdown→阅读器 HTML | `src-tauri/src/reader/md_to_html.rs` | 基于 `comrak` (GFM)，注入主题 CSS |
| 版本化缓存 | `src-tauri/src/reader/pipeline.rs` | 三级版本号缓存失效 |
| 4 套阅读器主题 | `src-tauri/src/reader/theme.rs` | classic/paper × light/dark，内联 CSS 注入 |
| 降级页面 | `src-tauri/src/reader/pipeline.rs` | 提取失败时生成带元数据和原文链接的个性化降级页面 |
| 阅读器面板 | `src/features/reader/components/reader_pane.tsx` | 主题切换、内容渲染 |

### 待实现 / 占位符 ⚠️

| 项目 | 现状 | 位置 |
|------|------|------|
| Readability.js 前端回退 | 完整实现但**未接入管道** | `src/features/reader/hooks/use_reader_fallback.ts` |
| decruft 失败后的前端补救 | `needs_fallback()` 已定义，未与 `use_reader_fallback` 对接 | `src-tauri/src/db/models.rs` |

> **集成方案**：当 Rust `decruft` 返回 `needs_fallback()` 时，走 Tauri IPC → 前端 `extractViaReadability()` → 回传结果 → 继续 Markdown 转换。

---

## ③ AI 功能之一：Summary Agent + LLM Providers

### 已完成 ✅

| 功能 | 实现位置 | 说明 |
|------|---------|------|
| OpenAI 兼容 Provider 管理 | `src-tauri/src/commands/agent_cmd.rs` | 添加/删除/测试连接 |
| Provider 配置 UI | `src/features/agent/components/agent_settings.tsx` | 名称、Base URL、API Key |
| API Key 安全存储 | `src-tauri/src/agent/credential_store.rs` | XOR 混淆 + hex 编码 + 文件权限 0o600 |
| 流式 AI 摘要 | `src-tauri/src/agent/summary.rs` | SSE 流式接收，通过 Tauri Channel 推送 |
| 3 个详细级别 | `src/features/reader/components/reader_summary.tsx` | brief / medium / detailed |
| 摘要 Prompt 模板 | `src-tauri/resources/prompts/summary.default.yaml` | YAML 模板 + `{{key}}` 变量替换 + `{{#cond}}` 条件渲染 |
| Prompt 模板引擎 | `src-tauri/src/agent/prompt_templates.rs` | 通用模板加载与渲染，支持自定义模板文件 |
| Token 用量记录 | `src-tauri/src/agent/summary.rs` | 完成后自动写入 `llm_usage_event` 表 |
| Agent 任务队列 | `src-tauri/src/agent/runtime.rs` | 1 活跃 + 1 等待槽位，最新替换策略 |

### 待实现 / 占位符 ⚠️

| 项目 | 现状 | 位置 |
|------|------|------|
| Summary 前端调用 | `run_summary` 被注释，显示 "requires Tauri runtime" 占位 | `src/features/reader/components/reader_summary.tsx` |
| Model Profile 管理 UI | 后端 CRUD 完整，前端无界面 | `src-tauri/src/db/agent_store.rs` |
| Agent Profile（各任务模型选择） | 后端数据模型就绪，前端无配置入口 | `src-tauri/src/db/models.rs` |
| Stronghold 加密 | 已依赖 stronghold 插件，仅用 XOR 回退 | `src-tauri/src/agent/credential_store.rs` |
| 自定义 Prompt 模板上传 | `load_custom_template()` 已实现，无 UI 入口 | `src-tauri/src/agent/prompt_templates.rs` |

---

## ④ AI 功能之二：Translation Agent

### 已完成 ✅

| 功能 | 实现位置 | 说明 |
|------|---------|------|
| 分段翻译 | `src-tauri/src/agent/translation.rs` | 按 `\n\n` 分段，逐段调用 LLM |
| 上下文传递 | `src-tauri/src/agent/translation.rs` | 每段翻译时传入上一段译文 |
| 双 Prompt 策略 | `src-tauri/resources/prompts/` | `translation.default.yaml` (v4) + `translation.hy-mt.yaml` (汉译优化) |
| 进度推送 | `src-tauri/src/agent/translation.rs` | 通过 Channel 推送 `progress` 事件 |
| 译文分段展示 | `src/features/reader/components/reader_translation.tsx` | 按 `segment` 事件逐段渲染 |
| 源内容哈希 | `src-tauri/src/agent/translation.rs` | SHA-256 计算，为缓存失效准备 |

### 待实现 / 占位符 ⚠️

| 项目 | 现状 | 位置 |
|------|------|------|
| 翻译前端调用 | `run_translation` 未实际调用，显示占位文本 | `src/features/reader/components/reader_translation.tsx` |
| 分段并发 | `concurrency` 声明为 3，但执行是串行 `for` 循环 | `src-tauri/src/agent/translation.rs` |
| 翻译缓存失效 | `compute_source_content_hash` 已实现但未集成 | `src-tauri/src/agent/translation.rs` |
| 翻译结果持久化 | `TranslationResult` / `TranslationSegment` 模型已定义但不写入 DB | `src-tauri/src/db/models.rs` |

> **翻译并发化**：将串行 `for` 改为 `buffer_unordered(concurrency)` 即可启用。

---

## ⑤ 辅助功能：多语言支持、日志上报与调试工具

### 已完成 ✅

| 功能 | 实现位置 | 说明 |
|------|---------|------|
| i18n 文件骨架 | `src/i18n/en.json`、`zh-Hans.json` | 文件存在，Zustand store 中有 `locale` 状态 |
| 日志插件 | `src-tauri/src/lib.rs` | `tauri-plugin-log` 已注册 |

### 待实现 / 占位符 ⚠️

| 项目 | 现状 | 位置 |
|------|------|------|
| 国际化文本 | `en.json` 和 `zh-Hans.json` 均为空 `{}` | `src/i18n/` |
| i18n 前端框架集成 | 无 `i18next` / `react-intl` 依赖 | — |
| 日志查看/导出 UI | 后端写日志，前端无法查看 | — |
| 开发者工具面板 | 未实现 | — |

> **实现路径**：安装 `i18next` + `react-i18next`，填充 JSON 文件，替换硬编码文本为 `t('key')`。

---

## ⑥ 辅助功能：大语言模型用量统计

### 已完成 ✅

| 功能 | 实现位置 | 说明 |
|------|---------|------|
| 用量事件记录 | `src-tauri/src/db/usage_store.rs` | `llm_usage_event` 表写入 prompt/completion/total tokens |
| 时间窗口查询 | `src-tauri/src/db/usage_store.rs` | 7d / 30d / 90d 汇总查询 |
| 用量报告 IPC | `src-tauri/src/commands/settings_cmd.rs` | `get_usage_report` command |
| Summary Agent 用量记录 | `src-tauri/src/agent/summary.rs` | 流式完成后自动插入 |

### 待实现 / 占位符 ⚠️

| 项目 | 现状 | 位置 |
|------|------|------|
| Translation / Tagging 用量记录 | `run_translation` 和 `run_tagging` 不记录 token 用量 | `src-tauri/src/agent/translation.rs`、`tagging.rs` |
| 用量统计前端 UI | 无展示界面 | — |
| 按 Provider / Model 分组统计 | 只实现了时间窗口总数 | `src-tauri/src/db/usage_store.rs` |

---

## ⑦ 笔记与文摘导出：笔记 + 单篇导出 + 多篇导出

### 已完成 ✅

| 功能 | 实现位置 | 说明 |
|------|---------|------|
| 笔记 CRUD（后端） | `src-tauri/src/db/note_store.rs` | `get_note` / `save_note` / `delete_note`，Markdown 格式 |
| 单篇摘要导出 | `src-tauri/src/commands/digest_cmd.rs` | `generate_digest` + `export_digest` |
| 多篇摘要导出 | `src-tauri/src/commands/digest_cmd.rs` | `export_multi_digest` |
| 3 套导出模板 | `src-tauri/resources/digest_templates/` | single_markdown、single_text (HTML)、multiple_markdown |
| Handlebars 模板引擎 | `src-tauri/src/digest/templates.rs` | 编译期嵌入模板，运行时渲染 |
| 单元测试 | `src-tauri/src/digest/templates.rs` | 4 个测试覆盖所有模板 |

### 待实现 / 占位符 ⚠️

| 项目 | 现状 | 位置 |
|------|------|------|
| 笔记前端保存 | `save_note` import 被注释，用 `setTimeout` 占位 | `src/features/reader/components/reader_note.tsx` |
| 笔记前端加载 | 未从后端加载已有笔记 | `src/features/reader/components/reader_note.tsx` |
| 笔记 IPC 注册 | `save_note` / `get_note` 未在 `lib.rs` 注册为 Tauri command | `src/lib/tauri_bindings.ts` |
| 摘要导出 UI | 无导出格式选择、保存路径选择的图形界面 | — |

> **笔记前端接入**：取消 `reader_note.tsx` import 注释 → 注册 Tauri command → `useQuery` 加载已有笔记 → `useMutation` 保存。

---

## ⑧ 标签系统：文章标签 + 按标签筛选 + Tag Agent + 标签管理

### 已完成 ✅

| 功能 | 实现位置 | 说明 |
|------|---------|------|
| 标签 CRUD（后端） | `src-tauri/src/db/tag_store.rs` | `list_tags` / `add_tag` / `remove_tag` / `rename_tag` / `delete_tag` |
| 标签合并 | `src-tauri/src/db/tag_store.rs` | 合并标签时迁移所有关联条目 |
| 批量打标 | `src-tauri/src/db/tag_store.rs` | 时间范围 + 多标签批量操作 |
| 标签规范化 | `src-tauri/src/db/tag_store.rs` | `normalized_name` 字段大小写统一 |
| Provisional 标签 | `src-tauri/src/db/models.rs` | AI 生成的暂定标签标记 |
| Tag Agent（后端） | `src-tauri/src/agent/tagging.rs` | LLM 建议 3-8 个标签（非流式） |
| 标签库管理 UI | `src/features/tags/components/tag_library.tsx` | 搜索、重命名、删除、暂定标签标记 |
| 标签输入 UI | `src/features/tags/components/tag_input.tsx` | 自动补全输入组件 |
| 标签 IPC 命令 | `src-tauri/src/commands/tag_cmd.rs` | 全部 7 个标签操作命令 |

### 待实现 / 占位符 ⚠️

| 项目 | 现状 | 位置 |
|------|------|------|
| Tag Agent 前端面板 | `reader_tagging.tsx` 为 `return null` 占位符 | `src/features/reader/components/reader_tagging.tsx` |
| 按标签筛选条目 | 后端 `entry_store` 支持 tag_mode 但前端未传递 | `src/features/entry/components/entry_list.tsx` |
| 侧边栏 Tags 视图 | 显示 "Tags will be available in Phase 5" | `src/components/layout/sidebar.tsx` |

> **Tags 视图接入**：在 `sidebar.tsx` 的 `view === "tags"` 分支中嵌入 `TagInput` + 标签列表，支持点击标签筛选条目。

---

## 大模型中立的 LLM Provider 架构

### 设计原则

Mercury **不绑定任何特定 LLM 服务商**，所有 AI 功能通过统一的 OpenAI 兼容 API 协议调用，天然支持：

- **云端服务**：OpenAI、Anthropic（兼容 API）、Groq、Together AI、DeepSeek 等
- **本地模型**：Ollama、LM Studio、vLLM、LocalAI 等任何提供 `/v1/chat/completions` 端点的服务
- **自定义服务**：自建网关、代理服务等

### 实现细节

| 组件 | 位置 | 说明 |
|------|------|------|
| Provider 配置 | `src-tauri/src/db/models.rs` | `name` + `base_url` + 可选 `display_name` |
| 模型配置 | `src-tauri/src/db/models.rs` | 每个 Provider 可配多个模型，按能力分类 |
| Agent 配置 | `src-tauri/src/db/models.rs` | 每种任务类型可指定主模型 + 备用模型 |
| HTTP 客户端 | `src-tauri/src/agent/provider.rs` | `chat_completion()` + `chat_completion_stream()` (SSE) |
| API Key 管理 | `src-tauri/src/agent/credential_store.rs` | 独立存储，按 provider 隔离 |

### 测试连接

Provider Settings UI (`agent_settings.tsx`) 提供一键测试连接功能，调用 `test_provider_connection` command 验证有效性。

---

## OPML 导入与导出

### 后端（已完成）

| 功能 | 位置 | 说明 |
|------|------|------|
| OPML 导入 | `src-tauri/src/feed/opml_import.rs` | 支持 OPML 1.0 / 1.1 / 2.0 |
| OPML 导出 | `src-tauri/src/feed/opml_export.rs` | 标准 OPML 2.0 XML |
| IPC 命令 | `src-tauri/src/commands/feed_cmd.rs` | `import_opml` + `export_opml` |
| Bootstrap OPML | `src-tauri/resources/bootstrap.opml` | 首次使用预装载示例 Feed |

### 前端（功能可用但组件为占位符）

OPML 导入/导出功能实际上可用——`sidebar.tsx` 直接使用 `tauri-plugin-dialog` 选择文件并调用后端 IPC 命令，效果完整。但 `opml_import.tsx` 和 `opml_export.tsx` 组件本身是 `return null` 占位符，预留了 UI 扩展空间（如导入预览、选择导入源等）。

---

## 键盘快捷键

| 快捷键 | 功能 |
|--------|------|
| `j` / `↓` | 下一条目 |
| `k` / `↑` | 上一条目 |
| `Enter` / `o` | 打开/选中当前条目 |
| `m` | 切换已读/未读 |
| `s` | 切换星标 |
| `r` | 打开阅读器面板 |
| `t` | 打开翻译面板 |
| `n` | 打开笔记面板 |
| `Shift + A` | 全部标为已读 |
| `Shift + R` | 刷新当前 Feed |
| `Ctrl + N` | 添加 Feed |

> 输入框/文本域中自动禁用快捷键。

---

## 快速开始

### 前置条件

#### 必需安装的软件

| 工具 | 最低版本 | 检查命令 | 说明 |
|------|---------|---------|------|
| **Node.js** | ≥ 18 | `node --version` | 运行时 + npm 包管理 |
| **Rust** | ≥ 1.77.2 | `rustc --version` | Rust 编译器 + Cargo 构建系统 |
| **Git** | 任意 | `git --version` | Clone 仓库 |

#### 平台特定依赖

| 平台 | 需要 | 说明 |
|------|------|------|
| **Windows** | WebView2 Runtime | Win 10/11 预装，无需额外安装 |
| **macOS** | ≥ 12.0 | 使用系统内置 WKWebView，无需额外安装 |
| **Linux** | `libwebkit2gtk-4.1-0` + `libgtk-3-0` | Ubuntu/Debian: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev` |

#### 确认无误的项目（无需额外安装）

| 项 | 原因 |
|---|------|
| SQLite | `rusqlite` crate 开启 `bundled` 特性，编译时自动内嵌 SQLite 3 |
| Tauri CLI | `npx tauri` 会自动下载正确的 CLI 版本 |
| Visual Studio / C++ Build Tools | Rust 工具链自带链接器（GNU 和 MSVC 工具链均可） |

### 环境验证

在终端依次运行以下命令，确认全部通过：

```bash
node --version    # 应 ≥ 18
rustc --version   # 应 ≥ 1.77.2
git --version     # 应输出 git version x.x.x
```

### 从 Clone 到运行

```bash
# 1. Clone 仓库
git clone git@github.com:Selvori/mercury4win-linux.git
cd mercury4win-linux

# 2. 安装前端依赖
npm install

# 3. 启动 Tauri 开发模式
npx tauri dev
```

首次运行 `npx tauri dev` 时：
- 自动下载 Tauri CLI（约 30 秒）
- 编译 Rust 依赖（约 300+ crates，首次耗时 5-15 分钟，之后增量编译只需几秒）
- 启动 Vite 开发服务器 + Rust 后端

### 开发命令

```bash
# 仅前端开发（不启动 Rust 后端，AI 功能不可用）
npm run dev

# 启动 Tauri 开发模式（完整应用）
npx tauri dev

# TypeScript 类型检查
npx tsc -b

# 代码检查
npm run lint
```

### 构建与打包

```bash
# 完整构建（生成 Windows .exe + .msi 安装程序）
npx tauri build

# 仅编译 Rust 发布版
cargo build --release --manifest-path src-tauri/Cargo.toml
```

构建产物位于 `src-tauri/target/release/bundle/`：
- `nsis/Mercury_x.x.x_x64-setup.exe` — NSIS 安装程序
- `msi/Mercury_x.x.x_x64_en-US.msi` — MSI 安装程序

### 注意事项

- **AI 功能按需配置**：摘要、翻译、标签等 AI 功能需要配置 OpenAI 兼容 API Key。不配置也可正常使用所有非 AI 功能（Feed 订阅、阅读、笔记等）。
- **跨平台开发**：macOS 用户无需额外配置；Linux 用户参照上方平台依赖安装 WebKit 库。
- **Firewall / VPN**：部分 Feed 源可能被网络限制，建议开发环境中配置可访问海外站点的网络环境。

---

## 项目结构

```
mercury4win-linux/
├── src/                          # React 前端（TypeScript）
│   ├── components/layout/        # 布局组件（AppShell, Sidebar, StatusBar）
│   ├── components/ui/            # shadcn/ui 基础组件
│   ├── features/
│   │   ├── feed/components/      # Feed 管理
│   │   ├── entry/components/     # 条目列表
│   │   ├── reader/components/    # 阅读器、摘要、翻译、笔记、标签面板
│   │   ├── tags/components/      # 标签输入与管理
│   │   └── agent/components/     # AI Provider 配置
│   ├── hooks/                    # 键盘快捷键
│   ├── i18n/                     # 国际化（骨架已就绪）
│   ├── lib/                      # 工具函数、Tauri IPC 绑定、常量
│   ├── stores/                   # Zustand 状态管理
│   └── types/                    # TypeScript 类型定义
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── commands/             # 33 个 Tauri IPC 命令
│   │   ├── db/                   # 数据层（连接池、迁移、模型、17 个 store）
│   │   ├── feed/                 # Feed 解析、同步、OPML
│   │   ├── reader/               # 内容管道、主题
│   │   ├── agent/                # AI 运行时、Provider、模板
│   │   ├── digest/               # 摘要导出
│   │   └── utils/                # 路径工具
│   └── resources/                # YAML 提示词、HBS 模板、OPML
├── package.json
├── vite.config.ts
└── tsconfig.json
```

---

## 路线图概要

| 优先级 | 项目 | 依赖后端 |
|--------|------|----------|
| P0 | 笔记前端接入（save_note IPC 注册 + ReaderNote 加载/保存） | note_store.rs ✅ |
| P0 | Tag Agent 前端面板（reader_tagging.tsx） | tagging.rs ✅ |
| P0 | 侧边栏 Tags 视图 | tag_store.rs ✅ |
| P0 | Summary / Translation 前端调用 | summary.rs / translation.rs ✅ |
| P1 | Readability.js 回退接入管道 | use_reader_fallback.ts ✅ |
| P1 | i18n 国际化 | i18n JSON 文件（骨架就绪） |
| P1 | Model Profile / Agent Profile 配置 UI | agent_store.rs ✅ |
| P1 | 翻译并发化 | translation.rs ✅ |
| P2 | 用量统计前端 UI | usage_store.rs ✅ |
| P2 | Stronghold 加密 | credential_store.rs（XOR 回退可用） |
| P2 | 自定义 Prompt 模板 UI | prompt_templates.rs ✅ |
| P2 | 翻译/标签用量记录 | 参考 summary.rs 模板 |
| P3 | 日志查看 UI | tauri-plugin-log ✅ |
| P3 | OPML 导入导出 UI 组件 | 后端已完成 |
