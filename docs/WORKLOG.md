# 工作日志

按实质任务追加，使用日期和主题区分同一天的任务。只记录实际动作与证据；当前问题状态统一查 ISSUES，当前概况查 STATUS。

## 2026-10-12 · 更新参考仓库与重要功能审查

- 目的：更新 Mercury，检查重要功能移植是否正确。
- 基线：当前项目 HEAD 395f997，包含开始时已有的未提交改动；参考项目从 3cbcbc1 快进到 5b2e58e / v1.4.32。
- 产出：`docs/audits/mercury-porting-2026-10-12.md` 和隔离探针 `porting_audit_probe.rs`。业务实现未修复，临时测试模块接入已移除。
- 验证：前端 lint / build 通过，cargo check 通过并有 17 条警告；原有测试 32 通过 / 4 失败，隔离探针 3 通过 / 10 失败；全仓格式检查失败，探针格式通过。
- 限制：未做完整桌面、真实 AI、三平台验收；探针使用内存 SQLite / 本地 HTTP，无真实用户数据。
- 跟进：问题现登记为 M4W-001…M4W-018，详情与复现证据保留在审查报告。

## 2026-10-12 · 登记 RSS 导入样例与建立项目记录机制

- 目的：保存用户指定 hn-popular10+1.opml 为后续导入样例，建立持续的信息记录机制。
- 变更：原样复制到 tests/fixtures/opml，以 .gitattributes 保持样例字节；建立 docs 导航、STATUS / ISSUES / DECISIONS / TESTING / WORKLOG；将维护规则接入 AGENTS.md 和根 README。
- 决策：D-001 仓库记录分工、D-002 固定 OPML 基线；D-003 登记既有兼容约定。登记审查问题，不将其标记为已修复。
- 验证：原文件 / 副本 SHA-256 相同；严格 UTF-8、XML 结构、11 个唯一 HTTPS xmlUrl、10 个 htmlUrl / 1 个缺省值通过；新增文档链接与 Git diff 检查通过。
- 未运行：实际 RSS 导入 / 网络同步 / Tauri 对话框，未重复执行前后端构建（本次仅文档和固定样例变更）。
- 遗留：下次涉及 Feed / OPML 任务按 TESTING.md 执行隔离验收并记录逐源结果；业务缺陷仍按 ISSUES 处理。

## 2026-10-12 · 整理并提交修复前基线

- 用户要求先处理未提交改动，因此暂停 M4W-001 子任务；业务修复尚未写入代码。
- 基线：395f997 加既有项目名称统一、固定历史凭据混淆字节与兼容测试、审查及记录文件。本轮新增串行修复顺序文档。
- 将源代码/包名称与文档/审查/固定样例分开提交，保留全部已有修改；仅本地提交，不推送、不发布。
- Windows PowerShell 7.6.5 实测：npm.cmd run lint、npm.cmd run build、cargo check --locked --manifest-path src-tauri/Cargo.toml 通过；Rust 保留 17 条已有警告。
- cargo test --locked --manifest-path src-tauri/Cargo.toml credential_store：2 通过、0 失败；rustfmt --edition 2021 --check src-tauri/src/agent/credential_store.rs 与 git diff --check 通过。
- 未重跑全套 Rust 测试及桌面/跨平台验收；此前报告中的失败仍有效，不将本次提交作为业务缺陷已修复的证据。