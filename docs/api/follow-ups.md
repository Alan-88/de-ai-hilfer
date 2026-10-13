# 追问生成与记录管理

追问记录属于知识条目 `entry_id`，由 `follow_up_id`（响应中为 `id`）标识，与词典 `lexeme_id` 不通用。答案保存 Markdown 原文，保留换行；网页将原始 HTML 转为文本，过滤不安全链接，不执行外部回答中的脚本。

## 内部只读 Agent

- 追问模型实际可调用 `lexeme_query`，复用现有 Agent/MCP 背后的词汇服务。`resolve` 查关联词候选，`inspect` 读取词典词性/释义和已有分析。没有候选时说明缺乏依据；没有已有分析时不初始化词条。
- 仅开放读取，不开放学习、初始化、文章或追问写工具。最多5个模型回合、6次工具调用，每次最多3个词/ID，总时限180秒、每次模型请求90秒，临时网络错误每回合最多重试1次。
- 工具回合保留服务商的续接元数据（例如Gemini thought signature），不丢弃后重构消息；后端日志只记录回合、工具数量与耗时，不记录问答内容或工具参数。
- 默认模型沿用当前查词任务配置及动作级 model_override；没有持久配置时使用 env 的 follow_up/follow_up_pro。模型需支持函数工具，不能调用时明确失败，不降级到无关词典摘要。
- 已取消150字硬约束和本地180字截断兜底。输出预算为2048 tokens（Gemini网关沿用不传max_tokens约定），长度上限、缺少正常结束标识、空输出、超时或工具预算耗尽不写库。
- 默认按“深度解析与避坑”的单条理解型解释快问快答：1–2个短段落、通常2–5句中文，必要时附简短例句/对比例句，讲清判断模型与辨析边界。默认不输出完整报告、多级章节或表格；用户明确要求才展开。这是提示词目标，不截断已生成回答。
- 当前分析上下文最多24000字符，最近3条历史回答各最多4000字符，工具查到的已存分析最多16000字符。仅上下文可节选，回答不硬截断。工具结果和历史内容视为数据。
- 提示词区分语言依据与法律、税务或报销规则；词典不支持后者的绝对断言。工具调用不等于事实保证，仍需按实际回答判断质量。
- 网页仍调用 `/api/v1/follow-ups/stream` 并接收进度/完成/错误事件；工具回合不暴露为答案，验证后完整展示回答，不逐token显示半成品。新追问失败保留输入，已有追问重试失败保留原记录。

## 外部记录管理（Agent Bearer认证）

| 方法与路径（前缀 `/api/v1/agent`） | 请求与结果 |
| --- | --- |
| `GET /follow-ups?entry_id=ID` | `{follow_ups:[...]}`，每项含 id/entry_id/question/answer/created_at |
| `GET /follow-ups/:id` | 一条完整记录 |
| `POST /follow-ups` | `{entry_id,question,answer,idempotency_key}`，201返回记录；直接保存外部答案，不调用AI |
| `PUT /follow-ups/:id` | `{question,answer,expected_question,expected_answer}`；与先前读取的内容一致才更新，否则409 |
| `DELETE /follow-ups/:id` | 删除指定记录，返回 `{deleted:true,id}`；不存在404，无撤销历史 |

问题最多4000字符、回答最多100000字符，均非空；不允许编辑时移动到另一词条。创建必须使用1–128字节幂等键，相同键/内容返回原结果，换内容409；删除后重复旧创建键也不会复活记录。

Web 使用 `GET /api/v1/follow-ups?entry_id=ID` 读取该词条的全部记录，响应 `{follow_ups:[...]}`；`/api/v1/follow-ups/:id` 提供读取/编辑/删除。新生成沿用 `POST /api/v1/follow-ups` 与 `/stream`，参数 `{entry_id,question,quality_mode?,model_override?,retry_id?}`。retry_id必须属于entry_id；成功原位替换，生成期间有其他编辑或删除则拒绝覆盖。历史兜底记录不自动改写或清理。

追问持久保存，普通/紧凑/智能分析与流式complete的最终词条结果均返回保存的 `follow_ups`，刷新后重新查询可恢复。候选预览不加载历史。网页查词结果保留追问入口和记录管理；词库单词详情与学习答案仅展示，均无输入、编辑、重试或删除追问的按钮。学习在揭示答案后独立读取记录，换词、隐藏答案或卸载时取消请求，不改变学习会话/评分契约。三处使用“追问与联想”外层卡片，按保存顺序逐条显示内部问答卡片，不合并或截断答案。

## 异步生成、CLI与MCP

内部生成通过通用批量队列的 `follow_up_generate` 操作：`{entry_id:ID,question,retry_id?:ID,model_override?}`。ID支持前序结果引用；结果为 `{answer,follow_up,model,quality_mode}`，记录ID可引用 `/follow_up/id`。生成写入与任务去重凭据同一事务提交，重启后不重复创建已提交回答。重试沿用同一任务清单；输入改变使用新任务和新键。

```bash
./tools/deai follow-up list ENTRY_ID
./tools/deai follow-up get FOLLOW_UP_ID
./tools/deai follow-up create --stdin --idempotency-key external-answer-1 --yes < answer.json
./tools/deai follow-up edit FOLLOW_UP_ID --stdin --yes < edited-answer.json
./tools/deai follow-up delete FOLLOW_UP_ID --yes
./tools/deai follow-up generate --stdin --idempotency-key question-1 --yes < question.json
./tools/deai follow-up retry FOLLOW_UP_ID --idempotency-key retry-1 --yes
./tools/deai batch get TASK_ID
```

answer.json包含entry_id/question/answer；edited-answer.json包含question/answer/expected_question/expected_answer；question.json包含entry_id/question。生成及重试支持 `--provider NAME --model ID`。CLI未提供键时生成UUID，响应丢失后不能猜该键，建议显式指定；写请求不自动网络重发。

MCP 0.4新增3个工具：

- `follow_up_query(action=list/get)`：只读。
- `follow_up_write(action=create/edit/delete)`：外部答案管理；create必须给幂等键，edit必须给读取时的原文。
- `follow_up_generate(action=create/retry,idempotency_key,...)`：立即返回任务ID，用batch_query轮询；retry只需follow_up_id，会读取原问题。待完成状态不是生成成功。

通过 `deai_guide(topic=follow_ups)` 读取说明。客户端需刷新工具发现。外部Agent可以只用只读查询获取依据、在外部生成，然后保存回答，无需内部模型调用。编辑/删除暂为指定记录的同步操作，未加入批量operation清单。
