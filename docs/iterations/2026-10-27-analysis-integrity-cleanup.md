# 2026-10-27：解析完整性检查与恢复链路清理

## 问题与边界

`gar` 已保存完整 IPA、五条释义和两个语法分支，但前端因正文出现“词源”、标题不含“词源”而弃用整份结构。现代分阶段 Markdown 只保存学习内容模块，整份回退无法恢复 Model A 的核心数据。

本轮保留完整性检查，统一采用“保留有效结构，只恢复缺失部分”。未重新生成词条、修改数据库或猜补语法属性。CLI/MCP 共用后端生成链路，工具 schema 无变化。

## 清理结果

- **前端共享解析器**：查词、词库详情、学习答案、文章快捷查词统一受益。删除全局稀疏判断及词源关键词规则；按模块恢复缺项，同标题用法只补空字段；模块自带例句不再重复添加为独立例句。仅语法分支或词汇网络有内容的结构也保持有效。
- **Markdown 边界**：嵌套列表保留在正文；不再把中文标题冒号后半段强拆成正文；语法表按表头和分隔行识别，不过滤所有含“特征”的数据行。词汇网络仅识别真正的分组标题，释义中提到“同义词”不再吞掉词条。
- **后端覆盖检查**：复用原文预解析检查用法、解析条目和各词汇关系组的完整数量；空正文不计作已覆盖。删除词源标题特判和“有任意搭配就算覆盖语法表行”的旁路。同标题用法只补空字段，完整模块不因标题表达差异额外追加副本。
- **词汇网络去重**：仅合并完整字段相同的重复项，保留 `duzen/Duzen`、同词不同词性和不同义项。
- **生成边界**：必需 section 必须有内容，并存在完整用法块与解析条目；去掉 800 字节门槛和句尾标点猜测。普通响应检查已返回的 `finish_reason`；流式响应检查完成状态或 `[DONE]`，中断不得当作成功。未提供完成原因的普通响应保留提供商兼容。
- **JSON 边界**：删除裁尾、补引号/括号以抢救截断 JSON 的流程及无用报告类型。保留完整 JSON 的内部引号转义兼容并记录日志。数组字段允许约定的空值与可解析的单项，异常非空类型明确报错，不再静默变为空数组。
- **语法空值**：仍按 Model A 约定允许空字符串/数组；前端按词性字段显示 `-` 或“无特殊语法特征”，不据此拒绝整词。

## 全库只读验证

通过 GET API 抓取固定快照；6 路有界并发，失败请求补齐后共 **2,237 / 2,237** 条。后续对比使用同一快照，避免生成或数据变动干扰。

| 检查 | 结果 |
| --- | --- |
| 前端完整输出逐字段完全相同 | 2,190 条 |
| 恢复旧判断误弃的结构 | 41 条，含 gar、heißen、Tafel |
| 恢复额外词汇网络漏项 | 6 条：du、Film、Fernseher、Bescheid、braun、Schnee |
| 原有有效字段丢失 | 0 |
| 后端恢复流程回放保持原样 | 2,231 条 |
| 后端回放变化 | 仅上述 6 条的 word_network |
| 原文结构错误 / 恢复后仍未通过检查 | 0 / 0 |

脚本可重复运行，不连接数据库、不调用生成接口：

```sh
cd frontend-web
node scripts/structured_analysis_regression.mjs
node scripts/audit_structured_entries.mjs capture ../output/analysis-integrity-baseline.json
# 修改解析器后，使用同一快照对比
node scripts/audit_structured_entries.mjs compare ../output/analysis-integrity-baseline.json ../output/analysis-integrity-comparison.json
cd ..
SQLX_OFFLINE=true cargo run --manifest-path backend/Cargo.toml --bin audit_structured_entries -- --snapshot output/analysis-integrity-baseline.json
```

快照、完整差异报告、后端报告与测试输出保存在 `output/analysis-integrity-*`，不纳入 Git。

## 验证与部署

- 前端针对性回归、Svelte 检查与构建通过；Rust 单元测试 59 通过、1 项隔离数据库测试按原约定跳过。覆盖真实缺失、空语法、完整数据不变、词汇网络不同义项、嵌套列表、异常类型、截断响应和 JSON 拒绝。
- Svelte 原有两项警告（初始 props 捕获、line-clamp）保留。隔离数据库批量任务集成测试未运行；本轮不修改写入或调度逻辑。
- 前后端容器已重新构建并部署。公网查词与词库详情中，gar 的两组释义和语法浮层均恢复，比较级/最高级显示 `-`；1280px 亮色、390px 亮/暗主题验证通过，移动端无横向溢出，浏览器无错误。部署前后 gar API 完整响应相同。浏览器原有页面缓存曾显示旧资源，重新加载带版本查询参数的页面后确认使用新构建。
- 未额外付费重跑真实模型生成；完整性检查不替代语言事实正确性校验。
- 架构复核：保持现有边界。生成状态在 AI 传输层、原文预解析及覆盖检查在服务层、页面恢复在前端共享解析器，页面组件不添加单词或关键词例外。

## 截图

- 改前：`output/playwright/gar-integrity-before.png`、`normal-abnehmen-integrity-before.png`。
- 改后 PC：`output/playwright/gar-integrity-after-desktop-canvas.png`；词库语法浮层：`gar-integrity-library-desktop-grammar.png`。
- 改后移动端：`output/playwright/gar-integrity-after-mobile-canvas.png`、`gar-integrity-after-mobile-dark.png`；词库语法浮层：`gar-integrity-library-mobile-dark-grammar.png`。
- 正常词条对照：`output/playwright/normal-abnehmen-integrity-before.png` / `normal-abnehmen-integrity-after.png`。
