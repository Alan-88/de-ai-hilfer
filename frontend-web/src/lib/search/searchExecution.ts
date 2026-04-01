import { normalizeSearchText } from "$lib/search/match";
import type { DBSuggestion, QualityMode, RecentItem } from "$lib/types";

interface DirectAnalyzeParams {
  query: string;
  qualityMode: QualityMode;
  forceRefresh: boolean;
  recentItems: RecentItem[];
  suggestions: DBSuggestion[];
}

export function shouldPreferDirectAnalyze({
  query,
  qualityMode,
  forceRefresh,
  recentItems,
  suggestions,
}: DirectAnalyzeParams): boolean {
  if (forceRefresh || qualityMode !== "default") {
    return false;
  }

  const normalizedQuery = normalizeSearchText(query);
  if (!normalizedQuery) {
    return false;
  }

  if (
    recentItems.some((item) => normalizeSearchText(item.query_text) === normalizedQuery)
  ) {
    return true;
  }

  return suggestions.some((suggestion) =>
    isKnowledgeSuggestionMatch(suggestion, normalizedQuery),
  );
}

function isKnowledgeSuggestionMatch(
  suggestion: DBSuggestion,
  normalizedQuery: string,
): boolean {
  if (suggestion.source !== "知识库" || suggestion.entry_id <= 0) {
    return false;
  }

  if (normalizeSearchText(suggestion.query_text) === normalizedQuery) {
    return true;
  }

  const aliasText = extractAliasText(suggestion.preview);
  return Boolean(aliasText) && normalizeSearchText(aliasText) === normalizedQuery;
}

function extractAliasText(preview: string): string {
  if (!preview.startsWith("↪ ")) {
    return "";
  }

  const alias = preview.slice(2).split(" · ")[0]?.trim();
  return alias ?? "";
}
