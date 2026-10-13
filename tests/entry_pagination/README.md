# M4W-006 文章分页回归

2026-10-12，HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加保留的 M4W-001、M4W-016、M4W-005、M4W-008 与本项未提交修改。Windows / PowerShell 7.6.5，Rust 1.97.0 GNU、Node v24.12.0、临时 Happy DOM 20.14.6。仅使用内存 SQLite、内存 IPC，没有打开真实用户数据库、读取凭据或请求远程站点。

## 行为与范围

生产查询按 `published_at DESC, id DESC` 排序，结构化游标 `{ published_at: string | null, id: number }` 保存最后一条已返回文章的排序值。非空日期继续查询更早日期、同日期较小 ID 和 NULL；空日期只继续查询 NULL 且较小 ID。游标不依赖边界文章仍存在。读取 `limit + 1` 判断尾页，尾页 `next_cursor` 为 null，不额外提供空页。limit 在数据层限定为 1～500。

总数只使用当前筛选条件，不加入游标；Rust / Tauri `load_entries` / TypeScript `EntryPage` 和 IPC wrapper 使用相同游标对象。原数字字符串游标是内部未使用的旧契约，本项改为完整对象；不存在用户数据格式迁移。文章日期或订阅在浏览过程中被同步改动时，查询反映当前数据，并非冻结快照；失效查询后 React Query 从第一页顺序重取已加载页。

EntryList 使用 useInfiniteQuery 累积页面和明确的“加载更多”按钮，保留原有 entries 缓存键前缀及失效调用；不会因后台加载清空列表。按钮为原生可聚焦 button，请求期间禁用重复加载；失败显示双语提示并可重试。范围切换使用独立 Query Key，旧页完成不能写进新范围。未扩展 M4W-011 的搜索、未读、标签 All / Any、批量操作和快捷键语义。

## 复现命令与实际结果

仓库根目录运行：

```powershell
cargo check --locked --manifest-path src-tauri/Cargo.toml
./tests/reader_pipeline/run_rust.ps1 -Filter m4w_006_tests
node tests/entry_pagination/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
npm.cmd run lint
npm.cmd run build
rustfmt --edition 2021 --config skip_children=true --check src-tauri/src/db/entry_store.rs src-tauri/src/db/models.rs src-tauri/src/db/entry_store/m4w_006_tests.rs
git diff --check
```

临时 Happy DOM 安装方式见 [原回归脚手架](../reader_ownership/README.md)。Windows GNU 测试复用 [run_rust.ps1](../reader_pipeline/run_rust.ps1) 的 Common Controls 6 sidecar 与进程局部 PATH，没有修改全局环境。第一次测试编译因新增测试将 COUNT 读为 rusqlite 不支持的 usize 失败；改为 i64 后定向测试实际通过，没有将编译失败算作通过。

| 检查 | 实际结果 |
| --- | --- |
| cargo check | 通过，15 条已有警告 |
| M4W-006 SQLite 定向回归 | 4 通过、0 失败 |
| 生产 EntryList + 生产 TypeScript IPC wrapper | 6 组隔离 DOM 交互通过 |
| npm lint / build | 通过 |
| 定向 Rust 格式 / diff | 通过；entry_cmd 仅检查 imports 与 load_entries，保留其他命令历史格式 |

Rust 覆盖 237 条乱序插入、同日期及 NULL 日期，分别以 1 / 7 / 100 / 完整总数翻页，对照独立 ORDER BY 查询验证精确顺序、完整性、去重、计数和尾页；另验证 feed + unread + search + tag 原有组合筛选、空结果、边界删除后继续及晚插旧文章、0 / u32::MAX limit。游标 JSON 序列化 / 反序列化含 NULL 同步核对。

DOM 使用 StrictMode、React Query、生产 EntryList、生产 Button 和实际 load_entries / mark_read / delete_entry wrapper。只替换 `@tauri-apps/api/core` native transport 与 i18next；不 mock 生产分页 Hook / wrapper。六组实际断言：

1. 100→200→237 顺序精确，总数固定；复合游标传递正确，pending 保留列表并禁用按钮，尾页按钮消失，最后一行 Enter 可选择。
2. 最后一页已读 / 删除后顺序重刷所有已加载页，状态更新，总数变为 236，无边界重复。
3. 切换订阅不显示其他范围旧行；下一页失败保留 100 行、显示 alert，同游标重试成功到 200 行。
4. 下一页延迟完成前切换订阅，旧完成不能混入新订阅。
5. 首次失败和成功空列表区分，重试恢复；无选择时不发查询。
6. 标签范围继续通过生产 wrapper 传递 feedId=null 和 tagIds，标签业务筛选在 Rust 组合测试核对。

未验证：完整 Tauri IPC / WebView2 桌面滚动、Tab / Enter 按钮操作、真实同步后的列表刷新、视觉排版和三平台运行。DOM 验证原生按钮与行的可操作结构，不能替代 WebView2 默认键盘事件和布局验收。未重跑全部 Rust 或全仓格式，历史失败仍归 M4W-018。M4W-006 保持“待验证”。
