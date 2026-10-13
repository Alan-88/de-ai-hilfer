# 通用异步批量操作

本接口接收确定的业务操作清单，不限定文章场景。Agent先只读解析候选和核对状态，再提交用户已经确认的操作；执行器不自动猜词、改书签或追加操作。

## 提交、查看与重试

- `POST /api/v1/agent/batches`：返回202与任务对象，立即结束请求，不等待生成。必须提供`idempotency_key`，相同键与相同有效请求返回同一任务，换内容会返回409。
- `GET /api/v1/agent/batches/:id`：查看逐操作、逐词状态与结果；`GET /api/v1/agent/batches?limit=20`查看近期任务。查询只读，建议每3–5秒轮询。
- `POST /api/v1/agent/batches/:id/retry`：保持原清单，重试未成功操作，成功操作与成功初始化项不重跑；活跃任务返回409。修改输入需新清单与新键。
- 所有上述接口沿用Agent Bearer认证；MCP仍只转发HTTP API，不访问数据库。

任务状态：`queued`、`running`、`succeeded`、`completed_with_errors`。操作状态：`pending`、`running`、`succeeded`、`failed`、`skipped`。只有终态与实际结果可证明完成；已排队不等于已生成。

每批最多100个操作、有效JSON不超过1MB；操作之间严格按顺序执行。当前最多3批并行，一个初始化操作内最多100词、最多3个词并发，整个后端进程的生成并发同样限制为3，Web流式查词与Agent初始化共用限额。同词按原型合并等待并在落库前重新查缓存。

## 操作结构

每项为`{id, depends_on?, operation}`，`id`唯一、1–64位ASCII字母/数字/下划线/连字符。`depends_on`只能指向之前的操作，要求其整项成功。

| operation.action | 参数 | 结果 |
| --- | --- | --- |
| `lexeme_initialize` | `lexeme_ids: number[]`，可选`model_override: {provider_name,model_id}` | `results: [{input,ok,value?:{entry_id,query_text,source},error?}]`；处理中项有`status: pending/running` |
| `learning_membership` | `mutation: add/remove/reset`，`target_type: entry/lexeme`（默认entry），`ids: ID[]` | 逐项`results`，成功项含`entry_id/status`；add保留已有进度，remove保留知识与历史日志，reset重置长期卡到New |
| `folder_create` | `name`，可选`parent_id: ID/null` | `folder_id/name/parent_id`；已有同名目录不会自动被当作本次创建成功 |
| `article_import` | `title/body`，可选`folder_id: ID/null` | `article_id/title/folder_id`；复用原子书签验证，失败不写文章 |

`ID`为正整数或`{step:"earlier-operation-id",path:"/json/pointer"}`。引用成功结果中的正整数；不能前向引用。部分失败的初始化操作中，成功项的`/results/0/value/entry_id`仍可引用；失败项引用缺失会跳过当前操作。显式`depends_on`则要求整项成功。`on_error`默认`continue`（独立后续项继续），`stop`会在失败后跳过全部后续项。整批不是全有全无的事务，不回滚已成功操作。

```json
{
  "idempotency_key": "owner-approved-plan-20261013-1",
  "on_error": "continue",
  "operations": [
    {"id":"words","operation":{"action":"lexeme_initialize","lexeme_ids":[123,456]}},
    {"id":"learn","operation":{"action":"learning_membership","mutation":"add","ids":[{"step":"words","path":"/results/0/value/entry_id"}]}},
    {"id":"folder","operation":{"action":"folder_create","name":"B1"}},
    {"id":"article","depends_on":["words"],"operation":{"action":"article_import","title":"Reisen","body":"Ich plane meine [Reisen](Reise).","folder_id":{"step":"folder","path":"/folder_id"}}}
  ]
}
```

示例数字必须替换为只读接口返回且已确认的实际ID；词与文章并无强制绑定，任意支持的操作可单独提交或组合。当前不接受任意HTTP请求、代码、删除文章或日内评分纠正；新业务能力按同一operation契约扩展。

## 恢复与边界

- 任务、结果和原始清单持久化在PostgreSQL；启动后自动恢复queued/running任务，执行期间持有任务锁，防止重复执行。
- 创建目录/文章以及学习变更与操作凭据在同一事务提交。崩溃发生在业务写入之后也不会重复创建或再次reset。相同内容在不同新任务下仍是不同操作，不做全文去重。
- A、B、C分别保存通过现有校验的阶段结果，24小时有效，包含词典、prompt、模型路由和质量配置身份；B失败复用A，C失败复用A+B，落库失败复用完整C。改配置或过期后重新生成；Web强制重新生成绕过缓存。
- A临时网络错误最多重试2次，B/C沿用已有有限重试；永久错误、无可靠候选、质量校验失败明确失败，不进行无界重试。
- 单词生成并发和同词锁是单后端进程范围，批量任务锁跨进程；多副本部署前需要统一模型配额和生成锁。
- 任务与操作凭据是运行数据，不进入用户内容备份；全量恢复拒绝queued/running任务，恢复时清除旧任务、凭据和阶段缓存，避免旧ID指向新内容。
- 批量入口减少写调用次数，不能修改Spark的权限审批策略。MCP的`batch_submit/batch_retry`如实标记写操作；`batch_query`标记只读。

## CLI与MCP

```bash
./tools/deai entry initialize 123 456 --async --idempotency-key confirmed-words --yes
./tools/deai batch submit --stdin --yes < approved-plan.json
./tools/deai batch get TASK_ID
./tools/deai batch list
./tools/deai batch retry TASK_ID --yes
```

CLI不加`--async`时保留同步初始化兼容性。MCP 0.3的`lexeme_initialize`沿用参数并新增可选`idempotency_key`，响应改为任务对象；不得按旧的完成词条响应解释。推荐通用清单用`batch_submit`，再用`batch_query`；`deai_guide(topic=batch_operations)`提供按需说明。

隔离数据库集成测试仅在显式提供`DEAI_BATCH_TEST_DATABASE_URL`并指定`deai_test`数据库时运行，会清理该测试数据库中的业务测试数据：

```bash
DEAI_BATCH_TEST_DATABASE_URL=postgres://postgres:TEST_PASSWORD@127.0.0.1:55439/deai_test \
  cargo test --lib durable_batches_and_generation_checkpoints -- --ignored
```

进程重启/API测试需先`cargo build --bin server`，以名为
`deai-batch-tests-20261013`的隔离测试容器提供上面的数据库，然后从仓库根运行：

```bash
DEAI_BATCH_TEST_DATABASE_URL=postgres://postgres:TEST_PASSWORD@127.0.0.1:55439/deai_test \
  node --test tools/deai-batch-restart.test.mjs
```
