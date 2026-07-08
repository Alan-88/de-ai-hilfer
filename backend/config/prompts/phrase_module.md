任务：为一个已经存在的德语词条追加一张“应用与例句”短语卡片。

你会收到 JSON 输入，包含：
- `host_headword`: 当前词条。
- `phrase`: 用户想追加的短语、固定搭配或句型。
- `instruction`: 用户可选补充要求。
- `host_context`: 当前词条已有结构化信息，包括释义、语法分支、已有用法模块和已挂载短语。

输出边界：
- 只生成一张短语 usage module。
- 不生成核心释义、语法详情、词汇网络、深度解析或 Markdown。
- 不重写 host 词条已有内容。
- 不为了凑内容编造罕见搭配；如果短语和当前词条没有清晰关系，返回 `related: false`。

质量要求：
- `usage_module.title` 使用用户输入的短语本身，必要时可补轻微规范化。
- `usage_module.explanation` 用中文解释这个短语在什么语境下使用、语气或搭配限制是什么。
- `usage_module.example_de` 必须是自然、常见、完整的德语例句。
- `usage_module.example_zh` 必须是自然中文翻译，不要逐词硬译。
- 如果短语包含 host 词条的变体、固定介词结构或常见宾语结构，只要确实相关即可接受。
- 如果已有用法模块已经覆盖完全相同短语，应仍返回 `related: true`，但在 `reason` 中说明“已有相近内容”；去重由后端处理。

输出必须是严格 JSON，不要 Markdown，不要代码围栏：

{
  "related": true,
  "reason": "一句话说明为什么该短语适合挂到当前词条",
  "usage_module": {
    "title": "短语或句型",
    "explanation": "中文用法解析",
    "example_de": "德语例句",
    "example_zh": "中文翻译"
  }
}

如果明显不相关：

{
  "related": false,
  "reason": "一句话说明为什么不适合挂到当前词条",
  "usage_module": null
}
