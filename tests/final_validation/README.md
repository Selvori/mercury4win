# M4W-018 最终验证入口

完整结果、初始失败分类及格式范围见 [2026-10-13 验证报告](../../docs/audits/final-validation-2026-10-13.md)。原审查和此前逐项回归记录保留当时基线。

## Rust 与前端检查

在仓库根目录运行，使用现有锁文件和依赖：

```powershell
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo check --locked --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm.cmd run lint
npm.cmd run build
git diff --check
```

Windows GNU 测试 executable 若在 harness 启动前报 `0xc0000139`，使用既有脚本为忽略的构建产物补 Common Controls 6 sidecar，并局部加入 WebView2 loader PATH：

```powershell
pwsh -NoProfile -File tests/reader_pipeline/run_rust.ps1 -Filter ''
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

空 filter 运行完整 library 测试，不设置单线程或忽略测试；后续普通 cargo 命令还包含 binary / doc 入口。需要准确记录实际使用的方式，不把初始 executable 加载失败计为产品测试失败。

## 全部隔离 DOM 回归

```powershell
pwsh -NoProfile -File tests/final_validation/run_dom.ps1
```

默认使用已有临时 Happy DOM 20.14.6、jsdom 30.1.2，以及 Codex 已安装 Node 24.19.0；脚本不安装依赖。可通过 `-HappyDomPath`、`-JsdomPath`、`-SafeNodePath` 指定实际安装路径。安全净化回归必须使用满足 jsdom engines 的 Node 与 jsdom，不能替换为 Happy DOM。

脚本顺序执行 15 套生产组件 / 实际 IPC wrapper 回归（主题套件为三种初始状态），并核对双语资源。四个 fixture 由真实 Rust DTO / renderer 定向生成到独立临时目录；用量 bundle 直接加载 JSON，不改写仓库 fixture.ts。生产代码只替换 native transport，不访问用户数据、凭据、真实 AI 或远程站点。每套单独保存 log，汇总 summary.json 包含 exit code 和实际断言计数；失败返回非零。输出目录可用 `-OutputDirectory` 指定，环境变量与工作目录结束后恢复。

Rust fixture 生成是准备步骤，不重复计入完整 Rust 套件总数；各 DOM“项 / 组”沿用原脚手架计数，安全 URL 变体等内部断言不单独累加。隔离 DOM / fixture 成功不能替代 native Tauri / WebView2、系统焦点 / 字体排版、实际退出重启、真实 Provider 和三平台验收。
