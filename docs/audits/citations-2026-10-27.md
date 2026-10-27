# 简报冻结段落引用修复

2026-10-27，Asia/Shanghai。基线 `b93753a` 加此前保留的自动简报 / JSON 容器修复及本次修改；Windows / PowerShell 7.6.7 / Rust 1.97.0 GNU / 完整 Tauri debug 与 WebView2。保留已有工作区改动，未提交、推送、发布、升级依赖或修改用户数据库 / 凭据。

## 问题与修复

用户报告 `Quote not found in article 585`。没有取得用户实际的模型引文或读取该文章；无法判断这一次是空白转写、引文改写、来源混淆还是编造。旧实现要求模型输出的 quote 在本批冻结原文中逐字且唯一出现，单个失败会阻止整个简报保存。

自动简报改用 `topic-information-v2`：程序把本次选中文章的完整可用文本分为稳定编号的段落，并在请求中提供 `entry_id / passage_id / text`。模型引用编号，程序从冻结正文恢复原文和 UTF-8 字节位置。段落只是引用寻址单元，不宣称原文章存在相同排版段落；全体片段拼接等于原文，不截断或改写正文。每段通常最多 800 字符，短尾段可合并，仍低于既有 4,000 字节引用上限。

仅当前选中文章及有效段落编号可引用；非法文章 / 段落、缺少引用、错误类型和超限仍拒绝。若同时提供 quote，必须在指定段内准确对应，不能用有效段落编号掩盖编造的引文。引用存在仍不能证明模型表述正确。

旧 quote 格式及历史继续兼容。精确匹配失败时，仅允许唯一的空白规范化匹配（换行、空格、制表符、NBSP 等），并保存原正文切片，不保存模型改写。大小写、标点、词语、翻译、插入省略号、歧义及过长跨度不自动放宽。重复片段可由明确段落编号区分，不能由模糊搜索任意选取位置。

保存的 Citation 仍是原 `entry_id / quote / content_version / start / end`，无需 Schema / TS 类型迁移；旧简报、原文高亮和 Markdown 导出沿用原契约。选文缓存身份保持，因引用失败后的重试可复用筛选，不增加自动纠错 AI 请求。请求及结果记录新的总结 Prompt 版本。

实现：[引用寻址及校验](../../src-tauri/src/agent/topic.rs)、[自动简报请求](../../src-tauri/src/agent/topic_auto.rs)、[引用回归](../../src-tauri/src/db/topic_store/tests.rs)、[生产 HTTP 回归](../../src-tauri/src/db/topic_auto_store/tests.rs)。受控原生 Provider 同步使用段落编号。

## 自动检查

- `./tests/reader_pipeline/run_rust.ps1 db::topic_`：32 个入口通过，含 3 个本轮未启用的可选 fixture 导出入口。新增 5 项覆盖中文 / emoji / NBSP / 换行的准确 UTF-8 回映、重写 / 重复 / 重叠匹配拒绝、完整段落拆分与大小边界、重复文本按编号定位、非法编号后复用筛选重试。
- 生产 loopback HTTP 测试实际收到编号段落而非要求模型重抄正文；成功后所有引用等于冻结正文中的对应字节切片。无效段落失败不保存简报或推进检查位置；重试只多发一次总结请求。
- `cargo check --locked --offline --manifest-path src-tauri/Cargo.toml`、`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 通过。
- 独立 identifier 完整 Tauri debug 构建通过；结束后默认 `com.mercury.reader` 配置的 debug 构建也通过，目标产物已恢复为正常应用配置。没有前端源码 / 迁移变更，未重跑前端全套或完整 Rust library；此前 4 项 socket 取消超时失败不算本次通过。

## Windows 桌面实际验证

用户明确恢复 Computer Use 授权后，使用该技能的公开 sky API 控制独立 `com.mercury4win.citations20261027` 应用。7 篇固定合成文章，只有 loopback 受控 Provider；未使用真实模型、日常凭据或日常数据库。

1. 原生点击主题追踪，输入“AI 编程工具引用回归”，选择“创建并生成简报”。受控总结故意返回编造 quote，界面实际显示 `Quote not found in article 2`；数据库为 0 简报 / 0 已检查，run 为 failed。
2. 将本地 Provider 改为返回请求中真实的 passage_id，原生点击重试，成功保存 #1，冻结 3 篇，候选变为 0，生成更新按钮禁用。总请求顺序为 **selection → summary → summary**，重试没有再次筛选。
3. 原生打开中文文章 #2 的引用，弹窗精确显示并高亮冻结原文，UTF-8 字节为 0–75；Escape 关闭后焦点回到原引用按钮。
4. 原生标记版本已读，正常退出并重新启动应用，从卡片阅读同一简报，再次打开引用，原文、版本与位置保持，已读位置仍为 1。重启没有新增 HTTP 请求。
5. 只读检查该合成库：3 个保存引用均等于冻结正文的 UTF-8 切片；1 个失败 run / 1 个成功 run / 1 个简报，7 条已检查记录，外键错误 0。三条 HTTP 用量均为受控成功响应；其中第一个总结虽 HTTP 成功，业务引用校验失败，两者没有混记。

[结构化合成证据](citations-2026-10-27-evidence.json)。原始请求、UI 文本、构建日志与合成库保留在忽略的 `tmp/citations-native-20261027/` 及命名隔离应用目录中。测试应用已正常退出，受控 Provider 已停止。

## 未验证

这验证了编号协议、原生引用显示及失败恢复，不证明真实模型会正确选择支持句或正确理解文章。用户文章 585 的原始问题与真实 Provider 重试尚未核验，M4W-025 保持待验证。其他平台、大型正文集合、完整更新 / 原生导出路径不由本轮替代验收；旧历史报告和失败保留。
