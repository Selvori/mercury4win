# Mercury v0.1.0 — 项目报告

> 跨平台 RSS 阅读器 | Tauri v2 (Rust + React TypeScript)  
> 生成日期：2026-07-15 | 版本：v0.1.0

---

## 一、项目概述

Mercury 是一款 **本地优先、开源免费的 RSS 阅读器**，支持 **Windows / macOS / Linux** 三平台。项目源自 macOS SwiftUI 原生应用 Mercury 的跨平台移植，技术栈全面替换为 **Tauri v2 + Rust + React 19 + TypeScript**，在保留原应用核心设计理念的基础上，实现了更广泛的平台支持。

Mercury 提供完整的 Feed 订阅管理、阅读器模式文章提取与渲染，以及基于 OpenAI 兼容 API 的 AI 摘要、翻译和智能标签功能。所有数据 100% 存储在用户本地，无需注册账号，无需云服务。

---

## 二、技术栈

### 桌面框架

| 组件 | 技术 | 版本 |
|------|------|------|
| 桌面运行时 | Tauri v2 | ≥2.11 |
| 系统 WebView | WebView2 (Win) / WKWebView (Mac) / WebKitGTK (Lin) | — |

### 后端 (Rust)

| 领域 | 库 | 版本 | 用途 |
|------|-----|------|------|
| 数据库 | rusqlite + deadpool-sqlite | 0.38 / 0.13 | SQLite 连接 (bundled 编译) |
| 数据库模式 | WAL + 连接池 (max_size=8) | — | 并发读写不互斥 |
| Feed 解析 | feed-rs | 2 | RSS / Atom / JSON Feed |
| OPML | opml | 1 | OPML 1.0/1.1/2.0 导入 |
| 内容提取 | decruft | 0.2 | Rust 原生可读性提取 |
| HTML→Markdown | htmd | 0.5 | Turndown.js Rust 移植 |
| Markdown→HTML | comrak | 0.29 | GFM 渲染 (表格/删除线/任务列表/自动链接) |
| HTTP 客户端 | reqwest | 0.12 | 同步抓取 + SSE 流式 |
| 模板引擎 | Handlebars | 6 | 文摘导出模板 |
| YAML 解析 | serde_yaml | 0.9 | 提示词模板加载 |
| 密钥存储 | 自定义 XOR + 十六进制文件 | — | API Key 本地混淆存储 |
| 异步运行时 | Tokio | 1 | Tauri 默认运行时 |
| 时间处理 | chrono | 0.4 | ISO 8601 序列化 |
| UUID 生成 | uuid | 1 | Provider/Model ID |
| 流式工具 | futures-util | 0.3 | SSE 字节流消费 |
| 哈希 | sha2 + hex | 0.10/0.4 | 翻译缓存失效 |
| HTML 转义 | html-escape | 0.2 | 降级页安全输出 |

### 前端 (React + TypeScript)

| 领域 | 库 | 版本 | 用途 |
|------|-----|------|------|
| UI 框架 | React + TypeScript | 19 + ~6.0 | 组件化 UI |
| 构建工具 | Vite | 8 | HMR 开发服务器 + 生产构建 |
| CSS 框架 | Tailwind CSS | v4 | 工具优先 CSS + 暗色模式 |
| 组件库 | shadcn/ui (自定义) | — | 无运行时依赖的组件源码 |
| 图标 | Lucide React | 1.24 | 一致性图标系统 |
| 服务端状态 | TanStack Query | 5 | 数据获取/缓存/失效 |
| 客户端状态 | Zustand | 5 | 主题/语言全局状态 |
| 内容提取 (fallback) | @mozilla/readability | 0.6 | 前端 Readability.js 兜底 |
| 工具函数 | clsx + tailwind-merge | 2/3 | 类名合并 |
| Markdown 渲染 | marked | 18 | 前端可选 Markdown 渲染 |
| Tauri API | @tauri-apps/api + 5 plugins | 2.11 | IPC/对话框/剪贴板/通知/更新器 |

### IPC 流式方案

AI 摘要和翻译使用 **Tauri v2 Channel API** 实现流式输出：每个任务创建独立 `Channel<String>`，Rust 端通过 SSE 字节流消费 LLM 响应，每收到一个 token delta 即通过专属 Channel 推送到前端，天然隔离多任务并发，无需全局事件广播或手动 taskId 过滤。

### 项目规模

| 指标 | 数值 |
|------|------|
| Rust 源文件 | 46 |
| Rust 代码行数 | ~4,850 |
| TypeScript/TSX 源文件 | 43 |
| 前端代码行数 | ~2,400 |
| 数据库表 | 17 |
| Tauri IPC 命令 | 33 |
| 发布二进制大小 | 49 MB |
| 安装包大小 | ~11 MB (EXE) / ~16 MB (MSI) |

---

## 三、架构设计

### 分层架构

```
┌─────────────────────────────────────┐
│  React 前端 (TypeScript + Tailwind) │
│  ┌─────────┐ ┌──────┐ ┌─────────┐  │
│  │ TanStack │ │Zustand│ │useState │  │
│  │  Query   │ │ Store │ │(local)  │  │
│  └────┬─────┘ └───┬──┘ └────┬────┘  │
│       │ 服务端缓存  │ 全局状态 │ UI状态 │
├───────┴────────────┴─────────┴───────┤
│  Tauri v2 IPC 层                     │
│  ┌────────────────────────────────┐  │
│  │   invoke() / Channel<String>   │  │
│  └───────────────┬────────────────┘  │
├──────────────────┼───────────────────┤
│  Rust 后端 (src-tauri/src/)          │
│  ┌────────────────────────────────┐  │
│  │ commands/  ── 33 个命令处理器   │  │
│  ├────────────────────────────────┤  │
│  │ feed/  ── 订阅源引擎           │  │
│  │ reader/ ── 阅读器管道           │  │
│  │ agent/  ── AI 智能体            │  │
│  │ digest/ ── 文摘系统             │  │
│  ├────────────────────────────────┤  │
│  │ db/     ── 持久化层 (SQLite)    │  │
│  │ utils/  ── 路径工具             │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘
```

### 数据库 Schema (17 表)

**核心表**: `feed`, `entry`, `content`, `content_html_cache`, `entry_note`  
**标签系统**: `tag`, `tag_alias`, `entry_tag`  
**AI 配置**: `agent_provider_profile`, `agent_model_profile`, `agent_profile`  
**AI 运行时**: `agent_task_run`, `llm_usage_event`  
**翻译系统**: `translation_result`, `translation_segment`  
**摘要系统**: `summary_result`  
**系统**: `settings`

SQLite WAL 模式，`PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000`。连接池大小 8，前台读取与后台写入互不阻塞。

### 阅读器管道

```
源 HTML (RSS description 或 文章 URL 抓取)
  → decruft::extract()     → cleaned_html (清理后 HTML + 标题 + 作者)
  → htmd::convert()         → markdown (规范 Markdown)
  → comrak::markdown_to_html() → reader_html (带主题 CSS 的完整页面)
  → content_html_cache      → 按 (theme_id, entry_id) 缓存
```

**分层重建策略** (含版本号缓存失效):
1. 有 Markdown + 版本匹配 → 直接渲染
2. 有 cleaned_html + 版本匹配 → 转换 Markdown 后渲染
3. 有源 HTML → 运行完整管道
4. 无缓存 → 抓取文章 URL → 运行完整管道 (失败则使用 RSS summary 降级)

**降级策略** (新增): 当 `decruft` 无法提取有效内容且 RSS description 仅含元数据时，生成包含条目标题、作者、日期、可提取文本片段和原文链接的个性化降级页面，确保不同条目不会显示相同的空内容。

---

## 四、功能清单

### Feed 管理

| 功能 | 状态 | 说明 |
|------|------|------|
| RSS 0.9/1.0/2.0 解析 | done | 基于 feed-rs |
| Atom Feed 解析 | done | 基于 feed-rs |
| JSON Feed 解析 | done | 基于 feed-rs |
| Feed 添加/删除 | done | 验证 Feed URL 有效性后添加 |
| Feed 同步 (单条) | done | HTTP GET → 解析 → 条目 upsert |
| Feed 批量同步 | done | 每组 3 条限速，防止被封 |
| OPML 1.0/1.1/2.0 导入 | done | 支持嵌套文件夹递归解析 |
| OPML 导出 | done | 手动生成 OPML 2.0 XML |
| Feed 未读计数 | done | SQL LEFT JOIN 实时统计 |
| Feed 标题自动获取 | done | 首次添加时从 Feed XML 提取 |
| Feed 元数据更新 | done | 每次同步时更新标题/站点 URL |

### 条目管理

| 功能 | 状态 | 说明 |
|------|------|------|
| 条目列表 (按 Feed 筛选) | done | 点击侧边栏 Feed 过滤 |
| 条目列表 (按标签筛选) | done | SQL IN 子查询 |
| 条目列表 (关键词搜索) | done | LIKE 模糊匹配标题/摘要 |
| 分页 (游标模式) | done | 基于 `published_at DESC, id DESC` + 游标 ID |
| 标为已读/未读 | done | 批量 + 单条 |
| 星标 | done | 布尔标记 |
| 软删除 | done | `is_deleted = 1` |
| 全部标为已读 (按 Feed) | done | SQL UPDATE |
| 条目去重 | done | 按 `(feed_id, guid)` 或 `(feed_id, url)` |
| 条目详情 (含 feed 标题 + 标签) | done | JOIN + 子查询 |

### 阅读器模式

| 功能 | 状态 | 说明 |
|------|------|------|
| 文章内容提取 (Rust) | done | decruft 提取正文 |
| HTML → Markdown 转换 | done | htmd (GFM 表格支持) |
| Markdown → 阅读器 HTML | done | comrak (GFM: 表格/删除线/任务列表/自动链接) |
| 4 种阅读主题 | done | classic/paper × light/dark |
| 主题实时切换 | done | 切换后重新渲染含主题 CSS |
| 渲染缓存 | done | `content_html_cache` 按 (theme_id, entry_id) |
| 分层重建 | done | 4 步策略 + 版本号缓存失效 |
| Cloudflare 绕过 | done | Chrome 131 User-Agent + 质询页面检测 |
| 降级页面 | done | 提取失败时显示个性化元数据页 |
| 前端 Readability.js fallback | 代码就绪 | `use_reader_fallback.ts`，后端暂未调用 |

### AI 智能体

| 功能 | 状态 | 说明 |
|------|------|------|
| Provider 管理 (CRUD) | done | 支持任意 OpenAI 兼容 API |
| API Key 安全存储 | done | XOR 混淆 + 本地文件 (后续升级 Stronghold) |
| 连接测试 | done | 发送 "respond with just 'ok'" 测试请求 |
| Model 配置 | partial | 后端 store 已实现，前端 UI 待补充 |
| Agent Profile (每任务类型) | done | `agent_profile` 表支持主/备模型 |
| **AI 摘要 (流式)** | done | 3 级详细程度 (brief/medium/detailed)，SSE 流式 |
| **AI 翻译 (分段)** | done | 按段落分割，逐段翻译 + 上文上下文 |
| **AI 标签建议** | done | 调用 LLM 返回逗号分隔标签名 |
| 提示词模板引擎 | done | YAML + `{{#cond}}...{{/cond}}` 条件渲染 |
| 内置提示词 (4 个) | done | summary/translation/translation.hy-mt/tagging |
| 流式 IPC (Channel API) | done | 每任务独立 Channel，无事件窜频 |
| 任务队列运行时 | done | 1 active + 1 waiting 槽位，仅最新任务进入等待 |
| LLM Token 用量统计 | done | 7d/30d/90d 窗口查询 |

### 标签系统

| 功能 | 状态 | 说明 |
|------|------|------|
| 标签 CRUD | done | 创建/重命名/删除 |
| 标签合并 | done | 关联转移 + 源标签删除 |
| 标签库搜索 | done | LIKE 模糊搜索 |
| 标签使用计数 | done | SQL COUNT 实时更新 |
| 临时标签 | done | AI 建议的标签标记为 `is_provisional` |
| 标签输入自动补全 | done | 从标签库下拉 + 点击建议添加 |
| 批量打标签 (按时间范围) | done | 事务内批量 INSERT OR IGNORE |
| 标签别名去重 | done | `normalized_name` UNIQUE 约束 |

### 笔记

| 功能 | 状态 | 说明 |
|------|------|------|
| 笔记存储 (Markdown) | done | `entry_note` 表 |
| 笔记编辑 UI | done | Markdown 文本编辑器 |
| 笔记保存/删除 | partial | 前端 UI 就绪，IPC 命令待注册 |

### 文摘导出

| 功能 | 状态 | 说明 |
|------|------|------|
| 单篇导出 (Markdown) | done | Handlebars 模板 |
| 单篇导出 (HTML) | done | Handlebars 模板 |
| 多篇导出 (Markdown) | done | Handlebars 模板 |
| 自定义模板 | done | 支持从磁盘加载 `.hbs` 文件 |
| 导出到文件 | done | Tauri `save()` 对话框 → 写文件 |

### 用户体验

| 功能 | 状态 | 说明 |
|------|------|------|
| 三栏布局 | done | 侧边栏 / 条目列表 / 阅读器 |
| 暗色模式 | done | light/dark/auto (跟随系统 + localStorage) |
| 键盘快捷键 (12 个) | done | j/k 导航, m 标已读, s 星标, r/t/n 面板切换, Ctrl+N 添加, Shift+R 同步, Shift+A 全部已读 |
| 毛玻璃效果 | done | 对话框/弹窗 backdrop-blur, Linux fallback 到半透明背景 |
| 无障碍 | done | focus-visible 轮廓, prefers-reduced-motion |
| 空状态/加载态/错误态 | done | 共用 `EmptyState`/`LoadingState`/`ErrorState` 组件 |
| 安装程序 | done | Windows NSIS (.exe) + WiX (.msi) |
| 平台最低版本 | 已配置 | macOS ≥12.0, Linux 需 webkit2gtk-4.1 + gtk-3 |

### 待完成 (后续版本)

| 功能 | 状态 |
|------|------|
| 前端 Digest/Usage/Settings 组件目录 | 目录缺失，后端 IPC 已就绪 |
| 国际化 (en/zh-Hans) | JSON 文件为空 |
| 状态栏动态数据 | 目前显示静态 "Ready" |
| Stronghold API Key 加密 | 目前使用 XOR 混淆 |
| Readability.js fallback 自动调用 | 代码就绪但管道未触发 |
| 翻译并发执行 | 并发变量已声明但顺序执行 |
| 翻译缓存失效 (sourceContentHash) | 函数存在但未接入 |

---

## 五、构建与交付

### 构建命令

```bash
# Rust 后端
cargo build --manifest-path src-tauri/Cargo.toml --release

# React 前端
npm run build

# Tauri 打包 (含安装程序)
npx tauri build
```

### 构建验证

| 检查项 | 状态 |
|--------|------|
| `cargo check` | 通过 |
| `cargo build --release` | 通过 (生成 49MB mercury.exe) |
| `npm run build` | 通过 (生成 ~287KB JS + ~31KB CSS) |
| `npx tauri build` | 通过 (生成 11MB setup.exe + 16MB MSI) |

### 交付产物

```
src-tauri/target/release/
├── mercury.exe                                  (49 MB)
└── bundle/
    ├── nsis/Mercury_0.1.0_x64-setup.exe         (11 MB)
    └── msi/Mercury_0.1.0_x64_en-US.msi          (16 MB)
```

---

