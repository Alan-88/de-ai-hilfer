<script lang="ts">
  import type { GrammarRow } from "$lib/analysis/structuredAnalysis";
  import { slide } from "svelte/transition";

  let {
    grammarRows = [],
    isStreaming = false,
    initialVisibleCount = 4
  }: {
    grammarRows: GrammarRow[];
    isStreaming?: boolean;
    initialVisibleCount?: number;
  } = $props();

  let isExpanded = $state(false);

  // 德语语法高价值字段排序优先级
  const priorityKeys = [
    "词性", "性别", "复数", "第二格",
    "及物性", "及物", "助动词",
    "过去式", "过去分词", "现在分词",
    "支配", "变格", "比较级", "最高级"
  ];

  const sortedRows = $derived([...grammarRows].sort((a, b) => {
    const idxA = priorityKeys.findIndex(pk => a.key.includes(pk));
    const idxB = priorityKeys.findIndex(pk => b.key.includes(pk));

    if (idxA !== -1 && idxB !== -1) return idxA - idxB;
    if (idxA !== -1) return -1;
    if (idxB !== -1) return 1;
    return 0;
  }));

  const visibleRows = $derived(isExpanded ? sortedRows : sortedRows.slice(0, initialVisibleCount));
  const hasMore = $derived(sortedRows.length > initialVisibleCount);

  function toggleExpand() {
    isExpanded = !isExpanded;
  }
</script>

<div class="grammar-card-container">
  {#if sortedRows.length > 0}
    <div class="grammar-grid">
      {#each visibleRows as row (row.key)}
        <div class="g-item" transition:slide={{ duration: 200 }}>
          <span class="g-key">{row.key}</span>
          <span class="g-val">{row.value}</span>
        </div>
      {/each}
    </div>

    {#if hasMore}
      <button
        class="expand-btn"
        onclick={toggleExpand}
        aria-expanded={isExpanded}
      >
        <span>{isExpanded ? "收起部分语法" : `展开更多 (${sortedRows.length - initialVisibleCount}+)`}</span>
        <i class="ph" class:ph-caret-up={isExpanded} class:ph-caret-down={!isExpanded}></i>
      </button>
    {/if}
  {:else}
    <div class="empty-state">
      <p>{isStreaming ? "正在解析语法特征..." : "未发现显著语法特征"}</p>
    </div>
  {/if}
</div>

<style>
  .grammar-card-container {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    width: 100%;
  }

  .grammar-grid {
    display: flex;
    flex-direction: column;
  }

  .g-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.65rem 0;
    border-bottom: 1px solid var(--border-color);
    gap: 1rem;
  }

  .g-item:last-child {
    border-bottom: none;
  }

  .g-key {
    font-size: 0.85rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .g-val {
    font-size: 0.95rem;
    font-weight: 700;
    color: var(--accent-main);
    text-align: right;
    word-break: break-all;
  }

  .expand-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    width: 100%;
    padding: 0.5rem;
    margin-top: 0.25rem;
    background: var(--bg-color);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    font-size: 0.8rem;
    font-weight: 600;
    transition: all 0.2s ease;
  }

  .expand-btn:hover {
    background: var(--btn-secondary);
    color: var(--text-main);
    border-color: var(--accent-main);
  }

  .empty-state {
    padding: 1.5rem 0;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.85rem;
    font-style: italic;
  }

  :global(.canvas-dark) .g-val {
    color: #93c5fd; /* 浅蓝色，在深色模式下更亮 */
  }
</style>
