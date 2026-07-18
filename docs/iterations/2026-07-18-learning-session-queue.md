# 2026-07-18 学习 Session 新词队列收口

## 问题

- `limit_new_words=5` 不是学习引擎 V3 的设计决策，而是旧 V2 接口遗留并在迁移时沿用的每次 session 参数。
- 它不是每日上限；完成一轮后重新启动 session 还会取下一批，但当前 UI 没有表达这层行为。

## 本轮调整

- 从前端、V2/V3 handler、V3 请求模型与学习服务中删除 `limit_new_words`。
- session 现在收入全部到期复习词，以及全部由用户明确加入学习但尚未首次学习的新词。
- 不增加每日上限或设置项；“加入复习”本身就是新词准入闸门。

## 验证

- `cargo fmt --check`、`cargo check` 通过。
- 学习日内状态机测试 `4 / 4` 通过。
- `svelte-check` 0 error，生产构建通过。
- 部署后以无请求体启动 V3 session，当前 4 条到期新词全部进入队列；启动前后 `learning_progress` 规范化快照一致，没有提交评分或写入学习进度。
