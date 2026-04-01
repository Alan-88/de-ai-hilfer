import { apiFetch } from "$lib/api";
import type { LearningProgressMapResponse, LearningProgressView, LearningSessionResponse, LearningStatsResponse, ReviewQuality } from "$lib/types";

const API_BASE = "/learning";

export function getLearningSession(limitNewWords = 5) {
  return apiFetch<LearningSessionResponse>(`${API_BASE}/session/v2?limit_new_words=${limitNewWords}`);
}

export function submitReview(entryId: number, quality: ReviewQuality) {
  return apiFetch<LearningProgressView>(`${API_BASE}/review/v2/${entryId}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ quality }),
  });
}

export function addWordToLearning(entryId: number) {
  return apiFetch<LearningProgressView>(`${API_BASE}/add/${entryId}`, {
    method: "POST",
  });
}

export function getLearningProgress() {
  return apiFetch<LearningProgressMapResponse>(`${API_BASE}/progress`);
}

export function getLearningStats() {
  return apiFetch<LearningStatsResponse>(`${API_BASE}/stats`);
}
