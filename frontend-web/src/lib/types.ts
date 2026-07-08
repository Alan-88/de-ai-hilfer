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

export interface StructuredAnalysisMeaning {
  part_of_speech: string;
  chinese: string;
  english: string;
}

export interface StructuredAnalysisUsageModule {
  title: string;
  explanation: string;
  example_de: string;
  example_zh: string;
}

export interface StructuredAnalysisExample {
  de: string;
  zh: string;
}

export interface StructuredAnalysisGrammarRow {
  key: string;
  value: string;
}

export interface StructuredAnalysisWordNetworkItem {
  term: string;
  part_of_speech: string;
  chinese: string;
  english: string;
  note: string;
}

export interface StructuredAnalysisWordNetwork {
  family: StructuredAnalysisWordNetworkItem[];
  synonyms: StructuredAnalysisWordNetworkItem[];
  antonyms: StructuredAnalysisWordNetworkItem[];
}

export interface StructuredAnalysisDeepInsight {
  title: string;
  content_markdown: string;
}

export interface ModelAGrammar {
  genders: string[];
  noun_class: string;
  plural_forms: string[];
  genitive_forms: string[];
  separable: string;
  transitivity: string;
  reflexive: string;
  auxiliaries: string[];
  present_3sg: string;
  preterite_3sg: string;
  partizip_ii: string;
  comparative: string;
  superlative: string;
  governs_cases: string[];
  word_order: string;
}

export interface ModelAMeaning {
  zh: string;
  en: string;
}

export interface GrammarBranch {
  selector: string;
  pos: string;
  meanings: ModelAMeaning[];
  grammar: ModelAGrammar;
}

export interface StructuredAnalysisDocument {
  headword: string;
  phonetic: string;
  meanings: StructuredAnalysisMeaning[];
  usage_modules: StructuredAnalysisUsageModule[];
  collocations: string[];
  examples: StructuredAnalysisExample[];
  grammar_rows: StructuredAnalysisGrammarRow[];
  grammar_branches?: GrammarBranch[];
  word_network?: StructuredAnalysisWordNetwork | null;
  deep_insights: StructuredAnalysisDeepInsight[];
}

export interface AnalyzeResponse {
  entry_id: number;
  query_text: string;
  analysis_markdown: string;
  structured_analysis?: StructuredAnalysisDocument | null;
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
  structured_analysis?: StructuredAnalysisDocument | null;
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
  structured_analysis?: StructuredAnalysisDocument | null;
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

export interface AddPhraseModuleRequest {
  phrase: string;
  instruction?: string | null;
  quality_mode?: QualityMode;
}

export interface DeletePhraseModuleRequest {
  source_phrase_entry_id: number;
  phrase?: string | null;
}

export interface AnalyzeStreamRequest {
  query_text: string;
  quality_mode?: QualityMode;
  force_refresh?: boolean;
  entry_id?: number;
  generation_hint?: string;
  model_override?: AiModelOverride | null;
}

export interface AiModelOverride {
  provider_name: string;
  model_id: string;
}

export interface DatabaseImportResponse {
  message: string;
}

export interface AiProviderProfileView {
  id?: number | null;
  name: string;
  base_url: string;
  model_ids: string[];
  is_default: boolean;
  api_key_set: boolean;
  api_key_preview?: string | null;
}

export interface AiTaskModelSettingView {
  task_key: "analyze" | "phrase" | "structure" | "embedding" | "intelligent_search";
  provider_name?: string | null;
  model_id?: string | null;
  inherit_task_key?: string | null;
}

export interface AiSettingsResponse {
  profiles: AiProviderProfileView[];
  task_settings: AiTaskModelSettingView[];
}

export interface AiProviderProfileInput {
  id?: number | null;
  name: string;
  base_url: string;
  api_key?: string | null;
  model_ids?: string[];
  is_default?: boolean;
}

export interface AiTaskModelSettingInput {
  task_key: AiTaskModelSettingView["task_key"];
  provider_name?: string | null;
  model_id?: string | null;
  inherit_task_key?: string | null;
}

export interface AiSettingsUpdateRequest {
  profiles: AiProviderProfileInput[];
  task_settings: AiTaskModelSettingInput[];
}

export interface AiModelTestRequest {
  profile_id?: number | null;
  profile_name: string;
  base_url: string;
  api_key?: string | null;
  model_id: string;
}

export interface AiModelTestResponse {
  success: boolean;
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
  structured_analysis?: StructuredAnalysisDocument | null;
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
