<script lang="ts">
  import {
    markdownToPlainText,
    parseAnalysisMarkdown,
    renderMarkdownHtml,
    resolveStructuredAnalysis,
  } from "$lib/analysis/structuredAnalysis";
  import FollowUpCard from "$lib/components/bento/FollowUpCard.svelte";
  import type { AiModelOverride, AnalyzeResponse, AttachedPhraseModule, FollowUpItem, PhraseUsageModule, QualityMode, RecentItem } from "$lib/types";
  import { slide } from "svelte/transition";
  import GrammarFeatureCard from "$lib/components/bento/GrammarFeatureCard.svelte";
  import GrammarBranchPopover from "$lib/components/bento/GrammarBranchPopover.svelte";
  import PhraseModuleAddControl from "$lib/components/bento/PhraseModuleAddControl.svelte";

  type StructuredAnalysis = ReturnType<typeof resolveStructuredAnalysis>;
  type AttachedPhraseBlock = AttachedPhraseModule & { structured: StructuredAnalysis };
  type ActionModelOption = AiModelOverride & {
    key: string;
    label: string;
  };

  let {
    result,
    isStreaming = false,
    isAddingToLearning = false,
    isUpdatingPhraseAttachment = false,
    isAddingPhraseModule = false,
    recentItems = [],
    actionModelOptions = [],
    selectedActionModelKey = "",
    selectedActionModelOverride = null,
    onAddToLearning,
    onRegenerate,
    onActionModelChange,
    onSelectRecent,
    onDeleteAttachedPhrase,
    onAddPhraseModule,
    onnewFollowUp
  } = $props<{
    result: AnalyzeResponse;
    isStreaming?: boolean;
    isAddingToLearning?: boolean;
    isUpdatingPhraseAttachment?: boolean;
    isAddingPhraseModule?: boolean;
    recentItems?: RecentItem[];
    actionModelOptions?: ActionModelOption[];
    selectedActionModelKey?: string;
    selectedActionModelOverride?: AiModelOverride | null;
    onAddToLearning: () => void;
    onRegenerate: (mode: QualityMode, hint: string, modelOverride?: AiModelOverride | null) => void;
    onActionModelChange?: (key: string) => void;
    onSelectRecent: (query: string) => void;
    onDeleteAttachedPhrase?: (item: AttachedPhraseModule) => void;
    onAddPhraseModule?: (phrase: string) => Promise<void> | void;
    onnewFollowUp?: (item: FollowUpItem) => void;
  }>();

  let showCustomRegenerate = $state(false);
  let customRegenerateHint = $state("");
  let activeGrammarBranch = $state<import("$lib/types").GrammarBranch | null>(null);
  let popoverTriggerRect = $state<DOMRect | null>(null);

  function formatPos(branch: import("$lib/types").GrammarBranch): string {
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
    const groups: (import("$lib/types").GrammarBranch[])[] = [];
    if (!structured.grammarBranches) return groups;

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

  // 严格同步：使用刚才基于真实数据库样本修复的解析逻辑
  const structured = $derived(
    resolveStructuredAnalysis(
      result.analysis_markdown || "",
      result.structured_analysis,
      result.query_text
    )
  );
  const useBranchUI = $derived((structured.grammarBranches?.length ?? 0) > 0);
  const attachedPhraseBlocks = $derived(
    (result.attached_phrase_modules ?? []).map((item: AttachedPhraseModule) => ({
      ...item,
      structured: parseAnalysisMarkdown(item.analysis_markdown || "", item.phrase)
    }))
  );
  const usageFeed = $derived([
    ...structured.usageModules.map((usage) => ({
      kind: "base" as const,
      title: usage.title,
      explanation: usage.explanation,
      example: usage.example
    })),
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
  const mainMeaning = $derived(structured.meanings[0]);
  const deepInsights = $derived(structured.deepInsights);
  const actionProviderOptions = $derived(
    Array.from(new Map(actionModelOptions.map((option: ActionModelOption) => [option.provider_name, option.provider_name])).values())
  );
  const selectedActionProviderName = $derived(
    actionModelOptions.find((option: ActionModelOption) => option.key === selectedActionModelKey)?.provider_name
      ?? actionProviderOptions[0]
      ?? ""
  );
  const selectedProviderModelOptions = $derived(
    actionModelOptions.filter((option: ActionModelOption) => option.provider_name === selectedActionProviderName)
  );
  const displayedModelName = $derived(displayModelName(result.model));
  const shouldShowSourceBadge = $derived(shouldDisplaySourceBadge(result.source, result.model));

  $effect(() => {
    result.entry_id;
    result.query_text;
  });

  function getSourceClass(source?: string) {
    if (!source) return "source-ai";
    const s = source.toLowerCase();
    if (s.includes("db") || s.includes("数据")) return "source-db";
    if (s.includes("pro")) return "source-pro";
    return "source-flash";
  }

  function displayModelName(model?: string | null) {
    if (!model) return "";
    const trimmed = model.trim();
    return trimmed.split(/\s*\/\s*|\s*:\s*/).filter(Boolean).at(-1) ?? trimmed;
  }

  function shouldDisplaySourceBadge(source?: string | null, model?: string | null) {
    if (!source) return false;
    const s = source.toLowerCase();
    if (model && (s.includes("flash") || s.includes("pro"))) return false;
    return true;
  }

  function firstModelKeyForProvider(providerName: string) {
    return actionModelOptions.find((option: ActionModelOption) => option.provider_name === providerName)?.key ?? "";
  }

  function selectActionProvider(providerName: string) {
    const nextKey = firstModelKeyForProvider(providerName);
    if (nextKey) onActionModelChange?.(nextKey);
  }

  function emitFollowUp(item: FollowUpItem) {
    if (onnewFollowUp) onnewFollowUp(item);
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

      <div class="meaning-row" class:is-branch-list={useBranchUI}>
        {#if useBranchUI}
          {#each branchGroups() as group}
            <div class="branch-group-row">
              {#each group as branch}
                <button
                  class="branch-meaning-item"
                  onclick={(e) => {
                    activeGrammarBranch = branch;
                    popoverTriggerRect = (e.currentTarget as HTMLElement).getBoundingClientRect();
                  }}
                  title="点击查看详细语法"
                >
                  <span class="pos-badge compact">{formatPos(branch)}</span>
                  <span class="zh-text">{formatMeanings(branch.meanings)}</span>
                  <i class="ph ph-info branch-info-icon"></i>
                </button>
              {/each}
            </div>
          {/each}
        {:else}
          {#if mainMeaning}
            <span class="pos-badge">{mainMeaning.partOfSpeech}</span>
            <span class="zh-text">{mainMeaning.chinese}</span>
            {#if mainMeaning.english}
              <span class="en-text">{mainMeaning.english}</span>
            {/if}
          {/if}
        {/if}
      </div>

      <div class="meta-row">
        {#if shouldShowSourceBadge}
          <span class="mini-label {getSourceClass(result.source)}">
            {result.source || "AI"}
          </span>
        {/if}
        {#if displayedModelName}
          <span class="mini-label model-name">{displayedModelName}</span>
        {/if}
        {#if structured.sourceType === "structured"}
          <span class="mini-label source-pro structured-badge">
            <i class="ph-fill ph-database"></i> 结构化
          </span>
        {/if}
      </div>
    </div>

    <div class="header-actions">
      <div class="action-icons">
        <button
          class="icon-btn"
          class:is-custom-action={showCustomRegenerate}
          onclick={() => showCustomRegenerate
            ? onRegenerate("default", customRegenerateHint, selectedActionModelOverride)
            : onRegenerate("default", "", null)}
          disabled={isStreaming}
          title={showCustomRegenerate ? "按本次动作设置重新生成" : "按当前配置重新生成"}
        >
          <i class="ph ph-arrows-clockwise"></i>
        </button>
        <button
          class="icon-btn"
          class:is-active={showCustomRegenerate}
          onclick={() => showCustomRegenerate = !showCustomRegenerate}
          disabled={isStreaming}
          title="本次动作设置"
        >
          <i class="ph ph-sliders-horizontal"></i>
        </button>
      </div>
      <button class="btn-primary learn-btn" onclick={onAddToLearning} disabled={result.entry_id <= 0 || isStreaming || isAddingToLearning}>
        <i class="ph-fill ph-star"></i>
        <span>{isAddingToLearning ? "..." : "加入学习"}</span>
      </button>
    </div>
  </div>

  {#if showCustomRegenerate}
    <div class="bento-card card-full custom-regenerate-card" transition:slide>
      <div class="custom-action-head">
        <div class="card-title"><i class="ph-fill ph-sliders-horizontal"></i> 本次动作设置</div>
        <div class="custom-model-controls">
          <select
            aria-label="渠道"
            title="渠道"
            value={selectedActionProviderName}
            onchange={(event) => selectActionProvider(event.currentTarget.value)}
            disabled={isStreaming || actionProviderOptions.length === 0}
          >
            {#each actionProviderOptions as provider}
              <option value={provider}>{provider}</option>
            {/each}
          </select>
          <select
            aria-label="模型"
            title="模型"
            value={selectedActionModelKey}
            onchange={(event) => onActionModelChange?.(event.currentTarget.value)}
            disabled={isStreaming || selectedProviderModelOptions.length === 0}
          >
            {#each selectedProviderModelOptions as option}
              <option value={option.key}>{option.model_id}</option>
            {/each}
          </select>
        </div>
      </div>
      <textarea
        bind:value={customRegenerateHint}
        class="compact-textarea"
        placeholder="额外提示，可留空。例如：多讲固定搭配；例句更长一点..."
        disabled={isStreaming}
      ></textarea>
    </div>
  {/if}

  <!-- 应用与例句: 结构化展示 -->
  <div class="bento-card" class:card-main={!useBranchUI} class:card-full={useBranchUI}>
    <div class="card-title-row">
      <div class="card-title"><i class="ph-fill ph-lightbulb"></i> 应用与例句</div>
      {#if result.entry_id > 0 && onAddPhraseModule}
        <PhraseModuleAddControl
          disabled={isStreaming}
          isLoading={isAddingPhraseModule}
          onSubmit={onAddPhraseModule}
        />
      {/if}
    </div>
    <div class="usage-list">
      {#if usageFeed.length > 0}
        {#each usageFeed as usage}
          <div class="surface-card usage-item" class:is-attached={usage.kind === "attached"}>
            <div class="usage-head">
              <p class="small-copy"><strong>{usage.title}</strong></p>
              {#if usage.kind === "attached"}
                <span class="mini-label source-flash attached-phrase-label">短语追加</span>
                <button
                  class="attached-trash-btn"
                  type="button"
                  onclick={() => onDeleteAttachedPhrase?.(usage.attachment)}
                  disabled={isUpdatingPhraseAttachment || isStreaming}
                  title="删除这条短语卡片"
                  aria-label={`删除短语卡片：${usage.attachment.phrase}`}
                >
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path d="M9 3h6l1 2h4v2H4V5h4l1-2Z" />
                    <path d="M6 9h12l-.75 11A2.2 2.2 0 0 1 15.06 22H8.94a2.2 2.2 0 0 1-2.19-2L6 9Zm4 2v8h2v-8h-2Zm4 0v8h2v-8h-2Z" />
                  </svg>
                </button>
              {/if}
            </div>
            {#if usage.explanation}
              <div class="card-copy usage-explanation markdown-compact">{@html renderMarkdownHtml(usage.explanation)}</div>
            {/if}
            {#if usage.example?.de}
              <div class="example-box">
                <div class="de-line markdown-compact">{@html renderMarkdownHtml(usage.example.de)}</div>
                {#if usage.example.zh}
                  <div class="zh-line markdown-compact">{@html renderMarkdownHtml(usage.example.zh)}</div>
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
            <div class="de-line markdown-compact">{@html renderMarkdownHtml(example.de)}</div>
            {#if example.zh}
              <div class="zh-line markdown-compact">{@html renderMarkdownHtml(example.zh)}</div>
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
  {#if !useBranchUI}
    <div class="bento-card card-side">
      <div class="card-title"><i class="ph-fill ph-text-aa"></i> 语法特性</div>
      <GrammarFeatureCard
        grammarRows={structured.grammarRows}
        isStreaming={isStreaming}
      />
    </div>
  {/if}

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
      {:else if !isStreaming}
        <div class="markdown-compact surface-card">
          {@html renderMarkdownHtml(result.analysis_markdown)}
        </div>
      {/if}
    </div>

    {#if structured.wordNetwork.family.length > 0 || structured.wordNetwork.synonyms.length > 0 || structured.wordNetwork.antonyms.length > 0}
      <div class="tag-cloud-mini">
        {#each [
          ...structured.wordNetwork.family,
          ...structured.wordNetwork.synonyms,
          ...structured.wordNetwork.antonyms
        ] as item}
          <span
            class="mini-tag-btn"
            title="{item.partOfSpeech ? `[${item.partOfSpeech}] ` : ''}${item.chinese || item.english || ''}"
          >
            {item.term}
          </span>
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
          modelOverride={selectedActionModelOverride}
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
  .bento-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 1.25rem;
    width: 100%;
    max-width: 1400px;
    margin: 0 auto 4rem;
    padding: 0 1rem;
    box-sizing: border-box;
  }

  /* Header 优化 */
  .card-header {
    grid-column: span 4; display: flex; justify-content: space-between; align-items: flex-end;
    padding: 1.5rem 2rem;
  }
  .header-main-info { display: flex; flex-direction: column; gap: 0.4rem; min-width: 0; }

  .title-row { display: flex; align-items: baseline; gap: 0.8rem; }
  .word-title { font-size: 2.2rem; font-weight: 800; color: var(--text-main); letter-spacing: -0.02em; margin: 0; }

  /* 音标专用字体栈修复：强制使用标准 IPA 支持字体 */
  .word-phonetic {
    font-family: "Arial Unicode MS", "Lucida Sans Unicode", sans-serif;
    color: var(--text-muted); font-size: 1.1rem; line-height: 1.6;
    background: var(--bg-color); padding: 0.1rem 0.4rem; border-radius: 4px;
  }

  .meaning-row { display: flex; align-items: center; gap: 0.6rem; margin-top: 0.2rem; }

  /* Branch UI 专属样式 */
  .meaning-row.is-branch-list {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.35rem;
    margin-top: 0.85rem;
  }
  .branch-group-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 1.25rem;
    width: 100%;
  }
  .branch-meaning-item {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.25rem 0.5rem;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    transition: all 0.2s ease;
    cursor: pointer;
    margin-left: -0.5rem; /* 抵消内边距，保持视觉对齐 */
  }
  .branch-meaning-item:hover {
    background: var(--btn-secondary);
    transform: translateX(4px);
  }
  .branch-info-icon {
    font-size: 0.85rem;
    color: var(--text-muted);
    opacity: 0;
    transition: opacity 0.2s ease;
  }
  .branch-meaning-item:hover .branch-info-icon {
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
  .en-text { font-size: 0.98rem; font-style: italic; color: var(--text-muted); }

  .meta-row { display: flex; gap: 0.5rem; margin-top: 0.4rem; }
  .mini-label { font-size: 0.65rem; font-weight: 800; padding: 0.15rem 0.4rem; border-radius: 4px; text-transform: uppercase; opacity: 0.7; }
  .source-db { background: #e0f2fe; color: #0369a1; }
  .source-flash { background: #dcfce7; color: #15803d; }
  .source-pro { background: #f5f3ff; color: #6d28d9; }
  .structured-badge {
    background: #ecfdf5;
    color: #059669;
    border: 1px solid #10b981;
    display: flex;
    align-items: center;
    gap: 0.2rem;
  }
  .model-name { background: var(--bg-color); color: var(--text-muted); border: 1px solid var(--border-color); }

  .header-actions { display: flex; flex-direction: column; align-items: flex-end; gap: 0.75rem; }
  .action-icons { display: flex; gap: 0.5rem; }
  .icon-btn {
    width: 2.2rem; height: 2.2rem; border-radius: 50%; background: var(--btn-secondary);
    color: var(--text-main); display: flex; align-items: center; justify-content: center; font-size: 1rem;
  }
  .icon-btn.is-active {
    color: var(--accent-main);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent-main) 35%, transparent);
  }
  .icon-btn.is-custom-action {
    background: var(--accent-main);
    color: #fff;
    box-shadow: 0 4px 15px var(--accent-glow);
  }
  .learn-btn { padding: 0.5rem 1rem; font-size: 0.85rem; }

  .custom-regenerate-card {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .custom-action-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }
  .custom-action-head .card-title {
    margin-bottom: 0;
    flex: 0 0 auto;
  }
  .custom-model-controls {
    display: flex;
    justify-content: flex-end;
    gap: 0.55rem;
    min-width: 0;
    flex: 1 1 auto;
  }
  .custom-model-controls select {
    min-height: 2.15rem;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--bg-color);
    color: var(--text-main);
    padding: 0 0.65rem;
    font-size: 0.82rem;
    font-weight: 700;
    min-width: 8.5rem;
    max-width: 16rem;
  }
  .custom-model-controls select:first-child { max-width: 10rem; }
  .custom-model-controls select:last-child { flex: 1 1 12rem; }
  /* 其它卡片微调 */
  .card-main { grid-column: span 3; display: flex; flex-direction: column; gap: 1rem; }
  .card-side { grid-column: span 1; display: flex; flex-direction: column; gap: 1rem; }
  .card-full { grid-column: span 4; }

  .card-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }
  .attached-trash-btn svg {
    width: 1.05rem;
    height: 1.05rem;
    fill: currentColor;
  }

  .usage-list, .insights-stack { display: flex; flex-direction: column; gap: 1rem; }
  .usage-item { position: relative; }
  .usage-item.is-attached { padding-right: 3rem; }
  .usage-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
  .attached-phrase-label {
    margin-right: 1.6rem;
  }
  .attached-trash-btn {
    position: absolute;
    top: 0.75rem;
    right: 0.75rem;
    width: 2rem;
    height: 2rem;
    border-radius: 999px;
    color: var(--text-muted);
    background: color-mix(in srgb, var(--card-bg) 84%, transparent);
    border: 1px solid var(--border-color);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transform: translateY(-4px) scale(0.96);
    pointer-events: none;
    transition: opacity 0.18s ease, transform 0.18s ease, color 0.18s ease, background 0.18s ease;
  }
  .usage-item.is-attached:hover .attached-trash-btn,
  .attached-trash-btn:focus-visible {
    opacity: 1;
    transform: translateY(0) scale(1);
    pointer-events: auto;
  }
  .attached-trash-btn:hover:not(:disabled) {
    color: #dc2626;
    background: color-mix(in srgb, #fee2e2 82%, var(--card-bg));
  }
  .attached-trash-btn:disabled {
    cursor: wait;
    opacity: 0.5;
  }
  .example-box { background: var(--bg-color); padding: 1rem; border-radius: 8px; border-left: 3px solid var(--border-color); }
  .de-line { font-weight: 700; font-size: 1.05rem; }
  .zh-line { color: var(--text-muted); font-size: 0.9rem; margin-top: 0.4rem; }

  .tag-cloud-mini { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: 1.5rem; }
  .mini-tag-btn { font-size: 0.8rem; padding: 0.25rem 0.6rem; background: var(--bg-color); border-radius: 6px; border: 1px solid var(--border-color); }

  .recent-mini-list { display: flex; flex-direction: column; gap: 0.6rem; }
  .recent-mini-btn {
    width: 100%; padding: 0.75rem; text-align: left; background: var(--bg-color); border-radius: 8px;
    display: flex; flex-direction: column; border: 1px solid transparent;
  }
  .recent-mini-btn:hover { border-color: var(--accent-main); }
  .recent-mini-btn .p-text {
    font-size: 0.8rem;
    color: var(--text-muted);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    display: block;
    width: 100%;
  }

  .compact-textarea { width: 100%; min-height: 4rem; padding: 0.75rem; border-radius: 8px; background: var(--bg-color); border: 1px solid var(--border-color); font-size: 0.9rem; resize: vertical; }
  @media (max-width: 900px) {
    .bento-grid { grid-template-columns: minmax(0, 1fr); }
    .card-header, .card-main, .card-side, .card-full { grid-column: span 1; }
    .card-header { flex-direction: row; align-items: center; padding: 1rem; }
    .header-actions { flex-direction: row; }
    .custom-action-head { align-items: flex-start; flex-direction: column; }
    .custom-model-controls { width: 100%; justify-content: flex-start; }
    .custom-model-controls select { min-width: 0; }
    .card-title-row {
      align-items: flex-start;
      flex-direction: column;
    }
  }

  @media (max-width: 600px) {
    .card-header { align-items: stretch; flex-direction: column; gap: 1rem; }
    .header-actions { align-items: center; flex-direction: row; justify-content: flex-end; }
    .title-row, .meta-row { flex-wrap: wrap; }
  }
</style>
