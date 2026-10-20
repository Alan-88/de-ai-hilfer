# 查词候选

`GET /api/v1/suggestions?q=...&mode=lexical|semantic&limit=1..100`

- `q` 最多500个Unicode字符；空输入返回空列表。
- 缺省 `mode=lexical`、`limit=10`，文章快捷查词等现有调用方继续使用原默认行为。非法模式、数量或过长查询返回400。
- `lexical` 沿用词头、词形、别名、近似拼写的来源和排序；扩容同步扩大各来源取数，不改变精确词形优先规则。
- `semantic` 按持久化embedding模型配置嵌入完整查询，仅从有同模型、同维度向量的已初始化词条中取最近候选。过滤在排序和LIMIT前完成；同lexeme选择最早创建、ID最小的知识条目。距离相同时按频率、词头和ID稳定排序。
- 响应仍为 `{ suggestions: DBSuggestion[] }`，包含真实 `entry_id`、`query_text`、已有释义 `preview`；semantic的 `suggestion_type=knowledge_semantic`。semantic和超过10条的lexical列表省略完整解析内容（`analysis_markdown=""`、`structured_analysis=null`），默认lexical响应保持原有内容。
- 此接口只读，不初始化或分析词条。embedding失败、无兼容索引等返回错误；无意义查询仍可能有语义近邻，排名不代表正确概率。

主搜索框含Unicode汉字时请求semantic，其他输入走lexical（包含ä/ö/ü/ß/ẞ）。中文约200ms防抖，IME组合期间暂停；一次取100，首显20，再按20本地展开。此入口与原Tab联想线索功能独立。其他页面继续默认lexical/10，CLI/MCP没有新增工具或契约变更。

候选面板受实际visualViewport与可见底栏约束，列表最大420px、内部滚动。新增候选不得增加面板、文档或主页面高度。
