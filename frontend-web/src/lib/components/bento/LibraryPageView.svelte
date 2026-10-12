<script lang="ts">
  import { goto } from "$app/navigation";
  import { deleteEntry, getEntryDetail, getLibraryEntriesPage } from "$lib/queryApi";
  import { getLearningProgress, addWordToLearning } from "$lib/learningApi";
  import LibraryEntryCard from "$lib/components/LibraryEntryCard.svelte";
  import LibraryEntryDetailPanel from "$lib/components/LibraryEntryDetailPanel.svelte";
  import { searchStore } from "$lib/stores/search";
  import type {
    EntryDetailResponse,
    LearningProgressView,
    LibraryCounts,
    LibraryEntriesPageResponse,
    LibraryTab,
    RecentItem,
  } from "$lib/types";

  let { active = false } = $props();
  const PAGE_SIZE = 24;

  // 原始数据状态
  let entries = $state<RecentItem[]>([]);
  let progressMap = $state<Record<number, LearningProgressView>>({});
  let isLoading = $state(false);
  let isLoadingMore = $state(false);
  let error = $state("");
  let totalCount = $state(0);
  let tabCounts = $state<LibraryCounts | null>(null);
  let nextCursor = $state<string | null>(null);
  let requestSerial = 0;
  
  // 过滤与 UI 状态
  let filterText = $state("");
  let activeTab = $state<LibraryTab>("all");
  
  // 详情侧滑窗
  let selectedEntryDetail = $state<EntryDetailResponse | null>(null);
  let isFetchingDetail = $state(false);
  let isDeleting = $state(false);
  let addingEntryId = $state<number | null>(null);
  let wasActive = false;

  async function loadProgress() {
    try {
      const progressData = await getLearningProgress();
      progressMap = progressData.progress;
    } catch (e) {
      error = e instanceof Error ? e.message : "获取学习进度失败";
    }
  }

  async function loadEntries(append = false) {
    const serial = ++requestSerial;
    if (append) {
      if (!nextCursor) return;
      isLoadingMore = true;
    } else {
      isLoading = true;
      error = "";
      nextCursor = null;
    }

    try {
      const page = await getLibraryEntriesPage({
        q: filterText,
        tab: activeTab,
        cursor: append ? nextCursor : null,
        limit: PAGE_SIZE,
      });
      if (serial !== requestSerial) return;

      applyPage(page, append);
    } catch (e) {
      if (serial !== requestSerial) return;
      error = e instanceof Error ? e.message : "获取词库数据失败";
      if (!append) entries = [];
    } finally {
      if (serial !== requestSerial) return;
      isLoading = false;
      isLoadingMore = false;
    }
  }

  function applyPage(page: LibraryEntriesPageResponse, append: boolean) {
    totalCount = page.total;
    tabCounts = page.counts;
    nextCursor = page.next_cursor ?? null;

    if (!append) {
      entries = page.items;
      return;
    }

    const seen = new Set(entries.map((entry) => entry.entry_id));
    entries = [
      ...entries,
      ...page.items.filter((entry) => !seen.has(entry.entry_id)),
    ];
  }

  async function handleLoadMore() {
    if (!nextCursor || isLoadingMore) return;
    await loadEntries(true);
  }

  function handleTabChange(tab: LibraryTab) {
    if (activeTab === tab) return;
    activeTab = tab;
    void loadEntries(false);
  }

  async function handleQuickAdd(id: number) {
    if (addingEntryId !== null) return;
    addingEntryId = id;
    try {
      const newProgress = await addWordToLearning(id);
      progressMap = { ...progressMap, [id]: newProgress };
      await loadEntries(false);
    } catch (e) {
      alert("加入学习失败");
    } finally {
      addingEntryId = null;
    }
  }

  async function handleViewDetail(id: number) {
    isFetchingDetail = true;
    try {
      selectedEntryDetail = await getEntryDetail(id);
    } catch (e) {
      alert("加载详情失败");
    } finally {
      isFetchingDetail = false;
    }
  }

  async function handleDelete(id: number) {
    if (!confirm("确定要删除吗？")) return;
    isDeleting = true;
    try {
      await deleteEntry(id);
      selectedEntryDetail = null;
      entries = entries.filter(e => e.entry_id !== id);
      delete progressMap[id];
      await loadEntries(false);
    } catch (e) {
      alert("删除失败");
    } finally {
      isDeleting = false;
    }
  }

  function handleViewInSearch(query: string) {
    searchStore.reset();
    searchStore.setQuery(query);
    void goto(`/?q=${encodeURIComponent(query)}`);
  }

  $effect(() => {
    if (!active) {
      wasActive = false;
      return;
    }

    const firstActivation = !wasActive;
    wasActive = true;
    if (firstActivation) {
      void loadProgress();
    }
    const timer = setTimeout(() => {
      void loadEntries(false);
    }, filterText.trim() ? 180 : 0);

    return () => clearTimeout(timer);
  });
</script>

<header class="page-header">
  <div class="header-main">
    <h1>知识词库</h1>
  </div>

</header>

<div class="library-controls">
  <div class="header-toolbar">
    <div class="search-bar surface-card">
      <i class="ph ph-magnifying-glass"></i>
      <input bind:value={filterText} placeholder="搜索词条..." />
    </div>
  </div>

  <div class="tabs-row">
    <button class="tab-btn" class:active={activeTab === 'all'} onclick={() => handleTabChange('all')}>
      全部 <span class="count">{tabCounts?.all ?? '—'}</span>
    </button>
    <button class="tab-btn" class:active={activeTab === 'learning'} onclick={() => handleTabChange('learning')}>
      学习中 <span class="count">{tabCounts?.learning ?? '—'}</span>
    </button>
    <button class="tab-btn" class:active={activeTab === 'review'} onclick={() => handleTabChange('review')}>
      待复习 <span class="count">{tabCounts?.review ?? '—'}</span>
    </button>
    <button class="tab-btn" class:active={activeTab === 'new'} onclick={() => handleTabChange('new')}>
      新词 <span class="count">{tabCounts?.new ?? '—'}</span>
    </button>
  </div>
</div>

<div class="library-container">
  {#if isLoading && entries.length === 0}
    <div class="library-grid">
      {#each Array(6) as _}
        <div class="skeleton-card bento-card" style="height: 180px;">
          <div class="skeleton-block" style="width: 40%;"></div>
          <div class="skeleton-block" style="margin-top: 1rem;"></div>
        </div>
      {/each}
    </div>
  {:else if error}
    <div class="message-surface error">{error}</div>
  {:else if entries.length === 0}
    <div class="empty-state surface-card">
      <i class="ph ph-folder-open"></i>
      <p>这里空空如也</p>
    </div>
  {:else}
    <div class="library-grid">
      {#each entries as entry (entry.entry_id)}
        <LibraryEntryCard 
          {entry} 
          progress={progressMap[entry.entry_id]}
          isAdding={addingEntryId === entry.entry_id}
          onViewDetail={handleViewDetail}
          onQuickAdd={handleQuickAdd}
        />
      {/each}
    </div>
    <div class="results-footer">
      <span class="results-meta">已加载 {entries.length} / {totalCount}</span>
      {#if nextCursor}
        <button class="btn-secondary load-more-btn" onclick={() => void handleLoadMore()} disabled={isLoadingMore}>
          {#if isLoadingMore}
            加载中...
          {:else}
            加载更多
          {/if}
        </button>
      {/if}
    </div>
  {/if}
</div>

<LibraryEntryDetailPanel 
  entry={selectedEntryDetail} 
  learningProgress={selectedEntryDetail ? progressMap[selectedEntryDetail.entry_id] ?? null : null}
  isAddingToLearning={selectedEntryDetail ? addingEntryId === selectedEntryDetail.entry_id : false}
  isDeleting={isDeleting}
  onClose={() => selectedEntryDetail = null}
  onDelete={handleDelete}
  onAddToLearning={handleQuickAdd}
  onViewInSearch={handleViewInSearch}
/>

{#if isFetchingDetail}
  <div class="global-loading-overlay">
    <div class="spinner"></div>
  </div>
{/if}

<style>
  .page-header { margin-bottom: 1rem; }
  .header-main h1 { font-size: 2.25rem; font-weight: 800; margin-bottom: 0.5rem; }

  .library-controls { position: sticky; top: -2rem; z-index: 80; display: flex; align-items: center; justify-content: space-between; gap: 1rem; width: calc(100vw - var(--desktop-sidebar-width)); height: var(--desktop-top-chrome-height); margin-left: calc((100% - (100vw - var(--desktop-sidebar-width))) / 2); margin-bottom: 2rem; padding: 0 max(1rem, calc((100vw - var(--desktop-sidebar-width) - var(--page-content-width)) / 2 + 1rem)); border-bottom: 1px solid var(--border-color); background: var(--bg-color); }
  .header-toolbar { display: flex; flex: 0 0 auto; gap: 0.75rem; align-items: center; order: 2; }
  .search-bar { display: flex; align-items: center; gap: 0.75rem; padding: 0.6rem 1.25rem; border-radius: var(--radius-full); width: 280px; background: var(--control-bg); }
  .search-bar input { background: transparent; width: 100%; color: var(--text-main); }
  .search-bar i { color: var(--text-muted); font-size: 1.1rem; }

  .tabs-row { display: flex; flex: 1; gap: 0.5rem; min-width: 0; overflow-x: auto; scrollbar-width: none; }
  .tabs-row::-webkit-scrollbar { display: none; }
  .tab-btn { padding: 0.6rem 1.25rem; border-radius: var(--radius-md); background: transparent; color: var(--text-muted); font-weight: 700; white-space: nowrap; display: flex; align-items: center; gap: 0.5rem; }
  .tab-btn.active { background: var(--btn-secondary); color: var(--accent-main); }
  .tab-btn .count { font-size: 0.75rem; opacity: 0.6; background: var(--border-color); padding: 0.1rem 0.4rem; border-radius: 4px; }

  .library-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 1.5rem; }
  .results-footer { display: flex; justify-content: space-between; align-items: center; gap: 1rem; margin-top: 1.75rem; }
  .results-meta { color: var(--text-muted); font-size: 0.92rem; }
  .load-more-btn { min-width: 8rem; }

  .empty-state { padding: 5rem 2rem; text-align: center; color: var(--text-muted); display: flex; flex-direction: column; align-items: center; gap: 1rem; }
  .empty-state i { font-size: 3rem; opacity: 0.4; }
  @keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }

  .global-loading-overlay { position: fixed; inset: 0; background: color-mix(in srgb, var(--card-bg) 50%, transparent); backdrop-filter: blur(2px); z-index: 2000; display: flex; align-items: center; justify-content: center; }
  .spinner { width: 2.5rem; height: 2.5rem; border: 3px solid var(--surface-passive); border-top-color: var(--accent-main); border-radius: 50%; animation: spin 1s linear infinite; }

  @media (min-width: 769px) and (max-width: 1200px) {
    .library-controls { padding-right: 7.25rem; }
    .search-bar { width: 220px; }
  }

  @media (max-width: 768px) {
    .library-controls { display: contents; }
    .header-toolbar { width: 100%; margin-bottom: 1rem; }
    .search-bar { flex: 1; }
    .tabs-row { position: sticky; top: -1.5rem; z-index: 80; order: 2; display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 0.25rem; width: calc(100% + 2rem); margin: 0 -1rem 2rem; padding: 1.5rem 1rem 0.75rem; overflow-x: hidden; border-bottom: 1px solid var(--border-color); background: var(--bg-color); }
    .tab-btn { flex-direction: column; justify-content: flex-start; gap: 2px; min-height: 44px; padding: 0.4rem; }
    .tab-btn .count { padding: 0; background: transparent; font-size: 11px; line-height: 14px; }
    .results-footer { flex-direction: column; align-items: stretch; }
  }

</style>
