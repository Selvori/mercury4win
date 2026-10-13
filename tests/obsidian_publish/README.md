# M4W-013 Obsidian Publish 静态阅读分支

2026-10-13，HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加保留的001、016、005、008、006、011 /019、017、003、002、004、009、007、010、012与本项未提交改动。Windows /PowerShell7.6.5、Rust /Cargo1.97.0 `x86_64-pc-windows-gnu`。只使用内存SQLite与127.0.0.1随机端口HTTP，没有访问用户库 /凭据、远程站点或执行脚本；未提交 /推送。

## 依据与实现范围

只读上游 `G:/sth_interesting/mercury` 的README、`docs/features/obsidian-pub.md`、ReaderPipelineResolver /ReaderObsidianPipeline及Resolver /TypeImplementations /ObsidianIntegration测试。上游文档明确v1只做高可信shell识别、静态Markdown URL、直接抓正文和共享渲染；媒体使用 `/cache/{uid}` JSON对象的键作公开资源索引。

[upstream-shell.html](upstream-shell.html) 直接取自上游 `ReaderPipelineResolverTests.swift` 的 `obsidianShellHTML` 字符串（上游基线 `5b2e58ebccc772263b0fc70360f667c3abf03eea`），保留Chad Nauseam自定义域名和实际uid /shard /page路径。没有对远程当前页面重新抓取；测试中的其他shell /Markdown /资源索引是依据此契约构造的隔离fixture。

DOM要求精确Publish HTTPS base及内联script中静态 `window.siteInfo={JSON};` 和 `window.preloadPage=fetch("URL")` / `f("URL")`。忽略注释 /字符串 /模板中的伪声明；双引号JSON支持Unicode和转义斜线，单引号仅无反斜线字面量。URL按最终shell响应地址解析，保留路径编码和query，校验HTTP目标为HTTPS `publish.obsidian.md` 或数字shard `publish-N.obsidian.md`、无凭据 /非默认端口 /fragment，`/access/{uid}/...md`中的uid与siteInfo host /uid一致；不从自定义域名猜vault或page ID。不匹配则沿用default提取。生产不允许loopback Markdown目标；只有 `cfg(test)` 的端点验证器允许127.0.0.1 HTTP，便于真实HTTP传输和数据库回归。

分支使用受限重定向Client，在发出下一跳之前拒绝离开Publish endpoint或当前vault；最多10跳，每请求15秒。HTTP错误、HTML /JSON正文、空响应、非法UTF-8、没有可读Markdown的纯raw HTML明确失败。初次解析只保存 `pipeline_type=obsidian`、shell HTML、最终shell URL、`resolved_intermediate_content=Markdown URL`和 `display_mode=pending`；不保存Markdown或成功HTML缓存。失败继续原分支重试，005临时导读提示仍有效。正常完成才存 canonical Markdown和共享Reader缓存，`cleaned_html` /Readability标题作者版本保持空。重建失败不写新的成功正文。

Obsidian当前Markdown可离线重渲染 /命中主题字体缓存，无须Readability；Markdown过期从intermediate URL重抓，缺失身份从shell重新静态解析并保存。普通Readability从2升3，确保以前误存的shell不会被旧Markdown /渲染缓存直接命中；普通Markdown /render版本仍2 /4。005显式Feed正文优先路径及008分层base /缓存契约保留。007正文有效性调用共享按管道判定，避免将Obsidian成功正文误报缺失；007当前成功fixture改readability3，专门过期fixture保留。004 translation_document与摘要source_snapshot直接使用canonical Markdown，无须改其业务，缓存命中 /只重渲染不改变source hash。

普通Markdown链接 /图片用comrak AST按最终Markdown响应URL规范相对 /根相对 /编码URL；链接仅HTTP(S) /mailto /页内锚点，图片仅HTTP(S)。原始HTML最终仍由共享comrak安全设置禁用。媒体 `![[...]]` 在正文文本节点中按exact path /decoded path、唯一suffix或basename匹配公开索引，构造逐路径组件编码的 `/access/{uid}/{resourcePath}` URL；索引错误、缺失或歧义保留文本，不猜图片路径，代码块 /inline code不改写。本文只覆盖图片及常见媒体附件；普通wikilink、笔记transclusion、完整Obsidian语法与视觉复刻、登录 /私有站点、动态preload计算、generic SPA hydration不在本项范围。

## 实际命令与结果

仓库根目录运行：

```powershell
./tests/reader_pipeline/run_rust.ps1 -Filter m4w_013_tests
./tests/reader_pipeline/run_rust.ps1 -Filter m4w_005_tests
./tests/reader_pipeline/run_rust.ps1 -Filter m4w_008_tests
./tests/reader_pipeline/run_rust.ps1 -Filter agent::translation_segments::tests
./tests/reader_pipeline/run_rust.ps1 -Filter m4w_007
./tests/reader_pipeline/run_rust.ps1 -Filter m4w_013_redirect_media_safety_persistence_cache_and_consumers
cargo check --locked --manifest-path src-tauri/Cargo.toml
rustfmt --edition 2021 --check src-tauri/src/reader/obsidian.rs src-tauri/src/reader/pipeline.rs src-tauri/src/reader/m4w_013_tests.rs src-tauri/src/digest/templates.rs src-tauri/src/digest/m4w_007_tests.rs
rustfmt --edition 2021 --config skip_children=true --check src-tauri/src/reader/mod.rs
git diff --check
```

| 检查 | 实际结果 |
| --- | --- |
| 本项Rust定向 | 7通过，最终整组复跑15.05秒；补充percent-encoded媒体fixture及明确alt断言后该1项再通过 |
| 005 /008定向回归 | 9 /8通过 |
| 004原translation_segments | 6通过 |
| 007原文摘导出 | 5通过 |
| cargo check | 通过，6条既有unused import警告 |
| 6文件定向格式 /diff | 通过；Git有既有LF→CRLF提示 |

7项包含：真实上游shell、Publish域名 /自定义域名 /JSON转义 /相对URL与错误marker；两级302、中文Markdown路径、普通图片 /链接 /危险协议与raw script、索引图片 /歧义及代码保留；独立持久字段、摘要来源 /翻译document /007导出；SQLite拒写trigger证明重复缓存不写、render过期离线重渲染及hash稳定；503临时导读 /pending无成功缓存 /恢复 /Markdown版本重抓 /缺intermediate恢复；旧default shell版本2缓存升级重识别及显式Feed回归；HTML /JSON /空 /纯script /非法UTF-8与索引失败；跳出endpoint /变更uid的302拒绝且被禁止的本地listener没有收到请求；drop实际HTTP future关闭socket，15秒请求超时不写成功且可重试。

中间失败如实保留：首次测试字符串转义语法错误；首次AST实现跨await导致Tauri future不满足Send，改为网络阶段不持有comrak AST；新增旧cache fixture漏 `updated_at` 触发NOT NULL，补真实schema字段后7项通过。初次007 filter `m4w_007_tests`匹配0项，不计通过；改正确 `m4w_007` 后实际5项通过。没有弱化断言，没有更改历史审查 /018失败。

收尾UTF-8检查确认12个本项文件无BOM、严格解码成功。一次临时 `git -c core.autocrlf=false diff --check` 将既有CRLF行误判为行尾空白；检查实际仓库 `core.autocrlf=true` 后恢复普通 `git diff --check` 实际通过，没有改Git配置或为此重写行尾。

未验证：native Tauri /WebView2真实IPC、媒体实际加载 /链接打开 /排版、真实在线Publish及当前服务兼容、完整桌面退出重启、Linux /macOS；drop是生产future取消边界测试，不声称已新增Reader UI取消入口。没有前端改动，未运行lint /build或DOM；未运行全套Rust /全仓fmt。M4W-013保持待验证，下一项为014，尚未启动。
