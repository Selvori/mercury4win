# Mercury 重要功能移植审查

审查日期：2026-10-12。结论：**当前项目具备基础阅读器框架，但不能认定上游重要功能已正确、完整移植。** AI 持久化、任务生命周期、笔记切换隔离、正文读取、分页和文摘导出存在影响实际使用的问题。

## 基线与范围

- 参考仓库：`G:\sth_interesting\mercury`，远程 `https://github.com/neolee/mercury.git`。
- 已执行 `git pull --ff-only`：`3cbcbc1cd5702d7ce822ac2d9c5faf66ceb95a8d` → `5b2e58ebccc772263b0fc70360f667c3abf03eea`（v1.4.32）。此次更新仅修改 Xcode 项目版本信息与 appcast；更新后工作区干净。
- 被审查仓库：`G:\sth_interesting\mercury4win`，HEAD 为 `395f99757d1fd7977ab028f4e39b6a9e10488be5`。审查对象包含开始时已有的全部未提交改动，结论不能仅归因于该提交或本次上游更新。
- 先阅读上游 README、功能文档和工程约定，再对照 Swift 实现、测试与本项目 Rust / React / IPC 调用链。历史设计稿仅作线索，以现有实现为准。
- 本次交付是检查报告与复现探针；未修复业务实现，未提交、发布、访问实际用户数据库或调用外部 AI。
- “已复现”表示现有测试或隔离探针确认；“静态确认”表示代码调用链已确认，桌面操作尚未执行。不能把编译成功等同于用户流程通过。

## 功能对照

| 重要功能 | 当前状态 | 依据与限制 |
| --- | --- | --- |
| RSS / Atom / JSON Feed | 基础解析可用 | 三种最小样例解析通过；添加、同步、去重有后端与 UI 路径，尚未全面验证真实站点 |
| OPML 导入 / 导出 | 基础链路存在 | 有文件对话框、IPC、嵌套 outline 递归解析、重复源跳过与进度消息；未执行桌面对话框及网络导入验收 |
| 已读 / 收藏 / 删除 | 部分实现 | 有持久化操作；查询过滤、分页、快捷键与部分缓存失效不完整 |
| 正文提取 / Markdown / 阅读主题 | 部分实现、有缺陷 | 基础转换测试通过；摘要误当全文、相对 URL、失败提示和版本重建问题已复现 |
| 笔记 | 数据存储可用，UI 隔离有缺陷 | 隔离数据库 CRUD 通过；文章切换草稿不重置，可能写错文章 |
| AI 摘要 | 部分实现 | 有流式请求；结果不落库、无缓存恢复、自动摘要未接入，任务生命周期不完整 |
| AI 翻译 | 部分实现，核心语义缺失 | 并发请求与 HY-MT 模板入口存在；无持久化、断点恢复、稳定分段和内联双语布局，失败状态错误 |
| Provider / Model / 自定义 Prompt | 部分实现 | 设置、测试连接和主模型选择存在；自定义模板和备用模型未进入实际执行路径 |
| 标签与标签库 | 部分实现 | 单篇手动标签及 AI 建议存在；合并失败，别名、规范化、多选 All、批量 AI 审核流程未完成 |
| 文摘分享 / 单篇及多篇导出 | 尚未形成可用流程 | 无 UI 调用入口，模板实际渲染报错，缺少摘要、笔记和原始 URL 的完整输出契约 |
| LLM 用量统计 | 部分实现 | 有总量展示；Provider 一律记作 default，失败与请求粒度统计不准确，无多维比较 |
| 中英文、主题、字体 | 基础接入 | i18next、暗色、字体配置存在；部分英文硬编码、主题选择和系统主题同步仍有缺口，未做跨平台视觉验收 |

## 优先修复的问题

### P1-01 笔记草稿会跨文章保留，保存时可能覆盖另一篇笔记

- 位置：`src/features/reader/components/reader_note.tsx:19`、`:29`；`reader_pane.tsx:215`。
- 触发：在文章 A 编辑笔记但不保存，直接从同一文章列表选择 B，再点击保存。
- 原因：父组件未按文章 ID 设置 key；子组件的 `draft`、`is_editing` 没有在 ID 变化时重置，保存闭包却使用新 `entry_id`。B 的已有笔记可能被 A 的草稿覆盖。摘要、翻译和标签建议的异步回调也缺少文章 / 任务身份校验。
- 上游依据：`Mercury/Mercury/Digest/Shared/DigestNoteController.swift` 的 reset / load / snapshot 和 entryId 校验；翻译的 `AgentDisplayOwnershipPolicy`。
- 验证等级：静态确认；数据库 CRUD 探针通过，问题在界面状态归属。桌面复现尚未执行。
- 修复方向：草稿、保存请求和异步回调绑定文章身份；切换时处理未保存草稿，旧结果不得投影到新文章。仅重置显示不等于取消后台任务。

### P1-02 摘要和翻译成功后不保存，关闭面板或重启后丢失

- 位置：`src-tauri/src/agent/summary.rs:90`、`translation.rs:171`；`src-tauri/src/db/migrations.rs:165`、`:187`。
- 数据库虽然创建 `summary_result`、`translation_result`、`translation_segment`、`agent_task_run`，生产代码没有对应结果写入 / 查询链路；哈希函数也没有调用方。
- 摘要与翻译只发送 Channel 数据，结果保留在 React state。重新打开面板不能从数据库恢复，必须重复请求。
- 上游依据：`Agent/Summary/AppModel+SummaryStorage.swift:38`；`Agent/Translation/AppModel+TranslationStorage.swift:307`、`:383`，包含成功落库、分段 checkpoint、来源哈希和状态校验。
- 已复现：本地模拟 API 返回有效摘要 / 翻译，任务返回成功，但两种结果表行数均为 0。

### P1-03 AI 队列和取消只修改状态，未控制实际任务

- 位置：`src-tauri/src/commands/agent_cmd.rs:214`、`:243`、`:278`、`:307`。
- 每次 submit 后立即执行网络请求，没有根据“运行 / 等待”决定是否启动。正常成功和失败路径不调用 runtime.complete；路由失败也留下 running 状态。
- cancel_agent_task 仅调用 complete，没有取消令牌或任务句柄，无法中止 HTTP / 分段执行；若存在等待项，还会在状态上将其提升为 running。
- 上游依据：`Agent/Runtime/AgentRuntimeEngine.swift:18` 的启动决策、owner / activeToken 及有界等待队列；上游约定每种任务 1 active + 1 waiting，离开等待文章清除等待意图，正在运行的任务不自动取消。
- 验证等级：静态确认。现有 6 项 runtime 单元测试只验证独立状态机，没有覆盖命令与实际执行器是否遵守状态机。

### P1-04 翻译全部失败仍报告完成；分段与展示不符合上游契约

- 位置：`src-tauri/src/agent/translation.rs:42`、`:173`、`:201`；`src/features/reader/components/reader_translation.tsx:23`、`:41`。
- 并发模式忽略 Err，继续记 success 用量并发送 done，最终返回 Ok。UI 收到单段 error 即解除 loading，但后台其他请求仍可能运行。
- 已复现：本地 API 返回 HTTP 500，所有段失败，函数仍返回 `Ok(())`。
- 当前按 Markdown 双换行分段，而上游按 HTML p / ul / ol 分段，排除列表内重复 p，可添加标题作者段。当前会把标题、代码等当普通段；无稳定 sourceSegmentId / 段重试 / checkpoint。
- UI 仅显示右侧译文列表，无逐段原译文映射；切换文章不清空 segments、不回到原文。最大并发数未按上游 1…5 限制。
- 上游依据：`Agent/Translation/TranslationContracts.swift`、`TranslationSegmentExtractor.swift`、`AppModel+TranslationExecution+PerSegment.swift` 及 `MercuryTest/TranslationStoragePersistenceTests.swift`。

### P1-05 Reader 把非空 RSS 摘要一律视为全文

- 位置：`src-tauri/src/feed/parser.rs:52`；`src-tauri/src/reader/pipeline.rs:60`。
- parser 将 content body 与 summary 合并保存，reader 对任何非空 summary 直接渲染，不再访问文章 URL。普通只提供导读的 Feed 因而只显示摘要，摘要 / 翻译也使用不完整正文。
- 已复现：RSS 提供短 teaser、本地文章服务提供完整长文，最终 HTML 不包含完整文章标记。
- 上游依据：`Reader/ReaderBuildPipeline.swift:172` 的 source document fetch 和完整处理路径。
- 修复方向：保留 Feed 全文与摘要的来源区别，允许全文 Feed 直接阅读，同时为 teaser 源抓取正文，避免退回“一律抓取”造成全文 Feed 回归。

### P1-06 文章列表最多展示 100 条；后端游标也会漏文章

- 位置：`src/features/entry/components/entry_list.tsx:24`、`:30`；`src-tauri/src/db/entry_store.rs:67`、`:89`、`:120`。
- UI 固定 limit=100，无 next_cursor 消费、下一页按钮或无限滚动。显示总数超过 100 时，后续文章仍不可访问。
- SQL 按 published_at DESC, id DESC 排序，却只用 id < cursor 翻页；发布日期与插入顺序不同会漏项或重复，总数也包含游标过滤。
- 已复现：ID=1 是较新文章，ID=2 是较旧文章，第一页 limit=1 返回 ID=1；第二页为空，遗漏 ID=2。
- 上游依据：`Core/Database/EntryQueryBuilder.swift:104` 使用与排序对应的复合游标。

### P1-07 文摘导出模板无法渲染，且用户流程未接通

- 位置：`src-tauri/src/digest/templates.rs:43`；`src-tauri/resources/digest_templates/`；`src-tauri/src/commands/digest_cmd.rs`。
- 模板使用 Mustache 式 `{{#byline}}` / `{{#entries}}`，Rust Handlebars 按 helper 解释，报 `Helper not found byline` / `Helper not found entries`。
- 现有单篇、多篇 Markdown 模板测试均实际失败；single_text 同样含 byline 节语法。
- 搜索前端调用链：generate_digest / export_digest 只有 bindings，无界面调用；export_multi_digest 无对应前端封装。模板与数据仅围绕正文，未实现上游可选笔记 / 已保存摘要，模板还未输出原始 URL。single_text 实际为 HTML，不是上游纯文本分享语义。
- 上游依据：README “使用笔记与文摘分享/导出”、`docs/features/note-and-sharing.md`、`Digest/Shared/DigestComposition.swift` 及单篇 / 多篇导出 view model 测试。

### P2-08 Reader 相对 URL、错误提示和版本重建存在缺陷

- 位置：`src-tauri/src/reader/pipeline.rs:30`、`:120`、`:171`、`:218`。
- 提取时传入 None，document_base_url 始终 None，没有把相对图片和链接绑定到文章地址。探针确认 `/image.png` 未解析为站点绝对 URL，iframe srcDoc 也无文章 base。
- 失败路径把完整 HTML body 传给默认禁用 raw HTML 的 comrak Markdown 渲染器，提示正文被忽略。探针确认 “Full article content is not available.” 不出现在结果中；不能通过直接开放不可信 raw HTML 粗暴修复。
- 只要 markdown / cleaned_html 存在就复用，不检查存储版本；cleaned_html 分支也未保存重建的 markdown。探针确认 markdown_version=0 仍使用过期正文。
- build_reader_content 未调用已有 get_cached_reader_html，每次重渲染并写缓存。empty marker 也未被识别为终态，会再次尝试抓取。
- 上游依据：`Reader/ReaderDefaultPipeline.swift` 的版本规划与 baseURL 处理；`Reader/ReaderBuildPipeline.swift:81` 的缓存快照 / 分层重建；`MercuryTest/ReaderPipelineVersioningTests.swift`。

### P2-09 上传的自定义 Prompt 和备用模型没有实际生效

- 位置：`src-tauri/src/commands/agent_cmd.rs:126`、`:351`；`agent/summary.rs:39`、`translation.rs:54`、`tagging.rs:36`。
- 上传成功后写入 *.custom.yaml，但三个执行器均直接调用 load_builtin_template。load_custom_template 只用于设置展示，没有运行时优先加载。
- fallback_model_id 可保存，但 resolve_agent_model 不读取它；主模型调用失败时也不尝试备用模型。无兼容模型时会猜测 gpt-4o-mini；主模型选择还未校验对应 capability。
- 摘要界面固定初始 medium，未应用已保存 detail_level；上游的确认开启、1 秒防抖、串行自动摘要未实现。
- 验证等级：静态确认；上游依据 `docs/agent-prompts.md`、`docs/features/summary-agent.md` 与 `Agent/Shared/` 的 Prompt / route 实现。

### P2-10 标签合并必然失败，规范化、别名和批量 AI 流程不完整

- 位置：`src-tauri/src/db/tag_store.rs:52`、`:160`、`:218`；`src/lib/tauri_bindings.ts:388`；`src-tauri/src/commands/tag_cmd.rs:59`。
- merge_tags 删除关联 SQL 使用 ?2，但只绑定一个参数。已复现错误 `Wrong number of parameters passed to query. Got 1, needed 2`；事务回滚，无法合并。
- 规范化仅 trim + lowercase，不折叠 - / _ / . / 空白；已复现 Machine-Learning 与 machine learning 创建两个标签。tag_alias 只有表结构，无读写使用，合并也不保留别名。
- batch_tag 前端仅传 timeRange，后端还要求 tag_ids，IPC 参数不匹配；后端操作只是把固定标签批量赋给时间范围文章，并非上游 AI 分析、提议审核、确认应用流程。
- 当前标签库 UI 只有重命名 / 删除，无合并、别名、临时标签提升、清理未使用标签；无无模型本地建议 / 相关文章推荐。
- 上游依据：`Tags/TagNormalization.swift`、`Core/Database/TagLibraryStore.swift`、`Agent/Tagging/AppModel+TagBatchExecution.swift`、`MercuryTest/TagAliasBypassTests.swift`。

### P2-11 搜索、未读、组合标签、快捷键尚未形成完整用户路径

- EntryList 固定 unread_only=false、不传 search；无文章搜索 / 未读开关。AppShell 标签选择为单 ID，后端接收 tag_mode 但忽略，只执行 Any 语义，无法实现 All。
- mark_all_read_in_scope 已按 Feed / 标签跨页更新，**并非仅第一页标记**；但参数不包含搜索 / unread / tag mode，不能满足上游完整 query-scoped 契约。
- AppShell 的 m / s 回调是注释占位；j / k、刷新、面板切换等未传给 useKeyboardShortcuts。收藏按钮存在，但无收藏列表入口；已读按钮只能置 true，无 UI 还原未读路径。
- 删除文章仅失效 entries，未同时更新 feeds / tags 计数和清除已选 Reader；不要把数据已删除与 UI 已同步视为同一件事。
- 上游依据：AGENTS.md “Key Behavioral Contracts”、`Core/Database/EntryQueryBuilder.swift` 和 `Feed/Views/EntryListView.swift`。

### P2-12 用量统计与 Provider 生命周期偏离上游

- 位置：`src-tauri/src/agent/summary.rs:104`、`translation.rs:181`、`tagging.rs:113`；`src-tauri/src/db/usage_store.rs`、`agent_store.rs:54`。
- provider_name 固定 default；翻译把多段请求合并成一个 event，所以 API 请求数不等于实际请求数。失败 / 取消不完整记录，翻译甚至将全失败记为 success；标签重试只保留最后成功 usage。
- 流式请求没有 stream_options.include_usage；对于要求显式请求 usage 的提供商，缺失 usage 会被当作 0。
- 统计 UI 只有时间窗总量，没有 Provider / Model / Agent 的分组与比较。
- Provider / Model 使用硬删除而非 archive；被 agent_profile 引用的模型可能因外键限制无法删除。删除 Provider 先移除凭据，再删除数据库记录，后一步失败会使仍存在的配置丢失密钥。
- 上游依据：`docs/features/token-usage.md` 的逐请求记录、失败保留、软删除契约；`Agent/Shared/AgentExecutionShared.swift` 的 LLMUsageEventContext。

## 其他需要补齐的移植项

- 上游 Obsidian Publish 正文分支未移植：当前 pipeline_type 固定 default，resolved_intermediate_content 为 None；上游有 ReaderPipelineResolver / ReaderObsidianPipeline 实现。
- 后端列出 paper 主题，但 ReaderPane 硬编码 classic；自动系统主题变更只直接切换根 class，未更新供 iframe 使用的响应式状态，Reader 可能等下次渲染才跟随。
- Reader 失败信息、AI 标签空状态等仍有硬编码英文；部分按钮无可理解的可访问名称。
- 原文 iframe 未允许脚本执行是合理边界；笔记预览直接使用 marked + dangerouslySetInnerHTML，没有内容净化，粘贴来源不明 Markdown 中的 HTML 需进一步验证和处理。
- 原文与流式网络边界还需专项回归：SSE 按每个 bytes chunk 做 from_utf8_lossy，跨块中文可能损坏；错误 JSON 使用 format 拼接，包含引号的错误可能无法解析。这两项本次仅静态识别，未用网络分块探针验收。

## 实际验证结果

| 检查 | 结果 |
| --- | --- |
| npm.cmd run lint | 通过 |
| npm.cmd run build | 通过，TypeScript + Vite |
| cargo check --locked --manifest-path src-tauri/Cargo.toml | 通过，17 条已有警告 |
| cargo test --locked --manifest-path src-tauri/Cargo.toml（原有测试） | 32 通过，4 失败 |
| 隔离探针（--lib porting_audit_probe） | 3 通过，10 失败，失败即下列契约差异 |
| cargo fmt --manifest-path src-tauri/Cargo.toml -- --check | 未通过，现有 build.rs、模块及多处源码存在格式差异；未重排业务代码 |
| rustfmt --edition 2021 --check docs/audits/porting_audit_probe.rs | 通过 |

原有 4 项失败：

1. `test_load_builtin_tagging`：模板 version=2，测试仍断言 1，属于测试预期陈旧，不能据此认定 YAML 无法加载。
2. `test_single_markdown_template`：Helper not found byline，实际产品缺陷。
3. `test_multiple_markdown_template`：Helper not found entries，实际产品缺陷。
4. `test_render_with_theme`：断言完整 `<h1>Title</h1>` 与现有标题锚点输出不符；补充探针确认标题和正文仍存在，不等于标题渲染失败。

隔离探针明细：

| 探针 | 结果 |
| --- | --- |
| rss_atom_and_json_feed_parse | 通过 |
| note_storage_roundtrip_preserves_other_entries | 通过 |
| heading_failure_is_not_missing_content | 通过 |
| pagination_retains_older_entries_with_higher_ids | 失败：第二页漏 ID=2 |
| tag_merge_succeeds | 失败：SQL 参数数不符 |
| equivalent_tag_separators_reuse_canonical_tag | 失败：生成两个标签 ID |
| successful_translation_is_persisted | 失败：结果表为 0 行 |
| successful_summary_is_persisted | 失败：结果表为 0 行 |
| failed_translation_does_not_report_success | 失败：HTTP 500 后返回 Ok |
| outdated_markdown_is_rebuilt_from_cleaned_html | 失败：仍复用旧正文 |
| fallback_html_is_visible_to_reader | 失败：失败提示被忽略 |
| reader_resolves_relative_images_against_article_url | 失败：未应用文章 base URL |
| teaser_feed_does_not_replace_full_article | 失败：短摘要阻止抓取全文 |

探针使用单连接内存 SQLite 和 127.0.0.1 随机端口模拟 HTTP，无真实 API Key。初次尝试独立 integration-test exe 在当前环境报 STATUS_ENTRYPOINT_NOT_FOUND（0xc0000139）；改为临时 lib 单元测试模块后正常执行，以上结果来自后者，不能把初次加载失败计入功能失败。

探针保存在 `docs/audits/porting_audit_probe.rs`，没有永久挂入应用测试入口。需要复现时，在 `src-tauri/src/lib.rs` 临时加入以下声明，然后运行过滤测试；完成后只移除这三行，保留用户其他改动：

```rust
#[cfg(test)]
#[path = "../../docs/audits/porting_audit_probe.rs"]
mod porting_audit_probe;
```

```powershell
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib porting_audit_probe -- --nocapture
```

未验证部分：Windows 完整 Tauri 桌面交互、真实站点网络适配、真实 AI 提供商、macOS / Linux 打包、上游 Swift 构建。旧库版本升级只有现有 v1 新建 / 重复运行测试，不能据此承诺跨版本迁移或与上游数据库兼容。

## 建议修复顺序

1. 优先防止写错数据：笔记文章归属、旧 AI 回调隔离；补充对应交互回归。
2. 恢复核心阅读能力：Feed 全文 / 摘要来源区分、相对 URL、分页、失败提示、版本重建。
3. 完成 AI 任务闭环：调度与取消、持久化、成功 / 失败状态、断点恢复、自定义 Prompt、备用模型。
4. 完成上游主要产品能力：双语分段、文摘导出、标签库 / AI 批量审核、用量分组；同时修复现有失败测试。
5. 最后执行隔离桌面验收及三平台检查。验收应包含切换文章、关闭面板、重启、部分失败、取消和无 AI 配置场景。
