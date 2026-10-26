# 主题追踪与增量简报

入口：侧边栏「主题追踪」。新建专题时可保存一份智能视图规则副本；相对日期仍滚动，原视图修改、改名或删除不改变副本。候选始终需要手动确认。也可在普通列表、搜索或智能视图中点「选择文章」，勾选后「加入专题」。删除专题只删除专题内容，不删除原文章、文章笔记、收藏或视图。

资料页显示已纳入、新增纳入、上次成功使用、正文缺失和内容／归组变化；相同正文默认归入同一证据组，可以人工更正。自动组只识别相同正文，近似转载需要用户和模型审查；不同网站并不自动构成独立证据。正文仅使用本地抽取文本，不用文章摘要补充缺失正文，不自动全库抓取。主动「抓取／重建正文」使用既有正文管道，失败可重试。

显式生成使用「设置 → 任务」中摘要任务的已启用主／备用模型路由，专题任务、结果、取消槽和用量类型独立。界面展示实际路由，开始时冻结配置。每次最多 40 篇，当前正文总计不超过 100,000 字符，当前＋基线完整请求不超过 160,000 字符；超限拒绝，不静默截断。这些是输入上限，不保证任意模型都有足够上下文；模型拒绝或输出不完整时保留旧成功版本。

首次输出为跨来源对照，增量请求同时提供上个成功版本的资料正文／结论和当前确认范围。新增、修订、移除、分组改变及正文就绪均进入比较；仅查看、刷新或保存个人判断不触发 AI。无变化时界面禁用生成，后端也在读取配置／凭据前返回。失败／取消不推进基线，写入前重新核对来源、专题修订和成功基线，使用现有提交围栏原子保存。

简报历史保留每版来源、原文文本、版本标识、分组、模型归属及时间；个人判断独立保存，生成不覆盖。点击证据显示保存正文并精确高亮唯一原文片段；位置为快照中的 UTF-8 字节偏移。历史引用不会重新绑定到当前正文；无法对应当前资料时明确提示，仍可打开正确文章。结构校验拒绝未知 ID、非原文／歧义片段、缺少关键引用、错误类别或超长输出。引用位置有效不代表判断正确。Markdown 导出保留引用和版本，以及已保存的个人判断。

## 自动回归

从仓库根目录，Windows GNU 沿用既有 runner：

```powershell
./tests/reader_pipeline/run_rust.ps1 db::topic_store::tests
./tests/reader_pipeline/run_rust.ps1 db::search_store::tests
cargo check --locked --offline --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm.cmd run lint
npm.cmd run build
```

专题模块 12 项入口（包含两个可选 fixture 导出），使用内存历史 Schema 或隔离临时磁盘库。覆盖 V1–V6 升级／重复迁移、CRUD／规则副本、成员／人工归组、正文就绪／增删改、笔记隔离、快照引用、无效／歧义引用、输入上限、陈旧范围／并发基线／取消围栏、真实 loopback HTTP 首次／增量／非法输出／503／重试及用量、实际 HTTP 取消、连接池重开和等价正文格式不产生假增量。测试文件不读取安装应用数据库或凭据。

生产 DTO 的界面回归：

```powershell
$env:M4W_TOPIC_WIRE_JSON = Join-Path (Get-Location) 'tmp/topic-ui-new.json'
try { ./tests/reader_pipeline/run_rust.ps1 export_topic_ui_fixture }
finally { Remove-Item Env:M4W_TOPIC_WIRE_JSON -ErrorAction SilentlyContinue }
node tests/topics/run_browser.mjs <playwright-core/index.mjs> tmp/topic-ui-new.json <browser.exe>
```

目标文件必须不存在。runner 使用单独安装的 Playwright 和真实 Edge，不改项目依赖。10 组覆盖生产 SQLite DTO、缺失正文、重复组、变化标签、引用／Escape 焦点恢复、个人判断、抓取失败、非法引用保留版本、重试／历史／无变化、本地刷新无 AI、12 种语言／主题／尺寸布局、规则副本 IPC、取消与卸载归属。mock 仅替换 IPC，不把它作为 native 或真实模型质量证据。既有文摘、筛选和分页 DOM 及搜索浏览器回归仍保留。

## 隔离桌面

使用独立 Tauri identifier。可选 `M4W_TOPIC_NATIVE_DB` 导出全新 V7 合成库；`M4W_TOPIC_SCHEMA6_NATIVE_DB` 导出真实 V6 合成库供生产升级。均拒绝覆盖已有文件。`provider.py` 只监听 loopback，不转发任何远程模型请求，可用 `control.json` 的 `mode=ok/invalid/error` 和 `delay` 控制响应；`requests.jsonl` 仅记录合成资料。

`native_fixture.py` 只接受明确命名的 `com.mercury4win.topics20261026/mercury.db`，并核对合成标题后才允许准备路由／修订或读取证据。不得把这个限制改掉用来操作用户数据库。

复验要点：无 AI 建专题／资料／备注 → 绑定规则副本 → 首次简报与引用 → 列表多选加入新证据＋正文修订 → 非法引用／取消保留旧基线 → 重试增量 → 历史旧引文与当前正文不同 → 正常退出重启 → 原生 Markdown 导出。另验证旧库升级、归档／恢复、人工归组、移除、正文抓取失败及删除保留原数据。只有实际完成的步骤可记为 native 通过。

实际检查和平台边界见 [专项报告](../../docs/audits/topics-2026-10-26.md)。受控 Provider 验证请求／保存／失败协议，不能代替真实模型的证据支持率、转载识别和简报质量评估。
