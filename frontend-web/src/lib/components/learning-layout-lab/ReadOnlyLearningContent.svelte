<script lang="ts">
  import { renderMarkdownHtml } from "$lib/analysis/structuredAnalysis";
  import type { GrammarBranch, StructuredAnalysisDocument } from "$lib/types";

  let { analysis, onGrammarOpen }: {
    analysis: StructuredAnalysisDocument;
    onGrammarOpen: (branch: GrammarBranch, triggerRect: DOMRect) => void;
  } = $props();

  const branch = $derived(analysis.grammar_branches?.[0] ?? null);
  const branchChinese = $derived(branch?.meanings.map((meaning) => meaning.zh).filter(Boolean).join("；") ?? "");
  const branchEnglish = $derived(branch?.meanings.map((meaning) => meaning.en).filter(Boolean).join("; ") ?? "");
  const networkGroups = $derived([
    { label: "词族", items: analysis.word_network?.family ?? [] },
    { label: "近义", items: analysis.word_network?.synonyms ?? [] },
    { label: "反义", items: analysis.word_network?.antonyms ?? [] },
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
    <button
      class="meaning-summary"
      class:interactive={branch}
      onclick={(event) => branch && onGrammarOpen(branch, event.currentTarget.getBoundingClientRect())}
      disabled={!branch}
      title={branch ? "点击查看详细语法" : undefined}
    >
      <span>{branch ? formatPos(branch) : analysis.meanings[0]?.part_of_speech}</span>
      <div><strong>{branchChinese || analysis.meanings.map((meaning) => meaning.chinese).join("；")}</strong>{#if branchEnglish}<small>{branchEnglish}</small>{/if}</div>
      {#if branch}<i class="ph ph-info"></i>{/if}
    </button>
  </section>

  <section class="section-card usage-section">
    <div class="section-title"><i class="ph-fill ph-lightbulb"></i> 应用与例句</div>
    <div class="usage-list">
      {#each analysis.usage_modules as usage}
        <article class="usage-item">
          <strong>{usage.title}</strong>
          <div class="usage-copy">{@html renderMarkdownHtml(usage.explanation)}</div>
          <div class="example-box">
            <div class="de-line">{@html renderMarkdownHtml(usage.example_de)}</div>
            <div class="zh-line">{@html renderMarkdownHtml(usage.example_zh)}</div>
          </div>
        </article>
      {/each}
    </div>
  </section>

  <section class="section-card insight-section">
    <div class="section-title"><i class="ph-fill ph-brain"></i> 深度解析与避坑</div>
    <div class="insights-stack">
      {#each analysis.deep_insights as insight}
        <article class="insight-block">
          <strong>{insight.title}</strong>
          <div class="insight-copy">{@html renderMarkdownHtml(insight.content_markdown)}</div>
        </article>
      {/each}
    </div>
  </section>

  {#if networkGroups.length}
    <section class="section-card network-section">
      <div class="section-title"><i class="ph-fill ph-share-network"></i> 词汇网络</div>
      {#each networkGroups as group}
        <div class="network-group">
          <span class="network-label">{group.label}</span>
          <div class="network-grid">
            {#each group.items as item}
              <article>
                <strong>{item.term}{#if item.part_of_speech}<small>{item.part_of_speech}</small>{/if}</strong>
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

  .meaning-summary { display: flex; align-items: flex-start; gap: 0.8rem; width: 100%; padding: 1rem; border-radius: var(--radius-md); background: color-mix(in srgb, var(--accent-main) 8%, var(--bg-color)); color: var(--text-main); text-align: left; }
  .meaning-summary.interactive:hover { background: color-mix(in srgb, var(--accent-main) 12%, var(--bg-color)); }
  .meaning-summary:disabled { cursor: default; }
  .meaning-summary > span { flex: 0 0 auto; padding: 0.22rem 0.6rem; border-radius: 6px; background: var(--accent-main); color: white; font-size: 0.8rem; font-weight: 800; }
  .meaning-summary > div { display: grid; flex: 1; gap: 0.3rem; }
  .meaning-summary strong { font-size: 1.06rem; line-height: 1.55; }
  .meaning-summary small { color: var(--text-muted); line-height: 1.45; }
  .meaning-summary > i { align-self: center; color: var(--text-muted); opacity: 0; transition: opacity 0.15s ease; }
  .meaning-summary.interactive:hover > i, .meaning-summary.interactive:focus-visible > i { opacity: 1; }

  .usage-list, .insights-stack { display: grid; gap: 0.8rem; }
  .usage-item, .insight-block { padding: 1rem; border-radius: var(--radius-md); background: var(--bg-color); }
  .usage-copy, .insight-copy { margin-top: 0.45rem; color: var(--text-muted); line-height: 1.65; }
  .example-box { margin-top: 0.85rem; padding: 0.85rem 1rem; border-left: 3px solid var(--accent-main); border-radius: 8px; background: var(--card-bg); }
  .de-line { font-weight: 750; }
  .zh-line { margin-top: 0.35rem; color: var(--text-muted); font-size: 0.9rem; }

  .network-group + .network-group { margin-top: 1rem; }
  .network-label { display: block; margin-bottom: 0.55rem; color: var(--text-muted); font-size: 0.76rem; font-weight: 800; }
  .network-grid { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  .network-grid article { display: grid; gap: 0.12rem; max-width: 240px; padding: 0.48rem 0.7rem; border: 1px solid var(--border-color); border-radius: 9px; background: var(--bg-color); }
  .network-grid article strong { display: flex; align-items: baseline; gap: 0.4rem; font-size: 0.86rem; }
  .network-grid article > span { overflow: hidden; color: var(--text-muted); font-size: 0.76rem; text-overflow: ellipsis; white-space: nowrap; }
  .network-grid small { color: var(--accent-main); font-size: 0.67rem; font-weight: 750; }

</style>
