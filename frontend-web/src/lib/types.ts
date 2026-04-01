export interface FollowUpItem {
  id: number;
  question: string;
  answer: string;
  created_at?: string;
}

export interface PhraseHostCandidate {
  headword: string;
  source: string;
  score: number;
}

export type PhraseLookupConfidence = "high" | "medium" | "low";

export interface PhraseLookupInfo {
  phrase: string;
  best_host_headword?: string | null;
  confidence: PhraseLookupConfidence;
  host_candidates: PhraseHostCandidate[];
}

export interface PhraseUsageModule {
  title: string;
  explanation: string;
  example_de: string;
  example_zh: string;
}

export interface PhraseUsagePreview {
  meaning_zh: string;
  meaning_en: string;
  usage_module: PhraseUsageModule;
}

export interface AttachedPhraseModule {
  phrase: string;
  host_headword: string;
  source_phrase_entry_id: number;
  usage_module?: PhraseUsageModule | null;
  analysis_markdown: string;
  confidence: PhraseLookupConfidence;
  attached_at: string;
}

export interface AnalyzeResponse {
  entry_id: number;
  query_text: string;
  analysis_markdown: string;
  phrase_lookup?: PhraseLookupInfo | null;
  phrase_usage_preview?: PhraseUsagePreview | null;
  attached_phrase_modules: AttachedPhraseModule[];
  source: "generated" | "知识库" | string;
  model?: string;
  quality_mode?: QualityMode;
  follow_ups: FollowUpItem[];
}

export interface EntryDetailResponse {
  entry_id: number;
  query_text: string;
  entry_type: string;
  prototype?: string | null;
  analysis_markdown: string;
  phrase_lookup?: PhraseLookupInfo | null;
  phrase_usage_preview?: PhraseUsagePreview | null;
  attached_phrase_modules: AttachedPhraseModule[];
  source: string;
  model?: string | null;
  quality_mode?: QualityMode | null;
  tags: string[];
  aliases: string[];
  follow_ups: FollowUpItem[];
  created_at: string;
  updated_at: string;
}

export interface EntryDeleteResponse {
  message: string;
  deleted_entry_id: number;
}

export interface RecentItem {
  entry_id: number;
  query_text: string;
  preview: string;
}

export type LibraryTab = "all" | "learning" | "review" | "new";

export interface LibraryEntriesPageResponse {
  items: RecentItem[];
  total: number;
  next_cursor?: string | null;
  limit: number;
}

export interface DBSuggestion {
  suggestion_type: string;
  entry_id: number;
  query_text: string;
  preview: string;
  analysis_markdown: string;
  source: string;
  follow_ups: FollowUpItem[];
}

export interface SuggestionResponse {
  suggestions: DBSuggestion[];
}

export interface StatusResponse {
  status: string;
  db_status: string;
}

export interface IntelligentSearchRequest {
  term: string;
  hint: string;
}

export interface AttachPhraseRequest {
  phrase_entry_id?: number | null;
  host_headword: string;
  phrase?: string;
  phrase_lookup?: PhraseLookupInfo | null;
  phrase_usage_preview?: PhraseUsagePreview | null;
  analysis_markdown?: string;
  model?: string;
  quality_mode?: QualityMode;
}

export interface DetachPhraseRequest {
  host_entry_id: number;
  source_phrase_entry_id: number;
}

export interface AnalyzeStreamRequest {
  query_text: string;
  quality_mode?: QualityMode;
  force_refresh?: boolean;
  entry_id?: number;
  generation_hint?: string;
}

export interface DatabaseImportResponse {
  message: string;
}

export interface FollowUpCreateResponse {
  answer: string;
  follow_up: FollowUpItem;
  model?: string;
  quality_mode?: QualityMode;
}

export type QualityMode = "default" | "pro";

export interface LearningProgressView {
  entry_id: number;
  review_count: number;
  next_review_at: string;
  last_reviewed_at?: string;
  scheduled_days: number;
  stability: number;
  difficulty: number;
  state: number;
}

export interface LearningSessionWord {
  entry_id: number;
  query_text: string;
  analysis_markdown: string;
  repetitions_left: number;
  progress: LearningProgressView | null;
}

export interface LearningSessionResponse {
  current_word: LearningSessionWord | null;
  completed_count: number;
  total_count: number;
  is_completed: boolean;
}

export interface LearningProgressMapResponse {
  progress: Record<number, LearningProgressView>;
}

export interface LearningStatsResponse {
  total_words: number;
  due_today: number;
  average_stability: number;
}

export enum ReviewQuality {
  COMPLETELY_FORGOT = 0,
  INCORRECT_BUT_REMEMBERED = 1,
  INCORRECT_WITH_HINT = 2,
  HESITANT = 3,
  CORRECT_WITH_HESITATION = 4,
  PERFECT = 5
}
