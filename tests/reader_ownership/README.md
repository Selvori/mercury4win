# M4W-001 Reader 归属回归

回归使用生产 `ReaderPane`、笔记 / 摘要 / 翻译 / 标签组件、React StrictMode、React Query 和 Zustand。IPC 使用可延迟完成的内存替身；Channel、i18next 和字体设置对话框为测试替身。不会打开真实数据库、读取凭据或请求 AI。

## 执行

从仓库根目录运行，使用已有 esbuild 依赖。可直接在浏览器中执行，无需额外依赖：

```powershell
node tests/reader_ownership/serve.mjs
```

打开输出的 `127.0.0.1` URL；页面和控制台输出结果。结束后 Ctrl+C 停止服务。这验证 React 组件归属行为，不构成 Tauri IPC 或 WebView2 桌面验收。

没有可用浏览器时，可以把 Happy DOM 单独安装到临时目录后运行。安装源为 npm 官方仓库 `https://registry.npmjs.org/`，本次固定版本为 `20.14.6`；不要把它装入本项目或修改锁文件：

```powershell
$task_dom_dir = Join-Path $env:TEMP 'mercury4win-reader-tests'
npm.cmd install --prefix "$task_dom_dir" --no-save --package-lock=false happy-dom@20.14.6
node tests/reader_ownership/run_dom.mjs "$task_dom_dir/node_modules/happy-dom/lib/index.js"
```

成功时输出含 15 项 `passed` 的 JSON，进程返回 0；断言失败输出 `error` 并返回 1。DOM 运行时使用 Node 的 MessageChannel 支持 React `act`，运行后关闭任务和消息端口；20 秒超时会报错。

## 2026-10-12 实测

Windows、PowerShell 7.6.5、Node v24.12.0、Happy DOM 20.14.6。代码基线为 HEAD `c90567bd1f0f93b0a7f6e4cc00a19b24520ac2de` 加 M4W-001 未提交改动。执行上述 DOM 命令，15 项全部通过：

1. A 草稿→B 编辑 / 保存：请求使用 B 的 ID 和文本，B 的延迟完成不关闭 A 编辑器或修改 A 笔记。
2. 关闭 / 重开笔记面板、卸载 / 重建 Reader 后按文章恢复草稿。
3. 保存期间继续编辑：旧完成不能清除新草稿；取消只丢弃当前文章草稿。
4. 保存期间关闭重开不能重复提交；文本先改变再改回相同内容仍受修订号保护。
5. 保存失败保留草稿、显示错误，重开后仍可见；成功重试清理对应草稿 / 错误。
6. 摘要 A→B：旧 chunk / done / Promise 错误不改变 B 的内容、错误和 loading。
7. 摘要关闭重开：旧错误回调不影响新请求。
8. 翻译 A→B：译文 / 进度重置，旧 segment / progress / error / done / Promise 错误被隔离。
9. 同文章重新开始翻译：旧请求事件 / 失败不影响新请求。
10. 翻译关闭重开：旧流不影响新请求。
11. 标签 A→B：AI 建议和手工输入不跨文章，B loading 不被旧成功解除。
12. 标签关闭重开：旧 Promise 失败不影响新请求。
13. 延迟添加标签使用原文章 ID，失效原文章缓存，不能删除新文章建议。
14. 延迟删除标签使用原文章 / 标签 ID，只失效原文章详情缓存。
15. 切换 / 关闭展示不使后台请求自动结束：展示隔离与取消是不同操作。

浏览器连接工具实测没有可用浏览器，故未执行浏览器版。未验证 WebView2 布局、真实 Tauri IPC、真实 AI、macOS / Linux。桌面关闭条件仍需在隔离应用数据上完成 A/B 切换、面板关闭 / 重开、保存失败 / 重试和延迟 AI 回调验收。

笔记草稿仅保留在本次应用会话内；关闭应用 / 重启后不恢复未保存草稿。已提交保存继续由后端持久化，切换文章 / 关闭面板不取消保存或 AI 请求。AI 的执行取消和持久化分别归属 M4W-003 / M4W-002。
