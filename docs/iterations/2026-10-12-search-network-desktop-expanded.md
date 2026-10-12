# 查词 PC 词汇网络默认展开

- 用户确认顶部紧凑比例可保留，但 PC 词族不应逐条点击才能阅读解释。
- 查词结果词汇网络在 >600px 时默认展开词族、近义、反义条目的已有词性及中英文解释；≤600px 保留默认折叠和键盘/触控切换。沿用现有 details，不增加新组件或状态管理。
- 只改变展开初态，不调整字号、间距、搜索栏或其他页面。检查 active-roadmap，无优先级变化；无 API/CLI/MCP 契约变化。
- 改前/改后真实浏览器截图保存于 output/playwright/search-network-{before,after}-{390,1440}-{canvas,canvas-dark}.png。
- npm run check：0 错误、2 项原有警告；生产镜像构建成功，仅更新 frontend 容器。
- 公网 Chromium 双主题验证 390/1440px 初态、词条解释可见性、键盘切换，以及 600/601px 和桌面/移动宽度往返切换，无横向溢出。复用只读取得的真实 abnehmen 条目，分析响应在浏览器内替换，其余写请求阻断。
- architecture-review：保持现状；窗口宽度使用 Svelte 自带响应式值，展示初态由原结果组件负责，无新依赖或跨页面状态。未在实体手机/Safari 复测。
