# 2026-09-18 学习状态纠正

## 本轮实现

- 新增当前业务日的 Agent / CLI 操作：
  - `learning reset-intraday <entry-id> --yes`
  - `learning undo <entry-id> --yes`
  - `learning correct-first <entry-id> --rating known|fuzzy|forgotten --yes`
- `reset-intraday` 保留首答和长期 FSRS，只让后续日内反馈重新进入可重建状态。
- `undo` 只回退最新一条有效的非首答反馈。
- `correct-first` 使用首答前快照重新计算 FSRS，并在事务中更新进度、追加纠正记录；当前业务日的打开 session 会失效。
- 原始 `learning_review_logs` 永不删除或改写。旧日志没有快照时，首次纠正会明确失败而不是猜测。
- `learning explain`、仓库 Skill 和便携 Skill 已同步这些边界。

## 未做

- Agent 仍不读取浏览器 session 的实时内存顺序；`learning plan` 只是从数据库重建的今日视图。
- 调度参数暂不开放 CLI 修改，继续先积累真实日志。

## 验证

- `cargo fmt --check`
- `cargo check`
- `cargo test services::learning_session::tests`
- `cargo test services::agent_api::tests`
- `node tools/deai --self-test`
