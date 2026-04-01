<script lang="ts">
  import { onMount } from "svelte";
  import LearningFlashcard from "$lib/components/bento/LearningFlashcard.svelte";
  import { getLearningSession, getLearningStats, submitReview } from "$lib/learningApi";
  import type {
    LearningSessionResponse,
    LearningSessionWord,
    LearningStatsResponse,
    ReviewQuality,
  } from "$lib/types";

  let { active = false } = $props();

  let sessionState = $state<LearningSessionResponse | null>(null);
  let stats = $state<LearningStatsResponse | null>(null);
  let isLoading = $state(false);
  let hasLoaded = $state(false);
  let isSubmitting = $state(false);
  let error = $state("");
  let maxSessionSize = $state(0);

  async function loadNextWord() {
    isLoading = true;
    error = "";

    try {
      const [nextSession, nextStats] = await Promise.all([getLearningSession(), getLearningStats()]);
      sessionState = nextSession;
      stats = nextStats;
      if (nextSession.total_count > maxSessionSize) {
        maxSessionSize = nextSession.total_count;
      }
      hasLoaded = true;
    } catch (loadError) {
      error = loadError instanceof Error ? loadError.message : "获取学习会话失败";
    } finally {
      isLoading = false;
    }
  }

  async function handleReviewed(quality: ReviewQuality) {
    const currentWord = sessionState?.current_word;
    if (!currentWord || isSubmitting) return;
    isSubmitting = true;

    try {
      await submitReview(currentWord.entry_id, quality);
      await loadNextWord();
    } catch (reviewError) {
      error = reviewError instanceof Error ? reviewError.message : "提交复习失败";
    } finally {
      isSubmitting = false;
    }
  }

  onMount(() => {
    if (active) void loadNextWord();
  });

  // 这里的派生状态
  const reviewedCount = $derived(maxSessionSize > 0 ? Math.max(maxSessionSize - (sessionState?.total_count ?? 0), 0) : 0);
  const sessionProgress = $derived(maxSessionSize > 0 ? (reviewedCount / maxSessionSize) * 100 : (sessionState?.is_completed ? 100 : 0));
</script>

<header class="page-header">
  <h1>沉浸复习</h1>
  <p>先回忆，再揭晓答案。把节奏压回到主动提取，而不是被动浏览。</p>
</header>

<div class="stats-grid">
  <div class="bento-card stat-card">
    <div class="label">当前轮次</div>
    <div class="num">{maxSessionSize > 0 ? `${reviewedCount}/${maxSessionSize}` : "0/0"}</div>
  </div>
  <div class="bento-card stat-card">
    <div class="label">学习中</div>
    <div class="num">{stats?.total_words ?? 0}</div>
  </div>
  <div class="bento-card stat-card">
    <div class="label">今日待复习</div>
    <div class="num">{stats?.due_today ?? 0}</div>
  </div>
  <div class="bento-card stat-card">
    <div class="label">平均稳定性</div>
    <div class="num">{(stats?.average_stability ?? 0).toFixed(2)}</div>
  </div>
</div>

<div class="flashcard-container">
  <div class="progress-bar-wrapper">
    <div class="progress-fill" style="width: {sessionProgress}%"></div>
  </div>

  {#if isLoading && !hasLoaded}
    <div class="bento-card loading-card">
      <div class="skeleton-block" style="width: 40%; height: 2rem;"></div>
      <div class="skeleton-block" style="margin-top: 2rem; height: 10rem;"></div>
    </div>
  {:else if error}
    <div class="message-surface error">
      <p>{error}</p>
      <button class="btn-secondary" onclick={loadNextWord} style="margin-top: 1rem;">
        <i class="ph ph-arrows-clockwise"></i> 重试
      </button>
    </div>
  {:else if sessionState?.is_completed}
    <div class="surface-card empty-state">
      <i class="ph-fill ph-check-circle"></i>
      <p>今天的内容已全部学完！</p>
      <p class="small-copy">你可以去词库继续添加新词，或稍后再来。</p>
    </div>
  {:else if sessionState?.current_word}
    <LearningFlashcard 
      wordData={sessionState.current_word} 
      isSubmitting={isSubmitting} 
      onRate={handleReviewed} 
    />
  {/if}
</div>

<style>
  .page-header { width: 100%; margin-bottom: 2.5rem; }
  .page-header h1 { font-size: 2.25rem; font-weight: 800; margin-bottom: 0.5rem; }
  .page-header p { color: var(--text-muted); font-size: 1.1rem; }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1.25rem;
    width: 100%;
    margin-bottom: 2.5rem;
  }

  .stat-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 1.5rem;
    text-align: center;
  }

  .stat-card .label { font-size: 0.85rem; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
  .stat-card .num { font-size: 2rem; font-weight: 800; color: var(--accent-main); margin-top: 0.5rem; }

  .flashcard-container {
    width: 100%;
    max-width: 640px;
    margin: 0 auto;
  }

  .progress-bar-wrapper {
    width: 100%;
    height: 8px;
    background: var(--btn-secondary);
    border-radius: var(--radius-full);
    margin-bottom: 2rem;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent-main);
    transition: width 0.5s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .empty-state {
    text-align: center;
    padding: 4rem 2rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .empty-state i { font-size: 4rem; color: var(--success-text); }
  .empty-state p { font-size: 1.25rem; font-weight: 600; }

  @media (max-width: 768px) {
    .stats-grid { grid-template-columns: repeat(2, 1fr); }
  }
</style>
