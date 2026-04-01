# De-AI-Hilfer

一个基于 Rust (Axum) 的德语学习检索增强系统。

这个项目的重点不是“把大模型接起来”，而是围绕德语词典数据做更严格的词法建模与查询链路设计：通过 `lexeme / surface-form` 重建、9 万级向量嵌入、多路召回与词典事实约束，减少词形误判与生成漂移，优化查词、短语解析与学习闭环。

## What This Repo Demonstrates

- 用 Rust 后端承接真实的词典查询、缓存、流式响应与兜底逻辑
- 用 PostgreSQL + `pgvector` 管理结构化词典事实层与向量索引
- 将德语词典从“原始词条”重建为更适合检索与分析的 `lexeme / surface-form` 模型
- 在运行时组合多路召回，而不是粗暴做成长文档 `chunking` 式 RAG
- 在 AI 生成链路中引入词典事实约束、缓存优先、故障切换与预热防污染策略

## Tech Stack

| Layer | Choice |
| --- | --- |
| Frontend | SvelteKit + TypeScript |
| Backend | Rust + Axum |
| Database | PostgreSQL + `pgvector` |
| AI Gateway | OpenAI-compatible remote API |
| Analysis Models | Gemini primary, GLM fallback |
| Embedding Model | `glm-embedding-3` |
| Infra | Docker Compose + nginx + external gateway |

## Architecture

```text
User Query
  -> SvelteKit UI
  -> Axum API
  -> Multi-stage query resolution
       -> exact knowledge hit
       -> dictionary headword hit
       -> form resolution
       -> fuzzy candidate search
       -> embedding-assisted headword inference
       -> AI inference fallback
  -> dictionary-grounded analysis generation or cached response
  -> knowledge cache / learning flow
```

### Runtime Components

- `frontend-web/`
  SvelteKit Web UI for search, suggestions, library, follow-up and learning actions.
- `backend/`
  Rust + Axum API for query resolution, analysis generation, caching, phrase attach/detach and learning orchestration.
- `PostgreSQL + pgvector`
  Stores dictionary facts, knowledge entries, learning state and lexeme embeddings.
- `compose.yaml`
  Two-container deployment entry: frontend served by `nginx`, backend reverse-proxied behind it.

## Retrieval Pipeline

当前运行时链路不是标准“长文档切块 -> 向量检索 -> 生成”的 RAG，而是面向词典数据的多级词法检索流程：

1. `Exact hit`
   优先命中知识库缓存或词典 headword。
2. `Form resolution`
   对变位、分词、表层形式做归一，映射到真正可分析的词位对象。
3. `Fuzzy matching`
   对大小写、变音符号和轻度拼写错误做宽松召回。
4. `Embedding-assisted inference`
   用 lexeme embedding 辅助推断更可能的 headword。
5. `AI fallback`
   只有在前几层都不够稳时，才让模型介入推断或生成。

## Data Modeling

项目没有直接把 Kaikki/Wiktionary 原始词条平铺使用，而是做了额外的事实层重建：

- `dictionary_raw_entries`
  保留上游原始记录。
- `dictionary_lexemes`
  承接真正可分析的词位对象。
- `dictionary_surface_forms`
  承接表层形式、词形映射与候选召回。
- `dictionary_lexeme_embeddings`
  绑定到 lexeme，而不是旧的 headword 压平对象。

当前已完成约 `93,626` 条 lexeme embedding 回填。

## Key Engineering Decisions

### 1. Why Not Chunking-Style RAG?

词典数据不是普通长文档。它天然带有强结构：

- 原型词
- 变位词
- 词性
- form-of / alias / surface-form 关系

如果直接把词典说明切成文本块再做向量检索，会丢失这些词法关系，导致：

- 检索目标不稳定
- 不同词性被混淆
- 表层形式容易把模型带偏

所以这里优先做的是 `lexeme / surface-form` 建模，再把 SQL 检索与 embedding 近邻结合起来，先锁定“该分析谁”，再让 AI 基于词典事实层生成。

### 2. Why Runtime Fallback But No Cross-Model Pollution In Prewarm?

运行时和批量预热的目标不一样：

- 运行时链路优先保证响应连续性
- 预热链路优先保证数据一致性

因此：

- 用户查询时，Gemini 主模型超时或失败，可以切到 GLM 兜底
- 批量预热时，严格禁止跨模型污染，只接受主模型结果

预热脚本遇到非主模型结果或临时兜底结果时，会删除缓存并按退避策略重试，避免知识库被不同模型的风格和质量混写。

### 3. Why Separate Cached Hits From Streaming Analysis?

不是所有查询都值得走 SSE。

- 对已落库词条，直接返回完整缓存结果，优先追求稳定与低延迟
- 对首次生成或强制刷新，再启用流式分析

这样可以避免用户在“本来已经有缓存”的场景下，仍然长时间停留在“分析中”状态。

### 4. Why Dictionary-Grounded Generation?

这里的 AI 不是自由生成器，而是被词典事实层约束的分析器。

目标不是“让模型看起来会德语”，而是：

- 让模型围绕正确词位展开
- 降低词形误判
- 降低生成漂移
- 在模型失败时仍保留确定性兜底路径

## Current Scope

当前已打通的主链路包括：

- 搜索与候选词
- 缓存命中与流式分析
- `intelligent_search` 多路推断
- 短语宿主候选与挂载
- 词库浏览与分页
- 基础学习闭环
- 数据管理入口

需要明确的是：

- embedding 已接入运行时检索
- 但项目还没有做成“检索相关解释片段再参与生成”的完整生成型 RAG

当前准确定位更接近：

- dictionary-grounded AI analysis
- embedding-assisted lexical retrieval
- multi-stage query resolution

## Minimal Run Note

这个仓库保留最小运行入口，但 README 不以“普通用户开箱即用”为目标。

```bash
docker compose up -d
```

如果只想理解项目结构与链路，阅读当前 README 与源码已经足够。

## Deployment Note

- frontend: static SvelteKit build served by `nginx`
- backend: Rust/Axum API
- reverse proxy: can be attached to an external gateway such as Caddy
- database: PostgreSQL + `pgvector`

## Data Source & Distribution Note

项目依赖第三方原始词典数据与本地整理语料。

这些内容不会随仓库直接提供：

- Kaikki / Wiktionary-derived raw dictionary dump
- 本地整理的调试样本与运行日志
- 本地课程语料与 `telc` PDF 词表源

原因包括：

- 数据体积
- 版权 / 分发限制
- 本地实验环境耦合

因此，这个仓库更适合作为：

- 架构展示
- 检索链路复盘
- Rust + Svelte 全栈工程样本

而不是面向普通用户的完整可复现产品包。

## Repository Boundary

公开仓库默认只保留：

- 应用源码
- 运行配置
- 容器化部署入口
- 面向技术展示的 README

以下内容默认不进入公开仓库：

- 内部迭代记录与 Agent 记忆文档
- 本地调试样本、运行日志与预热报告
- 第三方原始词典 dump 与课程材料
- 本地环境密钥与私有网关配置
