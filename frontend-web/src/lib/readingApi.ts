import { apiFetch } from "$lib/api";

export interface ReadingFolder {
  id: number;
  parent_id: number | null;
  name: string;
  created_at: string;
  updated_at: string;
}

export interface ReadingArticleSummary {
  id: number;
  folder_id: number | null;
  title: string;
  completed: boolean;
  bookmark_count: number;
  memory_percent: number | null;
  last_opened_at: string | null;
  created_at: string;
  updated_at: string;
}

export type ReadingContentSegment =
  | { kind: "text"; text: string }
  | {
      kind: "bookmark";
      occurrence_index: number;
      surface: string;
      form_of: string;
      lexeme_id: number | null;
      entry_id: number | null;
    };

export interface ReadingArticleDetail {
  id: number;
  folder_id: number | null;
  title: string;
  body: string;
  completed: boolean;
  last_opened_at: string | null;
  created_at: string;
  updated_at: string;
  segments: ReadingContentSegment[];
  bookmark_count: number;
}

export interface ReadingIndexResponse {
  folders: ReadingFolder[];
  articles: ReadingArticleSummary[];
}

export interface BookmarkValidationIssue {
  occurrence_index: number;
  surface: string;
  form_of: string;
  reason: string;
}

export interface ReadingErrorPayload {
  message: string;
  bookmarks?: BookmarkValidationIssue[];
}

export function getReadingIndex() {
  return apiFetch<ReadingIndexResponse>("/reading");
}

export function getReadingArticle(articleId: number) {
  return apiFetch<ReadingArticleDetail>(`/reading/articles/${articleId}`);
}

export function createReadingArticle(payload: { title: string; body: string; folder_id: number | null }) {
  return apiFetch<ReadingArticleDetail>("/reading/articles", jsonRequest("POST", payload));
}

export function saveReadingArticle(
  articleId: number,
  payload: { title: string; body: string; folder_id: number | null },
) {
  return apiFetch<ReadingArticleDetail>(`/reading/articles/${articleId}`, jsonRequest("PUT", payload));
}

export function renameReadingArticle(articleId: number, title: string) {
  return apiFetch<{ id: number }>(`/reading/articles/${articleId}/title`, jsonRequest("PUT", { title }));
}

export function moveReadingArticle(articleId: number, folderId: number | null) {
  return apiFetch<{ id: number }>(`/reading/articles/${articleId}/folder`, jsonRequest("PUT", { folder_id: folderId }));
}

export function setReadingArticleStatus(articleId: number, completed: boolean) {
  return apiFetch<{ id: number }>(`/reading/articles/${articleId}/status`, jsonRequest("PUT", { completed }));
}

export function markReadingArticleOpened(articleId: number) {
  return apiFetch<{ id: number }>(`/reading/articles/${articleId}/opened`, { method: "PUT" });
}

export function deleteReadingArticle(articleId: number) {
  return apiFetch<{ id: number }>(`/reading/articles/${articleId}`, { method: "DELETE" });
}

export function createReadingFolder(name: string, parentId: number | null) {
  return apiFetch<ReadingFolder>("/reading/folders", jsonRequest("POST", { name, parent_id: parentId }));
}

export function updateReadingFolder(folderId: number, name: string, parentId: number | null) {
  return apiFetch<ReadingFolder>(`/reading/folders/${folderId}`, jsonRequest("PUT", { name, parent_id: parentId }));
}

export function deleteReadingFolder(folderId: number) {
  return apiFetch<{ id: number }>(`/reading/folders/${folderId}`, { method: "DELETE" });
}

function jsonRequest(method: string, body: unknown): RequestInit {
  return {
    method,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  };
}
