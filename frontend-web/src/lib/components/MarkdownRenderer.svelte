<script lang="ts">
  import { renderMarkdownHtml } from "$lib/analysis/structuredAnalysis";
  import type { FollowUpItem } from "$lib/types";

  export let markdownContent = "";
  export let followUps: FollowUpItem[] = [];

  let renderedHtml = "";

  $: {
    const history =
      followUps.length === 0
        ? ""
        : `\n\n---\n\n## 追问历史\n\n${followUps
            .map((item) => `**你问：** ${item.question}\n\n**AI答：** ${item.answer}`)
            .join("\n\n---\n\n")}`;

    renderedHtml = renderMarkdownHtml(`${markdownContent}${history}`);
  }
</script>

<div class="markdown">
  {@html renderedHtml}
</div>

<style>
  .markdown {
    color: var(--text);
    line-height: 1.75;
  }

  .markdown :global(h1),
  .markdown :global(h2),
  .markdown :global(h3),
  .markdown :global(h4) {
    font-family: var(--font-ui);
    margin: 1.2rem 0 0.6rem;
    color: var(--accent-strong);
  }

  .markdown :global(p),
  .markdown :global(li) {
    color: var(--text);
  }

  .markdown :global(code) {
    background: var(--panel-alt);
    padding: 0.1rem 0.35rem;
    border-radius: 0.25rem;
  }

  .markdown :global(table) {
    width: 100%;
    border-collapse: collapse;
    margin: 1rem 0;
  }

  .markdown :global(th),
  .markdown :global(td) {
    border: 1px solid var(--border);
    padding: 0.65rem;
    text-align: left;
  }
</style>
