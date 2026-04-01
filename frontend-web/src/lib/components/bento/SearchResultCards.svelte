<script lang="ts">
  import {
    markdownToPlainText,
    parseAnalysisMarkdown,
    renderMarkdownHtml,
  } from "$lib/analysis/structuredAnalysis";
  import FollowUpCard from "$lib/components/bento/FollowUpCard.svelte";
  import type { AnalyzeResponse, AttachedPhraseModule, FollowUpItem, PhraseUsageModule, QualityMode, RecentItem } from "$lib/types";
  import { slide, fade } from "svelte/transition";

  type StructuredAnalysis = ReturnType<typeof parseAnalysisMarkdown>;
  type AttachedPhraseBlock = AttachedPhraseModule & { structured: StructuredAnalysis };

  let { 
    result, 
    isStreaming = false, 
    isAddingToLearning = false, 
    isUpdatingPhraseAttachment = false,
    recentItems = [], 
    onAddToLearning,
    onRegenerate,
    onSelectRecent,
    onSelectPhraseHost,
    onDetachAttachedPhrase,
    onnewFollowUp 
  } = $props<{
    result: AnalyzeResponse;
    isStreaming?: boolean;
    isAddingToLearning?: boolean;
    isUpdatingPhraseAttachment?: boolean;
    recentItems?: RecentItem[];
    onAddToLearning: () => void;
    onRegenerate: (mode: QualityMode, hint: string) => void;
    onSelectRecent: (query: string) => void;
    onSelectPhraseHost?: (headword: string, mode?: "attach" | "view") => void;
    onDetachAttachedPhrase?: (item: AttachedPhraseModule) => void;
    onnewFollowUp?: (item: FollowUpItem) => void;
  }>();

  let showRegenerateHint = $state(false);
  let regenerateHint = $state("");
  let pendingHostHeadword = $state<string | null>(null);

  // 严格同步：使用刚才基于真实数据库样本修复的解析逻辑
  const structured = $derived(parseAnalysisMarkdown(result.analysis_markdown || "", result.query_text));
  const phrasePreview = $derived(result.phrase_usage_preview ?? null);
  const isPhrasePreview = $derived(Boolean(result.phrase_lookup && phrasePreview));
  const attachedPhraseBlocks = $derived(
    (result.attached_phrase_modules ?? []).map((item: AttachedPhraseModule) => ({
      ...item,
      structured: parseAnalysisMarkdown(item.analysis_markdown || "", item.phrase)
    }))
  );
  const usageFeed = $derived([
    ...(phrasePreview
      ? [
          {
            kind: "phrase_preview" as const,
            title: phrasePreview.usage_module.title,
            explanation: phrasePreview.usage_module.explanation,
            example: {
              de: phrasePreview.usage_module.example_de,
              zh: phrasePreview.usage_module.example_zh
            }
          }
        ]
      : structured.usageModules.map((usage) => ({
          kind: "base" as const,
          title: usage.title,
          explanation: usage.explanation,
          example: usage.example
        }))),
    ...attachedPhraseBlocks.map((attachment: AttachedPhraseBlock) => {
      const fallbackUsage = attachment.structured.usageModules[0];
      const usage = attachment.usage_module
        ? {
            title: attachment.usage_module.title,
            explanation: attachment.usage_module.explanation,
            example: {
              de: attachment.usage_module.example_de,
              zh: attachment.usage_module.example_zh
            }
          }
        : fallbackUsage;

      return {
        kind: "attached" as const,
        attachment,
        title: usage?.title ?? attachment.phrase,
        explanation: usage?.explanation ?? "",
        example: usage?.example,
        fallbackHtml:
          !attachment.usage_module && attachment.structured.usageModules.length === 0
            ? renderMarkdownHtml(attachment.analysis_markdown)
            : ""
      };
    })
  ]);
  
  const hasMeanings = $derived(structured.meanings.length > 0);
  const mainMeaning = $derived(
    phrasePreview
      ? {
          partOfSpeech: "短语",
          chinese: phrasePreview.meaning_zh,
          english: ""
        }
      : structured.meanings[0]
  );
  const deepInsights = $derived(isPhrasePreview ? [] : structured.deepInsights);
  const needsHostConfirmation = $derived(
    Boolean(result.phrase_lookup && result.phrase_lookup.confidence !== "high")
  );

  $effect(() => {
    result.entry_id;
    result.query_text;
    pendingHostHeadword = null;
  });

  function getSourceClass(source?: string) {
    if (!source) return "source-ai";
    const s = source.toLowerCase();
    if (s.includes("db") || s.includes("数据")) return "source-db";
    if (s.includes("pro")) return "source-pro";
    return "source-flash";
  }

  function emitFollowUp(item: FollowUpItem) {
    if (onnewFollowUp) onnewFollowUp(item);
  }

  function confidenceLabel(value?: "high" | "medium" | "low" | null) {
    if (value === "high") return "高置信";
    if (value === "medium") return "中等置信";
    return "待确认";
  }

  function phraseActionHint(value?: "high" | "medium" | "low" | null) {
    if (value === "high") return "点击候选后会直接挂载当前短语，并切换到对应主词结果。";
    if (value === "medium") return "当前候选有一定把握。点击候选后会先进入确认，再决定是仅查看主词，还是正式挂载。";
    return "当前候选偏弱。点击候选后不会立刻写库，而是先让你确认主词是否合适。";
  }

  function handleCandidateClick(headword: string) {
    if (needsHostConfirmation) {
      pendingHostHeadword = headword;
      return;
    }
    onSelectPhraseHost?.(headword, "attach");
  }

  function clearPendingHost() {
    pendingHostHeadword = null;
  }
</script>

<div class="bento-grid">
  <!-- 精简后的 Header: 修复比例与音标 -->
  <div class="bento-card card-header">
    <div class="header-main-info">
      <div class="title-row">
        <h1 class="word-title">{structured.headword || result.query_text}</h1>
        {#if structured.phonetic}
          <span class="word-phonetic">{structured.phonetic}</span>
        {/if}
      </div>
      
      <div class="meaning-row">
        {#if mainMeaning}
          <span class="pos-badge">{mainMeaning.partOfSpeech}</span>
          <span class="zh-text">{mainMeaning.chinese}</span>
          {#if !phrasePreview && mainMeaning.english}
            <span class="en-text">{mainMeaning.english}</span>
          {/if}
        {/if}
        {#if phrasePreview?.meaning_en}
          <span class="word-phonetic">{phrasePreview.meaning_en}</span>
        {/if}
      </div>

      <div class="meta-row">
        <span class="mini-label {getSourceClass(result.source)}">
          {result.source || "AI"}
        </span>
        {#if result.model}
          <span class="mini-label model-name">{result.model}</span>
        {/if}
      </div>
    </div>
    
    <div class="header-actions">
      <div class="action-icons">
        <button class="icon-btn" onclick={() => onRegenerate("default", regenerateHint)} disabled={isStreaming} title="重新生成">
          <i class="ph ph-arrows-clockwise"></i>
        </button>
        <button class="icon-btn pro-btn" onclick={() => onRegenerate("pro", regenerateHint)} disabled={isStreaming} title="Pro 增强模式">
          <i class="ph-fill ph-lightning"></i>
        </button>
        <button class="icon-btn" onclick={() => showRegenerateHint = !showRegenerateHint} disabled={isStreaming} title="提示词调整">
          <i class="ph ph-sliders-horizontal"></i>
        </button>
      </div>
      <button class="btn-primary learn-btn" onclick={onAddToLearning} disabled={result.entry_id <= 0 || isStreaming || isAddingToLearning}>
        <i class="ph-fill ph-star"></i>
        <span>{isAddingToLearning ? "..." : "加入学习"}</span>
      </button>
    </div>
  </div>

  {#if result.phrase_lookup}
    <div class="bento-card card-full phrase-host-card">
      <div class="card-title"><i class="ph-fill ph-git-branch"></i> 短语宿主候选</div>
      <div class="phrase-meta">
        <span class="mini-label source-db">短语解析</span>
        <span class="mini-label model-name">{confidenceLabel(result.phrase_lookup.confidence)}</span>
        {#if result.phrase_lookup.best_host_headword}
          <span class="card-copy">当前优先关联到 <strong>{result.phrase_lookup.best_host_headword}</strong></span>
        {:else}
          <span class="card-copy">当前还没有足够稳的主词，请手动选择查看。</span>
        {/if}
      </div>
      <p class="card-copy phrase-action-hint">{phraseActionHint(result.phrase_lookup.confidence)}</p>
      {#if result.phrase_lookup.host_candidates.length > 0}
        <div class="host-chip-list">
          {#each result.phrase_lookup.host_candidates as candidate}
            <button
              class="host-chip"
              onclick={() => handleCandidateClick(candidate.headword)}
              disabled={isUpdatingPhraseAttachment}
              title={`来源：${candidate.source}`}
              class:is-pending={pendingHostHeadword === candidate.headword}
            >
              <strong>{candidate.headword}</strong>
              <span>{candidate.source}</span>
            </button>
          {/each}
        </div>
      {/if}

      {#if pendingHostHeadword}
        <div class="surface-card phrase-confirm-panel" transition:slide>
          <div class="phrase-confirm-head">
            <p class="small-copy"><strong>确认宿主主词：{pendingHostHeadword}</strong></p>
            <span class="mini-label model-name">{confidenceLabel(result.phrase_lookup.confidence)}</span>
          </div>
          <p class="card-copy">
            你可以先只查看 <strong>{pendingHostHeadword}</strong> 的词条，不写入短语挂载；确认合适后，再把当前短语正式挂到这个主词下面。
          </p>
          <div class="phrase-confirm-actions">
            <button
              class="btn-secondary confirm-btn"
              type="button"
              onclick={() => onSelectPhraseHost?.(pendingHostHeadword, "view")}
              disabled={isUpdatingPhraseAttachment}
            >
              仅查看主词
            </button>
            <button
              class="btn-primary confirm-btn"
              type="button"
              onclick={() => onSelectPhraseHost?.(pendingHostHeadword, "attach")}
              disabled={isUpdatingPhraseAttachment}
            >
              挂载到主词
            </button>
            <button
              class="icon-btn ghost-close-btn"
              type="button"
              onclick={clearPendingHost}
              disabled={isUpdatingPhraseAttachment}
              title="取消"
            >
              <i class="ph ph-x"></i>
            </button>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  {#if showRegenerateHint}
    <div class="bento-card card-full" transition:slide>
      <div class="card-title"><i class="ph-fill ph-sliders-horizontal"></i> 本次生成要求</div>
      <textarea
        bind:value={regenerateHint}
        class="compact-textarea"
        placeholder="例如：多讲固定搭配；例句更长一点..."
        disabled={isStreaming}
      ></textarea>
    </div>
  {/if}

  <!-- 应用与例句: 结构化展示 -->
  <div class="bento-card card-main">
    <div class="card-title"><i class="ph-fill ph-lightbulb"></i> 应用与例句</div>
    <div class="usage-list">
      {#if usageFeed.length > 0}
        {#each usageFeed as usage}
          <div class="surface-card usage-item">
            <div class="usage-head">
              <p class="small-copy"><strong>{usage.title}</strong></p>
              {#if usage.kind === "attached"}
                <div class="attached-phrase-actions">
                  <span class="mini-label source-flash">短语追加</span>
                  <span class="mini-label model-name">{usage.attachment.phrase}</span>
                  <button
                    class="mini-inline-btn"
                    type="button"
                    onclick={() => onDetachAttachedPhrase?.(usage.attachment)}
                    disabled={isUpdatingPhraseAttachment || isStreaming}
                    title="移除这条短语追加"
                  >
                    <i class="ph ph-x"></i>
                  </button>
                </div>
              {/if}
            </div>
            {#if usage.explanation}
              <p class="card-copy usage-explanation">{usage.explanation}</p>
            {/if}
            {#if usage.example?.de}
              <div class="example-box">
                <div class="de-line">{usage.example.de}</div>
                {#if usage.example.zh}
                  <div class="zh-line">{usage.example.zh}</div>
                {/if}
              </div>
            {:else if usage.kind === "attached" && usage.fallbackHtml}
              <div class="markdown-compact">
                {@html usage.fallbackHtml}
              </div>
            {/if}
          </div>
        {/each}
      {:else if structured.examples.length > 0}
        {#each structured.examples as example}
          <div class="surface-card example-item">
            <div class="de-line">{example.de}</div>
            {#if example.zh}
              <div class="zh-line">{example.zh}</div>
            {/if}
          </div>
        {/each}
      {:else if !isStreaming}
        <div class="message-surface muted">
          <p>详细分析已放入下方深度解析卡片。</p>
        </div>
      {/if}

    </div>
  </div>

  <!-- 语法特性 -->
  <div class="bento-card card-side">
    <div class="card-title"><i class="ph-fill ph-text-aa"></i> 语法特性</div>
    <div class="grammar-list-compact">
      {#if structured.grammarRows.length > 0}
        {#each structured.grammarRows as row}
          <div class="g-row">
            <span class="g-key">{row.key}</span>
            <span class="g-val">{row.value}</span>
          </div>
        {/each}
      {:else}
        <div class="empty-msg">{isStreaming ? "解析中..." : "未提取到表格"}</div>
      {/if}
    </div>
  </div>

  <!-- 深度解析: 严格使用修复后的 deepInsights -->
  <div class="bento-card card-full">
    <div class="card-title"><i class="ph-fill ph-brain"></i> 深度解析与避坑</div>
    <div class="insights-stack">
      {#if deepInsights.length > 0}
        {#each deepInsights as insight}
          <div class="surface-card insight-block">
            <p class="insight-label"><strong>{insight.title}</strong></p>
            <div class="markdown-compact">
              {@html insight.html}
            </div>
          </div>
        {/each}
      {:else if isPhrasePreview || result.phrase_lookup}
        <div class="message-surface muted">
          <p>当前展示的是短语预览，重点信息已经放在“应用与例句”里。</p>
        </div>
      {:else if !isStreaming}
        <div class="markdown-compact surface-card">
          {@html renderMarkdownHtml(result.analysis_markdown)}
        </div>
      {/if}
    </div>

    {#if structured.family.length > 0 || structured.synonyms.length > 0 || structured.antonyms.length > 0}
      <div class="tag-cloud-mini">
        {#each [...structured.family, ...structured.synonyms, ...structured.antonyms] as item}
          <span class="mini-tag-btn">{item}</span>
        {/each}
      </div>
    {/if}
  </div>

  <!-- 追问 -->
  <div class="bento-card card-main">
    <div class="card-title"><i class="ph-fill ph-chat-circle-dots"></i> 追问与联想</div>
    {#if result.entry_id > 0}
      <div class="follow-up-wrapper surface-card">
        <FollowUpCard
          entryId={result.entry_id}
          history={result.follow_ups}
          disabled={isStreaming}
          onnewFollowUp={emitFollowUp}
        />
      </div>
    {:else}
      <div class="message-surface muted">
        <p>结果暂未写入词库。</p>
      </div>
    {/if}
  </div>

  <!-- 侧边历史 -->
  <div class="bento-card card-side">
    <div class="card-title"><i class="ph-fill ph-clock-counter-clockwise"></i> 最近查询</div>
    <div class="recent-mini-list">
      {#each recentItems.slice(0, 5) as item}
        <button class="recent-mini-btn" onclick={() => onSelectRecent(item.query_text)}>
          <strong>{item.query_text}</strong>
          <span class="p-text">{item.preview}</span>
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .bento-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1.25rem; width: 100%; margin-bottom: 4rem; }

  /* Header 优化 */
  .card-header { 
    grid-column: span 3; display: flex; justify-content: space-between; align-items: flex-end; 
    padding: 1.5rem 2rem;
  }
  .header-main-info { display: flex; flex-direction: column; gap: 0.4rem; }
  
  .title-row { display: flex; align-items: baseline; gap: 0.8rem; }
  .word-title { font-size: 2.2rem; font-weight: 800; color: var(--text-main); letter-spacing: -0.02em; margin: 0; }
  
  /* 音标专用字体栈修复：强制使用标准 IPA 支持字体 */
  .word-phonetic { 
    font-family: "Arial Unicode MS", "Lucida Sans Unicode", sans-serif;
    color: var(--text-muted); font-size: 1.1rem; line-height: 1.6;
    background: var(--bg-color); padding: 0.1rem 0.4rem; border-radius: 4px;
  }

  .meaning-row { display: flex; align-items: center; gap: 0.6rem; margin-top: 0.2rem; }
  .pos-badge { background: var(--accent-main); color: white; padding: 0.1rem 0.4rem; border-radius: 4px; font-size: 0.75rem; font-weight: 800; }
  .zh-text { font-size: 1.1rem; font-weight: 700; color: var(--text-main); }
  .en-text { font-size: 0.98rem; font-style: italic; color: var(--text-muted); }

  .meta-row { display: flex; gap: 0.5rem; margin-top: 0.4rem; }
  .mini-label { font-size: 0.65rem; font-weight: 800; padding: 0.15rem 0.4rem; border-radius: 4px; text-transform: uppercase; opacity: 0.7; }
  .source-db { background: #e0f2fe; color: #0369a1; }
  .source-flash { background: #dcfce7; color: #15803d; }
  .source-pro { background: #f5f3ff; color: #6d28d9; }
  .model-name { background: var(--bg-color); color: var(--text-muted); border: 1px solid var(--border-color); }

  .header-actions { display: flex; flex-direction: column; align-items: flex-end; gap: 0.75rem; }
  .action-icons { display: flex; gap: 0.5rem; }
  .icon-btn { 
    width: 2.2rem; height: 2.2rem; border-radius: 50%; background: var(--btn-secondary); 
    color: var(--text-main); display: flex; align-items: center; justify-content: center; font-size: 1rem;
  }
  .pro-btn { color: #6d28d9; }
  .learn-btn { padding: 0.5rem 1rem; font-size: 0.85rem; }

  /* 其它卡片微调 */
  .card-main { grid-column: span 2; display: flex; flex-direction: column; gap: 1rem; }
  .card-side { grid-column: span 1; display: flex; flex-direction: column; gap: 1rem; }
  .card-full { grid-column: span 3; }

  .usage-list, .insights-stack { display: flex; flex-direction: column; gap: 1rem; }
  .usage-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
  .example-box { background: var(--bg-color); padding: 1rem; border-radius: 8px; border-left: 3px solid var(--border-color); }
  .de-line { font-weight: 700; font-size: 1.05rem; }
  .zh-line { color: var(--text-muted); font-size: 0.9rem; margin-top: 0.4rem; }

  .grammar-list-compact { display: flex; flex-direction: column; }
  .g-row { display: flex; justify-content: space-between; padding: 0.6rem 0; border-bottom: 1px solid var(--border-color); font-size: 0.9rem; }
  .g-key { color: var(--text-muted); }
  .g-val { font-weight: 700; color: var(--accent-main); }

  .tag-cloud-mini { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: 1.5rem; }
  .mini-tag-btn { font-size: 0.8rem; padding: 0.25rem 0.6rem; background: var(--bg-color); border-radius: 6px; border: 1px solid var(--border-color); }

  .recent-mini-list { display: flex; flex-direction: column; gap: 0.6rem; }
  .recent-mini-btn { 
    width: 100%; padding: 0.75rem; text-align: left; background: var(--bg-color); border-radius: 8px; 
    display: flex; flex-direction: column; border: 1px solid transparent;
  }
  .recent-mini-btn:hover { border-color: var(--accent-main); }
  .recent-mini-btn .p-text { font-size: 0.8rem; color: var(--text-muted); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }

  .compact-textarea { width: 100%; min-height: 4rem; padding: 0.75rem; border-radius: 8px; background: var(--bg-color); border: 1px solid var(--border-color); font-size: 0.9rem; }
  .phrase-host-card { display: flex; flex-direction: column; gap: 0.9rem; }
  .phrase-meta { display: flex; align-items: center; gap: 0.6rem; flex-wrap: wrap; }
  .host-chip-list { display: flex; flex-wrap: wrap; gap: 0.75rem; }
  .phrase-action-hint { margin: 0; color: var(--text-muted); }
  .host-chip {
    background: var(--bg-color);
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 0.55rem 0.9rem;
    display: flex;
    align-items: baseline;
    gap: 0.45rem;
    color: var(--text-main);
  }
  .host-chip span { font-size: 0.78rem; color: var(--text-muted); }
  .host-chip.is-pending {
    border-color: var(--accent-main);
    box-shadow: inset 0 0 0 1px var(--accent-main);
  }
  .phrase-confirm-panel {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
    border: 1px solid var(--border-color);
  }
  .phrase-confirm-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .phrase-confirm-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .confirm-btn {
    min-height: 2.4rem;
    padding: 0.55rem 1rem;
  }
  .ghost-close-btn {
    background: transparent;
    border: 1px solid var(--border-color);
  }
  .attached-phrase-actions { display: flex; align-items: center; gap: 0.5rem; }
  .mini-inline-btn {
    width: 1.9rem;
    height: 1.9rem;
    border-radius: 999px;
    background: transparent;
    color: var(--text-muted);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .mini-inline-btn:hover:not(:disabled) {
    background: var(--btn-secondary);
    color: var(--text-main);
  }
  .mini-inline-btn:disabled,
  .host-chip:disabled {
    opacity: 0.55;
    cursor: wait;
  }

  @media (max-width: 900px) {
    .bento-grid { grid-template-columns: 1fr; }
    .card-header, .card-main, .card-side, .card-full { grid-column: span 1; }
    .card-header { flex-direction: row; align-items: center; padding: 1rem; }
    .header-actions { flex-direction: row; }
  }
</style>
