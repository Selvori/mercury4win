# M4W-008 Reader URL、版本与缓存回归

2026-10-12，HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加保留的 M4W-001、M4W-016、M4W-005 及本项未提交改动。Windows 11 / PowerShell 7.6.5，Rust / Cargo 1.97.0 `x86_64-pc-windows-gnu`，Node v24.12.0、临时 Happy DOM 20.14.6。仅使用内存 SQLite、本地 HTTP 与内存 IPC，没有访问用户数据库、凭据或远程站点。

## 实现与边界

抓取先保留最终响应 URL，再用 HTML DOM 读取第一个带 href 的 base；仅 HTTP(S) base 有效，非法 base 回退响应 URL。Feed 正文使用文章 URL 作基准。链接 href / 图片 src 在 HTML → Markdown 前经过 DOM URL 规范化；相对、根相对、协议相对和实体编码 URL 均能解析，链接保留页内锚点和 mailto，其余危险/本地协议被移除。Reader 点击仅 HTTP(S) 交给系统浏览器，页内锚点留在文档，其他协议禁止 iframe 导航。raw HTML 仍由 comrak 默认禁用，iframe 仍只有 allow-same-origin。未增加桌面权限。

Readability / Markdown / Reader render 版本为 2 / 2 / 4。上游过期时不使用下游缓存；优先有效 Markdown，其次有效 cleaned_html，其次持久 source HTML，最后才抓取。Markdown 重建落库；正文存储与所有主题/字体缓存失效在同一事务，其他主题不会继续返回旧内容。只有全部相关版本有效才读取 HTML 缓存；渲染版本过期只重渲染，主题、字体家族和字号继续分别进入缓存键。

已解析 base 持久保存，离线重建不再次拼接相对 base；M4W-005 的版本 1 行只曾保存文章 URL，重建时重新解析 HTML base。新增 markup5ever_rcdom 0.38.0 直接依赖以操作 htmd 的同一个 DOM；锁文件仅添加已有包的直接依赖关系，没有升级包。

empty 终态仅表示没有 URL、显式正文和导读的稳定无来源情况；同版本 empty 不反复写库或抓取，旧版本 empty 仍重新判断。HTTP / 连接 / 挑战页 / 提取 / 转换故障不保存 empty 或成功缓存，005 的摘要临时展示和重试继续有效。普通错误显示原因及双语重试按钮。失败重建保持原持久层和缓存不变，当前请求报告故障或展示临时导读。

## 命令与实际结果

仓库根目录运行：

```powershell
cargo check --locked --manifest-path src-tauri/Cargo.toml
./tests/reader_pipeline/run_rust.ps1 m4w_008_tests
./tests/reader_pipeline/run_rust.ps1 m4w_005_tests
./tests/reader_pipeline/run_rust.ps1 reader::
npm.cmd run lint
npm.cmd run build
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" ../reader_pipeline/browser.tsx
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" ../feed_content/browser.tsx
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
rustfmt --edition 2021 --check src-tauri/src/reader/pipeline.rs src-tauri/src/reader/html_to_md.rs src-tauri/src/reader/m4w_008_tests.rs src-tauri/src/db/content_store.rs
rustfmt --edition 2021 --config skip_children=true --check src-tauri/src/reader/mod.rs
git diff --check
```

| 检查 | 结果 |
| --- | --- |
| M4W-008 Rust 定向回归 | 8 通过、0 失败 |
| M4W-005 Rust 回归 | 原 9 项通过 |
| cargo check | 通过，15 条已有警告 |
| 生产 ReaderPane 错误重试 / 链接导航与沙箱 | 2 项隔离 DOM 通过 |
| M4W-005 生产 ReaderPane | 原 2 项通过 |
| M4W-001 归属回归 | 原 15 项通过 |
| lint / build、本项 Rust 格式、diff 检查 | 通过 |
| 扩大到全部现有 Reader 单元测试 | 21 通过、1 失败；原 `md_to_html::tests::test_render_with_theme` 的 `<h1>Title</h1>` 精确预期未适配 header ID，留待 M4W-018 |

8 项 Rust 覆盖：base / URL 协议安全；302 最终地址 + 相对 base、图片/链接及离线重建；过期 Markdown 从当前 cleaned_html 重建落库并失效其他主题；过期 Readability 优先原始 source；缓存命中通过拒绝写入的 SQLite trigger 证明不重写；仅渲染版本失效及暗色/serif/22px 独立缓存；empty / 旧 empty / 临时网络失败的重试区分；失败重建不改变原持久层；其中缓存与渲染身份放在同一测试，另有版本 1 旧 base 本地重建测试。

## Windows GNU 测试启动问题

初次 `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib m4w_008_tests` 编译成功，但新 `app_lib-54fad253a9415ff6.exe` 在进入 harness 前报 `0xc0000139 STATUS_ENTRYPOINT_NOT_FOUND`。局部调整 GCC / Rust bin PATH 未解决，未记作业务通过。

objdump 核对旧 `app_lib-19e54d3c249f8340.exe` 与新 exe 均无内嵌资源 manifest，均导入 comctl32 的 TaskDialogIndirect；DLL 检查发现默认 Common Controls 缺少该入口。旧 exe 旁实际已有 `src-tauri/target/debug/deps/app_lib-19e54d3c249f8340.exe.manifest`（2026-10-11），声明 Common Controls 6。复用该 manifest 到新 exe 旁、刷新 exe 修改时间令 Windows 重读，并仅为进程追加 target/debug 以找到 WebView2Loader，测试才正常运行。这解释了先前同环境不同测试 exe 的启动差异，未改写历史报告。

`run_rust.ps1` 把步骤变为可重复命令：由 cargo JSON 获取实际 lib 测试 exe，写相同 Common Controls 6 sidecar、刷新时间、设置并恢复当前进程 PATH，再执行过滤测试并返回真实退出码。仅影响 Git 忽略的构建产物，不修改全局 PATH、注册表、应用配置或用户文件；针对当前 Windows x64 GNU 环境。其他平台可直接使用 cargo test 对应过滤器。

未验证：完整 Tauri IPC、真实浏览器/WebView2 的实际图片加载、系统浏览器打开、视觉排版、桌面重启和三平台运行；未执行全套 Rust 或全仓格式检查，本项定向格式通过不代表历史格式问题关闭。M4W-008 保持“待验证”。

## 2026-10-25 native 部分验收

历史段落保留当时2 / 2 / 4基线；当前生产版本为3 / 2 / 5。`4b652fb`加保留的001 / 016 / 005未提交改动，完整隔离Tauri / WebView2实际通过无导读503可见错误、真实按钮重试恢复且失败不落成功缓存；302最终地址 + HTML相对base、相对图片实际加载，正文链接点击使系统Edge请求正确目标并返回200。危险JS / file / data链接在native变普通文字，未补点击。

浏览器目标页截图 / UIA因Computer Use当前URL策略检查终止而未取得；立即停止CU并reset，未报告物理Escape，也未泛化为全部Windows不可用。旧source / Markdown / render预置三篇尚未执行native动作，缓存命中 / 主题字体与重启复用仍未验证；此次source锚点被展开为绝对URL的语义需核对。最低关闭条件不足，M4W-008保持待验证。正式[报告](../../docs/audits/native-008-2026-10-25.md)、[脱敏事实](../../docs/audits/native-008-2026-10-25-evidence.json)及[UIA](../../docs/audits/native-008-ui-2026-10-25.txt)记录边界。仅本机18748测试服务已停止，隔离数据与scoped审计保留；无本项源码变更，未重复Rust / DOM / 构建。
