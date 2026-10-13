# M4W-017 SSE 与错误事件回归

2026-10-12，HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加保留的 M4W-001、016、005、008、006、011 /019 及本项未提交修改；Windows / PowerShell 7.6.5、Rust 1.97.0 GNU。仅用 127.0.0.1 HTTP、内存 SQLite 和真实 Tauri `Channel<String>` 的内存接收回调；不访问用户数据库、凭据或远程 AI。

## 行为约定

- SSE 先以字节保存未结束行和事件，遇到空行后严格 UTF-8 解码。支持 LF、CRLF、CR（含跨块 CRLF）、开头 BOM、注释 / metadata、`data:` 可选单空格与多行 data。参考 [WHATWG SSE 解析标准](https://html.spec.whatwg.org/multipage/server-sent-events.html#event-stream-interpretation)；这里只处理一次完成请求，没有 EventSource 的自动重连。
- 每个完整 data 事件作为一个 JSON completion 处理，合法角色 / 空 delta 和 usage-only 事件保留；完整 `[DONE]` 才返回成功，之后数据不再读取。`finish_reason` 或 HTTP EOF 均不代替 `[DONE]`。不带双换行的 `[DONE]` 属于未完成事件，返回错误。
- 非法 UTF-8、非法 JSON、缺少 choices / usage、API error、未结束事件 / 字节、正常 EOF 缺少终止、HTTP 传输断开或 Channel 发送失败均返回错误。保留已发出的部分内容，但摘要随后发送 `error`，不发 `done`。不把非法编码替换成字符，也不静默丢弃非法 JSON。
- 行和完整事件的累计字节上限为 8 MiB，含注释及忽略字段；未结束行或不停追加 data 的事件超限立即报错。该上限是单事件限额，不限制整篇摘要长度。
- 摘要发送 serde_json 序列化的 `{type:"chunk",content}`，与既有 ReaderSummary 消费者匹配；正文恰好是 JSON、引号、换行或控制字符也保持原文本。错误维持摘要 `message` 与翻译 `segment` / `error` 契约，统一序列化，防止字符串拼接破坏 JSON。
- 翻译仍用非流式请求；本项只修复串行 / 并发两条路径的错误编码。并发翻译错误后仍可能返回 done / success 的已有业务缺陷留在 M4W-004，没有在本项改写失败聚合、分段、缓存或重试。调度 / 取消与结果持久化分别留在 M4W-003 /002。

## 命令与实际结果

从仓库根目录执行，复用 [Common Controls 6 sidecar / 局部 PATH 脚本](../reader_pipeline/run_rust.ps1)，脚本仅修改忽略的构建产物并返回真实测试退出码。

```powershell
pwsh -NoProfile -File tests/reader_pipeline/run_rust.ps1 -Filter m4w_017_tests
cargo check --locked --manifest-path src-tauri/Cargo.toml
rustfmt --edition 2021 --config skip_children=true --check src-tauri/src/agent/provider.rs src-tauri/src/agent/provider/sse.rs src-tauri/src/agent/summary.rs src-tauri/src/agent/translation.rs src-tauri/src/agent/m4w_017_tests.rs
git diff --check
```

修复前首轮探针 0 通过、4 失败：跨块中文、SSE 多行 / 可选空格用量、EOF 误成功和 quote / newline 错误 JSON。首轮跨块断言先被原始 chunk 不符合 JSON 协议拦住，补充 raw 回退观察后再次执行，实际得到 `���文`，明确复现编码破坏；没有把失败算作通过。

修复后 12 项实际通过：

1. 本地 HTTP 拆开中文字节后中文 / emoji / 引号 / 换行保持完整。
2. 本地 HTTP 多行 data、可选空格、注释、CRLF 跨块及 usage 2 /3 /5 正确。
3. 正常 HTTP EOF 缺少 `[DONE]` 返回失败。
4. 摘要 / 翻译错误 JSON 往返恢复引号、换行、反斜线、Unicode、控制字符和 JSON 形文本。
5. LF / CRLF / CR 每种完整输入在所有字节位置二分，以及逐个单字节输入；BOM、注释、metadata、中文 / emoji 与看似 done 的正文不改变内容 / 用量 / 真正终止。
6. 不完整事件不发数据，`[DONE]` 要求空行；完整终止之后的无效字节不再处理。
7. 未结束行和未结束多行事件超 8 MiB 报错。
8. Channel 接收失败不误成功。
9. 本地 HTTP 非法 UTF-8 / JSON、无 choices / usage、API error、截断 DONE 与截断中文均失败。
10. 真正 HTTP chunked 传输中途断开保留 partial chunk，返回 Stream error。
11. 实际 run_summary + 内存库 + 本地 HTTP + Channel：成功发送 chunk / done；提前 EOF、带引号换行的 API error 发送可解析 error，没有 done。
12. 实际 run_translation 的串行 / 并发代码路径收到本地 HTTP 500 的引号 / 换行 / 中文 / 反斜线错误，Channel 中 segment / error 可解析且完整。

cargo check 通过，13 条既有警告；5 个本项 Rust 文件格式检查与 git diff 检查通过。没有修改前端、依赖、数据库迁移或权限，因此未运行 lint / build；没有执行全套 Rust 或全仓格式。

最后检查中曾临时用 `git -c core.autocrlf=false diff --check`，导致仓库现有 CRLF 文件被当成整文件新增并报 CR 尾空白。核对实际配置为 `core.autocrlf=true`、索引 LF / 工作区 CRLF 后恢复项目配置，`git -c core.safecrlf=false diff --check` 通过（仅隐藏转换提示）。本项 9 个文本文件另外严格 UTF-8 / 无 BOM / 行尾空白检查通过；WORKLOG 追加段落与原文统一 CRLF，没有改动全局配置或据该检查误改其他文件。

未验证：native Tauri IPC / WebView2 展示、真实提供商的网络分块 / 终止兼容性、真实 AI 与三平台。M4W-017 保持待验证，隔离 transport / agent / Channel 通过不替代实际桌面验收。真实 provider 不发送 `[DONE]` 将明确失败，这是本次防止断流误成功的协议选择。
