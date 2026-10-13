<script lang="ts">
  import type { RecentItem, LearningProgressView } from "$lib/types";

  let {
    entry,
    progress,
    isAdding = false,
    onViewDetail,
    onQuickAdd
  }: {
    entry: RecentItem;
    progress?: LearningProgressView;
    isAdding?: boolean;
    onViewDetail: (id: number) => void;
    onQuickAdd: (id: number) => void;
  } = $props();

  function getStatusInfo(p?: LearningProgressView) {
    if (!p) return { label: "新词", class: "new" };
    const next = new Date(p.next_review_at);
    if (next <= new Date()) return { label: "待复习", class: "review" };
    return { label: "学习中", class: "learning" };
  }

  function formatNextReview(dateStr: string) {
    const d = new Date(dateStr);
    return d.toLocaleDateString("zh-CN", { month: "short", day: "numeric" });
  }

  const status = $derived(getStatusInfo(progress));
</script>

<article class="entry-card bento-card">
  <div class="card-top">
    <h3 class="word-text">{entry.query_text}</h3>
    <div class="status-badge {status.class}">
      {status.label}
    </div>
    <p class="word-preview">{entry.preview}</p>
  </div>

  <div class="card-bottom">
    <div class="meta-info">
      {#if progress}
        <span class="review-date">
          <i class="ph ph-calendar"></i> {formatNextReview(progress.next_review_at)}
        </span>
        <span class="stat-pill">稳定性 {progress.stability.toFixed(1)}</span>
      {:else}
        <span class="muted-copy">未加入学习计划</span>
      {/if}
    </div>
    
    <div class="card-actions">
      {#if !progress}
        <button
          class="btn-icon-text"
          onclick={() => onQuickAdd(entry.entry_id)}
          title={isAdding ? "正在加入学习" : "加入学习"}
          aria-label={isAdding ? "正在加入学习" : "加入学习"}
          aria-busy={isAdding}
          disabled={isAdding}
        >
          <i class={isAdding ? "ph ph-spinner" : "ph-fill ph-plus-circle"}></i>
        </button>
      {/if}
      <button class="btn-secondary small-btn" onclick={() => onViewDetail(entry.entry_id)}>
        详情 <i class="ph ph-caret-right"></i>
      </button>
    </div>
  </div>
</article>

<style>
  .entry-card { display: flex; flex-direction: column; justify-content: space-between; height: 100%; gap: 1.5rem; padding: 1.5rem; }
  
  .card-top { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: start; column-gap: 1rem; row-gap: 0.4rem; }
  .word-text { min-width: 0; overflow: hidden; overflow-wrap: anywhere; color: var(--text-main); font-size: 1.4rem; font-weight: 800; display: -webkit-box; line-clamp: 2; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
  .word-preview { grid-column: 1 / -1; overflow: hidden; color: var(--text-muted); font-size: 0.92rem; line-height: 1.5; display: -webkit-box; line-clamp: 2; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }

  .status-badge { white-space: nowrap; padding: 0.25rem 0.6rem; border-radius: 6px; font-size: 0.75rem; font-weight: 800; text-transform: uppercase; }
  .status-badge.new { background: var(--surface-passive); color: var(--text-muted); }
  .status-badge.learning { background: var(--success-bg); color: var(--success-text); }
  .status-badge.review { background: var(--warning-bg); color: var(--warning-text); }

  .card-bottom { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: 1rem; padding-top: 1rem; border-top: 1px solid var(--border-color); }
  
  .meta-info { display: flex; min-width: 0; align-items: center; gap: 0.75rem; font-size: 0.8rem; font-weight: 600; color: var(--text-muted); }
  .review-date { display: flex; align-items: center; gap: 0.3rem; white-space: nowrap; }
  .stat-pill { white-space: nowrap; background: var(--bg-color); padding: 0.1rem 0.4rem; border-radius: 4px; }

  .card-actions { display: flex; align-items: center; gap: 0.5rem; }
  .btn-icon-text { background: transparent; color: var(--accent-main); font-size: 1.5rem; display: flex; align-items: center; }
  .btn-icon-text:hover { transform: scale(1.1); }
  .small-btn { white-space: nowrap; padding: 0.4rem 0.8rem; font-size: 0.85rem; border-radius: var(--radius-sm); }

  @media (max-width: 600px) {
    .entry-card { padding: 14px; gap: 10px; }
    .card-top { column-gap: 8px; row-gap: 4px; }
    .word-text { font-size: 18px; line-height: 1.3; }
    .word-preview { font-size: 13px; line-height: 1.5; }
    .status-badge { font-size: 10px; padding: 3px 6px; }
    .card-bottom { gap: 8px; padding-top: 8px; }
    .meta-info { gap: 6px; font-size: 11px; }
    .small-btn { font-size: 12px; padding: 6px 9px; }
  }
</style>
