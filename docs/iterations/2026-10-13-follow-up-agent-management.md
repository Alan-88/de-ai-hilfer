# 追问只读 Agent 与记录管理

## 问题与结果

Quittung（entry 2623）的追问 id 1 并非前端裁切：原模型请求20秒超时后，服务端静默写入截到180字符的词典摘要。模板要求查询关联词，运行时却禁止工具，造成行为矛盾。

追问模型现运行有界只读工具循环，提供 lexeme_query 的 resolve/inspect，复用现有 Agent/MCP 后端词汇服务，不经公网绕行或复制词典业务。最多5回合、6次查询，每次最多3个词/ID；总时限180秒，单请求90秒。没有初始化或写工具。临时网络错误每回合最多重试一次；没有伪回答兜底，空输出、长度截断或未正常结束不落库。

保留回答原文与 Markdown 换行，取消150字限制。上下文可以节选，回答不硬截断。新增“词典语言依据不等于法律/税务依据”的提示约束，避免凭词义推导绝对规则。

用户补充明确追问属于快问快答，风格与篇幅对齐“深度解析与避坑”的单条。按现有Stage2理解型解析原则，默认1–2个短段落、通常2–5句中文，说明用法原因、判断模型与辨析边界，必要时短例句；不写寒暄、多级报告、表格或无关延伸。用户明确要求时可展开，回答不硬截断。

## 使用入口

- 网页查词与词库共用追问历史组件，可编辑问题/回答、确认删除、原位重试。失败保留原回答；编辑采用读取内容对比，生成期间被编辑/删除时拒绝覆盖。
- Agent API、仓库/便携CLI、两套Skill、MCP指南同步。MCP 0.4新增 follow_up_query/write/generate，总计14工具；外部Agent可直接创建或编辑Markdown回答，不强制内部模型生成。
- 内部生成接通用批量队列，提交立即返回任务ID，用只读查询轮询。生成结果和去重凭据同事务提交，恢复不重复创建已提交记录。
- 具体schema、幂等、ID含义与CLI命令见 `docs/api/follow-ups.md`，批量契约见 `docs/api/agent-batches.md`。

## 验证与部署

- Rust原48项单元测试通过；新增模型续接元数据保留测试通过，确保Gemini工具thought signature及服务商消息字段往返不丢失。隔离PostgreSQL与模拟模型的显式集成测试通过，覆盖resolve→inspect→完整回答、截断拒绝、网络重试、轮数上限、失败不写库、原位重试冲突、幂等及编辑删除。
- 独立HTTP后端集成测试通过，覆盖认证、创建去重、改键内容与旧编辑409、失败重试保留原回答、删除后404。
- MCP构建及9项测试、CLI批量/追问转发测试、仓库/便携CLI自检、Markdown安全渲染测试通过；Skill格式验证通过。前端检查0错误、2项原有警告，生产构建通过。
- Docker Rust1.88、Node22镜像完成构建并部署前后端/MCP。镜像源网络超时后，后端使用现有公共Cargo缓存离线构建，正式Dockerfile和依赖锁均未改动。MCP重建需使用 `--env-file mcp-gateway/.env`；遗漏导致启动失败已纠正，网关恢复healthy。
- 公网OAuth+PKCE、14工具发现、follow_up_query真实读取与follow_ups指南通过。真实gemini-3-flash-preview生成独立Quittung测试追问约8秒完成，回答完整；并非仅依靠模拟测试。原id 1未编辑或重试；测试id 2在截图后删除，回答证据保留在截图与任务结果中。
- 快问快答版本首轮实际请求达180秒总时限，任务明确失败且未保存回答。补充回合/工具数量/耗时日志用于排查，日志不含问题、答案或工具参数；内容篇幅与服务商响应延迟分别验证。
- 保留续接元数据后重试同任务成功，真实日志确认3个模型回合（工具调用1+1+0），总计约8秒，回答为两个短段落加一组对比例句。未根据缺少日志的前次超时断言具体原因；模型服务仍可能超时。临时测试id 3截图后删除，原id 1保留。
- architecture-review结论：结构合理，保持现状。模型通信在AI客户端，追问服务拥有生成/事务生命周期，SQL在repository，网页共享记录组件，CLI/MCP只消费后端契约。没有新增依赖或独立Agent框架。

## 截图

真实浏览器，390×844与1280×900；编辑和删除仅打开后取消，未改原记录。

- 修改前：`output/playwright/quittung-follow-up-mobile.png`
- 移动亮/暗：`quittung-follow-up-controls-mobile-light.png`、`quittung-follow-up-controls-mobile-dark.png`
- PC亮/暗：`quittung-follow-up-controls-desktop-light.png`、`quittung-follow-up-controls-desktop-dark.png`
- 编辑/确认删除：`quittung-follow-up-edit-mobile-light.png`、`quittung-follow-up-delete-mobile-light.png`
- 完整生成回答：`quittung-follow-up-complete-mobile-light.png`、`quittung-follow-up-complete-desktop-light.png`
- 最终快问快答：`quittung-follow-up-short-mobile-light.png`、`quittung-follow-up-short-mobile-dark.png`、`quittung-follow-up-short-desktop-light.png`

其余截图均位于 `output/playwright/`。首轮测试回答的法律效力措辞暴露了过度推断，已据此补提示约束；未把截图回答当作法律事实。

## 边界

工具环境只读，不能初始化不存在的关联词；模型需支持函数工具。工具回合不作为答案流出，网页等待完整验证后展示，暂不逐token显示。工具不保证回答全部准确，新增依据约束仍需后续实际使用验证。

编辑/删除暂未加入通用批量operation，删除无撤销历史；CAS保护生成执行期间的改动，不承诺覆盖所有排队前编辑场景。Spark真实客户端刷新与审批体验未复测，需要刷新工具发现。部署重启使内存学习session失效，重新进入学习页按持久日志恢复。
