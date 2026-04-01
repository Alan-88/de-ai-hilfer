<script lang="ts">
  import { fade, slide } from "svelte/transition";
  import {
    parseAnalysisMarkdown,
    renderMarkdownHtml,
  } from "$lib/analysis/structuredAnalysis";
  import type { EntryDetailResponse } from "$lib/types";

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

  const structured = $derived(entry ? parseAnalysisMarkdown(entry.analysis_markdown, entry.query_text) : null);
  const primaryMeaning = $derived(structured?.meanings[0] ?? null);
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
          <div class="drawer-meta">
            <span class="meta-pill">#{entry.entry_id}</span>
            <span class="meta-pill">{entry.entry_type}</span>
            <span class="meta-pill"><i class="ph ph-clock"></i> {formatDateTime(entry.updated_at)}</span>
          </div>
        </div>
      </header>

      <div class="drawer-content">
        <section class="content-section">
          <div class="card-title"><i class="ph-fill ph-article"></i> 完整分析内容</div>
          <div class="analysis-layout">
            <div class="surface-card analysis-main-card">
              {#if primaryMeaning}
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
                        <p class="card-copy usage-copy">{usage.explanation}</p>
                      {/if}
                      {#if usage.example?.de}
                        <div class="example-box">
                          <div class="de-line">{usage.example.de}</div>
                          {#if usage.example.zh}
                            <div class="zh-line">{usage.example.zh}</div>
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
                      <div class="de-line">{example.de}</div>
                      {#if example.zh}
                        <div class="zh-line">{example.zh}</div>
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

            <div class="surface-card analysis-side-card">
              <div class="small-copy"><strong>语法摘要</strong></div>
              {#if structured?.grammarRows.length}
                <div class="grammar-list">
                  {#each structured.grammarRows as row}
                    <div class="grammar-row">
                      <span>{row.key}</span>
                      <span class="g-val">{row.value}</span>
                    </div>
                  {/each}
                </div>
              {:else}
                <p class="muted-copy">当前词条没有结构化语法表。</p>
              {/if}

              {#if structured?.family.length || structured?.synonyms.length || structured?.antonyms.length}
                <div class="network-block">
                  <div class="small-copy"><strong>词汇网络</strong></div>
                  <div class="tag-cloud">
                    {#each [...(structured?.family ?? []), ...(structured?.synonyms ?? []), ...(structured?.antonyms ?? [])] as item}
                      <span class="tag">{item}</span>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
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
  }

  .detail-close-floating {
    position: absolute;
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

  .drawer-content { display: flex; flex-direction: column; gap: 2.5rem; }
  
  .content-section { display: flex; flex-direction: column; gap: 1rem; }

  .analysis-layout {
    display: grid;
    grid-template-columns: minmax(0, 2fr) minmax(260px, 1fr);
    gap: 1rem;
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
  .grammar-list { display: flex; flex-direction: column; gap: 0.2rem; }
  .grammar-row {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.65rem 0;
    border-bottom: 1px solid var(--border-color);
  }
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
