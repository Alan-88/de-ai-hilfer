# Gemini Spark 远程 MCP

日期：2026-10-07

## 契约

- 只对齐现有 `deai-learning-import` Skill 和 CLI 已有能力，不为 MCP 新增业务接口。
- 使用领域级工具 + `action` 参数，避免每个 HTTP endpoint 单独成为一个工具，也避免把读写操作混进一个总入口。
- 工具集合：`deai_guide`、`lexeme_query`、`lexeme_initialize`、`learning_query`、`learning_membership`、`learning_correction`、`article_query`、`article_import`。
- Skill 的本地仓库文件操作不迁移到 MCP。

## 实现

- 新增独立 `mcp-gateway` Node 服务，使用官方 MCP TypeScript SDK v2 和 Streamable HTTP。
- 网关只负责 OAuth、工具 schema、批量结果和内部 HTTP 转发，不访问 PostgreSQL。
- OAuth 使用静态 client credentials、Authorization Code、PKCE `S256` 和 owner password；保留精确 redirect URI 白名单，同时仅允许 Gemini Spark 使用 Google callback 域名的 HTTPS `/r/` 路径。失败密码按来源地址做短时限流。
- callback 明确返回 `302`；authorization code 单次使用并在 5 分钟后过期。
- access / refresh token 使用服务端 HMAC 签名，容器重启不要求 Spark 重新绑定；需要撤销时轮换 signing secret。
- GET 请求 60 秒超时并最多重试两次；写请求不重试；词条初始化等待上限为 10 分钟。
- 初始化结果只返回 entry、query、source、model 等必要字段，避免把完整结构化词条重复塞入 MCP 上下文。
- 初始化响应提供简短 server instructions，只说明文章优先工作流、核心对象关系和写操作边界。
- `deai_guide` 按需提供 `overview`、`article_learning`、`vocabulary_workflow`、`learning_v3`、`learning_diagnostics`、`learning_corrections` 六类结构化说明；现有 `learning_query(action=explain)` 保留并复用同一份 V3 文档。
- Compose 服务放在 `mcp` profile 下，现有默认部署不受未配置 OAuth 的网关影响。

## 验证

- `npm run check`
- OAuth metadata、授权页、PKCE code exchange、signed token 和 bearer gate 端到端通过。
- MCP 客户端成功读取 server instructions、发现 8 个工具，并通过 `lexeme_query` 转发到假后端。
- `learning_query(action=explain)` 能读取 V3 状态机说明。
- Docker 镜像构建和容器 `/health`、OAuth metadata 烟测通过。
- 公共 Agent API 的真实 `learning plan` 只读转发通过。
- 生产网关已通过 Caddy 按路径挂载到 `https://de.alan-studio.uk/mcp`，不新增 DNS；主站其他路径保持原前端路由。
- 已从公网完成 OAuth + PKCE、8 个工具发现和经 MCP 调用真实 `learning plan` 的端到端烟测。
- 根据 Spark 实际绑定修正 Origin 校验边界：OAuth 表单允许 opaque browser context 的 `Origin: null`，MCP 传输端点仍执行严格 Origin 白名单。
- Spark 界面实际提供可复制 redirect URI；生产配置已切换为该 URI 的精确白名单，Google `/r/` host 规则只作为可选 fallback。
- Spark 实际回调曾在 `302` 后停滞；授权响应已进一步收敛为仅含 `code + state`，移除未声明且非必要的 RFC 9207 `iss` 参数。
- Chrome 会把 `form-action` 应用于表单提交后的整条重定向链。只放行 Google callback origin 时，callback 后继续回到 Gemini 的跳转仍会被浏览器拦截，并把地址栏留在授权页。授权页现允许 HTTPS 表单重定向链，服务端仍只会向经过精确白名单校验的 callback URI 发出 `302`。

## 最终状态

- 已在 Spark 中完成真实账号绑定并确认可正常使用；后续新增或调整 Agent 能力时，同步维护仓库 CLI、Skill、MCP 工具契约和指南。
