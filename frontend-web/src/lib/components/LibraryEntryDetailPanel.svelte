<script lang="ts">
  import { fade, slide } from "svelte/transition";
  import {
    resolveStructuredAnalysis,
    renderMarkdownHtml,
  } from "$lib/analysis/structuredAnalysis";
  import GrammarFeatureCard from "$lib/components/bento/GrammarFeatureCard.svelte";
  import GrammarBranchPopover from "$lib/components/bento/GrammarBranchPopover.svelte";
  import type { EntryDetailResponse, GrammarBranch } from "$lib/types";

  let {
    entry = $bindable(),
    isDeleting = false,
    onClose,
    onDelete,
    onViewInSearch
  }: {
    entry: EntryDetailResponse | null;
    isDeleting?: boolean;
    onClose: () => void;
    onDelete: (id: number) => void;
    onViewInSearch: (query: string) => void;
  } = $props();

  function formatDateTime(value: string) {
    return new Date(value).toLocaleString("zh-CN", {
      month: "short", day: "numeric", hour: "2-digit", minute: "2-digit"
    });
  }

  const structured = $derived(
    entry
      ? resolveStructuredAnalysis(
          entry.analysis_markdown,
          entry.structured_analysis,
          entry.query_text
        )
      : null
  );
  const primaryMeaning = $derived(structured?.meanings[0] ?? null);

  let activeGrammarBranch = $state<GrammarBranch | null>(null);
  let popoverTriggerRect = $state<DOMRect | null>(null);
  const useBranchUI = $derived((structured?.grammarBranches?.length ?? 0) > 0);

  function formatPos(branch: GrammarBranch): string {
    const p = (branch.pos || "").toLowerCase();
    const g = branch.grammar;
    const posTokens = p.split(/[\s,]+/);

    if (posTokens.includes("verb")) {
      let base = "v.";
      const trans = (g.transitivity || "").toLowerCase().trim();
      if (trans === "transitive") base = "vt.";
      else if (trans === "intransitive") base = "vi.";
      else if (trans === "both") base = "vt./vi.";

      let res = base;
      const refl = (g.reflexive || "").toLowerCase().trim();
      if (refl === "optional" || refl === "required") res += " refl.";

      const sep = (g.separable || "").toLowerCase().trim();
      if (sep === "separable") res += " (sep.)";
      return res;
    }

    if (posTokens.includes("noun")) {
      const genderMap: Record<string, string> = {
        "masculine": "m.", "feminine": "f.", "neuter": "n.", "plural": "pl."
      };
      // 支持多性别名词拼接，如 (m./n.) n.
      const genders = (g.genders || [])
        .map(v => genderMap[v.toLowerCase().trim()] || "")
        .filter(Boolean);

      const prefix = genders.length > 0 ? `(${genders.join("/")}) ` : "";
      return `${prefix}n.`;
    }

    if (posTokens.includes("adjective") && posTokens.includes("adverb")) return "adj./adv.";
    if (posTokens.includes("adjective")) return "adj.";
    if (posTokens.includes("adverb")) return "adv.";
    if (posTokens.includes("pronoun")) return "pron.";
    if (posTokens.includes("preposition")) return "prep.";
    if (posTokens.includes("conjunction")) return "conj.";
    if (posTokens.includes("article")) return "art.";

    return branch.pos;
  }

  function formatMeanings(meanings: {zh: string, en: string}[]): string {
    return meanings.map(m => m.zh).filter(Boolean).join("；");
  }

  // 分支按词性分组，同词性放一排
  const branchGroups = $derived(() => {
    const groups: (GrammarBranch[])[] = [];
    if (!structured?.grammarBranches) return groups;

    structured.grammarBranches.forEach(branch => {
      const lastGroup = groups[groups.length - 1];
      if (lastGroup && lastGroup[0].pos === branch.pos) {
        lastGroup.push(branch);
      } else {
        groups.push([branch]);
      }
    });
    return groups;
  });
</script>

{#if entry}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="detail-overlay" onclick={onClose} transition:fade={{ duration: 200 }}>
    <div class="drawer-panel" onclick={(e) => e.stopPropagation()} transition:slide={{ axis: 'x', duration: 300 }}>
      <button class="detail-close-floating" onclick={onClose} aria-label="关闭详情">
        <i class="ph ph-x"></i>
      </button>

      <header class="drawer-head">
        <div class="head-main">
          <h2 class="word-title">{entry.query_text}</h2>
          {#if useBranchUI}
            <div class="drawer-branch-list">
              {#each branchGroups() as group}
                <div class="drawer-branch-group-row">
                  {#each group as branch}
                    <button
                      class="drawer-branch-item"
                      onclick={(e) => {
                        activeGrammarBranch = branch;
                        popoverTriggerRect = (e.currentTarget as HTMLElement).getBoundingClientRect();
                      }}
                    >
                      <span class="pos-badge compact">{formatPos(branch)}</span>
                      <span class="zh-text">{formatMeanings(branch.meanings)}</span>
                      <i class="ph ph-info branch-info-icon"></i>
                    </button>
                  {/each}
                </div>
              {/each}
            </div>
          {:else}
            <div class="drawer-meta">
              <span class="meta-pill">#{entry.entry_id}</span>
              <span class="meta-pill">{entry.entry_type}</span>
              <span class="meta-pill"><i class="ph ph-clock"></i> {formatDateTime(entry.updated_at)}</span>
            </div>
          {/if}
        </div>
      </header>

      <div class="drawer-content">
        <section class="content-section">
          <div class="card-title"><i class="ph-fill ph-article"></i> 完整分析内容</div>
          <div class="analysis-layout" class:is-full={useBranchUI}>
            <div class="surface-card analysis-main-card">
              {#if primaryMeaning && !useBranchUI}
                <div class="surface-card info-block">
                  <p class="small-copy">核心义项</p>
                  <p class="card-copy">
                    <strong>{primaryMeaning.partOfSpeech}</strong> {primaryMeaning.chinese}
                  </p>
                </div>
              {/if}

              {#if structured?.usageModules.length}
                <div class="section-stack">
                  {#each structured.usageModules as usage}
                    <div class="surface-card info-block">
                      <p class="small-copy"><strong>{usage.title}</strong></p>
                      {#if usage.explanation}
                        <div class="card-copy usage-copy markdown-compact">{@html renderMarkdownHtml(usage.explanation)}</div>
                      {/if}
                      {#if usage.example?.de}
                        <div class="example-box">
                          <div class="de-line markdown-compact">{@html renderMarkdownHtml(usage.example.de)}</div>
                          {#if usage.example.zh}
                            <div class="zh-line markdown-compact">{@html renderMarkdownHtml(usage.example.zh)}</div>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  {/each}
                </div>
              {:else if structured?.examples.length}
                <div class="section-stack">
                  {#each structured.examples as example}
                    <div class="surface-card info-block">
                      <div class="de-line markdown-compact">{@html renderMarkdownHtml(example.de)}</div>
                      {#if example.zh}
                        <div class="zh-line markdown-compact">{@html renderMarkdownHtml(example.zh)}</div>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}

              {#if structured?.deepInsights.length}
                <div class="section-stack">
                  <div class="small-copy"><strong>深度解析与避坑</strong></div>
                  {#each structured.deepInsights as insight}
                    <div class="surface-card info-block">
                      <p class="small-copy"><strong>{insight.title}</strong></p>
                      <div class="markdown-compact insight-html">
                        {@html insight.html}
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>

            {#if !useBranchUI}
              <div class="surface-card analysis-side-card">
                <div class="small-copy"><strong>语法摘要</strong></div>
                <GrammarFeatureCard grammarRows={structured?.grammarRows ?? []} />

                {#if structured?.wordNetwork.family.length || structured?.wordNetwork.synonyms.length || structured?.wordNetwork.antonyms.length}
                  <div class="network-block">
                    <div class="small-copy"><strong>词汇网络</strong></div>
                    <div class="tag-cloud">
                      {#each [
                        ...(structured?.wordNetwork.family ?? []),
                        ...(structured?.wordNetwork.synonyms ?? []),
                        ...(structured?.wordNetwork.antonyms ?? [])
                      ] as item}
                        <span
                          class="tag"
                          title="{item.partOfSpeech ? `[${item.partOfSpeech}] ` : ''}${item.chinese || item.english || ''}"
                        >
                          {item.term}
                        </span>
                      {/each}
                    </div>
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        </section>

        {#if entry.follow_ups.length > 0}
          <section class="content-section">
            <div class="card-title"><i class="ph-fill ph-chat-circle-dots"></i> 历史追问记录</div>
            <div class="follow-up-history-list">
              {#each entry.follow_ups as fu}
                <article class="surface-card fu-item">
                  <div class="fu-q">Q: {fu.question}</div>
                  <div class="markdown-compact fu-a">
                    {@html renderMarkdownHtml(fu.answer)}
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
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(4px);
    z-index: 1000;
    display: flex;
    justify-content: flex-end;
  }

  .drawer-panel {
    width: 100%;
    max-width: 800px;
    height: 100%;
    background: var(--bg-color);
    box-shadow: -10px 0 30px rgba(0, 0, 0, 0.1);
    display: flex;
    flex-direction: column;
    position: relative;
    padding: 3rem 2rem;
    overflow-y: auto;
    scrollbar-gutter: stable;
  }

  .detail-close-floating {
    position: fixed;
    top: 1.5rem;
    right: 1.5rem;
    width: 3rem;
    height: 3rem;
    border-radius: 50%;
    background: var(--card-bg);
    border: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.5rem;
    cursor: pointer;
    box-shadow: var(--shadow-sm);
  }

  .drawer-head { margin-bottom: 2.5rem; }
  .word-title { font-size: 2.5rem; font-weight: 800; margin-bottom: 0.75rem; color: var(--text-main); }
  .drawer-meta { display: flex; flex-wrap: wrap; gap: 0.5rem; }

  /* Branch UI 专属样式 */
  .drawer-branch-list {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-top: 1rem;
  }
  .drawer-branch-group-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 1.25rem;
    width: 100%;
  }
  .drawer-branch-item {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.25rem 0.5rem;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    transition: all 0.2s ease;
    cursor: pointer;
    margin-left: -0.5rem;
  }
  .drawer-branch-item:hover {
    background: var(--btn-secondary);
    transform: translateX(4px);
  }
  .branch-info-icon {
    font-size: 0.85rem;
    color: var(--text-muted);
    opacity: 0;
    transition: opacity 0.2s ease;
  }
  .drawer-branch-item:hover .branch-info-icon {
    color: var(--accent-main);
    opacity: 1;
  }
  .pos-badge { background: var(--accent-main); color: white; padding: 0.1rem 0.4rem; border-radius: 4px; font-size: 0.75rem; font-weight: 800; }
  .pos-badge.compact {
    min-width: 2rem;
    text-align: center;
    background: var(--text-main);
    color: var(--bg-color);
  }
  .zh-text { font-size: 1.1rem; font-weight: 700; color: var(--text-main); }

  .drawer-content { display: flex; flex-direction: column; gap: 2.5rem; }

  .content-section { display: flex; flex-direction: column; gap: 1rem; }

  .analysis-layout {
    display: grid;
    grid-template-columns: minmax(0, 2fr) minmax(260px, 1fr);
    gap: 1rem;
  }
  .analysis-layout.is-full {
    grid-template-columns: 1fr;
  }

  .analysis-main-card,
  .analysis-side-card {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .section-stack { display: flex; flex-direction: column; gap: 0.85rem; }
  .info-block { display: flex; flex-direction: column; gap: 0.55rem; }
  .usage-copy { margin-top: 0.15rem; }
  .example-box {
    background: var(--bg-color);
    padding: 1rem;
    border-radius: 10px;
    border-left: 3px solid var(--border-color);
  }

  .de-line { font-weight: 700; font-size: 1.02rem; }
  .zh-line { color: var(--text-muted); margin-top: 0.45rem; line-height: 1.6; }
  .insight-html { line-height: 1.75; }
  .network-block { display: flex; flex-direction: column; gap: 0.75rem; }
  .tag-cloud { display: flex; flex-wrap: wrap; gap: 0.5rem; }

  .fu-q { font-weight: 700; color: var(--accent-main); margin-bottom: 0.75rem; font-size: 1.05rem; }
  .fu-a { border-top: 1px solid var(--border-color); padding-top: 0.75rem; margin-top: 0.75rem; }

  .follow-up-history-list { display: flex; flex-direction: column; gap: 1rem; }
  .fu-item { padding: 1.25rem; }

  .debug-section { margin-top: 3rem; border-top: 1px solid var(--border-color); padding-top: 1rem; }
  .debug-section summary { font-size: 0.85rem; color: var(--text-muted); cursor: pointer; padding: 0.5rem 0; }
  .raw-markdown {
    background: var(--btn-secondary);
    padding: 1rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    overflow-x: auto;
    font-family: monospace;
    margin-top: 0.5rem;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .drawer-actions {
    flex-direction: row;
    gap: 1rem;
    margin-top: 2rem;
    padding-top: 2rem;
    border-top: 2px dashed var(--border-color);
  }

  .danger-text { color: var(--danger-text); }

  @media (max-width: 768px) {
    .drawer-panel { padding: 2rem 1rem; }
    .word-title { font-size: 2rem; }
    .drawer-actions { flex-direction: column; }
    .analysis-layout { grid-template-columns: 1fr; }
  }
</style>
