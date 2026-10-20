<script lang="ts">
  import { onMount, tick } from "svelte";
  import { browser } from "$app/environment";
  import { edgeSwipe } from "$lib/actions/edgeSwipe";
  import { fade } from "svelte/transition";
  import { addWordToLearning, getLearningProgress } from "$lib/learningApi";
  import {
    addPhraseModuleToEntry,
    analyzeWord,
    deletePhraseModuleFromEntry,
    getAiSettings,
    getRecentEntries,
    getSuggestions,
    intelligentSearch
  } from "$lib/queryApi";
  import AnalysisRetryCard from "$lib/components/bento/AnalysisRetryCard.svelte";
  import SearchResultCards from "$lib/components/bento/SearchResultCards.svelte";
  import {
    actionModelOverrideForKey,
    buildActionModelOptions,
    defaultActionModelKey,
  } from "$lib/ai/actionModels";
  import { markdownToPlainText, renderMarkdownHtml, resolveStructuredAnalysis } from "$lib/analysis/structuredAnalysis";
  import { shouldPreferDirectAnalyze } from "$lib/search/searchExecution";
  import { streamAnalyze } from "$lib/streamApi";
  import { searchStore } from "$lib/stores/search";
  import type {
    AnalyzeResponse,
    AiModelOverride,
    AiSettingsResponse,
    AttachedPhraseModule,
    DBSuggestion,
    FollowUpItem,
    QualityMode,
    RecentItem,
  } from "$lib/types";

  let { active = false, showAdvancedOnMount = false } = $props();

  // 本地 UI 状态
  let recentItems = $state<RecentItem[]>([]);
  let learningEntryIds = $state<Set<number>>(new Set());
  let suggestions = $state<DBSuggestion[]>([]);
  let showSuggestions = $state(false);
  let searchFocused = $state(false);
  let viewportHeight = $state<number | null>(null);
  let viewportTop = $state(0);
  let isAddingToLearning = $state(false);
  let showAdvanced = $state(showAdvancedOnMount);
  let advancedHint = $state("");
  let advancedPending = $state<AnalyzeResponse | null>(null);
  let isAdvancedLoading = $state(false);
  let isUpdatingPhraseAttachment = $state(false);
  let isAddingPhraseModule = $state(false);
  let suggestionsQuery = $state("");
  let suggestionsLoading = $state(false);
  let suggestionsError = $state("");
  let visibleSuggestionCount = $state(20);
  let suggestionList = $state<HTMLDivElement | null>(null);
  let suggestionPanel = $state<HTMLDivElement | null>(null);
  let suggestionPanelHeight = $state(460);
  let composing = false;
  let compositionEndedAt = 0;
  let suggestionVersion = 0;
  let suggestionTimer: ReturnType<typeof setTimeout> | undefined;
  let suggestionController: AbortController | null = null;
  let suggestionRequest: { query: string; promise: Promise<DBSuggestion[]> } | null = null;
  let currentSearchController: AbortController | null = null;
  let hasMounted = $state(false);
  let mainInputRef = $state<HTMLInputElement | null>(null);
  let advancedInputRef = $state<HTMLInputElement | null>(null);
  let aiSettings = $state<AiSettingsResponse | null>(null);
  let selectedActionModelKey = $state("");
  let failedAnalysis = $state<FailedAnalysisRequest | null>(null);
  let switchingInput = false;
  let searchMoveAnimation: Animation | null = null;

  const s = searchStore;

  type FailedAnalysisRequest = {
    query: string;
    qualityMode: QualityMode;
    forceRefresh: boolean;
    generationHint: string;
    modelOverride: AiModelOverride | null;
  };

  function focusSoon(target: () => HTMLInputElement | null) {
    requestAnimationFrame(() => target()?.focus());
  }

  async function setSearchFocus(next: boolean) {
    if (searchFocused === next) return;
    const row = mainInputRef?.closest<HTMLElement>(".main-row");
    const panel = row?.closest<HTMLElement>(".search-panel-group");
    const previousTop = row?.getBoundingClientRect().top;
    const previousReveal = panel && searchMoveAnimation?.playState === "running"
      ? { clipPath: getComputedStyle(panel).clipPath, opacity: getComputedStyle(panel).opacity, visibility: "visible" }
      : null;
    searchMoveAnimation?.cancel();
    searchMoveAnimation = null;
    searchFocused = next;
    if (!row || !panel || previousTop === undefined || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    await tick();
    if (searchFocused !== next || !panel.isConnected) return;
    if (isSearching) {
      if (getComputedStyle(panel).getPropertyValue("--result-search-wide").trim() === "1") return;
      const collapsed = { clipPath: "inset(0 0 0 calc(100% - 44px) round 9999px)", opacity: 0, visibility: "visible" };
      const expanded = { clipPath: "inset(0 round 9999px)", opacity: 1, visibility: "visible" };
      searchMoveAnimation = panel.animate([previousReveal ?? (next ? collapsed : expanded), next ? expanded : collapsed],
        { duration: 260, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" });
      return;
    }
    const offset = previousTop - row.getBoundingClientRect().top;
    searchMoveAnimation = panel.animate(
      [{ transform: `translateY(${offset}px)` }, { transform: "translateY(0)" }],
      { duration: 260, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" },
    );
  }

  async function openResultSearch() {
    switchingInput = true;
    void setSearchFocus(true);
    await tick();
    mainInputRef?.focus();
    mainInputRef?.select();
    switchingInput = false;
  }

  function handleResultBack() {
    if (searchFocused) dismissSearchFocus();
    else clearSearchResult();
  }

  async function toggleAdvanced() {
    switchingInput = true;
    void setSearchFocus(true);
    showSuggestions = false;
    showAdvanced = !showAdvanced;
    await tick();
    (showAdvanced ? advancedInputRef : mainInputRef)?.focus();
    switchingInput = false;
  }

  function dismissSearchFocus() {
    cancelSuggestions();
    void setSearchFocus(false);
    showSuggestions = false;
    mainInputRef?.blur();
    advancedInputRef?.blur();
  }

  async function handleSearch(
    term = $s.query,
    qualityMode: QualityMode = "default",
    forceRefresh = false,
    generationHint = "",
    useSuggestionDefault = true,
    modelOverride: AiModelOverride | null = null
  ) {
    const rawQuery = term.trim();
    if (!rawQuery) return;
    const query = await resolveSearchTarget(rawQuery, {
      enabled: useSuggestionDefault && !forceRefresh && !generationHint.trim(),
    });
    if (!query) return;
    dismissSearchFocus();

    const failedRequest: FailedAnalysisRequest = {
      query,
      qualityMode,
      forceRefresh,
      generationHint,
      modelOverride,
    };
    const resultBeforeRequest = $s.result;
    failedAnalysis = null;

    currentSearchController?.abort();
    currentSearchController = new AbortController();
    const preferDirectAnalyze = shouldPreferDirectAnalyze({
      query,
      qualityMode,
      forceRefresh,
      recentItems,
      suggestions,
    });

    s.update(state => ({
      ...state,
      query,
      isLoading: true,
      isStreaming: !preferDirectAnalyze,
      error: "",
      activeQualityMode: qualityMode,
      result: forceRefresh
        ? state.result
        : preferDirectAnalyze
          ? null
          : {
              entry_id: 0,
              query_text: query,
              analysis_markdown: "",
              structured_analysis: null,
              attached_phrase_modules: [],
              source: qualityMode === "pro" ? "Pro" : "Flash",
              quality_mode: qualityMode,
              follow_ups: [],
            }
    }));

    showSuggestions = false;
    advancedPending = null;

    try {
      if (preferDirectAnalyze) {
        const response = await analyzeWord(query);
        s.update(state => ({
          ...state,
          query: response.query_text,
          result: response,
          activeModel: response.model ?? "",
        }));
        void fetchRecentItems();
        return;
      }

      const existingEntryId = forceRefresh ? $s.result?.entry_id : undefined;
      await streamAnalyze(
        {
          query_text: query,
          quality_mode: qualityMode,
          force_refresh: forceRefresh,
          entry_id: existingEntryId,
          generation_hint: generationHint.trim() || undefined,
          model_override: modelOverride,
        },
        {
          signal: currentSearchController.signal,
          onMeta: (payload) => {
            s.update(state => ({
              ...state,
              activeModel: payload.model,
              result: state.result ? { ...state.result, source: payload.source, model: payload.model } : null
            }));
          },
          onDelta: (payload) => {
            s.update(state => {
              if (!state.result) return state;
              return {
                ...state,
                result: {
                  ...state.result,
                  analysis_markdown: state.result.analysis_markdown + payload.delta
                }
              };
            });
          },
          onComplete: (payload) => {
            failedAnalysis = null;
            s.update(state => ({ ...state, result: payload, activeModel: payload.model ?? state.activeModel }));
            void fetchRecentItems();
          },
          onError: (payload) => {
            setAnalysisFailure(payload.message, failedRequest, resultBeforeRequest);
          },
        }
      );
    } catch (searchError) {
      if (!(searchError instanceof DOMException && searchError.name === "AbortError")) {
        const message = searchError instanceof Error ? searchError.message : "查询失败";
        if (preferDirectAnalyze) {
          s.setError(message);
        } else {
          setAnalysisFailure(message, failedRequest, resultBeforeRequest);
        }
      }
    } finally {
      s.update(state => ({ ...state, isLoading: false, isStreaming: false }));
    }
  }

  async function handleAdvancedSearch() {
    if (!$s.query.trim()) {
      s.setError("请先在主搜索栏输入一个德语词或近似拼写。");
      return;
    }
    dismissSearchFocus();
    isAdvancedLoading = true;
    failedAnalysis = null;
    s.setError("");
    advancedPending = null;
    try {
      const result = await intelligentSearch({
        term: $s.query.trim(),
        hint: advancedHint.trim(),
      });
      if (result.source === "需要AI推断") {
        advancedPending = result;
        s.setResult(null);
        return;
      }
      s.setQuery(result.query_text);
      s.setResult(result);
      void fetchRecentItems();
      showAdvanced = false;
    } catch (e) {
      s.setError("高级查询失败");
    } finally {
      isAdvancedLoading = false;
    }
  }

  async function fetchRecentItems() {
    recentItems = await getRecentEntries();
  }

  async function fetchLearningProgress() {
    try {
      const { progress } = await getLearningProgress();
      learningEntryIds = new Set(Object.keys(progress).map(Number));
    } catch {
      learningEntryIds = new Set();
    }
  }

  async function fetchAiSettings() {
    try {
      aiSettings = await getAiSettings();
      selectedActionModelKey = defaultActionModelKey(aiSettings);
    } catch {
      aiSettings = null;
      selectedActionModelKey = "";
    }
  }

  function isSemanticQuery(query: string) {
    return /\p{Script=Han}/u.test(query);
  }

  function cancelSuggestions() {
    clearTimeout(suggestionTimer);
    suggestionController?.abort();
    suggestionController = null;
    suggestionRequest = null;
    suggestionVersion++;
    suggestionsLoading = false;
  }

  function fetchSuggestions() {
    const query = $s.query.trim();
    if (composing) return;
    if (suggestionRequest?.query === query) return;
    cancelSuggestions();
    if (suggestionsQuery === query && !suggestionsError) return;
    suggestionsError = "";
    if (!query || (!isSemanticQuery(query) && query.length < 2)) {
      suggestions = [];
      suggestionsQuery = "";
      return;
    }
    suggestionsLoading = true;
    showSuggestions = searchFocused;
    suggestionTimer = setTimeout(() => { void requestSuggestions(query).catch(() => {}); }, isSemanticQuery(query) ? 200 : 0);
  }

  function requestSuggestions(query: string): Promise<DBSuggestion[]> {
    clearTimeout(suggestionTimer);
    if (suggestionRequest?.query === query) return suggestionRequest.promise;
    cancelSuggestions();
    const version = suggestionVersion;
    const controller = new AbortController();
    suggestionController = controller;
    suggestionsLoading = true;
    suggestionsError = "";
    const promise = (async () => {
      try {
        const resp = await getSuggestions(query, { mode: isSemanticQuery(query) ? "semantic" : "lexical", limit: 100, signal: controller.signal });
        if (version !== suggestionVersion || $s.query.trim() !== query) throw new DOMException("Stale suggestions", "AbortError");
        suggestions = resp.suggestions;
        suggestionsQuery = query;
        visibleSuggestionCount = 20;
        if (suggestionList) suggestionList.scrollTop = 0;
        return resp.suggestions;
      } catch (error) {
        if (version === suggestionVersion && $s.query.trim() === query && !controller.signal.aborted) {
          suggestionsError = "候选查询失败，请重试";
          suggestions = [];
          suggestionsQuery = "";
        }
        throw error;
      } finally {
        if (version === suggestionVersion) {
          suggestionsLoading = false;
          suggestionRequest = null;
        }
      }
    })();
    suggestionRequest = { query, promise };
    return promise;
  }

  async function resolveSearchTarget(query: string, options: { enabled: boolean }) {
    if (!query || !options.enabled || showAdvanced) return query;
    if (!isSemanticQuery(query) && query.length < 2) return query;
    try {
      const items = suggestionsQuery === query && !suggestionsError ? suggestions : await requestSuggestions(query);
      if ($s.query.trim() !== query) return "";
      if (isSemanticQuery(query) && !items.length) return "";
      return items[0]?.query_text?.trim() || query;
    } catch (error) {
      if (isSemanticQuery(query) || $s.query.trim() !== query || (error instanceof DOMException && error.name === "AbortError")) return "";
      return query;
    }
  }

  function handleFollowUpHistory(items: FollowUpItem[]) {
    s.update(state => {
      if (!state.result) return state;
      return {
        ...state,
        result: {
          ...state.result,
          follow_ups: items
        }
      };
    });
  }

  async function addCurrentWordToLearning() {
    if (!$s.result || $s.result.entry_id <= 0) return;
    isAddingToLearning = true;
    try {
      await addWordToLearning($s.result.entry_id);
      learningEntryIds = new Set([...learningEntryIds, $s.result.entry_id]);
    } catch (e) {
      s.setError("加入学习失败");
    } finally {
      isAddingToLearning = false;
    }
  }

  async function handleDeleteAttachedPhrase(item: AttachedPhraseModule) {
    if (!$s.result || $s.result.entry_id <= 0) return;

    try {
      isUpdatingPhraseAttachment = true;
      s.update(state => ({ ...state, isLoading: true, error: "" }));
      const updated = await deletePhraseModuleFromEntry($s.result.entry_id, {
        source_phrase_entry_id: item.source_phrase_entry_id,
        phrase: item.phrase,
      });
      s.setQuery(updated.query_text);
      s.setResult(updated);
      void fetchRecentItems();
    } catch (e) {
      s.setError("短语移除失败，请稍后重试。");
    } finally {
      isUpdatingPhraseAttachment = false;
      s.update(state => ({ ...state, isLoading: false }));
    }
  }

  async function handleAddPhraseModule(phrase: string) {
    if (!$s.result || $s.result.entry_id <= 0) return;

    try {
      isAddingPhraseModule = true;
      s.update(state => ({ ...state, error: "" }));
      const updated = await addPhraseModuleToEntry($s.result.entry_id, {
        phrase,
        quality_mode: $s.result.quality_mode ?? "default",
      });
      s.setQuery(updated.query_text);
      s.setResult(updated);
      void fetchRecentItems();
    } catch (e) {
      s.setError(e instanceof Error ? e.message : "短语卡片添加失败，请稍后重试。");
      throw e;
    } finally {
      isAddingPhraseModule = false;
    }
  }

  function clearSearchResult() {
    dismissSearchFocus();
    currentSearchController?.abort();
    s.reset();
    advancedPending = null;
    showAdvanced = false;
    advancedHint = "";
    failedAnalysis = null;
  }

  function setAnalysisFailure(
    message: string,
    request: FailedAnalysisRequest,
    previousResult: AnalyzeResponse | null,
  ) {
    failedAnalysis = request;
    s.update(state => ({
      ...state,
      error: message,
      result: request.forceRefresh ? previousResult : null,
    }));
  }

  function retryFailedAnalysis(modelOverride: AiModelOverride | null) {
    const request = failedAnalysis;
    if (!request) return;
    void handleSearch(
      request.query,
      request.qualityMode,
      request.forceRefresh,
      request.generationHint,
      false,
      modelOverride,
    );
  }

  async function cancelSearch() {
    currentSearchController?.abort();
    s.update(state => ({ ...state, isLoading: false, isStreaming: false }));
    // 如果还没结果，清空占位
    if (!$s.result || $s.result.entry_id === 0) {
      s.setResult(null);
    }
  }

  // 键盘交互优化
  function handleKeyDown(e: KeyboardEvent) {
    if (composing || e.isComposing || e.keyCode === 229 || (e.key === "Enter" && performance.now() - compositionEndedAt < 80)) {
      if (e.key === "Enter") e.preventDefault();
      return;
    }
    if (e.key === "Tab") {
      e.preventDefault();
      void toggleAdvanced();
    }
    if (e.key === "Escape") {
      if (showSuggestions) {
        dismissSearchFocus();
      } else if (showAdvanced) {
        void toggleAdvanced();
      } else if (searchFocused) {
        dismissSearchFocus();
      } else if (isSearching) {
        clearSearchResult();
      }
    }
  }

  onMount(() => {
    hasMounted = true;
    void fetchRecentItems();
    void fetchLearningProgress();
    void fetchAiSettings();
    // 软键盘通常只缩小 visualViewport，布局视口与 dvh 仍可能保持原高度。
    const viewport = window.visualViewport;
    function updateViewport() {
      viewportHeight = viewport?.height ?? window.innerHeight;
      viewportTop = viewport?.offsetTop ?? 0;
    }
    updateViewport();
    viewport?.addEventListener("resize", updateViewport);
    viewport?.addEventListener("scroll", updateViewport);
    window.addEventListener("resize", updateViewport);
    return () => {
      cancelSuggestions();
      searchMoveAnimation?.cancel();
      viewport?.removeEventListener("resize", updateViewport);
      viewport?.removeEventListener("scroll", updateViewport);
      window.removeEventListener("resize", updateViewport);
    };
  });

  const isSearching = $derived(Boolean($s.result || $s.isLoading || $s.error || advancedPending));
  const structuredResult = $derived($s.result ? resolveStructuredAnalysis(
    $s.result.analysis_markdown || "", $s.result.structured_analysis, $s.result.query_text,
  ) : null);
  $effect(() => {
    if (!browser || !active || !isSearching) return;
    const mainArea = document.querySelector<HTMLElement>(".main-area");
    if (!mainArea) return;
    const gesture = edgeSwipe(mainArea, handleResultBack);
    return () => gesture.destroy();
  });
  const isCurrentWordInLearning = $derived(Boolean($s.result && learningEntryIds.has($s.result.entry_id)));
  // 派生：是否处于专注搜索状态（唤起建议面板时）
  const isSearchFocusing = $derived(showSuggestions && (suggestionsLoading || suggestionsError || suggestionsQuery === $s.query.trim() || suggestions.length > 0 || (!$s.query.trim() && recentItems.length > 0)));
  $effect(() => {
    if (!suggestionPanel || !searchFocused) return;
    const panel = suggestionPanel;
    const measure = () => {
      const viewport = window.visualViewport;
      const bottom = (viewport?.offsetTop ?? 0) + (viewport?.height ?? window.innerHeight);
      const nav = document.querySelector<HTMLElement>(".bottom-nav");
      const navTop = nav && getComputedStyle(nav).display !== "none" && getComputedStyle(nav).visibility !== "hidden" ? nav.getBoundingClientRect().top : undefined;
      const panelTop = panel.getBoundingClientRect().top;
      const visibleBottom = navTop !== undefined && navTop > panelTop ? Math.min(bottom, navTop) : bottom;
      suggestionPanelHeight = Math.max(0, visibleBottom - panelTop - 12);
    };
    measure();
    const frame = requestAnimationFrame(measure);
    const observer = new ResizeObserver(measure);
    observer.observe(panel.parentElement!);
    window.visualViewport?.addEventListener("resize", measure);
    window.visualViewport?.addEventListener("scroll", measure);
    window.addEventListener("resize", measure);
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      window.visualViewport?.removeEventListener("resize", measure);
      window.visualViewport?.removeEventListener("scroll", measure);
      window.removeEventListener("resize", measure);
    };
  });
  const actionModelOptions = $derived(buildActionModelOptions(aiSettings));
  const selectedActionModelOverride = $derived(
    actionModelOverrideForKey(actionModelOptions, selectedActionModelKey)
  );
</script>

<div id="page-search" class="search-page-container" class:active={active} class:is-searching={isSearching}
  style:--search-viewport-height={viewportHeight === null ? "100dvh" : `${viewportHeight}px`}
  style:--search-viewport-top={`${viewportTop}px`}>
  <!-- 搜索专注遮罩层 -->
  {#if searchFocused}
    <button class="search-overlay" aria-label="关闭搜索建议" transition:fade={{ duration: 200 }} onclick={dismissSearchFocus}></button>
  {/if}

  <div class="search-main-wrapper" class:result-chrome={isSearching} class:focusing={searchFocused}
    onfocusout={(event) => {
      if (!switchingInput && !event.currentTarget.contains(event.relatedTarget as Node | null)) {
        void setSearchFocus(false);
        cancelSuggestions();
        showSuggestions = false;
      }
    }}>
    {#if isSearching}
      <button type="button" class="back-btn" onclick={handleResultBack} aria-label={searchFocused ? "收起搜索" : "返回首页"}>
        <i class="ph ph-arrow-left" aria-hidden="true"></i>
      </button>
      <div class="result-identity">
        <h1>{structuredResult?.headword || $s.result?.query_text || $s.query}</h1>
        {#if structuredResult?.phonetic}
          <div class="result-phonetic">{structuredResult.phonetic}</div>
        {/if}
      </div>
      <button type="button" class="result-search-open" onclick={() => $s.isLoading ? cancelSearch() : openResultSearch()} aria-label={$s.isLoading ? "取消分析" : "展开搜索"} aria-expanded={searchFocused}>
        <i class={$s.isLoading ? "ph-fill ph-stop" : "ph ph-magnifying-glass"} aria-hidden="true"></i>
      </button>
    {/if}
    {#if !isSearching && hasMounted}
      <div class="search-intro" in:fade={{ delay: 100 }}>
        <h1>发现，不仅仅是查词</h1>
        <p>基于 AI 的深度德语词义解析与联想助手</p>
      </div>
    {/if}

    <div class="search-panel-group" class:has-suggestions={isSearchFocusing}>
      <form class="search-form" onsubmit={(e) => { e.preventDefault(); if (composing || performance.now() - compositionEndedAt < 80) return; (showAdvanced && advancedHint.trim()) ? handleAdvancedSearch() : handleSearch(); }}>
        <div class="input-row main-row" class:is-generating-box={$s.isStreaming}>
          <input
            bind:this={mainInputRef}
            bind:value={$s.query}
            placeholder="输入德语单词或中文含义..."
            aria-label="查询德语单词或中文含义"
            oninput={(event) => {
              s.setQuery(event.currentTarget.value);
              visibleSuggestionCount = 20;
              if (suggestionList) suggestionList.scrollTop = 0;
              fetchSuggestions();
            }}
            oncompositionstart={() => { composing = true; cancelSuggestions(); }}
            oncompositionend={() => { composing = false; compositionEndedAt = performance.now(); fetchSuggestions(); }}
            onkeydown={handleKeyDown}
            onfocus={() => { void setSearchFocus(true); showSuggestions = true; fetchSuggestions(); }}
            autocomplete="off"
          />
          <div class="actions">
            {#if $s.query}
              <button type="button" class="icon-btn" onclick={() => { s.setQuery(""); fetchSuggestions(); focusSoon(() => mainInputRef); }} title="清空">
                <i class="ph ph-x"></i>
              </button>
            {/if}
            <button type="button" class="shortcut-btn" aria-label="联想线索" aria-expanded={showAdvanced} class:is-active={showAdvanced} onclick={toggleAdvanced}>
              <i class="ph ph-magic-wand"></i><span>↹ Tab 联想</span>
            </button>
            <button
              type={$s.isLoading ? "button" : "submit"}
              class="btn-primary search-submit-btn"
              class:is-canceling={$s.isLoading}
              aria-label={$s.isLoading ? "取消分析" : "分析"}
              onclick={(event) => { if ($s.isLoading) { event.preventDefault(); void cancelSearch(); } }}
            >
              {#if $s.isLoading}
                <span class="btn-text-content loading-text"><i class="ph-bold ph-spinner-gap spin"></i><span class="search-submit-label">分析中</span></span>
                <span class="btn-text-content cancel-text"><i class="ph-fill ph-stop"></i><span class="search-submit-label">取消</span></span>
              {:else}
                <i class="ph-bold ph-magnifying-glass"></i><span class="search-submit-label">分析</span>
              {/if}
            </button>
          </div>
        </div>

        {#if showAdvanced}
          <div class="input-row sub-row">
            <i class="ph ph-magic-wand sub-icon"></i>
            <input
              bind:this={advancedInputRef}
              bind:value={advancedHint}
              placeholder="添加线索，例如：关于法律、口语、或者它的近义词..."
              onfocus={() => { void setSearchFocus(true); showSuggestions = false; }}
              onkeydown={(e) => { if(e.key === "Tab") handleKeyDown(e); if(e.key === "Escape") handleKeyDown(e); }}
              autocomplete="off"
            />
          </div>
        {/if}
      </form>

      {#if isSearchFocusing}
        <div class="integrated-suggestions" bind:this={suggestionPanel} style:--suggestion-available-height={`${suggestionPanelHeight}px`} in:fade={{ duration: 150 }} out:fade={{ duration: 100 }}>
          <div class="panel-header">
            { !$s.query.trim() ? "最近查询记录" : isSemanticQuery($s.query) ? "中文找词" : "词汇建议" }
            {#if suggestionsLoading}<span role="status"> · 查询中…</span>{/if}
          </div>
          <div class="suggestion-list" bind:this={suggestionList} aria-busy={suggestionsLoading}>
            {#if !$s.query.trim()}
              {#each recentItems as item}
                <button class="suggestion-item" onclick={() => handleSearch(item.query_text, "default", false, "", false)}>
                  <i class="ph ph-clock-counter-clockwise"></i>
                  <div class="text-content">
                    <span class="q">{item.query_text}</span>
                    <span class="p">{item.preview}</span>
                  </div>
                </button>
              {/each}
            {:else}
              {#if suggestionsError}
                <button class="suggestion-item" onclick={() => { mainInputRef?.focus({ preventScroll: true }); fetchSuggestions(); }}>{suggestionsError}</button>
              {:else if !suggestionsLoading && suggestionsQuery === $s.query.trim() && !suggestions.length}
                <p class="candidate-status">暂无候选，请换个说法</p>
              {/if}
              {#each suggestions.slice(0, visibleSuggestionCount) as sg}
                <button class="suggestion-item" disabled={suggestionsQuery !== $s.query.trim()} onclick={() => handleSearch(sg.query_text, "default", false, "", false)}>
                  <i class="ph ph-magnifying-glass"></i>
                  <div class="text-content">
                    <span class="q">{sg.query_text}</span>
                    <span class="p">{markdownToPlainText(sg.preview)}</span>
                  </div>
                </button>
              {/each}
              {#if suggestions.length > visibleSuggestionCount && suggestionsQuery === $s.query.trim()}
                <button class="suggestion-item load-more" onmousedown={(e) => e.preventDefault()} onclick={() => { visibleSuggestionCount += 20; }}>加载更多（已显示 {visibleSuggestionCount} / {suggestions.length}）</button>
              {/if}
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>

  <div class="result-area" class:visible={isSearching}>
    {#if $s.error && failedAnalysis}
      <AnalysisRetryCard
        message={$s.error}
        options={actionModelOptions}
        selectedKey={selectedActionModelKey}
        disabled={$s.isLoading}
        onModelChange={(key) => selectedActionModelKey = key}
        onRetryOriginal={() => retryFailedAnalysis(failedAnalysis?.modelOverride ?? null)}
        onRetrySelected={() => retryFailedAnalysis(selectedActionModelOverride)}
      />
    {:else if $s.error}
      <div class="message-surface error" transition:fade>{$s.error}</div>
    {/if}

    {#if advancedPending}
      <div class="bento-card" transition:fade>
        <div class="card-title"><i class="ph-fill ph-brain"></i> 联想找词状态</div>
        <p class="card-copy">{advancedPending.query_text}</p>
        <div class="markdown-compact">{@html renderMarkdownHtml(advancedPending.analysis_markdown)}</div>
      </div>
    {/if}

    {#if $s.isLoading && !$s.result}
      <div class="loading-skeleton">
        <div class="bento-card" style="height: 120px;"><div class="skeleton-block" style="width: 40%;"></div></div>
        <div class="bento-card" style="height: 300px;"><div class="skeleton-block"></div></div>
      </div>
    {:else if $s.result && structuredResult}
      <SearchResultCards
        result={$s.result}
        structured={structuredResult}
        isStreaming={$s.isStreaming}
        isAddingToLearning={isAddingToLearning}
        isLearningAdded={isCurrentWordInLearning}
        isUpdatingPhraseAttachment={isUpdatingPhraseAttachment}
        isAddingPhraseModule={isAddingPhraseModule}
        recentItems={recentItems}
        onAddToLearning={addCurrentWordToLearning}
        actionModelOptions={actionModelOptions}
        selectedActionModelKey={selectedActionModelKey}
        selectedActionModelOverride={selectedActionModelOverride}
        onActionModelChange={(key) => selectedActionModelKey = key}
        onRegenerate={(mode, hint, modelOverride = null) => handleSearch($s.query, mode, true, hint, true, modelOverride)}
        onSelectRecent={(q) => handleSearch(q, "default", false, "", false)}
        onDeleteAttachedPhrase={handleDeleteAttachedPhrase}
        onAddPhraseModule={handleAddPhraseModule}
        onhistoryChange={handleFollowUpHistory}
      />
    {/if}
  </div>
</div>

<style>
  .search-page-container { width: 100%; display: flex; flex-direction: column; align-items: center; position: relative; }

  .search-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.1);
    backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
    z-index: 90;
  }

  .search-main-wrapper {
    --theme-clearance: 0px;
    width: 100%; max-width: 740px; margin-top: 25vh;
    z-index: 100; position: relative;
  }
  .is-searching .search-main-wrapper {
    position: sticky;
    top: calc(var(--search-viewport-top) - 2rem);
    display: grid;
    grid-template-columns: calc(44px + var(--theme-clearance)) minmax(0, 1fr) calc(44px + var(--theme-clearance));
    column-gap: 12px;
    align-items: center;
    width: calc(100vw - var(--desktop-sidebar-width));
    min-height: var(--desktop-top-chrome-height);
    max-width: none;
    margin: -2rem -2rem 0;
    padding: 12px 32px;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-color);
  }
  .search-main-wrapper.focusing:not(.result-chrome) {
    position: fixed;
    top: calc(var(--search-viewport-top) + 24px);
    left: calc(var(--desktop-sidebar-width) + 32px);
    right: calc(32px + var(--theme-clearance));
    width: auto;
    max-width: 740px;
    height: auto;
    margin: 0 auto;
    padding: 0;
    border: none;
    background: transparent;
  }
  .focusing .search-intro { display: none; }
  .search-main-wrapper.focusing:not(.result-chrome) .search-panel-group {
    width: 100%;
    max-width: none;
    display: flex;
    flex-direction: column;
    max-height: calc(var(--search-viewport-height) - 48px - env(safe-area-inset-bottom));
  }
  .focusing .search-form { flex-shrink: 0; }
  .focusing .panel-header { flex-shrink: 0; }
  .focusing .suggestion-list { min-height: 0; }

  .search-intro { margin-bottom: 2.5rem; pointer-events: none; }
  .search-intro h1 { font-size: clamp(2rem, 5vw, 3rem); font-weight: 800; letter-spacing: -0.03em; margin-bottom: 0.5rem; color: var(--text-main); }
  .search-intro p { margin-left: 14rem; color: var(--text-muted); font-size: 1.15rem; }

  .search-panel-group {
    position: relative; width: 100%; display: flex; flex-direction: column; gap: 8px;
  }

  .has-suggestions .main-row { box-shadow: var(--shadow-hover); }

  .search-form {
    display: flex; flex-direction: column; gap: 8px;
  }
  .input-row {
    border: 1px solid var(--border-color); border-radius: 9999px;
    box-shadow: var(--shadow-sm);
    display: flex; align-items: center; padding: 0.5rem 0.5rem 0.5rem 1.5rem; gap: 1rem;
  }
  .input-row input { flex: 1; min-width: 0; background: transparent; font-size: 1.25rem; color: var(--text-main); height: 3.5rem; }

  .main-row {
    position: relative;
    z-index: 2;
    background: var(--control-bg);
    /* 右侧内距等于按钮的上下留白，让内外胶囊的右端圆弧同心。 */
    padding-right: calc(0.5rem + (3.5rem - 2.8rem) / 2);
  }
  .sub-row {
    padding: 0.85rem 1.5rem;
    background: var(--bg-color);
  }
  .sub-row input { font-size: 1.05rem; height: 2.5rem; }

  .actions { display: flex; align-items: center; gap: 0.5rem; }
  .icon-btn { background: transparent; color: var(--text-muted); width: 2.5rem; height: 2.5rem; display: flex; align-items: center; justify-content: center; border-radius: 50%; }
  .icon-btn:hover { background: var(--btn-secondary); color: var(--text-main); }
  .shortcut-btn { background: transparent; color: var(--text-muted); font-size: 0.9rem; font-weight: 700; display: flex; align-items: center; gap: 0.4rem; padding: 0 0.75rem; height: 44px; border-radius: 9999px; white-space: nowrap; flex-shrink: 0; }
  .shortcut-btn:hover { color: var(--accent-main); }
  .shortcut-btn.is-active { color: var(--accent-main); background: var(--btn-secondary); }
  .shortcut-btn:focus-visible { outline: 2px solid var(--accent-main); outline-offset: 2px; }

  .back-btn {
    grid-column: 1; grid-row: 1; justify-self: start; align-self: start;
    width: 44px; height: 44px; border-radius: 50%;
    background: transparent; color: var(--text-main);
    display: flex; align-items: center; justify-content: center;
  }
  .back-btn:hover, .back-btn:active { background: var(--btn-secondary); }
  .back-btn:focus-visible { outline: 2px solid var(--accent-main); outline-offset: 2px; }

  .result-identity { grid-column: 2; grid-row: 1; min-width: 0; text-align: center; transition: opacity 180ms ease; }
  .result-identity h1 { margin: 0; color: var(--text-main); font-size: 40px; font-weight: 800; line-height: 1.1; letter-spacing: -0.02em; overflow-wrap: anywhere; }
  .result-phonetic { margin-top: 4px; font-family: "Arial Unicode MS", "Lucida Sans Unicode", sans-serif; color: var(--text-muted); font-size: 16px; line-height: 1.4; overflow-wrap: anywhere; }
  .result-search-open { grid-column: 3; grid-row: 1; justify-self: end; align-self: start; width: 44px; height: 44px; margin-right: var(--theme-clearance); border-radius: 50%; background: transparent; color: var(--text-main); display: flex; align-items: center; justify-content: center; }
  .result-search-open:hover, .result-search-open:active { background: var(--btn-secondary); }
  .result-search-open:focus-visible { outline: 2px solid var(--accent-main); outline-offset: 2px; }
  .back-btn, .result-search-open { font-size: 24px; }
  .result-chrome.focusing .result-identity, .result-chrome.focusing .result-search-open { opacity: 0; visibility: hidden; pointer-events: none; }
  .result-chrome .search-panel-group {
    --result-search-wide: 0;
    position: absolute; top: 12px; left: 88px; right: calc(32px + var(--theme-clearance)); width: auto;
    max-height: calc(var(--search-viewport-height) - 24px - env(safe-area-inset-bottom));
    visibility: hidden; pointer-events: none;
  }
  .result-chrome.focusing .search-panel-group { visibility: visible; pointer-events: auto; }
  .result-chrome .search-form { flex-shrink: 0; }
  .result-chrome .input-row { padding: 6px 16px; gap: 4px; }
  .result-chrome .main-row { padding-right: 6px; }
  .result-chrome .input-row input { font-size: 16px; height: 44px; }
  .result-chrome .actions { gap: 0; flex-shrink: 0; }
  .result-chrome .actions .icon-btn { width: 32px; height: 44px; flex-shrink: 0; }
  .result-chrome .shortcut-btn { display: flex; width: 44px; padding: 0; justify-content: center; }
  .result-chrome .shortcut-btn span { display: none; }
  .result-chrome .search-submit-btn { width: 44px; height: 44px; flex-shrink: 0; gap: 0; }
  .result-chrome .search-submit-label { display: none; }
  .result-chrome .search-submit-btn i { font-size: 18px; line-height: 1; }
  .result-chrome .btn-text-content { gap: 0; }
  .result-chrome .search-submit-btn.is-canceling .loading-text { display: none; }
  .result-chrome .search-submit-btn.is-canceling .cancel-text { transform: none; opacity: 1; }

  @media (min-width: 769px) {
    .search-main-wrapper { --theme-clearance: 96px; }
  }

  @media (min-width: 1201px) {
    .is-searching .search-main-wrapper {
      --search-column-width: calc(var(--theme-clearance) + clamp(220px, calc((100vw - var(--desktop-sidebar-width) - 64px - 2 * var(--theme-clearance)) * 0.3), 340px));
      grid-template-columns: var(--search-column-width) minmax(0, 1fr) var(--search-column-width);
    }
    .result-chrome .search-panel-group {
      --result-search-wide: 1;
      grid-column: 3; grid-row: 1; position: relative; top: auto; left: auto; right: auto;
      width: calc(100% - var(--theme-clearance)); align-self: start; visibility: visible; pointer-events: auto;
    }
    .result-chrome .result-search-open { display: none; }
    .result-chrome.focusing .result-identity { opacity: 1; visibility: visible; }
    .result-chrome:not(.focusing) .actions .icon-btn, .result-chrome:not(.focusing) .shortcut-btn { display: none; }
    .result-chrome .integrated-suggestions, .result-chrome .sub-row { position: absolute; right: 0; width: min(420px, calc(100vw - var(--desktop-sidebar-width) - 64px - var(--theme-clearance))); }
    .result-chrome .sub-row { top: 66px; }
    .result-chrome .integrated-suggestions { top: 66px; }
  }

  .search-submit-btn {
    position: relative;
    overflow: hidden;
    width: 7.2rem;
    height: 2.8rem;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .search-submit-btn:hover { transform: none; }

  .btn-text-content {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    transition: all 0.3s cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .cancel-text {
    transform: translateY(100%);
    opacity: 0;
    color: var(--text-main);
  }

  .search-submit-btn.is-canceling:hover {
    background: var(--btn-secondary);
    border: 1px solid var(--border-color);
    color: var(--text-main);
    box-shadow: none;
  }

  .search-submit-btn.is-canceling:hover .loading-text {
    transform: translateY(-100%);
    opacity: 0;
  }

  .search-submit-btn.is-canceling:hover .cancel-text {
    transform: translateY(0);
    opacity: 1;
  }

  .integrated-suggestions {
    max-height: var(--suggestion-available-height, 460px);
    display: flex; flex-direction: column; min-height: 0;
    background: var(--card-bg); border: 1px solid var(--border-color);
    border-radius: 24px;
    box-shadow: 0 20px 40px rgba(0,0,0,0.12); z-index: 200; overflow: hidden;
  }

  .panel-header { padding: 0.75rem 1.5rem; font-size: 0.75rem; font-weight: 800; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.1em; background: var(--bg-color); }

  .suggestion-list {
    min-height: 0;
    overscroll-behavior: contain;
    max-height: 420px;
    overflow-y: auto;
    scrollbar-gutter: stable;
    border-bottom-left-radius: inherit;
    border-bottom-right-radius: inherit;
    background: var(--card-bg);
  }
  .candidate-status { padding: 16px 24px; color: var(--text-muted); }
  .load-more { justify-content: center; color: var(--text-main); }
  .suggestion-item:disabled { opacity: 0.5; cursor: wait; }
  .suggestion-item {
    width: 100%; padding: 1.1rem 1.5rem; display: flex; align-items: flex-start;
    gap: 1.25rem; border-bottom: 1px solid var(--border-color); background: transparent; text-align: left;
  }
  .suggestion-item:last-child {
    border-bottom: none;
    border-bottom-left-radius: inherit;
    border-bottom-right-radius: inherit;
  }
  .suggestion-item:hover { background: var(--btn-secondary); }
  .suggestion-item i { margin-top: 0.3rem; color: var(--text-muted); font-size: 1.1rem; }
  .text-content { display: flex; flex-direction: column; gap: 0.25rem; min-width: 0; }
  .suggestion-item .q { overflow-wrap: anywhere; }
  .suggestion-item .q { font-weight: 700; color: var(--text-main); font-size: 1.1rem; }
  .suggestion-item .p { font-size: 0.9rem; color: var(--text-muted); line-height: 1.4; display: -webkit-box; -webkit-line-clamp: 1; -webkit-box-orient: vertical; overflow: hidden; }

  .result-area { width: 100%; margin-top: 2.5rem; display: none; }
  .result-area.visible { display: block; animation: fadeIn 0.5s ease; padding-bottom: 5rem; }

  .loading-skeleton { display: flex; flex-direction: column; gap: 1.5rem; }

  @media (min-width: 601px) and (max-width: 1200px) {
    .result-identity h1 { font-size: 32px; }
  }

  @media (max-width: 768px) {
    .search-intro p { margin-left: 0; text-align: right; }
    .back-btn { position: static; flex: 0 0 44px; }
    .search-main-wrapper { margin-top: 15vh; }
    .is-searching .search-main-wrapper { top: calc(var(--search-viewport-top) - 1.5rem); width: calc(100% + 2rem); min-height: 72px; margin: -1.5rem -1rem 0; padding: 8px 16px; }
    .result-chrome .search-panel-group { top: 8px; left: 72px; right: 16px; max-height: calc(var(--search-viewport-height) - 16px - env(safe-area-inset-bottom)); }
    .input-row { padding: 6px 16px; gap: 4px; }
    .main-row { padding-right: 6px; }
    .input-row input { height: 44px; font-size: 16px; }
    .actions { gap: 0; flex-shrink: 0; }
    .actions .icon-btn { width: 32px; height: 44px; flex-shrink: 0; }
    .shortcut-btn { display: flex; width: 44px; height: 44px; padding: 0; justify-content: center; }
    .shortcut-btn span { display: none; }
    .search-submit-btn { width: 44px; height: 44px; flex-shrink: 0; gap: 0; }
    .search-submit-btn .search-submit-label { display: none; }
    .search-submit-btn i { font-size: 18px; line-height: 1; }
    .btn-text-content { gap: 0; }
    .search-submit-btn.is-canceling .loading-text { display: none; }
    .search-submit-btn.is-canceling .cancel-text { transform: none; opacity: 1; }
    .search-main-wrapper.focusing:not(.result-chrome) { top: calc(var(--search-viewport-top) + 8px); left: 16px; right: 16px; width: auto; margin: 0; padding: 0; }
    .search-main-wrapper.focusing:not(.result-chrome) .search-panel-group { max-height: calc(var(--search-viewport-height) - 16px - env(safe-area-inset-bottom)); }
    .suggestion-list { overscroll-behavior: contain; }
    .integrated-suggestions { border-radius: 20px; }
    .sub-row { padding: 6px 16px; gap: 8px; }
  }

  @media (max-width: 600px) {
    .result-identity h1 { font-size: 26px; }
    .result-phonetic { font-size: 13px; }
    .search-main-wrapper { margin-top: clamp(40px, 12dvh, 96px); }
    .search-intro { margin-bottom: 16px; padding-inline: 16px; text-align: left; }
    .search-intro h1 { font-size: 26px; line-height: 1.2; margin-bottom: 8px; }
    .search-intro p { font-size: 12px; line-height: 1.5; text-align: right; }
    .result-area { margin-top: 16px; }
    .panel-header { padding: 8px 12px; font-size: 11px; }
    .suggestion-item { padding: 10px 12px; gap: 8px; }
    .suggestion-item .q { font-size: 14px; }
    .suggestion-item .p { font-size: 12px; }
  }

  @media (prefers-reduced-motion: reduce) {
    .result-identity { transition: none; }
  }

</style>
