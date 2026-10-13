# 追问历史回读与只读详情展示

## 问题与结果

Quittung（entry 2623）刷新后追问消失，是查词缓存与生成响应固定返回空的 follow_ups，而当前页面只在内存中追加新回答；数据库记录未丢失。本轮保留旧兜底记录 id 1 与用户新回答 id 4，未创建、编辑或删除线上追问。

- 普通/紧凑分析、智能查询知识库命中、流式complete及已有词条重新生成，均读取保存的追问。共享缓存响应与repository投影，词库和短语响应复用投影，候选预览仍不加载历史。
- Web补GET /api/v1/follow-ups?entry_id=ID，复用已有列表handler。Agent API、CLI、Skill、MCP契约不变，已检查同步维护线，无需新增工具。
- 词库详情已有历史展示，本轮改为只读。学习卡片在揭示答案后加载历史；取消旧请求并清空前一词的状态，错误明确显示，不改变学习调度、会话或评分schema。
- 用户补充多条记录的视觉要求后，三处统一“追问与联想”外层大卡片、内部每条独立小卡片，类似深度解析；删除查词页多余套框。词库和学习无追问输入或记录管理按钮，查词保留。

## 验证与部署

- Rust 49项单元测试通过；显式隔离PostgreSQL集成测试通过，新增普通/紧凑/智能/SSE/词库回读覆盖，并断言回读不调用模型。repository投影收敛后重新通过集成测试。
- 前端check为0错误、2项已有SearchPageView警告；最终卡片改动后重新check与Node22 Docker生产构建通过。格式检查和git diff --check通过。
- 公网GET历史和缓存分析返回id 1、4。真实浏览器复现旧版本历史为空；新版刷新后重新查询恢复两条，逐条数量与保存顺序在查词、词库和学习中核对。查词历史有6个管理按钮，另外两处为0且没有发送追问入口。
- PC 1280×900、移动390×844与亮暗主题截图检查通过；学习内容在自身滚动区中逐条可达，未出现文档横向溢出。
- 学习截图使用真实Quittung详情、学习进度和未模拟的追问GET；仅拦截session start选择这个词，未提交评分或改变真实学习队列。测试后撤销拦截。这属于展示验证，未覆盖真实评分提交。
- 前后端已部署。后端官方构建拉取crates索引卡住后，复用现有公共Cargo缓存在Rust1.88中locked/offline构建；正式Dockerfile、依赖与锁文件未改。最终前端镜像正常由Compose构建，MCP保持健康。
- architecture-review结论：结构合理，保持现状。数据读取/投影在repository，分析服务共用缓存响应，组件拥有请求取消生命周期，共享历史组件用只读模式复用渲染，无新增依赖。
- 已检查current-state、active-roadmap及API文档，本轮不改变开发优先级。

## 截图

目录：output/playwright/。

- 修改前：follow-up-reload-search-before.png。
- 最终查词：follow-up-search-group-desktop-light.png、follow-up-search-group-mobile-light.png、follow-up-search-group-mobile-dark.png。
- 最终词库：follow-up-library-group-desktop-light.png、follow-up-library-group-mobile-light.png、follow-up-library-group-mobile-dark.png。
- 最终学习：follow-up-learning-group-desktop-light.png、follow-up-learning-group-mobile-light.png、follow-up-learning-group-mobile-dark.png；mobile-dark-bottom.png记录滚至第二条底部（完整文件名前缀同上）。
- 早期只读版本截图仍保留为follow-up-*-readonly-*及follow-up-reload-search-*，最终大/小卡片以group系列为准。

## 边界

没有自动修复旧兜底答案，id 1的原始截断内容仍保留；需要时可在查词页或CLI/MCP主动处理。刷新不会自动恢复页面内未提交的问题。学习历史加载失败会显示错误，下次揭示/进入时重新读取；本轮未新增追问入口或扩展评分流程。
