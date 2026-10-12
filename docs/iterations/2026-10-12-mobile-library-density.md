# 词库详情移动密度试点

将前几轮真实浏览器密度草案落地，只修改 LibraryEntryDetailPanel 的组件样式，不改共用 LearningAnalysisContent、接口或学习状态。

- ≤600px：中文核心短释义 13px、行高20px；长解释与德语例句15px，例句中文翻译13.5px。字重保留。
- 英文释义明确为12px，消除原先受 button 默认字号影响而约11.11px的不确定值。
- 详情标题28px；外层卡片内边距12px，嵌套说明8px，例句10px。保留两套主题的背景、边框和例句左侧强调线；关闭按钮保留44px点击区域。
- ≤360px：标题和学习操作分行，词性标签单独占一行，避免长词或词性标签挤压主要内容。
- 桌面保持原排版；学习页、文章查词栏和普通查词页暂不迁移。路线图已检查，无优先级变化。

验证：npm run check、生产构建；公开页面 abnehmen 的320/390/1280px、canvas/canvas-dark截图，核对释义13px/20px、桌面16.96px、无横向溢出以及改前改后分析全文一致。类型检查保留两项已有warning。仅重建前端容器，无业务数据写入。

截图在本机 output/playwright/rich-responsive-references/release-before-* 和 release-after-*，不提交图片到仓库。真实手机阅读舒适度待用户体验。

结构复核：样式保留在词库详情组件，以现有 detail-analysis 容器限定共享内容样式；没有增加状态、参数或抽象。结论：结构合理，保持现状。
