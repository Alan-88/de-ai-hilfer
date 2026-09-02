# 2026-09-02 Architecture Consolidation

## 目标

复查历史迭代中缺少职责证据的拆分，纠正纯转发、固定参数和已经失去原职责的模块。判断依据是状态、生命周期、复用方、算法边界和依赖方向，不以文件数量为目标。

## 安全基线

- 原始提交：`5fbbdcb`
- 回退分支：`codex/pre-architecture-consolidation-20260902`
- 工作分支：`codex/architecture-consolidation`
- 每组低风险调整独立提交并完成对应构建检查。

## 已完成

1. 将 grounded chat options 归还 generation/runtime，删除 `analysis_grounded_options.rs`。
2. 将流式结果类型和 analyze meta helper 归还实际所有者，删除 `stream_analyze_runtime.rs`。
3. 将结构化 prompt fallback 并入 `analysis_grounded_prompt.rs`，删除 `analysis_structure_prompt.rs`。
4. 将环境分析模型映射归入 `ai_model_resolver.rs`，删除 `analyze_runtime.rs`。
5. 删除 6 个没有入口的旧前端文件：旧学习卡、旧追问、旧 Markdown/评分辅助链、旧词库详情和只为旧学习卡服务的增强器。

## 保留边界

- `analysis_grounded_generation` 同时服务 runtime 与 backfill，是实际复用边界。
- `analysis_grounded_ab_cache` 独立拥有 TTL、互斥状态、保留/删除生命周期和测试。
- Model C retry、quality、seed、transform 分别承载可独立验证的策略或转换算法。
- `query_suggestions`、`dictionary_merge`、`prewarm_selection` 虽由历史结构调整产生，但目前各自拥有完整算法和明确调用边界。
- 前端剩余组件均有活跃入口；SvelteKit 的薄 route 文件属于框架入口，不参与合并。

## 后续审查点

- `analyze.rs` 与 `analyze_stream.rs` 重复候选解析、缓存读取和持久化流程。
- `follow_up.rs` 与 `follow_up_stream.rs` 重复上下文准备、模型 fallback 和落库流程。
- 两组都涉及流取消和错误语义，不能用机械拼文件处理；需要先提炼行为测试，再决定共享核心还是保持重复。

## 验证

- `cargo fmt --all -- --check`
- `cargo test --lib`：34 passed
- `cargo check --all-targets`
- `npm run check`：0 errors，保留 5 条既有 warnings
- `npm run build`
