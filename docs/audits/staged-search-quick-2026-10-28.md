# 多语言关键词先展示、AI继续核验

日期2026-10-28，基线81f7b2e加全部保留未提交改动；Windows / PowerShell7.6.7 / Tauri WebView2。用户明确允许真实API及Computer Use。此次使用computer-use技能，关闭目标窗口采用Alt+F4，无重复点右上角。

## 变更

算法topic-search-quick100-v4、Schema10不变。复用一次主题理解的多语言词，全部组扁平OR，在硬条件内最新100篇本地资料中命中任一词就展示；标题12、摘要4、正文2、显式笔记8的每词字段权重，短ASCII词按词边界。初步成员单独标注未AI核验，随后继续原有标题/预览/连续正文/最终依据核验，确认成员优先，逐步增减；默认笔记不参与。

关键词预览保存在既有阶段缓存，staged会话flags保存初步数量；旧会话不受影响。每次成员/判定变化发布新不可变令牌，空快照也更新。可见matches包含初步+确认，scan.matches仍只计AI确认。分页/排序/批量使用点击时的令牌且不调用AI，版本过期仍阻止严格批量。旧算法检查点不再recent/resume，旧结果可读。

NaturalFilter错误保留当前快照、取消恢复此前范围、收起继续、紧凑态仍可暂停/取消。同查询渐进快照不再清空Reader选择；初步结果不展示AI已核验依据标签。同步688对中英文资源。

## 离线结果

- 14项staged Rust通过、1付费测试默认忽略；首个标题HTTP响应被闸门阻塞时，快照可读/分页/严格批量，部分概念单独命中可见。AI可移除700/701弱命中并加入无扩展词104，旧令牌不变。另验证失败保留及默认不查笔记、显式笔记编辑后过期。
- 原topic6项、普通搜索12项通过（另1合成性能忽略）。未重跑完整library；此前219通过/4既有socket取消失败/4忽略的全套基线不改写。
- 生产DOM4组通过：初步标签/读取、分页/批量旧令牌、空快照、错误保留、首次取消、收起继续、卸载暂停、Reader保持及晚到围栏。
- npm.cmd run lint、npm.cmd run build、cargo check --locked --manifest-path src-tauri/Cargo.toml、cargo fmt --manifest-path src-tauri/Cargo.toml -- --check通过。688对资源键/插值、UTF-8和diff检查另核对。隔离及默认identifier完整debug构建通过；既有chunk大小提示保留。独立target最初因日常实例持有DLL/EXE报os error32，后用根target成功，不强杀持有者。

## 真实API：保留首扫失败及显式恢复

只读现有摘要主路由ecnu-plus的一份凭据，密钥仅在内存；网络内容全部来自71篇合成内存文章，无日常文章/笔记外发、无日常库写入。固定单例models_zh，不代表七例重测。

[首扫报告](staged-search-quick-2026-10-28-live.json)及[逐请求原始合成交换](staged-search-quick-2026-10-28-live.requests.jsonl)：15实际HTTP、34,502已报token，首个非空关键词结果17.547秒，首个AI确认550.375秒，总664.269秒。最后final批含105资金说明书，模型未给实际保存依据却判断相关，严格校验拒绝整批，留下5确认/8未完成；不能算首扫成功。

[恢复报告](staged-search-quick-2026-10-28-resume.json)、[检查点](staged-search-quick-2026-10-28-resume.resume.json)及[新增请求](staged-search-quick-2026-10-28-resume.requests.jsonl)：严格输入匹配回放15个已付费响应，重建同一invalidJudgment；测试显式恢复一次，新增8实际HTTP、10,774token，71篇/26块/12预期结果完成，初步余量0。此次新增总计23HTTP、45,276已报token；含回放的44ms/126ms不是首次性能，43.556秒也不是冷启动全程。没有为通过放宽原文依据检查。

## Windows原生

[结构证据](staged-search-quick-2026-10-28-native.json)，[全新补验UIA](staged-search-quick-2026-10-28-ui.txt)。本地18763受控Provider不转发，无凭据；受控语义借用标签，仅证明协议/交互。

1. 第一实例com.mercury4win.quick20261028、609篇合成库冻结100篇：标题仍进行时24初步/0确认已可读，Sable文章打开；注入预览503后17初步仍可读，当前Reader保留，显式恢复100标题/26块/12预期匹配完成。25loopback（24成功/1故意失败）。当时UIA因会话重置未落盘，操作观察保留在本次工具输出，首个快照和最终结果另有数据库证据。后续资料被改变（当前0文章/新来源），本次未覆盖或还原；无法用这份变动后库声称重启结果读取通过。
2. 全新com.mercury4win.quickfinal20261028、71篇合成库：首个标题响应前实际UIA标题0/71、24初步/0确认，打开Sable；收起面板时显示3确认/10待核验、6/26块、暂停/取消可用，Reader保持。收起状态完成71标题/26块/12预期结果、0待核验；22loopback全成功，71文章仍保留。截图现场检查未落盘，UIA文字及数据库快照落盘。

两实例累计47loopback，46成功/1故意失败，无笔记标记出站、凭据文件0、外键错误0。首次会话恢复时sky变量已重置，重新初始化并选择唯一窗口；一次无截图UIA操作报coordinate input geometry unavailable，重取含截图状态后继续。此前日常旧窗口遇用户输入保护时停止关闭，未反复尝试；本轮继续时旧进程已不在。

## 边界

初步OR匹配可能包含无关文章，后续核验会变化；需等待一次主题理解，不保证固定首屏秒数或完整AI更快。首扫真实final仍发生校验中断，恢复成功不等于稳定率证明。未用日常库发起新搜索，旧M4W-031/032待验证状态保持；Linux/macOS、安装包升级及广泛模型质量未验证。未提交、推送或发布。


## 交付与清理

两个隔离窗口均Alt+F4退出，18763监听停止。最终默认配置完整构建成功，已启动src-tauri/target/debug/mercury.exe，Computer Use确认唯一默认Mercury窗口和全文搜索入口；未提交日常新AI查询。

30个本轮相关文件UTF-8/无BOM、JSON/JSONL及文档链接核对通过。最初替换字符全文件断言命中WORKLOG历史SSE损坏示例（刻意记录的三个替换字符）；保留历史，改为仅检查本轮新增记录，新增文本无替换字符。git diff --check通过；CRLF提示为既有工作区行尾设置。
