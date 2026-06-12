任务：根据德语词典 facts，为查询词输出 canonical lexical branches 的 JSON 骨架。

你会收到：
- 查询词
- 一份来自词典源记录的结构化 facts JSON

输出范围：只包含 branch、简洁中英释义和稳定语法特性。不要输出例句、搭配、词汇网络、词源或学习解析。

这些 facts 是证据，不是程序替你做好的 branch 结论。请基于这些词典事实，产出适合作为学习主结果的 canonical branches。

branch 原则：
1. 只有当同一拼写词已经明显分成不同词汇分支时，才拆成多个 entries。
2. 常见应拆分的情况：不同词性；名词性别/屈折体系明显冲突；动词可分/不可分冲突；强/弱变化对应不同核心分支。
3. 不要因为支配格、一般搭配限制、反身标签这类属性就机械拆分 entry。
4. 如果同一 branch 下只是多义，请保留在同一个 entry 下的 meanings 数组中。
5. 不同词性默认拆分成不同 entries，包括 `adjective` 与 `adverb`。即使语义接近，也不要为了省卡片而跨词性合并。

大小写规则：
1. 把查询词的大小写当作强信号。
2. 小写查询优先小写 branch，不要把仅属于大写名词或专名的 branch 混进来。
3. 大写查询优先大写 branch，不要把仅属于小写动词或形容词的 branch 混进来。
4. 只有当词典事实清楚显示：同一拼写在当前大小写下本来就存在多个常见独立 branch，才一起保留。

默认忽略项：
1. 专名、姓氏、缩略词。
2. 纯变位形式、纯过去式/分词形式、纯 form-of 说明，不要因为它们在事实里出现就机械生成独立 branch；但如果词典 source row 本身就是非 `form-of` 的独立词性条目（例如独立的形容词或名词），则应视为可保留的 branch。
3. 仅在边缘 sense 中短暂出现、但不构成学习主目标的附带名词化或派生用法。
4. 带有 `dialectal`、地域、文体、罕见色彩的 sense，默认视为次要证据；除非它本身明显是该拼写下常见且可独立学习的 branch，否则不要把它抬成 canonical branch。
5. 但如果同一拼写下还存在另一个常见、可独立学习的通用词汇分支，即使它不是最核心义项，也应保留，不要误删。

字段取值规则：
- `pos` 只用英文小写：`noun`, `verb`, `adjective`, `adverb`, `preposition`, `conjunction`, `particle`, `interjection`。
- `genders` 只用：`masculine`, `feminine`, `neuter`。
- `noun_class` 只用：`strong`, `weak`, `mixed`。
- `separable` 只用：`separable`, `inseparable`，否则留空。
- `transitivity` 只用：`transitive`, `intransitive`, `both`，否则留空。
- `reflexive` 只用：`none`, `optional`, `required`，否则留空。
- `governs_cases` 只用：`nominative`, `genitive`, `dative`, `accusative`。
- `word_order` 只用：`main_clause`, `subordinate_clause`，否则留空。

`selector` 统一格式：
- 名词优先写成：`Substantiv · m.` / `Substantiv · f.` / `Substantiv · n.` / `Substantiv · m./n.`。
- 动词优先写成能区分兄弟分支的最小标签：如 `Verb · trennbar`、`Verb · untrennbar`、`Verb · stark`、`Verb · schwach`。
- 若只靠一个标签仍无法区分，再补第二维，如 `Verb · stark · intransitiv`。
- 形容词、副词默认只写 `Adjektiv`、`Adverb`。
- 介词、连词默认只写 `Präposition`、`Konjunktion`；只有同词性内部仍需拆分时，才补 `· Gen.`、`· Nebensatz` 这类最小限定。
- 不要把随机屈折例形、整句解释、语体标签（如 `ugs.`、`veraltet`）塞进 selector。

输出要求：
1. 只能输出合法 JSON，不要输出 Markdown、代码块或额外说明。
2. 字段缺失时使用空字符串、空数组，不要猜测。
3. `selector` 是给前端切换 branch 用的短标签，必须遵循上述统一格式，例如 `Substantiv · m.`、`Verb · trennbar`、`Präposition`。
4. `meanings` 只保留简洁的中英释义，不要写长解释。
5. `grammar` 只保留稳定事实，不要写场景解释。
6. 如果 facts 中出现多条 source rows，请先判断它们是独立 branch 还是同一 branch 的不同义项，再决定是否拆分。
7. 如果证据不足以唯一确定，请保守输出最常见的 1-3 个 branch，不要为了“完整”而扩写。

输出 schema：
{model_a_schema}
