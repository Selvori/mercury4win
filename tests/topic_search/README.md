> 当前生产入口已改为[全标题分阶段检索](../staged_search/README.md)。本页的36篇流程及旧真实报告保留为v4历史基线；本目录 DOM runner 现验证当前生产组件。

# AI 主题搜索

入口为「全文搜索 → AI 主题搜索」，替代仅将自然语言变成 ALL 关键词的交互。旧解析 IPC、普通关键词、已有智能视图和专题功能保留。

明确提交后复用摘要主备路由：语义理解与硬条件 → 中英 OR 同义词 / AND 必要概念 / 多路 OR → 参数化本地召回 → 模型逐篇判定、排序及提供逐字依据。不是只做翻译。检索已有标题、摘要和正文，不下载缺失正文。默认不检索 / 发送笔记，仅明确请求笔记或手选笔记字段才允许；has_note 只判断存在性。隐私提示反映候选片段实际会发送的行为。

“新”表示发布事件，不默认限制最近七天。明确来源、标签、读 / 收藏 / 笔记状态、日期及字段在本地约束；手动条件一起应用，冲突报错。普通关键词不混入 AI 条件。模型只能返回词面、名称和给定候选 ID，不能执行 SQL。

每次最多 3 批 × 12 候选。原路线最多 6、每路最多 4 组、每组最多 8 词，扩展合计最多 3,500 字符；程序补宽泛路线和原句中的陌生型号后最多 16 路。交错语言、元数据 / 正文和新旧时间通道，不只取最新若干篇。显示完整本地候选总数及剩余检查数，明确继续才产生后续请求。没有命中不代表全库没有相关文章。

正文最多三个 700 字片段，按命中密度与具体程度优先，检查首尾命中；标题最多 360 字、摘要最多 700 字。缺失 / 截断上下文有标记。模型须覆盖全部候选且 ID 唯一，分数 0–100；至少 60 才入选，必须有来自提供片段的逐字依据。

Schema 9 保存不可变的已发布范围。分页 / 全部已读使用后端验证的令牌，不传巨型 ID 数组，不再请求模型；未知、篡改或未发布令牌拒绝处理，绝不退回全库。继续先分叉检查记录，再发布新范围；失败、取消、条件编辑及晚到响应保留旧有效列表。内容变化提示过期并阻止批量操作；读 / 收藏变化保留成员，但续查须重新确认。相对日期锚定该次检查时间。新搜索才清理超过七天且不在最近 20 次内的记录，短期失败不挤掉本周有效范围。

AI 快照暂不能保存成智能视图，UI 说明原因，后端也拒绝把临时范围存成智能视图 / 专题规则。退出后普通条件视图仍可保存。

响应只接受完整 JSON 对象或完整外层 JSON 围栏，不从说明文字中猜取对象。路线中重复的 required_source / recent_days / starred_only 只有与已验证的全局硬条件一致才归一；未知字段和冲突拒绝。创建范围前确认索引已刷新并固定读快照，连续更新造成无法取得干净快照时明确失败，不缓存虚假的完整空结果。

## 固定资料与受控复验

[corpus.json](corpus.json) 为人工标注的 20 篇中英合成文章、7 个固定案例。包含陌生型号、180 天旧文、正文长前缀之后的发布消息，以及融资 / 教程 / 旧模型顺带提及 / 汽车 / 数据库图 / 提示注入干扰；另有数据库性能、开源构建工具两个非 AI 主题、明确一周 / 收藏 / 来源条件和无相关文章案例。模型质量按最终入选 ID 对照人工标签。

```powershell
./tests/reader_pipeline/run_rust.ps1 topic_search
./tests/reader_pipeline/run_rust.ps1 db::
./tests/reader_pipeline/run_rust.ps1 agent::runtime::tests
./tests/reader_pipeline/run_rust.ps1 agent::search_filter
node tests/topic_search/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
npm.cmd run lint
npm.cmd run build
cargo check --locked --offline --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
git diff --check
```

DOM 只替换 native transport，运行生产 EntryList、SearchControls、NaturalFilter 和 IPC wrapper，覆盖阶段 / 依据 / 分页 / 批量 / 显式继续 / 禁保存 / 草稿 / 失败 / 取消 / 连续请求 / 收起 / 卸载 / 退出 / 双语。原 natural_search/run_dom.mjs 兼容转向本套件，旧 DOM 源码仅保留历史语义。

Rust 受控 HTTP 以人工标签构造确定性回复，验证生产 HTTP、校验、召回、SQLite、分页、缓存、批量与取消，不证明真实模型理解质量。迁移验证 V8 升级、重复运行及保留笔记。片段测试先复现 maintenance 内的 ai 抢占后部正文，再修正通过。

## 真实 API 与原生

付费入口默认忽略，仅在明确授权后运行：

```powershell
./tests/topic_search/run_live.ps1 -AppDataDirectory '<已核实的应用数据目录>' -ReportPath '<不存在的报告绝对路径.json>'
```

只读复制 AI 配置白名单，内存解码选中主模型凭据；业务资料全部来自合成库，不读取日常文章 / 正文 / 笔记，不写日常库。每轮 7 例，每例最多理解 1 次 + 判定 3 次，主模型最多 28 次，没有 fallback / 自动重试。诊断单例额外设置 M4W_TOPIC_LIVE_CASE，不能计作最终整套通过。逐例 progress 与最终报告保留实际交换、命中、漏选 / 误选、请求和服务端 token 用量，失败不覆盖。

[真实 API 报告及历史失败](../../docs/audits/topic-search-live-2026-10-28.md)。[Windows 隔离验收](../../docs/audits/topic-search-native-2026-10-28.md)。seed_native.py 从已核实的合成 V5 fixture 新建目标，拒绝覆盖；应用实际迁移。native_provider.py 只监听本机 18762，不转发外网。原生库为固定 20 篇 + 60 篇干扰项，用于分批与交互验收，不用于模型质量结论。

其他平台、大型用户库、模型稳定性及真实网络 fallback / 取消完整矩阵未验证，历史失败及旧问题状态保留。
