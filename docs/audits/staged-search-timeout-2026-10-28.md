# “新的AI模型”搜索超时修复（2026-10-28）

代码基线：81f7b2e，加保留的全部未提交改动；Windows / PowerShell7.6.7 / WebView2。问题M4W-031待用户原库续跑确认。此前分阶段检索报告和失败记录保持不变。

## 原故障依据

只读实际应用SQLite中的扫描状态和请求事实，未导出用户库、文章或笔记。北京时间15:30:15.355～15:30:42.839，search_plan成功；15:30:43.012～15:32:43.022，search_titles超时，用量未返回。该批64个标题，固定范围609篇，title_done均为0，64项标记timeout，其余545项待处理，未开始预览/正文。旧Mercury.log最新到前一天，没有本次报错，不能据此认定模型格式失败。事实证明请求耗尽120秒deadline；批量输出过大是根据每标题独立判定/理由和后续小批次耗时作出的工程判断，无法从未知响应证明服务商内部耗时原因。

## 修改

- agent/staged_search.rs：标题最多16项/6000序列化字符；仍遍历完整硬允许范围，关键词不作准入。无总token预算、自动重试或正文截断。
- db/staged_search_store.rs：显式恢复在claim清除错误前按上次失败标题数减半，至少1项、最多16项。原64项失败改用16；新16项超时改用8。
- 阶段日志含phase/items/elapsed_ms/error，不含查询、文章、原始模型输出、Provider错误正文或凭据。
- 首轮真实复测出现 `[{"articles":[...]}]`；batch_value只解包单元素/单已知键/数组值。仍严格检查未知字段、真实ID、唯一完整批次和原文引用，多个容器/嵌套数组不能借此绕过校验。原始合成交换加入protocol_cases.json。
- tests/staged_search/native_provider.py支持只延迟指定阶段，并把新合成负例ID视为无标签；仅loopback，不转发外网。

## 真实API

使用当前主模型ecnu-plus，仅读路由表与选中一份凭据到内存；文章均为固定71篇合成语料。明确只对本查询诊断/从零复验，没有把它当成七主题全套。

| 轮次 | HTTP请求 | 已报token | 结果 |
| --- | ---: | ---: | --- |
| 首轮 | 15 | 34,400 | 全部标题通过；17/26块时整篇容器解析失败，已确认3篇，失败保留 |
| 修正后从零 | 23 | 43,126 | 71标题、26块全部处理结束；预期12篇全部命中，漏选/误选0；258.374秒 |
| 本任务合计 | 38 | 77,526 | 两轮均无回放，HTTP全部成功；首轮业务解析失败没有改成成功 |

首轮五个标题批次各6.319～34.447秒；最终五批7.288～17.019秒，均小于120秒。一次成功不能保证今后服务商不超时。实际原库609篇没有在本轮发送给模型。

证据：[首轮失败](staged-search-timeout-2026-10-28-live.json)、[首轮逐请求](staged-search-timeout-2026-10-28-live.requests.jsonl)、[最终从零](staged-search-timeout-2026-10-28-recheck.json)、[最终逐请求](staged-search-timeout-2026-10-28-recheck.requests.jsonl)。

## 离线与桌面

- 10项定向staged_search Rust通过，包含609篇/无自动重试/旧64项恢复/8项自动完成/不重新规划、来源版本和范围、真实响应协议回归、暂停实际HTTP及旧拥有者围栏。付费入口默认忽略，另按上节显式执行通过。
- 生产DOM4组、npm run lint、npm run build、cargo check --locked、cargo fmt -- --check通过。Tauri独立debug完整构建通过，已有大chunk提示。未运行全library；此前4项socket取消超时失败基线未因本轮而关闭。
- 使用computer-use技能在独立identifier、609篇合成库及18763loopback完成实际120秒超时。日志elapsed_ms=120020，16项失败后UI恢复至8项/批；完整609/609标题、607模型初筛、26/26资料块、12结果、1资料不足、0剩余/错误。94个HTTP事实=93成功/1超时，超时用量未知，无笔记外发/密钥文件/外键错误。
- native构建包含标题缩批/恢复/日志修复，先于随后单元素容器修复；该格式由保存交换Rust及最终真实API单独覆盖，没有冒充最后代码重新完整native。
- 隔离应用正常退出，Provider进程已停止，18763无监听。日常dev因源码监听自动重编译/重启；CU确认原检查点可以只读载入，恢复按钮可见，未点击原库恢复，原search_filter事实仍为原先2条。

原生证据：[结构化请求事实/批次](staged-search-timeout-2026-10-28-native.json)、[超时与完成UI文字](staged-search-timeout-2026-10-28-ui.txt)。未保存截图或日常文章内容。界面截图与UIA曾出现不同窗口，重新激活隔离目标并重读后才继续操作。

## 复验入口与边界

```powershell
$env:CARGO_TARGET_DIR="$PWD/tmp/topic-search-native-20261028/target"
./tests/reader_pipeline/run_rust.ps1 staged_search
node tests/topic_search/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
```

明确授权下真实复验：设置M4W_STAGED_CASE=models_zh、M4W_STAGED_SKIP_BASELINE=1，使用tests/staged_search/run_live.ps1及新的ReportPath；默认不读取日常业务资料。合成609篇验收须使用新的隔离identifier和合成种子；不可清空日常库。

用户原库本次609篇尚未续跑，M4W-031保持待验证。其他平台、安装升级、任意模型/规模稳定性未验证。未提交、推送、发布、改依赖/锁文件或创建新聊天/子代理。
