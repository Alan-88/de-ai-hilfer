import { apiFetch, apiUrl } from "$lib/api";
import type {
  AnalyzeResponse,
  AttachPhraseRequest,
  DatabaseImportResponse,
  DetachPhraseRequest,
  EntryDeleteResponse,
  EntryDetailResponse,
  IntelligentSearchRequest,
  LibraryEntriesPageResponse,
  LibraryTab,
  RecentItem,
  StatusResponse,
  SuggestionResponse,
} from "$lib/types";

const API_BASE = "";

export function analyzeWord(queryText: string) {
  return apiFetch<AnalyzeResponse>(`${API_BASE}/analyze`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query_text: queryText }),
  });
}

export function getRecentEntries() {
  return apiFetch<RecentItem[]>(`${API_BASE}/entries/recent`);
}

export function getAllEntries() {
  return apiFetch<RecentItem[]>(`${API_BASE}/entries/all`);
}

export function getLibraryEntriesPage(params: {
  q?: string;
  tab?: LibraryTab;
  cursor?: string | null;
  limit?: number;
}) {
  const searchParams = new URLSearchParams();
  if (params.q?.trim()) searchParams.set("q", params.q.trim());
  if (params.tab && params.tab !== "all") searchParams.set("tab", params.tab);
  if (params.cursor) searchParams.set("cursor", params.cursor);
  if (params.limit) searchParams.set("limit", String(params.limit));
  const suffix = searchParams.toString();
  return apiFetch<LibraryEntriesPageResponse>(
    `${API_BASE}/entries${suffix ? `?${suffix}` : ""}`,
  );
}

export function getEntryDetail(entryId: number) {
  return apiFetch<EntryDetailResponse>(`${API_BASE}/entries/${entryId}`);
}

export function deleteEntry(entryId: number) {
  return apiFetch<EntryDeleteResponse>(`${API_BASE}/entries/${entryId}`, {
    method: "DELETE",
  });
}

export function getSuggestions(query: string) {
  return apiFetch<SuggestionResponse>(`${API_BASE}/suggestions?q=${encodeURIComponent(query)}`);
}

export function getServerStatus() {
  return apiFetch<StatusResponse>(`${API_BASE}/status`);
}

export function intelligentSearch(payload: IntelligentSearchRequest) {
  return apiFetch<AnalyzeResponse>(`${API_BASE}/intelligent_search`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
}

export function attachPhraseToHost(payload: AttachPhraseRequest) {
  return apiFetch<AnalyzeResponse>(`${API_BASE}/phrases/attach`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
}

export function detachPhraseFromHost(payload: DetachPhraseRequest) {
  return apiFetch<AnalyzeResponse>(`${API_BASE}/phrases/detach`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
}

export function getDatabaseExportUrl() {
  return apiUrl(`${API_BASE}/database/export`);
}

export function importDatabaseBackup(file: File) {
  const formData = new FormData();
  formData.append("backup_file", file);
  return apiFetch<DatabaseImportResponse>(`${API_BASE}/database/import`, {
    method: "POST",
    body: formData,
  });
}
