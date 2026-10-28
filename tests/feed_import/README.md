# RSS 导入去重、重试与失败清理

2026-10-29，基线99a6bb7加保留的RSS诊断改动。生产入口为`feed/import_service.rs`，普通/流式OPML命令及手动添加共用同一条保存路径。

- 地址比较保留路径大小写、查询参数、非默认端口和账户信息，归一HTTP/HTTPS、www、尾斜线和fragment。
- 不同地址的别名要求同名、站点相同（兼容旧RSS缺站点元数据）且至少3个文章链接重合，并覆盖较小文章窗口的80%。仅同名不去重；不同网站/栏目不能按名字合并。小于3篇或内容不充分的不同地址不推断为同源。
- 首次抓取/解析/空Feed失败后等待300ms重试一次。每次Feed请求30秒上限，涵盖接收正文；不改共享HTTP客户端或AI超时。数据库错误不重复HTTP。
- 新订阅和首批文章在同一事务提交；失败时无新空订阅。重新导入旧版从未同步且没有任何文章的占位源，成功恢复原ID，失败仅移除该空占位；已有成功源/含软删除文章的源不清理。既有有文章的重复源不自动合并或删除。
- 同一文件重复URL按进度序号区分，避免后一次“跳过”覆盖前一次“已添加”。双语显示重试中和重复说明。

## 回归

```powershell
./tests/reader_pipeline/run_rust.ps1 feed::import_tests
./tests/reader_pipeline/run_rust.ps1 m4w_005_tests
./tests/reader_pipeline/run_rust.ps1 m4w_006_tests
./tests/reader_pipeline/run_rust.ps1 m4w_011_tests
node tests/feed_import/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/accessibility/check_resources.mjs
npm.cmd run lint
npm.cmd run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

10项默认Rust覆盖重试成功、两次失败后继续、空正文/非法Feed/0文章、地址与内容去重、同名不同站/栏目、并发事务、SQL回滚、旧占位恢复/删除以及有限超时；另1项默认忽略的本地公开响应快照测试验证实际阮一峰/小众软件四地址只保存两源。该可选测试设置`IMPORT_PUBLIC_SNAPSHOTS`为含`response-02.bin`、`03`、`20`、`26`的前轮诊断目录后，用已设置Common Controls 6 sidecar及PATH的库测试exe运行`recorded_ruanyifeng_and_appinn_aliases_are_deduplicated --ignored`，无需联网。HappyDOM使用原有隔离依赖，不改npm锁文件。

原生受控验收启动`python tests/feed_import/native_server.py tmp/feed-import-native-20261029`，在新identifier测试库通过实际文件对话框导入生成的`import.opml`。预期5条为添加2、跳过2、失败1：retry的503→200、mirror重复、fail两次503、good继续、retry原URL重复；只产生6个loopback请求，侧栏仅Next Feed/Retry Feed各3篇，重启保持。不要向日常库导入这些合成源。详情和边界见[验收报告](../../docs/audits/feed-import-2026-10-29.md)。
