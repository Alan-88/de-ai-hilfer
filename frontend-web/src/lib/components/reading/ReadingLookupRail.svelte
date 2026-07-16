<script lang="ts">
  import { resolveStructuredAnalysis, type StructuredAnalysis } from "$lib/analysis/structuredAnalysis";
  import GrammarBranchPopover from "$lib/components/bento/GrammarBranchPopover.svelte";
  import LearningAnalysisContent from "$lib/components/bento/LearningAnalysisContent.svelte";
  import { getEntryDetail } from "$lib/queryApi";
  import type { EntryDetailResponse, GrammarBranch } from "$lib/types";

  let { selectedKey, open = false, onClose } = $props<{
    selectedKey: string;
    open?: boolean;
    onClose: () => void;
  }>();

  const entryIds: Record<string, number> = {
    Pauschalreise: 2565,
    Reise: 1266,
    "übernachten": 2055,
    erholen: 2563,
    bequem: 1094,
  };

  const cache = new Map<number, EntryDetailResponse>();
  let query = $state("Pauschalreise");
  let entry = $state<EntryDetailResponse | null>(null);
  let analysis = $state<StructuredAnalysis | null>(null);
  let error = $state("");
  let loading = $state(true);
  let requestId = 0;
  let activeGrammarBranch = $state<GrammarBranch | null>(null);
  let popoverTriggerRect = $state<DOMRect | null>(null);

  $effect(() => {
    if (selectedKey) void showResult(selectedKey);
  });

  async function showResult(key: string) {
    const currentRequest = ++requestId;
    query = key;
    activeGrammarBranch = null;
    const entryId = entryIds[key];
    if (!entryId) {
      entry = null;
      analysis = null;
      loading = false;
      error = `${key} 尚未初始化`;
      return;
    }
    loading = true;
    error = "";
    try {
      const next = cache.get(entryId) ?? await getEntryDetail(entryId);
      if (currentRequest !== requestId) return;
      cache.set(entryId, next);
      entry = next;
      analysis = resolveStructuredAnalysis(next.analysis_markdown, next.structured_analysis, next.query_text);
    } catch (cause) {
      if (currentRequest !== requestId) return;
      entry = null;
      analysis = null;
      error = cause instanceof Error ? cause.message : "词条加载失败";
    } finally {
      if (currentRequest === requestId) loading = false;
    }
  }

  function submitSearch() {
    const normalized = query.trim().toLowerCase();
    const key = Object.keys(entryIds).find((item) => item.toLowerCase() === normalized);
    if (key) void showResult(key);
    else error = "未找到可靠候选";
  }

  function openGrammar(branch: GrammarBranch, triggerRect: DOMRect) {
    activeGrammarBranch = branch;
    popoverTriggerRect = triggerRect;
  }
</script>

<aside class="lookup-rail" class:mobile-open={open} aria-label="查词栏">
  <header class="lookup-head">
    <form onsubmit={(event) => { event.preventDefault(); submitSearch(); }}>
      <i class="ph ph-magnifying-glass"></i>
      <input bind:value={query} aria-label="查询单词" autocomplete="off" />
      <button type="submit" aria-label="查询"><i class="ph ph-arrow-right"></i></button>
    </form>
    <button class="mobile-close" onclick={onClose} aria-label="关闭查词栏"><i class="ph ph-x"></i></button>
  </header>

  <div class="lookup-scroll">
    {#if loading}
      <div class="loading-result"><span></span><span></span><span></span></div>
    {:else if error}
      <div class="empty-result"><i class="ph ph-warning-circle"></i><span>{error}</span></div>
    {:else if entry && analysis}
      <section class="word-head">
        <div class="word-title-row">
          <h2>{analysis.headword || entry.query_text}</h2>
          <button class="open-entry" aria-label="打开完整词条"><i class="ph ph-arrow-square-out"></i></button>
        </div>
        <div class="word-meta-row">
          {#if analysis.phonetic}<span class="word-phonetic">{analysis.phonetic}</span>{/if}
          <button class="learning-action"><i class="ph ph-target"></i> 加入复习</button>
        </div>
      </section>

      <LearningAnalysisContent
        {analysis}
        attachedPhraseModules={entry.attached_phrase_modules}
        onGrammarOpen={openGrammar}
      />

    {/if}
  </div>
</aside>

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
  .lookup-rail { display: flex; min-width: 0; min-height: 0; flex-direction: column; overflow: hidden; border-left: 1px solid var(--border-color); background: color-mix(in srgb, var(--card-bg) 78%, var(--bg-color)); }
  .lookup-head { display: flex; align-items: center; gap: 0.55rem; padding: 1rem; border-bottom: 1px solid var(--border-color); }
  form { display: flex; flex: 1; align-items: center; gap: 0.5rem; min-width: 0; height: 42px; padding: 0 0.45rem 0 0.8rem; border: 1px solid var(--border-color); border-radius: var(--radius-full); background: var(--card-bg); box-shadow: var(--shadow-sm); }
  form > i { color: var(--text-muted); }
  input { min-width: 0; flex: 1; border: 0; outline: 0; background: transparent; color: var(--text-main); font: inherit; font-weight: 700; }
  form button, .mobile-close { display: grid; width: 32px; height: 32px; place-items: center; border-radius: 50%; background: var(--accent-main); color: white; }
  .mobile-close { display: none; flex: 0 0 auto; background: var(--btn-secondary); color: var(--text-main); }
  .lookup-scroll { min-height: 0; flex: 1; overflow-y: auto; padding: 1.35rem; overscroll-behavior: contain; }
  .word-head { display: grid; gap: 0.65rem; padding: 0.35rem 0 1.2rem; }
  .word-title-row, .word-meta-row { display: flex; min-width: 0; align-items: center; justify-content: space-between; gap: 0.8rem; }
  .word-head h2 { margin: 0; color: var(--text-main); font-size: 2.15rem; line-height: 1.12; letter-spacing: -0.035em; }
  .word-phonetic { min-width: 0; color: var(--text-muted); font-family: "Arial Unicode MS", "Lucida Sans Unicode", sans-serif; font-size: 0.86rem; line-height: 1.6; }
  .open-entry { display: grid; width: 36px; height: 36px; flex: 0 0 auto; place-items: center; border-radius: 50%; background: var(--btn-secondary); color: var(--text-main); }
  .learning-action { display: flex; min-width: 112px; min-height: 36px; flex: 0 0 auto; align-items: center; justify-content: center; gap: 0.4rem; padding: 0 0.9rem; border-radius: var(--radius-full); background: var(--accent-main); color: white; font-weight: 800; }
  .empty-result { display: flex; align-items: center; gap: 0.65rem; padding: 1rem; border: 1px dashed var(--border-color); border-radius: var(--radius-md); color: var(--text-muted); }
  .loading-result { display: flex; justify-content: center; gap: 0.4rem; padding: 3rem 0; }
  .loading-result span { width: 7px; height: 7px; border-radius: 50%; background: var(--text-muted); animation: pulse 900ms ease-in-out infinite alternate; }
  .loading-result span:nth-child(2) { animation-delay: 150ms; }
  .loading-result span:nth-child(3) { animation-delay: 300ms; }
  @keyframes pulse { to { opacity: 0.25; transform: translateY(-3px); } }

  :global(.lookup-rail .learning-content) { gap: 0.8rem; padding-bottom: 1.25rem; }
  :global(.lookup-rail .learning-content .section-card) { padding: 0.9rem; border-radius: var(--radius-md); }
  :global(.lookup-rail .learning-content .meaning-summary) { gap: 0.6rem; padding: 0.8rem; }
  :global(.lookup-rail .learning-content .meaning-summary strong) { font-size: 0.94rem; }
  :global(.lookup-rail .learning-content .usage-item) { padding: 0.1rem 0 0.8rem; border-radius: 0; background: transparent; }
  :global(.lookup-rail .learning-content .usage-item + .usage-item) { padding-top: 0.85rem; border-top: 1px solid var(--border-color); }
  :global(.lookup-rail .learning-content .usage-item > strong) { font-size: 0.94rem; line-height: 1.45; }
  :global(.lookup-rail .learning-content .usage-copy) { margin-top: 0.35rem; font-size: 0.86rem; line-height: 1.6; }
  :global(.lookup-rail .learning-content .example-box) { margin-top: 0.7rem; padding: 0.8rem 0.9rem; border-left-width: 3px; border-left-color: var(--accent-main); background: var(--card-bg); }
  :global(.lookup-rail .learning-content .de-line) { font-size: 0.94rem; line-height: 1.45; }
  :global(.lookup-rail .learning-content .zh-line) { font-size: 0.82rem; line-height: 1.5; }
  :global(.lookup-rail .learning-content .insight-block) { padding: 0.1rem 0 0.8rem; border-radius: 0; background: transparent; }
  :global(.lookup-rail .learning-content .insight-block + .insight-block) { padding-top: 0.85rem; border-top: 1px solid var(--border-color); }
  :global(.lookup-rail .learning-content .insight-copy) { font-size: 0.86rem; line-height: 1.65; }
  :global(.lookup-rail .learning-content .network-grid article) { max-width: 100%; }

  @media (max-width: 900px) {
    .lookup-rail { position: fixed; inset: 0 0 0 auto; z-index: 260; width: min(390px, 92vw); transform: translateX(102%); box-shadow: -18px 0 50px rgba(20, 18, 31, 0.2); transition: transform 180ms ease; }
    .lookup-rail.mobile-open { transform: translateX(0); }
    .mobile-close { display: grid; }
  }
</style>
