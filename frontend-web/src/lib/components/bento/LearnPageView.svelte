<script lang="ts">
  import { onMount } from "svelte";
  import LearningFlashcard from "$lib/components/bento/LearningFlashcard.svelte";
  import { startLearningSession, submitLearningReviewV3 } from "$lib/learningApi";
  import type { LearningRecallRating, LearningSessionV3Response } from "$lib/types";

  let { active = false } = $props();

  let sessionState = $state<LearningSessionV3Response | null>(null);
  let isLoading = $state(false);
  let hasLoaded = $state(false);
  let isSubmitting = $state(false);
  let error = $state("");
  let maxSessionSize = $state(0);

  async function loadSession() {
    isLoading = true;
    error = "";

    try {
      const nextSession = await startLearningSession();
      sessionState = nextSession;
      maxSessionSize = Math.max(maxSessionSize, nextSession.total_count);
      hasLoaded = true;
    } catch (loadError) {
      error = loadError instanceof Error ? loadError.message : "获取学习会话失败";
    } finally {
      isLoading = false;
    }
  }

  async function handleReviewed(rating: LearningRecallRating) {
    const currentWord = sessionState?.current_word;
    const currentSessionId = sessionState?.session_id;
    if (!currentWord || !currentSessionId || isSubmitting) return;
    isSubmitting = true;
    error = "";

    try {
      sessionState = await submitLearningReviewV3(currentSessionId, currentWord.entry_id, rating);
    } catch (reviewError) {
      error = reviewError instanceof Error ? reviewError.message : "提交复习失败";
    } finally {
      isSubmitting = false;
    }
  }

  onMount(() => {
    if (active) void loadSession();
  });

  const reviewedCount = $derived(sessionState?.completed_count ?? 0);
  const sessionProgress = $derived(maxSessionSize > 0
    ? (reviewedCount / maxSessionSize) * 100
    : sessionState?.is_completed ? 100 : 0);
</script>

<div class="learning-page">
  <header class="page-header"><h1>沉浸复习</h1></header>

  <main class="session-shell">
    <div class="progress-row">
      <div class="progress" aria-label="本轮学习进度"><span style:width={`${sessionProgress}%`}></span></div>
      <strong>{reviewedCount} / {maxSessionSize}</strong>
    </div>

    <div class="card-stage">
      {#if isLoading && !hasLoaded}
        <div class="bento-card loading-card">
          <div class="skeleton-block" style="width: 34%; height: 2.5rem;"></div>
          <div class="skeleton-block" style="width: 58%; height: 1rem;"></div>
        </div>
      {:else if error}
        <div class="message-surface error state-card">
          <p>{error}</p>
          <button class="btn-secondary" onclick={loadSession}><i class="ph ph-arrows-clockwise"></i> 重试</button>
        </div>
      {:else if sessionState?.is_completed}
        <div class="surface-card empty-state">
          <i class="ph-fill ph-check-circle"></i>
          <p>今天的内容已全部学完</p>
        </div>
      {:else if sessionState?.current_word}
        <LearningFlashcard
          wordData={sessionState.current_word}
          isSubmitting={isSubmitting}
          onRate={handleReviewed}
        />
      {/if}
    </div>
  </main>
</div>

<style>
  .learning-page { display: flex; flex-direction: column; width: 100%; height: calc(100dvh - 4rem); overflow: hidden; }
  .page-header { flex: 0 0 auto; margin-bottom: 1.2rem; }
  .page-header h1 { font-size: 1.75rem; font-weight: 800; }
  .session-shell { display: flex; flex: 1; flex-direction: column; min-height: 0; }
  .progress-row { display: flex; flex: 0 0 auto; align-items: center; gap: 1rem; width: min(940px, 100%); margin: 0 auto 1rem; }
  .progress { flex: 1; height: 8px; overflow: hidden; border-radius: var(--radius-full); background: var(--btn-secondary); }
  .progress span { display: block; height: 100%; background: var(--accent-main); transition: width 240ms ease; }
  .progress-row > strong { min-width: 52px; color: var(--text-muted); font-size: 0.88rem; font-variant-numeric: tabular-nums; text-align: right; }
  .card-stage { display: flex; flex: 1; min-height: 0; }
  .loading-card, .state-card, .empty-state { display: flex; flex: 1; flex-direction: column; align-items: center; justify-content: center; gap: 1rem; width: min(940px, 100%); margin: 0 auto; }
  .loading-card { min-height: 0; }
  .state-card button { margin-top: 0.5rem; }
  .empty-state { text-align: center; }
  .empty-state i { color: var(--success-text); font-size: 4rem; }
  .empty-state p { font-size: 1.2rem; font-weight: 700; }

  @media (max-width: 768px) {
    .learning-page { height: calc(100dvh - 7rem - env(safe-area-inset-bottom)); }
    .page-header { margin-bottom: 0.9rem; }
    .page-header h1 { font-size: 1.55rem; }
    .progress-row { margin-bottom: 0.75rem; }
  }
</style>
