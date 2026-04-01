import type { RecentItem } from "$lib/types";

export function normalizeSearchText(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/ä/g, "a")
    .replace(/ö/g, "o")
    .replace(/ü/g, "u")
    .replace(/ß/g, "ss")
    .replace(/[^a-z0-9]/g, "");
}

export function rankLibraryEntries(entries: RecentItem[], query: string): RecentItem[] {
  const normalizedQuery = normalizeSearchText(query);
  if (!normalizedQuery) return entries;

  return entries
    .map((entry) => rankEntry(entry, query, normalizedQuery))
    .filter((item): item is RankedEntry => item !== null)
    .sort((left, right) =>
      left.matchTier - right.matchTier ||
      left.distance - right.distance ||
      left.entry.query_text.localeCompare(right.entry.query_text, "de")
    )
    .map((item) => item.entry);
}

interface RankedEntry {
  entry: RecentItem;
  matchTier: number;
  distance: number;
}

function rankEntry(entry: RecentItem, query: string, normalizedQuery: string): RankedEntry | null {
  const queryLower = query.trim().toLowerCase();
  const entryLower = entry.query_text.trim().toLowerCase();
  const normalizedEntry = normalizeSearchText(entry.query_text);
  if (!normalizedEntry) return null;

  const distance = levenshteinDistance(normalizedQuery, normalizedEntry);
  const threshold = fuzzyThreshold(normalizedQuery.length);

  let matchTier = -1;
  if (entryLower.startsWith(queryLower)) {
    matchTier = 0;
  } else if (normalizedEntry.startsWith(normalizedQuery)) {
    matchTier = 1;
  } else if (distance <= threshold) {
    matchTier = 2;
  }

  if (matchTier === -1) return null;
  return { entry, matchTier, distance };
}

function fuzzyThreshold(length: number): number {
  if (length <= 4) return 1;
  if (length <= 7) return 2;
  return 3;
}

function levenshteinDistance(left: string, right: string): number {
  const leftChars = Array.from(left);
  const rightChars = Array.from(right);
  const prev = Array.from({ length: rightChars.length + 1 }, (_, index) => index);
  const curr = Array.from({ length: rightChars.length + 1 }, () => 0);

  for (let i = 0; i < leftChars.length; i += 1) {
    curr[0] = i + 1;

    for (let j = 0; j < rightChars.length; j += 1) {
      const cost = leftChars[i] === rightChars[j] ? 0 : 1;
      curr[j + 1] = Math.min(prev[j + 1] + 1, curr[j] + 1, prev[j] + cost);
    }

    for (let j = 0; j < curr.length; j += 1) {
      prev[j] = curr[j];
    }
  }

  return prev[rightChars.length];
}
