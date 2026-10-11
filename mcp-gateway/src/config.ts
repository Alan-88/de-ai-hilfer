export interface GatewayConfig {
  publicUrl: URL;
  mcpUrl: URL;
  port: number;
  clientId: string;
  clientSecret: string;
  redirectUris: ReadonlySet<string>;
  redirectHosts: ReadonlySet<string>;
  ownerPassword: string;
  tokenSigningSecret: string;
  backendUrl: string;
  backendToken: string;
  allowInsecureIssuer: boolean;
}

export function loadConfig(env: NodeJS.ProcessEnv = process.env): GatewayConfig {
  const publicUrl = normalizePublicUrl(required(env, "MCP_PUBLIC_URL"));
  const port = positiveInteger(env.MCP_PORT ?? "3000", "MCP_PORT");
  const redirectUris = new Set(commaSeparated(env.MCP_OAUTH_REDIRECT_URIS).map(normalizeRedirectUri));
  const redirectHosts = new Set(commaSeparated(env.MCP_OAUTH_REDIRECT_HOSTS).map(normalizeRedirectHost));
  if (!redirectUris.size && !redirectHosts.size) {
    throw new Error("MCP_OAUTH_REDIRECT_URIS or MCP_OAUTH_REDIRECT_HOSTS is required");
  }
  const tokenSigningSecret = required(env, "MCP_TOKEN_SIGNING_SECRET");
  if (Buffer.byteLength(tokenSigningSecret) < 32) {
    throw new Error("MCP_TOKEN_SIGNING_SECRET must contain at least 32 bytes");
  }

  const allowInsecureIssuer = publicUrl.protocol === "http:";
  if (allowInsecureIssuer && env.MCP_ALLOW_INSECURE_LOCAL !== "1") {
    throw new Error("MCP_PUBLIC_URL must use HTTPS unless MCP_ALLOW_INSECURE_LOCAL=1");
  }

  const clientSecret = required(env, "MCP_OAUTH_CLIENT_SECRET");
  const ownerPassword = required(env, "MCP_OWNER_PASSWORD");
  if (Buffer.byteLength(clientSecret) < 24) {
    throw new Error("MCP_OAUTH_CLIENT_SECRET must contain at least 24 bytes");
  }
  if (Buffer.byteLength(ownerPassword) < 12) {
    throw new Error("MCP_OWNER_PASSWORD must contain at least 12 bytes");
  }

  return {
    publicUrl,
    mcpUrl: new URL("/mcp", publicUrl),
    port,
    clientId: required(env, "MCP_OAUTH_CLIENT_ID"),
    clientSecret,
    redirectUris,
    redirectHosts,
    ownerPassword,
    tokenSigningSecret,
    backendUrl: required(env, "DEAI_API_URL").replace(/\/+$/, ""),
    backendToken: required(env, "DEAI_API_TOKEN"),
    allowInsecureIssuer,
  };
}

function commaSeparated(raw: string | undefined): string[] {
  return (raw ?? "")
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean);
}

function required(env: NodeJS.ProcessEnv, name: string): string {
  const value = env[name]?.trim();
  if (!value) throw new Error(`${name} is required`);
  return value;
}

function positiveInteger(raw: string, name: string): number {
  const value = Number(raw);
  if (!Number.isSafeInteger(value) || value <= 0 || value > 65_535) {
    throw new Error(`${name} must be a valid TCP port`);
  }
  return value;
}

function normalizePublicUrl(raw: string): URL {
  const url = new URL(raw);
  if (url.username || url.password || url.search || url.hash) {
    throw new Error("MCP_PUBLIC_URL must not contain credentials, query parameters or a fragment");
  }
  url.pathname = "/";
  return url;
}

function normalizeRedirectUri(raw: string): string {
  const url = new URL(raw);
  if (!url.protocol.startsWith("http")) throw new Error(`invalid redirect URI: ${raw}`);
  if (url.hash) throw new Error(`redirect URI must not contain a fragment: ${raw}`);
  return url.href;
}

function normalizeRedirectHost(raw: string): string {
  const normalized = raw.toLowerCase();
  const url = new URL(`https://${normalized}`);
  if (url.hostname !== normalized || url.username || url.password || url.port || url.pathname !== "/") {
    throw new Error(`invalid redirect host: ${raw}`);
  }
  return normalized;
}
