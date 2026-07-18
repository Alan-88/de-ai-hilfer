# Active Roadmap

最后更新：2026-07-18

本路线图只保留“当前阶段仍值得继续做”的事项。

## P0：词典重构、稳定性与数据安全

### 1. 词典层建模重构

目标：彻底修正当前“`headword` 压扁上游多记录”的根问题，统一候选、分析目标与 embedding 的语义。

当前进度：

- 新词典表结构已落地并完成过一轮重建
- `/api/v1/suggestions` 已优先使用 `lexeme + surface_index`
- 运行时 `headword/form` 解析已切到新的词典事实层
- embedding 初始化与检索已切到新的 `dictionary_lexeme_embeddings`
- 学习顺序排序已切到新的 `dictionary_lexeme_embeddings`
- `knowledge` exact hit 已改为 `query_text exact-only`，旧 alias 不再抢解析链路
- `knowledge_entries` 已完成第一阶段落点迁移：
  - 新增 `lexeme_id`
  - 新写入优先绑定 `dictionary_lexemes.id`
  - 历史普通词条已完成一轮“唯一可判定即回填”的自动迁移
  - 当前库内普通词条首轮回填结果为 `2145 / 2145`
- `dictionary_learning_order` 已完成第二阶段落点迁移：
  - 新增 `lexeme_id`
  - 学习顺序导入/更新优先绑定 `dictionary_lexemes.id`
  - 当前库内首轮回填结果为 `2204 / 2268`
  - 剩余 `64` 条先保留为歧义/词形异常样本池，后续集中处理
- 2026-04-09 已补 ETL 代码修复：
  - 不再把“含有 form-of sense 的混合词条”整条降级为 `pure form`
  - `Mixed` 词条重新进入 `lexeme` 层
  - 同时剥离 form-of senses，避免独立义项被错误 gloss 抢首位
  - 2026-04-10 已完成真实字典重建与 embedding 回填
- 2026-04-10 已补“重建前缓存保护”链路：
  - `export_knowledge_snapshot`
  - `dry_run_restore_knowledge_snapshot`
  - `restore_knowledge_snapshot`
- 已在当前数据库上完成真实 dry-run：
  - `2145 / 2145 knowledge_entries` 可恢复
  - `3 / 3 learning_progress` 可保留
  - `0 rejected`
  - 说明“全量重建字典与 embedding，再回灌预热知识库”的方案可行
- 2026-04-10 已完成真实重建与恢复：
  - `dictionary_lexemes = 94098`
  - `dictionary_surface_forms = 1789051`
  - `dictionary_lexeme_embeddings(glm-embedding-3) = 94098`
  - `knowledge_entries = 2145` 已恢复
  - `learning_progress = 3` 已恢复
  - dangling `knowledge_entries.lexeme_id = 0`

 剩余工作：

- 在真实重建后回归验证以下样本：
  - `Anzug`
  - `Aufzug`
  - `Zähler`
  - `Ansehen`
  - `Schreiben`
- 继续观察 rebuilt 数据上的候选排序与弱 `form-of` 提示行为：
  - `reisen / Reisen`
  - `gefallen / Gefallen`
- 让生成阶段直接理解独立 `lexeme` 上下文，而不是继续兼容旧 `DictionaryRaw`
- 逐步回收旧 `dictionary_raw(headword PK)` 与旧 `dictionary_embeddings` 依赖点
- 把 `knowledge` 读路径、学习排序与管理导入导出进一步统一到 `lexeme_id`
- 复核 `dictionary_learning_order` 里暂未绑定 `lexeme_id` 的 `64` 条样本：
  - 主要预期是歧义词头、form-like 条目和教材词表中的表层形式
  - 继续遵循“异常归档，最后集中清算”
- 为后续迁移保留“异常样本池”策略：
  - 当前未出现需要人工判定的历史词条
  - 后续若出现无法唯一回填或运行时无法唯一落点的条目，先归档，最后集中处理

下一阶段应按以下方向推进：

- 保留上游逐条记录的 `raw entry` 层
- 构建真正可分析的 `lexeme` 层：
  - 只收非 `form-of` 的独立原型词条
  - 同词头、同大小写下的多个独立原型词性合并为一个可分析对象
- 构建独立 `surface index`：
  - 承接 `forms`
  - 承接 `form_of`
  - 承接 alias / embedding
  - 只负责候选召回，不负责分析阶段跳转
- 让候选列表展示真正可分析的原型词条，而不是 `form-of` 本体
- 让 `analyze` 真正只消费新的 `lexeme` 对象
- 让 embedding 绑定新的可分析对象，而不是旧的 `headword`

### 2. 模型链路稳定性

目标：把当前剩余风险从“偶发不可用”继续压到“可接受、可解释、可恢复”。

建议继续做：

- 2026-05-07 普通词正式生成链路已切到 A/B/C：
  - Model A 生成 `grammar_branches` / 语法骨架 JSON
  - Model B 生成用法例句 / 词汇网络 / 深度解析 Markdown
  - Model C 负责结构化抽取
  - 组装后的 `structured` 会携带 `grammar_branches`
  - stream / non-stream 已验证都能使用该正式链路
  - A/B/C 失败时不再写库，避免字典兜底和半成品污染 `knowledge_entries`
- 2026-05-19 已完成结构化主链路迁移收口：
  - 全库普通词 `grammar_branches` 覆盖为 `2161 / 2161`
  - 正式 `analyze / analyze_stream` 已不再依赖 `structured_archive` 运行时补修
  - `?grammarPreview=1` 灰度门控已删除，`grammar_branches` 展示已转正
  - 前后端正式词汇网络链路已统一到 `word_network`
  - `family / synonyms / antonyms` 的正式展示兼容已移除
  - 迁移期实验入口已清理：
    - `compare_model_a*`
    - `compare_stage2*`
    - `dry_run_structured_backfill*`
    已从仓库移除
  - 当前保留的离线工具只剩：
    - `audit_structured_entries`
    - `staged_structured_backfill`
    - `scripts/staged_backfill_worker.sh`
  - 2026-06-12 已把结构化链路 prompt 收口到 `backend/config/prompts/`：
    - `model_a.md`
    - `stage2.md`
    - `structure.md`
    - `default.yaml` 的同名字段继续作为临时 override 入口，默认保持空值。
  - 2026-07-08 已把动作级“额外提示词”接入普通词 A/B/C 新链路：
    - 只传给 Model B / Stage 2，用于影响用法例句、词汇网络、深度解析的呈现。
    - 不传给 Model A，避免改变释义/语法骨架。
    - 不传给 Model C，避免结构化抽取器二次理解或补写。
- 2026-05-07 已给正式 A/B/C 链路补 Model C-only transient retry：
  - OpenRouter timeout / rate limit / upstream unavailable 会在同一次请求内只重试 Model C
  - 已生成的 Model A / Model B 结果不会重复生成
  - 耗尽重试后仍失败不落库
- 2026-05-07 已启动历史 `regenerate_abc` 小批 backfill：
  - runner 新增 `STRUCTURED_BACKFILL_MODE=regenerate_abc`
  - 默认仍 dry-run，只有 `STRUCTURED_BACKFILL_APPLY=true` 才写库
  - 已迁移并验证 `Anfrage / erholen / pauschal / Unterkunft / zelten / Pauschalreise / künftig / Vielfalt / derselbe`
  - 2026-05-07 继续迁移 `Verwaltungssprache / Heimat / aufgeben`，当前 `grammar_branches` 覆盖约 `23 / 2161`
  - runner 已补 `STRUCTURED_BACKFILL_APPLY_REPORT=<report.json>`：
    - dry-run 需使用 `STRUCTURED_BACKFILL_INCLUDE_FULL=1` 保留完整 `analysis`
    - 人工审查 report 后可复用 dry-run 产物无 AI 写库
    - 这避免 apply 阶段重复调用 Model A / B / C
  - `borgen / ursprünglich / gründen / vorsichtig` 最新 dry-run 因 timeout 或 empty response 被归类为 transient deferred，未写库
  - backfill 已改为 strict primary only：
    - 正式查询保留 Stage2 fallback 以保证用户可用性
    - 离线 backfill 不接受 GLM fallback 产物
    - Stage2 主模型失败时 deferred，避免批量语料跨模型污染
- 下一步不直接全量 backfill，应继续做：
  - 当前 Model C provider 已切换判断方向：
    - `glm-4.7-flash` 在当前网关上主要暴露 `429/timeout`
    - `gpt-4o-mini` 已在 C-only 基准中验证更稳，现已成为正式候选
  - staged backfill 现已支持：
    - `prepare_ab` 串行准备 A/B 缓存
    - `run_c` 在单协调器内做有界并发消费
    - 默认目标配置为 `gpt-4o-mini + C 并发 4`
  - 下一步重点转为：
    - 观察真实大批量 backfill 下的 deferred 率
    - 继续区分 provider transient、模型截断、结构化质量闸门失败
    - 若真实长跑中 deferred 仍极低，再考虑是否继续上探更高并发
  - 已新增持久化分阶段 backfill 队列：
    - bin：`staged_structured_backfill`
    - `prepare_ab` 缓存 Model A 输出 + Model B Markdown
    - `run_c` 慢速消费 Model C
    - `run_c` 对 transient Model C 失败默认做 3 次外层指数冷却重试，耗尽后才记录 failed
    - `apply_ready` 只 apply A/B/C 全成功且通过 quality gate 的结果
    - `zusammenstoßen` canary 已完整通过并写库
    - 当前 `grammar_branches` 覆盖约 `24 / 2161`
- 下一步新增明确边界：
  - 短语挂载链路已收口到“主词条内添加短语卡片”
  - 新入口：`POST /api/v1/entries/:entry_id/phrase-modules`
  - 删除入口：`DELETE /api/v1/entries/:entry_id/phrase-modules`
  - 新 prompt：`backend/config/prompts/phrase_module.md`
  - 新链路只追加一张结构化 `usage_module`，不再生成整篇短语 Markdown 词解
  - 短语卡删除链路已修复 JS 安全整数精度问题，并补 `phrase` 兜底删除
  - 旧全局短语搜索、宿主候选、`/api/v1/phrases/attach`、`/api/v1/phrases/detach` 已退役
  - Agent 式“追问后修改卡片”暂定为后续大版本，不并入本轮短语入口迁移
- 2026-07-08 已启动“模型配置系统”第一阶段：
  - 新增供应商 Profile 与任务默认模型的数据库表。
  - 新增 `GET/PUT /api/v1/ai-settings`，可管理 Base URL、API Key、可用模型 id 与任务默认模型。
  - 数据库为空时返回环境变量兜底视图，避免设置页首次打开为空。
  - 初版用户侧任务只保留：查词、短语、结构化输出、embedding、联想搜索。
  - 重新生成 / 追问属于动作级临时选择模型，不作为全局独立任务项。
  - 普通查词的隐藏拼写纠错 / 原型识别兜底已退役，不再进入配置系统。
  - 在线 AI 调用已接入统一 resolver：
    - 查词、短语、结构化输出、embedding、联想搜索会读取持久化任务配置。
    - 追问默认沿用查词模型，并已支持动作级临时改选。
    - 离线 backfill / embedding 初始化脚本仍保留显式环境变量配置，不受设置页影响。
  - 设置页 UI 已接入 `GET/PUT /api/v1/ai-settings`，保持现有 bento 管理页风格：
    - 供应商 Profile 管理
    - 任务默认模型路由
    - 短语沿用查词模型
    - 环境变量密钥不回传时的首次保存保护
  - 动作级模型选择已补基础版：重新生成 / 追问可在查词结果头部临时选择已配置模型，不写回全局设置。
  - 2026-07-13 已补主分析失败后的显式重试 / 换模型：
    - 失败卡直接提供“按原设置重试”。
    - 可选择已配置渠道与非 embedding 模型后重试。
    - 重试继续复用动作级 `model_override`，不新增后端模型入口。
  - 学习模块整理放在配置系统之后，作为独立大阶段推进。
- 2026-07-08 已完成普通查词确定性收口：
  - 后端 `analyze / analyze_stream` 删除隐藏 AI spell/prototype fallback。
  - exact headword、词形还原仍保留；全局短语查询已退役；无可靠候选时返回“未找到可靠候选”且不写库。
  - 前端搜索提交会在候选存在时默认进入第一候选，避免直接回车触发不可控 AI 猜词。
  - 显式 AI 找词能力继续归属“联想搜索”。
- 复查 `analyze` / `follow-up` / `intelligent_search` 的超时、重试和故障切换策略。
- 明确第三方模型失败时的返回分类：
  - 可重试
  - 可切换模型
  - 必须走本地兜底
- 补一轮真实运行日志复盘，确认失败是否主要集中在单个模型或单种场景。
- 2026-03-29 已完成一轮窄收口：
  - `AnalysisMode::Full` 单次超时从 `25s` 提高到 `40s`
  - `analyze` 生成链改为“Gemini 先重试一次，再切 fallback”
  - 本地样本 `Karte` 已验证可通过重试恢复到 `gemini-3-flash-preview`
- 已知需要优先复查的具体样本：
  - `Lampe`
  - `gefallen`
  - 这两个词在 2026-03-28 的完整分析回归中落入了 `字典兜底`
  - 2026-03-29 已进一步定位：
    - 更像是 `full analyze` 模型生成阶段超时/失败
    - 当前不是原型解析错误
  - 下一步不再优先做“大改路由”，而是继续观察：
    - `Lampe`
    - `gefallen`
    - 是否在新超时 + 主模型重试后仍频繁掉入兜底

### 3. 数据导入恢复验证

目标：确认导入功能在真实备份上的行为边界，而不是只有接口存在。

建议继续做：

- 准备一份小型真实备份做恢复演练。
- 验证恢复后：
  - `knowledge_entries`
  - `follow_ups`
  - `learning_progress`
  三者是否一致。
- 补一份“恢复操作说明”，避免后续误用。

## P1：功能增强

### 4. 文章驱动的学习导入

当前进度：

- 第一阶段底座已完成：受保护的 Agent API、仓库内 CLI、项目局部 Skill。
- 已能完成“文章表层词 -> 候选原型 -> 状态检查 -> 用户确认 -> 初始化 -> 加入学习”的 Agent 工作流。
- CLI 已补齐阅读文章 `list/get/import`，可将用户确认过的 `[表层词形](原型)` 正文送入正式文章模块。
- context 和文章来源暂不入库；相同拼写的不同出现位置仍需分别判断。
- 文章目录、持久化和正式阅读页已于 2026-07-17 落地：
  - `/learn/articles` 使用多层文件夹 + 文章列表。
  - `/learn/articles/:id` 使用文章正文 + 常驻窄版查词栏。
  - 正文采用 `[表层词形](原型)` 书签语法，派生索引在保存事务中重建。
  - 保存前要求原型唯一命中已有完整结构化释义的词条；失败时定位到具体书签并拒绝保存。
  - 阅读状态收敛为 `未读完 / 已完成`，通过目录拖动或长按菜单切换。
  - 导入、编辑、文件夹/文章 CRUD、右栏真实查询与加入复习均已接通。
  - 文章数据已纳入数据库导出与恢复。
  - 正文渲染态已支持“单次位置选词 / 直接右键 -> 候选确认 -> 写回书签”；仅词典候选会明确触发正式生成，不自动处理同拼写位置。
  - 移动端长按菜单已补文件夹选择层，桌面拖动逻辑保持不变。
  - 文章 CRUD 与入口审计已收口：阅读页可直接编辑文章、标记词可取消、空书签浮层已禁用、右栏学习状态已同步；触屏端文件夹移动入口已补齐，无效排序按钮已移除。

下一步：

- 用一篇真实 Netzwerk neu B1 语料执行首次完整站内阅读与 Skill 导入对照，观察候选质量和人工确认成本。
- 在实体移动设备上复核长按选词与系统文字选择菜单是否冲突。
- 暂不建设 MCP、Tauri、后台批量生成队列或 FSRS 原始字段编辑。

### 5. 学习页增强

目标：在 V3 学习引擎已落地的基础上，继续打磨真实背词体验与个人化调度。

当前进度：

- 2026-07-09 已完成学习引擎 V3：
  - 第一次反馈立即提交长期 FSRS。
  - 当天后续复现只影响日内队列。
  - 新词 / 复习词使用三档语义按钮。
  - 新增 `learning_review_logs`，为后续个人化模型保留数据。
  - 前端学习页已切到 V3 接口和三档评分。
- 2026-07-10 新增独立 `/learn/layout-lab`；三种 PC 横屏初稿现已收敛为居中主卡，不修改正式学习页、不写库。
- 2026-07-11 已用数据库真实结构化快照验证完整答案滚动，并复用查词页语法标志和点击浮窗。
- 2026-07-12 已收口全局滚动条与主要页面的 sticky / fixed 控制关系，为正式学习页迁移统一滚动行为。
- 2026-07-12 已把选定布局迁入正式 `/learn`：
  - 回忆态整卡揭晓，答案卡内部滚动，三档评分固定在卡片下方。
  - 首次揭晓默认定位应用与例句，仍可向上查看释义与语法。
  - 直接渲染 V3 返回的完整结构化分析，多语法分支复用既有浮窗。
  - 挂载短语并入应用与例句；学习响应已补对应字段。
  - 已完成桌面、移动端和双主题只读验证，没有提交评分或改写学习数据。
  - 人工审查通过后，`/learn/layout-lab`、`/learn/mock` 及其实验专用代码已删除。

建议继续做：

- 用自然出现的非空挂载短语学习词复核一次“短语并入应用与例句”。
- 观察真实背词体验，微调日内卡片间隔与今日最多出现次数。
- 决定 session 是否需要从内存状态升级到数据库状态。
- 基于 `learning_review_logs` 做个人化调参，而不是直接上复杂强化学习。
- 继续补 `insight` / `quiz` / 动态例句等学习内容呈现。
- 更清晰的复习状态反馈

### 5. 词库与管理流补强

目标：完善已有能力，而不是新增全新模块。

建议继续做：

- 数据管理页面的操作反馈与边界提示补强
- 词库页现已改为“页面激活时同步 + 服务端分页”
- 2026-03-31 已完成：
  - 新增 `GET /api/v1/entries`
  - 搜索与 tab 筛选下沉到后端
  - 前端词库页改为 `load more`
  - 详情仍按点击单独请求
- 下一步继续观察与补强：
  - `learning / review / new` tab 的真实数据表现
  - 空态、异常态与分页末尾提示
  - 是否需要把 `load more` 进一步演进成 infinite scroll
- 词库详情仍保持按点击单独请求，不需要改成列表预加载详情
- `Reisen` 候选排序问题已在 2026-03-29 做窄修复：
  - 当前顺序恢复为 `Reise -> Reis -> reisen`
  - 后续若继续调排序，只允许围绕同类 tie-break 小步验证，避免扩散成全局权重大改

## P2：字典与 AI 层增强

### 6. ETL 与字典层增强

建议继续做：

- 深化 Kaikki 清洗
- 减少噪音释义
- 视需要补音频缓存

### 7. AI 生成质量优化

这部分现在不是最高优先级，但已有可继续推进的基础。

建议继续做：

- 使用现有 `prompt_ab_eval.py` 做小步盲评
- 优先验证是否真的改善用户感受，而不是只追求结构更规整
- 2026-04-24 已落地“双阶段结构化归档”：
  - 主模型继续产出 Markdown
  - 第二阶段 `minimax-m2.5` 抽取 `structured`
  - 前端已优先消费 `structured_analysis`
- 2026-04-26 曾补结构化归档失败止损：
  - 当时使用 `analysis.structured_archive` 记录失败原因与尝试次数
  - 该机制已在 2026-05-19 退出正式主链路并完成数据清理
- 2026-04-26 已补结构化归档异步化：
  - 缓存命中不再同步等待第二阶段结构化
  - SSE 缓存命中也不再被第二阶段阻塞
  - 新生成词条若首轮未产出 `structured`，改为落库后后台补做
- 2026-04-26 已补第二阶段稳定性收口：
  - 第二阶段不再固定 `max_tokens = 900`
  - 结构化 prompt 已改为更紧凑的最小稳定抽取
  - JSON 解析新增轻量截断修复
- 2026-04-26 已确认并修复一条比“结构化归档”更前面的质量回退：
  - `stream_analyze` 曾被错误实现成“压缩版内容链路”
  - 这不是单纯的 transport 差异，而是实质上的 prompt / 参数分叉
  - 已收口为：
    - 普通单词分析统一使用共享 Markdown 规格
    - `stream / non-stream` 对齐到同一套 `AnalysisMode::Full` 生成参数
    - `stream` 仅保留 SSE 传输职责，不再偷偷承担“内容压缩”职责
- 2026-04-26 已开始把第二阶段结构化抽取从“半理解半压缩”改回“纯抽取”：
  - 已去掉：
    - “最多 2 条”
    - “最有价值”
    - “优先保证”式压缩导向
  - 已新增：
    - `word_network.{family,synonyms,antonyms}[]` 对象结构
    - 后端结构化质量闸门
    - 前端对历史半残 `structured` 的回退兜底
- 2026-05-07 已完成正式链路迁移第一步：
  - `analyze` 普通词生成现在使用 A/B/C 链路
  - `helfen` 流式 canary 成功落库 `entry_id=2571`
  - `glitzern` 非流式 canary 成功落库 `entry_id=2572`
  - `helfen` 首次失败路径已验证“不落库”
  - 后端 preview 已能从 `grammar_branches` 生成列表摘要
  - 尚未启动历史 backfill
- 下一步继续观察：
  - 这轮“输出欲望恢复”修复后的稳定性
  - 这轮“第二阶段纯抽取化”后的成功率与信息保真度
  - A/B/C 新链路实验中，`Branch Skeleton -> 用法内容层 -> 结构化归档` 的组合是否能稳定保持原有内容质量
  - Model C 的 C0 预解析 + LLM 修正方案是否还存在静默漏解
  - 针对扩大样本中暴露的内容质量问题，小幅约束 prompt：
    - 德语格名的中德术语映射必须准确
    - 不把形容词屈折形式误判为比较级
    - usage module 标题必须是自然搭配或可复用句型
    - 例句翻译必须忠实对应德语句子，尤其避免 `spielen` 在音乐语境下误译为“打球”
  - 继续用小批次验证 A/B/C 链路，而不是一次性大批量：
    - 已通过 `spielen / sitzen / stellen / legen / alt / weil / obwohl / für / mit`
    - 下一步重点不只是 `parse_ok`，还要继续人工抽查内容是否漏解、错译或过度语法讲义化
    - Model C 的内部引号 repair 仍需统计频率，避免把“可修复坏 JSON”误认为彻底稳定
    - Model C 调用级 transient retry 已补到离线实验脚本；后续应把 provider 波动与 JSON 输出质量分开统计
    - C0 seed 已修复一类词汇网络字段错位，但质量闸门仍不能完全证明字段语义对位，需要继续人工抽查典型样本
    - 词汇网络目前是结构化层最容易出现“parse 成功但字段错位”的区域；后续抽查优先看带冠词、带性别、无方括号、带 note 的条目
  - 是否仍存在个别词条只在上游波动时掉入 `字典兜底`
  - 第二阶段结构化成功率是否还能继续提高
  - 2026-04-29 已完成一轮 Stage 2 离线对照收口：
    - `grounded_markdown` 继续保留为主候选
    - `grounded_direct_json` 继续降级为实验支线，不进入主链
    - 已验证：
      - `gefallen` 的 secondary branch 不再挤占 core branch 的高频 `usage_modules`
      - `übersetzen` 在压 branch 权重后仍能保持合理的主次用法覆盖
  - 2026-05-03 已开始安全 backfill 小批写库：
    - 当前 runner 已支持 dry-run / apply 双模式
    - 结构化抽取 prompt 已进入可审查配置文件
    - 已落库 `26` 条 `structured` object
    - 当前策略是小批推进并区分 provider transient failure 与内容/schema failure
    - `speichern` 已在 2026-05-04 补跑成功，当前失败队列清空
    - runner 已新增 transient retry/defer：
      - 明确上游限流或波动不会再混入普通失败计数
      - 最终仍失败的 provider transient case 会记为 `skipped_transient`
      - 真正 `failed` 主要留给 JSON/schema/quality gate 等需要排查的问题
  - 2026-05-04 已纠偏：
    - 不再把旧 Markdown backfill 作为当前主线继续扩大
    - 当前优先级切回 A/B/C 新链路可用性验证
    - `grammar_branches` 已进入后端正式 structured response model
    - 当时的 `compare_stage2` 已支持验证通过后写库
    - `vertragen` 已作为第一条新链路验证样本写入库并可通过 API 返回 branch UI 数据
    - 该实验入口已在 2026-05-19 归档删除

下一步建议：

- 暂停扩大历史 Markdown backfill，先围绕 A/B/C 新链路继续小样本验证。
- 下一批建议选择 3-5 个语法结构不同的词，例如：
  - `Band`
  - `übersetzen`
  - `gefallen`
  - `während`
  - `absolut`
- 每个词只有在以下条件都满足时才写库：
  - Model A parse OK
  - grounded Markdown 生成完成
  - Model C parse OK
  - structure quality OK
  - `grammar_branches` 非空
- 每批后检查：
  - `success_count / failure_count`
  - `skipped_count / transient_deferred_count`
  - `error_kind_counts`
  - `transient_error_kind_counts`
  - `repair_kind_counts`
  - 是否出现内容型失败或静默漏解迹象
- provider timeout / 429 / empty response 不立即判定为 prompt 失败，优先进入稍后重试队列。
- 第二阶段结构提取 prompt 补 JSON 转义约束后，可修复一类“裸双引号导致 parse failed”的坏样本。
- 下一步更具体地聚焦三件事：
  - 先把 `usage_modules` 视作“近冻结”状态继续观察：
    - `gefallen` 这类 mixed-branch 敏感词已做窄修复
    - 但还不应在没有更多样本前宣布彻底收工
  - 把 Stage 2 当前离线结论评估是否并入主链
  - 继续打磨 `deep_insights` 的“理解型”风格，而不是只追求格式稳定
  - `word_network` 只做轻量约束，避免为了过度规整而牺牲联想价值
  - 并行继续解决第二阶段结构化提取的坏 JSON：
    - 当前它已经不是 `usage_modules` 问题的附属现象
    - 而是独立的结构化稳定性问题
  - 2026-04-29 已确认 `deep_insights` 的下一阶段方向：
    - 继续压“理解型标题”和“解释式正文”
    - 减少 `陷阱 / 提醒 / 常见错误` 这类考试口吻在非必要场景下的出现
    - 但不要为了文风统一而牺牲像 `gefallen` / `während` 这种已经比较自然的解释质量
  - 2026-04-29 已继续验证：
    - `Band` 这类高密度多义词的 `deep_insights` 可以通过 prompt 继续从“清单化”往“模型化理解”收口
    - 但这类改进已经越来越容易被第二阶段结构化波动掩盖
  - 因此当前更值得优先切到：
    - 单独治理第二阶段结构化提取稳定性
    - 再决定何时把 Stage 2 内容链路并入主链
  - 2026-05-01 已完成第一轮结构化稳定性工程收口：
    - 结构化归档单次调用已从 `60s` 提高到 `90s`
    - 同一次归档任务内现在会先做一次内部重试
    - JSON 修复逻辑已拆成独立模块，开始覆盖：
      - 裸双引号
      - 裸双引号 + 截断
    - 下一步要验证的已经不是“代码是否具备能力”，而是：
      - 真实坏样本命中率有没有提升
      - 质量闸门失败是否显著下降
      - 后台补结构化的超时比例是否下降
  - 2026-05-01 的坏样本回放已给出一个更清晰的优先级：
    - “裸双引号 + 截断”这类历史坏 JSON 已明显缓解
    - 质量闸门里“全文出现词源 -> 强制要求深度解析标题含词源”的误判已修掉
    - provider 返回体异常（如字段为 `null`）的兼容问题已修复：
      - 非流式 `AiClient` 现已兼容 `message.content = null`
      - 也兼容 `message.content = [...]` 的分段文本变体
      - 解码失败时会附带 body 预览，便于复盘 provider 变体
    - 当前下一优先级转为：
      - 持续观察真实后台补结构化任务的失败分布
      - 专门筛查“parse 成功但内容被静默漏抽”的样本
  - 2026-05-01 已继续补一轮“静默漏解”防线：
    - 质量闸门不再只看 section 是否存在
    - 现在会检查：
      - `usage_modules` 是否覆盖 Markdown 中的完整模块数
      - `deep_insights` 是否覆盖顶层小节数
      - `word_network` 是否覆盖词族 / 同义词 / 反义词子区块
      - `grammar_rows` 是否覆盖语法表格；若某行本质被抽到 `collocations[]`，也按语义覆盖处理
    - 这轮回放已确认：
      - `helfen`
      - `pauschal`
      - `erholen`
      - `Band`
      - `Anfrage`
      在当前链路下均达到 `parse_ok = true` 且 `structure_quality_ok = true`
  - 2026-05-01 已补结构化只读审计工具：
    - `cargo run --bin audit_structured_entries`
    - 可直接导出：
      - 当前已落成 `structured` 的健康度列表
      - 当前缺 `structured` 的待补池
    - 当前审计显示：
      - `analysis.markdown` 词条约 `2155`
      - 真正已落成 `structured` object 的仅 `5`
    - 这说明下一阶段如果继续盯结构化稳定性，不应只看坏样本回放
    - 还要把“历史大盘几乎还没 backfill”作为同级问题处理
  - 2026-05-01 已补只读 backfill dry-run 工具：
    - 旧 `dry_run_structured_backfill` 已在 2026-05-19 归档删除
    - 读取缺 `structured` 的历史 Markdown
    - 支持 `STRUCTURED_BACKFILL_DRY_RUN_WORDS` 指定词表
    - 调用当前同一套结构化抽取和质量闸门
    - 只产出报告，不写库
    - 首轮最长 5 条历史词条回放：
      - `4 / 5` 成功
      - `funktionieren` 失败
      - 失败链路为：重复字段坏 JSON 后重试超时
    - 当前结论：
      - 可以继续扩大 dry-run 样本
      - 还不能直接开启全量写库 backfill
  - 2026-05-01 已根据 dry-run 结果继续微调结构化 prompt：
    - 扩大到历史最长 Markdown 前 20 条后，基线为 `18 / 20` 成功
    - 失败最终都落在上游超时
    - 但 `verlängern` 第一次尝试暴露出旧 Markdown 四行 usage block 漏抽问题
    - 已在结构化 prompt 中明确旧格式映射：
      - 第 1 行标题
      - 第 2 行解释
      - 第 3 行德语例句
      - 第 4 行中文翻译
    - 微调后复测前 5 条：
      - `4 / 5` 成功
      - `verlängern` 已恢复 3 条 usage module
      - 唯一失败 `Architektur` 为超时
    - 指定词混合样本验证：
      - `8 / 10` 成功
      - 失败均为 timeout / 504
      - `helfen` 属于字典兜底 Markdown，已从 prompt 质量判断中跳过
      - 过滤后 `künftig / Anfrage / Auge` 均成功保留 usage 与 deep insights
  - 第二阶段归档耗时是否还能继续压缩
  - `retry_exhausted` 样本是否要走定时离线补偿
  - 是否需要补一条历史知识库批量回填 / 重生成脚本
    - 当前倾向：需要
    - 理由：现有结构化大盘覆盖率太低，仅靠被动命中补做，收敛速度不够
- “主模型直接纯 JSON 输出”仍只保留为备选方向，不默认上线
- 2026-04-28 已补 Model A 离线对照实验收口：
  - 当前主实验方向：
    - `light_grounded`
  - 当前辅助压力线：
    - `branch_guided`
  - 当前已覆盖的关键样本类别：
    - 同拼写多性名词
    - 可分/不可分与强弱动词分支
    - 分词形容词/同拼写多词性
    - 功能词的支配格/语序
    - 大小写分流
    - `form-of` 混入边界样本
  - 当前确认的剩余问题：
    - prose summary 会把边缘义、地域/文体 tag、关键 forms 混在一起，容易把 Model A 带偏
    - `Reisen` 这类 invalid test input 仍会暴露“正式链路必须先保证原型再进入 analyze”的契约
  - 2026-04-28 已完成下一步收口的一半：
  - 2026-04-29 已完成 Stage 2 首轮离线对照：
    - 已新增：
      - 上述 `compare_stage2*` 实验脚本已在 2026-05-19 归档删除
    - 已验证代表样本：
      - `helfen`
      - `gefallen`
      - `während`
      - `übersetzen`
    - 当前观察结果：
      - `grounded_markdown` 相比 `legacy_markdown` 更容易保留自适应 usage units 与更完整的 `deep_insights`
      - `grounded_direct_json` 虽然可解析，但在多个样本上持续更“短”、更“收”
    - 因此下一步优先级明确为：
      - 先把 `grounded_markdown` 的输入形态继续收口到 `dictionary facts + Stage 1 output`
      - 暂不把 `direct_json` 推进为主链路
      - 随后再集中打磨 Stage 2 的“usage unit 判定规则”和“深度解析排版稳定性”
  - 2026-04-29 当前又补了一轮窄收口：
    - 当时 `compare_stage2` 默认模式已改为只跑 `grounded_markdown`
    - `direct_json` 不再作为默认并行对照线
    - 当前继续保留 `direct_json` 仅用于：
      - 定向回归
      - 将来若要重新挑战“主模型直接出结构化”时的实验基线
  - 当前 Stage 2 的主线任务已进一步收敛成三件事：
    - 微调 `grounded_markdown` 的 `usage_modules` 选材边界
    - 把 `deep_insights` 收口到“理解型优先”的稳定风格
    - 专门解决 `markdown -> structured` 第二阶段抽取的漏块/丢内容问题
  - 2026-05-02 结构化抽取阶段已完成一轮 `word_network` 保真修复：
    - 修复词性内部逗号导致字段错位
    - 修复无方括号条目导致 term 截断
    - 修复中文说明括号被误删
    - `Kiefer` 与 `Ton / Lehre / Absatz` 小批次复测均通过
  - 当前下一步：
    - 不再继续单点 prompt 微调
    - 已进入历史 Markdown 结构化 dry-run
    - 前 10 条只读 dry-run 从 `2/10` 修复到 `8/10`
    - 前 30 条只读 dry-run 达到 `29/30`，唯一失败词单独复测成功
    - offset 30 的第二批 30 条只读 dry-run 为 `26/30`，4 个失败词单独复测全部成功
    - offset 60 的第三批 30 条只读 dry-run 为 `30/30`
    - 已补结构化归档 repair 观测：
      - repair 日志带 query
      - 大幅截断修复会被拒绝并触发重试
    - 2026-05-03 已完成 backfill runner 第一阶段工程化：
      - `structure_prompt` 外置到 prompt 配置
      - runner 默认 dry-run
      - `STRUCTURED_BACKFILL_APPLY=true` 才写库
      - report 记录错误分类、repair 分类、repair events 和 apply 结果
      - 已完成 `vertragen` 单条 apply 验证
    - 已确认一类历史格式兼容问题：
      - 旧版 `* 词族 (Wortfamilie)` 曾被质量闸门误判为“无关系桶”
      - 现已兼容
    - 下一轮改为小批 apply 5-10 条并前端抽查
    - 新词 `analyze` 仍不切到 A/B/C 新链路
    - `light_grounded` 已切到结构化 facts payload
    - 不再依赖 prose summary + branch hints
    - 已用旧测试集 + 补充样本整批回归
  - 当前这条线的下一步：
    - 评估是否把同样的 facts payload 思路迁入正式 analyze 链路
    - 继续收紧 forms facts 的提取策略，减少无关 token
    - 再决定是否仍需额外 prompt 微调
  - 2026-04-29 已完成 Stage 1 表示层口径确认：
    - `adj / adv` 不再尝试合并
    - 双性同义项默认合并
    - `selector` 采用统一模板与最小区分信息
  - 这意味着下一步可把注意力从“Stage 1 怎么展示”转向：
    - Stage 2 如何消费 Stage 1 结果
    - Stage 2 是否同时保留 dictionary facts 作为证据层

## 约束

1. 恢复范围之外的新功能，不进入本路线图，除非用户明确确认。
2. 每完成一项，必须同步更新：
   - `docs/status/current-state.md`
   - `docs/iterations/`
