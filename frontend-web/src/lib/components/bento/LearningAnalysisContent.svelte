<script lang="ts">
  import { renderMarkdownHtml, type StructuredAnalysis } from "$lib/analysis/structuredAnalysis";
  import type { AttachedPhraseModule, GrammarBranch, ModelAMeaning } from "$lib/types";

  let {
    analysis,
    attachedPhraseModules = [],
    onGrammarOpen,
    usageElement = $bindable(null),
  } = $props<{
    analysis: StructuredAnalysis;
    attachedPhraseModules?: AttachedPhraseModule[];
    onGrammarOpen: (branch: GrammarBranch, triggerRect: DOMRect) => void;
    usageElement?: HTMLElement | null;
  }>();

  const usageModules = $derived([
    ...analysis.usageModules,
    ...attachedPhraseModules.flatMap((item: AttachedPhraseModule) => item.usage_module ? [{
      title: item.usage_module.title,
      explanation: item.usage_module.explanation,
      example: {
        de: item.usage_module.example_de,
        zh: item.usage_module.example_zh,
      },
    }] : []),
  ]);
  const networkGroups = $derived([
    { label: "词族", items: analysis.wordNetwork.family },
    { label: "近义", items: analysis.wordNetwork.synonyms },
    { label: "反义", items: analysis.wordNetwork.antonyms },
  ].filter((group) => group.items.length > 0));

  function formatPos(value: GrammarBranch) {
    const posTokens = value.pos.toLowerCase().split(/[\s,]+/);
    const grammar = value.grammar;
    if (posTokens.includes("verb")) {
      const transitivity = { transitive: "vt.", intransitive: "vi.", both: "vt./vi." }[grammar.transitivity] ?? "v.";
      const reflexive = ["optional", "required"].includes(grammar.reflexive) ? " refl." : "";
      const separable = grammar.separable === "separable" ? " (sep.)" : "";
      return `${transitivity}${reflexive}${separable}`;
    }
    if (posTokens.includes("noun")) {
      const genderMap: Record<string, string> = { masculine: "m.", feminine: "f.", neuter: "n.", plural: "pl." };
      const genders = grammar.genders.map((gender) => genderMap[gender.toLowerCase()] ?? "").filter(Boolean);
      return `${genders.length ? `(${genders.join("/")}) ` : ""}n.`;
    }
    if (posTokens.includes("adjective") && posTokens.includes("adverb")) return "adj./adv.";
    if (posTokens.includes("adjective")) return "adj.";
    if (posTokens.includes("adverb")) return "adv.";
    if (posTokens.includes("pronoun")) return "pron.";
    if (posTokens.includes("preposition")) return "prep.";
    if (posTokens.includes("conjunction")) return "conj.";
    if (posTokens.includes("article")) return "art.";
    return value.pos;
  }
</script>

<div class="learning-content">
  <section class="section-card meaning-section">
    <div class="section-title"><i class="ph-fill ph-translate"></i> 核心释义</div>
    <div class="meaning-list">
      {#if analysis.grammarBranches.length}
        {#each analysis.grammarBranches as branch}
          <button
            class="meaning-summary interactive"
            onclick={(event) => onGrammarOpen(branch, event.currentTarget.getBoundingClientRect())}
            title="点击查看详细语法"
          >
            <span>{formatPos(branch)}</span>
            <div>
              <strong>{branch.meanings.map((meaning: ModelAMeaning) => meaning.zh).filter(Boolean).join("；")}</strong>
              {#if branch.meanings.some((meaning: ModelAMeaning) => meaning.en)}
                <small>{branch.meanings.map((meaning: ModelAMeaning) => meaning.en).filter(Boolean).join("; ")}</small>
              {/if}
            </div>
            <i class="ph ph-info"></i>
          </button>
        {/each}
      {:else}
        {#each analysis.meanings as meaning}
          <div class="meaning-summary">
            <span>{meaning.partOfSpeech}</span>
            <div><strong>{meaning.chinese}</strong>{#if meaning.english}<small>{meaning.english}</small>{/if}</div>
          </div>
        {/each}
      {/if}
    </div>
  </section>

  {#if usageModules.length || analysis.examples.length}
    <section class="section-card usage-section" bind:this={usageElement}>
      <div class="section-title"><i class="ph-fill ph-lightbulb"></i> 应用与例句</div>
      <div class="usage-list">
        {#each usageModules as usage}
          <article class="usage-item">
            <strong>{usage.title}</strong>
            <div class="usage-copy">{@html renderMarkdownHtml(usage.explanation)}</div>
            {#if usage.example}
              <div class="example-box">
                <div class="de-line">{@html renderMarkdownHtml(usage.example.de)}</div>
                <div class="zh-line">{@html renderMarkdownHtml(usage.example.zh)}</div>
              </div>
            {/if}
          </article>
        {/each}
        {#if usageModules.length === 0}
          {#each analysis.examples as example}
            <article class="usage-item example-box">
              <div class="de-line">{@html renderMarkdownHtml(example.de)}</div>
              <div class="zh-line">{@html renderMarkdownHtml(example.zh)}</div>
            </article>
          {/each}
        {/if}
      </div>
    </section>
  {/if}

  {#if analysis.deepInsights.length}
    <section class="section-card insight-section">
      <div class="section-title"><i class="ph-fill ph-brain"></i> 深度解析与避坑</div>
      <div class="insights-stack">
        {#each analysis.deepInsights as insight}
          <article class="insight-block">
            <strong>{insight.title}</strong>
            <div class="insight-copy">{@html insight.html}</div>
          </article>
        {/each}
      </div>
    </section>
  {/if}

  {#if networkGroups.length}
    <section class="section-card network-section">
      <div class="section-title"><i class="ph-fill ph-share-network"></i> 词汇网络</div>
      {#each networkGroups as group}
        <div class="network-group">
          <span class="network-label">{group.label}</span>
          <div class="network-grid">
            {#each group.items as item}
              <article>
                <strong>{item.term}{#if item.partOfSpeech}<small>{item.partOfSpeech}</small>{/if}</strong>
                <span>{item.chinese}</span>
              </article>
            {/each}
          </div>
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  .learning-content { display: grid; gap: 1rem; padding: 0 0 2rem; }
  .section-card { padding: 1.2rem; border: 1px solid var(--border-color); border-radius: var(--radius-lg); background: color-mix(in srgb, var(--card-bg) 86%, var(--bg-color)); }
  .section-title { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 1rem; color: var(--text-muted); font-size: 0.82rem; font-weight: 800; letter-spacing: 0.05em; }
  .section-title i { color: var(--accent-main); font-size: 1rem; }

  .meaning-list { display: grid; gap: 0.6rem; }
  .meaning-summary { display: flex; align-items: flex-start; gap: 0.8rem; width: 100%; padding: 1rem; border-radius: var(--radius-md); background: color-mix(in srgb, var(--accent-main) 8%, var(--bg-color)); color: var(--text-main); text-align: left; }
  .meaning-summary.interactive:hover { background: color-mix(in srgb, var(--accent-main) 12%, var(--bg-color)); }
  .meaning-summary > span { flex: 0 0 auto; padding: 0.22rem 0.6rem; border-radius: 6px; background: var(--accent-main); color: white; font-size: 0.8rem; font-weight: 800; }
  .meaning-summary > div { display: grid; flex: 1; gap: 0.3rem; min-width: 0; }
  .meaning-summary strong { font-size: 1.06rem; line-height: 1.55; }
  .meaning-summary small { color: var(--text-muted); line-height: 1.45; }
  .meaning-summary > i { align-self: center; color: var(--text-muted); opacity: 0; transition: opacity 0.15s ease; }
  .meaning-summary.interactive:hover > i, .meaning-summary.interactive:focus-visible > i { opacity: 1; }

  .usage-list, .insights-stack { display: grid; gap: 0.8rem; }
  .usage-item, .insight-block { padding: 1rem; border-radius: var(--radius-md); background: var(--bg-color); }
  .usage-copy, .insight-copy { margin-top: 0.45rem; color: var(--text-muted); line-height: 1.65; }
  .example-box { margin-top: 0.85rem; padding: 0.85rem 1rem; border-left: 3px solid var(--accent-main); border-radius: 8px; background: var(--card-bg); }
  .usage-item.example-box { margin-top: 0; }
  .de-line { font-weight: 750; }
  .zh-line { margin-top: 0.35rem; color: var(--text-muted); font-size: 0.9rem; }

  .network-group + .network-group { margin-top: 1rem; }
  .network-label { display: block; margin-bottom: 0.55rem; color: var(--text-muted); font-size: 0.76rem; font-weight: 800; }
  .network-grid { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  .network-grid article { display: grid; gap: 0.12rem; max-width: 240px; padding: 0.48rem 0.7rem; border: 1px solid var(--border-color); border-radius: 9px; background: var(--bg-color); }
  .network-grid article strong { display: flex; align-items: baseline; gap: 0.4rem; font-size: 0.86rem; }
  .network-grid article > span { overflow: hidden; color: var(--text-muted); font-size: 0.76rem; text-overflow: ellipsis; white-space: nowrap; }
  .network-grid small { color: var(--accent-main); font-size: 0.67rem; font-weight: 750; }

  @media (max-width: 600px) {
    .section-card { padding: 0.9rem; }
    .meaning-summary { padding: 0.85rem; }
  }
</style>
