# 查词最近查询文字主题适配

- 最近查询按钮未设置文字颜色，词头继承浏览器按钮默认色，在 canvas-dark 下显示为黑色。
- 在 SearchResultCards 的按钮样式中增加 color: var(--text-main)，简介继续使用 --text-muted。不改全局按钮、布局或交互。
- 检查 current-state、active-roadmap 与迭代文档；更新当前状态，无优先级或 API/CLI/MCP 契约变化。
- 改前深色截图可见 output/playwright/search-network-after-1440-canvas-dark.png；改后截图为 search-recent-theme-{390,1440}-{canvas,canvas-dark}.png。
- 类型检查 0 错误、2 项原有警告，生产镜像构建通过，仅更新 frontend 容器。公网 Chromium 在 390/1440px 双主题验证全部最近查询词头的计算颜色与 --text-main 一致；亮色 rgb(52,46,85)，暗色 rgb(234,229,239)。没有写入业务数据。
- 结构复查：颜色留在对应组件内，复用现有主题 token，无新状态、依赖或全局覆盖。未在实体手机/Safari 复测。
