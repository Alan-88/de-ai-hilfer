<script lang="ts">
  import ReadOnlyLearningContent from "$lib/components/learning-layout-lab/ReadOnlyLearningContent.svelte";
  import GrammarBranchPopover from "$lib/components/bento/GrammarBranchPopover.svelte";
  import { vertragenSample } from "$lib/components/learning-layout-lab/realLearningSample";
  import type { GrammarBranch } from "$lib/types";

  let { revealed = $bindable(false) } = $props<{ revealed?: boolean }>();
  let contentElement = $state<HTMLDivElement | null>(null);
  let wordElement = $state<HTMLHeadingElement | null>(null);
  let dockedTitle = $state(false);
  let activeGrammarBranch = $state<GrammarBranch | null>(null);
  let popoverTriggerRect = $state<DOMRect | null>(null);

  function handleScroll() {
    if (!contentElement || !wordElement) return;
    const boundary = contentElement.getBoundingClientRect().top;
    const wordTop = wordElement.getBoundingClientRect().top;
    if (!dockedTitle && wordTop <= boundary) dockedTitle = true;
    if (dockedTitle && wordTop > boundary + 12) dockedTitle = false;
  }

  function openGrammar(branch: GrammarBranch, triggerRect: DOMRect) {
    activeGrammarBranch = branch;
    popoverTriggerRect = triggerRect;
  }
</script>

<section class="layout" aria-label="学习卡片交互预览">
  <div class="progress-row"><div class="progress"><span style="width: 43%"></span></div><strong>18 / 42</strong></div>
  <div class="bento-card flashcard">
    {#if !revealed}
      <button class="recall-surface" aria-label="查看 vertragen 的答案" onclick={() => revealed = true}>
        <div class="meta"><span>复习 · 第 2 次</span><span>上次：7月3日</span></div>
        <div class="recall-content">
          <h2>{vertragenSample.headword}</h2>
          <div class="recall-prompt"><strong>回忆词义与用法</strong><span>点击卡片查看答案</span></div>
        </div>
      </button>
    {:else}
      <div class="answer-shell">
        <div class="meta answer-meta"><span>复习 · 第 2 次</span><strong class:visible={dockedTitle}>{vertragenSample.headword}</strong><span>上次：7月3日</span></div>
        <div class="scroll-content" bind:this={contentElement} onscroll={handleScroll}>
          <header class="word-intro"><h2 bind:this={wordElement}>{vertragenSample.headword}</h2><span>{vertragenSample.meanings[0].part_of_speech}</span></header>
          <ReadOnlyLearningContent analysis={vertragenSample} onGrammarOpen={openGrammar} />
        </div>
      </div>
    {/if}
  </div>
  {#if revealed}<div class="rating-bar"><button class="forgot">忘记</button><button>模糊</button><button class="known">认识</button></div>{/if}
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
  .layout { width: min(940px, 100%); margin: 0 auto; }
  .progress-row { display: flex; align-items: center; gap: 1rem; margin-bottom: 1rem; }
  .progress { flex: 1; height: 8px; overflow: hidden; border-radius: var(--radius-full); background: var(--btn-secondary); }
  .progress span { display: block; height: 100%; background: var(--accent-main); }
  .progress-row > strong { min-width: 48px; color: var(--text-muted); font-size: 0.88rem; font-variant-numeric: tabular-nums; }
  .flashcard { height: clamp(540px, calc(100vh - 230px), 680px); overflow: hidden; padding: 0; }
  .recall-surface { width: 100%; height: 100%; padding: clamp(1.5rem, 3vw, 2.5rem); border-radius: inherit; background: transparent; color: var(--text-main); text-align: inherit; }
  .meta { display: flex; align-items: center; justify-content: space-between; gap: 1rem; color: var(--text-muted); font-size: 0.8rem; font-weight: 700; }
  .recall-content { height: calc(100% - 1rem); display: flex; flex-direction: column; align-items: center; justify-content: center; gap: clamp(4.5rem, 11vh, 7rem); text-align: center; }
  h2 { font-size: clamp(3rem, 5vw, 4.4rem); font-weight: 800; line-height: 1.1; letter-spacing: 0; }
  .recall-prompt { display: grid; gap: 0.45rem; color: var(--text-muted); }
  .recall-prompt strong { color: var(--text-main); font-size: 1.08rem; }
  .recall-prompt span { font-size: 0.9rem; }
  .answer-shell { height: 100%; display: flex; flex-direction: column; padding: 1.5rem clamp(1.5rem, 3vw, 2.5rem) 0; }
  .answer-meta { position: relative; flex: 0 0 auto; min-height: 42px; padding-bottom: 0.8rem; }
  .answer-meta > strong { position: absolute; left: 50%; bottom: 0.8rem; max-width: 52%; overflow: visible; transform: translateX(-50%) translateY(3px); color: var(--text-main); font-size: 1.8rem; line-height: 1.15; opacity: 0; text-overflow: ellipsis; white-space: nowrap; transition: opacity 140ms ease, transform 140ms ease; pointer-events: none; }
  .answer-meta > strong.visible { transform: translateX(-50%) translateY(0); opacity: 1; }
  .scroll-content { flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain; scrollbar-gutter: stable; }
  .word-intro { display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 190px; padding: 1.5rem 0; }
  .word-intro > span { margin-top: 0.4rem; color: var(--text-muted); font-size: 0.82rem; font-weight: 700; }
  .rating-bar { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.75rem; margin-top: 1rem; }
  .rating-bar button { min-height: 52px; border-radius: var(--radius-md); background: var(--btn-secondary); color: var(--btn-secondary-text); font-weight: 800; }
  .rating-bar .forgot { background: var(--danger-bg); color: var(--danger-text); }
  .rating-bar .known { background: var(--success-bg); color: var(--success-text); }
  @media (max-width: 680px) { .flashcard { height: calc(100dvh - 250px); min-height: 500px; } }
</style>
