<script lang="ts">
  import { tick } from "svelte";
  import { resolveStructuredAnalysis } from "$lib/analysis/structuredAnalysis";
  import GrammarBranchPopover from "$lib/components/bento/GrammarBranchPopover.svelte";
  import LearningAnalysisContent from "$lib/components/bento/LearningAnalysisContent.svelte";
  import FollowUpHistory from "$lib/components/bento/FollowUpHistory.svelte";
  import { getFollowUps } from "$lib/followUpApi";
  import type { FollowUpItem, GrammarBranch, LearningRecallRating, LearningSessionWord } from "$lib/types";

  let { wordData, isSubmitting = false, onRate } = $props<{
    wordData: LearningSessionWord;
    isSubmitting: boolean;
    onRate: (rating: LearningRecallRating) => void;
  }>();

  let showingAnswer = $state(false);
  let contentElement = $state<HTMLDivElement | null>(null);
  let usageElement = $state<HTMLElement | null>(null);
  let activeGrammarBranch = $state<GrammarBranch | null>(null);
  let popoverTriggerRect = $state<DOMRect | null>(null);
  let followUps = $state<FollowUpItem[]>([]);
  let followUpLoading = $state(false);
  let followUpError = $state("");

  const structured = $derived(resolveStructuredAnalysis(
    wordData.analysis_markdown,
    wordData.structured_analysis,
    wordData.query_text,
  ));
  const headword = $derived(structured.headword || wordData.query_text);
  const ratingOptions = $derived([
    {
      label: wordData.phase === "new" ? "不认识" : "忘记",
      value: "forgotten",
      className: "forgotten",
    },
    { label: "模糊", value: "fuzzy", className: "fuzzy" },
    { label: "认识", value: "known", className: "known" },
  ] satisfies Array<{ label: string; value: LearningRecallRating; className: string }>);

  $effect(() => {
    wordData.entry_id;
    showingAnswer = false;
    activeGrammarBranch = null;
    usageElement = null;
  });

  $effect(() => {
    const entryId = wordData.entry_id;
    followUps = []; followUpError = ""; followUpLoading = false;
    if (!showingAnswer || entryId <= 0) return;
    const controller = new AbortController();
    followUpLoading = true;
    void getFollowUps(entryId, controller.signal)
      .then(response => { if (!controller.signal.aborted) followUps = response.follow_ups; })
      .catch(cause => { if (!controller.signal.aborted) followUpError = cause instanceof Error ? cause.message : "历史追问加载失败"; })
      .finally(() => { if (!controller.signal.aborted) followUpLoading = false; });
    return () => controller.abort();
  });

  function learningStateLabel() {
    if (wordData.phase === "new") return "新词";
    if (wordData.phase === "review") return "复习";
    if (wordData.phase === "intraday") return "今日重现";
    return "学习";
  }

  function formatDate(value?: string | null) {
    if (!value) return "首次出现";
    return new Intl.DateTimeFormat("zh-CN", { month: "numeric", day: "numeric" }).format(new Date(value));
  }

  async function revealAnswer() {
    showingAnswer = true;
    await tick();
    requestAnimationFrame(() => {
      if (!contentElement) return;
      const top = usageElement
        ? usageElement.getBoundingClientRect().top
          - contentElement.getBoundingClientRect().top
          + contentElement.scrollTop
        : 0;
      contentElement.scrollTo({ top, behavior: "auto" });
    });
  }

  function openGrammar(branch: GrammarBranch, triggerRect: DOMRect) {
    activeGrammarBranch = branch;
    popoverTriggerRect = triggerRect;
  }
</script>

<section class="flashcard-shell" aria-label={`${headword} 学习卡片`}>
  <div class="bento-card flashcard">
    {#if !showingAnswer}
      <button class="recall-surface" aria-label={`查看 ${headword} 的答案`} onclick={revealAnswer}>
        <div class="meta"><span>{learningStateLabel()} · 第 {wordData.appearance_count_today ?? 1} 次</span><span>上次：{formatDate(wordData.progress?.last_reviewed_at)}</span></div>
        <div class="recall-content">
          <h2>{headword}</h2>
          <div class="recall-prompt"><strong>回忆词义与用法</strong><span>点击卡片查看答案</span></div>
        </div>
      </button>
    {:else}
      <div class="answer-shell">
        <h2 class="answer-title">{headword}</h2>
        <div class="meta answer-meta">
          <span>{learningStateLabel()} · 第 {wordData.appearance_count_today ?? 1} 次</span>
          <span>上次：{formatDate(wordData.progress?.last_reviewed_at)}</span>
        </div>
        <div class="scroll-content" bind:this={contentElement}>
          <LearningAnalysisContent
            analysis={structured}
            attachedPhraseModules={wordData.attached_phrase_modules ?? []}
            onGrammarOpen={openGrammar}
            bind:usageElement
            interleavedMeanings
          />
          {#if followUps.length || followUpLoading || followUpError}
            <section class="bento-card follow-up-section">
              <div class="card-title"><i class="ph-fill ph-chat-circle-dots"></i> 追问与联想</div>
              {#if followUpLoading}<p class="small-copy" role="status">正在加载历史追问…</p>{/if}
              {#if followUpError}<p class="message-surface error" role="alert">{followUpError}</p>{/if}
              <FollowUpHistory entryId={wordData.entry_id} history={followUps} readOnly />
            </section>
          {/if}
        </div>
      </div>
    {/if}
  </div>

  {#if showingAnswer}
    <div class="rating-bar">
      {#each ratingOptions as option}
        <button
          class={option.className}
          disabled={isSubmitting}
          onclick={() => onRate(option.value)}
        >{option.label}</button>
      {/each}
    </div>
  {/if}
</section>

{#if activeGrammarBranch}
  <GrammarBranchPopover
    branch={activeGrammarBranch}
    triggerRect={popoverTriggerRect}
    onClose={() => {
      activeGrammarBranch = null;
      popoverTriggerRect = null;
    }}
  />
{/if}

<style>
  .follow-up-section { padding: 1.2rem; margin-bottom: 2rem; }
  .follow-up-section:hover { transform: none; box-shadow: var(--shadow-sm); }
  .flashcard-shell { width: 100%; min-height: 0; }
  .flashcard { height: clamp(540px, calc(100vh - 230px), 680px); overflow: hidden; padding: 0; }
  .recall-surface { width: 100%; height: 100%; padding: clamp(1.5rem, 3vw, 2.5rem); border-radius: inherit; background: transparent; color: var(--text-main); text-align: inherit; }
  .meta { display: flex; align-items: center; justify-content: space-between; gap: 1rem; color: var(--text-muted); font-size: 0.8rem; font-weight: 700; }
  .recall-content { height: calc(100% - 1rem); display: flex; flex-direction: column; align-items: center; justify-content: center; gap: clamp(4.5rem, 11vh, 7rem); text-align: center; }
  h2 { font-size: clamp(3rem, 5vw, 4.4rem); font-weight: 800; line-height: 1.1; letter-spacing: 0; }
  .recall-prompt { display: grid; gap: 0.45rem; color: var(--text-muted); }
  .recall-prompt strong { color: var(--text-main); font-size: 1.08rem; }
  .recall-prompt span { font-size: 0.9rem; }

  .answer-shell { height: 100%; display: flex; flex-direction: column; padding: 1.5rem clamp(1.5rem, 3vw, 2.5rem) 16px; }
  .answer-title { flex: 0 0 auto; font-size: 40px; line-height: 1.15; text-align: center; overflow-wrap: anywhere; margin-bottom: 12px; }
  .answer-meta { flex: 0 0 auto; padding-bottom: 12px; }
  .scroll-content { flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain; scrollbar-gutter: stable; }

  .rating-bar { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 0.75rem; margin-top: 1rem; }
  .rating-bar button { min-height: 52px; border-radius: var(--radius-md); background: var(--btn-secondary); color: var(--btn-secondary-text); font-weight: 800; }
  .rating-bar button:hover:not(:disabled) { transform: translateY(-2px); box-shadow: var(--shadow-hover); }
  .rating-bar button:disabled { cursor: wait; opacity: 0.55; }
  .rating-bar .forgotten { background: var(--danger-bg); color: var(--danger-text); }
  .rating-bar .known { background: var(--success-bg); color: var(--success-text); }

  @media (max-width: 768px) {
    .flashcard-shell { position: relative; }
    .flashcard { height: calc(100dvh - 208px - env(safe-area-inset-bottom)); min-height: 0; }
    .flashcard:hover { transform: none; box-shadow: var(--shadow-sm); }
    .recall-surface { padding: 1.25rem; }
    .answer-shell { padding: 1.1rem 1rem 5.25rem; }
    .meta { gap: 0.5rem; font-size: 0.72rem; }
    .answer-title { font-size: 32px; }
    .rating-bar { position: absolute; z-index: 4; right: 0; bottom: 0; left: 0; gap: 0.5rem; margin: 0; padding: 1rem; }
  }
  @media (max-width: 600px) {
    .answer-title { font-size: 26px; margin-bottom: 8px; }
    .follow-up-section { padding: 10px; border-radius: 16px; }
    .follow-up-section .card-title { font-size: 12px; margin-bottom: 8px; }
  }
</style>
