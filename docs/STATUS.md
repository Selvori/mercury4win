# 当前项目状态

最后更新：2026-10-24。

## 当前结论

移植审查的 18 个串行修复阶段均已完成实现与隔离验证，相关改动及后续翻译 / 字体滚轮修复已纳入此前本地 Git 提交；中文化、设置内语言选择和单行导航由本轮本地提交记录。实际产品验收仍待完成。当前完整 Rust、前端构建、所有已登记隔离 DOM 回归与 Rust 格式基线通过。问题状态只维护在 [ISSUES.md](ISSUES.md)：M4W-018 的测试语义 / 格式与 M4W-019 的详情契约已关闭，其余 18 项（含新增 M4W-020）保持待验证。

- 当前代码包含此前 `f732ee7` 的全部移植修复、翻译 / 字体滚轮修复、开发启动脚本和验证文件，以及中文化、语言选择持久化、设置导航与对应回归记录；0.4.0 基线 `0c8258d` 已于 2026-10-24 推送至 GitHub main，未发布或创建标签。schema 为 5，Reader Readability / Markdown / render 为 3 / 2 / 5。
- 上游参考 Mercury：`5b2e58ebccc772263b0fc70360f667c3abf03eea` / v1.4.32，仅作语义参考。原 [2026-10-12 审查](audits/mercury-porting-2026-10-12.md) 的基线和历史失败保留，本轮结果不改写历史。
- 逐项修复、限制和证据见 [问题台账](ISSUES.md)、[工作日志](WORKLOG.md)、[串行顺序](audits/porting-fix-sequence-2026-10-12.md) 及各 tests README。固定 RSS 导入样例仍为 [hn-popular10+1.opml](../tests/fixtures/opml/hn-popular10+1.opml)，验收约定见 [TESTING.md](TESTING.md)。

## 当前最终验证基线

2026-10-13，Windows / PowerShell 7.6.5、Rust 1.97.0 GNU；下表是提交前串行修复工作区的实际结果，完整命令、失败分类和格式范围见 [最终验证报告](audits/final-validation-2026-10-13.md)，可复现入口见 [tests/final_validation](../tests/final_validation/README.md)。

| 检查 | 实际结果 |
| --- | --- |
| 完整 cargo test --locked | 134 通过、0 失败、0 忽略，默认并行；binary / doc 0 项入口也完成 |
| cargo check --locked | 通过，0 Rust warning |
| 全仓 Rust cargo fmt --check | 通过；format 前后 71 文件快照核对，22 文件纯 formatter 改动 |
| npm.cmd run lint / build | 通过；build 保留已有 >500kB chunk 提示 |
| 15 套生产组件 / wrapper 隔离 DOM | 125 项 / 组通过，安全回归使用 Node 24.19.0 + jsdom 30.1.2；其余 Happy DOM 20.14.6 |
| 双语资源、插值与静态键引用 | 351 英文 / 351 中文键一致 |
| diff / UTF-8 / 文档链接 | 通过 |
| native 桌面、真实 AI、在线站点和三平台 | 未验证 |

Rust 使用当前 GNU 测试产物的 Common Controls 6 sidecar 环境；DOM 的 DTO / HTML 为真实 Rust 临时 fixture，但这些均不等于 native IPC 或桌面排版验收。没有读写真实用户数据库 / 凭据，也未声称运行不存在的 npm test。

2026-10-13 最新翻译变更：按钮 / `T` 直接启动或恢复翻译，不显示侧栏、控制条、进度或状态；完整缓存直接展示，失败不自动循环重试。分段版本升级为 `reader-html-block-v3`，覆盖标题、引用正文、表格文字及图注 / 定义文本，代码块保持原文；旧翻译记录保留但不命中新版本。此次 cargo check、翻译 / 结果存储 14 项 Rust、5 套 64 组隔离 DOM、lint / build 和定向格式检查通过，详见 [最新翻译回归](../tests/translation/README.md#2026-10-13-静默直接翻译与漏译范围)。上表保留此前完整验证基线，此次未重跑完整套件，native 桌面及真实 Provider 仍待验证。

2026-10-13 最新字体滚轮修复：相同文章更新字体 / 字号 / 主题时保留 iframe，避免在设置弹窗的 inert 背景中重新创建 frame。Edge 155 生产组件真实滚轮复现后，修复后的 8 组浏览器与 61 组既有 DOM、lint / build / diff 通过；[证据与命令](../tests/reader_scroll/README.md)。完整 Tauri / WebView2 和触控板仍待验收，M4W-020 保持待验证；未改 Rust 或重跑全仓套件。

## 最新中文化

2026-10-13 最新中文化（基于 f732ee7，纳入本次本地提交）：无有效偏好时默认简体中文，在设置窗口顶部切换中英并保存，主界面无独立语言控件；HTML 语言、日期和数字同步，中文用量文案保留小写 token。补齐设置 / 文件窗口中文，文摘标题随界面语言预览及导出，旧 IPC 省略 locale 时保持英文。资源现为 359 / 359 键；初次中文化的 7 项文摘 Rust、37 组 DOM / 编译 / 定向格式基线保留。此次入口及术语调整的 6 组真实 Edge、20 组可访问性 / 标签 DOM、lint / build / 资源 / diff 通过，未改 Rust 或重跑全套。[改动与验证](../tests/localization/README.md)。已有 AI 输出语言、数据库和凭据保持；完整 Tauri / WebView2 退出重启及系统窗口仍待验证，M4W-015 保持待验证。

## 最新设置导航

2026-10-13 设置导航布局：修正英文 Tag Library 换行，减小导航按钮横向留白并禁止整行折行；常规设置窗口完整显示所有入口，窄窗口横向滚动。中英文 1200 / 800 / 600px 布局及最后入口的 Tab / 点击验证通过，最新 7 组 Edge、lint / build / diff 通过。[证据](../tests/localization/README.md)。本次只改前端，未重跑 Rust / 全套 DOM，native 桌面仍待验收。

## 最新云端构建

2026-10-24：0.4.0 基线 `0c8258d` 已推送，手动触发的 [三平台构建](https://github.com/Selvori/mercury4win/actions/runs/40003947795) 最终成功。Windows MSI / NSIS、Linux DEB、macOS ARM64 DMG 均生成，三个 artifact 存在且非空；首轮 Linux 镜像下载缓慢，取消后仅重跑未完成作业，第二次通过，没有修改工作流或源码。本次本地 lint / build、cargo check --locked --offline 也通过。main 推送本身不自动触发此工作流，手动运行的 release 作业按预期跳过；未创建标签或发布，安装 / 升级与桌面运行尚未验证。

## 下一步

2026-10-13 版本统一：README、前端 APP_VERSION、npm / Cargo 清单与锁文件、Tauri 配置统一为 0.4.0，高于现有最高标签 v0.3.1；本地 main 合入已保存的远端提交 5563cad，README 冲突以新版本解决。npm lint / build、cargo check --locked --offline、版本一致性 / UTF-8 / diff 检查通过，依赖版本无变化。当时仅本地提交；最新推送与构建状态见上节，既有产品验收状态保持。

以隔离应用数据、固定 F-001 和合成凭据 / 本地 Provider 完成 native Tauri / WebView2 验收：切换文章、面板关闭 / 重开、取消与部分失败、完整退出重启、无 AI 配置、导入 / 导出对话框、键盘 / 焦点 / iframe、实际主题 / 字体 / 图片。再核对真实 Provider 和在线站点兼容，以及 Windows / Linux / macOS 打包与运行；按各问题最低关闭条件附证据后更新台账。当前不再有下一实现编号。
