# 移动词库紧凑卡片落地

## 实现

- 用户确认浏览器预览，移动端采用紧凑卡片，PC保持当前密度。
- 仅≤600px调整LibraryEntryCard本地样式：24px内距改14px，内容上下间隔24px改10px，词头22.4px改18px/1.3，简介14.72px改13px/1.5。状态10px、3px/6px内距；元数据11px/6px间隔；底行8px间隔及上内距，详情按钮12px/6px/9px内距。
- LibraryPageView同断点网格由300px最小列宽改为minmax(0,1fr)，间隔24px改12px；320px屏的288px内容区不再溢出。组件控制卡片密度，页面控制列宽/卡片间隔，未增加全局样式或运行时状态。
- 保留卡片/按钮背景与圆角、字重、右上状态独立列和不换行规则、标题/简介最多两行、日期/稳定性、加入学习及详情操作；未改变释义简介接口或切分逻辑。>600px样式保持原值。

## 截图与维护

- 改前真实截图沿用`output/playwright/library-density-before-{320,390}-{canvas,canvas-dark}.png`；设计预览为`library-density-preview-*`，本轮上线常态为`library-density-release-after-{320,390,1280}-*`，学习状态为`library-density-release-learning-*`，避免混淆预览和发布。
- 检查current-state、active-roadmap和iterations；更新当前状态与本记录，无优先级变化。
- 未在实体手机评估缩小后的触控手感；浏览器核查不提交学习或删除数据。

## 验证与发布

- `npm run check`：0错误，保留SearchPageView已有的2条警告；`docker compose build frontend`成功，仅更新frontend容器，未重启backend。
- 公开页面真实浏览器检查320/390/600/601/1280px及canvas/canvas-dark：移动尺寸符合预览，网格与状态标签不溢出，长词/简介保留两行省略，详情可打开/关闭，>600px词头22.4px、内距24px保持原值。
- 等待“学习中”筛选实际加载后重新核查320/390px两主题，日期、稳定性和详情按钮均不溢出；1280px两主题的24张学习卡片无卡片溢出或按钮重叠。
- 桌面旧布局中brauchen/buchen的元数据宽度分别超过分配宽度约3px/5px，占用部分与按钮的16px间隔，但未覆盖按钮。本次新增样式仅在≤600px生效，保留桌面密度，此问题留待桌面卡片调整时处理。
- 架构检查：卡片组件控制自身密度，页面组件控制网格，职责保持清晰；未改变接口、CLI/MCP、共享状态或依赖。结构化释义简介的独立后续项保持原优先级。
