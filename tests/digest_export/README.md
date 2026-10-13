# M4W-007 文摘预览与导出回归

日期：2026-10-13。基线：HEAD `c90567b` 加保留的001、016、005、008、006、011 /019、017、003、002、004、009及本项未提交改动。Windows / PowerShell7.6.5、Rust1.97.0 GNU、Node v24.12.0、临时Happy DOM20.14.6。没有读取用户数据库、凭据或调用真实AI /网络。

## 实现

- 三套内置模板改为合法Handlebars `#each` / `#if`，严格模式拒绝缺字段，未知模板 /不支持的篇数明确失败。`single_text`现在是真正纯文本，`single_markdown`和`multiple_markdown`为Markdown；原模板的HTML占位不保留成伪纯文本格式。标题和URL必需，缺失明确报错；作者遵循上游`entry.author -> readability_byline -> feed title -> empty`，空作者省略标签。Markdown元数据转义，正文 /笔记 /摘要维持原Markdown；纯文本通过安全comrak渲染和DOM抽取去除格式，保留链接目标和段落。
- 装配只读已保存内容；正文仅包含008当前readability /markdown版本、非empty的缓存，过期或缺失明确列入missing，需先在Reader获取 /重建。005生产Feed源更新事务删除派生content，避免导出旧正文；不联网抓文。笔记读取note_store，不包含001会话草稿，有草稿时窗口提示。摘要直接读取009 `latest_summary`，必须是当前source匹配完整success，带模型 /语言 /详细度；配置停用或无凭据仍可读，source变化不导出旧成功。旧slot不删除，不请求生成。
- 单篇Reader工具栏和多篇列表入口均接通，预览使用`pre`纯文字展示实际文件内容。单篇默认包含已有非空保存笔记 /成功摘要，正文默认false；多篇遵循上游摘要 /笔记默认false，正文也false，可分别选择。缺失的选中字段逐篇提示并省略；读失败阻止导出且可重试。保存对话框提供md /txt筛选，取消显示“没有写入”，失败可见，成功显示路径。
- 多选冻结当前已加载列表，停止后续页 /刷新 /已读 /收藏 /删除 /快捷键，Reader通过inert暂时停止交互；按列表序生成ID，与勾选顺序无关，没有硬性5篇上限。确认 /取消保持Reader选择，范围变更 /卸载清理选择。单篇切文关闭窗口，迟到结果按查询key /组件生命周期拒收。
- `preview_digest`注册、Rust模型 /TS类型 /真实wrapper一致；`export_multi_digest`补齐封装。导出重新装配并与`expectedContent`核对，笔记或来源变化拒绝覆盖已有文件并要求刷新。`tokio::fs::write`写UTF-8无BOM，不在异步线程阻塞同步写。现有generate /export命令保留，options为可选（缺省全false）；生产界面始终提交选项和已展示内容。
- 仍使用每次保存对话框选择位置；没有新增上游macOS分享服务、配置固定导出目录、模板自定义、窗口内生成摘要 /编辑笔记。此次完成007台账要求的可选已存内容与导出，不删除三套原支持模板。共享Dialog焦点约定留015。

## 命令与实际结果

```powershell
& tests/reader_pipeline/run_rust.ps1 -Filter m4w_007
node tests/digest_export/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/reader_ownership/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
node tests/entry_pagination/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js"
$env:M4W_DETAIL_WIRE_PATH = Join-Path $env:TEMP 'm4w-019-detail-wire.json'
& tests/reader_pipeline/run_rust.ps1 -Filter m4w_011_tests
node tests/entry_filters/run_dom.mjs "$env:TEMP/mercury4win-reader-tests/node_modules/happy-dom/lib/index.js" "$env:M4W_DETAIL_WIRE_PATH"
Remove-Item Env:M4W_DETAIL_WIRE_PATH
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm.cmd run lint
npm.cmd run build
rustfmt --edition 2021 --check src-tauri/src/digest/templates.rs src-tauri/src/digest/m4w_007_tests.rs src-tauri/src/commands/digest_cmd.rs
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
git diff --check
```

- 本项5项Rust通过：真实内存SQLite /store和Handlebars的单篇Markdown /纯文本（中文 /特殊字符 /原URL /笔记 /已存摘要）、多篇序 /缺项 /作者完整回退链、默认选项 /source旧成功拒绝 /错误输入、旧版本正文拒绝与实际生产Feed源变更清除、三个格式临时中文文件UTF-8无BOM往返 /内容相等 /笔记变更后旧预览拒绝覆盖原文件 /真实目录写失败。Fixture只插入Feed /entry /content /note及success槽，不配置Provider /凭据，成功验证此路径独立于AI配置。
- 本项7组DOM通过，生产DigestDialog /EntryList /ReaderPane及实际TSwrapper，仅替换native IPC transport和保存对话框：已有记录默认 /会话草稿提示与排除 /恶意HTML仅纯文字、精确预览内容与options导出、txt扩展与缺项提示、对话框取消不调用export /对话框错误 /写失败可重试、默认读取错误及预览错误阻止 /恢复、迟到预览 /多篇默认与IPC顺序、列表freeze /列表序与范围清理、Reader入口 /切文关闭 /inert与无AI配置IPC。
- 原001的15项、006的6组、011的6组DOM复测通过，共27项 /组；011四项Rust复测生成实际019详情JSON后再由生产wrapper /ReaderTagging读取。旧001mock因Reader新静态导入缺三个export符号首次编译失败，补“此回归不应调用”抛错stub后原断言全部通过，不用该mock验证007。011第一次未提供必需JSON参数失败，按原README生成 /提供后六组通过。
- cargo check通过（6条既有未用import警告）、lint /build通过，build保留大于500kB的chunk提示；本项3文件定向rustfmt /git diff检查通过。全仓cargo fmt检查实际失败，仍为build.rs、agent /commands /db /feed /lib /reader等已有格式差异，没有重排无关文件。未运行全套Rust，018历史断言问题保留。
- 最初Rust fixture漏`entry_note.updated_at`导致4项准备失败，补必需列后3通过 /1个摘要归属Markdown括号转义断言失败，修正真实语义断言后5项通过；这些失败没有记录为通过。一次补丁定位失败未应用任何文件，重新读取实际内容后应用。

## 桌面验收尚未运行

完整native Tauri /WebView2的真实IPC、系统保存对话框取消 /覆盖 /路径权限、键盘inert /iframe /窗口布局、实际文件打开与完整应用重启、Linux /macOS未验证；DOM对话框被替换为内存函数，不能证明系统对话框行为。没有需要真实AI的验证。

后续隔离桌面步骤：用临时测试数据启动完整桌面；为A保存笔记 /已有summary slot，留会话草稿；从Reader打开单篇窗口核对默认 /草稿提示、Markdown和纯文本、原URL、切文关闭；勾选缺失正文 /笔记 /摘要核对提示。系统保存选择取消、合法中文路径和不可写路径，确认取消无文件、合法内容与预览一致、失败有提示；预览后变更已存笔记再导出，核对拒绝覆盖并刷新。列表逆序勾选B /A，确认Reader /后续页冻结、文件仍A /B顺序；取消和筛选切换清理选择。重新启动核对保存文件独立可读。满足上述用户路径后再关闭007。
