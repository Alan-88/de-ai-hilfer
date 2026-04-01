<script lang="ts">
  import { onMount } from "svelte";
  import { browser } from "$app/environment";
  import { fade, slide } from "svelte/transition";
  import { addWordToLearning } from "$lib/learningApi";
  import { analyzeWord, attachPhraseToHost, detachPhraseFromHost, getRecentEntries, getSuggestions, intelligentSearch } from "$lib/queryApi";
  import SearchResultCards from "$lib/components/bento/SearchResultCards.svelte";
  import { markdownToPlainText, renderMarkdownHtml } from "$lib/analysis/structuredAnalysis";
  import { shouldPreferDirectAnalyze } from "$lib/search/searchExecution";
  import { streamAnalyze } from "$lib/streamApi";
  import { searchStore } from "$lib/stores/search";
  import type {
    AnalyzeResponse,
    AttachedPhraseModule,
    DBSuggestion,
    FollowUpItem,
    QualityMode,
    RecentItem,
  } from "$lib/types";

  let { active = false, showAdvancedOnMount = false } = $props();

  // 本地 UI 状态
  let recentItems = $state<RecentItem[]>([]);
  let suggestions = $state<DBSuggestion[]>([]);
  let showSuggestions = $state(false);
  let isAddingToLearning = $state(false);
  let showAdvanced = $state(showAdvancedOnMount);
  let advancedHint = $state("");
  let advancedPending = $state<AnalyzeResponse | null>(null);
  let isAdvancedLoading = $state(false);
  let isUpdatingPhraseAttachment = $state(false);
  let currentSearchController: AbortController | null = null;
  let hasMounted = $state(false);
  let mainInputRef = $state<HTMLInputElement | null>(null);
  let advancedInputRef = $state<HTMLInputElement | null>(null);

  const s = searchStore;

  function focusSoon(target: HTMLInputElement | null) {
    requestAnimationFrame(() => target?.focus());
  }

  async function handleSearch(
    term = $s.query,
    qualityMode: QualityMode = "default",
    forceRefresh = false,
    generationHint = ""
  ) {
    const query = term.trim();
    if (!query) return;

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
              phrase_usage_preview: null,
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
            s.update(state => ({ ...state, result: payload, activeModel: payload.model ?? state.activeModel }));
            void fetchRecentItems();
          },
          onError: (payload) => {
            s.setError(payload.message);
          },
        }
      );
    } catch (searchError) {
      if (!(searchError instanceof DOMException && searchError.name === "AbortError")) {
        s.setError(searchError instanceof Error ? searchError.message : "查询失败");
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
    isAdvancedLoading = true;
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

  async function fetchSuggestions() {
    if ($s.query.trim().length < 2) {
      suggestions = [];
      showSuggestions = false;
      return;
    }
    try {
      const resp = await getSuggestions($s.query);
      suggestions = resp.suggestions;
      showSuggestions = suggestions.length > 0;
    } catch {
      suggestions = [];
    }
  }

  function handleNewFollowUp(item: FollowUpItem) {
    s.update(state => {
      if (!state.result) return state;
      return {
        ...state,
        result: {
          ...state.result,
          follow_ups: [...state.result.follow_ups, item]
        }
      };
    });
  }

  async function addCurrentWordToLearning() {
    if (!$s.result || $s.result.entry_id <= 0) return;
    isAddingToLearning = true;
    try {
      await addWordToLearning($s.result.entry_id);
    } catch (e) {
      s.setError("加入学习失败");
    } finally {
      isAddingToLearning = false;
    }
  }

  async function handlePhraseHostSelection(headword: string, mode: "attach" | "view" = "attach") {
    const host = headword.trim();
    if (!host) return;

    if (mode === "view") {
      await handleSearch(host);
      return;
    }

    if (!$s.result?.phrase_lookup || $s.result.entry_id <= 0) {
      await handleSearch(host);
      return;
    }

    try {
      isUpdatingPhraseAttachment = true;
      s.update(state => ({ ...state, isLoading: true, error: "" }));
      const attached = await attachPhraseToHost({
        phrase_entry_id: $s.result.entry_id > 0 ? $s.result.entry_id : null,
        host_headword: host,
        phrase: $s.result.query_text,
        phrase_lookup: $s.result.phrase_lookup ?? null,
        phrase_usage_preview: $s.result.phrase_usage_preview ?? null,
        analysis_markdown: $s.result.analysis_markdown,
        model: $s.result.model,
        quality_mode: $s.result.quality_mode
      });
      s.setQuery(attached.query_text);
      s.setResult(attached);
      advancedPending = null;
      showSuggestions = false;
      showAdvanced = false;
      advancedHint = "";
      void fetchRecentItems();
    } catch (e) {
      s.setError("短语挂载失败，已切换为查看主词。");
      await handleSearch(host);
    } finally {
      isUpdatingPhraseAttachment = false;
      s.update(state => ({ ...state, isLoading: false }));
    }
  }

  async function handleDetachPhraseHost(item: AttachedPhraseModule) {
    if (!$s.result || $s.result.entry_id <= 0) return;

    try {
      isUpdatingPhraseAttachment = true;
      s.update(state => ({ ...state, isLoading: true, error: "" }));
      const detached = await detachPhraseFromHost({
        host_entry_id: $s.result.entry_id,
        source_phrase_entry_id: item.source_phrase_entry_id,
      });
      s.setQuery(detached.query_text);
      s.setResult(detached);
      void fetchRecentItems();
    } catch (e) {
      s.setError("短语移除失败，请稍后重试。");
    } finally {
      isUpdatingPhraseAttachment = false;
      s.update(state => ({ ...state, isLoading: false }));
    }
  }

  function clearSearchResult() {
    currentSearchController?.abort();
    s.reset();
    advancedPending = null;
    showAdvanced = false;
    advancedHint = "";
  }

  // 键盘交互优化
  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Tab") {
      e.preventDefault();
      if (showAdvanced) {
        showAdvanced = false;
        focusSoon(mainInputRef);
      } else {
        showAdvanced = true;
        focusSoon(advancedInputRef);
      }
    }
    if (e.key === "Escape") {
      if (showSuggestions) {
        showSuggestions = false;
      } else if (showAdvanced) {
        showAdvanced = false;
        focusSoon(mainInputRef);
      } else if (isSearching) {
        clearSearchResult();
      }
    }
  }

  onMount(() => {
    hasMounted = true;
    void fetchRecentItems();
  });

  const isSearching = $derived(Boolean($s.result || $s.isLoading || $s.error || advancedPending));
  // 派生：是否处于专注搜索状态（唤起建议面板时）
  const isSearchFocusing = $derived(showSuggestions && (suggestions.length > 0 || (!$s.query.trim() && recentItems.length > 0)));
</script>

<div id="page-search" class="search-page-container" class:active={active} class:is-searching={isSearching}>
  {#if isSearching}
    <button class="back-btn" onclick={clearSearchResult} transition:fade aria-label="返回首页">
      <i class="ph ph-arrow-left"></i>
    </button>
  {/if}

  <!-- 搜索专注遮罩层 -->
  {#if isSearchFocusing}
    <div class="search-overlay" transition:fade={{ duration: 200 }} onclick={() => showSuggestions = false}></div>
  {/if}

  <div class="search-main-wrapper" class:focusing={isSearchFocusing}>
    {#if !isSearching && hasMounted}
      <div class="search-intro" in:fade={{ delay: 100 }}>
        <h1>发现，不仅仅是查词</h1>
        <p>基于 AI 的深度德语词义解析与联想助手</p>
      </div>
    {/if}

    <div class="search-panel-group" class:has-suggestions={isSearchFocusing}>
      <form class="search-form" onsubmit={(e) => { e.preventDefault(); (showAdvanced && advancedHint.trim()) ? handleAdvancedSearch() : handleSearch(); }}>
        <div class="input-row main-row" class:is-generating-box={$s.isStreaming}>
          <i class="ph ph-magnifying-glass search-icon"></i>
          <input
            bind:this={mainInputRef}
            bind:value={$s.query}
            placeholder="输入德语单词、短语或近似拼写..."
            oninput={fetchSuggestions}
            onkeydown={handleKeyDown}
            onfocus={() => showSuggestions = true}
            autocomplete="off"
          />
          <div class="actions">
            {#if $s.query}
              <button type="button" class="icon-btn" onclick={() => { s.setQuery(""); focusSoon(mainInputRef); }} title="清空">
                <i class="ph ph-x"></i>
              </button>
            {/if}
            <button type="button" class="shortcut-btn" onclick={() => { showAdvanced = !showAdvanced; focusSoon(advancedInputRef); }}>
              <span>↹</span> Tab 联想
            </button>
            <button type="submit" class="btn-primary search-submit-btn" disabled={$s.isLoading}>
              {$s.isLoading ? "分析中" : "分析"}
            </button>
          </div>
        </div>

        {#if showAdvanced}
          <div class="input-row sub-row" transition:slide={{ duration: 300 }}>
            <i class="ph ph-magic-wand sub-icon"></i>
            <input
              bind:this={advancedInputRef}
              bind:value={advancedHint}
              placeholder="添加线索，例如：关于法律、口语、或者它的近义词..."
              onkeydown={(e) => { if(e.key === "Tab") handleKeyDown(e); if(e.key === "Escape") handleKeyDown(e); }}
              autocomplete="off"
            />
          </div>
        {/if}
      </form>

      {#if isSearchFocusing}
        <div class="integrated-suggestions" in:fade={{ duration: 150 }} out:fade={{ duration: 100 }}>
          <div class="panel-header">
            { !$s.query.trim() ? "最近查询记录" : "联想建议" }
          </div>
          <div class="suggestion-list">
            {#if !$s.query.trim()}
              {#each recentItems as item}
                <button class="suggestion-item" onclick={() => handleSearch(item.query_text)}>
                  <i class="ph ph-clock-counter-clockwise"></i>
                  <div class="text-content">
                    <span class="q">{item.query_text}</span>
                    <span class="p">{item.preview}</span>
                  </div>
                </button>
              {/each}
            {:else}
              {#each suggestions as sg}
                <button class="suggestion-item" onclick={() => handleSearch(sg.query_text)}>
                  <i class="ph ph-magnifying-glass"></i>
                  <div class="text-content">
                    <span class="q">{sg.query_text}</span>
                    <span class="p">{markdownToPlainText(sg.preview)}</span>
                  </div>
                </button>
              {/each}
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>

  <div class="result-area" class:visible={isSearching}>
    {#if $s.error}
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
    {:else if $s.result}
      <SearchResultCards
        result={$s.result}
        isStreaming={$s.isStreaming}
        isAddingToLearning={isAddingToLearning}
        isUpdatingPhraseAttachment={isUpdatingPhraseAttachment}
        recentItems={recentItems}
        onAddToLearning={addCurrentWordToLearning}
        onRegenerate={(mode, hint) => handleSearch($s.query, mode, true, hint)}
        onSelectRecent={(q) => handleSearch(q)}
        onSelectPhraseHost={handlePhraseHostSelection}
        onDetachAttachedPhrase={handleDetachPhraseHost}
        onnewFollowUp={handleNewFollowUp}
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
    width: 100%; max-width: 740px; margin-top: 25vh; 
    transition: all var(--transition-smooth); z-index: 100; position: relative;
  }
  .is-searching .search-main-wrapper { margin-top: 0; }
  .search-main-wrapper.focusing { transform: scale(1.02); }

  .search-intro { margin-bottom: 2.5rem; pointer-events: none; }
  .search-intro h1 { font-size: clamp(2rem, 5vw, 3rem); font-weight: 800; letter-spacing: -0.03em; margin-bottom: 0.5rem; color: var(--text-main); }
  .search-intro p { margin-left: 14rem; color: var(--text-muted); font-size: 1.15rem; }

  .search-panel-group { 
    position: relative; width: 100%; background: var(--card-bg);
    border: 1px solid var(--border-color); border-radius: 28px;
    box-shadow: var(--shadow-sm); transition: all var(--transition-smooth);
  }
  
  .has-suggestions { border-bottom-left-radius: 0; border-bottom-right-radius: 0; box-shadow: var(--shadow-hover); }

  .input-row { display: flex; align-items: center; padding: 0.5rem 0.5rem 0.5rem 1.5rem; gap: 1rem; }
  .input-row input { flex: 1; background: transparent; font-size: 1.25rem; color: var(--text-main); height: 3.5rem; }

  .main-row { position: relative; z-index: 2; }
  .sub-row { border-top: 1px solid var(--border-color); padding: 0.85rem 1.5rem; background: var(--bg-color); }
  .sub-row input { font-size: 1.05rem; height: 2.5rem; }

  .actions { display: flex; align-items: center; gap: 0.5rem; }
  .icon-btn { background: transparent; color: var(--text-muted); width: 2.5rem; height: 2.5rem; display: flex; align-items: center; justify-content: center; border-radius: 50%; }
  .icon-btn:hover { background: var(--btn-secondary); color: var(--text-main); }
  .shortcut-btn { background: transparent; color: var(--text-muted); font-size: 0.9rem; font-weight: 700; display: flex; align-items: center; gap: 0.4rem; padding: 0 0.75rem; white-space: nowrap; }
  .shortcut-btn:hover { color: var(--accent-main); }

  .back-btn {
    position: fixed; top: 1.5rem; left: calc(240px + 1.5rem); z-index: 150;
    width: 2.8rem; height: 2.8rem; border-radius: 50%;
    background: var(--card-bg); border: 1px solid var(--border-color);
    box-shadow: var(--shadow-sm); display: flex; align-items: center; justify-content: center;
  }

  .integrated-suggestions {
    position: absolute; top: 100%; left: -1px; right: -1px;
    background: var(--card-bg); border: 1px solid var(--border-color);
    border-top: none; border-bottom-left-radius: 24px; border-bottom-right-radius: 24px;
    box-shadow: 0 20px 40px rgba(0,0,0,0.12); z-index: 200; overflow: hidden;
  }

  .panel-header { padding: 0.75rem 1.5rem; font-size: 0.75rem; font-weight: 800; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.1em; background: var(--bg-color); }

  .suggestion-list { max-height: 420px; overflow-y: auto; }
  .suggestion-item {
    width: 100%; padding: 1.1rem 1.5rem; display: flex; align-items: flex-start;
    gap: 1.25rem; border-bottom: 1px solid var(--border-color); background: transparent; text-align: left;
  }
  .suggestion-item:last-child { border-bottom: none; }
  .suggestion-item:hover { background: var(--btn-secondary); }
  .suggestion-item i { margin-top: 0.3rem; color: var(--text-muted); font-size: 1.1rem; }
  .text-content { display: flex; flex-direction: column; gap: 0.25rem; }
  .suggestion-item .q { font-weight: 700; color: var(--text-main); font-size: 1.1rem; }
  .suggestion-item .p { font-size: 0.9rem; color: var(--text-muted); line-height: 1.4; display: -webkit-box; -webkit-line-clamp: 1; -webkit-box-orient: vertical; overflow: hidden; }

  .result-area { width: 100%; margin-top: 2.5rem; display: none; }
  .result-area.visible { display: block; animation: fadeIn 0.5s ease; padding-bottom: 5rem; }

  .loading-skeleton { display: flex; flex-direction: column; gap: 1.5rem; }

  @media (max-width: 768px) {
    .search-intro p { margin-left: 0; text-align: center; }
    .back-btn { left: 1rem; top: 1rem; width: 2.5rem; height: 2.5rem; }
    .search-main-wrapper { margin-top: 15vh; }
    .shortcut-btn { display: none; }
    .search-panel-group { border-radius: 20px; }
    .integrated-suggestions { border-bottom-left-radius: 20px; border-bottom-right-radius: 20px; }
  }
</style>
