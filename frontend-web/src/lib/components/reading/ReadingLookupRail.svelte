<script lang="ts">
  import { markdownToPlainText, resolveStructuredAnalysis, type StructuredAnalysis } from "$lib/analysis/structuredAnalysis";
  import GrammarBranchPopover from "$lib/components/bento/GrammarBranchPopover.svelte";
  import LearningAnalysisContent from "$lib/components/bento/LearningAnalysisContent.svelte";
  import { addWordToLearning, getLearningProgress } from "$lib/learningApi";
  import { analyzeWord, getEntryDetail, getSuggestions } from "$lib/queryApi";
  import type { AnalyzeResponse, DBSuggestion, EntryDetailResponse, GrammarBranch } from "$lib/types";

  let { selectedKey, selectedEntryId, open = false, onClose } = $props<{
    selectedKey: string;
    selectedEntryId: number | null;
    open?: boolean;
    onClose: () => void;
  }>();

  const cache = new Map<number, EntryDetailResponse>();
  let query = $state("");
  let entry = $state<EntryDetailResponse | AnalyzeResponse | null>(null);
  let analysis = $state<StructuredAnalysis | null>(null);
  let error = $state("");
  let loading = $state(false);
  let learningAdded = $state(false);
  let requestId = 0;
  let learningEntryIds: Set<number> | null = null;
  let learningIdsRequest: Promise<Set<number>> | null = null;
  let activeGrammarBranch = $state<GrammarBranch | null>(null);
  let popoverTriggerRect = $state<DOMRect | null>(null);
  let suggestions = $state<DBSuggestion[]>([]);
  let suggestionsQuery = $state("");
  let showSuggestions = $state(false);
  let suggestionRequestId = 0;

  $effect(() => {
    if (selectedEntryId) void showEntry(selectedEntryId, selectedKey);
    else if (selectedKey) showUnavailable(selectedKey);
    else clearEntry();
  });

  function clearEntry() {
    requestId += 1;
    clearSuggestions();
    query = "";
    entry = null;
    analysis = null;
    error = "";
    loading = false;
    learningAdded = false;
  }

  function getLearningEntryIds() {
    if (learningEntryIds) return Promise.resolve(learningEntryIds);
    if (!learningIdsRequest) {
      learningIdsRequest = getLearningProgress()
        .then(({ progress }) => {
          learningEntryIds = new Set(Object.keys(progress).map(Number));
          return learningEntryIds;
        })
        .catch(() => new Set<number>())
        .finally(() => learningIdsRequest = null);
    }
    return learningIdsRequest;
  }

  async function showEntry(entryId: number, key: string) {
    const currentRequest = ++requestId;
    clearSuggestions();
    query = key;
    activeGrammarBranch = null;
    learningAdded = false;
    loading = true;
    error = "";
    try {
      const [next, learningIds] = await Promise.all([
        cache.get(entryId) ?? getEntryDetail(entryId),
        getLearningEntryIds(),
      ]);
      if (currentRequest !== requestId) return;
      cache.set(entryId, next);
      entry = next;
      learningAdded = learningIds.has(entryId);
      analysis = resolveStructuredAnalysis(next.analysis_markdown, next.structured_analysis, next.query_text);
    } catch (cause) {
      if (currentRequest !== requestId) return;
      entry = null;
      analysis = null;
      error = cause instanceof Error ? cause.message : "词条加载失败";
    } finally {
      if (currentRequest === requestId) loading = false;
    }
  }

  function showUnavailable(key: string) {
    requestId += 1;
    clearSuggestions();
    query = key;
    entry = null;
    analysis = null;
    loading = false;
    error = `${key} 对应的完整词条已不可用`;
  }

  async function fetchLookupSuggestions() {
    const term = query.trim();
    const currentRequest = ++suggestionRequestId;
    if (term.length < 2) {
      suggestions = [];
      suggestionsQuery = "";
      showSuggestions = false;
      return;
    }

    try {
      const response = await getSuggestions(term);
      if (currentRequest !== suggestionRequestId || query.trim() !== term) return;
      suggestions = response.suggestions;
      suggestionsQuery = term;
      showSuggestions = suggestions.length > 0;
    } catch {
      if (currentRequest !== suggestionRequestId) return;
      suggestions = [];
      suggestionsQuery = term;
      showSuggestions = false;
    }
  }

  function clearSuggestions() {
    suggestionRequestId += 1;
    suggestions = [];
    suggestionsQuery = "";
    showSuggestions = false;
  }

  async function submitSearch(selectedCandidate?: DBSuggestion) {
    const term = (selectedCandidate?.query_text ?? query).trim();
    if (!term) return;
    const currentRequest = ++requestId;
    suggestionRequestId += 1;
    showSuggestions = false;
    loading = true;
    error = "";
    learningAdded = false;
    try {
      const candidate = selectedCandidate ?? (
        suggestionsQuery === term && suggestions.length > 0
          ? suggestions[0]
          : (await getSuggestions(term)).suggestions[0]
      );
      if (!candidate) throw new Error("未找到可靠候选");
      if (candidate.entry_id > 0) {
        const next = cache.get(candidate.entry_id) ?? await getEntryDetail(candidate.entry_id);
        cache.set(candidate.entry_id, next);
        if (currentRequest !== requestId) return;
        entry = next;
      } else {
        const next = await analyzeWord(candidate.query_text);
        if (currentRequest !== requestId) return;
        entry = next;
      }
      query = entry.query_text;
      suggestions = [];
      suggestionsQuery = "";
      analysis = resolveStructuredAnalysis(entry.analysis_markdown, entry.structured_analysis, entry.query_text);
      if (!analysis) throw new Error("词条缺少结构化释义");
      const resolvedEntry = entry;
      const learningIds = await getLearningEntryIds();
      if (currentRequest !== requestId) return;
      learningAdded = learningIds.has(resolvedEntry.entry_id);
    } catch (cause) {
      if (currentRequest !== requestId) return;
      entry = null;
      analysis = null;
      error = cause instanceof Error ? cause.message : "词条加载失败";
    } finally {
      if (currentRequest === requestId) loading = false;
    }
  }

  async function addLearning() {
    if (!entry) return;
    try {
      await addWordToLearning(entry.entry_id);
      (learningEntryIds ??= new Set()).add(entry.entry_id);
      learningAdded = true;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "加入复习失败";
    }
  }

  function openGrammar(branch: GrammarBranch, triggerRect: DOMRect) {
    activeGrammarBranch = branch;
    popoverTriggerRect = triggerRect;
  }

  function handleSearchFocusOut(event: FocusEvent) {
    const next = event.relatedTarget;
    if (!(next instanceof Node) || !(event.currentTarget instanceof HTMLElement) || !event.currentTarget.contains(next)) {
      showSuggestions = false;
    }
  }
</script>

<aside class="lookup-rail" class:mobile-open={open} aria-label="查词栏">
  <header class="lookup-head">
    <div class="lookup-search" onfocusout={handleSearchFocusOut}>
      <form onsubmit={(event) => { event.preventDefault(); void submitSearch(); }}>
        <i class="ph ph-magnifying-glass"></i>
        <input
          bind:value={query}
          aria-label="查询单词"
          aria-expanded={showSuggestions}
          aria-controls="reading-lookup-suggestions"
          autocomplete="off"
          oninput={() => void fetchLookupSuggestions()}
          onfocus={() => {
            if (suggestionsQuery === query.trim() && suggestions.length > 0) showSuggestions = true;
            else void fetchLookupSuggestions();
          }}
          onkeydown={(event) => { if (event.key === "Escape") showSuggestions = false; }}
        />
        <button type="submit" aria-label="查询"><i class="ph ph-arrow-right"></i></button>
      </form>

      {#if showSuggestions && suggestions.length > 0}
        <div id="reading-lookup-suggestions" class="lookup-suggestions">
          {#each suggestions as suggestion}
            <button type="button" onclick={() => void submitSearch(suggestion)}>
              <i class="ph ph-magnifying-glass"></i>
              <span>
                <strong>{suggestion.query_text}</strong>
                <small>{markdownToPlainText(suggestion.preview)}</small>
              </span>
              {#if suggestion.entry_id <= 0}<em>生成</em>{/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>
    <button class="mobile-close" onclick={onClose} aria-label="关闭查词栏"><i class="ph ph-x"></i></button>
  </header>

  <div class="lookup-scroll">
    {#if loading}
      <div class="loading-result"><span></span><span></span><span></span></div>
    {:else if error}
      <div class="empty-result"><i class="ph ph-warning-circle"></i><span>{error}</span></div>
    {:else if entry && analysis}
      <section class="word-head">
        <div class="word-title-row">
          <h2>{analysis.headword || entry.query_text}</h2>
          <span class="entry-spacer"></span>
        </div>
        <div class="word-meta-row">
          {#if analysis.phonetic}<span class="word-phonetic">{analysis.phonetic}</span>{/if}
          <button class:added={learningAdded} class="learning-action" disabled={learningAdded} onclick={() => void addLearning()}><i class={learningAdded ? "ph ph-check" : "ph ph-target"}></i> {learningAdded ? "已加入" : "加入复习"}</button>
        </div>
      </section>

      <LearningAnalysisContent
        {analysis}
        attachedPhraseModules={entry.attached_phrase_modules}
        onGrammarOpen={openGrammar}
      />

    {/if}
  </div>
</aside>

{#if activeGrammarBranch}
  <GrammarBranchPopover
    branch={activeGrammarBranch}
    triggerRect={popoverTriggerRect}
    onClose={() => {
      activeGrammarBranch = null;
      popoverTriggerRect = null;
    }}
  />
{/if}

<style>
  .lookup-rail { display: flex; min-width: 0; min-height: 0; flex-direction: column; overflow: hidden; border-left: 1px solid var(--border-color); background: color-mix(in srgb, var(--card-bg) 78%, var(--bg-color)); }
  .lookup-head { position: relative; z-index: 5; display: flex; align-items: center; gap: 0.55rem; padding: 1rem; border-bottom: 1px solid var(--border-color); }
  .lookup-search { position: relative; min-width: 0; flex: 1; }
  form { display: flex; width: 100%; align-items: center; gap: 0.5rem; min-width: 0; height: 42px; padding: 0 0.45rem 0 0.8rem; border: 1px solid var(--border-color); border-radius: var(--radius-full); background: var(--card-bg); box-shadow: var(--shadow-sm); }
  form > i { color: var(--text-muted); }
  input { min-width: 0; flex: 1; border: 0; outline: 0; background: transparent; color: var(--text-main); font: inherit; font-weight: 700; }
  form button, .mobile-close { display: grid; width: 32px; height: 32px; place-items: center; border-radius: 50%; background: var(--accent-main); color: white; }
  .mobile-close { display: none; flex: 0 0 auto; background: var(--btn-secondary); color: var(--text-main); }
  .lookup-suggestions { position: absolute; top: calc(100% + 0.45rem); right: 0; left: 0; display: grid; max-height: min(330px, 48vh); overflow-y: auto; padding: 0.38rem; border: 1px solid var(--border-color); border-radius: var(--radius-md); background: var(--card-bg); box-shadow: var(--shadow-hover); }
  .lookup-suggestions > button { display: grid; min-height: 54px; grid-template-columns: 22px minmax(0, 1fr) auto; align-items: center; gap: 0.55rem; padding: 0.55rem 0.6rem; border-radius: 9px; background: transparent; color: var(--text-main); text-align: left; }
  .lookup-suggestions > button:hover, .lookup-suggestions > button:focus-visible { background: var(--btn-secondary); outline: none; }
  .lookup-suggestions > button > i { color: var(--text-muted); }
  .lookup-suggestions span { display: grid; min-width: 0; gap: 0.12rem; }
  .lookup-suggestions strong, .lookup-suggestions small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .lookup-suggestions strong { font-size: 0.84rem; }
  .lookup-suggestions small { color: var(--text-muted); font-size: 0.7rem; }
  .lookup-suggestions em { padding: 0.22rem 0.4rem; border-radius: var(--radius-full); background: color-mix(in srgb, var(--accent-main) 10%, transparent); color: var(--accent-main); font-size: 0.6rem; font-style: normal; font-weight: 850; }
  .lookup-scroll { min-height: 0; flex: 1; overflow-y: auto; padding: 1.35rem; overscroll-behavior: contain; }
  .word-head { display: grid; gap: 0.65rem; padding: 0.35rem 0 1.2rem; }
  .word-title-row, .word-meta-row { display: flex; min-width: 0; align-items: center; justify-content: space-between; gap: 0.8rem; }
  .word-head h2 { margin: 0; color: var(--text-main); font-size: 2.15rem; line-height: 1.12; letter-spacing: -0.035em; }
  .word-phonetic { min-width: 0; color: var(--text-muted); font-family: "Arial Unicode MS", "Lucida Sans Unicode", sans-serif; font-size: 0.86rem; line-height: 1.6; }
  .learning-action { display: flex; min-width: 112px; min-height: 36px; flex: 0 0 auto; align-items: center; justify-content: center; gap: 0.4rem; padding: 0 0.9rem; border-radius: var(--radius-full); background: var(--accent-main); color: white; font-weight: 800; }
  .learning-action.added { background: color-mix(in srgb, var(--accent-main) 72%, #4f8665); }
  .learning-action:disabled { cursor: default; }
  .empty-result { display: flex; align-items: center; gap: 0.65rem; padding: 1rem; border: 1px dashed var(--border-color); border-radius: var(--radius-md); color: var(--text-muted); }
  .loading-result { display: flex; justify-content: center; gap: 0.4rem; padding: 3rem 0; }
  .loading-result span { width: 7px; height: 7px; border-radius: 50%; background: var(--text-muted); animation: pulse 900ms ease-in-out infinite alternate; }
  .loading-result span:nth-child(2) { animation-delay: 150ms; }
  .loading-result span:nth-child(3) { animation-delay: 300ms; }
  @keyframes pulse { to { opacity: 0.25; transform: translateY(-3px); } }

  :global(.lookup-rail .learning-content) { gap: 0.8rem; padding-bottom: 1.25rem; }
  :global(.lookup-rail .learning-content .section-card) { padding: 0.9rem; border-radius: var(--radius-md); }
  :global(.lookup-rail .learning-content .meaning-summary) { gap: 0.6rem; padding: 0.8rem; }
  :global(.lookup-rail .learning-content .meaning-summary strong) { font-size: 0.94rem; }
  :global(.lookup-rail .learning-content .usage-item) { padding: 0.1rem 0 0.8rem; border-radius: 0; background: transparent; }
  :global(.lookup-rail .learning-content .usage-item + .usage-item) { padding-top: 0.85rem; border-top: 1px solid var(--border-color); }
  :global(.lookup-rail .learning-content .usage-item > strong) { font-size: 0.94rem; line-height: 1.45; }
  :global(.lookup-rail .learning-content .usage-copy) { margin-top: 0.35rem; font-size: 0.86rem; line-height: 1.6; }
  :global(.lookup-rail .learning-content .example-box) { margin-top: 0.7rem; padding: 0.8rem 0.9rem; border-left-width: 3px; border-left-color: var(--accent-main); background: var(--card-bg); }
  :global(.lookup-rail .learning-content .de-line) { font-size: 0.94rem; line-height: 1.45; }
  :global(.lookup-rail .learning-content .zh-line) { font-size: 0.82rem; line-height: 1.5; }
  :global(.lookup-rail .learning-content .insight-block) { padding: 0.1rem 0 0.8rem; border-radius: 0; background: transparent; }
  :global(.lookup-rail .learning-content .insight-block + .insight-block) { padding-top: 0.85rem; border-top: 1px solid var(--border-color); }
  :global(.lookup-rail .learning-content .insight-copy) { font-size: 0.86rem; line-height: 1.65; }
  :global(.lookup-rail .learning-content .network-grid article) { max-width: 100%; }

  @media (max-width: 900px) {
    .lookup-rail { position: absolute; inset: 0 0 0 auto; z-index: 30; width: min(390px, 88%); border: 1px solid var(--border-color); border-right: 0; border-radius: var(--radius-md) 0 0 var(--radius-md); transform: translateX(102%); box-shadow: -18px 0 50px rgba(20, 18, 31, 0.2); transition: transform 180ms ease; }
    .lookup-rail.mobile-open { transform: translateX(0); }
    .mobile-close { display: grid; }
  }
</style>
