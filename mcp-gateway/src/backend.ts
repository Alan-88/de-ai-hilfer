const DEFAULT_TIMEOUT_MS = 60_000;
const INITIALIZE_TIMEOUT_MS = 10 * 60_000;
const READ_RETRIES = 2;

export type FetchImplementation = typeof fetch;

export class DeaiApi {
  readonly #baseUrl: string;
  readonly #token: string;
  readonly #fetch: FetchImplementation;

  constructor(baseUrl: string, token: string, fetchImplementation: FetchImplementation = fetch) {
    this.#baseUrl = baseUrl.replace(/\/+$/, "");
    this.#token = token;
    this.#fetch = fetchImplementation;
  }

  get(path: string): Promise<unknown> {
    return this.request(path);
  }

  post(path: string, body?: unknown, timeoutMs = DEFAULT_TIMEOUT_MS): Promise<unknown> {
    return this.request(path, {
      method: "POST",
      timeoutMs,
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
  }

  put(path: string, body?: unknown): Promise<unknown> {
    return this.request(path, {
      method: "PUT",
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
  }

  delete(path: string): Promise<unknown> {
    return this.request(path, { method: "DELETE" });
  }

  initializeLexeme(lexemeId: number, body: unknown): Promise<unknown> {
    return this.post(`/agent/lexemes/${lexemeId}/initialize`, body, INITIALIZE_TIMEOUT_MS);
  }

  async entryIdForLexeme(lexemeId: number): Promise<number> {
    const state = await this.get(`/agent/lexemes/${lexemeId}`);
    const entryId = readNestedNumber(state, "knowledge", "entry_id");
    if (!entryId) {
      throw new Error(`lexeme ${lexemeId} is not initialized`);
    }
    return entryId;
  }

  private async request(
    path: string,
    options: { method?: string; body?: string; timeoutMs?: number } = {},
  ): Promise<unknown> {
    const method = (options.method ?? "GET").toUpperCase();
    const attempts = method === "GET" ? READ_RETRIES + 1 : 1;
    let lastError: unknown;

    for (let attempt = 0; attempt < attempts; attempt += 1) {
      try {
        const response = await this.#fetch(`${this.#baseUrl}${path}`, {
          method,
          signal: AbortSignal.timeout(options.timeoutMs ?? DEFAULT_TIMEOUT_MS),
          ...(options.body === undefined ? {} : { body: options.body }),
          headers: {
            authorization: `Bearer ${this.#token}`,
            ...(options.body === undefined ? {} : { "content-type": "application/json" }),
          },
        });
        const raw = await response.text();
        const payload = parsePayload(raw);
        if (!response.ok) {
          throw new BackendResponseError(response.status, payload);
        }
        return payload;
      } catch (error) {
        lastError = error;
        if (error instanceof BackendResponseError || attempt + 1 >= attempts) {
          throw normalizeRequestError(method, path, error, attempt + 1);
        }
        await delay(250 * 2 ** attempt);
      }
    }

    throw normalizeRequestError(method, path, lastError, attempts);
  }
}

export async function runMany<T>(
  inputs: readonly T[],
  operation: (input: T) => Promise<unknown>,
): Promise<{ results: Array<{ input: T; ok: true; value: unknown } | { input: T; ok: false; error: string }> }> {
  const results: Array<
    { input: T; ok: true; value: unknown } | { input: T; ok: false; error: string }
  > = [];
  for (const input of inputs) {
    try {
      results.push({ input, ok: true, value: await operation(input) });
    } catch (error) {
      results.push({ input, ok: false, error: errorMessage(error) });
    }
  }
  return { results };
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

class BackendResponseError extends Error {
  constructor(
    readonly status: number,
    readonly payload: unknown,
  ) {
    super(`${status} ${formatPayload(payload)}`);
  }
}

function normalizeRequestError(method: string, path: string, error: unknown, attempts: number): Error {
  if (error instanceof BackendResponseError) {
    return new Error(`${method} ${path}: ${error.message}`);
  }
  const cause = error instanceof Error && "cause" in error ? error.cause : undefined;
  const code = isRecord(cause) && typeof cause.code === "string" ? `${cause.code}: ` : "";
  const message = isRecord(cause) && typeof cause.message === "string" ? cause.message : errorMessage(error);
  return new Error(`${method} ${path}: ${code}${message} after ${attempts} attempt(s)`);
}

function parsePayload(raw: string): unknown {
  if (!raw) return null;
  try {
    return JSON.parse(raw) as unknown;
  } catch {
    return raw;
  }
}

function formatPayload(payload: unknown): string {
  return typeof payload === "string" ? payload : JSON.stringify(payload);
}

function readNestedNumber(value: unknown, parent: string, child: string): number | undefined {
  if (!isRecord(value) || !isRecord(value[parent])) return undefined;
  const candidate = value[parent][child];
  return typeof candidate === "number" && Number.isSafeInteger(candidate) && candidate > 0
    ? candidate
    : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
