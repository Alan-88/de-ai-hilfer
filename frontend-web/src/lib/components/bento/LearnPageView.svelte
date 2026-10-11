<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import LearningFlashcard from "$lib/components/bento/LearningFlashcard.svelte";
  import { getLearningSessionNext, startLearningSession, submitLearningReviewV3 } from "$lib/learningApi";
  import type { LearningRecallRating, LearningSessionV3Response } from "$lib/types";

  let { active = false } = $props();

  let sessionState = $state<LearningSessionV3Response | null>(null);
  let isLoading = $state(false);
  let hasLoaded = $state(false);
  let isSubmitting = $state(false);
  let error = $state("");
  let maxSessionSize = $state(0);
  let waitTimer: ReturnType<typeof setTimeout> | undefined;

  function clearWaitTimer() {
    if (waitTimer) {
      clearTimeout(waitTimer);
      waitTimer = undefined;
    }
  }

  function applySessionState(nextSession: LearningSessionV3Response) {
    sessionState = nextSession;
    maxSessionSize = Math.max(maxSessionSize, nextSession.total_count);
    hasLoaded = true;
    scheduleNextIfWaiting(nextSession);
  }

  function scheduleNextIfWaiting(state: LearningSessionV3Response) {
    clearWaitTimer();
    if (state.is_completed || state.current_word || !state.next_available_at) return;

    const waitMs = Math.max(250, new Date(state.next_available_at).getTime() - Date.now());
    waitTimer = setTimeout(() => {
      void loadNext(state.session_id);
    }, waitMs);
  }

  async function loadNext(sessionId: string) {
    try {
      const nextSession = await getLearningSessionNext(sessionId);
      if (sessionState?.session_id !== sessionId) return;
      applySessionState(nextSession);
    } catch (loadError) {
      error = loadError instanceof Error ? loadError.message : "获取下一张卡片失败";
    }
  }

  async function loadSession() {
    clearWaitTimer();
    isLoading = true;
    error = "";

    try {
      const nextSession = await startLearningSession();
      applySessionState(nextSession);
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
      applySessionState(await submitLearningReviewV3(currentSessionId, currentWord.entry_id, rating));
    } catch (reviewError) {
      error = reviewError instanceof Error ? reviewError.message : "提交复习失败";
    } finally {
      isSubmitting = false;
    }
  }

  onMount(() => {
    if (active) void loadSession();
  });

  onDestroy(clearWaitTimer);

  const reviewedCount = $derived(sessionState?.completed_count ?? 0);
  const sessionProgress = $derived(maxSessionSize > 0
    ? (reviewedCount / maxSessionSize) * 100
    : sessionState?.is_completed ? 100 : 0);
</script>

<div class="learning-page">
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
      {:else if sessionState && !sessionState.is_completed}
        <div class="surface-card waiting-state">
          <i class="ph ph-hourglass"></i>
          <p>下一张复现稍后出现</p>
        </div>
      {/if}
    </div>
  </main>
</div>

<style>
  .learning-page { width: min(1040px, calc(100vw - 304px)); margin-left: 50%; transform: translateX(-50%); }
  .session-shell { width: min(940px, 100%); margin: 0 auto; }
  .progress-row { display: flex; align-items: center; gap: 1rem; margin-bottom: 1rem; }
  .progress { flex: 1; height: 8px; overflow: hidden; border-radius: var(--radius-full); background: var(--surface-passive); }
  .progress span { display: block; height: 100%; background: var(--accent-main); transition: width 240ms ease; }
  .progress-row > strong { min-width: 52px; color: var(--text-muted); font-size: 0.88rem; font-variant-numeric: tabular-nums; text-align: right; }
  .card-stage { min-height: 0; }
  .loading-card, .state-card, .empty-state { display: flex; height: clamp(540px, calc(100vh - 230px), 680px); flex-direction: column; align-items: center; justify-content: center; gap: 1rem; }
  .state-card button { margin-top: 0.5rem; }
  .empty-state { text-align: center; }
  .empty-state i { color: var(--success-text); font-size: 4rem; }
  .empty-state p { font-size: 1.2rem; font-weight: 700; }
  .waiting-state { display: flex; height: clamp(540px, calc(100vh - 230px), 680px); flex-direction: column; align-items: center; justify-content: center; gap: 1rem; text-align: center; }
  .waiting-state i { color: var(--accent-main); font-size: 3.5rem; }
  .waiting-state p { color: var(--text-muted); font-size: 1.05rem; font-weight: 700; }

  @media (max-width: 768px) {
    .learning-page { width: 100%; margin-left: 0; transform: none; }
    .progress-row { margin-bottom: 0.75rem; }
  }
</style>
