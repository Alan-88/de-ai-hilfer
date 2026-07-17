const defaultBaseUrl = "/api/v1";

export const API_BASE_URL =
  (typeof window !== "undefined" && window.localStorage.getItem("de-ai-api-base")) ||
  defaultBaseUrl;

export function apiUrl(path: string) {
  return `${API_BASE_URL}${path}`;
}

export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public payload: unknown,
  ) {
    super(message);
  }
}

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(apiUrl(path), init);
  if (!response.ok) {
    let detail = `Request failed: ${response.status}`;
    let payload: unknown = null;
    try {
      payload = await response.json();
      if (payload && typeof payload === "object") {
        const error = payload as { detail?: string; message?: string };
        detail = error.detail || error.message || detail;
      }
    } catch {
      // ignore json decode failure
    }
    throw new ApiError(detail, response.status, payload);
  }

  return (await response.json()) as T;
}
