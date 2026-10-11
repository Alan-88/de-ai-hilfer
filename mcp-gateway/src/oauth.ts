import {
  createHash,
  createHmac,
  randomBytes,
  timingSafeEqual,
} from "node:crypto";

import {
  OAuthError,
  OAuthErrorCode,
  type AuthInfo,
  type OAuthMetadata,
  type OAuthTokenVerifier,
} from "@modelcontextprotocol/server";
import type { Request, Response } from "express";

import type { GatewayConfig } from "./config.js";

const AUTHORIZATION_CODE_TTL_SECONDS = 5 * 60;
const ACCESS_TOKEN_TTL_SECONDS = 60 * 60;
const REFRESH_TOKEN_TTL_SECONDS = 30 * 24 * 60 * 60;
const SUPPORTED_SCOPE = "mcp";
const AUTHORIZATION_FAILURE_WINDOW_MS = 10 * 60_000;
const MAX_AUTHORIZATION_FAILURES = 5;

interface AuthorizationRequest {
  clientId: string;
  redirectUri: string;
  codeChallenge: string;
  state?: string;
  resource?: string;
  scope: string;
}

interface AuthorizationCode extends AuthorizationRequest {
  expiresAt: number;
}

interface SignedTokenPayload {
  typ: "access" | "refresh";
  iss: string;
  aud: string;
  cid: string;
  sub: string;
  scope: string;
  iat: number;
  exp: number;
  jti: string;
}

export class OAuthService {
  readonly metadata: OAuthMetadata;
  readonly verifier: OAuthTokenVerifier;
  readonly #config: GatewayConfig;
  readonly #codes = new Map<string, AuthorizationCode>();
  readonly #authorizationFailures = new Map<string, number[]>();

  constructor(config: GatewayConfig) {
    this.#config = config;
    this.metadata = {
      issuer: issuer(config),
      authorization_endpoint: new URL("/authorize", config.publicUrl).href,
      token_endpoint: new URL("/token", config.publicUrl).href,
      response_types_supported: ["code"],
      grant_types_supported: ["authorization_code", "refresh_token"],
      code_challenge_methods_supported: ["S256"],
      token_endpoint_auth_methods_supported: ["client_secret_basic", "client_secret_post"],
      scopes_supported: [SUPPORTED_SCOPE],
    };
    this.verifier = {
      verifyAccessToken: async (token) => this.verifyAccessToken(token),
    };
  }

  authorizePage = (request: Request, response: Response): void => {
    secureAuthorizationResponse(response);
    try {
      const authorization = this.parseAuthorizationRequest(request.query);
      response
        .status(200)
        .type("html")
        .send(renderAuthorizationPage(authorization));
    } catch (error) {
      response.status(400).type("text").send(errorMessage(error));
    }
  };

  approveAuthorization = (request: Request, response: Response): void => {
    secureAuthorizationResponse(response);
    try {
      const authorization = this.parseAuthorizationRequest(request.body);
      const source = request.ip || request.socket.remoteAddress || "unknown";
      if (this.isAuthorizationBlocked(source)) {
        response.set("retry-after", String(AUTHORIZATION_FAILURE_WINDOW_MS / 1000));
        response.status(429).type("text").send("Too many failed authorization attempts. Try again later.");
        return;
      }
      const password = singleString(request.body?.password, "password");
      if (!safeEqual(password, this.#config.ownerPassword)) {
        this.recordAuthorizationFailure(source);
        response.status(401).type("html").send(renderAuthorizationPage(authorization, "Password rejected."));
        return;
      }
      this.#authorizationFailures.delete(source);

      this.cleanupCodes();
      const code = randomBytes(32).toString("base64url");
      this.#codes.set(code, {
        ...authorization,
        expiresAt: epochSeconds() + AUTHORIZATION_CODE_TTL_SECONDS,
      });

      const callback = new URL(authorization.redirectUri);
      callback.searchParams.set("code", code);
      if (authorization.state) callback.searchParams.set("state", authorization.state);
      response.redirect(302, callback.href);
    } catch (error) {
      response.status(400).type("text").send(errorMessage(error));
    }
  };

  token = (request: Request, response: Response): void => {
    response.set({ "cache-control": "no-store", pragma: "no-cache" });
    try {
      this.verifyClient(request);
      const grantType = singleString(request.body?.grant_type, "grant_type");
      if (grantType === "authorization_code") {
        this.exchangeAuthorizationCode(request, response);
        return;
      }
      if (grantType === "refresh_token") {
        this.exchangeRefreshToken(request, response);
        return;
      }
      sendOAuthError(response, 400, "unsupported_grant_type", "Only authorization_code and refresh_token are supported.");
    } catch (error) {
      const status = error instanceof ClientAuthenticationError ? 401 : 400;
      if (status === 401) response.set("www-authenticate", 'Basic realm="token"');
      sendOAuthError(response, status, status === 401 ? "invalid_client" : "invalid_request", errorMessage(error));
    }
  };

  private parseAuthorizationRequest(source: Record<string, unknown>): AuthorizationRequest {
    const responseType = singleString(source.response_type, "response_type");
    if (responseType !== "code") throw new Error("response_type must be code");

    const clientId = singleString(source.client_id, "client_id");
    if (!safeEqual(clientId, this.#config.clientId)) throw new Error("unknown client_id");

    const redirectUri = normalizeUrl(singleString(source.redirect_uri, "redirect_uri"));
    if (!isAllowedRedirectUri(redirectUri, this.#config)) throw new Error("redirect_uri is not registered");

    const codeChallengeMethod = singleString(source.code_challenge_method, "code_challenge_method");
    if (codeChallengeMethod !== "S256") throw new Error("code_challenge_method must be S256");
    const codeChallenge = singleString(source.code_challenge, "code_challenge");
    if (!/^[A-Za-z0-9_-]{43,128}$/.test(codeChallenge)) throw new Error("invalid code_challenge");

    const scope = optionalString(source.scope) ?? SUPPORTED_SCOPE;
    const scopes = scope.split(/\s+/).filter(Boolean);
    if (scopes.length !== 1 || scopes[0] !== SUPPORTED_SCOPE) throw new Error("unsupported scope");

    const resource = optionalString(source.resource);
    if (resource && !sameUrl(resource, this.#config.mcpUrl.href)) throw new Error("resource does not match this MCP server");

    const state = optionalString(source.state);
    return {
      clientId,
      redirectUri,
      codeChallenge,
      scope,
      ...(state ? { state } : {}),
      ...(resource ? { resource } : {}),
    };
  }

  private verifyClient(request: Request): void {
    const basic = request.header("authorization");
    let clientId: string;
    let clientSecret: string;
    if (basic?.startsWith("Basic ")) {
      const decoded = Buffer.from(basic.slice(6), "base64").toString("utf8");
      const separator = decoded.indexOf(":");
      if (separator < 0) throw new ClientAuthenticationError("invalid Basic client credentials");
      clientId = decoded.slice(0, separator);
      clientSecret = decoded.slice(separator + 1);
    } else {
      clientId = singleString(request.body?.client_id, "client_id");
      clientSecret = singleString(request.body?.client_secret, "client_secret");
    }
    if (!safeEqual(clientId, this.#config.clientId) || !safeEqual(clientSecret, this.#config.clientSecret)) {
      throw new ClientAuthenticationError("client authentication failed");
    }
  }

  private exchangeAuthorizationCode(request: Request, response: Response): void {
    const code = singleString(request.body?.code, "code");
    const authorization = this.#codes.get(code);
    this.#codes.delete(code);
    if (!authorization || authorization.expiresAt < epochSeconds()) {
      sendOAuthError(response, 400, "invalid_grant", "authorization code is invalid or expired");
      return;
    }

    const redirectUri = normalizeUrl(singleString(request.body?.redirect_uri, "redirect_uri"));
    const verifier = singleString(request.body?.code_verifier, "code_verifier");
    const challenge = createHash("sha256").update(verifier).digest("base64url");
    if (redirectUri !== authorization.redirectUri || !safeEqual(challenge, authorization.codeChallenge)) {
      sendOAuthError(response, 400, "invalid_grant", "redirect URI or PKCE verifier does not match");
      return;
    }
    this.sendTokens(response, authorization.scope);
  }

  private exchangeRefreshToken(request: Request, response: Response): void {
    const refreshToken = singleString(request.body?.refresh_token, "refresh_token");
    let payload: SignedTokenPayload;
    try {
      payload = this.verifySignedToken(refreshToken, "refresh");
    } catch {
      sendOAuthError(response, 400, "invalid_grant", "refresh token is invalid or expired");
      return;
    }
    if (!safeEqual(payload.cid, this.#config.clientId)) {
      sendOAuthError(response, 400, "invalid_grant", "refresh token belongs to another client");
      return;
    }
    this.sendTokens(response, payload.scope);
  }

  private sendTokens(response: Response, scope: string): void {
    const now = epochSeconds();
    const common = {
      iss: issuer(this.#config),
      aud: this.#config.mcpUrl.href,
      cid: this.#config.clientId,
      sub: "owner",
      scope,
      iat: now,
    };
    const accessToken = this.signToken({
      ...common,
      typ: "access",
      exp: now + ACCESS_TOKEN_TTL_SECONDS,
      jti: randomBytes(16).toString("base64url"),
    });
    const refreshToken = this.signToken({
      ...common,
      typ: "refresh",
      exp: now + REFRESH_TOKEN_TTL_SECONDS,
      jti: randomBytes(16).toString("base64url"),
    });
    response.status(200).json({
      token_type: "Bearer",
      access_token: accessToken,
      expires_in: ACCESS_TOKEN_TTL_SECONDS,
      refresh_token: refreshToken,
      scope,
    });
  }

  private verifyAccessToken(token: string): AuthInfo {
    try {
      const payload = this.verifySignedToken(token, "access");
      return {
        token,
        clientId: payload.cid,
        scopes: payload.scope.split(/\s+/).filter(Boolean),
        expiresAt: payload.exp,
        resource: new URL(payload.aud),
      };
    } catch {
      throw new OAuthError(OAuthErrorCode.InvalidToken, "access token is invalid or expired");
    }
  }

  private signToken(payload: SignedTokenPayload): string {
    const encoded = Buffer.from(JSON.stringify(payload)).toString("base64url");
    const signature = createHmac("sha256", this.#config.tokenSigningSecret).update(encoded).digest("base64url");
    return `${encoded}.${signature}`;
  }

  private verifySignedToken(token: string, type: SignedTokenPayload["typ"]): SignedTokenPayload {
    const [encoded, signature, extra] = token.split(".");
    if (!encoded || !signature || extra) throw new Error("malformed token");
    const expected = createHmac("sha256", this.#config.tokenSigningSecret).update(encoded).digest("base64url");
    if (!safeEqual(signature, expected)) throw new Error("invalid token signature");
    const payload = JSON.parse(Buffer.from(encoded, "base64url").toString("utf8")) as unknown;
    if (!isSignedTokenPayload(payload) || payload.typ !== type) throw new Error("invalid token payload");
    if (payload.exp <= epochSeconds()) throw new Error("expired token");
    if (payload.iss !== issuer(this.#config) || !sameUrl(payload.aud, this.#config.mcpUrl.href)) {
      throw new Error("token issuer or audience mismatch");
    }
    return payload;
  }

  private cleanupCodes(): void {
    const now = epochSeconds();
    for (const [code, authorization] of this.#codes) {
      if (authorization.expiresAt <= now) this.#codes.delete(code);
    }
  }

  private isAuthorizationBlocked(source: string): boolean {
    const attempts = this.recentAuthorizationFailures(source);
    return attempts.length >= MAX_AUTHORIZATION_FAILURES;
  }

  private recordAuthorizationFailure(source: string): void {
    const attempts = this.recentAuthorizationFailures(source);
    attempts.push(Date.now());
    this.#authorizationFailures.set(source, attempts);
  }

  private recentAuthorizationFailures(source: string): number[] {
    const cutoff = Date.now() - AUTHORIZATION_FAILURE_WINDOW_MS;
    const attempts = (this.#authorizationFailures.get(source) ?? []).filter((timestamp) => timestamp > cutoff);
    if (attempts.length) this.#authorizationFailures.set(source, attempts);
    else this.#authorizationFailures.delete(source);
    return attempts;
  }
}

class ClientAuthenticationError extends Error {}

function renderAuthorizationPage(request: AuthorizationRequest, error?: string): string {
  const hidden = [
    ["response_type", "code"],
    ["client_id", request.clientId],
    ["redirect_uri", request.redirectUri],
    ["code_challenge", request.codeChallenge],
    ["code_challenge_method", "S256"],
    ["scope", request.scope],
    ...(request.state ? [["state", request.state]] : []),
    ...(request.resource ? [["resource", request.resource]] : []),
  ]
    .map(([name, value]) => `<input type="hidden" name="${escapeHtml(name ?? "")}" value="${escapeHtml(value ?? "")}">`)
    .join("\n");
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Connect De-AI-Hilfer</title>
  <style>
    :root { color-scheme: light; font-family: ui-sans-serif, system-ui, sans-serif; background: #faf7f1; color: #332840; }
    body { min-height: 100vh; margin: 0; display: grid; place-items: center; }
    main { width: min(420px, calc(100vw - 40px)); padding: 32px; border: 1px solid #eadfd4; border-radius: 22px; background: #fffdfa; box-shadow: 0 22px 60px rgb(51 40 64 / 10%); }
    h1 { margin: 0 0 10px; font-size: 24px; }
    p { margin: 0 0 24px; line-height: 1.55; color: #706477; }
    label { display: grid; gap: 8px; font-weight: 650; }
    input[type=password] { box-sizing: border-box; width: 100%; padding: 12px 14px; border: 1px solid #d9cdc3; border-radius: 12px; background: #fff; color: inherit; font: inherit; }
    button { width: 100%; margin-top: 18px; padding: 12px 16px; border: 0; border-radius: 12px; background: #e77c42; color: #fff; font: 700 15px/1 inherit; cursor: pointer; }
    .error { color: #a43c2d; }
  </style>
</head>
<body>
  <main>
    <h1>Connect De-AI-Hilfer</h1>
    <p>Authorize Gemini Spark to use your vocabulary, article and learning tools.</p>
    ${error ? `<p class="error">${escapeHtml(error)}</p>` : ""}
    <form method="post" action="/authorize">
      ${hidden}
      <label>Owner password<input name="password" type="password" required autocomplete="current-password" autofocus></label>
      <button type="submit">Authorize</button>
    </form>
  </main>
</body>
</html>`;
}

function singleString(value: unknown, name: string): string {
  if (typeof value !== "string" || !value) throw new Error(`${name} is required`);
  return value;
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" && value ? value : undefined;
}

function normalizeUrl(value: string): string {
  return new URL(value).href;
}

function isAllowedRedirectUri(value: string, config: GatewayConfig): boolean {
  if (config.redirectUris.has(value)) return true;

  const url = new URL(value);
  return (
    url.protocol === "https:" &&
    !url.username &&
    !url.password &&
    !url.port &&
    !url.hash &&
    config.redirectHosts.has(url.hostname) &&
    url.pathname.startsWith("/r/")
  );
}

function sameUrl(left: string, right: string): boolean {
  return new URL(left).href.replace(/\/$/, "") === new URL(right).href.replace(/\/$/, "");
}

function safeEqual(left: string, right: string): boolean {
  const leftBuffer = Buffer.from(left);
  const rightBuffer = Buffer.from(right);
  return leftBuffer.length === rightBuffer.length && timingSafeEqual(leftBuffer, rightBuffer);
}

function epochSeconds(): number {
  return Math.floor(Date.now() / 1000);
}

function issuer(config: GatewayConfig): string {
  return config.publicUrl.href.replace(/\/$/, "");
}

function escapeHtml(value: string): string {
  return value.replace(/[&<>"]/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[character] ?? character);
}

function sendOAuthError(response: Response, status: number, error: string, description: string): void {
  response.status(status).json({ error, error_description: description });
}

function secureAuthorizationResponse(response: Response): void {
  response.set({
    "cache-control": "no-store",
    "content-security-policy": "default-src 'none'; style-src 'unsafe-inline'; form-action 'self' https:; frame-ancestors 'none'; base-uri 'none'",
    "referrer-policy": "no-referrer",
    "x-content-type-options": "nosniff",
    "x-frame-options": "DENY",
  });
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function isSignedTokenPayload(value: unknown): value is SignedTokenPayload {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const payload = value as Record<string, unknown>;
  return (
    (payload.typ === "access" || payload.typ === "refresh") &&
    typeof payload.iss === "string" &&
    typeof payload.aud === "string" &&
    typeof payload.cid === "string" &&
    typeof payload.sub === "string" &&
    typeof payload.scope === "string" &&
    typeof payload.iat === "number" &&
    typeof payload.exp === "number" &&
    typeof payload.jti === "string"
  );
}
