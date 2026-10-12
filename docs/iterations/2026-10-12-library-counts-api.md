# 词库四类数量统一输出

原来分类栏仅“全部”和“学习中”显示数量，分别来自未搜索时的全部列表总数和前端学习进度缓存。此次统一使用后端词库分页响应，补齐待复习、新词，避免统计口径与筛选结果不一致。

## 接口与语义

沿用 `GET /api/v1/entries?q=...&tab=all|learning|review|new&limit=...&cursor=...`，新增响应字段：

```json
{
  "counts": { "all": 2192, "learning": 71, "review": 9, "new": 2121 }
}
```

- `all`：符合q的全部知识词条，不是原始词典全部记录。
- `learning`：符合q且存在learning_progress的词条。
- `review`：符合q且已经到期（due_date ≤ 当前请求时间）的学习词条；未搜索时与learning/stats的due_today语义一致。包含到期但尚未首学的计划词；不等于包含日内重现的V3背词会话总数，也不是“今日已学习数”。
- `new`：符合q且没有learning_progress的词条，直接从数据库统计，不由前端相减。

counts不受当前tab、limit、cursor影响；搜索条件q影响四项。当前标签的total直接取counts对应字段。聚合统计和列表使用同一个分类条件函数、同一个请求时间，避免跨查询的到期时间边界差异。仍为两条SQL查询，保留原有分页方式；统计和列表之间发生并发业务写入时仍可能暂时存在差异，没有增加事务快照或实时推送。

前端统一消费counts，初次未取得统计时显示“—”。加入学习、删除词条后重新加载分页及统计；保留移动上下排布、桌面横排。

## 验证与维护

- 隔离PostgreSQL容器中运行backend/tests/library_counts.rs：分类成员与数量一致、到期/未来到期、德语变音归一化搜索、空结果、绑定参数及分页。临时表不接触线上数据，测试容器已清理。
- cargo查询服务测试、npm run check（0错误、2项既有警告）、前后端生产镜像构建通过。
- frontend-web/tests/libraryCounts.browser.js在公网验证四标签与counts/total一致、无搜索时与学习统计一致、搜索/空结果、标签切换、320/390/1280px与canvas/canvas-dark。
- 线上核对结果：2192 / 71 / 9 / 2121；截图为output/playwright/rich-responsive-references/library-four-counts-*，包含before和各断点、主题截图。
- architecture-review：结构合理。查询仓储拥有筛选和统计，服务映射响应，页面拥有展示与刷新；共用既有接口，无新路由、store或数据库迁移。保持现状。
- 已检查current-state及active-roadmap，更新当前状态，无优先级变化。已核对CLI/MCP调用链，此次仅扩充Web词库列表响应，不改变Agent能力或工具契约。
- 已部署前后端容器，后端重启会清空内存背词会话，持久化学习进度与日志保留。实体手机尚未复测。
