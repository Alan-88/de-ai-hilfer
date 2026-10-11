# 2026-09-18 Agent 学习只读诊断

## 本轮实现

- 新增受保护接口 `GET /api/v1/agent/learning/logs`：
  - 默认查询当前业务日。
  - 支持 `date`、`entry_id`、`session_id`、`limit` 过滤。
  - 返回评分原始记录和按评分、首次反馈、日内反馈、长期提交汇总的统计。
- 新增受保护接口 `GET /api/v1/agent/learning/plan`：
  - 合并长期到期词、新词和未完成的日内恢复卡片。
  - 返回每张卡的来源、阶段、出现次数、连续答对次数、下次复现时间和是否当前可用。
  - 明确标记为 `reconstructible`，只表示可从数据库重建的今日计划，不读取或改变浏览器 session 的内存顺序。
- `tools/deai` 新增：
  - `learning logs`
  - `learning plan`
- `tools/deai learning explain` 新增按需读取的 V3 规则说明，不访问 API、不写库；局部 Skill 与 MiniMax Skill 均提供该入口。
- 局部 Skill 和 MiniMax 便携 Skill 已补充使用说明。

## 已确认的调度语义

- 首答为“忘记”后，后续答对增加 5 分钟冷却就是当前采用的短时稳定机制。
- 不再另行实现一个更长的“沉寂 / 延后验证”规则；后续只根据真实日志评估现有参数。

## 下一阶段写入能力

- 日内状态重置。
- 非首次反馈回退，例如“模糊 -> 认识”后撤销并重新进入日内循环。
- 首次反馈纠正，例如“认识 -> 忘记”；这会影响长期 FSRS，需要保存评分前状态快照或建立追加式纠正记录。
- 当前已打开浏览器 session 的实时队列诊断。
- 上述动作均应追加审计记录，不物理删除原始评分日志；纠正后需要重建受影响卡片，并使打开中的 session 失效或重建。

## 验证

- Rust Agent API 测试通过。
- 学习状态机测试 `8 / 8` 通过。
- `cargo check`、`cargo fmt --check`、`git diff --check` 通过。
- `tools/deai --self-test` 通过。
- 本轮接口为只读，不新增学习日志或学习进度。
