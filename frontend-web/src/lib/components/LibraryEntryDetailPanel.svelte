<script lang="ts">
  import { fade, slide } from "svelte/transition";
  import {
    renderMarkdownHtml,
    resolveStructuredAnalysis,
  } from "$lib/analysis/structuredAnalysis";
  import GrammarBranchPopover from "$lib/components/bento/GrammarBranchPopover.svelte";
  import LearningAnalysisContent from "$lib/components/bento/LearningAnalysisContent.svelte";
  import type { EntryDetailResponse, GrammarBranch, LearningProgressView } from "$lib/types";

  let {
    entry = $bindable(),
    learningProgress = null,
    isAddingToLearning = false,
    isDeleting = false,
    onClose,
    onDelete,
    onAddToLearning,
    onViewInSearch,
  } = $props<{
    entry: EntryDetailResponse | null;
    learningProgress?: LearningProgressView | null;
    isAddingToLearning?: boolean;
    isDeleting?: boolean;
    onClose: () => void;
    onDelete: (id: number) => void;
    onAddToLearning?: (id: number) => void;
    onViewInSearch: (query: string) => void;
  }>();

  const structured = $derived(
    entry
      ? resolveStructuredAnalysis(
          entry.analysis_markdown,
          entry.structured_analysis,
          entry.query_text,
        )
      : null,
  );

  const learningStatus = $derived(
    learningProgress
      ? new Date(learningProgress.next_review_at) <= new Date()
        ? { label: "待复习", className: "review" }
        : { label: "学习中", className: "learning" }
      : null,
  );

  let activeGrammarBranch = $state<GrammarBranch | null>(null);
  let popoverTriggerRect = $state<DOMRect | null>(null);
  $effect(() => {
    entry?.entry_id;
    activeGrammarBranch = null;
    popoverTriggerRect = null;
  });

  function edgeSwipe(element: HTMLDivElement) {
    let start: { x: number; y: number } | null = null;
    const cancel = () => { start = null; };
    const handleStart = (event: TouchEvent) => {
      cancel();
      if (event.defaultPrevented || event.touches.length !== 1 || !matchMedia("(max-width: 600px)").matches) return;
      const touch = event.touches[0];
      const edgeOffset = touch.clientX - element.getBoundingClientRect().left;
      if (edgeOffset >= 0 && edgeOffset <= 20) start = { x: touch.clientX, y: touch.clientY };
    };
    const handleMove = (event: TouchEvent) => {
      if (!start) return;
      if (event.touches.length !== 1) return cancel();
      const dx = event.touches[0].clientX - start.x;
      const dy = Math.abs(event.touches[0].clientY - start.y);
      if (dx < -8 || (dy > 8 && dy >= Math.abs(dx))) return cancel();
      // Reserve only a confirmed horizontal edge gesture; vertical scrolling remains native.
      if (dx > 8 && dx > dy * 1.5 && event.cancelable) event.preventDefault();
    };
    const handleEnd = (event: TouchEvent) => {
      const origin = start;
      cancel();
      if (!origin || event.changedTouches.length !== 1 || event.touches.length !== 0) return;
      const dx = event.changedTouches[0].clientX - origin.x;
      const dy = Math.abs(event.changedTouches[0].clientY - origin.y);
      if (dx >= 72 && dx > dy * 1.5) onClose();
    };
    element.addEventListener("touchstart", handleStart, { passive: true });
    element.addEventListener("touchmove", handleMove, { passive: false });
    element.addEventListener("touchend", handleEnd, { passive: true });
    element.addEventListener("touchcancel", cancel, { passive: true });
    return {
      destroy() {
        element.removeEventListener("touchstart", handleStart);
        element.removeEventListener("touchmove", handleMove);
        element.removeEventListener("touchend", handleEnd);
        element.removeEventListener("touchcancel", cancel);
      },
    };
  }

  function formatDateTime(value: string) {
    return new Date(value).toLocaleString("zh-CN", {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function formatNextReview(value: string) {
    return new Date(value).toLocaleDateString("zh-CN", {
      month: "short",
      day: "numeric",
    });
  }

  function openGrammar(branch: GrammarBranch, triggerRect: DOMRect) {
    activeGrammarBranch = branch;
    popoverTriggerRect = triggerRect;
  }
</script>

{#if entry}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="detail-overlay" onclick={onClose} transition:fade={{ duration: 200 }}>
    <div class="drawer-shell" use:edgeSwipe onclick={(event) => event.stopPropagation()} transition:slide={{ axis: "x", duration: 300 }}>
      <div class="detail-toolbar">
        <h2 class="word-title">{entry.query_text}</h2>
        <button class="detail-close-floating" onclick={onClose} aria-label="返回词库">
          <i class="ph ph-arrow-left" aria-hidden="true"></i>
        </button>
      </div>
    <div class="drawer-panel">

      <header class="drawer-head">
          <div class="title-actions">
            {#if learningStatus}
              <span class="learning-status {learningStatus.className}">{learningStatus.label}</span>
            {/if}
            <button
              class="learning-action"
              class:added={Boolean(learningProgress)}
              onclick={() => onAddToLearning?.(entry.entry_id)}
              disabled={Boolean(learningProgress) || isAddingToLearning || !onAddToLearning}
              aria-busy={isAddingToLearning}
            >
              <i class={learningProgress ? "ph ph-check" : isAddingToLearning ? "ph ph-spinner" : "ph ph-target"}></i>
              {learningProgress ? "已加入学习" : isAddingToLearning ? "正在加入..." : "加入学习"}
            </button>
          </div>
        <div class="drawer-meta">
          <span class="meta-pill">#{entry.entry_id}</span>
          <span class="meta-pill">{entry.entry_type}</span>
          <span class="meta-pill"><i class="ph ph-clock"></i> {formatDateTime(entry.updated_at)}</span>
          {#if learningProgress}
            <span class="meta-pill"><i class="ph ph-calendar"></i> 下次复习 {formatNextReview(learningProgress.next_review_at)}</span>
          {/if}
        </div>
      </header>

      <div class="drawer-content">
        {#if structured}
          <section class="content-section">
            <div class="card-title"><i class="ph-fill ph-article"></i> 词条解析</div>
            <div class="detail-analysis">
              <LearningAnalysisContent
                interleavedMeanings
                analysis={structured}
                attachedPhraseModules={entry.attached_phrase_modules}
                onGrammarOpen={openGrammar}
              />
            </div>
          </section>
        {/if}

        {#if entry.follow_ups.length > 0}
          <section class="content-section">
            <div class="card-title"><i class="ph-fill ph-chat-circle-dots"></i> 历史追问记录</div>
            <div class="follow-up-history-list">
              {#each entry.follow_ups as followUp}
                <article class="surface-card fu-item">
                  <div class="fu-q">Q: {followUp.question}</div>
                  <div class="markdown-compact fu-a">
                    {@html renderMarkdownHtml(followUp.answer)}
                  </div>
                </article>
              {/each}
            </div>
          </section>
        {/if}

        <section class="content-section drawer-actions">
          <button class="btn-primary" onclick={() => onViewInSearch(entry!.query_text)}>
            <i class="ph ph-magnifying-glass"></i> 在搜索页打开
          </button>
          <button class="btn-secondary danger-text" onclick={() => onDelete(entry!.entry_id)} disabled={isDeleting}>
            <i class="ph ph-trash"></i> {isDeleting ? "正在删除..." : "删除词条"}
          </button>
        </section>

        <details class="debug-section">
          <summary>调试信息 (Markdown 原文)</summary>
          <pre class="raw-markdown"><code>{entry.analysis_markdown}</code></pre>
        </details>
      </div>
    </div>
    </div>
  </div>

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
{/if}

<style>
  .detail-overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    justify-content: flex-end;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(4px);
  }

  .drawer-shell { position: relative; display: grid; grid-template-rows: auto minmax(0, 1fr); width: 100%; max-width: 800px; height: 100%; }
  .detail-toolbar { position: relative; z-index: 2; display: flex; min-height: 72px; align-items: center; justify-content: center; padding: 14px 72px; background: var(--bg-color); border-bottom: 1px solid var(--border-color); }

  .drawer-panel {
    position: relative;
    display: flex;
    width: 100%;
    max-width: 800px;
    height: 100%;
    flex-direction: column;
    min-height: 0;
    padding: 1.5rem 2rem;
    overflow-y: auto;
    overscroll-behavior: contain;
    background: var(--bg-color);
    box-shadow: -10px 0 30px rgba(0, 0, 0, 0.1);
    scrollbar-gutter: stable;
    border-bottom: 16px solid var(--bg-color);
  }

  .detail-close-floating {
    position: absolute;
    z-index: 3;
    top: 14px;
    left: 16px;
    display: flex;
    width: 44px;
    height: 44px;
    align-items: center;
    justify-content: center;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--text-main);
    cursor: pointer;
    font-size: 1.5rem;
  }

  @media (hover: hover) {
    .detail-close-floating:hover { background: var(--card-bg); }
  }
  .detail-close-floating:active { background: var(--card-bg); }
  .detail-close-floating:focus-visible { outline: 2px solid var(--accent-main); outline-offset: 2px; }

  .drawer-head { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: 10px; margin-bottom: 2rem; }
  .word-title { width: 100%; min-width: 0; margin: 0; text-align: center; overflow-wrap: anywhere; color: var(--text-main); font-size: 2.5rem; font-weight: 800; line-height: 1.1; }
  .title-actions { grid-column: 2; grid-row: 1; display: flex; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: 0.55rem; }
  .learning-action { display: flex; min-width: 112px; min-height: 36px; align-items: center; justify-content: center; gap: 0.4rem; padding: 0 0.9rem; border-radius: var(--radius-full); background: var(--accent-main); color: white; font-weight: 800; }
  .learning-action.added { background: color-mix(in srgb, var(--accent-main) 72%, #4f8665); cursor: default; }
  .learning-action:disabled { cursor: default; }
  .learning-status { padding: 0.25rem 0.6rem; border-radius: 6px; font-size: 0.75rem; font-weight: 800; white-space: nowrap; }
  .learning-status.learning { background: var(--success-bg); color: var(--success-text); }
  .learning-status.review { background: var(--warning-bg); color: var(--warning-text); }
  .drawer-meta { grid-column: 1; grid-row: 1; display: flex; flex-wrap: wrap; gap: 0.5rem; color: var(--text-muted); }
  .meta-pill { display: inline-flex; align-items: center; gap: 0.3rem; padding: 0.28rem 0.55rem; border-radius: var(--radius-sm); background: var(--surface-passive); font-size: 0.78rem; }

  .drawer-content { display: flex; flex-direction: column; gap: 2.5rem; }
  .content-section { display: flex; flex-direction: column; gap: 1rem; }
  .detail-analysis :global(.learning-content) { padding-bottom: 0; }
  .detail-analysis :global(.meaning-summary) { position: relative; display: block; }
  .detail-analysis :global(.meaning-summary.interactive) { padding-right: 2rem; }
  .detail-analysis :global(.meaning-summary > i) { position: absolute; top: 50%; right: 10px; transform: translateY(-50%); }

  .fu-q { margin-bottom: 0.75rem; color: var(--accent-main); font-size: 1.05rem; font-weight: 700; }
  .fu-a { margin-top: 0.75rem; padding-top: 0.75rem; border-top: 1px solid var(--border-color); }
  .follow-up-history-list { display: flex; flex-direction: column; gap: 1rem; }
  .fu-item { padding: 1.25rem; }

  .debug-section { margin-top: 3rem; padding-top: 1rem; border-top: 1px solid var(--border-color); }
  .debug-section summary { padding: 0.5rem 0; color: var(--text-muted); cursor: pointer; font-size: 0.85rem; }
  .raw-markdown { margin-top: 0.5rem; padding: 1rem; overflow-x: auto; border-radius: var(--radius-sm); background: var(--surface-passive); font-family: monospace; font-size: 0.8rem; white-space: pre-wrap; word-break: break-all; }

  .drawer-actions { flex-direction: row; gap: 1rem; margin-top: 2rem; padding-top: 2rem; border-top: 2px dashed var(--border-color); }
  .danger-text { color: var(--danger-text); }

  @media (max-width: 768px) {
    .drawer-panel { padding: 1.5rem 1rem; }
    .word-title { font-size: 2rem; }
    .title-actions { align-items: flex-end; flex-direction: column; }
    .drawer-actions { flex-direction: row; justify-content: flex-start; gap: 8px; margin-top: 12px; padding-top: 16px; }
    .drawer-actions button { flex: 0 0 auto; min-height: 38px; padding: 8px 20px; font-size: 12px; white-space: nowrap; }
  }

  @media (max-width: 600px) {
    .detail-overlay { background: var(--bg-color); backdrop-filter: none; }
    .detail-toolbar { min-height: 56px; padding: 13px 60px; }
    .drawer-panel { padding: 16px; border-bottom: 0; }
    .drawer-head { min-width: 0; grid-template-columns: minmax(0, 1fr) auto; gap: 8px; margin-bottom: 12px; }
    .word-title { font-size: 26px; }
    .title-actions { gap: 4px; }
    .learning-action { min-width: 0; min-height: 32px; padding: 0 10px; gap: 4px; font-size: 11px; white-space: nowrap; }
    .learning-status { padding: 2px 5px; font-size: 10px; }
    .drawer-meta { align-content: center; align-items: center; gap: 4px; }
    .meta-pill { gap: 3px; padding: 2px 5px; font-size: 10px; line-height: 16px; }
    .detail-close-floating { top: 6px; left: 8px; }
    .drawer-content { gap: 16px; }
    .content-section { gap: 8px; }

    /* Keep the density pilot local to library details before adopting it elsewhere. */
    .detail-analysis :global(.learning-content) { gap: 10px; }
    .detail-analysis :global(.section-card) { padding: 10px; border-radius: 16px; }
    .detail-analysis :global(.section-title) { margin-bottom: 8px; font-size: 12px; }
    .detail-analysis :global(.meaning-summary) { position: relative; display: block; padding: 10px; border-radius: 10px; font-size: 0; line-height: 20px; }
    .detail-analysis :global(.meaning-summary.interactive) { padding-right: 10px; }
    .detail-analysis :global(.meaning-summary > i) { display: none; }
    .detail-analysis :global(.usage-list),
    .detail-analysis :global(.insights-stack) { gap: 10px; }
    .detail-analysis :global(.usage-item),
    .detail-analysis :global(.insight-block) { padding: 6px; border-radius: 10px; font-size: 13px; line-height: 1.5; }
    .detail-analysis :global(.usage-copy),
    .detail-analysis :global(.insight-copy) { margin-top: 5px; line-height: 1.5; }
    .detail-analysis :global(.example-box) { margin-top: 6px; padding: 8px; }
    .detail-analysis :global(.usage-item.example-box) { margin-top: 0; }
    .detail-analysis :global(.zh-line) { font-size: 11.5px; line-height: 1.5; margin-top: 4px; }
  }

</style>
