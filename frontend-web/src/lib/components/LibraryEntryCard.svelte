<script lang="ts">
  import type { RecentItem, LearningProgressView } from "$lib/types";

  let {
    entry,
    progress,
    onViewDetail,
    onQuickAdd
  }: {
    entry: RecentItem;
    progress?: LearningProgressView;
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
    <div class="word-info">
      <h3 class="word-text">{entry.query_text}</h3>
      <p class="word-preview">{entry.preview}</p>
    </div>
    <div class="status-badge {status.class}">
      {status.label}
    </div>
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
        <button class="btn-icon-text" onclick={() => onQuickAdd(entry.entry_id)} title="加入学习">
          <i class="ph-fill ph-plus-circle"></i>
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
  
  .card-top { display: flex; justify-content: space-between; align-items: flex-start; gap: 1rem; }
  .word-text { font-size: 1.4rem; font-weight: 800; color: var(--text-main); margin-bottom: 0.4rem; }
  .word-preview { font-size: 0.92rem; color: var(--text-muted); line-height: 1.5; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }

  .status-badge { padding: 0.25rem 0.6rem; border-radius: 6px; font-size: 0.75rem; font-weight: 800; text-transform: uppercase; }
  .status-badge.new { background: var(--btn-secondary); color: var(--text-muted); }
  .status-badge.learning { background: var(--success-bg); color: var(--success-text); }
  .status-badge.review { background: var(--warning-bg); color: var(--warning-text); }

  .card-bottom { display: flex; justify-content: space-between; align-items: center; gap: 1rem; padding-top: 1rem; border-top: 1px solid var(--border-color); }
  
  .meta-info { display: flex; align-items: center; gap: 0.75rem; font-size: 0.8rem; font-weight: 600; color: var(--text-muted); }
  .review-date { display: flex; align-items: center; gap: 0.3rem; }
  .stat-pill { background: var(--bg-color); padding: 0.1rem 0.4rem; border-radius: 4px; }

  .card-actions { display: flex; align-items: center; gap: 0.5rem; }
  .btn-icon-text { background: transparent; color: var(--accent-main); font-size: 1.5rem; display: flex; align-items: center; }
  .btn-icon-text:hover { transform: scale(1.1); }
  .small-btn { padding: 0.4rem 0.8rem; font-size: 0.85rem; border-radius: var(--radius-sm); }
</style>
