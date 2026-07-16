# Mercury

**本地优先、开源、跨平台 RSS 阅读器** — 基于 Tauri v2（Rust + React 19），所有数据 100% 本地 SQLite 存储，集成 OpenAI 兼容 API 实现 AI 摘要、翻译与智能标签。

> v0.2.0 · MIT License · Windows / macOS / Linux

---

## 功能概览

| 功能 | 状态 | 说明 |
|------|------|------|
| Feed 订阅管理 | ✅ | RSS/Atom/JSON Feed 解析、添加、编辑、删除、批量同步 |
| OPML 导入/导出 | ✅ | 支持 OPML 1.0/1.1/2.0，完整 UI 对话框 |
| 条目列表 | ✅ | 游标分页、关键词搜索、未读筛选、已读/星标/删除 |
| 三栏可拖拽布局 | ✅ | 侧边栏 ↔ 条目列表 ↔ 阅读器 + 侧面板，自由调整宽度 |
| 阅读器模式 | ✅ | 4 层内容管道 — 正文提取 → Markdown → GFM 渲染 → 主题 CSS |
| 笔记 | ✅ | Markdown 编辑保存，逐篇文章独立笔记 |
| AI 摘要 | ✅ | 流式 SSE，3 级详细度（brief/medium/detailed），切换后自动重新生成 |
| AI 翻译 | ✅ | 分段并发翻译，上下文传递保持连贯，支持多种 prompt 策略 |
| AI 智能标签 | ✅ | LLM 分析文章内容建议 3-8 个标签，一键应用 |
| 标签管理 | ✅ | 标签库搜索、重命名、删除、合并、批量打标 |
| 文摘导出 | ✅ | 单篇/多篇导出，Markdown/HTML 格式，Handlebars 模板引擎 |
| Provider 管理 | ✅ | OpenAI 兼容 API，多 Provider + 多 Model，Agent Profile 独立配置 |
| Prompt 模板 | ✅ | YAML 模板 + `{{key}}` 替换 + `{{#cond}}` 条件渲染，支持自定义上传 |
| 用量统计 | ✅ | 7d/30d/90d Token 用量展示 |
| 国际化 | ✅ | 英语 / 简体中文，i18next + react-i18next |
| 暗色模式 | ✅ | light/dark/auto 三种模式，跟随系统 |
| 键盘快捷键 | ✅ | j/k 导航、m 已读、s 星标、r/t/n 面板切换等 11 个快捷键 |

---

## 技术架构

```
┌────────────────────────────────────────────────┐
│  React 19 + TypeScript                         │
│  (Tailwind CSS v4 · shadcn/ui · TanStack Query │
│   Zustand · i18next)                           │
├────────────────────────────────────────────────┤
│  Tauri v2 IPC Bridge (40+ Commands)            │
├────────────────────────────────────────────────┤
│  Rust Backend                                  │
│  ┌──────────┬──────────┬──────────┬─────────┐ │
│  │ Feed Sync│ Reader   │ AI Agent │ Digest  │ │
│  │ Parser   │ Pipeline │ Runtime  │ Export  │ │
│  └──────────┴──────────┴──────────┴─────────┘ │
├────────────────────────────────────────────────┤
│  SQLite (WAL · deadpool-sqlite · 17 tables)    │
└────────────────────────────────────────────────┘
```

### 大模型中立的 AI 架构

Mercury **不绑定任何特定 LLM 服务商**，所有 AI 功能通过统一的 OpenAI 兼容 API 协议调用，天然支持：

- **云端服务**：OpenAI、Anthropic、Groq、Together AI、DeepSeek 等
- **本地模型**：Ollama、LM Studio、vLLM、LocalAI 等任何提供 `/v1/chat/completions` 端点的服务
- **Agent Profile**：每种任务类型（Summary/Translation/Tagging）可独立配置首选模型和目标语言

---

## 快速开始

### 前置条件

| 工具 | 版本 | 检查 |
|------|------|------|
| Node.js | ≥ 18 | `node --version` |
| Rust | ≥ 1.77.2 | `rustc --version` |
| Git | 任意 | `git --version` |

**Linux 额外依赖**：`sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev`

Windows 和 macOS 无需额外安装。

### 运行

```bash
git clone git@github.com:Selvori/mercury4win-linux.git
cd mercury4win-linux
npm install
npx tauri dev
```

首次运行会编译 Rust 依赖（300+ crates，5-15 分钟），之后增量编译只需几秒。

### 开发命令

```bash
npm run dev        # 仅前端开发（不含 Rust 后端）
npx tauri dev      # 完整 Tauri 开发模式
npx tsc -b         # TypeScript 类型检查
npm run lint       # oxlint 代码检查
npx tauri build    # 构建发布版本
```

### 构建产物

```
src-tauri/target/release/
├── mercury.exe                              (49 MB)
└── bundle/
    ├── nsis/Mercury_0.2.0_x64-setup.exe     (11 MB)
    └── msi/Mercury_0.2.0_x64_en-US.msi      (16 MB)
```

---

## 使用指南

### AI 功能配置

AI 功能（摘要/翻译/标签）需要配置 OpenAI 兼容 API Key：

1. 点击侧边栏 **Settings** → **Providers** → **Add Provider**
2. 填写 Provider 名称、Base URL（如 `https://api.openai.com/v1`）、API Key
3. 切换到 **Models** 标签，选择 Provider 后添加 Model（如 `gpt-4o-mini`）
4. 切换到 **Profiles** 标签，为每种任务类型选择模型和目标语言
5. 不配置 AI 也可正常使用所有非 AI 功能

### 阅读器面板

阅读器顶部工具栏：

| 图标 | 功能 |
|------|------|
| 📖 | 阅读器视图 |
| 📝 | 笔记编辑 |
| ✨ | AI 摘要（3 级详细度，切换后自动重新生成） |
| 🌐 | AI 翻译（分段展示） |
| 🏷️ | AI 标签建议 + 手动标签 |

### 键盘快捷键

| 快捷键 | 功能 |
|--------|------|
| `j` / `↓` | 下一个条目 |
| `k` / `↑` | 上一个条目 |
| `Enter` / `o` | 打开当前条目 |
| `m` | 切换已读/未读 |
| `s` | 切换星标 |
| `r` | 阅读器面板 |
| `t` | 翻译面板 |
| `n` | 笔记面板 |
| `Shift + A` | 全部标为已读 |
| `Shift + R` | 刷新当前 Feed |
| `Ctrl + N` | 添加 Feed |

---

## 项目结构

```
├── src/                          # React 前端
│   ├── components/layout/        # AppShell, Sidebar, StatusBar, ResizeHandle
│   ├── components/ui/            # shadcn/ui 组件 (Button, Dialog, Input 等)
│   ├── features/
│   │   ├── agent/components/     # AI Provider/Model/Profile/Prompt 配置 UI
│   │   ├── entry/components/     # 条目列表
│   │   ├── feed/components/      # Feed 管理, OPML 导入导出
│   │   ├── reader/components/    # 阅读器, 摘要, 翻译, 笔记, 标签面板
│   │   ├── tags/components/      # 标签库管理
│   │   └── usage/components/     # Token 用量统计
│   ├── hooks/                    # 键盘快捷键, 面板拖拽
│   ├── i18n/                     # en.json, zh-Hans.json
│   ├── lib/                      # Tauri IPC 绑定, 工具函数
│   ├── stores/                   # Zustand (主题, 语言)
│   └── types/                    # TypeScript 类型定义
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── commands/             # 40+ Tauri IPC 命令
│   │   ├── db/                   # 数据层 (连接池, 迁移, 模型, 7 个 store)
│   │   ├── feed/                 # Feed 解析, 同步, OPML
│   │   ├── reader/               # 内容管道, 主题 CSS
│   │   ├── agent/                # AI 运行时, Provider, 模板引擎, 密钥存储
│   │   ├── digest/               # Handlebars 导出模板
│   │   └── utils/                # 路径工具
│   └── resources/                # YAML Prompt 模板, HBS 导出模板, Bootstrap OPML
├── package.json
├── vite.config.ts
└── tsconfig.json
```

### 数据库 Schema (17 表)

**核心**：`feed`, `entry`, `content`, `content_html_cache`, `entry_note`
**标签系统**：`tag`, `tag_alias`, `entry_tag`
**AI 配置**：`agent_provider_profile`, `agent_model_profile`, `agent_profile`
**AI 运行时**：`agent_task_run`, `llm_usage_event`
**翻译**：`translation_result`, `translation_segment`
**摘要**：`summary_result`
**系统**：`settings`

---

## 技术栈明细

### Rust 后端

| 库 | 用途 |
|----|------|
| tauri 2 | 桌面运行时 |
| rusqlite + deadpool-sqlite | SQLite (WAL, 连接池 8) |
| feed-rs | RSS/Atom/JSON Feed 解析 |
| opml | OPML 1.0/1.1/2.0 |
| decruft + htmd + comrak | 内容提取 → Markdown → GFM 渲染 |
| reqwest + futures-util | HTTP 客户端 + SSE 流式 |
| handlebars + serde_yaml | 导出模板 + Prompt 模板 |
| tokio | 异步运行时 |
| sha2 + hex | 翻译缓存哈希 |

### React 前端

| 库 | 用途 |
|----|------|
| React 19 + TypeScript 6 | UI 框架 |
| Tailwind CSS v4 | 样式 |
| shadcn/ui + Lucide React | 组件库 + 图标 |
| TanStack Query 5 | 服务端状态缓存 |
| Zustand 5 | 全局状态 (主题/语言) |
| i18next + react-i18next | 国际化 |
| @mozilla/readability | 前端内容提取回退 |
| marked | Markdown 渲染 |

### 项目规模

| 指标 | 数值 |
|------|------|
| Rust 源文件 | 47 |
| TypeScript/TSX 源文件 | 44 |
| 数据库表 | 17 |
| Tauri IPC 命令 | 40+ |
| 发布二进制 | 49 MB |
| 安装包 | ~11 MB (NSIS) / ~16 MB (MSI) |

---

## License

MIT
