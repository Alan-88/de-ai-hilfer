import { pathToFileURL } from "node:url";

import {
  createMcpExpressApp,
  getOAuthProtectedResourceMetadataUrl,
  mcpAuthMetadataRouter,
  originValidation,
  requireBearerAuth,
} from "@modelcontextprotocol/express";
import { toNodeHandler } from "@modelcontextprotocol/node";
import { createMcpHandler } from "@modelcontextprotocol/server";
import express, { type Express } from "express";

import { DeaiApi } from "./backend.js";
import { loadConfig, type GatewayConfig } from "./config.js";
import { OAuthService } from "./oauth.js";
import { buildMcpServer } from "./tools.js";

export function createGateway(config: GatewayConfig, api = new DeaiApi(config.backendUrl, config.backendToken)): Express {
  const allowedHosts = [config.publicUrl.hostname, "localhost", "127.0.0.1"];
  const app = createMcpExpressApp({
    host: "0.0.0.0",
    allowedHosts,
    jsonLimit: "2mb",
  });
  app.disable("x-powered-by");
  app.set("trust proxy", 1);
  app.use(express.urlencoded({ extended: false, limit: "64kb" }));

  const oauth = new OAuthService(config);
  app.get("/health", (_request, response) => {
    response.status(200).json({ status: "ok", service: "de-ai-hilfer-mcp" });
  });
  app.use(
    mcpAuthMetadataRouter({
      oauthMetadata: oauth.metadata,
      resourceServerUrl: config.mcpUrl,
      scopesSupported: ["mcp"],
      resourceName: "De-AI-Hilfer",
      dangerouslyAllowInsecureIssuerUrl: config.allowInsecureIssuer,
    }),
  );
  app.get("/authorize", oauth.authorizePage);
  app.post("/authorize", oauth.approveAuthorization);
  app.post("/token", oauth.token);

  const authenticate = requireBearerAuth({
    verifier: oauth.verifier,
    requiredScopes: ["mcp"],
    resourceMetadataUrl: getOAuthProtectedResourceMetadataUrl(config.mcpUrl),
    expectedResource: config.mcpUrl,
  });
  const mcp = toNodeHandler(createMcpHandler(() => buildMcpServer(api)));
  app.all("/mcp", originValidation(allowedHosts), authenticate, (request, response) => {
    void mcp(request, response, request.body);
  });

  return app;
}

function main(): void {
  const config = loadConfig();
  const app = createGateway(config);
  app.listen(config.port, "0.0.0.0", () => {
    console.log(`De-AI-Hilfer MCP listening on 0.0.0.0:${config.port}`);
    console.log(`Public endpoint: ${config.mcpUrl.href}`);
  });
}

const entrypoint = process.argv[1] ? pathToFileURL(process.argv[1]).href : undefined;
if (entrypoint === import.meta.url) main();
