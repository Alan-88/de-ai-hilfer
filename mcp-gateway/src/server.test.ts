import { createHash } from "node:crypto";
import { createServer, type Server } from "node:http";
import { once } from "node:events";
import test from "node:test";
import assert from "node:assert/strict";

import { Client, StreamableHTTPClientTransport } from "@modelcontextprotocol/client";

import type { GatewayConfig } from "./config.js";
import { createGateway } from "./server.js";

test("OAuth-protected MCP exposes the Skill-aligned tools and forwards a lexeme query", async (context) => {
  const backend = createServer((request, response) => {
    assert.equal(request.headers.authorization, "Bearer backend-token");
    const url = new URL(request.url ?? "/", "http://backend.test");
    if (url.pathname === "/api/v1/agent/lexemes/resolve") {
      response.setHeader("content-type", "application/json");
      response.end(JSON.stringify({ surface: url.searchParams.get("q"), candidates: [{ id: 123, headword: "gehen" }] }));
      return;
    }
    response.statusCode = 404;
    response.end("not found");
  });
  await listen(backend);
  context.after(() => backend.close());

  const gatewayPort = await freePort();
  const callback = `http://127.0.0.1:${gatewayPort}/spark-callback`;
  const publicUrl = new URL(`http://127.0.0.1:${gatewayPort}/`);
  const config: GatewayConfig = {
    publicUrl,
    mcpUrl: new URL("/mcp", publicUrl),
    port: gatewayPort,
    clientId: "spark-client",
    clientSecret: "spark-secret",
    redirectUris: new Set([callback]),
    redirectHosts: new Set(["oauth-redirect.googleusercontent.com"]),
    ownerPassword: "owner-password",
    tokenSigningSecret: "test-signing-secret-with-at-least-32-bytes",
    backendUrl: `http://127.0.0.1:${addressPort(backend)}/api/v1`,
    backendToken: "backend-token",
    allowInsecureIssuer: true,
  };
  const gateway = createGateway(config).listen(gatewayPort, "127.0.0.1");
  await once(gateway, "listening");
  context.after(() => gateway.close());

  const metadataResponse = await fetch(`${publicUrl.href}.well-known/oauth-authorization-server`);
  assert.equal(metadataResponse.status, 200);
  const metadata = (await metadataResponse.json()) as { code_challenge_methods_supported?: string[] };
  assert.deepEqual(metadata.code_challenge_methods_supported, ["S256"]);

  const verifier = "this-is-a-valid-pkce-code-verifier-with-more-than-43-characters";
  const challenge = createHash("sha256").update(verifier).digest("base64url");
  const authorization = new URL("/authorize", publicUrl);
  authorization.search = new URLSearchParams({
    response_type: "code",
    client_id: config.clientId,
    redirect_uri: callback,
    code_challenge: challenge,
    code_challenge_method: "S256",
    scope: "mcp",
    state: "spark-state",
    resource: config.mcpUrl.href,
  }).toString();
  const page = await fetch(authorization);
  assert.equal(page.status, 200);
  assert.match(await page.text(), /Connect De-AI-Hilfer/);
  assert.match(page.headers.get("content-security-policy") ?? "", /form-action 'self' https:/);

  const approvalBody = new URLSearchParams(authorization.searchParams);
  approvalBody.set("password", "wrong-password");
  for (let attempt = 0; attempt < 5; attempt += 1) {
    const rejected = await fetch(authorization, {
      method: "POST",
      redirect: "manual",
      headers: {
        "content-type": "application/x-www-form-urlencoded",
        "x-forwarded-for": "203.0.113.10",
      },
      body: approvalBody,
    });
    assert.equal(rejected.status, 401);
  }
  const limited = await fetch(authorization, {
    method: "POST",
    redirect: "manual",
    headers: {
      "content-type": "application/x-www-form-urlencoded",
      "x-forwarded-for": "203.0.113.10",
    },
    body: approvalBody,
  });
  assert.equal(limited.status, 429);

  approvalBody.set("password", config.ownerPassword);
  const approval = await fetch(authorization, {
    method: "POST",
    redirect: "manual",
    headers: {
      "content-type": "application/x-www-form-urlencoded",
      origin: "null",
    },
    body: approvalBody,
  });
  assert.equal(approval.status, 302);
  const callbackUrl = new URL(requiredHeader(approval, "location"));
  assert.equal(callbackUrl.searchParams.get("state"), "spark-state");
  assert.equal(callbackUrl.searchParams.get("iss"), null);

  const tokenResponse = await fetch(new URL("/token", publicUrl), {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      grant_type: "authorization_code",
      client_id: config.clientId,
      client_secret: config.clientSecret,
      code: callbackUrl.searchParams.get("code") ?? "",
      redirect_uri: callback,
      code_verifier: verifier,
    }),
  });
  assert.equal(tokenResponse.status, 200);
  const tokens = (await tokenResponse.json()) as { access_token: string; refresh_token: string };
  assert.ok(tokens.access_token);
  assert.ok(tokens.refresh_token);

  for (const origin of ["null", "https://evil.example"]) {
    const rejectedOrigin = await fetch(config.mcpUrl, {
      method: "POST",
      headers: {
        authorization: `Bearer ${tokens.access_token}`,
        "content-type": "application/json",
        origin,
      },
      body: "{}",
    });
    assert.equal(rejectedOrigin.status, 403, origin);
  }

  const client = new Client({ name: "gateway-test", version: "1.0.0" }, { capabilities: {} });
  const transport = new StreamableHTTPClientTransport(config.mcpUrl, {
    authProvider: { token: async () => tokens.access_token },
  });
  await client.connect(transport);
  context.after(() => transport.close());

  assert.match(client.getInstructions() ?? "", /article-first German learning system/);

  const listed = await client.listTools();
  assert.deepEqual(
    listed.tools.map((tool) => tool.name).sort(),
    [
      "article_import",
      "article_query",
      "batch_query",
      "batch_retry",
      "batch_submit",
      "deai_guide",
      "follow_up_generate",
      "follow_up_query",
      "follow_up_write",
      "learning_correction",
      "learning_membership",
      "learning_query",
      "lexeme_initialize",
      "lexeme_query",
    ],
  );

  const result = await client.callTool({
    name: "lexeme_query",
    arguments: { action: "resolve", surfaces: ["ging"] },
  });
  assert.equal(result.isError, undefined);
  const text = result.content.find((item) => item.type === "text");
  assert.ok(text && text.type === "text");
  assert.match(text.text, /"headword": "gehen"/);

  const overview = await client.callTool({ name: "deai_guide", arguments: { topic: "overview" } });
  const overviewText = overview.content.find((item) => item.type === "text");
  assert.ok(overviewText && overviewText.type === "text");
  const overviewGuide = JSON.parse(overviewText.text) as { topic?: string; concepts?: string[] };
  assert.equal(overviewGuide.topic, "overview");
  assert.ok(overviewGuide.concepts?.some((concept) => concept.includes("lexeme_id")));

  const guide = await client.callTool({ name: "learning_query", arguments: { action: "explain" } });
  const guideText = guide.content.find((item) => item.type === "text");
  assert.ok(guideText && guideText.type === "text");
  assert.match(guideText.text, /active intraday workset target is 10 cards/);
});

test("OAuth accepts only constrained redirects on a trusted Spark callback host", async (context) => {
  const gatewayPort = await freePort();
  const publicUrl = new URL(`http://127.0.0.1:${gatewayPort}/`);
  const config: GatewayConfig = {
    publicUrl,
    mcpUrl: new URL("/mcp", publicUrl),
    port: gatewayPort,
    clientId: "spark-client",
    clientSecret: "spark-secret",
    redirectUris: new Set(),
    redirectHosts: new Set(["oauth-redirect.googleusercontent.com"]),
    ownerPassword: "owner-password",
    tokenSigningSecret: "test-signing-secret-with-at-least-32-bytes",
    backendUrl: "http://backend.test/api/v1",
    backendToken: "backend-token",
    allowInsecureIssuer: true,
  };
  const gateway = createGateway(config).listen(gatewayPort, "127.0.0.1");
  await once(gateway, "listening");
  context.after(() => gateway.close());

  const accepted = await fetch(
    authorizationUrl(publicUrl, config, "https://oauth-redirect.googleusercontent.com/r/spark-client"),
  );
  assert.equal(accepted.status, 200);

  const rejected = [
    "http://oauth-redirect.googleusercontent.com/r/spark-client",
    "https://oauth-redirect.googleusercontent.com/not-r/spark-client",
    "https://oauth-redirect.googleusercontent.com.evil.example/r/spark-client",
    "https://evil-oauth-redirect.googleusercontent.com/r/spark-client",
  ];
  for (const redirectUri of rejected) {
    const response = await fetch(authorizationUrl(publicUrl, config, redirectUri));
    assert.equal(response.status, 400, redirectUri);
  }
});

async function listen(server: Server): Promise<void> {
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
}

async function freePort(): Promise<number> {
  const server = createServer();
  await listen(server);
  const port = addressPort(server);
  await new Promise<void>((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
  return port;
}

function addressPort(server: Server): number {
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("server does not have a TCP address");
  return address.port;
}

function requiredHeader(response: Response, name: string): string {
  const value = response.headers.get(name);
  if (!value) throw new Error(`missing ${name} header`);
  return value;
}

function authorizationUrl(publicUrl: URL, config: GatewayConfig, redirectUri: string): URL {
  const verifier = "this-is-a-valid-pkce-code-verifier-with-more-than-43-characters";
  const url = new URL("/authorize", publicUrl);
  url.search = new URLSearchParams({
    response_type: "code",
    client_id: config.clientId,
    redirect_uri: redirectUri,
    code_challenge: createHash("sha256").update(verifier).digest("base64url"),
    code_challenge_method: "S256",
    scope: "mcp",
    resource: config.mcpUrl.href,
  }).toString();
  return url;
}
