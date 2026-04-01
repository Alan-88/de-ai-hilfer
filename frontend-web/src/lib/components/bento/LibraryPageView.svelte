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
  let allCount = $state(0);
  let nextCursor = $state<string | null>(null);
  let requestSerial = 0;
  
  // 过滤与 UI 状态
  let filterText = $state("");
  let activeTab = $state<LibraryTab>("all");
  
  // 详情侧滑窗
  let selectedEntryDetail = $state<EntryDetailResponse | null>(null);
  let isFetchingDetail = $state(false);
  let isDeleting = $state(false);
  let wasActive = $state(false);

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
    nextCursor = page.next_cursor ?? null;

    if (!filterText.trim() && activeTab === "all") {
      allCount = page.total;
    }

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

  async function handleQuickAdd(id: number) {
    try {
      const newProgress = await addWordToLearning(id);
      progressMap[id] = newProgress;
    } catch (e) {
      alert("加入学习失败");
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
      totalCount = Math.max(0, totalCount - 1);
      allCount = Math.max(0, allCount - 1);
      delete progressMap[id];
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
    const timer = setTimeout(() => {
      if (firstActivation) {
        void loadProgress();
      }
      void loadEntries(false);
    }, filterText.trim() ? 180 : 0);

    return () => clearTimeout(timer);
  });
</script>

<header class="page-header">
  <div class="header-main">
    <h1>知识词库</h1>
    <p>管理你已发现的德语宝库。当前视图共有 {totalCount} 个词条。</p>
  </div>
  
  <div class="header-toolbar">
    <div class="search-bar surface-card">
      <i class="ph ph-magnifying-glass"></i>
      <input bind:value={filterText} placeholder="搜索词条..." />
    </div>
  </div>
</header>

<div class="tabs-row">
  <button class="tab-btn" class:active={activeTab === 'all'} onclick={() => activeTab = 'all'}>
    全部 <span class="count">{allCount}</span>
  </button>
  <button class="tab-btn" class:active={activeTab === 'learning'} onclick={() => activeTab = 'learning'}>
    学习中 <span class="count">{Object.keys(progressMap).length}</span>
  </button>
  <button class="tab-btn" class:active={activeTab === 'review'} onclick={() => activeTab = 'review'}>
    待复习
  </button>
  <button class="tab-btn" class:active={activeTab === 'new'} onclick={() => activeTab = 'new'}>
    新词
  </button>
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
  isDeleting={isDeleting}
  onClose={() => selectedEntryDetail = null}
  onDelete={handleDelete}
  onViewInSearch={handleViewInSearch}
/>

{#if isFetchingDetail}
  <div class="global-loading-overlay">
    <div class="spinner"></div>
  </div>
{/if}

<style>
  .page-header { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 2rem; gap: 2rem; }
  .header-main h1 { font-size: 2.25rem; font-weight: 800; margin-bottom: 0.5rem; }
  .header-main p { color: var(--text-muted); }

  .header-toolbar { display: flex; gap: 0.75rem; align-items: center; }
  .search-bar { display: flex; align-items: center; gap: 0.75rem; padding: 0.6rem 1.25rem; border-radius: var(--radius-full); width: 280px; }
  .search-bar input { background: transparent; width: 100%; color: var(--text-main); }
  .search-bar i { color: var(--text-muted); font-size: 1.1rem; }

  .tabs-row { display: flex; gap: 0.5rem; margin-bottom: 2rem; padding-bottom: 1rem; border-bottom: 1px solid var(--border-color); overflow-x: auto; }
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

  .global-loading-overlay { position: fixed; inset: 0; background: rgba(255,255,255,0.5); backdrop-filter: blur(2px); z-index: 2000; display: flex; align-items: center; justify-content: center; }
  .spinner { width: 2.5rem; height: 2.5rem; border: 3px solid var(--btn-secondary); border-top-color: var(--accent-main); border-radius: 50%; animation: spin 1s linear infinite; }

  @media (max-width: 768px) {
    .page-header { flex-direction: column; align-items: flex-start; gap: 1.5rem; }
    .header-toolbar { width: 100%; }
    .search-bar { flex: 1; }
    .results-footer { flex-direction: column; align-items: stretch; }
  }
</style>
