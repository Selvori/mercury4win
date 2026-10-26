# 文章工具栏标题宽度验证

2026-10-26；基线 `4b652fb` 加任务开始前全部未提交改动与本项 ReaderPane 布局增量。Windows / PowerShell 7.6.7 / Node 24.12.0 / Edge 156.0.4314.42。M4W-023 待完整桌面验证。

用户截图中窄窗口右侧操作消失、宽窗口标题仍过早省略。静态确认标题 flex 容器缺少 `min-w-0`，标题又有 `max-w-md`。修复仅调整 ReaderPane 的宽度 / 收缩类，并为标题增加完整文字 tooltip。

实际执行：

```powershell
npm.cmd run lint
npm.cmd run build
node tmp/reader-toolbar-layout/check.mjs
git diff --check
```

lint、build、diff 通过；build 保留已有大 chunk 提示。一次性浏览器检查位于忽略目录，复用临时 playwright-core，运行生产 ReaderPane / 构建 CSS / i18next / QueryClient；仅 native transport 与浏览器打开调用为合成 mock，所有非 loopback 网络被拦截。未改项目依赖。

实际通过：

- 中英文各 400 / 600 / 924 / 1181 / 1701px，共十种阅读区宽度，全部八个按钮边界完整处于工具栏内、中心点可命中，工具栏无横向溢出。
- 长英文标题在 400px 时省略，1701px 时完整；中文界面标题可用宽度由 46px 增至 1347px，英文由 16px 增至 1317px。
- 800px 打开笔记侧栏后主阅读区为 474px，工具栏无横向溢出。阅读设置窗口打开 / Escape 关闭、浏览器 opener mock 接收文章 URL、文摘窗口打开通过。
- 窄 / 宽截图已查看，浏览器与服务正常关闭。

本地证据：`tmp/reader-toolbar-layout/result.json`、`400.png`、`1701.png`。首次临时文摘 mock 缺少 `missing` 数组，导致文摘打开断言超时；补齐 mock 后同命令通过，产品代码无需额外修改。

没有验证完整 Tauri / WebView2、实际系统浏览器、文件保存导出、其他平台；宽度指阅读区自身，不等同整个三栏应用窗口。未重跑 Rust / 全套回归，未访问用户数据库或凭据。
