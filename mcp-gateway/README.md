# De-AI-Hilfer MCP gateway

Remote Streamable HTTP MCP adapter for Gemini Spark. The gateway owns OAuth and
tool schemas, then forwards operations to the existing De-AI-Hilfer HTTP API.
It does not access PostgreSQL or implement vocabulary and learning logic.

## Agent guidance

The initialization response carries short server instructions that establish
the article-first workflow, the `lexeme_id` / `entry_id` boundary and the rule
that writes require explicit confirmation.

Detailed guidance is available on demand through `deai_guide` with these
topics:

- `overview`
- `article_learning`
- `vocabulary_workflow`
- `learning_v3`
- `learning_diagnostics`
- `learning_corrections`

`learning_query(action=explain)` remains available and reuses the same detailed
V3 scheduler document as `deai_guide(topic=learning_v3)`.

## Configuration

Copy `.env.example` to `.env` and fill every secret. Gemini Spark uses
`MCP_OAUTH_CLIENT_ID` and `MCP_OAUTH_CLIENT_SECRET` through its advanced MCP
connection settings. Spark does not expose a callback field: it supplies a
Google-owned `redirect_uri` automatically when authorization starts. If Spark
offers **Copy redirect URI**, paste that exact value into
`MCP_OAUTH_REDIRECT_URIS`; this is the preferred production setup. When an
exact value is unavailable, `MCP_OAUTH_REDIRECT_HOSTS` can admit
`oauth-redirect.googleusercontent.com` only over HTTPS and only below `/r/`.

The OAuth flow is Authorization Code with PKCE `S256`. The authorization page
asks for `MCP_OWNER_PASSWORD`; authorization codes are single-use and expire
after five minutes. Failed password attempts are limited per source address.
Access and refresh tokens are signed with
`MCP_TOKEN_SIGNING_SECRET`, so container restarts do not disconnect Spark.

## Development

```bash
npm install
npm run check
npm run dev
```

Container deployment is isolated behind a Compose profile so incomplete OAuth
configuration cannot affect the existing application:

```bash
docker compose --env-file mcp-gateway/.env --profile mcp up -d --build mcp-gateway
```

The MCP endpoint is `/mcp`, health is `/health`, and OAuth metadata is exposed
through the standard well-known endpoints. Dynamic Client Registration is not
enabled; use Spark's advanced static-client settings.

The production deployment shares `https://de.alan-studio.uk` with the web app.
Caddy sends `/mcp`, `/authorize`, `/token`, and `/.well-known/oauth-*` to this
gateway and leaves every other path on the existing frontend.
