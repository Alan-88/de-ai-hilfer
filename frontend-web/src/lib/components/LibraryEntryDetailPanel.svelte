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
    <div class="drawer-panel" onclick={(event) => event.stopPropagation()} transition:slide={{ axis: "x", duration: 300 }}>
      <button class="detail-close-floating" onclick={onClose} aria-label="关闭详情">
        <i class="ph ph-x"></i>
      </button>

      <header class="drawer-head">
        <div class="title-line">
          <h2 class="word-title">{entry.query_text}</h2>
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
                compactMeanings
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

  .drawer-panel {
    position: relative;
    display: flex;
    width: 100%;
    max-width: 800px;
    height: 100%;
    flex-direction: column;
    padding: 3rem 2rem;
    overflow-y: auto;
    background: var(--bg-color);
    box-shadow: -10px 0 30px rgba(0, 0, 0, 0.1);
    scrollbar-gutter: stable;
  }

  .detail-close-floating {
    position: fixed;
    top: 1.5rem;
    right: 1.5rem;
    display: flex;
    width: 3rem;
    height: 3rem;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-color);
    border-radius: 50%;
    background: var(--card-bg);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
    font-size: 1.5rem;
  }

  .drawer-head { display: grid; gap: 0.9rem; margin-bottom: 2rem; }
  .title-line { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; padding-right: 3.5rem; }
  .word-title { margin: 0; color: var(--text-main); font-size: 2.5rem; font-weight: 800; line-height: 1.1; }
  .title-actions { display: flex; flex: 0 0 auto; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: 0.55rem; }
  .learning-action { display: flex; min-width: 112px; min-height: 36px; align-items: center; justify-content: center; gap: 0.4rem; padding: 0 0.9rem; border-radius: var(--radius-full); background: var(--accent-main); color: white; font-weight: 800; }
  .learning-action.added { background: color-mix(in srgb, var(--accent-main) 72%, #4f8665); cursor: default; }
  .learning-action:disabled { cursor: default; }
  .learning-status { padding: 0.25rem 0.6rem; border-radius: 6px; font-size: 0.75rem; font-weight: 800; white-space: nowrap; }
  .learning-status.learning { background: var(--success-bg); color: var(--success-text); }
  .learning-status.review { background: var(--warning-bg); color: var(--warning-text); }
  .drawer-meta { display: flex; flex-wrap: wrap; gap: 0.5rem; color: var(--text-muted); }
  .meta-pill { display: inline-flex; align-items: center; gap: 0.3rem; padding: 0.28rem 0.55rem; border-radius: var(--radius-sm); background: var(--surface-passive); font-size: 0.78rem; }

  .drawer-content { display: flex; flex-direction: column; gap: 2.5rem; }
  .content-section { display: flex; flex-direction: column; gap: 1rem; }
  .detail-analysis :global(.learning-content) { padding-bottom: 0; }

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
    .drawer-panel { padding: 2rem 1rem; }
    .title-line { padding-right: 2.8rem; }
    .word-title { font-size: 2rem; }
    .title-actions { align-items: flex-end; flex-direction: column; }
    .drawer-actions { flex-direction: column; }
  }

  @media (max-width: 600px) {
    .drawer-panel { padding: 20px 16px; }
    .drawer-head { min-width: 0; gap: 8px; margin-bottom: 12px; }
    .title-line { min-width: 0; padding-right: 56px; }
    .word-title { min-width: 0; font-size: 26px; overflow-wrap: anywhere; }
    .detail-close-floating { top: 16px; right: 16px; width: 44px; height: 44px; font-size: 20px; }
    .drawer-content { gap: 16px; }
    .content-section { gap: 8px; }

    /* Keep the density pilot local to library details before adopting it elsewhere. */
    .detail-analysis :global(.learning-content) { gap: 10px; }
    .detail-analysis :global(.section-card) { padding: 10px; border-radius: 16px; }
    .detail-analysis :global(.section-title) { margin-bottom: 8px; font-size: 12px; }
    .detail-analysis :global(.meaning-summary) { position: relative; display: block; padding: 10px; border-radius: 10px; font-size: 0; line-height: 20px; }
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

  @media (max-width: 360px) {
    .title-line { flex-direction: column; gap: 10px; }
    .title-actions { flex-direction: row; flex-wrap: wrap; justify-content: flex-start; }
  }
</style>
