<script lang="ts">
  import { tick, untrack } from "svelte";
  import { markdownToPlainText, resolveStructuredAnalysis, type StructuredAnalysis } from "$lib/analysis/structuredAnalysis";
  import GrammarBranchPopover from "$lib/components/bento/GrammarBranchPopover.svelte";
  import LearningAnalysisContent from "$lib/components/bento/LearningAnalysisContent.svelte";
  import { addWordToLearning, getLearningProgress } from "$lib/learningApi";
  import { analyzeWord, getEntryDetail, getSuggestions } from "$lib/queryApi";
  import type { AnalyzeResponse, DBSuggestion, EntryDetailResponse, GrammarBranch } from "$lib/types";

  let { selectedKey, selectedEntryId, selectionVersion, open = false, onClose } = $props<{
    selectedKey: string;
    selectedEntryId: number | null;
    selectionVersion: number;
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
  let searchOpen = $state(false);
  let searchInput = $state<HTMLInputElement>();
  let suggestionsElement = $state<HTMLElement>();

  $effect(() => {
    selectionVersion;
    untrack(closeSearch);
    if (selectedEntryId) void showEntry(selectedEntryId, selectedKey);
    else if (selectedKey) showUnavailable(selectedKey);
    else clearEntry();
  });

  $effect(() => {
    if (!open) untrack(closeSearch);
  });

  $effect(() => {
    if (!suggestionsElement) return;
    const panel = suggestionsElement;
    const rail = panel.closest('.lookup-rail');
    const viewport = window.visualViewport;
    const measure = () => {
      const bottom = Math.min((viewport?.height ?? window.innerHeight) + (viewport?.offsetTop ?? 0) - 12, (rail?.getBoundingClientRect().bottom ?? window.innerHeight) - 10);
      panel.style.maxHeight = `${Math.max(0, Math.min(330, bottom - panel.getBoundingClientRect().top))}px`;
    };
    const observer = new ResizeObserver(measure);
    if (rail) observer.observe(rail);
    window.addEventListener('resize', measure);
    viewport?.addEventListener('resize', measure);
    viewport?.addEventListener('scroll', measure);
    measure();
    return () => {
      observer.disconnect();
      window.removeEventListener('resize', measure);
      viewport?.removeEventListener('resize', measure);
      viewport?.removeEventListener('scroll', measure);
    };
  });

  async function openSearch() {
    searchOpen = true;
    await tick();
    if (searchOpen) searchInput?.focus();
  }

  function closeSearch() {
    if (!searchOpen) return;
    searchOpen = false;
    clearSuggestions();
    if (entry) query = entry.query_text;
    if (searchInput === document.activeElement) searchInput?.blur();
  }

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
      if (currentRequest !== suggestionRequestId || query.trim() !== term || !searchOpen) return;
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
      closeSearch();
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
      closeSearch();
    }
  }
</script>

<aside class="lookup-rail" class:mobile-open={open} aria-label="查词栏">
  <header class="lookup-head" class:search-open={searchOpen} onfocusout={handleSearchFocusOut}>
    <button class="rail-back" class:search-expanded={searchOpen} onclick={() => searchOpen ? closeSearch() : onClose()} aria-label={searchOpen ? "收起搜索" : "关闭查词栏"}><i class="ph ph-arrow-left"></i></button>
    <section class="word-head" aria-label="当前单词" aria-hidden={searchOpen}>
      {#if !loading && !error && entry && analysis}
        <h2>{analysis.headword || entry.query_text}</h2>
        {#if analysis.phonetic}<span class="word-phonetic">{analysis.phonetic}</span>{/if}
      {:else}
        <span class="lookup-title">{loading ? "查询中" : "快捷查词"}</span>
      {/if}
    </section>
    <button class="search-open-btn" onclick={() => void openSearch()} aria-label="展开搜索" aria-expanded={searchOpen}><i class="ph ph-magnifying-glass"></i></button>
    <div class="lookup-search" inert={!searchOpen} aria-hidden={!searchOpen}>
      <form onsubmit={(event) => { event.preventDefault(); void submitSearch(); }}>
        <input
          bind:this={searchInput}
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
          onkeydown={(event) => { if (event.key === "Escape") closeSearch(); }}
        />
        <button type="submit" aria-label="查询" disabled={loading}><i class={loading ? "ph ph-spinner-gap spin" : "ph ph-magnifying-glass"}></i></button>
      </form>

      {#if showSuggestions && suggestions.length > 0}
        <div id="reading-lookup-suggestions" class="lookup-suggestions" bind:this={suggestionsElement}>
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
  </header>

  <div class="lookup-scroll">
    {#if loading}
      <div class="loading-result"><span></span><span></span><span></span></div>
    {:else if error}
      <div class="empty-result"><i class="ph ph-warning-circle"></i><span>{error}</span></div>
    {:else if entry && analysis}
      <div class="word-actions">
        <button class:added={learningAdded} class="learning-action" disabled={learningAdded} onclick={() => void addLearning()}><i class={learningAdded ? "ph ph-check" : "ph ph-target"}></i> {learningAdded ? "已加入" : "加入复习"}</button>
      </div>

      <LearningAnalysisContent
        interleavedMeanings
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
  .lookup-rail { container: word-detail / inline-size; display: flex; min-width: 0; min-height: 0; flex-direction: column; overflow: hidden; border-left: 1px solid var(--border-color); background: color-mix(in srgb, var(--card-bg) 78%, var(--bg-color)); }
  .lookup-head { position: relative; z-index: 5; display: grid; flex: 0 0 auto; grid-template-columns: 44px minmax(0, 1fr) 44px; align-items: start; column-gap: 6px; min-height: 76px; padding: 12px 10px; border-bottom: 1px solid var(--border-color); }
  .lookup-search { position: absolute; top: 16px; left: 60px; right: 10px; min-width: 0; visibility: hidden; opacity: 0; pointer-events: none; transition: opacity 180ms ease, visibility 0s linear 260ms; }
  .lookup-search form { clip-path: inset(0 0 0 100%); transition: clip-path 260ms ease; }
  .search-open .lookup-search { visibility: visible; opacity: 1; pointer-events: auto; transition-delay: 0s; }
  .search-open .lookup-search form { clip-path: inset(0); }
  .search-open .word-head, .search-open .search-open-btn { visibility: hidden; opacity: 0; pointer-events: none; }
  form { display: flex; width: 100%; align-items: center; gap: 0.5rem; min-width: 0; height: 44px; padding: 0 5px 0 12px; border: 1px solid var(--border-color); border-radius: var(--radius-full); background: var(--card-bg); box-shadow: var(--shadow-sm); }
  input { min-width: 0; flex: 1; border: 0; outline: 0; background: transparent; color: var(--text-main); font: inherit; font-weight: 700; }
  form button { display: grid; width: 32px; height: 32px; flex: 0 0 auto; place-items: center; border-radius: 50%; background: var(--accent-main); color: white; }
  .rail-back, .search-open-btn { display: grid; width: 44px; height: 44px; margin-top: 4px; place-items: center; border-radius: 50%; background: transparent; color: var(--text-main); font-size: 22px; }
  .rail-back:hover, .rail-back:active, .search-open-btn:hover, .search-open-btn:active { background: var(--btn-secondary); }
  .rail-back:focus-visible, .search-open-btn:focus-visible, form button:focus-visible { outline: 2px solid var(--accent-main); outline-offset: 2px; }
  .lookup-suggestions { position: absolute; top: calc(100% + 0.45rem); right: 0; left: 0; display: grid; max-height: min(330px, 48vh); overflow-y: auto; padding: 0.38rem; border: 1px solid var(--border-color); border-radius: var(--radius-md); background: var(--card-bg); box-shadow: var(--shadow-hover); }
  .lookup-suggestions > button { display: grid; min-height: 54px; grid-template-columns: 22px minmax(0, 1fr) auto; align-items: center; gap: 0.55rem; padding: 0.55rem 0.6rem; border-radius: 9px; background: transparent; color: var(--text-main); text-align: left; }
  .lookup-suggestions > button:hover, .lookup-suggestions > button:focus-visible { background: var(--btn-secondary); outline: none; }
  .lookup-suggestions > button > i { color: var(--text-muted); }
  .lookup-suggestions span { display: grid; min-width: 0; gap: 0.12rem; }
  .lookup-suggestions strong, .lookup-suggestions small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .lookup-suggestions strong { font-size: 0.84rem; }
  .lookup-suggestions small { color: var(--text-muted); font-size: 0.7rem; }
  .lookup-suggestions em { padding: 0.22rem 0.4rem; border-radius: var(--radius-full); background: color-mix(in srgb, var(--accent-main) 10%, transparent); color: var(--accent-main); font-size: 0.6rem; font-style: normal; font-weight: 850; }
  .lookup-scroll { min-height: 0; flex: 1; overflow-y: auto; padding: 10px; overscroll-behavior: contain; }
  .word-head { display: grid; min-width: 0; gap: 4px; text-align: center; transition: opacity 180ms ease; }
  .lookup-title { align-self: center; color: var(--text-muted); font-size: 14px; line-height: 44px; }
  .word-head h2 { margin: 0; min-width: 0; color: var(--text-main); font-size: 26px; line-height: 1.15; letter-spacing: -0.025em; overflow-wrap: anywhere; }
  .word-phonetic { min-width: 0; color: var(--text-muted); font-family: "Arial Unicode MS", "Lucida Sans Unicode", sans-serif; font-size: 13px; line-height: 1.4; overflow-wrap: anywhere; }
  .word-actions { display: flex; justify-content: flex-end; margin-bottom: 8px; }
  .learning-action { display: flex; min-height: 32px; flex: 0 0 auto; align-items: center; justify-content: center; gap: 4px; padding: 0 12px; border-radius: var(--radius-full); background: var(--accent-main); color: white; font-size: 11px; font-weight: 800; white-space: nowrap; }
  .learning-action.added { background: color-mix(in srgb, var(--accent-main) 72%, #4f8665); }
  .learning-action:disabled { cursor: default; }
  .empty-result { display: flex; align-items: center; gap: 0.65rem; padding: 1rem; border: 1px dashed var(--border-color); border-radius: var(--radius-md); color: var(--text-muted); }
  .loading-result { display: flex; justify-content: center; gap: 0.4rem; padding: 3rem 0; }
  .loading-result span { width: 7px; height: 7px; border-radius: 50%; background: var(--text-muted); animation: pulse 900ms ease-in-out infinite alternate; }
  .loading-result span:nth-child(2) { animation-delay: 150ms; }
  .loading-result span:nth-child(3) { animation-delay: 300ms; }
  @keyframes pulse { to { opacity: 0.25; transform: translateY(-3px); } }

  @media (max-width: 900px) {
    .lookup-rail { position: absolute; inset: 0 0 0 auto; z-index: 30; width: min(390px, 88%); border: 1px solid var(--border-color); border-right: 0; border-radius: var(--radius-md) 0 0 var(--radius-md); transform: translateX(102%); pointer-events: none; transition: transform 180ms ease; }
    .lookup-rail.mobile-open { transform: translateX(0); box-shadow: -18px 0 50px rgba(20, 18, 31, 0.2); pointer-events: auto; }
  }

  @media (min-width: 901px) {
    .rail-back:not(.search-expanded) { visibility: hidden; pointer-events: none; }
  }

  @media (prefers-reduced-motion: reduce) {
    .lookup-search, .lookup-search form, .word-head { transition: none; }
  }
</style>
