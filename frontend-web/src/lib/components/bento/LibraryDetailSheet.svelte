<script lang="ts">
  import {
    parseAnalysisMarkdown,
    renderMarkdownHtml,
  } from "$lib/analysis/structuredAnalysis";
  import type { EntryDetailResponse } from "$lib/types";

  export let detail: EntryDetailResponse | null = null;
  export let isLoading = false;
  export let error = "";
  export let isDeleting = false;
  export let onClose: () => void = () => {};
  export let onOpenAnalysis: () => void = () => {};
  export let onDelete: () => void = () => {};

  function formatDateTime(value: string) {
    return new Intl.DateTimeFormat("zh-CN", {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(value));
  }

  $: structured = detail ? parseAnalysisMarkdown(detail.analysis_markdown, detail.query_text) : null;
  $: primaryMeaning = structured?.meanings[0] ?? null;
  let showRawMarkdown = false;
  $: if (detail) {
    showRawMarkdown = false;
  }
</script>

<svelte:window on:keydown={(event) => event.key === "Escape" && onClose()} />

<div class="detail-overlay" role="presentation" on:click={(event) => event.currentTarget === event.target && onClose()}>
  <button class="detail-close-floating" type="button" aria-label="关闭词条详情" on:click={onClose}>
    <i class="ph ph-x"></i>
  </button>
  <div class="drawer-panel" role="dialog" aria-modal="true" aria-label="词条详情">
    <div class="drawer-head">
      <div>
        <div class="word-title" style="font-size:2rem;">{detail?.query_text ?? "词条详情"}</div>
        {#if structured?.phonetic}
          <div class="word-phonetic">{structured.phonetic}</div>
        {/if}
      </div>
    </div>

    {#if isLoading}
      <div class="surface-card">
        <div class="skeleton-block"></div>
        <div class="skeleton-block" style="margin-top:0.8rem;"></div>
        <div class="skeleton-block" style="margin-top:0.8rem; width:60%;"></div>
      </div>
    {:else if error}
      <div class="message-surface error">{error}</div>
    {:else if detail}
      <div class="drawer-meta">
        <span class="meta-pill"><i class="ph ph-hash"></i>ID {detail.entry_id}</span>
        <span class="meta-pill"><i class="ph ph-database"></i>{detail.source}</span>
        <span class="meta-pill"><i class="ph ph-clock"></i>{formatDateTime(detail.updated_at)}</span>
      </div>

      {#if detail.tags.length > 0}
        <div style="margin-bottom:1rem;">
          {#each detail.tags as tag}
            <span class="tag">{tag}</span>
          {/each}
        </div>
      {/if}

      {#if detail.aliases.length > 0}
        <div class="surface-card" style="margin-bottom:1rem;">
          <p class="small-copy"><strong>关联写法</strong></p>
          <p class="card-copy">{detail.aliases.join(" / ")}</p>
        </div>
      {/if}

      <div class="bento-grid">
        <div class="bento-card card-main">
          <div class="card-title"><i class="ph-fill ph-lightbulb"></i> 核心义项与用法</div>
          {#if primaryMeaning}
            <div class="surface-card" style="margin-bottom:1rem;">
              <p class="small-copy">核心义项</p>
              <p class="card-copy">
                <strong>{primaryMeaning.partOfSpeech}</strong> {primaryMeaning.chinese}
                {#if primaryMeaning.english}
                  <span class="muted-copy"> · {primaryMeaning.english}</span>
                {/if}
              </p>
            </div>
          {/if}

          {#if structured?.usageModules.length}
            {#each structured.usageModules as usage}
              <div class="surface-card" style="margin-bottom:0.85rem;">
                <p class="small-copy"><strong>{usage.title}</strong></p>
                {#if usage.explanation}
                  <p class="card-copy" style="margin-top:0.4rem;">{usage.explanation}</p>
                {/if}
                {#if usage.example?.de}
                  <div class="example-item" style="margin-top:0.75rem;">
                    <div class="de-line">{usage.example.de}</div>
                    {#if usage.example.zh}
                      <div class="zh-line" style="margin-top:0.4rem;">{usage.example.zh}</div>
                    {/if}
                  </div>
                {/if}
              </div>
            {/each}
          {:else if structured?.examples.length}
            {#each structured.examples as example}
              <div class="surface-card" style="margin-bottom:0.85rem;">
                <div class="de-line">{example.de}</div>
                {#if example.zh}
                  <div class="zh-line" style="margin-top:0.4rem;">{example.zh}</div>
                {/if}
              </div>
            {/each}
          {:else}
            <p class="small-copy">当前词条没有提取到独立例句。</p>
          {/if}
        </div>
        <div class="bento-card card-side">
          <div class="card-title"><i class="ph-fill ph-text-aa"></i> 语法摘要</div>
          {#if structured?.grammarRows.length}
            {#each structured.grammarRows as row}
              <div class="grammar-row">
                <span>{row.key}</span>
                <span class="g-val">{row.value}</span>
              </div>
            {/each}
          {:else}
            <p class="small-copy">当前词条没有结构化语法表。</p>
          {/if}

          {#if structured?.family.length || structured?.synonyms.length || structured?.antonyms.length}
            <div class="card-title" style="margin-top:1.25rem;"><i class="ph-fill ph-share-network"></i> 词汇网络</div>
            <div>
              {#each [...(structured?.family ?? []), ...(structured?.synonyms ?? []), ...(structured?.antonyms ?? [])] as item}
                <span class="tag">{item}</span>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      {#if structured?.deepInsights.length}
        <div class="surface-card" style="margin-top:1rem;">
          <div class="card-title"><i class="ph-fill ph-brain"></i> 深度解析与避坑</div>
          {#each structured.deepInsights as insight}
            <div class="surface-card" style="margin-bottom:0.85rem;">
              <p class="small-copy"><strong>{insight.title}</strong></p>
              <div class="markdown-compact">{@html insight.html}</div>
            </div>
          {/each}
        </div>
      {/if}

      <div class="surface-card" style="margin-top:1rem;">
        <div class="card-title"><i class="ph-fill ph-code-block"></i> 调试视图</div>
        <div class="inline-actions">
          <button class="btn-secondary" type="button" on:click={() => (showRawMarkdown = !showRawMarkdown)}>
            <i class={`ph ${showRawMarkdown ? "ph-caret-up" : "ph-caret-down"}`}></i>
            {showRawMarkdown ? "收起完整 Markdown" : "展开完整 Markdown"}
          </button>
        </div>
        {#if showRawMarkdown}
          <div class="markdown-compact" style="margin-top:0.85rem;">{@html renderMarkdownHtml(detail.analysis_markdown)}</div>
        {/if}
      </div>

      {#if detail.follow_ups.length > 0}
        <div class="surface-card" style="margin-top:1rem;">
          <div class="card-title"><i class="ph-fill ph-chat-centered-text"></i> 追问记录</div>
          <div class="follow-up-history">
            {#each detail.follow_ups as item (item.id)}
              <article>
                <p><strong>你问：</strong>{item.question}</p>
                <div class="markdown-compact">{@html renderMarkdownHtml(item.answer)}</div>
              </article>
            {/each}
          </div>
        </div>
      {/if}

      <div class="inline-actions" style="margin-top:1rem;">
        <button class="btn-secondary" type="button" on:click={onOpenAnalysis}>
          <i class="ph ph-magnifying-glass"></i> 回到查词页
        </button>
        <button class="btn-primary" type="button" on:click={onDelete} disabled={isDeleting}>
          <i class="ph ph-trash"></i> {isDeleting ? "删除中..." : "删除词条"}
        </button>
      </div>
    {/if}
  </div>
</div>
