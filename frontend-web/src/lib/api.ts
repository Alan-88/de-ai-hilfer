const defaultBaseUrl = "/api/v1";

export const API_BASE_URL =
  (typeof window !== "undefined" && window.localStorage.getItem("de-ai-api-base")) ||
  defaultBaseUrl;

export function apiUrl(path: string) {
  return `${API_BASE_URL}${path}`;
}

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(apiUrl(path), init);
  if (!response.ok) {
    let detail = `Request failed: ${response.status}`;
    try {
      const error = await response.json();
      detail = error.detail || error.message || detail;
    } catch {
      // ignore json decode failure
    }
    throw new Error(detail);
  }

  return (await response.json()) as T;
}
