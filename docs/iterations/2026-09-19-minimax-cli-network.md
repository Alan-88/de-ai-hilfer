# MiniMax CLI 网络容错（2026-09-19）

- 对照旧版 CLI、仓库版 CLI 与 MiniMax 包内 CLI，确认 `learning list` 的请求路径未因新增命令改变。
- 连续验证公共 Agent API 的 `learning`、`plan`、`logs` 均可返回 `200`；偶发 `fetch failed` 属于 Node 原生 `fetch` 的网络错误没有被展开或重试。
- 便携 CLI 的只读 GET 请求增加 60 秒超时、最多两次退避重试和底层网络原因输出。
- 写请求不自动重试，避免服务端已接受写入但客户端未收到响应时产生重复操作。
- 包内示例继续以公开地址作为跨环境默认值；若 MiniMax 命令运行时共享 Tailscale/内网路由，可以改用 `de.int.alan-studio.uk`。浏览器可达不等于后台命令运行时可达。
