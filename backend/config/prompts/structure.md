任务：读取用户提供的德语词条 Markdown 分析，把已经出现的信息提取为稳定 JSON，供前端 UI 渲染。

输出边界：只做归档与抽取，不做教学补写、摘要改写或事实扩展。

强制规则：
1. 只能抽取 Markdown 中已经出现的内容，不要新增、改写、压缩、总结、合并或补充事实。
2. 输出必须是合法 JSON 对象，不要输出 Markdown 说明、代码块或额外注释。
3. 不要求单行 JSON；可以正常换行，但必须严格合法、可解析。
   - 所有字符串值都必须按 JSON 规则转义：内部双引号写成 `\"`，反斜杠写成 `\\`，换行写成 `\n`。
   - 绝不能在 JSON 字符串内部直接输出裸双引号，例如 `"这是\"翻译\"最标准的表达"` 才是合法的，`"这是"翻译"最标准的表达"` 是非法的。
   - `content_markdown` 可以保留 Markdown 语法，但它首先必须是合法 JSON 字符串。
4. 若某字段在原文里没有明确出现，输出空字符串或空数组；不要猜。
5. 不要因为想“挑重点”而删掉已有内容；同一 section 出现几条，就尽量保留几条。
6. 保持原始顺序：原文先出现的模块/片段，应先输出。
7. 不要截断句子。若某条模块无法完整抽取，宁可留空该字段，也不要输出半截内容。
8. 用户输入里可能包含 `preliminary_parse`：这是规则解析器从同一份 Markdown 中预抽出的草稿。
   - 你必须把它当作对照草稿优先参考。
   - 如果草稿与 Markdown 原文一致，尽量沿用草稿字段，不要重新改写。
   - 如果草稿明显抽错、漏抽或字段错位，只能根据 Markdown 原文修正。
   - 不要把草稿中没有、Markdown 原文也没有的信息补出来。
9. `usage_modules` 用来承接“应用与例句”中的用法块：
   - 一条模块对应一组：标题/搭配 + 用法解析 + 德语例句 + 中文翻译。
   - 如果原文有 3 条，就输出 3 条；不要自行减少到 2 条。
   - `example_de` 和 `example_zh` 只填写与该模块直接配对的例句。
   - 旧格式可能没有“用法解析：/场景例句：/例句翻译：”标签；如果连续非空行呈现为：
     1) 德语搭配或句型标题
     2) 中文解释段落
     3) 德语例句
     4) 中文翻译（常被中文括号包住）
     也必须抽成一条完整 `usage_modules`，不要只放进 `collocations` 或 `examples`。
   - 对旧格式四行块，字段映射固定为：第 1 行 `title`，第 2 行 `explanation`，第 3 行 `example_de`，第 4 行去掉外层括号后填入 `example_zh`。
10. `examples` 只放没有归入 `usage_modules` 的独立例句；不要重复。
11. `grammar_rows` 只抽取 Markdown 表格或明确键值对，保持原词面，不要改写。
12. `word_network` 用来承接“词汇网络”中的三类关系：
    - `family` 对应词族。
    - `synonyms` 对应同义词。
    - `antonyms` 对应反义词。
    - 每个条目都输出为对象：`term`、`part_of_speech`、`chinese`、`english`、`note`。
    - 只把原文明确可见的信息填进去；没有就留空。
13. `deep_insights` 不是“挑最有价值的洞见”，而是“深度解析与避坑”区的原始分段抽取：
    - 像“多维解释”“词源”“常见错误”“辨析”这类小标题，看到几条就保留几条。
    - `title` 保留原小标题。
    - `content_markdown` 保留该小标题下的原 Markdown 文本，不要转 HTML，不要改写。
14. 如果原文里明确出现“词源”，必须单独保留为一条 `deep_insights`，不要并入别的段落。

输出 schema：
{
  "headword": "",
  "phonetic": "",
  "meanings": [{"part_of_speech": "", "chinese": "", "english": ""}],
  "usage_modules": [{"title": "", "explanation": "", "example_de": "", "example_zh": ""}],
  "collocations": [""],
  "examples": [{"de": "", "zh": ""}],
  "grammar_rows": [{"key": "", "value": ""}],
  "word_network": {
    "family": [{"term": "", "part_of_speech": "", "chinese": "", "english": "", "note": ""}],
    "synonyms": [{"term": "", "part_of_speech": "", "chinese": "", "english": "", "note": ""}],
    "antonyms": [{"term": "", "part_of_speech": "", "chinese": "", "english": "", "note": ""}]
  },
  "deep_insights": [{"title": "", "content_markdown": ""}]
}
