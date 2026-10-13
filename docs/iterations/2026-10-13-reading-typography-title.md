# 移动文章字号试用与标题完整度

## 实现与取舍

- 精读需要较松的密度。本轮仅将≤600px正文从1.2rem/1.95（默认19.2px/37.44px）试调为1.125rem/1.9（18px/34.2px），字重、字间距、左右边距、正文上下留白及标记样式保留；PC和中间宽度正文原字号保留。
- 移动标题原1.15rem（18.4px）保留，从单行省略改为最多两行、1.25行高，长词可换行；超出两行仍省略。标题在固定栏始终可见，因此本轮不在正文重复居中标题，以减少首屏重复与占位。
- 按用户要求，PC（>900px）恢复标题上方目录路径，75px最小顶栏；移动继续隐藏路径。folderPath由article.folder_id和folders派生，编辑后自动反映新目录，不恢复额外手动同步状态。
- 固定底栏、独立正文滚动、标记列表和快捷查词逻辑保留。改动集中在ReadingArticleView，无API/CLI/MCP契约或全局样式变化。

## 截图与维护

- npm run check为0错误、2项原有SearchPageView警告，生产镜像构建通过，仅更新frontend容器。
- readingArticleChrome.browser.js通过320/390/1280px双主题及390×540矮屏：移动18px/34.2px、标题18.4px且最多两行，PC1280px正文仍21.12px/42.24px、标题23.2px，目录路径仅PC可见；固定上下栏、长列表滚动、标记跳转、查词入口、栏外关闭与空标记状态均通过。
- architecture-review：结构合理，保持现状；复用目录路径helper及组件内派生状态，没有新增职责或跨页面状态。
- 同一真实文章、同一390/1280×900视口和双主题，改前`output/playwright/reading-typography-before-*`，改后`reading-typography-after-*`；菜单展开为`reading-typography-menu-{before,after}-*`。保留上一轮reading-chrome截图。
- 移动并排对照为`reading-typography-compare-390-{canvas,canvas-dark}.png`，由上述真实浏览器前后截图按原尺寸并排生成，左旧右新。
- 检查current-state、active-roadmap与iterations，更新当前状态及本迭代；无优先级变化。
- 最终移动字号仍待用户实际连续阅读判断，未做实体手机阅读舒适度评估。
